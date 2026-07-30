//! Thermal-gradient matching (placement tier).
//!
//! Ported from `backend/constraints/src/placement_level/thermal.rs`.

use pnr_core::ids::Target;
use pnr_core::layout::Layout;
use crate::rule::Rule;

/// **Thermal-gradient constraint.** A temperature difference across a matched
/// pair produces mismatch via device thermal coefficients — BJT `Vbe ≈ −2 mV/°C`
/// (~8% `Id`/°C), MOSFET `Vth ≈ −1…−2 mV/°C` (~2–4% `Id`/°C). Tolerance scales by
/// tier: Minimal 2.0 °C, Moderate 0.5 °C, Exceptional 0.1 °C (bandgap / ADC
/// reference). Placing partners on isotherms and common-centroid about the heat
/// source cancels the first-order gradient.
///
/// - **Enforcement:** [`crate::Mode::Hard`] — bounds `max_delta_mc` (priority 75).
/// - **Arity:** Device↔Device.
/// - **Books:** PNR_ANALOG 00/4.3, 02/4.A.5 (#50), 04/2.6 (#64); AOAL ch01/1.2,
///   ch08/8.2.7 (#62); FOLD 6.6/6.6.4 (#8).
#[derive(Clone, Copy)]
pub struct ThermalGradient {
    pub a: Target,
    pub b: Target,
    /// Max tolerable ΔT between the pair, milli-°C (tier-derived).
    pub max_delta_mc: i32,
    /// Safety margin on `max_delta_mc`, percent — the optimiser targets
    /// `max_delta_mc·(1 − margin)` while the raw value stays the hard floor.
    pub margin_pct: u8,
}

impl Rule for ThermalGradient {
    type On = Layout;
    /// Engine thermal pull (`engine::soft_terms`): proximity form with zero target
    /// gap and weight 3, `3 · d² · 1e-3` — pulls partners onto a shared isotherm.
    ///
    /// This stays a **distance** proxy rather than the measured ΔT on purpose:
    /// [`Layout::temp_mc`] is refreshed at epoch boundaries, so within an epoch
    /// it is constant and would give a trial move no gradient at all. Pulling
    /// partners together is the smooth per-move guide; the measured ΔT below is
    /// the verification (see `backend/TODO.md` §1).
    fn cost(self, l: &Layout) -> f32 {
        let (ax, ay) = l.centre(self.a);
        let (bx, by) = l.centre(self.b);
        let dx = (ax - bx) as f32;
        let dy = (ay - by) as f32;
        3.0 * (dx * dx + dy * dy) * 1e-3
    }

    /// Measured ΔT across the pair is within tolerance.
    ///
    /// Reads the solver's [`Layout::temp_mc`], so this is a live function of
    /// placement: move a partner onto the other's isotherm and it clears. With
    /// no power data the solver reports a uniform die, ΔT is `0`, and the rule
    /// is trivially satisfied — honestly so, rather than by construction.
    fn satisfied(self, l: &Layout) -> bool {
        l.delta_temp_mc(self.a, self.b) <= self.max_delta_mc
    }

    /// Fraction of the ΔT budget still unspent.
    fn headroom(self, l: &Layout) -> f32 {
        let budget = self.max_delta_mc.max(1) as f32;
        1.0 - (l.delta_temp_mc(self.a, self.b) as f32 / budget)
    }

    fn margin(self) -> f32 {
        f32::from(self.margin_pct) / 100.0
    }

    /// Measured ΔT past tolerance, as a fraction of tolerance.
    ///
    /// Reads [`Layout::temp_mc`] like [`satisfied`](Rule::satisfied) does, **not** the
    /// centre-distance proxy `cost` uses. The proxy exists because `temp_mc` is refreshed
    /// at epoch boundaries and so gives a trial move no gradient; Θ is evaluated at those
    /// same boundaries, so it can afford the real measurement — and it should, because a
    /// distance is not a temperature and normalising nm² by milli-°C would be meaningless.
    ///
    /// A tier difference is exactly what normalisation buys here: a bandgap at 100 m°C and
    /// a loose pair at 2000 m°C both read `1.0` when they miss by their own tolerance, so
    /// Θ ranks them by how badly each blew *its* budget rather than by how tight the
    /// budget was.
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
