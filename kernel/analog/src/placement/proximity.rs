//! Preferential proximity (placement tier).
//!
//! Ported from `backend/constraints/src/placement_level/proximity.rs`.

use pnr_core::ids::Target;
use pnr_core::layout::Layout;
use crate::rule::Rule;

/// **Proximity.** A soft pull clustering functionally related devices (matched
/// pair near its tail source, cascode near its bias) to shorten sensitive routing
/// and improve matching. Unlike isolation it never prohibits — it only attracts.
///
/// - **Enforcement:** [`crate::Mode::Cost`] (soft, priority 40).
/// - **Arity:** Device↔Device.
/// - **Books:** AOAL ch06/6.4, ch13; PNR_ANALOG 00/1.3 (matching-rule hierarchy).
#[derive(Clone, Copy)]
pub struct Proximity {
    pub a: Target,
    pub b: Target,
    /// Target max separation, `nm`.
    pub max_distance_nm: i32,
}

impl Rule for Proximity {
    type On = Layout;
    /// Squared excess of centre distance beyond `max_distance_nm`; `0.0` within.
    ///
    /// The engine's proximity pull (`engine::soft_terms`): `weight · ex² · 1e-3`
    /// with `ex = (d − gap).max(0)` and rule weight 1.
    fn cost(self, l: &Layout) -> f32 {
        let (ax, ay) = l.centre(self.a);
        let (bx, by) = l.centre(self.b);
        let dx = (ax - bx) as f32;
        let dy = (ay - by) as f32;
        let d = (dx * dx + dy * dy).sqrt();
        let ex = (d - self.max_distance_nm as f32).max(0.0);
        ex * ex * 1e-3
    }

    fn retarget(self, cell_of: &[u16]) -> Self {
        Self { a: self.a.retarget(cell_of), b: self.b.retarget(cell_of), ..self }
    }
}
