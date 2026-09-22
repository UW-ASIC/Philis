//! Per-net parasitic budget (routing tier).

use pnr_core::ids::NetId;
use pnr_core::routes::Routes;
use pnr_core::{BipartiteHypergraph, UnionFind};
use crate::rule::Rule;

/// Per-net R budget, lowered (by the annotator) to a drawn-length cap the
/// router can check.
#[derive(Clone, Copy)]
pub struct ParasiticBudget {
    pub net: NetId,
    /// Drawn length at which the R/C budget is spent, nm — what is checked.
    pub max_len_nm: i64,
    /// Safety margin on `max_len_nm`, percent.
    pub margin_pct: u8,
}

impl Rule for ParasiticBudget {
    type On = Routes;
    /// `len²·1e-6`.
    ///
    /// ponytail: `1e-6` is a hand-tuned weight; `(len/max_len)²` is the
    /// unit-free form, but `gr`/`dr` blend this cost, so changing it re-weights
    /// their PEX tier — re-baseline routing fixtures in the same commit.
    fn cost(self, r: &Routes) -> f32 {
        let len = r.length(self.net) as f32;
        len * len * 1e-6
    }
    /// Unrouted (zero length) passes: that is an open, not a parasitic miss.
    fn satisfied(self, r: &Routes) -> bool {
        r.length(self.net) <= self.max_len_nm
    }
    fn touches(self, out: &mut Vec<u32>) {
        out.push(u32::from(self.net.0));
    }
    fn headroom(self, r: &Routes) -> f32 {
        1.0 - r.length(self.net) as f32 / self.max_len_nm.max(1) as f32
    }
    fn margin(self) -> f32 {
        f32::from(self.margin_pct) / 100.0
    }
    /// Against the raw `max_len_nm`, not the derated target.
    fn residual(self, r: &Routes) -> f32 {
        let budget = self.max_len_nm as f32;
        crate::rule::over(r.length(self.net) as f32 - budget, budget)
    }

    /// One per net with ≥ 2 device terminals.
    ///
    /// ponytail: placeholder budgets (1 kΩ, 100 fF, 1 mm); the real lowering
    /// is `max_r / sheet_r` per layer from the PDK.
    fn extract(hg: &BipartiteHypergraph, _uf: &mut UnionFind) -> Vec<Self> {
        hg.net_devices
            .iter()
            .enumerate()
            .filter(|(_, devs)| devs.len() >= 2)
            .map(|(n, _)| ParasiticBudget {
                net: NetId(n as u16),
                max_len_nm: 1_000_000,
                margin_pct: 20,
            })
            .collect()
    }
}
