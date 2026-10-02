//! Differential-pair route matching (routing tier).

use pnr_core::ids::NetId;
use pnr_core::routes::Routes;
use pnr_core::{BipartiteHypergraph, UnionFind};
use crate::placement::matching_pair::{is_diff_pair, D};
use crate::rule::Rule;
use super::Stack;

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
/// With the deck's stack it compares what each side presents instead (MAT-11:
/// matched R and C seen by each side): ground C and series R from the stack,
/// the worst relative difference. That is what the circuit sees, and it lets
/// sides with unequal pin sets (a common-centroid ABBA cell gives one drain
/// three pads and the other two) match when their RC does.
///
/// ponytail: ground C and the runs' summed R (pads and cuts left out), not a
/// terminal-resolved RC network, and no coupling term; no `project` — `dr` fixes it by mirrored rip-up.
#[derive(Clone, Copy)]
pub struct Differential {
    pub pos: NetId,
    pub neg: NetId,
    /// Max length mismatch, percent ×10.
    pub max_len_delta_pct10: i32,
    pub same_layer_required: bool,
    /// Per-layer R/C; `None` = the geometric signature.
    pub stack: Option<&'static Stack>,
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
            let rel = |f: &dyn Fn(&[pnr_core::geom::Shape]) -> f32| {
                let (a, b) = (f(r.shapes(self.pos)), f(r.shapes(self.neg)));
                let mean = (a + b) / 2.0;
                if mean > 0.0 { (a - b).abs() / mean * 100.0 } else { 0.0 }
            };
            // R of the runs alone: pads and cuts follow the pin set, and a
            // paralleled pin's pad summed in would read as series R.
            let runs = |s: &[pnr_core::geom::Shape]| st.resistance_ohm(&s.iter().copied().filter(|q| q.rect.w != q.rect.h).collect::<Vec<_>>());
            return rel(&|s| st.ground_af(s)).max(rel(&runs));
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

    /// One per diff pair: its two drain nets, 5% budget, same layers.
    fn extract(hg: &BipartiteHypergraph, _uf: &mut UnionFind) -> Vec<Self> {
        let mut out = Vec::new();
        let n = hg.device_count();
        for a in 0..n {
            for b in (a + 1)..n {
                if is_diff_pair(hg, a, b) {
                    out.push(Differential {
                        pos: hg.device_nets[a][D],
                        neg: hg.device_nets[b][D],
                        max_len_delta_pct10: 50,
                        same_layer_required: true,
                        stack: None,
                    });
                }
            }
        }
        out
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
        Differential { pos: NetId(0), neg: NetId(1), max_len_delta_pct10: 50, same_layer_required: true, stack: None }
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
        diode_layer: None,
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
}
