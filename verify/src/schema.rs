//! PDK information schema for the verify crate.
//!
//! Defines exactly what PDK parameters verify needs. Comprehensive PDK loaders
//! fill this schema, and `Deck::from_json` adapts standalone JSON decks into the
//! same representation before resolution and validation.

use crate::params::{ErcParams, PexLayerParams};
use std::collections::HashMap;

/// What the verify crate needs from a PDK.
pub struct VerifySchema {
    /// Layer definitions: name → (GDS layer, GDS datatype).
    pub layers: HashMap<String, (i32, i32)>,
    /// DRC rules.
    pub drc_rules: Vec<DrcRuleSchema>,
    /// PEX process constants keyed by layer name.
    pub pex: HashMap<String, PexLayerParams>,
    /// ERC thresholds (defaulted — see ErcParams).
    pub erc: ErcParams,
    /// LVS extraction options.
    pub lvs: LvsSchema,
    /// PDK-driven connectivity (conductors + vias). Empty → legacy hardcoded fallback.
    pub connectivity: ConnectivitySchema,
    /// PDK-driven device recognition. Empty → legacy hardcoded MOS-only fallback.
    pub devices: DeviceSchema,
}

/// One DRC rule as provided by the PDK. Flat bag of optional fields —
/// `Deck::from_schema` resolves layer names and builds typed `DrcRuleParam`s.
#[derive(Clone, Debug)]
pub struct DrcRuleSchema {
    /// Stable rule identifier reported in violations. For legacy schemas that
    /// leave this empty, `Deck::from_schema` falls back to `kind`.
    pub id: String,
    /// Rule implementation selector (for example `min_width`). This is
    /// deliberately independent from `id`, allowing multiple instances of one
    /// rule kind on different layers or with different thresholds.
    pub kind: String,
    pub enabled: bool,
    pub layer: Option<String>,
    pub layer_b: Option<String>,
    pub outer: Option<String>,
    pub inner: Option<String>,
    pub reference: Option<String>,
    pub min: Option<i64>,
    pub max: Option<i32>,
    pub grid: Option<i32>,
    pub window: Option<i32>,
    pub min_frac: Option<f64>,
    pub max_frac: Option<f64>,
    pub ratio: Option<f64>,
    pub eol_width: Option<i32>,
    pub eol_spacing: Option<i32>,
    pub allowed: Option<Vec<i32>>,
    pub width_threshold: Option<i32>,
    pub wide_spacing: Option<i32>,
    pub prl_threshold: Option<i32>,
    pub prl_spacing: Option<i32>,
    pub min_count: Option<i32>,
    pub within: Option<i32>,
    pub array_threshold: Option<i32>,
    pub array_spacing: Option<i32>,
    pub max_dist: Option<i32>,
    pub num_colors: Option<i32>,
    /// Ordered metal stack, bottom-up, for cumulative antenna (CAR).
    pub layers: Option<Vec<String>>,
    /// Diode/junction marker layer: connection waives the antenna check.
    pub diode_layer: Option<String>,
}

impl Default for DrcRuleSchema {
    fn default() -> Self {
        Self {
            id: String::new(),
            kind: String::new(),
            enabled: true,
            layer: None,
            layer_b: None,
            outer: None,
            inner: None,
            reference: None,
            min: None,
            max: None,
            grid: None,
            window: None,
            min_frac: None,
            max_frac: None,
            ratio: None,
            eol_width: None,
            eol_spacing: None,
            allowed: None,
            width_threshold: None,
            wide_spacing: None,
            prl_threshold: None,
            prl_spacing: None,
            min_count: None,
            within: None,
            array_threshold: None,
            array_spacing: None,
            max_dist: None,
            num_colors: None,
            layers: None,
            diode_layer: None,
        }
    }
}

/// Parse the JSON `drc` section into rule schemas.
///
/// Two encodings are accepted:
///
/// * Legacy/object form: `{ "min_width": { ... } }`. The object key is both
///   the ID and kind unless the body supplies an explicit `id` or `kind`.
/// * Explicit/list form: `[{ "id": "M1.W.1", "kind": "min_width", ... }]`.
///
/// The object form can also express repeated kinds by using distinct keys and
/// putting `"kind": "min_width"` in each body.
pub fn drc_rules_from_json(value: &serde_json::Value) -> Result<Vec<DrcRuleSchema>, String> {
    match value {
        serde_json::Value::Null => Ok(Vec::new()),
        serde_json::Value::Object(rules) => rules
            .iter()
            .map(|(key, body)| drc_rule_from_json(Some(key), body))
            .collect(),
        serde_json::Value::Array(rules) => rules
            .iter()
            .enumerate()
            .map(|(index, body)| {
                drc_rule_from_json(None, body)
                    .map_err(|e| format!("DRC rule at array index {index}: {e}"))
            })
            .collect(),
        _ => Err("`drc` must be an object map or an array of rule objects".into()),
    }
}

fn drc_rule_from_json(
    map_key: Option<&str>,
    value: &serde_json::Value,
) -> Result<DrcRuleSchema, String> {
    use serde_json::Value;

    let object = value.as_object().ok_or_else(|| match map_key {
        Some(key) => format!("DRC rule `{key}` must be an object"),
        None => "DRC rule must be an object".into(),
    })?;
    let string = |field: &str| optional_string(object, field);

    let explicit_id = string("id")?;
    let explicit_kind = string("kind")?;
    let id = explicit_id
        .or_else(|| map_key.map(str::to_owned))
        .ok_or_else(|| "missing required string field `id`".to_string())?;
    let kind = explicit_kind
        .or_else(|| map_key.map(str::to_owned))
        .ok_or_else(|| format!("DRC rule `{id}` is missing required string field `kind`"))?;
    let context = format!("DRC rule `{id}`");

    // The rule adapter is intentionally hand-written so the legacy map and the
    // modern array encodings share one schema.  Keep that flexibility without
    // serde's usual fail-open treatment of unknown fields: a misspelled limit
    // must stop deck loading instead of silently removing a check.
    const FIELDS: &[&str] = &[
        "id",
        "kind",
        "enabled",
        "layer",
        "layer_a",
        "layer_b",
        "outer",
        "inner",
        "ref",
        "reference",
        "min",
        "max",
        "grid",
        "window",
        "min_frac",
        "max_frac",
        "ratio",
        "eol_width",
        "eol_spacing",
        "allowed",
        "width_threshold",
        "wide_spacing",
        "prl_threshold",
        "prl_spacing",
        "min_count",
        "within",
        "array_threshold",
        "array_spacing",
        "max_dist",
        "num_colors",
        "layers",
        "diode_layer",
    ];
    if let Some(field) = object
        .keys()
        .find(|field| !FIELDS.contains(&field.as_str()))
    {
        return Err(format!("{context}: unknown property `{field}`"));
    }

    let enabled = match object.get("enabled") {
        None => true,
        Some(Value::Bool(enabled)) => *enabled,
        Some(_) => return Err(format!("{context}: `enabled` must be a boolean")),
    };

    let layer = string("layer")?;
    let layer_a = string("layer_a")?;
    if layer.is_some() && layer_a.is_some() && layer != layer_a {
        return Err(format!(
            "{context}: `layer` and legacy alias `layer_a` disagree"
        ));
    }

    Ok(DrcRuleSchema {
        id,
        kind,
        enabled,
        layer: layer.or(layer_a),
        layer_b: string("layer_b")?,
        outer: string("outer")?,
        inner: string("inner")?,
        reference: optional_string_alias(object, "ref", "reference", &context)?,
        min: optional_i64(object, "min", &context)?,
        max: optional_i32(object, "max", &context)?,
        grid: optional_i32(object, "grid", &context)?,
        window: optional_i32(object, "window", &context)?,
        min_frac: optional_f64(object, "min_frac", &context)?,
        max_frac: optional_f64(object, "max_frac", &context)?,
        ratio: optional_f64(object, "ratio", &context)?,
        eol_width: optional_i32(object, "eol_width", &context)?,
        eol_spacing: optional_i32(object, "eol_spacing", &context)?,
        allowed: optional_i32_array(object, "allowed", &context)?,
        width_threshold: optional_i32(object, "width_threshold", &context)?,
        wide_spacing: optional_i32(object, "wide_spacing", &context)?,
        prl_threshold: optional_i32(object, "prl_threshold", &context)?,
        prl_spacing: optional_i32(object, "prl_spacing", &context)?,
        min_count: optional_i32(object, "min_count", &context)?,
        within: optional_i32(object, "within", &context)?,
        array_threshold: optional_i32(object, "array_threshold", &context)?,
        array_spacing: optional_i32(object, "array_spacing", &context)?,
        max_dist: optional_i32(object, "max_dist", &context)?,
        num_colors: optional_i32(object, "num_colors", &context)?,
        layers: optional_string_array(object, "layers", &context)?,
        diode_layer: string("diode_layer")?,
    })
}

fn optional_string(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Result<Option<String>, String> {
    match object.get(field) {
        None => Ok(None),
        Some(serde_json::Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(format!("`{field}` must be a string")),
    }
}

fn optional_string_alias(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &str,
    alias: &str,
    context: &str,
) -> Result<Option<String>, String> {
    let primary = optional_string(object, field)?;
    let alias_value = optional_string(object, alias)?;
    if primary.is_some() && alias_value.is_some() && primary != alias_value {
        return Err(format!("{context}: `{field}` and `{alias}` disagree"));
    }
    Ok(primary.or(alias_value))
}

fn optional_i64(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &str,
    context: &str,
) -> Result<Option<i64>, String> {
    match object.get(field) {
        None => Ok(None),
        Some(value) => value
            .as_i64()
            .map(Some)
            .ok_or_else(|| format!("{context}: `{field}` must be a signed 64-bit integer")),
    }
}

fn optional_i32(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &str,
    context: &str,
) -> Result<Option<i32>, String> {
    let Some(value) = optional_i64(object, field, context)? else {
        return Ok(None);
    };
    i32::try_from(value)
        .map(Some)
        .map_err(|_| format!("{context}: `{field}` value {value} is outside the i32 range"))
}

fn optional_f64(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &str,
    context: &str,
) -> Result<Option<f64>, String> {
    match object.get(field) {
        None => Ok(None),
        Some(value) => value
            .as_f64()
            .map(Some)
            .ok_or_else(|| format!("{context}: `{field}` must be a finite JSON number")),
    }
}

fn optional_i32_array(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &str,
    context: &str,
) -> Result<Option<Vec<i32>>, String> {
    let Some(value) = object.get(field) else {
        return Ok(None);
    };
    let array = value
        .as_array()
        .ok_or_else(|| format!("{context}: `{field}` must be an integer array"))?;
    array
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let value = value
                .as_i64()
                .ok_or_else(|| format!("{context}: `{field}[{index}]` must be an integer"))?;
            i32::try_from(value)
                .map_err(|_| format!("{context}: `{field}[{index}]` is outside the i32 range"))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn optional_string_array(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &str,
    context: &str,
) -> Result<Option<Vec<String>>, String> {
    let Some(value) = object.get(field) else {
        return Ok(None);
    };
    let array = value
        .as_array()
        .ok_or_else(|| format!("{context}: `{field}` must be a string array"))?;
    array
        .iter()
        .enumerate()
        .map(|(index, value)| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{context}: `{field}[{index}]` must be a string"))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

#[derive(Clone, Debug)]
pub struct PropertyTolerance {
    pub abs_nm: i32,
    pub rel_pct: f64,
}

impl Default for PropertyTolerance {
    fn default() -> Self {
        PropertyTolerance {
            abs_nm: 10,
            rel_pct: 0.02,
        }
    }
}

#[derive(Default)]
pub struct LvsSchema {
    pub cut_required: bool,
    pub w_tolerance: PropertyTolerance,
    pub l_tolerance: PropertyTolerance,
    pub fail_on_floating: bool,
    pub hierarchical: bool,
    pub equate_cells: Vec<(String, String)>,
}

/// PDK-driven connectivity: which layers conduct and which vias bridge them.
/// Replaces the hardcoded `connective_layers()` function.
#[derive(Clone, Default)]
pub struct ConnectivitySchema {
    pub conductors: Vec<String>,
    pub vias: Vec<ViaSchema>,
    /// Same-layer touching polygons connect (default true, matches KLayout).
    pub intra_layer_touch: bool,
    /// Global net names that connect across hierarchy boundaries.
    pub global_nets: Vec<String>,
}

#[derive(Clone)]
pub struct ViaSchema {
    pub layer: String,
    pub connects: Vec<String>,
}

/// PDK-driven device recognition rules.
#[derive(Clone, Default)]
pub struct DeviceSchema {
    pub mos_rules: Vec<MosRuleSchema>,
    pub bjt_rules: Vec<BjtRuleSchema>,
    pub resistor_rules: Vec<ResistorRuleSchema>,
    pub diode_rules: Vec<DiodeRuleSchema>,
    pub cap_rules: Vec<CapRuleSchema>,
    pub derived_layers: Vec<DerivedLayerSchema>,
}

/// MOS device: gate_layer over channel_layer, type chosen by implant.
#[derive(Clone)]
pub struct MosRuleSchema {
    pub name: String,
    pub gate_layer: String,
    pub channel_layer: String,
    pub type_implant: String,
    pub device_type: String,
    pub flavor_markers: Vec<(String, String)>,
    /// Well layer for body terminal extraction. None → body net = shared substrate.
    pub well_layer: Option<String>,
    /// Device class tag for comparison. None → "mos" (default). "dmos" gives a distinct kind.
    pub device_class: Option<String>,
}

/// BJT: three-layer intersection with type marker.
#[derive(Clone)]
pub struct BjtRuleSchema {
    pub name: String,
    pub collector_layer: String,
    pub base_layer: String,
    pub emitter_layer: String,
    pub type_marker: String,
    pub device_type: String, // "npn" or "pnp"
}

/// Derived layer: boolean operation on existing layers.
#[derive(Clone)]
pub struct DerivedLayerSchema {
    pub name: String,
    pub op: DerivedLayerOp,
}

#[derive(Clone)]
pub enum DerivedLayerOp {
    And(Vec<String>),
    Subtract { base: String, minus: String },
    Or(Vec<String>),
}

/// Configurable property combination for series/parallel merging.
#[derive(Clone, Debug, PartialEq)]
pub enum CombineRule {
    Add,
    Par,
    Min,
    Max,
    Critical,
}

/// Two-terminal resistor: body with marker, no gate crossing.
#[derive(Clone)]
pub struct ResistorRuleSchema {
    pub name: String,
    pub body_layer: String,
    pub marker_layer: String,
    pub terminal_layer: String,
}

/// Diode: junction of two layers with implant marker.
#[derive(Clone)]
pub struct DiodeRuleSchema {
    pub name: String,
    pub anode_layer: String,
    pub cathode_layer: String,
    pub implant_layer: String,
}

/// MIM/MOS capacitor: two plates with optional marker.
#[derive(Clone)]
pub struct CapRuleSchema {
    pub name: String,
    pub top_layer: String,
    pub bottom_layer: String,
    pub marker_layer: Option<String>,
}
