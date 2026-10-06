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
#[derive(Clone, Copy)]
pub struct DtiBand {
    pub a: Target,
    pub b: Target,
    /// Max gap that counts as abutting, nm.
    pub s_max_nm: i32,
    /// Min gap that counts as separated, nm.
    pub d_dti_nm: i32,
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

    /// Band width, nm, floored at 1.
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
        for t in [self.a, self.b] {
            if let Target::Device(d) = t {
                out.push(u32::from(d.0));
            }
        }
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
