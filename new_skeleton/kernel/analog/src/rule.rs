//! The HOT seam: theory → algorithm, as static-dispatch data.
//!
//! See `docs/adr/0002-requirement-seam-static-trait-open-batches.md` for why
//! this is a static trait over per-kind arrays and not `Box<dyn Rule>` or a
//! `Kind` enum.

use pnr_core::ids::BranchId;
use pnr_core::{BipartiteHypergraph, UnionFind};

/// One piece of analog theory: a self-scoring value that also knows **where it
/// applies**.
///
/// A `Rule` is a small `Copy` struct carrying the targets it relates (e.g.
/// `ThermalGradient { a, b }`). The engine stores rules in **homogeneous per-kind
/// arrays** and evaluates each array through a generic, monomorphised loop, so
/// `cost`/`satisfied` **inline** — there is no `dyn` on the per-rule path.
///
/// [`On`](Rule::On) is the state a rule scores against — [`crate::Layout`] for
/// placement rules (Device↔Device / group↔group), [`crate::Routes`] for routing
/// rules (Net arity). This is why the tiers under `placement/` and `routing/`
/// are distinct: they read different geometry.
///
/// A rule is also **self-extracting** ([`Rule::extract`]): the theory — not the
/// annotator — decides where in a netlist the rule instantiates. The annotator
/// only *calls* these extracts and schedules the results.
pub trait Rule: Copy {
    /// The state this rule reads: device positions for placement, route geometry
    /// for routing.
    type On;

    /// Objective contribution at `state`, in abstract cost units (lower is
    /// better). Used when applied as [`crate::Mode::Cost`]. **Pure.**
    fn cost(self, state: &Self::On) -> f32;

    /// Whether the rule is satisfied at `state`. Used when applied as
    /// [`crate::Mode::Hard`]; `false` makes the candidate illegal. Defaults to
    /// `true` for pure-objective rules.
    #[inline]
    fn satisfied(self, _state: &Self::On) -> bool {
        true
    }

    /// Fraction of this rule's budget still unspent at `state`, clamped to
    /// `[0, 1]`: `1.0` = untouched, `0.0` = at (or past) the raw spec.
    ///
    /// Only **budget** rules — a per-net R/C cap, a spacing floor, an antenna
    /// ratio — can answer this, and they override it. The default `0.0` means
    /// "no declared slack", which makes [`RuleBatch::criticality`] report the
    /// rule as fully critical so it carries full weight in the objective. That
    /// is the conservative default: a rule that cannot describe its own headroom
    /// is never quietly discounted.
    #[inline]
    fn headroom(self, _state: &Self::On) -> f32 {
        0.0
    }

    /// How far **past** its budget this rule is at `state`, as a fraction of that
    /// budget. `0.0` = satisfied; `0.5` = 50% over; `2.0` = three times the allowance.
    ///
    /// This is the Θ tier's unit, and it is deliberately *not* [`headroom`](Rule::headroom)
    /// inverted. `headroom` is clamped to `[0, 1]` and saturates at `0.0` the moment a
    /// rule reaches its spec, so it cannot distinguish "just over" from "5× over" —
    /// which is exactly why every consumer fell back to counting violations, and why a
    /// count then let the search sit on a huge violation forever as long as it added no
    /// new one. `residual` is the unclamped other half.
    ///
    /// **Normalised by the rule's own budget**, which is what makes it summable. Raw
    /// magnitudes are not: routing overflow is a dimensionless node count and a spacing
    /// shortfall is nm, so adding them weighs a 1-node overflow the same as a 1 nm
    /// miss. `CouplingBudget::cost` already carries a hand-tuned `× 1e-3` "so it is
    /// commensurate with the other routing costs" — a fudge factor that exists only
    /// because the quantity was never normalised. Dividing by the budget removes the
    /// need for it.
    ///
    /// The default is `0.0` when satisfied and `1.0` when not — i.e. "past spec by one
    /// full budget's worth", which degrades exactly to the violation-count behaviour
    /// consumers have today. That is the honest default: it never *understates* a
    /// violation, and a rule that can quantify its own overshoot overrides it. Every
    /// [`crate::Mode::Budget`] rule should override it; a rule that cannot is arguably
    /// not a budget.
    #[inline]
    fn residual(self, state: &Self::On) -> f32 {
        if self.satisfied(state) {
            0.0
        } else {
            1.0
        }
    }

    /// Move `state` onto this rule's feasible set, exactly.
    ///
    /// Some constraints are **equalities on integers** — a mirror equation, an
    /// exact alignment — and a penalty can approach those but never land on
    /// them: annealing gets `xa + xb − 2·axis` close to zero and stops, and
    /// `satisfied` stays false forever. The only way to satisfy such a rule is to
    /// *project*, which is why every silicon-proven flow pairs its optimiser with
    /// an exact step (`backend/TODO.md` §2, §6).
    ///
    /// A rule that can restore its own feasibility overrides this; one whose
    /// feasible set is an inequality (a spacing floor, a budget) leaves it alone —
    /// for those the optimiser's gradient is already the right tool.
    ///
    /// `grid` is a quantisation hint: the projection should leave coordinates on
    /// that lattice so a later snap cannot undo it. `1` means "unconstrained".
    ///
    /// Projection is **best-effort and local**: it fixes this rule, and may break
    /// another. The caller is expected to re-check legality and roll back if the
    /// trade was bad.
    #[inline]
    fn project(self, state: &mut Self::On, grid: i32) {
        let _ = (state, grid);
    }

    /// Push the ids of the state elements this rule constrains — net ids for a
    /// routing rule, device ids for a placement rule.
    ///
    /// This is what makes **targeted repair** possible: a router that knows only
    /// *how many* rules are violated can do nothing but re-run and hope, because
    /// its negotiation loop reroutes a net only when that net sits on an
    /// over-capacity resource — which a crosstalk or antenna violation does not
    /// imply. Naming the offending nets lets the router rip up exactly those.
    ///
    /// Ids are bare `u32` so the seam stays generic over [`Rule::On`]; each tier
    /// interprets them against its own table. The default pushes nothing, which
    /// simply means "no targeted repair for this rule".
    #[inline]
    fn touches(self, out: &mut Vec<u32>) {
        let _ = out;
    }

    /// Re-express this rule's device targets in a **collapsed cell space**:
    /// `cell_of[device] = cell index` (n_cells ≤ n_devices).
    ///
    /// Group collapse (PLAN §2) merges a matched group into one placeable cell, so
    /// a rule extracted against *device* ids must be re-pointed at the cells that
    /// now carry them. A rule whose two targets land in the same cell degenerates
    /// honestly: a `Symmetry` self-pair means "centre the merged macro on the
    /// axis", a separation term reads 0 — the constraint is drawn, not deleted.
    ///
    /// Default is identity: rules that target nets (routing tier) or carry no
    /// targets never change. Every placement rule carrying a
    /// [`pnr_core::Target::Device`] must override this — the caller is expected to
    /// assert `touches() < n_cells` after retargeting to catch a missing override.
    #[inline]
    #[must_use]
    fn retarget(self, cell_of: &[u16]) -> Self {
        let _ = cell_of;
        self
    }

    /// The disjunctive commitment this rule owns, if it owns one: `(id, seed)` —
    /// `seed` is the recognised-structure starting commitment, `true` = isolate.
    ///
    /// This is the seam the branch *move* was missing (PLAN §4b): `Layout::branch`
    /// carries the per-pair Boolean and `dp` can flip a bit, but `RuleBatch` is
    /// type-erased and [`touches`](Rule::touches) names device ids, not branch ids —
    /// so without this, a flip is a guess at an index and prices nothing. A rule
    /// whose feasible set is an either-or (`DtiBand`, and eventually
    /// injector-exclusion and the implant-merge guard) overrides it; everything
    /// else is a single component and has no commitment to name.
    #[inline]
    fn branch(self) -> Option<(BranchId, bool)> {
        None
    }

    /// Safety margin held back from the raw budget, as a fraction in `[0, 1)`.
    ///
    /// The raw spec is the *terminal hard floor* ([`satisfied`](Rule::satisfied));
    /// `budget·(1 − margin)` is the target the optimiser is actually pulled to. A
    /// rule starts feeling pressure once its [`headroom`](Rule::headroom) falls
    /// below `margin`, so a converged run lands inside the margin rather than on
    /// the spec itself. `0.0` = no derating.
    #[inline]
    fn margin(self) -> f32 {
        0.0
    }

    /// Find **every instance of this rule** in the bipartite device↔net
    /// hypergraph, and return them as `Copy` rule values ready to score.
    ///
    /// This is where a rule's *recognition* lives (the theory of "a differential
    /// pair is two same-type devices sharing a tail and cross-coupled", etc.):
    /// walk `hg`, test applicability, emit an instance per match.
    ///
    /// **Groups via union-find.** When a rule relates a *set* of devices to
    /// another *set* (e.g. the two multi-finger halves of a matched pair), it
    /// `union`s each side's device indices in `uf` and emits the instance over
    /// [`pnr_core::Target::Group`]s; a device↔device relation emits
    /// [`pnr_core::Target::Device`] and needn't touch `uf`. The annotator reads
    /// the final union-find sets as the group table and as each block's members.
    ///
    /// **Pure** except for the `uf` mutations, which are monotone (only unions).
    /// The default recognises nothing — a rule that isn't structurally
    /// discoverable (or not yet migrated) simply yields no instances.
    fn extract(hg: &BipartiteHypergraph, uf: &mut UnionFind) -> Vec<Self>
    where
        Self: Sized,
    {
        let _ = (hg, uf);
        Vec::new()
    }
}

/// Type-erased view over *one kind's whole array*, for a given scored state `On`.
///
/// The **only** `dyn` in the system, and it is cold: called once per kind per
/// move, never once per rule. The inner loop it wraps stays monomorphised.
pub trait RuleBatch<On> {
    fn cost(&self, state: &On) -> f32;
    fn violations(&self, state: &On) -> u32;
    /// Σ [`Rule::residual`] over the batch — this batch's contribution to Θ.
    ///
    /// Summable because each term is normalised by its own budget. The default `0.0`
    /// suits a batch in the `hard` or `cost` arm, which has no budget to be past; the
    /// `Vec<R>` blanket impl fills it in for real.
    ///
    /// Summed rather than reported per-instance because every consumer only ever sums:
    /// Θ adds it across stages, and Φ adds hard-violation margins the same way. A
    /// per-instance vector would be a wider API for no reader.
    fn residual(&self, state: &On) -> f64 {
        let _ = state;
        0.0
    }
    /// A stable name for this batch's rule kind, for reporting (e.g. the
    /// benchmark's per-constraint-type satisfaction summary). Defaults to `"?"`;
    /// the `Vec<R>` blanket impl fills it with the concrete rule type name.
    fn kind(&self) -> &'static str {
        "?"
    }
    /// Number of individual rules in the batch (total, satisfied-or-not).
    fn count(&self) -> usize {
        0
    }
    /// Worst per-rule cost among the *violating* rules — a proxy severity metric
    /// (the `Rule` trait exposes cost, not an nm margin). `0.0` when clean.
    fn worst_cost(&self, state: &On) -> f32 {
        let _ = state;
        0.0
    }

    /// How urgently this batch needs attention at `state`, in `[0, 1]` — the
    /// weight its cost carries in the blended objective.
    ///
    /// Derived from the tightest rule's [`Rule::headroom`] against its
    /// [`Rule::margin`]: a batch with plenty of slack scores `0` and is ignored;
    /// one at or past its raw spec scores `1` and dominates. This is the analog
    /// of PathFinder's per-connection criticality `A_ij = D_ij/D_max`, recomputed
    /// every iteration rather than frozen as a user-set weight — see
    /// `backend/TODO.md` §3.
    ///
    /// Defaults to `1.0` (fully critical) so a batch that cannot describe its
    /// headroom keeps full weight.
    fn criticality(&self, state: &On) -> f32 {
        let _ = state;
        1.0
    }

    /// Ids touched by the rules in this batch that are **violated** at `state`,
    /// appended to `out` — the repair targets. See [`Rule::touches`].
    fn violating_ids(&self, state: &On, out: &mut Vec<u32>) {
        let _ = (state, out);
    }

    /// Ids touched by **every** rule in this batch, satisfied or not, appended
    /// to `out` — contrast [`violating_ids`](RuleBatch::violating_ids), which
    /// filters to the violated subset and therefore needs a state to score.
    ///
    /// This is the FD-PEX flagging seam: the oracle tier probes the cells the
    /// budget/cost rules *care about*, not just the ones currently failing —
    /// a satisfied coupling budget still wants its victim steered downhill.
    /// Takes no state for the same reason: which ids a rule constrains is
    /// static. The default pushes nothing, matching [`Rule::touches`]'s "no
    /// targeted repair for this rule".
    fn touched(&self, out: &mut Vec<u32>) {
        let _ = out;
    }

    /// Project `state` onto this batch's feasible set. See [`Rule::project`].
    fn project(&self, state: &mut On, grid: i32) {
        let _ = (state, grid);
    }

    /// Re-point every rule in the batch at a collapsed cell space. See
    /// [`Rule::retarget`]. Default no-op suits net-targeted (routing) batches.
    fn retarget(&mut self, cell_of: &[u16]) {
        let _ = cell_of;
    }

    /// Append every `(BranchId, seed)` this batch's rules own. See [`Rule::branch`].
    /// Default no-op: a kind with no disjunctions contributes nothing, so a
    /// consumer summing over `reqs.hard` sees exactly the ids that exist.
    fn branches(&self, out: &mut Vec<(BranchId, bool)>) {
        let _ = out;
    }
}

impl<R: Rule> RuleBatch<R::On> for Vec<R> {
    #[inline]
    fn cost(&self, s: &R::On) -> f32 {
        self.iter().map(|r| r.cost(s)).sum()
    }
    #[inline]
    fn violations(&self, s: &R::On) -> u32 {
        self.iter().filter(|r| !r.satisfied(s)).count() as u32
    }
    #[inline]
    fn residual(&self, s: &R::On) -> f64 {
        self.iter().map(|r| f64::from(r.residual(s))).sum()
    }
    fn kind(&self) -> &'static str {
        // Concrete rule type name, e.g. `philis::…::ThermalGradient`; the caller
        // trims the path. No per-Rule boilerplate needed.
        std::any::type_name::<R>()
    }
    fn count(&self) -> usize {
        self.len()
    }
    fn worst_cost(&self, s: &R::On) -> f32 {
        self.iter()
            .filter(|r| !r.satisfied(s))
            .map(|r| r.cost(s))
            .fold(0.0, f32::max)
    }
    #[inline]
    fn criticality(&self, s: &R::On) -> f32 {
        self.iter().map(|r| rule_criticality(*r, s)).fold(0.0, f32::max)
    }
    fn violating_ids(&self, s: &R::On, out: &mut Vec<u32>) {
        for r in self.iter().filter(|r| !r.satisfied(s)) {
            r.touches(out);
        }
    }
    fn touched(&self, out: &mut Vec<u32>) {
        for r in self.iter() {
            r.touches(out);
        }
    }
    fn project(&self, s: &mut R::On, grid: i32) {
        for r in self.iter() {
            r.project(s, grid);
        }
    }
    fn retarget(&mut self, cell_of: &[u16]) {
        for r in self.iter_mut() {
            *r = r.retarget(cell_of);
        }
    }
    fn branches(&self, out: &mut Vec<(BranchId, bool)>) {
        for r in self.iter() {
            if let Some(b) = r.branch() {
                out.push(b);
            }
        }
    }
}

/// Criticality of one rule: how far its headroom has fallen into its margin.
///
/// `headroom ≥ margin` ⇒ `0` (slack to spare, no pressure); `headroom = 0` ⇒ `1`
/// (at the raw spec). With no margin declared the rule is simply `1 − headroom`,
/// so a default rule (`headroom = 0`) stays fully weighted.
#[inline]
fn rule_criticality<R: Rule>(r: R, s: &R::On) -> f32 {
    let h = r.headroom(s).clamp(0.0, 1.0);
    let m = r.margin().clamp(0.0, 0.999);
    if m <= 0.0 {
        1.0 - h
    } else {
        ((m - h) / m).clamp(0.0, 1.0)
    }
}

/// One overshoot, expressed as a fraction of the budget it overshot — the single
/// arithmetic every [`Rule::residual`] override uses.
///
/// `excess` is the *signed* amount past the spec in the rule's own natural unit, and
/// `budget` is the spec in that same unit. Two shapes reduce to one call:
///
/// - a **cap** exceeded (parasitic length, antenna ratio, ΔT, coupling sum):
///   `over(measured − cap, cap)`
/// - a **floor** undershot (crosstalk spacing, isolation distance):
///   `over(floor − measured, floor)`
///
/// Both come out dimensionless, which is the whole point (D17): Θ sums across rules
/// whose raw units are nm, aF, milli-°C and a bare ratio, and a 1 nm miss must not
/// weigh the same as a 1-node overflow. Dividing by each rule's own spec is what makes
/// the terms addable, and it is also what removes the need for a hand-tuned scale
/// factor like `CouplingBudget::cost`'s late `× 1e-3`.
///
/// It lives here rather than being inlined six times so the zero-budget guard exists
/// once. A `0` spec is a bad rule, not a runtime condition, but it must not produce a
/// `NaN` that then poisons the whole Θ sum — one un-orderable term makes every
/// lexicographic comparison in the run meaningless.
#[inline]
#[must_use]
pub(crate) fn over(excess: f32, budget: f32) -> f32 {
    if budget <= 0.0 {
        // No declared allowance: any excess at all is a full budget's worth, which is
        // the same conservative answer `Rule::residual`'s default gives.
        return f32::from(excess > 0.0);
    }
    (excess / budget).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy)]
    struct Budgeted {
        /// Fraction of budget already spent.
        used: f32,
        margin: f32,
    }
    impl Rule for Budgeted {
        type On = ();
        fn cost(self, _: &()) -> f32 {
            self.used
        }
        fn satisfied(self, _: &()) -> bool {
            self.used <= 1.0
        }
        fn headroom(self, _: &()) -> f32 {
            (1.0 - self.used).clamp(0.0, 1.0)
        }
        fn margin(self) -> f32 {
            self.margin
        }
    }

    #[derive(Clone, Copy)]
    struct Plain;
    impl Rule for Plain {
        type On = ();
        fn cost(self, _: &()) -> f32 {
            7.0
        }
    }

    #[test]
    fn slack_rich_budget_is_ignored_tight_one_dominates() {
        let s = ();
        // 50% spent, 20% margin → headroom 0.5 > 0.2 → no pressure.
        let slack: Vec<Budgeted> = vec![Budgeted { used: 0.5, margin: 0.2 }];
        assert_eq!(slack.criticality(&s), 0.0);
        // 90% spent → headroom 0.1, half-way into the 0.2 margin → 0.5.
        let tight: Vec<Budgeted> = vec![Budgeted { used: 0.9, margin: 0.2 }];
        assert!((tight.criticality(&s) - 0.5).abs() < 1e-6);
        // At the raw spec → fully critical, and past it → still 1 (clamped).
        let at_spec: Vec<Budgeted> = vec![Budgeted { used: 1.0, margin: 0.2 }];
        assert_eq!(at_spec.criticality(&s), 1.0);
        let over: Vec<Budgeted> = vec![Budgeted { used: 1.5, margin: 0.2 }];
        assert_eq!(over.criticality(&s), 1.0);
        assert_eq!(over.violations(&s), 1);
    }

    #[test]
    fn batch_criticality_follows_the_tightest_rule() {
        let s = ();
        let mixed: Vec<Budgeted> = vec![
            Budgeted { used: 0.1, margin: 0.2 },
            Budgeted { used: 0.95, margin: 0.2 },
        ];
        assert!((mixed.criticality(&s) - 0.75).abs() < 1e-6);
    }

    /// One net drawn as a single horizontal wire `len` nm long, on layer 0.
    ///
    /// `Routes::length` sums `max(w, h)` per shape and `net_ratio_x100` sums `w·h`, so
    /// one rect drives both the length-budget and the antenna-ratio rules from the same
    /// geometry — which is what lets the two be compared at a *proportionally equal*
    /// overshoot below.
    fn one_wire(len: i32, h: i32) -> pnr_core::Routes {
        use pnr_core::geom::{LayerId, Rect, Shape};
        pnr_core::Routes {
            wires: vec![vec![Shape { layer: LayerId(0), rect: Rect { x: 0, y: 0, w: len, h } }]],
        }
    }

    fn parasitic(max_len_nm: i64) -> crate::routing::ParasiticBudget {
        crate::routing::ParasiticBudget {
            net: pnr_core::NetId(0),
            max_r_mohm: 1_000_000,
            max_c_af: 100_000_000,
            max_len_nm,
            margin_pct: 20,
        }
    }

    #[test]
    fn residual_is_zero_inside_budget_and_proportional_past_it() {
        let p = parasitic(10_000);
        // Comfortably inside: nothing past the spec, so nothing for Θ to sum.
        assert_eq!(p.residual(&one_wire(4_000, 100)), 0.0);
        // Exactly at the spec is still satisfied — the raw budget is the floor, and a
        // run that lands on it has not failed.
        assert_eq!(p.residual(&one_wire(10_000, 100)), 0.0);
        assert!(p.satisfied(&one_wire(10_000, 100)));
        // Past it, the residual is the overshoot as a fraction of the budget — *not* a
        // count. 1.5× the budget reads 0.5, and 2× reads 1.0: "one full budget over".
        assert!((p.residual(&one_wire(15_000, 100)) - 0.5).abs() < 1e-6);
        assert!((p.residual(&one_wire(20_000, 100)) - 1.0).abs() < 1e-6);
        // Three times the allowance is 2.0, so the search cannot sit on a large
        // violation while congratulating itself for adding no new one (D2).
        assert!((p.residual(&one_wire(30_000, 100)) - 2.0).abs() < 1e-6);
        // The batch sums the same numbers, and a count would have said `1` for all three.
        let batch: Vec<crate::routing::ParasiticBudget> = vec![p];
        assert!((batch.residual(&one_wire(30_000, 100)) - 2.0).abs() < 1e-6);
        assert_eq!(batch.violations(&one_wire(30_000, 100)), 1);
    }

    #[test]
    fn different_units_give_comparable_residuals_at_equal_overshoot() {
        // The property that makes Θ summable, and the one D17 says was violated.
        //
        // `ParasiticBudget` is drawn length in **nm**; `Antenna` is a metal/gate area
        // **ratio ×100**. Same wire, both rules 50% past their own spec.
        let r = one_wire(15_000, 100); // length 15_000 nm, area 1.5e6 nm²
        let p = parasitic(10_000); // 50% over a 10_000 nm cap
        // area/NOMINAL_GATE_AREA ×100 = 1.5e6/1e4 ×100 = 15_000; cap it at 10_000.
        let a = crate::routing::Antenna { net: pnr_core::NetId(0), max_ratio_x100: 10_000, margin_pct: 20 };

        let (pr, ar) = (p.residual(&r), a.residual(&r));
        assert!((pr - 0.5).abs() < 1e-6, "length residual {pr}");
        assert!((ar - 0.5).abs() < 1e-6, "antenna residual {ar}");
        assert!((pr - ar).abs() < 1e-6, "equal proportional overshoot ⇒ equal residual");

        // And the reason it has to be `residual` rather than `cost` that Θ sums: the raw
        // costs of the same two misses differ by more than an order of magnitude (225 vs
        // 5000 here), purely because one is scaled nm² and the other a fixed-point ratio.
        // Adding *those* would let the choice of unit pick the priority.
        let (pc, ac) = (p.cost(&r), a.cost(&r));
        let spread = pc.max(ac) / pc.min(ac);
        assert!(spread > 10.0, "raw costs are incommensurable: {pc} vs {ac} (×{spread})");
    }

    /// `touched` is the unfiltered half of `violating_ids`: every rule's ids,
    /// satisfied or not — the FD-PEX flagging contract.
    #[test]
    fn touched_pushes_every_rule_not_just_violating() {
        #[derive(Clone, Copy)]
        struct T {
            id: u32,
            ok: bool,
        }
        impl Rule for T {
            type On = ();
            fn cost(self, _: &()) -> f32 {
                0.0
            }
            fn satisfied(self, _: &()) -> bool {
                self.ok
            }
            fn touches(self, out: &mut Vec<u32>) {
                out.push(self.id);
            }
        }
        let batch: Vec<T> = vec![T { id: 1, ok: true }, T { id: 2, ok: false }];
        let mut all = Vec::new();
        batch.touched(&mut all);
        assert_eq!(all, vec![1, 2], "touched must not filter on satisfaction");
        let mut viol = Vec::new();
        batch.violating_ids(&(), &mut viol);
        assert_eq!(viol, vec![2], "violating_ids still filters");
    }

    #[test]
    fn rule_without_declared_headroom_keeps_full_weight() {
        // The backward-compatibility guarantee: every pre-existing rule defaults
        // to headroom 0 / margin 0, so criticality is 1 and the objective is
        // unchanged.
        let s = ();
        let plain: Vec<Plain> = vec![Plain];
        assert_eq!(plain.criticality(&s), 1.0);
        assert_eq!(plain.cost(&s), 7.0);
    }
}
