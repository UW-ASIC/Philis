//! Thermal-gradient matching (placement tier).

use pnr_core::ids::Target;
use pnr_core::layout::Layout;
use crate::rule::Rule;

/// Matched partners must see |ΔT| ≤ `max_delta_mc` (milli-°C).
///
/// `cost` prices the pair's ΔT from the live field ([`Layout::live_delta_temp_mc`]),
/// so a move is rewarded for putting the pair on one isotherm, not merely close
/// together (Hastings 3e eq.8.23, PDF p.388: mismatch ∝ TC·d·∂T/∂x; place the
/// separation along the isotherm). `satisfied`, `headroom` and `residual` read
/// the epoch-frozen [`Layout::temp_mc`], so Θ is fixed within an anneal.
#[derive(Clone, Copy)]
pub struct ThermalGradient {
    pub a: Target,
    pub b: Target,
    pub max_delta_mc: i32,
    /// Safety margin on `max_delta_mc`, percent.
    pub margin_pct: u8,
}

/// Cost at `ΔT = max_delta_mc`: the old distance pull's weight at a 10 µm
/// pitch (`3·(10 µm)²·1e-3`), so the objective's scale is unchanged.
const AT_SPEC_COST: f32 = 3.0e5;

impl Rule for ThermalGradient {
    type On = Layout;
    /// `AT_SPEC_COST·(ΔT/max)²` over the live field; `0` on an unpowered die.
    fn cost(self, l: &Layout) -> f32 {
        let u = l.live_delta_temp_mc(self.a, self.b) / self.max_delta_mc.max(1) as f32;
        AT_SPEC_COST * u * u
    }
    /// Vacuously true with no power data (see [`Rule::applicable`]).
    fn satisfied(self, l: &Layout) -> bool {
        l.delta_temp_mc(self.a, self.b) <= self.max_delta_mc
    }
    fn headroom(self, l: &Layout) -> f32 {
        1.0 - l.delta_temp_mc(self.a, self.b) as f32 / self.max_delta_mc.max(1) as f32
    }
    fn usage(self, l: &Layout) -> Option<f32> {
        self.applicable(l)
            .then(|| l.delta_temp_mc(self.a, self.b) as f32 / self.max_delta_mc.max(1) as f32)
    }
    /// An unpowered die has no gradient to match against.
    fn applicable(self, l: &Layout) -> bool {
        l.power_uw.iter().any(|&p| p != 0)
    }
    /// All-zero power is "no operating point", not "cool".
    ///
    /// ponytail: a genuinely unbiased block also reads unknown; add a
    /// `power_known` flag to `Layout` if one ever needs certifying.
    fn known(self, l: &Layout) -> bool {
        self.applicable(l)
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
            units: Default::default(),
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
    fn cost_rewards_the_isotherm_not_closeness() {
        // Both 20 µm from the heater, 40 µm apart: one isotherm, no cost —
        // cheaper than a closer pair straddling the gradient.
        let iso = bench(20_000, -20_000);
        let near = bench(5_000, 15_000);
        assert!(pair().cost(&iso) < 1.0);
        assert!(pair().cost(&near) > pair().cost(&iso));
    }

    #[test]
    fn without_power_data_the_rule_is_honestly_inert() {
        let mut l = bench(5_000, 80_000);
        l.power_uw = vec![0; 3];
        l.refresh_temps();
        assert!(pair().satisfied(&l));
        assert!(!pair().applicable(&l), "and reports itself not applicable");
        assert_eq!(pair().usage(&l), None);
    }
}
