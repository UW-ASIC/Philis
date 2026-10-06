//! Terminal legalization: push devices apart until every pair is at least
//! its per-pair gap ([`PlaceRules::gaps`]) apart on some axis. Every overlap
//! is a defect — `cellgen` emits one self-contained macro per cell, so
//! nothing shares diffusion.

use analog::Requirements;
use gp::mechanics::{analog_violations, snap};
use gp::PlaceRules;
use pnr_core::Layout;

/// Relax encroaching pairs apart along their cheaper axis, then re-project
/// violated hard batches (a pushed symmetry partner is re-mirrored), until
/// clean, stalled, or `max_sweeps`. Positions stay on `grid`, and a sweep that
/// still raises hard violations is rolled back.
/// Returns the residual encroachment area, nm² (nonzero when the die cannot
/// fit). `max_sweeps = 0` only measures. Each sweep is O(n²) pair tests plus
/// one hard-rule projection.
///
/// ponytail: O(n²) pairwise relaxation; a constraint graph + LP compaction if
/// blocks reach thousands of devices.
pub fn separate_overlaps(
    l: &mut Layout,
    reqs: &Requirements<Layout>,
    rules: &PlaceRules,
    max_sweeps: u32,
) -> f64 {
    let n = l.x.len();
    let g = rules.grid.max(1);
    let encroachment = |l: &Layout| rules.encroachment(l);

    for _ in 0..max_sweeps {
        let before = encroachment(l);
        if before <= 0.0 {
            return 0.0;
        }
        let before_hard = analog_violations(reqs, l);
        let (save_x, save_y, save_axis) = (l.x.clone(), l.y.clone(), l.axis.clone());

        for a in 0..n {
            for b in (a + 1)..n {
                let (gx, gy) = rules.gaps(l, a, b);
                let ox = (l.hw[a] + l.hw[b] + gx) - (l.x[a] - l.x[b]).abs();
                let oy = (l.hh[a] + l.hh[b] + gy) - (l.y[a] - l.y[b]).abs();
                if ox <= 0 || oy <= 0 {
                    continue;
                }
                let push = |whole: i32| (whole / 2 + whole % 2, whole / 2);
                // Clear the encroachment plus one grid step.
                if ox <= oy {
                    let (pa, pb) = push(ox + g);
                    let dir = if l.x[a] <= l.x[b] { -1 } else { 1 };
                    l.x[a] = snap(l.x[a] + dir * pa, g);
                    l.x[b] = snap(l.x[b] - dir * pb, g);
                } else {
                    let (pa, pb) = push(oy + g);
                    let dir = if l.y[a] <= l.y[b] { -1 } else { 1 };
                    l.y[a] = snap(l.y[a] + dir * pa, g);
                    l.y[b] = snap(l.y[b] - dir * pb, g);
                }
            }
        }
        project_violated(reqs, l, g);
        l.refresh_temps();

        // A sweep that breaks a hard rule or leaves more encroachment than it
        // found is undone: the legalizer never returns a worse layout.
        let after = encroachment(l);
        if analog_violations(reqs, l) > before_hard || after > before {
            l.x = save_x;
            l.y = save_y;
            l.axis = save_axis;
            l.refresh_temps();
            return before;
        }
        if after == before {
            return after;
        }
    }
    encroachment(l)
}

/// Project each hard batch that is violated at its turn onto its feasible
/// set, in batch order, snapping to `grid`. Temperatures are left stale.
pub(crate) fn project_violated(reqs: &Requirements<Layout>, l: &mut Layout, grid: i32) {
    for batch in &reqs.hard {
        // Re-checked per batch: an earlier projection may have fixed it, and a
        // satisfied SymmetryGroup would still re-average its axis.
        if batch.violations(l) > 0 {
            batch.project(l, grid);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gp::mechanics::encroachment;
    use pnr_core::ids::DeviceId;

    fn layout(centres: &[(i32, i32)], half: i32) -> Layout {
        let n = centres.len();
        Layout {
            x: centres.iter().map(|c| c.0).collect(),
            y: centres.iter().map(|c| c.1).collect(),
            hw: vec![half; n],
            hh: vec![half; n],
            variant: vec![0; n],
            axis: vec![0; n],
            branch: vec![false; n],
            groups: (0..n).map(|i| vec![DeviceId(i as u16)]).collect(),
            orient: vec![pnr_core::Orient::default(); n],
            power_uw: vec![0; n],
            temp_mc: vec![0; n],
            units: Default::default(),
        }
    }

    #[test]
    fn fully_overlapping_devices_are_separated() {
        // Two devices stacked exactly on top of each other.
        let mut l = layout(&[(0, 0), (0, 0)], 500);
        let reqs = Requirements::<Layout>::default();
        assert!(encroachment(&l, 0) > 0.0);
        let residual = separate_overlaps(&mut l, &reqs, &PlaceRules::uniform(5, 0), 64);
        assert_eq!(residual, 0.0, "legalizer must clear the overlap");
        assert_eq!(encroachment(&l, 0), 0.0);
    }

    #[test]
    fn already_legal_layout_is_untouched() {
        let mut l = layout(&[(0, 0), (10_000, 0)], 500);
        let reqs = Requirements::<Layout>::default();
        let before = (l.x.clone(), l.y.clone());
        let residual = separate_overlaps(&mut l, &reqs, &PlaceRules::uniform(5, 0), 64);
        assert_eq!(residual, 0.0);
        assert_eq!((l.x, l.y), before);
    }

    #[test]
    fn devices_in_one_group_are_separated_too() {
        // Grouping used to exempt a pair from separation, on the premise that the
        // generator had drawn them as one merged finger stack. `cellgen` draws one
        // self-contained macro per device, so a group landing on itself is two
        // devices' geometry stacked — the shipped defect this pass now fixes.
        let mut l = layout(&[(0, 0), (0, 0)], 500);
        l.groups = vec![vec![DeviceId(0), DeviceId(1)]];
        let reqs = Requirements::<Layout>::default();
        let residual = separate_overlaps(&mut l, &reqs, &PlaceRules::uniform(5, 0), 64);
        assert_eq!(residual, 0.0, "a stacked group is overlap like any other");
        assert_eq!(encroachment(&l, 0), 0.0, "and it must actually be pulled apart");
    }

    #[test]
    fn every_pair_separates_regardless_of_grouping() {
        // Four devices in two groups, all landing on each other.
        let mut l = layout(&[(0, 0), (0, 0), (100, 0), (100, 0)], 500);
        l.groups = vec![
            vec![DeviceId(0), DeviceId(1)],
            vec![DeviceId(2), DeviceId(3)],
        ];
        let reqs = Requirements::<Layout>::default();
        let residual = separate_overlaps(&mut l, &reqs, &PlaceRules::uniform(5, 0), 64);
        assert_eq!(residual, 0.0);
        assert_eq!(encroachment(&l, 0), 0.0);
        l.debug_check_placed("separate_overlaps");
    }

    #[test]
    fn clearance_pushes_past_mere_non_overlap() {
        // Two devices that merely touch are "not overlapping" yet still illegal:
        // the layers inside their boxes (wells, diffusion) owe the PDK real
        // spacing, and at a grid step apart they merge into one polygon.
        let mut l = layout(&[(0, 0), (1_000, 0)], 500); // edges exactly touching
        let reqs = Requirements::<Layout>::default();
        assert_eq!(encroachment(&l, 0), 0.0, "precondition: no raw overlap");

        separate_overlaps(&mut l, &reqs, &PlaceRules::uniform(5, 1_300), 64);
        let gap = (l.x[1] - l.x[0]) - (l.hw[0] + l.hw[1]);
        assert!(gap >= 1_300, "expected >=1300nm of clearance, got {gap}");
    }

    #[test]
    fn separates_along_the_cheaper_axis() {
        // Deep in x, shallow in y ⇒ push apart vertically, leaving x alone.
        let mut l = Layout {
            x: vec![0, 0],
            y: vec![0, 900],
            hw: vec![5_000; 2],
            hh: vec![500; 2],
            variant: vec![0; 2],
            axis: vec![0; 2],
            branch: vec![false; 2],
            groups: vec![vec![DeviceId(0)], vec![DeviceId(1)]],
            orient: vec![pnr_core::Orient::default(); 2],
            power_uw: vec![0; 2],
            temp_mc: vec![0; 2],
            units: Default::default(),
        };
        let reqs = Requirements::<Layout>::default();
        separate_overlaps(&mut l, &reqs, &PlaceRules::uniform(5, 0), 64);
        assert_eq!(encroachment(&l, 0), 0.0);
        assert_eq!(l.x, vec![0, 0], "x was the expensive axis; it must not move");
    }

    /// Hard batch pinning cell `cell`'s x at `at`; its projection writes `at`
    /// when `projects`, and stamps `y[cell] = mark` (to see whether it ran).
    struct PinX {
        cell: usize,
        at: i32,
        projects: bool,
        mark: i32,
    }

    impl analog::RuleBatch<Layout> for PinX {
        fn cost(&self, _: &Layout) -> f32 {
            0.0
        }
        fn violations(&self, l: &Layout) -> u32 {
            u32::from(l.x[self.cell] != self.at)
        }
        fn project(&self, l: &mut Layout, _grid: i32) {
            l.y[self.cell] = self.mark;
            if self.projects {
                l.x[self.cell] = self.at;
            }
        }
    }

    #[test]
    fn empty_and_single_cell_layouts_are_clean() {
        let reqs = Requirements::<Layout>::default();
        let rules = PlaceRules::uniform(5, 1_000);
        assert_eq!(separate_overlaps(&mut layout(&[], 500), &reqs, &rules, 64), 0.0);
        let mut l = layout(&[(7, 9)], 500);
        assert_eq!(separate_overlaps(&mut l, &reqs, &rules, 64), 0.0);
        assert_eq!((l.x[0], l.y[0]), (7, 9));
    }

    #[test]
    fn zero_sweeps_only_measures() {
        let mut l = layout(&[(0, 0), (0, 0)], 500);
        let reqs = Requirements::<Layout>::default();
        let rules = PlaceRules::uniform(5, 0);
        assert_eq!(separate_overlaps(&mut l, &reqs, &rules, 0), 1e6);
        assert_eq!((l.x.clone(), l.y.clone()), (vec![0, 0], vec![0, 0]));
    }

    /// Equal centres: the lower index goes left, the other right, both on grid.
    #[test]
    fn coincident_pair_splits_both_ways_on_grid() {
        let mut l = layout(&[(0, 0), (0, 0)], 500);
        let reqs = Requirements::<Layout>::default();
        assert_eq!(separate_overlaps(&mut l, &reqs, &PlaceRules::uniform(5, 0), 64), 0.0);
        assert!(l.x[0] < 0 && l.x[1] > 0, "{:?}", l.x);
        assert!(l.x.iter().chain(&l.y).all(|v| v % 5 == 0), "{:?} {:?}", l.x, l.y);
    }

    /// A sweep that breaks a hard rule it cannot repair is rolled back whole.
    #[test]
    fn a_sweep_that_breaks_a_hard_rule_is_rolled_back() {
        let mut l = layout(&[(0, 0), (0, 0)], 500);
        let reqs = Requirements::<Layout> {
            hard: vec![Box::new(PinX { cell: 0, at: 0, projects: false, mark: 0 }), Box::new(PinX { cell: 1, at: 0, projects: false, mark: 0 })],
            ..Default::default()
        };
        let residual = separate_overlaps(&mut l, &reqs, &PlaceRules::uniform(5, 0), 64);
        assert_eq!((l.x.clone(), l.y.clone()), (vec![0, 0], vec![0, 0]));
        assert_eq!(residual, 1e6, "the residual is the rolled-back layout's");
    }

    /// Only batches still violated at their turn are projected.
    #[test]
    fn project_violated_skips_satisfied_batches() {
        let mut l = layout(&[(40, 0), (0, 0)], 500);
        let reqs = Requirements::<Layout> {
            hard: vec![
                Box::new(PinX { cell: 0, at: 0, projects: true, mark: 11 }),
                Box::new(PinX { cell: 1, at: 0, projects: true, mark: 22 }),
                // Satisfied only after the first batch's projection.
                Box::new(PinX { cell: 0, at: 0, projects: true, mark: 33 }),
            ],
            ..Default::default()
        };
        project_violated(&reqs, &mut l, 5);
        assert_eq!((l.x.clone(), l.y.clone()), (vec![0, 0], vec![11, 0]));
    }

    /// Property over random piles: the legalizer never returns a layout more
    /// encroached than it was given, and its return value is the encroachment
    /// of the layout it leaves.
    #[test]
    fn never_returns_a_worse_layout_and_reports_its_own_residual() {
        let mut rng = gp::mechanics::SplitMix64::new(9);
        let reqs = Requirements::<Layout>::default();
        let rules = PlaceRules::uniform(5, 300);
        for case in 0..200 {
            let n = 2 + rng.below(9);
            let centres: Vec<(i32, i32)> =
                (0..n).map(|_| (rng.below(4_000) as i32 - 2_000, rng.below(4_000) as i32 - 2_000)).collect();
            let mut l = layout(&centres, 400 + rng.below(800) as i32);
            let before = rules.encroachment(&l);
            let sweeps = rng.below(4) as u32;
            let residual = separate_overlaps(&mut l, &reqs, &rules, sweeps);
            assert!(residual <= before, "case {case}: {before} -> {residual}");
            assert_eq!(residual, rules.encroachment(&l), "case {case}");
        }
    }
}
