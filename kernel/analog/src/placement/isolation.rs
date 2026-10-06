//! Substrate-noise isolation (placement tier).

use pnr_core::ids::Target;
use pnr_core::layout::Layout;
use crate::rule::Rule;

/// Keep noisy `a` at least `min_distance_nm` edge-to-edge from sensitive `b`:
/// spacing attenuates substrate propagation (Charbon et al. 2001 ch.8, PDF
/// p.127), up to a saturation distance past which it buys nothing. A search
/// aid, not the electrical acceptance test.
#[derive(Clone, Copy, Debug)]
pub struct Isolation {
    /// Aggressor.
    pub a: Target,
    /// Victim.
    pub b: Target,
    /// Required edge-to-edge distance, nm; `≤ 0` is always satisfied.
    pub min_distance_nm: i32,
}

impl Rule for Isolation {
    type On = Layout;
    /// `(shortfall / min_distance)²` (PLC-18: dimensionless).
    fn cost(self, l: &Layout) -> f32 {
        let m = self.min_distance_nm.max(1) as f32;
        let s = (m - l.edge_gap(self.a, self.b)).max(0.0) / m;
        s * s
    }
    fn satisfied(self, l: &Layout) -> bool {
        l.edge_gap(self.a, self.b) >= self.min_distance_nm as f32
    }
    fn touches(self, out: &mut Vec<u32>) {
        super::push_devices(out, &[self.a, self.b]);
    }
    fn retarget(self, cell_of: &[u16]) -> Self {
        Self { a: self.a.retarget(cell_of), b: self.b.retarget(cell_of), ..self }
    }
    /// Linear shortfall / required distance (squares do not sum across rules).
    fn residual(self, l: &Layout) -> f32 {
        let floor = self.min_distance_nm as f32;
        crate::rule::over(floor - l.edge_gap(self.a, self.b), floor)
    }
}

/// Keeps a substrate injector equidistant from both halves of a differential
/// pair: Charbon §8.3.1 (L3382–3399), an equidistant injector couples a
/// common-mode disturbance only. Cost-only (no threshold in the source), so
/// it is always satisfied and never contributes residual.
#[derive(Clone, Copy, Debug)]
pub struct SubstrateBalance {
    /// Noise injector.
    pub aggressor: Target,
    /// First half of the pair.
    pub a: Target,
    /// Second half of the pair.
    pub b: Target,
}

impl Rule for SubstrateBalance {
    type On = Layout;
    /// `((d_ca − d_cb)/d_ab)²` on centre distances: in `[0, 1]` by the triangle
    /// inequality, 1 on the pair's axis (Charbon's worst case), 0 on its
    /// bisector and when the halves coincide.
    fn cost(self, l: &Layout) -> f32 {
        let d = |p: Target, q: Target| {
            let ((px, py), (qx, qy)) = (l.centre(p), l.centre(q));
            f64::from(px - qx).hypot(f64::from(py - qy))
        };
        let ab = d(self.a, self.b);
        if ab == 0.0 {
            return 0.0;
        }
        let s = (d(self.aggressor, self.a) - d(self.aggressor, self.b)) / ab;
        (s * s) as f32
    }
    fn touches(self, out: &mut Vec<u32>) {
        super::push_devices(out, &[self.aggressor, self.a, self.b]);
    }
    fn retarget(self, cell_of: &[u16]) -> Self {
        Self { aggressor: self.aggressor.retarget(cell_of), a: self.a.retarget(cell_of), b: self.b.retarget(cell_of) }
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
            Isolation { a: d(3), b: d(1), min_distance_nm: 10_000 },
            Isolation { a: d(7), b: g, min_distance_nm: 10_000 },
            Isolation { a: g, b: g, min_distance_nm: 10_000 },
        ];
        let mut ids = Vec::new();
        batch.touched(&mut ids);
        assert_eq!(ids, [3, 1, 7]);
    }

    /// Device 0 = a, 1 = b, 2.. = aggressors, at the given centres.
    fn at(c: &[(i32, i32)]) -> Layout {
        let n = c.len();
        Layout {
            x: c.iter().map(|p| p.0).collect(),
            y: c.iter().map(|p| p.1).collect(),
            hw: vec![500; n],
            hh: vec![500; n],
            orient: vec![pnr_core::Orient::default(); n],
            variant: vec![0; n],
            axis: vec![],
            branch: vec![],
            groups: vec![],
            power_uw: vec![0; n],
            temp_mc: vec![0; n],
            units: Default::default(),
        }
    }

    fn balance(g: u16) -> SubstrateBalance {
        let d = |i| Target::Device(DeviceId(i));
        SubstrateBalance { aggressor: d(g), a: d(0), b: d(1) }
    }

    #[test]
    fn an_aggressor_on_the_bisector_costs_nothing() {
        let l = at(&[(-5_000, 0), (5_000, 0), (0, 10_000)]);
        assert!(balance(2).cost(&l).abs() < 1e-6, "{}", balance(2).cost(&l));
        assert!(balance(2).satisfied(&l) && balance(2).residual(&l) == 0.0, "cost-only");
    }

    #[test]
    fn off_axis_costs() {
        let l = at(&[(-5_000, 0), (5_000, 0), (-20_000, 0), (-5_000, 10_000)]);
        // On the axis: d_ca 15 000, d_cb 25 000, d_ab 10 000.
        assert!((balance(2).cost(&l) - 1.0).abs() < 1e-6, "{}", balance(2).cost(&l));
        // d_ca 10 000, d_cb √2e8 = 14 142.1: (4 142.1/10 000)² = 0.1716.
        assert!((balance(3).cost(&l) - 0.1716).abs() < 1e-3, "{}", balance(3).cost(&l));
        let mut ids = Vec::new();
        balance(3).touches(&mut ids);
        assert_eq!(ids, [3, 0, 1]);
    }
}
