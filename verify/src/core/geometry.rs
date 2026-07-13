//! Data-oriented geometry core.
//!
//! Everything the checkers touch lives here in struct-of-arrays (SoA) form. There are no
//! per-polygon heap objects and no pointers between shapes — references are `u32` indices
//! into flat arrays. This is what makes the store (a) cache-friendly for the CPU scanline
//! passes and (b) trivially uploadable to a GPU as flat buffers (see `gpu` module).
//!
//! Coordinates are `i32` database units (nm in the conformance suite).
//!
//! Layout, per the DOD skill:
//!   * `verts_x` / `verts_y`  — parallel arrays of every vertex of every polygon (hot).
//!   * `poly_layer`           — layer id per polygon (warm; used to filter).
//!   * `poly_vert_start/len`  — index range into the vertex arrays (the "handle").
//! A polygon is therefore an index range, not an object. Iterating all met1 edges never
//! loads a byte of any other layer's coordinates it doesn't need.

/// Exact predicates, validated polygon sets, and fail-closed rectilinear booleans.
///
/// This is the migration target for checkers that currently use private geometry
/// approximations.  See the module documentation for the supported-semantics
/// contract; legacy helpers in this file remain available while callers migrate.
/// Lives at `core::exact`; re-exported here so `geometry::exact` paths keep working.
pub use crate::core::exact;

/// A layer identifier. Small integer, indexes the layer table. `u16` keeps references tiny.
pub type LayerId = u16;

/// Handle to a polygon: just an index into the SoA arrays. Copyable, pointer-free.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PolyId(pub u32);

/// An axis-aligned bounding box, kept alongside polygons for fast reject (hot/cold split:
/// bbox is hot for spatial queries, the full vertex list is colder).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bbox {
    pub xmin: i32,
    pub ymin: i32,
    pub xmax: i32,
    pub ymax: i32,
}

impl Bbox {
    #[inline]
    pub fn empty() -> Self {
        Bbox { xmin: i32::MAX, ymin: i32::MAX, xmax: i32::MIN, ymax: i32::MIN }
    }
    #[inline]
    pub fn include(&mut self, x: i32, y: i32) {
        if x < self.xmin { self.xmin = x; }
        if y < self.ymin { self.ymin = y; }
        if x > self.xmax { self.xmax = x; }
        if y > self.ymax { self.ymax = y; }
    }
    #[inline]
    pub fn width_i64(&self) -> i64 { i64::from(self.xmax) - i64::from(self.xmin) }
    #[inline]
    pub fn height_i64(&self) -> i64 { i64::from(self.ymax) - i64::from(self.ymin) }
    /// Compatibility span for APIs whose declared coordinate capacity is i32.
    /// Full-range boxes saturate instead of overflowing or panicking; exact and
    /// validation code must use [`Self::width_i64`].
    #[inline]
    pub fn width(&self) -> i32 {
        self.width_i64().clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
    }
    /// See [`Self::width`].
    #[inline]
    pub fn height(&self) -> i32 {
        self.height_i64().clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
    }
    /// Do two bboxes come within `dist` of each other? Used to prune spacing pairs.
    #[inline]
    pub fn within(&self, o: &Bbox, dist: i32) -> bool {
        let dist = i64::from(dist);
        i64::from(self.xmin) - dist <= i64::from(o.xmax)
            && i64::from(o.xmin) - dist <= i64::from(self.xmax)
            && i64::from(self.ymin) - dist <= i64::from(o.ymax)
            && i64::from(o.ymin) - dist <= i64::from(self.ymax)
    }
    #[inline]
    pub fn overlaps(&self, o: &Bbox) -> bool { self.within(o, 0) }
    /// Smallest bbox covering both. `empty()` is the identity.
    #[inline]
    pub fn union(&self, o: &Bbox) -> Bbox {
        Bbox {
            xmin: self.xmin.min(o.xmin),
            ymin: self.ymin.min(o.ymin),
            xmax: self.xmax.max(o.xmax),
            ymax: self.ymax.max(o.ymax),
        }
    }
    /// Overlap region, `None` when disjoint. Zero-width/height touching
    /// regions are returned, matching `overlaps`.
    #[inline]
    pub fn intersection(&self, o: &Bbox) -> Option<Bbox> {
        if !self.overlaps(o) { return None; }
        Some(Bbox {
            xmin: self.xmin.max(o.xmin),
            ymin: self.ymin.max(o.ymin),
            xmax: self.xmax.min(o.xmax),
            ymax: self.ymax.min(o.ymax),
        })
    }
    /// Bbox of a point sequence; `empty()` for an empty iterator.
    pub fn from_points(pts: impl IntoIterator<Item = (i32, i32)>) -> Bbox {
        let mut bb = Bbox::empty();
        for (x, y) in pts { bb.include(x, y); }
        bb
    }
}

/// The one big flat store. All checkers operate over borrowed slices of this — never over
/// owned per-shape objects. This is the primary public data structure the library exposes:
/// users can build one directly (immediate mode) instead of going through GDS.
#[derive(Default, Clone)]
pub struct GeometryStore {
    // --- vertex arrays (hot) ---
    pub verts_x: Vec<i32>,
    pub verts_y: Vec<i32>,
    // --- per-polygon (warm) ---
    pub poly_layer: Vec<LayerId>,
    pub poly_vert_start: Vec<u32>,
    pub poly_vert_len: Vec<u32>,
    pub poly_bbox: Vec<Bbox>,
    /// Lossless stream annotations carried through checked hierarchy flattening.
    /// Directly constructed polygons receive empty entries.
    pub poly_properties: Vec<Vec<(i16, String)>>,
    /// Root-to-instance path for each polygon. Directly constructed polygons
    /// receive an empty path.
    pub poly_hierarchy_path: Vec<Vec<String>>,
    /// Per-layer polygon buckets (index = LayerId, insertion order preserved).
    /// Maintained by `add_polygon` so `polys_on_layer` is O(k), not an O(N)
    /// full-store scan — it has 30+ call sites in DRC/LVS/PEX, many in loops.
    layer_index: Vec<Vec<u32>>,
    // --- net label annotations (cold) ---
    /// Maps polygon index to a net name label. Callers assign labels; LVS extraction checks
    /// that polygons sharing the same net carry consistent labels (or no label). Empty by
    /// default — label-driven extraction is opt-in.
    pub net_labels: std::collections::HashMap<u32, String>,
    // --- text annotations (cold) ---
    pub text_x: Vec<i32>,
    pub text_y: Vec<i32>,
    pub text_layer: Vec<i32>,
    pub text_datatype: Vec<i32>,
    pub text_string: Vec<String>,
    pub text_properties: Vec<Vec<(i16, String)>>,
    pub text_hierarchy_path: Vec<Vec<String>>,
}

impl GeometryStore {
    pub fn new() -> Self { Self::default() }

    #[inline]
    pub fn poly_count(&self) -> usize { self.poly_layer.len() }

    #[inline]
    pub fn text_count(&self) -> usize { self.text_string.len() }

    pub fn add_text(&mut self, layer: i32, datatype: i32, x: i32, y: i32, text: String) {
        self.add_text_annotated(layer, datatype, x, y, text, Vec::new(), Vec::new());
    }

    pub fn add_text_annotated(
        &mut self,
        layer: i32,
        datatype: i32,
        x: i32,
        y: i32,
        text: String,
        properties: Vec<(i16, String)>,
        hierarchy_path: Vec<String>,
    ) {
        self.text_x.push(x);
        self.text_y.push(y);
        self.text_layer.push(layer);
        self.text_datatype.push(datatype);
        self.text_string.push(text);
        self.text_properties.push(properties);
        self.text_hierarchy_path.push(hierarchy_path);
    }

    /// Append a polygon given as (x,y) vertex pairs. Returns its handle.
    /// The vertices are assumed to be a closed ring given without repeating the first point.
    pub fn add_polygon(&mut self, layer: LayerId, pts: &[(i32, i32)]) -> PolyId {
        self.add_polygon_annotated(layer, pts, Vec::new(), Vec::new())
    }

    pub fn add_polygon_annotated(
        &mut self,
        layer: LayerId,
        pts: &[(i32, i32)],
        properties: Vec<(i16, String)>,
        hierarchy_path: Vec<String>,
    ) -> PolyId {
        let start = self.verts_x.len() as u32;
        let mut bb = Bbox::empty();
        for &(x, y) in pts {
            self.verts_x.push(x);
            self.verts_y.push(y);
            bb.include(x, y);
        }
        let id = PolyId(self.poly_layer.len() as u32);
        self.poly_layer.push(layer);
        self.poly_vert_start.push(start);
        self.poly_vert_len.push(pts.len() as u32);
        self.poly_bbox.push(bb);
        self.poly_properties.push(properties);
        self.poly_hierarchy_path.push(hierarchy_path);
        if self.layer_index.len() <= layer as usize {
            self.layer_index.resize_with(layer as usize + 1, Vec::new);
        }
        self.layer_index[layer as usize].push(id.0);
        id
    }

    /// Convenience: append an axis-aligned rectangle.
    pub fn add_rect(&mut self, layer: LayerId, x: i32, y: i32, w: i32, h: i32) -> PolyId {
        self.add_polygon(layer, &[(x, y), (x + w, y), (x + w, y + h), (x, y + h)])
    }

    /// Borrow a polygon's vertex slice range. Zero-copy; returns index bounds.
    #[inline]
    pub fn poly_range(&self, p: PolyId) -> (usize, usize) {
        let s = self.poly_vert_start[p.0 as usize] as usize;
        let n = self.poly_vert_len[p.0 as usize] as usize;
        (s, s + n)
    }

    #[inline]
    pub fn poly_vertex(&self, base: usize, i: usize) -> (i32, i32) {
        (self.verts_x[base + i], self.verts_y[base + i])
    }

    /// Iterate a polygon's vertices in ring order. Borrowed, zero-copy view over
    /// the SoA arrays; the ring is given without repeating the first point,
    /// matching `add_polygon`.
    #[inline]
    pub fn vertices(&self, p: PolyId) -> impl Iterator<Item = (i32, i32)> + '_ {
        let (s, e) = self.poly_range(p);
        (s..e).map(move |i| (self.verts_x[i], self.verts_y[i]))
    }

    /// Iterate a polygon's directed edges, including the ring-closing wrap.
    /// Absorbs the manual `poly_range` + `(i + 1) % n` idiom at call sites.
    #[inline]
    pub fn edges_of(&self, p: PolyId) -> impl Iterator<Item = Edge> + '_ {
        let (s, e) = self.poly_range(p);
        let n = e - s;
        (0..n).map(move |i| {
            let (x0, y0) = self.poly_vertex(s, i);
            let (x1, y1) = self.poly_vertex(s, (i + 1) % n);
            Edge { x0, y0, x1, y1, poly: p.0 }
        })
    }

    /// Validated exact polygon for `p`: one fail-closed gate bundling vertex
    /// count, capacity, degeneracy, and self-intersection checks. This is the
    /// supported bridge from the SoA store into [`exact`] semantics — callers
    /// must not re-implement per-site validity checks.
    pub fn poly_as_exact(&self, p: PolyId) -> Result<exact::Polygon, exact::ExactGeometryError> {
        let pts: Vec<exact::Point> =
            self.vertices(p).map(|(x, y)| exact::Point { x, y }).collect();
        exact::Polygon::from_boundary_walk(pts)
    }

    /// Iterate polygon indices on a given layer. Existence-based filtering: the caller loops
    /// only the polygons it cares about. Served from the per-layer bucket index — O(k) in
    /// the layer's polygon count, insertion order (== old scan order) preserved.
    /// Borrowed, allocation-free; `.collect()` at the few sites that index or sort.
    pub fn polys_on_layer(&self, layer: LayerId) -> impl Iterator<Item = PolyId> + '_ {
        self.layer_index
            .get(layer as usize)
            .into_iter()
            .flat_map(|v| v.iter().copied().map(PolyId))
    }

    /// Signed area*2 of a polygon (shoelace). Positive => CCW. Used by min_area and by
    /// orientation-dependent checks.
    pub fn signed_area2_exact(&self, p: PolyId) -> Option<i128> {
        let (s, e) = self.poly_range(p);
        let n = e - s;
        let mut area = 0_i128;
        for i in 0..n {
            let (x0, y0) = self.poly_vertex(s, i);
            let (x1, y1) = self.poly_vertex(s, (i + 1) % n);
            let term = i128::from(x0)
                .checked_mul(i128::from(y1))?
                .checked_sub(i128::from(x1).checked_mul(i128::from(y0))?)?;
            area = area.checked_add(term)?;
        }
        Some(area)
    }

    /// Compatibility measurement for legacy i64 rule/report APIs. Exact
    /// validation uses [`Self::signed_area2_exact`]; out-of-range values are
    /// clamped here only after that validation has emitted a capacity error.
    pub fn signed_area2(&self, p: PolyId) -> i64 {
        match self.signed_area2_exact(p) {
            Some(area) => i64::try_from(area)
                .unwrap_or(if area < 0 { i64::MIN } else { i64::MAX }),
            None => i64::MAX,
        }
    }

    pub fn area_exact(&self, p: PolyId) -> Option<i128> {
        self.signed_area2_exact(p)?.checked_abs().map(|area2| area2 / 2)
    }

    pub fn area(&self, p: PolyId) -> i64 {
        self.area_exact(p)
            .and_then(|area| i64::try_from(area).ok())
            .unwrap_or(i64::MAX)
    }
}

/// A directed edge, materialized for scanline / edge-pair passes. This is the SoA "edge
/// stream" the DRC spacing/width algorithms consume. We build it on demand for a layer so
/// the hot loop iterates a dense array of edges with no polygon indirection.
#[derive(Clone, Copy, Debug)]
pub struct Edge {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
    pub poly: u32, // which polygon this edge belongs to (for same-poly filtering)
}

impl Edge {
    #[inline]
    pub fn dx_i64(&self) -> i64 { i64::from(self.x1) - i64::from(self.x0) }
    #[inline]
    pub fn dy_i64(&self) -> i64 { i64::from(self.y1) - i64::from(self.y0) }
    #[inline]
    pub fn dx(&self) -> i32 {
        self.dx_i64().clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
    }
    #[inline]
    pub fn dy(&self) -> i32 {
        self.dy_i64().clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
    }
    #[inline]
    pub fn len2_i128(&self) -> i128 {
        let dx = i128::from(self.dx_i64());
        let dy = i128::from(self.dy_i64());
        dx * dx + dy * dy
    }
    #[inline]
    pub fn len2(&self) -> i64 { i64::try_from(self.len2_i128()).unwrap_or(i64::MAX) }
    #[inline]
    pub fn is_horizontal(&self) -> bool { self.y0 == self.y1 }
    #[inline]
    pub fn is_vertical(&self) -> bool { self.x0 == self.x1 }
}

/// Build the dense edge list for one layer. Output is a flat Vec<Edge> — SoA-adjacent and
/// GPU-uploadable. Edges are emitted in polygon order (CCW ring => interior on the left).
pub fn build_edges(store: &GeometryStore, layer: LayerId) -> Vec<Edge> {
    let mut edges = Vec::new();
    for p in store.polys_on_layer(layer) {
        edges.extend(store.edges_of(p));
    }
    edges
}

/// Squared Euclidean distance between two segments' closest points (any angle:
/// for non-crossing segments the minimum is always at an endpoint).
/// Returns 0 if they touch/cross. This is the primitive both spacing and corner checks use.
pub fn seg_seg_dist2(a: &Edge, b: &Edge) -> i64 {
    // If bounding boxes overlap and the segments intersect, distance is 0.
    if segments_intersect(a, b) {
        return 0;
    }
    let mut best = i64::MAX;
    for &(px, py) in &[(a.x0, a.y0), (a.x1, a.y1)] {
        best = best.min(point_seg_dist2(px, py, b));
    }
    for &(px, py) in &[(b.x0, b.y0), (b.x1, b.y1)] {
        best = best.min(point_seg_dist2(px, py, a));
    }
    best
}

#[inline]
fn point_seg_dist2(px: i32, py: i32, e: &Edge) -> i64 {
    let vx = i128::from(e.dx_i64());
    let vy = i128::from(e.dy_i64());
    let wx = i128::from(px) - i128::from(e.x0);
    let wy = i128::from(py) - i128::from(e.y0);
    let c1 = vx * wx + vy * wy;
    if c1 <= 0 {
        return i64::try_from(wx * wx + wy * wy).unwrap_or(i64::MAX);
    }
    let c2 = vx * vx + vy * vy;
    if c2 <= c1 {
        let dx = i128::from(px) - i128::from(e.x1);
        let dy = i128::from(py) - i128::from(e.y1);
        return i64::try_from(dx * dx + dy * dy).unwrap_or(i64::MAX);
    }
    // projection falls on the segment: d² = |w|² − c1²/c2, exact in i128
    // (f64 here loses ulps on diagonal edges at large coordinates)
    let Some(num) = (wx * wx + wy * wy)
        .checked_mul(c2)
        .and_then(|lhs| c1.checked_mul(c1).and_then(|rhs| lhs.checked_sub(rhs)))
    else {
        // The legacy scalar-distance API cannot represent this intermediate.
        // DRC validation rejects such coordinate extents before rule execution.
        return i64::MAX;
    };
    i64::try_from(num / c2).unwrap_or(i64::MAX)
}

fn orient(ax: i64, ay: i64, bx: i64, by: i64, cx: i64, cy: i64) -> i128 {
    (i128::from(bx) - i128::from(ax)) * (i128::from(cy) - i128::from(ay))
        - (i128::from(by) - i128::from(ay)) * (i128::from(cx) - i128::from(ax))
}

fn on_seg(ax: i64, ay: i64, bx: i64, by: i64, cx: i64, cy: i64) -> bool {
    cx >= ax.min(bx) && cx <= ax.max(bx) && cy >= ay.min(by) && cy <= ay.max(by)
}

pub fn segments_intersect(a: &Edge, b: &Edge) -> bool {
    let (ax, ay, bx, by) = (a.x0 as i64, a.y0 as i64, a.x1 as i64, a.y1 as i64);
    let (cx, cy, dx, dy) = (b.x0 as i64, b.y0 as i64, b.x1 as i64, b.y1 as i64);
    let d1 = orient(cx, cy, dx, dy, ax, ay);
    let d2 = orient(cx, cy, dx, dy, bx, by);
    let d3 = orient(ax, ay, bx, by, cx, cy);
    let d4 = orient(ax, ay, bx, by, dx, dy);
    if ((d1 > 0) != (d2 > 0)) && ((d3 > 0) != (d4 > 0)) {
        return true;
    }
    (d1 == 0 && on_seg(cx, cy, dx, dy, ax, ay))
        || (d2 == 0 && on_seg(cx, cy, dx, dy, bx, by))
        || (d3 == 0 && on_seg(ax, ay, bx, by, cx, cy))
        || (d4 == 0 && on_seg(ax, ay, bx, by, dx, dy))
}

/// Area of a polygon clipped to an axis-aligned window (Sutherland–Hodgman + shoelace).
/// Exact for rectilinear geometry; f64 for the fractional intersection points diagonal
/// edges can produce. This is what density checks need — a bbox-based coverage estimate
/// wildly overstates non-convex shapes like combs.
pub fn clipped_area(
    store: &GeometryStore, p: PolyId, xmin: i32, ymin: i32, xmax: i32, ymax: i32,
) -> f64 {
    clipped_area_i64(
        store,
        p,
        i64::from(xmin),
        i64::from(ymin),
        i64::from(xmax),
        i64::from(ymax),
    )
}

/// Wide-coordinate density clip. Window edges may extend past the i32 layout
/// domain even though every stored vertex is representable (for example a
/// 10-DBU window anchored five DBU below i32::MAX).
pub fn clipped_area_i64(
    store: &GeometryStore, p: PolyId, xmin: i64, ymin: i64, xmax: i64, ymax: i64,
) -> f64 {
    let (s, e) = store.poly_range(p);
    // Work in window-relative coordinates. Shoelace on absolute coordinates
    // catastrophically cancels a 5x5 area translated near i32::MAX.
    let (origin_x, origin_y) = (i128::from(xmin), i128::from(ymin));
    let mut ring: Vec<(f64, f64)> = (s..e)
        .map(|i| {
            (
                (i128::from(store.verts_x[i]) - origin_x) as f64,
                (i128::from(store.verts_y[i]) - origin_y) as f64,
            )
        })
        .collect();
    // clip against each half-plane: keep(pt) true => inside
    let planes: [(f64, bool, bool); 4] = [
        (0.0, true, true),
        ((i128::from(xmax) - origin_x) as f64, true, false),
        (0.0, false, true),
        ((i128::from(ymax) - origin_y) as f64, false, false),
    ];
    for &(c, is_x, keep_ge) in &planes {
        if ring.is_empty() { return 0.0; }
        let val = |pt: (f64, f64)| if is_x { pt.0 } else { pt.1 };
        let inside = |pt: (f64, f64)| if keep_ge { val(pt) >= c } else { val(pt) <= c };
        let mut out: Vec<(f64, f64)> = Vec::with_capacity(ring.len() + 4);
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            let (ia, ib) = (inside(a), inside(b));
            let cross = |a: (f64, f64), b: (f64, f64)| -> (f64, f64) {
                let t = (c - val(a)) / (val(b) - val(a));
                (a.0 + t * (b.0 - a.0), a.1 + t * (b.1 - a.1))
            };
            if ia {
                out.push(a);
                if !ib { out.push(cross(a, b)); }
            } else if ib {
                out.push(cross(a, b));
            }
        }
        ring = out;
    }
    let mut a2 = 0.0;
    for i in 0..ring.len() {
        let (x0, y0) = ring[i];
        let (x1, y1) = ring[(i + 1) % ring.len()];
        a2 += x0 * y1 - x1 * y0;
    }
    (a2 / 2.0).abs()
}

/// Is a point strictly inside a polygon? Even-odd ray cast; points exactly on the boundary
/// return false. Integer-exact for the on-edge test, half-open on crossing counts.
pub fn point_in_poly(store: &GeometryStore, p: PolyId, px: i32, py: i32) -> bool {
    let (s, e) = store.poly_range(p);
    let n = e - s;
    let (px, py) = (px as i64, py as i64);
    let mut inside = false;
    for i in 0..n {
        let (x0, y0) = store.poly_vertex(s, i);
        let (x1, y1) = store.poly_vertex(s, (i + 1) % n);
        let (x0, y0, x1, y1) = (x0 as i64, y0 as i64, x1 as i64, y1 as i64);
        // on-boundary => not strictly inside
        if orient(x0, y0, x1, y1, px, py) == 0 && on_seg(x0, y0, x1, y1, px, py) {
            return false;
        }
        if (y0 > py) != (y1 > py) {
            // exact crossing test: px < x-intersection of the edge with the horizontal ray
            let lhs = i128::from(x1 - x0) * i128::from(py - y0);
            let rhs = i128::from(px - x0) * i128::from(y1 - y0);
            let cross = if y1 > y0 { lhs > rhs } else { lhs < rhs };
            if cross { inside = !inside; }
        }
    }
    inside
}

/// Does the polygon's boundary properly cross itself (bow-tie / figure-8)?
/// Only PROPER crossings of non-adjacent edges count: collinear-overlap slits
/// (the GDS keyhole representation of holes) are legal and must not flag.
/// O(n²) over the ring — polygons are small (rects dominate); revisit with a
/// sweep if fractured all-angle data shows up.
pub fn poly_self_intersects(store: &GeometryStore, p: PolyId) -> bool {
    let (s, e) = store.poly_range(p);
    let n = e - s;
    if n < 4 { return false; } // triangle can't self-cross
    let edge = |i: usize| -> Edge {
        let (x0, y0) = store.poly_vertex(s, i);
        let (x1, y1) = store.poly_vertex(s, (i + 1) % n);
        Edge { x0, y0, x1, y1, poly: p.0 }
    };
    // Sweep along the axis with more bbox-min spread: crossing edges must have
    // overlapping bboxes, so the look-ahead window stays local instead of the
    // all-pairs O(n²) that melts on many-thousand-vertex comb polygons.
    let bb = store.poly_bbox[p.0 as usize];
    let sweep_x = bb.width_i64() >= bb.height_i64();
    let lo = |ed: &Edge| if sweep_x { ed.x0.min(ed.x1) } else { ed.y0.min(ed.y1) };
    let hi = |ed: &Edge| if sweep_x { ed.x0.max(ed.x1) } else { ed.y0.max(ed.y1) };
    let mut order: Vec<u32> = (0..n as u32).collect();
    order.sort_unstable_by_key(|&i| lo(&edge(i as usize)));
    for w in 0..n {
        let i = order[w] as usize;
        let a = edge(i);
        if a.len2_i128() == 0 { continue; }
        let a_hi = hi(&a);
        for &jj in order[w + 1..].iter() {
            let j = jj as usize;
            let b = edge(j);
            if lo(&b) > a_hi { break; } // sweep window closed
            // adjacent edges share a vertex; skip (incl. the ring wrap)
            if j == (i + 1) % n || i == (j + 1) % n { continue; }
            if b.len2_i128() == 0 { continue; }
            let d1 = orient(b.x0 as i64, b.y0 as i64, b.x1 as i64, b.y1 as i64, a.x0 as i64, a.y0 as i64);
            let d2 = orient(b.x0 as i64, b.y0 as i64, b.x1 as i64, b.y1 as i64, a.x1 as i64, a.y1 as i64);
            let d3 = orient(a.x0 as i64, a.y0 as i64, a.x1 as i64, a.y1 as i64, b.x0 as i64, b.y0 as i64);
            let d4 = orient(a.x0 as i64, a.y0 as i64, a.x1 as i64, a.y1 as i64, b.x1 as i64, b.y1 as i64);
            if ((d1 > 0) != (d2 > 0)) && ((d3 > 0) != (d4 > 0)) && d1 != 0 && d2 != 0 && d3 != 0 && d4 != 0 {
                return true;
            }
        }
    }
    false
}
/// Integer sqrt floor, for reporting measured distances from squared values.
pub fn isqrt(n: i64) -> i64 {
    if n < 0 { return 0; }
    let mut x = (n as f64).sqrt() as i64;
    while i128::from(x + 1) * i128::from(x + 1) <= i128::from(n) { x += 1; }
    while i128::from(x) * i128::from(x) > i128::from(n) { x -= 1; }
    x
}

#[cfg(test)]
mod core_api_tests {
    use super::*;

    #[test]
    fn bbox_set_ops() {
        let a = Bbox { xmin: 0, ymin: 0, xmax: 10, ymax: 10 };
        let b = Bbox { xmin: 5, ymin: 5, xmax: 20, ymax: 20 };
        assert_eq!(a.union(&b), Bbox { xmin: 0, ymin: 0, xmax: 20, ymax: 20 });
        assert_eq!(a.intersection(&b), Some(Bbox { xmin: 5, ymin: 5, xmax: 10, ymax: 10 }));
        let far = Bbox { xmin: 100, ymin: 100, xmax: 110, ymax: 110 };
        assert_eq!(a.intersection(&far), None);
        assert_eq!(Bbox::from_points([(3, 4), (-1, 7)]), Bbox { xmin: -1, ymin: 4, xmax: 3, ymax: 7 });
        assert_eq!(Bbox::from_points(std::iter::empty()), Bbox::empty());
    }

    #[test]
    fn iterators_match_manual_walk() {
        let mut store = GeometryStore::new();
        let p = store.add_rect(0, 0, 0, 10, 5);
        assert_eq!(
            store.vertices(p).collect::<Vec<_>>(),
            vec![(0, 0), (10, 0), (10, 5), (0, 5)]
        );
        let edges: Vec<Edge> = store.edges_of(p).collect();
        assert_eq!(edges.len(), 4);
        // ring closes back to the first vertex
        assert_eq!((edges[3].x1, edges[3].y1), (0, 0));
        assert!(edges.iter().all(|e| e.poly == p.0));
    }

    #[test]
    fn poly_as_exact_is_fail_closed() {
        let mut store = GeometryStore::new();
        let good = store.add_rect(0, 0, 0, 10, 5);
        assert!(store.poly_as_exact(good).is_ok());
        // bow-tie must be a typed error, never an empty/clean result
        let bad = store.add_polygon(0, &[(0, 0), (10, 10), (10, 0), (0, 10)]);
        assert!(store.poly_as_exact(bad).is_err());
        let degenerate = store.add_polygon(0, &[(0, 0), (1, 1)]);
        assert!(store.poly_as_exact(degenerate).is_err());
    }
}
