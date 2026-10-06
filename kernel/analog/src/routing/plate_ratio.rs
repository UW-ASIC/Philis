//! Capacitor-set plate leads (routing tier, budget): every bit's lead
//! capacitance per unit equal, and no bottom plate under the top (RTE-20).

use pnr_core::geom::{Rect, Shape};
use pnr_core::ids::NetId;
use pnr_core::routes::Routes;

use super::coupling::{net_pair_af, screens_but};
use super::Stack;
use crate::rule::RuleBatch;

/// One capacitor set as the router sees it: the shared top plate, each
/// bottom-plate (bit) net with its unit count, one unit's capacitance and the
/// placed array (absolute nm). Built per layout by the library from a
/// capacitor `Unitization`.
#[derive(Clone, Debug, PartialEq)]
pub struct PlateSet {
    /// The shared top-plate net.
    pub top: NetId,
    /// Each bit's bottom-plate net and its unit count (`0` reads as 1).
    pub bits: Vec<(NetId, u32)>,
    /// One unit's C, aF; `NAN` = unknown (no `c_af` on any one-unit member).
    pub c_unit_af: f32,
    /// The placed array's bounding box, absolute nm: metal wholly inside
    /// it is plate, not lead.
    pub array: Rect,
}

/// A bit's lead (its routed metal outside the array) loads it with
/// `C_i = ground C + Σ_m coupling to net m` (screened, [`net_pair_af`]); per
/// unit these must agree, or the ratio the array draws is off (Hastings §8.2
/// rule C10: "insert jogs or dead-end branches into its lead until the two
/// capacitances match"). `spread = max_i |C_i/n_i − mean_j C_j/n_j| / C_unit`,
/// percent, against `tol_pct10 / 10` %. Top and bits are kept `space_nm`
/// apart and never overlap on adjacent layers ([`RuleBatch::separations`]).
#[derive(Clone, Debug)]
pub struct PlateRatio {
    /// The capacitor set.
    pub set: PlateSet,
    /// Allowed spread, percent ×10 (`10` = 1 %).
    pub tol_pct10: i32,
    /// The stack lead C is measured on.
    pub stack: &'static Stack,
    /// The first routed metal's spacing, nm.
    pub space_nm: i32,
}

impl PlateRatio {
    /// Per bit (in `set.bits` order), its lead C, aF: ground C of its wire
    /// shapes not wholly inside `set.array`, plus their screened coupling to
    /// every other routed net. O(nets · shapes²).
    #[must_use]
    pub fn lead_af(&self, r: &Routes) -> Vec<f32> {
        let a = self.set.array;
        let inside = |s: &Shape| s.rect.x >= a.x && s.rect.y >= a.y && s.rect.x + s.rect.w <= a.x + a.w && s.rect.y + s.rect.h <= a.y + a.h;
        self.set
            .bits
            .iter()
            .map(|&(bit, _)| {
                let lead: Vec<Shape> = r.shapes(bit).iter().copied().filter(|s| !inside(s)).collect();
                let b = bit.0 as usize;
                // An unrouted net couples nothing: skip it before building its screens.
                let coupled: f32 = (0..r.wires.len())
                    .filter(|&m| m != b && !r.wires[m].is_empty())
                    .map(|m| net_pair_af(Some(self.stack), &lead, &r.wires[m], &screens_but(r, &[b, m])))
                    .sum();
                self.stack.ground_af(&lead) + coupled
            })
            .collect()
    }

    /// `C_i / n_i` per bit, aF (a unit count of `0` reads as 1).
    #[must_use]
    pub fn per_unit_af(&self, r: &Routes) -> Vec<f32> {
        self.lead_af(r).iter().zip(&self.set.bits).map(|(&c, &(_, n))| c / n.max(1) as f32).collect()
    }

    /// Spread of the per-unit lead C over one unit's C, percent; `None`
    /// when unknown (no unit C, or a bit unrouted).
    #[must_use]
    pub fn spread_pct(&self, r: &Routes) -> Option<f32> {
        let known = self.set.c_unit_af.is_finite() && self.set.c_unit_af > 0.0 && self.set.bits.iter().all(|&(b, _)| !r.shapes(b).is_empty());
        if !known || self.set.bits.is_empty() {
            return None;
        }
        let per = self.per_unit_af(r);
        let mean = per.iter().sum::<f32>() / per.len() as f32;
        Some(per.iter().map(|c| (c - mean).abs()).fold(0.0, f32::max) / self.set.c_unit_af * 100.0)
    }

    /// `over(spread − tol, tol)`; `0` when unknown.
    fn residual(&self, r: &Routes) -> f32 {
        let tol = self.tol_pct10 as f32 / 10.0;
        self.spread_pct(r).map_or(0.0, |s| crate::rule::over(s - tol, tol))
    }
}

/// Every [`PlateRatio`] of one routed layout (budget arm).
#[derive(Clone, Debug, Default)]
pub struct PlateRatios(pub Vec<PlateRatio>);

impl RuleBatch<Routes> for PlateRatios {
    fn cost(&self, r: &Routes) -> f32 {
        self.0.iter().map(|p| p.residual(r)).sum()
    }
    fn violations(&self, r: &Routes) -> u32 {
        self.0.iter().filter(|p| p.residual(r) > 0.0).count() as u32
    }
    fn residual(&self, r: &Routes) -> f64 {
        self.0.iter().map(|p| f64::from(p.residual(r))).sum()
    }
    fn kind(&self) -> &'static str {
        "PlateRatio"
    }
    /// Equalised by dead-end stubs after geometry (`dr`), not by a reroute.
    fn repair_kind(&self) -> crate::RepairKind {
        crate::RepairKind::None
    }
    fn count(&self) -> usize {
        self.0.len()
    }
    fn unknown(&self, r: &Routes) -> u32 {
        self.0.iter().filter(|p| p.spread_pct(r).is_none()).count() as u32
    }
    fn touched(&self, out: &mut Vec<u32>) {
        for p in &self.0 {
            out.push(u32::from(p.set.top.0));
            out.extend(p.set.bits.iter().map(|b| u32::from(b.0 .0)));
        }
    }
    fn violating_ids(&self, r: &Routes, out: &mut Vec<u32>) {
        for p in self.0.iter().filter(|p| p.residual(r) > 0.0) {
            out.extend(p.set.bits.iter().map(|b| u32::from(b.0 .0)));
        }
    }
    /// Top against every bit: `space_nm` apart, never stacked (C_TB).
    fn separations(&self, out: &mut Vec<(u32, u32, i32, bool)>) {
        for p in &self.0 {
            out.extend(p.set.bits.iter().map(|b| (u32::from(p.set.top.0), u32::from(b.0 .0), p.space_nm, true)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing::stack::Layer;
    use pnr_core::geom::LayerId;

    /// Ground C only (30 aF/µm², no fringe, negligible lateral `ε·t`): a
    /// lead's C is its area.
    pub(crate) fn area_stack() -> &'static Stack {
        Box::leak(Box::new(Stack { layers: vec![Layer { id: 1, area_af_um2: 30.0, lateral: 1e-9, ..Layer::default() }], antenna_cumulative: false, diode: None }))
    }

    /// Bits of 1/2/4/8 units with equal 10 µm met1 leads, 20 µm apart: per
    /// unit the 1-unit bit's lead weighs eight times the 8-unit bit's, so
    /// the spread is far over 1 %.
    #[test]
    fn lead_capacitance_is_equalised_per_unit() {
        let set = PlateSet { top: NetId(0), bits: [1, 2, 4, 8].iter().enumerate().map(|(i, &n)| (NetId(i as u16 + 1), n)).collect(), c_unit_af: 1_000.0, array: Rect { x: 0, y: 0, w: 100_000, h: 1_000 } };
        let rule = PlateRatio { set, tol_pct10: 10, stack: area_stack(), space_nm: 140 };
        let mut wires = vec![Vec::new()];
        for i in 0..4 {
            wires.push(vec![Shape { layer: LayerId(1), rect: Rect { x: i * 20_000, y: 1_000, w: 260, h: 10_000 } }]);
        }
        let r = Routes { wires, ..Default::default() };
        let per = rule.per_unit_af(&r);
        assert!((per[0] - 78.0).abs() < 0.1 && (per[3] - 9.75).abs() < 0.1, "{per:?}");
        assert!(rule.spread_pct(&r).unwrap() > 1.0);
        assert!(PlateRatios(vec![rule.clone()]).violations(&r) == 1);
        let mut seps = Vec::new();
        PlateRatios(vec![rule]).separations(&mut seps);
        assert_eq!(seps, vec![(0, 1, 140, true), (0, 2, 140, true), (0, 3, 140, true), (0, 4, 140, true)]);
    }

    fn set(bits: &[(u16, u32)], c_unit_af: f32) -> PlateSet {
        PlateSet { top: NetId(0), bits: bits.iter().map(|&(n, k)| (NetId(n), k)).collect(), c_unit_af, array: Rect { x: 0, y: 0, w: 100_000, h: 1_000 } }
    }
    fn rule(set: PlateSet) -> PlateRatio {
        PlateRatio { set, tol_pct10: 10, stack: area_stack(), space_nm: 140 }
    }
    /// A 10 µm met1 lead above the array at `x`: 2.6 µm², 78 aF.
    fn lead(x: i32) -> Shape {
        Shape { layer: LayerId(1), rect: Rect { x, y: 1_000, w: 260, h: 10_000 } }
    }

    #[test]
    fn unknown_unit_c_an_unrouted_bit_or_no_bits_is_unknown() {
        let r = Routes { wires: vec![vec![], vec![lead(0)], vec![]], ..Default::default() };
        assert_eq!(rule(set(&[(1, 1)], f32::NAN)).spread_pct(&r), None);
        assert_eq!(rule(set(&[(1, 1)], 0.0)).spread_pct(&r), None);
        assert_eq!(rule(set(&[(1, 1), (2, 1)], 1_000.0)).spread_pct(&r), None, "bit 2 unrouted");
        assert_eq!(rule(set(&[], 1_000.0)).spread_pct(&r), None);
        let batch = PlateRatios(vec![rule(set(&[(1, 1), (2, 1)], 1_000.0))]);
        assert_eq!((batch.unknown(&r), batch.violations(&r), batch.residual(&r)), (1, 0, 0.0));
    }

    #[test]
    fn mirrored_equal_leads_have_no_spread() {
        let r = Routes { wires: vec![vec![], vec![lead(0)], vec![lead(20_000)]], ..Default::default() };
        let p = rule(set(&[(1, 1), (2, 1)], 1_000.0));
        assert!(p.spread_pct(&r).unwrap() < 1e-3, "{:?}", p.spread_pct(&r));
        assert_eq!(PlateRatios(vec![p.clone()]).violations(&r), 0);
        assert_eq!(rule(set(&[(1, 1)], 1_000.0)).spread_pct(&r), Some(0.0), "one bit has nothing to differ from");
    }

    /// Equal 78 aF leads on a 1-unit and a 2-unit bit: 78 and 39 per unit,
    /// mean 58.5, spread 19.5 / 1 000 = 1.95 %.
    #[test]
    fn spread_is_the_worst_per_unit_gap_over_one_unit() {
        let r = Routes { wires: vec![vec![], vec![lead(0)], vec![lead(20_000)]], ..Default::default() };
        let p = rule(set(&[(1, 1), (2, 2)], 1_000.0));
        assert!((p.spread_pct(&r).unwrap() - 1.95).abs() < 0.01, "{:?}", p.spread_pct(&r));
        let batch = |tol_pct10| PlateRatios(vec![PlateRatio { tol_pct10, ..p.clone() }]);
        assert!((batch(10).residual(&r) - 0.95).abs() < 0.01, "{}", batch(10).residual(&r));
        assert_eq!(batch(0).residual(&r), 1.0, "a zero tolerance fails any spread");
        assert_eq!(batch(20).residual(&r), 0.0);
        let mut ids = Vec::new();
        batch(10).violating_ids(&r, &mut ids);
        assert_eq!(ids, vec![1, 2], "the bits, not the top");
    }

    #[test]
    fn a_zero_unit_count_reads_as_one() {
        let r = Routes { wires: vec![vec![], vec![lead(0)]], ..Default::default() };
        let p = rule(set(&[(1, 0)], 1_000.0));
        assert_eq!(p.per_unit_af(&r), p.lead_af(&r));
    }

    #[test]
    fn metal_wholly_inside_the_array_is_plate_not_lead() {
        let inside = Shape { layer: LayerId(1), rect: Rect { x: 1_000, y: 100, w: 5_000, h: 500 } };
        let p = rule(set(&[(1, 1)], 1_000.0));
        assert_eq!(p.lead_af(&Routes { wires: vec![vec![], vec![inside]], ..Default::default() }), vec![0.0]);
        let straddle = Shape { rect: Rect { y: 500, h: 1_000, ..inside.rect }, ..inside };
        let c = p.lead_af(&Routes { wires: vec![vec![], vec![straddle]], ..Default::default() })[0];
        assert!((c - 150.0).abs() < 0.1, "the whole straddling shape is lead: {c}");
    }

    #[test]
    fn hooks_list_the_top_then_the_bits() {
        let b = PlateRatios(vec![rule(set(&[(3, 1), (4, 2)], 1_000.0))]);
        let mut t = Vec::new();
        b.touched(&mut t);
        assert_eq!(t, vec![0, 3, 4]);
        assert_eq!((b.kind(), b.repair_kind(), b.count()), ("PlateRatio", crate::RepairKind::None, 1));
        assert_eq!(PlateRatios::default().count(), 0);
    }
}
