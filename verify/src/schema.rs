//! PDK information schema for the verify crate.
//!
//! Defines exactly what PDK parameters verify needs. The comprehensive PDK
//! reader in frontend/core/pdk.rs fills this schema — verify never reads
//! JSON directly.

use crate::params::PexLayerParams;
use std::collections::HashMap;

/// What the verify crate needs from a PDK.
pub struct VerifySchema {
    /// Layer definitions: name → (GDS layer, GDS datatype).
    pub layers: HashMap<String, (i32, i32)>,
    /// DRC rules.
    pub drc_rules: Vec<DrcRuleSchema>,
    /// PEX process constants keyed by layer name.
    pub pex: HashMap<String, PexLayerParams>,
    /// LVS extraction options.
    pub lvs: LvsSchema,
    /// PDK-driven connectivity (conductors + vias). Empty → legacy hardcoded fallback.
    pub connectivity: ConnectivitySchema,
    /// PDK-driven device recognition. Empty → legacy hardcoded MOS-only fallback.
    pub devices: DeviceSchema,
}

/// One DRC rule as provided by the PDK. Flat bag of optional fields —
/// `Deck::from_schema` resolves layer names and builds typed `DrcRuleParam`s.
pub struct DrcRuleSchema {
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
}

#[derive(Clone, Debug)]
pub struct PropertyTolerance {
    pub abs_nm: i32,
    pub rel_pct: f64,
}

impl Default for PropertyTolerance {
    fn default() -> Self {
        PropertyTolerance { abs_nm: 10, rel_pct: 0.02 }
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
pub enum CombineRule { Add, Par, Min, Max, Critical }

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
