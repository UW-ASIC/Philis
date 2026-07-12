//! DRC engine.
//!
//! Every rule is implemented as a transformation over the SoA `GeometryStore` producing a
//! flat `Vec<Violation>`. The public entry `run_drc` loops the resolved rule deck and
//! `match`es each `DrcRuleParam` (tagged-union dispatch, no vtables) to the geometric kernel
//! below. Each kernel is written as a plain function over borrowed slices so the *same body*
//! could be lowered to a GPU kernel (see `gpu` module) — the CPU path just runs it in a loop.
//!
//! Algorithm choices follow the research:
//!   * width / spacing / notch  -> edge-pair distance (scanline-pruned by bbox)
//!   * enclosure / overlap / ext -> layer-vs-layer edge/bbox relations
//!   * area                      -> shoelace
//!   * edge length / angle / grid-> per-edge / per-vertex predicates
//!   * density                   -> windowed coverage fraction

use crate::geometry::*;
use crate::traits::{self as gpu, Backend, VerifyCheck};
use crate::params::{Deck, DrcRuleParam, LayerTable};

/// A single rule violation. Flat, serializable, comparable against the manifest.
#[derive(Debug, Clone)]
pub struct Violation {
    pub rule_id: String,
    pub kind: String,
    pub layer: String,
    pub measured: i64,     // measured value in DBU (or -1 where N/A, e.g. angle)
    pub limit: i64,        // the rule limit
    pub x: i32,            // a representative location
    pub y: i32,
}

pub struct DrcReport {
    pub violations: Vec<Violation>,
}

impl DrcReport {
    pub fn by_kind(&self, kind: &str) -> Vec<&Violation> {
        self.violations.iter().filter(|v| v.kind == kind).collect()
    }
}

// --- Rule structs implementing VerifyCheck ---------------------------------

pub struct MinWidthRule { pub id: String, pub layer: LayerId, pub min: i32 }
pub struct MinSpacingRule { pub id: String, pub layer: LayerId, pub min: i32, pub strict: bool }
pub struct MinSpacingDiffRule { pub id: String, pub a: LayerId, pub b: LayerId, pub min: i32 }
pub struct MinEnclosureRule { pub id: String, pub outer: LayerId, pub inner: LayerId, pub min: i32 }
pub struct MinExtensionRule { pub id: String, pub layer: LayerId, pub reference: LayerId, pub min: i32 }
pub struct MinAreaRule { pub id: String, pub layer: LayerId, pub min: i64 }
pub struct MaxWidthRule { pub id: String, pub layer: LayerId, pub max: i32 }
pub struct NotchRule { pub id: String, pub layer: LayerId, pub min: i32 }
pub struct MinEdgeLengthRule { pub id: String, pub layer: LayerId, pub min: i32 }
pub struct OffGridRule { pub id: String, pub grid: i32 }
pub struct AngleRule { pub id: String, pub allowed: Vec<i32> }
pub struct DensityRule { pub id: String, pub layer: LayerId, pub window: i32, pub frac_limit: f64, pub is_min: bool }
pub struct OverlapRule { pub id: String, pub a: LayerId, pub b: LayerId, pub min: i32 }
pub struct CornerToCornerRule { pub id: String, pub layer: LayerId, pub min: i32 }
pub struct AntennaRule { pub id: String, pub layer: LayerId, pub ratio: f64 }
pub struct AntennaCarRule { pub id: String, pub layers: Vec<LayerId>, pub ratio: f64, pub diode: Option<LayerId> }
pub struct EolSpacingRule { pub id: String, pub layer: LayerId, pub eol_width: i32, pub eol_spacing: i32 }
pub struct WideDependentSpacingRule { pub id: String, pub layer: LayerId, pub width_threshold: i32, pub wide_spacing: i32 }
pub struct PrlSpacingRule { pub id: String, pub layer: LayerId, pub prl_threshold: i32, pub prl_spacing: i32 }
pub struct AsymmetricEnclosureRule { pub id: String, pub outer: LayerId, pub inner: LayerId, pub min_one_side: i32 }
pub struct MinEnclosedAreaRule { pub id: String, pub layer: LayerId, pub min_hole_area: i64 }
pub struct CheesingRule { pub id: String, pub layer: LayerId, pub max_area_no_slot: i64 }
pub struct RedundantViaRule { pub id: String, pub layer: LayerId, pub min_count: i32, pub within: i32 }
pub struct ViaArraySpacingRule { pub id: String, pub layer: LayerId, pub array_threshold: i32, pub array_spacing: i32 }
pub struct MaxDistanceToTapRule { pub id: String, pub diff_layer: LayerId, pub tap_layer: LayerId, pub max_dist: i32 }
pub struct MultiPatterningRule { pub id: String, pub layer: LayerId, pub num_colors: i32, pub color_spacing: i32 }

impl VerifyCheck for MinWidthRule {
    type Output = Vec<Violation>;
    fn id(&self) -> &str { &self.id }
    fn run(&self, store: &GeometryStore, deck: &Deck, backend: Backend) -> Vec<Violation> {
        let mut out = Vec::new();
        check_min_width(store, &deck.layers, self.layer, self.min, backend, &self.id, &mut out);
        out
    }
}

impl VerifyCheck for MinSpacingRule {
    type Output = Vec<Violation>;
    fn id(&self) -> &str { &self.id }
    fn run(&self, store: &GeometryStore, deck: &Deck, backend: Backend) -> Vec<Violation> {
        let mut out = Vec::new();
        check_spacing_same(store, &deck.layers, self.layer, self.min, backend, self.strict, &self.id, &mut out);
        out
    }
}

impl VerifyCheck for MinSpacingDiffRule {
    type Output = Vec<Violation>;
    fn id(&self) -> &str { &self.id }
    fn run(&self, store: &GeometryStore, deck: &Deck, backend: Backend) -> Vec<Violation> {
        let mut out = Vec::new();
        check_spacing_diff(store, &deck.layers, self.a, self.b, self.min, backend, &self.id, &mut out);
        out
    }
}

impl VerifyCheck for MinEnclosureRule {
    type Output = Vec<Violation>;
    fn id(&self) -> &str { &self.id }
    fn run(&self, store: &GeometryStore, deck: &Deck, backend: Backend) -> Vec<Violation> {
        let mut out = Vec::new();
        check_enclosure(store, &deck.layers, self.outer, self.inner, self.min, backend, &self.id, &mut out);
        out
    }
}

impl VerifyCheck for MinExtensionRule {
    type Output = Vec<Violation>;
    fn id(&self) -> &str { &self.id }
    fn run(&self, store: &GeometryStore, deck: &Deck, _backend: Backend) -> Vec<Violation> {
        let mut out = Vec::new();
        check_extension(store, &deck.layers, self.layer, self.reference, self.min, &self.id, &mut out);
        out
    }
}

impl VerifyCheck for MinAreaRule {
    type Output = Vec<Violation>;
    fn id(&self) -> &str { &self.id }
    fn run(&self, store: &GeometryStore, deck: &Deck, _backend: Backend) -> Vec<Violation> {
        let mut out = Vec::new();
        check_min_area(store, &deck.layers, self.layer, self.min, &self.id, &mut out);
        out
    }
}

impl VerifyCheck for MaxWidthRule {
    type Output = Vec<Violation>;
    fn id(&self) -> &str { &self.id }
    fn run(&self, store: &GeometryStore, deck: &Deck, _backend: Backend) -> Vec<Violation> {
        let mut out = Vec::new();
        check_max_width(store, &deck.layers, self.layer, self.max, &self.id, &mut out);
        out
    }
}

impl VerifyCheck for NotchRule {
    type Output = Vec<Violation>;
    fn id(&self) -> &str { &self.id }
    fn run(&self, store: &GeometryStore, deck: &Deck, backend: Backend) -> Vec<Violation> {
        let mut out = Vec::new();
        check_notch(store, &deck.layers, self.layer, self.min, backend, &self.id, &mut out);
        out
    }
}

impl VerifyCheck for MinEdgeLengthRule {
    type Output = Vec<Violation>;
    fn id(&self) -> &str { &self.id }
    fn run(&self, store: &GeometryStore, deck: &Deck, backend: Backend) -> Vec<Violation> {
        let mut out = Vec::new();
        check_min_edge_length(store, &deck.layers, self.layer, self.min, backend, &self.id, &mut out);
        out
    }
}

impl VerifyCheck for OffGridRule {
    type Output = Vec<Violation>;
    fn id(&self) -> &str { &self.id }
    fn run(&self, store: &GeometryStore, deck: &Deck, backend: Backend) -> Vec<Violation> {
        let mut out = Vec::new();
        check_off_grid(store, &deck.layers, self.grid, backend, &self.id, &mut out);
        out
    }
}

impl VerifyCheck for AngleRule {
    type Output = Vec<Violation>;
    fn id(&self) -> &str { &self.id }
    fn run(&self, store: &GeometryStore, deck: &Deck, backend: Backend) -> Vec<Violation> {
        let mut out = Vec::new();
        check_angle(store, &deck.layers, &self.allowed, backend, &self.id, &mut out);
        out
    }
}

impl VerifyCheck for DensityRule {
    type Output = Vec<Violation>;
    fn id(&self) -> &str { &self.id }
    fn run(&self, store: &GeometryStore, deck: &Deck, _backend: Backend) -> Vec<Violation> {
        let mut out = Vec::new();
        check_density(store, &deck.layers, self.layer, self.window, self.frac_limit, self.is_min, &self.id, &mut out);
        out
    }
}

impl VerifyCheck for OverlapRule {
    type Output = Vec<Violation>;
    fn id(&self) -> &str { &self.id }
    fn run(&self, store: &GeometryStore, deck: &Deck, _backend: Backend) -> Vec<Violation> {
        let mut out = Vec::new();
        check_overlap(store, &deck.layers, self.a, self.b, self.min, &self.id, &mut out);
        out
    }
}

impl VerifyCheck for CornerToCornerRule {
    type Output = Vec<Violation>;
    fn id(&self) -> &str { &self.id }
    fn run(&self, store: &GeometryStore, deck: &Deck, backend: Backend) -> Vec<Violation> {
        let mut out = Vec::new();
        check_corner_to_corner(store, &deck.layers, self.layer, self.min, backend, &self.id, &mut out);
        out
    }
}

impl VerifyCheck for AntennaRule {
    type Output = Vec<Violation>;
    fn id(&self) -> &str { &self.id }
    fn run(&self, store: &GeometryStore, deck: &Deck, _backend: Backend) -> Vec<Violation> {
        let mut out = Vec::new();
        check_antenna(store, deck, self.layer, self.ratio, &self.id, &mut out);
        out
    }
}

impl VerifyCheck for AntennaCarRule {
    type Output = Vec<Violation>;
    fn id(&self) -> &str { &self.id }
    fn run(&self, store: &GeometryStore, deck: &Deck, _backend: Backend) -> Vec<Violation> {
        let mut out = Vec::new();
        check_antenna_car(store, deck, &self.layers, self.ratio, self.diode, &self.id, &mut out);
        out
    }
}

impl VerifyCheck for EolSpacingRule {
    type Output = Vec<Violation>;
    fn id(&self) -> &str { &self.id }
    fn run(&self, store: &GeometryStore, deck: &Deck, backend: Backend) -> Vec<Violation> {
        let mut out = Vec::new();
        check_eol_spacing(store, &deck.layers, self.layer, self.eol_width, self.eol_spacing, backend, &self.id, &mut out);
        out
    }
}

impl VerifyCheck for WideDependentSpacingRule {
    type Output = Vec<Violation>;
    fn id(&self) -> &str { &self.id }
    fn run(&self, store: &GeometryStore, deck: &Deck, _backend: Backend) -> Vec<Violation> {
        let mut out = Vec::new();
        check_wide_dependent_spacing(store, &deck.layers, self.layer, self.width_threshold, self.wide_spacing, &self.id, &mut out);
        out
    }
}

impl VerifyCheck for PrlSpacingRule {
    type Output = Vec<Violation>;
    fn id(&self) -> &str { &self.id }
    fn run(&self, store: &GeometryStore, deck: &Deck, _backend: Backend) -> Vec<Violation> {
        let mut out = Vec::new();
        check_prl_spacing(store, &deck.layers, self.layer, self.prl_threshold, self.prl_spacing, &self.id, &mut out);
        out
    }
}

impl VerifyCheck for AsymmetricEnclosureRule {
    type Output = Vec<Violation>;
    fn id(&self) -> &str { &self.id }
    fn run(&self, store: &GeometryStore, deck: &Deck, _backend: Backend) -> Vec<Violation> {
        let mut out = Vec::new();
        check_asymmetric_enclosure(store, &deck.layers, self.outer, self.inner, self.min_one_side, &self.id, &mut out);
        out
    }
}

impl VerifyCheck for MinEnclosedAreaRule {
    type Output = Vec<Violation>;
    fn id(&self) -> &str { &self.id }
    fn run(&self, store: &GeometryStore, deck: &Deck, _backend: Backend) -> Vec<Violation> {
        let mut out = Vec::new();
        check_min_enclosed_area(store, &deck.layers, self.layer, self.min_hole_area, &self.id, &mut out);
        out
    }
}

impl VerifyCheck for CheesingRule {
    type Output = Vec<Violation>;
    fn id(&self) -> &str { &self.id }
    fn run(&self, store: &GeometryStore, deck: &Deck, _backend: Backend) -> Vec<Violation> {
        let mut out = Vec::new();
        check_cheesing(store, &deck.layers, self.layer, self.max_area_no_slot, &self.id, &mut out);
        out
    }
}

impl VerifyCheck for RedundantViaRule {
    type Output = Vec<Violation>;
    fn id(&self) -> &str { &self.id }
    fn run(&self, store: &GeometryStore, deck: &Deck, _backend: Backend) -> Vec<Violation> {
        let mut out = Vec::new();
        check_redundant_via(store, &deck.layers, self.layer, self.min_count, self.within, &self.id, &mut out);
        out
    }
}

impl VerifyCheck for ViaArraySpacingRule {
    type Output = Vec<Violation>;
    fn id(&self) -> &str { &self.id }
    fn run(&self, store: &GeometryStore, deck: &Deck, _backend: Backend) -> Vec<Violation> {
        let mut out = Vec::new();
        check_via_array_spacing(store, &deck.layers, self.layer, self.array_threshold, self.array_spacing, &self.id, &mut out);
        out
    }
}

impl VerifyCheck for MaxDistanceToTapRule {
    type Output = Vec<Violation>;
    fn id(&self) -> &str { &self.id }
    fn run(&self, store: &GeometryStore, deck: &Deck, _backend: Backend) -> Vec<Violation> {
        let mut out = Vec::new();
        check_max_distance_to_tap(store, &deck.layers, self.diff_layer, self.tap_layer, self.max_dist, &self.id, &mut out);
        out
    }
}

impl VerifyCheck for MultiPatterningRule {
    type Output = Vec<Violation>;
    fn id(&self) -> &str { &self.id }
    fn run(&self, store: &GeometryStore, deck: &Deck, _backend: Backend) -> Vec<Violation> {
        let mut out = Vec::new();
        check_multi_patterning(store, &deck.layers, self.layer, self.num_colors, self.color_spacing, &self.id, &mut out);
        out
    }
}

/// Build boxed trait objects from the deck's DRC rule parameters.
pub fn drc_rules_from_deck(
    deck: &Deck, strict: bool,
) -> Vec<Box<dyn VerifyCheck<Output = Vec<Violation>> + Send + Sync>> {
    deck.drc_rules.iter().map(|p| -> Box<dyn VerifyCheck<Output = Vec<Violation>> + Send + Sync> {
        match p {
            DrcRuleParam::MinWidth { id, layer, min } =>
                Box::new(MinWidthRule { id: id.clone(), layer: *layer, min: *min }),
            DrcRuleParam::MinSpacing { id, layer, min } =>
                Box::new(MinSpacingRule { id: id.clone(), layer: *layer, min: *min, strict }),
            DrcRuleParam::MinSpacingDiff { id, a, b, min } =>
                Box::new(MinSpacingDiffRule { id: id.clone(), a: *a, b: *b, min: *min }),
            DrcRuleParam::MinEnclosure { id, outer, inner, min } =>
                Box::new(MinEnclosureRule { id: id.clone(), outer: *outer, inner: *inner, min: *min }),
            DrcRuleParam::MinExtension { id, layer, reference, min } =>
                Box::new(MinExtensionRule { id: id.clone(), layer: *layer, reference: *reference, min: *min }),
            DrcRuleParam::MinArea { id, layer, min } =>
                Box::new(MinAreaRule { id: id.clone(), layer: *layer, min: *min }),
            DrcRuleParam::MaxWidth { id, layer, max } =>
                Box::new(MaxWidthRule { id: id.clone(), layer: *layer, max: *max }),
            DrcRuleParam::Notch { id, layer, min } =>
                Box::new(NotchRule { id: id.clone(), layer: *layer, min: *min }),
            DrcRuleParam::MinEdgeLength { id, layer, min } =>
                Box::new(MinEdgeLengthRule { id: id.clone(), layer: *layer, min: *min }),
            DrcRuleParam::OffGrid { id, grid } =>
                Box::new(OffGridRule { id: id.clone(), grid: *grid }),
            DrcRuleParam::Angle { id, allowed } =>
                Box::new(AngleRule { id: id.clone(), allowed: allowed.clone() }),
            DrcRuleParam::MinDensity { id, layer, window, min_frac } =>
                Box::new(DensityRule { id: id.clone(), layer: *layer, window: *window, frac_limit: *min_frac, is_min: true }),
            DrcRuleParam::MaxDensity { id, layer, window, max_frac } =>
                Box::new(DensityRule { id: id.clone(), layer: *layer, window: *window, frac_limit: *max_frac, is_min: false }),
            DrcRuleParam::Overlap { id, a, b, min } =>
                Box::new(OverlapRule { id: id.clone(), a: *a, b: *b, min: *min }),
            DrcRuleParam::CornerToCorner { id, layer, min } =>
                Box::new(CornerToCornerRule { id: id.clone(), layer: *layer, min: *min }),
            DrcRuleParam::Antenna { id, layer, ratio } =>
                Box::new(AntennaRule { id: id.clone(), layer: *layer, ratio: *ratio }),
            DrcRuleParam::AntennaCar { id, layers, ratio, diode } =>
                Box::new(AntennaCarRule { id: id.clone(), layers: layers.clone(), ratio: *ratio, diode: *diode }),
            DrcRuleParam::EolSpacing { id, layer, eol_width, eol_spacing } =>
                Box::new(EolSpacingRule { id: id.clone(), layer: *layer, eol_width: *eol_width, eol_spacing: *eol_spacing }),
            DrcRuleParam::WideDependentSpacing { id, layer, width_threshold, wide_spacing } =>
                Box::new(WideDependentSpacingRule { id: id.clone(), layer: *layer, width_threshold: *width_threshold, wide_spacing: *wide_spacing }),
            DrcRuleParam::PrlSpacing { id, layer, prl_threshold, prl_spacing } =>
                Box::new(PrlSpacingRule { id: id.clone(), layer: *layer, prl_threshold: *prl_threshold, prl_spacing: *prl_spacing }),
            DrcRuleParam::AsymmetricEnclosure { id, outer, inner, min_one_side } =>
                Box::new(AsymmetricEnclosureRule { id: id.clone(), outer: *outer, inner: *inner, min_one_side: *min_one_side }),
            DrcRuleParam::MinEnclosedArea { id, layer, min_hole_area } =>
                Box::new(MinEnclosedAreaRule { id: id.clone(), layer: *layer, min_hole_area: *min_hole_area }),
            DrcRuleParam::Cheesing { id, layer, max_area_no_slot } =>
                Box::new(CheesingRule { id: id.clone(), layer: *layer, max_area_no_slot: *max_area_no_slot }),
            DrcRuleParam::RedundantVia { id, layer, min_count, within } =>
                Box::new(RedundantViaRule { id: id.clone(), layer: *layer, min_count: *min_count, within: *within }),
            DrcRuleParam::ViaArraySpacing { id, layer, array_threshold, array_spacing } =>
                Box::new(ViaArraySpacingRule { id: id.clone(), layer: *layer, array_threshold: *array_threshold, array_spacing: *array_spacing }),
            DrcRuleParam::MaxDistanceToTap { id, diff_layer, tap_layer, max_dist } =>
                Box::new(MaxDistanceToTapRule { id: id.clone(), diff_layer: *diff_layer, tap_layer: *tap_layer, max_dist: *max_dist }),
            DrcRuleParam::MultiPatterning { id, layer, num_colors, color_spacing } =>
                Box::new(MultiPatterningRule { id: id.clone(), layer: *layer, num_colors: *num_colors, color_spacing: *color_spacing }),
        }
    }).collect()
}

/// Run the whole deck against the store. Restricting to a set of polygons (one cell) is done
/// by the caller building a store that only contains that cell.
pub fn run_drc(store: &GeometryStore, deck: &Deck) -> DrcReport {
    run_drc_backend(store, deck, Backend::Cpu)
}

/// Same as [`run_drc`], with an explicit backend. `Backend::Gpu` uses the CUDA edge-pair
/// prefilter for the spacing scans when a device is available; the report is identical to
/// the CPU one either way (see `gpu` module contract).
pub fn run_drc_backend(store: &GeometryStore, deck: &Deck, backend: Backend) -> DrcReport {
    run_drc_backend_strict(store, deck, backend, deck.strict)
}

pub fn run_drc_backend_strict(store: &GeometryStore, deck: &Deck, backend: Backend, strict: bool) -> DrcReport {
    run_drc_impl(store, deck, backend, strict, |_| true)
}

/// In-loop variant: density rules skipped. The engine's convergence loop always
/// waives density (whole-die density is meaningless mid-iteration), so computing
/// the window clips every iteration is a full-geometry pass of pure waste.
pub fn run_drc_no_density(store: &GeometryStore, deck: &Deck) -> DrcReport {
    run_drc_impl(store, deck, Backend::Cpu, deck.strict, |r| {
        !matches!(r, DrcRuleParam::MinDensity { .. } | DrcRuleParam::MaxDensity { .. })
    })
}

fn run_drc_impl(
    store: &GeometryStore,
    deck: &Deck,
    backend: Backend,
    strict: bool,
    keep: impl Fn(&DrcRuleParam) -> bool + Sync,
) -> DrcReport {
    use rayon::prelude::*;
    let rules = drc_rules_from_deck(deck, strict);
    // Rules are independent read-only scans over the store; run them in parallel
    // and concatenate in rule order so the report is deterministic.
    // `rules` is 1:1 with `deck.drc_rules` (see drc_rules_from_deck's map).
    let per_rule: Vec<Vec<Violation>> = rules
        .par_iter()
        .zip(deck.drc_rules.par_iter())
        .filter(|(_, p)| keep(p))
        .map(|(rule, _)| rule.run(store, deck, backend))
        .collect();
    // Always-on input validity: downstream checks assume simple polygons, so a
    // boundary that properly crosses itself is reported, not silently mis-measured.
    let mut violations = check_polygon_validity(store, &deck.layers);
    violations.extend(per_rule.into_iter().flatten());
    // Coincident identical polygons (e.g. two pins of one device sharing a pad)
    // are one merged shape in real DRC; each copy reports the same violation.
    // Exact duplicates are double counts, never two distinct defects.
    violations.sort_unstable_by(|a, b| {
        (&a.rule_id, &a.kind, &a.layer, a.measured, a.limit, a.x, a.y)
            .cmp(&(&b.rule_id, &b.kind, &b.layer, b.measured, b.limit, b.x, b.y))
    });
    violations.dedup_by(|a, b| {
        a.rule_id == b.rule_id && a.kind == b.kind && a.layer == b.layer
            && a.measured == b.measured && a.limit == b.limit && a.x == b.x && a.y == b.y
    });
    DrcReport { violations }
}

/// Parse-don't-validate at the geometry boundary: every polygon is scanned once,
/// on load, for the degeneracies the rule kernels are not defined over —
/// self-crossing boundaries and zero-area shapes. Keyhole slits (legal GDS holes)
/// pass; proper bow-tie crossings do not.
fn check_polygon_validity(store: &GeometryStore, lt: &LayerTable) -> Vec<Violation> {
    let mut out = Vec::new();
    for p in 0..store.poly_count() {
        let pid = PolyId(p as u32);
        let bb = store.poly_bbox[p];
        let issue = if poly_self_intersects(store, pid) {
            Some("self_intersecting")
        } else if store.area(pid) == 0 {
            Some("zero_area")
        } else {
            None
        };
        if let Some(kind_detail) = issue {
            out.push(Violation {
                rule_id: "polygon_validity".into(), kind: "polygon_validity".into(),
                layer: lt.name(store.poly_layer[p]).into(),
                measured: if kind_detail == "zero_area" { 0 } else { 1 },
                limit: 0, x: bb.xmin, y: bb.ymin,
            });
        }
    }
    out
}

// --- width ------------------------------------------------------------------
// For axis-aligned polygons, width = min(bbox.width, bbox.height) is exact for convex
// rectangles; for general rectilinear polygons we additionally scan opposing parallel edges.
// The conformance width cases are rectangles, so the bbox measure is exact; we still emit
// the opposing-edge scan for robustness on non-convex shapes.
fn check_min_width(
    store: &GeometryStore, lt: &LayerTable, layer: LayerId, min: i32, backend: Backend,
    rule_id: &str, out: &mut Vec<Violation>,
) {
    let polys = store.polys_on_layer(layer);
    let clean = gpu_poly_clean_mask(store, &polys, min, backend);
    for (k, &p) in polys.iter().enumerate() {
        let bb = store.poly_bbox[p.0 as usize];
        let (s, e) = store.poly_range(p);
        let n = e - s;
        // bbox fast path is exact ONLY for axis-aligned rectangles; a rotated
        // parallelogram has 4 vertices too and must take the facing-gap scan.
        let axis_aligned_rect = n == 4 && (0..4).all(|i| {
            let (x0, y0) = store.poly_vertex(s, i);
            let (x1, y1) = store.poly_vertex(s, (i + 1) % 4);
            x0 == x1 || y0 == y1
        });
        if axis_aligned_rect {
            let w = bb.width().min(bb.height());
            if w < min {
                out.push(Violation {
                    rule_id: rule_id.into(), kind: "min_width".into(),
                    layer: lt.name(layer).into(), measured: w as i64, limit: min as i64,
                    x: bb.xmin, y: bb.ymin,
                });
            }
            continue;
        }
        if clean.as_ref().is_some_and(|c| c[k]) { continue; } // GPU: no gap anywhere near min
        // Rectilinear: one violation per narrow INTERIOR facing gap — an L with two thin
        // arms is two violation sites, not one. Exterior gaps (notches) are excluded;
        // counting them as widths is the classic false positive naive edge scans produce
        // on U-shapes.
        for (d, mx, my) in facing_gaps(store, p, true) {
            if d < min {
                out.push(Violation {
                    rule_id: rule_id.into(), kind: "min_width".into(),
                    layer: lt.name(layer).into(), measured: d as i64, limit: min as i64,
                    x: mx, y: my,
                });
            }
        }
    }
}

/// Scan facing parallel edge pairs of one polygon; return (distance, mid_x, mid_y) for
/// each pair with positive overlap span. `interior` selects pairs whose gap midpoint lies
/// inside the polygon (width measurements) vs outside (notch gaps). This midpoint
/// disambiguation is what stops a U-shape's notch from being reported as a narrow width
/// and its arm widths from being reported as notches.
// ponytail: 1-dbu gaps have no strict-interior sample point and classify as exterior;
// far below any real rule limit, so accepted.
fn facing_gaps(store: &GeometryStore, p: PolyId, interior: bool) -> Vec<(i32, i32, i32)> {
    let edges = poly_edges(store, p);
    let n = edges.len();
    let mut out = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            let a = &edges[i];
            let b = &edges[j];
            let (d, mx, my) = if a.is_vertical() && b.is_vertical() {
                let ylo = a.y0.min(a.y1).max(b.y0.min(b.y1));
                let yhi = a.y0.max(a.y1).min(b.y0.max(b.y1));
                if ylo >= yhi { continue; }
                ((a.x0 - b.x0).abs(), (a.x0 + b.x0) / 2, (ylo + yhi) / 2)
            } else if a.is_horizontal() && b.is_horizontal() {
                let xlo = a.x0.min(a.x1).max(b.x0.min(b.x1));
                let xhi = a.x0.max(a.x1).min(b.x0.max(b.x1));
                if xlo >= xhi { continue; }
                ((a.y0 - b.y0).abs(), (xlo + xhi) / 2, (a.y0 + b.y0) / 2)
            } else {
                // parallel diagonal edges (45° routing): perpendicular gap where
                // the edges overlap tangentially. Integer where it matters,
                // f64 only for the sample-point coordinates.
                let (adx, ady) = (a.dx() as i64, a.dy() as i64);
                let (bdx, bdy) = (b.dx() as i64, b.dy() as i64);
                if adx * bdy - ady * bdx != 0 { continue; } // not parallel
                let len2 = adx * adx + ady * ady;
                if len2 == 0 { continue; }
                let t = |px: i64, py: i64| (px - a.x0 as i64) * adx + (py - a.y0 as i64) * ady;
                let (tb0, tb1) = (t(b.x0 as i64, b.y0 as i64), t(b.x1 as i64, b.y1 as i64));
                let lo = tb0.min(tb1).max(0);
                let hi = tb0.max(tb1).min(len2);
                if lo >= hi { continue; } // no tangential overlap
                // signed offset of b's line along a's left normal (−ady, adx): d×w
                let cross = adx * (b.y0 as i64 - a.y0 as i64) - ady * (b.x0 as i64 - a.x0 as i64);
                let d = isqrt(cross * cross / len2) as i32;
                // sample point: middle of the overlap span, halfway between the lines
                let tm = (lo + hi) as f64 / 2.0 / len2 as f64;
                let half = cross as f64 / 2.0 / len2 as f64;
                let mx = (a.x0 as f64 + adx as f64 * tm - ady as f64 * half).round() as i32;
                let my = (a.y0 as f64 + ady as f64 * tm + adx as f64 * half).round() as i32;
                (d, mx, my)
            };
            if d == 0 { continue; }
            if point_in_poly(store, p, mx, my) == interior {
                out.push((d, mx, my));
            }
        }
    }
    out
}

/// Is polygon `inner` strictly inside polygon `outer`? All vertices strictly interior —
/// exact for the disjoint-boundary cases this is used on (a touching boundary counts as
/// "not strictly inside", which is the conservative answer for both callers).
fn poly_strictly_inside(store: &GeometryStore, inner: PolyId, outer: PolyId) -> bool {
    let (s, e) = store.poly_range(inner);
    (s..e).all(|i| point_in_poly(store, outer, store.verts_x[i], store.verts_y[i]))
}

/// Enumerate polygon pairs whose bboxes come within `min` of each other, by sweep:
/// sort along one axis, only look ahead while ranges can still be close, filter on
/// the other axis. O(P log P + K) — the candidate generator every pairwise rule shares.
///
/// The sweep axis is chosen by bbox-min spread: sweeping the axis where shapes are
/// spread out keeps the look-ahead window small. Hardcoding x degenerates to O(P²)
/// on layouts of full-width horizontal bars (every xmin equal → window = everything).
/// `pb = None` => all unordered pairs within `pa`. `pb = Some(..)` => cross pairs only.
pub(crate) fn candidate_pairs(
    store: &GeometryStore, pa: &[PolyId], pb: Option<&[PolyId]>, min: i32,
) -> Vec<(PolyId, PolyId)> {
    // pick the sweep axis with the larger min-coordinate spread
    let mut xlo = i32::MAX; let mut xhi = i32::MIN;
    let mut ylo = i32::MAX; let mut yhi = i32::MIN;
    for &p in pa.iter().chain(pb.unwrap_or(&[])) {
        let b = store.poly_bbox[p.0 as usize];
        xlo = xlo.min(b.xmin); xhi = xhi.max(b.xmin);
        ylo = ylo.min(b.ymin); yhi = yhi.max(b.ymin);
    }
    let sweep_x = (xhi.saturating_sub(xlo)) >= (yhi.saturating_sub(ylo));

    // (sweep_min, sweep_max, poly, from_b)
    let mut items: Vec<(i32, i32, PolyId, bool)> = Vec::with_capacity(
        pa.len() + pb.map_or(0, |b| b.len()));
    let key = |b: &Bbox| if sweep_x { (b.xmin, b.xmax) } else { (b.ymin, b.ymax) };
    for &p in pa {
        let (lo, hi) = key(&store.poly_bbox[p.0 as usize]);
        items.push((lo, hi, p, false));
    }
    for &p in pb.unwrap_or(&[]) {
        let (lo, hi) = key(&store.poly_bbox[p.0 as usize]);
        items.push((lo, hi, p, true));
    }
    items.sort_unstable_by_key(|it| it.0);
    let mut out = Vec::new();
    for i in 0..items.len() {
        let (_, hi_i, pi, bi) = items[i];
        for &(_, _, pj, bj) in items[i + 1..].iter()
            .take_while(|it| it.0 <= hi_i.saturating_add(min))
        {
            if pb.is_some() && bi == bj { continue; } // cross-set pairs only
            let ba = store.poly_bbox[pi.0 as usize];
            let bb = store.poly_bbox[pj.0 as usize];
            let other_near = if sweep_x {
                ba.ymin - min <= bb.ymax && bb.ymin - min <= ba.ymax
            } else {
                ba.xmin - min <= bb.xmax && bb.xmin - min <= ba.xmax
            };
            if other_near {
                // keep (set A, set B) order for cross-set queries
                if bi { out.push((pj, pi)) } else { out.push((pi, pj)) }
            }
        }
    }
    out
}

// --- same-layer merge groups --------------------------------------------------
/// Union-find over a layer's polygons where touching/overlapping polys (distance
/// 0, directly or transitively) form one merged shape. Real DRC merges before
/// checking, so a sub-min gap between parts of one merged shape is not a
/// SPACING violation — it is a NOTCH of the merged compound, and
/// check_spacing_same reports it as such so nothing escapes.
fn merge_groups(
    store: &GeometryStore, cands: &[(PolyId, PolyId)], far: Option<&Vec<bool>>, n_polys: usize,
    idx_of: &std::collections::HashMap<u32, u32>,
) -> Vec<u32> {
    let mut parent: Vec<u32> = (0..n_polys as u32).collect();
    fn find(parent: &mut [u32], x: u32) -> u32 {
        let mut r = x;
        while parent[r as usize] != r {
            parent[r as usize] = parent[parent[r as usize] as usize];
            r = parent[r as usize];
        }
        r
    }
    for (k, &(pa, pb)) in cands.iter().enumerate() {
        if far.is_some_and(|f| f[k]) { continue; } // clearly apart: cannot touch
        if poly_poly_dist2_within(store, pa, pb, 1) == 0 {
            let (ia, ib) = (idx_of[&pa.0], idx_of[&pb.0]);
            let (ra, rb) = (find(&mut parent, ia), find(&mut parent, ib));
            if ra != rb { parent[ra as usize] = rb; }
        }
    }
    (0..n_polys as u32).map(|i| find(&mut parent, i)).collect()
}

/// The open region between two near bboxes: the facing band when they overlap
/// on one axis, else the box between nearest corners.
fn gap_rect(a: Bbox, b: Bbox) -> Bbox {
    let ix0 = a.xmin.max(b.xmin);
    let ix1 = a.xmax.min(b.xmax);
    let iy0 = a.ymin.max(b.ymin);
    let iy1 = a.ymax.min(b.ymax);
    if ix1 > ix0 {
        // x-overlap, gap in y
        Bbox { xmin: ix0, ymin: a.ymax.min(b.ymax), xmax: ix1, ymax: a.ymin.max(b.ymin) }
    } else if iy1 > iy0 {
        Bbox { xmin: a.xmax.min(b.xmax), ymin: iy0, xmax: a.xmin.max(b.xmin), ymax: iy1 }
    } else {
        // diagonal: box between nearest corners
        Bbox {
            xmin: a.xmax.min(b.xmax),
            ymin: a.ymax.min(b.ymax),
            xmax: a.xmin.max(b.xmin),
            ymax: a.ymin.max(b.ymin),
        }
    }
}

/// True if a single OTHER polygon (rectangle) on the layer covers the whole
/// gap region between `pa` and `pb` — the merged shape is solid there.
fn gap_region_covered(store: &GeometryStore, polys: &[PolyId], pa: PolyId, pb: PolyId) -> bool {
    let g = gap_rect(store.poly_bbox[pa.0 as usize], store.poly_bbox[pb.0 as usize]);
    polys.iter().any(|&p| {
        if p == pa || p == pb {
            return false;
        }
        // rectangle covers == bbox covers; restrict to 4-vertex polys so an
        // L-shape's bbox cannot fake coverage
        let (s, e) = store.poly_range(p);
        if e - s != 4 {
            return false;
        }
        let b = store.poly_bbox[p.0 as usize];
        b.xmin <= g.xmin && b.ymin <= g.ymin && b.xmax >= g.xmax && b.ymax >= g.ymax
    })
}

/// Gap boxes of same-shape sub-min gaps (notches). Both sides belong to one
/// merged same-net shape, so filling the gap with metal is always electrically
/// safe — used as a post-merge DRC repair (pad-row corner slivers etc.).
pub fn same_shape_gap_fills(store: &GeometryStore, deck: &Deck) -> Vec<(LayerId, Bbox)> {
    let mut fills = Vec::new();
    for r in &deck.drc_rules {
        let DrcRuleParam::MinSpacing { layer, min, .. } = r else { continue };
        let (layer, min) = (*layer, *min);
        let polys = store.polys_on_layer(layer);
        let min2 = (min as i64) * (min as i64);
        let cands = candidate_pairs(store, &polys, None, min);
        let idx_of: std::collections::HashMap<u32, u32> =
            polys.iter().enumerate().map(|(i, p)| (p.0, i as u32)).collect();
        let group = merge_groups(store, &cands, None, polys.len(), &idx_of);
        for &(pa, pb) in &cands {
            let d2 = poly_poly_dist2_within(store, pa, pb, min);
            if d2 == 0 || d2 >= min2 { continue; }
            if poly_strictly_inside(store, pa, pb) || poly_strictly_inside(store, pb, pa) {
                continue;
            }
            if group[idx_of[&pa.0] as usize] != group[idx_of[&pb.0] as usize] {
                continue; // different nets — not fillable
            }
            if gap_region_covered(store, &polys, pa, pb) {
                continue; // already solid
            }
            let g = gap_rect(store.poly_bbox[pa.0 as usize], store.poly_bbox[pb.0 as usize]);
            if g.xmax > g.xmin && g.ymax > g.ymin {
                // Inflate by min/2 so the fill overlaps both sides and is
                // itself min_width-clean; stays inside the pair's hull, where
                // foreign metal would already be a spacing violation.
                let h = min / 2;
                fills.push((layer, Bbox {
                    xmin: g.xmin - h, ymin: g.ymin - h,
                    xmax: g.xmax + h, ymax: g.ymax + h,
                }));
            }
        }
    }
    fills
}

// --- same-layer spacing (external) ------------------------------------------
fn check_spacing_same(
    store: &GeometryStore, lt: &LayerTable, layer: LayerId, min: i32, backend: Backend,
    strict: bool, rule_id: &str, out: &mut Vec<Violation>,
) {
    let polys = store.polys_on_layer(layer);
    let min2 = (min as i64) * (min as i64);
    let cands = candidate_pairs(store, &polys, None, min);
    let far = gpu_far_mask(store, &cands, min, backend);
    let idx_of: std::collections::HashMap<u32, u32> =
        polys.iter().enumerate().map(|(i, p)| (p.0, i as u32)).collect();
    let group = merge_groups(store, &cands, far.as_ref(), polys.len(), &idx_of);
    for (k, &(pa, pb)) in cands.iter().enumerate() {
        if far.as_ref().is_some_and(|f| f[k]) { continue; } // GPU-cleared: clearly >= min
        let ba = store.poly_bbox[pa.0 as usize];
        // NOTE: overlapping *bboxes* do NOT mean the shapes touch (interlocking L
        // shapes). Only actual contact — direct or through a chain of touching
        // polygons (merge_groups) — makes the pair one merged shape.
        let d2 = poly_poly_dist2_within(store, pa, pb, min);
        if d2 == 0 { continue; } // abutting/crossing => merged shape
        if d2 < min2 {
            if poly_strictly_inside(store, pa, pb) || poly_strictly_inside(store, pb, pa) {
                continue; // hole/island of the same layer, merge semantics
            }
            // within one merged shape a sub-min gap is a notch of the
            // compound, not external spacing — and only if the gap region is
            // actually EXPOSED: a third polygon covering it makes the merged
            // shape solid there (bridge blocks over pad pairs).
            let same_shape = group[idx_of[&pa.0] as usize] == group[idx_of[&pb.0] as usize];
            if same_shape && gap_region_covered(store, &polys, pa, pb) {
                continue;
            }
            let d = isqrt(d2);
            // In STRICT mode, same-net gaps are spacing violations too
            let kind = if same_shape && !strict { "notch" } else { "min_spacing" };
            if std::env::var("PNR_DEBUG_NOTCH").is_ok() {
                let bb = store.poly_bbox[pb.0 as usize];
                let (sa, ea) = store.poly_range(pa);
                let (sb, eb) = store.poly_range(pb);
                eprintln!("[notch-dbg] {kind} d={d} pa=({},{},{},{})v{} pb=({},{},{},{})v{} same_shape={same_shape}",
                    ba.xmin, ba.ymin, ba.xmax, ba.ymax, ea - sa,
                    bb.xmin, bb.ymin, bb.xmax, bb.ymax, eb - sb);
            }
            out.push(Violation {
                rule_id: rule_id.into(),
                kind: kind.into(),
                layer: lt.name(layer).into(), measured: d, limit: min as i64,
                x: ba.xmax, y: ba.ymin,
            });
        }
    }
}

// --- different-layer spacing ------------------------------------------------
fn check_spacing_diff(
    store: &GeometryStore, lt: &LayerTable, a: LayerId, b: LayerId, min: i32, backend: Backend,
    rule_id: &str, out: &mut Vec<Violation>,
) {
    let pas = store.polys_on_layer(a);
    let pbs = store.polys_on_layer(b);
    let min2 = (min as i64) * (min as i64);
    let cands = candidate_pairs(store, &pas, Some(&pbs), min);
    let far = gpu_far_mask(store, &cands, min, backend);
    for (k, &(pa, pb)) in cands.iter().enumerate() {
        if far.as_ref().is_some_and(|f| f[k]) { continue; }
        let ba = store.poly_bbox[pa.0 as usize];
        let d2 = poly_poly_dist2_within(store, pa, pb, min);
        if d2 == 0 { continue; } // touching/crossing layers: not a spacing pair
        if d2 < min2 {
            if poly_strictly_inside(store, pa, pb) || poly_strictly_inside(store, pb, pa) {
                continue;
            }
            out.push(Violation {
                rule_id: rule_id.into(), kind: "min_spacing_diff".into(),
                layer: format!("{}:{}", lt.name(a), lt.name(b)),
                measured: isqrt(d2), limit: min as i64, x: ba.xmax, y: ba.ymin,
            });
        }
    }
}

/// Below this many device-side evaluations the CPU finishes before a GPU round trip
/// even starts — the mask builders decline (return None) and the exact path runs.
/// This is what makes `Backend::Gpu` always-safe: dense scans go to the GPU, sparse
/// ones stay on the CPU, per rule and per layer.
// ponytail: fixed break-even from RTX 4060 measurements; make it a Deck knob if a
// wildly different GPU/CPU pairing ever needs tuning.
const GPU_MIN_PAIR_WORK: u64 = 1 << 18;
const GPU_MIN_LINEAR_WORK: usize = 1 << 20;

/// GPU prefilter: for each candidate polygon pair, true iff EVERY edge-pair distance is
/// comfortably above the rule limit — such pairs can only produce "no violation" on the
/// exact path, so they are safe to skip. None => no GPU; run everything exactly.
fn gpu_far_mask(
    store: &GeometryStore, cands: &[(PolyId, PolyId)], min: i32, backend: Backend,
) -> Option<Vec<bool>> {
    if backend != Backend::Gpu || cands.is_empty() { return None; }
    // unique edge pool: each polygon's edges are materialized (and uploaded) exactly once;
    // the edge-pair cross product itself is enumerated on the device (see gpu::pair_near)
    let mut edges: Vec<Edge> = Vec::new();
    let mut range: std::collections::HashMap<u32, (u32, u32)> = std::collections::HashMap::new();
    for &(pa, pb) in cands {
        for p in [pa, pb] {
            if !range.contains_key(&p.0) {
                let s = edges.len() as u32;
                edges.extend(poly_edges(store, p));
                range.insert(p.0, (s, edges.len() as u32));
            }
        }
    }
    let descs: Vec<(u32, u32, u32, u32)> = cands.iter().map(|&(pa, pb)| {
        let (a0, a1) = range[&pa.0];
        let (b0, b1) = range[&pb.0];
        (a0, a1, b0, b1)
    }).collect();
    let work: u64 = descs.iter()
        .map(|&(a0, a1, b0, b1)| ((a1 - a0) as u64) * ((b1 - b0) as u64)).sum();
    if work < GPU_MIN_PAIR_WORK { return None; } // CPU finishes first
    // margin over the f32 approximation; anything near the limit is exact-rechecked
    let thr2 = (min as f32) * (min as f32) * 1.05 + 4.0;
    let flags = gpu::pair_near_flags(&edges, &descs, thr2)?;
    Some(flags.into_iter().map(|f| f == 0).collect())
}

/// GPU prefilter for the same-polygon facing-gap scans (width/notch): true per polygon
/// iff no facing edge pair is anywhere near `min` — such polygons can skip the exact
/// interior/exterior scan entirely. None => no GPU; scan everything exactly.
fn gpu_poly_clean_mask(
    store: &GeometryStore, polys: &[PolyId], min: i32, backend: Backend,
) -> Option<Vec<bool>> {
    if backend != Backend::Gpu || polys.is_empty() { return None; }
    let mut edges: Vec<Edge> = Vec::new();
    let mut descs = Vec::with_capacity(polys.len());
    for &p in polys {
        let s = edges.len() as u32;
        edges.extend(poly_edges(store, p));
        descs.push((s, edges.len() as u32));
    }
    let work: u64 = descs.iter().map(|&(s, e)| ((e - s) as u64).pow(2)).sum();
    if work < GPU_MIN_PAIR_WORK { return None; } // CPU finishes first
    let thr = (min as f32) * 1.02 + 1.0; // gaps are exact integers in f32; small margin
    let flags = gpu::poly_near_flags(&edges, &descs, thr)?;
    Some(flags.into_iter().map(|f| f == 0).collect())
}

/// Min squared distance between two polygons' edge sets, exact below `cutoff`.
/// Returns exactly `cutoff²` when the true distance is >= cutoff — every caller
/// only branches on distances below its rule limit, so the far side needs no
/// precision. Bounding the search is what kills the |Ea|×|Eb| blowup: b-edges
/// are bucketed on a uniform grid and each a-edge only visits buckets within
/// `cutoff` of its bbox. Two 16k-edge interlocked combs drop from 256M seg-seg
/// evaluations to ~zero when nothing is within the limit.
fn poly_poly_dist2_within(store: &GeometryStore, pa: PolyId, pb: PolyId, cutoff: i32) -> i64 {
    let ea = poly_edges(store, pa);
    let eb = poly_edges(store, pb);
    let cutoff = cutoff.max(1);
    let cut2 = (cutoff as i64) * (cutoff as i64);

    // small pairs (rects vs rects): brute force beats grid setup
    if ea.len() * eb.len() <= 1024 {
        let mut best = cut2;
        for x in &ea {
            for y in &eb {
                best = best.min(seg_seg_dist2(x, y));
                if best == 0 { return 0; }
            }
        }
        return best;
    }

    // bucket b-edges by bbox on a cutoff-sized grid. The floor keeps tiny cutoffs
    // (the touch test uses 1nm) from exploding long edges into thousands of cells.
    let cell = ((cutoff as i64) * 4).max(512);
    let key = |x: i64, y: i64| ((x.div_euclid(cell)) as i32, (y.div_euclid(cell)) as i32);
    let mut grid: std::collections::HashMap<(i32, i32), Vec<u32>> =
        std::collections::HashMap::new();
    for (j, e) in eb.iter().enumerate() {
        let (x0, x1) = (e.x0.min(e.x1) as i64, e.x0.max(e.x1) as i64);
        let (y0, y1) = (e.y0.min(e.y1) as i64, e.y0.max(e.y1) as i64);
        let (kx0, ky0) = key(x0, y0);
        let (kx1, ky1) = key(x1, y1);
        for kx in kx0..=kx1 {
            for ky in ky0..=ky1 {
                grid.entry((kx, ky)).or_default().push(j as u32);
            }
        }
    }

    let mut best = cut2;
    let mut stamp = vec![u32::MAX; eb.len()]; // dedupe candidates per a-edge
    for (i, a) in ea.iter().enumerate() {
        let (ax0, ax1) = (a.x0.min(a.x1) as i64, a.x0.max(a.x1) as i64);
        let (ay0, ay1) = (a.y0.min(a.y1) as i64, a.y0.max(a.y1) as i64);
        let (kx0, ky0) = key(ax0 - cutoff as i64, ay0 - cutoff as i64);
        let (kx1, ky1) = key(ax1 + cutoff as i64, ay1 + cutoff as i64);
        for kx in kx0..=kx1 {
            for ky in ky0..=ky1 {
                let Some(cands) = grid.get(&(kx, ky)) else { continue };
                for &j in cands {
                    if stamp[j as usize] == i as u32 { continue; }
                    stamp[j as usize] = i as u32;
                    let b = &eb[j as usize];
                    // bbox lower bound before the exact kernel
                    let (bx0, bx1) = (b.x0.min(b.x1) as i64, b.x0.max(b.x1) as i64);
                    let (by0, by1) = (b.y0.min(b.y1) as i64, b.y0.max(b.y1) as i64);
                    let dx = (bx0 - ax1).max(ax0 - bx1).max(0);
                    let dy = (by0 - ay1).max(ay0 - by1).max(0);
                    if dx * dx + dy * dy >= best { continue; }
                    best = best.min(seg_seg_dist2(a, b));
                    if best == 0 { return 0; }
                }
            }
        }
    }
    best
}

fn poly_edges(store: &GeometryStore, p: PolyId) -> Vec<Edge> {
    let (s, e) = store.poly_range(p);
    let n = e - s;
    let mut v = Vec::with_capacity(n);
    for i in 0..n {
        let (x0, y0) = store.poly_vertex(s, i);
        let (x1, y1) = store.poly_vertex(s, (i + 1) % n);
        v.push(Edge { x0, y0, x1, y1, poly: p.0 });
    }
    v
}

// --- enclosure (outer must enclose inner by >= min on all sides) ------------
fn check_enclosure(
    store: &GeometryStore, lt: &LayerTable, outer: LayerId, inner: LayerId, min: i32,
    backend: Backend, rule_id: &str, out: &mut Vec<Violation>,
) {
    let outers = store.polys_on_layer(outer);
    // phase 1: containment (CPU point-in-poly); unhosted inners are zero-enclosure.
    // Collect EVERY containing outer: the inner passes if its BEST host
    // encloses it — merged-metal semantics (a wire clipping the corner of a
    // via must not fail a via that its pad fully encloses).
    let mut hosted: Vec<(PolyId, PolyId)> = Vec::new();
    for pi in store.polys_on_layer(inner) {
        let ib = store.poly_bbox[pi.0 as usize];
        // containment truly, NOT by bbox: an L-shaped outer's bbox contains
        // points the polygon doesn't cover.
        let mut any = false;
        for &po in &outers {
            if poly_strictly_inside(store, pi, po) {
                hosted.push((pi, po));
                any = true;
            }
        }
        if !any {
            out.push(Violation {
                rule_id: rule_id.into(), kind: "min_enclosure".into(),
                layer: lt.name(inner).into(), measured: 0, limit: min as i64,
                x: ib.xmin, y: ib.ymin,
            });
        }
    }
    // phase 2: margin = min inner-boundary-to-outer-boundary distance, best
    // host wins. The GPU clears pairs whose margin is comfortably >= min
    // (clearing the inner entirely); the rest are measured exactly.
    let far = gpu_far_mask(store, &hosted, min, backend);
    let mut best: std::collections::HashMap<u32, i64> = std::collections::HashMap::new();
    let mut cleared: std::collections::HashSet<u32> = std::collections::HashSet::new();
    for (k, &(pi, po)) in hosted.iter().enumerate() {
        if far.as_ref().is_some_and(|f| f[k]) {
            cleared.insert(pi.0);
            continue;
        }
        // exact for any polygon pair, equals the per-side margins on rectangles.
        let worst = i64::from(isqrt(poly_poly_dist2_within(store, pi, po, min + 1)) as i32);
        let e = best.entry(pi.0).or_insert(i64::MIN);
        *e = (*e).max(worst);
    }
    for (pi, m) in best {
        if !cleared.contains(&pi) && m < i64::from(min) {
            let ib = store.poly_bbox[pi as usize];
            out.push(Violation {
                rule_id: rule_id.into(), kind: "min_enclosure".into(),
                layer: lt.name(inner).into(), measured: m, limit: min as i64,
                x: ib.xmin, y: ib.ymin,
            });
        }
    }
}

// --- extension (layer extends past reference by >= min) ---------------------
// e.g. poly gate endcap over diff. We measure how far `layer` sticks out beyond `reference`
// along the axis of the reference, on each protruding side, taking the minimum protrusion
// where the two overlap.
fn check_extension(
    store: &GeometryStore, lt: &LayerTable, layer: LayerId, reference: LayerId, min: i32,
    rule_id: &str, out: &mut Vec<Violation>,
) {
    let refs = store.polys_on_layer(reference);
    for pl in store.polys_on_layer(layer) {
        let lb = store.poly_bbox[pl.0 as usize];
        for &pr in &refs {
            let rb = store.poly_bbox[pr.0 as usize];
            if !lb.overlaps(&rb) { continue; }
            // Poly is a vertical bar crossing a horizontal diff: measure vertical protrusion.
            // Determine crossing orientation by which dimension of `layer` is longer.
            if lb.height() >= lb.width() {
                // vertical bar: extension is top and bottom past the reference
                let bottom_ext = rb.ymin - lb.ymin;
                let top_ext = lb.ymax - rb.ymax;
                for (ext, yy) in [(bottom_ext, rb.ymin), (top_ext, rb.ymax)] {
                    if ext < min {
                        out.push(Violation {
                            rule_id: rule_id.into(), kind: "min_extension".into(),
                            layer: lt.name(layer).into(), measured: ext as i64,
                            limit: min as i64, x: lb.xmin, y: yy,
                        });
                    }
                }
            } else {
                let left_ext = rb.xmin - lb.xmin;
                let right_ext = lb.xmax - rb.xmax;
                for (ext, xx) in [(left_ext, rb.xmin), (right_ext, rb.xmax)] {
                    if ext < min {
                        out.push(Violation {
                            rule_id: rule_id.into(), kind: "min_extension".into(),
                            layer: lt.name(layer).into(), measured: ext as i64,
                            limit: min as i64, x: xx, y: lb.ymin,
                        });
                    }
                }
            }
        }
    }
}

// --- min area ---------------------------------------------------------------
fn check_min_area(
    store: &GeometryStore, lt: &LayerTable, layer: LayerId, min: i64,
    rule_id: &str, out: &mut Vec<Violation>,
) {
    // Merged-shape semantics (real DRC merges before measuring): a polygon's
    // area counts together with everything it touches. Group sum over-counts
    // shared overlap — conservative toward passing; boolean union if that
    // ever matters. One violation per merged group, not per fragment.
    let polys = store.polys_on_layer(layer);
    let idx_of: std::collections::HashMap<u32, u32> =
        polys.iter().enumerate().map(|(i, p)| (p.0, i as u32)).collect();
    let cands = candidate_pairs(store, &polys, None, 1);
    let group = merge_groups(store, &cands, None, polys.len(), &idx_of);
    let mut gsum: Vec<i64> = vec![0; polys.len()];
    for (i, &p) in polys.iter().enumerate() {
        gsum[group[i] as usize] += store.area(p);
    }
    for (i, &p) in polys.iter().enumerate() {
        let g = group[i] as usize;
        // report on the group root only, so a merged shape yields one violation
        if g == i && gsum[g] < min {
            let bb = store.poly_bbox[p.0 as usize];
            out.push(Violation {
                rule_id: rule_id.into(), kind: "min_area".into(),
                layer: lt.name(layer).into(), measured: gsum[g], limit: min,
                x: bb.xmin, y: bb.ymin,
            });
        }
    }
}

// --- max width (slotting trigger) -------------------------------------------
fn check_max_width(
    store: &GeometryStore, lt: &LayerTable, layer: LayerId, max: i32,
    rule_id: &str, out: &mut Vec<Violation>,
) {
    for p in store.polys_on_layer(layer) {
        let bb = store.poly_bbox[p.0 as usize];
        let w = bb.width().min(bb.height());
        if w > max {
            out.push(Violation {
                rule_id: rule_id.into(), kind: "max_width".into(),
                layer: lt.name(layer).into(), measured: w as i64, limit: max as i64,
                x: bb.xmin, y: bb.ymin,
            });
        }
    }
}

// --- notch (internal facing edges of the SAME polygon too close) ------------
fn check_notch(
    store: &GeometryStore, lt: &LayerTable, layer: LayerId, min: i32, backend: Backend,
    rule_id: &str, out: &mut Vec<Violation>,
) {
    let polys = store.polys_on_layer(layer);
    let clean = gpu_poly_clean_mask(store, &polys, min, backend);
    for (k, &p) in polys.iter().enumerate() {
        if clean.as_ref().is_some_and(|c| c[k]) { continue; }
        // a notch is a facing pair whose gap is OUTSIDE the polygon (see facing_gaps);
        // interior pairs are widths and belong to min_width, not here.
        for (d, mx, my) in facing_gaps(store, p, false) {
            if d < min {
                out.push(Violation {
                    rule_id: rule_id.into(), kind: "notch".into(),
                    layer: lt.name(layer).into(), measured: d as i64,
                    limit: min as i64, x: mx, y: my,
                });
            }
        }
    }
}

// --- min edge length --------------------------------------------------------
fn check_min_edge_length(
    store: &GeometryStore, lt: &LayerTable, layer: LayerId, min: i32, backend: Backend,
    rule_id: &str, out: &mut Vec<Violation>,
) {
    let min2 = (min as i64) * (min as i64);
    let edges = build_edges(store, layer);
    // GPU prefilter: edges with len2 clearly >= min2 need no exact check
    let approx = if backend == Backend::Gpu && edges.len() >= GPU_MIN_LINEAR_WORK {
        gpu::approx_edge_len2(&edges)
    } else {
        None
    };
    let thr = (min as f32) * (min as f32) * 1.05 + 4.0;
    for (i, e) in edges.iter().enumerate() {
        if approx.as_ref().is_some_and(|a| a[i] >= thr) { continue; }
        let l2 = e.len2();
        if l2 > 0 && l2 < min2 {
            out.push(Violation {
                rule_id: rule_id.into(), kind: "min_edge_length".into(),
                layer: lt.name(layer).into(), measured: isqrt(l2), limit: min as i64,
                x: e.x0, y: e.y0,
            });
        }
    }
}

// --- off grid ---------------------------------------------------------------
fn check_off_grid(
    store: &GeometryStore, lt: &LayerTable, grid: i32, backend: Backend,
    rule_id: &str, out: &mut Vec<Violation>,
) {
    // GPU path is exact (integer remainder), so flags are authoritative; emission stays
    // on the CPU in polygon order so output is identical either way.
    let flags = if backend == Backend::Gpu && store.verts_x.len() >= GPU_MIN_LINEAR_WORK {
        gpu::offgrid_flags(&store.verts_x, &store.verts_y, grid)
    } else {
        None
    };
    for p in 0..store.poly_count() as u32 {
        let pid = PolyId(p);
        let layer = store.poly_layer[p as usize];
        let (s, e) = store.poly_range(pid);
        for i in s..e {
            if flags.as_ref().is_some_and(|f| f[i] == 0) { continue; }
            let x = store.verts_x[i];
            let y = store.verts_y[i];
            if x % grid != 0 || y % grid != 0 {
                let measured = if x % grid != 0 { x } else { y };
                out.push(Violation {
                    rule_id: rule_id.into(), kind: "off_grid".into(),
                    layer: lt.name(layer).into(), measured: measured as i64,
                    limit: grid as i64, x, y,
                });
            }
        }
    }
}

// --- angle (edges must have an allowed orientation in degrees) --------------
fn check_angle(
    store: &GeometryStore, lt: &LayerTable, allowed: &[i32], backend: Backend,
    rule_id: &str, out: &mut Vec<Violation>,
) {
    // GPU prefilter: |sin(edge - allowed)| < sin(0.4 deg) is clearly within the CPU's
    // 0.5 deg tolerance -> skip. Borderline and violating edges are exact-checked below.
    const SIN_04_DEG: f32 = 0.006_981_3;
    let mut all_edges: Vec<(Edge, LayerId)> = Vec::new();
    for p in 0..store.poly_count() as u32 {
        let layer = store.poly_layer[p as usize];
        all_edges.extend(poly_edges(store, PolyId(p)).into_iter().map(|e| (e, layer)));
    }
    let dev = if backend == Backend::Gpu && all_edges.len() >= GPU_MIN_LINEAR_WORK {
        let flat: Vec<Edge> = all_edges.iter().map(|(e, _)| *e).collect();
        gpu::approx_angle_dev(&flat, allowed)
    } else {
        None
    };
    for (i, (e, layer)) in all_edges.iter().enumerate() {
        if e.dx() == 0 && e.dy() == 0 { continue; }
        if dev.as_ref().is_some_and(|d| d[i] < SIN_04_DEG) { continue; }
        let ang = edge_angle_deg(e);
        let ok = allowed.iter().any(|&a| ang_matches(ang, a));
        if !ok {
            out.push(Violation {
                rule_id: rule_id.into(), kind: "angle".into(),
                layer: lt.name(*layer).into(), measured: -1, limit: 0,
                x: e.x0, y: e.y0,
            });
        }
    }
}

fn edge_angle_deg(e: &Edge) -> f64 {
    let a = (e.dy() as f64).atan2(e.dx() as f64).to_degrees();
    // normalize to [0,180)
    let mut a = a % 180.0;
    if a < 0.0 { a += 180.0; }
    a
}
fn ang_matches(ang: f64, allowed: i32) -> bool {
    let target = (allowed as f64) % 180.0;
    (ang - target).abs() < 0.5 || (ang - target - 180.0).abs() < 0.5
}

// --- density (windowed coverage) --------------------------------------------
// Tiles the layer's global bbox into `window`-sized windows and computes the covered
// fraction of the layer polygons in each. Simplified but exact for axis-aligned rectangles
// under our conformance geometry (single-window cases).
fn check_density(
    store: &GeometryStore, lt: &LayerTable, layer: LayerId, window: i32, frac_limit: f64,
    is_min: bool, rule_id: &str, out: &mut Vec<Violation>,
) {
    if window <= 0 { return; }
    let polys = store.polys_on_layer(layer);
    if polys.is_empty() { return; }
    // global bbox
    let mut gb = Bbox::empty();
    for &p in &polys {
        let b = store.poly_bbox[p.0 as usize];
        gb.include(b.xmin, b.ymin);
        gb.include(b.xmax, b.ymax);
    }
    // Tile the field into window-sized tiles anchored at the field origin — a single
    // anchored window misses violations anywhere past the first window. Coverage is
    // accumulated per window in one pass over the polygons (O(P + windows), not O(P*W)),
    // clipping each polygon to the window exactly (bbox coverage overstates combs badly).
    // ponytail: assumes same-layer shapes don't overlap each other (true after merge;
    // overlapping input shapes would double-count).
    let win_area = (window as f64) * (window as f64);
    let nx = ((gb.xmax - gb.xmin) as i64 + window as i64 - 1) / window as i64;
    let ny = ((gb.ymax - gb.ymin) as i64 + window as i64 - 1) / window as i64;
    let mut covered: std::collections::HashMap<(i64, i64), f64> = std::collections::HashMap::new();
    for &p in &polys {
        let b = store.poly_bbox[p.0 as usize];
        let wi0 = ((b.xmin - gb.xmin) / window) as i64;
        let wi1 = (((b.xmax - gb.xmin) - 1) / window) as i64;
        let wj0 = ((b.ymin - gb.ymin) / window) as i64;
        let wj1 = (((b.ymax - gb.ymin) - 1) / window) as i64;
        for wi in wi0..=wi1.min(nx - 1) {
            for wj in wj0..=wj1.min(ny - 1) {
                let wx0 = gb.xmin + (wi as i32) * window;
                let wy0 = gb.ymin + (wj as i32) * window;
                let a = clipped_area(store, p, wx0, wy0, wx0 + window, wy0 + window);
                if a > 0.0 {
                    *covered.entry((wi, wj)).or_insert(0.0) += a;
                }
            }
        }
    }
    for wj in 0..ny {
        for wi in 0..nx {
            let frac = *covered.get(&(wi, wj)).unwrap_or(&0.0) / win_area;
            let bad = if is_min { frac < frac_limit } else { frac > frac_limit };
            if bad {
                out.push(Violation {
                    rule_id: rule_id.into(),
                    kind: if is_min { "min_density".into() } else { "max_density".into() },
                    layer: lt.name(layer).into(),
                    measured: (frac * 1_000_000.0) as i64, // frac scaled to ppm to fit i64
                    limit: (frac_limit * 1_000_000.0) as i64,
                    x: gb.xmin + (wi as i32) * window,
                    y: gb.ymin + (wj as i32) * window,
                });
            }
        }
    }
}

// --- overlap (two layers must overlap by >= min) ----------------------------
fn check_overlap(
    store: &GeometryStore, lt: &LayerTable, a: LayerId, b: LayerId, min: i32,
    rule_id: &str, out: &mut Vec<Violation>,
) {
    let bs = store.polys_on_layer(b);
    for pa in store.polys_on_layer(a) {
        let ba = store.poly_bbox[pa.0 as usize];
        for &pb in &bs {
            let bb = store.poly_bbox[pb.0 as usize];
            // overlap region
            let ix0 = ba.xmin.max(bb.xmin);
            let iy0 = ba.ymin.max(bb.ymin);
            let ix1 = ba.xmax.min(bb.xmax);
            let iy1 = ba.ymax.min(bb.ymax);
            if ix1 <= ix0 || iy1 <= iy0 { continue; } // no overlap at all: not this rule's job
            let ov = (ix1 - ix0).min(iy1 - iy0);
            if ov < min {
                out.push(Violation {
                    rule_id: rule_id.into(), kind: "overlap".into(),
                    layer: format!("{}:{}", lt.name(a), lt.name(b)),
                    measured: ov as i64, limit: min as i64, x: ix0, y: iy0,
                });
            }
        }
    }
}

// --- corner to corner -------------------------------------------------------
fn check_corner_to_corner(
    store: &GeometryStore, lt: &LayerTable, layer: LayerId, min: i32, backend: Backend,
    rule_id: &str, out: &mut Vec<Violation>,
) {
    let polys = store.polys_on_layer(layer);
    let min2 = (min as i64) * (min as i64);
    let cands = candidate_pairs(store, &polys, None, min);
    // edge-pair distance lower-bounds corner distance, so the same GPU prefilter applies
    let far = gpu_far_mask(store, &cands, min, backend);
    let idx_of: std::collections::HashMap<u32, u32> =
        polys.iter().enumerate().map(|(i, p)| (p.0, i as u32)).collect();
    let group = merge_groups(store, &cands, far.as_ref(), polys.len(), &idx_of);
    for (k, &(pa, pb)) in cands.iter().enumerate() {
        if far.as_ref().is_some_and(|f| f[k]) { continue; }
        if group[idx_of[&pa.0] as usize] == group[idx_of[&pb.0] as usize] {
            continue; // parts of one merged shape (see merge_groups)
        }
        {
            let ba = store.poly_bbox[pa.0 as usize];
            let bb = store.poly_bbox[pb.0 as usize];
            if ba.overlaps(&bb) { continue; }
            // Only fire when the shapes are diagonally offset (no x or y span overlap),
            // otherwise it's an edge-spacing situation handled by min_spacing.
            let x_overlap = ba.xmax.min(bb.xmax) > ba.xmin.max(bb.xmin);
            let y_overlap = ba.ymax.min(bb.ymax) > ba.ymin.max(bb.ymin);
            if x_overlap || y_overlap { continue; }
            // nearest corners
            let (sa, ea) = store.poly_range(pa);
            let (sb, eb) = store.poly_range(pb);
            let mut best = i64::MAX;
            let mut bx = 0;
            let mut by = 0;
            for u in sa..ea {
                for w in sb..eb {
                    let dx = (store.verts_x[u] - store.verts_x[w]) as i64;
                    let dy = (store.verts_y[u] - store.verts_y[w]) as i64;
                    let d2 = dx * dx + dy * dy;
                    if d2 < best { best = d2; bx = store.verts_x[u]; by = store.verts_y[u]; }
                }
            }
            if best < min2 {
                out.push(Violation {
                    rule_id: rule_id.into(), kind: "corner_to_corner".into(),
                    layer: lt.name(layer).into(), measured: isqrt(best), limit: min as i64,
                    x: bx, y: by,
                });
            }
        }
    }
}

// --- EOL spacing (end-of-line) ------------------------------------------------
// An edge shorter than `eol_width` gets an enlarged spacing zone of `eol_spacing`.
// If any other polygon's nearest edge is within that zone, it's a violation.
// ponytail: simplified to bbox-distance from short-edge endpoints to other polygons;
// full EOL uses a rectangular extension zone, upgrade if measurement precision matters.
fn check_eol_spacing(
    store: &GeometryStore, lt: &LayerTable, layer: LayerId, eol_width: i32,
    eol_spacing: i32, backend: Backend, rule_id: &str, out: &mut Vec<Violation>,
) {
    let polys = store.polys_on_layer(layer);
    let eol_sp2 = (eol_spacing as i64) * (eol_spacing as i64);
    let cands = candidate_pairs(store, &polys, None, eol_spacing);
    let far = gpu_far_mask(store, &cands, eol_spacing, backend);
    let idx_of: std::collections::HashMap<u32, u32> =
        polys.iter().enumerate().map(|(i, p)| (p.0, i as u32)).collect();
    let group = merge_groups(store, &cands, far.as_ref(), polys.len(), &idx_of);
    for (k, &(pa, pb)) in cands.iter().enumerate() {
        if far.as_ref().is_some_and(|f| f[k]) { continue; }
        if group[idx_of[&pa.0] as usize] == group[idx_of[&pb.0] as usize] { continue; }
        let d2 = poly_poly_dist2_within(store, pa, pb, eol_spacing);
        if d2 == 0 || d2 >= eol_sp2 { continue; }
        // A short edge projects a rectangular zone of depth eol_spacing along its
        // OUTWARD normal; only geometry entering that zone violates. A neighbor
        // beside the wire end is plain min_spacing territory, not EOL.
        let zone_hit = |p_eol: PolyId, p_other: PolyId| -> Option<(i64, i32, i32)> {
            let ccw = store.signed_area2(p_eol) > 0;
            let ea = poly_edges(store, p_eol);
            let eb = poly_edges(store, p_other);
            let mut best: Option<(i64, i32, i32)> = None;
            for a in &ea {
                let elen2 = a.len2();
                if elen2 == 0 || elen2 >= (eol_width as i64) * (eol_width as i64) { continue; }
                // manhattan outward normal (diagonal EOL edges: skip, no zone defined)
                let (udx, udy) = (a.dx().signum(), a.dy().signum());
                if udx != 0 && udy != 0 { continue; }
                let (nx, ny) = if ccw { (udy, -udx) } else { (-udy, udx) };
                let zone = Bbox {
                    xmin: a.x0.min(a.x1) + nx.min(0) * eol_spacing,
                    xmax: a.x0.max(a.x1) + nx.max(0) * eol_spacing,
                    ymin: a.y0.min(a.y1) + ny.min(0) * eol_spacing,
                    ymax: a.y0.max(a.y1) + ny.max(0) * eol_spacing,
                };
                for b in &eb {
                    let eb_box = Bbox {
                        xmin: b.x0.min(b.x1), xmax: b.x0.max(b.x1),
                        ymin: b.y0.min(b.y1), ymax: b.y0.max(b.y1),
                    };
                    if !zone.overlaps(&eb_box) { continue; }
                    let sd = seg_seg_dist2(a, b);
                    if sd > 0 && sd < eol_sp2 && best.is_none_or(|(bd, _, _)| sd < bd) {
                        best = Some((sd, a.x0, a.y0));
                    }
                }
            }
            best
        };
        if let Some((sd, x, y)) = zone_hit(pa, pb).or_else(|| zone_hit(pb, pa)) {
            out.push(Violation {
                rule_id: rule_id.into(), kind: "eol_spacing".into(),
                layer: lt.name(layer).into(), measured: isqrt(sd), limit: eol_spacing as i64,
                x, y,
            });
        }
    }
}

// --- antenna (PAR: metal area / gate area) ------------------------------------
// For each gate (poly-over-diff crossing), sum connected metal area on `layer`
// and compare to the gate area. ratio = metal_area / gate_area; violation if > limit.
// ponytail: simplified single-layer antenna. Multi-layer cumulative (CAR) and
// diode relief need the full connectivity graph; add when needed.
fn check_antenna(
    store: &GeometryStore, deck: &Deck, layer: LayerId, ratio: f64,
    rule_id: &str, out: &mut Vec<Violation>,
) {
    use crate::lvs::extract_netlist;

    let lt = &deck.layers;
    let ext = match extract_netlist(store, deck) {
        Ok(e) => e,
        Err(_) => return,
    };
    let poly_l = match lt.id("poly") { Some(l) => l, None => return };
    let diff_l = match lt.id("diff") { Some(l) => l, None => return };

    // Gate area per gate net: sum poly-over-diff intersection areas
    let mut gate_area: std::collections::HashMap<u32, i64> = std::collections::HashMap::new();
    for p in store.polys_on_layer(poly_l) {
        let pb = store.poly_bbox[p.0 as usize];
        for d in store.polys_on_layer(diff_l) {
            let db = store.poly_bbox[d.0 as usize];
            let ix = pb.xmax.min(db.xmax) - pb.xmin.max(db.xmin);
            let iy = pb.ymax.min(db.ymax) - pb.ymin.max(db.ymin);
            if ix > 0 && iy > 0 {
                let net = ext.net_of_poly[p.0 as usize];
                if net != u32::MAX {
                    *gate_area.entry(net).or_insert(0) += (ix as i64) * (iy as i64);
                }
            }
        }
    }

    // Metal area per net on the antenna layer
    let mut metal_area: std::collections::HashMap<u32, i64> = std::collections::HashMap::new();
    for m in store.polys_on_layer(layer) {
        let net = ext.net_of_poly[m.0 as usize];
        if net != u32::MAX {
            *metal_area.entry(net).or_insert(0) += store.area(m);
        }
    }

    // Check each gate net
    for (&net, &ga) in &gate_area {
        if ga == 0 { continue; }
        let ma = *metal_area.get(&net).unwrap_or(&0);
        let r = ma as f64 / ga as f64;
        if r > ratio {
            let pos = ext.net_of_poly.iter().enumerate()
                .find(|(_, &n)| n == net)
                .map(|(i, _)| store.poly_bbox[i])
                .unwrap_or(Bbox::empty());
            out.push(Violation {
                rule_id: rule_id.into(), kind: "antenna".into(),
                layer: lt.name(layer).into(),
                measured: (r * 1000.0) as i64, // ratio × 1000 to fit i64
                limit: (ratio * 1000.0) as i64,
                x: pos.xmin, y: pos.ymin,
            });
        }
    }
}

// --- antenna CAR (cumulative, per fabrication stage) ---------------------------
// During fab, metal-k is etched while only layers <= k exist. So the check runs
// once per stack stage: connectivity is rebuilt with conductors up to layers[k]
// (plus gate layers and any via whose endpoints are all present), and the
// cumulative collecting area of layers[0..=k] on each gate's net is compared to
// ratio × gate area. A shape on the diode layer connected to the net waives the
// gate (junction leaks the charge off).
// ponytail: connection = bbox overlap, matching extract's convention on the
// rectangle geometry the suite uses; diffusion is NOT in the graph (poly-over-diff
// is a gate, not a connection), so relief is explicit via the diode marker.
fn check_antenna_car(
    store: &GeometryStore, deck: &Deck, stack: &[LayerId], ratio: f64,
    diode: Option<LayerId>, rule_id: &str, out: &mut Vec<Violation>,
) {
    let lt = &deck.layers;
    // gate polys and their areas: gate_layer ∩ channel_layer per MOS rule
    let mut gates: Vec<(PolyId, i64)> = Vec::new(); // (gate poly, gate area)
    let mut gate_layers: Vec<LayerId> = Vec::new();
    for mr in &deck.devices.mos_rules {
        if !gate_layers.contains(&mr.gate_layer) { gate_layers.push(mr.gate_layer); }
        for g in store.polys_on_layer(mr.gate_layer) {
            let gb = store.poly_bbox[g.0 as usize];
            let mut area = 0i64;
            for d in store.polys_on_layer(mr.channel_layer) {
                let db = store.poly_bbox[d.0 as usize];
                let ix = gb.xmax.min(db.xmax) - gb.xmin.max(db.xmin);
                let iy = gb.ymax.min(db.ymax) - gb.ymin.max(db.ymin);
                if ix > 0 && iy > 0 { area += (ix as i64) * (iy as i64); }
            }
            if area > 0 && !gates.iter().any(|&(p, _)| p == g) { gates.push((g, area)); }
        }
    }
    if gates.is_empty() { return; }

    // worst cumulative ratio per gate poly across stages
    let mut worst: std::collections::HashMap<u32, f64> = std::collections::HashMap::new();

    for k in 0..stack.len() {
        // stage-active layers: gates + metals up to k + diode + vias fully present
        let metals = &stack[..=k];
        let mut active: Vec<LayerId> = gate_layers.clone();
        active.extend_from_slice(metals);
        if let Some(dl) = diode { active.push(dl); }
        // a via exists at this stage iff every layer it connects already exists
        for &(vl, ref connects) in &deck.connectivity.vias {
            if !active.contains(&vl) && connects.iter().all(|c| active.contains(c)) {
                active.push(vl);
            }
        }

        // union-find over active polys, connected on bbox overlap
        let polys: Vec<PolyId> = active.iter()
            .flat_map(|&l| store.polys_on_layer(l)).collect();
        let idx: std::collections::HashMap<u32, usize> =
            polys.iter().enumerate().map(|(i, p)| (p.0, i)).collect();
        let mut parent: Vec<usize> = (0..polys.len()).collect();
        fn find(parent: &mut Vec<usize>, i: usize) -> usize {
            let mut r = i;
            while parent[r] != r { r = parent[r]; }
            let mut c = i;
            while parent[c] != r { let n = parent[c]; parent[c] = r; c = n; }
            r
        }
        for (pa, pb) in candidate_pairs(store, &polys, None, 0) {
            let ba = store.poly_bbox[pa.0 as usize];
            let bb = store.poly_bbox[pb.0 as usize];
            if ba.overlaps(&bb) {
                let (ra, rb) = (find(&mut parent, idx[&pa.0]), find(&mut parent, idx[&pb.0]));
                if ra != rb { parent[ra] = rb; }
            }
        }

        // per-component: cumulative metal area + diode presence
        let mut metal_area: std::collections::HashMap<usize, i64> = std::collections::HashMap::new();
        let mut relieved: std::collections::HashSet<usize> = std::collections::HashSet::new();
        for &p in &polys {
            let root = find(&mut parent, idx[&p.0]);
            let l = store.poly_layer[p.0 as usize];
            if metals.contains(&l) {
                *metal_area.entry(root).or_insert(0) += store.area(p);
            }
            if diode == Some(l) { relieved.insert(root); }
        }

        for &(g, ga) in &gates {
            if !idx.contains_key(&g.0) { continue; }
            let root = find(&mut parent, idx[&g.0]);
            if relieved.contains(&root) { continue; }
            let ma = *metal_area.get(&root).unwrap_or(&0);
            let r = ma as f64 / ga as f64;
            let w = worst.entry(g.0).or_insert(0.0);
            if r > *w { *w = r; }
        }
    }

    for (&g, &r) in &worst {
        if r > ratio {
            let bb = store.poly_bbox[g as usize];
            out.push(Violation {
                rule_id: rule_id.into(), kind: "antenna_car".into(),
                layer: lt.name(stack[stack.len() - 1]).into(),
                measured: (r * 1000.0) as i64,
                limit: (ratio * 1000.0) as i64,
                x: bb.xmin, y: bb.ymin,
            });
        }
    }
}

// --- wide-dependent spacing ---------------------------------------------------
// When EITHER polygon in a pair is "wide" (min bbox dimension >= width_threshold),
// the pair must satisfy a larger spacing requirement (wide_spacing) instead of the
// normal min_spacing. This is the standard foundry rule for wide-metal spacing.
fn check_wide_dependent_spacing(
    store: &GeometryStore, lt: &LayerTable, layer: LayerId, width_threshold: i32,
    wide_spacing: i32, rule_id: &str, out: &mut Vec<Violation>,
) {
    let polys = store.polys_on_layer(layer);
    let ws2 = (wide_spacing as i64) * (wide_spacing as i64);
    let cands = candidate_pairs(store, &polys, None, wide_spacing);
    for &(pa, pb) in &cands {
        let ba = store.poly_bbox[pa.0 as usize];
        let bb = store.poly_bbox[pb.0 as usize];
        let wa = ba.width().min(ba.height());
        let wb = bb.width().min(bb.height());
        if wa < width_threshold && wb < width_threshold { continue; }
        let d2 = poly_poly_dist2_within(store, pa, pb, wide_spacing);
        if d2 == 0 { continue; } // overlapping/abutting, merged shape
        if poly_strictly_inside(store, pa, pb) || poly_strictly_inside(store, pb, pa) {
            continue;
        }
        if d2 < ws2 {
            out.push(Violation {
                rule_id: rule_id.into(), kind: "wide_dependent_spacing".into(),
                layer: lt.name(layer).into(), measured: isqrt(d2), limit: wide_spacing as i64,
                x: ba.xmax, y: ba.ymin,
            });
        }
    }
}

// --- PRL spacing (parallel run length dependent) -----------------------------
// When two same-layer polygons have a parallel run length >= prl_threshold,
// their spacing must be >= prl_spacing. PRL is the length of the overlapping
// projection along one axis when shapes face each other.
fn check_prl_spacing(
    store: &GeometryStore, lt: &LayerTable, layer: LayerId, prl_threshold: i32,
    prl_spacing: i32, rule_id: &str, out: &mut Vec<Violation>,
) {
    let polys = store.polys_on_layer(layer);
    let ps2 = (prl_spacing as i64) * (prl_spacing as i64);
    let cands = candidate_pairs(store, &polys, None, prl_spacing);
    for &(pa, pb) in &cands {
        let ba = store.poly_bbox[pa.0 as usize];
        let bb = store.poly_bbox[pb.0 as usize];
        // compute PRL: overlapping projection along an axis where the shapes face each other
        let x_overlap = ba.xmax.min(bb.xmax) - ba.xmin.max(bb.xmin);
        let y_overlap = ba.ymax.min(bb.ymax) - ba.ymin.max(bb.ymin);
        // shapes face each other in y (gap in y) => PRL is x_overlap
        // shapes face each other in x (gap in x) => PRL is y_overlap
        let x_gap = ba.xmin.max(bb.xmin) - ba.xmax.min(bb.xmax); // positive if separated in x
        let y_gap = ba.ymin.max(bb.ymin) - ba.ymax.min(bb.ymax); // positive if separated in y
        let prl = if x_overlap > 0 && y_gap > 0 {
            x_overlap
        } else if y_overlap > 0 && x_gap > 0 {
            y_overlap
        } else {
            continue; // diagonal or overlapping, no parallel run
        };
        if prl < prl_threshold { continue; }
        let d2 = poly_poly_dist2_within(store, pa, pb, prl_spacing);
        if d2 == 0 { continue; } // abutting/overlapping
        if d2 < ps2 {
            out.push(Violation {
                rule_id: rule_id.into(), kind: "prl_spacing".into(),
                layer: lt.name(layer).into(), measured: isqrt(d2), limit: prl_spacing as i64,
                x: ba.xmax, y: ba.ymin,
            });
        }
    }
}

// --- asymmetric enclosure ----------------------------------------------------
// Each inner polygon must have enclosure >= min_one_side on at least ONE side
// of each axis pair (left/right, top/bottom). Passes if max(left,right) >= min
// AND max(top,bottom) >= min.
fn check_asymmetric_enclosure(
    store: &GeometryStore, lt: &LayerTable, outer: LayerId, inner: LayerId, min_one_side: i32,
    rule_id: &str, out: &mut Vec<Violation>,
) {
    let outers = store.polys_on_layer(outer);
    for pi in store.polys_on_layer(inner) {
        let ib = store.poly_bbox[pi.0 as usize];
        let mut best_ok = false;
        for &po in &outers {
            if !poly_strictly_inside(store, pi, po) { continue; }
            let ob = store.poly_bbox[po.0 as usize];
            let left_enc = ib.xmin - ob.xmin;
            let right_enc = ob.xmax - ib.xmax;
            let bottom_enc = ib.ymin - ob.ymin;
            let top_enc = ob.ymax - ib.ymax;
            let x_ok = left_enc.max(right_enc) >= min_one_side;
            let y_ok = bottom_enc.max(top_enc) >= min_one_side;
            if x_ok && y_ok {
                best_ok = true;
                break;
            }
        }
        if !best_ok {
            // find the best measured enclosure from any host for reporting
            let mut best_measured: i32 = 0;
            for &po in &outers {
                if !poly_strictly_inside(store, pi, po) { continue; }
                let ob = store.poly_bbox[po.0 as usize];
                let left_enc = ib.xmin - ob.xmin;
                let right_enc = ob.xmax - ib.xmax;
                let bottom_enc = ib.ymin - ob.ymin;
                let top_enc = ob.ymax - ib.ymax;
                let m = left_enc.max(right_enc).min(bottom_enc.max(top_enc));
                best_measured = best_measured.max(m);
            }
            out.push(Violation {
                rule_id: rule_id.into(), kind: "asymmetric_enclosure".into(),
                layer: lt.name(inner).into(), measured: best_measured as i64,
                limit: min_one_side as i64, x: ib.xmin, y: ib.ymin,
            });
        }
    }
}

// --- min enclosed area (hole area) -------------------------------------------
// Detect "holes" — same-layer polygons where one is strictly inside another.
// The enclosed region's area = outer.area - inner.area. If this enclosed area
// < min_hole_area, emit violation.
// ponytail: simplified to bbox-based area for conformance geometry.
fn check_min_enclosed_area(
    store: &GeometryStore, lt: &LayerTable, layer: LayerId, min_hole_area: i64,
    rule_id: &str, out: &mut Vec<Violation>,
) {
    let polys = store.polys_on_layer(layer);
    // containment implies bbox overlap, so the x-sweep candidate generator
    // replaces the old all-pairs loop (quadratic on real layouts).
    for (pa, pb) in candidate_pairs(store, &polys, None, 0) {
        // unordered pair: try both nestings
        for (outer, inner) in [(pa, pb), (pb, pa)] {
            if !poly_strictly_inside(store, inner, outer) { continue; }
            let enclosed = store.area(outer) - store.area(inner);
            if enclosed < min_hole_area {
                let bb = store.poly_bbox[inner.0 as usize];
                out.push(Violation {
                    rule_id: rule_id.into(), kind: "min_enclosed_area".into(),
                    layer: lt.name(layer).into(), measured: enclosed,
                    limit: min_hole_area, x: bb.xmin, y: bb.ymin,
                });
            }
        }
    }
}

// --- cheesing (large unslotted plates) ---------------------------------------
// Any polygon on the layer whose area > max_area_no_slot that does NOT have a
// same-layer polygon strictly inside it (i.e., no slot/hole) is a violation.
fn check_cheesing(
    store: &GeometryStore, lt: &LayerTable, layer: LayerId, max_area_no_slot: i64,
    rule_id: &str, out: &mut Vec<Violation>,
) {
    let polys = store.polys_on_layer(layer);
    // only over-limit plates need a slot search; slots bbox-overlap their plate,
    // so one sweep over (plates, all) replaces the old all-pairs scan.
    let plates: Vec<PolyId> =
        polys.iter().copied().filter(|&p| store.area(p) > max_area_no_slot).collect();
    if plates.is_empty() { return; }
    let mut slotted: std::collections::HashSet<u32> = std::collections::HashSet::new();
    for (plate, q) in candidate_pairs(store, &plates, Some(&polys), 0) {
        if plate != q && poly_strictly_inside(store, q, plate) { slotted.insert(plate.0); }
    }
    for &p in &plates {
        let area = store.area(p);
        let has_slot = slotted.contains(&p.0);
        if !has_slot {
            let bb = store.poly_bbox[p.0 as usize];
            out.push(Violation {
                rule_id: rule_id.into(), kind: "cheesing".into(),
                layer: lt.name(layer).into(), measured: area,
                limit: max_area_no_slot, x: bb.xmin, y: bb.ymin,
            });
        }
    }
}

// --- redundant via (isolated via check) --------------------------------------
// Each polygon on the via layer must have at least `min_count - 1` other
// same-layer polygons whose bbox center is within `within` distance.
fn check_redundant_via(
    store: &GeometryStore, lt: &LayerTable, layer: LayerId, min_count: i32, within: i32,
    rule_id: &str, out: &mut Vec<Violation>,
) {
    let polys = store.polys_on_layer(layer);
    let within2 = (within as i64) * (within as i64);
    // centers sit inside their bboxes, so center-distance <= within implies the
    // bboxes come within `within`: the x-sweep candidate set is a superset.
    let center = |p: PolyId| -> (i64, i64) {
        let bb = store.poly_bbox[p.0 as usize];
        (((bb.xmin as i64) + (bb.xmax as i64)) / 2,
         ((bb.ymin as i64) + (bb.ymax as i64)) / 2)
    };
    let mut neighbors: std::collections::HashMap<u32, i32> =
        polys.iter().map(|p| (p.0, 0)).collect();
    for (pa, pb) in candidate_pairs(store, &polys, None, within) {
        let (ax, ay) = center(pa);
        let (bx, by) = center(pb);
        let (dx, dy) = (ax - bx, ay - by);
        if dx * dx + dy * dy <= within2 {
            *neighbors.get_mut(&pa.0).unwrap() += 1;
            *neighbors.get_mut(&pb.0).unwrap() += 1;
        }
    }
    for &p in &polys {
        let count = neighbors[&p.0];
        if count < min_count - 1 {
            let bb = store.poly_bbox[p.0 as usize];
            out.push(Violation {
                rule_id: rule_id.into(), kind: "redundant_via".into(),
                layer: lt.name(layer).into(), measured: (count + 1) as i64,
                limit: min_count as i64, x: bb.xmin, y: bb.ymin,
            });
        }
    }
}

// --- via array spacing -------------------------------------------------------
// When a group of > array_threshold polygons are clustered (each within
// array_spacing of another), check that all pairs within the group have
// spacing >= array_spacing. Build adjacency groups via union-find.
// ponytail: simplified — just check if any pair in a large group violates.
fn check_via_array_spacing(
    store: &GeometryStore, lt: &LayerTable, layer: LayerId, array_threshold: i32,
    array_spacing: i32, rule_id: &str, out: &mut Vec<Violation>,
) {
    let polys = store.polys_on_layer(layer);
    let n = polys.len();
    if n == 0 { return; }
    let as2 = (array_spacing as i64) * (array_spacing as i64);
    let cands = candidate_pairs(store, &polys, None, array_spacing);
    // union-find: two vias within array_spacing are in the same group
    let idx_of: std::collections::HashMap<u32, u32> =
        polys.iter().enumerate().map(|(i, p)| (p.0, i as u32)).collect();
    let mut parent: Vec<u32> = (0..n as u32).collect();
    fn find(parent: &mut [u32], x: u32) -> u32 {
        let mut r = x;
        while parent[r as usize] != r {
            parent[r as usize] = parent[parent[r as usize] as usize];
            r = parent[r as usize];
        }
        r
    }
    // track which candidate pairs are within array_spacing
    let mut close_pairs: Vec<(u32, u32)> = Vec::new();
    for &(pa, pb) in &cands {
        let d2 = poly_poly_dist2_within(store, pa, pb, array_spacing + 1);
        if d2 > 0 && d2 <= as2 {
            let ia = idx_of[&pa.0];
            let ib = idx_of[&pb.0];
            let (ra, rb) = (find(&mut parent, ia), find(&mut parent, ib));
            if ra != rb { parent[ra as usize] = rb; }
            close_pairs.push((ia, ib));
        }
    }
    // resolve all parents
    let groups: Vec<u32> = (0..n as u32).map(|i| find(&mut parent, i)).collect();
    // count group sizes
    let mut group_size: std::collections::HashMap<u32, i32> = std::collections::HashMap::new();
    for &g in &groups {
        *group_size.entry(g).or_insert(0) += 1;
    }
    // for groups larger than array_threshold, emit one violation per group
    let mut flagged_groups: std::collections::HashSet<u32> = std::collections::HashSet::new();
    for &(ia, ib) in &close_pairs {
        let ga = find(&mut parent, ia);
        if *group_size.get(&ga).unwrap_or(&0) <= array_threshold { continue; }
        if !flagged_groups.insert(ga) { continue; }
        let pa = polys[ia as usize];
        let pb = polys[ib as usize];
        let d2 = poly_poly_dist2_within(store, pa, pb, array_spacing + 1);
        if d2 > 0 && d2 < as2 {
            let ba = store.poly_bbox[pa.0 as usize];
            out.push(Violation {
                rule_id: rule_id.into(), kind: "via_array_spacing".into(),
                layer: lt.name(layer).into(), measured: isqrt(d2), limit: array_spacing as i64,
                x: ba.xmin, y: ba.ymin,
            });
        }
    }
}

// --- max distance to tap (well tie proximity) --------------------------------
// For each polygon on diff_layer, check that there exists at least one polygon
// on tap_layer whose bbox center is within max_dist of every corner of the diff
// bbox. If any diff corner is farther than max_dist from all taps, emit violation.
fn check_max_distance_to_tap(
    store: &GeometryStore, lt: &LayerTable, diff_layer: LayerId, tap_layer: LayerId,
    max_dist: i32, rule_id: &str, out: &mut Vec<Violation>,
) {
    let taps = store.polys_on_layer(tap_layer);
    let md2 = (max_dist as i64) * (max_dist as i64);
    // precompute tap bbox centers
    let tap_centers: Vec<(i64, i64)> = taps.iter().map(|&t| {
        let tb = store.poly_bbox[t.0 as usize];
        (((tb.xmin as i64) + (tb.xmax as i64)) / 2,
         ((tb.ymin as i64) + (tb.ymax as i64)) / 2)
    }).collect();
    for dp in store.polys_on_layer(diff_layer) {
        let db = store.poly_bbox[dp.0 as usize];
        let corners = [
            (db.xmin as i64, db.ymin as i64),
            (db.xmax as i64, db.ymin as i64),
            (db.xmax as i64, db.ymax as i64),
            (db.xmin as i64, db.ymax as i64),
        ];
        for &(cx, cy) in &corners {
            let covered = tap_centers.iter().any(|&(tx, ty)| {
                let dx = cx - tx;
                let dy = cy - ty;
                dx * dx + dy * dy <= md2
            });
            if !covered {
                out.push(Violation {
                    rule_id: rule_id.into(), kind: "max_distance_to_tap".into(),
                    layer: lt.name(diff_layer).into(), measured: 0, limit: max_dist as i64,
                    x: cx as i32, y: cy as i32,
                });
                break; // one violation per diff polygon is enough
            }
        }
    }
}

// --- multi-patterning (greedy graph coloring) --------------------------------
// Build a conflict graph: two polygons within color_spacing are "conflicting"
// (can't share a color). Attempt greedy graph coloring with num_colors colors.
// If coloring fails, emit one violation and stop.
// ponytail: greedy coloring, not optimal — false positives possible on
// pathological layouts; upgrade to backtracking if needed.
fn check_multi_patterning(
    store: &GeometryStore, lt: &LayerTable, layer: LayerId, num_colors: i32,
    color_spacing: i32, rule_id: &str, out: &mut Vec<Violation>,
) {
    let polys = store.polys_on_layer(layer);
    let n = polys.len();
    if n == 0 { return; }
    let cs2 = (color_spacing as i64) * (color_spacing as i64);
    let cands = candidate_pairs(store, &polys, None, color_spacing);
    let idx_of: std::collections::HashMap<u32, usize> =
        polys.iter().enumerate().map(|(i, p)| (p.0, i)).collect();
    // build adjacency list
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(pa, pb) in &cands {
        let d2 = poly_poly_dist2_within(store, pa, pb, color_spacing);
        if d2 > 0 && d2 < cs2 {
            let ia = idx_of[&pa.0];
            let ib = idx_of[&pb.0];
            adj[ia].push(ib);
            adj[ib].push(ia);
        }
    }
    // greedy coloring
    let mut color: Vec<i32> = vec![-1; n];
    for i in 0..n {
        let mut used = vec![false; num_colors as usize];
        for &nb in &adj[i] {
            if color[nb] >= 0 {
                used[color[nb] as usize] = true;
            }
        }
        let mut assigned = false;
        for c in 0..num_colors {
            if !used[c as usize] {
                color[i] = c;
                assigned = true;
                break;
            }
        }
        if !assigned {
            let bb = store.poly_bbox[polys[i].0 as usize];
            out.push(Violation {
                rule_id: rule_id.into(), kind: "multi_patterning".into(),
                layer: lt.name(layer).into(), measured: num_colors as i64,
                limit: num_colors as i64, x: bb.xmin, y: bb.ymin,
            });
            return; // one violation and stop
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Repro: via pad (A) and stub leg (C) 45nm apart, bridged by bar (B)
    // covering the gap — merged shape is solid there, no notch.
    #[test]
    fn bridged_same_net_gap_is_not_a_notch() {
        let mut store = GeometryStore::new();
        let met1: LayerId = 0;
        store.add_rect(met1, 8335, 7945, 290, 290); // A: pad
        store.add_rect(met1, 8335, 7945, 625, 290); // B: bar covering A..C gap
        store.add_rect(met1, 8670, 7945, 290, 585); // C: leg
        let mut defs = std::collections::HashMap::new();
        defs.insert("met1".to_string(), crate::params::LayerDef { layer: 68, datatype: 20 });
        let lt = LayerTable::from_defs(&defs);
        let mut out = Vec::new();
        check_spacing_same(&store, &lt, met1, 140, Backend::Cpu, false, "min_spacing", &mut out);
        assert!(out.is_empty(), "bridged gap flagged: {out:?}");
    }
}
