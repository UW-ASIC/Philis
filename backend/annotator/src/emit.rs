//! Recognition-driven placement constraints (Q6a, ALIGN model).
//!
//! The recogniser found the blocks; here each recognised **leaf primitive** emits
//! the geometric constraints its [`BlockKind`] implies — the mapping documented on
//! [`BlockKind`] itself (`DiffPair ⇒ symmetry + matching(Cross) + thermal + CC`,
//! `CurrentMirror ⇒ matching(Mirror) + proximity`, …). This replaces the old
//! per-rule `extract`-recognition: the `analog` rules keep `cost`/`satisfied` (the
//! hot path) but no longer re-walk the hypergraph to re-find their own patterns.
//!
//! ## Leaves, not composites
//!
//! A composite block (a telescopic-OTA core) carries its primitives as
//! [`Block::sub_blocks`]. Constraints emit from those leaves — emitting only from
//! the top-level composite would *lose* the internal diff pair's symmetry/matching.
//! So [`placement`] descends: a block with `sub_blocks` emits from each child; a
//! block without is itself the leaf.
//!
//! ## Enforcement partition
//!
//! **Hard** (legality, restricts placement): `Symmetry`, `Isolation`, `DtiBand`
//! (the disjunctive trench band — one batch for the whole tier, ids minted densely
//! by [`Dti`]; its cost copy is what prices `dp`'s branch flip).
//! **Budget** (Θ, priced by `gp::Prices`): `ThermalGradient` — it carries a spec
//! (`max_delta_mc`) *and* a `margin_pct`, it is tradeable (a run may sit at a
//! positive ΔT residual while converging), and it accumulates on a derived field;
//! PLAN §4d calls the gradient constraint a budget outright. It sits beside the
//! three routing-tier budgets (`CrosstalkExclusion`, `ParasiticBudget`,
//! `CouplingBudget`; see [`crate::extract`]) as the one placement-tier entry in
//! the middle arm. **Cost** (objective): `CommonCentroid`, `Proximity`,
//! `MatchingPair`.
//!
//! ## Group-level constraints
//!
//! Two families are properties of a whole recognised **stage**, not of a pair,
//! and are emitted once per top-level block rather than per leaf:
//!
//! * `SymmetryGroup` — every mirror pair in the stage shares **one** axis. Private
//!   per-pair axes let the pairs drift onto different mirror lines, which
//!   satisfies each rule individually and still is not a symmetric stage.
//! * `CentroidGroup` — the A-side devices' centroid against the B-side devices'
//!   centroid, area-weighted. Pairwise centroids are a weaker, different
//!   condition: each pair can be centred on its own point while the array as a
//!   whole stays lopsided, which leaves exactly the gradient the pattern exists
//!   to cancel.
//!
//! Every matched structure contributes to both — a current mirror's legs, a
//! cascode and a load are matched devices as much as a differential pair is, and
//! a ΔT across a mirror's legs shifts the copied current directly.
//!
//! `MatchingPair` is Cost here, not Hard, because Pelgrom's law splits by owner
//! (see `backend/TODO.md` §2): `σ²_u = A²/(W·L)` is fixed by the *device
//! generator* — finger and segment count — and no placement move can change a
//! device's area, so gating SA moves on it is inert. Placement owns only the
//! distance term `σ²_s = S²·r²`, which is exactly what `MatchingPair::cost`
//! scores. The area budget is checked in `cells`, where picking a bigger variant
//! can actually satisfy it.
//!
//! Scalar budgets are representative defaults (the annotator has no PDK handle —
//! `cells`/the placer refine them), matching the convention in
//! [`crate::constraints`].

use analog::placement::{
    DtiBand, Isolation, Matching, MatchingPair, Proximity, Symmetry, ThermalGradient,
};
use analog::placement::cc::CentroidGroup;
use analog::placement::symmetry::SymmetryGroup;
use analog::Requirements;
use pnr_core::ids::{AxisId, BranchId, DeviceId, Target};
use pnr_core::layout::Layout;
use pnr_core::{BipartiteHypergraph, DeviceKind};

use crate::block::{Block, BlockKind};

/// Safety margin held back from every thermal budget, percent (`backend/TODO.md`
/// §5b). The raw `max_delta_mc` stays the hard floor; the optimiser is pulled to
/// `max_delta_mc·(1 − margin)` so a converged run lands inside the margin.
///
/// ponytail: one constant for every pair. Per-net/per-tier margins (a bandgap
/// reference wants more derating than a bias leg) need the tier data the
/// annotator does not carry yet.
const THERMAL_MARGIN_PCT: u8 = 20;

/// Deep-trench banding for every emitted [`DtiBand`]: abut under 200 nm (one shared
/// trench), fully separated past 2 µm, the 1800 nm between forbidden — the same
/// representative numbers `analog::placement::dti`'s own tests use.
///
/// ponytail: one band for the whole die. The real `s_max`/`d_dti` are a per-process
/// PDK entry (trench width + well enclosure), and the PDK is the source of truth in
/// this repo — when a process disagrees, extend the PDK schema and read them there
/// rather than widening these constants.
const DTI_S_MAX_NM: i32 = 200;
const DTI_D_DTI_NM: i32 = 2_000;

/// Pelgrom `A_Vth`, µV·µm — the representative per-kind constant the old
/// `matching_pair::extract` used (real value is a PDK entry).
fn avt(kind: DeviceKind) -> i32 {
    match kind {
        DeviceKind::Pmos => 5000,
        _ => 4000,
    }
}

/// Build the placement [`Requirements`] from the recognised blocks. Descends into
/// `sub_blocks` so composites emit their primitives' constraints.
#[must_use]
pub fn placement(blocks: &[Block], hg: &BipartiteHypergraph) -> Requirements<Layout> {
    let mut r = Requirements::<Layout>::default();
    let mut dti = Dti::default();
    for (bi, b) in blocks.iter().enumerate() {
        // One mirror axis per top-level block — a differential *stage* is
        // symmetric as a whole, so its diff pair, cascodes and load must share a
        // line. A private axis per pair lets them drift onto different lines,
        // which satisfies every individual rule and is still not a symmetric
        // stage (see `analog::placement::SymmetryGroup`).
        let axis = AxisId(bi as u16);
        let mut stage = Stage::default();
        emit_block(b, hg, &mut r, axis, &mut stage, &mut dti);
        if !stage.syms.is_empty() {
            // Hard for legality, Cost for the gradient that leads there — the
            // same pairing every other hard rule gets.
            //
            // This double-registration stays `Hard`. It must not become a budget just
            // because it matches the "in both arms" shape that *locates* one in
            // `extract.rs` — symmetry is an **exact equality**, and a budget is by
            // definition tradeable at a price.
            //
            // It is also the one case where the Cost half is not merely redundant but
            // actively insufficient: PLAN §4a shows a penalty gradient on
            // `|x_i + x_j − 2·axis|` converges to *near*-feasible and then parks one
            // grid unit off, because the gradient vanishes exactly where it would have
            // to bite, and on an integer grid there is no smaller step to take. So the
            // Cost arm here is a heuristic that leads toward the manifold and provably
            // cannot land on it.
            //
            // The real fix is representational, not a reweighting: a symmetric-feasible
            // topological encoding (Balasa–Lampaert sequence pair, ASF-B*-tree, TCG-S)
            // where every decoded placement satisfies the equality *by the decoding
            // rule*, so the optimizer only ever searches the feasible manifold and
            // "approaches arbitrarily closely" never arises. PLAN ranks that its
            // highest-leverage item; it is logged as debt in `docs/API-WISH.md`.
            r.cost.push(Box::new(SymmetryGroup(stage.syms.clone())));
            r.hard.push(Box::new(SymmetryGroup(stage.syms)));
        }
        if !stage.a_side.is_empty() && !stage.b_side.is_empty() {
            // One centroid condition for the whole matched array. Pairwise
            // centroids are a weaker, different condition: each pair can be
            // centred on its own point while the array stays lopsided.
            r.cost.push(Box::new(CentroidGroup {
                a_side: stage.a_side,
                b_side: stage.b_side,
            }));
        }
    }
    if !dti.rules.is_empty() {
        // Every id was minted from one counter in emission order, so density and
        // distinctness are by construction; this assert is the tripwire for anyone
        // who later emits a `DtiBand` without going through `Dti::pair`. Two pairs
        // sharing an id would couple two independent disjunctions into one flip.
        debug_assert!(
            dti.rules.iter().enumerate().all(|(i, d)| usize::from(d.branch.0) == i),
            "BranchIds must be dense and distinct, in emission order"
        );
        // ONE batch for the whole tier, hard + cost. Hard is the classification —
        // the band is legality, and `satisfied` is the full disjunction. The cost
        // copy is what makes a branch *flip* priceable at all: `satisfied`/`residual`
        // are deliberately branch-blind (see `analog::placement::dti` — branch-aware
        // legality would report a violation while a flip is pending and Φ-monotone
        // acceptance would reject the move resolving it), so with a hard copy alone
        // the flip would change nothing any tier can see and every commitment would
        // ship on its seed. `DtiBand::cost` is the branch-aware half; it lands on
        // the PEX tier, where `dp::try_branch` prices the flip. Never the budget
        // arm: a disjunction is not tradeable at any price, and `Prices::bind`'s
        // hard∩budget assert would rightly object.
        hard_and_cost(dti.rules, &mut r);
    }
    r
}

/// The DTI disjunction accumulator: one [`DtiBand`] and one densely-minted
/// [`BranchId`] per recognised pair, across the whole `placement()` call — the
/// three-point producer contract in `analog::placement::dti`'s module note.
///
/// Allocation order = emission order, which is stable because `pattern::recognize`
/// returns matches in a deterministic priority/instance order and the annotator runs
/// once per run (D13) — so an id never renumbers between epochs and never silently
/// transfers one pair's commitment to another.
#[derive(Default)]
struct Dti {
    rules: Vec<DtiBand>,
    /// Next fresh id; `u16` because [`BranchId`] is.
    next: u16,
}

impl Dti {
    /// One pair, one fresh id, seeded from the **recognised structure** (`true` =
    /// isolate) — never from geometry, which has not been placed yet and would only
    /// re-derive the accident the branch exists to eliminate (PLAN §4b).
    fn pair(&mut self, a: DeviceId, b: DeviceId, seed_isolate: bool) {
        self.rules.push(DtiBand {
            a: Target::Device(a),
            b: Target::Device(b),
            s_max_nm: DTI_S_MAX_NM,
            d_dti_nm: DTI_D_DTI_NM,
            branch: BranchId(self.next),
            seed_isolate,
        });
        self.next += 1;
    }
}

/// What a recognised stage contributes to its group-level constraints: the mirror
/// pairs that share its axis, and the two sides of its matched array.
#[derive(Default)]
struct Stage {
    syms: Vec<Symmetry>,
    a_side: Vec<DeviceId>,
    b_side: Vec<DeviceId>,
}

impl Stage {
    /// Record a matched pair: it mirrors about the stage axis and contributes one
    /// device to each side of the array.
    fn pair(&mut self, a: DeviceId, b: DeviceId, axis: AxisId) {
        self.syms.push(Symmetry {
            a: Target::Device(a),
            b: Target::Device(b),
            axis,
        });
        self.a_side.push(a);
        self.b_side.push(b);
    }
}

/// Emit for one block: recurse into children if present, else treat as a leaf.
fn emit_block(
    b: &Block,
    hg: &BipartiteHypergraph,
    r: &mut Requirements<Layout>,
    axis: AxisId,
    stage: &mut Stage,
    dti: &mut Dti,
) {
    if !b.sub_blocks.is_empty() {
        for c in &b.sub_blocks {
            emit_block(c, hg, r, axis, stage, dti);
        }
        return;
    }
    emit_leaf(b.kind, &b.devices, hg, r, axis, stage, dti);
}

/// Kind of device `d` (defaults to Nmos if out of range — only used for `avt`).
fn kind_of(hg: &BipartiteHypergraph, d: DeviceId) -> DeviceKind {
    hg.kinds.get(d.0 as usize).copied().unwrap_or(DeviceKind::Nmos)
}

/// Register `rules` as **both** legality and objective.
///
/// A Hard rule states *where the feasible set is*; its `cost` is the continuous
/// slope that leads there. Registering it only as Hard leaves the optimiser with
/// a bare step function — it can reject a move that makes things worse but has
/// no gradient telling it which way is better, so it stalls on plateaus. Every
/// silicon-proven flow pairs the two (MAGICAL's `fSYM` penalty beside its
/// legalizer); this is that pairing.
///
/// **It is the named exception to `analog::Requirements`' "a batch belongs to
/// exactly one arm".** The two are reconcilable — the `cost` copy is a shaping
/// heuristic for a `hard` rule, not a second classification, and no tier
/// double-counts it (`|V|` counts the `hard` copy, PEX sums the `cost` one).
/// Nothing prices it either: `gp::Prices` prices `reqs.budget` alone, and
/// `Prices::bind` asserts no kind sits in both `hard` and `budget` — the one
/// cross-registration that *would* gate a batch as legality and price it as
/// tradeable at once.
fn hard_and_cost<R>(rules: Vec<R>, r: &mut Requirements<Layout>)
where
    R: analog::rule::Rule<On = Layout> + 'static,
{
    r.cost.push(Box::new(rules.clone()));
    r.hard.push(Box::new(rules));
}

/// Register `rules` as **budget** (Θ, priced) *and* objective.
///
/// The budget copy is the classification: `gp::Prices` accrues λ/ρ against it,
/// its `residual` is what `Report::budget_violations` and Θ sum, and it is ramped
/// toward hard as the run converges (D15). Used for `ThermalGradient`, the one
/// placement-tier budget.
///
/// The cost copy is kept **deliberately**, not out of habit: `ThermalGradient`'s
/// `residual`/`satisfied` read `Layout::temp_mc`, which is epoch-frozen — the
/// thermal field is re-solved at epoch boundaries, so between refreshes a trial
/// move changes the measured ΔT of nothing. The priced budget copy therefore has
/// no per-move gradient, and a budget-only registration would delete the
/// per-move isotherm pull entirely. `ThermalGradient::cost` is the
/// centre-distance proxy that *does* move with every trial, so the cost copy is
/// the slope the search feels between thermal refreshes. No tier double-counts:
/// Θ sums the budget copy, PEX sums the cost one, and `gp::Prices` never prices
/// `cost`.
fn budget_and_cost<R>(rules: Vec<R>, r: &mut Requirements<Layout>)
where
    R: analog::rule::Rule<On = Layout> + 'static,
{
    r.cost.push(Box::new(rules.clone()));
    r.budget.push(Box::new(rules));
}

/// Emit the constraint set a leaf primitive implies (the [`BlockKind`] mapping).
///
/// Every recognised pair also gets a [`DtiBand`] via `dti`, seeded from what the
/// pair *is*: a matched structure (diff pair, mirror leg, cascode, load) belongs in
/// one trench — same well, diffusion-shareable — so it seeds `share`; a bias
/// reference is the noisy/sensitive case and seeds `isolate` (a private ring). The
/// third seeding rule in `dti.rs`'s note — an injector on a `<1 kΩ` path from a pad
/// — is **not constructible today**: nothing in the netlist model names a pad or a
/// path resistance, so it is ledgered in `docs/API-WISH.md` rather than faked.
#[allow(clippy::too_many_arguments)]
fn emit_leaf(
    kind: BlockKind,
    devs: &[DeviceId],
    hg: &BipartiteHypergraph,
    r: &mut Requirements<Layout>,
    axis: AxisId,
    stage: &mut Stage,
    dti: &mut Dti,
) {
    let td = |d: DeviceId| Target::Device(d);
    match kind {
        // Diff pair: mirror-symmetric, cross-matched, thermally coupled, CC.
        BlockKind::DiffPair if devs.len() >= 2 => {
            let (a, b) = (devs[0], devs[1]);
            stage.pair(a, b, axis);
            dti.pair(a, b, false);
            r.cost.push(Box::new(vec![MatchingPair {
                a: td(a),
                b: td(b),
                max_dvth_mv10: 10,
                w_ratio: (1, 1),
                avt_uv_um: avt(kind_of(hg, a)),
                matching: Matching::Cross,
            }]));
            budget_and_cost(
                vec![ThermalGradient {
                    a: td(a),
                    b: td(b),
                    max_delta_mc: 500,
                    margin_pct: THERMAL_MARGIN_PCT,
                }],
                r,
            );
        }
        // Current mirror: each output leg matches the reference (leg 0) on L,
        // placed close to it.
        BlockKind::CurrentMirror if devs.len() >= 2 => {
            let refd = devs[0];
            let mut mp = Vec::new();
            let mut prox = Vec::new();
            let mut therm = Vec::new();
            for &out in &devs[1..] {
                mp.push(MatchingPair {
                    a: td(refd),
                    b: td(out),
                    max_dvth_mv10: 10,
                    w_ratio: (1, 1),
                    avt_uv_um: avt(kind_of(hg, refd)),
                    matching: Matching::Mirror,
                });
                prox.push(Proximity { a: td(refd), b: td(out), max_distance_nm: 5_000 });
                // A ΔT across a mirror's legs is *the* classic mismatch source —
                // it shifts the copied current directly. The legs need the same
                // isotherm treatment a diff pair gets.
                therm.push(ThermalGradient {
                    a: td(refd),
                    b: td(out),
                    max_delta_mc: 500,
                    margin_pct: THERMAL_MARGIN_PCT,
                });
                // Reference and output leg are the two sides of a matched array.
                stage.a_side.push(refd);
                stage.b_side.push(out);
                // One trench per leg pair, not per mirror: each leg's commitment
                // is its own disjunction.
                dti.pair(refd, out, false);
            }
            r.cost.push(Box::new(mp));
            r.cost.push(Box::new(prox));
            budget_and_cost(therm, r);
        }
        // Cascode / active load: matched pair, kept adjacent (cascode).
        BlockKind::Cascode if devs.len() >= 2 => {
            let (a, b) = (devs[0], devs[1]);
            stage.a_side.push(a);
            stage.b_side.push(b);
            dti.pair(a, b, false);
            budget_and_cost(
                vec![ThermalGradient {
                    a: td(a),
                    b: td(b),
                    max_delta_mc: 500,
                    margin_pct: THERMAL_MARGIN_PCT,
                }],
                r,
            );
            r.cost.push(Box::new(vec![MatchingPair {
                a: td(a),
                b: td(b),
                max_dvth_mv10: 10,
                w_ratio: (1, 1),
                avt_uv_um: avt(kind_of(hg, a)),
                matching: Matching::Mirror,
            }]));
            r.cost.push(Box::new(vec![Proximity { a: td(a), b: td(b), max_distance_nm: 5_000 }]));
        }
        BlockKind::Load if devs.len() >= 2 => {
            let (a, b) = (devs[0], devs[1]);
            stage.a_side.push(a);
            stage.b_side.push(b);
            dti.pair(a, b, false);
            budget_and_cost(
                vec![ThermalGradient {
                    a: td(a),
                    b: td(b),
                    max_delta_mc: 500,
                    margin_pct: THERMAL_MARGIN_PCT,
                }],
                r,
            );
            r.cost.push(Box::new(vec![MatchingPair {
                a: td(a),
                b: td(b),
                max_dvth_mv10: 10,
                w_ratio: (1, 1),
                avt_uv_um: avt(kind_of(hg, a)),
                matching: Matching::Mirror,
            }]));
        }
        // Bias generator: isolate the reference from the nearest aggressor when the
        // block carries a pair; a lone diode-connected reference has no partner to
        // relate, so it emits none (the cell-tier isolation directive covers it).
        BlockKind::BiasGen if devs.len() >= 2 => {
            // The noisy/sensitive case: a bias reference wants its own trench, so
            // the pair seeds `isolate` — the one recognised structure today whose
            // starting commitment is the far component.
            dti.pair(devs[0], devs[1], true);
            // Isolation is the clean case for the pairing: a continuous gap hinge,
            // so the penalty gradient walks straight to the feasible set.
            hard_and_cost(
                vec![Isolation {
                    a: td(devs[0]),
                    b: td(devs[1]),
                    min_distance_nm: 5_000,
                }],
                r,
            );
        }
        _ => {}
    }
}
