//! Crosstalk exclusion (routing tier).

use crate::rule::Rule;
use pnr_core::geom::Rect;
use pnr_core::ids::NetId;
use pnr_core::routes::Routes;

/// Same-layer clearance between nets `a` and `b` ≥ `min_spacing_nm`.
/// Nets sharing no layer do not couple and always pass.
#[derive(Clone, Copy)]
pub struct CrosstalkExclusion {
    pub a: NetId,
    pub b: NetId,
    pub min_spacing_nm: i32,
    /// Safety margin on the floor, percent.
    pub margin_pct: u8,
}

impl CrosstalkExclusion {
    /// Min same-layer edge gap between the nets' shapes, nm; `f32::MAX` when
    /// they share no layer. ponytail: O(|A|·|B|), analog nets are short.
    fn clearance(self, r: &Routes) -> f32 {
        let mut best = f32::MAX;
        for wa in r.shapes(self.a) {
            for wb in r.shapes(self.b) {
                if wa.layer == wb.layer {
                    best = best.min(rect_gap(&wa.rect, &wb.rect));
                }
            }
        }
        best
    }
}

/// Euclidean edge-to-edge gap between two rects (`0` if touching/overlapping).
fn rect_gap(p: &Rect, q: &Rect) -> f32 {
    let dx = (q.x - (p.x + p.w)).max(p.x - (q.x + q.w)).max(0);
    let dy = (q.y - (p.y + p.h)).max(p.y - (q.y + q.h)).max(0);
    ((dx as f32).powi(2) + (dy as f32).powi(2)).sqrt()
}

impl Rule for CrosstalkExclusion {
    type On = Routes;
    const REPAIR: crate::RepairKind = crate::RepairKind::KeepAway;
    /// Spacing shortfall, nm.
    fn cost(self, r: &Routes) -> f32 {
        let d = self.clearance(r);
        if d == f32::MAX {
            return 0.0;
        }
        (self.min_spacing_nm as f32 - d).max(0.0)
    }
    fn satisfied(self, r: &Routes) -> bool {
        self.clearance(r) >= self.min_spacing_nm as f32
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
        Some((
            u32::from(self.a.0),
            u32::from(self.b.0),
            self.min_spacing_nm,
            false,
        ))
    }
    /// `(d − floor) / floor`; `1.0` when the nets share no layer.
    fn headroom(self, r: &Routes) -> f32 {
        let floor = self.min_spacing_nm.max(1) as f32;
        let d = self.clearance(r);
        if d == f32::MAX {
            return 1.0;
        }
        (d - floor) / floor
    }
    /// `floor / clearance`: past `1.0` the nets sit closer than the floor.
    /// `None` when they share no layer.
    fn usage(self, r: &Routes) -> Option<f32> {
        let d = self.clearance(r);
        (d != f32::MAX).then(|| self.min_spacing_nm as f32 / d.max(1.0))
    }
    fn margin(self) -> f32 {
        f32::from(self.margin_pct) / 100.0
    }
    fn residual(self, r: &Routes) -> f32 {
        let d = self.clearance(r);
        if d == f32::MAX {
            return 0.0;
        }
        let floor = self.min_spacing_nm as f32;
        crate::rule::over(floor - d, floor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_core::geom::{LayerId, Shape};

    fn met1(x: i32) -> Shape {
        Shape {
            layer: LayerId(1),
            rect: Rect {
                x,
                y: 0,
                w: 1_000,
                h: 260,
            },
        }
    }
    fn rule() -> CrosstalkExclusion {
        CrosstalkExclusion {
            a: NetId(0),
            b: NetId(1),
            min_spacing_nm: 280,
            margin_pct: 0,
        }
    }

    #[test]
    fn an_unrouted_pair_is_unknown() {
        let r = Routes {
            wires: vec![vec![met1(0)], vec![]],
            ..Default::default()
        };
        assert!(!rule().known(&r));
        let both = Routes {
            wires: vec![vec![met1(0)], vec![met1(1_280)]],
            ..Default::default()
        };
        assert!(rule().known(&both));
    }

    #[test]
    fn the_floor_is_an_edge_gap() {
        // Edge gap 280 nm (1 000 → 1 280): at the floor.
        let ok = Routes {
            wires: vec![vec![met1(0)], vec![met1(1_280)]],
            ..Default::default()
        };
        assert!(rule().satisfied(&ok));
        assert_eq!(rule().residual(&ok), 0.0);
        // Edge gap 270 nm: 10 nm short.
        let near = Routes {
            wires: vec![vec![met1(0)], vec![met1(1_270)]],
            ..Default::default()
        };
        assert!(!rule().satisfied(&near));
        assert_eq!(rule().residual(&near), crate::rule::over(10.0, 280.0));
    }
}
