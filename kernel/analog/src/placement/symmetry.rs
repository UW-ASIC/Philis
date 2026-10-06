//! Mirror symmetry about a shared axis (placement tier).

use pnr_core::ids::{AxisId, Target};
use pnr_core::layout::Layout;
use crate::rule::{Rule, RuleBatch};

/// Direction of a symmetry axis (C14): `V` mirrors in x about a vertical line.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AxisDir {
    /// Vertical axis: partners mirror in x, share y.
    #[default]
    V,
    /// Horizontal axis: partners mirror in y, share x.
    H,
}

/// How distinct partners are drawn (PLC-21): `Perfect` = same orient (PLC-03);
/// `Mirror` = `b` is `a` reflected about the vertical axis (`orient[b] = orient[a].then(Mx180)`),
/// legal only when every unit's φ has no x component ([`crate::matching::moments::mirror_allowed`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SymMode {
    /// Partners drawn with the same orientation.
    #[default]
    Perfect,
    /// `b` drawn as `a` reflected about the vertical axis.
    Mirror,
}

/// Partners mirror about `axis`: `x_a + x_b = 2·axis`, `y_a = y_b`, and
/// distinct partners also share `hw`, `hh`, `variant` and `orient` up to `mode`. The mirror
/// equation is an exact integer equality, so it is enforced by
/// [`Rule::project`], not by weight; `residual` is the mirror error in µm.
///
/// `a == b` is legal: a pair collapsed into one cell degenerates to "centre on
/// the axis". Group targets are scored but never projected.
#[derive(Clone, Copy, Debug)]
pub struct Symmetry {
    /// Left partner.
    pub a: Target,
    /// Right partner.
    pub b: Target,
    /// Slot in [`Layout::axis`] holding the mirror line's x, nm.
    pub axis: AxisId,
    /// How the partners' orientations relate.
    pub mode: SymMode,
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
        super::push_devices(out, &[self.a, self.b]);
    }

    fn retarget(self, cell_of: &[u16]) -> Self {
        Self { a: self.a.retarget(cell_of), b: self.b.retarget(cell_of), ..self }
    }

    /// `Some` only when both sides are devices.
    fn mirror_pair(self) -> Option<(u32, u32, u16)> {
        match (self.a, self.b) {
            (Target::Device(a), Target::Device(b)) => Some((u32::from(a.0), u32::from(b.0), self.axis.0)),
            _ => None,
        }
    }

    /// `Some` only for two distinct devices.
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

    /// Distinct partners drawn alike (PLC-03), the orient related by `mode`
    /// (PLC-21). A column too short to hold either index reads as equal.
    fn same_shape(self, l: &Layout) -> bool {
        let Some((ia, ib)) = self.indices(l) else { return true };
        fn eq<T: PartialEq>(v: &[T], a: usize, b: usize) -> bool {
            v.get(a).zip(v.get(b)).is_none_or(|(a, b)| a == b)
        }
        let orient_ok = l.orient.get(ia).zip(l.orient.get(ib)).is_none_or(|(&oa, &ob)| {
            ob == match self.mode {
                SymMode::Perfect => oa,
                SymMode::Mirror => oa.then(pnr_core::Orient::Mx180),
            }
        });
        ia == ib || (l.hw[ia] == l.hw[ib] && l.hh[ia] == l.hh[ib] && eq(&l.variant, ia, ib) && orient_ok)
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

    /// Midpoint of the partners' on-grid x, truncated toward zero; `None`
    /// unless both are in-range devices.
    fn midpoint_x(self, l: &Layout, g: i32) -> Option<i32> {
        let (ia, ib) = self.indices(l)?;
        Some((snap_to(l.x[ia], g) + snap_to(l.x[ib], g)) / 2)
    }
}

/// Mirror pairs of one differential stage, sharing **one** axis. Per-pair
/// projection would let each pair drift onto its own line; choosing the shared
/// axis needs every pair at once. Scoring delegates to the `Vec<Symmetry>`
/// batch; only projection and the mirror-mode hooks differ.
#[derive(Clone, Debug)]
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
    // ponytail: on SymmetryGroup only; a bare `Vec<Symmetry>` is never a placement batch.
    fn mirrored_pairs(&self, out: &mut Vec<(u32, u32)>) {
        out.extend(self.0.iter().filter(|r| r.mode == SymMode::Mirror).filter_map(|r| r.matched_pair()));
    }
    fn demote_mirrors(&mut self, keep: &dyn Fn(u32, u32) -> bool) {
        for r in &mut self.0 {
            if r.mode == SymMode::Mirror && r.matched_pair().is_none_or(|(a, b)| !keep(a, b)) {
                r.mode = SymMode::Perfect;
            }
        }
    }
    fn residual(&self, l: &Layout) -> f64 {
        self.0.residual(l)
    }

    /// Put every pair on one axis at the mean of their midpoints (minimum total
    /// displacement), and write that axis to every pair's [`Layout::axis`]
    /// slot. A group without one in-range device pair is left untouched.
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

/// Nearest multiple of `g` (floored at 1), half away from zero (matches the
/// placer's `snap`).
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
            mode: SymMode::Perfect,
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
            Symmetry { a: Target::Device(DeviceId(0)), b: Target::Device(DeviceId(1)), axis: shared, mode: SymMode::Perfect },
            Symmetry { a: Target::Device(DeviceId(2)), b: Target::Device(DeviceId(3)), axis: shared, mode: SymMode::Perfect },
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
            Symmetry { a: Target::Device(DeviceId(0)), b: Target::Device(DeviceId(1)), axis: shared, mode: SymMode::Perfect },
            Symmetry { a: Target::Device(DeviceId(2)), b: Target::Device(DeviceId(3)), axis: shared, mode: SymMode::Perfect },
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
    fn mirror_pair_satisfies_only_with_mx180_partner() {
        use pnr_core::Orient::{Mx180, R0, R90};
        let mut l = layout(-500, 0, 500, 0);
        let mirror = Symmetry { mode: SymMode::Mirror, ..rule() };
        let with = |l: &mut Layout, a, b| l.orient = vec![a, b];
        with(&mut l, R0, R0);
        assert!(!mirror.satisfied(&l));
        with(&mut l, R0, Mx180);
        assert!(mirror.satisfied(&l));
        with(&mut l, R90, R90.then(Mx180));
        assert!(mirror.satisfied(&l));
        with(&mut l, R0, Mx180);
        assert!(!rule().satisfied(&l), "Perfect still wants equal orients");
    }

    #[test]
    fn demote_mirrors_keeps_only_allowed_pairs() {
        let pair = |a: u16, b: u16| Symmetry { a: Target::Device(DeviceId(a)), b: Target::Device(DeviceId(b)), axis: AxisId(0), mode: SymMode::Mirror };
        let mut g = SymmetryGroup(vec![pair(0, 1), pair(2, 3)]);
        g.demote_mirrors(&|a, _| a == 2);
        let mut out = Vec::new();
        g.mirrored_pairs(&mut out);
        assert_eq!(out, [(2, 3)]);
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
            mode: SymMode::Perfect,
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
            mode: SymMode::Perfect,
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

#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use pnr_core::ids::{DeviceId, GroupId};
    use pnr_core::Orient;

    fn d(i: u16) -> Target {
        Target::Device(DeviceId(i))
    }

    fn layout(xs: &[i32], ys: &[i32]) -> Layout {
        let n = xs.len();
        Layout {
            x: xs.to_vec(),
            y: ys.to_vec(),
            hw: vec![100; n],
            hh: vec![100; n],
            axis: vec![0],
            groups: vec![vec![DeviceId(0)]],
            orient: vec![Orient::default(); n],
            variant: vec![0; n],
            branch: Vec::new(),
            power_uw: vec![0; n],
            temp_mc: vec![0; n],
            units: Default::default(),
        }
    }

    fn sym(a: Target, b: Target) -> Symmetry {
        Symmetry { a, b, axis: AxisId(0), mode: SymMode::Perfect }
    }

    #[test]
    fn snap_rounds_half_away_from_zero() {
        for (v, g, want) in [(0, 5, 0), (7, 5, 5), (8, 5, 10), (-7, 5, -5), (-8, 5, -10), (2, 4, 4), (-2, 4, -4), (1, 4, 0), (13, 1, 13), (13, 0, 13), (13, -3, 13)] {
            assert_eq!(snap_to(v, g), want, "snap_to({v}, {g})");
        }
    }

    #[test]
    fn defaults_are_vertical_and_perfect() {
        assert_eq!(AxisDir::default(), AxisDir::V);
        assert_eq!(SymMode::default(), SymMode::Perfect);
    }

    #[test]
    fn cost_is_the_squared_error_over_l_ref() {
        // ex = 100 + 300 − 0 = 400, ey = 0 − 200 = −200: |e| = 600 nm.
        let l = layout(&[100, 300], &[0, 200]);
        let e = 600.0 / l.l_ref();
        assert!((sym(d(0), d(1)).cost(&l) - e * e).abs() < 1e-6);
        assert!((sym(d(0), d(1)).residual(&l) - 0.6).abs() < 1e-6, "µm");
    }

    #[test]
    fn unequal_extents_or_orients_break_a_perfect_pair() {
        let mut l = layout(&[-500, 500], &[0, 0]);
        assert!(sym(d(0), d(1)).satisfied(&l));
        l.hw[1] = 200;
        assert!(!sym(d(0), d(1)).satisfied(&l), "hw");
        l.hw[1] = 100;
        l.hh[0] = 50;
        assert!(!sym(d(0), d(1)).satisfied(&l), "hh");
        l.hh[0] = 100;
        l.orient[1] = Orient::R180;
        assert!(!sym(d(0), d(1)).satisfied(&l), "orient");
        // The mirror equation still holds: the error terms are zero.
        assert_eq!(sym(d(0), d(1)).cost(&l), 0.0);
    }

    #[test]
    fn a_self_pair_ignores_shape_and_wants_centre_on_axis() {
        let mut l = layout(&[0, 9], &[0, 0]);
        l.orient[0] = Orient::R90;
        assert!(sym(d(0), d(0)).satisfied(&l));
        l.x[0] = 10;
        assert!(!sym(d(0), d(0)).satisfied(&l));
    }

    #[test]
    fn group_targets_score_but_never_project() {
        let mut l = layout(&[-300, 300], &[0, 0]);
        let r = sym(Target::Group(GroupId(0)), d(1));
        assert!(r.satisfied(&l), "group 0 = device 0's box; shape is not compared");
        l.x[1] = 1_000;
        assert!(!r.satisfied(&l));
        let before = (l.x.clone(), l.y.clone(), l.axis.clone());
        r.project(&mut l, 5);
        assert_eq!((l.x.clone(), l.y.clone(), l.axis.clone()), before);
        assert_eq!(r.mirror_pair(), None);
        assert_eq!(r.matched_pair(), None);
    }

    #[test]
    fn pair_ids_distinguish_self_pairs() {
        let r = Symmetry { axis: AxisId(3), ..sym(d(2), d(5)) };
        assert_eq!(r.mirror_pair(), Some((2, 5, 3)));
        assert_eq!(r.matched_pair(), Some((2, 5)));
        let s = sym(d(4), d(4));
        assert_eq!(s.mirror_pair(), Some((4, 4, 0)));
        assert_eq!(s.matched_pair(), None, "nothing to draw alike");
    }

    #[test]
    fn projection_skips_out_of_range_devices_and_axis_slots() {
        let mut l = layout(&[1_003, 5_017], &[40, 260]);
        let before = (l.x.clone(), l.y.clone(), l.axis.clone());
        sym(d(0), d(9)).project(&mut l, 5);
        assert_eq!((l.x.clone(), l.y.clone(), l.axis.clone()), before, "device 9 does not exist");
        // An axis id past `Layout::axis` still lands the pair, without growing the table.
        let r = Symmetry { axis: AxisId(4), ..sym(d(0), d(1)) };
        r.project(&mut l, 5);
        assert_eq!(l.axis.len(), 1);
        assert_eq!(l.y[0], l.y[1]);
        assert_eq!((l.x[0] + l.x[1]) % 2, 0);
    }

    #[test]
    fn a_zero_grid_reads_as_one() {
        let mut l = layout(&[0, 7], &[0, 3]);
        sym(d(0), d(1)).project(&mut l, 0);
        assert!(sym(d(0), d(1)).satisfied(&l), "{:?}", (&l.x, &l.y, &l.axis));
    }

    #[test]
    fn group_project_on_no_device_pairs_is_a_no_op() {
        let mut l = layout(&[1, 2], &[3, 4]);
        let before = (l.x.clone(), l.y.clone(), l.axis.clone());
        SymmetryGroup(vec![]).project(&mut l, 5);
        SymmetryGroup(vec![sym(Target::Group(GroupId(0)), d(1))]).project(&mut l, 5);
        assert_eq!((l.x.clone(), l.y.clone(), l.axis.clone()), before);
    }

    #[test]
    fn group_criticality_is_all_or_nothing() {
        let g = SymmetryGroup(vec![sym(d(0), d(1))]);
        assert_eq!(g.criticality(&layout(&[-5, 5], &[0, 0])), 0.0);
        assert_eq!(g.criticality(&layout(&[-5, 6], &[0, 0])), 1.0);
        assert_eq!((g.count(), g.kind()), (1, "Symmetry"));
    }

    #[test]
    fn group_reports_each_violated_pairs_residual() {
        // Pair (0, 1) is 1 µm off its axis; pair (2, 3) is exact.
        let l = layout(&[-500, 1_500, -10, 10], &[0, 0, 0, 0]);
        let g = SymmetryGroup(vec![sym(d(0), d(1)), sym(d(2), d(3))]);
        let mut out = Vec::new();
        g.violating_residuals(&l, &mut out);
        assert_eq!(out, [(0, 1.0), (1, 1.0)], "the per-rule residual, not NaN");
        let mut ids = Vec::new();
        g.violating_ids(&l, &mut ids);
        assert_eq!(ids, [0, 1]);
    }

    #[test]
    fn group_delegates_pairs_and_retarget() {
        let mut g = SymmetryGroup(vec![sym(d(0), d(1)), Symmetry { mode: SymMode::Mirror, ..sym(d(2), d(3)) }]);
        let (mut mp, mut m, mut mir, mut t) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        g.mirror_pairs(&mut mp);
        g.matched_pairs(&mut m);
        g.mirrored_pairs(&mut mir);
        g.touched(&mut t);
        assert_eq!(mp, [(0, 1, 0), (2, 3, 0)]);
        assert_eq!(m, [(0, 1), (2, 3)]);
        assert_eq!(mir, [(2, 3)]);
        assert_eq!(t, [0, 1, 2, 3]);
        g.retarget(&[0, 0, 1, 1]);
        assert_eq!((g.0[0].a, g.0[0].b, g.0[1].a, g.0[1].b), (d(0), d(0), d(1), d(1)));
        // Collapsed into self-pairs: a Mirror self-pair has no partner to reflect.
        g.demote_mirrors(&|_, _| true);
        assert_eq!(g.0[1].mode, SymMode::Perfect);
    }
}
