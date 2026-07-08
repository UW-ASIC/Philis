//! Parameter-deck construction from a [`VerifySchema`].

use crate::geometry::LayerId;
use crate::schema::{PropertyTolerance, VerifySchema};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
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
}

/// One resolved DRC rule. This is a tagged union (enum) rather than a trait object: the DOD
/// skill's "tagged unions over virtual dispatch" — the rule engine `match`es over this in a
/// tight loop with no vtable indirection.
#[derive(Debug, Clone)]
pub enum DrcRuleParam {
    MinWidth { id: String, layer: LayerId, min: i32 },
    MinSpacing { id: String, layer: LayerId, min: i32 },
    MinSpacingDiff { id: String, a: LayerId, b: LayerId, min: i32 },
    MinEnclosure { id: String, outer: LayerId, inner: LayerId, min: i32 },
    MinExtension { id: String, layer: LayerId, reference: LayerId, min: i32 },
    MinArea { id: String, layer: LayerId, min: i64 },
    MaxWidth { id: String, layer: LayerId, max: i32 },
    Notch { id: String, layer: LayerId, min: i32 },
    MinEdgeLength { id: String, layer: LayerId, min: i32 },
    OffGrid { id: String, grid: i32 },
    Angle { id: String, allowed: Vec<i32> },
    MinDensity { id: String, layer: LayerId, window: i32, min_frac: f64 },
    MaxDensity { id: String, layer: LayerId, window: i32, max_frac: f64 },
    Overlap { id: String, a: LayerId, b: LayerId, min: i32 },
    CornerToCorner { id: String, layer: LayerId, min: i32 },
    Antenna { id: String, layer: LayerId, ratio: f64 },
    EolSpacing { id: String, layer: LayerId, eol_width: i32, eol_spacing: i32 },
    WideDependentSpacing { id: String, layer: LayerId, width_threshold: i32, wide_spacing: i32 },
    PrlSpacing { id: String, layer: LayerId, prl_threshold: i32, prl_spacing: i32 },
    AsymmetricEnclosure { id: String, outer: LayerId, inner: LayerId, min_one_side: i32 },
    MinEnclosedArea { id: String, layer: LayerId, min_hole_area: i64 },
    Cheesing { id: String, layer: LayerId, max_area_no_slot: i64 },
    RedundantVia { id: String, layer: LayerId, min_count: i32, within: i32 },
    ViaArraySpacing { id: String, layer: LayerId, array_threshold: i32, array_spacing: i32 },
    MaxDistanceToTap { id: String, diff_layer: LayerId, tap_layer: LayerId, max_dist: i32 },
    MultiPatterning { id: String, layer: LayerId, num_colors: i32, color_spacing: i32 },
}

impl DrcRuleParam {
    pub fn id(&self) -> &str {
        use DrcRuleParam::*;
        match self {
            MinWidth { id, .. } | MinSpacing { id, .. } | MinSpacingDiff { id, .. }
            | MinEnclosure { id, .. } | MinExtension { id, .. } | MinArea { id, .. }
            | MaxWidth { id, .. } | Notch { id, .. } | MinEdgeLength { id, .. }
            | OffGrid { id, .. } | Angle { id, .. } | MinDensity { id, .. }
            | MaxDensity { id, .. } | Overlap { id, .. } | CornerToCorner { id, .. }
            | Antenna { id, .. } | EolSpacing { id, .. }
            | WideDependentSpacing { id, .. }
            | PrlSpacing { id, .. } | AsymmetricEnclosure { id, .. }
            | MinEnclosedArea { id, .. } | Cheesing { id, .. }
            | RedundantVia { id, .. } | ViaArraySpacing { id, .. }
            | MaxDistanceToTap { id, .. } | MultiPatterning { id, .. } => id,
        }
    }
}

impl Deck {
    /// Build a `Deck` from a filled [`VerifySchema`].
    pub fn from_schema(schema: VerifySchema) -> Result<Deck, String> {
        let layer_defs: HashMap<String, LayerDef> = schema.layers.into_iter()
            .map(|(name, (layer, datatype))| (name, LayerDef { layer, datatype }))
            .collect();
        let layers = LayerTable::from_defs(&layer_defs);
        let lid = |name: Option<&str>| -> Result<LayerId, String> {
            let n = name.ok_or_else(|| "missing layer name".to_string())?;
            layers.id(n).ok_or_else(|| format!("unknown layer '{}'", n))
        };

        let mut drc_rules = Vec::new();
        for r in &schema.drc_rules {
            if !r.enabled { continue; }
            let rule = match r.kind.as_str() {
                "min_width" => {
                    let min = r.min.unwrap_or(0) as i32;
                    if min == 0 { continue; }
                    DrcRuleParam::MinWidth { id: r.kind.clone(), layer: lid(r.layer.as_deref())?, min }
                }
                "min_spacing" => {
                    let min = r.min.unwrap_or(0) as i32;
                    if min == 0 { continue; }
                    DrcRuleParam::MinSpacing { id: r.kind.clone(), layer: lid(r.layer.as_deref())?, min }
                }
                "min_spacing_diff" => {
                    let min = r.min.unwrap_or(0) as i32;
                    if min == 0 { continue; }
                    DrcRuleParam::MinSpacingDiff {
                        id: r.kind.clone(), a: lid(r.layer.as_deref())?,
                        b: lid(r.layer_b.as_deref())?, min,
                    }
                }
                "min_enclosure" | "well_enclosure" => {
                    let min = r.min.unwrap_or(0) as i32;
                    if min == 0 { continue; }
                    DrcRuleParam::MinEnclosure {
                        id: r.kind.clone(), outer: lid(r.outer.as_deref())?,
                        inner: lid(r.inner.as_deref())?, min,
                    }
                }
                "min_extension" => {
                    let min = r.min.unwrap_or(0) as i32;
                    if min == 0 { continue; }
                    DrcRuleParam::MinExtension {
                        id: r.kind.clone(), layer: lid(r.layer.as_deref())?,
                        reference: lid(r.reference.as_deref())?, min,
                    }
                }
                "min_area" => {
                    let min = r.min.unwrap_or(0);
                    if min == 0 { continue; }
                    DrcRuleParam::MinArea { id: r.kind.clone(), layer: lid(r.layer.as_deref())?, min }
                }
                "max_width" => {
                    let max = r.max.unwrap_or(0);
                    if max == 0 { continue; }
                    DrcRuleParam::MaxWidth { id: r.kind.clone(), layer: lid(r.layer.as_deref())?, max }
                }
                "notch" => {
                    let min = r.min.unwrap_or(0) as i32;
                    if min == 0 { continue; }
                    DrcRuleParam::Notch { id: r.kind.clone(), layer: lid(r.layer.as_deref())?, min }
                }
                "min_edge_length" => {
                    let min = r.min.unwrap_or(0) as i32;
                    if min == 0 { continue; }
                    DrcRuleParam::MinEdgeLength { id: r.kind.clone(), layer: lid(r.layer.as_deref())?, min }
                }
                "off_grid" => {
                    let grid = r.grid.unwrap_or(0);
                    if grid == 0 { continue; }
                    DrcRuleParam::OffGrid { id: r.kind.clone(), grid }
                }
                "angle" => {
                    DrcRuleParam::Angle {
                        id: r.kind.clone(),
                        allowed: r.allowed.clone().unwrap_or_default(),
                    }
                }
                "min_density" => {
                    DrcRuleParam::MinDensity {
                        id: r.kind.clone(), layer: lid(r.layer.as_deref())?,
                        window: r.window.unwrap_or(0),
                        min_frac: r.min_frac.unwrap_or(0.0),
                    }
                }
                "max_density" => {
                    DrcRuleParam::MaxDensity {
                        id: r.kind.clone(), layer: lid(r.layer.as_deref())?,
                        window: r.window.unwrap_or(0),
                        max_frac: r.max_frac.unwrap_or(1.0),
                    }
                }
                "overlap" => {
                    let min = r.min.unwrap_or(0) as i32;
                    if min == 0 { continue; }
                    DrcRuleParam::Overlap {
                        id: r.kind.clone(), a: lid(r.layer.as_deref())?,
                        b: lid(r.layer_b.as_deref())?, min,
                    }
                }
                "corner_to_corner" => {
                    let min = r.min.unwrap_or(0) as i32;
                    if min == 0 { continue; }
                    DrcRuleParam::CornerToCorner { id: r.kind.clone(), layer: lid(r.layer.as_deref())?, min }
                }
                "antenna" => {
                    let ratio = r.ratio.unwrap_or(0.0);
                    if ratio == 0.0 { continue; }
                    DrcRuleParam::Antenna { id: r.kind.clone(), layer: lid(r.layer.as_deref())?, ratio }
                }
                "eol_spacing" => {
                    let ew = r.eol_width.unwrap_or(0);
                    let es = r.eol_spacing.unwrap_or(0);
                    if ew == 0 || es == 0 { continue; }
                    DrcRuleParam::EolSpacing {
                        id: r.kind.clone(), layer: lid(r.layer.as_deref())?,
                        eol_width: ew, eol_spacing: es,
                    }
                }
                "wide_dependent_spacing" => {
                    let wt = r.width_threshold.unwrap_or(0);
                    let ws = r.wide_spacing.unwrap_or(0);
                    if wt == 0 || ws == 0 { continue; }
                    DrcRuleParam::WideDependentSpacing {
                        id: r.kind.clone(), layer: lid(r.layer.as_deref())?,
                        width_threshold: wt, wide_spacing: ws,
                    }
                }
                "prl_spacing" => {
                    let pt = r.prl_threshold.unwrap_or(0);
                    let ps = r.prl_spacing.unwrap_or(0);
                    if pt == 0 || ps == 0 { continue; }
                    DrcRuleParam::PrlSpacing {
                        id: r.kind.clone(), layer: lid(r.layer.as_deref())?,
                        prl_threshold: pt, prl_spacing: ps,
                    }
                }
                "asymmetric_enclosure" => {
                    let m = r.min.unwrap_or(0) as i32;
                    if m == 0 { continue; }
                    DrcRuleParam::AsymmetricEnclosure {
                        id: r.kind.clone(), outer: lid(r.outer.as_deref())?,
                        inner: lid(r.inner.as_deref())?, min_one_side: m,
                    }
                }
                "min_enclosed_area" => {
                    let m = r.min.unwrap_or(0);
                    if m == 0 { continue; }
                    DrcRuleParam::MinEnclosedArea {
                        id: r.kind.clone(), layer: lid(r.layer.as_deref())?, min_hole_area: m,
                    }
                }
                "cheesing" => {
                    let m = r.max.unwrap_or(0) as i64;
                    if m == 0 { continue; }
                    DrcRuleParam::Cheesing {
                        id: r.kind.clone(), layer: lid(r.layer.as_deref())?, max_area_no_slot: m,
                    }
                }
                "redundant_via" => {
                    let mc = r.min_count.unwrap_or(2);
                    let w = r.within.unwrap_or(0);
                    if w == 0 { continue; }
                    DrcRuleParam::RedundantVia {
                        id: r.kind.clone(), layer: lid(r.layer.as_deref())?,
                        min_count: mc, within: w,
                    }
                }
                "via_array_spacing" => {
                    let at = r.array_threshold.unwrap_or(0);
                    let asp = r.array_spacing.unwrap_or(0);
                    if at == 0 || asp == 0 { continue; }
                    DrcRuleParam::ViaArraySpacing {
                        id: r.kind.clone(), layer: lid(r.layer.as_deref())?,
                        array_threshold: at, array_spacing: asp,
                    }
                }
                "max_distance_to_tap" => {
                    let md = r.max_dist.unwrap_or(0);
                    if md == 0 { continue; }
                    DrcRuleParam::MaxDistanceToTap {
                        id: r.kind.clone(), diff_layer: lid(r.layer.as_deref())?,
                        tap_layer: lid(r.layer_b.as_deref())?, max_dist: md,
                    }
                }
                "multi_patterning" => {
                    let nc = r.num_colors.unwrap_or(2);
                    let cs = r.min.unwrap_or(0) as i32;
                    if cs == 0 { continue; }
                    DrcRuleParam::MultiPatterning {
                        id: r.kind.clone(), layer: lid(r.layer.as_deref())?,
                        num_colors: nc, color_spacing: cs,
                    }
                }
                other => return Err(format!("unknown DRC rule '{}'", other)),
            };
            drc_rules.push(rule);
        }
        drc_rules.sort_by(|a, b| a.id().cmp(b.id()));

        let mut pex = HashMap::new();
        for (name, p) in schema.pex {
            if let Some(id) = layers.id(&name) {
                pex.insert(id, p);
            }
        }

        // Resolve connectivity schema → ConnectivityConfig
        let connectivity = {
            let cs = &schema.connectivity;
            if cs.conductors.is_empty() {
                ConnectivityConfig::default()
            } else {
                let conductors = cs.conductors.iter()
                    .filter_map(|n| layers.id(n)).collect();
                let vias = cs.vias.iter().filter_map(|v| {
                    let vid = layers.id(&v.layer)?;
                    let connects = v.connects.iter()
                        .filter_map(|c| layers.id(c)).collect();
                    Some((vid, connects))
                }).collect();
                ConnectivityConfig { conductors, vias }
            }
        };

        // Resolve device schema → DeviceConfig
        let devices = {
            let ds = &schema.devices;
            let mos_rules = ds.mos_rules.iter().filter_map(|r| {
                Some(MosRule {
                    name: r.name.clone(),
                    gate_layer: layers.id(&r.gate_layer)?,
                    channel_layer: layers.id(&r.channel_layer)?,
                    type_implant: layers.id(&r.type_implant)?,
                    device_type: r.device_type.clone(),
                    flavor_markers: r.flavor_markers.iter().filter_map(|(l, f)| {
                        Some((layers.id(l)?, f.clone()))
                    }).collect(),
                    well_layer: r.well_layer.as_ref().and_then(|w| layers.id(w)),
                    device_class: r.device_class.clone(),
                })
            }).collect();
            let bjt_rules = ds.bjt_rules.iter().filter_map(|r| {
                Some(BjtRule {
                    name: r.name.clone(),
                    collector_layer: layers.id(&r.collector_layer)?,
                    base_layer: layers.id(&r.base_layer)?,
                    emitter_layer: layers.id(&r.emitter_layer)?,
                    type_marker: layers.id(&r.type_marker)?,
                    device_type: r.device_type.clone(),
                })
            }).collect();
            let resistor_rules = ds.resistor_rules.iter().filter_map(|r| {
                Some(ResistorRule {
                    name: r.name.clone(),
                    body_layer: layers.id(&r.body_layer)?,
                    marker_layer: layers.id(&r.marker_layer)?,
                    terminal_layer: layers.id(&r.terminal_layer)?,
                })
            }).collect();
            let diode_rules = ds.diode_rules.iter().filter_map(|r| {
                Some(DiodeRule {
                    name: r.name.clone(),
                    anode_layer: layers.id(&r.anode_layer)?,
                    cathode_layer: layers.id(&r.cathode_layer)?,
                    implant_layer: layers.id(&r.implant_layer)?,
                })
            }).collect();
            let cap_rules = ds.cap_rules.iter().filter_map(|r| {
                Some(CapRule {
                    name: r.name.clone(),
                    top_layer: layers.id(&r.top_layer)?,
                    bottom_layer: layers.id(&r.bottom_layer)?,
                    marker_layer: r.marker_layer.as_ref().and_then(|m| layers.id(m)),
                })
            }).collect();
            DeviceConfig { mos_rules, bjt_rules, resistor_rules, diode_rules, cap_rules }
        };

        Ok(Deck {
            layers, drc_rules, pex, dbu_nm: 1.0, lvs_cut_required: schema.lvs.cut_required,
            strict: false, connectivity, devices,
            w_tolerance: schema.lvs.w_tolerance, l_tolerance: schema.lvs.l_tolerance,
            fail_on_floating: schema.lvs.fail_on_floating,
            intra_layer_touch: schema.connectivity.intra_layer_touch,
            global_nets: schema.connectivity.global_nets.clone(),
        })
    }

    /// Parse a PDK deck from a JSON string. Delegates to `from_schema` internally.
    pub fn from_json(text: &str) -> Result<Self, String> {
        use crate::schema::{DrcRuleSchema, LvsSchema, VerifySchema};
        use serde::Deserialize;

        #[derive(Deserialize, Default)]
        struct RawDrc {
            #[serde(flatten)]
            rules: HashMap<String, serde_json::Value>,
        }

        #[derive(Deserialize)]
        struct Doc {
            #[serde(default)]
            layers: HashMap<String, LayerDef>,
            #[serde(default)]
            drc: HashMap<String, serde_json::Value>,
            #[serde(default)]
            pex: HashMap<String, PexLayerParams>,
            #[serde(default)]
            lvs: LvsRaw,
            #[serde(default)]
            connectivity: ConnRaw,
            #[serde(default)]
            device_recognition: DevRecogRaw,
        }

        #[derive(Deserialize, Default)]
        struct LvsRaw {
            #[serde(default)]
            cut_required: bool,
        }

        #[derive(Deserialize, Default)]
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
        struct ViaRaw {
            layer: String,
            connects: Vec<String>,
        }

        #[derive(Deserialize, Default)]
        struct DevRecogRaw {
            #[serde(default)]
            mos: Vec<MosRaw>,
        }
        #[derive(Deserialize)]
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

        let doc: Doc = serde_json::from_str(text).map_err(|e| e.to_string())?;

        let layers = doc.layers.into_iter()
            .map(|(name, def)| (name, (def.layer, def.datatype)))
            .collect();

        let v_i64 = |v: &serde_json::Value, k: &str| v.get(k).and_then(|x| x.as_i64());
        let v_f64 = |v: &serde_json::Value, k: &str| v.get(k).and_then(|x| x.as_f64());
        let v_str = |v: &serde_json::Value, k: &str| v.get(k).and_then(|x| x.as_str()).map(String::from);
        let v_bool = |v: &serde_json::Value, k: &str, d: bool| v.get(k).and_then(|x| x.as_bool()).unwrap_or(d);

        let drc_rules = doc.drc.iter().map(|(id, v)| {
            DrcRuleSchema {
                kind: id.clone(),
                enabled: v_bool(v, "enabled", true),
                layer: v_str(v, "layer").or_else(|| v_str(v, "layer_a")),
                layer_b: v_str(v, "layer_b"),
                outer: v_str(v, "outer"),
                inner: v_str(v, "inner"),
                reference: v_str(v, "ref"),
                min: v_i64(v, "min"),
                max: v_i64(v, "max").map(|x| x as i32),
                grid: v_i64(v, "grid").map(|x| x as i32),
                window: v_i64(v, "window").map(|x| x as i32),
                min_frac: v_f64(v, "min_frac"),
                max_frac: v_f64(v, "max_frac"),
                ratio: v_f64(v, "ratio"),
                eol_width: v_i64(v, "eol_width").map(|x| x as i32),
                eol_spacing: v_i64(v, "eol_spacing").map(|x| x as i32),
                allowed: v.get("allowed")
                    .and_then(|x| x.as_array())
                    .map(|a| a.iter().filter_map(|e| e.as_i64().map(|n| n as i32)).collect()),
                width_threshold: v_i64(v, "width_threshold").map(|x| x as i32),
                wide_spacing: v_i64(v, "wide_spacing").map(|x| x as i32),
                prl_threshold: v_i64(v, "prl_threshold").map(|x| x as i32),
                prl_spacing: v_i64(v, "prl_spacing").map(|x| x as i32),
                min_count: v_i64(v, "min_count").map(|x| x as i32),
                within: v_i64(v, "within").map(|x| x as i32),
                array_threshold: v_i64(v, "array_threshold").map(|x| x as i32),
                array_spacing: v_i64(v, "array_spacing").map(|x| x as i32),
                max_dist: v_i64(v, "max_dist").map(|x| x as i32),
                num_colors: v_i64(v, "num_colors").map(|x| x as i32),
            }
        }).collect();

        Self::from_schema(VerifySchema {
            layers,
            drc_rules,
            pex: doc.pex,
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
                if doc.connectivity.conductors.is_empty() {
                    ConnectivitySchema::default()
                } else {
                    ConnectivitySchema {
                        conductors: doc.connectivity.conductors,
                        vias: doc.connectivity.vias.into_iter().map(|v| ViaSchema {
                            layer: v.layer, connects: v.connects,
                        }).collect(),
                        intra_layer_touch: doc.connectivity.intra_layer_touch,
                        global_nets: doc.connectivity.global_nets,
                    }
                }
            },
            devices: {
                use crate::schema::{DeviceSchema, MosRuleSchema};
                if doc.device_recognition.mos.is_empty() {
                    DeviceSchema::default()
                } else {
                    DeviceSchema {
                        mos_rules: doc.device_recognition.mos.into_iter().map(|m| MosRuleSchema {
                            name: m.name, gate_layer: m.gate_layer,
                            channel_layer: m.channel_layer, type_implant: m.type_implant,
                            device_type: m.device_type, flavor_markers: m.flavor_markers,
                            well_layer: m.well_layer, device_class: m.device_class,
                        }).collect(),
                        ..Default::default()
                    }
                }
            },
        })
    }
}
