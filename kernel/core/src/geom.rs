//! Geometry primitives. Plain data, all coordinates in nanometres (`nm`).

use crate::ids::NetId;

/// A PDK layer, by index into the PDK's layer table.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct LayerId(pub u16);

/// Axis-aligned rectangle in `nm`. `(x, y)` is the lower-left corner.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    /// Closed intervals on both axes: edge or corner contact counts, so two
    /// abutting shapes on one conductor are connected.
    #[must_use]
    pub fn touches(&self, o: &Rect) -> bool {
        self.x <= o.x + o.w && o.x <= self.x + self.w && self.y <= o.y + o.h && o.y <= self.y + self.h
    }
}

/// How a placed cell is turned before stamping: the D4 group, the same eight
/// transforms GDSII `SREF` encodes as `(angle, mirror_x)`. Which members are
/// legal for a device is a constraint concern, not this type's.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Orient {
    #[default]
    R0,
    R90,
    R180,
    R270,
    /// Mirror about the x-axis (`y ↦ −y`), then rotate by the named angle.
    Mx,
    Mx90,
    Mx180,
    Mx270,
}

impl Orient {
    /// The four 90° members: they swap a device's `hw`/`hh`.
    #[must_use]
    pub fn swaps_axes(self) -> bool {
        matches!(self, Orient::R90 | Orient::R270 | Orient::Mx90 | Orient::Mx270)
    }

    /// Map a point about the origin.
    #[must_use]
    pub fn apply(self, x: i32, y: i32) -> (i32, i32) {
        match self {
            Orient::R0 => (x, y),
            Orient::R90 => (-y, x),
            Orient::R180 => (-x, -y),
            Orient::R270 => (y, -x),
            Orient::Mx => (x, -y),
            Orient::Mx90 => (y, x),
            Orient::Mx180 => (-x, y),
            Orient::Mx270 => (-y, -x),
        }
    }

    /// Map a rectangle about the origin, renormalised to lower-left `(x, y)`.
    /// Grid-preserving.
    #[must_use]
    pub fn apply_rect(self, r: Rect) -> Rect {
        let (x0, y0) = self.apply(r.x, r.y);
        let (x1, y1) = self.apply(r.x + r.w, r.y + r.h);
        Rect { x: x0.min(x1), y: y0.min(y1), w: (x1 - x0).abs(), h: (y1 - y0).abs() }
    }
}

/// One drawn rectangle on one layer — the atom of [`crate::Macro`] geometry.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Shape {
    pub layer: LayerId,
    pub rect: Rect,
}

/// A physical point where a net attaches to a macro — a routing entry point.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Pin {
    pub name: String,
    pub net: NetId,
    pub at: Rect,
    /// The layer the pin is drawn on (pin access must not guess it).
    pub layer: LayerId,
}

/// Signal direction of a port.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dir {
    In,
    Out,
    InOut,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_contact_touches_and_a_gap_does_not() {
        let a = Rect { x: 0, y: 0, w: 10, h: 10 };
        assert!(a.touches(&Rect { x: 10, y: 0, w: 5, h: 5 }));
        assert!(!a.touches(&Rect { x: 11, y: 0, w: 5, h: 5 }));
    }

    const ALL: [Orient; 8] = [
        Orient::R0,
        Orient::R90,
        Orient::R180,
        Orient::R270,
        Orient::Mx,
        Orient::Mx90,
        Orient::Mx180,
        Orient::Mx270,
    ];

    /// D4 is a group of order 8: no two members may act identically, or the
    /// placer's rotate move would propose a no-op it believes is a real turn.
    #[test]
    fn the_eight_transforms_are_distinct() {
        // (1, 2) is asymmetric under every axis, so it separates all 8.
        let mut seen: Vec<(i32, i32)> = ALL.iter().map(|o| o.apply(1, 2)).collect();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), 8);
    }

    /// `swaps_axes` is what the placer trusts when it transposes `hw`/`hh`; if it
    /// disagrees with the actual transform the layout's extents go stale.
    #[test]
    fn swaps_axes_matches_the_drawn_extents() {
        let r = Rect { x: 10, y: 20, w: 300, h: 700 };
        for o in ALL {
            let t = o.apply_rect(r);
            if o.swaps_axes() {
                assert_eq!((t.w, t.h), (r.h, r.w), "{o:?}");
            } else {
                assert_eq!((t.w, t.h), (r.w, r.h), "{o:?}");
            }
        }
    }

    /// Four quarter-turns are the identity, and a rect stays on-grid throughout —
    /// the property the stamping site relies on to keep geometry snapped.
    #[test]
    fn rotation_is_grid_preserving_and_periodic() {
        let r = Rect { x: 15, y: -40, w: 305, h: 700 };
        let mut p = (r.x, r.y);
        for _ in 0..4 {
            p = Orient::R90.apply(p.0, p.1);
            assert_eq!((p.0 % 5, p.1 % 5), (0, 0), "left the 5nm grid");
        }
        assert_eq!(p, (r.x, r.y));
        assert_eq!(Orient::R180.apply_rect(Orient::R180.apply_rect(r)), r);
    }
}
