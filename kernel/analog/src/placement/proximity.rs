//! Preferential proximity (placement tier).

use pnr_core::ids::Target;
use pnr_core::layout::Layout;
use crate::rule::Rule;

/// Soft pull keeping related devices within `max_distance_nm` (centre to
/// centre). Attracts only; never prohibits.
#[derive(Clone, Copy)]
pub struct Proximity {
    pub a: Target,
    pub b: Target,
    pub max_distance_nm: i32,
}

impl Rule for Proximity {
    type On = Layout;
    /// `ex²·1e-3`, `ex` = distance past `max_distance_nm`.
    fn cost(self, l: &Layout) -> f32 {
        let (ax, ay) = l.centre(self.a);
        let (bx, by) = l.centre(self.b);
        let dx = (ax - bx) as f32;
        let dy = (ay - by) as f32;
        let ex = ((dx * dx + dy * dy).sqrt() - self.max_distance_nm as f32).max(0.0);
        ex * ex * 1e-3
    }

    fn retarget(self, cell_of: &[u16]) -> Self {
        Self { a: self.a.retarget(cell_of), b: self.b.retarget(cell_of), ..self }
    }
}
