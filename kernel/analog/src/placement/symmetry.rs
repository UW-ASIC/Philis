//! Mirror symmetry about a shared axis (placement tier).

use pnr_core::ids::{AxisId, Target};
use pnr_core::layout::Layout;
use crate::rule::{Rule, RuleBatch};

/// Direction of a symmetry axis (C14): `V` mirrors in x about a vertical line.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AxisDir {
    #[default]
    V,
    H,
}

/// Partners mirror about `axis`: `x_a + x_b = 2·axis`, `y_a = y_b`, and
/// distinct partners also share `hw`, `hh`, `variant`, `orient`. The mirror
/// equation is an exact integer equality, so it is enforced by
/// [`Rule::project`], not by weight; `residual` is the mirror error in µm.
#[derive(Clone, Copy)]
pub struct Symmetry {
    pub a: Target,
    pub b: Target,
    pub axis: AxisId,
}

impl Rule for Symmetry {
    type On = Layout;
    /// `((|ex| + |ey|) / L_ref)²` with `ex = xa + xb − 2·axis`, `ey = ya − yb`,
    /// `L_ref` = [`Layout::l_ref`] (PLC-18: dimensionless).
    fn cost(self, l: &Layout) -> f32 {
        let (ex, ey) = self.error(l);
        let e = (ex.abs() + ey.abs()) as f32 / l.l_ref();
        e * e
    }
    fn satisfied(self, l: &Layout) -> bool {
        self.error(l) == (0, 0) && self.same_shape(l)
    }

    /// Mirror error `|ex| + |ey|` in µm: a hard margin that is a length, not a count.
    fn residual(self, l: &Layout) -> f32 {
        let (ex, ey) = self.error(l);
        (ex.abs() + ey.abs()) as f32 / 1000.0
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

    fn mirror_pair(self) -> Option<(u32, u32, u16)> {
        match (self.a, self.b) {
            (Target::Device(a), Target::Device(b)) => Some((u32::from(a.0), u32::from(b.0), self.axis.0)),
            _ => None,
        }
    }

    fn matched_pair(self) -> Option<(u32, u32)> {
        match (self.a, self.b) {
            (Target::Device(a), Target::Device(b)) if a != b => Some((u32::from(a.0), u32::from(b.0))),
            _ => None,
        }
    }

    /// Mirror a **lone** pair about its own midpoint, on-grid (no device has
    /// to travel far). Pairs sharing a stage axis go through [`SymmetryGroup`].
    /// Group targets are skipped: a group centre is not assignable.
    fn project(self, l: &mut Layout, grid: i32) {
        let g = grid.max(1);
        let Some(mid) = self.midpoint_x(l, g) else { return };
        let axis = snap_to(mid, g);
        self.mirror_about(l, axis, g);
        if let Some(slot) = l.axis.get_mut(self.axis.0 as usize) {
            *slot = axis;
        }
    }
}

impl Symmetry {
    /// Mirror-equation error `(xa + xb − 2·axis, ya − yb)`.
    fn error(self, l: &Layout) -> (i32, i32) {
        let (ax, ay) = l.centre(self.a);
        let (bx, by) = l.centre(self.b);
        (ax + bx - 2 * l.axis_x(self.axis), ay - by)
    }

    /// Perfect symmetry (PLC-03; Mirror mode is PLC-21): distinct partners drawn
    /// alike. A column too short to hold either index reads as equal.
    fn same_shape(self, l: &Layout) -> bool {
        let Some((ia, ib)) = self.indices(l) else { return true };
        fn eq<T: PartialEq>(v: &[T], a: usize, b: usize) -> bool {
            v.get(a).zip(v.get(b)).is_none_or(|(a, b)| a == b)
        }
        ia == ib || (l.hw[ia] == l.hw[ib] && l.hh[ia] == l.hh[ib] && eq(&l.variant, ia, ib) && eq(&l.orient, ia, ib))
    }

    /// Device indices when both targets are in-range devices. `ia == ib` is
    /// allowed: a pair collapsed into one macro degenerates to "centre on axis".
    fn indices(self, l: &Layout) -> Option<(usize, usize)> {
        let (Target::Device(da), Target::Device(db)) = (self.a, self.b) else {
            return None;
        };
        let (ia, ib) = (da.0 as usize, db.0 as usize);
        (ia < l.x.len() && ib < l.x.len()).then_some((ia, ib))
    }

    /// Place the pair at `axis ∓ half` on one row. Snapping `axis` and `half`
    /// independently keeps both partners on-grid and their sum exactly `2·axis`.
    fn mirror_about(self, l: &mut Layout, axis: i32, g: i32) {
        let Some((ia, ib)) = self.indices(l) else { return };
        let d = snap_to(l.x[ib], g) - snap_to(l.x[ia], g);
        // Ceiling of |d|/2 on the grid, sign kept: a projected pair never moves closer.
        let half = d.signum() * ((d.abs() + 2 * g - 1) / (2 * g)) * g;
        l.x[ia] = axis - half;
        l.x[ib] = axis + half;
        let my = snap_to((l.y[ia] + l.y[ib]) / 2, g);
        l.y[ia] = my;
        l.y[ib] = my;
    }

    fn midpoint_x(self, l: &Layout, g: i32) -> Option<i32> {
        let (ia, ib) = self.indices(l)?;
        Some((snap_to(l.x[ia], g) + snap_to(l.x[ib], g)) / 2)
    }
}

/// Mirror pairs of one differential stage, sharing **one** axis. Per-pair
/// projection would let each pair drift onto its own line; choosing the shared
/// axis needs every pair at once.
pub struct SymmetryGroup(pub Vec<Symmetry>);

impl RuleBatch<Layout> for SymmetryGroup {
    fn cost(&self, l: &Layout) -> f32 {
        self.0.cost(l)
    }
    fn violations(&self, l: &Layout) -> u32 {
        self.0.violations(l)
    }
    fn kind(&self) -> &'static str {
        "Symmetry"
    }
    fn count(&self) -> usize {
        self.0.len()
    }
    fn worst_cost(&self, l: &Layout) -> f32 {
        self.0.worst_cost(l)
    }
    /// Exact equality: fully critical or not at all.
    fn criticality(&self, l: &Layout) -> f32 {
        f32::from(self.violations(l) > 0)
    }
    fn violating_ids(&self, l: &Layout, out: &mut Vec<u32>) {
        self.0.violating_ids(l, out);
    }
    fn touched(&self, out: &mut Vec<u32>) {
        self.0.touched(out);
    }
    fn retarget(&mut self, cell_of: &[u16]) {
        self.0.retarget(cell_of);
    }
    fn mirror_pairs(&self, out: &mut Vec<(u32, u32, u16)>) {
        self.0.mirror_pairs(out);
    }
    fn matched_pairs(&self, out: &mut Vec<(u32, u32)>) {
        self.0.matched_pairs(out);
    }
    fn residual(&self, l: &Layout) -> f64 {
        self.0.residual(l)
    }

    /// Put every pair on one axis at the mean of their midpoints (minimum total
    /// displacement).
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

/// Nearest multiple of `g`, half away from zero (matches the placer's `snap`).
#[inline]
fn snap_to(v: i32, g: i32) -> i32 {
    let g = g.max(1);
    let (q, r) = (v / g, v % g);
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
            units: Default::default(),
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
    fn symmetry_residual_is_the_mirror_error_in_um() {
        let l = layout(-1_000, 0, 1_500, 0);
        assert_eq!(SymmetryGroup(vec![rule()]).residual(&l), 0.5);
    }

    #[test]
    fn odd_parity_projection_never_shrinks_the_pair() {
        let mut l = layout(0, 0, 25, 0);
        rule().project(&mut l, 5);
        assert_eq!(l.x[1] - l.x[0], 30);
        assert!(rule().satisfied(&l));
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
            units: Default::default(),
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
            units: Default::default(),
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
    fn unequal_variants_are_not_symmetric() {
        let mut l = layout(-500, 0, 500, 0);
        assert!(rule().satisfied(&l));
        l.variant = vec![0, 1];
        assert!(!rule().satisfied(&l));
    }

    #[test]
    fn retarget_identity_map_is_a_no_op() {
        // The regression anchor for a netlist with no matched groups: cell i is
        // device i, so retargeting must change nothing on any rule kind.
        let identity = [0u16, 1];
        let r = rule().retarget(&identity);
        assert_eq!((r.a, r.b, r.axis), (rule().a, rule().b, rule().axis));
        let mut p = crate::placement::matched_set::pair(0, 1);
        crate::rule::RuleBatch::retarget(&mut p, &identity);
        assert_eq!(p.members, vec![DeviceId(0), DeviceId(1)]);
        assert_eq!(p.cell_of, identity.to_vec());
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
        let mut set = crate::placement::matched_set::pair(0, 2);
        set.retarget(&cell_of);
        // Members stay schematic devices (units are owned by those); the map
        // is kept for the cell-centre fallback.
        assert_eq!(set.members, vec![DeviceId(0), DeviceId(2)]);
        assert_eq!(set.cell_of, cell_of.to_vec());
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
