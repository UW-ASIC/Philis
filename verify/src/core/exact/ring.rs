//! Validated simple ring: the atom every polygon is built from.

use super::error::ExactGeometryError;
use super::point::{cross, on_segment, Point};
use super::predicates::{
    classify_point_in_ring, classify_point_scaled2, classify_segment_intersection,
    PointClassification, SegmentIntersection,
};

/// Winding required by the canonical polygon-set representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Winding {
    Clockwise,
    CounterClockwise,
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

/// A validated simple ring, canonicalized to start at its lexicographically
/// smallest vertex. Its winding is retained until it is placed in a `Polygon`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ring {
    pub(super) vertices: Vec<Point>,
    pub(super) area2: i128,
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

    pub(super) fn classify_scaled2(&self, px2: i128, py2: i128) -> PointClassification {
        classify_point_scaled2(&self.vertices, px2, py2)
    }

    pub(super) fn orient_as(mut self, winding: Winding) -> Self {
        if self.winding() != winding {
            self.vertices.reverse();
            self.area2 = -self.area2;
            rotate_to_minimum(&mut self.vertices);
        }
        self
    }
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

pub(super) fn rotate_to_minimum(vertices: &mut Vec<Point>) {
    if let Some((index, _)) = vertices.iter().enumerate().min_by_key(|(_, p)| **p) {
        vertices.rotate_left(index);
    }
}
