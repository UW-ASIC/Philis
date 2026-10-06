//! One circuit spec as a shared parasitic budget over the routed nets
//! (routing tier, budget).

use pnr_core::ids::NetId;
use pnr_core::routes::Routes;
use crate::rule::RuleBatch;

/// `Σ_i w_i · C_i ≤ limit`: one spec bound's linearised miss, where `C_i` is
/// net `i`'s routed capacitance (drawn length × `af_per_nm`) and `w_i =
/// sign·(∂f/∂C_i) / headroom` from finite-difference sensitivities at the
/// schematic operating point (`sign` −1 for a floor, +1 for a ceiling). One row
/// per bound, so nets **share** the bound's margin: a node the metric barely
/// feels may take C another cannot — what independent per-net caps cannot
/// express (they reject feasible trades and double-spend). A bound the
/// schematic already misses has `limit = 0` and `w_i = sign·(∂f/∂C_i) /
/// |bound|`: any adverse C is a residual (do not worsen, GRAEB-11).
///
/// Helpful terms are dropped unless `Policy.credit_helpful` (BAL2-16): a net
/// whose C helps the metric earns no credit by default. The model is linear
/// around the schematic point; the post-layout simulation remains the judge
/// (`library::perf`), this is the router's cheap guide to it.
///
/// Master §6 C12: the R, differential and coupling terms are RTE-21's; EXT-25
/// fills `r_*` and `coupling` (PERF-12 step 3) and leaves `diff_pairs` empty.
/// Until RTE-21 measures them a row carrying any of them reads
/// [`Self::unknown`], and `used` prices the ground-C part only.
#[derive(Clone)]
pub struct PerformanceBudget {
    /// The spec's metric, for reports.
    pub metric: String,
    /// Nets whose ground C spends the bound, parallel to `weights`.
    pub nets: Vec<NetId>,
    /// Per-net weight, 1/aF (fraction of the headroom one aF spends).
    pub weights: Vec<f32>,
    /// Routed capacitance per nm of wire, aF.
    pub af_per_nm: f32,
    /// `1.0` for a bound with headroom; `0.0` = do-not-worsen row.
    /// `residual = (used − limit).max(0)`.
    pub limit: f32,
    /// Nets whose series resistance spends the bound.
    pub r_nets: Vec<NetId>,
    /// Per-net weight, 1/Ω.
    pub r_weights: Vec<f32>,
    /// Net pairs whose ground-C imbalance spends the bound.
    pub diff_pairs: Vec<(NetId, NetId)>,
    /// Per-pair weight, 1/aF on `|C_a − C_b|`.
    pub diff_weights: Vec<f32>,
    /// `(a, b, w)`: weight 1/aF on the coupling C_ab (PERF-12 step 3).
    pub coupling: Vec<(NetId, NetId, f32)>,
}

impl PerformanceBudget {
    /// A ground-C-only row with headroom (`limit` 1, every other term empty);
    /// RTE-21 reuses it.
    #[must_use]
    pub fn ground_c(metric: String, nets: Vec<NetId>, weights: Vec<f32>, af_per_nm: f32) -> Self {
        Self { metric, nets, weights, af_per_nm, limit: 1.0, r_nets: Vec::new(), r_weights: Vec::new(), diff_pairs: Vec::new(), diff_weights: Vec::new(), coupling: Vec::new() }
    }

    /// Spent fraction of the bound's headroom (of `|bound|` when `limit = 0`):
    /// `Σ w_i · length_i · af_per_nm` over the ground-C terms only. Pairs past
    /// the shorter of `nets` / `weights` are ignored.
    fn used(&self, r: &Routes) -> f32 {
        self.nets.iter().zip(&self.weights).map(|(&n, &w)| w * r.length(n) as f32 * self.af_per_nm).sum()
    }
}

impl RuleBatch<Routes> for PerformanceBudget {
    fn cost(&self, r: &Routes) -> f32 {
        self.residual(r) as f32
    }
    fn violations(&self, r: &Routes) -> u32 {
        u32::from(self.used(r) > self.limit)
    }
    fn residual(&self, r: &Routes) -> f64 {
        f64::from((self.used(r) - self.limit).max(0.0))
    }
    /// `1` while the row carries R, differential or coupling terms nobody
    /// measures yet (RTE-21 replaces this once `stack` measures them).
    fn unknown(&self, _r: &Routes) -> u32 {
        u32::from(!self.r_nets.is_empty() || !self.diff_pairs.is_empty() || !self.coupling.is_empty())
    }
    fn kind(&self) -> &'static str {
        "PerformanceBudget"
    }
    fn repair_kind(&self) -> crate::RepairKind {
        crate::RepairKind::Budget
    }
    fn count(&self) -> usize {
        1
    }
    fn criticality(&self, r: &Routes) -> f32 {
        (self.used(r) / self.limit.max(1e-6)).clamp(0.0, 1.0)
    }
    /// Spent fraction of the headroom; `None` for a zero-limit row, which has
    /// no budget to spend ([`crate::rule::Rule::usage`]).
    fn worst_usage(&self, r: &Routes) -> Option<f32> {
        (self.limit > 0.0).then(|| self.used(r) / self.limit)
    }
    fn violating_ids(&self, r: &Routes, out: &mut Vec<u32>) {
        if self.used(r) > self.limit {
            self.touched(out);
        }
    }
    /// The nets that spend the margin (positive weight): ground C, series R
    /// and both nets of a coupling term.
    fn touched(&self, out: &mut Vec<u32>) {
        out.extend(self.nets.iter().zip(&self.weights).chain(self.r_nets.iter().zip(&self.r_weights)).filter(|(_, &w)| w > 0.0).map(|(n, _)| u32::from(n.0)));
        out.extend(self.coupling.iter().filter(|c| c.2 > 0.0).flat_map(|c| [u32::from(c.0 .0), u32::from(c.1 .0)]));
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
        let b = PerformanceBudget::ground_c("ugf".into(), vec![NetId(0), NetId(1)], vec![1e-2, 1e-4], 1.0);
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

    /// A bound the schematic already misses: any adverse C violates.
    #[test]
    fn a_zero_limit_row_violates_on_any_adverse_c() {
        let b = PerformanceBudget { limit: 0.0, ..PerformanceBudget::ground_c("gain:min".into(), vec![NetId(0)], vec![0.1], 1.0) };
        // One net routed 1 nm (`length` is the long side).
        let r = Routes { wires: vec![vec![Shape { layer: LayerId(0), rect: Rect { x: 0, y: 0, w: 1, h: 1 } }]], ..Default::default() };
        assert_eq!(b.violations(&r), 1);
        assert!((b.residual(&r) - 0.1).abs() < 1e-7, "{}", b.residual(&r));
        assert_eq!(b.criticality(&r), 1.0);
        assert_eq!(b.worst_usage(&r), None, "no budget, no fraction of one");
        // No wire, nothing spent: met.
        assert_eq!(b.violations(&Routes { wires: vec![Vec::new()], ..Default::default() }), 0);
    }

    #[test]
    fn ground_c_helper_is_todays_row() {
        let b = PerformanceBudget::ground_c("ugf:min".into(), vec![NetId(0)], vec![0.5], 2.0);
        assert_eq!((b.limit, b.af_per_nm, b.weights.as_slice()), (1.0, 2.0, &[0.5][..]));
        assert!(b.r_nets.is_empty() && b.r_weights.is_empty() && b.diff_pairs.is_empty() && b.diff_weights.is_empty() && b.coupling.is_empty());
        assert_eq!(b.unknown(&Routes::default()), 0);
    }

    /// An R term nobody measures yet is unknown, not a free pass; `used` keeps the ground part.
    #[test]
    fn extra_terms_are_unknown_not_zero() {
        let mut b = PerformanceBudget::ground_c("ugf:min".into(), vec![NetId(0)], vec![0.01], 1.0);
        let r = Routes { wires: vec![wire(50), wire(10)], ..Default::default() };
        let ground = b.used(&r);
        (b.r_nets, b.r_weights) = (vec![NetId(1)], vec![0.5]);
        assert_eq!(b.unknown(&r), 1);
        assert_eq!(b.used(&r), ground);
        assert!((ground - 0.5).abs() < 1e-6);
        let mut ids = Vec::new();
        b.touched(&mut ids);
        assert_eq!(ids, vec![0, 1]);
    }

    #[test]
    fn batch_identity() {
        let b = PerformanceBudget::ground_c("x".into(), vec![NetId(0)], vec![1.0], 1.0);
        assert_eq!((b.kind(), b.repair_kind(), b.count()), ("PerformanceBudget", crate::RepairKind::Budget, 1));
    }

    /// A helpful (negative-weight) net lowers `used`; criticality floors at 0
    /// and the net spends nothing, so repair never targets it.
    #[test]
    fn a_helpful_net_spends_nothing() {
        let b = PerformanceBudget::ground_c("x".into(), vec![NetId(0)], vec![-0.01], 1.0);
        let r = Routes { wires: vec![wire(50)], ..Default::default() };
        assert!((b.used(&r) + 0.5).abs() < 1e-6);
        assert_eq!((b.criticality(&r), b.violations(&r), b.residual(&r)), (0.0, 0, 0.0));
        let mut ids = Vec::new();
        b.touched(&mut ids);
        assert!(ids.is_empty(), "{ids:?}");
    }

    /// Exactly at the limit is met: `used ≤ limit`.
    #[test]
    fn at_the_limit_is_met() {
        let b = PerformanceBudget::ground_c("x".into(), vec![NetId(0)], vec![0.0625], 1.0);
        let r = Routes { wires: vec![wire(16)], ..Default::default() };
        assert_eq!(b.used(&r), 1.0);
        assert_eq!((b.violations(&r), b.residual(&r), b.criticality(&r), b.worst_usage(&r)), (0, 0.0, 1.0, Some(1.0)));
        let mut ids = Vec::new();
        b.violating_ids(&r, &mut ids);
        assert!(ids.is_empty());
    }

    #[test]
    fn unrouted_nets_spend_nothing() {
        let b = PerformanceBudget::ground_c("x".into(), vec![NetId(9)], vec![1.0], 1.0);
        let r = Routes::default();
        assert_eq!((b.used(&r), b.violations(&r), b.residual(&r), b.worst_usage(&r), b.criticality(&r)), (0.0, 0, 0.0, Some(0.0), 0.0));
    }

    /// A coupling term touches both its nets (positive weight only), and
    /// every unmeasured term kind makes the row unknown.
    #[test]
    fn coupling_and_differential_terms_are_unknown() {
        let mut b = PerformanceBudget::ground_c("x".into(), Vec::new(), Vec::new(), 1.0);
        b.coupling = vec![(NetId(2), NetId(5), 0.1), (NetId(7), NetId(8), 0.0)];
        let mut ids = Vec::new();
        b.touched(&mut ids);
        assert_eq!(ids, vec![2, 5]);
        assert_eq!(b.unknown(&Routes::default()), 1);
        b.coupling.clear();
        (b.diff_pairs, b.diff_weights) = (vec![(NetId(0), NetId(1))], vec![0.1]);
        assert_eq!(b.unknown(&Routes::default()), 1);
    }
}
