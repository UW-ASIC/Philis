//! # `gp` — global placement (an Algorithm-contract stage)
//!
//! Exposes the [`GlobalPlacer`] trait and two impls: the real [`Analytical`]
//! placer (momentum gradient descent on HPWL + density, nudged by the analog
//! `Cost` rules) and a [`Placeholder`]. Concrete algorithms own their own
//! mechanics — that's what makes them swappable. The **active** algorithm is
//! chosen in exactly ONE place: `frontend/library`'s `algorithms` selection
//! module.
//!
//! The objective is **analog-blind**: built-in HPWL + overlap/density, plus the
//! slack-blended analog cost `Σ criticality(b)·cost(b)`, plus the priced budget
//! residuals `Σ −λ_b·residual_b`. No analog weight is hardcoded here —
//! thermal/proximity/isolation weights arrive inside the rules via `reqs`, and the
//! multipliers arrive in [`Prices`] ([`mechanics::analog_cost`]).
//!
//! Legality = `Σ reqs.hard[b].violations == 0` plus zero device overlap, and it
//! **gates the descent**: a step that raises Φ = `(violating batches, Σ residual)`
//! lexicographically is backtracked and the step size halved, so the coarse stage
//! cannot walk through a hard rule and leave `dp` to recover.
//!
//! `gp` does **not** search variants (D8) — it writes a starting `Layout::variant`,
//! sizes every extent and net from the chosen alternative, and leaves the reshape move
//! to `dp`.
//!
//! [`mechanics`] holds the shared SA/placement primitives (`SplitMix64`, HPWL
//! from macro pins, overlap, objective/legality wiring, canvas). `backend/dp`
//! depends on this crate to reuse them.

pub mod mechanics;

use analog::Requirements;
use pnr_core::geom::LayerId;
use pnr_core::{Layout, Macro, Report};

use mechanics::{
    analog_cost, analog_phi, canvas_side, choose_variants, clamp_to_die, half_extents,
    initial_layout, report, Nets,
    SplitMix64,
};

/// A global-placement algorithm. Consumes the placement [`Requirements`] (the hot
/// per-move rules — including the `verify`-fed DRC/LVS/ERC **Hard** rules and the
/// PEX **Cost** rule), and produces a coarse placement + satisfaction [`Report`].
/// The drawn alternatives for **one** placeable cell — the variant space the
/// placer may search over (PLAN §2).
///
/// Pre-drawn geometry, not a generator handle. Deliberate: `(group, variant) ->
/// Macro` is a pure, byte-deterministic function of the generators, so it is drawn
/// **once per run** and reused across every epoch and every trial move. A callback
/// would redraw on each proposal, and a placer able to redraw would have to depend
/// on `cells`, which it must not.
///
/// Ordering is the enumeration order of `cells::Cell::enumerate` after group
/// collapse, so `alternatives[layout.variant[i]]` is cell `i`'s current geometry.
///
/// The space arrives **already collapsed**. Per PLAN §2 the same-variant and
/// integer-ratio requirements on a matched group are equality constraints on the
/// group's variant vector, shrinking the naive product space `|Var|^|g|` to a
/// handful of legal joint assignments. That collapse is what makes the search
/// tractable, it happens upstream, and the placer must not try to recombine
/// survivors per-device — doing so re-enters the illegal region the collapse
/// removed.
pub struct VariantSpace {
    pub alternatives: Vec<Macro>,
    /// **Matching lock.** Cells sharing a lock id must hold the **same** variant
    /// index at all times; `None` is a cell that may be reshaped alone.
    ///
    /// This is the one piece of PLAN §2's group collapse that reaches a placer today.
    /// The collapse itself is still outstanding (`cellgen::enumerate` emits one cell
    /// per *device*), so a differential pair is two independent cells with two
    /// independent variant indices — and a reshape move that draws one of them
    /// uniformly will happily give one half of the pair a different fingering,
    /// footprint and pin face from the other. DRC cannot see it, LVS cannot see it,
    /// and matching is the entire purpose of the pair. The lock is what `dp` needs to
    /// reshape the members **jointly** instead.
    ///
    /// `dp` gets `Requirements<Layout>`, not `analog::Constraints`, so it cannot read
    /// `Unitization::same_variant_required` itself; the producer resolves that
    /// constraint into dense ids here (`library::cellgen::enumerate`).
    ///
    /// Contract on the producer, because "the same index" is otherwise meaningless:
    /// **cells sharing a lock id must have index-compatible variant spaces** — the same
    /// alternatives in the same enumeration order, or at least the same count with
    /// index `v` naming the same construction in each. A consumer may assume nothing
    /// beyond the count and must not index past the shortest member's space.
    pub lock: Option<u16>,
}

/// **Augmented-Lagrangian state**: the multiplier λ and penalty weight ρ per budget
/// batch, carried across epochs.
///
/// Owned by the orchestrator and passed `&mut`, for the same two reasons as
/// `gr::Negotiation`:
///
/// 1. **The ratchet only works if it persists.** PLAN §6 raises ρ for constraints
///    whose residual is *not* shrinking — a statement about history.
///    Without it [`mechanics::analog_cost`] weights each batch by its live budget
///    headroom, an adaptive penalty rebuilt from nothing on every call, so a budget
///    that has been binding for fifty epochs is priced exactly like one that just
///    became binding. That is the "silently consuming margin" failure PLAN §3b names.
/// 2. **The terminator must read it.** PLAN §5 terminates on
///    `‖λ_{k+1} − λ_k‖ < δ` — multiplier stationarity, which certifies that no
///    budget is being actively traded. Feasibility alone is explicitly *not*
///    termination.
///
/// λ is the **shadow price** of its constraint: it names which budgets are actually
/// binding, which is the diagnostic PLAN §6 promotes on.
pub struct Prices {
    // ponytail: opaque. Batch identity is `Requirements`' business, and publishing a
    // keying scheme here would freeze the type-erased batch layout into this crate's
    // API before the `kernel/analog` three-way split has decided what a batch is.
    //
    // Keyed by `(rule-kind name, ordinal among same-kind batches)` and NOT by
    // position in `reqs.budget`. `Requirements::budget`'s doc calls its order a
    // contract, but a positional key is only stable if the annotator is deterministic
    // (it is) AND the in-loop feedback batches are *appended* rather than interleaved,
    // and a reordering there "yields plausible wrong numbers rather than a crash" —
    // one budget silently inheriting another's accumulated price. Keying by kind
    // removes the dependency instead of resting on it; `bind` still asserts the
    // append-only shape, because a violation means the annotator moved and the caller
    // should hear about it even though the prices survive.
    //
    // `BTreeMap`, not `HashMap`, for `gr::Negotiation`'s reason: `drift()` is a float
    // reduction over every entry, so iteration order must be fixed or the
    // stationarity probe PLAN §5 terminates on is nondeterministic.
    priced: std::collections::BTreeMap<Key, Price>,
    /// This epoch's `−λ` per **positional** batch index — the hot-path read.
    /// Rebuilt by [`Prices::bind`] from `priced`; empty ⇒ every weight reads `0`,
    /// which reproduces the pre-price objective exactly.
    weight: Vec<f32>,
    /// `‖λ_{k+1} − λ_k‖` from the last [`Prices::settle`]. `INFINITY` until one has
    /// run: no dual step has happened, so nothing is stationary yet.
    drift: f64,
    /// Last epoch's key order, for the append-only assertion in [`Prices::bind`].
    order: Vec<Key>,
}

/// Hand-written rather than derived for one field: `drift` starts at `INFINITY`, not
/// `0.0`. A derived `Default` would hand the terminator a fresh `Prices` reading
/// "prices are perfectly stationary" before a single dual step had run, which is the
/// one answer PLAN §5 must never get wrong.
impl Default for Prices {
    fn default() -> Self {
        Self {
            priced: std::collections::BTreeMap::new(),
            weight: Vec::new(),
            drift: f64::INFINITY,
            order: Vec::new(),
        }
    }
}

/// `(rule-kind name, ordinal among same-kind batches)`.
type Key = (&'static str, u32);

/// One batch's augmented-Lagrangian state.
#[derive(Clone, Copy, Debug)]
struct Price {
    /// The multiplier. **Non-positive**, because PLAN §3b writes the Lagrangian as
    /// `L_ρ(x,λ) = f(x) − λᵀc(x)` and its dual step as `λ ← λ − ρ·c(x)` with a
    /// non-negative residual `c`. So λ walks downward and `−λ` is the accumulated
    /// price. The sign is kept rather than folded so [`Prices::settle`] reads exactly
    /// as PLAN writes it.
    lambda: f32,
    /// Penalty weight ρ, ratcheted for a residual that is not shrinking (PLAN §6).
    rho: f32,
    /// Residual at the previous dual step, for the shrink test. `INFINITY` before the
    /// first one so a batch's debut never counts as stalled.
    residual: f32,
}

/// ρ's floor. The residual is normalised (a fraction of the batch's own budget), so a
/// batch sitting one full budget past spec accrues `0.25` of price per epoch and takes a
/// handful of epochs to dominate rather than one — PLAN §6 wants budgets *ramped* to
/// effectively hard, not switched.
const RHO_FLOOR: f32 = 0.25;
/// Escalation factor for a residual that is not shrinking.
const RHO_GAIN: f32 = 2.0;
const RHO_MAX: f32 = 64.0;
/// Cap on `|λ|`.
///
/// ponytail: a flat cap, not a proper multiplier reset. A budget that is *physically*
/// unsatisfiable (a die too small for its devices) has a residual that never shrinks,
/// so its price would climb without bound and eventually swamp every other term in the
/// objective — the cap is what stops one impossible constraint from erasing the rest of
/// the layout. Ceiling: a batch that reaches the cap stops moving, which `drift()` then
/// reads as "settled" when it is really "saturated"; a caller wanting to tell those
/// apart needs the residual, which `Report::budget_violations` carries. Upgrade path is
/// the textbook one — reset λ and restart ρ when the residual stalls at a positive
/// floor, which is also PLAN §5's variant-space-binding diagnostic.
const LAMBDA_MAX: f32 = 64.0;

impl Prices {
    /// Fresh prices — first epoch, every λ at zero and ρ at its floor.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `‖λ_{k+1} − λ_k‖` from the most recent dual update — PLAN §5's stationarity
    /// test. Small ⇒ prices have settled ⇒ no budget is being traded.
    ///
    /// `INFINITY` before the first [`Prices::settle`], so "we have not run a dual
    /// step yet" can never be mistaken for "the prices are stationary".
    #[must_use]
    pub fn drift(&self) -> f64 {
        self.drift
    }

    /// Bind the carried prices to **this** epoch's batch order, before the primal
    /// minimisation starts reading them. No dual step.
    ///
    /// Symmetric with `gr::Negotiation::seed`/`accumulate`: `bind` seeds the stage
    /// from the accumulated field, [`Prices::settle`] folds the epoch's result back.
    /// The two-call shape is what lets the hot path read a weight by array index
    /// instead of a map lookup — `mechanics::analog_cost` runs tens of millions of
    /// times per `dp` call.
    pub fn bind(&mut self, reqs: &Requirements<Layout>) {
        let order = keys(reqs);
        // The ordering hazard, asserted rather than assumed. Prices are keyed by rule
        // kind so they survive an interleave regardless; what does not survive is the
        // *reader's* assumption that the annotator emits a stable schedule, and PLAN
        // §8f(iii) is explicit that a nondeterministic price must never reach a
        // decision. `debug_assert` because the failure is a diagnosis, not a
        // corruption — the run stays correct, it just stops being explicable.
        debug_assert!(
            self.order.is_empty()
                || order.starts_with(&self.order)
                || self.order.starts_with(&order),
            "gp::Prices: budget-batch schedule changed shape between epochs\n  was: \
             {:?}\n  now: {:?}\nin-loop feedback batches must be appended, not \
             interleaved",
            self.order,
            order
        );
        // A batch registered in both `hard` and `budget` is gated as legality *and*
        // priced as tradeable — D15's "exactly one arm" broken in the one way that
        // silently corrupts the tier ordering. (The hard+cost pairing is the named
        // exception — a cost copy is a gradient, and Prices never prices `cost`.)
        debug_assert!(
            reqs.budget
                .iter()
                .all(|b| reqs.hard.iter().all(|h| h.kind() != b.kind())),
            "gp::Prices: a batch kind is registered in both `hard` and `budget` — \
             it would be gated as legality and priced as tradeable at once"
        );
        self.weight.clear();
        self.weight.extend(
            order
                .iter()
                .map(|k| self.priced.get(k).map_or(0.0, |p| -p.lambda)),
        );
        self.order = order;
    }

    /// The **dual update**, `λ ← λ − ρ·c(x)` (PLAN §3b), at the primal optimum `l`.
    ///
    /// Called once per `place`, not once per inner iteration: PLAN §3b's alternation
    /// is "a primal minimisation over x, then a dual update", and one `place` call
    /// *is* one primal minimisation. Stepping λ per descent iteration instead would
    /// take 500 dual steps inside a single stage call and saturate every multiplier
    /// before the orchestrator's first feedback epoch, which is the opposite of the
    /// slow ratchet PLAN §6 asks for.
    ///
    /// ρ escalates for a batch whose residual **did not shrink** since the last
    /// step, and holds otherwise — PLAN §6's adaptive-penalty discipline ("increase ρ
    /// for constraints whose residual is not shrinking, decrease pressure on those
    /// with slack"). A batch that has gone slack has residual `0`, so its λ stops
    /// moving and it contributes nothing to [`Prices::drift`]: stationarity falls out
    /// of the same arithmetic instead of needing its own bookkeeping.
    ///
    /// `c(x)` is `RuleBatch::residual` — the *unclamped* overshoot, normalised by each
    /// rule's own budget. Not `criticality`, which saturates at `1.0` the moment a rule
    /// reaches its spec and so cannot tell "just over" from "5× over"; a dual step on a
    /// saturating residual ratchets a barely-missed budget as hard as a hopeless one.
    pub fn settle(&mut self, reqs: &Requirements<Layout>, l: &Layout) {
        let mut drift_sq = 0.0f64;
        for (bi, key) in keys(reqs).into_iter().enumerate() {
            let c = reqs.budget[bi].residual(l).max(0.0) as f32;
            let p = self.priced.entry(key).or_insert(Price {
                lambda: 0.0,
                rho: RHO_FLOOR,
                residual: f32::INFINITY,
            });
            if c > 0.0 && c >= p.residual {
                p.rho = (p.rho * RHO_GAIN).min(RHO_MAX);
            }
            p.residual = c;
            let next = (p.lambda - p.rho * c).max(-LAMBDA_MAX);
            drift_sq += f64::from(next - p.lambda).powi(2);
            p.lambda = next;
        }
        self.drift = drift_sq.sqrt();
        // Republish so a caller that reads the objective after `settle` sees the
        // prices it just paid for.
        self.bind(reqs);
    }

    /// `−λ` for budget batch `bi` in the order [`Prices::bind`] last saw — the hot-path
    /// read. `0.0` for an unbound index, which is what makes a `Prices::default()`
    /// reproduce the pre-price objective.
    #[inline]
    #[must_use]
    pub(crate) fn weight_of(&self, bi: usize) -> f32 {
        self.weight.get(bi).copied().unwrap_or(0.0)
    }
}

/// Stable price keys for `reqs.budget`, in positional order.
///
/// The ordinal is counted by scanning the prefix rather than with a counting map:
/// `reqs.budget` holds one batch per *rule kind*, so it is a handful of entries and the
/// O(k²) scan is cheaper than the allocation — and this runs twice per epoch, not per
/// move.
fn keys(reqs: &Requirements<Layout>) -> Vec<Key> {
    (0..reqs.budget.len())
        .map(|bi| {
            let kind = reqs.budget[bi].kind();
            let ordinal = reqs.budget[..bi].iter().filter(|o| o.kind() == kind).count() as u32;
            (kind, ordinal)
        })
        .collect()
}

pub trait GlobalPlacer {
    /// Coarse-place `macros` to minimise the `Cost` rules while respecting the
    /// `Hard` rules, deterministically for `seed`. `layers` is the PDK-permitted
    /// routing/placement metal stack; global placement is layer-agnostic (it
    /// optimises HPWL + density + analog cost, none of which name a metal), so it
    /// is threaded for a uniform stage contract but not consumed here.
    ///
    /// `variants[i]` are cell `i`'s drawn alternatives. Global placement does **not**
    /// search them — a coarse spread has no routability or parasitic signal to choose
    /// on, and PLAN §2 is explicit that pricing a variant means actually routing it.
    /// `gp`'s only duties here are to write a starting [`Layout::variant`] and to
    /// size `hw`/`hh` from the chosen alternative. Variant *search* belongs to `dp`.
    ///
    /// `prices` carries the augmented-Lagrangian state across epochs; see [`Prices`].
    /// Determinism is over `(inputs, seed, prices)`.
    fn place(
        &self,
        macros: &[Macro],
        variants: &[VariantSpace],
        reqs: &Requirements<Layout>,
        layers: &[LayerId],
        prices: &mut Prices,
        seed: u64,
    ) -> (Layout, Report);
}

/// Tunables for the analytical global stage. Defaults ported from
/// `engine::GlobalCfg` (docs "Stage 1: analytical global placement").
#[derive(Clone, Debug)]
pub struct GlobalCfg {
    pub max_iters: u32,
    pub min_iters: u32,
    pub overflow_target: f32,
    pub utilization: f32,
    pub grid: i32,
    /// Initial step fraction of the die span (engine `step0`).
    pub step0: f32,
    pub step_min: f32,
    pub step_decay: f32,
    pub momentum: f32,
    /// Initial density-push weight (engine `lambda0`).
    pub lambda0: f32,
    pub target_util: f32,
    /// Finite-difference probe (nm) for the analog-cost gradient.
    pub analog_probe: i32,
}

impl Default for GlobalCfg {
    fn default() -> Self {
        Self {
            max_iters: 500,
            min_iters: 60,
            overflow_target: 0.15,
            utilization: 0.4,
            grid: 5,
            step0: 0.04,
            step_min: 0.002,
            step_decay: 0.995,
            momentum: 0.85,
            lambda0: 0.5,
            target_util: 0.7,
            analog_probe: 64,
        }
    }
}

/// The real global placer: analytical (momentum gradient descent + density push)
/// over the analog-blind objective. Swap this in at `frontend/library`.
#[derive(Default)]
pub struct Analytical {
    pub cfg: GlobalCfg,
}

impl GlobalPlacer for Analytical {
    fn place(
        &self,
        macros: &[Macro],
        variants: &[VariantSpace],
        reqs: &Requirements<Layout>,
        _layers: &[LayerId],
        prices: &mut Prices,
        seed: u64,
    ) -> (Layout, Report) {
        // Global placement is layer-agnostic (see the trait doc): the objective is
        // HPWL + density + analog cost, none of which names a routing layer.
        let cfg = &self.cfg;
        let n = macros.len();
        let mut rng = SplitMix64::new(seed);

        // ---- variant seeding (D8: gp proposes, dp searches) ------------------
        // Index 0 of every cell's collapsed alternative set. Not a choice so much as
        // an admission: a coarse spread has no routability or parasitic signal to
        // price a variant with, and PLAN §2 is explicit that pricing one means
        // actually drawing and routing it. So `gp` takes the enumeration's first
        // survivor deterministically and leaves the search to `dp`, where a reshape
        // is scored against real neighbours.
        //
        // Everything downstream then measures the *chosen* geometry — extents,
        // canvas, and net connectivity — because a variant's pins are as much its
        // identity as its bbox.
        let variant = vec![0u16; n];
        let drawn = choose_variants(macros, variants, &variant);

        // Carried λ/ρ, bound to this epoch's batch order before the descent reads
        // the objective (D9). Nothing else in the loop touches `prices`; the dual
        // step is the single `settle` after the descent converges.
        prices.bind(reqs);

        // Canvas: square die from inflated footprints + utilization (docs).
        let (hw, hh) = half_extents(&drawn);
        let side = canvas_side(&hw, &hh, cfg.utilization, cfg.grid);

        // Stage 0: jittered initial layout at the die centre.
        let mut l = initial_layout(&drawn, variant, side, &mut rng);
        l.debug_check("gp: initial_layout");
        debug_assert!(
            (0..n).all(|i| l.hw[i] <= side && l.hh[i] <= side),
            "gp: a macro is larger than the canvas ({side} nm) — canvas_side under-sized it"
        );
        let nets = Nets::from_macros(&drawn);

        if n == 0 {
            let rep = report(&nets, reqs, &l, prices);
            return (l, rep);
        }

        // ---- Stage 1: analytical descent (analog-blind) -------------------
        // gradient buffers (f32 nm-scale) + momentum velocity + density bins.
        let mut gx = vec![0.0f32; n];
        let mut gy = vec![0.0f32; n];
        let mut vx = vec![0.0f32; n];
        let mut vy = vec![0.0f32; n];

        // density bins: clamp(ceil(sqrt(n)),4,24) per docs.
        let nb = ((n as f32).sqrt().ceil() as usize).clamp(4, 24);
        let bw = (side as f32 / nb as f32).max(1.0);
        let bh = bw;
        let mut util = vec![0.0f32; nb * nb];

        let mut step = cfg.step0;
        let mut lambda = cfg.lambda0;
        let span = side as f32;
        let target_util = cfg.target_util;

        // Backtrack buffers, hoisted: two allocations per descent iteration → zero.
        // `copy_from_slice` per iteration, `mem::swap` on backtrack — byte-identical
        // trajectories to the old clone-per-iteration.
        let mut save_x = vec![0i32; n];
        let mut save_y = vec![0i32; n];

        for iter in 0..cfg.max_iters {
            gx.iter_mut().for_each(|g| *g = 0.0);
            gy.iter_mut().for_each(|g| *g = 0.0);

            // (a) HPWL subgradient: each net's bbox-extreme cells feel a unit
            // pull toward the net's interior (engine DescentCore).
            for ni in 0..nets.count() {
                let cells = nets.row(ni);
                let (mut x0, mut x1, mut y0, mut y1) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
                for &c in cells {
                    let i = c as usize;
                    x0 = x0.min(l.x[i]);
                    x1 = x1.max(l.x[i]);
                    y0 = y0.min(l.y[i]);
                    y1 = y1.max(l.y[i]);
                }
                for &c in cells {
                    let i = c as usize;
                    if l.x[i] >= x1 {
                        gx[i] += 1.0;
                    }
                    if l.x[i] <= x0 {
                        gx[i] -= 1.0;
                    }
                    if l.y[i] >= y1 {
                        gy[i] += 1.0;
                    }
                    if l.y[i] <= y0 {
                        gy[i] -= 1.0;
                    }
                }
            }

            // (b) analog-cost gradient by central finite difference. This keeps
            // the descent analog-BLIND: it never names a term, it only reads
            // `analog_cost` around the current point and follows −∇. Probe once per
            // device per axis; cheap relative to a full SA and correct for any
            // rule set the caller supplies.
            //
            // With `prices` bound this is the **primal step of the augmented
            // Lagrangian** (PLAN §3b): the probed quantity is `f(x) − λᵀc(x)`, so the
            // descent follows the priced constraint gradient as well as the objective,
            // and a budget that has been binding for several epochs pulls harder than
            // one that just became binding. That is the whole reason λ is carried.
            let probe = cfg.analog_probe.max(1);
            let base_probe = 1.0f32 / (2.0 * probe as f32);
            for i in 0..n {
                let ox = l.x[i];
                l.x[i] = ox + probe;
                let cp = analog_cost(reqs, &l, prices);
                l.x[i] = ox - probe;
                let cm = analog_cost(reqs, &l, prices);
                l.x[i] = ox;
                gx[i] += (cp - cm) * base_probe;

                let oy = l.y[i];
                l.y[i] = oy + probe;
                let cpy = analog_cost(reqs, &l, prices);
                l.y[i] = oy - probe;
                let cmy = analog_cost(reqs, &l, prices);
                l.y[i] = oy;
                gy[i] += (cpy - cmy) * base_probe;
            }

            // (c) density push: charge each device's whole area to its centre
            // bin, then push overfull bins toward the least-occupied neighbour
            // (engine DescentCore density push).
            util.iter_mut().for_each(|u| *u = 0.0);
            let bin_of = |x: i32, y: i32| -> usize {
                let bx = ((x as f32 / bw) as usize).min(nb - 1);
                let by = ((y as f32 / bh) as usize).min(nb - 1);
                by * nb + bx
            };
            for i in 0..n {
                let b = bin_of(l.x[i], l.y[i]);
                util[b] += 4.0 * l.hw[i] as f32 * l.hh[i] as f32 / (bw * bh);
            }
            for i in 0..n {
                let b = bin_of(l.x[i], l.y[i]);
                let over = util[b] - target_util;
                if over <= 0.0 {
                    continue;
                }
                let (bx, by) = (b % nb, b / nb);
                let mut best = (util[b], 0i32, 0i32);
                for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    let (tx, ty) = (bx as i32 + dx, by as i32 + dy);
                    if tx >= 0 && ty >= 0 && (tx as usize) < nb && (ty as usize) < nb {
                        let u = util[ty as usize * nb + tx as usize];
                        if u < best.0 {
                            best = (u, dx, dy);
                        }
                    }
                }
                gx[i] -= lambda * over * best.1 as f32;
                gy[i] -= lambda * over * best.2 as f32;
            }

            // (d) momentum step, scaled so the largest gradient moves ~step·span.
            let gmax = gx
                .iter()
                .chain(gy.iter())
                .fold(0.0f32, |m, g| m.max(g.abs()))
                .max(1e-6);
            let scale = step * span / gmax;

            // Legality gate (backend/TODO.md item 3). Without this the coarse
            // stage walks straight through a hard rule — a fixed DTI band, an
            // isolation gap — and leaves `dp` to dig back out, which it can only
            // do with bounded moves. Descent proposes; legality disposes.
            //
            // The gate is Φ, compared lexicographically (D10 / PLAN §3a), not the
            // bare violation count it used to be: a step that trades three shallow
            // violations for one deep one lowered the count and used to be taken.
            //
            // `analog_phi`, not `report(..).phi()`: `report` also folds device
            // overlap into `hard_violations`, and including it here would stop the
            // descent dead. Global placement *starts* from a jittered pile at the die
            // centre, so overlap area starts near maximal and the density push spends
            // several iterations trading local overlap for global spread — a
            // monotone-overlap gate rejects those steps, halves the step size on each
            // rejection, and reaches `step_min` before anything has moved. Overlap
            // becomes a Φ term in `dp`, where moves are bounded and the layout is
            // already spread.
            let before_phi = analog_phi(reqs, &l);
            save_x.copy_from_slice(&l.x);
            save_y.copy_from_slice(&l.y);
            for i in 0..n {
                vx[i] = cfg.momentum * vx[i] - scale * gx[i];
                vy[i] = cfg.momentum * vy[i] - scale * gy[i];
                l.x[i] = clamp_to_die(l.x[i] + vx[i] as i32, l.hw[i], side);
                l.y[i] = clamp_to_die(l.y[i] + vy[i] as i32, l.hh[i], side);
            }
            // Thermal rules read `temp_mc`; the field is stale the moment devices
            // move, so refresh before judging legality. Once per descent
            // iteration is the global-constraint cadence (TODO §1).
            l.refresh_temps();

            if analog_phi(reqs, &l) > before_phi {
                // Backtrack: undo the step, halve it, and kill the momentum that
                // pushed into the violation so the next iteration re-aims rather
                // than repeating the same push.
                std::mem::swap(&mut l.x, &mut save_x);
                std::mem::swap(&mut l.y, &mut save_y);
                l.refresh_temps();
                vx.iter_mut().for_each(|v| *v = 0.0);
                vy.iter_mut().for_each(|v| *v = 0.0);
                step = (step * 0.5).max(cfg.step_min);
            }

            // (e) cooling + density-weight ramp.
            step = (step * cfg.step_decay).max(cfg.step_min);
            let overflow = bin_overflow(&util, &l, target_util, bw, bh);
            if overflow > 0.05 {
                lambda = (lambda * 1.05).min(1e3);
            }

            // Stop: after min_iters, once spreading meets the overflow target.
            if iter + 1 >= cfg.min_iters && overflow <= cfg.overflow_target {
                break;
            }
        }

        l.debug_check("gp: descent output");
        debug_assert!(
            (0..n).all(|i| l.x[i] >= l.hw[i] && l.x[i] <= side.max(l.hw[i])),
            "gp: a device centre escaped the die after descent — clamp_to_die was bypassed"
        );
        // The dual step, once, at the primal optimum (PLAN §3b's alternation). The
        // residual it prices is the converged one, not the jittered start's — pricing
        // the start would ratchet ρ on a layout no one is proposing.
        prices.settle(reqs, &l);
        let rep = report(&nets, reqs, &l, prices);
        (l, rep)
    }
}

/// Post-fill density overflow: Σ over bins of `(util − target)·bin_area`,
/// normalised by total device area (engine `Bins::overflow`).
fn bin_overflow(util: &[f32], l: &Layout, target_util: f32, bw: f32, bh: f32) -> f32 {
    let total: f32 = (0..l.x.len())
        .map(|i| 4.0 * l.hw[i] as f32 * l.hh[i] as f32)
        .sum();
    if total <= 0.0 {
        return 0.0;
    }
    let over: f32 = util
        .iter()
        .map(|&u| (u - target_util).max(0.0) * bw * bh)
        .sum();
    over / total
}

/// The default drop-in. Replace at the single selection point in
/// `frontend/library`. Kept for parity with the other stages' skeletons.
#[derive(Default)]
pub struct Placeholder;

impl GlobalPlacer for Placeholder {
    fn place(
        &self,
        macros: &[Macro],
        variants: &[VariantSpace],
        reqs: &Requirements<Layout>,
        layers: &[LayerId],
        prices: &mut Prices,
        seed: u64,
    ) -> (Layout, Report) {
        Analytical::default().place(macros, variants, reqs, layers, prices, seed)
    }
}

#[cfg(test)]
mod price_tests {
    use super::*;
    use analog::Rule;

    /// A budget rule whose residual is **read out of the layout**, so a test can drive
    /// it epoch by epoch: `x[0] = 1000` ⇒ one full budget past spec, `x[0] = 0` ⇒
    /// satisfied.
    #[derive(Clone, Copy)]
    struct Budget;
    impl Rule for Budget {
        type On = Layout;
        fn cost(self, _: &Layout) -> f32 {
            1.0
        }
        fn residual(self, l: &Layout) -> f32 {
            (l.x[0] as f32 / 1000.0).clamp(0.0, 1.0)
        }
    }

    fn bench() -> (Requirements<Layout>, Layout) {
        let reqs = Requirements {
            hard: Vec::new(),
            budget: vec![Box::new(vec![Budget])],
            cost: Vec::new(),
        };
        let l = Layout {
            x: vec![1_000],
            y: vec![0],
            hw: vec![100],
            hh: vec![100],
            variant: vec![0],
            axis: vec![0],
            branch: vec![false],
            groups: vec![vec![pnr_core::DeviceId(0)]],
            orient: vec![pnr_core::Orient::default(); 1],
            power_uw: vec![0],
            temp_mc: vec![0],
        };
        (reqs, l)
    }

    /// The whole reason λ/ρ are carried instead of recomputed (D9): a budget that has
    /// been binding for several epochs must cost strictly more each epoch. With the
    /// old headroom-only weighting all three epochs below priced identically.
    #[test]
    fn rho_escalates_for_a_residual_that_does_not_shrink() {
        let (reqs, l) = bench();
        let mut prices = Prices::new();

        // No dual step yet ⇒ no price, and explicitly *not* stationary.
        prices.bind(&reqs);
        assert_eq!(prices.weight_of(0), 0.0);
        assert!(prices.drift().is_infinite(), "a fresh Prices must not read as settled");

        // `x[0] = 1000` pins the residual at one full budget, so it never shrinks.
        let mut weights = Vec::new();
        for _ in 0..4 {
            prices.settle(&reqs, &l);
            weights.push(prices.weight_of(0));
        }

        // Price rising is the ratchet; the *increments* rising is ρ escalating, which
        // is the part a recompute-from-headroom weight can never do.
        let steps: Vec<f32> = weights.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(
            weights.windows(2).all(|w| w[1] > w[0]),
            "price must rise every epoch: {weights:?}"
        );
        assert!(
            steps.windows(2).all(|s| s[1] > s[0]),
            "rho must escalate, so each epoch's step must exceed the last: {steps:?}"
        );
    }

    /// PLAN §5 terminates on `‖λ_{k+1} − λ_k‖ < δ`. That probe is only usable if it
    /// actually falls as the budget stops binding — a batch that has gone slack must
    /// stop moving its multiplier, and `drift` must say so.
    #[test]
    fn drift_decreases_as_prices_settle() {
        let (reqs, mut l) = bench();
        let mut prices = Prices::new();

        // Residual 1.0 → 0.5 → 0.1 → 0: it shrinks every epoch, so ρ holds at its floor
        // and the dual step shrinks with it.
        let mut drifts = Vec::new();
        for residual in [1_000, 500, 100, 0] {
            l.x[0] = residual;
            prices.settle(&reqs, &l);
            drifts.push(prices.drift());
        }
        assert!(
            drifts.windows(2).all(|d| d[1] < d[0]),
            "drift must fall as the residual shrinks: {drifts:?}"
        );
        assert_eq!(
            *drifts.last().unwrap(),
            0.0,
            "a satisfied budget trades nothing, so its price is stationary"
        );
    }

    /// Prices are keyed by rule kind, not by position, so a batch keeps its price when
    /// the annotator appends an in-loop feedback batch ahead of the next epoch.
    #[test]
    fn price_survives_an_appended_batch() {
        let (reqs, l) = bench();
        let mut prices = Prices::new();
        prices.settle(&reqs, &l);
        let paid = prices.weight_of(0);
        assert!(paid > 0.0);

        // Next epoch: `Requirements` re-derived with a feedback batch appended.
        let grown = Requirements {
            hard: Vec::new(),
            budget: vec![Box::new(vec![Budget]), Box::new(vec![Budget, Budget])],
            cost: Vec::new(),
        };
        prices.bind(&grown);
        assert_eq!(prices.weight_of(0), paid, "the surviving batch kept its price");
        assert_eq!(prices.weight_of(1), 0.0, "the new batch starts unpriced");
    }
}
