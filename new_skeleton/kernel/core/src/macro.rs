//! The common currency of geometry: [`Macro`].

use crate::geom::{Pin, Rect, Shape};

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
