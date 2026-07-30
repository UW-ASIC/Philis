//! Crosstalk exclusion (routing tier).
//!
//! Ported from `backend/constraints/src/routing_level/crosstalk.rs`.

use pnr_core::geom::Rect;
use pnr_core::ids::NetId;
use pnr_core::routes::Routes;
use pnr_core::{BipartiteHypergraph, UnionFind};
use crate::placement::matching_pair::{is_diff_pair, D, G};
use crate::rule::Rule;

/// **Crosstalk exclusion.** Coupling capacitance between adjacent wires injects
/// noise into a victim: `ΔV = (Cc/Ctot)·ΔV_aggressor`. `Cc` grows as wires run
/// close and parallel, so a minimum run-adjacent spacing bounds the injected
/// noise. Clocks are the worst aggressors; the spacing floor varies by layer and
/// whether a shield is interposed.
///
/// - **Enforcement:** [`crate::Mode::Hard`] (legality, priority 70).
/// - **Arity:** Net↔Net (bidirectional aggressor/victim pair).
/// - **Books:** AOAL ch02/2.7.6, ch06/6.3, ch15/15.4 (#43); FOLD 7.3/7.3.3 (#22);
///   ALS 4.4/4.4.3 (#13); PNR_ANALOG 02/5.A.2 (#55).
// `a`/`b` were `net_a`/`net_b` String net names in the old constraint code;
// `min_spacing_um: f64` there is `min_spacing_nm: i32` here (nm, integer data).
#[derive(Clone, Copy)]
pub struct CrosstalkExclusion {
    pub a: NetId,
    pub b: NetId,
    pub min_spacing_nm: i32,
    /// Safety margin held back from `min_spacing_nm`, percent. The raw floor is
    /// the terminal hard limit; the router is pulled to keep extra clearance.
    pub margin_pct: u8,
}

impl CrosstalkExclusion {
    /// Minimum same-layer clearance (nm) between the two nets' drawn shapes.
    /// Mirrors `engine::routing::net_pair_clearance`: only shapes on a shared
    /// layer couple, so cross-layer pairs are skipped. `f32::MAX` when the nets
    /// share no layer (nothing adjacent → no coupling).
    /// ponytail: O(|A|·|B|) rect scan — analog nets are short (as in backend).
    fn clearance(self, r: &Routes) -> f32 {
        let sa = r.shapes(self.a);
        let sb = r.shapes(self.b);
        let mut best = f32::MAX;
        for wa in sa {
            for wb in sb {
                if wa.layer != wb.layer {
                    continue;
                }
                best = best.min(rect_gap(&wa.rect, &wb.rect));
            }
        }
        best
    }
}

/// Edge-to-edge gap between two axis-aligned rects (0 if they overlap/touch).
/// Same distance family as the backend's centre-to-centre node scan, but on
/// drawn extents rather than grid nodes.
fn rect_gap(p: &Rect, q: &Rect) -> f32 {
    let dx = (q.x - (p.x + p.w)).max(p.x - (q.x + q.w)).max(0);
    let dy = (q.y - (p.y + p.h)).max(p.y - (q.y + q.h)).max(0);
    ((dx as f32).powi(2) + (dy as f32).powi(2)).sqrt()
}

impl Rule for CrosstalkExclusion {
    type On = Routes;
    /// Spacing shortfall in nm (0 once clearance meets the floor). The backend
    /// feeds the same `min_nm - d` shortfall back as placement pressure.
    fn cost(self, r: &Routes) -> f32 {
        let d = self.clearance(r);
        if d == f32::MAX {
            return 0.0; // no shared layer → not adjacent
        }
        (self.min_spacing_nm as f32 - d).max(0.0)
    }
    /// Minimum run-adjacent spacing between the two nets ≥ `min_spacing_nm`.
    fn satisfied(self, r: &Routes) -> bool {
        self.clearance(r) >= self.min_spacing_nm as f32
    }

    fn touches(self, out: &mut Vec<u32>) {
        out.push(u32::from(self.a.0));
        out.push(u32::from(self.b.0));
    }

    /// Fraction of the spacing budget still in hand: `0.0` right at the floor,
    /// `1.0` once the pair is a full extra floor-width apart. Coupling falls off
    /// with distance, so there is no value in scoring beyond that.
    fn headroom(self, r: &Routes) -> f32 {
        let floor = self.min_spacing_nm.max(1) as f32;
        let d = self.clearance(r);
        if d == f32::MAX {
            return 1.0; // no shared layer → no coupling → all the slack there is
        }
        (d - floor) / floor
    }

    fn margin(self) -> f32 {
        f32::from(self.margin_pct) / 100.0
    }

    /// Spacing shortfall as a fraction of the floor — the same nm number [`cost`](Rule::cost)
    /// returns, divided by the spec that made it a violation.
    ///
    /// This is the pair that makes Θ summable in practice. `cost` here is nm and
    /// `Antenna::cost` is a bare ratio ×100; added raw, a 400 nm spacing miss outweighs a
    /// 4× antenna blowout by two orders of magnitude for no reason but the choice of unit.
    /// Both `residual`s are "fraction of my own spec", so `1.0` means the same severity on
    /// either side (D17).
    ///
    /// Nets that share no layer do not couple, so there is no shortfall to report — the
    /// same `f32::MAX` early-out `cost` and `headroom` take.
    fn residual(self, r: &Routes) -> f32 {
        let d = self.clearance(r);
        if d == f32::MAX {
            return 0.0;
        }
        let floor = self.min_spacing_nm as f32;
        crate::rule::over(floor - d, floor)
    }

    /// **Recognition.** Coupling from an aggressor to a sensitive victim is bounded
    /// by run-adjacent spacing. In a differential stage the input **gate** nets are
    /// the sensitive small-signal victims, and the output **drain** nets are the
    /// large-swing aggressors; letting the output couple back onto the input is the
    /// classic feedback/oscillation path. So per recognised diff pair, emit
    /// exclusion between each of its input gate nets and each of its output drain
    /// nets (Net↔Net), skipping any pair on the same net.
    // TODO: clocks are the worst aggressors, but flagging a *clock* net needs
    // `NetClassification`, absent from the hypergraph — only the diff-stage
    // input↔output pairs are recognised structurally here.
    fn extract(hg: &BipartiteHypergraph, uf: &mut UnionFind) -> Vec<Self> {
        let _ = uf;
        let mut out = Vec::new();
        let n = hg.device_count();
        for a in 0..n {
            for b in (a + 1)..n {
                if !is_diff_pair(hg, a, b) {
                    continue;
                }
                let gates = [hg.device_nets[a][G], hg.device_nets[b][G]];
                let drains = [hg.device_nets[a][D], hg.device_nets[b][D]];
                for &g in &gates {
                    for &d in &drains {
                        if g != d {
                            out.push(CrosstalkExclusion {
                                a: g,
                                b: d,
                                min_spacing_nm: 400, // ~2× min metal pitch floor
                                margin_pct: 25,
                            });
                        }
                    }
                }
            }
        }
        out
    }
}
