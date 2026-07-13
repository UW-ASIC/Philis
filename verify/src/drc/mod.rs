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
use crate::backend::Backend;
use crate::rule::VerifyCheck;
pub mod gpu;
use crate::params::{Deck, DrcRuleParam, LayerTable};

pub mod coloring;
pub mod derived;
pub mod fill;
pub mod production;
pub mod results;

/// Maximum number of sliding density windows evaluated by one rule/recheck.
/// Each window performs at least one exact boolean operation, so this shares
/// the rectilinear kernel's explicit 16M-work ceiling.
pub const MAX_DENSITY_WINDOW_WORK: usize =
    crate::geometry::exact::MAX_RECTILINEAR_BOOLEAN_CELLS;

pub(crate) fn density_window_work(
    xmin: i32,
    ymin: i32,
    xmax: i32,
    ymax: i32,
    window: i32,
    step: i32,
) -> Result<usize, crate::geometry::exact::ExactGeometryError> {
    let axis_count = |lo: i32, hi: i32| -> Option<u128> {
        let span = u128::try_from(i64::from(hi) - i64::from(lo)).ok()?;
        let window = u128::try_from(window).ok()?;
        let step = u128::try_from(step).ok()?;
        if span == 0 || window == 0 || step == 0 {
            return None;
        }
        let remaining = span.saturating_sub(window);
        remaining
            .checked_add(step - 1)?
            .checked_div(step)?
            .checked_add(1)
    };
    let requested = axis_count(xmin, xmax)
        .and_then(|nx| axis_count(ymin, ymax).and_then(|ny| nx.checked_mul(ny)))
        .ok_or(crate::geometry::exact::ExactGeometryError::ArithmeticOverflow)?;
    if requested > MAX_DENSITY_WINDOW_WORK as u128 {
        return Err(
            crate::geometry::exact::ExactGeometryError::CapacityExceeded {
                cells: usize::try_from(requested).unwrap_or(usize::MAX),
                limit: MAX_DENSITY_WINDOW_WORK,
            },
        );
    }
    usize::try_from(requested)
        .map_err(|_| crate::geometry::exact::ExactGeometryError::ArithmeticOverflow)
}

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
    // Capacity validation must precede every legacy kernel. Several W1 rule
    // result types still use i32/i64 measurements, so geometry outside those
    // declared numeric limits is a typed error, never an invitation to execute
    // overflowing candidate arithmetic.
    let mut violations = check_polygon_validity(store, &deck.layers);
    if violations.iter().any(|violation| violation.kind == "geometry_capacity") {
        return DrcReport { violations };
    }
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
    use crate::geometry::exact::{ExactGeometryError, SegmentIntersection};

    let mut out = Vec::new();
    for p in 0..store.poly_count() {
        let pid = PolyId(p as u32);
        let bb = store.poly_bbox[p];
        let issue = match store.poly_as_exact(pid) {
            Ok(polygon) => {
                let area = polygon.area2() / 2;
                let span_x = i64::from(bb.xmax) - i64::from(bb.xmin);
                let span_y = i64::from(bb.ymax) - i64::from(bb.ymin);
                (span_x > i64::from(i32::MAX)
                    || span_y > i64::from(i32::MAX)
                    || area > i128::from(i64::MAX))
                    .then_some("geometry_capacity")
            }
            Err(ExactGeometryError::DegenerateRing) => Some("zero_area"),
            // The admitted compatibility GDS adapter deliberately retains a
            // collapsed boundary so this always-on scan can diagnose it. A
            // zero-width rectangle reaches the exact constructor as duplicate
            // consecutive vertices, but its physical defect is still zero
            // area. Preserve that stable measurement without weakening the
            // rejection of nonzero malformed contacts.
            Err(
                ExactGeometryError::DuplicateConsecutiveVertex { .. }
                | ExactGeometryError::TooFewVertices { .. },
            ) if store.signed_area2_exact(pid) == Some(0) => Some("zero_area"),
            Err(ExactGeometryError::ArithmeticOverflow)
            | Err(ExactGeometryError::CapacityExceeded { .. }) => Some("geometry_capacity"),
            Err(ExactGeometryError::SelfIntersection {
                kind: SegmentIntersection::Proper,
                ..
            }) => Some("self_intersecting"),
            Err(_) => Some("invalid_boundary_contact"),
        };
        if let Some(kind_detail) = issue {
            out.push(Violation {
                rule_id: "__geometry__".into(),
                kind: if kind_detail == "geometry_capacity" {
                    "geometry_capacity".into()
                } else {
                    "polygon_validity".into()
                },
                layer: lt.name(store.poly_layer[p]).into(),
                measured: if kind_detail == "zero_area" { 0 } else { 1 },
                limit: 0, x: bb.xmin, y: bb.ymin,
            });
        }
    }
    if !out.iter().any(|violation| violation.kind == "geometry_capacity")
        && store.poly_count() > 0
    {
        let mut extent = Bbox::empty();
        for bbox in &store.poly_bbox {
            extent.include(bbox.xmin, bbox.ymin);
            extent.include(bbox.xmax, bbox.ymax);
        }
        // This is also the rule-arithmetic contract: with every coordinate
        // delta <= i32::MAX, the worst diagonal cross is < 2*extent² and its
        // square, dot products, rational sample numerators and distances all
        // fit i128. Larger layouts are rejected before any legacy kernel.
        if extent.width_i64() > i64::from(i32::MAX)
            || extent.height_i64() > i64::from(i32::MAX)
            || !legacy_rule_arithmetic_fits(&extent)
        {
            out.push(Violation {
                rule_id: "__geometry__".into(), kind: "geometry_capacity".into(),
                layer: lt.name(store.poly_layer[0]).into(), measured: 1, limit: 0,
                x: extent.xmin, y: extent.ymin,
            });
        }
    }
    out
}

fn legacy_rule_arithmetic_fits(extent: &Bbox) -> bool {
    let span = i128::from(extent.width_i64().max(extent.height_i64()).max(0));
    span.checked_mul(span)
        .and_then(|square| square.checked_mul(2))
        .and_then(|cross_bound| cross_bound.checked_mul(cross_bound))
        .is_some()
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
    let polys: Vec<PolyId> = store.polys_on_layer(layer).collect();
    let clean = gpu_poly_clean_mask(store, &polys, min, backend);
    for (k, &p) in polys.iter().enumerate() {
        let bb = store.poly_bbox[p.0 as usize];
        let (s, e) = store.poly_range(p);
        let n = e - s;
        // bbox fast path is exact ONLY for axis-aligned rectangles; a rotated
        // parallelogram has 4 vertices too and must take the facing-gap scan.
        let axis_aligned_rect =
            n == 4 && store.edges_of(p).all(|ed| ed.x0 == ed.x1 || ed.y0 == ed.y1);
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
        let gaps = match facing_gaps(store, p, true) {
            Ok(gaps) => gaps,
            Err(_) => {
                push_geometry_capacity(store, lt, p, out);
                return;
            }
        };
        for (d, mx, my) in gaps {
            if d < i64::from(min) {
                out.push(Violation {
                    rule_id: rule_id.into(), kind: "min_width".into(),
                    layer: lt.name(layer).into(), measured: d, limit: min as i64,
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
fn facing_gaps(
    store: &GeometryStore, p: PolyId, interior: bool,
) -> Result<Vec<(i64, i32, i32)>, crate::geometry::exact::ExactGeometryError> {
    use crate::geometry::exact::ExactGeometryError;

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
                (
                    (i64::from(a.x0) - i64::from(b.x0)).abs(),
                    midpoint_i32(a.x0, b.x0)?,
                    midpoint_i32(ylo, yhi)?,
                )
            } else if a.is_horizontal() && b.is_horizontal() {
                let xlo = a.x0.min(a.x1).max(b.x0.min(b.x1));
                let xhi = a.x0.max(a.x1).min(b.x0.max(b.x1));
                if xlo >= xhi { continue; }
                (
                    (i64::from(a.y0) - i64::from(b.y0)).abs(),
                    midpoint_i32(xlo, xhi)?,
                    midpoint_i32(a.y0, b.y0)?,
                )
            } else {
                // parallel diagonal edges (45° routing): perpendicular gap where
                // the edges overlap tangentially. All projection/distance/sample
                // arithmetic stays exact in i128; the marker alone is rounded.
                let (adx, ady) = (i128::from(a.dx_i64()), i128::from(a.dy_i64()));
                let (bdx, bdy) = (i128::from(b.dx_i64()), i128::from(b.dy_i64()));
                if adx * bdy - ady * bdx != 0 { continue; } // not parallel
                let len2 = adx * adx + ady * ady;
                if len2 == 0 { continue; }
                let t = |px: i32, py: i32| {
                    (i128::from(px) - i128::from(a.x0)) * adx
                        + (i128::from(py) - i128::from(a.y0)) * ady
                };
                let (tb0, tb1) = (t(b.x0, b.y0), t(b.x1, b.y1));
                let lo = tb0.min(tb1).max(0);
                let hi = tb0.max(tb1).min(len2);
                if lo >= hi { continue; } // no tangential overlap
                // signed offset of b's line along a's left normal (−ady, adx): d×w
                let cross = adx * (i128::from(b.y0) - i128::from(a.y0))
                    - ady * (i128::from(b.x0) - i128::from(a.x0));
                let cross2 = cross
                    .checked_mul(cross)
                    .ok_or(ExactGeometryError::ArithmeticOverflow)?;
                let distance2 = cross2 / len2;
                let d = isqrt(
                    i64::try_from(distance2)
                        .map_err(|_| ExactGeometryError::ArithmeticOverflow)?,
                );
                // Middle of tangential overlap, halfway between the lines:
                // q = a0 + (v*(lo+hi) + normal*cross) / (2*|v|²).
                let denominator = len2
                    .checked_mul(2)
                    .ok_or(ExactGeometryError::ArithmeticOverflow)?;
                let tangent = lo + hi;
                let dx_num = adx
                    .checked_mul(tangent)
                    .and_then(|value| ady.checked_mul(cross).and_then(|n| value.checked_sub(n)))
                    .ok_or(ExactGeometryError::ArithmeticOverflow)?;
                let dy_num = ady
                    .checked_mul(tangent)
                    .and_then(|value| adx.checked_mul(cross).and_then(|n| value.checked_add(n)))
                    .ok_or(ExactGeometryError::ArithmeticOverflow)?;
                let mx_offset = round_ratio_i128(dx_num, denominator)?;
                let my_offset = round_ratio_i128(dy_num, denominator)?;
                let mx = checked_i32(i128::from(a.x0) + mx_offset)?;
                let my = checked_i32(i128::from(a.y0) + my_offset)?;
                (d, mx, my)
            };
            if d == 0 { continue; }
            if point_in_poly(store, p, mx, my) == interior {
                out.push((d, mx, my));
            }
        }
    }
    Ok(out)
}

fn midpoint_i32(
    a: i32, b: i32,
) -> Result<i32, crate::geometry::exact::ExactGeometryError> {
    checked_i32((i128::from(a) + i128::from(b)) / 2)
}

fn checked_i32(value: i128) -> Result<i32, crate::geometry::exact::ExactGeometryError> {
    i32::try_from(value).map_err(|_| crate::geometry::exact::ExactGeometryError::ArithmeticOverflow)
}

fn round_ratio_i128(
    numerator: i128, denominator: i128,
) -> Result<i128, crate::geometry::exact::ExactGeometryError> {
    use crate::geometry::exact::ExactGeometryError;
    if denominator <= 0 { return Err(ExactGeometryError::ArithmeticOverflow); }
    let half = denominator / 2;
    if numerator >= 0 {
        numerator
            .checked_add(half)
            .map(|value| value / denominator)
            .ok_or(ExactGeometryError::ArithmeticOverflow)
    } else {
        numerator
            .checked_sub(half)
            .map(|value| value / denominator)
            .ok_or(ExactGeometryError::ArithmeticOverflow)
    }
}

fn push_geometry_capacity(
    store: &GeometryStore, lt: &LayerTable, polygon: PolyId, out: &mut Vec<Violation>,
) {
    let bbox = store.poly_bbox[polygon.0 as usize];
    out.push(Violation {
        rule_id: "__geometry__".into(), kind: "geometry_capacity".into(),
        layer: lt.name(store.poly_layer[polygon.0 as usize]).into(),
        measured: 1, limit: 0, x: bbox.xmin, y: bbox.ymin,
    });
}

/// Is polygon `inner` strictly inside polygon `outer`? All vertices strictly interior —
/// exact for the disjoint-boundary cases this is used on (a touching boundary counts as
/// "not strictly inside", which is the conservative answer for both callers).
fn poly_strictly_inside(store: &GeometryStore, inner: PolyId, outer: PolyId) -> bool {
    let polygon = |poly: PolyId| {
        crate::geometry::exact::Polygon::from_outer(
            store.vertices(poly)
                .map(|(x, y)| crate::geometry::exact::Point::new(x, y))
                .collect(),
        )
    };
    let (Ok(inner), Ok(outer)) = (polygon(inner), polygon(outer)) else {
        return false;
    };
    let inner_ring = inner.outer().vertices();
    let outer_ring = outer.outer().vertices();
    inner_ring.iter().all(|&point| {
        outer.classify_point(point) == crate::geometry::exact::PointClassification::Inside
    }) && (0..inner_ring.len()).all(|i| {
        (0..outer_ring.len()).all(|j| {
            crate::geometry::exact::classify_segment_intersection(
                inner_ring[i], inner_ring[(i + 1) % inner_ring.len()],
                outer_ring[j], outer_ring[(j + 1) % outer_ring.len()],
            ) == crate::geometry::exact::SegmentIntersection::None
        })
    })
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
    let sweep_x = i64::from(xhi) - i64::from(xlo) >= i64::from(yhi) - i64::from(ylo);
    let min = i64::from(min);

    // (sweep_min, sweep_max, poly, from_b)
    let mut items: Vec<(i64, i64, PolyId, bool)> = Vec::with_capacity(
        pa.len() + pb.map_or(0, |b| b.len()));
    let key = |b: &Bbox| {
        let (lo, hi) = if sweep_x { (b.xmin, b.xmax) } else { (b.ymin, b.ymax) };
        (i64::from(lo), i64::from(hi))
    };
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
            .take_while(|it| it.0 <= hi_i + min)
        {
            if pb.is_some() && bi == bj { continue; } // cross-set pairs only
            let ba = store.poly_bbox[pi.0 as usize];
            let bb = store.poly_bbox[pj.0 as usize];
            let other_near = if sweep_x {
                i64::from(ba.ymin) - min <= i64::from(bb.ymax)
                    && i64::from(bb.ymin) - min <= i64::from(ba.ymax)
            } else {
                i64::from(ba.xmin) - min <= i64::from(bb.xmax)
                    && i64::from(bb.xmin) - min <= i64::from(ba.xmax)
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
pub fn same_shape_gap_fills(
    store: &GeometryStore,
    deck: &Deck,
) -> Result<Vec<(LayerId, Bbox)>, crate::geometry::exact::ExactGeometryError> {
    if store.poly_count() > 0 {
        let mut extent = Bbox::empty();
        for bbox in &store.poly_bbox {
            extent.include(bbox.xmin, bbox.ymin);
            extent.include(bbox.xmax, bbox.ymax);
        }
        let required = extent.width_i64().max(extent.height_i64()).max(0);
        if required > i64::from(i32::MAX) || !legacy_rule_arithmetic_fits(&extent) {
            return Err(crate::geometry::exact::ExactGeometryError::CapacityExceeded {
                cells: usize::try_from(required).unwrap_or(usize::MAX),
                limit: i32::MAX as usize,
            });
        }
    }
    let mut fills = Vec::new();
    for r in &deck.drc_rules {
        let DrcRuleParam::MinSpacing { layer, min, .. } = r else { continue };
        let (layer, min) = (*layer, *min);
        let polys: Vec<PolyId> = store.polys_on_layer(layer).collect();
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
                let h = i64::from(min / 2);
                let clamp = |value: i64| {
                    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
                };
                let fill = Bbox {
                    xmin: clamp(i64::from(g.xmin) - h),
                    ymin: clamp(i64::from(g.ymin) - h),
                    xmax: clamp(i64::from(g.xmax) + h),
                    ymax: clamp(i64::from(g.ymax) + h),
                };
                let required = fill.width_i64().max(fill.height_i64()).max(0);
                if required > i64::from(i32::MAX) {
                    return Err(crate::geometry::exact::ExactGeometryError::CapacityExceeded {
                        cells: usize::try_from(required).unwrap_or(usize::MAX),
                        limit: i32::MAX as usize,
                    });
                }
                fills.push((layer, fill));
            }
        }
    }
    Ok(fills)
}

// --- same-layer spacing (external) ------------------------------------------
fn check_spacing_same(
    store: &GeometryStore, lt: &LayerTable, layer: LayerId, min: i32, backend: Backend,
    strict: bool, rule_id: &str, out: &mut Vec<Violation>,
) {
    let polys: Vec<PolyId> = store.polys_on_layer(layer).collect();
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
    let pas: Vec<PolyId> = store.polys_on_layer(a).collect();
    let pbs: Vec<PolyId> = store.polys_on_layer(b).collect();
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
    poly_poly_dist2_within_wide(store, pa, pb, i64::from(cutoff))
}

fn poly_poly_dist2_within_wide(
    store: &GeometryStore, pa: PolyId, pb: PolyId, cutoff: i64,
) -> i64 {
    let ea = poly_edges(store, pa);
    let eb = poly_edges(store, pb);
    let cutoff = cutoff.max(1);
    let cut2 = cutoff.checked_mul(cutoff).unwrap_or(i64::MAX);

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
    let cell = cutoff.saturating_mul(4).max(512);
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
        let (kx0, ky0) = key(ax0.saturating_sub(cutoff), ay0.saturating_sub(cutoff));
        let (kx1, ky1) = key(ax1.saturating_add(cutoff), ay1.saturating_add(cutoff));
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
    store.edges_of(p).collect()
}

// --- enclosure (outer must enclose inner by >= min on all sides) ------------
fn check_enclosure(
    store: &GeometryStore, lt: &LayerTable, outer: LayerId, inner: LayerId, min: i32,
    backend: Backend, rule_id: &str, out: &mut Vec<Violation>,
) {
    let outers: Vec<PolyId> = store.polys_on_layer(outer).collect();
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
        let worst = isqrt(poly_poly_dist2_within_wide(
            store,
            pi,
            po,
            i64::from(min) + 1,
        ));
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
    let refs: Vec<PolyId> = store.polys_on_layer(reference).collect();
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
    // Real DRC measures connected union components. Summing fragment areas is
    // a false-clean when fragments overlap, so use the shared exact boolean.
    let merged = match derived::layer_polygon_set(store, layer, Some(lt.id_to_name.len())) {
        Ok(merged) => merged,
        Err(_) => {
            let marker = store.polys_on_layer(layer).next()
                .map(|poly| store.poly_bbox[poly.0 as usize])
                .unwrap_or(Bbox { xmin: 0, ymin: 0, xmax: 0, ymax: 0 });
            out.push(Violation {
                rule_id: rule_id.into(), kind: "min_area_geometry_error".into(),
                layer: lt.name(layer).into(), measured: -1, limit: min,
                x: marker.xmin, y: marker.ymin,
            });
            return;
        }
    };
    for polygon in merged.polygons() {
        let area = polygon.area2() / 2;
        if area < i128::from(min) {
            let marker = polygon.outer().vertices()[0];
            out.push(Violation {
                rule_id: rule_id.into(), kind: "min_area".into(),
                layer: lt.name(layer).into(),
                measured: i64::try_from(area).unwrap_or(i64::MAX), limit: min,
                x: marker.x, y: marker.y,
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
    let polys: Vec<PolyId> = store.polys_on_layer(layer).collect();
    let clean = gpu_poly_clean_mask(store, &polys, min, backend);
    for (k, &p) in polys.iter().enumerate() {
        if clean.as_ref().is_some_and(|c| c[k]) { continue; }
        // a notch is a facing pair whose gap is OUTSIDE the polygon (see facing_gaps);
        // interior pairs are widths and belong to min_width, not here.
        let gaps = match facing_gaps(store, p, false) {
            Ok(gaps) => gaps,
            Err(_) => {
                push_geometry_capacity(store, lt, p, out);
                return;
            }
        };
        for (d, mx, my) in gaps {
            if d < i64::from(min) {
                out.push(Violation {
                    rule_id: rule_id.into(), kind: "notch".into(),
                    layer: lt.name(layer).into(), measured: d,
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
        let l2 = e.len2_i128();
        if l2 > 0 && l2 < i128::from(min2) {
            out.push(Violation {
                rule_id: rule_id.into(), kind: "min_edge_length".into(),
                layer: lt.name(layer).into(),
                measured: isqrt(i64::try_from(l2).unwrap_or(i64::MAX)), limit: min as i64,
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
    let polys: Vec<PolyId> = store.polys_on_layer(layer).collect();
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
    let window = i64::from(window);
    let win_area = (window as f64) * (window as f64);
    let Some(nx) = gb
        .width_i64()
        .checked_add(window - 1)
        .map(|span| span / window)
    else {
        push_geometry_capacity(store, lt, polys[0], out);
        return;
    };
    let Some(ny) = gb
        .height_i64()
        .checked_add(window - 1)
        .map(|span| span / window)
    else {
        push_geometry_capacity(store, lt, polys[0], out);
        return;
    };
    if nx <= 0 || ny <= 0 { return; }
    let Some(window_count) = nx.checked_mul(ny) else {
        push_geometry_capacity(store, lt, polys[0], out);
        return;
    };
    if window_count
        > i64::try_from(crate::geometry::exact::MAX_RECTILINEAR_BOOLEAN_CELLS)
            .unwrap_or(i64::MAX)
    {
        push_geometry_capacity(store, lt, polys[0], out);
        return;
    }
    let mut covered: std::collections::HashMap<(i64, i64), f64> = std::collections::HashMap::new();
    for &p in &polys {
        let b = store.poly_bbox[p.0 as usize];
        let wi0 = (i64::from(b.xmin) - i64::from(gb.xmin)) / window;
        let wi1 = (i64::from(b.xmax) - i64::from(gb.xmin) - 1) / window;
        let wj0 = (i64::from(b.ymin) - i64::from(gb.ymin)) / window;
        let wj1 = (i64::from(b.ymax) - i64::from(gb.ymin) - 1) / window;
        for wi in wi0..=wi1.min(nx - 1) {
            for wj in wj0..=wj1.min(ny - 1) {
                let Some((wx0, wy0, wx1, wy1)) =
                    density_window_bounds(gb, wi, wj, window)
                else {
                    push_geometry_capacity(store, lt, p, out);
                    return;
                };
                let a = clipped_area_i64(store, p, wx0, wy0, wx1, wy1);
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
                let Some((wx0, wy0, _, _)) = density_window_bounds(gb, wi, wj, window)
                else {
                    push_geometry_capacity(store, lt, polys[0], out);
                    return;
                };
                let (Ok(x), Ok(y)) = (i32::try_from(wx0), i32::try_from(wy0)) else {
                    push_geometry_capacity(store, lt, polys[0], out);
                    return;
                };
                out.push(Violation {
                    rule_id: rule_id.into(),
                    kind: if is_min { "min_density".into() } else { "max_density".into() },
                    layer: lt.name(layer).into(),
                    measured: (frac * 1_000_000.0) as i64, // frac scaled to ppm to fit i64
                    limit: (frac_limit * 1_000_000.0) as i64,
                    x,
                    y,
                });
            }
        }
    }
}

fn density_window_bounds(
    global: Bbox, wi: i64, wj: i64, window: i64,
) -> Option<(i64, i64, i64, i64)> {
    let wx0 = wi.checked_mul(window)?.checked_add(i64::from(global.xmin))?;
    let wy0 = wj.checked_mul(window)?.checked_add(i64::from(global.ymin))?;
    Some((wx0, wy0, wx0.checked_add(window)?, wy0.checked_add(window)?))
}

// --- overlap (two layers must overlap by >= min) ----------------------------
fn check_overlap(
    store: &GeometryStore, lt: &LayerTable, a: LayerId, b: LayerId, min: i32,
    rule_id: &str, out: &mut Vec<Violation>,
) {
    let b_union = match derived::layer_polygon_set(store, b, Some(lt.id_to_name.len())) {
        Ok(set) => set,
        Err(_) => {
            out.push(Violation {
                rule_id: rule_id.into(), kind: "overlap_geometry_error".into(),
                layer: format!("{}:{}", lt.name(a), lt.name(b)),
                measured: -1, limit: min as i64, x: 0, y: 0,
            });
            return;
        }
    };
    for pa in store.polys_on_layer(a) {
        let polygon = crate::geometry::exact::Polygon::from_outer(
            store.vertices(pa)
                .map(|(x, y)| crate::geometry::exact::Point::new(x, y))
                .collect(),
        );
        let marker = store.poly_bbox[pa.0 as usize];
        let intersection = polygon
            .map(crate::geometry::exact::PolygonSet::from_polygon)
            .map_err(derived::DerivedError::from)
            .and_then(|a_set| {
                crate::geometry::exact::rectilinear_intersection(&a_set, &b_union)
                    .map_err(derived::DerivedError::from)
            });
        let Ok(intersection) = intersection else {
            out.push(Violation {
                rule_id: rule_id.into(), kind: "overlap_geometry_error".into(),
                layer: format!("{}:{}", lt.name(a), lt.name(b)),
                measured: -1, limit: min as i64, x: marker.xmin, y: marker.ymin,
            });
            continue;
        };
        let mut best = 0_i32;
        let mut unsupported = false;
        for component in intersection.polygons() {
            if !component.holes().is_empty() || component.outer().vertices().len() != 4 {
                unsupported = true;
                break;
            }
            let mut xs: Vec<_> = component.outer().vertices().iter().map(|p| p.x).collect();
            let mut ys: Vec<_> = component.outer().vertices().iter().map(|p| p.y).collect();
            xs.sort_unstable(); xs.dedup();
            ys.sort_unstable(); ys.dedup();
            if xs.len() != 2 || ys.len() != 2 {
                unsupported = true;
                break;
            }
            best = best.max((xs[1] - xs[0]).min(ys[1] - ys[0]));
        }
        if unsupported {
            out.push(Violation {
                rule_id: rule_id.into(), kind: "overlap_geometry_error".into(),
                layer: format!("{}:{}", lt.name(a), lt.name(b)),
                measured: -1, limit: min as i64, x: marker.xmin, y: marker.ymin,
            });
        } else if best < min {
            out.push(Violation {
                rule_id: rule_id.into(), kind: "overlap".into(),
                layer: format!("{}:{}", lt.name(a), lt.name(b)),
                measured: best as i64, limit: min as i64,
                x: marker.xmin, y: marker.ymin,
            });
        }
    }
}

// --- corner to corner -------------------------------------------------------
fn check_corner_to_corner(
    store: &GeometryStore, lt: &LayerTable, layer: LayerId, min: i32, backend: Backend,
    rule_id: &str, out: &mut Vec<Violation>,
) {
    let polys: Vec<PolyId> = store.polys_on_layer(layer).collect();
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
            let mut best = i64::MAX;
            let mut bx = 0;
            let mut by = 0;
            for (ux, uy) in store.vertices(pa) {
                for (wx, wy) in store.vertices(pb) {
                    let dx = (ux - wx) as i64;
                    let dy = (uy - wy) as i64;
                    let d2 = dx * dx + dy * dy;
                    if d2 < best { best = d2; bx = ux; by = uy; }
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
    let polys: Vec<PolyId> = store.polys_on_layer(layer).collect();
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
                let elen2 = a.len2_i128();
                if elen2 == 0
                    || elen2 >= i128::from(eol_width) * i128::from(eol_width)
                {
                    continue;
                }
                // manhattan outward normal (diagonal EOL edges: skip, no zone defined)
                let (udx, udy) = (a.dx().signum() as i32, a.dy().signum() as i32);
                if udx != 0 && udy != 0 { continue; }
                let (nx, ny) = if ccw { (udy, -udx) } else { (-udy, udx) };
                let zone = Bbox {
                    xmin: a.x0.min(a.x1).saturating_add(nx.min(0) * eol_spacing),
                    xmax: a.x0.max(a.x1).saturating_add(nx.max(0) * eol_spacing),
                    ymin: a.y0.min(a.y1).saturating_add(ny.min(0) * eol_spacing),
                    ymax: a.y0.max(a.y1).saturating_add(ny.max(0) * eol_spacing),
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
    let polys: Vec<PolyId> = store.polys_on_layer(layer).collect();
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
    let polys: Vec<PolyId> = store.polys_on_layer(layer).collect();
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
    let outers: Vec<PolyId> = store.polys_on_layer(outer).collect();
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

// Extract actual keyhole cycles from a single GDS boundary walk. Separate
// same-polarity boundaries are filled material, never negative-space evidence.
fn keyhole_hole_rings(
    store: &GeometryStore,
    polygon: PolyId,
) -> Vec<crate::geometry::exact::Ring> {
    store.poly_as_exact(polygon)
        .map(|component| component.holes().to_vec())
        .unwrap_or_default()
}

// --- min enclosed area (actual hole/keyhole area) ----------------------------
fn check_min_enclosed_area(
    store: &GeometryStore, lt: &LayerTable, layer: LayerId, min_hole_area: i64,
    rule_id: &str, out: &mut Vec<Violation>,
) {
    for polygon in store.polys_on_layer(layer) {
        for hole in keyhole_hole_rings(store, polygon) {
            let area = hole.signed_area2().abs() / 2;
            if area < i128::from(min_hole_area) {
                let marker = hole.vertices()[0];
                out.push(Violation {
                    rule_id: rule_id.into(), kind: "min_enclosed_area".into(),
                    layer: lt.name(layer).into(),
                    measured: i64::try_from(area).unwrap_or(i64::MAX),
                    limit: min_hole_area, x: marker.x, y: marker.y,
                });
            }
        }
    }
}

// --- cheesing (large unslotted plates) ---------------------------------------
// Any over-limit polygon must carry actual hole/keyhole evidence in its own
// boundary. Nested same-polarity polygons are material and cannot waive it.
fn check_cheesing(
    store: &GeometryStore, lt: &LayerTable, layer: LayerId, max_area_no_slot: i64,
    rule_id: &str, out: &mut Vec<Violation>,
) {
    for p in store.polys_on_layer(layer) {
        let area = store.area(p);
        if area > max_area_no_slot && keyhole_hole_rings(store, p).is_empty() {
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
    let polys: Vec<PolyId> = store.polys_on_layer(layer).collect();
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
    let polys: Vec<PolyId> = store.polys_on_layer(layer).collect();
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
        let d2 = poly_poly_dist2_within_wide(
            store,
            pa,
            pb,
            i64::from(array_spacing) + 1,
        );
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
        let d2 = poly_poly_dist2_within_wide(
            store,
            pa,
            pb,
            i64::from(array_spacing) + 1,
        );
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
    let md2 = (max_dist as i64) * (max_dist as i64);
    // precompute tap bbox centers
    let tap_centers: Vec<(i64, i64)> = store.polys_on_layer(tap_layer).map(|t| {
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

// --- multi-patterning (complete bounded graph coloring) ----------------------
// Build a conflict graph: two polygons within color_spacing are "conflicting"
// (can't share a color). The shared DSATUR/backtracking solver is complete within
// its declared node/search bounds. Capacity exhaustion is a fail-closed marker,
// never treated as evidence that the graph is clean or uncolorable.
fn check_multi_patterning(
    store: &GeometryStore, lt: &LayerTable, layer: LayerId, num_colors: i32,
    color_spacing: i32, rule_id: &str, out: &mut Vec<Violation>,
) {
    let polys: Vec<PolyId> = store.polys_on_layer(layer).collect();
    let n = polys.len();
    if n == 0 { return; }
    let cs2 = (color_spacing as i64) * (color_spacing as i64);
    let cands = candidate_pairs(store, &polys, None, color_spacing);
    let idx_of: std::collections::HashMap<u32, usize> =
        polys.iter().enumerate().map(|(i, p)| (p.0, i)).collect();
    let mut conflicts = Vec::new();
    for &(pa, pb) in &cands {
        let d2 = poly_poly_dist2_within(store, pa, pb, color_spacing);
        if d2 > 0 && d2 < cs2 {
            let ia = idx_of[&pa.0];
            let ib = idx_of[&pb.0];
            conflicts.push((ia, ib));
        }
    }
    let problem = coloring::ColoringProblem::new(n, num_colors as usize, conflicts);
    match coloring::solve_coloring(&problem) {
        Ok(_) => {}
        Err(error) => {
            let (kind, witness) = match error {
                coloring::ColoringError::Uncolorable { witness } => {
                    ("multi_patterning", witness.first().copied().unwrap_or(0))
                }
                coloring::ColoringError::CapacityExceeded { .. }
                | coloring::ColoringError::SearchLimit { .. }
                | coloring::ColoringError::Invalid(_) => ("multi_patterning_error", 0),
            };
            let bb = store.poly_bbox[polys[witness.min(n - 1)].0 as usize];
            out.push(Violation {
                rule_id: rule_id.into(), kind: kind.into(),
                layer: lt.name(layer).into(), measured: num_colors as i64,
                limit: num_colors as i64, x: bb.xmin, y: bb.ymin,
            });
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

    #[test]
    fn same_shape_gap_repairs_saturate_at_min_and_report_wide_capacity() {
        let deck = Deck::from_json(
            r#"{"layers":{"m1":{"layer":1,"datatype":0}},"drc":{
                "S":{"kind":"min_spacing","layer":"m1","min":10}}}"#,
        )
        .unwrap();
        let layer = deck.layers.id("m1").unwrap();

        let base = i32::MIN + 1;
        let mut near_min = GeometryStore::new();
        near_min.add_rect(layer, base, base, 10, 10);
        near_min.add_rect(layer, base + 15, base, 10, 10);
        near_min.add_rect(layer, base, base + 8, 25, 2);
        let fills = same_shape_gap_fills(&near_min, &deck).unwrap();
        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].1.ymin, i32::MIN);

        let mut too_wide = GeometryStore::new();
        too_wide.add_polygon(layer, &[
            (0, i32::MIN), (10, i32::MIN), (10, i32::MAX), (0, i32::MAX),
        ]);
        too_wide.add_polygon(layer, &[
            (15, i32::MIN), (25, i32::MIN), (25, i32::MAX), (15, i32::MAX),
        ]);
        too_wide.add_rect(layer, 0, 0, 25, 1);
        assert!(matches!(
            same_shape_gap_fills(&too_wide, &deck),
            Err(crate::geometry::exact::ExactGeometryError::CapacityExceeded { .. })
        ));
    }

    #[test]
    fn min_area_uses_boolean_union_and_overlap_requires_a_counterpart() {
        let mut defs = std::collections::HashMap::new();
        defs.insert("a".to_string(), crate::params::LayerDef { layer: 1, datatype: 0 });
        defs.insert("b".to_string(), crate::params::LayerDef { layer: 2, datatype: 0 });
        let lt = LayerTable::from_defs(&defs);
        let (a, b) = (lt.id("a").unwrap(), lt.id("b").unwrap());
        let mut store = GeometryStore::new();
        store.add_rect(a, 0, 0, 10, 10);
        store.add_rect(a, 5, 0, 10, 10);

        let mut violations = Vec::new();
        check_min_area(&store, &lt, a, 175, "A.MIN", &mut violations);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].measured, 150);

        violations.clear();
        check_overlap(&store, &lt, a, b, 2, "A.OVERLAP.B", &mut violations);
        assert_eq!(violations.len(), 2);
        assert!(violations.iter().all(|violation| violation.measured == 0));
    }

    #[test]
    fn overlap_uses_exact_contact_not_concave_bboxes() {
        let mut defs = std::collections::HashMap::new();
        defs.insert("a".to_string(), crate::params::LayerDef { layer: 1, datatype: 0 });
        defs.insert("b".to_string(), crate::params::LayerDef { layer: 2, datatype: 0 });
        let lt = LayerTable::from_defs(&defs);
        let (a, b) = (lt.id("a").unwrap(), lt.id("b").unwrap());
        let mut store = GeometryStore::new();
        store.add_polygon(a, &[
            (0, 0), (10, 0), (10, 2), (2, 2), (2, 10), (0, 10),
        ]);
        // This rectangle is inside the L-shape's bbox, but in its empty concavity.
        store.add_rect(b, 5, 5, 3, 3);
        let mut violations = Vec::new();
        check_overlap(&store, &lt, a, b, 2, "A.OVERLAP.B", &mut violations);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].measured, 0);
    }

    #[test]
    fn only_actual_keyhole_cycles_count_as_holes_or_slots() {
        let mut defs = std::collections::HashMap::new();
        defs.insert("m1".to_string(), crate::params::LayerDef { layer: 1, datatype: 0 });
        let lt = LayerTable::from_defs(&defs);
        let m1 = lt.id("m1").unwrap();

        let mut nested_material = GeometryStore::new();
        nested_material.add_rect(m1, 0, 0, 20, 20);
        nested_material.add_rect(m1, 8, 8, 4, 4);
        let mut violations = Vec::new();
        check_min_enclosed_area(&nested_material, &lt, m1, 40, "M1.HOLE", &mut violations);
        assert!(violations.is_empty(), "same-polarity material became a hole");
        check_cheesing(&nested_material, &lt, m1, 100, "M1.SLOT", &mut violations);
        assert_eq!(violations.len(), 1, "nested material waived cheesing");

        let mut keyhole = GeometryStore::new();
        keyhole.add_polygon(m1, &[
            (0, 0), (20, 0), (20, 20), (12, 20), (12, 16),
            (16, 16), (16, 12), (8, 12), (8, 16), (12, 16),
            (12, 20), (0, 20),
        ]);
        violations.clear();
        check_min_enclosed_area(&keyhole, &lt, m1, 40, "M1.HOLE", &mut violations);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].measured, 32);
        violations.clear();
        check_cheesing(&keyhole, &lt, m1, 100, "M1.SLOT", &mut violations);
        assert!(violations.is_empty());
    }
}
