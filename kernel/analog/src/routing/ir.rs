//! DC IR drop along a current-carrying net (routing tier, budget).

use pnr_core::ids::NetId;
use pnr_core::routes::Routes;
use crate::rule::Rule;
use super::Stack;

/// The drop a net's routing adds, `I · R_route`, stays within `max_drop_uv`
/// (PWR-02; Lampaert 1999 eq. 2.33 as `R ≤ ΔV_max / I`). `R_route` is the R
/// of the net's most resistive path — sheet R × squares per run plus each
/// cut's R, from the deck ([`Stack::path_resistance_ohm`]).
///
/// Unknown without the stack, a current, or a route.
///
/// ponytail: the whole current is charged to the worst path, which bounds
/// every terminal's drop; the terminal-resolved drop needs a network solve
/// with the terminal currents.
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
        (self.current_ua >= 0 && !shapes.is_empty()).then(|| self.current_ua as f32 * st.path_resistance_ohm(shapes))
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

    /// 1 mA through 20 □ of 0.125 Ω/□ (2.5 Ω) plus a 4.5 Ω cut drops 7 mV.
    #[test]
    fn drop_is_current_times_route_resistance() {
        let stack: &'static Stack = Box::leak(Box::new(Stack {
            layers: vec![Layer { id: 1, sheet_ohm: 0.125, ..Layer::default() }, Layer { id: 2, sheet_ohm: 4.5, cut: true, ..Layer::default() }],
            antenna_cumulative: false,
        diode_layer: None,
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
}
