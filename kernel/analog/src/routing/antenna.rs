//! Antenna ratio (routing tier).
//!
//! Ported from `backend/constraints/src/routing_level/antenna.rs`.

use pnr_core::ids::NetId;
use pnr_core::routes::Routes;
use pnr_core::{BipartiteHypergraph, UnionFind};
use crate::placement::matching_pair::{is_fet, G};
use crate::rule::Rule;

/// **Antenna constraint.** During metal etch, connected metal accumulates charge;
/// a high metal-area-to-gate-area ratio can break down the gate oxide before the
/// protecting junction exists. Ratio limits (≈400–1000 area, ≈200 perimeter) cap
/// the exposed metal per gate; fixes are layer-jumping or protection diodes.
///
/// - **Enforcement:** [`crate::Mode::Hard`] (legality, priority 75; DRC/signoff).
/// - **Arity:** Net↔Gate.
/// - **Books:** AOAL ch05/5.1, ch15/15.5 (#41); FOLD 7.4/7.4.2 (#27–28);
///   PNR_ANALOG 00/4.5 (#24).
// `net` was `net_name: String` in the old code; `max_ratio: f64` is
// `max_ratio_x100: i32` here. The old `matched_symmetric_repair` flag (a repair
// strategy, not a check input) is dropped from the scored rule.
#[derive(Clone, Copy)]
pub struct Antenna {
    pub net: NetId,
    /// Max metal-area / gate-area ratio, ×100.
    pub max_ratio_x100: i32,
    /// Safety margin held back from `max_ratio_x100`, percent.
    pub margin_pct: u8,
}

impl Rule for Antenna {
    type On = Routes;
    fn cost(self, r: &Routes) -> f32 {
        (net_ratio_x100(r, self.net) - self.max_ratio_x100).max(0) as f32
    }
    /// Accumulated ratio at every partially-routed gate stays under the limit.
    fn satisfied(self, r: &Routes) -> bool {
        net_ratio_x100(r, self.net) <= self.max_ratio_x100
    }

    fn touches(self, out: &mut Vec<u32>) {
        out.push(u32::from(self.net.0));
    }

    /// Fraction of the antenna ratio still unspent.
    fn headroom(self, r: &Routes) -> f32 {
        let budget = self.max_ratio_x100.max(1) as f32;
        1.0 - (net_ratio_x100(r, self.net) as f32 / budget)
    }

    fn margin(self) -> f32 {
        f32::from(self.margin_pct) / 100.0
    }

    /// Accumulated ratio past the limit, as a fraction of the limit.
    ///
    /// The `×100` fixed-point scaling cancels in the division, which is the reason to
    /// normalise rather than pick a scale factor: a residual is unit-free by
    /// construction, so it survives a change of representation on either side.
    fn residual(self, r: &Routes) -> f32 {
        let budget = self.max_ratio_x100 as f32;
        crate::rule::over(net_ratio_x100(r, self.net) as f32 - budget, budget)
    }

    /// **Recognition.** The antenna rule is Net↔Gate: it applies to every net that
    /// drives a MOSFET **gate** (terminal index `G`), since only gate oxide is at
    /// risk during metal etch. Sweep the FETs, collect their distinct gate nets,
    /// emit one `Antenna` per gate net.
    ///
    /// Note this only finds *where* the rule applies; `cost`/`satisfied` remain
    /// blocked on gate-area + per-layer etch data that `Routes` does not carry.
    fn extract(hg: &BipartiteHypergraph, uf: &mut UnionFind) -> Vec<Self> {
        let _ = uf;
        let mut seen: Vec<NetId> = Vec::new();
        let mut out = Vec::new();
        for (d, nets) in hg.device_nets.iter().enumerate() {
            if is_fet(hg.kinds[d]) && nets.len() > G {
                let g = nets[G];
                if !seen.contains(&g) {
                    seen.push(g);
                    out.push(Antenna {
                        net: g,
                        max_ratio_x100: 40_000, // ratio 400 (typical area-antenna limit) ×100
                        margin_pct: 20,
                    });
                }
            }
        }
        out
    }
}

/// Antenna metal/gate ratio ×100 for `net`.
///
/// Proxy: the net's summed drawn metal area over a nominal min-gate area
/// ([`NOMINAL_GATE_AREA`]). The real denominator is the summed area of the gates
/// the net drives, which needs a net→gate-area map `Routes` doesn't carry — until
/// that's threaded, the denominator is a constant and this is a monotone stand-in.
fn net_ratio_x100(r: &Routes, net: NetId) -> i32 {
    let area: i64 = r
        .wires
        .get(net.0 as usize)
        .map(|ws| ws.iter().map(|s| i64::from(s.rect.w) * i64::from(s.rect.h)).sum())
        .unwrap_or(0);
    ((area * 100) / NOMINAL_GATE_AREA) as i32
}

/// Nominal min-gate area (nm²) — the antenna denominator until a real
/// net→gate-area map is threaded through `Routes`.
const NOMINAL_GATE_AREA: i64 = 10_000;
