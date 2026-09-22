//! Thermal-gradient matching (placement tier).

use pnr_core::ids::Target;
use pnr_core::layout::Layout;
use crate::rule::Rule;

/// Matched partners must see |ΔT| ≤ `max_delta_mc` (milli-°C).
///
/// `cost` is a distance pull (weight 3), because [`Layout::temp_mc`] is
/// refreshed per epoch and gives a trial move no gradient; `satisfied`,
/// `headroom` and `residual` read the measured ΔT.
#[derive(Clone, Copy)]
pub struct ThermalGradient {
    pub a: Target,
    pub b: Target,
    pub max_delta_mc: i32,
    /// Safety margin on `max_delta_mc`, percent.
    pub margin_pct: u8,
}

impl Rule for ThermalGradient {
    type On = Layout;
    fn cost(self, l: &Layout) -> f32 {
        let (ax, ay) = l.centre(self.a);
        let (bx, by) = l.centre(self.b);
        let dx = (ax - bx) as f32;
        let dy = (ay - by) as f32;
        3.0 * (dx * dx + dy * dy) * 1e-3
    }
    /// Trivially true with no power data (uniform die).
    fn satisfied(self, l: &Layout) -> bool {
        l.delta_temp_mc(self.a, self.b) <= self.max_delta_mc
    }
    fn headroom(self, l: &Layout) -> f32 {
        1.0 - l.delta_temp_mc(self.a, self.b) as f32 / self.max_delta_mc.max(1) as f32
    }
    fn margin(self) -> f32 {
        f32::from(self.margin_pct) / 100.0
    }
    fn residual(self, l: &Layout) -> f32 {
        let budget = self.max_delta_mc as f32;
        crate::rule::over(l.delta_temp_mc(self.a, self.b) as f32 - budget, budget)
    }
    fn retarget(self, cell_of: &[u16]) -> Self {
        Self { a: self.a.retarget(cell_of), b: self.b.retarget(cell_of), ..self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_core::ids::DeviceId;

    /// Heater at the origin, two matched partners placed around it.
    fn bench(x1: i32, x2: i32) -> Layout {
        let mut l = Layout {
            x: vec![0, x1, x2],
            y: vec![0, 0, 0],
            hw: vec![500; 3],
            hh: vec![500; 3],
            axis: vec![0],
            groups: vec![],
            orient: vec![pnr_core::Orient::default(); 3],
            variant: vec![0; 3],
            branch: Vec::new(),
            power_uw: vec![10_000, 0, 0],
            temp_mc: vec![0; 3],
        };
        l.refresh_temps();
        l
    }

    fn pair() -> ThermalGradient {
        ThermalGradient {
            a: Target::Device(DeviceId(1)),
            b: Target::Device(DeviceId(2)),
            max_delta_mc: 100,
            margin_pct: 20,
        }
    }

    #[test]
    fn placement_drives_the_thermal_check() {
        // Lopsided: partner 1 sits far closer to the heater than partner 2.
        let bad = bench(5_000, 80_000);
        assert!(!pair().satisfied(&bad), "a real gradient must fail the budget");
        assert!(pair().headroom(&bad) <= 0.0);

        // Mirrored about the heater: one isotherm, gradient gone.
        let good = bench(20_000, -20_000);
        assert!(pair().satisfied(&good), "isothermal pair must pass");
        assert!(pair().headroom(&good) > 0.9);
    }

    #[test]
    fn without_power_data_the_rule_is_honestly_inert() {
        let mut l = bench(5_000, 80_000);
        l.power_uw = vec![0; 3];
        l.refresh_temps();
        assert!(pair().satisfied(&l));
    }
}
