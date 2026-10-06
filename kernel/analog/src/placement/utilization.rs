//! Declared utilization floor (placement tier, budget).

use pnr_core::layout::Layout;
use crate::rule::RuleBatch;

/// The cells fill at least `u_min` of their enclosing box: footprint ≤
/// Σ cell area / `u_min`. Its residual is Θ, so it outranks extracted C in
/// the search key: a layout may not buy C with sprawl past this.
///
/// ponytail: a declared policy, not physics — a feasible optimum is a vector
/// (area, C, …) and scalarising it is the designer's call (Graeb 2007 ch.1).
/// An area spec from the design would replace `u_min`.
#[derive(Clone, Copy, Debug)]
pub struct Utilization {
    /// Minimum fill fraction, in `(0, 1]`; `≤ 0` disables the rule.
    pub u_min: f32,
}

impl Utilization {
    /// Footprint over the allowed footprint (`1.0` = at the floor); `0` with
    /// no cell area or a non-positive `u_min`.
    fn used(&self, l: &Layout) -> f32 {
        let cells: f64 = (0..l.x.len()).map(|i| 4.0 * f64::from(l.hw[i]) * f64::from(l.hh[i])).sum();
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
        assert_eq!((u.violations(&two_cells(0)), u.residual(&two_cells(0))), (0, 0.0));
        // 2 µm² of cells in a 1 × 4 µm box: 50% < 60%, 20% over the allowed box.
        let sprawl = two_cells(2_000);
        assert_eq!(u.violations(&sprawl), 1);
        assert!((u.residual(&sprawl) - 0.2).abs() < 1e-6);
    }
}

#[cfg(test)]
mod cleanup_tests {
    use super::*;

    fn cells(xs: &[i32], half: i32) -> Layout {
        let n = xs.len();
        Layout {
            x: xs.to_vec(),
            y: vec![0; n],
            hw: vec![half; n],
            hh: vec![half; n],
            axis: vec![],
            groups: vec![],
            orient: vec![pnr_core::Orient::default(); n],
            variant: vec![0; n],
            branch: Vec::new(),
            power_uw: vec![0; n],
            temp_mc: vec![0; n],
            units: Default::default(),
        }
    }

    #[test]
    fn no_cells_or_no_area_is_never_a_violation() {
        let u = Utilization { u_min: 0.9 };
        for l in [cells(&[], 500), cells(&[0, 5_000], 0)] {
            assert_eq!((u.cost(&l), u.violations(&l), u.residual(&l)), (0.0, 0, 0.0));
            assert_eq!(u.worst_usage(&l), Some(0.0));
        }
    }

    #[test]
    fn a_non_positive_floor_disables_the_rule() {
        let sprawl = cells(&[0, 1_000_000], 500);
        for u_min in [0.0, -1.0] {
            let u = Utilization { u_min };
            assert_eq!((u.violations(&sprawl), u.criticality(&sprawl)), (0, 0.0));
        }
    }

    #[test]
    fn exactly_at_the_floor_is_legal() {
        // Abutted pair at u_min = 1: footprint equals the cells' area.
        let l = cells(&[0, 1_000], 500);
        let u = Utilization { u_min: 1.0 };
        assert_eq!(u.worst_usage(&l), Some(1.0));
        assert_eq!((u.violations(&l), u.residual(&l), u.cost(&l)), (0, 0.0, 0.0));
        assert_eq!(u.criticality(&l), 1.0);
    }

    #[test]
    fn criticality_clamps_and_cost_equals_residual() {
        let l = cells(&[0, 3_000], 500);
        let u = Utilization { u_min: 1.0 };
        // 2 µm² of cells in a 4 × 1 µm box: used 2.
        assert_eq!(u.worst_usage(&l), Some(2.0));
        assert_eq!(u.criticality(&l), 1.0);
        assert_eq!(f64::from(u.cost(&l)), u.residual(&l));
        assert_eq!((u.count(), u.kind()), (1, "Utilization"));
        let half = Utilization { u_min: 0.25 };
        assert_eq!(half.criticality(&l), 0.5);
    }
}
