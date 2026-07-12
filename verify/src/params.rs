//! Parameter-deck construction from a [`VerifySchema`].

use crate::geometry::LayerId;
use crate::schema::{PropertyTolerance, VerifySchema};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayerDef {
    pub layer: i32,
    pub datatype: i32,
}

/// Resolves symbolic layer names (e.g. "met1") to internal `LayerId`s and to GDS
/// (layer,datatype) pairs. Built once, then shared read-only by every checker.
#[derive(Debug, Default)]
pub struct LayerTable {
    pub name_to_id: HashMap<String, LayerId>,
    pub id_to_name: Vec<String>,
    pub id_to_gds: Vec<(i32, i32)>,
    pub gds_to_id: HashMap<(i32, i32), LayerId>,
}

impl LayerTable {
    pub fn from_defs(defs: &HashMap<String, LayerDef>) -> Self {
        // deterministic order: sort by (layer,datatype) so ids are stable across runs
        let mut items: Vec<(&String, &LayerDef)> = defs.iter().collect();
        items.sort_by_key(|(_, d)| (d.layer, d.datatype));
        let mut t = LayerTable::default();
        for (name, d) in items {
            let id = t.id_to_name.len() as LayerId;
            t.name_to_id.insert(name.clone(), id);
            t.id_to_name.push(name.clone());
            t.id_to_gds.push((d.layer, d.datatype));
            t.gds_to_id.insert((d.layer, d.datatype), id);
        }
        t
    }

    pub fn id(&self, name: &str) -> Option<LayerId> {
        self.name_to_id.get(name).copied()
    }
    pub fn name(&self, id: LayerId) -> &str {
        &self.id_to_name[id as usize]
    }
    pub fn from_gds(&self, layer: i32, datatype: i32) -> Option<LayerId> {
        self.gds_to_id.get(&(layer, datatype)).copied()
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct PexLayerParams {
    pub sheet_res_ohm_sq: f64,
    pub area_cap_af_um2: f64,
    pub fringe_cap_af_um: f64,
    pub coupling_cap_af_um: f64,
    pub coupling_ref_spacing_nm: f64,
    /// Fixed per-via resistance (ohm). When set, every polygon on this layer
    /// gets this R regardless of geometry. Used for via/contact layers.
    #[serde(default)]
    pub via_res_ohm: f64,
    #[serde(default)]
    pub interlayer_cap_af_um2: f64,
}

/// Resolved connectivity: LayerIds instead of strings.
#[derive(Clone, Default)]
pub struct ConnectivityConfig {
    pub conductors: Vec<LayerId>,
    pub vias: Vec<(LayerId, Vec<LayerId>)>,
}

/// Resolved MOS device rule.
#[derive(Clone)]
pub struct MosRule {
    pub name: String,
    pub gate_layer: LayerId,
    pub channel_layer: LayerId,
    pub type_implant: LayerId,
    pub device_type: String,
    pub flavor_markers: Vec<(LayerId, String)>,
    pub well_layer: Option<LayerId>,
    pub device_class: Option<String>,
}

/// Resolved BJT device rule.
#[derive(Clone)]
pub struct BjtRule {
    pub name: String,
    pub collector_layer: LayerId,
    pub base_layer: LayerId,
    pub emitter_layer: LayerId,
    pub type_marker: LayerId,
    pub device_type: String,
}

/// Resolved two-terminal device rules.
#[derive(Clone)]
pub struct ResistorRule {
    pub name: String,
    pub body_layer: LayerId,
    pub marker_layer: LayerId,
    pub terminal_layer: LayerId,
}

#[derive(Clone)]
pub struct DiodeRule {
    pub name: String,
    pub anode_layer: LayerId,
    pub cathode_layer: LayerId,
    pub implant_layer: LayerId,
}

#[derive(Clone)]
pub struct CapRule {
    pub name: String,
    pub top_layer: LayerId,
    pub bottom_layer: LayerId,
    pub marker_layer: Option<LayerId>,
}

/// Resolved device config.
#[derive(Clone, Default)]
pub struct DeviceConfig {
    pub mos_rules: Vec<MosRule>,
    pub bjt_rules: Vec<BjtRule>,
    pub resistor_rules: Vec<ResistorRule>,
    pub diode_rules: Vec<DiodeRule>,
    pub cap_rules: Vec<CapRule>,
}

/// ERC thresholds. Every field defaults to today's built-in value, so decks
/// without an "erc" section behave identically; override per PDK in params.json.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErcParams {
    /// antenna_electrical: cumulative metal area / gate area limit.
    #[serde(default = "d_erc_antenna")]
    pub antenna_ratio: f64,
    /// em_current_density: minimum metal width on current-carrying nets (nm).
    #[serde(default = "d_erc_em_w")]
    pub em_min_width_nm: i32,
    /// p2p_resistance: per-net total wire resistance limit (ohm).
    #[serde(default = "d_erc_p2p")]
    pub p2p_r_limit_ohm: f64,
    /// missing_tie: max distance from a diff corner to the nearest tap (nm).
    #[serde(default = "d_erc_tie")]
    pub tie_max_dist_nm: i32,
}
fn d_erc_antenna() -> f64 {
    200.0
}
fn d_erc_em_w() -> i32 {
    200
}
fn d_erc_p2p() -> f64 {
    10.0
}
fn d_erc_tie() -> i32 {
    2000
}
impl Default for ErcParams {
    fn default() -> Self {
        ErcParams {
            antenna_ratio: d_erc_antenna(),
            em_min_width_nm: d_erc_em_w(),
            p2p_r_limit_ohm: d_erc_p2p(),
            tie_max_dist_nm: d_erc_tie(),
        }
    }
}

/// The fully-resolved deck the checkers consume.
pub struct Deck {
    pub layers: LayerTable,
    pub drc_rules: Vec<DrcRuleParam>,
    pub pex: HashMap<LayerId, PexLayerParams>,
    pub dbu_nm: f64,
    /// LVS extraction requires explicit cut (via) shapes to connect layers.
    pub lvs_cut_required: bool,
    /// STRICT mode: off-grid → hard error, acute angle → hard error,
    /// same-net spacing enforced. Default false (nominal mode).
    pub strict: bool,
    /// PDK-driven connectivity. Empty → legacy hardcoded fallback.
    pub connectivity: ConnectivityConfig,
    /// PDK-driven device recognition. Empty → legacy hardcoded MOS-only.
    pub devices: DeviceConfig,
    pub w_tolerance: PropertyTolerance,
    pub l_tolerance: PropertyTolerance,
    pub fail_on_floating: bool,
    pub intra_layer_touch: bool,
    pub global_nets: Vec<String>,
    /// ERC thresholds (all defaulted; see [`ErcParams`]).
    pub erc: ErcParams,
}

/// One resolved DRC rule. This is a tagged union (enum) rather than a trait object: the DOD
/// skill's "tagged unions over virtual dispatch" — the rule engine `match`es over this in a
/// tight loop with no vtable indirection.
#[derive(Debug, Clone)]
pub enum DrcRuleParam {
    MinWidth {
        id: String,
        layer: LayerId,
        min: i32,
    },
    MinSpacing {
        id: String,
        layer: LayerId,
        min: i32,
    },
    MinSpacingDiff {
        id: String,
        a: LayerId,
        b: LayerId,
        min: i32,
    },
    MinEnclosure {
        id: String,
        outer: LayerId,
        inner: LayerId,
        min: i32,
    },
    MinExtension {
        id: String,
        layer: LayerId,
        reference: LayerId,
        min: i32,
    },
    MinArea {
        id: String,
        layer: LayerId,
        min: i64,
    },
    MaxWidth {
        id: String,
        layer: LayerId,
        max: i32,
    },
    Notch {
        id: String,
        layer: LayerId,
        min: i32,
    },
    MinEdgeLength {
        id: String,
        layer: LayerId,
        min: i32,
    },
    OffGrid {
        id: String,
        grid: i32,
    },
    Angle {
        id: String,
        allowed: Vec<i32>,
    },
    MinDensity {
        id: String,
        layer: LayerId,
        window: i32,
        min_frac: f64,
    },
    MaxDensity {
        id: String,
        layer: LayerId,
        window: i32,
        max_frac: f64,
    },
    Overlap {
        id: String,
        a: LayerId,
        b: LayerId,
        min: i32,
    },
    CornerToCorner {
        id: String,
        layer: LayerId,
        min: i32,
    },
    Antenna {
        id: String,
        layer: LayerId,
        ratio: f64,
    },
    /// Cumulative antenna ratio: at each metal-stack stage k, the charge-collecting
    /// area of layers[0..=k] connected to a gate (with only vias up to that stage)
    /// must stay under `ratio` × gate area — unless a diode-layer shape is on the net.
    AntennaCar {
        id: String,
        layers: Vec<LayerId>,
        ratio: f64,
        diode: Option<LayerId>,
    },
    EolSpacing {
        id: String,
        layer: LayerId,
        eol_width: i32,
        eol_spacing: i32,
    },
    WideDependentSpacing {
        id: String,
        layer: LayerId,
        width_threshold: i32,
        wide_spacing: i32,
    },
    PrlSpacing {
        id: String,
        layer: LayerId,
        prl_threshold: i32,
        prl_spacing: i32,
    },
    AsymmetricEnclosure {
        id: String,
        outer: LayerId,
        inner: LayerId,
        min_one_side: i32,
    },
    MinEnclosedArea {
        id: String,
        layer: LayerId,
        min_hole_area: i64,
    },
    Cheesing {
        id: String,
        layer: LayerId,
        max_area_no_slot: i64,
    },
    RedundantVia {
        id: String,
        layer: LayerId,
        min_count: i32,
        within: i32,
    },
    ViaArraySpacing {
        id: String,
        layer: LayerId,
        array_threshold: i32,
        array_spacing: i32,
    },
    MaxDistanceToTap {
        id: String,
        diff_layer: LayerId,
        tap_layer: LayerId,
        max_dist: i32,
    },
    MultiPatterning {
        id: String,
        layer: LayerId,
        num_colors: i32,
        color_spacing: i32,
    },
}

impl DrcRuleParam {
    pub fn id(&self) -> &str {
        use DrcRuleParam::*;
        match self {
            MinWidth { id, .. }
            | MinSpacing { id, .. }
            | MinSpacingDiff { id, .. }
            | MinEnclosure { id, .. }
            | MinExtension { id, .. }
            | MinArea { id, .. }
            | MaxWidth { id, .. }
            | Notch { id, .. }
            | MinEdgeLength { id, .. }
            | OffGrid { id, .. }
            | Angle { id, .. }
            | MinDensity { id, .. }
            | MaxDensity { id, .. }
            | Overlap { id, .. }
            | CornerToCorner { id, .. }
            | Antenna { id, .. }
            | AntennaCar { id, .. }
            | EolSpacing { id, .. }
            | WideDependentSpacing { id, .. }
            | PrlSpacing { id, .. }
            | AsymmetricEnclosure { id, .. }
            | MinEnclosedArea { id, .. }
            | Cheesing { id, .. }
            | RedundantVia { id, .. }
            | ViaArraySpacing { id, .. }
            | MaxDistanceToTap { id, .. }
            | MultiPatterning { id, .. } => id,
        }
    }
}

fn drc_rule_error(id: &str, kind: &str, message: impl std::fmt::Display) -> String {
    format!("DRC rule `{id}` ({kind}): {message}")
}

fn required_positive_i32_from_i64(
    id: &str,
    kind: &str,
    field: &str,
    value: Option<i64>,
) -> Result<i32, String> {
    let value = value
        .ok_or_else(|| drc_rule_error(id, kind, format!("missing required `{field}` parameter")))?;
    if value <= 0 {
        return Err(drc_rule_error(
            id,
            kind,
            format!("`{field}` must be positive, got {value}"),
        ));
    }
    i32::try_from(value).map_err(|_| {
        drc_rule_error(
            id,
            kind,
            format!("`{field}` value {value} is outside the i32 range"),
        )
    })
}

fn required_positive_i32(
    id: &str,
    kind: &str,
    field: &str,
    value: Option<i32>,
) -> Result<i32, String> {
    let value = value
        .ok_or_else(|| drc_rule_error(id, kind, format!("missing required `{field}` parameter")))?;
    if value <= 0 {
        return Err(drc_rule_error(
            id,
            kind,
            format!("`{field}` must be positive, got {value}"),
        ));
    }
    Ok(value)
}

fn required_positive_i64(
    id: &str,
    kind: &str,
    field: &str,
    value: Option<i64>,
) -> Result<i64, String> {
    let value = value
        .ok_or_else(|| drc_rule_error(id, kind, format!("missing required `{field}` parameter")))?;
    if value <= 0 {
        return Err(drc_rule_error(
            id,
            kind,
            format!("`{field}` must be positive, got {value}"),
        ));
    }
    Ok(value)
}

fn required_positive_f64(
    id: &str,
    kind: &str,
    field: &str,
    value: Option<f64>,
) -> Result<f64, String> {
    let value = value
        .ok_or_else(|| drc_rule_error(id, kind, format!("missing required `{field}` parameter")))?;
    if !value.is_finite() || value <= 0.0 {
        return Err(drc_rule_error(
            id,
            kind,
            format!("`{field}` must be finite and positive, got {value}"),
        ));
    }
    Ok(value)
}

fn required_fraction(id: &str, kind: &str, field: &str, value: Option<f64>) -> Result<f64, String> {
    let value = value
        .ok_or_else(|| drc_rule_error(id, kind, format!("missing required `{field}` parameter")))?;
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(drc_rule_error(
            id,
            kind,
            format!("`{field}` must be finite and in [0, 1], got {value}"),
        ));
    }
    Ok(value)
}

impl Deck {
    /// Build a `Deck` from a filled [`VerifySchema`].
    pub fn from_schema(schema: VerifySchema) -> Result<Deck, String> {
        let mut gds_pairs = std::collections::HashMap::new();
        for (name, &(layer, datatype)) in &schema.layers {
            if name.trim().is_empty() {
                return Err("layer name must not be empty".into());
            }
            if layer < 0
                || layer > i32::from(i16::MAX)
                || datatype < 0
                || datatype > i32::from(i16::MAX)
            {
                return Err(format!(
                    "layer `{name}` has GDS pair ({layer}, {datatype}) outside the supported non-negative i16 range"
                ));
            }
            if let Some(previous) = gds_pairs.insert((layer, datatype), name.as_str()) {
                return Err(format!(
                    "layers `{previous}` and `{name}` share duplicate GDS pair ({layer}, {datatype})"
                ));
            }
        }
        let layer_defs: HashMap<String, LayerDef> = schema
            .layers
            .into_iter()
            .map(|(name, (layer, datatype))| (name, LayerDef { layer, datatype }))
            .collect();
        let layers = LayerTable::from_defs(&layer_defs);
        let lid = |name: Option<&str>| -> Result<LayerId, String> {
            let n = name.ok_or_else(|| "missing layer name".to_string())?;
            layers.id(n).ok_or_else(|| format!("unknown layer '{}'", n))
        };

        let mut drc_rules = Vec::new();
        let mut rule_ids = std::collections::HashSet::new();
        for r in &schema.drc_rules {
            if r.kind.trim().is_empty() {
                return Err("DRC rule kind must not be empty".into());
            }
            let id = if r.id.trim().is_empty() {
                r.kind.clone()
            } else {
                r.id.clone()
            };
            if id.trim().is_empty() {
                return Err("DRC rule ID must not be empty".into());
            }
            if !rule_ids.insert(id.clone()) {
                return Err(format!("duplicate DRC rule ID `{id}`"));
            }
            let kind = r.kind.as_str();
            let min_i32 = || required_positive_i32_from_i64(&id, kind, "min", r.min);
            let layer = || lid(r.layer.as_deref()).map_err(|e| drc_rule_error(&id, kind, e));
            let layer_b = || lid(r.layer_b.as_deref()).map_err(|e| drc_rule_error(&id, kind, e));
            let outer = || lid(r.outer.as_deref()).map_err(|e| drc_rule_error(&id, kind, e));
            let inner = || lid(r.inner.as_deref()).map_err(|e| drc_rule_error(&id, kind, e));

            let rule = match kind {
                "min_width" => DrcRuleParam::MinWidth {
                    id: id.clone(),
                    layer: layer()?,
                    min: min_i32()?,
                },
                "min_spacing" => DrcRuleParam::MinSpacing {
                    id: id.clone(),
                    layer: layer()?,
                    min: min_i32()?,
                },
                "min_spacing_diff" => DrcRuleParam::MinSpacingDiff {
                    id: id.clone(),
                    a: layer()?,
                    b: layer_b()?,
                    min: min_i32()?,
                },
                "min_enclosure" | "well_enclosure" => DrcRuleParam::MinEnclosure {
                    id: id.clone(),
                    outer: outer()?,
                    inner: inner()?,
                    min: min_i32()?,
                },
                "min_extension" => DrcRuleParam::MinExtension {
                    id: id.clone(),
                    layer: layer()?,
                    reference: lid(r.reference.as_deref())
                        .map_err(|e| drc_rule_error(&id, kind, e))?,
                    min: min_i32()?,
                },
                "min_area" => DrcRuleParam::MinArea {
                    id: id.clone(),
                    layer: layer()?,
                    min: required_positive_i64(&id, kind, "min", r.min)?,
                },
                "max_width" => DrcRuleParam::MaxWidth {
                    id: id.clone(),
                    layer: layer()?,
                    max: required_positive_i32(&id, kind, "max", r.max)?,
                },
                "notch" => DrcRuleParam::Notch {
                    id: id.clone(),
                    layer: layer()?,
                    min: min_i32()?,
                },
                "min_edge_length" => DrcRuleParam::MinEdgeLength {
                    id: id.clone(),
                    layer: layer()?,
                    min: min_i32()?,
                },
                "off_grid" => DrcRuleParam::OffGrid {
                    id: id.clone(),
                    grid: required_positive_i32(&id, kind, "grid", r.grid)?,
                },
                "angle" => {
                    let allowed = r.allowed.clone().ok_or_else(|| {
                        drc_rule_error(&id, kind, "missing required `allowed` parameter")
                    })?;
                    if allowed.is_empty() {
                        return Err(drc_rule_error(&id, kind, "`allowed` must not be empty"));
                    }
                    if let Some(angle) = allowed.iter().find(|&&angle| !(0..180).contains(&angle)) {
                        return Err(drc_rule_error(
                            &id,
                            kind,
                            format!("allowed angle {angle} is outside [0, 180)"),
                        ));
                    }
                    DrcRuleParam::Angle {
                        id: id.clone(),
                        allowed,
                    }
                }
                "min_density" => DrcRuleParam::MinDensity {
                    id: id.clone(),
                    layer: layer()?,
                    window: required_positive_i32(&id, kind, "window", r.window)?,
                    min_frac: required_fraction(&id, kind, "min_frac", r.min_frac)?,
                },
                "max_density" => DrcRuleParam::MaxDensity {
                    id: id.clone(),
                    layer: layer()?,
                    window: required_positive_i32(&id, kind, "window", r.window)?,
                    max_frac: required_fraction(&id, kind, "max_frac", r.max_frac)?,
                },
                "overlap" => DrcRuleParam::Overlap {
                    id: id.clone(),
                    a: layer()?,
                    b: layer_b()?,
                    min: min_i32()?,
                },
                "corner_to_corner" => DrcRuleParam::CornerToCorner {
                    id: id.clone(),
                    layer: layer()?,
                    min: min_i32()?,
                },
                "antenna" => DrcRuleParam::Antenna {
                    id: id.clone(),
                    layer: layer()?,
                    ratio: required_positive_f64(&id, kind, "ratio", r.ratio)?,
                },
                "antenna_car" => {
                    let stack = r.layers.as_ref().ok_or_else(|| {
                        drc_rule_error(&id, kind, "missing required `layers` parameter")
                    })?;
                    if stack.is_empty() {
                        return Err(drc_rule_error(&id, kind, "`layers` must not be empty"));
                    }
                    let mut seen = std::collections::HashSet::new();
                    let mut stack_ids = Vec::with_capacity(stack.len());
                    for name in stack {
                        if !seen.insert(name) {
                            return Err(drc_rule_error(
                                &id,
                                kind,
                                format!("duplicate layer `{name}` in antenna stack"),
                            ));
                        }
                        stack_ids.push(lid(Some(name)).map_err(|e| drc_rule_error(&id, kind, e))?);
                    }
                    let diode = r
                        .diode_layer
                        .as_deref()
                        .map(|name| lid(Some(name)).map_err(|e| drc_rule_error(&id, kind, e)))
                        .transpose()?;
                    DrcRuleParam::AntennaCar {
                        id: id.clone(),
                        layers: stack_ids,
                        ratio: required_positive_f64(&id, kind, "ratio", r.ratio)?,
                        diode,
                    }
                }
                "eol_spacing" => DrcRuleParam::EolSpacing {
                    id: id.clone(),
                    layer: layer()?,
                    eol_width: required_positive_i32(&id, kind, "eol_width", r.eol_width)?,
                    eol_spacing: required_positive_i32(&id, kind, "eol_spacing", r.eol_spacing)?,
                },
                "wide_dependent_spacing" => DrcRuleParam::WideDependentSpacing {
                    id: id.clone(),
                    layer: layer()?,
                    width_threshold: required_positive_i32(
                        &id,
                        kind,
                        "width_threshold",
                        r.width_threshold,
                    )?,
                    wide_spacing: required_positive_i32(&id, kind, "wide_spacing", r.wide_spacing)?,
                },
                "prl_spacing" => DrcRuleParam::PrlSpacing {
                    id: id.clone(),
                    layer: layer()?,
                    prl_threshold: required_positive_i32(
                        &id,
                        kind,
                        "prl_threshold",
                        r.prl_threshold,
                    )?,
                    prl_spacing: required_positive_i32(&id, kind, "prl_spacing", r.prl_spacing)?,
                },
                "asymmetric_enclosure" => DrcRuleParam::AsymmetricEnclosure {
                    id: id.clone(),
                    outer: outer()?,
                    inner: inner()?,
                    min_one_side: min_i32()?,
                },
                "min_enclosed_area" => DrcRuleParam::MinEnclosedArea {
                    id: id.clone(),
                    layer: layer()?,
                    min_hole_area: required_positive_i64(&id, kind, "min", r.min)?,
                },
                "cheesing" => DrcRuleParam::Cheesing {
                    id: id.clone(),
                    layer: layer()?,
                    max_area_no_slot: i64::from(required_positive_i32(&id, kind, "max", r.max)?),
                },
                "redundant_via" => {
                    let min_count = required_positive_i32(&id, kind, "min_count", r.min_count)?;
                    if min_count < 2 {
                        return Err(drc_rule_error(
                            &id,
                            kind,
                            format!("`min_count` must be at least 2, got {min_count}"),
                        ));
                    }
                    DrcRuleParam::RedundantVia {
                        id: id.clone(),
                        layer: layer()?,
                        min_count,
                        within: required_positive_i32(&id, kind, "within", r.within)?,
                    }
                }
                "via_array_spacing" => {
                    let array_threshold =
                        required_positive_i32(&id, kind, "array_threshold", r.array_threshold)?;
                    if array_threshold < 2 {
                        return Err(drc_rule_error(
                            &id,
                            kind,
                            format!("`array_threshold` must be at least 2, got {array_threshold}"),
                        ));
                    }
                    DrcRuleParam::ViaArraySpacing {
                        id: id.clone(),
                        layer: layer()?,
                        array_threshold,
                        array_spacing: required_positive_i32(
                            &id,
                            kind,
                            "array_spacing",
                            r.array_spacing,
                        )?,
                    }
                }
                "max_distance_to_tap" => DrcRuleParam::MaxDistanceToTap {
                    id: id.clone(),
                    diff_layer: layer()?,
                    tap_layer: layer_b()?,
                    max_dist: required_positive_i32(&id, kind, "max_dist", r.max_dist)?,
                },
                "multi_patterning" => {
                    let num_colors = required_positive_i32(&id, kind, "num_colors", r.num_colors)?;
                    if !(2..=64).contains(&num_colors) {
                        return Err(drc_rule_error(
                            &id,
                            kind,
                            format!("`num_colors` must be in [2, 64], got {num_colors}"),
                        ));
                    }
                    DrcRuleParam::MultiPatterning {
                        id: id.clone(),
                        layer: layer()?,
                        num_colors,
                        color_spacing: min_i32()?,
                    }
                }
                other => return Err(drc_rule_error(&id, kind, format!("unknown kind `{other}`"))),
            };
            // Disabled entries are still fully parsed and validated.  This
            // catches unsupported kinds, misspelled layers and malformed units
            // in the declared deck inventory; `enabled` controls execution,
            // not whether the schema is understood.
            if r.enabled {
                drc_rules.push(rule);
            }
        }
        drc_rules.sort_by(|a, b| a.id().cmp(b.id()));

        let mut pex = HashMap::new();
        for (name, p) in schema.pex {
            let id = layers
                .id(&name)
                .ok_or_else(|| format!("PEX parameters reference unknown layer `{name}`"))?;
            for (field, value) in [
                ("sheet_res_ohm_sq", p.sheet_res_ohm_sq),
                ("area_cap_af_um2", p.area_cap_af_um2),
                ("fringe_cap_af_um", p.fringe_cap_af_um),
                ("coupling_cap_af_um", p.coupling_cap_af_um),
                ("coupling_ref_spacing_nm", p.coupling_ref_spacing_nm),
                ("via_res_ohm", p.via_res_ohm),
                ("interlayer_cap_af_um2", p.interlayer_cap_af_um2),
            ] {
                if !value.is_finite() || value < 0.0 {
                    return Err(format!(
                        "PEX layer `{name}`: `{field}` must be finite and non-negative, got {value}"
                    ));
                }
            }
            if p.coupling_cap_af_um > 0.0 && p.coupling_ref_spacing_nm <= 0.0 {
                return Err(format!(
                    "PEX layer `{name}`: `coupling_ref_spacing_nm` must be positive when \
                     `coupling_cap_af_um` is enabled"
                ));
            }
            pex.insert(id, p);
        }

        // Resolve connectivity schema → ConnectivityConfig
        let connectivity = {
            let cs = &schema.connectivity;
            if cs.conductors.is_empty() && cs.vias.is_empty() {
                ConnectivityConfig::default()
            } else {
                if cs.conductors.is_empty() {
                    return Err(
                        "connectivity schema declares vias but contains no conductors".into(),
                    );
                }
                let mut conductor_names = std::collections::HashSet::new();
                let conductors = cs
                    .conductors
                    .iter()
                    .map(|name| {
                        if !conductor_names.insert(name.as_str()) {
                            return Err(format!(
                                "connectivity conductor layer `{name}` is duplicated"
                            ));
                        }
                        layers.id(name).ok_or_else(|| {
                            format!("connectivity conductor references unknown layer `{name}`")
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let mut vias = Vec::with_capacity(cs.vias.len());
                let mut via_layers = std::collections::HashSet::new();
                for via in &cs.vias {
                    if !via_layers.insert(via.layer.as_str()) {
                        return Err(format!(
                            "connectivity cut layer `{}` has duplicate via declarations",
                            via.layer,
                        ));
                    }
                    let via_id = layers.id(&via.layer).ok_or_else(|| {
                        format!(
                            "connectivity via references unknown cut layer `{}`",
                            via.layer
                        )
                    })?;
                    if via.connects.is_empty() {
                        return Err(format!(
                            "connectivity via `{}` must connect at least one conductor layer",
                            via.layer,
                        ));
                    }
                    let mut connected_names = std::collections::HashSet::new();
                    let connects = via
                        .connects
                        .iter()
                        .map(|name| {
                            if !connected_names.insert(name.as_str()) {
                                return Err(format!(
                                    "connectivity via `{}` repeats conductor layer `{name}`",
                                    via.layer,
                                ));
                            }
                            if !conductor_names.contains(name.as_str()) {
                                return Err(format!(
                                    "connectivity via `{}` references layer `{name}` that is not declared as a conductor",
                                    via.layer,
                                ));
                            }
                            layers.id(name).ok_or_else(|| {
                                format!(
                                    "connectivity via `{}` references unknown conductor layer `{name}`",
                                    via.layer,
                                )
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    vias.push((via_id, connects));
                }
                ConnectivityConfig { conductors, vias }
            }
        };

        if !schema.erc.antenna_ratio.is_finite()
            || schema.erc.antenna_ratio <= 0.0
            || schema.erc.em_min_width_nm <= 0
            || !schema.erc.p2p_r_limit_ohm.is_finite()
            || schema.erc.p2p_r_limit_ohm <= 0.0
            || schema.erc.tie_max_dist_nm <= 0
        {
            return Err(
                "ERC thresholds must be finite and positive in their documented units".into(),
            );
        }

        // Resolve device schema → DeviceConfig
        let devices = {
            let ds = &schema.devices;
            let device_layer = |device_kind: &str,
                                rule_name: &str,
                                role: &str,
                                layer_name: &str|
             -> Result<LayerId, String> {
                layers.id(layer_name).ok_or_else(|| {
                    format!(
                        "device-recognition {device_kind} rule `{rule_name}` references unknown \
                         {role} layer `{layer_name}`"
                    )
                })
            };

            let mut mos_rules = Vec::with_capacity(ds.mos_rules.len());
            let mut mos_names = std::collections::HashSet::new();
            for rule in &ds.mos_rules {
                if rule.name.trim().is_empty() || !mos_names.insert(rule.name.as_str()) {
                    return Err(format!(
                        "device-recognition MOS rule has an empty or duplicate name `{}`",
                        rule.name
                    ));
                }
                if !matches!(rule.device_type.as_str(), "nmos" | "pmos") {
                    return Err(format!(
                        "device-recognition MOS rule `{}` has unsupported device type `{}`",
                        rule.name, rule.device_type
                    ));
                }
                if rule
                    .device_class
                    .as_ref()
                    .is_some_and(|class| class.trim().is_empty())
                {
                    return Err(format!(
                        "device-recognition MOS rule `{}` has an empty device class",
                        rule.name
                    ));
                }
                let mut flavor_layers = std::collections::HashSet::new();
                for (layer_name, flavor) in &rule.flavor_markers {
                    if !matches!(
                        flavor.as_str(),
                        "lvt" | "Lvt" | "LVT" | "hvt" | "Hvt" | "HVT"
                    ) {
                        return Err(format!(
                            "device-recognition MOS rule `{}` has unsupported flavor `{flavor}`",
                            rule.name
                        ));
                    }
                    if !flavor_layers.insert(layer_name.as_str()) {
                        return Err(format!(
                            "device-recognition MOS rule `{}` repeats flavor-marker layer `{layer_name}`",
                            rule.name
                        ));
                    }
                }
                let flavor_markers = rule
                    .flavor_markers
                    .iter()
                    .map(|(layer_name, flavor)| {
                        device_layer("MOS", &rule.name, "flavor marker", layer_name)
                            .map(|layer| (layer, flavor.clone()))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let well_layer = rule
                    .well_layer
                    .as_deref()
                    .map(|name| device_layer("MOS", &rule.name, "well", name))
                    .transpose()?;
                mos_rules.push(MosRule {
                    name: rule.name.clone(),
                    gate_layer: device_layer("MOS", &rule.name, "gate", &rule.gate_layer)?,
                    channel_layer: device_layer("MOS", &rule.name, "channel", &rule.channel_layer)?,
                    type_implant: device_layer(
                        "MOS",
                        &rule.name,
                        "type implant",
                        &rule.type_implant,
                    )?,
                    device_type: rule.device_type.clone(),
                    flavor_markers,
                    well_layer,
                    device_class: rule.device_class.clone(),
                });
            }

            let mut bjt_rules = Vec::with_capacity(ds.bjt_rules.len());
            let mut bjt_names = std::collections::HashSet::new();
            for rule in &ds.bjt_rules {
                if rule.name.trim().is_empty() || !bjt_names.insert(rule.name.as_str()) {
                    return Err(format!(
                        "device-recognition BJT rule has an empty or duplicate name `{}`",
                        rule.name
                    ));
                }
                if !matches!(rule.device_type.as_str(), "npn" | "pnp") {
                    return Err(format!(
                        "device-recognition BJT rule `{}` has unsupported device type `{}`",
                        rule.name, rule.device_type
                    ));
                }
                bjt_rules.push(BjtRule {
                    name: rule.name.clone(),
                    collector_layer: device_layer(
                        "BJT",
                        &rule.name,
                        "collector",
                        &rule.collector_layer,
                    )?,
                    base_layer: device_layer("BJT", &rule.name, "base", &rule.base_layer)?,
                    emitter_layer: device_layer("BJT", &rule.name, "emitter", &rule.emitter_layer)?,
                    type_marker: device_layer("BJT", &rule.name, "type marker", &rule.type_marker)?,
                    device_type: rule.device_type.clone(),
                });
            }

            let mut resistor_rules = Vec::with_capacity(ds.resistor_rules.len());
            let mut resistor_names = std::collections::HashSet::new();
            for rule in &ds.resistor_rules {
                if rule.name.trim().is_empty() || !resistor_names.insert(rule.name.as_str()) {
                    return Err(format!(
                        "device-recognition resistor rule has an empty or duplicate name `{}`",
                        rule.name
                    ));
                }
                resistor_rules.push(ResistorRule {
                    name: rule.name.clone(),
                    body_layer: device_layer("resistor", &rule.name, "body", &rule.body_layer)?,
                    marker_layer: device_layer(
                        "resistor",
                        &rule.name,
                        "marker",
                        &rule.marker_layer,
                    )?,
                    terminal_layer: device_layer(
                        "resistor",
                        &rule.name,
                        "terminal",
                        &rule.terminal_layer,
                    )?,
                });
            }

            let mut diode_rules = Vec::with_capacity(ds.diode_rules.len());
            let mut diode_names = std::collections::HashSet::new();
            for rule in &ds.diode_rules {
                if rule.name.trim().is_empty() || !diode_names.insert(rule.name.as_str()) {
                    return Err(format!(
                        "device-recognition diode rule has an empty or duplicate name `{}`",
                        rule.name
                    ));
                }
                diode_rules.push(DiodeRule {
                    name: rule.name.clone(),
                    anode_layer: device_layer("diode", &rule.name, "anode", &rule.anode_layer)?,
                    cathode_layer: device_layer(
                        "diode",
                        &rule.name,
                        "cathode",
                        &rule.cathode_layer,
                    )?,
                    implant_layer: device_layer(
                        "diode",
                        &rule.name,
                        "implant",
                        &rule.implant_layer,
                    )?,
                });
            }

            let mut cap_rules = Vec::with_capacity(ds.cap_rules.len());
            let mut cap_names = std::collections::HashSet::new();
            for rule in &ds.cap_rules {
                if rule.name.trim().is_empty() || !cap_names.insert(rule.name.as_str()) {
                    return Err(format!(
                        "device-recognition capacitor rule has an empty or duplicate name `{}`",
                        rule.name
                    ));
                }
                let marker_layer = rule
                    .marker_layer
                    .as_deref()
                    .map(|name| device_layer("capacitor", &rule.name, "marker", name))
                    .transpose()?;
                cap_rules.push(CapRule {
                    name: rule.name.clone(),
                    top_layer: device_layer("capacitor", &rule.name, "top plate", &rule.top_layer)?,
                    bottom_layer: device_layer(
                        "capacitor",
                        &rule.name,
                        "bottom plate",
                        &rule.bottom_layer,
                    )?,
                    marker_layer,
                });
            }
            DeviceConfig {
                mos_rules,
                bjt_rules,
                resistor_rules,
                diode_rules,
                cap_rules,
            }
        };

        Ok(Deck {
            layers,
            drc_rules,
            pex,
            dbu_nm: 1.0,
            lvs_cut_required: schema.lvs.cut_required,
            strict: false,
            connectivity,
            devices,
            w_tolerance: schema.lvs.w_tolerance,
            l_tolerance: schema.lvs.l_tolerance,
            fail_on_floating: schema.lvs.fail_on_floating,
            intra_layer_touch: schema.connectivity.intra_layer_touch,
            global_nets: schema.connectivity.global_nets.clone(),
            erc: schema.erc,
        })
    }

    /// Parse a PDK deck from a JSON string. Delegates to `from_schema` internally.
    pub fn from_json(text: &str) -> Result<Self, String> {
        use crate::schema::{drc_rules_from_json, LvsSchema, VerifySchema};
        use serde::Deserialize;

        #[derive(Deserialize)]
        struct Doc {
            #[serde(default)]
            layers: HashMap<String, LayerDef>,
            #[serde(default)]
            drc: serde_json::Value,
            #[serde(default)]
            pex: HashMap<String, PexLayerParams>,
            #[serde(default)]
            lvs: LvsRaw,
            #[serde(default)]
            connectivity: ConnRaw,
            #[serde(default)]
            device_recognition: DevRecogRaw,
            #[serde(default)]
            erc: ErcParams,
        }

        #[derive(Deserialize, Default)]
        #[serde(deny_unknown_fields)]
        struct LvsRaw {
            #[serde(default)]
            cut_required: bool,
        }

        #[derive(Deserialize, Default)]
        #[serde(deny_unknown_fields)]
        struct ConnRaw {
            #[serde(default)]
            conductors: Vec<String>,
            #[serde(default)]
            vias: Vec<ViaRaw>,
            #[serde(default)]
            intra_layer_touch: bool,
            #[serde(default)]
            global_nets: Vec<String>,
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct ViaRaw {
            layer: String,
            connects: Vec<String>,
        }

        #[derive(Deserialize, Default)]
        #[serde(deny_unknown_fields)]
        struct DevRecogRaw {
            #[serde(default)]
            mos: Vec<MosRaw>,
            #[serde(default)]
            bjt: Vec<BjtRaw>,
            #[serde(default)]
            resistor: Vec<ResistorRaw>,
            #[serde(default)]
            diode: Vec<DiodeRaw>,
            #[serde(default, alias = "capacitor")]
            cap: Vec<CapRaw>,
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct MosRaw {
            name: String,
            gate_layer: String,
            channel_layer: String,
            type_implant: String,
            device_type: String,
            #[serde(default)]
            flavor_markers: Vec<(String, String)>,
            well_layer: Option<String>,
            device_class: Option<String>,
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct BjtRaw {
            name: String,
            collector_layer: String,
            base_layer: String,
            emitter_layer: String,
            type_marker: String,
            device_type: String,
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct ResistorRaw {
            name: String,
            body_layer: String,
            marker_layer: String,
            terminal_layer: String,
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct DiodeRaw {
            name: String,
            anode_layer: String,
            cathode_layer: String,
            implant_layer: String,
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct CapRaw {
            name: String,
            top_layer: String,
            bottom_layer: String,
            #[serde(default)]
            marker_layer: Option<String>,
        }

        let doc: Doc = serde_json::from_str(text).map_err(|e| e.to_string())?;

        let layers = doc
            .layers
            .into_iter()
            .map(|(name, def)| (name, (def.layer, def.datatype)))
            .collect();

        let drc_rules = drc_rules_from_json(&doc.drc)?;

        Self::from_schema(VerifySchema {
            layers,
            drc_rules,
            pex: doc.pex,
            erc: doc.erc,
            lvs: LvsSchema {
                cut_required: doc.lvs.cut_required,
                w_tolerance: PropertyTolerance::default(),
                l_tolerance: PropertyTolerance::default(),
                fail_on_floating: false,
                hierarchical: false,
                equate_cells: Vec::new(),
            },
            connectivity: {
                use crate::schema::{ConnectivitySchema, ViaSchema};
                if doc.connectivity.conductors.is_empty() && doc.connectivity.vias.is_empty() {
                    ConnectivitySchema::default()
                } else {
                    ConnectivitySchema {
                        conductors: doc.connectivity.conductors,
                        vias: doc
                            .connectivity
                            .vias
                            .into_iter()
                            .map(|v| ViaSchema {
                                layer: v.layer,
                                connects: v.connects,
                            })
                            .collect(),
                        intra_layer_touch: doc.connectivity.intra_layer_touch,
                        global_nets: doc.connectivity.global_nets,
                    }
                }
            },
            devices: {
                use crate::schema::{
                    BjtRuleSchema, CapRuleSchema, DeviceSchema, DiodeRuleSchema, MosRuleSchema,
                    ResistorRuleSchema,
                };
                DeviceSchema {
                    mos_rules: doc
                        .device_recognition
                        .mos
                        .into_iter()
                        .map(|m| MosRuleSchema {
                            name: m.name,
                            gate_layer: m.gate_layer,
                            channel_layer: m.channel_layer,
                            type_implant: m.type_implant,
                            device_type: m.device_type,
                            flavor_markers: m.flavor_markers,
                            well_layer: m.well_layer,
                            device_class: m.device_class,
                        })
                        .collect(),
                    bjt_rules: doc
                        .device_recognition
                        .bjt
                        .into_iter()
                        .map(|rule| BjtRuleSchema {
                            name: rule.name,
                            collector_layer: rule.collector_layer,
                            base_layer: rule.base_layer,
                            emitter_layer: rule.emitter_layer,
                            type_marker: rule.type_marker,
                            device_type: rule.device_type,
                        })
                        .collect(),
                    resistor_rules: doc
                        .device_recognition
                        .resistor
                        .into_iter()
                        .map(|rule| ResistorRuleSchema {
                            name: rule.name,
                            body_layer: rule.body_layer,
                            marker_layer: rule.marker_layer,
                            terminal_layer: rule.terminal_layer,
                        })
                        .collect(),
                    diode_rules: doc
                        .device_recognition
                        .diode
                        .into_iter()
                        .map(|rule| DiodeRuleSchema {
                            name: rule.name,
                            anode_layer: rule.anode_layer,
                            cathode_layer: rule.cathode_layer,
                            implant_layer: rule.implant_layer,
                        })
                        .collect(),
                    cap_rules: doc
                        .device_recognition
                        .cap
                        .into_iter()
                        .map(|rule| CapRuleSchema {
                            name: rule.name,
                            top_layer: rule.top_layer,
                            bottom_layer: rule.bottom_layer,
                            marker_layer: rule.marker_layer,
                        })
                        .collect(),
                    derived_layers: Vec::new(),
                }
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drc::run_drc;
    use crate::geometry::GeometryStore;
    use crate::schema::{
        BjtRuleSchema, CapRuleSchema, ConnectivitySchema, DeviceSchema, DiodeRuleSchema,
        DrcRuleSchema, LvsSchema, MosRuleSchema, ResistorRuleSchema, VerifySchema, ViaSchema,
    };

    fn base_schema() -> VerifySchema {
        VerifySchema {
            layers: [
                ("known".to_string(), (1, 0)),
                ("met1".to_string(), (2, 0)),
                ("met2".to_string(), (3, 0)),
            ]
            .into_iter()
            .collect(),
            drc_rules: Vec::new(),
            pex: HashMap::new(),
            erc: ErcParams::default(),
            lvs: LvsSchema::default(),
            connectivity: ConnectivitySchema::default(),
            devices: DeviceSchema::default(),
        }
    }

    #[test]
    fn repeated_rule_kinds_run_with_distinct_ids() {
        let deck = Deck::from_json(
            r#"{
                "layers": {
                    "met1": {"layer": 1, "datatype": 0},
                    "met2": {"layer": 2, "datatype": 0}
                },
                "drc": {
                    "M1.W.1": {"kind": "min_width", "layer": "met1", "min": 100},
                    "M2.W.7": {"kind": "min_width", "layer": "met2", "min": 200}
                }
            }"#,
        )
        .expect("deck with repeated rule kinds");

        let mut store = GeometryStore::new();
        store.add_rect(deck.layers.id("met1").unwrap(), 0, 0, 50, 500);
        store.add_rect(deck.layers.id("met2").unwrap(), 1000, 0, 150, 500);
        let report = run_drc(&store, &deck);
        let mut ids: Vec<&str> = report
            .violations
            .iter()
            .filter(|violation| violation.kind == "min_width")
            .map(|violation| violation.rule_id.as_str())
            .collect();
        ids.sort_unstable();
        assert_eq!(ids, ["M1.W.1", "M2.W.7"]);
    }

    #[test]
    fn explicit_array_and_legacy_map_encodings_are_supported() {
        let array = Deck::from_json(
            r#"{
                "layers": {"met1": {"layer": 1, "datatype": 0}},
                "drc": [
                    {"id": "M1.W.1", "kind": "min_width", "layer": "met1", "min": 100}
                ]
            }"#,
        )
        .expect("explicit DRC array");
        assert_eq!(array.drc_rules[0].id(), "M1.W.1");

        let legacy = Deck::from_json(
            r#"{
                "layers": {"met1": {"layer": 1, "datatype": 0}},
                "drc": {"min_width": {"layer": "met1", "min": 100}}
            }"#,
        )
        .expect("legacy DRC map");
        assert_eq!(legacy.drc_rules[0].id(), "min_width");

        let duplicate = Deck::from_json(
            r#"{
                "layers": {"met1": {"layer": 1, "datatype": 0}},
                "drc": [
                    {"id": "DUP", "kind": "min_width", "layer": "met1", "min": 100},
                    {"id": "DUP", "kind": "min_spacing", "layer": "met1", "min": 100}
                ]
            }"#,
        )
        .err()
        .expect("duplicate rule IDs must fail");
        assert!(
            duplicate.contains("duplicate DRC rule ID `DUP`"),
            "{duplicate}"
        );
    }

    #[test]
    fn rule_ids_are_independent_in_from_schema_too() {
        let mut schema = base_schema();
        schema.drc_rules = vec![
            DrcRuleSchema {
                id: "M1.W".into(),
                kind: "min_width".into(),
                layer: Some("met1".into()),
                min: Some(100),
                ..Default::default()
            },
            DrcRuleSchema {
                id: "M2.W".into(),
                kind: "min_width".into(),
                layer: Some("met2".into()),
                min: Some(200),
                ..Default::default()
            },
        ];
        let deck = Deck::from_schema(schema).expect("schema");
        assert_eq!(
            deck.drc_rules
                .iter()
                .map(DrcRuleParam::id)
                .collect::<Vec<_>>(),
            ["M1.W", "M2.W"],
        );
        let mut store = GeometryStore::new();
        store.add_rect(deck.layers.id("met1").unwrap(), 0, 0, 50, 500);
        store.add_rect(deck.layers.id("met2").unwrap(), 1000, 0, 150, 500);
        let report = run_drc(&store, &deck);
        let mut ids: Vec<&str> = report
            .violations
            .iter()
            .map(|violation| violation.rule_id.as_str())
            .collect();
        ids.sort_unstable();
        assert_eq!(ids, ["M1.W", "M2.W"]);

        assert!(
            DrcRuleSchema::default().enabled,
            "programmatic schema omission must match JSON's enabled=true default"
        );
    }

    #[test]
    fn malformed_numeric_rules_fail_construction() {
        let cases = [
            (
                r#"{"kind":"min_width","layer":"met1","min":0}"#,
                "must be positive",
            ),
            (
                r#"{"kind":"min_width","layer":"met1","min":-1}"#,
                "must be positive",
            ),
            (
                r#"{"kind":"min_width","layer":"met1","min":2147483648}"#,
                "outside the i32 range",
            ),
            (
                r#"{"kind":"max_density","layer":"met1","window":100,"max_frac":1.1}"#,
                "in [0, 1]",
            ),
            (r#"{"kind":"angle","allowed":[]}"#, "must not be empty"),
            (
                r#"{"kind":"multi_patterning","layer":"met1","min":100,"num_colors":1}"#,
                "in [2, 64]",
            ),
        ];
        for (body, expected) in cases {
            let json = format!(
                r#"{{"layers":{{"met1":{{"layer":1,"datatype":0}}}},"drc":{{"R":{body}}}}}"#
            );
            let error = Deck::from_json(&json)
                .err()
                .expect("invalid rule must fail");
            assert!(
                error.contains(expected),
                "expected `{expected}` in error, got `{error}`"
            );
        }

        let mut schema = base_schema();
        schema.drc_rules.push(DrcRuleSchema {
            id: "DENS.NAN".into(),
            kind: "min_density".into(),
            enabled: true,
            layer: Some("met1".into()),
            window: Some(100),
            min_frac: Some(f64::NAN),
            ..Default::default()
        });
        let error = Deck::from_schema(schema)
            .err()
            .expect("NaN density must fail");
        assert!(error.contains("finite"), "unexpected error: {error}");
    }

    #[test]
    fn unknown_drc_properties_fail_construction() {
        let error = Deck::from_json(
            r#"{
                "layers": {"met1": {"layer": 1, "datatype": 0}},
                "drc": {
                    "M1.W.1": {
                        "kind": "min_width",
                        "layer": "met1",
                        "mni": 100,
                        "min": 100
                    }
                }
            }"#,
        )
        .err()
        .expect("an unknown DRC property must not be ignored");
        assert!(
            error.contains("M1.W.1") && error.contains("unknown property `mni`"),
            "unexpected error: {error}"
        );

        let error = Deck::from_json(
            r#"{
                "layers": {"met1": {"layer": 1, "datatype": 0}},
                "drc": {"UNKNOWN": {"kind": "min_wdith", "enabled": false}}
            }"#,
        )
        .err()
        .expect("disabled operations must still be understood by the engine");
        assert!(error.contains("unknown kind `min_wdith`"), "{error}");

        let error = Deck::from_json(
            r#"{
                "layers": {"met1": {"layer": 1, "datatype": 0}},
                "drc": {
                    "M2.W": {
                        "kind": "min_width",
                        "enabled": false,
                        "layer": "ghost",
                        "min": 100
                    }
                }
            }"#,
        )
        .err()
        .expect("disabled rules must not hide unknown layer references");
        assert!(error.contains("unknown layer 'ghost'"), "{error}");
    }

    #[test]
    fn unknown_pex_and_connectivity_layers_are_errors() {
        let mut schema = base_schema();
        schema.pex.insert(
            "ghost".into(),
            PexLayerParams {
                sheet_res_ohm_sq: 1.0,
                area_cap_af_um2: 1.0,
                fringe_cap_af_um: 1.0,
                coupling_cap_af_um: 1.0,
                coupling_ref_spacing_nm: 1.0,
                via_res_ohm: 0.0,
                interlayer_cap_af_um2: 0.0,
            },
        );
        let error = Deck::from_schema(schema)
            .err()
            .expect("unknown PEX layer must fail");
        assert!(error.contains("PEX") && error.contains("ghost"), "{error}");

        let mut schema = base_schema();
        schema.pex.insert(
            "met1".into(),
            PexLayerParams {
                sheet_res_ohm_sq: f64::NAN,
                area_cap_af_um2: 1.0,
                fringe_cap_af_um: 1.0,
                coupling_cap_af_um: 1.0,
                coupling_ref_spacing_nm: 1.0,
                via_res_ohm: 0.0,
                interlayer_cap_af_um2: 0.0,
            },
        );
        let error = Deck::from_schema(schema)
            .err()
            .expect("non-finite PEX coefficient must fail");
        assert!(
            error.contains("sheet_res_ohm_sq") && error.contains("finite"),
            "{error}"
        );

        let mut schema = base_schema();
        schema.pex.insert(
            "met1".into(),
            PexLayerParams {
                sheet_res_ohm_sq: 1.0,
                area_cap_af_um2: 1.0,
                fringe_cap_af_um: 1.0,
                coupling_cap_af_um: 1.0,
                coupling_ref_spacing_nm: 0.0,
                via_res_ohm: 0.0,
                interlayer_cap_af_um2: 0.0,
            },
        );
        let error = Deck::from_schema(schema)
            .err()
            .expect("enabled coupling needs positive reference spacing");
        assert!(
            error.contains("coupling_ref_spacing_nm") && error.contains("positive"),
            "{error}"
        );

        let mut schema = base_schema();
        schema.connectivity.conductors = vec!["ghost".into()];
        let error = Deck::from_schema(schema)
            .err()
            .expect("unknown conductor must fail");
        assert!(
            error.contains("connectivity conductor") && error.contains("ghost"),
            "{error}"
        );

        let mut schema = base_schema();
        schema.connectivity.conductors = vec!["known".into()];
        schema.connectivity.vias = vec![ViaSchema {
            layer: "ghost".into(),
            connects: vec!["known".into()],
        }];
        let error = Deck::from_schema(schema)
            .err()
            .expect("unknown via cut layer must fail");
        assert!(
            error.contains("connectivity via") && error.contains("ghost"),
            "{error}"
        );

        let mut schema = base_schema();
        schema.connectivity.conductors = vec!["known".into()];
        schema.connectivity.vias = vec![ViaSchema {
            layer: "met1".into(),
            connects: vec!["known".into(), "ghost".into()],
        }];
        let error = Deck::from_schema(schema)
            .err()
            .expect("unknown via conductor must fail");
        assert!(
            error.contains("connectivity via") && error.contains("ghost"),
            "{error}"
        );
    }

    #[test]
    fn every_device_class_rejects_unknown_layer_references() {
        let device_cases: Vec<(&str, DeviceSchema)> = vec![
            (
                "MOS",
                DeviceSchema {
                    mos_rules: vec![MosRuleSchema {
                        name: "nmos".into(),
                        gate_layer: "known".into(),
                        channel_layer: "known".into(),
                        type_implant: "known".into(),
                        device_type: "nmos".into(),
                        flavor_markers: vec![("ghost".into(), "lvt".into())],
                        well_layer: None,
                        device_class: None,
                    }],
                    ..Default::default()
                },
            ),
            (
                "BJT",
                DeviceSchema {
                    bjt_rules: vec![BjtRuleSchema {
                        name: "npn".into(),
                        collector_layer: "known".into(),
                        base_layer: "known".into(),
                        emitter_layer: "known".into(),
                        type_marker: "ghost".into(),
                        device_type: "npn".into(),
                    }],
                    ..Default::default()
                },
            ),
            (
                "resistor",
                DeviceSchema {
                    resistor_rules: vec![ResistorRuleSchema {
                        name: "res".into(),
                        body_layer: "known".into(),
                        marker_layer: "known".into(),
                        terminal_layer: "ghost".into(),
                    }],
                    ..Default::default()
                },
            ),
            (
                "diode",
                DeviceSchema {
                    diode_rules: vec![DiodeRuleSchema {
                        name: "dio".into(),
                        anode_layer: "known".into(),
                        cathode_layer: "known".into(),
                        implant_layer: "ghost".into(),
                    }],
                    ..Default::default()
                },
            ),
            (
                "capacitor",
                DeviceSchema {
                    cap_rules: vec![CapRuleSchema {
                        name: "cap".into(),
                        top_layer: "known".into(),
                        bottom_layer: "known".into(),
                        marker_layer: Some("ghost".into()),
                    }],
                    ..Default::default()
                },
            ),
        ];

        for (kind, devices) in device_cases {
            let mut schema = base_schema();
            schema.devices = devices;
            let error = Deck::from_schema(schema)
                .err()
                .unwrap_or_else(|| panic!("{kind} unknown layer unexpectedly accepted"));
            assert!(
                error.contains("device-recognition")
                    && error.contains(kind)
                    && error.contains("ghost"),
                "unexpected {kind} error: {error}"
            );
        }
    }

    #[test]
    fn unsupported_device_and_flavor_names_are_errors() {
        let mut schema = base_schema();
        schema.devices.mos_rules.push(MosRuleSchema {
            name: "mystery".into(),
            gate_layer: "known".into(),
            channel_layer: "known".into(),
            type_implant: "known".into(),
            device_type: "gaafet".into(),
            flavor_markers: Vec::new(),
            well_layer: None,
            device_class: None,
        });
        let error = Deck::from_schema(schema)
            .err()
            .expect("an unsupported MOS kind must not default to NMOS");
        assert!(
            error.contains("unsupported device type `gaafet`"),
            "{error}"
        );

        let mut schema = base_schema();
        schema.devices.mos_rules.push(MosRuleSchema {
            name: "nmos".into(),
            gate_layer: "known".into(),
            channel_layer: "known".into(),
            type_implant: "known".into(),
            device_type: "nmos".into(),
            flavor_markers: vec![("known".into(), "ulvt".into())],
            well_layer: None,
            device_class: None,
        });
        let error = Deck::from_schema(schema)
            .err()
            .expect("an unsupported flavor must not default to standard Vt");
        assert!(error.contains("unsupported flavor `ulvt`"), "{error}");

        let mut schema = base_schema();
        schema.devices.bjt_rules.push(BjtRuleSchema {
            name: "q".into(),
            collector_layer: "known".into(),
            base_layer: "known".into(),
            emitter_layer: "known".into(),
            type_marker: "known".into(),
            device_type: "phototransistor".into(),
        });
        let error = Deck::from_schema(schema)
            .err()
            .expect("an unsupported BJT kind must not default to NPN");
        assert!(
            error.contains("unsupported device type `phototransistor`"),
            "{error}"
        );
    }

    #[test]
    fn deck_metadata_and_nested_properties_fail_closed() {
        let mut schema = base_schema();
        schema.layers.insert("met1_alias".into(), (2, 0));
        let error = Deck::from_schema(schema)
            .err()
            .expect("duplicate GDS pairs make one symbolic layer unreachable");
        assert!(error.contains("duplicate GDS pair (2, 0)"), "{error}");

        let mut schema = base_schema();
        schema
            .layers
            .insert("invalid".into(), (i32::from(i16::MAX) + 1, 0));
        let error = Deck::from_schema(schema)
            .err()
            .expect("the reader stores GDS layer numbers as signed 16-bit values");
        assert!(error.contains("non-negative i16 range"), "{error}");

        let mut schema = base_schema();
        schema.erc.em_min_width_nm = 0;
        let error = Deck::from_schema(schema)
            .err()
            .expect("zero ERC limits must not disable a check implicitly");
        assert!(error.contains("ERC thresholds"), "{error}");

        let mut schema = base_schema();
        schema.connectivity.conductors = vec!["known".into(), "known".into()];
        let error = Deck::from_schema(schema)
            .err()
            .expect("duplicate conductor membership must be rejected");
        assert!(
            error.contains("conductor layer `known` is duplicated"),
            "{error}"
        );

        let error = Deck::from_json(
            r#"{
                "full_pdk_metadata": {"owner": "caller"},
                "layers": {"met1": {"layer": 1, "datatype": 0}},
                "pex": {"met1": {
                    "sheet_res_ohm_sq": 1.0,
                    "area_cap_af_um2": 1.0,
                    "fringe_cap_af_um": 1.0,
                    "coupling_cap_af_um": 0.0,
                    "coupling_ref_spacing_nm": 0.0,
                    "sheet_res_ohm_sqq": 2.0
                }}
            }"#,
        )
        .err()
        .expect("unknown properties inside a recognized PEX section must fail");
        assert!(
            error.contains("unknown field `sheet_res_ohm_sqq`"),
            "{error}"
        );

        // Unrelated top-level sections belong to the complete PDK facade and
        // remain accepted by the standalone verification subset adapter.
        Deck::from_json(
            r#"{
                "full_pdk_metadata": {"owner": "caller"},
                "layers": {"met1": {"layer": 1, "datatype": 0}}
            }"#,
        )
        .expect("unrelated top-level full-PDK sections remain compatible");
    }
}
