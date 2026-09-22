//! Differential-pair route matching (routing tier).

use pnr_core::ids::NetId;
use pnr_core::routes::Routes;
use pnr_core::{BipartiteHypergraph, UnionFind};
use crate::placement::matching_pair::{is_diff_pair, D};
use crate::rule::Rule;

/// `pos`/`neg` routes match in length and, if required, in layer set.
///
/// ponytail: no R/C matching (needs extracted parasitics) and no `project` —
/// a layer mismatch is fixed by targeted rip-up of both nets in `dr`.
#[derive(Clone, Copy)]
pub struct Differential {
    pub pos: NetId,
    pub neg: NetId,
    /// Max length mismatch, percent ×10.
    pub max_len_delta_pct10: i32,
    pub same_layer_required: bool,
}

impl Differential {
    /// `|a − b| / mean · 100`; `0` when both are empty.
    fn len_delta_pct(self, r: &Routes) -> f32 {
        let a = r.length(self.pos) as f32;
        let b = r.length(self.neg) as f32;
        let avg = (a + b) / 2.0;
        if avg > 0.0 {
            (a - b).abs() / avg * 100.0
        } else {
            0.0
        }
    }

    fn layers_match(self, r: &Routes) -> bool {
        let layers = |n: NetId| {
            let mut v: Vec<u16> = r.shapes(n).iter().map(|s| s.layer.0).collect();
            v.sort_unstable();
            v.dedup();
            v
        };
        layers(self.pos) == layers(self.neg)
    }

    fn budget_pct(self) -> f32 {
        self.max_len_delta_pct10 as f32 / 10.0
    }
}

impl Rule for Differential {
    type On = Routes;
    fn touches(self, out: &mut Vec<u32>) {
        out.push(u32::from(self.pos.0));
        out.push(u32::from(self.neg.0));
    }
    /// Percent length delta past budget. Layer mismatch is not scored here.
    fn cost(self, r: &Routes) -> f32 {
        (self.len_delta_pct(r) - self.budget_pct()).max(0.0)
    }
    fn satisfied(self, r: &Routes) -> bool {
        self.len_delta_pct(r) <= self.budget_pct() && (!self.same_layer_required || self.layers_match(r))
    }
    /// max(length overshoot fraction, `1.0` on a required-layer mismatch).
    fn residual(self, r: &Routes) -> f32 {
        let budget = self.budget_pct();
        let len = crate::rule::over(self.len_delta_pct(r) - budget, budget);
        len.max(f32::from(self.same_layer_required && !self.layers_match(r)))
    }

    /// One per diff pair: its two drain nets, 5% budget, same layers.
    fn extract(hg: &BipartiteHypergraph, _uf: &mut UnionFind) -> Vec<Self> {
        let mut out = Vec::new();
        let n = hg.device_count();
        for a in 0..n {
            for b in (a + 1)..n {
                if is_diff_pair(hg, a, b) {
                    out.push(Differential {
                        pos: hg.device_nets[a][D],
                        neg: hg.device_nets[b][D],
                        max_len_delta_pct10: 50,
                        same_layer_required: true,
                    });
                }
            }
        }
        out
    }
}
