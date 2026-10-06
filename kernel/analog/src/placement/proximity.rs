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
