//! Preferential proximity (placement tier).

use pnr_core::ids::Target;
use pnr_core::layout::Layout;
use crate::rule::Rule;

/// Soft pull keeping related devices within `max_distance_nm` of each other,
/// edge to edge (a centre distance would make the spec depend on cell size).
/// Attracts only: a priced budget (Θ) and a cost pull, never a hard gate.
#[derive(Clone, Copy, Debug)]
pub struct Proximity {
    /// First device.
    pub a: Target,
    /// Second device.
    pub b: Target,
    /// Largest edge-to-edge gap still in spec, nm; `≤ 0` asks for touching.
    pub max_distance_nm: i32,
}

impl Proximity {
    /// Euclidean edge-to-edge gap, nm (`0` when touching or overlapping).
    #[inline]
    fn gap(self, l: &Layout) -> f32 {
        l.edge_gap(self.a, self.b)
    }
}

impl Rule for Proximity {
    type On = Layout;
    /// `(excess / max_distance)²`, excess = gap past `max_distance_nm` (PLC-18: dimensionless).
    fn cost(self, l: &Layout) -> f32 {
        let m = self.max_distance_nm.max(1) as f32;
        let e = (self.gap(l) - m).max(0.0) / m;
        e * e
    }
    fn satisfied(self, l: &Layout) -> bool {
        self.gap(l) <= self.max_distance_nm as f32
    }
    /// Gap past the spec over the spec (linear, so it sums across rules).
    fn residual(self, l: &Layout) -> f32 {
        let m = self.max_distance_nm as f32;
        crate::rule::over(self.gap(l) - m, m)
    }
    /// Gap over the spec (`1.0` = at the limit), spec floored at 1 nm.
    fn usage(self, l: &Layout) -> Option<f32> {
        Some(self.gap(l) / self.max_distance_nm.max(1) as f32)
    }
    fn touches(self, out: &mut Vec<u32>) {
        super::push_devices(out, &[self.a, self.b]);
    }
    fn retarget(self, cell_of: &[u16]) -> Self {
        Self { a: self.a.retarget(cell_of), b: self.b.retarget(cell_of), ..self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule::RuleBatch;
    use pnr_core::ids::{DeviceId, GroupId};

    /// Device ids in `(a, b)` order; a `Group` side contributes nothing.
    #[test]
    fn touched_yields_device_ids_only() {
        let (d, g) = (|i| Target::Device(DeviceId(i)), Target::Group(GroupId(0)));
        let batch = vec![
            Proximity { a: d(3), b: d(1), max_distance_nm: 5_000 },
            Proximity { a: g, b: d(7), max_distance_nm: 5_000 },
            Proximity { a: g, b: g, max_distance_nm: 5_000 },
        ];
        let mut ids = Vec::new();
        batch.touched(&mut ids);
        assert_eq!(ids, [3, 1, 7]);
    }
}

#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use pnr_core::ids::DeviceId;

    fn d(i: u16) -> Target {
        Target::Device(DeviceId(i))
    }

    /// Two 1 µm squares, `b` at `(x, y)`.
    fn at(x: i32, y: i32) -> Layout {
        Layout {
            x: vec![0, x],
            y: vec![0, y],
            hw: vec![500; 2],
            hh: vec![500; 2],
            orient: vec![pnr_core::Orient::default(); 2],
            variant: vec![0; 2],
            axis: vec![],
            branch: vec![],
            groups: vec![],
            power_uw: vec![0; 2],
            temp_mc: vec![0; 2],
            units: Default::default(),
        }
    }

    fn near(max: i32) -> Proximity {
        Proximity { a: d(0), b: d(1), max_distance_nm: max }
    }

    #[test]
    fn gap_is_euclidean_edge_to_edge() {
        // Edge gaps 3 µm and 4 µm → 5 µm.
        let l = at(4_000, 5_000);
        assert_eq!(near(5_000).usage(&l), Some(1.0));
        assert!(near(5_000).satisfied(&l));
        assert!(!near(4_999).satisfied(&l));
        // Overlapping: no gap at all.
        assert_eq!(near(5_000).usage(&at(300, 0)), Some(0.0));
    }

    #[test]
    fn cost_and_residual_scale_with_the_excess() {
        // 10 µm gap against a 5 µm limit: 100 % over.
        let l = at(11_000, 0);
        assert!((near(5_000).cost(&l) - 1.0).abs() < 1e-6);
        assert!((near(5_000).residual(&l) - 1.0).abs() < 1e-6);
        assert!((near(5_000).usage(&l).unwrap() - 2.0).abs() < 1e-6);
        // Inside the limit: free.
        let l = at(3_000, 0);
        assert_eq!((near(5_000).cost(&l), near(5_000).residual(&l)), (0.0, 0.0));
    }

    #[test]
    fn a_zero_limit_asks_for_touching() {
        assert!(near(0).satisfied(&at(1_000, 0)), "abutting");
        let apart = at(1_010, 0);
        assert!(!near(0).satisfied(&apart));
        assert_eq!(near(0).residual(&apart), 1.0, "no budget to divide: a full violation");
        assert!(near(0).cost(&apart) > 0.0, "the pull is still on");
        assert!(near(0).usage(&apart).unwrap().is_finite(), "usage floors the limit at 1 nm");
    }

    #[test]
    fn retarget_maps_both_sides() {
        let r = near(7).retarget(&[3, 2]);
        assert_eq!((r.a, r.b, r.max_distance_nm), (d(3), d(2), 7));
    }
}
