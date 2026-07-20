//! Validated, PDK-agnostic process data used by every backend stage.
//!
//! `pdks/<name>.json` gains a `"devices"` section mapping SPICE model names
//! to a device type plus the built-in generator (`cells/src/generators/`)
//! the device is drawn with. Custom PDK devices point `cell` at whichever
//! built-in generator they are *based off*. Groups of cells (macros) are
//! never defined here — those come from substrate3 `CellGenerator` impls.
//!
//! Same file also carries `"layers"`/`"drc"` (read by `Deck`) and `"cell"`
//! (read by `pnr_cells::pdk::Pdk`); each reader ignores the others' keys.

use std::collections::HashMap;

use pnr_cells::DeviceType;
use serde::{Deserialize, Deserializer};

/// Built-in generator a device maps onto (one per `cells/src/generators/*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CellBase {
    Mosfet,
    Resistor,
    Capacitor,
    Bjt,
    Diode,
    Inductor,
}

fn de_device_type<'de, D: Deserializer<'de>>(d: D) -> Result<DeviceType, D::Error> {
    let s = String::deserialize(d)?;
    match s.as_str() {
        "nmos" => Ok(DeviceType::Nmos),
        "pmos" => Ok(DeviceType::Pmos),
        "ncap" => Ok(DeviceType::Ncap),
        "pcap" => Ok(DeviceType::Pcap),
        "res" => Ok(DeviceType::Res),
        "cap" => Ok(DeviceType::Cap),
        "diode" => Ok(DeviceType::Diode),
        "bjt" => Ok(DeviceType::Bjt),
        _ => Err(serde::de::Error::custom(format!(
            "unknown device type `{s}`"
        ))),
    }
}

/// One PDK device model.
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceDef {
    /// Electrical classification.
    #[serde(rename = "type", deserialize_with = "de_device_type")]
    pub device_type: DeviceType,
    /// Generator this device is based off.
    pub cell: CellBase,
    /// Process-fixed default width in nm (models without `W=`).
    #[serde(default)]
    pub default_w: Option<i32>,
    /// Process-fixed default length in nm (models without `L=`).
    #[serde(default)]
    pub default_l: Option<i32>,
    /// DRC minimum width in nm — netlist W below this is clamped up
    /// (FinFET netlists carry sub-planar dims no planar process draws).
    #[serde(default)]
    pub min_w: Option<i32>,
    /// DRC minimum length in nm.
    #[serde(default)]
    pub min_l: Option<i32>,
}

/// Netlist-side PDK view: SPICE model name -> [`DeviceDef`] + grid.
#[derive(Debug, Clone)]
pub struct Pdk {
    devices: HashMap<String, DeviceDef>,
    grid: i32,
}

impl Default for Pdk {
    fn default() -> Self {
        Self {
            devices: HashMap::new(),
            grid: 1,
        }
    }
}

#[derive(Deserialize)]
struct Doc {
    #[serde(default)]
    devices: HashMap<String, DeviceDef>,
    #[serde(default)]
    drc: serde_json::Value,
}

impl Pdk {
    /// Parse the `"devices"` section (plus `drc.off_grid.grid`) from a PDK
    /// deck JSON. Model names are matched case-insensitively (SPICE convention).
    pub fn from_json(text: &str) -> Result<Self, String> {
        let doc: Doc = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let devices = doc
            .devices
            .into_iter()
            .map(|(k, v)| (k.to_ascii_lowercase(), v))
            .collect();
        let rules = gdsverify::schema::drc_rules_from_json(&doc.drc)?;
        let grid = manufacturing_grid(&rules)?;
        Ok(Self { devices, grid })
    }

    /// Look up a SPICE model name.
    pub fn device(&self, model: &str) -> Option<&DeviceDef> {
        self.devices.get(&model.to_ascii_lowercase())
    }

    /// Manufacturing grid in nm (`drc.off_grid.grid`, 1 if absent).
    pub fn grid(&self) -> i32 {
        self.grid
    }
}

fn manufacturing_grid(rules: &[gdsverify::schema::DrcRuleSchema]) -> Result<i32, String> {
    let mut grid = None;
    for rule in rules
        .iter()
        .filter(|rule| rule.enabled && rule.kind == "off_grid")
    {
        let value = rule.grid.ok_or_else(|| {
            format!(
                "DRC rule `{}` (off_grid): missing required `grid` parameter",
                rule.id
            )
        })?;
        if value <= 0 {
            return Err(format!(
                "DRC rule `{}` (off_grid): `grid` must be positive, got {value}",
                rule.id,
            ));
        }
        if let Some(previous) = grid {
            if previous != value {
                return Err(format!(
                    "PDK defines conflicting manufacturing grids {previous} and {value}"
                ));
            }
        } else {
            grid = Some(value);
        }
    }
    Ok(grid.unwrap_or(1))
}

// ---------------------------------------------------------------------------
// Comprehensive PDK loader — one JSON, all consumers served
// ---------------------------------------------------------------------------

use gdsverify::params::PexLayerParams;
use gdsverify::schema::{LvsSchema, VerifySchema};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LayerDef {
    layer: i32,
    datatype: i32,
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
    #[serde(default)]
    well_layer: Option<String>,
    #[serde(default)]
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

#[derive(Deserialize)]
struct FullDoc {
    #[serde(default)]
    layers: HashMap<String, LayerDef>,
    #[serde(default)]
    drc: serde_json::Value,
    #[serde(default)]
    pex: HashMap<String, PexLayerParams>,
    #[serde(default)]
    lvs: LvsRaw,
    #[serde(default)]
    cell: pnr_cells::pdk::Pdk,
    #[serde(default)]
    devices: HashMap<String, DeviceDef>,
    #[serde(default)]
    connectivity: ConnRaw,
    #[serde(default)]
    device_recognition: DevRecogRaw,
    #[serde(default)]
    erc: gdsverify::params::ErcParams,
}

fn build_verify_schema(doc: &FullDoc) -> Result<VerifySchema, String> {
    let layers = doc
        .layers
        .iter()
        .map(|(name, def)| (name.clone(), (def.layer, def.datatype)))
        .collect();

    let drc_rules = gdsverify::schema::drc_rules_from_json(&doc.drc)?;

    Ok(VerifySchema {
        layers,
        drc_rules,
        pex: doc.pex.clone(),
        erc: doc.erc.clone(),
        lvs: LvsSchema {
            cut_required: doc.lvs.cut_required,
            w_tolerance: gdsverify::schema::PropertyTolerance::default(),
            l_tolerance: gdsverify::schema::PropertyTolerance::default(),
            hierarchical: false,
            equate_cells: Vec::new(),
            fail_on_floating: false,
        },
        connectivity: {
            use gdsverify::schema::{ConnectivitySchema, ViaSchema};
            if doc.connectivity.conductors.is_empty() && doc.connectivity.vias.is_empty() {
                ConnectivitySchema::default()
            } else {
                ConnectivitySchema {
                    conductors: doc.connectivity.conductors.clone(),
                    vias: doc
                        .connectivity
                        .vias
                        .iter()
                        .map(|v| ViaSchema {
                            layer: v.layer.clone(),
                            connects: v.connects.clone(),
                        })
                        .collect(),
                    intra_layer_touch: doc.connectivity.intra_layer_touch,
                    global_nets: doc.connectivity.global_nets.clone(),
                }
            }
        },
        devices: {
            use gdsverify::schema::{
                BjtRuleSchema, CapRuleSchema, DeviceSchema, DiodeRuleSchema, MosRuleSchema,
                ResistorRuleSchema,
            };
            DeviceSchema {
                mos_rules: doc
                    .device_recognition
                    .mos
                    .iter()
                    .map(|m| MosRuleSchema {
                        name: m.name.clone(),
                        gate_layer: m.gate_layer.clone(),
                        channel_layer: m.channel_layer.clone(),
                        type_implant: m.type_implant.clone(),
                        device_type: m.device_type.clone(),
                        flavor_markers: m.flavor_markers.clone(),
                        well_layer: m.well_layer.clone(),
                        device_class: m.device_class.clone(),
                    })
                    .collect(),
                bjt_rules: doc
                    .device_recognition
                    .bjt
                    .iter()
                    .map(|rule| BjtRuleSchema {
                        name: rule.name.clone(),
                        collector_layer: rule.collector_layer.clone(),
                        base_layer: rule.base_layer.clone(),
                        emitter_layer: rule.emitter_layer.clone(),
                        type_marker: rule.type_marker.clone(),
                        device_type: rule.device_type.clone(),
                    })
                    .collect(),
                resistor_rules: doc
                    .device_recognition
                    .resistor
                    .iter()
                    .map(|rule| ResistorRuleSchema {
                        name: rule.name.clone(),
                        body_layer: rule.body_layer.clone(),
                        marker_layer: rule.marker_layer.clone(),
                        terminal_layer: rule.terminal_layer.clone(),
                    })
                    .collect(),
                diode_rules: doc
                    .device_recognition
                    .diode
                    .iter()
                    .map(|rule| DiodeRuleSchema {
                        name: rule.name.clone(),
                        anode_layer: rule.anode_layer.clone(),
                        cathode_layer: rule.cathode_layer.clone(),
                        implant_layer: rule.implant_layer.clone(),
                    })
                    .collect(),
                cap_rules: doc
                    .device_recognition
                    .cap
                    .iter()
                    .map(|rule| CapRuleSchema {
                        name: rule.name.clone(),
                        top_layer: rule.top_layer.clone(),
                        bottom_layer: rule.bottom_layer.clone(),
                        marker_layer: rule.marker_layer.clone(),
                    })
                    .collect(),
                derived_layers: Vec::new(),
            }
        },
    })
}

/// All PDK views loaded from a single `pdks/<name>.json`.
pub struct FullPdk {
    /// Netlist-side: SPICE model → generator type mapping.
    pub netlist: Pdk,
    /// Cell construction parameters (contact size, finger bounds, etc.).
    pub cells: pnr_cells::pdk::Pdk,
    /// Verify deck: layers, DRC rules, PEX constants, LVS params.
    pub deck: gdsverify::Deck,
}

/// Load all PDK views from one JSON string. Single parse, all consumers served.
///
/// Errors if required sections are missing — silent defaults hide broken PDKs.
pub fn load_pdk(json: &str) -> Result<FullPdk, String> {
    // Step 1: deserialize the shared document once. Individual backend stages
    // receive typed views derived from this same source of truth. Production
    // loading requires every construction parameter explicitly; Rust defaults
    // exist only for small unit tests and cannot define a real process.
    let raw: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let cell = raw
        .get("cell")
        .and_then(serde_json::Value::as_object)
        .ok_or("PDK incomplete: missing `cell` construction section")?;
    const CELL_KEYS: &[&str] = &[
        "contact",
        "sd_width",
        "min_finger_width",
        "max_finger_width",
        "poly_ext",
        "via_enclosure",
        "plate_spacing",
        "res_head",
        "well_enclosure",
        "via_spacing",
        "mcon_size",
        "m1_enc",
        "met1_space",
        "device_gap",
        "well_spacing",
        "dti",
        "res_corner_squares",
        "res_serpentine_aspect",
        "res_min_segment",
        "res_seg_gap",
        "bjt_max_emitter_stripe",
        "bjt_min_emitter_side",
        "bjt_base_frac",
        "bjt_collector_frac",
        "bjt_stripe_gap",
        "diode_gap",
        "ind_min_trace",
        "ind_min_diameter",
        "wpe_clearance_nm",
        "nwell_diff_enc",
        "min_guard_ring_width",
        "guard_licon_pitch",
        "p_epi_thickness",
        "p_well_depth",
        "n_well_depth",
        "retrograde_pwell",
        "lod_moat_ext_nm",
        "sheet_tolerance",
        "linewidth_control_nm",
    ];
    for key in CELL_KEYS {
        if !cell.contains_key(*key) {
            return Err(format!(
                "PDK incomplete: missing explicit `cell.{key}` construction parameter",
            ));
        }
    }
    let cell_layers = cell
        .get("layers")
        .and_then(serde_json::Value::as_object)
        .ok_or("PDK incomplete: missing `cell.layers` role map")?;
    const LAYER_KEYS: &[&str] = &[
        "diff",
        "poly",
        "rpoly",
        "li",
        "licon",
        "mcon",
        "met1",
        "nwell",
        "nsdm",
        "psdm",
        "tap",
        "routing_metals",
        "routing_vias",
    ];
    for key in LAYER_KEYS {
        if !cell_layers.contains_key(*key) {
            return Err(format!(
                "PDK incomplete: missing explicit `cell.layers.{key}` role",
            ));
        }
    }
    let doc: FullDoc = serde_json::from_value(raw).map_err(|e| e.to_string())?;

    if doc.cell.well_spacing < 0 {
        return Err("PDK invalid: `cell.well_spacing` must be non-negative".into());
    }
    if let Some(dti) = doc.cell.dti {
        if dti.shared_max_gap < 0 || dti.separated_min_gap <= dti.shared_max_gap {
            return Err("PDK invalid: DTI requires 0 <= shared_max_gap < separated_min_gap".into());
        }
    }

    // Step 2: reject missing electrical sections before any defaults can turn
    // an incomplete process into plausible-looking but invalid geometry.
    if doc.connectivity.conductors.is_empty() {
        return Err("PDK incomplete: missing `connectivity.conductors` — \
            LVS extraction requires at least one conductor layer"
            .into());
    }
    if doc.device_recognition.mos.is_empty() {
        return Err("PDK incomplete: missing `device_recognition.mos` — \
            LVS needs at least one MOS recognition rule"
            .into());
    }
    if doc.devices.is_empty() {
        return Err("PDK incomplete: missing `devices` section — \
            netlist parsing needs device model definitions"
            .into());
    }

    // Step 3: validate every physical layer role and the ordered routing stack
    // against this PDK's own layer table. Algorithms never infer process names.
    let layer_roles = [
        ("diffusion", &doc.cell.layers.diff),
        ("gate", &doc.cell.layers.poly),
        ("resistor body", &doc.cell.layers.rpoly),
        ("local interconnect", &doc.cell.layers.li),
        ("local contact", &doc.cell.layers.licon),
        ("metal contact", &doc.cell.layers.mcon),
        ("first metal", &doc.cell.layers.met1),
        ("well", &doc.cell.layers.nwell),
        ("n implant", &doc.cell.layers.nsdm),
        ("p implant", &doc.cell.layers.psdm),
        ("tap diffusion", &doc.cell.layers.tap),
    ];
    for (role, layer) in layer_roles {
        if !doc.layers.contains_key(layer) {
            return Err(format!(
                "PDK incomplete: `{layer}` selected for {role}, but no such layer exists",
            ));
        }
    }
    let metals = &doc.cell.layers.routing_metals;
    let vias = &doc.cell.layers.routing_vias;
    if metals.len() < 2 || vias.len() + 1 != metals.len() {
        return Err(format!(
            "PDK incomplete: routing stack needs N>=2 metals and N-1 vias; got {} and {}",
            metals.len(),
            vias.len(),
        ));
    }
    if metals.first() != Some(&doc.cell.layers.met1) {
        return Err("PDK inconsistent: `cell.layers.met1` must be routing_metals[0]".into());
    }
    for layer in metals.iter().chain(vias) {
        if !doc.layers.contains_key(layer) {
            return Err(format!(
                "PDK incomplete: routing layer `{layer}` is absent from `layers`",
            ));
        }
    }
    for metal in metals {
        if !doc.pex.contains_key(metal) {
            return Err(format!(
                "PDK incomplete: routing conductor `{metal}` has no PEX parameters",
            ));
        }
    }

    // Step 4: build verification, cell-construction, and netlist-model views.
    let verify_schema = build_verify_schema(&doc)?;
    let grid = manufacturing_grid(&verify_schema.drc_rules)?;
    let devices = doc
        .devices
        .into_iter()
        .map(|(k, v)| (k.to_ascii_lowercase(), v))
        .collect();
    Ok(FullPdk {
        netlist: Pdk { devices, grid },
        cells: doc.cell,
        deck: gdsverify::Deck::from_schema(verify_schema)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_devices_section() {
        let p = Pdk::from_json(
            r#"{
                "layers": {"met1": {"layer": 68, "datatype": 20}},
                "drc": {"off_grid": {"grid": 5}},
                "devices": {
                    "NFET_01v8": {"type": "nmos", "cell": "mosfet", "min_w": 420, "min_l": 150},
                    "res_po": {"type": "res", "cell": "resistor", "default_w": 1410}
                }
            }"#,
        )
        .unwrap();
        let n = p.device("nfet_01v8").expect("case-insensitive lookup");
        assert_eq!(n.device_type, DeviceType::Nmos);
        assert_eq!(n.cell, CellBase::Mosfet);
        assert_eq!(n.min_w, Some(420));
        assert_eq!(n.min_l, Some(150));
        assert_eq!(p.grid(), 5);
        let r = p.device("res_po").unwrap();
        assert_eq!(r.default_w, Some(1410));
        assert_eq!(r.default_l, None);
        assert!(p.device("nope").is_none());
    }

    #[test]
    fn unknown_type_errors() {
        let e = Pdk::from_json(r#"{"devices": {"x": {"type": "warp", "cell": "mosfet"}}}"#);
        assert!(e.is_err());
    }

    #[test]
    fn missing_section_is_empty() {
        let p = Pdk::from_json("{}").unwrap();
        assert!(p.device("nfet_01v8").is_none());
        assert_eq!(p.grid(), 1);
    }

    #[test]
    fn netlist_view_accepts_explicit_drc_array() {
        let p = Pdk::from_json(
            r#"{
                "drc": [
                    {"id": "GRID.1", "kind": "off_grid", "grid": 7}
                ]
            }"#,
        )
        .expect("explicit DRC array");
        assert_eq!(p.grid(), 7);
    }

    #[test]
    fn shipped_processes_define_every_backend_role() {
        for file in ["sky130.json", "generic_finfet.json"] {
            let json =
                std::fs::read_to_string(format!("{}/../pdks/{file}", env!("CARGO_MANIFEST_DIR"),))
                    .expect("read shipped PDK");
            let process = load_pdk(&json).expect("validate shipped PDK");

            assert!(process.cells.layers.routing_metals.len() >= 2);
            assert_eq!(
                process.cells.layers.routing_vias.len() + 1,
                process.cells.layers.routing_metals.len(),
            );
            if file == "sky130.json" {
                // pnp + npn recognition rules, each with its own marker layer.
                assert_eq!(process.deck.devices.bjt_rules.len(), 2);
            }
        }
    }

    #[test]
    fn comprehensive_loader_preserves_repeated_rule_kinds_and_ids() {
        let path = format!("{}/../pdks/sky130.json", env!("CARGO_MANIFEST_DIR"));
        let mut value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).expect("read PDK"))
                .expect("parse PDK JSON");
        let rules = value["drc"].as_object_mut().expect("DRC map");
        rules.insert(
            "M1.W.EXTRA".into(),
            serde_json::json!({"kind": "min_width", "layer": "met1", "min": 141}),
        );
        rules.insert(
            "M2.W.EXTRA".into(),
            serde_json::json!({"kind": "min_width", "layer": "met2", "min": 201}),
        );

        let process = load_pdk(&serde_json::to_string(&value).unwrap()).expect("full PDK");
        let ids: std::collections::HashSet<&str> = process
            .deck
            .drc_rules
            .iter()
            .map(gdsverify::DrcRuleParam::id)
            .collect();
        assert!(ids.contains("M1.W.EXTRA"));
        assert!(ids.contains("M2.W.EXTRA"));
    }

    #[test]
    fn comprehensive_loader_rejects_unknown_verification_properties() {
        let path = format!("{}/../pdks/sky130.json", env!("CARGO_MANIFEST_DIR"));
        let source = std::fs::read_to_string(path).expect("read shipped PDK");

        let mut value: serde_json::Value = serde_json::from_str(&source).expect("parse PDK JSON");
        value["connectivity"]
            .as_object_mut()
            .unwrap()
            .insert("conductros".into(), serde_json::json!([]));
        let error = load_pdk(&serde_json::to_string(&value).unwrap())
            .err()
            .expect("a connectivity typo must not be ignored");
        assert!(error.contains("unknown field `conductros`"), "{error}");

        let mut value: serde_json::Value = serde_json::from_str(&source).expect("parse PDK JSON");
        value["device_recognition"]["mos"][0]
            .as_object_mut()
            .unwrap()
            .insert("gate_layeer".into(), serde_json::json!("poly"));
        let error = load_pdk(&serde_json::to_string(&value).unwrap())
            .err()
            .expect("a device-recognition typo must not be ignored");
        assert!(error.contains("unknown field `gate_layeer`"), "{error}");

        let mut value: serde_json::Value = serde_json::from_str(&source).expect("parse PDK JSON");
        value["lvs"]
            .as_object_mut()
            .unwrap()
            .insert("cut_requried".into(), serde_json::json!(true));
        let error = load_pdk(&serde_json::to_string(&value).unwrap())
            .err()
            .expect("an LVS typo must not be ignored");
        assert!(error.contains("unknown field `cut_requried`"), "{error}");
    }

    #[test]
    fn production_factory_rejects_implicit_cell_defaults() {
        let error = load_pdk("{}").err().expect("incomplete PDK must fail");
        assert!(error.contains("cell"));
    }
}
