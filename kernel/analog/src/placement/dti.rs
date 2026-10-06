//! Deep-trench isolation banding (placement tier).

use pnr_core::ids::{BranchId, Target};
use pnr_core::layout::Layout;
use crate::rule::Rule;

/// Two devices either abut (gap `≤ s_max`, one shared trench) or fully
/// separate (gap `≥ d_dti`); the band between is illegal.
///
/// The feasible set is two disconnected intervals, so a penalty cannot pick a
/// side. `branch` indexes [`Layout::branch`], the committed side (`false` =
/// share, `true` = isolate) that `dp` flips as a discrete move; `cost` pulls
/// toward the committed side only.
///
/// Producer contract (`annotator::emit`): one dense [`BranchId`] per pair, seed
/// from recognised structure (not current geometry), ids stable across epochs.
#[derive(Clone, Copy, Debug)]
pub struct DtiBand {
    /// First device of the pair.
    pub a: Target,
    /// Second device of the pair.
    pub b: Target,
    /// Max gap that counts as abutting, nm.
    pub s_max_nm: i32,
    /// Min gap that counts as separated, nm.
    pub d_dti_nm: i32,
    /// Slot in [`Layout::branch`] holding the committed side.
    pub branch: BranchId,
    /// Starting commitment (`true` = isolate); `dp` re-seeds `Layout::branch`
    /// from it each epoch.
    pub seed_isolate: bool,
}

impl DtiBand {
    /// Committed side; an unsized `Layout::branch` reads as share.
    #[inline]
    fn isolating(self, l: &Layout) -> bool {
        l.branch.get(self.branch.0 as usize).copied().unwrap_or(false)
    }

    /// Distance, nm, to the committed side's interval.
    fn miss(self, l: &Layout) -> f32 {
        let gap = l.edge_gap(self.a, self.b);
        if self.isolating(l) {
            (self.d_dti_nm as f32 - gap).max(0.0)
        } else {
            (gap - self.s_max_nm as f32).max(0.0)
        }
    }

    /// Band width, nm, floored at 1 so `cost` never divides by zero.
    fn band(self) -> f32 {
        (self.d_dti_nm - self.s_max_nm).max(1) as f32
    }
}

impl Rule for DtiBand {
    type On = Layout;
    /// `(miss / band)²`, miss = distance to the committed side's interval,
    /// band = `d_dti_nm − s_max_nm` (PLC-18: dimensionless).
    fn cost(self, l: &Layout) -> f32 {
        let e = self.miss(l) / self.band();
        e * e
    }
    /// The full disjunction, **not** branch-aware: a pair in the uncommitted
    /// component is legal. A branch-aware check would flag the legal layout
    /// between a flip and the move realising it, and Φ-monotone acceptance in
    /// `dp` would reject exactly that move.
    fn satisfied(self, l: &Layout) -> bool {
        let gap = l.edge_gap(self.a, self.b);
        gap <= self.s_max_nm as f32 || gap >= self.d_dti_nm as f32
    }
    /// `0` when satisfied; else the miss over the band width.
    fn residual(self, l: &Layout) -> f32 {
        if self.satisfied(l) {
            return 0.0;
        }
        crate::rule::over(self.miss(l), (self.d_dti_nm - self.s_max_nm) as f32)
    }
    fn touches(self, out: &mut Vec<u32>) {
        super::push_devices(out, &[self.a, self.b]);
    }
    fn retarget(self, cell_of: &[u16]) -> Self {
        Self { a: self.a.retarget(cell_of), b: self.b.retarget(cell_of), ..self }
    }
    fn branch(self) -> Option<(BranchId, bool)> {
        Some((self.branch, self.seed_isolate))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_core::ids::DeviceId;

    /// PLC-10: dp re-scores a batch only when a touched cell moves.
    #[test]
    fn touched_names_both_targets() {
        use crate::rule::RuleBatch;
        let band = |a, b| DtiBand { a, b, s_max_nm: 0, d_dti_nm: 1_000, branch: BranchId(0), seed_isolate: false };
        let (d, g) = (|i| Target::Device(DeviceId(i)), Target::Group(pnr_core::GroupId(0)));
        let mut ids = Vec::new();
        vec![band(d(2), d(5)), band(g, g)].touched(&mut ids);
        assert_eq!(ids, [2, 5]);
    }

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
            units: Default::default(),
        }
    }

    /// Share at or below 200 nm, isolate at or above 2000 nm; the band strictly between is forbidden.
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
        // Both are 900 nm of an 1800 nm band: (900/1800)² = 0.25.
        assert!((share - 0.25).abs() < 1e-6, "share should pull together: {share}");
        assert!((isolate - 0.25).abs() < 1e-6, "isolate should push apart: {isolate}");

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
    fn a_pair_exactly_at_either_end_is_legal() {
        assert!(rule().satisfied(&bench(200, false)));
        assert!(rule().satisfied(&bench(2_000, true)));
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
            mode: crate::placement::SymMode::Perfect,
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
        assert!((rule().cost(&l) - 0.25).abs() < 1e-6);
        assert!(!rule().satisfied(&l));
    }
}

#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use crate::rule::RuleBatch;
    use pnr_core::ids::DeviceId;

    fn layout(gap: i32, isolate: bool) -> Layout {
        Layout {
            x: vec![0, 1_000 + gap],
            y: vec![0, 0],
            hw: vec![500; 2],
            hh: vec![500; 2],
            orient: vec![pnr_core::Orient::default(); 2],
            variant: vec![0; 2],
            axis: vec![],
            branch: vec![isolate],
            groups: vec![],
            power_uw: vec![0; 2],
            temp_mc: vec![0; 2],
            units: Default::default(),
        }
    }

    fn band(s_max_nm: i32, d_dti_nm: i32) -> DtiBand {
        DtiBand { a: Target::Device(DeviceId(0)), b: Target::Device(DeviceId(1)), s_max_nm, d_dti_nm, branch: BranchId(0), seed_isolate: true }
    }

    #[test]
    fn one_nm_into_the_band_is_one_band_fraction() {
        let r = band(200, 2_000);
        assert!(!r.satisfied(&layout(201, false)));
        assert!((r.residual(&layout(201, false)) - 1.0 / 1_800.0).abs() < 1e-7);
        assert!(!r.satisfied(&layout(1_999, true)));
        assert!((r.residual(&layout(1_999, true)) - 1.0 / 1_800.0).abs() < 1e-7);
        // Cost squares the same fraction.
        assert!((r.cost(&layout(201, false)) - (1.0f32 / 1_800.0).powi(2)).abs() < 1e-10);
    }

    #[test]
    fn an_empty_band_forbids_nothing() {
        // d_dti ≤ s_max: the two intervals cover every gap.
        for (s, d) in [(500, 500), (800, 300)] {
            for gap in [0, 300, 500, 800, 10_000] {
                let l = layout(gap, false);
                assert!(band(s, d).satisfied(&l), "s {s} d {d} gap {gap}");
                assert_eq!(band(s, d).residual(&l), 0.0);
                assert_eq!(RuleBatch::violations(&vec![band(s, d)], &l), 0);
            }
        }
        // `cost` still pulls toward the committed side, over a 1 nm band floor
        // (no division by zero or a negative width).
        let c = band(500, 500).cost(&layout(600, false));
        assert!((c - 100.0 * 100.0).abs() < 1e-3, "{c}");
    }

    #[test]
    fn branch_reports_the_id_and_seed() {
        assert_eq!(band(0, 1).branch(), Some((BranchId(0), true)));
        let r = DtiBand { branch: BranchId(7), seed_isolate: false, ..band(0, 1) };
        assert_eq!(r.branch(), Some((BranchId(7), false)));
    }

    #[test]
    fn a_branch_slot_past_the_table_reads_as_share() {
        let r = DtiBand { branch: BranchId(3), ..band(200, 2_000) };
        // Slot 3 is missing; slot 0 says isolate but belongs to someone else.
        let l = layout(1_100, true);
        assert!((r.cost(&l) - 0.25).abs() < 1e-6, "share: 900 of 1800 nm");
        assert_eq!(r.cost(&layout(100, true)), 0.0, "abutting satisfies share");
    }

    #[test]
    fn retarget_maps_both_sides_and_keeps_the_rest() {
        let r = band(10, 20).retarget(&[5, 6]);
        assert_eq!((r.a, r.b), (Target::Device(DeviceId(5)), Target::Device(DeviceId(6))));
        assert_eq!((r.s_max_nm, r.d_dti_nm, r.branch, r.seed_isolate), (10, 20, BranchId(0), true));
    }
}
