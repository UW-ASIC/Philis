//! Canonical polygon: CCW outer boundary and zero or more CW holes.

use super::contact::{classify_polygon_contact, classify_ring_boundary_contact, PolygonContact};
use super::error::ExactGeometryError;
use super::point::Point;
use super::predicates::PointClassification;
use super::ring::{Ring, Winding};

/// Current explicit capacity for one boundary record. The work limit below is
/// normally tighter, while this separate bound prevents an oversized record
/// from consuming unbounded memory before pair scanning begins.
pub const MAX_BOUNDARY_WALK_VERTICES: usize = 65_536;
pub const MAX_BOUNDARY_WALK_WORK: usize = 16_000_000;

/// A canonical polygon: CCW outer boundary and zero or more CW holes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Polygon {
    pub(super) outer: Ring,
    pub(super) holes: Vec<Ring>,
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
            if outer.classify_point(hole.vertices()[0]) != PointClassification::Inside {
                return Err(ExactGeometryError::HoleOutside { hole: index });
            }
            canonical_holes.push(hole);
        }

        canonical_holes.sort_by(|a, b| a.vertices().cmp(b.vertices()));
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
        if vertices.len() > MAX_BOUNDARY_WALK_VERTICES {
            return Err(ExactGeometryError::CapacityExceeded {
                cells: vertices.len(),
                limit: MAX_BOUNDARY_WALK_VERTICES,
            });
        }
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

        let vertex_count = vertices.len();
        let mut work = 0;
        charge_boundary_work(&mut work, vertex_count)?;
        let mut rings = Vec::new();
        split_boundary_walk(vertices, &mut rings, &mut work)?;
        if rings.len() == 1 {
            let ring = rings.pop().ok_or(ExactGeometryError::InternalTopology(
                "boundary decomposition lost its only ring",
            ))?;
            return Self::new(ring, Vec::new());
        }

        // Strict containment implies that the material boundary has greater
        // absolute area than every hole. Select that necessary candidate, then
        // let Polygon::new prove containment/contact exactly once.
        let outer_index = rings
            .iter()
            .enumerate()
            .max_by_key(|(_, ring)| ring.signed_area2().unsigned_abs())
            .map(|(index, _)| index)
            .ok_or(ExactGeometryError::InternalTopology(
                "boundary decomposition produced no rings",
            ))?;
        let outer = rings.swap_remove(outer_index);
        let holes = rings;
        // Reversing the complete walk reverses every ring and must not change
        // the result, while a same-polarity nested lobe is still not a hole.
        if holes.iter().any(|hole| hole.winding() == outer.winding()) {
            return Err(ExactGeometryError::InvalidBoundaryWalk(
                "contained rings have the same relative polarity as the material boundary",
            ));
        }
        charge_boundary_work(
            &mut work,
            vertex_count.checked_mul(vertex_count).unwrap_or(usize::MAX),
        )?;
        Self::new(outer, holes).map_err(|error| match error {
            ExactGeometryError::CapacityExceeded { .. }
            | ExactGeometryError::ArithmeticOverflow => error,
            _ => ExactGeometryError::InvalidBoundaryWalk(
                "slit-separated rings do not form one outer boundary with disjoint contained holes",
            ),
        })
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

    pub(super) fn classify_scaled2(&self, px2: i128, py2: i128) -> PointClassification {
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
    work: &mut usize,
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
                charge_boundary_work(work, 1)?;
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
            charge_boundary_work(work, count)?;
            let between = walk[first + 1..=second].to_vec();
            let mut outside = walk[second + 1..].to_vec();
            outside.extend_from_slice(&walk[..=first]);
            pending.push(outside);
            pending.push(between);
        } else {
            charge_boundary_work(work, count.checked_mul(count).unwrap_or(usize::MAX))?;
            rings.push(Ring::new(walk)?);
        }
    }
    Ok(())
}

fn charge_boundary_work(work: &mut usize, amount: usize) -> Result<(), ExactGeometryError> {
    let requested = work.checked_add(amount).unwrap_or(usize::MAX);
    if requested > MAX_BOUNDARY_WALK_WORK {
        return Err(ExactGeometryError::CapacityExceeded {
            cells: requested,
            limit: MAX_BOUNDARY_WALK_WORK,
        });
    }
    *work = requested;
    Ok(())
}
