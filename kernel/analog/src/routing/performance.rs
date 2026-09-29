//! One circuit spec as a shared parasitic budget over the routed nets
//! (routing tier, budget).

use pnr_core::ids::NetId;
use pnr_core::routes::Routes;
use crate::rule::RuleBatch;

/// `Σ_i w_i · C_i ≤ 1`: the spec's linearised miss, where `C_i` is net `i`'s
/// routed capacitance (drawn length × `af_per_nm`) and `w_i = −(∂f/∂C_i) /
/// headroom` from finite-difference sensitivities at the schematic operating
/// point. One row per spec, so nets **share** the spec's margin: a node the
/// metric barely feels may take C another cannot — what independent per-net
/// caps cannot express (they reject feasible trades and double-spend).
///
/// Signs are kept: a net whose C helps the metric earns credit. The model is
/// linear around the schematic point; the post-layout simulation remains the
/// judge (`library::perf`), this is the router's cheap guide to it.
#[derive(Clone)]
pub struct PerformanceBudget {
    /// The spec's metric, for reports.
    pub metric: String,
    pub nets: Vec<NetId>,
    /// Per-net weight, 1/aF (fraction of the headroom one aF spends).
    pub weights: Vec<f32>,
    /// Routed capacitance per nm of wire, aF.
    pub af_per_nm: f32,
}

impl PerformanceBudget {
    /// Spent fraction of the spec's headroom.
    fn used(&self, r: &Routes) -> f32 {
        self.nets.iter().zip(&self.weights).map(|(&n, &w)| w * r.length(n) as f32 * self.af_per_nm).sum()
    }
}

impl RuleBatch<Routes> for PerformanceBudget {
    fn cost(&self, r: &Routes) -> f32 {
        self.residual(r) as f32
    }
    fn violations(&self, r: &Routes) -> u32 {
        u32::from(self.used(r) > 1.0)
    }
    fn residual(&self, r: &Routes) -> f64 {
        f64::from((self.used(r) - 1.0).max(0.0))
    }
    fn kind(&self) -> &'static str {
        "PerformanceBudget"
    }
    fn count(&self) -> usize {
        1
    }
    fn criticality(&self, r: &Routes) -> f32 {
        self.used(r).clamp(0.0, 1.0)
    }
    fn worst_usage(&self, r: &Routes) -> Option<f32> {
        Some(self.used(r))
    }
    fn violating_ids(&self, r: &Routes, out: &mut Vec<u32>) {
        if self.used(r) > 1.0 {
            self.touched(out);
        }
    }
    /// The nets that spend the margin (positive weight).
    fn touched(&self, out: &mut Vec<u32>) {
        out.extend(self.nets.iter().zip(&self.weights).filter(|(_, &w)| w > 0.0).map(|(n, _)| u32::from(n.0)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_core::geom::{LayerId, Rect, Shape};

    fn wire(len: i32) -> Vec<Shape> {
        vec![Shape { layer: LayerId(0), rect: Rect { x: 0, y: 0, w: len, h: 10 } }]
    }

    /// The trade independent caps cannot make: a sensitive net and a numb one
    /// share one margin, so the numb one may run long.
    #[test]
    fn nets_share_one_margin_by_sensitivity() {
        // 1 aF/nm. Net 0 spends 1% of the margin per aF, net 1 0.01%.
        let b = PerformanceBudget { metric: "ugf".into(), nets: vec![NetId(0), NetId(1)], weights: vec![1e-2, 1e-4], af_per_nm: 1.0 };
        // 50 aF on the sensitive net + 4000 aF on the numb one: 0.5 + 0.4.
        let ok = Routes { wires: vec![wire(50), wire(4_000)], ..Default::default()  };
        assert!(b.violations(&ok) == 0 && (b.worst_usage(&ok).unwrap() - 0.9).abs() < 1e-4);
        // Swap them and the same total C blows the spec 40×.
        let bad = Routes { wires: vec![wire(4_000), wire(50)], ..Default::default()  };
        assert_eq!(b.violations(&bad), 1);
        assert!(b.residual(&bad) > 30.0);
        let mut ids = Vec::new();
        b.violating_ids(&bad, &mut ids);
        assert_eq!(ids, vec![0, 1]);
    }
}
