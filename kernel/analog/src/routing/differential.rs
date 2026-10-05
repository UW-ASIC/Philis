//! Differential-pair route matching (routing tier).

use std::collections::{BTreeMap, BTreeSet};
use pnr_core::ids::NetId;
use pnr_core::routes::Routes;
use pnr_core::geom::Shape;
use pnr_core::Terminal;
use crate::rule::Rule;
use super::Stack;
use super::coupling::net_pair_af;

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
/// mirror image matches); the via count per cut layer; and, with
/// `aggressor_weight`, the weighted coupling each side takes from every other
/// net. The result is the worst term. Sides with unequal terminal counts (an
/// ABBA cell) compare the runs' R and no via count.
///
/// ponytail: star-model terminal R (no tree solve), lateral coupling only
/// (no crossings, no screening; RTE-18); no `project` — `dr` fixes it by mirrored rip-up.
#[derive(Clone, Copy)]
pub struct Differential {
    pub pos: NetId,
    pub neg: NetId,
    /// Max length mismatch, percent ×10.
    pub max_len_delta_pct10: i32,
    pub same_layer_required: bool,
    /// Per-layer R/C; `None` = the geometric signature.
    pub stack: Option<&'static Stack>,
    /// Aggressor weight in `[0, 1]` by `NetId` (as [`super::CouplingBudget`]); a
    /// missing index reads 1.0; `None` = no coupling-asymmetry term.
    pub aggressor_weight: Option<&'static [f32]>,
}

impl Differential {
    /// `|a − b| / mean · 100`; `0` when both are empty.
    fn len_delta_pct(self, r: &Routes) -> f32 {
        let a = r.length(self.pos) as f32;
        let b = r.length(self.neg) as f32;
        let avg = (a + b) / 2.0;
        if avg > 0.0 {
            (a - b).abs() / avg * 100.0
        } else {
            0.0
        }
    }

    /// Signature mismatch, percent (see the type doc); total-length delta
    /// when layers are not required to match.
    fn mismatch_pct(self, r: &Routes) -> f32 {
        if let Some(st) = self.stack {
            let (pa, pb) = (r.shapes(self.pos), r.shapes(self.neg));
            let rel = |a: f32, b: f32| {
                let mean = (a + b) / 2.0;
                if mean > 0.0 { (a - b).abs() / mean * 100.0 } else { 0.0 }
            };
            let (ca, cb) = (st.ground_af(pa), st.ground_af(pb));
            let (ta, tb) = (r.terminals(self.pos), r.terminals(self.neg));
            let paired = !ta.is_empty() && ta.len() == tb.len();
            // R of the runs alone: pads and cuts follow the pin set, and a
            // paralleled pin's pad summed in would read as series R.
            let runs = |s: &[Shape]| st.resistance_ohm(&s.iter().copied().filter(|q| q.rect.w != q.rect.h).collect::<Vec<_>>());
            let r_term = if paired {
                let ohm = |s: &[Shape], t: &[Terminal]| st.terminal_resistance_ohm(s, &t.iter().map(|t| t.at).collect::<Vec<_>>());
                let (ra, rb) = (ohm(pa, ta), ohm(pb, tb));
                if ra.iter().chain(&rb).any(Option::is_none) {
                    100.0
                } else {
                    let sorted = |v: Vec<Option<f32>>| {
                        let mut v: Vec<f32> = v.into_iter().flatten().collect();
                        v.sort_by(f32::total_cmp);
                        v
                    };
                    let (ra, rb) = (sorted(ra), sorted(rb));
                    let mean = (ra.iter().sum::<f32>() + rb.iter().sum::<f32>()) / (ra.len() + rb.len()) as f32;
                    let worst = ra.iter().zip(&rb).map(|(a, b)| (a - b).abs()).fold(0.0, f32::max);
                    if mean > 0.0 { worst / mean * 100.0 } else { 0.0 }
                }
            } else {
                rel(runs(pa), runs(pb))
            };
            let vias = if paired {
                let cuts = |s: &[Shape]| {
                    let mut m: BTreeMap<u16, f32> = BTreeMap::new();
                    for q in s.iter().filter(|q| st.layers.iter().any(|l| l.id == q.layer.0 && l.cut)) {
                        *m.entry(q.layer.0).or_default() += 1.0;
                    }
                    m
                };
                let (na, nb) = (cuts(pa), cuts(pb));
                let get = |m: &BTreeMap<u16, f32>, l: &u16| m.get(l).copied().unwrap_or(0.0);
                let delta: f32 = na.keys().chain(nb.keys()).collect::<BTreeSet<_>>().into_iter().map(|l| (get(&na, l) - get(&nb, l)).abs()).sum();
                let mean = (na.values().sum::<f32>() + nb.values().sum::<f32>()) / 2.0;
                if mean > 0.0 { delta / mean * 100.0 } else { 0.0 }
            } else {
                0.0
            };
            let coupling = match self.aggressor_weight {
                Some(w) if ca + cb > 0.0 => {
                    let skew: f32 = r.wires.iter().enumerate()
                        .filter(|&(a, _)| a != self.pos.0 as usize && a != self.neg.0 as usize)
                        .map(|(a, x)| {
                            let wa = w.get(a).copied().unwrap_or(1.0);
                            let screens = if wa == 0.0 { Vec::new() } else { super::coupling::screens_but(r, &[self.pos.0 as usize, self.neg.0 as usize, a]) };
                            if wa == 0.0 { 0.0 } else { wa * (net_pair_af(Some(st), pa, x, &screens) - net_pair_af(Some(st), pb, x, &screens)).abs() }
                        })
                        .sum();
                    skew / ((ca + cb) / 2.0) * 100.0
                }
                _ => 0.0,
            };
            return rel(ca, cb).max(r_term).max(vias).max(coupling);
        }
        if !self.same_layer_required {
            return self.len_delta_pct(r);
        }
        // Per layer: (length, area, squares).
        let sig = |n: NetId| {
            let mut m: std::collections::BTreeMap<u16, (f64, f64, f64)> = std::collections::BTreeMap::new();
            for s in r.shapes(n) {
                let e = m.entry(s.layer.0).or_default();
                let (w, h) = (f64::from(s.rect.w), f64::from(s.rect.h));
                e.0 += w.max(h);
                e.1 += w * h;
                e.2 += f64::from(u8::from(s.rect.w == s.rect.h));
            }
            m
        };
        let (a, b) = (sig(self.pos), sig(self.neg));
        let layers: std::collections::BTreeSet<u16> = a.keys().chain(b.keys()).copied().collect();
        let term = |f: fn(&(f64, f64, f64)) -> f64| {
            let get = |m: &std::collections::BTreeMap<u16, (f64, f64, f64)>, l| m.get(&l).map_or(0.0, f);
            let delta: f64 = layers.iter().map(|&l| (get(&a, l) - get(&b, l)).abs()).sum();
            let mean = (a.values().map(f).sum::<f64>() + b.values().map(f).sum::<f64>()) / 2.0;
            if mean > 0.0 { delta / mean * 100.0 } else { 0.0 }
        };
        term(|e| e.0).max(term(|e| e.1)).max(term(|e| e.2)) as f32
    }

    fn budget_pct(self) -> f32 {
        self.max_len_delta_pct10 as f32 / 10.0
    }
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
}
