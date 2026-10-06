//! Total coupling onto one victim net, summed over **every** aggressor.
//!
//! Pairwise [`crate::routing::CrosstalkExclusion`] cannot express this: a net
//! flanked by five aggressors at the legal minimum spacing passes every
//! pairwise check while taking five times its coupling budget.

use pnr_core::geom::{Rect, Shape};
use pnr_core::ids::NetId;
use pnr_core::routes::Routes;

use crate::metadata::{NetClass, NetClassification};
use crate::rule::Rule;
use super::shield::{intersection_len, merge_spans};
use super::stack::{overlap_area_nm2, parallel};
use super::Stack;

/// `ε·h` in `C = ε·h·run/gap`, aF (εr 3.9, ~0.35 µm metal): two wires 400 nm
/// apart couple ~30 aF/µm of parallel run.
///
/// The fallback when the deck gives no per-layer `ε·t` ([`Stack::lateral_af`]).
const EPS_H_AF: f32 = 12.0;

/// Lateral coupling over `run` nm at `gap` nm on `layer`, aF: the deck's
/// `ε0·k·t·run/gap` when known (TOPO eq. 4.3), else `EPS_H_AF`. A gap below
/// 1 nm counts as 1 nm.
fn run_af(stack: Option<&Stack>, layer: u16, run: i32, gap: i32) -> f32 {
    stack.and_then(|s| s.lateral_run_af(layer, run, gap)).unwrap_or(EPS_H_AF * run as f32 / gap.max(1) as f32)
}

/// `shapes` merged per layer into maximal runs `(layer, rect, along x)`:
/// longest first, a shape joins a run it touches when it is not longer across
/// the run than along it (a collinear piece or a pad), extending the run's
/// span; the run keeps its own width. So a pad over a wire end counts once.
fn runs(shapes: &[Shape]) -> Vec<(u16, Rect, bool)> {
    let mut order: Vec<&Shape> = shapes.iter().collect();
    order.sort_by_key(|s| std::cmp::Reverse(s.rect.w.max(s.rect.h)));
    let mut out: Vec<(u16, Rect, bool)> = Vec::new();
    for s in order {
        let r = s.rect;
        let joined = out.iter_mut().find(|(l, q, horiz)| {
            let touch = r.x <= q.x + q.w && q.x <= r.x + r.w && r.y <= q.y + q.h && q.y <= r.y + r.h;
            *l == s.layer.0 && touch && if *horiz { r.w >= r.h } else { r.h >= r.w }
        });
        match joined {
            Some((_, q, true)) => {
                let (x0, x1) = (q.x.min(r.x), (q.x + q.w).max(r.x + r.w));
                (q.x, q.w) = (x0, x1 - x0);
            }
            Some((_, q, false)) => {
                let (y0, y1) = (q.y.min(r.y), (q.y + q.h).max(r.y + r.h));
                (q.y, q.h) = (y0, y1 - y0);
            }
            None => out.push((s.layer.0, r, r.w >= r.h)),
        }
    }
    out
}

/// Coupling between two nets, aF (Lampaert 1999 eq. 2.33; TOPO eqs. 4.2–4.3):
/// `a` is merged into runs ([`runs`]); per run and side, the parallel
/// same-layer shapes of `b` and `screens` (the nets between, the victim's
/// shield included) are taken nearest first and each counts only over the run
/// length no nearer shape already covers, so a wire behind another is
/// screened; `b`'s shares add `ε·t·length/gap`. End-on neighbours (beyond a
/// run's end) add their plain pair coupling. With a stack, `a` and `b` runs
/// on stack-adjacent metals overlapping in xy add `area · cross_af_um2`. The
/// one net-to-net measure: `Differential`, `CouplingBudget` and
/// `PlateRatio` read it.
///
/// ponytail: screening is O(n²) per run, as the unscreened scan was; the
/// fringe term of a crossing (the deck gives none) is omitted.
#[must_use]
pub fn net_pair_af(stack: Option<&Stack>, a: &[Shape], b: &[Shape], screens: &[Shape]) -> f32 {
    let mut c = 0.0;
    let ra = runs(a);
    for &(layer, r, horiz) in &ra {
        for below in [true, false] {
            // (gap, along span clipped to the run, counts) per shape on this side.
            let mut side: Vec<(i32, (i32, i32), bool)> = b
                .iter()
                .map(|q| (q, true))
                .chain(screens.iter().map(|q| (q, false)))
                .filter(|(q, _)| q.layer.0 == layer)
                .filter_map(|(q, mine)| {
                    let (v0, v1, s0, s1) = if horiz { (r.y, r.y + r.h, q.rect.y, q.rect.y + q.rect.h) } else { (r.x, r.x + r.w, q.rect.x, q.rect.x + q.rect.w) };
                    let gap = if below { (s1 <= v0).then_some(v0 - s1) } else { (s0 >= v1).then_some(s0 - v1) }?;
                    let (lo, hi) = if horiz { (r.x.max(q.rect.x), (r.x + r.w).min(q.rect.x + q.rect.w)) } else { (r.y.max(q.rect.y), (r.y + r.h).min(q.rect.y + q.rect.h)) };
                    (gap > 0 && lo < hi).then_some((gap, (lo, hi), mine))
                })
                .collect();
            side.sort_by_key(|&(gap, span, mine)| (gap, !mine, span));
            let mut covered: Vec<(i32, i32)> = Vec::new();
            for (gap, span, mine) in side {
                if mine {
                    let free = i64::from(span.1 - span.0) - intersection_len(&covered, &[span]);
                    c += run_af(stack, layer, free as i32, gap);
                }
                covered.push(span);
                merge_spans(&mut covered);
            }
        }
        // End-on: separated along the run, overlapping across it.
        for q in b.iter().filter(|q| q.layer.0 == layer) {
            if let Some((run, gap)) = parallel(&r, &q.rect) {
                let along = if horiz { q.rect.x >= r.x + r.w || q.rect.x + q.rect.w <= r.x } else { q.rect.y >= r.y + r.h || q.rect.y + q.rect.h <= r.y };
                if along {
                    c += run_af(stack, layer, run, gap);
                }
            }
        }
    }
    if let Some(st) = stack {
        let rb = runs(b);
        for &(la, p, _) in &ra {
            for &(lb, q, _) in &rb {
                let Some(per) = st.cross_af_um2(la, lb) else { continue };
                c += per * (overlap_area_nm2(&p, &q) as f64 * 1e-6) as f32;
            }
        }
    }
    c
}

/// Every routed wire shape of `r` but those of the nets in `skip` (indices
/// into [`Routes::wires`]): what screens a pair. Allocates; O(all shapes).
#[must_use]
pub fn screens_but(r: &Routes, skip: &[usize]) -> Vec<Shape> {
    r.wires.iter().enumerate().filter(|(n, _)| !skip.contains(n)).flat_map(|(_, w)| w.iter().copied()).collect()
}

/// Budget on the total coupling onto `net`, aF. Registered in the budget arm.
#[derive(Clone, Copy)]
pub struct CouplingBudget {
    /// The victim.
    pub net: NetId,
    /// Allowed weighted coupling onto the victim from all aggressors, aF.
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
            // An unrouted net couples nothing: skip it before building its screens.
            if shapes.is_empty() || other == self.net.0 as usize || self.exclude.is_some_and(|e| other == e.0 as usize) {
                continue;
            }
            let w = self.aggressor_weight.and_then(|w| w.get(other).copied()).unwrap_or(1.0);
            if w == 0.0 {
                continue;
            }
            total += w * net_pair_af(self.stack, victim, shapes, &screens_but(r, &[self.net.0 as usize, other]));
        }
        total
    }
}

impl Rule for CouplingBudget {
    type On = Routes;
    const REPAIR: crate::RepairKind = crate::RepairKind::KeepAway;
    /// Same as `residual`: the normalised overshoot.
    fn cost(self, r: &Routes) -> f32 {
        self.residual(r)
    }
    fn satisfied(self, r: &Routes) -> bool {
        self.total_af(r) <= self.max_coupling_af as f32
    }
    /// The victim is routed: an unrouted victim has no coupling to measure,
    /// and its vacuous zero must not certify the budget.
    fn known(self, r: &Routes) -> bool {
        !r.shapes(self.net).is_empty()
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

    /// Victim 10 µm along y; X at gap 280 over the full run, Y at gap 1 000
    /// behind X: Y couples nothing through X, and alone it does.
    #[test]
    fn a_wire_behind_a_nearer_wire_is_screened() {
        let victim = [wire(0, 0, 100, 10_000, 0)];
        let x = [wire(380, 0, 100, 10_000, 0)];
        let y = [wire(1_100, 0, 100, 10_000, 0)];
        assert_eq!(net_pair_af(None, &victim, &y, &x), 0.0);
        assert!(net_pair_af(None, &victim, &y, &[]) > 0.0);
    }

    /// A 260-wide met1 crossed by a 280-wide met2: 0.0728 µm² × 147.6 aF/µm².
    #[test]
    fn a_crossing_couples_by_overlap_area() {
        use crate::routing::stack::Layer;
        let stack = Stack {
            layers: vec![
                Layer { id: 0, cross_af_um2: 147.6, ..Layer::default() },
                Layer { id: 2, cut: true, ..Layer::default() },
                Layer { id: 1, ..Layer::default() },
            ],
            antenna_cumulative: false,
            diode: None,
        };
        let c = net_pair_af(Some(&stack), &[wire(0, 0, 5_000, 260, 0)], &[wire(1_000, -2_000, 280, 5_000, 1)], &[]);
        assert!((c - 10.75).abs() < 0.1, "{c}");
    }

    /// A pad over the victim's end merges into its run: the same C to an
    /// aggressor alongside as the wire alone.
    #[test]
    fn a_pad_over_a_wire_end_counts_once() {
        let w = wire(0, 0, 100, 10_000, 0);
        let pad = wire(-50, 9_800, 200, 200, 0);
        let agg = [wire(500, 2_000, 100, 9_000, 0)];
        let alone = net_pair_af(None, &[w], &agg, &[]);
        assert!(alone > 0.0);
        assert_eq!(net_pair_af(None, &[w, pad], &agg, &[]), alone);
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

    /// An unrouted victim has no coupling to measure: unknown, never a
    /// certified pass (Rule::known).
    #[test]
    fn an_unrouted_victim_is_unknown() {
        let b = budget(100)[0];
        let r = Routes { wires: vec![vec![], vec![wire(500, 0, 100, 10_000, 0)]], ..Default::default() };
        assert!(!b.known(&r));
        assert!(b.satisfied(&r) && b.residual(&r) == 0.0, "search cannot act on it");
        assert!(b.known(&routes(1, 400)));
    }

    #[test]
    fn runs_merge_collinear_pieces_and_keep_crossings_apart() {
        assert!(runs(&[]).is_empty());
        let (a, b) = (wire(0, 0, 5_000, 100, 0), wire(5_000, 0, 5_000, 100, 0));
        assert_eq!(runs(&[a, b]), vec![(0, Rect { x: 0, y: 0, w: 10_000, h: 100 }, true)]);
        let crossing = wire(2_000, -3_000, 100, 6_000, 0);
        assert_eq!(runs(&[a, b, crossing]).len(), 2);
        assert_eq!(runs(&[a, wire(5_000, 0, 5_000, 100, 1)]).len(), 2, "another layer never joins");
    }

    /// A collinear neighbour beyond the run's end couples over its facing
    /// width: 12 aF · 100 nm / 400 nm = 3 aF.
    #[test]
    fn an_end_on_neighbour_couples_over_its_facing_width() {
        let v = [wire(0, 0, 10_000, 100, 0)];
        let agg = [wire(10_400, 0, 5_000, 100, 0)];
        assert!((net_pair_af(None, &v, &agg, &[]) - 3.0).abs() < 1e-4);
    }

    #[test]
    fn empty_sides_couple_nothing_and_equal_wires_couple_symmetrically() {
        let v = [wire(0, 0, 100, 10_000, 0)];
        let a = [wire(500, 0, 100, 10_000, 0)];
        assert_eq!(net_pair_af(None, &[], &a, &[]), 0.0);
        assert_eq!(net_pair_af(None, &v, &[], &[]), 0.0);
        assert_eq!(net_pair_af(None, &v, &a, &[]), net_pair_af(None, &a, &v, &[]));
    }

    /// The deck's `ε·t` replaces the 12 aF fallback where the layer has one:
    /// 1 200 aF·nm/µm · 10 µm / 400 nm = 30 aF against 300 aF.
    #[test]
    fn the_decks_lateral_term_replaces_the_fallback() {
        use crate::routing::stack::Layer;
        let deck = Stack { layers: vec![Layer { id: 0, lateral: 1_200.0, ..Layer::default() }], ..Stack::default() };
        let blank = Stack { layers: vec![Layer { id: 0, ..Layer::default() }], ..Stack::default() };
        let v = [wire(0, 0, 100, 10_000, 0)];
        let a = [wire(500, 0, 100, 10_000, 0)];
        assert!((net_pair_af(Some(&deck), &v, &a, &[]) - 30.0).abs() < 1e-3);
        assert!((net_pair_af(None, &v, &a, &[]) - 300.0).abs() < 1e-3);
        assert!((net_pair_af(Some(&blank), &v, &a, &[]) - 300.0).abs() < 1e-3, "no ε·t on the layer");
    }

    #[test]
    fn screens_are_every_other_nets_wires() {
        let r = Routes { wires: vec![vec![wire(0, 0, 1, 1, 0)], vec![wire(1, 0, 1, 1, 0)], vec![wire(2, 0, 1, 1, 0)]], ..Default::default() };
        assert_eq!(screens_but(&r, &[0, 2]), vec![r.wires[1][0]]);
        assert!(screens_but(&r, &[0, 1, 2]).is_empty());
        assert_eq!(screens_but(&r, &[]).len(), 3);
    }

    #[test]
    fn default_weights_edge_cases() {
        let c = |n: u16, class| NetClassification { net: NetId(n), class, c_budget_af: None, max_coupling_af: None };
        assert!(CouplingBudget::default_weights(&[c(0, NetClass::Supply)], 0).is_empty());
        assert_eq!(CouplingBudget::default_weights(&[c(9, NetClass::Ground), c(1, NetClass::Substrate)], 2), vec![1.0, 0.0]);
    }

    #[test]
    fn a_zero_budget_fails_any_coupling() {
        let b = budget(0)[0];
        let r = routes(1, 400);
        assert_eq!(b.residual(&r), 1.0);
        assert!(!b.satisfied(&r));
        assert_eq!(b.usage(&r), Some(b.total_af(&r)), "a zero budget reads as 1 aF");
        let mut v = Vec::new();
        b.touches(&mut v);
        assert_eq!(v, vec![0]);
        assert!((b.margin() - 0.2).abs() < 1e-6);
    }
}
