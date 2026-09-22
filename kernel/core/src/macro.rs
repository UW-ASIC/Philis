//! [`Macro`] and the one placement stamp that turns local drawings into placed
//! shapes.

use crate::geom::{Pin, Rect, Shape};
use crate::layout::Layout;

/// A placeable unit of drawn geometry — what `cells` draws and `macroMaster`
/// injects; downstream cannot tell them apart.
#[derive(Clone, PartialEq, Debug)]
pub struct Macro {
    pub shapes: Vec<Shape>,
    /// Routing entry points.
    pub pins: Vec<Pin>,
    /// Bounding box of `shapes`, nm.
    pub bbox: Rect,
}

/// Stamp macro `m` at device `i`'s placed position: turn by `l.orient[i]`, then
/// centre the turned bbox on `(l.x[i], l.y[i])` using `l.hw/hh` (the layout is a
/// centre + half-extent model). Turning about the bbox corner keeps the result
/// on-grid. `i` past the layout (guard rings) means already absolute: returned
/// unchanged.
#[must_use]
pub fn place_macro(m: &Macro, l: &Layout, i: usize) -> Macro {
    if i >= l.x.len() {
        return m.clone();
    }
    let o = l.orient.get(i).copied().unwrap_or_default();
    let anchor = o.apply_rect(m.bbox);
    let (ax, ay) = (l.x[i] - l.hw[i] - anchor.x, l.y[i] - l.hh[i] - anchor.y);
    let shift = |r: Rect| {
        let r = o.apply_rect(r);
        Rect { x: r.x + ax, y: r.y + ay, w: r.w, h: r.h }
    };
    Macro {
        bbox: shift(m.bbox),
        shapes: m.shapes.iter().map(|s| Shape { layer: s.layer, rect: shift(s.rect) }).collect(),
        pins: m.pins.iter().map(|p| Pin { at: shift(p.at), ..p.clone() }).collect(),
    }
}

/// [`place_macro`] over the whole table.
#[must_use]
pub fn place_macros(macros: &[Macro], l: &Layout) -> Vec<Macro> {
    macros.iter().enumerate().map(|(i, m)| place_macro(m, l, i)).collect()
}
