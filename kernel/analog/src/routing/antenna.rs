//! Antenna ratio (routing tier).

use pnr_core::ids::NetId;
use pnr_core::routes::Routes;
use pnr_core::{BipartiteHypergraph, UnionFind};
use crate::placement::matching_pair::{is_fet, G};
use crate::rule::Rule;

/// Metal-area / gate-area ratio on a gate net stays under the limit.
#[derive(Clone, Copy)]
pub struct Antenna {
    pub net: NetId,
    /// Max ratio ×100.
    pub max_ratio_x100: i32,
    /// Safety margin, percent.
    pub margin_pct: u8,
}

impl Rule for Antenna {
    type On = Routes;
    fn cost(self, r: &Routes) -> f32 {
        (net_ratio_x100(r, self.net) - self.max_ratio_x100).max(0) as f32
    }
    fn satisfied(self, r: &Routes) -> bool {
        net_ratio_x100(r, self.net) <= self.max_ratio_x100
    }
    fn touches(self, out: &mut Vec<u32>) {
        out.push(u32::from(self.net.0));
    }
    fn headroom(self, r: &Routes) -> f32 {
        1.0 - net_ratio_x100(r, self.net) as f32 / self.max_ratio_x100.max(1) as f32
    }
    fn usage(self, r: &Routes) -> Option<f32> {
        Some(net_ratio_x100(r, self.net) as f32 / self.max_ratio_x100.max(1) as f32)
    }
    fn margin(self) -> f32 {
        f32::from(self.margin_pct) / 100.0
    }
    fn residual(self, r: &Routes) -> f32 {
        let budget = self.max_ratio_x100 as f32;
        crate::rule::over(net_ratio_x100(r, self.net) as f32 - budget, budget)
    }

    /// One per distinct FET gate net, limit ratio 400.
    fn extract(hg: &BipartiteHypergraph, _uf: &mut UnionFind) -> Vec<Self> {
        let mut seen: Vec<NetId> = Vec::new();
        for (d, nets) in hg.device_nets.iter().enumerate() {
            if is_fet(hg.kinds[d]) && nets.len() > G && !seen.contains(&nets[G]) {
                seen.push(nets[G]);
            }
        }
        seen.into_iter().map(|net| Antenna { net, max_ratio_x100: 40_000, margin_pct: 20 }).collect()
    }
}

/// Metal/gate ratio ×100 for `net`.
///
/// ponytail: the denominator is a constant nominal gate area, not the real
/// driven gate area (needs a net→gate-area map `Routes` lacks). Monotone proxy.
fn net_ratio_x100(r: &Routes, net: NetId) -> i32 {
    let area: i64 = r.shapes(net).iter().map(|s| i64::from(s.rect.w) * i64::from(s.rect.h)).sum();
    ((area * 100) / NOMINAL_GATE_AREA) as i32
}

/// Nominal min-gate area, nm².
const NOMINAL_GATE_AREA: i64 = 10_000;
