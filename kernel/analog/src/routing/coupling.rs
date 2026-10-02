//! Total coupling onto one victim net, summed over **every** aggressor.
//!
//! Pairwise [`crate::routing::CrosstalkExclusion`] cannot express this: a net
//! flanked by five aggressors at the legal minimum spacing passes every
//! pairwise check while taking five times its coupling budget.

use pnr_core::geom::Rect;
use pnr_core::ids::NetId;
use pnr_core::routes::Routes;

use crate::metadata::{NetClass, NetClassification};
use crate::rule::Rule;
use super::Stack;

/// `ε·h` in `C = ε·h·run/gap`, aF (εr 3.9, ~0.35 µm metal): two wires 400 nm
/// apart couple ~30 aF/µm of parallel run.
///
/// The fallback when the deck gives no per-layer `ε·t` ([`Stack::lateral_af`]).
const EPS_H_AF: f32 = 12.0;

/// Lateral coupling between two same-layer shapes, aF: the layer's own
/// `ε0·k·t·run/gap` from the deck when known (TOPO eq. 4.3), else `EPS_H_AF`.
/// Only shapes separated on one axis and overlapping on the other (a parallel
/// run) couple.
fn pair_coupling_af(stack: Option<&Stack>, layer: u16, p: &Rect, q: &Rect) -> f32 {
    if let Some(c) = stack.and_then(|s| s.lateral_af(layer, p, q)) {
        return c;
    }
    super::stack::parallel(p, q).map_or(0.0, |(run, gap)| EPS_H_AF * run as f32 / gap.max(1) as f32)
}

/// Budget on the total coupling onto `net`, aF. Registered in the budget arm.
#[derive(Clone, Copy)]
pub struct CouplingBudget {
    /// The victim.
    pub net: NetId,
    pub max_coupling_af: i64,
    /// Safety margin held back from the budget, percent.
    pub margin_pct: u8,
    /// Per-layer `ε·t`; `None` = one constant for the stack.
    pub stack: Option<&'static Stack>,
    /// The victim's shield reference: never an aggressor (a grounded shield is
    /// the remedy the rule asks for, BAL2-10, not coupling to book).
    pub exclude: Option<NetId>,
    /// Aggressor weight in `[0, 1]` by `NetId`; a missing index reads 1.0;
    /// `None` = all 1.0. `&'static` because rules are `Copy` (as `stack`).
    pub aggressor_weight: Option<&'static [f32]>,
}

impl CouplingBudget {
    /// `1.0` per net, `0.0` for the quiet rails (Supply, Ground, Substrate):
    /// coupling to a DC node injects no noise. `classes` entries past
    /// `n_nets` are ignored.
    #[must_use]
    pub fn default_weights(classes: &[NetClassification], n_nets: usize) -> Vec<f32> {
        let mut w = vec![1.0; n_nets];
        for c in classes {
            if matches!(c.class, NetClass::Supply | NetClass::Ground | NetClass::Substrate) {
                if let Some(x) = w.get_mut(c.net.0 as usize) {
                    *x = 0.0;
                }
            }
        }
        w
    }

    /// Summed coupling onto the victim from every other net except `exclude`,
    /// each pair weighted by its aggressor's `aggressor_weight`, aF.
    ///
    /// ponytail: O(victim_shapes × all_shapes), scalar. Measured ~25 ms over
    /// the whole local bench (~230 s), so no SIMD; bucket by layer/grid first
    /// if routes ever grow large enough to matter.
    fn total_af(self, r: &Routes) -> f32 {
        let victim = r.shapes(self.net);
        if victim.is_empty() {
            return 0.0;
        }
        let mut total = 0.0f32;
        for (other, shapes) in r.wires.iter().enumerate() {
            if other == self.net.0 as usize || self.exclude.is_some_and(|e| other == e.0 as usize) {
                continue;
            }
            let w = self.aggressor_weight.and_then(|w| w.get(other).copied()).unwrap_or(1.0);
            if w == 0.0 {
                continue;
            }
            for a in victim {
                for b in shapes {
                    if a.layer == b.layer {
                        total += w * pair_coupling_af(self.stack, a.layer.0, &a.rect, &b.rect);
                    }
                }
            }
        }
        total
    }
}

impl Rule for CouplingBudget {
    type On = Routes;
    /// Same as `residual`: the normalised overshoot.
    fn cost(self, r: &Routes) -> f32 {
        self.residual(r)
    }
    fn satisfied(self, r: &Routes) -> bool {
        self.total_af(r) <= self.max_coupling_af as f32
    }
    fn headroom(self, r: &Routes) -> f32 {
        1.0 - self.total_af(r) / self.max_coupling_af.max(1) as f32
    }
    fn usage(self, r: &Routes) -> Option<f32> {
        Some(self.total_af(r) / self.max_coupling_af.max(1) as f32)
    }
    fn margin(self) -> f32 {
        f32::from(self.margin_pct) / 100.0
    }
    fn residual(self, r: &Routes) -> f32 {
        let budget = self.max_coupling_af as f32;
        crate::rule::over(self.total_af(r) - budget, budget)
    }
    fn touches(self, out: &mut Vec<u32>) {
        out.push(u32::from(self.net.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule::RuleBatch;
    use pnr_core::geom::{LayerId, Shape};

    fn wire(x: i32, y: i32, w: i32, h: i32, layer: u16) -> Shape {
        Shape { layer: LayerId(layer), rect: Rect { x, y, w, h } }
    }

    /// A victim running vertically, with `n` aggressors beside it at `gap`.
    fn routes(n: usize, gap: i32) -> Routes {
        let mut wires = vec![vec![wire(0, 0, 100, 10_000, 0)]]; // net 0 = victim
        for i in 0..n {
            // Alternate sides so they are all genuinely adjacent to the victim.
            let x = if i % 2 == 0 { 100 + gap } else { -(gap + 100) };
            wires.push(vec![wire(x, 0, 100, 10_000, 0)]);
        }
        Routes { wires, ..Default::default()  }
    }

    fn budget(max_af: i64) -> Vec<CouplingBudget> {
        vec![CouplingBudget { net: NetId(0), max_coupling_af: max_af, margin_pct: 20, stack: None, exclude: None, aggressor_weight: None }]
    }

    #[test]
    fn coupling_accumulates_over_aggressors() {
        // This is the whole point: each aggressor sits at the *same* legal
        // spacing, so every pairwise check passes — but the total does not.
        let one = budget(400).cost(&routes(1, 400));
        let four = budget(400).cost(&routes(4, 400));
        let t1 = CouplingBudget { net: NetId(0), max_coupling_af: 400, margin_pct: 20, stack: None, exclude: None, aggressor_weight: None }
            .total_af(&routes(1, 400));
        let t4 = CouplingBudget { net: NetId(0), max_coupling_af: 400, margin_pct: 20, stack: None, exclude: None, aggressor_weight: None }
            .total_af(&routes(4, 400));
        assert!((t4 - 4.0 * t1).abs() < 1.0, "four equal neighbours ⇒ 4× coupling");
        assert_eq!(one, 0.0, "a single neighbour is inside budget");
        assert!(four > 0.0, "the same spacing with four neighbours is not");
        assert_eq!(budget(400).violations(&routes(4, 400)), 1);
        assert_eq!(budget(400).violations(&routes(1, 400)), 0);
    }

    #[test]
    fn residual_is_the_set_sum_normalised_by_the_budget() {
        // The budget is per victim over the **whole** aggressor set (PLAN §4c), so the
        // residual has to be measured on `total_af` — not per pair. Set the budget to
        // exactly one neighbour's contribution and the four-aggressor case is 4× it,
        // i.e. 3 full budgets over.
        let one = CouplingBudget { net: NetId(0), max_coupling_af: 1, margin_pct: 0, stack: None, exclude: None, aggressor_weight: None }
            .total_af(&routes(1, 400));
        let b = budget(one.round() as i64);
        assert_eq!(b[0].residual(&routes(1, 400)), 0.0, "at its budget ⇒ nothing past it");
        let four = b[0].residual(&routes(4, 400));
        assert!((four - 3.0).abs() < 0.05, "4× the coupling on a 1× budget ⇒ 3.0, got {four}");
        // The batch's Θ contribution is that same number — and `cost` now agrees with it,
        // because the `× 1e-3` that made them differ by a thousand is gone.
        assert!((b.residual(&routes(4, 400)) - f64::from(four)).abs() < 1e-6);
        assert!((b.cost(&routes(4, 400)) - four).abs() < 1e-6);
    }

    #[test]
    fn coupling_falls_off_with_spacing() {
        let near = CouplingBudget { net: NetId(0), max_coupling_af: 1, margin_pct: 0, stack: None, exclude: None, aggressor_weight: None }
            .total_af(&routes(1, 200));
        let far = CouplingBudget { net: NetId(0), max_coupling_af: 1, margin_pct: 0, stack: None, exclude: None, aggressor_weight: None }
            .total_af(&routes(1, 800));
        assert!((near / far - 4.0).abs() < 0.1, "1/d: 4× the gap ⇒ ¼ the coupling");
    }

    #[test]
    fn different_layers_do_not_couple_laterally() {
        let mut r = routes(1, 400);
        r.wires[1][0].layer = LayerId(1);
        let t = CouplingBudget { net: NetId(0), max_coupling_af: 1, margin_pct: 0, stack: None, exclude: None, aggressor_weight: None }.total_af(&r);
        assert_eq!(t, 0.0, "lateral coupling is same-layer");
    }

    #[test]
    fn criticality_rises_as_the_budget_is_consumed() {
        // Comfortable: well inside the margin ⇒ no pressure.
        let slack = budget(100_000).criticality(&routes(1, 400));
        assert_eq!(slack, 0.0);
        // Over budget ⇒ fully critical.
        let tight = budget(100).criticality(&routes(4, 400));
        assert_eq!(tight, 1.0);
    }

    #[test]
    fn magnitude_is_physical() {
        // Two min-width wires 400 nm apart, 10 µm of parallel run: ~300 aF.
        let t = CouplingBudget { net: NetId(0), max_coupling_af: 1, margin_pct: 0, stack: None, exclude: None, aggressor_weight: None }
            .total_af(&routes(1, 400));
        assert!((250.0..350.0).contains(&t), "expected ~300 aF, got {t}");
    }

    #[test]
    fn the_victims_own_shield_is_not_an_aggressor() {
        // 10 µm victim (net 0), reference (net 1) tracks both sides at 280 nm.
        let r = Routes {
            wires: vec![vec![wire(0, 0, 100, 10_000, 0)], vec![wire(380, 0, 100, 10_000, 0), wire(-380, 0, 100, 10_000, 0)]],
            ..Default::default()
        };
        let b = CouplingBudget { net: NetId(0), max_coupling_af: 1, margin_pct: 0, stack: None, exclude: None, aggressor_weight: None };
        let t = b.total_af(&r);
        assert!((t - 2.0 * 12.0 * 10_000.0 / 280.0).abs() < 0.5, "two 280 nm sides of EPS_H_AF: 857.1 aF, got {t}");
        assert_eq!(CouplingBudget { exclude: Some(NetId(1)), ..b }.total_af(&r), 0.0);
    }

    #[test]
    fn a_quiet_rail_weighs_nothing() {
        let w: &'static [f32] = Box::leak(vec![1.0, 0.0].into_boxed_slice());
        let b = CouplingBudget { net: NetId(0), max_coupling_af: 1, margin_pct: 0, stack: None, exclude: None, aggressor_weight: Some(w) };
        assert!(CouplingBudget { aggressor_weight: None, ..b }.total_af(&routes(1, 400)) > 0.0);
        assert_eq!(b.total_af(&routes(1, 400)), 0.0);
    }

    #[test]
    fn default_weights_zero_only_the_rails() {
        let c = |n: u16, class| NetClassification { net: NetId(n), class, c_budget_af: None, max_coupling_af: None };
        let classes = [c(0, NetClass::Supply), c(1, NetClass::Signal), c(2, NetClass::Ground)];
        assert_eq!(CouplingBudget::default_weights(&classes, 4), vec![0.0, 1.0, 0.0, 1.0]);
    }
}
