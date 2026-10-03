//! The hot seam: theory → algorithm as static-dispatch data.

use pnr_core::ids::BranchId;
use pnr_core::{BipartiteHypergraph, UnionFind};

/// What a router's repair does with a violated batch: the typed dispatch key,
/// so no consumer matches on [`RuleBatch::kind`] strings (AT-31).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepairKind {
    /// No repair.
    None,
    /// Rip up the named nets and reroute them plainly (the default).
    Reroute,
    /// A matched pair: one side follows the other's mirror image (`Differential`).
    Mirror,
    /// A shared node split into equal branches (`CommonNodes`).
    Balance,
    /// Victim and aggressor routed apart (`CrosstalkExclusion`, `CouplingBudget`).
    KeepAway,
    /// Jumpers, gate lift, diodes (`Antenna`).
    Antenna,
    /// Reference tracks drawn beside the victim (`Shield`).
    Shield,
    /// Electromigration: width is not a search resource yet, so no reroute
    /// trial can fix it (AT-24); repair spends nothing on it.
    Em,
    /// Series R priced by the current a net carries (`IrDrop`).
    Ir,
    /// A parasitic or performance budget (`ParasiticBudget`, `PerformanceBudget`).
    Budget,
}

/// One piece of analog theory: a small `Copy` value that scores itself against
/// [`Rule::On`] (`Layout` for placement, `Routes` for routing) and knows where
/// it applies ([`Rule::extract`]). Every method is pure except `project`.
pub trait Rule: Copy {
    type On;

    /// Objective contribution (lower is better).
    fn cost(self, state: &Self::On) -> f32;

    /// Legality. Default: always satisfied (pure objective).
    #[inline]
    fn satisfied(self, _state: &Self::On) -> bool {
        true
    }

    /// Unspent fraction of the budget in `[0, 1]`. Default `0.0` = fully
    /// critical, so a rule that cannot describe its slack keeps full weight.
    #[inline]
    fn headroom(self, _state: &Self::On) -> f32 {
        0.0
    }

    /// Overshoot past the spec as a fraction of the spec (`0.5` = 50% over),
    /// unclamped. Normalising by the rule's own spec is what makes Θ summable
    /// across units. Default: `0.0` satisfied, `1.0` violated.
    #[inline]
    fn residual(self, state: &Self::On) -> f32 {
        if self.satisfied(state) {
            0.0
        } else {
            1.0
        }
    }

    /// Spent fraction of the budget, unclamped (`1.0` = at the spec). `None`:
    /// no budget to spend (an exact equality) or not applicable. Reporting only.
    #[inline]
    fn usage(self, _state: &Self::On) -> Option<f32> {
        None
    }

    /// Whether the rule has anything to check on `state` (a thermal rule on an
    /// unpowered die does not). An inapplicable rule is satisfied vacuously.
    #[inline]
    fn applicable(self, _state: &Self::On) -> bool {
        true
    }

    /// Whether the inputs this rule reads exist (power from an operating point,
    /// a routed net). `false` = **unknown**: search treats it as satisfied (it
    /// cannot act on it), but a report must not certify it. Distinct from
    /// [`Rule::applicable`], which is a justified "nothing to check".
    #[inline]
    fn known(self, _state: &Self::On) -> bool {
        true
    }

    /// Move `state` exactly onto this rule's feasible set, on the `grid`
    /// lattice. For integer equalities a penalty can never close. Best-effort
    /// and local; the caller re-checks legality.
    #[inline]
    fn project(self, state: &mut Self::On, grid: i32) {
        let _ = (state, grid);
    }

    /// Push the ids this rule constrains (device ids for placement, net ids
    /// for routing) — the targets for repair and FD-PEX probing.
    #[inline]
    fn touches(self, out: &mut Vec<u32>) {
        let _ = out;
    }

    /// Re-point device targets through `cell_of[device] = cell` (group
    /// collapse). Every placement rule with a `Target::Device` must override.
    #[inline]
    #[must_use]
    fn retarget(self, cell_of: &[u16]) -> Self {
        let _ = cell_of;
        self
    }

    /// The disjunctive commitment this rule owns: `(id, seed)`, seed `true` =
    /// isolate. Only either-or rules (`DtiBand`) override.
    #[inline]
    fn branch(self) -> Option<(BranchId, bool)> {
        None
    }

    /// `(a, b, axis)` when this rule mirrors two devices about a shared axis:
    /// what a move must preserve to stay feasible without projection.
    #[inline]
    fn mirror_pair(self) -> Option<(u32, u32, u16)> {
        None
    }

    /// `(a, b)` cells that must be drawn alike (same orientation, same variant
    /// when their variant spaces agree): what dp's locks preserve. Default none.
    #[inline]
    fn matched_pair(self) -> Option<(u32, u32)> {
        None
    }

    /// `(victim, aggressor)` nets this rule wants routed apart: what a router
    /// can price as a keep-away field while searching, not only after.
    #[inline]
    fn keepaway(self) -> Option<(u32, u32)> {
        None
    }

    /// `(victim, reference)` when this rule asks for `victim` to be shielded by
    /// `reference` metal — what a router must generate, not only check.
    #[inline]
    fn shield(self) -> Option<(u32, u32)> {
        None
    }

    /// Safety margin in `[0, 1)`: pressure starts once `headroom < margin`.
    #[inline]
    fn margin(self) -> f32 {
        0.0
    }

    /// How a router repairs a violated instance; see [`RuleBatch::repair_kind`].
    const REPAIR: RepairKind = RepairKind::Reroute;

    /// Every instance of this rule in the netlist. Group↔group rules `union`
    /// each side in `uf` and emit `Target::Group`s. Default: none.
    fn extract(hg: &BipartiteHypergraph, uf: &mut UnionFind) -> Vec<Self>
    where
        Self: Sized,
    {
        let _ = (hg, uf);
        Vec::new()
    }
}

/// Type-erased view of one kind's whole array. Cold: called once per kind.
/// `Send + Sync`: a whole solve (its `Requirements` included) may run on a
/// worker thread (the library's multi-start).
pub trait RuleBatch<On>: Send + Sync {
    fn cost(&self, state: &On) -> f32;
    fn violations(&self, state: &On) -> u32;
    /// Σ [`Rule::residual`] — this batch's Θ contribution. Default: one full
    /// budget per violation, so a batch that cannot measure its overshoot still
    /// reads as failed (Graeb ch.1: fail is never pass).
    fn residual(&self, state: &On) -> f64 {
        f64::from(self.violations(state))
    }
    /// Stable kind name (price matching, reporting). Repair dispatches on
    /// [`RuleBatch::repair_kind`], never on this.
    fn kind(&self) -> &'static str {
        "?"
    }
    /// How a router repairs this batch's violations. Default `Reroute`.
    fn repair_kind(&self) -> RepairKind {
        RepairKind::Reroute
    }
    /// Number of rules in the batch.
    fn count(&self) -> usize {
        0
    }
    /// Worst cost among violating rules; `0.0` when clean.
    fn worst_cost(&self, state: &On) -> f32 {
        let _ = state;
        0.0
    }
    /// Weight of this batch's cost in `[0, 1]`, from the tightest rule's
    /// headroom against its margin. Default fully critical.
    fn criticality(&self, state: &On) -> f32 {
        let _ = state;
        1.0
    }
    /// Largest [`Rule::usage`] in the batch; `None` when no rule reports one.
    fn worst_usage(&self, state: &On) -> Option<f32> {
        let _ = state;
        None
    }
    /// Rules with nothing to check on `state` (see [`Rule::applicable`]).
    fn inapplicable(&self, state: &On) -> u32 {
        let _ = state;
        0
    }
    /// Rules whose inputs are missing (see [`Rule::known`]).
    fn unknown(&self, state: &On) -> u32 {
        let _ = state;
        0
    }
    /// Ids touched by the **violated** rules — the repair targets.
    fn violating_ids(&self, state: &On, out: &mut Vec<u32>) {
        let _ = (state, out);
    }
    /// [`RuleBatch::violating_ids`], each with the residual of the violated
    /// rule touching it ([`Rule::residual`], floored like
    /// [`RuleBatch::residual`]). Default `NaN`: the batch does not split its
    /// residual per rule.
    fn violating_residuals(&self, state: &On, out: &mut Vec<(u32, f32)>) {
        let mut ids = Vec::new();
        self.violating_ids(state, &mut ids);
        out.extend(ids.into_iter().map(|i| (i, f32::NAN)));
    }
    /// Ids touched by every rule, satisfied or not (see [`Rule::touches`]).
    fn touched(&self, out: &mut Vec<u32>) {
        let _ = out;
    }
    fn project(&self, state: &mut On, grid: i32) {
        let _ = (state, grid);
    }
    fn retarget(&mut self, cell_of: &[u16]) {
        let _ = cell_of;
    }
    /// Append every `(BranchId, seed)` the batch owns.
    fn branches(&self, out: &mut Vec<(BranchId, bool)>) {
        let _ = out;
    }
    /// Append every `(a, b, axis)` mirror pair (see [`Rule::mirror_pair`]).
    fn mirror_pairs(&self, out: &mut Vec<(u32, u32, u16)>) {
        let _ = out;
    }
    /// Append every matched pair (see [`Rule::matched_pair`]).
    fn matched_pairs(&self, out: &mut Vec<(u32, u32)>) {
        let _ = out;
    }
    /// Append every keep-away `(victim, aggressor)` (see [`Rule::keepaway`]).
    fn keepaway_pairs(&self, out: &mut Vec<(u32, u32)>) {
        let _ = out;
    }
    /// Append every shield request `(victim, reference)` (see [`Rule::shield`]).
    fn shield_pairs(&self, out: &mut Vec<(u32, u32)>) {
        let _ = out;
    }
    /// `(device_a, device_b, mV)`: the 1σ systematic allowance each matched pair has left after
    /// placement's own spend — what a routing rule may use. Default none.
    fn offset_allowances(&self, state: &On, out: &mut Vec<(u32, u32, f32)>) {
        let _ = (state, out);
    }
}

impl<R: Rule + Send + Sync> RuleBatch<R::On> for Vec<R> {
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
        self.iter().map(|r| rule_residual(*r, s)).sum()
    }
    fn kind(&self) -> &'static str {
        std::any::type_name::<R>()
    }
    fn repair_kind(&self) -> RepairKind {
        R::REPAIR
    }
    fn count(&self) -> usize {
        self.len()
    }
    fn worst_cost(&self, s: &R::On) -> f32 {
        self.iter().filter(|r| !r.satisfied(s)).map(|r| r.cost(s)).fold(0.0, f32::max)
    }
    #[inline]
    fn criticality(&self, s: &R::On) -> f32 {
        self.iter().map(|r| rule_criticality(*r, s)).fold(0.0, f32::max)
    }
    fn worst_usage(&self, s: &R::On) -> Option<f32> {
        self.iter().filter_map(|r| r.usage(s)).reduce(f32::max)
    }
    fn inapplicable(&self, s: &R::On) -> u32 {
        self.iter().filter(|r| !r.applicable(s)).count() as u32
    }
    fn unknown(&self, s: &R::On) -> u32 {
        self.iter().filter(|r| !r.known(s)).count() as u32
    }
    fn violating_ids(&self, s: &R::On, out: &mut Vec<u32>) {
        for r in self.iter().filter(|r| !r.satisfied(s)) {
            r.touches(out);
        }
    }
    fn violating_residuals(&self, s: &R::On, out: &mut Vec<(u32, f32)>) {
        let mut ids = Vec::new();
        for r in self.iter().filter(|r| !r.satisfied(s)) {
            r.touches(&mut ids);
            let x = rule_residual(*r, s) as f32;
            out.extend(ids.drain(..).map(|i| (i, x)));
        }
    }
    fn touched(&self, out: &mut Vec<u32>) {
        for r in self {
            r.touches(out);
        }
    }
    fn project(&self, s: &mut R::On, grid: i32) {
        for r in self {
            r.project(s, grid);
        }
    }
    fn retarget(&mut self, cell_of: &[u16]) {
        for r in self.iter_mut() {
            *r = r.retarget(cell_of);
        }
    }
    fn branches(&self, out: &mut Vec<(BranchId, bool)>) {
        out.extend(self.iter().filter_map(|r| r.branch()));
    }
    fn mirror_pairs(&self, out: &mut Vec<(u32, u32, u16)>) {
        out.extend(self.iter().filter_map(|r| r.mirror_pair()));
    }
    fn matched_pairs(&self, out: &mut Vec<(u32, u32)>) {
        out.extend(self.iter().filter_map(|r| r.matched_pair()));
    }
    fn keepaway_pairs(&self, out: &mut Vec<(u32, u32)>) {
        out.extend(self.iter().filter_map(|r| r.keepaway()));
    }
    fn shield_pairs(&self, out: &mut Vec<(u32, u32)>) {
        out.extend(self.iter().filter_map(|r| r.shield()));
    }
}

/// `0` with headroom ≥ margin, `1` at the raw spec; `1 − headroom` with no margin.
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

/// [`Rule::residual`], floored at one milli-budget for a violated rule: a
/// boolean fail whose measure rounds to `0` must still cost Θ, or the search
/// reads it as a pass (Graeb 2007 ch.1, four outcomes; EVD-07).
#[inline]
fn rule_residual<R: Rule>(r: R, s: &R::On) -> f64 {
    let x = f64::from(r.residual(s));
    if x > 0.0 || r.satisfied(s) {
        x
    } else {
        FAIL_FLOOR
    }
}

/// Smallest Θ a violated rule contributes: one milli-budget, the unit
/// `Violation::from_residual` counts in.
const FAIL_FLOOR: f64 = 1e-3;

/// `excess` past the spec as a fraction of `budget` (both in the rule's unit):
/// a cap is `over(measured − cap, cap)`, a floor `over(floor − measured, floor)`.
/// A non-positive budget yields `0`/`1` rather than a Θ-poisoning NaN.
#[inline]
#[must_use]
pub(crate) fn over(excess: f32, budget: f32) -> f32 {
    if budget <= 0.0 {
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
            ..Default::default()
        }
    }

    fn parasitic(max_len_nm: i64) -> crate::routing::ParasiticBudget {
        crate::routing::ParasiticBudget {
            net: pnr_core::NetId(0),
            max_len_nm,
            max_c_af: 0,
            margin_pct: 20,
            stack: None,
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
        // area/gate ×100 = 1.5e6/1e4 ×100 = 15_000; cap it at 10_000.
        let a = crate::routing::Antenna { net: pnr_core::NetId(0), max_ratio_x100: 10_000, gate_area_nm2: 10_000, margin_pct: 20, stack: None };

        let (pr, ar) = (p.residual(&r), a.residual(&r));
        assert!((pr - 0.5).abs() < 1e-6, "length residual {pr}");
        assert!((ar - 0.5).abs() < 1e-6, "antenna residual {ar}");
        assert!((pr - ar).abs() < 1e-6, "equal proportional overshoot ⇒ equal residual");

        // And the reason it has to be `residual` rather than `cost` that Θ sums: the raw
        // costs of the same two misses differ by more than an order of magnitude (2.25 vs
        // 5000 here): `ParasiticBudget`'s cost is its unit-free `(spent/budget)²`, but
        // `Antenna`'s is still in its fixed-point ×100 ratio. Adding *those* would let the
        // choice of unit pick the priority.
        let (pc, ac) = (p.cost(&r), a.cost(&r));
        let spread = pc.max(ac) / pc.min(ac);
        assert!(spread > 10.0, "raw costs are incommensurable: {pc} vs {ac} (×{spread})");
    }

    /// `touched` pushes every rule's ids, satisfied or not (FD-PEX flagging).
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
        assert_eq!(viol, vec![2], "violating_ids filters");
        let mut res = Vec::new();
        batch.violating_residuals(&(), &mut res);
        assert_eq!(res, vec![(2, 1.0)], "per violated rule, its own residual (default 1.0)");
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

    /// Repair dispatches on this, so each routing rule's kind is pinned: a
    /// rule that forgets its override falls back to a blind `Reroute`.
    #[test]
    fn every_routing_rule_declares_its_repair() {
        use crate::routing::*;
        use RepairKind as K;
        assert_eq!(Vec::<Differential>::new().repair_kind(), K::Mirror);
        let stack: &'static Stack = Box::leak(Box::default());
        assert_eq!(CommonNodes { nodes: Vec::new(), stack }.repair_kind(), K::Balance);
        assert_eq!(Vec::<CrosstalkExclusion>::new().repair_kind(), K::KeepAway);
        assert_eq!(Vec::<CouplingBudget>::new().repair_kind(), K::KeepAway);
        assert_eq!(Vec::<Antenna>::new().repair_kind(), K::Antenna);
        assert_eq!(Vec::<Shield>::new().repair_kind(), K::Shield);
        assert_eq!(Vec::<Electromigration>::new().repair_kind(), K::Em);
        assert_eq!(Vec::<IrDrop>::new().repair_kind(), K::Ir);
        assert_eq!(Vec::<ParasiticBudget>::new().repair_kind(), K::Budget);
        let perf = PerformanceBudget { metric: String::new(), nets: Vec::new(), weights: Vec::new(), af_per_nm: 0.0, limit: 1.0 };
        assert_eq!(perf.repair_kind(), K::Budget);
        // A rule that declares nothing is rerouted plainly.
        assert_eq!(vec![Plain].repair_kind(), K::Reroute);
    }

    /// A failed check whose measure says "0 over" (at the edge, or a boolean
    /// with no scale) must still cost Θ; so must a batch with no residual.
    #[test]
    fn a_failed_rule_never_reads_as_zero_theta() {
        #[derive(Clone, Copy)]
        struct EdgeFail;
        impl Rule for EdgeFail {
            type On = ();
            fn cost(self, _: &()) -> f32 {
                0.0
            }
            fn satisfied(self, _: &()) -> bool {
                false
            }
            fn residual(self, _: &()) -> f32 {
                0.0
            }
        }
        assert!(vec![EdgeFail].residual(&()) > 0.0);
        assert_eq!(vec![Plain].residual(&()), 0.0, "a pass stays free");

        struct Unmeasured;
        impl RuleBatch<()> for Unmeasured {
            fn cost(&self, _: &()) -> f32 {
                0.0
            }
            fn violations(&self, _: &()) -> u32 {
                2
            }
        }
        assert_eq!(Unmeasured.residual(&()), 2.0);
    }
}
