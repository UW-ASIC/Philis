//! Metal over a matched gate (routing tier, hard).

use pnr_core::geom::Rect;
use pnr_core::routes::Routes;

use crate::rule::Rule;
use super::stack::overlap_area_nm2;

/// Most routed metals [`MetalOverGate::metals`] lists.
pub const MAX_METALS: usize = 8;

/// No routed metal over `rect`, a moderately or exceptionally matched gate of
/// placed cell `cell` (absolute nm): "must not have metal leads crossing
/// their active areas" (Hastings §13.3 rule 17), on any metal (metal coverage
/// skews `I_D` "regardless of what metal layers are involved", §13.3.2).
/// Built per routed layout (the rect is placed geometry); `dr` evaluates it.
#[derive(Clone, Copy, Debug)]
pub struct MetalOverGate {
    /// The gate's active area, absolute nm.
    pub rect: Rect,
    /// Index of the placed cell the gate belongs to (for reports and repair).
    pub cell: u32,
    /// The routed metals' `LayerId`s, `u16::MAX`-padded: a cut or the pin-access
    /// conductor over the gate is not a lead.
    pub metals: [u16; MAX_METALS],
}

impl MetalOverGate {
    /// Σ overlap of every routed wire shape (any net) on a listed metal with
    /// `rect`, µm². Overlapping shapes count their shared area twice; cell
    /// metal ([`Routes::cell`]) is not a lead and is ignored. O(all shapes).
    #[must_use]
    pub fn overlap_um2(self, r: &Routes) -> f32 {
        let nm2: i64 = r.wires.iter().flatten().filter(|s| self.metals.contains(&s.layer.0)).map(|s| overlap_area_nm2(&s.rect, &self.rect)).sum();
        (nm2 as f64 * 1e-6) as f32
    }
}

impl Rule for MetalOverGate {
    type On = Routes;
    const REPAIR: crate::RepairKind = crate::RepairKind::None;
    fn cost(self, r: &Routes) -> f32 {
        self.residual(r)
    }
    fn satisfied(self, r: &Routes) -> bool {
        self.overlap_um2(r) == 0.0
    }
    /// Overlap area, µm² (the spec is zero, so not normalised).
    fn residual(self, r: &Routes) -> f32 {
        self.overlap_um2(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_core::geom::{LayerId, Shape};

    #[test]
    fn overlap_is_area_in_um2() {
        let mut metals = [u16::MAX; MAX_METALS];
        metals[0] = 1;
        let rule = MetalOverGate { rect: Rect { x: 500, y: 0, w: 1_000, h: 1_000 }, cell: 0, metals };
        // 1 000 × 500 nm shape, half (500 × 500) over the gate.
        let r = Routes { wires: vec![vec![Shape { layer: LayerId(1), rect: Rect { x: 0, y: 0, w: 1_000, h: 500 } }]], ..Default::default() };
        assert!((rule.residual(&r) - 0.25).abs() < 1e-6);
        assert!(!rule.satisfied(&r));
        // The same shape on a layer that is no routed metal is not a lead.
        let cut = Routes { wires: vec![vec![Shape { layer: LayerId(2), rect: Rect { x: 0, y: 0, w: 1_000, h: 500 } }]], ..Default::default() };
        assert_eq!(rule.residual(&cut), 0.0);
    }

    fn gate() -> MetalOverGate {
        let mut metals = [u16::MAX; MAX_METALS];
        metals[0] = 1;
        MetalOverGate { rect: Rect { x: 0, y: 0, w: 1_000, h: 1_000 }, cell: 0, metals }
    }
    fn m1(x: i32, y: i32, w: i32, h: i32) -> Shape {
        Shape { layer: LayerId(1), rect: Rect { x, y, w, h } }
    }

    #[test]
    fn edge_contact_and_cell_metal_are_not_leads() {
        assert!(gate().satisfied(&Routes::default()));
        let edge = Routes { wires: vec![vec![m1(1_000, 0, 500, 1_000)]], ..Default::default() };
        assert!(gate().satisfied(&edge), "touching the edge is not over");
        let cell = Routes { cell: vec![vec![m1(0, 0, 1_000, 1_000)]], ..Default::default() };
        assert!(gate().satisfied(&cell), "a cell's own metal is not a routed lead");
    }

    #[test]
    fn every_nets_overlap_adds() {
        let two = Routes { wires: vec![vec![m1(0, 0, 1_000, 100)], vec![m1(0, 900, 1_000, 100)]], ..Default::default() };
        assert!((gate().overlap_um2(&two) - 0.2).abs() < 1e-6);
        assert_eq!(gate().cost(&two), gate().residual(&two));
        // Translating gate and metal together changes nothing.
        let mv = |s: Shape| Shape { rect: Rect { x: s.rect.x - 7_000, y: s.rect.y + 3_000, ..s.rect }, ..s };
        let moved = Routes { wires: two.wires.iter().map(|w| w.iter().copied().map(mv).collect()).collect(), ..Default::default() };
        let g = MetalOverGate { rect: Rect { x: -7_000, y: 3_000, w: 1_000, h: 1_000 }, ..gate() };
        assert_eq!(g.overlap_um2(&moved), gate().overlap_um2(&two));
    }
}
