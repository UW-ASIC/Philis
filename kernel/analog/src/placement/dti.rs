//! Deep-trench isolation banding (placement tier).
//!
//! Ported from `backend/constraints/src/placement_level/dti.rs`.
//!
//! ## What the annotator owes this rule (and now pays — `annotator::emit`)
//!
//! [`DtiBand::branch`] is an index into [`Layout::branch`]. The producer is
//! `annotator::emit::placement`, which emits one `DtiBand` per recognised pair and
//! honours the three-point contract below; `dp`'s `try_branch` is the mover that
//! flips the commitment. The contract stays here because it binds every future
//! producer, not just the first one:
//!
//! 1. **Allocate one [`BranchId`] per emitted pair**, densely from `0`, and size
//!    `Layout::branch` to that count (`gp` writes the table — see
//!    `gp::mechanics`'s "a valid starting commitment per `DtiBand`"). Two pairs must never
//!    share an id: the commitment is per pair, and sharing would couple two independent
//!    disjunctions into one flip.
//! 2. **Seed the commitment from the recognised structure, not from geometry.** `false`
//!    (share) for pairs the recogniser says belong in one trench — same well, same
//!    matched group, diffusion-shareable. `true` (isolate) for a noisy/sensitive pair or
//!    an injector on a `<1 kΩ` path from a pad, which is forced into a private ring. A
//!    seed taken from the *current* gap would just re-derive the accident the branch
//!    exists to eliminate (PLAN §4b), since coarse placement has not run yet.
//! 3. **Keep the id stable across epochs.** `Requirements` order is already a contract
//!    for `gp::Prices`; `BranchId` is the same kind of contract for the search state, and
//!    a renumbering between epochs silently transfers one pair's commitment to another.
//!
//! Injector-exclusion and the implant-merge/LVS guard are the same disjunctive shape and
//! want the same treatment; `DtiBand` is the one that exists today.

use pnr_core::ids::{BranchId, Target};
use pnr_core::layout::Layout;
use crate::rule::Rule;

/// **Deep-trench isolation.** Two devices may either **abut** (gap `< s_max`,
/// sharing one trench) or be **fully separated** (gap `> d_dti`); the band between
/// is forbidden, because a partial trench neither isolates nor shares cleanly.
/// This makes DTI a discrete two-state distance rule, not a monotone spacing.
///
/// - **Enforcement:** [`crate::Mode::Hard`] (legality, priority 75).
/// - **Arity:** Device↔Device.
/// - **Books:** AOAL ch02/2.5.1 (#45); ALS 3.1/3.1.2 (#6); PNR_ANALOG 04/4.A (#65).
#[derive(Clone, Copy)]
pub struct DtiBand {
    pub a: Target,
    pub b: Target,
    /// Max gap to count as abutting (share one trench), `nm`.
    pub s_max_nm: i32,
    /// Min gap to count as fully separated, `nm`.
    pub d_dti_nm: i32,
    /// Which side of the disjunction this pair is **currently committed to** — an index
    /// into [`Layout::branch`], where `false` = share and `true` = isolate.
    ///
    /// This is PLAN §4b's `b ∈ {share, isolate}` search variable, and it is the whole
    /// reason the rule can be enforced at all. The feasible set is
    /// `{d ≤ s_max} ∪ {d ≥ d_dti}` — two disconnected components with an illegal
    /// interval between. A penalty on band penetration (which is exactly what `cost`
    /// below computes) is **maximal in the middle of the illegal gap** and descends away
    /// from it in *both* directions, so it tells the optimiser to leave the band but
    /// never which way. Which component a pair ends up in becomes an accident of the
    /// initial placement rather than a decision.
    ///
    /// With the branch explicit, each component is an ordinary convex interval the
    /// optimiser can satisfy cleanly, and the choice between them becomes a discrete
    /// move `dp` can flip and price. Same shape for injector-exclusion (a device on a
    /// <1 kΩ path from a pad forced into a private ring) and for the implant-merge/LVS
    /// guard — all three are "either share by design, or keep the keepout", and none is
    /// expressible as a penalty.
    ///
    /// It lives in `Layout` rather than in the rule because a rule is `Copy` and scored
    /// immutably, while the search must be able to *change* the commitment — and
    /// `Layout` is the search state. The rule holds an index rather than the bool
    /// because `RuleBatch` is type-erased: `dp` cannot reach inside a batch to find a
    /// branch, but it can flip `layout.branch[i]` and re-score.
    pub branch: BranchId,
    /// The recognised-structure **starting commitment** for [`branch`](DtiBand::branch):
    /// `true` = isolate. Carried on the rule (not in `Layout`) because the annotator
    /// runs once per run while `gp` hands `dp` a fresh all-`false` table every epoch —
    /// the seed has to survive on the one object that does, so the mover can re-write
    /// it at entry. Seeded from what the pair *is* (matched → share, noisy reference →
    /// isolate), never from the current gap, which would re-derive the accident the
    /// branch exists to eliminate.
    pub seed_isolate: bool,
}

impl DtiBand {
    /// Which component this pair is committed to: `true` = isolate, `false` = share.
    ///
    /// An out-of-range [`BranchId`] reads as `share`, matching `Layout::branch`'s
    /// documented "an all-`false` table is a valid starting commitment". That is not
    /// defensive padding — a caller scoring against a `Layout` whose table was never
    /// sized (a hand-built test bench, a stage upstream of `dp`'s seeding) reads the
    /// `share` branch and `cost` is the abut-or-nothing pull. A panic here would make
    /// the not-yet-seeded state a crash instead of a conservative default.
    #[inline]
    fn isolating(self, l: &Layout) -> bool {
        l.branch.get(self.branch.0 as usize).copied().unwrap_or(false)
    }

    /// Width of the forbidden interval, `nm` — this rule's budget.
    ///
    /// The natural denominator for [`Rule::residual`]: a pair sitting inside the band is
    /// at most one band-width from the branch it is committed to, so the residual lands in
    /// `(0, 1]` and is directly comparable with every other rule's "fraction of my own
    /// spec". Normalising by `s_max` or `d_dti` instead would make a process with a wide
    /// isolation rule look worse at identical geometry.
    #[inline]
    fn band_nm(self) -> f32 {
        (self.d_dti_nm - self.s_max_nm) as f32
    }
}

impl Rule for DtiBand {
    type On = Layout;
    /// Distance from the **committed** branch's feasible interval, `nm`.
    ///
    /// One branch, not the band. `false` (share) pulls the pair together toward
    /// `gap ≤ s_max`; `true` (isolate) pushes it apart toward `gap ≥ d_dti`. Each is an
    /// ordinary convex interval, so the gradient has exactly one direction to point and
    /// the optimiser can close it.
    ///
    /// What this replaces is worth keeping in view, because it is PLAN §4b's failure in
    /// one expression: the old branch-free metric
    /// `(gap − s_max).min(d_dti − gap).max(0)` **peaks dead centre in the forbidden
    /// interval** and descends away from it in *both* directions. It tells the optimiser
    /// to leave the band but never which side to leave by, so which component a pair ends
    /// up in is an accident of where it happened to start — and a pair can also be pushed
    /// out the "wrong" side, undoing a trench the router was counting on.
    fn cost(self, l: &Layout) -> f32 {
        let gap = l.edge_gap(self.a, self.b);
        if self.isolating(l) {
            (self.d_dti_nm as f32 - gap).max(0.0)
        } else {
            (gap - self.s_max_nm as f32).max(0.0)
        }
    }
    /// Gap is `< s_max` or `> d_dti` — never in the forbidden band.
    ///
    /// The **full disjunction**, deliberately, even though `cost` above is branch-aware.
    /// Legality is "in one of the two components", full stop; which one the search is
    /// currently committed to is a search detail, and a pair that has drifted into the
    /// *other* component is legal — not violating.
    ///
    /// Making this branch-aware is the trap. A branch flip and the geometry that realises
    /// it cannot be simultaneous: `dp` flips `layout.branch[i]`, then moves devices. In
    /// the interval between, a branch-aware `satisfied` reports a violation for a layout
    /// that is perfectly legal, `analog_phi` sees Φ rise, and Φ-monotone acceptance
    /// rejects the very move that was resolving the flip. The disjunction is also the
    /// honest statement of the physics: the trench either exists or it does not, and the
    /// commitment is bookkeeping about which one the search intends.
    fn satisfied(self, l: &Layout) -> bool {
        let gap = l.edge_gap(self.a, self.b);
        gap < self.s_max_nm as f32 || gap > self.d_dti_nm as f32
    }

    /// How far into the forbidden band the pair sits, as a fraction of the band width.
    ///
    /// `0.0` whenever [`satisfied`](Rule::satisfied) holds — the disjunction, not the
    /// branch, so a pair that legally drifted to the other component reports nothing. Only
    /// *inside* the band does the commitment matter, and there the residual is the distance
    /// left to travel on the committed branch. That keeps Φ (which sums hard residuals)
    /// consistent with `cost`: both point the same way, and both go to zero together.
    fn residual(self, l: &Layout) -> f32 {
        if self.satisfied(l) {
            return 0.0;
        }
        crate::rule::over(self.cost(l), self.band_nm())
    }

    fn retarget(self, cell_of: &[u16]) -> Self {
        Self { a: self.a.retarget(cell_of), b: self.b.retarget(cell_of), ..self }
    }

    /// `(id, seed)` — seed is the recognised-structure starting commitment,
    /// `true` = isolate. This is what lets `dp` learn which branch bits exist at
    /// all: the batch seam is type-erased, so without this override a flip move
    /// would be a guess at an index that prices nothing.
    fn branch(self) -> Option<(BranchId, bool)> {
        Some((self.branch, self.seed_isolate))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_core::ids::DeviceId;

    /// Two 1 µm-wide devices on one row, edge gap `gap` nm, and one branch slot.
    fn bench(gap: i32, isolate: bool) -> Layout {
        Layout {
            x: vec![0, 1_000 + gap],
            y: vec![0, 0],
            hw: vec![500; 2],
            hh: vec![500; 2],
            orient: vec![pnr_core::Orient::default(); 2],
            variant: vec![0; 2],
            axis: vec![0],
            branch: vec![isolate],
            groups: vec![],
            power_uw: vec![0; 2],
            temp_mc: vec![0; 2],
        }
    }

    /// Share below 200 nm, isolate above 2000 nm; the 1800 nm between is forbidden.
    fn rule() -> DtiBand {
        DtiBand {
            a: Target::Device(DeviceId(0)),
            b: Target::Device(DeviceId(1)),
            s_max_nm: 200,
            d_dti_nm: 2_000,
            branch: BranchId(0),
            seed_isolate: false,
        }
    }

    #[test]
    fn cost_follows_the_committed_branch_not_the_band_centre() {
        let mid = 1_100; // dead centre of the forbidden band
        let share = rule().cost(&bench(mid, false));
        let isolate = rule().cost(&bench(mid, true));
        // Same geometry, opposite instructions: `share` measures the distance still to
        // close (1100 − 200), `isolate` the distance still to open (2000 − 1100).
        assert!((share - 900.0).abs() < 1e-3, "share should pull together: {share}");
        assert!((isolate - 900.0).abs() < 1e-3, "isolate should push apart: {isolate}");

        // The discriminating case, and the one the old band-penetration metric could not
        // express: move the pair *closer* and the two branches disagree about whether it
        // improved. A branch-free penalty peaks mid-band and falls off both ways, so it
        // called this an improvement unconditionally (PLAN §4b).
        let closer = 400;
        assert!(rule().cost(&bench(closer, false)) < share, "closer is better when sharing");
        assert!(rule().cost(&bench(closer, true)) > isolate, "closer is worse when isolating");

        // Each branch bottoms out on its own interval, and only on its own.
        assert_eq!(rule().cost(&bench(100, false)), 0.0, "abutting satisfies `share`");
        assert!(rule().cost(&bench(100, true)) > 0.0, "abutting does not satisfy `isolate`");
        assert_eq!(rule().cost(&bench(3_000, true)), 0.0, "far apart satisfies `isolate`");
        assert!(rule().cost(&bench(3_000, false)) > 0.0, "far apart does not satisfy `share`");
    }

    #[test]
    fn satisfied_accepts_both_components_whatever_the_branch_says() {
        // The trap this rule documents: legality is the *disjunction*. A pair that has
        // drifted into the component it is not committed to is legal — reporting it as a
        // violation would make Φ rise on the move that resolves a pending flip, and
        // Φ-monotone acceptance in `dp` would reject exactly that move.
        for &isolate in &[false, true] {
            assert!(rule().satisfied(&bench(100, isolate)), "abutting is legal (share side)");
            assert!(rule().satisfied(&bench(3_000, isolate)), "separated is legal (isolate side)");
            assert!(!rule().satisfied(&bench(1_100, isolate)), "mid-band is never legal");
        }
        // ...and the residual agrees with `satisfied`, not with the branch: zero on both
        // components, positive only inside the band.
        assert_eq!(rule().residual(&bench(100, true)), 0.0);
        assert_eq!(rule().residual(&bench(3_000, false)), 0.0);
        // Mid-band, 900 nm of an 1800 nm band still to travel ⇒ half a budget.
        assert!((rule().residual(&bench(1_100, false)) - 0.5).abs() < 1e-3);
    }

    #[test]
    fn branches_collects_ids_and_seeds_and_only_from_overrides() {
        use crate::rule::RuleBatch;

        // A batch of `DtiBand`s hands the consumer exactly its `(id, seed)` pairs, in
        // rule order — the seam `dp`'s flip move seeds `Layout::branch` from.
        let batch = vec![
            DtiBand { branch: BranchId(0), seed_isolate: false, ..rule() },
            DtiBand { branch: BranchId(1), seed_isolate: true, ..rule() },
        ];
        let mut out = Vec::new();
        batch.branches(&mut out);
        assert_eq!(out, vec![(BranchId(0), false), (BranchId(1), true)]);

        // A kind without a `branch` override contributes nothing: `Symmetry` has no
        // disjunction to commit to, so a consumer summing over every hard batch sees
        // only the ids that exist.
        let sym = vec![crate::placement::Symmetry {
            a: Target::Device(DeviceId(0)),
            b: Target::Device(DeviceId(1)),
            axis: pnr_core::ids::AxisId(0),
        }];
        sym.branches(&mut out);
        assert_eq!(out.len(), 2, "a branch-less kind must not invent ids");
    }

    #[test]
    fn an_unassigned_branch_reads_as_share() {
        // Nothing constructs a `DtiBand` with a real `BranchId` yet (see the module note),
        // so an empty `Layout::branch` must not panic — it must read the documented
        // all-`false` starting commitment.
        let mut l = bench(1_100, true);
        l.branch.clear();
        assert!((rule().cost(&l) - 900.0).abs() < 1e-3);
        assert!(!rule().satisfied(&l));
    }
}
