//! Noise isolation (placement tier).
//!
//! Ported from `backend/constraints/src/placement_level/isolation.rs`.

use pnr_core::ids::Target;
use pnr_core::layout::Layout;
use crate::rule::Rule;

/// **Isolation.** Substrate noise couples from high-injection sources (charge
/// pumps, ESD diodes, power devices) to sensitive analog via the substrate.
/// Minimum separation plus guard rings attenuate it (20–40 dB/ring, 40–60 dB with
/// deep N-well). Depletion width grows 2–3× from 5 V to 40 V, so spacing scales
/// with operating voltage.
///
/// - **Enforcement:** [`crate::Mode::Hard`] (legality, priority 85).
/// - **Arity:** Device↔Device (noisy `a` → sensitive `b`), directional.
/// - **Books:** AOAL ch01/1.2.1, ch02/2.6.1, ch14/14.1–14.2 (#51); FOLD 7.1 (#15/#17);
///   ALS 3.1/3.1.2 (#7); PNR_ANALOG 02/4.A.6 (#51).
#[derive(Clone, Copy)]
pub struct Isolation {
    /// Noisy (aggressor) device.
    pub a: Target,
    /// Sensitive (victim) device.
    pub b: Target,
    pub min_distance_nm: i32,
}

impl Rule for Isolation {
    type On = Layout;
    /// Engine required-gap soft term (`engine::soft_terms`): `violation² · 4e-3`
    /// with `violation = (min_distance − gap).max(0)`. `gap` is the true
    /// edge-to-edge spacing from [`Layout::edge_gap`] (device extents).
    fn cost(self, l: &Layout) -> f32 {
        let gap = l.edge_gap(self.a, self.b);
        let violation = (self.min_distance_nm as f32 - gap).max(0.0);
        violation * violation * 4e-3
    }
    /// Edge-to-edge gap ≥ `min_distance_nm`.
    fn satisfied(self, l: &Layout) -> bool {
        l.edge_gap(self.a, self.b) >= self.min_distance_nm as f32
    }

    fn retarget(self, cell_of: &[u16]) -> Self {
        Self { a: self.a.retarget(cell_of), b: self.b.retarget(cell_of), ..self }
    }

    /// Separation shortfall as a fraction of the required distance.
    ///
    /// `cost` is `violation² · 4e-3` — squared, to make the tail hurt, and scaled to sit
    /// alongside the other placement pull terms. That is fine for an objective and wrong
    /// for Θ: squaring is not summable across rules (two rules 10% over would contribute
    /// `0.01` each while one 20% over contributes `0.04`, so Θ would depend on how a
    /// violation is *partitioned*), and the `4e-3` is a weight, not a unit conversion.
    /// The residual is therefore linear and divided by the spec.
    fn residual(self, l: &Layout) -> f32 {
        let floor = self.min_distance_nm as f32;
        crate::rule::over(floor - l.edge_gap(self.a, self.b), floor)
    }
}
