//! Mirror symmetry about a shared axis (placement tier).
//!
//! Ported from `backend/constraints/src/placement_level/symmetry.rs`
//! (`SymmetryGroup` / `MatchingPair`).

use pnr_core::ids::{AxisId, Target};
use pnr_core::layout::Layout;
use crate::rule::Rule;

/// **Mirror symmetry.** The two halves of a differential structure must be mirror
/// images about a shared axis (`x_b = 2·axis − x_a` at equal `y`), or systematic
/// offset and even-order distortion appear. Self-symmetric devices (tail sources)
/// sit on the axis. Hierarchical symmetry aligns multiple groups to one master
/// axis. Re-snapped exactly after annealing, so it is an invariant.
///
/// - **Enforcement:** [`crate::Mode::Hard`] (legality; a soft form exists for the
///   Minimal tier).
/// - **Arity:** Device↔Device (about an [`AxisId`]).
/// - **Books:** ALS 1.1/1.1.4 (#1), 2.7.1 (#36), 2.8.1 (#37); PNR_ANALOG 00/1.3–1.7.
#[derive(Clone, Copy)]
pub struct Symmetry {
    pub a: Target,
    pub b: Target,
    pub axis: AxisId,
}

impl Rule for Symmetry {
    type On = Layout;
    /// Engine `SymPair`: squared deviation from the mirror equation,
    /// `ex = xa + xb − 2·axis` (residual about the axis), `ey = ya − yb`. Reads the
    /// axis position from [`Layout::axis_x`].
    fn cost(self, l: &Layout) -> f32 {
        let (ax, ay) = l.centre(self.a);
        let (bx, by) = l.centre(self.b);
        let axis = l.axis_x(self.axis);
        let ex = (ax + bx - 2 * axis) as f32;
        let ey = (ay - by) as f32;
        (ex * ex + ey * ey) * 1e-3
    }
    /// Mirror equation holds exactly (partners are grid-snapped after annealing).
    fn satisfied(self, l: &Layout) -> bool {
        let (ax, ay) = l.centre(self.a);
        let (bx, by) = l.centre(self.b);
        let axis = l.axis_x(self.axis);
        (ax + bx - 2 * axis) == 0 && (ay - by) == 0
    }

    fn touches(self, out: &mut Vec<u32>) {
        if let Target::Device(d) = self.a {
            out.push(u32::from(d.0));
        }
        if let Target::Device(d) = self.b {
            out.push(u32::from(d.0));
        }
    }

    fn retarget(self, cell_of: &[u16]) -> Self {
        Self { a: self.a.retarget(cell_of), b: self.b.retarget(cell_of), ..self }
    }

    // `residual` is deliberately left at the trait default (`0.0` / `1.0`). There is
    // nothing to normalise by: an exact equality has no budget, so "fraction of my own
    // budget overshot" is not a quantity this rule possesses. Dividing the mirror error
    // by some reference length would invent an allowance the constraint does not grant
    // and would make a one-grid-unit miss look asymptotically free — which is exactly the
    // PLAN §4a failure `project` below exists to avoid. `1.0` when violated is the honest
    // answer: violated is violated, and the repair is projection, not a smaller number.

    /// Re-establish the mirror equation **exactly**, on the grid.
    ///
    /// This is the rule a penalty can never close: [`satisfied`](Rule::satisfied)
    /// is an equality on integers, so annealing drives the residual small and
    /// stops, leaving it violated forever. Projection places the pair
    /// symmetrically about their shared axis, on one row, at minimum displacement
    /// from wherever the optimiser left them.
    ///
    /// Exactness comes from snapping the axis and the half-separation to the
    /// lattice *independently* (see [`Symmetry::mirror_about`]): both multiples
    /// of `grid` means each partner lands on-grid and their sum is exactly
    /// `2·axis`, so a later grid snap is a no-op rather than a corruption. That
    /// is precisely how the old "snap preserves symmetry" comment failed —
    /// nothing had established a symmetry for the snap to preserve.
    ///
    /// This projects a **lone** pair, which owns its axis and so can move it to
    /// its own midpoint for free. Pairs that share a stage axis must be projected
    /// together — see [`SymmetryGroup`]. Group targets are skipped: a group's
    /// centre is a derived bounding box, not an assignable coordinate.
    fn project(self, l: &mut Layout, grid: i32) {
        let Some((ia, ib)) = self.indices(l) else { return };
        let g = grid.max(1);
        // A lone pair owns its axis, so the cheapest projection puts the axis at
        // the pair's own midpoint: no device has to travel.
        let axis = snap_to((snap_to(l.x[ia], g) + snap_to(l.x[ib], g)) / 2, g);
        self.mirror_about(l, axis, g);
        if let Some(slot) = l.axis.get_mut(self.axis.0 as usize) {
            *slot = axis;
        }
    }
}

impl Symmetry {
    /// Device indices of the pair, when both targets are devices and in range.
    ///
    /// `ia == ib` is allowed: after group collapse both partners of a pair can
    /// live inside one merged macro, and the rule degenerates to "centre the
    /// merged macro on the axis" — `mirror_about` computes `half = 0` and sets
    /// `x = axis`, which is exactly the surviving stage-level constraint. Refusing
    /// the self-pair here would leave an unprojectable hard equality (PLAN §4a's
    /// parking failure).
    fn indices(self, l: &Layout) -> Option<(usize, usize)> {
        let (Target::Device(da), Target::Device(db)) = (self.a, self.b) else {
            return None;
        };
        let (ia, ib) = (da.0 as usize, db.0 as usize);
        (ia < l.x.len() && ib < l.x.len()).then_some((ia, ib))
    }

    /// Place the pair symmetrically about `axis`, on the `grid` lattice.
    ///
    /// Both the axis and the half-separation are snapped to the lattice
    /// *independently*, which is what keeps the result exact: `x = axis ∓ half`
    /// with `axis` and `half` both multiples of `g` leaves each partner on-grid
    /// and their sum exactly `2·axis`, so [`Rule::satisfied`] holds and a later
    /// snap is a no-op.
    fn mirror_about(self, l: &mut Layout, axis: i32, g: i32) {
        let Some((ia, ib)) = self.indices(l) else { return };
        let half = snap_to((snap_to(l.x[ib], g) - snap_to(l.x[ia], g)) / 2, g);
        l.x[ia] = axis - half;
        l.x[ib] = axis + half;
        // The mirror equation demands equal y: both partners to one row.
        let my = snap_to((l.y[ia] + l.y[ib]) / 2, g);
        l.y[ia] = my;
        l.y[ib] = my;
    }

    /// This pair's midpoint on the mirror axis, `nm`.
    fn midpoint_x(self, l: &Layout, g: i32) -> Option<i32> {
        let (ia, ib) = self.indices(l)?;
        Some((snap_to(l.x[ia], g) + snap_to(l.x[ib], g)) / 2)
    }
}

/// Mirror pairs that share **one** axis — a differential *stage*, not a pair.
///
/// A differential stage is symmetric as a whole: the input pair, its cascodes and
/// its load all mirror about the same line. Giving each pair a private axis lets
/// them drift onto different lines, which is a layout that satisfies every
/// individual `Symmetry` rule and is still not a symmetric stage — the systematic
/// offset those rules exist to prevent comes straight back.
///
/// The batch exists because the fix cannot be expressed per rule: choosing the
/// shared axis needs to see every pair at once. Projecting pair by pair, each
/// moving the axis to its own midpoint, would just leave the axis wherever the
/// last pair put it.
///
/// This is the hierarchical symmetry of ALS §2.7/2.8 and the ASF-B\*-tree line of
/// work: one master axis per symmetry group.
pub struct SymmetryGroup(pub Vec<Symmetry>);

impl crate::rule::RuleBatch<Layout> for SymmetryGroup {
    fn cost(&self, l: &Layout) -> f32 {
        self.0.iter().map(|r| r.cost(l)).sum()
    }
    fn violations(&self, l: &Layout) -> u32 {
        self.0.iter().filter(|r| !r.satisfied(l)).count() as u32
    }
    fn kind(&self) -> &'static str {
        // Reported as the rule it enforces, not the container.
        "Symmetry"
    }
    fn count(&self) -> usize {
        self.0.len()
    }
    fn worst_cost(&self, l: &Layout) -> f32 {
        self.0
            .iter()
            .filter(|r| !r.satisfied(l))
            .map(|r| r.cost(l))
            .fold(0.0, f32::max)
    }
    fn criticality(&self, l: &Layout) -> f32 {
        // Exact-equality rule: either the stage is symmetric or it is not.
        if self.violations(l) > 0 {
            1.0
        } else {
            0.0
        }
    }
    fn violating_ids(&self, l: &Layout, out: &mut Vec<u32>) {
        for r in self.0.iter().filter(|r| !r.satisfied(l)) {
            r.touches(out);
        }
    }

    fn retarget(&mut self, cell_of: &[u16]) {
        for r in &mut self.0 {
            *r = r.retarget(cell_of);
        }
    }

    /// Put every pair on one shared axis.
    ///
    /// The axis is the mean of the pairs' midpoints — the choice that minimises
    /// total device displacement, so the stage becomes symmetric without undoing
    /// the annealer's work.
    fn project(&self, l: &mut Layout, grid: i32) {
        let g = grid.max(1);
        let mids: Vec<i32> = self.0.iter().filter_map(|r| r.midpoint_x(l, g)).collect();
        if mids.is_empty() {
            return;
        }
        let mean = mids.iter().map(|&m| i64::from(m)).sum::<i64>() / mids.len() as i64;
        let axis = snap_to(mean as i32, g);
        for r in &self.0 {
            r.mirror_about(l, axis, g);
            if let Some(slot) = l.axis.get_mut(r.axis.0 as usize) {
                *slot = axis;
            }
        }
    }
}

/// Nearest multiple of `g`. Local so `analog` keeps its zero dependencies.
#[inline]
fn snap_to(v: i32, g: i32) -> i32 {
    let g = g.max(1);
    let (q, r) = (v / g, v % g);
    // Round half away from zero, matching the placer's `snap`.
    if r.abs() * 2 >= g {
        (q + r.signum()) * g
    } else {
        q * g
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_core::ids::DeviceId;

    fn layout(xa: i32, ya: i32, xb: i32, yb: i32) -> Layout {
        Layout {
            x: vec![xa, xb],
            y: vec![ya, yb],
            hw: vec![100; 2],
            hh: vec![100; 2],
            axis: vec![0; 2],
            groups: vec![],
            orient: vec![pnr_core::Orient::default(); 2],
            variant: vec![0; 2],
            branch: Vec::new(),
            power_uw: vec![0; 2],
            temp_mc: vec![0; 2],
        }
    }

    fn rule() -> Symmetry {
        Symmetry {
            a: Target::Device(DeviceId(0)),
            b: Target::Device(DeviceId(1)),
            axis: AxisId(0),
        }
    }

    #[test]
    fn projection_satisfies_a_rule_no_penalty_could_close() {
        // Asymmetric and off-row — exactly what annealing leaves behind.
        let mut l = layout(1_003, 40, 5_017, 260);
        assert!(!rule().satisfied(&l), "precondition: starts violated");
        rule().project(&mut l, 5);
        assert!(rule().satisfied(&l), "projection must land exactly: {:?}", (&l.x, &l.y, &l.axis));
        assert_eq!(rule().cost(&l), 0.0);
    }

    #[test]
    fn projection_leaves_everything_on_grid() {
        // The parity case: an odd number of grid steps between the partners.
        let g = 5;
        let mut l = layout(0, 0, 5 * g, 3 * g); // ka+kb = 5, odd
        rule().project(&mut l, g);
        assert!(rule().satisfied(&l));
        for v in l.x.iter().chain(l.y.iter()).chain(l.axis.iter().take(1)) {
            assert_eq!(v % g, 0, "off-grid coordinate {v} would be snapped away");
        }
    }

    #[test]
    fn projection_survives_a_later_grid_snap() {
        // The invariant the old code assumed but never established.
        let g = 5;
        let mut l = layout(1_003, 40, 5_017, 260);
        rule().project(&mut l, g);
        for v in l.x.iter_mut().chain(l.y.iter_mut()) {
            *v = snap_to(*v, g);
        }
        assert!(rule().satisfied(&l), "snap must not undo the projection");
    }

    #[test]
    fn a_stage_shares_one_axis() {
        use crate::rule::RuleBatch;
        // Two pairs of a differential stage, sitting on *different* mirror lines:
        // each pair is individually symmetric, the stage is not.
        let mut l = Layout {
            x: vec![1_000, 5_000, 20_000, 30_000],
            y: vec![0, 0, 0, 0],
            hw: vec![100; 4],
            hh: vec![100; 4],
            axis: vec![0; 4],
            groups: vec![],
            orient: vec![pnr_core::Orient::default(); 4],
            variant: vec![0; 4],
            branch: Vec::new(),
            power_uw: vec![0; 4],
            temp_mc: vec![0; 4],
        };
        let shared = AxisId(0);
        let g = SymmetryGroup(vec![
            Symmetry { a: Target::Device(DeviceId(0)), b: Target::Device(DeviceId(1)), axis: shared },
            Symmetry { a: Target::Device(DeviceId(2)), b: Target::Device(DeviceId(3)), axis: shared },
        ]);
        // Pair midpoints are 3000 and 25000 — two different mirror lines.
        g.project(&mut l, 5);
        assert_eq!(g.violations(&l), 0, "every pair must end up symmetric");

        // ...and about the SAME line, which is the whole point.
        let axis = l.axis[0];
        assert_eq!(l.x[0] + l.x[1], 2 * axis, "pair 0 straddles the shared axis");
        assert_eq!(l.x[2] + l.x[3], 2 * axis, "pair 1 straddles the same axis");
        // Mean of the two midpoints, so neither pair is dragged further than needed.
        assert_eq!(axis, 14_000);
    }

    #[test]
    fn shared_axis_preserves_each_pair_separation() {
        use crate::rule::RuleBatch;
        let mut l = Layout {
            x: vec![0, 4_000, 20_000, 30_000],
            y: vec![0, 0, 0, 0],
            hw: vec![100; 4],
            hh: vec![100; 4],
            axis: vec![0; 4],
            groups: vec![],
            orient: vec![pnr_core::Orient::default(); 4],
            variant: vec![0; 4],
            branch: Vec::new(),
            power_uw: vec![0; 4],
            temp_mc: vec![0; 4],
        };
        let shared = AxisId(0);
        let grp = SymmetryGroup(vec![
            Symmetry { a: Target::Device(DeviceId(0)), b: Target::Device(DeviceId(1)), axis: shared },
            Symmetry { a: Target::Device(DeviceId(2)), b: Target::Device(DeviceId(3)), axis: shared },
        ]);
        grp.project(&mut l, 5);
        // Devices move onto the shared axis but keep their own spacing: the
        // projection re-centres, it does not resize the pairs.
        assert_eq!(l.x[1] - l.x[0], 4_000);
        assert_eq!(l.x[3] - l.x[2], 10_000);
        assert_eq!(grp.violations(&l), 0);
    }

    #[test]
    fn retarget_identity_map_is_a_no_op() {
        // The regression anchor for a netlist with no matched groups: cell i is
        // device i, so retargeting must change nothing on any rule kind.
        let identity = [0u16, 1];
        let r = rule().retarget(&identity);
        assert_eq!((r.a, r.b, r.axis), (rule().a, rule().b, rule().axis));
        let p = crate::placement::MatchingPair {
            a: Target::Device(DeviceId(0)),
            b: Target::Device(DeviceId(1)),
            max_dvth_mv10: 50,
            w_ratio: (1, 1),
            avt_uv_um: 3_000,
            matching: crate::placement::Matching::Cross,
        }
        .retarget(&identity);
        assert_eq!((p.a, p.b), (Target::Device(DeviceId(0)), Target::Device(DeviceId(1))));
    }

    #[test]
    fn retarget_maps_both_partners_through_cell_of() {
        // Devices 0 and 1 collapsed into cell 0, device 2 became cell 1.
        let cell_of = [0u16, 0, 1];
        let r = Symmetry {
            a: Target::Device(DeviceId(1)),
            b: Target::Device(DeviceId(2)),
            axis: AxisId(0),
        }
        .retarget(&cell_of);
        assert_eq!(r.a, Target::Device(DeviceId(0)));
        assert_eq!(r.b, Target::Device(DeviceId(1)));

        use crate::rule::RuleBatch;
        let mut grp = crate::placement::cc::CentroidGroup {
            a_side: vec![DeviceId(0), DeviceId(2)],
            b_side: vec![DeviceId(1)],
        };
        grp.retarget(&cell_of);
        assert_eq!(grp.a_side, vec![DeviceId(0), DeviceId(1)]);
        assert_eq!(grp.b_side, vec![DeviceId(0)]);
    }

    #[test]
    fn collapsed_self_pair_projects_to_the_axis_on_grid() {
        // After group collapse both partners live inside one merged macro: the
        // pair degenerates to a == b and the surviving constraint is "centre the
        // merged macro on the stage axis" — projection must land exactly and stay
        // on the lattice, or the hard equality becomes unprojectable (PLAN §4a).
        let g = 5;
        let mut l = layout(1_003, 40, 9_999, 260); // device 1 is unrelated ballast
        let self_pair = Symmetry {
            a: Target::Device(DeviceId(0)),
            b: Target::Device(DeviceId(0)),
            axis: AxisId(0),
        };
        self_pair.project(&mut l, g);
        assert!(self_pair.satisfied(&l), "self-pair must land exactly on the axis");
        assert_eq!(l.x[0], l.axis[0], "x = axis is the degenerate mirror equation");
        assert_eq!(l.x[0] % g, 0, "projection must stay on the lattice");
        assert_eq!(l.x[1], 9_999, "the unrelated device must not move");
    }

    #[test]
    fn projection_is_idempotent_and_minimal() {
        let mut l = layout(1_000, 0, 5_000, 0);
        rule().project(&mut l, 5);
        let once = (l.x.clone(), l.y.clone(), l.axis.clone());
        rule().project(&mut l, 5);
        assert_eq!((l.x.clone(), l.y.clone(), l.axis.clone()), once, "already-projected input must not drift");
        // An already-symmetric pair should not have moved at all.
        assert_eq!(l.x, vec![1_000, 5_000]);
    }
}
