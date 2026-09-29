//! Per-net parasitic budget (routing tier).

use pnr_core::ids::NetId;
use pnr_core::routes::Routes;
use pnr_core::{BipartiteHypergraph, UnionFind};
use crate::rule::Rule;
use super::Stack;

/// Per-net ground-capacitance budget (NET-02): the routed net's C, measured
/// per layer from the deck's `pex` (area + fringe; Lampaert 1999 eqs.
/// 2.29–2.32), stays within `max_c_af`. Without the stack or a C budget it
/// falls back to the drawn-length cap the annotator lowered it to.
///
/// ponytail: C only; series R (NET-01, Lampaert eq. 2.33 `L ≤ (R_max −
/// R_via)·W/R□`) needs an R budget the annotator does not derive.
#[derive(Clone, Copy)]
pub struct ParasiticBudget {
    pub net: NetId,
    /// Drawn length at which the budget is spent, nm — the fallback check.
    pub max_len_nm: i64,
    /// Ground-C budget, aF; `0` = check length.
    pub max_c_af: i64,
    /// Safety margin on the budget, percent.
    pub margin_pct: u8,
    /// The routing stack's per-layer parasitics; `None` = check length.
    pub stack: Option<&'static Stack>,
}

impl ParasiticBudget {
    /// `(spent, budget)`: extracted C in aF when both are known, else length.
    fn spent(self, r: &Routes) -> (f32, f32) {
        match self.stack {
            Some(s) if self.max_c_af > 0 => (s.ground_af(r.shapes(self.net)), self.max_c_af as f32),
            _ => (r.length(self.net) as f32, self.max_len_nm as f32),
        }
    }
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
    /// Unrouted (zero length) is satisfied for search — an open is LVS's to
    /// catch — but [`Rule::known`] keeps it out of any certificate.
    fn satisfied(self, r: &Routes) -> bool {
        let (spent, budget) = self.spent(r);
        spent <= budget
    }
    fn known(self, r: &Routes) -> bool {
        r.length(self.net) > 0
    }
    fn touches(self, out: &mut Vec<u32>) {
        out.push(u32::from(self.net.0));
    }
    fn headroom(self, r: &Routes) -> f32 {
        let (spent, budget) = self.spent(r);
        1.0 - spent / budget.max(1.0)
    }
    fn usage(self, r: &Routes) -> Option<f32> {
        let (spent, budget) = self.spent(r);
        Some(spent / budget.max(1.0))
    }
    fn margin(self) -> f32 {
        f32::from(self.margin_pct) / 100.0
    }
    /// Against the raw budget, not the derated target.
    fn residual(self, r: &Routes) -> f32 {
        let (spent, budget) = self.spent(r);
        crate::rule::over(spent - budget, budget)
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
                max_c_af: 0,
                margin_pct: 20,
                stack: None,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing::stack::Layer;
    use pnr_core::geom::{LayerId, Rect, Shape};

    /// The budget is capacitance from the deck's per-layer pex: the same 10 µm
    /// run passes on a low-C upper metal and fails on a high-C lower one; with
    /// no stack it is the length cap.
    #[test]
    fn the_budget_is_extracted_c_per_layer() {
        let stack: &'static Stack = Box::leak(Box::new(Stack {
            layers: vec![
                Layer { id: 0, area_af_um2: 40.0, fringe_af_um: 40.0, ..Layer::default() },
                Layer { id: 1, area_af_um2: 10.0, fringe_af_um: 20.0, ..Layer::default() },
            ],
            antenna_cumulative: false,
        diode_layer: None,
        }));
        let run = |layer| Routes { wires: vec![vec![Shape { layer: LayerId(layer), rect: Rect { x: 0, y: 0, w: 10_000, h: 500 } }]], ..Default::default()  };
        // m0: 40·5 + 2·40·10 = 1000 aF; m1: 10·5 + 2·20·10 = 450 aF.
        let b = ParasiticBudget { net: NetId(0), max_len_nm: 1_000_000, max_c_af: 600, margin_pct: 20, stack: Some(stack) };
        assert!(!b.satisfied(&run(0)) && b.satisfied(&run(1)));
        assert!((b.usage(&run(1)).unwrap() - 0.75).abs() < 1e-4);
        let length_only = ParasiticBudget { stack: None, max_len_nm: 5_000, ..b };
        assert!(!length_only.satisfied(&run(1)), "falls back to the length cap");
    }
}
