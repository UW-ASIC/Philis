//! Antenna ratio (routing tier).

use pnr_core::ids::NetId;
use pnr_core::routes::Routes;
use crate::rule::Rule;

/// Metal-area / gate-area ratio on a gate net stays under the limit.
#[derive(Clone, Copy)]
pub struct Antenna {
    pub net: NetId,
    /// Max ratio ×100 (the deck's antenna rule).
    pub max_ratio_x100: i32,
    /// Total gate area the net drives, nm².
    pub gate_area_nm2: i64,
    /// Safety margin, percent.
    pub margin_pct: u8,
}

impl Rule for Antenna {
    type On = Routes;
    fn cost(self, r: &Routes) -> f32 {
        (self.ratio_x100(r) - self.max_ratio_x100).max(0) as f32
    }
    fn satisfied(self, r: &Routes) -> bool {
        self.ratio_x100(r) <= self.max_ratio_x100
    }
    fn touches(self, out: &mut Vec<u32>) {
        out.push(u32::from(self.net.0));
    }
    fn headroom(self, r: &Routes) -> f32 {
        1.0 - self.ratio_x100(r) as f32 / self.max_ratio_x100.max(1) as f32
    }
    fn usage(self, r: &Routes) -> Option<f32> {
        Some(self.ratio_x100(r) as f32 / self.max_ratio_x100.max(1) as f32)
    }
    fn margin(self) -> f32 {
        f32::from(self.margin_pct) / 100.0
    }
    fn residual(self, r: &Routes) -> f32 {
        let budget = self.max_ratio_x100 as f32;
        crate::rule::over(self.ratio_x100(r) as f32 - budget, budget)
    }
}

impl Antenna {
    /// Metal/gate ratio ×100.
    ///
    /// ponytail: all routed metal on the net over its whole gate area — the
    /// cumulative, all-layers form, which over-estimates a per-layer rule.
    fn ratio_x100(self, r: &Routes) -> i32 {
        let area: i64 = r.shapes(self.net).iter().map(|s| i64::from(s.rect.w) * i64::from(s.rect.h)).sum();
        (area * 100 / self.gate_area_nm2.max(1)) as i32
    }
}
