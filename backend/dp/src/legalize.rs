//! Terminal legalization: push devices apart until every pair is at least
//! `clearance` apart on some axis. Every overlap is a defect — `cellgen` emits
//! one self-contained macro per cell, so nothing shares diffusion.

use analog::Requirements;
use gp::mechanics::{analog_violations, snap};
use pnr_core::Layout;

/// Relax encroaching pairs apart along their cheaper axis, then re-project
/// violated hard batches (a pushed symmetry partner is re-mirrored), until
/// clean, stalled, or `max_sweeps`. Pinned devices never move, positions stay
/// on `grid`, and a sweep that still raises hard violations is rolled back.
/// Returns the residual encroachment area (nonzero when the die cannot fit).
///
/// ponytail: O(n²) pairwise relaxation; a constraint graph + LP compaction if
/// blocks reach thousands of devices.
pub fn separate_overlaps(
    l: &mut Layout,
    reqs: &Requirements<Layout>,
    fixed: &[bool],
    grid: i32,
    clearance: i32,
    max_sweeps: u32,
) -> f64 {
    let n = l.x.len();
    let g = grid.max(1);
    let movable = |i: usize| !fixed.get(i).copied().unwrap_or(false);

    for _ in 0..max_sweeps {
        let before = encroachment(l, clearance);
        if before <= 0.0 {
            return 0.0;
        }
        let before_hard = analog_violations(reqs, l);
        let (save_x, save_y, save_axis) = (l.x.clone(), l.y.clone(), l.axis.clone());

        for a in 0..n {
            for b in (a + 1)..n {
                let ox = (l.hw[a] + l.hw[b] + clearance) - (l.x[a] - l.x[b]).abs();
                let oy = (l.hh[a] + l.hh[b] + clearance) - (l.y[a] - l.y[b]).abs();
                if ox <= 0 || oy <= 0 {
                    continue;
                }
                let (ma, mb) = (movable(a), movable(b));
                let push = |whole: i32| -> (i32, i32) {
                    match (ma, mb) {
                        (true, true) => (whole / 2 + whole % 2, whole / 2),
                        (true, false) => (whole, 0),
                        (false, true) => (0, whole),
                        (false, false) => (0, 0),
                    }
                };
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
        for batch in &reqs.hard {
            if batch.violations(l) > 0 {
                batch.project(l, g);
            }
        }
        for i in (0..n).filter(|&i| !movable(i)) {
            l.x[i] = save_x[i];
            l.y[i] = save_y[i];
        }
        l.refresh_temps();

        if analog_violations(reqs, l) > before_hard {
            l.x = save_x;
            l.y = save_y;
            l.axis = save_axis;
            l.refresh_temps();
            return encroachment(l, clearance);
        }
        if encroachment(l, clearance) >= before {
            return encroachment(l, clearance);
        }
    }
    encroachment(l, clearance)
}

/// Clearance-inflated overlap of `a` and `b`, nm²: nonzero iff their edge gap
/// is under `clearance` on both axes.
#[inline]
pub(crate) fn encroach(l: &Layout, a: usize, b: usize, clearance: i32) -> f64 {
    let ox = (l.hw[a] + l.hw[b] + clearance) - (l.x[a] - l.x[b]).abs();
    let oy = (l.hh[a] + l.hh[b] + clearance) - (l.y[a] - l.y[b]).abs();
    if ox > 0 && oy > 0 {
        f64::from(ox) * f64::from(oy)
    } else {
        0.0
    }
}

/// Total encroachment over all pairs.
pub(crate) fn encroachment(l: &Layout, clearance: i32) -> f64 {
    let n = l.x.len();
    (0..n).flat_map(|a| (a + 1..n).map(move |b| (a, b))).map(|(a, b)| encroach(l, a, b, clearance)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gp::mechanics::total_overlap;
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
        }
    }

    #[test]
    fn fully_overlapping_devices_are_separated() {
        // Two devices stacked exactly on top of each other.
        let mut l = layout(&[(0, 0), (0, 0)], 500);
        let reqs = Requirements::<Layout>::default();
        assert!(total_overlap(&l) > 0.0);
        let residual = separate_overlaps(&mut l, &reqs, &[false, false], 5, 0, 64);
        assert_eq!(residual, 0.0, "legalizer must clear the overlap");
        assert_eq!(total_overlap(&l), 0.0);
    }

    #[test]
    fn a_pinned_macro_never_moves() {
        let mut l = layout(&[(0, 0), (200, 0)], 500);
        let reqs = Requirements::<Layout>::default();
        separate_overlaps(&mut l, &reqs, &[true, false], 5, 0, 64);
        assert_eq!((l.x[0], l.y[0]), (0, 0), "pinned device must stay put");
        assert_eq!(total_overlap(&l), 0.0);
    }

    #[test]
    fn already_legal_layout_is_untouched() {
        let mut l = layout(&[(0, 0), (10_000, 0)], 500);
        let reqs = Requirements::<Layout>::default();
        let before = (l.x.clone(), l.y.clone());
        let residual = separate_overlaps(&mut l, &reqs, &[false, false], 5, 0, 64);
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
        let residual = separate_overlaps(&mut l, &reqs, &[false, false], 5, 0, 64);
        assert_eq!(residual, 0.0, "a stacked group is overlap like any other");
        assert_eq!(total_overlap(&l), 0.0, "and it must actually be pulled apart");
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
        let residual = separate_overlaps(&mut l, &reqs, &[false; 4], 5, 0, 64);
        assert_eq!(residual, 0.0);
        assert_eq!(total_overlap(&l), 0.0);
        l.debug_check_placed("separate_overlaps");
    }

    #[test]
    fn clearance_pushes_past_mere_non_overlap() {
        // Two devices that merely touch are "not overlapping" yet still illegal:
        // the layers inside their boxes (wells, diffusion) owe the PDK real
        // spacing, and at a grid step apart they merge into one polygon.
        let mut l = layout(&[(0, 0), (1_000, 0)], 500); // edges exactly touching
        let reqs = Requirements::<Layout>::default();
        assert_eq!(total_overlap(&l), 0.0, "precondition: no raw overlap");

        separate_overlaps(&mut l, &reqs, &[false, false], 5, 1_300, 64);
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
        };
        let reqs = Requirements::<Layout>::default();
        separate_overlaps(&mut l, &reqs, &[false, false], 5, 0, 64);
        assert_eq!(total_overlap(&l), 0.0);
        assert_eq!(l.x, vec![0, 0], "x was the expensive axis; it must not move");
    }
}
