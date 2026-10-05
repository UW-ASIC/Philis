//! Declared utilization floor (placement tier, budget).

use crate::rule::RuleBatch;
use pnr_core::layout::Layout;

/// The cells fill at least `u_min` of their enclosing box: footprint ≤
/// Σ cell area / `u_min`. Its residual is Θ, so it outranks extracted C in
/// the search key: a layout may not buy C with sprawl past this.
///
/// ponytail: a declared policy, not physics — a feasible optimum is a vector
/// (area, C, …) and scalarising it is the designer's call (Graeb 2007 ch.1).
/// An area spec from the design would replace `u_min`.
pub struct Utilization {
    pub u_min: f32,
}

impl Utilization {
    /// Footprint over the allowed footprint (`1.0` = at the floor).
    fn used(&self, l: &Layout) -> f32 {
        let cells: f64 = (0..l.x.len())
            .map(|i| 4.0 * f64::from(l.hw[i]) * f64::from(l.hh[i]))
            .sum();
        if cells <= 0.0 || self.u_min <= 0.0 {
            return 0.0;
        }
        (l.footprint_nm2() * f64::from(self.u_min) / cells) as f32
    }
}

impl RuleBatch<Layout> for Utilization {
    /// Excess footprint, as a fraction of the allowed one.
    fn cost(&self, l: &Layout) -> f32 {
        (self.used(l) - 1.0).max(0.0)
    }
    fn violations(&self, l: &Layout) -> u32 {
        u32::from(self.used(l) > 1.0)
    }
    fn residual(&self, l: &Layout) -> f64 {
        f64::from((self.used(l) - 1.0).max(0.0))
    }
    fn kind(&self) -> &'static str {
        "Utilization"
    }
    fn count(&self) -> usize {
        1
    }
    fn criticality(&self, l: &Layout) -> f32 {
        self.used(l).clamp(0.0, 1.0)
    }
    fn worst_usage(&self, l: &Layout) -> Option<f32> {
        Some(self.used(l))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_cells(gap: i32) -> Layout {
        Layout {
            x: vec![0, 1_000 + gap],
            y: vec![0, 0],
            hw: vec![500; 2],
            hh: vec![500; 2],
            axis: vec![0],
            groups: vec![],
            orient: vec![pnr_core::Orient::default(); 2],
            variant: vec![0; 2],
            branch: Vec::new(),
            power_uw: vec![0; 2],
            temp_mc: vec![0; 2],
            units: Default::default(),
        }
    }

    #[test]
    fn sprawl_past_the_floor_is_a_budget_overshoot() {
        let u = Utilization { u_min: 0.6 };
        // Abutted: 100% full.
        assert_eq!(
            (u.violations(&two_cells(0)), u.residual(&two_cells(0))),
            (0, 0.0)
        );
        // 2 µm² of cells in a 1 × 4 µm box: 50% < 60%, 20% over the allowed box.
        let sprawl = two_cells(2_000);
        assert_eq!(u.violations(&sprawl), 1);
        assert!((u.residual(&sprawl) - 0.2).abs() < 1e-6);
    }
}
