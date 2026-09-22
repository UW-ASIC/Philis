//! Straight-net alignment (routing tier).

use pnr_core::ids::NetId;
use pnr_core::routes::Routes;
use pnr_core::{BipartiteHypergraph, UnionFind};
use crate::placement::matching_pair::{is_diff_pair, D};
use crate::rule::Rule;

/// Prefer `net` routed as one straight segment (low RC). Objective only.
#[derive(Clone, Copy)]
pub struct StraightNet {
    pub net: NetId,
    /// Route runs vertically (spread measured in x); else horizontally (in y).
    pub vertical: bool,
}

impl Rule for StraightNet {
    type On = Routes;
    /// Spread of the net's shapes across the route axis, nm; `0` when unrouted.
    fn cost(self, r: &Routes) -> f32 {
        let (mut lo, mut hi) = (i32::MAX, i32::MIN);
        for s in r.shapes(self.net) {
            let (a, b) = if self.vertical { (s.rect.x, s.rect.x + s.rect.w) } else { (s.rect.y, s.rect.y + s.rect.h) };
            lo = lo.min(a);
            hi = hi.max(b);
        }
        if lo > hi {
            0.0
        } else {
            (hi - lo) as f32
        }
    }

    /// One per distinct diff-pair drain net, horizontal by default.
    fn extract(hg: &BipartiteHypergraph, _uf: &mut UnionFind) -> Vec<Self> {
        let mut seen: Vec<NetId> = Vec::new();
        let n = hg.device_count();
        for a in 0..n {
            for b in (a + 1)..n {
                if !is_diff_pair(hg, a, b) {
                    continue;
                }
                for net in [hg.device_nets[a][D], hg.device_nets[b][D]] {
                    if !seen.contains(&net) {
                        seen.push(net);
                    }
                }
            }
        }
        seen.into_iter().map(|net| StraightNet { net, vertical: false }).collect()
    }
}
