//! Total coupling onto one victim net (routing tier).
//!
//! [`crate::routing::CrosstalkExclusion`] bounds the *spacing* between one
//! aggressor and one victim. That is a pairwise condition, and it is not the
//! constraint the circuit actually has: injected noise is
//! `ΔV = (Cc_total / C_total)·ΔV_aggressor`, where `Cc_total` sums over **every**
//! neighbour. A net flanked by five aggressors, each sitting exactly at the
//! minimum spacing, satisfies every pairwise check while receiving five times the
//! coupling its budget allows.
//!
//! So the budget is per victim, over all aggressors — the same set-vs-pair
//! distinction that separates [`crate::placement::cc::CentroidGroup`] from a
//! pairwise common-centroid. This is what consumes the `max_coupling_af` that
//! `NetClassification` assigns per class (`backend/TODO.md` §5).

use pnr_core::geom::Rect;
use pnr_core::ids::NetId;
use pnr_core::routes::Routes;

/// Lateral coupling capacitance between two parallel conductors, in aF per unit
/// of `run/gap` ratio: `C = ε·h·L/d`, so this constant is `ε·h`.
///
/// For `εr = 3.9` and a ~0.35 µm metal thickness, `ε·h ≈ 12 aF`, i.e. two
/// min-width wires 400 nm apart couple ~30 aF per µm of parallel run.
///
/// ponytail: one constant for the whole stack. Real coupling is per-layer (metal
/// thickness and spacing both change) and includes a fringe term; when the PDK
/// carries `WireParasiticParams`, read `ε·h` per layer from it.
const EPS_H_AF: f32 = 12.0;

/// Coupling between two shapes on the same layer, aF.
///
/// Two rectangles couple when they are *separated along one axis and overlap
/// along the other* — that overlap is the parallel run. Shapes that overlap on
/// both axes are shorted or on different layers, not coupled; shapes that miss on
/// both are diagonal neighbours with no parallel run.
#[must_use]
fn pair_coupling_af(p: &Rect, q: &Rect) -> f32 {
    // Separation along each axis (>0 when disjoint on that axis).
    let gap_x = (q.x - (p.x + p.w)).max(p.x - (q.x + q.w));
    let gap_y = (q.y - (p.y + p.h)).max(p.y - (q.y + q.h));
    // Overlap along each axis (>0 when they share that span).
    let run_x = (p.x + p.w).min(q.x + q.w) - p.x.max(q.x);
    let run_y = (p.y + p.h).min(q.y + q.h) - p.y.max(q.y);

    let (run, gap) = if gap_x > 0 && run_y > 0 {
        (run_y, gap_x) // side by side, running vertically
    } else if gap_y > 0 && run_x > 0 {
        (run_x, gap_y) // stacked, running horizontally
    } else {
        return 0.0;
    };
    EPS_H_AF * run as f32 / gap.max(1) as f32
}

/// **Total coupling budget** for one victim net.
///
/// - **Enforcement:** [`crate::Mode::Hard`] — the budget is a spec, and the
///   optimiser is pulled to a derated target by [`CouplingBudget::margin_pct`].
/// - **Arity:** Net ↔ *all other nets* — this is the group form.
/// - **Books:** AOAL ch02/2.7.6, ch06/6.3, ch15/15.4; FOLD 7.3/7.3.3;
///   ALS 4.4/4.4.3.
#[derive(Clone, Copy)]
pub struct CouplingBudget {
    /// The victim.
    pub net: NetId,
    /// Total coupling capacitance allowed onto it, atto-farad.
    pub max_coupling_af: i64,
    /// Safety margin held back from the budget, percent.
    pub margin_pct: u8,
}

impl CouplingBudget {
    /// Summed coupling onto the victim from every other net, aF.
    ///
    /// ponytail: O(victim_shapes × all_other_shapes). Analog nets are short; if a
    /// design ever makes this hot, bucket the shapes by layer and grid cell first.
    #[must_use]
    pub fn total_af(self, r: &Routes) -> f32 {
        let victim = r.shapes(self.net);
        if victim.is_empty() {
            return 0.0;
        }
        let mut total = 0.0f32;
        for (other, shapes) in r.wires.iter().enumerate() {
            if other == self.net.0 as usize {
                continue; // a net does not couple to itself
            }
            for a in victim {
                for b in shapes {
                    if a.layer != b.layer {
                        continue; // lateral coupling is same-layer
                    }
                    total += pair_coupling_af(&a.rect, &b.rect);
                }
            }
        }
        total
    }

    /// Summed coupling **past** the budget, as a fraction of that budget.
    ///
    /// Θ's unit (D17). The numerator is [`total_af`](CouplingBudget::total_af) — the sum
    /// over the victim's *whole* aggressor set, which is the only form that expresses the
    /// constraint at all (PLAN §4c): five aggressors each at the legal minimum spacing
    /// pass every pairwise `CrosstalkExclusion` and land 5× over this budget.
    ///
    /// Measured against the **raw** `max_coupling_af`, not the `margin_pct`-derated
    /// target — same reason as `ParasiticBudget::residual`: the margin drives
    /// `criticality` early, Θ answers "how far past the spec".
    #[must_use]
    pub fn residual(self, r: &Routes) -> f32 {
        let budget = self.max_coupling_af as f32;
        crate::rule::over(self.total_af(r) - budget, budget)
    }
}

impl crate::rule::RuleBatch<Routes> for Vec<CouplingBudget> {
    /// Summed normalised overshoot — the same quantity as [`Self::residual`].
    ///
    /// The `× 1e-3` that used to sit here is **gone**. Its comment said the aF overshoot
    /// was scaled "so it is commensurate with the other routing costs rather than
    /// dwarfing them", which is an honest description of a fudge factor and also a
    /// diagnosis of the real defect: the quantity was never divided by the budget it
    /// overshot, so its magnitude was set by the unit (atto-farad — a coupling of a few
    /// hundred aF against a budget of a few hundred aF reads as `300`, not `1`), and the
    /// only way to make it comparable to anything was to guess a constant. Dividing by
    /// `max_coupling_af` fixes it at the source: `1.0` now means "one full budget over"
    /// for this rule exactly as it does for every other, no constant required.
    ///
    /// Safe to change in place because this batch is registered in the **budget** arm
    /// only (`annotator::extract::routing_classified`), and neither `gr::score` nor
    /// `dr::score` reads a budget-arm `cost`. Nothing consumes the old scale.
    fn cost(&self, r: &Routes) -> f32 {
        self.iter().map(|c| c.residual(r)).sum()
    }
    fn residual(&self, r: &Routes) -> f64 {
        self.iter().map(|c| f64::from(c.residual(r))).sum()
    }
    fn violations(&self, r: &Routes) -> u32 {
        self.iter()
            .filter(|c| c.total_af(r) > c.max_coupling_af as f32)
            .count() as u32
    }
    fn kind(&self) -> &'static str {
        "CouplingBudget"
    }
    fn count(&self) -> usize {
        self.len()
    }
    fn criticality(&self, r: &Routes) -> f32 {
        self.iter()
            .map(|c| {
                let budget = c.max_coupling_af.max(1) as f32;
                let headroom = (1.0 - c.total_af(r) / budget).clamp(0.0, 1.0);
                let m = (f32::from(c.margin_pct) / 100.0).clamp(0.0, 0.999);
                if m <= 0.0 {
                    1.0 - headroom
                } else {
                    ((m - headroom) / m).clamp(0.0, 1.0)
                }
            })
            .fold(0.0, f32::max)
    }
    fn violating_ids(&self, r: &Routes, out: &mut Vec<u32>) {
        for c in self.iter().filter(|c| c.total_af(r) > c.max_coupling_af as f32) {
            out.push(u32::from(c.net.0));
        }
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
