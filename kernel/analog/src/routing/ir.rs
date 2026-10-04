//! DC IR drop along a current-carrying net (routing tier, budget).

use pnr_core::ids::NetId;
use pnr_core::routes::Routes;
use crate::rule::Rule;
use super::Stack;

/// The drop a net's routing adds stays within `max_drop_uv` (PWR-02;
/// Lampaert 1999 eq. 2.33 as `R ≤ ΔV_max / I`). With every terminal current
/// known and reached, the drop is terminal-resolved (REL-04): the largest
/// Σ I_e·R_e between two points of the routed + cell metal tree
/// ([`super::current::net_flow`], the same tree EM checks), so a dead branch
/// adds nothing. Otherwise `I · R_route`, `R_route` the R of the routed
/// shapes' most resistive path ([`Stack::path_resistance_ohm`]): the whole
/// current charged to the worst path, an upper bound on every terminal's drop.
///
/// Unknown without the stack, or without both a current and a route.
///
/// ponytail: the fallback keeps a net with an unknown or open terminal known
/// (as the bound); a nodal solve would replace both arms.
#[derive(Clone, Copy)]
pub struct IrDrop {
    pub net: NetId,
    /// DC current the net carries, µA; negative = unknown.
    pub current_ua: i32,
    /// Allowed drop along the net's routing, µV.
    pub max_drop_uv: i64,
    /// Safety margin on the budget, percent.
    pub margin_pct: u8,
    pub stack: Option<&'static Stack>,
}

impl IrDrop {
    /// Drop along the routing, µV; `None` when unknown.
    fn drop_uv(self, r: &Routes) -> Option<f32> {
        let st = self.stack?;
        let shapes = r.shapes(self.net);
        let all = [shapes, r.cell_metal(self.net)].concat();
        super::current::net_flow(st, &all, r.terminals(self.net))
            .map(|f| f.drop_uv)
            .or_else(|| (self.current_ua >= 0 && !shapes.is_empty()).then(|| self.current_ua as f32 * st.path_resistance_ohm(shapes)))
    }
}

impl Rule for IrDrop {
    type On = Routes;
    const REPAIR: crate::RepairKind = crate::RepairKind::Ir;
    fn cost(self, r: &Routes) -> f32 {
        self.residual(r)
    }
    fn satisfied(self, r: &Routes) -> bool {
        self.drop_uv(r).is_none_or(|d| d <= self.max_drop_uv as f32)
    }
    fn known(self, r: &Routes) -> bool {
        self.drop_uv(r).is_some()
    }
    fn residual(self, r: &Routes) -> f32 {
        let budget = self.max_drop_uv as f32;
        self.drop_uv(r).map_or(0.0, |d| crate::rule::over(d - budget, budget))
    }
    fn headroom(self, r: &Routes) -> f32 {
        self.drop_uv(r).map_or(1.0, |d| 1.0 - d / (self.max_drop_uv as f32).max(1.0))
    }
    fn usage(self, r: &Routes) -> Option<f32> {
        Some(self.drop_uv(r)? / (self.max_drop_uv as f32).max(1.0))
    }
    fn margin(self) -> f32 {
        f32::from(self.margin_pct) / 100.0
    }
    fn touches(self, out: &mut Vec<u32>) {
        out.push(u32::from(self.net.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing::stack::Layer;
    use pnr_core::geom::{LayerId, Rect, Shape};
    use pnr_core::routes::Terminal;

    /// 1 mA through 20 □ of 0.125 Ω/□ (2.5 Ω) plus a 4.5 Ω cut drops 7 mV.
    #[test]
    fn drop_is_current_times_route_resistance() {
        let stack: &'static Stack = Box::leak(Box::new(Stack {
            layers: vec![Layer { id: 1, sheet_ohm: 0.125, ..Layer::default() }, Layer { id: 2, sheet_ohm: 4.5, cut: true, ..Layer::default() }],
            antenna_cumulative: false,
        diode: None,
        }));
        let r = Routes {
            wires: vec![vec![
                Shape { layer: LayerId(1), rect: Rect { x: 0, y: 0, w: 10_000, h: 500 } },
                Shape { layer: LayerId(2), rect: Rect { x: 0, y: 0, w: 170, h: 170 } },
            ]],
            ..Default::default()
        };
        let ir = |max_drop_uv| IrDrop { net: NetId(0), current_ua: 1_000, max_drop_uv, margin_pct: 20, stack: Some(stack) };
        assert!(ir(7_000).satisfied(&r) && !ir(6_000).satisfied(&r));
        assert!((ir(3_500).residual(&r) - 1.0).abs() < 1e-4, "twice the budget = one budget over");
        assert!(!IrDrop { stack: None, ..ir(1) }.known(&r), "no stack: unknown");
    }

    /// REL-04: 1 mA from A to B, 19.8 µm between the pin centres on 0.5 µm
    /// met1 (39.6 □, 4.95 Ω), drops 4.95 mV; the 100 □ branch to C carries
    /// nothing and adds nothing (it would add 12.5 mV). The port graph joins
    /// the two runs through the branch's 0-Ω overlap, 250 nm short of the
    /// centreline (4.89 mV), hence 1.5 %. The worst-path bound charges 1 mA
    /// to A→C (≈ 20 + 100 □, ≥ 13 mV).
    #[test]
    fn a_dead_branch_adds_no_drop() {
        let stack: &'static Stack = Box::leak(Box::new(Stack {
            layers: vec![Layer { id: 1, sheet_ohm: 0.125, ..Layer::default() }],
            antenna_cumulative: false,
            diode: None,
        }));
        let m1 = |x, y, w, h| Shape { layer: LayerId(1), rect: Rect { x, y, w, h } };
        let wires = vec![m1(0, 0, 10_000, 500), m1(10_000, 0, 10_000, 500), m1(9_750, 500, 500, 50_000)];
        let terms = vec![
            Terminal { at: Rect { x: 0, y: 0, w: 200, h: 500 }, ua: Some(1_000.0) },
            Terminal { at: Rect { x: 19_800, y: 0, w: 200, h: 500 }, ua: Some(-1_000.0) },
            Terminal { at: Rect { x: 9_750, y: 50_300, w: 500, h: 200 }, ua: Some(0.0) },
        ];
        let ir = IrDrop { net: NetId(0), current_ua: 1_000, max_drop_uv: 1, margin_pct: 20, stack: Some(stack) };
        let bound = ir.drop_uv(&Routes { wires: vec![wires.clone()], ..Default::default() }).unwrap();
        assert!(bound > 13_000.0, "bound {bound}");
        let d = ir.drop_uv(&Routes { wires: vec![wires], terms: vec![terms], ..Default::default() }).unwrap();
        assert!((d - 4_950.0).abs() <= 75.0, "terminal-resolved drop {d}");
    }
}
