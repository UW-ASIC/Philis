//! Antenna ratio (routing tier).

use pnr_core::geom::Shape;
use pnr_core::ids::NetId;
use pnr_core::routes::Routes;
use crate::rule::Rule;
use super::Stack;

/// Conductor-area / gate-area ratio on a gate net stays under the limit at
/// every etch stage (MFG-04): per layer at its own stage from the deck's
/// antenna rules ([`Stack::antenna`]; Hastings pp. 228–229), or — without the
/// stack — all routed metal against the tightest ratio.
#[derive(Clone, Copy)]
pub struct Antenna {
    /// The gate net checked.
    pub net: NetId,
    /// Max ratio ×100 (the deck's tightest antenna rule): the fallback limit.
    pub max_ratio_x100: i32,
    /// Total gate area the net drives, nm²: what a piece is charged when the
    /// routes carry no gate pins for the net ([`Stack::antenna`]).
    pub gate_area_nm2: i64,
    /// Safety margin, percent.
    pub margin_pct: u8,
    /// Per-stage limits; `None` = the cumulative all-layer fallback.
    pub stack: Option<&'static Stack>,
}

impl Rule for Antenna {
    type On = Routes;
    const REPAIR: crate::RepairKind = crate::RepairKind::Antenna;
    const LOCAL: bool = true;
    /// Ratio overshoot past the worst stage's limit, ×100.
    fn cost(self, r: &Routes) -> f32 {
        let (ratio, limit) = self.worst(r);
        (ratio - limit).max(0.0) * 100.0
    }
    fn satisfied(self, r: &Routes) -> bool {
        let (ratio, limit) = self.worst(r);
        ratio <= limit
    }
    /// An unrouted net has no conductor to measure: its ratio of 0 is not a pass.
    fn known(self, r: &Routes) -> bool {
        !r.shapes(self.net).is_empty()
    }
    fn touches(self, out: &mut Vec<u32>) {
        out.push(u32::from(self.net.0));
    }
    fn headroom(self, r: &Routes) -> f32 {
        let (ratio, limit) = self.worst(r);
        1.0 - ratio / limit.max(f32::MIN_POSITIVE)
    }
    fn usage(self, r: &Routes) -> Option<f32> {
        let (ratio, limit) = self.worst(r);
        Some(ratio / limit.max(f32::MIN_POSITIVE))
    }
    fn margin(self) -> f32 {
        f32::from(self.margin_pct) / 100.0
    }
    fn residual(self, r: &Routes) -> f32 {
        let (ratio, limit) = self.worst(r);
        crate::rule::over(ratio - limit, limit)
    }
}

impl Antenna {
    /// `(ratio, limit)` of the worst stage. Without the stack (or a stack
    /// with no antenna stage): all routed and cell metal over the gate area
    /// against the tightest ratio (the cumulative, all-layers form, which
    /// over-estimates a per-layer rule), in the ×100 fixed point it has
    /// always been measured in.
    fn worst(self, r: &Routes) -> (f32, f32) {
        let (wires, cell) = (r.shapes(self.net), r.cell_metal(self.net));
        if let Some(stack) = self.stack {
            // ponytail: O(all shapes) per evaluation; a per-layer spatial index if routing time shows it.
            let me = self.net.0 as usize;
            let others: Vec<Shape> = r.wires.iter().enumerate().chain(r.cell.iter().enumerate()).filter(|&(k, _)| k != me).flat_map(|(_, v)| v.iter().copied()).collect();
            if let Some(w) = stack.antenna(wires, cell, &others, r.gate_pins(self.net), self.gate_area_nm2) {
                return w;
            }
        }
        let area: i64 = wires.iter().chain(cell).map(|s| i64::from(s.rect.w) * i64::from(s.rect.h)).sum();
        ((area * 100 / self.gate_area_nm2.max(1)) as f32 / 100.0, self.max_ratio_x100 as f32 / 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing::stack::Layer;
    use pnr_core::geom::{LayerId, Rect};

    fn m1(x: i32, y: i32, w: i32, h: i32) -> Shape {
        Shape { layer: LayerId(1), rect: Rect { x, y, w, h } }
    }

    /// Fallback limit 50 (×100 = 5 000), 0.1 µm² of gate.
    fn rule(stack: Option<&'static Stack>) -> Antenna {
        Antenna { net: NetId(0), max_ratio_x100: 5_000, gate_area_nm2: 100_000, margin_pct: 20, stack }
    }

    /// Without a stack: 10 µm² of metal over 0.1 µm² of gate is ratio 100
    /// against the 50 fallback, one full limit over.
    #[test]
    fn without_a_stack_all_metal_counts_against_the_tightest_ratio() {
        let r = Routes { wires: vec![vec![m1(0, 0, 10_000, 1_000)]], ..Default::default() };
        let a = rule(None);
        assert!(a.known(&r) && !a.satisfied(&r));
        assert!((a.cost(&r) - 5_000.0).abs() < 1e-2, "{}", a.cost(&r));
        assert!((a.residual(&r) - 1.0).abs() < 1e-6);
        assert!((a.usage(&r).unwrap() - 2.0).abs() < 1e-6);
        assert!((a.headroom(&r) + 1.0).abs() < 1e-6);
        assert!((a.margin() - 0.2).abs() < 1e-6);
    }

    /// Cell metal is part of the conductor, but a net with nothing routed
    /// has no antenna measurement to certify.
    #[test]
    fn cell_metal_counts_but_alone_is_unknown() {
        let r = Routes { wires: vec![vec![]], cell: vec![vec![m1(0, 0, 4_000, 1_000)]], ..Default::default() };
        let a = rule(None);
        assert!(!a.known(&r), "nothing routed");
        assert!(a.satisfied(&r), "4 µm² / 0.1 µm² = 40 ≤ 50");
        assert!((a.usage(&r).unwrap() - 0.8).abs() < 1e-6);
        let both = Routes { wires: vec![vec![m1(0, 0, 2_000, 1_000)]], ..r };
        assert!(!a.satisfied(&both), "2 + 4 µm²: 60 > 50");
    }

    /// A stack whose deck checks no stage measures nothing per stage: the
    /// cumulative fallback applies, exactly as without a stack.
    #[test]
    fn a_stack_without_antenna_stages_falls_back() {
        let st: &'static Stack = Box::leak(Box::new(Stack { layers: vec![Layer { id: 1, ..Layer::default() }], ..Stack::default() }));
        let r = Routes { wires: vec![vec![m1(0, 0, 10_000, 1_000)]], ..Default::default() };
        assert_eq!(rule(Some(st)).residual(&r), rule(None).residual(&r));
        assert_eq!(rule(Some(st)).usage(&r), rule(None).usage(&r));
    }

    /// GAP-13 at the rule level: another net's wire **or** cell metal 140 nm
    /// off joins the piece (60 + 60 > 100); alone it passes.
    #[test]
    fn foreign_wires_and_cell_metal_join_by_latent_merge() {
        let st: &'static Stack =
            Box::leak(Box::new(Stack { layers: vec![Layer { id: 1, antenna_ratio: 100.0, latent_merge_nm: 140, ..Layer::default() }], ..Stack::default() }));
        let a = Antenna { gate_area_nm2: 1_000_000, ..rule(Some(st)) };
        let mine = vec![m1(0, 0, 60_000, 1_000)];
        let near = m1(0, 1_140, 60_000, 1_000);
        assert!(a.satisfied(&Routes { wires: vec![mine.clone()], ..Default::default() }), "60 ≤ 100");
        assert!(!a.satisfied(&Routes { wires: vec![mine.clone(), vec![near]], ..Default::default() }), "a foreign wire merges");
        let celled = Routes { wires: vec![mine, vec![]], cell: vec![vec![], vec![near]], ..Default::default() };
        assert!(!a.satisfied(&celled), "a foreign cell's metal merges too");
    }

    /// The latent merge reads other nets' shapes, so a score cached on this
    /// net's shapes alone goes stale when they move: not local (RTE-23).
    #[test]
    fn reading_foreign_nets_makes_the_rule_non_local() {
        assert!(!<Antenna as Rule>::LOCAL);
    }

    #[test]
    fn touches_the_gate_net() {
        let mut v = Vec::new();
        Antenna { net: NetId(7), ..rule(None) }.touches(&mut v);
        assert_eq!(v, vec![7]);
    }
}
