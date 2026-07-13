//! Exact point primitive and orientation predicates.

/// A point in layout database units.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

impl From<(i32, i32)> for Point {
    fn from((x, y): (i32, i32)) -> Self {
        Self { x, y }
    }
}

/// Orientation of an ordered point triple, or winding of a non-degenerate ring.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    Clockwise,
    Collinear,
    CounterClockwise,
}

/// Exact orientation over the complete `i32` coordinate range.
pub fn orientation(a: Point, b: Point, c: Point) -> Orientation {
    match cross(a, b, c).cmp(&0) {
        std::cmp::Ordering::Less => Orientation::Clockwise,
        std::cmp::Ordering::Equal => Orientation::Collinear,
        std::cmp::Ordering::Greater => Orientation::CounterClockwise,
    }
}

#[inline]
pub(super) fn cross(a: Point, b: Point, c: Point) -> i128 {
    let abx = b.x as i128 - a.x as i128;
    let aby = b.y as i128 - a.y as i128;
    let acx = c.x as i128 - a.x as i128;
    let acy = c.y as i128 - a.y as i128;
    abx * acy - aby * acx
}

/// True when `p` lies on the closed segment `ab`.
pub fn on_segment(a: Point, b: Point, p: Point) -> bool {
    cross(a, b, p) == 0
        && p.x >= a.x.min(b.x)
        && p.x <= a.x.max(b.x)
        && p.y >= a.y.min(b.y)
        && p.y <= a.y.max(b.y)
}
