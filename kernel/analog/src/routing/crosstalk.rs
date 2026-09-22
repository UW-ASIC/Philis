//! Crosstalk exclusion (routing tier).

use pnr_core::geom::Rect;
use pnr_core::ids::NetId;
use pnr_core::routes::Routes;
use pnr_core::{BipartiteHypergraph, UnionFind};
use crate::placement::matching_pair::{is_diff_pair, D, G};
use crate::rule::Rule;

/// Same-layer clearance between nets `a` and `b` ≥ `min_spacing_nm`.
/// Nets sharing no layer do not couple and always pass.
#[derive(Clone, Copy)]
pub struct CrosstalkExclusion {
    pub a: NetId,
    pub b: NetId,
    pub min_spacing_nm: i32,
    /// Safety margin on the floor, percent.
    pub margin_pct: u8,
}

impl CrosstalkExclusion {
    /// Min same-layer edge gap between the nets' shapes, nm; `f32::MAX` when
    /// they share no layer. ponytail: O(|A|·|B|), analog nets are short.
    fn clearance(self, r: &Routes) -> f32 {
        let mut best = f32::MAX;
        for wa in r.shapes(self.a) {
            for wb in r.shapes(self.b) {
                if wa.layer == wb.layer {
                    best = best.min(rect_gap(&wa.rect, &wb.rect));
                }
            }
        }
        best
    }
}

/// Euclidean edge-to-edge gap between two rects (`0` if touching/overlapping).
fn rect_gap(p: &Rect, q: &Rect) -> f32 {
    let dx = (q.x - (p.x + p.w)).max(p.x - (q.x + q.w)).max(0);
    let dy = (q.y - (p.y + p.h)).max(p.y - (q.y + q.h)).max(0);
    ((dx as f32).powi(2) + (dy as f32).powi(2)).sqrt()
}

impl Rule for CrosstalkExclusion {
    type On = Routes;
    /// Spacing shortfall, nm.
    fn cost(self, r: &Routes) -> f32 {
        let d = self.clearance(r);
        if d == f32::MAX {
            return 0.0;
        }
        (self.min_spacing_nm as f32 - d).max(0.0)
    }
    fn satisfied(self, r: &Routes) -> bool {
        self.clearance(r) >= self.min_spacing_nm as f32
    }
    fn touches(self, out: &mut Vec<u32>) {
        out.push(u32::from(self.a.0));
        out.push(u32::from(self.b.0));
    }
    /// `(d − floor) / floor`; `1.0` when the nets share no layer.
    fn headroom(self, r: &Routes) -> f32 {
        let floor = self.min_spacing_nm.max(1) as f32;
        let d = self.clearance(r);
        if d == f32::MAX {
            return 1.0;
        }
        (d - floor) / floor
    }
    fn margin(self) -> f32 {
        f32::from(self.margin_pct) / 100.0
    }
    fn residual(self, r: &Routes) -> f32 {
        let d = self.clearance(r);
        if d == f32::MAX {
            return 0.0;
        }
        let floor = self.min_spacing_nm as f32;
        crate::rule::over(floor - d, floor)
    }

    /// Per diff pair: each input gate net against each output drain net
    /// (the feedback path), 400 nm floor.
    ///
    /// ponytail: clock aggressors need `NetClassification`, which the
    /// hypergraph does not carry.
    fn extract(hg: &BipartiteHypergraph, _uf: &mut UnionFind) -> Vec<Self> {
        let mut out = Vec::new();
        let n = hg.device_count();
        for a in 0..n {
            for b in (a + 1)..n {
                if !is_diff_pair(hg, a, b) {
                    continue;
                }
                for g in [hg.device_nets[a][G], hg.device_nets[b][G]] {
                    for d in [hg.device_nets[a][D], hg.device_nets[b][D]] {
                        if g != d {
                            out.push(CrosstalkExclusion { a: g, b: d, min_spacing_nm: 400, margin_pct: 25 });
                        }
                    }
                }
            }
        }
        out
    }
}
