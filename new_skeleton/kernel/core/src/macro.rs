//! The common currency of geometry: [`Macro`], and the one placement stamp
//! ([`place_macro`] / [`place_macros`]) that turns local drawings into placed
//! shapes.

use crate::geom::{Orient, Pin, Rect, Shape};
use crate::layout::Layout;

/// A placeable unit of drawn geometry — the common currency of `cells`
/// (auto-drawn) and `macroMaster` (user-injected).
///
/// Placement and routing treat the two sources identically; nothing downstream
/// can tell a drawn cell from an injected one. That indistinguishability is why
/// both crates target this single type instead of two parallel ones.
#[derive(Clone, PartialEq, Debug)]
pub struct Macro {
    /// Drawn rectangles, one per `(layer, rect)`.
    pub shapes: Vec<Shape>,
    /// Points where nets attach — the routing entry points.
    pub pins: Vec<Pin>,
    /// Bounding box in `nm`, derived from `shapes`, cached for cheap placement
    /// queries.
    pub bbox: Rect,
}

/// Translate one macro into device `i`'s **placed** position, matching the
/// convention `frontend/library::geometry::collect` uses for emitted geometry.
///
/// Turning about the bbox lower-left corner (rather than the centre) keeps the
/// whole macro on the fabrication grid: every D4 orientation maps grid multiples
/// to grid multiples, whereas rotating about a centre lands half-pitch off
/// whenever an extent is an odd number of grid steps.
///
/// A device past the end of the layout keeps its local coordinates, which is the
/// same fallback `collect` applies (guard rings arrive as absolute-coordinate
/// macros appended after the device list).
///
/// Lives here rather than in `gr` (its original home) because the oracle tier
/// stamps bbox-scoped regions from inside `dp`, which must not depend on a
/// router to place a macro. `gr::place_macros` re-exports [`place_macros`], so
/// nothing else changed.
#[must_use]
pub fn place_macro(m: &Macro, l: &Layout, i: usize) -> Macro {
    if i >= l.x.len() {
        return m.clone(); // already absolute
    }
    let (cx, cy) = (l.x[i], l.y[i]);
    let o = l.orient.get(i).copied().unwrap_or_default();
    let anchor = o.apply_rect(m.bbox);
    // Centre the turned bbox on the placed centre. `Layout` is a
    // centre + half-extents model — `gp::mechanics::half_extents` builds
    // `hw`/`hh` from `bbox.w/2` and discards `bbox.x`/`bbox.y` — so
    // anchoring the macro's *local origin* here instead left every drawn
    // device offset from where the placer legalised it by
    // `(hw + bbox.x, hh + bbox.y)`, measured up to 20 µm. Guard rings are
    // appended past `l.x.len()` and are already absolute, so they stayed
    // put while the devices moved, and their tap bands cut through the
    // diffusion they were meant to surround.
    let (hw, hh) = (l.hw[i], l.hh[i]);
    let (ax, ay) = (cx - hw - anchor.x, cy - hh - anchor.y);
    let shift = |r: Rect| {
        let r = if o == Orient::R0 { r } else { o.apply_rect(r) };
        Rect { x: r.x + ax, y: r.y + ay, w: r.w, h: r.h }
    };
    Macro {
        bbox: shift(m.bbox),
        shapes: m
            .shapes
            .iter()
            .map(|s| Shape { layer: s.layer, rect: shift(s.rect) })
            .collect(),
        pins: m
            .pins
            .iter()
            .map(|p| Pin { at: shift(p.at), ..p.clone() })
            .collect(),
    }
}

/// Translate every macro into its **placed** position — [`place_macro`] over the
/// whole table.
#[must_use]
pub fn place_macros(macros: &[Macro], l: &Layout) -> Vec<Macro> {
    macros
        .iter()
        .enumerate()
        .map(|(i, m)| place_macro(m, l, i))
        .collect()
}
