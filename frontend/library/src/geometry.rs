//! Flatten a placed and routed solution into the shape list signoff and the
//! GDS writer consume, plus the rectangle helpers the output path shares.

use pnr_core::{Layout, Macro, Rect, Routes, Shape};

/// Returns every macro's shapes stamped at its placement (the one transform,
/// shared with the router via `gr::place_macros`), then every routed wire,
/// net by net. `macros` is indexed like `layout`; a macro past the layout is
/// taken as already absolute.
#[must_use]
pub fn collect(macros: &[Macro], layout: &Layout, routes: &Routes) -> Vec<Shape> {
    let mut out = Vec::new();
    for m in gr::place_macros(macros, layout) {
        out.extend(m.shapes);
    }
    for net in &routes.wires {
        out.extend_from_slice(net);
    }
    out
}

/// Returns the bounding box of `shapes`, `None` when there are none.
pub(crate) fn bbox(shapes: &[Shape]) -> Option<Rect> {
    let first = shapes.first()?.rect;
    let (mut x0, mut y0, mut x1, mut y1) = (first.x, first.y, first.x + first.w, first.y + first.h);
    for s in shapes {
        x0 = x0.min(s.rect.x);
        y0 = y0.min(s.rect.y);
        x1 = x1.max(s.rect.x + s.rect.w);
        y1 = y1.max(s.rect.y + s.rect.h);
    }
    Some(Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 })
}

/// Returns the Chebyshev edge-to-edge gap between `a` and `b`, nm: the larger
/// of the x and y separations, `0` when they touch or overlap. Symmetric.
pub(crate) fn rect_gap(a: &Rect, b: &Rect) -> i32 {
    let dx = (b.x - (a.x + a.w)).max(a.x - (b.x + b.w)).max(0);
    let dy = (b.y - (a.y + a.h)).max(a.y - (b.y + b.h)).max(0);
    dx.max(dy)
}

/// Placement quality of one placed layout (PLC-01), measured, never steered on.
#[derive(Clone, Copy, Debug, Default)]
pub struct PlacementMetrics {
    /// Footprint / Σ cell bbox area (Plantage's area usage; ring halos are
    /// inside each cell bbox). `1.0` = perfectly packed.
    pub area_usage: f32,
    /// Cells whose stamped bbox origin is off the cut lattice on either axis.
    pub lattice_off: u32,
    /// Clearance encroachment beyond plain overlap, nm².
    pub clearance_residue_nm2: f64,
    /// Plain pairwise bbox overlap, nm².
    pub overlap_nm2: f64,
    /// Matched pairs (`RuleBatch::matched_pairs`) drawn at different orient,
    /// or at different shape `(variant, hw, hh)` inside one shape set.
    pub matched_geometry_mismatch: u32,
    /// Symmetry islands beyond one per axis: Σ residual of the `SymmetryIsland`
    /// budget batches (PLC-12); `Some(0)` when there is none.
    pub islands_extra: Option<u32>,
    /// The same count over the recognition blocks (glue excluded), each with
    /// `touch = spacing.max_gap() + lattice`: metric only.
    pub clusters_extra: Option<u32>,
}

/// Returns the [`PlacementMetrics`] of `l` with cells drawn as `macros`
/// (indexed like `l`), against the cut `lattice` (nm; `<= 0` reads as 1) and
/// the per-pair cell spacing `rules`. Matched pairs come from every arm of
/// `reqs`; `locks` says which of them must share a shape; `groups` are the
/// recognition blocks, the last one the glue block.
///
/// # Panics
/// When `macros` is shorter than the layout.
#[must_use]
pub fn placement_metrics(
    macros: &[Macro],
    l: &Layout,
    lattice: i32,
    rules: &gp::PlaceRules,
    reqs: &analog::Requirements<Layout>,
    locks: &dp::locks::Locks,
    groups: &[Vec<pnr_core::DeviceId>],
) -> PlacementMetrics {
    let n = l.x.len();
    let cells: f64 = (0..n).map(|i| 4.0 * f64::from(l.hw[i]) * f64::from(l.hh[i])).sum();
    let lat = lattice.max(1);
    debug_assert_eq!(macros.len(), n, "macros not indexed like the layout");
    let lattice_off = (0..n)
        .filter(|&i| {
            let b = pnr_core::place_macro(&macros[i], l, i).bbox;
            b.x.rem_euclid(lat) != 0 || b.y.rem_euclid(lat) != 0
        })
        .count() as u32;
    let overlap_nm2 = gp::mechanics::encroachment(l, 0);
    let mut pairs = Vec::new();
    for b in reqs.hard.iter().chain(&reqs.budget).chain(&reqs.cost) {
        b.matched_pairs(&mut pairs);
    }
    pairs.retain(|&(a, b)| a != b && (a as usize) < n && (b as usize) < n);
    for p in &mut pairs {
        *p = (p.0.min(p.1), p.0.max(p.1));
    }
    pairs.sort_unstable();
    pairs.dedup();
    let shape = |i: usize| (l.variant.get(i), l.hw.get(i), l.hh.get(i));
    let locked = |a: usize, b: usize| locks.shape_of.get(a).copied().flatten().is_some_and(|s| locks.shape_of.get(b) == Some(&Some(s)));
    let matched_geometry_mismatch = pairs
        .iter()
        .map(|&(a, b)| (a as usize, b as usize))
        .filter(|&(a, b)| l.orient.get(a) != l.orient.get(b) || (locked(a, b) && shape(a) != shape(b)))
        .count() as u32;
    PlacementMetrics {
        area_usage: if cells > 0.0 { (l.footprint_nm2() / cells) as f32 } else { 0.0 },
        lattice_off,
        clearance_residue_nm2: rules.encroachment(l) - overlap_nm2,
        overlap_nm2,
        matched_geometry_mismatch,
        islands_extra: Some(reqs.budget.iter().filter(|b| b.kind() == "SymmetryIsland").map(|b| b.residual(l) as u32).sum()),
        clusters_extra: Some(clusters_extra(l, groups, rules.spacing.max_gap() + lattice)),
    }
}

/// Returns Σ over `groups` but the last (the glue block) of their islands past
/// the first ([`analog::placement::island::components`] at `touch_nm`);
/// a group with fewer than 2 distinct in-layout cells counts 0.
fn clusters_extra(l: &Layout, groups: &[Vec<pnr_core::DeviceId>], touch_nm: i32) -> u32 {
    let n = l.x.len();
    groups[..groups.len().saturating_sub(1)]
        .iter()
        .map(|g| {
            let mut ids: Vec<u16> = g.iter().map(|d| d.0).filter(|&d| usize::from(d) < n).collect();
            ids.sort_unstable();
            ids.dedup();
            let m: Vec<pnr_core::ids::Target> = ids.into_iter().map(|d| pnr_core::ids::Target::Device(pnr_core::DeviceId(d))).collect();
            if m.len() < 2 { 0 } else { analog::placement::island::components(l, &m, touch_nm) - 1 }
        })
        .sum()
}

/// Merges `layer`'s rects pairwise, until none qualify, wherever their union
/// is exactly one rectangle: same span on one axis and touching on the other,
/// or one inside the other. The drawn area is unchanged; the checker, which
/// reads a well per rect, then sees a bridged well as one. Other layers keep
/// their relative order; `layer`'s survivors move to the end.
///
/// Cost: O(n³) worst case in `layer`'s rect count (wells are few).
pub fn merge_rects(shapes: &mut Vec<Shape>, layer: pnr_core::LayerId) {
    // ponytail: restart-on-merge pair scan; a sweep line if a layer ever has
    // thousands of rects.
    let (mut on, rest): (Vec<Shape>, Vec<Shape>) = shapes.drain(..).partition(|s| s.layer == layer);
    let joined = |a: Rect, b: Rect| -> Option<Rect> {
        let (x0, x1) = (a.x.min(b.x), (a.x + a.w).max(b.x + b.w));
        let (y0, y1) = (a.y.min(b.y), (a.y + a.h).max(b.y + b.h));
        let rows = a.y == b.y && a.h == b.h && a.x <= b.x + b.w && b.x <= a.x + a.w;
        let cols = a.x == b.x && a.w == b.w && a.y <= b.y + b.h && b.y <= a.y + a.h;
        let inside = |p: Rect, q: Rect| p.x >= q.x && p.y >= q.y && p.x + p.w <= q.x + q.w && p.y + p.h <= q.y + q.h;
        (rows || cols || inside(a, b) || inside(b, a)).then_some(Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 })
    };
    'again: loop {
        for i in 0..on.len() {
            for j in i + 1..on.len() {
                if let Some(r) = joined(on[i].rect, on[j].rect) {
                    on[i].rect = r;
                    on.swap_remove(j);
                    continue 'again;
                }
            }
        }
        break;
    }
    shapes.extend(rest);
    shapes.extend(on);
}

/// Panics, in debug builds only, unless every pin of each net with at least
/// two pins touches that net's routed geometry (closed-interval xy contact
/// through pins and wires). `Routes::debug_check_joined` only proves the
/// wires are self-connected, which a net can satisfy while missing its pins
/// entirely. A no-op in release builds.
///
/// # Panics
/// Naming the net, when a net with ≥ 2 pins has no wires or leaves a pin
/// unreached; the message gives each orphan's nearest wire and its gap.
pub fn debug_check_connected(macros: &[Macro], layout: &Layout, routes: &Routes) {
    if !cfg!(debug_assertions) {
        return;
    }
    // ponytail: xy-only, ignores whether the touch has a via stack; O(k²) per net.
    let placed = gr::place_macros(macros, layout);
    let mut pins_of: Vec<Vec<Rect>> = vec![Vec::new(); routes.wires.len()];
    for p in placed.iter().flat_map(|m| &m.pins) {
        let net = p.net.0 as usize;
        if net >= pins_of.len() {
            pins_of.resize(net + 1, Vec::new());
        }
        pins_of[net].push(p.at);
    }
    for (net, pins) in pins_of.iter().enumerate() {
        // One pin is nothing to connect; zero means the net has no terminals here.
        if pins.len() < 2 {
            continue;
        }
        let wires = routes.wires.get(net).map_or(&[][..], Vec::as_slice);
        assert!(
            !wires.is_empty(),
            "routing: net {net} has {} pins and no routed geometry at all — it was \
             dropped, not routed",
            pins.len()
        );
        let unreached = unreached_pins(pins, wires);
        if unreached.is_empty() {
            continue;
        }
        // Nearest wire per orphaned pin: a few nm off is a track-snap problem, a
        // few µm off means the router aimed somewhere else entirely.
        let detail: Vec<String> = unreached
            .iter()
            .map(|&i| {
                let p = pins[i];
                match wires.iter().min_by_key(|s| rect_gap(&p, &s.rect)) {
                    Some(s) => format!(
                        "pin[{i}] {p:?} — nearest wire {:?} on layer {:?}, {} nm away",
                        s.rect,
                        s.layer,
                        rect_gap(&p, &s.rect)
                    ),
                    None => format!("pin[{i}] {p:?} — no wires"),
                }
            })
            .collect();
        panic!(
            "routing: net {net} is open — {} of {} pins unreached by its {} wires:\n  {}",
            unreached.len(),
            pins.len(),
            wires.len(),
            detail.join("\n  "),
        );
    }
}

/// Returns the indices of `pins` not reached from `pins[0]` by a flood over
/// pins ∪ `wires` under [`Rect::touches`]; empty when `pins` is empty.
fn unreached_pins(pins: &[Rect], wires: &[Shape]) -> Vec<usize> {
    if pins.is_empty() {
        return Vec::new();
    }
    let boxes: Vec<Rect> = pins.iter().copied().chain(wires.iter().map(|s| s.rect)).collect();
    let mut seen = vec![false; boxes.len()];
    let mut stack = vec![0usize];
    seen[0] = true;
    while let Some(a) = stack.pop() {
        for b in 0..boxes.len() {
            if !seen[b] && boxes[a].touches(&boxes[b]) {
                seen[b] = true;
                stack.push(b);
            }
        }
    }
    (0..pins.len()).filter(|&i| !seen[i]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_core::{LayerId, Orient};

    /// An L of two rects, so a turn is detectable (a single rect would look the
    /// same under R180).
    fn ell() -> Macro {
        let shapes = vec![
            Shape {
                layer: LayerId(1),
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 100,
                    h: 400,
                },
            },
            Shape {
                layer: LayerId(1),
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 300,
                    h: 100,
                },
            },
        ];
        Macro {
            shapes,
            pins: vec![],
            bbox: Rect {
                x: 0,
                y: 0,
                w: 300,
                h: 400,
            },
            units: Vec::new(),
            dummies: Vec::new(),
            ..Default::default()
        }
    }

    fn layout_of(o: Orient, x: i32, y: i32) -> Layout {
        Layout {
            x: vec![x],
            y: vec![y],
            hw: vec![0],
            hh: vec![0],
            axis: vec![],
            groups: vec![],
            orient: vec![o],
            variant: vec![0],
            branch: vec![],
            power_uw: vec![0],
            temp_mc: vec![0],
            units: Default::default(),
        }
    }

    fn bbox_of(shapes: &[Shape]) -> Rect {
        bbox(shapes).expect("shapes")
    }

    /// The invariant the whole flow depends on: drawn geometry must land where
    /// the placer legalised it. `Layout` is centre + half-extents, so the stamped
    /// bbox must be centred on `(x, y)` with those extents — not merely
    /// translated by them. Getting this wrong put guard rings through devices and
    /// shorted supply nets, and it is invisible whenever `hw`/`hh` are zero, so
    /// the extents here are deliberately non-zero.
    #[test]
    fn drawn_bbox_is_centred_on_the_placed_centre() {
        let routes = Routes { wires: vec![], ..Default::default()  };
        let m = ell();
        let (hw, hh) = (m.bbox.w / 2, m.bbox.h / 2);
        let mut l = layout_of(Orient::R0, 7_000, 3_000);
        l.hw = vec![hw];
        l.hh = vec![hh];
        let got = bbox_of(&collect(&[m], &l, &routes));
        assert_eq!((got.x, got.y), (7_000 - hw, 3_000 - hh), "bbox not centred");
        assert_eq!((got.w, got.h), (2 * hw, 2 * hh));
    }

    /// A macro whose local bbox does not start at the origin must still centre.
    /// This is the case that actually broke: cells are built around their own
    /// origin, so `bbox.x`/`bbox.y` are negative and were silently dropped.
    #[test]
    fn offset_local_bbox_still_centres() {
        let routes = Routes { wires: vec![], ..Default::default()  };
        let shifted = Macro {
            shapes: ell()
                .shapes
                .iter()
                .map(|s| Shape {
                    layer: s.layer,
                    rect: Rect {
                        x: s.rect.x - 2_000,
                        y: s.rect.y - 500,
                        ..s.rect
                    },
                })
                .collect(),
            pins: vec![],
            bbox: Rect {
                x: -2_000,
                y: -500,
                w: 300,
                h: 400,
            },
            units: Vec::new(),
            dummies: Vec::new(),
            ..Default::default()
        };
        let (hw, hh) = (150, 200);
        let mut l = layout_of(Orient::R0, 40_000, 60_000);
        l.hw = vec![hw];
        l.hh = vec![hh];
        let got = bbox_of(&collect(&[shifted], &l, &routes));
        assert_eq!((got.x, got.y), (40_000 - hw, 60_000 - hh));
    }

    /// A turned macro keeps its anchor and transposes its extents — the contract
    /// `dp::try_rotate` assumes when it swaps `hw`/`hh`.
    #[test]
    fn r90_transposes_and_keeps_the_anchor() {
        let routes = Routes { wires: vec![], ..Default::default()  };
        let flat = bbox_of(&collect(
            &[ell()],
            &layout_of(Orient::R0, 500, 900),
            &routes,
        ));
        let turned = bbox_of(&collect(
            &[ell()],
            &layout_of(Orient::R90, 500, 900),
            &routes,
        ));
        assert_eq!((turned.x, turned.y), (flat.x, flat.y), "anchor moved");
        assert_eq!(
            (turned.w, turned.h),
            (flat.h, flat.w),
            "extents not transposed"
        );
    }

    /// Every [`PlacementMetrics`] count and area on a hand-built layout, so a
    /// metric that silently reads 0 fails here (PLC-02/03/09 assert them 0).
    /// Three 200×200 cells (`hw = hh = 100`), lattice 100, clearance 50:
    /// c0 spans x 0..200, c1 100..300 (overlaps c0), c2 330..530 (30 nm from
    /// c1: inside clearance only; origin 330 is off the lattice).
    #[test]
    fn placement_metrics_counts_each_defect() {
        use analog::placement::symmetry::{SymMode, Symmetry};
        use pnr_core::ids::{AxisId, DeviceId, Target};
        let mut l = layout_of(Orient::R0, 0, 0);
        l.x = vec![100, 200, 430];
        l.y = vec![100; 3];
        l.hw = vec![100; 3];
        l.hh = vec![100; 3];
        l.orient = vec![Orient::R0; 3];
        l.variant = vec![0, 0, 1];
        let sym = |a: u16, b: u16| Symmetry { a: Target::Device(DeviceId(a)), b: Target::Device(DeviceId(b)), axis: AxisId(0), mode: SymMode::Perfect };
        let reqs = analog::Requirements::<Layout> {
            // (0, 2) differs in variant; (0, 1) matches; (1, 1) is a pair collapsed into one cell.
            hard: vec![Box::new(vec![sym(0, 2), sym(0, 1), sym(1, 1)])],
            budget: vec![],
            // Matched pairs come from every tier.
            cost: vec![Box::new(vec![sym(1, 2)])],
        };
        // No variants: every pair is shape-locked.
        let locks = dp::locks::locks(&reqs, 3, &[]);
        let m = placement_metrics(&vec![Macro::default(); 3], &l, 100, &gp::PlaceRules::uniform(100, 50), &reqs, &locks, &[]);
        assert_eq!(m.lattice_off, 1, "{m:?}");
        // c0–c1: 100 × 200.
        assert_eq!(m.overlap_nm2, 20_000.0, "{m:?}");
        // At clearance 50: c0–c1 150 × 250, c1–c2 20 × 250; minus the plain overlap.
        assert_eq!(m.clearance_residue_nm2, 37_500.0 + 5_000.0 - 20_000.0, "{m:?}");
        assert_eq!(m.matched_geometry_mismatch, 2, "{m:?}");
        // Footprint 530 × 200 over 3 × 200 × 200.
        assert!((m.area_usage - 106_000.0 / 120_000.0).abs() < 1e-6, "{m:?}");
        assert_eq!((m.islands_extra, m.clusters_extra), (Some(0), Some(0)), "no island batch, no block");
        // A turned partner counts on orient alone: (0, 1) joins.
        l.orient[1] = Orient::R90;
        let m = placement_metrics(&vec![Macro::default(); 3], &l, 100, &gp::PlaceRules::uniform(100, 50), &reqs, &locks, &[]);
        assert_eq!(m.matched_geometry_mismatch, 3, "{m:?}");
    }

    /// Four quarter-turns return the exact original geometry — no drift creeps in
    /// through the re-anchoring arithmetic.
    #[test]
    fn four_quarter_turns_are_the_identity() {
        let routes = Routes { wires: vec![], ..Default::default()  };
        let mut m = ell();
        for _ in 0..4 {
            let shapes = collect(&[m.clone()], &layout_of(Orient::R90, 0, 0), &routes);
            m = Macro {
                bbox: bbox_of(&shapes),
                shapes,
                pins: vec![],
                units: Vec::new(),
                dummies: Vec::new(),
                ..Default::default()
            };
        }
        let orig = ell();
        assert_eq!(m.bbox, orig.bbox);
        for (a, b) in m.shapes.iter().zip(&orig.shapes) {
            assert_eq!(a.rect, b.rect);
        }
    }

    fn r(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    fn on(layer: u16, rect: Rect) -> Shape {
        Shape { layer: LayerId(layer), rect }
    }

    /// A layout with no cells: every macro is taken as already absolute.
    fn no_cells() -> Layout {
        let mut l = layout_of(Orient::R0, 0, 0);
        for v in [&mut l.x, &mut l.y, &mut l.hw, &mut l.hh, &mut l.power_uw, &mut l.temp_mc] {
            v.clear();
        }
        l.orient.clear();
        l.variant.clear();
        l
    }

    #[test]
    fn collect_of_nothing_is_empty() {
        assert!(collect(&[], &no_cells(), &Routes::default()).is_empty());
    }

    /// Macro shapes come first, then wires net by net, in order.
    #[test]
    fn collect_puts_wires_after_cells_in_net_order() {
        let m = Macro { shapes: vec![on(1, r(0, 0, 5, 5))], ..Default::default() };
        let routes = Routes { wires: vec![vec![on(2, r(10, 0, 1, 1))], vec![], vec![on(3, r(20, 0, 1, 1)), on(4, r(30, 0, 1, 1))]], ..Default::default() };
        let got: Vec<u16> = collect(&[m], &no_cells(), &routes).iter().map(|s| s.layer.0).collect();
        assert_eq!(got, [1, 2, 3, 4]);
    }

    #[test]
    fn bbox_corners() {
        assert_eq!(bbox(&[]), None);
        assert_eq!(bbox(&[on(0, r(-3, 4, 0, 0))]), Some(r(-3, 4, 0, 0)), "a point is its own bbox");
        assert_eq!(bbox(&[on(0, r(-10, 0, 5, 5)), on(1, r(0, -20, 30, 1))]), Some(r(-10, -20, 40, 25)));
    }

    #[test]
    fn rect_gap_is_chebyshev_and_symmetric() {
        let a = r(0, 0, 10, 10);
        for (b, want) in [
            (r(5, 5, 10, 10), 0),   // overlap
            (r(10, 0, 5, 5), 0),    // edge contact
            (r(10, 10, 5, 5), 0),   // corner contact
            (r(17, 2, 5, 5), 7),    // right
            (r(-9, 0, 5, 5), 4),    // left
            (r(0, -8, 5, 5), 3),    // below
            (r(13, 16, 1, 1), 6),   // diagonal: max(3, 6)
        ] {
            assert_eq!(rect_gap(&a, &b), want, "{b:?}");
            assert_eq!(rect_gap(&b, &a), want, "{b:?} symmetric");
        }
    }

    fn merged(shapes: &[Shape]) -> Vec<Shape> {
        let mut v = shapes.to_vec();
        merge_rects(&mut v, LayerId(1));
        v
    }

    #[test]
    fn merge_rects_joins_abutting_rows_and_columns() {
        assert_eq!(merged(&[on(1, r(0, 0, 10, 5)), on(1, r(10, 0, 7, 5))]), [on(1, r(0, 0, 17, 5))]);
        assert_eq!(merged(&[on(1, r(0, 0, 5, 10)), on(1, r(0, 4, 5, 10))]), [on(1, r(0, 0, 5, 14))]);
        // A chain of three collapses to one, in any order.
        assert_eq!(merged(&[on(1, r(20, 0, 10, 5)), on(1, r(0, 0, 10, 5)), on(1, r(10, 0, 10, 5))]), [on(1, r(0, 0, 30, 5))]);
    }

    #[test]
    fn merge_rects_absorbs_a_contained_rect() {
        assert_eq!(merged(&[on(1, r(2, 2, 3, 3)), on(1, r(0, 0, 10, 10))]), [on(1, r(0, 0, 10, 10))]);
    }

    /// The union must be exactly one rectangle: a gap, an L or a step stays.
    #[test]
    fn merge_rects_keeps_non_rectangular_unions_apart() {
        for pair in [
            [on(1, r(0, 0, 10, 5)), on(1, r(11, 0, 10, 5))], // 1 nm gap
            [on(1, r(0, 0, 10, 5)), on(1, r(5, 0, 10, 8))],  // different heights
            [on(1, r(0, 0, 10, 5)), on(1, r(0, 5, 4, 5))],   // L
        ] {
            assert_eq!(merged(&pair).len(), 2, "{pair:?}");
        }
    }

    /// Other layers are untouched and keep their order; an empty list stays empty.
    #[test]
    fn merge_rects_leaves_other_layers() {
        let got = merged(&[on(2, r(0, 0, 1, 1)), on(1, r(0, 0, 10, 5)), on(3, r(5, 5, 1, 1)), on(1, r(10, 0, 5, 5))]);
        assert_eq!(got, [on(2, r(0, 0, 1, 1)), on(3, r(5, 5, 1, 1)), on(1, r(0, 0, 15, 5))]);
        assert!(merged(&[]).is_empty());
    }

    #[test]
    fn unreached_pins_floods_through_wires() {
        let pins = [r(0, 0, 2, 2), r(20, 0, 2, 2), r(50, 50, 1, 1)];
        // A wire bridging the first two, touching at edges.
        let wires = [on(1, r(2, 0, 18, 1))];
        assert_eq!(unreached_pins(&pins, &wires), [2]);
        assert!(unreached_pins(&[], &wires).is_empty());
        assert!(unreached_pins(&pins[..1], &[]).is_empty(), "one pin reaches itself");
        // Corner contact counts (closed intervals).
        assert!(unreached_pins(&[r(0, 0, 2, 2), r(2, 2, 2, 2)], &[]).is_empty());
    }

    fn pin(net: u16, at: Rect) -> pnr_core::Pin {
        pnr_core::Pin { name: String::new(), net: pnr_core::NetId(net), at, layer: LayerId(1) }
    }

    #[test]
    fn connected_nets_pass_the_debug_check() {
        let m = Macro { pins: vec![pin(0, r(0, 0, 2, 2)), pin(0, r(20, 0, 2, 2)), pin(1, r(90, 90, 1, 1))], ..Default::default() };
        let routes = Routes { wires: vec![vec![on(1, r(1, 0, 20, 1))]], ..Default::default() };
        debug_check_connected(&[m], &no_cells(), &routes);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "no routed geometry")]
    fn a_net_without_wires_fails_the_debug_check() {
        // Net 3 is past `routes.wires`: its pins still count.
        let m = Macro { pins: vec![pin(3, r(0, 0, 2, 2)), pin(3, r(20, 0, 2, 2))], ..Default::default() };
        debug_check_connected(&[m], &no_cells(), &Routes::default());
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "is open")]
    fn an_orphan_pin_fails_the_debug_check() {
        let m = Macro { pins: vec![pin(0, r(0, 0, 2, 2)), pin(0, r(20, 0, 2, 2))], ..Default::default() };
        let routes = Routes { wires: vec![vec![on(1, r(0, 0, 5, 1))]], ..Default::default() };
        debug_check_connected(&[m], &no_cells(), &routes);
    }

    /// Islands past the first per non-glue group; the glue (last) group, ids
    /// past the layout and duplicates never count.
    #[test]
    fn clusters_extra_counts_split_groups_only() {
        use pnr_core::DeviceId;
        let mut l = layout_of(Orient::R0, 0, 0);
        l.x = vec![100, 10_000, 300];
        l.y = vec![100; 3];
        l.hw = vec![100; 3];
        l.hh = vec![100; 3];
        l.orient = vec![Orient::R0; 3];
        l.variant = vec![0; 3];
        let g = |ids: &[u16]| ids.iter().map(|&d| DeviceId(d)).collect::<Vec<_>>();
        assert_eq!(clusters_extra(&l, &[], 10), 0);
        assert_eq!(clusters_extra(&l, &[g(&[0, 1])], 10), 0, "the only group is glue");
        assert_eq!(clusters_extra(&l, &[g(&[0, 1]), g(&[])], 10), 1);
        assert_eq!(clusters_extra(&l, &[g(&[0, 2]), g(&[])], 10), 0, "abutting cells are one island");
        assert_eq!(clusters_extra(&l, &[g(&[0, 0, 9]), g(&[])], 10), 0, "one distinct in-layout cell");
    }

    /// A non-positive lattice reads as 1 nm: nothing is off it.
    #[test]
    fn placement_metrics_with_no_lattice() {
        let mut l = layout_of(Orient::R0, 0, 0);
        l.x = vec![101];
        l.y = vec![103];
        l.hw = vec![100];
        l.hh = vec![100];
        let reqs = analog::Requirements::<Layout> { hard: vec![], budget: vec![], cost: vec![] };
        let locks = dp::locks::locks(&reqs, 1, &[]);
        let m = placement_metrics(&[Macro::default()], &l, 0, &gp::PlaceRules::uniform(100, 50), &reqs, &locks, &[]);
        assert_eq!((m.lattice_off, m.matched_geometry_mismatch, m.overlap_nm2), (0, 0, 0.0), "{m:?}");
    }
}
