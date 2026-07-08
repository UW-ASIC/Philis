//! Device table extending the PDK deck JSON.
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
        _ => Err(serde::de::Error::custom(format!("unknown device type `{s}`"))),
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
        Self { devices: HashMap::new(), grid: 1 }
    }
}

#[derive(Deserialize)]
struct OffGrid {
    grid: i32,
}

#[derive(Deserialize, Default)]
struct DrcSection {
    #[serde(default)]
    off_grid: Option<OffGrid>,
}

#[derive(Deserialize)]
struct Doc {
    #[serde(default)]
    devices: HashMap<String, DeviceDef>,
    #[serde(default)]
    drc: DrcSection,
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
        let grid = doc.drc.off_grid.map_or(1, |g| g.grid.max(1));
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

// ---------------------------------------------------------------------------
// Comprehensive PDK loader — one JSON, all consumers served
// ---------------------------------------------------------------------------

use gdsverify::params::PexLayerParams;
use gdsverify::schema::{DrcRuleSchema, LvsSchema, VerifySchema};

#[derive(Deserialize)]
struct LayerDef {
    layer: i32,
    datatype: i32,
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
    #[serde(default)]
    well_layer: Option<String>,
    #[serde(default)]
    device_class: Option<String>,
}

#[derive(Deserialize)]
struct FullDoc {
    #[serde(default)]
    layers: HashMap<String, LayerDef>,
    #[serde(default)]
    drc: HashMap<String, serde_json::Value>,
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
}

fn v_bool(v: &serde_json::Value, k: &str, default: bool) -> bool {
    v.get(k).and_then(|x| x.as_bool()).unwrap_or(default)
}
fn v_i64(v: &serde_json::Value, k: &str) -> Option<i64> {
    v.get(k).and_then(|x| x.as_i64())
}
fn v_f64(v: &serde_json::Value, k: &str) -> Option<f64> {
    v.get(k).and_then(|x| x.as_f64())
}
fn v_str<'a>(v: &'a serde_json::Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(|x| x.as_str())
}

fn build_verify_schema(doc: &FullDoc) -> VerifySchema {
    let layers = doc.layers.iter()
        .map(|(name, def)| (name.clone(), (def.layer, def.datatype)))
        .collect();

    let drc_rules = doc.drc.iter().map(|(id, v)| {
        DrcRuleSchema {
            kind: id.clone(),
            enabled: v_bool(v, "enabled", true),
            layer: v_str(v, "layer").or_else(|| v_str(v, "layer_a")).map(String::from),
            layer_b: v_str(v, "layer_b").map(String::from),
            outer: v_str(v, "outer").map(String::from),
            inner: v_str(v, "inner").map(String::from),
            reference: v_str(v, "ref").map(String::from),
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

    VerifySchema {
        layers,
        drc_rules,
        pex: doc.pex.clone(),
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
            if doc.connectivity.conductors.is_empty() {
                ConnectivitySchema::default()
            } else {
                ConnectivitySchema {
                    conductors: doc.connectivity.conductors.clone(),
                    vias: doc.connectivity.vias.iter().map(|v| ViaSchema {
                        layer: v.layer.clone(), connects: v.connects.clone(),
                    }).collect(),
                    intra_layer_touch: doc.connectivity.intra_layer_touch,
                    global_nets: doc.connectivity.global_nets.clone(),
                }
            }
        },
        devices: {
            use gdsverify::schema::{DeviceSchema, MosRuleSchema};
            if doc.device_recognition.mos.is_empty() {
                DeviceSchema::default()
            } else {
                DeviceSchema {
                    mos_rules: doc.device_recognition.mos.iter().map(|m| MosRuleSchema {
                        name: m.name.clone(), gate_layer: m.gate_layer.clone(),
                        channel_layer: m.channel_layer.clone(), type_implant: m.type_implant.clone(),
                        device_type: m.device_type.clone(), flavor_markers: m.flavor_markers.clone(),
                        well_layer: m.well_layer.clone(), device_class: m.device_class.clone(),
                    }).collect(),
                    ..Default::default()
                }
            }
        },
    }
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
    let doc: FullDoc = serde_json::from_str(json).map_err(|e| e.to_string())?;
    if doc.connectivity.conductors.is_empty() {
        return Err("PDK incomplete: missing `connectivity.conductors` — \
            LVS extraction requires at least one conductor layer".into());
    }
    if doc.device_recognition.mos.is_empty() {
        return Err("PDK incomplete: missing `device_recognition.mos` — \
            LVS needs at least one MOS recognition rule".into());
    }
    if doc.devices.is_empty() {
        return Err("PDK incomplete: missing `devices` section — \
            netlist parsing needs device model definitions".into());
    }
    let verify_schema = build_verify_schema(&doc);
    let grid = doc.drc.get("off_grid")
        .and_then(|v| v.get("grid"))
        .and_then(|v| v.as_i64())
        .map_or(1, |g| g.max(1) as i32);
    let devices = doc.devices.into_iter()
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
}
