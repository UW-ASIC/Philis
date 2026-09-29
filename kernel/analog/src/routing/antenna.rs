//! Antenna ratio (routing tier).

use pnr_core::ids::NetId;
use pnr_core::routes::Routes;
use crate::rule::Rule;
use super::Stack;

/// Conductor-area / gate-area ratio on a gate net stays under the limit at
/// every etch stage (MFG-04): per layer at its own stage from the deck's
/// antenna rules ([`Stack::antenna`]; Hastings pp. 228–229), or — without the
/// stack — all routed metal against the tightest ratio.
#[derive(Clone, Copy)]
pub struct Antenna {
    pub net: NetId,
    /// Max ratio ×100 (the deck's tightest antenna rule): the fallback limit.
    pub max_ratio_x100: i32,
    /// Total gate area the net drives, nm².
    pub gate_area_nm2: i64,
    /// Safety margin, percent.
    pub margin_pct: u8,
    /// Per-stage limits; `None` = the cumulative all-layer fallback.
    pub stack: Option<&'static Stack>,
}

impl Rule for Antenna {
    type On = Routes;
    fn cost(self, r: &Routes) -> f32 {
        let (ratio, limit) = self.worst(r);
        (ratio - limit).max(0.0) * 100.0
    }
    fn satisfied(self, r: &Routes) -> bool {
        let (ratio, limit) = self.worst(r);
        ratio <= limit
    }
    fn touches(self, out: &mut Vec<u32>) {
        out.push(u32::from(self.net.0));
    }
    fn headroom(self, r: &Routes) -> f32 {
        let (ratio, limit) = self.worst(r);
        1.0 - ratio / limit.max(f32::MIN_POSITIVE)
    }
    fn usage(self, r: &Routes) -> Option<f32> {
        let (ratio, limit) = self.worst(r);
        Some(ratio / limit.max(f32::MIN_POSITIVE))
    }
    fn margin(self) -> f32 {
        f32::from(self.margin_pct) / 100.0
    }
    fn residual(self, r: &Routes) -> f32 {
        let (ratio, limit) = self.worst(r);
        crate::rule::over(ratio - limit, limit)
    }
}

impl Antenna {
    /// `(ratio, limit)` of the worst stage. Without the stack: all routed metal
    /// over the gate area against the tightest ratio (the cumulative,
    /// all-layers form, which over-estimates a per-layer rule), in the ×100
    /// fixed point it has always been measured in.
    fn worst(self, r: &Routes) -> (f32, f32) {
        if let Some(w) = self.stack.and_then(|s| s.antenna(r.shapes(self.net), r.cell_metal(self.net), r.gate_pins(self.net), self.gate_area_nm2)) {
            return w;
        }
        let area: i64 = r.shapes(self.net).iter().chain(r.cell_metal(self.net)).map(|s| i64::from(s.rect.w) * i64::from(s.rect.h)).sum();
        ((area * 100 / self.gate_area_nm2.max(1)) as f32 / 100.0, self.max_ratio_x100 as f32 / 100.0)
    }
}
