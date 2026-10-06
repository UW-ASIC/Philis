//! Performance-driven placement term (PLC-16): a routing performance row's
//! ground-C estimate, priced on placement before any wire exists.

use pnr_core::layout::Layout;
use pnr_core::{Macro, NetId};

use crate::routing::PerformanceBudget;
use crate::rule::RuleBatch;

/// Share of a row's headroom placement may spend, leaving the rest to the
/// router's detours and vias the MST cannot see. Policy, not physics: Lampaert
/// §4.4 reserves a margin of the performance budget for routing (LAMP-34).
pub const RESERVE: f32 = 0.2;

/// A [`PerformanceBudget`]'s ground-C term, with each net's wire length
/// estimated as the rectilinear MST over its cells' pin boxes (edge to edge).
/// R, differential and coupling terms are RTE-21's and are ignored here.
///
/// Plain data: build it with [`PlacePerf::new`] from a routing row, or fill
/// the fields directly.
#[derive(Clone, Debug)]
pub struct PlacePerf {
    /// The routing row's metric name (reporting only).
    pub metric: String,
    /// One row per priced net; [`PlacePerf::mst_len`] indexes this.
    pub nets: Vec<PerfNet>,
    /// Ground capacitance per nm of wire, aF, scaled to the row's headroom.
    pub af_per_nm: f32,
    /// The routing row's `limit` (1 = headroom, 0 = do-not-worsen).
    pub limit: f32,
    /// Share of the headroom left to routing, in `[0, 1)` (see [`RESERVE`]).
    pub reserve: f32,
}

/// One net a [`PlacePerf`] prices.
#[derive(Clone, Debug, PartialEq)]
pub struct PerfNet {
    /// The net.
    pub net: NetId,
    /// The routing row's sensitivity weight for this net.
    pub weight: f32,
    /// `(cell, per-variant pin box (dx, dy, hx, hy) about the bbox centre, R0)`
    /// for every cell with a pin on the net.
    pub cells: Vec<(u16, Vec<(i32, i32, i32, i32)>)>,
}

impl PlacePerf {
    /// Builds the term for `row`; `variants[c]` are cell `c`'s alternatives.
    /// Nets touching fewer than two cells are dropped: they carry no placement
    /// wire. An alternative without a pin on a net uses a zero box at its bbox
    /// centre. Cost: O(nets · Σ pins).
    #[must_use]
    pub fn new(row: &PerformanceBudget, variants: &[&[Macro]], reserve: f32) -> Self {
        let mut nets = Vec::new();
        for (&net, &w) in row.nets.iter().zip(&row.weights) {
            let mut on = Vec::new();
            for (c, alts) in variants.iter().enumerate() {
                if !alts.iter().any(|m| m.pins.iter().any(|p| p.net == net)) {
                    continue;
                }
                let boxes = alts
                    .iter()
                    .map(|m| {
                        let mut it = m.pins.iter().filter(|p| p.net == net).map(|p| p.at);
                        let Some(first) = it.next() else { return (0, 0, 0, 0) };
                        let (x0, y0, x1, y1) = it.fold((first.x, first.y, first.x + first.w, first.y + first.h), |(a, b, c, d), r| {
                            (a.min(r.x), b.min(r.y), c.max(r.x + r.w), d.max(r.y + r.h))
                        });
                        let (cx, cy) = (m.bbox.x + m.bbox.w / 2, m.bbox.y + m.bbox.h / 2);
                        ((x0 + x1) / 2 - cx, (y0 + y1) / 2 - cy, (x1 - x0) / 2, (y1 - y0) / 2)
                    })
                    .collect();
                on.push((c as u16, boxes));
            }
            if on.len() >= 2 {
                nets.push(PerfNet { net, weight: w, cells: on });
            }
        }
        Self { metric: row.metric.clone(), nets, af_per_nm: row.af_per_nm, limit: row.limit, reserve }
    }

    /// Placed pin-box centre and half extents of cell `c` under its current
    /// variant (falling back to variant 0) and orientation.
    fn item(l: &Layout, c: u16, boxes: &[(i32, i32, i32, i32)]) -> (i64, i64, i64, i64) {
        let c = usize::from(c);
        let (bx, by, hx, hy) = boxes.get(usize::from(l.variant[c])).or(boxes.first()).copied().unwrap_or_default();
        let o = l.orient[c];
        let (dx, dy) = o.apply(bx, by);
        let (hx, hy) = if o.swaps_axes() { (hy, hx) } else { (hx, hy) };
        (i64::from(l.x[c] + dx), i64::from(l.y[c] + dy), i64::from(hx), i64::from(hy))
    }

    /// Returns the rectilinear MST length of net `n`, nm, edge to edge between
    /// pin boxes; `0` for a net with fewer than two cells.
    ///
    /// # Panics
    /// When `n` is out of range of `nets`, or a cell is out of range of `l`.
    // ponytail: dense Prim, O(k²); k ≤ ~10 cells per sensitive net.
    #[must_use]
    pub fn mst_len(&self, n: usize, l: &Layout) -> i64 {
        let pts: Vec<_> = self.nets[n].cells.iter().map(|(c, b)| Self::item(l, *c, b)).collect();
        if pts.len() < 2 {
            return 0;
        }
        let d = |a: (i64, i64, i64, i64), b: (i64, i64, i64, i64)| {
            ((a.0 - b.0).abs() - (a.2 + b.2)).max(0) + ((a.1 - b.1).abs() - (a.3 + b.3)).max(0)
        };
        let mut best: Vec<i64> = pts.iter().map(|&p| d(pts[0], p)).collect();
        let mut done = vec![false; pts.len()];
        done[0] = true;
        let mut total = 0;
        for _ in 1..pts.len() {
            let (j, _) = (0..pts.len()).filter(|&j| !done[j]).map(|j| (j, best[j])).min_by_key(|&(_, b)| b).expect("one left");
            total += best[j];
            done[j] = true;
            for k in 0..pts.len() {
                best[k] = best[k].min(d(pts[j], pts[k]));
            }
        }
        total
    }

    /// Σ w_i · mst_i · af_per_nm: fraction of the headroom the estimate spends.
    fn used(&self, l: &Layout) -> f32 {
        (0..self.nets.len()).map(|n| self.nets[n].weight * self.mst_len(n, l) as f32 * self.af_per_nm).sum()
    }

    /// Overshoot of the placement share `1 − reserve` past `limit`.
    fn residual_of(&self, used: f32) -> f32 {
        (used / (1.0 - self.reserve) - self.limit).max(0.0)
    }
}

impl RuleBatch<Layout> for PlacePerf {
    fn cost(&self, l: &Layout) -> f32 {
        self.residual_of(self.used(l))
    }
    fn violations(&self, l: &Layout) -> u32 {
        u32::from(self.residual_of(self.used(l)) > 0.0)
    }
    fn residual(&self, l: &Layout) -> f64 {
        f64::from(self.residual_of(self.used(l)))
    }
    fn kind(&self) -> &'static str {
        "PlacePerf"
    }
    fn count(&self) -> usize {
        1
    }
    fn local(&self) -> bool {
        true
    }
    fn touched(&self, out: &mut Vec<u32>) {
        let mut ids: Vec<u32> = self.nets.iter().flat_map(|n| &n.cells).map(|(c, _)| u32::from(*c)).collect();
        ids.sort_unstable();
        ids.dedup();
        out.extend(ids);
    }
    fn criticality(&self, l: &Layout) -> f32 {
        (self.used(l) / (1.0 - self.reserve)).clamp(0.0, 1.0)
    }
    fn worst_usage(&self, l: &Layout) -> Option<f32> {
        Some(self.used(l) / (1.0 - self.reserve))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_core::Orient;

    fn cells(xy: &[(i32, i32)]) -> Layout {
        let n = xy.len();
        Layout {
            x: xy.iter().map(|p| p.0).collect(),
            y: xy.iter().map(|p| p.1).collect(),
            hw: vec![500; n],
            hh: vec![500; n],
            axis: vec![0],
            groups: vec![],
            orient: vec![Orient::default(); n],
            variant: vec![0; n],
            branch: Vec::new(),
            power_uw: vec![0; n],
            temp_mc: vec![0; n],
            units: Default::default(),
        }
    }

    fn perf(boxes: &[(i32, i32, i32, i32)], limit: f32) -> PlacePerf {
        PlacePerf {
            metric: String::new(),
            nets: vec![PerfNet { net: NetId(0), weight: 1.0, cells: boxes.iter().enumerate().map(|(c, b)| (c as u16, vec![*b])).collect() }],
            af_per_nm: 1e-3,
            limit,
            reserve: RESERVE,
        }
    }

    #[test]
    fn mst_equals_hpwl_for_two_points() {
        let p = perf(&[(0, 0, 0, 0); 2], 1.0);
        assert_eq!(p.mst_len(0, &cells(&[(0, 0), (3_000, 4_000)])), 7_000);
    }

    #[test]
    fn abutting_pins_have_zero_mst() {
        let p = perf(&[(0, 0, 500, 500); 2], 1.0);
        assert_eq!(p.mst_len(0, &cells(&[(0, 0), (1_000, 0)])), 0);
        // One nm apart: one nm of wire.
        assert_eq!(p.mst_len(0, &cells(&[(0, 0), (1_001, 0)])), 1);
    }

    #[test]
    fn rotated_pin_box_follows_orient() {
        let p = perf(&[(400, 0, 100, 50), (0, 0, 0, 0)], 1.0);
        let mut l = cells(&[(0, 0), (0, 1_000)]);
        // R0: box centre (400, 0), half (100, 50): dx 400 − 100, dy 1000 − 50.
        assert_eq!(p.mst_len(0, &l), 300 + 950);
        l.orient[0] = Orient::R90;
        // R90: centre apply(400, 0) = (0, 400), half (50, 100): dx 0, dy 600 − 100.
        assert_eq!(Orient::R90.apply(400, 0), (0, 400));
        assert_eq!(p.mst_len(0, &l), 500);
    }

    #[test]
    fn residual_charges_only_past_the_reserve() {
        let p = perf(&[(0, 0, 0, 0); 2], 1.0);
        assert_eq!(p.residual_of(0.7), 0.0);
        assert!((p.residual_of(1.0) - 0.25).abs() < 1e-6);
        let p = perf(&[(0, 0, 0, 0); 2], 0.0);
        assert!((p.residual_of(0.4) - 0.5).abs() < 1e-6);
        // Through the trait: 7 µm × 1e-3 = 7 used → 7/0.8 − 1.
        let l = cells(&[(0, 0), (3_000, 4_000)]);
        let p = perf(&[(0, 0, 0, 0); 2], 1.0);
        assert!((p.residual(&l) - (7.0 / 0.8 - 1.0)).abs() < 1e-5);
        assert_eq!(p.violations(&l), 1);
    }
}

#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use pnr_core::{LayerId, Orient, Pin, Rect};

    fn cells(xy: &[(i32, i32)]) -> Layout {
        let n = xy.len();
        Layout {
            x: xy.iter().map(|p| p.0).collect(),
            y: xy.iter().map(|p| p.1).collect(),
            hw: vec![500; n],
            hh: vec![500; n],
            axis: vec![],
            groups: vec![],
            orient: vec![Orient::default(); n],
            variant: vec![0; n],
            branch: Vec::new(),
            power_uw: vec![0; n],
            temp_mc: vec![0; n],
            units: Default::default(),
        }
    }

    /// One net over point pins at every cell centre.
    fn points(n: usize, limit: f32) -> PlacePerf {
        PlacePerf {
            metric: "m".into(),
            nets: vec![PerfNet { net: NetId(0), weight: 1.0, cells: (0..n as u16).map(|c| (c, vec![(0, 0, 0, 0)])).collect() }],
            af_per_nm: 1e-3,
            limit,
            reserve: RESERVE,
        }
    }

    fn pin(net: u16, x: i32, y: i32, w: i32, h: i32) -> Pin {
        Pin { name: String::new(), net: NetId(net), at: Rect { x, y, w, h }, layer: LayerId(0) }
    }

    #[test]
    fn mst_of_fewer_than_two_cells_is_zero() {
        let l = cells(&[(0, 0)]);
        assert_eq!(points(0, 1.0).mst_len(0, &l), 0);
        assert_eq!(points(1, 1.0).mst_len(0, &l), 0);
    }

    #[test]
    fn mst_is_a_tree_not_a_star() {
        // Collinear 0, 1 µm, 5 µm: the tree is 5 µm, a star from 0 would be 6.
        let l = cells(&[(0, 0), (1_000, 0), (5_000, 0)]);
        assert_eq!(points(3, 1.0).mst_len(0, &l), 5_000);
        // An L of three: 3 µm + 4 µm.
        let l = cells(&[(0, 0), (3_000, 0), (3_000, 4_000)]);
        assert_eq!(points(3, 1.0).mst_len(0, &l), 7_000);
    }

    #[test]
    fn variant_past_the_boxes_falls_back_to_the_first() {
        let mut p = points(0, 1.0);
        p.nets[0].cells = vec![(0, vec![(100, 0, 0, 0)]), (1, vec![(0, 0, 0, 0)])];
        let mut l = cells(&[(0, 0), (1_000, 0)]);
        l.variant[0] = 3;
        assert_eq!(p.mst_len(0, &l), 900);
    }

    #[test]
    fn usage_criticality_and_reserve() {
        // 7 µm of wire × 1e-3 = 7 used; placement may spend 0.8 of it.
        let l = cells(&[(0, 0), (3_000, 4_000)]);
        let p = points(2, 1.0);
        let u = p.worst_usage(&l).unwrap();
        assert!((u - 7.0 / 0.8).abs() < 1e-4, "{u}");
        assert_eq!(p.criticality(&l), 1.0);
        assert!((p.cost(&l) - (7.0 / 0.8 - 1.0)).abs() < 1e-4);
        let near = cells(&[(0, 0), (100, 0)]);
        assert!((p.criticality(&near) - 0.1 / 0.8).abs() < 1e-6);
        assert_eq!((p.violations(&near), p.cost(&near)), (0, 0.0));
        // No reserve: the whole headroom.
        let all = PlacePerf { reserve: 0.0, ..points(2, 1.0) };
        assert!((all.worst_usage(&near).unwrap() - 0.1).abs() < 1e-6);
    }

    #[test]
    fn zero_weight_nets_cost_nothing() {
        let mut p = points(2, 1.0);
        p.nets[0].weight = 0.0;
        assert_eq!(p.worst_usage(&cells(&[(0, 0), (9_000, 0)])), Some(0.0));
    }

    #[test]
    fn touched_lists_each_cell_once_sorted() {
        let p = PlacePerf {
            nets: vec![
                PerfNet { net: NetId(0), weight: 1.0, cells: vec![(5, vec![]), (2, vec![])] },
                PerfNet { net: NetId(1), weight: 1.0, cells: vec![(2, vec![]), (0, vec![])] },
            ],
            ..points(0, 1.0)
        };
        let mut out = vec![9];
        p.touched(&mut out);
        assert_eq!(out, [9, 0, 2, 5]);
        assert!(p.local());
        assert_eq!((p.count(), p.kind()), (1, "PlacePerf"));
    }

    #[test]
    fn new_keeps_nets_on_two_cells_with_pin_boxes_about_the_bbox_centre() {
        // Cell 0: two net-0 pins spanning x 0…400, y 0…100 in a 1000² bbox.
        let c0 = Macro { pins: vec![pin(0, 0, 0, 100, 100), pin(0, 300, 0, 100, 100), pin(1, 0, 0, 10, 10)], bbox: Rect { x: 0, y: 0, w: 1_000, h: 1_000 }, ..Default::default() };
        // Cell 1: one net-0 pin; alternative 1 has none.
        let c1 = Macro { pins: vec![pin(0, 500, 500, 0, 0)], bbox: Rect { x: 0, y: 0, w: 1_000, h: 1_000 }, ..Default::default() };
        let c1_bare = Macro { bbox: Rect { x: 0, y: 0, w: 1_000, h: 1_000 }, ..Default::default() };
        let alts: [&[Macro]; 2] = [std::slice::from_ref(&c0), &[c1, c1_bare]];
        let row = PerformanceBudget::ground_c("ota".into(), vec![NetId(0), NetId(1), NetId(2)], vec![2.0, 3.0, 4.0], 0.5);
        let p = PlacePerf::new(&row, &alts, 0.3);
        // Net 1 sits on cell 0 only; net 2 nowhere: both dropped.
        assert_eq!(p.nets.iter().map(|n| (n.net, n.weight)).collect::<Vec<_>>(), [(NetId(0), 2.0)]);
        assert_eq!((p.metric.as_str(), p.af_per_nm, p.limit, p.reserve), ("ota", 0.5, 1.0, 0.3));
        assert_eq!(p.nets[0].cells[0], (0, vec![(200 - 500, 50 - 500, 200, 50)]));
        assert_eq!(p.nets[0].cells[1], (1, vec![(0, 0, 0, 0), (0, 0, 0, 0)]));
    }
}
