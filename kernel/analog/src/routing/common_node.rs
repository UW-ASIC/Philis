//! Common-node resistance balance (routing tier, budget).

use pnr_core::geom::Rect;
use pnr_core::ids::NetId;
use pnr_core::routes::Routes;

use crate::rule::RuleBatch;
use super::Stack;

/// Two matched devices share a source net (a pair's tail, a mirror's rail):
/// the routed R from the net's centre to each member's source pins must
/// agree, or the degeneration differs. `ΔR` in series with a source carrying
/// `I` reads as an offset `I·ΔR` (Hastings §13.3.3; a 10 Ω skew at 1 mS is a
/// 1% mirror error), so the budget is the offset share the pair allows,
/// `max_delta_ohm = η·σ_rand / I`.
///
/// Built per routed layout: the pins are placed geometry. Each side's pins
/// combine in parallel ([`Stack::terminal_resistance_ohm`]).
#[derive(Clone, Debug)]
pub struct CommonNode {
    pub net: NetId,
    /// Source pin rects of member a and member b.
    pub a: Vec<Rect>,
    pub b: Vec<Rect>,
    /// Where the current enters the net: every other pin on it (a tail
    /// device's drain, a rail's port). Empty = the net's centre.
    pub feeds: Vec<Rect>,
    /// Allowed `|R_a − R_b|`, Ω; `≤ 0` = unknown (no current or `A_VT`).
    pub max_delta_ohm: f32,
}

/// Every common node of one routed layout, measured on `stack`.
#[derive(Clone)]
pub struct CommonNodes {
    pub nodes: Vec<CommonNode>,
    pub stack: &'static Stack,
}

impl CommonNodes {
    /// `|R_a − R_b|`, Ω; `None` when a side's pins are not all reached.
    fn delta_ohm(&self, n: &CommonNode, r: &Routes) -> Option<f32> {
        let pins: Vec<Rect> = n.a.iter().chain(&n.b).copied().collect();
        let branch = if n.feeds.is_empty() {
            self.stack.terminal_resistance_ohm(r.shapes(n.net), &pins)
        } else {
            self.stack.fed_resistance_ohm(r.shapes(n.net), &n.feeds, &pins)
        };
        let side = |k: std::ops::Range<usize>| -> Option<f32> {
            let g: f32 = branch[k].iter().map(|r| r.map(|r| 1.0 / r.max(1e-3))).sum::<Option<f32>>()?;
            (g > 0.0).then(|| 1.0 / g)
        };
        Some((side(0..n.a.len())? - side(n.a.len()..pins.len())?).abs())
    }

    fn usage(&self, n: &CommonNode, r: &Routes) -> Option<f32> {
        (n.max_delta_ohm > 0.0).then_some(())?;
        Some(self.delta_ohm(n, r)? / n.max_delta_ohm)
    }
}

impl RuleBatch<Routes> for CommonNodes {
    fn cost(&self, r: &Routes) -> f32 {
        self.nodes.iter().filter_map(|n| self.usage(n, r)).sum()
    }
    fn violations(&self, r: &Routes) -> u32 {
        self.nodes.iter().filter(|n| self.usage(n, r).is_some_and(|u| u > 1.0)).count() as u32
    }
    fn residual(&self, r: &Routes) -> f64 {
        self.nodes.iter().filter_map(|n| self.usage(n, r)).map(|u| f64::from((u - 1.0).max(0.0))).sum()
    }
    fn kind(&self) -> &'static str {
        "CommonNode"
    }
    fn repair_kind(&self) -> crate::RepairKind {
        crate::RepairKind::Balance
    }
    fn count(&self) -> usize {
        self.nodes.len()
    }
    fn worst_usage(&self, r: &Routes) -> Option<f32> {
        self.nodes.iter().filter_map(|n| self.usage(n, r)).reduce(f32::max)
    }
    fn unknown(&self, r: &Routes) -> u32 {
        self.nodes.iter().filter(|n| self.usage(n, r).is_none()).count() as u32
    }
    fn touched(&self, out: &mut Vec<u32>) {
        out.extend(self.nodes.iter().map(|n| u32::from(n.net.0)));
    }
    fn violating_ids(&self, r: &Routes, out: &mut Vec<u32>) {
        out.extend(self.nodes.iter().filter(|n| self.usage(n, r).is_some_and(|u| u > 1.0)).map(|n| u32::from(n.net.0)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing::stack::Layer;
    use pnr_core::geom::{LayerId, Shape};

    /// A tail rail fed from one end: the near member's source sees less R
    /// than the far one's. Balanced (fed from the middle), the skew vanishes.
    #[test]
    fn a_rail_fed_from_one_end_skews_the_pair() {
        let stack: &'static Stack = Box::leak(Box::new(Stack {
            layers: vec![Layer { id: 1, sheet_ohm: 0.125, ..Layer::default() }],
            antenna_cumulative: false,
            diode_layer: None,
        }));
        let wire = |x, y, w, h| Shape { layer: LayerId(1), rect: Rect { x, y, w, h } };
        let pin = |x| Rect { x, y: 0, w: 100, h: 100 };
        let mut routes = Routes { wires: vec![Vec::new()], ..Default::default()  };
        // 40 µm rail at 0.5 µm: members at x = 0 and 10 µm, fed at 40 µm.
        routes.wires[0] = vec![wire(0, 0, 40_100, 500)];
        let node = |feed: i32, max| CommonNodes {
            nodes: vec![CommonNode { net: NetId(0), a: vec![pin(0)], b: vec![pin(10_000)], feeds: vec![pin(feed)], max_delta_ohm: max }],
            stack,
        };
        assert_eq!(node(40_000, 0.0).unknown(&routes), 1, "no budget: unknown, never a pass");
        // 10 µm more rail to a: 20 squares, 2.5 Ω of skew.
        let u = node(40_000, 1.0).worst_usage(&routes).unwrap();
        assert!((u - 2.5).abs() < 1e-3, "{u}");
        assert_eq!(node(40_000, 1.0).violations(&routes), 1);
        // Fed midway between the members: no skew.
        assert!(node(5_000, 1.0).worst_usage(&routes).unwrap() < 1e-3);
    }
}
