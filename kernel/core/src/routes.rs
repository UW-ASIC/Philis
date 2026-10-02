//! Realised route geometry — the state routing rules score against.

use crate::geom::{Rect, Shape};
use crate::ids::NetId;

/// A routed terminal and the DC current it draws from its net, µA
/// (+ into the device); `None` = unknown.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Terminal {
    pub at: Rect,
    pub ua: Option<f32>,
}

#[derive(Default)]
pub struct Routes {
    /// Drawn wire/via shapes per net, by [`NetId`].
    pub wires: Vec<Vec<Shape>>,
    /// Per net, the placed cells' metal its pins reach: drawn by the cells,
    /// not the router, and scored with the wires where a rule reads the whole
    /// conductor (antenna).
    pub cell: Vec<Vec<Shape>>,
    /// Per net, its gate pins' rects: where the conductor meets a gate.
    pub gates: Vec<Vec<crate::geom::Rect>>,
    /// Per net, every pin `dr` routed to, absolute, with its DC current
    /// (the electromigration rule's current sources and sinks).
    pub terms: Vec<Vec<Terminal>>,
}

impl Routes {
    /// Shapes of `net`; empty when unrouted or out of range (rules carry netlist
    /// ids, `wires` is sized by what the router realised).
    #[inline]
    #[must_use]
    pub fn shapes(&self, net: NetId) -> &[Shape] {
        self.wires.get(net.0 as usize).map_or(&[], Vec::as_slice)
    }

    /// [`Routes::cell`] of `net`; empty when unknown.
    #[must_use]
    pub fn cell_metal(&self, net: NetId) -> &[Shape] {
        self.cell.get(net.0 as usize).map_or(&[], Vec::as_slice)
    }

    /// [`Routes::gates`] of `net`; empty when unknown.
    #[must_use]
    pub fn gate_pins(&self, net: NetId) -> &[crate::geom::Rect] {
        self.gates.get(net.0 as usize).map_or(&[], Vec::as_slice)
    }

    /// [`Routes::terms`] of `net`; empty when unknown.
    #[must_use]
    pub fn terminals(&self, net: NetId) -> &[Terminal] {
        self.terms.get(net.0 as usize).map_or(&[], Vec::as_slice)
    }

    /// Debug-only stage-boundary check: no degenerate shapes, and each net is
    /// one connected component in xy (touching counts). An open here is an
    /// LVS `unconnected_pin` later. O(k²) per net.
    #[inline]
    pub fn debug_check(&self, ctx: &str) {
        if !cfg!(debug_assertions) {
            return;
        }
        for (net, shapes) in self.wires.iter().enumerate() {
            for (i, s) in shapes.iter().enumerate() {
                assert!(s.rect.w > 0 && s.rect.h > 0, "{ctx}: net {net} shape {i} is degenerate: {:?}", s.rect);
            }
            if shapes.len() < 2 {
                continue;
            }
            let mut seen = vec![false; shapes.len()];
            let mut stack = vec![0usize];
            seen[0] = true;
            while let Some(a) = stack.pop() {
                for b in 0..shapes.len() {
                    if !seen[b] && shapes[a].rect.touches(&shapes[b].rect) {
                        seen[b] = true;
                        stack.push(b);
                    }
                }
            }
            let reached = seen.iter().filter(|&&v| v).count();
            assert_eq!(
                reached,
                shapes.len(),
                "{ctx}: net {net} is open — {reached} of {} shapes reachable from the first",
                shapes.len()
            );
        }
    }

    /// Drawn metal length of `net`, nm: Σ of each shape's long side.
    #[must_use]
    pub fn length(&self, net: NetId) -> i64 {
        self.shapes(net).iter().map(|s| i64::from(s.rect.w.max(s.rect.h))).sum()
    }
}
