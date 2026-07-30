//! The read-only routing state a routing [`crate::Rule`] scores against.

use crate::geom::Shape;
use crate::ids::NetId;

/// Realised route geometry — the state `routing/` rules read, the analogue of
/// [`crate::Layout`] for the placement tier. A routing rule (crosstalk, antenna,
/// differential match…) needs wire shapes and per-net topology, which device
/// centres alone can't express — hence a separate scored state.
pub struct Routes {
    /// Drawn wire/via shapes per net, indexed by [`NetId`].
    pub wires: Vec<Vec<Shape>>,
}

impl Routes {
    /// Drawn shapes of `net`, or an empty slice when the net is unrouted **or out
    /// of range**.
    ///
    /// Out-of-range is a normal condition, not a caller bug: rules carry
    /// [`NetId`]s from the *netlist*, while `wires` is sized by whatever the
    /// router actually realised — a net the router compacted away or never saw
    /// simply has no geometry. Every rule must read shapes through here rather
    /// than indexing, or scoring a partially-routed state panics.
    #[inline]
    #[must_use]
    pub fn shapes(&self, net: NetId) -> &[Shape] {
        self.wires.get(net.0 as usize).map_or(&[], Vec::as_slice)
    }

    /// Stage-boundary check: every drawn shape has a real extent, and each net's
    /// geometry is a **single connected component**.
    ///
    /// A net that comes out of the router in two disjoint pieces is an open, and
    /// an open is exactly what LVS reports later as an `unconnected_pin` — far
    /// downstream of the stage that caused it. Connectivity is tested in xy with
    /// touching counted as connected, which is the right test here because a via
    /// and the two wires it joins are coincident in xy by construction.
    ///
    /// Compiled out without `debug_assertions`; O(k²) per net, k = shapes on that
    /// net. `RUSTFLAGS="-C debug-assertions=yes"` enables it in a release build.
    #[inline]
    pub fn debug_check(&self, ctx: &str) {
        if !cfg!(debug_assertions) {
            return;
        }
        for (net, shapes) in self.wires.iter().enumerate() {
            for (i, s) in shapes.iter().enumerate() {
                assert!(
                    s.rect.w > 0 && s.rect.h > 0,
                    "{ctx}: net {net} shape {i} is degenerate: {:?}",
                    s.rect
                );
            }
            if shapes.len() < 2 {
                continue;
            }
            // Flood from shape 0 over the touch relation.
            let mut seen = vec![false; shapes.len()];
            let mut stack = vec![0usize];
            seen[0] = true;
            while let Some(a) = stack.pop() {
                for b in 0..shapes.len() {
                    if seen[b] {
                        continue;
                    }
                    let (ra, rb) = (shapes[a].rect, shapes[b].rect);
                    let touch = ra.x <= rb.x + rb.w
                        && rb.x <= ra.x + ra.w
                        && ra.y <= rb.y + rb.h
                        && rb.y <= ra.y + ra.h;
                    if touch {
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

    /// Total drawn metal length of `net` in `nm` — the sum of each wire segment's
    /// long dimension. `0` for an unrouted or out-of-range net.
    #[must_use]
    pub fn length(&self, net: NetId) -> i64 {
        self.wires
            .get(net.0 as usize)
            .map(|ws| ws.iter().map(|s| i64::from(s.rect.w.max(s.rect.h))).sum())
            .unwrap_or(0)
    }
}
