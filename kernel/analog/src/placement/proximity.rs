//! Preferential proximity (placement tier).

use pnr_core::ids::Target;
use pnr_core::layout::Layout;
use crate::rule::Rule;

/// Soft pull keeping related devices within `max_distance_nm` of each other,
/// edge to edge (a centre distance would make the spec depend on cell size).
/// Attracts only: a priced budget (Θ) and a cost pull, never a hard gate.
#[derive(Clone, Copy)]
pub struct Proximity {
    pub a: Target,
    pub b: Target,
    pub max_distance_nm: i32,
}

impl Proximity {
    /// Euclidean edge-to-edge gap, nm (`0` when touching or overlapping).
    fn gap(self, l: &Layout) -> f32 {
        let ((ax, ay), (aw, ah)) = (l.centre(self.a), l.extent(self.a));
        let ((bx, by), (bw, bh)) = (l.centre(self.b), l.extent(self.b));
        let gx = ((ax - bx).abs() - aw - bw).max(0) as f32;
        let gy = ((ay - by).abs() - ah - bh).max(0) as f32;
        gx.hypot(gy)
    }
}

impl Rule for Proximity {
    type On = Layout;
    /// `ex²·1e-3`, `ex` = gap past `max_distance_nm`.
    fn cost(self, l: &Layout) -> f32 {
        let ex = (self.gap(l) - self.max_distance_nm as f32).max(0.0);
        ex * ex * 1e-3
    }
    fn satisfied(self, l: &Layout) -> bool {
        self.gap(l) <= self.max_distance_nm as f32
    }
    fn residual(self, l: &Layout) -> f32 {
        let m = self.max_distance_nm as f32;
        crate::rule::over(self.gap(l) - m, m)
    }
    fn usage(self, l: &Layout) -> Option<f32> {
        Some(self.gap(l) / self.max_distance_nm.max(1) as f32)
    }
    fn retarget(self, cell_of: &[u16]) -> Self {
        Self { a: self.a.retarget(cell_of), b: self.b.retarget(cell_of), ..self }
    }
}
