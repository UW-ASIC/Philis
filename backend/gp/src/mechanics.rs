//! Shared SA/placement mechanics, reused by `gp` (analytical global) and `dp`
//! (legalising detailed SA). Ported from `backend/engine` and
//! `backend/placement`, but **analog-blind**: connectivity comes from the
//! macros' pins, and every analog term arrives through `analog::Requirements`.
//! Nothing here hardcodes an analog weight.

use analog::Requirements;
use pnr_core::ids::{DeviceId, Target};
use pnr_core::{Layout, Macro, Report, Violation};

use crate::{Prices, VariantSpace};

// ---------------------------------------------------------------------------
// RNG — deterministic, seedable, dependency-free (SplitMix64).
// Ported verbatim from `backend/engine/src/traits.rs` so a fixed `seed`
// reproduces the same trajectory (per docs "Complexity and reproducibility").
// ---------------------------------------------------------------------------

/// Deterministic 64-bit PRNG. Identical inputs + `seed` ⇒ identical placement.
pub struct SplitMix64(pub u64);

impl SplitMix64 {
    #[inline]
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    #[inline(always)]
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, 1)`.
    #[inline(always)]
    pub fn f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform in `[0, 1)`.
    #[inline(always)]
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u32 << 24) as f32
    }

    /// Uniform integer in `[0, n)`; `n == 0` ⇒ `0`.
    #[inline(always)]
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }

    /// Uniform in `[-r, r]`.
    #[inline(always)]
    pub fn centered(&mut self, r: f32) -> f32 {
        (self.f32() * 2.0 - 1.0) * r
    }
}

// ---------------------------------------------------------------------------
// Net connectivity from macro pins (analog-blind HPWL source).
// ---------------------------------------------------------------------------

/// CSR: net → placed device indices **plus each one's pin offset**, built purely
/// from the macros' pins. No separate net table is passed — every `Pin` carries
/// its `NetId`, and pins on the same net across different macros define
/// connectivity (per the contract).
///
/// Nets with `< 2` distinct devices are dropped (they contribute no HPWL), so a
/// single-macro net or an unconnected pin is silently ignored, matching the old
/// engine's `nets with <2 cells dropped`.
///
/// **Why the offsets are here and not derived on the fly.** A variant change
/// relocates pins, and PLAN §2's whole argument for variant choice being a search
/// variable is that pin relocation — not footprint area — is what moves routability
/// and parasitics. HPWL over device *centres* is blind to it: two alternatives with
/// the same bbox and opposite pin faces score identically. So a net's terminal is
/// the pin, offset from the device centre, and the reshape move is priced on where
/// its pins land.
///
/// **What is hoisted and what is patched** (the split that makes the move
/// affordable):
///
/// - **Membership — `start`/`items`/`net` — is variant-invariant and built once.**
///   Every alternative of a cell is drawn for the same device terminals and rebound
///   to the same nets (`library::cellgen::bind_pins`), so a reshape can change pin
///   *geometry* but never which nets a cell is on. Rebuilding the CSR per trial move
///   would be pure waste, and it was measured rather than assumed: on a
///   430-device / 1290-pin / 500-net block (a MAGICAL-sized block), `from_macros` costs
///   **30.5 µs** against **0.018 µs** for [`Nets::reshape_cell`] — 1700×, on a move type
///   that fires ~5% of the ~5.7M trials such a block anneals through. A rebuild there
///   would add ~8 s per `place` call; the patch adds ~5 ms.
/// - **Geometry — `off` — is patched in place** for exactly the reshaped cell's rows,
///   and patched back on reject. `off[k]` is device `items[k]`'s pin centroid on that
///   net, relative to the macro's bbox centre, *before* the orientation transform;
///   [`Nets::pin`] applies `orient` at read time so the rotate move stays correct
///   without touching this table.
pub struct Nets {
    /// `start[i]..start[i+1]` slices `items`/`off` for net `i`.
    start: Vec<u32>,
    items: Vec<u32>,
    /// Pin offset from the device's bbox centre, parallel to `items`. Pre-orient.
    off: Vec<(i32, i32)>,
    /// The `NetId` of each retained row, so a reshape can look its offsets up in
    /// the new macro's pin list.
    net: Vec<u16>,
}

impl Nets {
    /// Group pins by `NetId` across all `macros`, dropping nets with `< 2`
    /// distinct devices. A device with several pins on one net contributes their
    /// **centroid** — a net has one terminal per device in a bbox-HPWL model, and
    /// the centroid is the cheapest unbiased choice among the fingers' contacts.
    #[must_use]
    pub fn from_macros(macros: &[Macro]) -> Self {
        let max_net = macros
            .iter()
            .flat_map(|m| m.pins.iter())
            .map(|p| p.net.0)
            .max();
        let Some(max_net) = max_net else {
            return Self { start: vec![0], items: Vec::new(), off: Vec::new(), net: Vec::new() };
        };
        // net id -> (device, pin offset) for every pin, unsorted.
        let mut per_net: Vec<Vec<(u32, i32, i32)>> = vec![Vec::new(); max_net as usize + 1];
        for (di, m) in macros.iter().enumerate() {
            let (cx, cy) = bbox_centre(m);
            for p in &m.pins {
                per_net[p.net.0 as usize].push((
                    di as u32,
                    p.at.x + p.at.w / 2 - cx,
                    p.at.y + p.at.h / 2 - cy,
                ));
            }
        }
        let mut start = vec![0u32];
        let mut items = Vec::new();
        let mut off = Vec::new();
        let mut net = Vec::new();
        for (ni, pins) in per_net.iter_mut().enumerate() {
            pins.sort_unstable_by_key(|&(d, ..)| d);
            let row0 = items.len();
            let mut k = 0usize;
            while k < pins.len() {
                let d = pins[k].0;
                let mut n = 0i64;
                let (mut sx, mut sy) = (0i64, 0i64);
                while k < pins.len() && pins[k].0 == d {
                    sx += i64::from(pins[k].1);
                    sy += i64::from(pins[k].2);
                    n += 1;
                    k += 1;
                }
                items.push(d);
                off.push(((sx / n) as i32, (sy / n) as i32));
            }
            // A net with one device (or none) contributes no HPWL: drop the row
            // rather than carry it, so `count()` reflects only retained nets.
            if items.len() - row0 >= 2 {
                start.push(items.len() as u32);
                net.push(ni as u16);
            } else {
                items.truncate(row0);
                off.truncate(row0);
            }
        }
        Self { start, items, off, net }
    }

    #[inline]
    #[must_use]
    pub fn count(&self) -> usize {
        self.start.len().saturating_sub(1)
    }

    #[inline]
    #[must_use]
    pub fn row(&self, i: usize) -> &[u32] {
        &self.items[self.start[i] as usize..self.start[i + 1] as usize]
    }

    /// Absolute position of the terminal at flat item index `k` — the device centre
    /// plus its pin offset, turned by the device's current [`pnr_core::Orient`].
    ///
    /// The turn is applied here rather than baked into `off` so the rotate move needs
    /// no bookkeeping: `orient` and the stored offset compose the same way `orient`
    /// and `hw`/`hh` do. A `Layout` whose `orient` is shorter than the device table
    /// (a hand-built one — see `dp`'s `can_rotate`) reads as un-turned.
    #[inline]
    #[must_use]
    pub fn pin(&self, k: usize, l: &Layout) -> (i32, i32) {
        let d = self.items[k] as usize;
        let (dx, dy) = self.off[k];
        let (dx, dy) = match l.orient.get(d) {
            Some(o) => o.apply(dx, dy),
            None => (dx, dy),
        };
        (l.x[d] + dx, l.y[d] + dy)
    }

    /// Flat item indices of net `i`, for [`Nets::pin`].
    #[inline]
    #[must_use]
    pub fn span(&self, i: usize) -> std::ops::Range<usize> {
        self.start[i] as usize..self.start[i + 1] as usize
    }

    /// The device the flat item `k` belongs to.
    #[inline]
    #[must_use]
    pub fn dev(&self, k: usize) -> usize {
        self.items[k] as usize
    }

    /// Repoint cell `c`'s pin offsets at `m`'s pins — the **only** part of this table
    /// a variant swap can change (see the type docs). `rows` are the nets `c` touches
    /// (`cell_nets()[c]`), so the patch costs `O(pins of c)` instead of a rebuild.
    ///
    /// A net that `m` has no pin for keeps its previous offset: the alternatives of one
    /// cell are drawn for the same terminals, so this cannot happen for a well-formed
    /// space, and silently zeroing the offset would move the terminal to the cell centre
    /// — a plausible wrong number rather than a visible fault.
    pub fn reshape_cell(&mut self, c: usize, rows: &[u32], m: &Macro) {
        let (cx, cy) = bbox_centre(m);
        for &ni in rows {
            let ni = ni as usize;
            let want = self.net[ni];
            let Some(k) = self.span(ni).find(|&k| self.items[k] as usize == c) else {
                continue;
            };
            let mut n = 0i64;
            let (mut sx, mut sy) = (0i64, 0i64);
            for p in m.pins.iter().filter(|p| p.net.0 == want) {
                sx += i64::from(p.at.x + p.at.w / 2 - cx);
                sy += i64::from(p.at.y + p.at.h / 2 - cy);
                n += 1;
            }
            debug_assert!(
                n > 0,
                "mechanics::reshape_cell: cell {c}'s new variant has no pin on net \
                 {want}, but the old one did — an alternative dropped a terminal, so \
                 the variant space is not one cell's"
            );
            if n > 0 {
                self.off[k] = ((sx / n) as i32, (sy / n) as i32);
            }
        }
    }

    /// Devices → nets they touch, so an incremental move only re-costs affected
    /// nets (`dp` uses this in its incremental HPWL delta, and to scope
    /// [`Nets::reshape_cell`]).
    #[must_use]
    pub fn cell_nets(&self, n_devices: usize) -> Vec<Vec<u32>> {
        let mut out = vec![Vec::new(); n_devices];
        for ni in 0..self.count() {
            for &c in self.row(ni) {
                out[c as usize].push(ni as u32);
            }
        }
        out
    }
}

/// A macro's bbox centre — the origin every pin offset is measured from, and the
/// point `Layout::x`/`y` places.
#[inline]
#[must_use]
fn bbox_centre(m: &Macro) -> (i32, i32) {
    (m.bbox.x + m.bbox.w / 2, m.bbox.y + m.bbox.h / 2)
}

/// Bounding-box HPWL over **pin** positions, summed across all retained nets, `nm`.
/// This is the built-in wirelength term the whole objective is anchored on.
///
/// Pins, not device centres: a reshape that keeps the footprint and moves the pins to
/// the other face is a real wirelength change, and a centre-based HPWL prices it at
/// exactly zero (PLAN §2). See [`Nets`].
#[must_use]
pub fn hpwl(nets: &Nets, l: &Layout) -> f64 {
    let mut total = 0.0f64;
    for ni in 0..nets.count() {
        let (mut x0, mut x1, mut y0, mut y1) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
        for k in nets.span(ni) {
            let (px, py) = nets.pin(k, l);
            x0 = x0.min(px);
            x1 = x1.max(px);
            y0 = y0.min(py);
            y1 = y1.max(py);
        }
        total += f64::from((x1 - x0) + (y1 - y0));
    }
    total
}

// ---------------------------------------------------------------------------
// Overlap (built-in density term + legality).
// ---------------------------------------------------------------------------

/// Rectangle overlap area of devices `a` and `b`, `nm²` (0 when disjoint).
#[inline]
#[must_use]
pub fn overlap_area(l: &Layout, a: usize, b: usize) -> f64 {
    let ox = (l.hw[a] + l.hw[b]) - (l.x[a] - l.x[b]).abs();
    let oy = (l.hh[a] + l.hh[b]) - (l.y[a] - l.y[b]).abs();
    if ox > 0 && oy > 0 {
        f64::from(ox) * f64::from(oy)
    } else {
        0.0
    }
}

/// Total pairwise overlap, `nm²`. Exact O(n²) scan — the truthful reference used
/// at stage ends and as the density term. (Engine `total_overlap`.)
#[must_use]
pub fn total_overlap(l: &Layout) -> f64 {
    let n = l.x.len();
    let mut t = 0.0;
    for a in 0..n {
        for b in a + 1..n {
            t += overlap_area(l, a, b);
        }
    }
    t
}

/// Pairwise overlap incident to `cells`, `nm²` — every pair with at least one
/// member in `cells`, counted once.
///
/// A move that repositions exactly `cells` leaves every other pair untouched, so
/// this is the only part of [`total_overlap`] that can change. Evaluating it on
/// both sides of a move yields the same difference as the full scan at
/// `O(n · cells.len())` instead of `O(n²)` — which is what makes the swap and
/// rotate moves affordable on the few-hundred-device blocks in the ALIGN and
/// MAGICAL suites.
///
/// `cells` is expected to be small (one or two devices) and free of duplicates.
#[must_use]
pub fn overlap_incident(l: &Layout, cells: &[usize]) -> f64 {
    let n = l.x.len();
    let mut t = 0.0;
    for (i, &c) in cells.iter().enumerate() {
        for b in 0..n {
            // Skip the self pair, and pairs whose other member is an earlier
            // entry of `cells` — those were already counted on that entry's pass.
            if b == c || cells[..i].contains(&b) {
                continue;
            }
            t += overlap_area(l, c, b);
        }
    }
    t
}

/// Group id per device, `None` when the device is in no multi-device group.
///
/// A singleton group names one device and therefore sanctions no abutment;
/// `initial_layout` hands those out by default, so they must not count.
#[must_use]
pub fn group_index(l: &Layout) -> Vec<Option<usize>> {
    let mut out = vec![None; l.x.len()];
    for (gi, members) in l.groups.iter().enumerate() {
        if members.len() < 2 {
            continue;
        }
        for d in members {
            if let Some(slot) = out.get_mut(d.0 as usize) {
                *slot = Some(gi);
            }
        }
    }
    out
}

/// Normalized overlap density: total overlap / total device area. Comparable
/// across block sizes; feeds the SA stop rule (`≤ 1e-4` ⇒ effectively legal).
#[must_use]
pub fn overlap_density(l: &Layout) -> f64 {
    let area: f64 = (0..l.x.len())
        .map(|i| 4.0 * f64::from(l.hw[i]) * f64::from(l.hh[i]))
        .sum();
    if area <= 0.0 {
        0.0
    } else {
        total_overlap(l) / area
    }
}

// ---------------------------------------------------------------------------
// Analog objective / legality wiring (the whole point of the contract).
// ---------------------------------------------------------------------------

/// Summed analog objective `Σ criticality(b)·cost(b)` over `reqs.cost` — the
/// **slack-blended** objective of `backend/TODO.md` §3.
///
/// Each batch's weight is derived from how far its tightest rule has eaten into
/// its safety margin, recomputed at every evaluation rather than fixed by a
/// user-set weight vector or ε-cap. A budget with slack to spare weighs ~0 and
/// stays out of the way; one at its raw spec weighs 1 and dominates. This is
/// PathFinder's `C = A·d + (1−A)·c` blend with `A` derived from budget headroom
/// instead of timing slack.
///
/// Still analog-blind: no analog weight is hardcoded, and a rule that declares no
/// headroom keeps criticality `1`, so the objective is unchanged for every rule
/// that has not opted in.
///
/// **Plus the priced budgets** — `L_ρ(x, λ) = f(x) − λᵀc(x)` (PLAN §3b), now that
/// `Requirements` has a budget arm to supply `c`:
///
/// ```text
/// f(x)  = Σ over reqs.cost    criticality(b)·cost(b)      the PEX objective
/// λᵀc(x) = Σ over reqs.budget  λ_b · residual_b(x)         the priced constraints
/// ```
///
/// The multiplier is what the recompute-from-headroom weighting could not be. D9: a
/// weight rebuilt from live headroom on every call prices a budget that has been
/// binding for fifty epochs exactly like one that just became binding, so it never
/// escalates and the optimiser keeps buying parasitic reduction with margin. λ is the
/// memory the blend was missing; ρ is the escalation ([`Prices::settle`]).
///
/// A `Prices::default()` reads `−λ = 0` for every batch, so an un-[`Prices::bind`]-ed
/// run scores the bare objective — which is what keeps a caller that has no
/// augmented-Lagrangian loop (a one-shot placement) working unchanged.
#[inline]
#[must_use]
pub fn analog_cost(reqs: &Requirements<Layout>, l: &Layout, prices: &Prices) -> f32 {
    let objective: f32 = reqs.cost.iter().map(|b| b.criticality(l) * b.cost(l)).sum();
    // `−λ_b · residual_b`. Non-negative: λ is non-positive by the sign convention
    // PLAN's `f − λᵀc` fixes, and a residual is an overshoot.
    let priced: f64 = reqs
        .budget
        .iter()
        .enumerate()
        .map(|(bi, b)| f64::from(prices.weight_of(bi)) * b.residual(l))
        .sum();
    objective + priced as f32
}

/// Φ's two elements over `reqs.hard`: `(violating batches, Σ per-batch residual)`.
///
/// Deliberately the same arithmetic [`report`] uses to fill
/// `Report::hard_violations` — one entry per violating batch carrying that batch's
/// residual — so this and `Report::phi()` cannot drift apart as reporting changes. It
/// exists separately only because the gate that consumes it runs per trial move,
/// while [`report`] costs a full HPWL plus an `O(n²)` overlap scan.
///
/// The margin is `RuleBatch::residual`, a **measured** overshoot normalised by each
/// rule's own budget, which is what D10 requires of anything both Φ and Θ sum. Its
/// default is `0`-or-`1` per rule, so a rule that cannot quantify its own overshoot
/// degrades exactly to the violation count consumers used to hardcode — never
/// understating, and upgrading the instant a rule overrides it.
///
/// [`report`] scales this into `Violation::margin`'s `i64`; the scale factor is
/// positive and common to both sides of every comparison, so Φ-ordering is identical
/// either way.
#[must_use]
pub fn analog_phi(reqs: &Requirements<Layout>, l: &Layout) -> (usize, f64) {
    let mut batches = 0usize;
    let mut margin = 0.0f64;
    for b in &reqs.hard {
        if b.violations(l) > 0 {
            batches += 1;
            margin += b.residual(l);
        }
    }
    (batches, margin)
}

/// **Θ**: the summed budget residual over `reqs.budget` — the middle tier of PLAN
/// §3b's `lex-min(V, Θ, PEX)`.
///
/// A sum and not a count, per D2: a budget missed by 1 nm and one missed by 1 µm are
/// not equally bad, and a count-based Θ lets the search sit on a large violation
/// indefinitely as long as it adds no new one. Comparable across batches because
/// `residual` is normalised by each rule's own budget.
#[inline]
#[must_use]
pub fn analog_theta(reqs: &Requirements<Layout>, l: &Layout) -> f64 {
    reqs.budget.iter().map(|b| b.residual(l)).sum()
}

/// Summed hard-rule violations `Σ batch.violations(layout)` over `reqs.hard`.
/// Zero ⇒ analog-legal.
#[inline]
#[must_use]
pub fn analog_violations(reqs: &Requirements<Layout>, l: &Layout) -> u32 {
    reqs.hard.iter().map(|b| b.violations(l)).sum()
}

/// Build the returned [`Report`]: exact achieved `cost` (built-in + analog) and
/// the concrete hard violations. `Report.hard_violations` is empty iff the
/// result is legal (analog hard rules **and** overlap both clean).
///
/// All three tiers are filled from `Requirements`' three arms, one entry per
/// violating batch. Every `margin` is a **measured** residual, never a violation
/// count: D10 requires it because both `Report::phi` and `Report::lex` *sum* margins,
/// so a count makes "one batch, three violations" read identically to "a 3 nm
/// shortfall" and lets the search park on a large violation forever as long as it adds
/// no new one.
///
/// The residual arrives as a fraction of each rule's own budget and is reported in
/// milli-budgets (see `Violation::from_residual`, the one home for that scale —
/// `library::lex_key` sums Θ across stages, so every stage must use it). The batch is
/// still the only `dyn` seam, so the *rule identity* inside a violating batch is
/// unrecoverable and the entry is named `"analog {tier} batch {i}"` — that is a
/// reporting-granularity limit, not a measurement one, and it does not affect any
/// tier's arithmetic.
#[must_use]
pub fn report(
    nets: &Nets,
    reqs: &Requirements<Layout>,
    l: &Layout,
    prices: &Prices,
) -> Report {
    let mut hard_violations = Vec::new();
    // Analog hard rules, batch-addressed. `analog_phi` must agree with this list
    // exactly — it is the per-move form of the same measurement.
    for (bi, b) in reqs.hard.iter().enumerate() {
        if b.violations(l) > 0 {
            hard_violations
                .push(Violation::from_residual(format!("analog hard batch {bi} ({:?})", b.kind()), b.residual(l)));
        }
    }
    // Θ: the budget arm, priced by `Prices` and driven to zero by the search rather
    // than gating it. A budget with a positive residual is not illegal geometry — it
    // is a constraint still being paid for, which is the distinction the middle tier
    // exists to express.
    let budget_violations = reqs
        .budget
        .iter()
        .enumerate()
        .filter_map(|(bi, b)| {
            let residual = b.residual(l);
            (residual > 0.0)
                .then(|| Violation::from_residual(format!("analog budget batch {bi}"), residual))
        })
        .collect();
    // Own overlap legality: any residual device overlap is a hard fail. This used
    // to exempt same-group pairs as "intentional abutment", but `cellgen` emits
    // one self-contained macro per device — nothing can share diffusion, so no
    // overlap is intentional. See `dp::legalize`'s module docs.
    let ov = total_overlap(l);
    if ov > 0.5 {
        hard_violations.push(Violation {
            rule: "device overlap".into(),
            margin: ov as i64,
        });
    }
    // Cost is the honest achieved objective at overlap_weight = 0 (overlap is a
    // legality concern, not a cost the caller compares across runs); HPWL +
    // analog cost is what a caller ranks solutions by.
    let cost = hpwl(nets, l) as f32 + analog_cost(reqs, l, prices);
    Report { hard_violations, budget_violations, cost }
}

// ---------------------------------------------------------------------------
// Canvas + initial Layout (docs "Initial canvas" + "Stage 0: initialization").
// ---------------------------------------------------------------------------

/// Half-extents of one drawn macro, `nm` — the right-hand side of the
/// [`Layout::variant`] invariant.
///
/// One definition, because two stages write `variant[i]` and must agree on what
/// `hw[i]`/`hh[i]` then have to be: `gp` when it seeds the starting variant, `dp`
/// when a reshape move swaps one. A second copy of `bbox.w / 2` is how that
/// invariant rots.
#[inline]
#[must_use]
pub fn variant_extents(m: &Macro) -> (i32, i32) {
    (m.bbox.w / 2, m.bbox.h / 2)
}

/// Per-device half-extents from macro bboxes, `nm`.
#[must_use]
pub fn half_extents(macros: &[Macro]) -> (Vec<i32>, Vec<i32>) {
    let mut hw = Vec::with_capacity(macros.len());
    let mut hh = Vec::with_capacity(macros.len());
    for m in macros {
        let (w, h) = variant_extents(m);
        hw.push(w);
        hh.push(h);
    }
    (hw, hh)
}

/// The macro each cell is **currently drawn as**:
/// `variants[i].alternatives[variant[i]]`, falling back to `macros[i]` for a cell
/// with no enumerated alternatives (or an out-of-range index, which an all-zero
/// `variant` against an empty `VariantSpace` is by construction — see
/// [`Layout::variant`]).
///
/// Everything a placement stage measures has to come from this and not from
/// `macros` directly: not just the extents but the **pins**, because a variant
/// change relocates them and that is the entire reason variant choice is a search
/// variable (PLAN §2). Feeding `Nets` the unchosen macro's pins would optimise HPWL
/// against connectivity the layout does not have.
///
/// Clones, because the extent/canvas/`Nets` helpers all want a plain `&[Macro]`.
/// Once per `place` call on tens-to-hundreds of cells; `dp`'s per-move reshape never
/// comes through here, it reads one alternative's bbox directly.
#[must_use]
pub fn choose_variants(
    macros: &[Macro],
    variants: &[VariantSpace],
    variant: &[u16],
) -> Vec<Macro> {
    macros
        .iter()
        .enumerate()
        .map(|(i, m)| {
            variants
                .get(i)
                .and_then(|v| v.alternatives.get(variant[i] as usize))
                .unwrap_or(m)
                .clone()
        })
        .collect()
}

/// Initial square canvas side (`nm`) per docs: area-derived side from summed
/// inflated footprints and clamped utilization, floored by the largest single
/// footprint, rounded up to `grid`.
#[must_use]
pub fn canvas_side(hw: &[i32], hh: &[i32], utilization: f32, grid: i32) -> i32 {
    let cell_margin = 0.0f64; // margin lives in the inflated bbox already.
    let total: f64 = hw
        .iter()
        .zip(hh)
        .map(|(&w, &h)| {
            (f64::from(2 * w) + cell_margin) * (f64::from(2 * h) + cell_margin)
        })
        .sum();
    let u = f64::from(utilization.clamp(0.05, 0.95));
    let area_side = if total > 0.0 {
        (total / u).sqrt().ceil() as i32
    } else {
        0
    };
    let largest = hw
        .iter()
        .zip(hh)
        .map(|(&w, &h)| (2 * w).max(2 * h))
        .max()
        .unwrap_or(1000);
    let side = area_side.max(largest).max(1);
    let g = grid.max(1);
    (side + g - 1) / g * g
}

/// Deterministic initial [`Layout`]: every device jittered within `0.15·min(die)`
/// of the die centre (docs "Stage 0"). `axis` is initialised at the die centre
/// and `groups` to single-device groups — safe defaults that let an analog rule
/// address an `AxisId`/`GroupId` without out-of-range panics when the caller has
/// not supplied real symmetry/group tables (see `place`'s TODO).
///
/// `macros` must already be the **chosen** variants ([`choose_variants`]) and
/// `variant` the indices they were chosen by: `hw`/`hh` are taken from these
/// bboxes, so passing the two out of step is the one way to break the
/// [`Layout::variant`] invariant at birth.
#[must_use]
pub fn initial_layout(
    macros: &[Macro],
    variant: Vec<u16>,
    side: i32,
    rng: &mut SplitMix64,
) -> Layout {
    let n = macros.len();
    let (hw, hh) = half_extents(macros);
    let (cx, cy) = (side / 2, side / 2);
    let jitter = (side as f32) * 0.15;
    let mut x = Vec::with_capacity(n);
    let mut y = Vec::with_capacity(n);
    for _ in 0..n {
        x.push(cx + rng.centered(jitter) as i32);
        y.push(cy + rng.centered(jitter) as i32);
    }
    // See module docs / place() TODO: real axis + group tables come from the
    // annotator, which the current `place()` signature does not carry. Sizing
    // both to `n` with sane defaults keeps any `AxisId(<n)` / `GroupId(<n)` a
    // rule references in-range.
    let axis = vec![cx; n];
    let groups: Vec<Vec<DeviceId>> =
        (0..n).map(|i| vec![DeviceId(i as u16)]).collect();
    // Power is unknown until a caller supplies operating points; zero makes every
    // device a pure heat sensor, so the thermal solver reports a uniform die.
    debug_assert_eq!(variant.len(), n, "initial_layout: variant table is not device-length");
    Layout {
        x,
        y,
        hw,
        hh,
        variant,
        axis,
        // Every disjunctive constraint starts committed to its `false` branch (for
        // `DtiBand`, "share a trench"). A valid starting commitment per
        // `Layout::branch`, and `gp` never flips one — the branch move is `dp`'s, and
        // is not implemented yet.
        //
        // Sized to the device count for the same reason `axis` is: a rule can address
        // any `BranchId` a block handed out, and the annotator's real table has not
        // reached this signature.
        branch: vec![false; n],
        groups,
        orient: vec![pnr_core::geom::Orient::default(); n],
        power_uw: vec![0; n],
        temp_mc: vec![0; n],
    }
}

/// Clamp a device centre so its (inflated) footprint stays inside the `[0,side]`
/// die, per docs "proposed cell centres are clamped to the die using base
/// half-extents".
#[inline]
#[must_use]
pub fn clamp_to_die(c: i32, half: i32, side: i32) -> i32 {
    let lo = half;
    let hi = (side - half).max(half);
    c.clamp(lo, hi)
}

/// Bounding box over inflated device footprints as `(xmin, ymin, xmax, ymax)`.
#[must_use]
pub fn footprint_bbox(l: &Layout) -> (i32, i32, i32, i32) {
    let mut b = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for i in 0..l.x.len() {
        b.0 = b.0.min(l.x[i] - l.hw[i]);
        b.1 = b.1.min(l.y[i] - l.hh[i]);
        b.2 = b.2.max(l.x[i] + l.hw[i]);
        b.3 = b.3.max(l.y[i] + l.hh[i]);
    }
    b
}

/// Snap `v` to the nearest multiple of `grid` (grid ≥ 1).
#[inline]
#[must_use]
pub fn snap(v: i32, grid: i32) -> i32 {
    let g = grid.max(1);
    ((v as f32 / g as f32).round() as i32) * g
}

/// Resolve a `Target` centre — mirrors `Layout::centre` but usable without a
/// borrow of the whole layout when only centres are needed.
#[inline]
#[must_use]
pub fn target_centre(l: &Layout, t: Target) -> (i32, i32) {
    l.centre(t)
}

#[cfg(test)]
mod overlap_incident_tests {
    use super::*;

    fn layout(cells: &[(i32, i32, i32, i32)]) -> Layout {
        Layout {
            x: cells.iter().map(|c| c.0).collect(),
            y: cells.iter().map(|c| c.1).collect(),
            hw: cells.iter().map(|c| c.2).collect(),
            hh: cells.iter().map(|c| c.3).collect(),
            variant: vec![0; cells.len()],
            axis: vec![],
            branch: vec![false; cells.len()],
            groups: vec![],
            orient: cells.iter().map(|_| pnr_core::Orient::default()).collect(),
            power_uw: vec![0; cells.len()],
            temp_mc: vec![0; cells.len()],
        }
    }

    /// The whole point of `overlap_incident`: on a move of exactly `moved`, its
    /// before/after difference must equal `total_overlap`'s. If that ever stops
    /// holding, the annealer silently anneals against the wrong cost.
    #[test]
    fn incident_difference_matches_full_scan_difference() {
        let cells = [
            (0, 0, 100, 100),
            (150, 0, 100, 100),
            (60, 60, 100, 100),
            (900, 900, 50, 50),
            (120, 30, 80, 40),
        ];
        // Every single- and two-device move over this cluster.
        for c in 0..cells.len() {
            for o in 0..cells.len() {
                let moved: Vec<usize> = if c == o { vec![c] } else { vec![c, o] };
                let mut l = layout(&cells);
                let full_before = total_overlap(&l);
                let inc_before = overlap_incident(&l, &moved);

                // Displace the moved devices; everything else stays put.
                for &m in &moved {
                    l.x[m] += 37;
                    l.y[m] -= 11;
                }
                let full_delta = total_overlap(&l) - full_before;
                let inc_delta = overlap_incident(&l, &moved) - inc_before;
                assert!(
                    (full_delta - inc_delta).abs() < 1e-6,
                    "moved={moved:?}: full {full_delta} vs incident {inc_delta}"
                );
            }
        }
    }

    /// A pair with both members in `moved` must be counted once, not twice.
    #[test]
    fn pair_inside_moved_is_not_double_counted() {
        let l = layout(&[(0, 0, 100, 100), (100, 0, 100, 100)]);
        assert_eq!(overlap_incident(&l, &[0, 1]), total_overlap(&l));
        assert!(overlap_incident(&l, &[0, 1]) > 0.0);
    }
}
