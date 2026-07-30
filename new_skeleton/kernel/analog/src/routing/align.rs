//! Straight-net alignment (routing tier).
//!
//! Ported from `backend/constraints/src/routing_level/align.rs` (`StraightNet`).

use pnr_core::ids::NetId;
use pnr_core::routes::Routes;
use pnr_core::{BipartiteHypergraph, UnionFind};
use crate::placement::matching_pair::{is_diff_pair, D};
use crate::rule::Rule;

/// **Straight net.** Aligning a net's pins along the axis perpendicular to its
/// route lets the router emit one straight segment with no branch points —
/// lower parasitic RC and better signal integrity. A layout heuristic, not a
/// device-physics rule.
///
/// - **Enforcement:** [`crate::Mode::Cost`] (soft, priority 50).
/// - **Arity:** Net↔Device (pin alignment).
/// - **Books:** none in repo (routing optimisation heuristic).
#[derive(Clone, Copy)]
pub struct StraightNet {
    pub net: NetId,
    /// Route runs vertically (align pins in `x`); else horizontal (align in `y`).
    pub vertical: bool,
}

impl Rule for StraightNet {
    type On = Routes;
    /// Spread of the net's shapes along the axis perpendicular to the route — the
    /// backend's `reconcile_geometry` straight-net metric (`hi - lo`, in nm). A
    /// single straight segment spreads by its own width; branches/jogs widen it.
    /// 0 when the net is unrouted.
    fn cost(self, r: &Routes) -> f32 {
        let (mut lo, mut hi) = (i32::MAX, i32::MIN);
        for s in r.shapes(self.net) {
            let (a, b) = if self.vertical {
                (s.rect.x, s.rect.x + s.rect.w) // route vertical → spread in x
            } else {
                (s.rect.y, s.rect.y + s.rect.h)
            };
            lo = lo.min(a);
            hi = hi.max(b);
        }
        if lo > hi {
            0.0 // unrouted
        } else {
            (hi - lo) as f32
        }
    }

    /// **Recognition.** Aligning a net's pins lets the router emit one straight,
    /// low-parasitic segment — worth most on the matched **drain** nets of a
    /// differential pair, whose two outputs benefit from clean symmetric routes.
    /// Emits one `StraightNet` per diff-pair drain net.
    ///
    /// `vertical` picks the alignment axis; the hypergraph carries no geometry, so
    /// it defaults to horizontal (align pins in `y`) — the placer flips it if the
    /// route direction disagrees. Nets are de-duplicated across pairs.
    fn extract(hg: &BipartiteHypergraph, uf: &mut UnionFind) -> Vec<Self> {
        let _ = uf;
        let mut seen: Vec<NetId> = Vec::new();
        let mut out = Vec::new();
        let n = hg.device_count();
        for a in 0..n {
            for b in (a + 1)..n {
                if !is_diff_pair(hg, a, b) {
                    continue;
                }
                for net in [hg.device_nets[a][D], hg.device_nets[b][D]] {
                    if !seen.contains(&net) {
                        seen.push(net);
                        out.push(StraightNet { net, vertical: false });
                    }
                }
            }
        }
        out
    }
}
