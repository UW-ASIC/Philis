//! Terminal legalization — the pass that *guarantees* what annealing only
//! *approaches*.
//!
//! Every silicon-proven analog flow pairs a soft-guided optimiser with a
//! dedicated exact-legality step, and none trusts a penalty driven to zero:
//! MAGICAL legalizes with a constraint graph plus LP compaction, ALIGN generates
//! correct-by-construction primitives on a snapped grid. The reason is
//! structural — a Metropolis walk returns its *best-seen* state, and if the
//! anneal budget runs out while an overlap remains, that overlap ships. See
//! `backend/TODO.md` §6.
//!
//! This is the smallest pass that closes that gap for the overlap family:
//! relaxation that pushes overlapping devices apart along their minimum
//! translation axis, under a legality guard so it cannot trade one violation for
//! another.
//!
//! ## Every overlap is a defect, because nothing merges devices
//!
//! Analog devices *can* abut on purpose — a current mirror sharing source/drain
//! diffusion, an interdigitated array drawn as one merged finger stack. This
//! pass used to encode that by exempting same-group pairs from separation and
//! moving each group as a rigid unit, on the premise that "the generator that
//! drew them owns their internal geometry".
//!
//! That premise is false in this flow. `library::cellgen::generate` emits **one
//! macro per device**, always `DeviceGroup { devices: vec![i] }` — each with its
//! own diffusion moat, bulk tap strip, implants and well. No two macros can
//! share diffusion, so no overlap between them is ever intentional, and nothing
//! upstream arranges group members into an abutment in the first place. The
//! exemption did not preserve a structure; it froze whatever random offset
//! `gp`'s jitter happened to produce, which is how two matched nfets shipped
//! stacked 76% on top of each other and LVS reported a channel that
//! "ambiguously matches MOS rules [nmos, pmos]".
//!
//! So: every overlapping pair separates. `Layout::groups` still identifies
//! matched sets for analog rules and guard rings — it just no longer grants
//! permission to overlap.
//!
//! ponytail: restore abutment when `cellgen` can emit a merged multi-device
//! macro. `cells::mosfet` already draws one (`n_dev > 1`, `finger_sequence`'s
//! ABBA interdigitation); it is unreachable because `cellgen` never asks for it.
//! At that point the group *is* one macro and the exemption is unnecessary
//! anyway — which is the real reason this is a deletion and not a rewrite.

use analog::Requirements;
use gp::mechanics::{analog_violations, snap, total_overlap};
use pnr_core::Layout;

/// Push overlapping devices apart until the layout is overlap-free.
///
/// Each sweep moves every overlapping pair apart along the axis needing the
/// *least* displacement (minimum translation vector), which keeps the placement
/// as close as possible to the annealed solution — the same "minimum device
/// displacement" objective a constraint-graph legalizer optimises.
///
/// Guarantees offered:
/// - `fixed[i]` devices never move (injected macros stay pinned);
/// - a sweep that increases `Σ reqs.hard.violations` is rolled back, so
///   legalizing overlap cannot break symmetry, isolation or a DTI band;
/// - positions stay on `grid`.
///
/// Returns the residual overlap area, `0.0` when clean. It is
/// **not** guaranteed to reach zero: a die too small for its devices has no
/// overlap-free arrangement, and the caller must surface that rather than loop
/// forever.
///
/// ponytail: O(n²) per sweep, pairwise relaxation rather than a constraint graph
/// + LP compaction. Analog blocks are tens of devices, so this is nothing; if a
/// block ever grows to thousands, swap in the constraint-graph formulation
/// (that is the documented upgrade, not a rewrite of the caller).
pub fn separate_overlaps(
    l: &mut Layout,
    reqs: &Requirements<Layout>,
    fixed: &[bool],
    grid: i32,
    clearance_nm: i32,
    max_sweeps: u32,
) -> f64 {
    let n = l.x.len();
    if n < 2 {
        return 0.0;
    }
    let g = grid.max(1);
    // Devices must end up `clearance` apart, not merely non-overlapping: the
    // bbox bounds drawn geometry, but the layers inside it still owe the PDK
    // real spacing (see `DetailedCfg::clearance_nm`).
    let clearance = clearance_nm.max(0);
    let movable = |i: usize| !fixed.get(i).copied().unwrap_or(false);

    for _ in 0..max_sweeps {
        let before_overlap = encroachment(l, clearance);
        if before_overlap <= 0.0 {
            return 0.0;
        }
        let before_hard = analog_violations(reqs, l);
        let (save_x, save_y) = (l.x.clone(), l.y.clone());

        for a in 0..n {
            for b in (a + 1)..n {
                // Encroachment, not just overlap: two boxes a grid step apart are
                // still illegal when the PDK wants a micron between their wells.
                let ox = (l.hw[a] + l.hw[b] + clearance) - (l.x[a] - l.x[b]).abs();
                let oy = (l.hh[a] + l.hh[b] + clearance) - (l.y[a] - l.y[b]).abs();
                if ox <= 0 || oy <= 0 {
                    continue;
                }
                let (ma, mb) = (movable(a), movable(b));
                if !ma && !mb {
                    continue; // both pinned: the caller's problem, not ours
                }
                let push = |whole: i32| -> (i32, i32) {
                    match (ma, mb) {
                        (true, true) => (whole / 2 + whole % 2, whole / 2),
                        (true, false) => (whole, 0),
                        (false, true) => (0, whole),
                        (false, false) => (0, 0),
                    }
                };
                if ox <= oy {
                    let need = ox + g; // clear the encroachment, plus one grid step
                    let (pa, pb) = push(need);
                    let dir = if l.x[a] <= l.x[b] { -1 } else { 1 };
                    l.x[a] = snap(l.x[a] + dir * pa, g);
                    l.x[b] = snap(l.x[b] - dir * pb, g);
                } else {
                    let need = oy + g;
                    let (pa, pb) = push(need);
                    let dir = if l.y[a] <= l.y[b] { -1 } else { 1 };
                    l.y[a] = snap(l.y[a] + dir * pa, g);
                    l.y[b] = snap(l.y[b] - dir * pb, g);
                }
            }
        }

        // Positions changed ⇒ the thermal field is stale, and hard rules read it.
        l.refresh_temps();

        // Legality guard: never trade an overlap for a symmetry/isolation break.
        // Roll back and stop, leaving the caller the honest residual.
        if analog_violations(reqs, l) > before_hard {
            l.x = save_x;
            l.y = save_y;
            l.refresh_temps();
            return encroachment(l, clearance);
        }
        // No progress ⇒ relaxation has stalled (typically a die too small).
        if encroachment(l, clearance) >= before_overlap {
            return encroachment(l, clearance);
        }
    }
    encroachment(l, clearance)
}

/// **Encroachment** area: overlap measured with each box inflated by `clearance`
/// on every side.
///
/// With `clearance == 0` this is exactly `total_overlap`. Above zero it also
/// counts devices that are merely *too close* — the condition the raw overlap
/// measure cannot see, and the one the PDK actually cares about, since two boxes
/// a grid step apart still merge their wells.
fn encroachment(l: &Layout, clearance: i32) -> f64 {
    if clearance <= 0 {
        return total_overlap(l);
    }
    let n = l.x.len();
    let mut t = 0.0;
    for a in 0..n {
        for b in (a + 1)..n {
            let ox = (l.hw[a] + l.hw[b] + clearance) - (l.x[a] - l.x[b]).abs();
            let oy = (l.hh[a] + l.hh[b] + clearance) - (l.y[a] - l.y[b]).abs();
            if ox > 0 && oy > 0 {
                t += f64::from(ox) * f64::from(oy);
            }
        }
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;
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
