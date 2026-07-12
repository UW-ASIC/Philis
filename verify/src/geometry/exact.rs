//! Exact layout-geometry foundation.
//!
//! # Supported semantics
//!
//! Predicates in this module accept the complete `i32` coordinate domain and make
//! every topological decision with `i128` integer arithmetic. Rings are stored
//! without a repeated closing vertex, must be simple and non-degenerate, and may
//! contain arbitrary-angle edges. [`Polygon`] canonicalizes outer rings to
//! counter-clockwise winding and holes to clockwise winding. Holes must be
//! strictly contained and may not touch or overlap each other.
//!
//! Boolean union, intersection, and subtraction are exact for rectilinear input.
//! They build the arrangement induced by input x/y coordinates, classify each
//! open arrangement face, and reconstruct oriented boundaries. This is coordinate
//! decomposition, not a fixed-resolution raster: no rounding or sampling scale is
//! involved, and output vertices are input coordinates. Disconnected components,
//! holes, and boundary-connected cut-outs (often called keyholes/notches) are
//! preserved. Arbitrary-angle boolean input returns [`ExactGeometryError::Unsupported`].
//!
//! # Checker migration contract
//!
//! DRC/LVS callers may use a bbox only to reject impossible candidates. A candidate
//! that survives must be decided by these predicates or booleans. Unsupported
//! geometry and capacity limits are errors and must propagate to the run status;
//! callers must never substitute bbox overlap, a raster estimate, or `CLEAN`.
//! Arbitrary-angle booleans, exact offsets, and scalable sweep-line construction
//! remain future work, so this module intentionally exposes no approximate fallback.

use std::collections::BTreeMap;
use std::fmt;

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

/// The exact topological relationship between two closed segments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentIntersection {
    None,
    /// A single shared endpoint or a point-on-segment contact.
    Touch,
    /// Interior points of both non-collinear segments cross.
    Proper,
    /// A collinear interval of positive length is shared.
    Overlap,
}

/// Classification of a point against a simple ring.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointClassification {
    Outside,
    Boundary,
    Inside,
}

/// Filled-area relationship of two simple polygon rings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolygonContact {
    Disjoint,
    /// Boundaries meet, but interiors do not overlap.
    Touch,
    /// The intersection has positive area (including equality/containment).
    AreaOverlap,
}

/// Winding required by the canonical polygon-set representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Winding {
    Clockwise,
    CounterClockwise,
}

/// Boolean operation supported by [`rectilinear_boolean`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BooleanOp {
    Union,
    Intersection,
    Subtraction,
}

/// Stable, fail-closed errors returned by exact geometry construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExactGeometryError {
    TooFewVertices {
        count: usize,
    },
    DuplicateConsecutiveVertex {
        index: usize,
    },
    DegenerateRing,
    ArithmeticOverflow,
    InvalidBoundaryWalk(&'static str),
    SelfIntersection {
        edge_a: usize,
        edge_b: usize,
        kind: SegmentIntersection,
    },
    HoleOutside {
        hole: usize,
    },
    HoleTouchesOuter {
        hole: usize,
    },
    HoleConflict {
        hole_a: usize,
        hole_b: usize,
        contact: PolygonContact,
    },
    Unsupported {
        operation: BooleanOp,
        reason: &'static str,
    },
    CapacityExceeded {
        cells: usize,
        limit: usize,
    },
    InternalTopology(&'static str),
}

impl fmt::Display for ExactGeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewVertices { count } => {
                write!(f, "ring has {count} vertices; at least three are required")
            }
            Self::DuplicateConsecutiveVertex { index } => {
                write!(
                    f,
                    "ring has a duplicate consecutive vertex at index {index}"
                )
            }
            Self::DegenerateRing => write!(f, "ring has zero signed area"),
            Self::ArithmeticOverflow => write!(f, "exact geometry arithmetic overflowed i128"),
            Self::InvalidBoundaryWalk(reason) => {
                write!(f, "invalid boundary walk: {reason}")
            }
            Self::SelfIntersection {
                edge_a,
                edge_b,
                kind,
            } => write!(
                f,
                "ring edges {edge_a} and {edge_b} have forbidden {kind:?} contact"
            ),
            Self::HoleOutside { hole } => {
                write!(f, "hole {hole} is not strictly contained by its outer ring")
            }
            Self::HoleTouchesOuter { hole } => {
                write!(f, "hole {hole} touches or crosses its outer ring")
            }
            Self::HoleConflict {
                hole_a,
                hole_b,
                contact,
            } => write!(
                f,
                "holes {hole_a} and {hole_b} have forbidden {contact:?} contact"
            ),
            Self::Unsupported { operation, reason } => {
                write!(f, "{operation:?} is unsupported: {reason}")
            }
            Self::CapacityExceeded { cells, limit } => write!(
                f,
                "rectilinear arrangement requires {cells} cells (limit {limit})"
            ),
            Self::InternalTopology(reason) => {
                write!(f, "boolean boundary reconstruction failed: {reason}")
            }
        }
    }
}

impl std::error::Error for ExactGeometryError {}

/// Exact orientation over the complete `i32` coordinate range.
pub fn orientation(a: Point, b: Point, c: Point) -> Orientation {
    match cross(a, b, c).cmp(&0) {
        std::cmp::Ordering::Less => Orientation::Clockwise,
        std::cmp::Ordering::Equal => Orientation::Collinear,
        std::cmp::Ordering::Greater => Orientation::CounterClockwise,
    }
}

#[inline]
fn cross(a: Point, b: Point, c: Point) -> i128 {
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

/// Classify the intersection of two closed segments exactly.
pub fn classify_segment_intersection(
    a0: Point,
    a1: Point,
    b0: Point,
    b1: Point,
) -> SegmentIntersection {
    if a0.x.max(a1.x) < b0.x.min(b1.x)
        || b0.x.max(b1.x) < a0.x.min(a1.x)
        || a0.y.max(a1.y) < b0.y.min(b1.y)
        || b0.y.max(b1.y) < a0.y.min(a1.y)
    {
        return SegmentIntersection::None;
    }

    let o1 = cross(a0, a1, b0);
    let o2 = cross(a0, a1, b1);
    let o3 = cross(b0, b1, a0);
    let o4 = cross(b0, b1, a1);

    if o1 == 0 && o2 == 0 && o3 == 0 && o4 == 0 {
        let use_x = a0.x != a1.x || b0.x != b1.x;
        let (a_lo, a_hi, b_lo, b_hi) = if use_x {
            (
                a0.x.min(a1.x),
                a0.x.max(a1.x),
                b0.x.min(b1.x),
                b0.x.max(b1.x),
            )
        } else {
            (
                a0.y.min(a1.y),
                a0.y.max(a1.y),
                b0.y.min(b1.y),
                b0.y.max(b1.y),
            )
        };
        let lo = a_lo.max(b_lo);
        let hi = a_hi.min(b_hi);
        return if lo < hi {
            SegmentIntersection::Overlap
        } else if lo == hi {
            SegmentIntersection::Touch
        } else {
            SegmentIntersection::None
        };
    }

    if opposite_signs(o1, o2) && opposite_signs(o3, o4) {
        return SegmentIntersection::Proper;
    }
    if (o1 == 0 && on_segment(a0, a1, b0))
        || (o2 == 0 && on_segment(a0, a1, b1))
        || (o3 == 0 && on_segment(b0, b1, a0))
        || (o4 == 0 && on_segment(b0, b1, a1))
    {
        SegmentIntersection::Touch
    } else {
        SegmentIntersection::None
    }
}

#[inline]
fn opposite_signs(a: i128, b: i128) -> bool {
    (a < 0 && b > 0) || (a > 0 && b < 0)
}

/// Exact signed twice-area. Positive is counter-clockwise.
pub fn ring_signed_area2(points: &[Point]) -> Result<i128, ExactGeometryError> {
    if points.len() < 3 {
        return Err(ExactGeometryError::TooFewVertices {
            count: points.len(),
        });
    }
    let mut area = 0_i128;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        let term = (a.x as i128)
            .checked_mul(b.y as i128)
            .and_then(|v| v.checked_sub((b.x as i128) * (a.y as i128)))
            .ok_or(ExactGeometryError::ArithmeticOverflow)?;
        area = area
            .checked_add(term)
            .ok_or(ExactGeometryError::ArithmeticOverflow)?;
    }
    Ok(area)
}

/// Classify an integer point against a simple ring.
pub fn classify_point_in_ring(points: &[Point], p: Point) -> PointClassification {
    classify_point_scaled2(points, p.x as i128 * 2, p.y as i128 * 2)
}

/// A validated simple ring, canonicalized to start at its lexicographically
/// smallest vertex. Its winding is retained until it is placed in a [`Polygon`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ring {
    vertices: Vec<Point>,
    area2: i128,
}

impl Ring {
    pub fn new(mut vertices: Vec<Point>) -> Result<Self, ExactGeometryError> {
        while vertices.len() > 1 && vertices.first() == vertices.last() {
            vertices.pop();
        }
        if vertices.len() < 3 {
            return Err(ExactGeometryError::TooFewVertices {
                count: vertices.len(),
            });
        }
        for i in 0..vertices.len() {
            if vertices[i] == vertices[(i + 1) % vertices.len()] {
                return Err(ExactGeometryError::DuplicateConsecutiveVertex { index: i });
            }
        }

        remove_redundant_collinear_vertices(&mut vertices);
        if vertices.len() < 3 {
            return Err(ExactGeometryError::TooFewVertices {
                count: vertices.len(),
            });
        }
        validate_simple(&vertices)?;
        let area2 = ring_signed_area2(&vertices)?;
        if area2 == 0 {
            return Err(ExactGeometryError::DegenerateRing);
        }
        rotate_to_minimum(&mut vertices);
        Ok(Self { vertices, area2 })
    }

    pub fn vertices(&self) -> &[Point] {
        &self.vertices
    }

    pub fn signed_area2(&self) -> i128 {
        self.area2
    }

    pub fn winding(&self) -> Winding {
        if self.area2 > 0 {
            Winding::CounterClockwise
        } else {
            Winding::Clockwise
        }
    }

    pub fn is_rectilinear(&self) -> bool {
        self.vertices.iter().enumerate().all(|(i, &a)| {
            let b = self.vertices[(i + 1) % self.vertices.len()];
            a.x == b.x || a.y == b.y
        })
    }

    pub fn classify_point(&self, p: Point) -> PointClassification {
        classify_point_in_ring(&self.vertices, p)
    }

    fn classify_scaled2(&self, px2: i128, py2: i128) -> PointClassification {
        classify_point_scaled2(&self.vertices, px2, py2)
    }

    fn orient_as(mut self, winding: Winding) -> Self {
        if self.winding() != winding {
            self.vertices.reverse();
            self.area2 = -self.area2;
            rotate_to_minimum(&mut self.vertices);
        }
        self
    }
}

/// A canonical polygon: CCW outer boundary and zero or more CW holes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Polygon {
    outer: Ring,
    holes: Vec<Ring>,
}

impl Polygon {
    pub fn new(outer: Ring, holes: Vec<Ring>) -> Result<Self, ExactGeometryError> {
        let outer = outer.orient_as(Winding::CounterClockwise);
        let mut canonical_holes = Vec::with_capacity(holes.len());
        for (index, hole) in holes.into_iter().enumerate() {
            let hole = hole.orient_as(Winding::Clockwise);
            match classify_ring_boundary_contact(&outer, &hole) {
                PolygonContact::Touch | PolygonContact::AreaOverlap => {
                    return Err(ExactGeometryError::HoleTouchesOuter { hole: index });
                }
                PolygonContact::Disjoint => {}
            }
            if outer.classify_point(hole.vertices[0]) != PointClassification::Inside {
                return Err(ExactGeometryError::HoleOutside { hole: index });
            }
            canonical_holes.push(hole);
        }

        canonical_holes.sort_by(|a, b| a.vertices.cmp(&b.vertices));
        for i in 0..canonical_holes.len() {
            for j in i + 1..canonical_holes.len() {
                let contact = classify_polygon_contact(&canonical_holes[i], &canonical_holes[j]);
                if contact != PolygonContact::Disjoint {
                    return Err(ExactGeometryError::HoleConflict {
                        hole_a: i,
                        hole_b: j,
                        contact,
                    });
                }
            }
        }
        Ok(Self {
            outer,
            holes: canonical_holes,
        })
    }

    pub fn from_outer(vertices: Vec<Point>) -> Result<Self, ExactGeometryError> {
        Self::new(Ring::new(vertices)?, Vec::new())
    }

    /// Construct one polygon from a GDS-style boundary walk.
    ///
    /// GDS holes may be encoded by walking a slit in both directions, producing
    /// one non-simple boundary record.  Every exact reverse-edge pair is cut at
    /// once, yielding one simple material boundary and zero or more simple hole
    /// rings.  The material boundary is selected by strict geometric
    /// containment, not by input winding, so reversing the complete record does
    /// not change its meaning.
    pub fn from_boundary_walk(mut vertices: Vec<Point>) -> Result<Self, ExactGeometryError> {
        while vertices.len() > 1 && vertices.first() == vertices.last() {
            vertices.pop();
        }
        if vertices.len() < 3 {
            return Err(ExactGeometryError::TooFewVertices {
                count: vertices.len(),
            });
        }
        for index in 0..vertices.len() {
            if vertices[index] == vertices[(index + 1) % vertices.len()] {
                return Err(ExactGeometryError::DuplicateConsecutiveVertex { index });
            }
        }

        let mut rings = Vec::new();
        split_boundary_walk(vertices, &mut rings)?;
        if rings.len() == 1 {
            let ring = rings.pop().ok_or(ExactGeometryError::InternalTopology(
                "boundary decomposition lost its only ring",
            ))?;
            return Self::new(ring, Vec::new());
        }

        let mut result = None;
        for outer_index in 0..rings.len() {
            let outer = rings[outer_index].clone();
            let holes: Vec<_> = rings
                .iter()
                .enumerate()
                .filter(|(index, _)| *index != outer_index)
                .map(|(_, ring)| ring.clone())
                .collect();
            // Reversing the complete walk reverses every ring and must not
            // change the result, while a same-polarity nested lobe is still not
            // a hole. Preserve relative polarity, not absolute orientation.
            if holes.iter().any(|hole| hole.winding() == outer.winding()) {
                continue;
            }
            if let Ok(polygon) = Self::new(outer, holes) {
                if result.is_some() {
                    return Err(ExactGeometryError::InvalidBoundaryWalk(
                        "more than one ring can be the material boundary",
                    ));
                }
                result = Some(polygon);
            }
        }
        result.ok_or(ExactGeometryError::InvalidBoundaryWalk(
            "slit-separated rings do not form one outer boundary with disjoint contained holes",
        ))
    }

    pub fn outer(&self) -> &Ring {
        &self.outer
    }

    pub fn holes(&self) -> &[Ring] {
        &self.holes
    }

    pub fn area2(&self) -> i128 {
        self.outer.signed_area2() + self.holes.iter().map(Ring::signed_area2).sum::<i128>()
    }

    pub fn is_rectilinear(&self) -> bool {
        self.outer.is_rectilinear() && self.holes.iter().all(Ring::is_rectilinear)
    }

    pub fn classify_point(&self, p: Point) -> PointClassification {
        self.classify_scaled2(p.x as i128 * 2, p.y as i128 * 2)
    }

    fn classify_scaled2(&self, px2: i128, py2: i128) -> PointClassification {
        match self.outer.classify_scaled2(px2, py2) {
            PointClassification::Outside => PointClassification::Outside,
            PointClassification::Boundary => PointClassification::Boundary,
            PointClassification::Inside => {
                for hole in &self.holes {
                    match hole.classify_scaled2(px2, py2) {
                        PointClassification::Inside => return PointClassification::Outside,
                        PointClassification::Boundary => return PointClassification::Boundary,
                        PointClassification::Outside => {}
                    }
                }
                PointClassification::Inside
            }
        }
    }
}

fn split_boundary_walk(
    vertices: Vec<Point>,
    rings: &mut Vec<Ring>,
) -> Result<(), ExactGeometryError> {
    let mut pending = vec![vertices];
    while let Some(mut walk) = pending.pop() {
        while walk.len() > 1 && walk.first() == walk.last() {
            walk.pop();
        }
        let count = walk.len();
        let mut split = None;
        'pairs: for first in 0..count {
            let first_end = (first + 1) % count;
            for second in first + 1..count {
                let second_end = (second + 1) % count;
                if walk[first] != walk[second_end] || walk[first_end] != walk[second] {
                    continue;
                }
                let adjacent = second == first + 1 || (first == 0 && second == count - 1);
                if adjacent {
                    return Err(ExactGeometryError::InvalidBoundaryWalk(
                        "an immediately retraced edge is a spike, not a keyhole slit",
                    ));
                }
                split = Some((first, second));
                break 'pairs;
            }
        }

        if let Some((first, second)) = split {
            let between = walk[first + 1..=second].to_vec();
            let mut outside = walk[second + 1..].to_vec();
            outside.extend_from_slice(&walk[..=first]);
            pending.push(outside);
            pending.push(between);
        } else {
            rings.push(Ring::new(walk)?);
        }
    }
    Ok(())
}

/// A deterministic collection of polygon components. Components are sorted by
/// canonical outer/hole vertex order. Set membership is their union.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PolygonSet {
    polygons: Vec<Polygon>,
}

impl PolygonSet {
    pub fn new(mut polygons: Vec<Polygon>) -> Result<Self, ExactGeometryError> {
        polygons.sort_by(|a, b| {
            a.outer
                .vertices
                .cmp(&b.outer.vertices)
                .then_with(|| compare_holes(&a.holes, &b.holes))
        });
        Ok(Self { polygons })
    }

    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_polygon(polygon: Polygon) -> Self {
        Self {
            polygons: vec![polygon],
        }
    }

    pub fn polygons(&self) -> &[Polygon] {
        &self.polygons
    }

    pub fn into_polygons(self) -> Vec<Polygon> {
        self.polygons
    }

    pub fn component_count(&self) -> usize {
        self.polygons.len()
    }

    /// Sum of component signed twice-areas. Components supplied to [`Self::new`]
    /// may overlap; use a union boolean first when a union-area measurement is
    /// required. Boolean results are non-overlapping, so this is their set area.
    pub fn area2(&self) -> i128 {
        self.polygons.iter().map(Polygon::area2).sum()
    }

    pub fn is_rectilinear(&self) -> bool {
        self.polygons.iter().all(Polygon::is_rectilinear)
    }

    pub fn classify_point(&self, p: Point) -> PointClassification {
        self.classify_scaled2(p.x as i128 * 2, p.y as i128 * 2)
    }

    fn classify_scaled2(&self, px2: i128, py2: i128) -> PointClassification {
        let mut boundary = false;
        for polygon in &self.polygons {
            match polygon.classify_scaled2(px2, py2) {
                PointClassification::Inside => return PointClassification::Inside,
                PointClassification::Boundary => boundary = true,
                PointClassification::Outside => {}
            }
        }
        if boundary {
            PointClassification::Boundary
        } else {
            PointClassification::Outside
        }
    }
}

/// Classify two filled simple rings without using their bounding boxes as a decision.
pub fn classify_polygon_contact(a: &Ring, b: &Ring) -> PolygonContact {
    if !bbox_may_touch(a, b) {
        return PolygonContact::Disjoint;
    }

    let mut boundary_contact = false;
    for i in 0..a.vertices.len() {
        let a0 = a.vertices[i];
        let a1 = a.vertices[(i + 1) % a.vertices.len()];
        for j in 0..b.vertices.len() {
            let b0 = b.vertices[j];
            let b1 = b.vertices[(j + 1) % b.vertices.len()];
            match classify_segment_intersection(a0, a1, b0, b1) {
                SegmentIntersection::Proper => return PolygonContact::AreaOverlap,
                SegmentIntersection::Touch | SegmentIntersection::Overlap => {
                    boundary_contact = true;
                }
                SegmentIntersection::None => {}
            }
        }
    }

    // Vertices alone are insufficient for aligned containment (for example a
    // half-width rectangle whose four corners lie on the containing boundary).
    // Edge midpoints are exact rationals represented in doubled coordinates.
    if ring_has_strict_sample_inside(a, b) || ring_has_strict_sample_inside(b, a) {
        return PolygonContact::AreaOverlap;
    }
    if canonical_cycle_equal(a, b) {
        return PolygonContact::AreaOverlap;
    }
    if boundary_contact {
        PolygonContact::Touch
    } else {
        PolygonContact::Disjoint
    }
}

/// Exact rectilinear boolean. See the module-level supported-semantics contract.
pub fn rectilinear_boolean(
    lhs: &PolygonSet,
    rhs: &PolygonSet,
    operation: BooleanOp,
) -> Result<PolygonSet, ExactGeometryError> {
    ensure_rectilinear(lhs, operation)?;
    ensure_rectilinear(rhs, operation)?;

    if lhs.polygons.is_empty() || rhs.polygons.is_empty() {
        return match operation {
            BooleanOp::Union if lhs.polygons.is_empty() => Ok(rhs.clone()),
            BooleanOp::Union => Ok(lhs.clone()),
            BooleanOp::Intersection => Ok(PolygonSet::empty()),
            BooleanOp::Subtraction if lhs.polygons.is_empty() => Ok(PolygonSet::empty()),
            BooleanOp::Subtraction => Ok(lhs.clone()),
        };
    }

    let mut xs = Vec::new();
    let mut ys = Vec::new();
    collect_coordinates(lhs, &mut xs, &mut ys);
    collect_coordinates(rhs, &mut xs, &mut ys);
    xs.sort_unstable();
    ys.sort_unstable();
    xs.dedup();
    ys.dedup();
    if xs.len() < 2 || ys.len() < 2 {
        return Ok(PolygonSet::empty());
    }

    let nx = xs.len() - 1;
    let ny = ys.len() - 1;
    let cells = nx
        .checked_mul(ny)
        .ok_or(ExactGeometryError::CapacityExceeded {
            cells: usize::MAX,
            limit: MAX_RECTILINEAR_BOOLEAN_CELLS,
        })?;
    if cells > MAX_RECTILINEAR_BOOLEAN_CELLS {
        return Err(ExactGeometryError::CapacityExceeded {
            cells,
            limit: MAX_RECTILINEAR_BOOLEAN_CELLS,
        });
    }
    let mut occupied = vec![false; cells];
    for y in 0..ny {
        let py2 = ys[y] as i128 + ys[y + 1] as i128;
        for x in 0..nx {
            let px2 = xs[x] as i128 + xs[x + 1] as i128;
            let in_lhs = lhs.classify_scaled2(px2, py2) == PointClassification::Inside;
            let in_rhs = rhs.classify_scaled2(px2, py2) == PointClassification::Inside;
            occupied[y * nx + x] = match operation {
                BooleanOp::Union => in_lhs || in_rhs,
                BooleanOp::Intersection => in_lhs && in_rhs,
                BooleanOp::Subtraction => in_lhs && !in_rhs,
            };
        }
    }
    reconstruct_rectilinear_set(&xs, &ys, nx, ny, &occupied)
}

pub fn rectilinear_union(
    lhs: &PolygonSet,
    rhs: &PolygonSet,
) -> Result<PolygonSet, ExactGeometryError> {
    rectilinear_boolean(lhs, rhs, BooleanOp::Union)
}

pub fn rectilinear_intersection(
    lhs: &PolygonSet,
    rhs: &PolygonSet,
) -> Result<PolygonSet, ExactGeometryError> {
    rectilinear_boolean(lhs, rhs, BooleanOp::Intersection)
}

pub fn rectilinear_subtraction(
    lhs: &PolygonSet,
    rhs: &PolygonSet,
) -> Result<PolygonSet, ExactGeometryError> {
    rectilinear_boolean(lhs, rhs, BooleanOp::Subtraction)
}

/// Explicit capacity guard for the current coordinate-decomposition implementation.
pub const MAX_RECTILINEAR_BOOLEAN_CELLS: usize = 16_000_000;

fn compare_holes(a: &[Ring], b: &[Ring]) -> std::cmp::Ordering {
    a.len().cmp(&b.len()).then_with(|| {
        a.iter()
            .map(|r| &r.vertices)
            .cmp(b.iter().map(|r| &r.vertices))
    })
}

fn validate_simple(vertices: &[Point]) -> Result<(), ExactGeometryError> {
    let n = vertices.len();
    for i in 0..n {
        let a0 = vertices[i];
        let a1 = vertices[(i + 1) % n];
        for j in i + 1..n {
            let adjacent = j == i + 1 || (i == 0 && j == n - 1);
            let b0 = vertices[j];
            let b1 = vertices[(j + 1) % n];
            let kind = classify_segment_intersection(a0, a1, b0, b1);
            if adjacent {
                if kind != SegmentIntersection::Touch {
                    return Err(ExactGeometryError::SelfIntersection {
                        edge_a: i,
                        edge_b: j,
                        kind,
                    });
                }
            } else if kind != SegmentIntersection::None {
                return Err(ExactGeometryError::SelfIntersection {
                    edge_a: i,
                    edge_b: j,
                    kind,
                });
            }
        }
    }
    Ok(())
}

fn remove_redundant_collinear_vertices(vertices: &mut Vec<Point>) {
    loop {
        if vertices.len() <= 3 {
            return;
        }
        let n = vertices.len();
        let remove = (0..n).find(|&i| {
            let a = vertices[(i + n - 1) % n];
            let b = vertices[i];
            let c = vertices[(i + 1) % n];
            cross(a, b, c) == 0 && on_segment(a, c, b)
        });
        if let Some(i) = remove {
            vertices.remove(i);
        } else {
            return;
        }
    }
}

fn rotate_to_minimum(vertices: &mut Vec<Point>) {
    if let Some((index, _)) = vertices.iter().enumerate().min_by_key(|(_, p)| **p) {
        vertices.rotate_left(index);
    }
}

fn classify_point_scaled2(points: &[Point], px2: i128, py2: i128) -> PointClassification {
    let mut inside = false;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        let ax2 = a.x as i128 * 2;
        let ay2 = a.y as i128 * 2;
        let bx2 = b.x as i128 * 2;
        let by2 = b.y as i128 * 2;
        let det = (bx2 - ax2) * (py2 - ay2) - (by2 - ay2) * (px2 - ax2);
        if det == 0
            && px2 >= ax2.min(bx2)
            && px2 <= ax2.max(bx2)
            && py2 >= ay2.min(by2)
            && py2 <= ay2.max(by2)
        {
            return PointClassification::Boundary;
        }
        if (ay2 > py2) != (by2 > py2) && ((by2 > ay2 && det > 0) || (by2 < ay2 && det < 0)) {
            inside = !inside;
        }
    }
    if inside {
        PointClassification::Inside
    } else {
        PointClassification::Outside
    }
}

fn classify_ring_boundary_contact(a: &Ring, b: &Ring) -> PolygonContact {
    let mut touch = false;
    for i in 0..a.vertices.len() {
        let a0 = a.vertices[i];
        let a1 = a.vertices[(i + 1) % a.vertices.len()];
        for j in 0..b.vertices.len() {
            let b0 = b.vertices[j];
            let b1 = b.vertices[(j + 1) % b.vertices.len()];
            match classify_segment_intersection(a0, a1, b0, b1) {
                SegmentIntersection::Proper => return PolygonContact::AreaOverlap,
                SegmentIntersection::Touch | SegmentIntersection::Overlap => touch = true,
                SegmentIntersection::None => {}
            }
        }
    }
    if touch {
        PolygonContact::Touch
    } else {
        PolygonContact::Disjoint
    }
}

fn ring_has_strict_sample_inside(subject: &Ring, container: &Ring) -> bool {
    for i in 0..subject.vertices.len() {
        let a = subject.vertices[i];
        if container.classify_point(a) == PointClassification::Inside {
            return true;
        }
        let b = subject.vertices[(i + 1) % subject.vertices.len()];
        if container.classify_scaled2(a.x as i128 + b.x as i128, a.y as i128 + b.y as i128)
            == PointClassification::Inside
        {
            return true;
        }
    }
    false
}

fn canonical_cycle_equal(a: &Ring, b: &Ring) -> bool {
    if a.vertices.len() != b.vertices.len() {
        return false;
    }
    if a.vertices == b.vertices {
        return true;
    }
    let mut reversed = b.vertices.clone();
    reversed.reverse();
    rotate_to_minimum(&mut reversed);
    a.vertices == reversed
}

fn bbox_may_touch(a: &Ring, b: &Ring) -> bool {
    let bbox = |ring: &Ring| {
        let mut xmin = i32::MAX;
        let mut ymin = i32::MAX;
        let mut xmax = i32::MIN;
        let mut ymax = i32::MIN;
        for p in &ring.vertices {
            xmin = xmin.min(p.x);
            ymin = ymin.min(p.y);
            xmax = xmax.max(p.x);
            ymax = ymax.max(p.y);
        }
        (xmin, ymin, xmax, ymax)
    };
    let aa = bbox(a);
    let bb = bbox(b);
    aa.0 <= bb.2 && bb.0 <= aa.2 && aa.1 <= bb.3 && bb.1 <= aa.3
}

fn ensure_rectilinear(set: &PolygonSet, operation: BooleanOp) -> Result<(), ExactGeometryError> {
    if set.is_rectilinear() {
        Ok(())
    } else {
        Err(ExactGeometryError::Unsupported {
            operation,
            reason: "arbitrary-angle boolean topology is not implemented",
        })
    }
}

fn collect_coordinates(set: &PolygonSet, xs: &mut Vec<i32>, ys: &mut Vec<i32>) {
    for polygon in &set.polygons {
        for ring in std::iter::once(&polygon.outer).chain(polygon.holes.iter()) {
            for point in &ring.vertices {
                xs.push(point.x);
                ys.push(point.y);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct DirectedEdge {
    start: Point,
    end: Point,
}

fn reconstruct_rectilinear_set(
    xs: &[i32],
    ys: &[i32],
    nx: usize,
    ny: usize,
    occupied: &[bool],
) -> Result<PolygonSet, ExactGeometryError> {
    let filled = |x: isize, y: isize| -> bool {
        x >= 0
            && y >= 0
            && (x as usize) < nx
            && (y as usize) < ny
            && occupied[y as usize * nx + x as usize]
    };
    let mut edges = Vec::new();
    for y in 0..ny {
        for x in 0..nx {
            if !occupied[y * nx + x] {
                continue;
            }
            let (x0, x1, y0, y1) = (xs[x], xs[x + 1], ys[y], ys[y + 1]);
            if !filled(x as isize, y as isize - 1) {
                edges.push(DirectedEdge {
                    start: Point::new(x0, y0),
                    end: Point::new(x1, y0),
                });
            }
            if !filled(x as isize + 1, y as isize) {
                edges.push(DirectedEdge {
                    start: Point::new(x1, y0),
                    end: Point::new(x1, y1),
                });
            }
            if !filled(x as isize, y as isize + 1) {
                edges.push(DirectedEdge {
                    start: Point::new(x1, y1),
                    end: Point::new(x0, y1),
                });
            }
            if !filled(x as isize - 1, y as isize) {
                edges.push(DirectedEdge {
                    start: Point::new(x0, y1),
                    end: Point::new(x0, y0),
                });
            }
        }
    }
    if edges.is_empty() {
        return Ok(PolygonSet::empty());
    }
    edges.sort_unstable();
    let mut outgoing: BTreeMap<Point, Vec<usize>> = BTreeMap::new();
    for (index, edge) in edges.iter().enumerate() {
        outgoing.entry(edge.start).or_default().push(index);
    }
    for indices in outgoing.values_mut() {
        indices.sort_unstable_by_key(|&i| edges[i].end);
    }

    let mut used = vec![false; edges.len()];
    let mut rings = Vec::new();
    for start_edge in 0..edges.len() {
        if used[start_edge] {
            continue;
        }
        let start = edges[start_edge].start;
        let mut ring_points = Vec::new();
        let mut current = start_edge;
        loop {
            if used[current] {
                return Err(ExactGeometryError::InternalTopology(
                    "boundary edge was revisited",
                ));
            }
            used[current] = true;
            let edge = edges[current];
            ring_points.push(edge.start);
            if edge.end == start {
                break;
            }
            let candidates = outgoing
                .get(&edge.end)
                .ok_or(ExactGeometryError::InternalTopology("open boundary chain"))?;
            current = choose_continuation(edge, candidates, &edges, &used).ok_or(
                ExactGeometryError::InternalTopology("no unused boundary continuation"),
            )?;
            if ring_points.len() > edges.len() {
                return Err(ExactGeometryError::InternalTopology(
                    "boundary chain did not close",
                ));
            }
        }
        rings.push(Ring::new(ring_points)?);
    }

    let mut outers: Vec<(Ring, Vec<Ring>)> = rings
        .iter()
        .filter(|ring| ring.winding() == Winding::CounterClockwise)
        .cloned()
        .map(|outer| (outer, Vec::new()))
        .collect();
    let holes: Vec<Ring> = rings
        .into_iter()
        .filter(|ring| ring.winding() == Winding::Clockwise)
        .collect();
    for hole in holes {
        let witness = hole.vertices[0];
        let owner = outers
            .iter()
            .enumerate()
            .filter(|(_, (outer, _))| outer.classify_point(witness) == PointClassification::Inside)
            .min_by_key(|(_, (outer, _))| outer.signed_area2())
            .map(|(index, _)| index)
            .ok_or(ExactGeometryError::InternalTopology(
                "hole has no containing outer ring",
            ))?;
        outers[owner].1.push(hole);
    }
    let polygons = outers
        .into_iter()
        .map(|(outer, holes)| Polygon::new(outer, holes))
        .collect::<Result<Vec<_>, _>>()?;
    PolygonSet::new(polygons)
}

fn choose_continuation(
    incoming: DirectedEdge,
    candidates: &[usize],
    edges: &[DirectedEdge],
    used: &[bool],
) -> Option<usize> {
    let incoming_dir = direction(incoming.start, incoming.end)?;
    candidates
        .iter()
        .copied()
        .filter(|&index| !used[index])
        .min_by_key(|&index| {
            let next_dir = direction(edges[index].start, edges[index].end).unwrap_or(0);
            let delta = (next_dir + 4 - incoming_dir) % 4;
            let turn_preference = match delta {
                1 => 0, // left: separates corner-touching components
                0 => 1, // straight
                3 => 2, // right
                _ => 3, // reversal (invalid topology, but deterministic)
            };
            (turn_preference, edges[index].end)
        })
}

/// E=0, N=1, W=2, S=3.
fn direction(a: Point, b: Point) -> Option<u8> {
    if a.y == b.y && b.x > a.x {
        Some(0)
    } else if a.x == b.x && b.y > a.y {
        Some(1)
    } else if a.y == b.y && b.x < a.x {
        Some(2)
    } else if a.x == b.x && b.y < a.y {
        Some(3)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: i32, y: i32) -> Point {
        Point::new(x, y)
    }

    fn rectangle(x0: i32, y0: i32, x1: i32, y1: i32) -> PolygonSet {
        PolygonSet::from_polygon(
            Polygon::from_outer(vec![p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)]).unwrap(),
        )
    }

    #[test]
    fn predicates_cover_extreme_coordinates_without_overflow() {
        let lo = i32::MIN;
        let hi = i32::MAX;
        assert_eq!(
            orientation(p(lo, lo), p(hi, lo), p(hi, hi)),
            Orientation::CounterClockwise
        );
        let ring = Ring::new(vec![p(lo, lo), p(hi, lo), p(hi, hi), p(lo, hi)]).unwrap();
        let span = hi as i128 - lo as i128;
        assert_eq!(ring.signed_area2(), 2 * span * span);
        assert_eq!(ring.classify_point(p(0, 0)), PointClassification::Inside);
    }

    #[test]
    fn boundary_walk_decomposition_is_orientation_invariant_and_multi_hole() {
        let points = vec![
            p(0, 0),
            p(100, 0),
            p(100, 100),
            p(70, 100),
            p(70, 80),
            p(90, 80),
            p(90, 60),
            p(60, 60),
            p(60, 80),
            p(70, 80),
            p(70, 100),
            p(40, 100),
            p(40, 80),
            p(50, 80),
            p(50, 60),
            p(20, 60),
            p(20, 80),
            p(40, 80),
            p(40, 100),
            p(0, 100),
        ];
        let polygon = Polygon::from_boundary_walk(points.clone()).unwrap();
        assert_eq!(polygon.holes().len(), 2);
        assert_eq!(polygon.area2(), 17_600);

        let reversed = Polygon::from_boundary_walk(points.into_iter().rev().collect()).unwrap();
        assert_eq!(reversed, polygon);
    }

    #[test]
    fn boundary_walk_rejects_external_and_disjoint_retraced_lobes() {
        let external = vec![
            p(0, 0),
            p(-5, 0),
            p(-5, -5),
            p(-10, -5),
            p(-10, 0),
            p(-5, 0),
            p(0, 0),
            p(10, 0),
            p(10, 10),
            p(0, 10),
        ];
        assert!(matches!(
            Polygon::from_boundary_walk(external),
            Err(ExactGeometryError::InvalidBoundaryWalk(_))
        ));

        let disjoint = vec![
            p(10, 5),
            p(20, 5),
            p(20, 10),
            p(30, 10),
            p(30, 0),
            p(20, 0),
            p(20, 5),
            p(10, 5),
            p(10, 10),
            p(0, 10),
            p(0, 0),
            p(10, 0),
        ];
        assert!(matches!(
            Polygon::from_boundary_walk(disjoint),
            Err(ExactGeometryError::InvalidBoundaryWalk(_))
        ));

        let same_polarity = vec![
            p(0, 0),
            p(10, 0),
            p(10, 10),
            p(5, 10),
            p(5, 8),
            p(3, 8),
            p(3, 3),
            p(8, 3),
            p(8, 8),
            p(5, 8),
            p(5, 10),
            p(0, 10),
        ];
        assert!(matches!(
            Polygon::from_boundary_walk(same_polarity),
            Err(ExactGeometryError::InvalidBoundaryWalk(_))
        ));
    }

    #[test]
    fn segment_classification_distinguishes_cross_touch_and_overlap() {
        assert_eq!(
            classify_segment_intersection(p(0, 0), p(4, 4), p(0, 4), p(4, 0)),
            SegmentIntersection::Proper
        );
        assert_eq!(
            classify_segment_intersection(p(0, 0), p(4, 0), p(4, 0), p(4, 4)),
            SegmentIntersection::Touch
        );
        assert_eq!(
            classify_segment_intersection(p(0, 0), p(4, 0), p(2, 0), p(6, 0)),
            SegmentIntersection::Overlap
        );
        assert_eq!(
            classify_segment_intersection(p(0, 0), p(1, 0), p(2, 0), p(3, 0)),
            SegmentIntersection::None
        );
    }

    #[test]
    fn ring_validation_rejects_degeneracy_and_self_contact() {
        assert!(matches!(
            Ring::new(vec![p(0, 0), p(1, 0), p(2, 0)]),
            Err(ExactGeometryError::TooFewVertices { .. })
                | Err(ExactGeometryError::DegenerateRing)
                | Err(ExactGeometryError::SelfIntersection { .. })
        ));
        assert!(matches!(
            Ring::new(vec![p(0, 0), p(4, 4), p(0, 4), p(4, 0)]),
            Err(ExactGeometryError::SelfIntersection {
                kind: SegmentIntersection::Proper,
                ..
            })
        ));
        assert!(matches!(
            Ring::new(vec![p(0, 0), p(4, 0), p(2, 0), p(2, 2), p(0, 2)]),
            Err(ExactGeometryError::SelfIntersection { .. })
        ));
    }

    #[test]
    fn polygon_canonicalizes_reversed_winding_and_validates_holes() {
        let outer = Ring::new(vec![p(0, 0), p(0, 10), p(10, 10), p(10, 0)]).unwrap();
        let hole = Ring::new(vec![p(2, 2), p(8, 2), p(8, 8), p(2, 8)]).unwrap();
        let polygon = Polygon::new(outer, vec![hole]).unwrap();
        assert_eq!(polygon.outer().winding(), Winding::CounterClockwise);
        assert_eq!(polygon.holes()[0].winding(), Winding::Clockwise);
        assert_eq!(polygon.area2(), 128);
        assert_eq!(polygon.classify_point(p(1, 1)), PointClassification::Inside);
        assert_eq!(
            polygon.classify_point(p(5, 5)),
            PointClassification::Outside
        );

        let touching = Ring::new(vec![p(0, 2), p(2, 2), p(2, 4), p(0, 4)]).unwrap();
        assert!(matches!(
            Polygon::new(polygon.outer().clone(), vec![touching]),
            Err(ExactGeometryError::HoleTouchesOuter { .. })
        ));
    }

    #[test]
    fn polygon_contact_rejects_the_concave_bbox_trap() {
        let concave = Ring::new(vec![
            p(0, 0),
            p(10, 0),
            p(10, 2),
            p(2, 2),
            p(2, 10),
            p(0, 10),
        ])
        .unwrap();
        let in_bbox_but_disjoint = Ring::new(vec![p(4, 4), p(9, 4), p(9, 9), p(4, 9)]).unwrap();
        assert_eq!(
            classify_polygon_contact(&concave, &in_bbox_but_disjoint),
            PolygonContact::Disjoint
        );
        let edge_touch = Ring::new(vec![p(2, 4), p(4, 4), p(4, 6), p(2, 6)]).unwrap();
        assert_eq!(
            classify_polygon_contact(&concave, &edge_touch),
            PolygonContact::Touch
        );
    }

    #[test]
    fn rectilinear_booleans_preserve_holes_keyholes_and_components() {
        let outer = rectangle(0, 0, 10, 10);
        let inner = rectangle(2, 2, 8, 8);
        let donut = rectilinear_subtraction(&outer, &inner).unwrap();
        assert_eq!(donut.component_count(), 1);
        assert_eq!(donut.polygons()[0].holes().len(), 1);
        assert_eq!(donut.area2(), 128);

        let boundary_cut = rectangle(0, 3, 6, 7);
        let keyhole = rectilinear_subtraction(&outer, &boundary_cut).unwrap();
        assert_eq!(keyhole.component_count(), 1);
        assert!(keyhole.polygons()[0].holes().is_empty());
        assert_eq!(
            keyhole.classify_point(p(1, 5)),
            PointClassification::Outside
        );
        assert_eq!(keyhole.classify_point(p(8, 5)), PointClassification::Inside);

        let disjoint = rectilinear_union(&rectangle(0, 0, 2, 2), &rectangle(5, 0, 7, 2)).unwrap();
        assert_eq!(disjoint.component_count(), 2);
        assert_eq!(disjoint.area2(), 16);

        let splitter = rectangle(4, -1, 6, 11);
        let split = rectilinear_subtraction(&outer, &splitter).unwrap();
        assert_eq!(split.component_count(), 2);
        assert_eq!(split.area2(), 160);
    }

    #[test]
    fn corner_touch_stays_disconnected_and_edge_touch_merges() {
        let a = rectangle(0, 0, 2, 2);
        let corner = rectilinear_union(&a, &rectangle(2, 2, 4, 4)).unwrap();
        assert_eq!(corner.component_count(), 2);
        let edge = rectilinear_union(&a, &rectangle(2, 0, 4, 2)).unwrap();
        assert_eq!(edge.component_count(), 1);
        assert_eq!(edge.polygons()[0].outer().vertices().len(), 4);
    }

    #[test]
    fn arbitrary_angle_boolean_fails_closed() {
        let triangle =
            PolygonSet::from_polygon(Polygon::from_outer(vec![p(0, 0), p(4, 0), p(2, 3)]).unwrap());
        let err = rectilinear_union(&triangle, &rectangle(0, 0, 1, 1)).unwrap_err();
        assert_eq!(
            err,
            ExactGeometryError::Unsupported {
                operation: BooleanOp::Union,
                reason: "arbitrary-angle boolean topology is not implemented",
            }
        );
    }

    #[derive(Clone, Copy)]
    struct LShape {
        ox: i32,
        oy: i32,
        width: i32,
        height: i32,
        cut_x: i32,
        cut_y: i32,
    }

    impl LShape {
        fn polygon(self) -> PolygonSet {
            let x = self.ox;
            let y = self.oy;
            let w = self.width;
            let h = self.height;
            let cx = self.cut_x;
            let cy = self.cut_y;
            PolygonSet::from_polygon(
                Polygon::from_outer(vec![
                    p(x, y),
                    p(x + w, y),
                    p(x + w, y + cy),
                    p(x + cx, y + cy),
                    p(x + cx, y + h),
                    p(x, y + h),
                ])
                .unwrap(),
            )
        }

        /// Independent unit-cell oracle: bounding rectangle minus the top-right cutout.
        fn covers_cell(self, x: i32, y: i32) -> bool {
            let local_x = x - self.ox;
            let local_y = y - self.oy;
            local_x >= 0
                && local_y >= 0
                && local_x < self.width
                && local_y < self.height
                && !(local_x >= self.cut_x && local_y >= self.cut_y)
        }
    }

    fn next(seed: &mut u64) -> u32 {
        *seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (*seed >> 32) as u32
    }

    #[test]
    fn randomized_rectilinear_differential_against_independent_cell_oracle() {
        let mut seed = 0x5eed_cafe_d15c_a11u64;
        for case in 0..250 {
            let make = |seed: &mut u64| {
                let width = 2 + (next(seed) % 5) as i32;
                let height = 2 + (next(seed) % 5) as i32;
                LShape {
                    ox: (next(seed) % 7) as i32 - 3,
                    oy: (next(seed) % 7) as i32 - 3,
                    width,
                    height,
                    cut_x: 1 + (next(seed) % (width as u32 - 1)) as i32,
                    cut_y: 1 + (next(seed) % (height as u32 - 1)) as i32,
                }
            };
            let a = make(&mut seed);
            let b = make(&mut seed);
            for operation in [
                BooleanOp::Union,
                BooleanOp::Intersection,
                BooleanOp::Subtraction,
            ] {
                let actual = rectilinear_boolean(&a.polygon(), &b.polygon(), operation)
                    .unwrap_or_else(|e| panic!("case {case} {operation:?}: {e}"));
                for y in -4..9 {
                    for x in -4..9 {
                        let expected = match operation {
                            BooleanOp::Union => a.covers_cell(x, y) || b.covers_cell(x, y),
                            BooleanOp::Intersection => a.covers_cell(x, y) && b.covers_cell(x, y),
                            BooleanOp::Subtraction => a.covers_cell(x, y) && !b.covers_cell(x, y),
                        };
                        let got = actual.classify_scaled2(x as i128 * 2 + 1, y as i128 * 2 + 1)
                            == PointClassification::Inside;
                        assert_eq!(got, expected, "case {case} {operation:?} cell ({x},{y})");
                    }
                }
            }
        }
    }
}
