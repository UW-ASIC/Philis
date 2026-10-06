//! Matched-device orientation across cells (placement tier, plan-02 MAT-05):
//! channel axes parallel (Hastings rule 7, "designers should always make sure
//! that the orientation of matched transistors are equal", hastings.txt
//! L42490–42498, PDF p.714) and equal mean S→D current Φ (eq. 13.61). Read
//! from the members' placed units, so a partner drawn in its own cell and
//! turned by dp is checked like a merged one.

use pnr_core::ids::DeviceId;
use pnr_core::layout::Layout;

use crate::matching::moments::{phi_equal, sums, Axis, Pt};

/// Which orientation condition a set must meet.
///
/// `PhiZero` (Φ = 0 for a set that must cancel its own current direction) is
/// MAT-12's, its only emitter.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OrientCheck {
    /// Every member's current runs on one axis: none `Mixed`, not both H and
    /// V. Units with φ = (0, 0) (capacitors) say nothing and never violate.
    Axis,
    /// Every member's mean φ equals member 0's ([`phi_equal`]). Residual
    /// `max_d |ΔΦx| + |ΔΦy|`, dimensionless, ≤ 4.
    Phi,
}

/// One orientation condition over a matched set of schematic devices. A
/// member without placed units is **unknown** (no drawing, no direction).
#[derive(Clone, Debug)]
pub struct OrientationSet {
    /// Schematic devices; member 0 is the reference for [`OrientCheck::Phi`].
    pub members: Vec<DeviceId>,
    /// The condition the members must meet.
    pub check: OrientCheck,
    /// `device → cell`, set by `retarget`; empty = the ids name cells.
    pub cell_of: Vec<u16>,
}

impl OrientationSet {
    /// Cell holding `d`: through `cell_of`, else `d` itself names the cell.
    fn cell(&self, d: DeviceId) -> u32 {
        u32::from(self.cell_of.get(d.0 as usize).copied().unwrap_or(d.0))
    }

    /// `0.0` = satisfied, `> 0` = violated, `None` = unknown (a member
    /// without placed units). Fewer than two members is vacuously `0.0`.
    fn residual_of(&self, l: &Layout) -> Option<f32> {
        let Some((&d0, rest)) = self.members.split_first() else { return Some(0.0) };
        if rest.is_empty() {
            return Some(0.0);
        }
        let of = |d: DeviceId| Some(sums(l.units.of_device(l, d).map(Pt::from))).filter(|m| m.n > 0);
        // Every member is reduced even after a violation: one without units
        // makes the whole set unknown.
        let s0 = of(d0);
        let mut known = s0.is_some();
        let s0 = s0.unwrap_or_default();
        let (mut mixed, mut h, mut v) = (s0.axis == Axis::Mixed, s0.axis == Axis::H, s0.axis == Axis::V);
        let p0 = s0.phi();
        let mut worst_phi = 0.0f64;
        for &d in rest {
            let Some(m) = of(d) else {
                known = false;
                continue;
            };
            (mixed, h, v) = (mixed | (m.axis == Axis::Mixed), h | (m.axis == Axis::H), v | (m.axis == Axis::V));
            if !phi_equal(&s0, &m) {
                let p = m.phi();
                worst_phi = worst_phi.max((p0.0 - p.0).abs() + (p0.1 - p.1).abs());
            }
        }
        known.then(|| match self.check {
            OrientCheck::Axis => f32::from(u8::from(mixed | (h & v))),
            OrientCheck::Phi => worst_phi as f32,
        })
    }
}

impl crate::rule::RuleBatch<Layout> for OrientationSet {
    fn cost(&self, l: &Layout) -> f32 {
        self.residual_of(l).unwrap_or(0.0)
    }
    fn violations(&self, l: &Layout) -> u32 {
        u32::from(self.residual_of(l).is_some_and(|r| r > 0.0))
    }
    fn residual(&self, l: &Layout) -> f64 {
        f64::from(self.residual_of(l).unwrap_or(0.0))
    }
    fn unknown(&self, l: &Layout) -> u32 {
        u32::from(self.residual_of(l).is_none())
    }
    fn kind(&self) -> &'static str {
        "Orientation"
    }
    fn count(&self) -> usize {
        1
    }
    /// Members stay schematic devices (units are owned by those); the map
    /// serves the repair ids.
    fn retarget(&mut self, cell_of: &[u16]) {
        self.cell_of = cell_of.to_vec();
    }
    fn touched(&self, out: &mut Vec<u32>) {
        out.extend(self.members.iter().map(|&d| self.cell(d)));
    }
    fn matched_pairs(&self, out: &mut Vec<(u32, u32)>) {
        let Some((&m0, rest)) = self.members.split_first() else { return };
        let c0 = self.cell(m0);
        out.extend(rest.iter().map(|&m| (c0, self.cell(m))));
    }
    /// Every member's cell when the set is violated: the condition is
    /// set-wide, so no single member is to blame.
    fn violating_ids(&self, l: &Layout, out: &mut Vec<u32>) {
        if self.violations(l) > 0 {
            self.touched(out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matching::moments::mirror_allowed;
    use crate::rule::RuleBatch;
    use pnr_core::{Orient, Rect, Unit, UnitLib};
    use std::sync::Arc;

    /// Devices 0 and 1, each alone in its own 100 × 100 cell drawing `phis`
    /// (one unit per entry); cell 1 turned `o1`.
    fn two_cells(phis: &[(i8, i8)], o1: Orient) -> Layout {
        let units: Vec<Unit> = phis.iter().map(|&phi| Unit { owner: 0, x: 50, y: 50, weight: 10, phi, sa: 0, sb: 0 }).collect();
        let alts = [(Rect { x: 0, y: 0, w: 100, h: 100 }, &units[..])];
        let lib = UnitLib::build(vec![0, 1], &[vec![DeviceId(0)], vec![DeviceId(1)]], [&alts[..], &alts[..]].into_iter());
        Layout {
            x: vec![0, 10_000],
            y: vec![0, 0],
            hw: vec![50; 2],
            hh: vec![50; 2],
            axis: vec![0; 2],
            groups: vec![],
            orient: vec![Orient::R0, o1],
            variant: vec![0; 2],
            branch: Vec::new(),
            power_uw: vec![0; 2],
            temp_mc: vec![0; 2],
            units: Arc::new(lib),
        }
    }

    fn set(check: OrientCheck) -> OrientationSet {
        OrientationSet { members: vec![DeviceId(0), DeviceId(1)], check, cell_of: Vec::new() }
    }

    #[test]
    fn a_quarter_turned_partner_is_illegal() {
        let s = set(OrientCheck::Axis);
        assert_eq!(s.violations(&two_cells(&[(1, 0)], Orient::R90)), 1);
        assert_eq!(s.violations(&two_cells(&[(1, 0)], Orient::R0)), 0);
    }

    #[test]
    fn a_mirrored_odd_finger_partner_breaks_phi() {
        let s = set(OrientCheck::Phi);
        let l = two_cells(&[(1, 0), (-1, 0), (1, 0)], Orient::Mx180);
        assert_eq!(s.violations(&l), 1);
        assert!((s.residual(&l) - 2.0 / 3.0).abs() < 1e-6, "{}", s.residual(&l));
        assert_eq!(set(OrientCheck::Axis).violations(&l), 0, "mirrored, still parallel");

        let l = two_cells(&[(1, 0), (-1, 0)], Orient::Mx180);
        assert_eq!(s.violations(&l), 0);
        let m: Vec<_> = [0, 1].map(|d| sums(l.units.of_device(&l, DeviceId(d)).map(Pt::from))).to_vec();
        assert!(mirror_allowed(&m));

        let l = two_cells(&[(1, 0), (-1, 0), (1, 0)], Orient::R0);
        let m: Vec<_> = [0, 1].map(|d| sums(l.units.of_device(&l, DeviceId(d)).map(Pt::from))).to_vec();
        assert!(!mirror_allowed(&m), "net φx survives no mirror");
    }

    #[test]
    fn a_flipped_vertical_partner_breaks_phi() {
        let s = set(OrientCheck::Phi);
        for o in [Orient::Mx, Orient::R180] {
            let l = two_cells(&[(0, 1)], o);
            assert_eq!((s.violations(&l), s.residual(&l)), (1, 2.0), "{o:?}");
        }
    }

    #[test]
    fn capacitor_units_never_violate() {
        let l = two_cells(&[(0, 0)], Orient::R90);
        for c in [OrientCheck::Axis, OrientCheck::Phi] {
            assert_eq!((set(c).violations(&l), set(c).unknown(&l)), (0, 0), "{c:?}");
        }
    }

    #[test]
    fn no_units_is_unknown() {
        let mut l = two_cells(&[(1, 0)], Orient::R90);
        l.units = Arc::default();
        for c in [OrientCheck::Axis, OrientCheck::Phi] {
            assert_eq!((set(c).violations(&l), set(c).unknown(&l)), (0, 1), "{c:?}");
        }
        // One member drawn, its partner not: still unknown.
        let units = [Unit { owner: 0, x: 50, y: 50, weight: 10, phi: (1, 0), sa: 0, sb: 0 }];
        let alts = [(Rect { x: 0, y: 0, w: 100, h: 100 }, &units[..])];
        let none = [(Rect { x: 0, y: 0, w: 100, h: 100 }, &[][..])];
        l.units = Arc::new(UnitLib::build(vec![0, 1], &[vec![DeviceId(0)], vec![DeviceId(1)]], [&alts[..], &none[..]].into_iter()));
        for c in [OrientCheck::Axis, OrientCheck::Phi] {
            assert_eq!((set(c).violations(&l), set(c).unknown(&l)), (0, 1), "{c:?}");
        }
    }
}

#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use crate::rule::RuleBatch;
    use pnr_core::{Orient, Rect, Unit, UnitLib};
    use std::sync::Arc;

    /// Devices 0 and 1 in cells 0 and 1, each drawing one unit per entry of
    /// its own φ list.
    fn cells(phis: [&[(i8, i8)]; 2]) -> Layout {
        let units: Vec<Vec<Unit>> = phis.iter().map(|p| p.iter().map(|&phi| Unit { owner: 0, x: 50, y: 50, weight: 10, phi, sa: 0, sb: 0 }).collect()).collect();
        let a0 = [(Rect { x: 0, y: 0, w: 100, h: 100 }, &units[0][..])];
        let a1 = [(Rect { x: 0, y: 0, w: 100, h: 100 }, &units[1][..])];
        let lib = UnitLib::build(vec![0, 1], &[vec![DeviceId(0)], vec![DeviceId(1)]], [&a0[..], &a1[..]].into_iter());
        Layout {
            x: vec![0, 10_000],
            y: vec![0, 0],
            hw: vec![50; 2],
            hh: vec![50; 2],
            axis: vec![],
            groups: vec![],
            orient: vec![Orient::R0; 2],
            variant: vec![0; 2],
            branch: Vec::new(),
            power_uw: vec![0; 2],
            temp_mc: vec![0; 2],
            units: Arc::new(lib),
        }
    }

    fn set(members: &[u16], check: OrientCheck) -> OrientationSet {
        OrientationSet { members: members.iter().map(|&d| DeviceId(d)).collect(), check, cell_of: Vec::new() }
    }

    #[test]
    fn fewer_than_two_members_is_vacuously_met() {
        let l = cells([&[(1, 0)], &[(0, 1)]]);
        for c in [OrientCheck::Axis, OrientCheck::Phi] {
            for m in [&[][..], &[0][..]] {
                let s = set(m, c);
                assert_eq!((s.cost(&l), s.violations(&l), s.residual(&l), s.unknown(&l)), (0.0, 0, 0.0, 0), "{c:?} {m:?}");
                let mut ids = Vec::new();
                s.violating_ids(&l, &mut ids);
                assert!(ids.is_empty());
            }
        }
    }

    #[test]
    fn one_member_with_mixed_axes_breaks_the_axis_check() {
        let l = cells([&[(1, 0), (0, 1)], &[(1, 0)]]);
        assert_eq!(set(&[0, 1], OrientCheck::Axis).violations(&l), 1);
        assert_eq!(set(&[0, 1], OrientCheck::Axis).cost(&l), 1.0);
    }

    #[test]
    fn phi_residual_is_the_worst_member_l1_gap() {
        // Member 1 reversed in x: |1 − (−1)| + 0 = 2.
        let l = cells([&[(1, 0)], &[(-1, 0)]]);
        let s = set(&[0, 1], OrientCheck::Phi);
        assert_eq!((s.residual(&l), s.cost(&l)), (2.0, 2.0));
        // Equal φ: met, whatever the magnitudes' units count.
        let l = cells([&[(1, 0)], &[(1, 0), (1, 0)]]);
        assert_eq!(s.violations(&l), 0);
    }

    #[test]
    fn ids_go_through_cell_of() {
        let mut s = set(&[0, 1, 2], OrientCheck::Axis);
        s.retarget(&[4, 4, 6]);
        let (mut t, mut p) = (Vec::new(), Vec::new());
        s.touched(&mut t);
        s.matched_pairs(&mut p);
        assert_eq!(t, [4, 4, 6]);
        assert_eq!(p, [(4, 4), (4, 6)]);
        assert_eq!((s.count(), s.kind()), (1, "Orientation"));
        let mut none = Vec::new();
        set(&[], OrientCheck::Axis).matched_pairs(&mut none);
        assert!(none.is_empty());
    }

    #[test]
    fn violating_ids_name_every_member_only_when_violated() {
        let s = set(&[0, 1], OrientCheck::Phi);
        let mut ids = Vec::new();
        s.violating_ids(&cells([&[(1, 0)], &[(1, 0)]]), &mut ids);
        assert!(ids.is_empty());
        s.violating_ids(&cells([&[(1, 0)], &[(-1, 0)]]), &mut ids);
        assert_eq!(ids, [0, 1]);
    }
}
