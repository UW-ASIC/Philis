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
}
