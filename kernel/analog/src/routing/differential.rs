//! Differential-pair route matching (routing tier).

use std::collections::BTreeMap;
use pnr_core::ids::NetId;
use pnr_core::routes::Routes;
use pnr_core::geom::Shape;
use pnr_core::Terminal;
use crate::rule::Rule;
use super::Stack;
use super::coupling::{net_pair_af, screens_but};

/// `pos`/`neg` routes match electrically, not just in total length: with
/// `same_layer_required`, their **route signatures** agree — per layer, drawn
/// length (R and C scale with it), drawn area (width), and square shapes (cuts
/// and their landing pads: the via count). Equal length split differently
/// across layers, or with a different via count, is unequal RC.
///
/// The mismatch is the worst of the three, each `Σ_layer |a − b|` over the
/// pair's mean total, in percent — so a stray layer counts by how much of the
/// route it carries.
///
/// With the deck's stack it compares what each side presents: ground C;
/// series R from the net's centre to each terminal (sorted values, so a
/// mirror image matches; a terminal no shape reaches reads 100 %); the via
/// count per cut layer; and, with `aggressor_weight`, the weighted coupling
/// each side takes from every other net. The result is the worst term.
/// Sides with unequal terminal counts (an ABBA cell) compare the runs' R and
/// no via count. `same_layer_required` is ignored with a stack.
///
/// ponytail: star-model terminal R (no tree solve); no `project` — `dr`
/// fixes it by mirrored rip-up.
#[derive(Clone, Copy)]
pub struct Differential {
    /// The positive side of the pair.
    pub pos: NetId,
    /// The negative side of the pair.
    pub neg: NetId,
    /// Max mismatch, percent ×10 (`50` = 5 %).
    pub max_len_delta_pct10: i32,
    /// Without a stack: compare the per-layer route signature, not only the
    /// total drawn length.
    pub same_layer_required: bool,
    /// Per-layer R/C; `None` = the geometric signature.
    pub stack: Option<&'static Stack>,
    /// Aggressor weight in `[0, 1]` by `NetId` (as [`super::CouplingBudget`]); a
    /// missing index reads 1.0; `None` = no coupling-asymmetry term.
    pub aggressor_weight: Option<&'static [f32]>,
}

/// `|a − b|` over their mean, percent; `0` when the mean is not positive.
fn rel_pct(a: f32, b: f32) -> f32 {
    let mean = (a + b) / 2.0;
    if mean > 0.0 { (a - b).abs() / mean * 100.0 } else { 0.0 }
}

impl Differential {
    /// `|a − b| / mean · 100` of the total drawn lengths; `0` when both are
    /// empty.
    fn len_delta_pct(self, r: &Routes) -> f32 {
        rel_pct(r.length(self.pos) as f32, r.length(self.neg) as f32)
    }

    /// Mismatch, percent (see the type doc): the stack's electrical terms
    /// when a stack is given, else the geometric signature, else (without
    /// `same_layer_required`) the total-length delta.
    fn mismatch_pct(self, r: &Routes) -> f32 {
        match self.stack {
            Some(st) => self.electrical_pct(st, r),
            None if self.same_layer_required => self.signature_pct(r),
            None => self.len_delta_pct(r),
        }
    }

    /// Worst of the ground-C, terminal-R, via-count and coupling-skew
    /// mismatches measured on `st`, percent.
    fn electrical_pct(self, st: &Stack, r: &Routes) -> f32 {
        let (pa, pb) = (r.shapes(self.pos), r.shapes(self.neg));
        let (ca, cb) = (st.ground_af(pa), st.ground_af(pb));
        let (ta, tb) = (r.terminals(self.pos), r.terminals(self.neg));
        let paired = !ta.is_empty() && ta.len() == tb.len();
        let r_term = if paired { terminal_r_pct(st, pa, ta, pb, tb) } else { runs_r_pct(st, pa, pb) };
        let vias = if paired { via_count_pct(st, pa, pb) } else { 0.0 };
        let coupling = match self.aggressor_weight {
            Some(w) if ca + cb > 0.0 => self.coupling_skew_af(st, w, r) / ((ca + cb) / 2.0) * 100.0,
            _ => 0.0,
        };
        rel_pct(ca, cb).max(r_term).max(vias).max(coupling)
    }

    /// `Σ_m w_m · |C(pos, m) − C(neg, m)|` over every other net `m`, aF.
    fn coupling_skew_af(self, st: &Stack, w: &[f32], r: &Routes) -> f32 {
        let (pa, pb) = (r.shapes(self.pos), r.shapes(self.neg));
        let (p, n) = (self.pos.0 as usize, self.neg.0 as usize);
        r.wires
            .iter()
            .enumerate()
            .filter(|&(m, _)| m != p && m != n)
            .map(|(m, x)| (w.get(m).copied().unwrap_or(1.0), m, x))
            .filter(|&(wm, ..)| wm != 0.0)
            .map(|(wm, m, x)| {
                let screens = screens_but(r, &[p, n, m]);
                wm * (net_pair_af(Some(st), pa, x, &screens) - net_pair_af(Some(st), pb, x, &screens)).abs()
            })
            .sum()
    }

    /// Worst per-layer `(length, area, squares)` signature mismatch, percent.
    fn signature_pct(self, r: &Routes) -> f32 {
        let sig = |n: NetId| {
            let mut m: BTreeMap<u16, [f64; 3]> = BTreeMap::new();
            for s in r.shapes(n) {
                let e = m.entry(s.layer.0).or_default();
                let (w, h) = (f64::from(s.rect.w), f64::from(s.rect.h));
                e[0] += w.max(h);
                e[1] += w * h;
                e[2] += f64::from(u8::from(s.rect.w == s.rect.h));
            }
            m
        };
        let (a, b) = (sig(self.pos), sig(self.neg));
        (0..3).map(|k| per_layer_pct(&a, &b, |e| e[k])).fold(0.0, f64::max) as f32
    }

    /// The budget, percent.
    fn budget_pct(self) -> f32 {
        self.max_len_delta_pct10 as f32 / 10.0
    }
}

/// `Σ_layer |f(a) − f(b)|` over the mean of `Σ f(a)` and `Σ f(b)`, percent;
/// a layer only one side has counts in full. `0` when the mean is not
/// positive.
fn per_layer_pct<T>(a: &BTreeMap<u16, T>, b: &BTreeMap<u16, T>, f: impl Fn(&T) -> f64) -> f64 {
    let get = |m: &BTreeMap<u16, T>, l: &u16| m.get(l).map_or(0.0, &f);
    let delta: f64 = a.keys().map(|l| (get(a, l) - get(b, l)).abs()).sum::<f64>() + b.keys().filter(|l| !a.contains_key(l)).map(|l| get(b, l).abs()).sum::<f64>();
    let mean = (a.values().map(&f).sum::<f64>() + b.values().map(&f).sum::<f64>()) / 2.0;
    if mean > 0.0 { delta / mean * 100.0 } else { 0.0 }
}

/// Worst gap between the two sides' centre-to-terminal R, sorted so a mirror
/// image matches, over the mean R, percent; `100` when a terminal is
/// unreached.
fn terminal_r_pct(st: &Stack, pa: &[Shape], ta: &[Terminal], pb: &[Shape], tb: &[Terminal]) -> f32 {
    let sorted = |s: &[Shape], t: &[Terminal]| -> Option<Vec<f32>> {
        let at: Vec<_> = t.iter().map(|t| t.at).collect();
        let mut v: Vec<f32> = st.terminal_resistance_ohm(s, &at).into_iter().collect::<Option<_>>()?;
        v.sort_by(f32::total_cmp);
        Some(v)
    };
    let (Some(ra), Some(rb)) = (sorted(pa, ta), sorted(pb, tb)) else { return 100.0 };
    let mean = (ra.iter().sum::<f32>() + rb.iter().sum::<f32>()) / (ra.len() + rb.len()) as f32;
    let worst = ra.iter().zip(&rb).map(|(a, b)| (a - b).abs()).fold(0.0, f32::max);
    if mean > 0.0 { worst / mean * 100.0 } else { 0.0 }
}

/// [`rel_pct`] of the runs' series R alone (non-square shapes): pads and
/// cuts follow the pin set, and a paralleled pin's pad summed in would read
/// as series R.
fn runs_r_pct(st: &Stack, pa: &[Shape], pb: &[Shape]) -> f32 {
    let runs = |s: &[Shape]| st.resistance_ohm(&s.iter().copied().filter(|q| q.rect.w != q.rect.h).collect::<Vec<_>>());
    rel_pct(runs(pa), runs(pb))
}

/// Per cut layer of `st`, the two sides' cut counts' summed gap over the
/// mean count, percent.
fn via_count_pct(st: &Stack, pa: &[Shape], pb: &[Shape]) -> f32 {
    let cuts = |s: &[Shape]| {
        let mut m: BTreeMap<u16, f64> = BTreeMap::new();
        for q in s.iter().filter(|q| st.layers.iter().any(|l| l.id == q.layer.0 && l.cut)) {
            *m.entry(q.layer.0).or_default() += 1.0;
        }
        m
    };
    per_layer_pct(&cuts(pa), &cuts(pb), |&n| n) as f32
}

impl Rule for Differential {
    type On = Routes;
    const REPAIR: crate::RepairKind = crate::RepairKind::Mirror;
    fn touches(self, out: &mut Vec<u32>) {
        out.push(u32::from(self.pos.0));
        out.push(u32::from(self.neg.0));
    }
    /// Percent signature mismatch past budget.
    fn cost(self, r: &Routes) -> f32 {
        (self.mismatch_pct(r) - self.budget_pct()).max(0.0)
    }
    fn satisfied(self, r: &Routes) -> bool {
        self.mismatch_pct(r) <= self.budget_pct()
    }
    fn residual(self, r: &Routes) -> f32 {
        let budget = self.budget_pct();
        crate::rule::over(self.mismatch_pct(r) - budget, budget)
    }
    /// Unknown until both sides have metal (AR-44): an unrouted side is not a match.
    fn known(self, r: &Routes) -> bool {
        !r.shapes(self.pos).is_empty() && !r.shapes(self.neg).is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_core::geom::{LayerId, Rect, Shape};

    fn seg(layer: u16, x: i32, w: i32, h: i32) -> Shape {
        Shape { layer: LayerId(layer), rect: Rect { x, y: 0, w, h } }
    }
    fn pair() -> Differential {
        Differential { pos: NetId(0), neg: NetId(1), max_len_delta_pct10: 50, same_layer_required: true, stack: None, aggressor_weight: None }
    }

    /// m1 (id 1), via (2, cut), m2 (3): the stack.rs test helper's values.
    fn rc_stack() -> &'static Stack {
        use crate::routing::stack::Layer;
        let metal = |id| Layer { id, area_af_um2: 25.0, fringe_af_um: 40.0, lateral: 3.9 * 8.854 * 360.0, sheet_ohm: 0.125, ..Layer::default() };
        Box::leak(Box::new(Stack { layers: vec![metal(1), Layer { id: 2, sheet_ohm: 4.5, cut: true, ..Layer::default() }, metal(3)], antenna_cumulative: false, diode: None }))
    }
    fn wire(x: i32, y: i32, w: i32, h: i32) -> Shape {
        Shape { layer: LayerId(1), rect: Rect { x, y, w, h } }
    }
    fn term(x: i32, y: i32) -> pnr_core::Terminal {
        pnr_core::Terminal { at: Rect { x, y, w: 170, h: 170 }, ua: None }
    }

    #[test]
    fn equal_length_on_the_same_layers_can_still_mismatch() {
        // Same total length (10 µm), same layer set {1, 2}, split 8+2 vs 2+8.
        let r = Routes { wires: vec![vec![seg(1, 0, 8_000, 140), seg(2, 0, 2_000, 140)], vec![seg(1, 0, 2_000, 140), seg(2, 0, 8_000, 140)]], ..Default::default()  };
        assert!(pair().len_delta_pct(&r) == 0.0, "the old metric sees nothing");
        assert!(!pair().satisfied(&r));

        // One extra via (cut + pad squares) on a 10 µm route: a via count mismatch.
        let base = vec![seg(1, 0, 10_000, 140)];
        let mut extra = base.clone();
        extra.push(seg(3, 0, 170, 170));
        extra.push(seg(3, 500, 170, 170));
        let r = Routes { wires: vec![base.clone(), extra], ..Default::default()  };
        assert!(!pair().satisfied(&r));

        // Wider wire, same length: area (width, so R and C) differs.
        let r = Routes { wires: vec![base.clone(), vec![seg(1, 0, 10_000, 280)]], ..Default::default()  };
        assert!(!pair().satisfied(&r));

        // Identical signatures pass.
        let r = Routes { wires: vec![base.clone(), base], ..Default::default()  };
        assert!(pair().satisfied(&r));
        assert_eq!(pair().residual(&r), 0.0);
    }

    /// With the stack, the pair is judged on the R and C each side presents:
    /// an extra landing pad (an ABBA cell gives one drain three pins, the
    /// other two) is a fraction of a percent and passes; a 20% longer trunk
    /// does not.
    #[test]
    fn with_the_stack_unequal_pad_counts_match_when_rc_does() {
        use crate::routing::stack::Layer;
        let stack: &'static Stack = Box::leak(Box::new(Stack {
            layers: vec![
                Layer { id: 1, area_af_um2: 25.0, fringe_af_um: 40.0, sheet_ohm: 0.125, ..Layer::default() },
                Layer { id: 3, area_af_um2: 36.0, fringe_af_um: 40.0, sheet_ohm: 12.8, ..Layer::default() },
            ],
            antenna_cumulative: false,
        diode: None,
        }));
        let rc = Differential { stack: Some(stack), ..pair() };
        let trunk = vec![seg(1, 0, 30_000, 290)];
        let mut padded = trunk.clone();
        padded.push(seg(3, 0, 170, 170));
        let r = Routes { wires: vec![padded, trunk.clone()], ..Default::default()  };
        assert!(!pair().satisfied(&r), "the geometric signature counts the pad");
        assert!(rc.satisfied(&r), "one pad of RC on a 30 µm trunk is noise");
        let r = Routes { wires: vec![vec![seg(1, 0, 36_000, 290)], trunk], ..Default::default()  };
        assert!(!rc.satisfied(&r), "20% more wire is 20% more R and C");
    }

    /// A dead-end stub off the trunk sums the same total length as the
    /// straight run, but its terminal sees more series R: the terminal-R term
    /// catches it where the runs' R alone would not.
    #[test]
    fn a_stub_does_not_match_terminal_resistance() {
        let rc = Differential { stack: Some(rc_stack()), ..pair() };
        let pos = vec![wire(0, 0, 30_000, 290)];
        let neg = vec![wire(0, 5_000, 20_000, 290), wire(10_000, 5_000, 290, 10_000)];
        let r = Routes {
            wires: vec![pos, neg],
            terms: vec![vec![term(0, 60), term(29_830, 60)], vec![term(0, 5_060), term(19_830, 5_060)]],
            ..Default::default()
        };
        assert!(rc.mismatch_pct(&r) > 5.0);
        assert!(!rc.satisfied(&r));
    }

    /// Terminal R compares each side's R values sorted ascending, not
    /// terminals paired in list order: neg lists its terminals in a
    /// different order from their mirror images, so only the sorted
    /// comparison reads zero.
    #[test]
    fn an_exact_mirror_is_zero() {
        let rc = Differential { stack: Some(rc_stack()), ..pair() };
        let pos = vec![wire(0, 0, 30_000, 290)];
        let neg = vec![wire(50_000, 0, 30_000, 290)];
        let r = Routes {
            wires: vec![pos, neg],
            terms: vec![
                vec![term(0, 60), term(8_000, 60), term(29_830, 60)],
                vec![term(50_000, 60), term(71_830, 60), term(79_830, 60)],
            ],
            ..Default::default()
        };
        assert!(rc.mismatch_pct(&r) <= 1e-4);
        assert_eq!(rc.residual(&r), 0.0);
    }

    /// One extra cut on one side of an otherwise identical pair is a via
    /// count mismatch. The cut lands on a terminal: a dead end there adds no
    /// ground C and no terminal R, while mid-wire its port would move the
    /// star centre and the terminal-R term would fire instead.
    #[test]
    fn an_unmatched_cut_breaks_the_pair() {
        let rc = Differential { stack: Some(rc_stack()), ..pair() };
        let cut = Shape { layer: LayerId(2), rect: Rect { x: 0, y: 60, w: 170, h: 170 } };
        let side = |y| vec![wire(0, y, 10_000, 290)];
        let terms = |y| vec![term(0, y + 60), term(9_830, y + 60)];
        let mut pos = side(0);
        pos.push(cut);
        let r = Routes { wires: vec![pos.clone(), side(1_000)], terms: vec![terms(0), terms(1_000)], ..Default::default() };
        assert!(rc.mismatch_pct(&r) > rc.budget_pct(), "{}", rc.mismatch_pct(&r));
        let mut neg = side(1_000);
        neg.push(Shape { rect: Rect { y: 1_060, ..cut.rect }, ..cut });
        let r = Routes { wires: vec![pos, neg], terms: vec![terms(0), terms(1_000)], ..Default::default() };
        assert_eq!(rc.mismatch_pct(&r), 0.0);
    }

    /// A third net coupling onto only one side of the pair breaks the match;
    /// zeroing that net's aggressor weight restores it.
    #[test]
    fn a_one_sided_aggressor_breaks_the_pair() {
        let pos = vec![wire(0, 0, 10_000, 290)];
        let neg = vec![wire(0, -20_000, 10_000, 290)];
        let aggressor = vec![wire(0, 570, 10_000, 290)];
        let r = Routes {
            wires: vec![pos, neg, aggressor],
            terms: vec![
                vec![term(0, 60), term(9_830, 60)],
                vec![term(0, -19_940), term(9_830, -19_940)],
                vec![],
            ],
            ..Default::default()
        };
        let weights: &'static [f32] = &[1.0, 1.0, 1.0];
        let rc = Differential { stack: Some(rc_stack()), aggressor_weight: Some(weights), ..pair() };
        assert!(rc.residual(&r) > 0.0);
        let weights: &'static [f32] = &[1.0, 1.0, 0.0];
        let rc = Differential { stack: Some(rc_stack()), aggressor_weight: Some(weights), ..pair() };
        assert_eq!(rc.mismatch_pct(&r), 0.0);
    }

    /// An unrouted side is unknown, not a (vacuous) match.
    #[test]
    fn an_unrouted_side_is_unknown() {
        let rc = Differential { stack: Some(rc_stack()), ..pair() };
        let r = Routes { wires: vec![vec![wire(0, 0, 10_000, 290)], vec![]], ..Default::default() };
        assert!(!rc.known(&r));
        let r = Routes { wires: vec![vec![wire(0, 0, 10_000, 290)], vec![wire(0, 0, 10_000, 290)]], ..Default::default() };
        assert!(rc.known(&r));
    }

    #[test]
    fn rel_pct_is_the_gap_over_the_mean() {
        assert_eq!(rel_pct(0.0, 0.0), 0.0);
        assert_eq!(rel_pct(1.0, 3.0), 100.0);
        assert_eq!(rel_pct(3.0, 1.0), 100.0);
        assert_eq!(rel_pct(2.0, 2.0), 0.0);
    }

    #[test]
    fn per_layer_pct_counts_a_one_sided_layer_in_full() {
        let a: BTreeMap<u16, f64> = [(1u16, 10.0)].into();
        let b: BTreeMap<u16, f64> = [(2u16, 10.0)].into();
        assert_eq!(per_layer_pct(&a, &b, |&x| x), 200.0);
        assert_eq!(per_layer_pct(&b, &a, |&x| x), 200.0);
        assert_eq!(per_layer_pct(&a, &a, |&x| x), 0.0);
        let e: BTreeMap<u16, f64> = BTreeMap::new();
        assert_eq!(per_layer_pct(&e, &e, |&x| x), 0.0);
    }

    /// Without layer matching only the total length counts: the 8+2 / 2+8
    /// split passes, a 10 % longer side does not.
    #[test]
    fn without_layer_matching_only_total_length_counts() {
        let p = Differential { same_layer_required: false, ..pair() };
        let split = Routes { wires: vec![vec![seg(1, 0, 8_000, 140), seg(2, 0, 2_000, 140)], vec![seg(1, 0, 2_000, 140), seg(2, 0, 8_000, 140)]], ..Default::default() };
        assert!(p.satisfied(&split));
        let long = Routes { wires: vec![vec![seg(1, 0, 10_000, 140)], vec![seg(1, 0, 9_000, 140)]], ..Default::default() };
        assert!(!p.satisfied(&long));
        assert!((p.cost(&long) - (1_000.0 / 9_500.0 * 100.0 - 5.0)).abs() < 1e-3, "{}", p.cost(&long));
    }

    #[test]
    fn swapping_the_sides_changes_nothing() {
        let r = Routes { wires: vec![vec![seg(1, 0, 8_000, 140), seg(2, 0, 2_000, 140)], vec![seg(1, 0, 10_000, 140)]], ..Default::default() };
        let swap = |d: Differential| Differential { pos: d.neg, neg: d.pos, ..d };
        assert_eq!(pair().mismatch_pct(&r), swap(pair()).mismatch_pct(&r));
        let rc = Differential { stack: Some(rc_stack()), ..pair() };
        assert_eq!(rc.mismatch_pct(&r), swap(rc).mismatch_pct(&r));
    }

    #[test]
    fn two_empty_sides_are_unknown_and_measure_nothing() {
        let r = Routes { wires: vec![vec![], vec![]], ..Default::default() };
        assert!(!pair().known(&r));
        assert_eq!(pair().mismatch_pct(&r), 0.0);
        assert_eq!(Differential { stack: Some(rc_stack()), ..pair() }.mismatch_pct(&r), 0.0);
    }

    /// A terminal no shape reaches has no R to match: a full 100 %.
    #[test]
    fn an_unreached_terminal_reads_a_full_mismatch() {
        let rc = Differential { stack: Some(rc_stack()), ..pair() };
        let r = Routes {
            wires: vec![vec![wire(0, 0, 10_000, 290)], vec![wire(0, 1_000, 10_000, 290)]],
            terms: vec![vec![term(0, 60), term(50_000, 60)], vec![term(0, 1_060), term(9_830, 1_060)]],
            ..Default::default()
        };
        assert_eq!(rc.mismatch_pct(&r), 100.0);
    }

    #[test]
    fn a_zero_budget_passes_only_an_exact_match() {
        let exact = Differential { max_len_delta_pct10: 0, ..pair() };
        let base = vec![seg(1, 0, 10_000, 140)];
        let same = Routes { wires: vec![base.clone(), base.clone()], ..Default::default() };
        assert!(exact.satisfied(&same) && exact.residual(&same) == 0.0);
        let off = Routes { wires: vec![base, vec![seg(1, 0, 10_001, 140)]], ..Default::default() };
        assert_eq!(exact.residual(&off), 1.0);
        let mut v = Vec::new();
        exact.touches(&mut v);
        assert_eq!(v, vec![0, 1]);
    }
}
