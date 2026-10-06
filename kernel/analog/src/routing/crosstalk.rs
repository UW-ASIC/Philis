//! Crosstalk exclusion (routing tier).

use pnr_core::ids::NetId;
use pnr_core::routes::Routes;
use crate::rule::Rule;
use super::stack::edge_gap_sq;

/// Same-layer clearance between nets `a` and `b` ≥ `min_spacing_nm`
/// (Euclidean edge-to-edge, so a diagonal neighbour counts by its corner
/// distance). Nets sharing no layer do not couple and always pass. The
/// router also enforces it while searching ([`Rule::separation`], RTE-18).
#[derive(Clone, Copy)]
pub struct CrosstalkExclusion {
    /// One net of the pair (the victim for keep-away pricing).
    pub a: NetId,
    /// The other net (the aggressor for keep-away pricing).
    pub b: NetId,
    /// Required same-layer clearance, nm; `≤ 0` = any clearance passes.
    pub min_spacing_nm: i32,
    /// Safety margin on the floor, percent.
    pub margin_pct: u8,
}

impl CrosstalkExclusion {
    /// Min same-layer edge gap between the nets' shapes, nm (`0` when they
    /// touch or overlap); `None` when they share no layer.
    fn clearance(self, r: &Routes) -> Option<f32> {
        // ponytail: O(|A|·|B|), analog nets are short.
        let (sa, sb) = (r.shapes(self.a), r.shapes(self.b));
        let best = sa.iter().flat_map(|wa| sb.iter().filter(move |wb| wb.layer == wa.layer).map(move |wb| edge_gap_sq(&wa.rect, &wb.rect))).min()?;
        Some((best as f64).sqrt() as f32)
    }
}

impl Rule for CrosstalkExclusion {
    type On = Routes;
    const REPAIR: crate::RepairKind = crate::RepairKind::KeepAway;
    /// Spacing shortfall, nm.
    fn cost(self, r: &Routes) -> f32 {
        self.clearance(r).map_or(0.0, |d| (self.min_spacing_nm as f32 - d).max(0.0))
    }
    fn satisfied(self, r: &Routes) -> bool {
        self.clearance(r).is_none_or(|d| d >= self.min_spacing_nm as f32)
    }
    /// Both nets routed: an unrouted net has no clearance to measure, and
    /// "no shared layer" must not certify it (unknown is never pass).
    fn known(self, r: &Routes) -> bool {
        !r.shapes(self.a).is_empty() && !r.shapes(self.b).is_empty()
    }
    fn touches(self, out: &mut Vec<u32>) {
        out.push(u32::from(self.a.0));
        out.push(u32::from(self.b.0));
    }
    fn keepaway(self) -> Option<(u32, u32)> {
        Some((u32::from(self.a.0), u32::from(self.b.0)))
    }
    /// Enforced in the search (RTE-18): same-layer only.
    fn separation(self) -> Option<(u32, u32, i32, bool)> {
        Some((u32::from(self.a.0), u32::from(self.b.0), self.min_spacing_nm, false))
    }
    /// `(d − floor) / floor`; `1.0` when the nets share no layer.
    fn headroom(self, r: &Routes) -> f32 {
        let floor = self.min_spacing_nm.max(1) as f32;
        self.clearance(r).map_or(1.0, |d| (d - floor) / floor)
    }
    /// `floor / clearance`: past `1.0` the nets sit closer than the floor.
    /// `None` when they share no layer.
    fn usage(self, r: &Routes) -> Option<f32> {
        self.clearance(r).map(|d| self.min_spacing_nm as f32 / d.max(1.0))
    }
    fn margin(self) -> f32 {
        f32::from(self.margin_pct) / 100.0
    }
    fn residual(self, r: &Routes) -> f32 {
        let floor = self.min_spacing_nm as f32;
        self.clearance(r).map_or(0.0, |d| crate::rule::over(floor - d, floor))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_core::geom::{LayerId, Rect, Shape};

    fn met1(x: i32) -> Shape {
        Shape { layer: LayerId(1), rect: Rect { x, y: 0, w: 1_000, h: 260 } }
    }
    fn rule() -> CrosstalkExclusion {
        CrosstalkExclusion { a: NetId(0), b: NetId(1), min_spacing_nm: 280, margin_pct: 0 }
    }

    #[test]
    fn an_unrouted_pair_is_unknown() {
        let r = Routes { wires: vec![vec![met1(0)], vec![]], ..Default::default() };
        assert!(!rule().known(&r));
        let both = Routes { wires: vec![vec![met1(0)], vec![met1(1_280)]], ..Default::default() };
        assert!(rule().known(&both));
    }

    #[test]
    fn the_floor_is_an_edge_gap() {
        // Edge gap 280 nm (1 000 → 1 280): at the floor.
        let ok = Routes { wires: vec![vec![met1(0)], vec![met1(1_280)]], ..Default::default() };
        assert!(rule().satisfied(&ok));
        assert_eq!(rule().residual(&ok), 0.0);
        // Edge gap 270 nm: 10 nm short.
        let near = Routes { wires: vec![vec![met1(0)], vec![met1(1_270)]], ..Default::default() };
        assert!(!rule().satisfied(&near));
        assert_eq!(rule().residual(&near), crate::rule::over(10.0, 280.0));
    }
}
