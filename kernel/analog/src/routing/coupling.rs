//! Total coupling onto one victim net, summed over **every** aggressor.
//!
//! Pairwise [`crate::routing::CrosstalkExclusion`] cannot express this: a net
//! flanked by five aggressors at the legal minimum spacing passes every
//! pairwise check while taking five times its coupling budget.

use pnr_core::geom::Rect;
use pnr_core::ids::NetId;
use pnr_core::routes::Routes;

use crate::rule::Rule;

/// `ε·h` in `C = ε·h·run/gap`, aF (εr 3.9, ~0.35 µm metal): two wires 400 nm
/// apart couple ~30 aF/µm of parallel run.
///
/// ponytail: one constant for the whole stack; read per-layer `ε·h` (plus a
/// fringe term) from the PDK when it carries wire parasitics.
const EPS_H_AF: f32 = 12.0;

/// Lateral coupling between two same-layer shapes, aF. Only shapes separated
/// on one axis and overlapping on the other (a parallel run) couple.
fn pair_coupling_af(p: &Rect, q: &Rect) -> f32 {
    let gap_x = (q.x - (p.x + p.w)).max(p.x - (q.x + q.w));
    let gap_y = (q.y - (p.y + p.h)).max(p.y - (q.y + q.h));
    let run_x = (p.x + p.w).min(q.x + q.w) - p.x.max(q.x);
    let run_y = (p.y + p.h).min(q.y + q.h) - p.y.max(q.y);
    let (run, gap) = if gap_x > 0 && run_y > 0 {
        (run_y, gap_x)
    } else if gap_y > 0 && run_x > 0 {
        (run_x, gap_y)
    } else {
        return 0.0;
    };
    EPS_H_AF * run as f32 / gap.max(1) as f32
}

/// Budget on the total coupling onto `net`, aF. Registered in the budget arm.
#[derive(Clone, Copy)]
pub struct CouplingBudget {
    /// The victim.
    pub net: NetId,
    pub max_coupling_af: i64,
    /// Safety margin held back from the budget, percent.
    pub margin_pct: u8,
}

impl CouplingBudget {
    /// Summed coupling onto the victim from every other net, aF.
    ///
    /// ponytail: O(victim_shapes × all_shapes); bucket by layer/grid if it
    /// ever shows up in a profile.
    fn total_af(self, r: &Routes) -> f32 {
        let victim = r.shapes(self.net);
        if victim.is_empty() {
            return 0.0;
        }
        let mut total = 0.0f32;
        for (other, shapes) in r.wires.iter().enumerate() {
            if other == self.net.0 as usize {
                continue;
            }
            for a in victim {
                for b in shapes {
                    if a.layer == b.layer {
                        total += pair_coupling_af(&a.rect, &b.rect);
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
        Routes { wires }
    }

    fn budget(max_af: i64) -> Vec<CouplingBudget> {
        vec![CouplingBudget { net: NetId(0), max_coupling_af: max_af, margin_pct: 20 }]
    }

    #[test]
    fn coupling_accumulates_over_aggressors() {
        // This is the whole point: each aggressor sits at the *same* legal
        // spacing, so every pairwise check passes — but the total does not.
        let one = budget(400).cost(&routes(1, 400));
        let four = budget(400).cost(&routes(4, 400));
        let t1 = CouplingBudget { net: NetId(0), max_coupling_af: 400, margin_pct: 20 }
            .total_af(&routes(1, 400));
        let t4 = CouplingBudget { net: NetId(0), max_coupling_af: 400, margin_pct: 20 }
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
        let one = CouplingBudget { net: NetId(0), max_coupling_af: 1, margin_pct: 0 }
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
        let near = CouplingBudget { net: NetId(0), max_coupling_af: 1, margin_pct: 0 }
            .total_af(&routes(1, 200));
        let far = CouplingBudget { net: NetId(0), max_coupling_af: 1, margin_pct: 0 }
            .total_af(&routes(1, 800));
        assert!((near / far - 4.0).abs() < 0.1, "1/d: 4× the gap ⇒ ¼ the coupling");
    }

    #[test]
    fn different_layers_do_not_couple_laterally() {
        let mut r = routes(1, 400);
        r.wires[1][0].layer = LayerId(1);
        let t = CouplingBudget { net: NetId(0), max_coupling_af: 1, margin_pct: 0 }.total_af(&r);
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
        let t = CouplingBudget { net: NetId(0), max_coupling_af: 1, margin_pct: 0 }
            .total_af(&routes(1, 400));
        assert!((250.0..350.0).contains(&t), "expected ~300 aF, got {t}");
    }
}
