//! Substrate-noise isolation (placement tier).

use pnr_core::ids::Target;
use pnr_core::layout::Layout;
use crate::rule::Rule;

/// Keep noisy `a` at least `min_distance_nm` edge-to-edge from sensitive `b`:
/// spacing attenuates substrate propagation (Charbon et al. 2001 ch.8, PDF
/// p.127), up to a saturation distance past which it buys nothing. A search
/// aid, not the electrical acceptance test.
#[derive(Clone, Copy)]
pub struct Isolation {
    /// Aggressor.
    pub a: Target,
    /// Victim.
    pub b: Target,
    pub min_distance_nm: i32,
}

impl Rule for Isolation {
    type On = Layout;
    /// `shortfall²·4e-3`.
    fn cost(self, l: &Layout) -> f32 {
        let v = (self.min_distance_nm as f32 - l.edge_gap(self.a, self.b)).max(0.0);
        v * v * 4e-3
    }
    fn satisfied(self, l: &Layout) -> bool {
        l.edge_gap(self.a, self.b) >= self.min_distance_nm as f32
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
