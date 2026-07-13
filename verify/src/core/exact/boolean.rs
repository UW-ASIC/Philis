//! Exact rectilinear booleans by coordinate decomposition.

use std::collections::BTreeMap;

use super::error::ExactGeometryError;
use super::point::Point;
use super::polygon::Polygon;
use super::polygon_set::PolygonSet;
use super::predicates::PointClassification;
use super::ring::{Ring, Winding};

/// Boolean operation supported by [`rectilinear_boolean`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BooleanOp {
    Union,
    Intersection,
    Subtraction,
}

/// Explicit capacity guard for the current coordinate-decomposition implementation.
pub const MAX_RECTILINEAR_BOOLEAN_CELLS: usize = 16_000_000;

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
            for point in ring.vertices() {
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
        let witness = hole.vertices()[0];
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
