//! GDSII output.
//!
//! Minimal flat GDSII writer: one structure, BOUNDARY records only, 1 nm
//! database unit — the exact subset `gdsverify::read_gds` consumes, so every
//! write round-trips through signoff.

use std::io::Write;
use std::path::Path;

use gdsverify::{GeometryStore, LayerTable};

/// Annotation text label — emitted as a GDS TEXT record.
/// Layer/datatype are raw GDS numbers (not LayerTable ids).
pub struct TextLabel {
    pub x: i32,
    pub y: i32,
    pub layer: i16,
    pub datatype: i16,
    pub text: String,
}

// GDSII record types (type << 8 | data-format).
const HEADER: u16 = 0x0002;
const BGNLIB: u16 = 0x0102;
const LIBNAME: u16 = 0x0206;
const UNITS: u16 = 0x0305;
const BGNSTR: u16 = 0x0502;
const STRNAME: u16 = 0x0606;
const ENDSTR: u16 = 0x0700;
const ENDLIB: u16 = 0x0400;
const BOUNDARY: u16 = 0x0800;
const TEXT: u16 = 0x0C00;
const LAYER: u16 = 0x0D02;
const DATATYPE: u16 = 0x0E02;
const TEXTTYPE: u16 = 0x1602;
const XY: u16 = 0x1003;
const GDS_STRING: u16 = 0x1906;
const ENDEL: u16 = 0x1100;

fn rec(out: &mut Vec<u8>, rt: u16, data: &[u8]) {
    out.extend_from_slice(&(4 + data.len() as u16).to_be_bytes());
    out.extend_from_slice(&rt.to_be_bytes());
    out.extend_from_slice(data);
}

fn rec_i16(out: &mut Vec<u8>, rt: u16, vals: &[i16]) {
    let data: Vec<u8> = vals.iter().flat_map(|v| v.to_be_bytes()).collect();
    rec(out, rt, &data);
}

fn rec_str(out: &mut Vec<u8>, rt: u16, s: &str) {
    let mut data = s.as_bytes().to_vec();
    if data.len() % 2 == 1 {
        data.push(0); // GDS strings are even-padded
    }
    rec(out, rt, &data);
}

/// f64 -> GDSII 8-byte excess-64 real.
fn gds_real(v: f64) -> [u8; 8] {
    if v == 0.0 {
        return [0; 8];
    }
    let (mut m, mut e) = (v.abs(), 64i32);
    while m >= 1.0 {
        m /= 16.0;
        e += 1;
    }
    while m < 1.0 / 16.0 {
        m *= 16.0;
        e -= 1;
    }
    let mantissa = (m * 2f64.powi(56)) as u64;
    let mut b = mantissa.to_be_bytes();
    b[0] = (if v < 0.0 { 0x80 } else { 0 }) | (e as u8);
    b
}

/// Serialize a flat [`GeometryStore`] as one GDSII structure.
#[must_use]
pub fn gds_bytes(store: &GeometryStore, layers: &LayerTable, cell_name: &str, labels: &[TextLabel]) -> Vec<u8> {
    let mut out = Vec::with_capacity(64 + store.poly_count() * 64);
    let ts = [0i16; 12]; // fixed zero timestamps: deterministic output
    rec_i16(&mut out, HEADER, &[600]);
    rec_i16(&mut out, BGNLIB, &ts);
    rec_str(&mut out, LIBNAME, "pnr");
    // 1 nm DBU: 1e-3 user units per dbu, 1e-9 meters per dbu.
    let mut units = Vec::with_capacity(16);
    units.extend_from_slice(&gds_real(1e-3));
    units.extend_from_slice(&gds_real(1e-9));
    rec(&mut out, UNITS, &units);
    rec_i16(&mut out, BGNSTR, &ts);
    rec_str(&mut out, STRNAME, cell_name);

    for p in 0..store.poly_count() as u32 {
        let (start, end) = store.poly_range(gdsverify::PolyId(p));
        let (layer, datatype) = layers.id_to_gds[store.poly_layer[p as usize] as usize];
        rec(&mut out, BOUNDARY, &[]);
        rec_i16(&mut out, LAYER, &[layer as i16]);
        rec_i16(&mut out, DATATYPE, &[datatype as i16]);
        let mut xy = Vec::with_capacity((end - start + 1) * 8);
        for i in 0..(end - start) {
            let (x, y) = store.poly_vertex(start, i);
            xy.extend_from_slice(&x.to_be_bytes());
            xy.extend_from_slice(&y.to_be_bytes());
        }
        // close the loop
        let (x0, y0) = store.poly_vertex(start, 0);
        xy.extend_from_slice(&x0.to_be_bytes());
        xy.extend_from_slice(&y0.to_be_bytes());
        rec(&mut out, XY, &xy);
        rec(&mut out, ENDEL, &[]);
    }

    for lbl in labels {
        rec(&mut out, TEXT, &[]);
        rec_i16(&mut out, LAYER, &[lbl.layer]);
        rec_i16(&mut out, TEXTTYPE, &[lbl.datatype]);
        let mut xy = Vec::with_capacity(8);
        xy.extend_from_slice(&lbl.x.to_be_bytes());
        xy.extend_from_slice(&lbl.y.to_be_bytes());
        rec(&mut out, XY, &xy);
        rec_str(&mut out, GDS_STRING, &lbl.text);
        rec(&mut out, ENDEL, &[]);
    }

    rec(&mut out, ENDSTR, &[]);
    rec(&mut out, ENDLIB, &[]);
    out
}

/// Write the store to a .gds file.
#[allow(clippy::missing_errors_doc)]
pub fn write_gds(
    store: &GeometryStore,
    layers: &LayerTable,
    cell_name: &str,
    path: &Path,
    labels: &[TextLabel],
) -> std::io::Result<()> {
    std::fs::File::create(path)?.write_all(&gds_bytes(store, layers, cell_name, labels))
}

/// Serialize signoff results as JSON for the visualizer sidecar.
pub fn signoff_json(s: &crate::orchestrator::SignoffReport) -> String {
    use std::fmt::Write;
    let mut j = String::from("{\n");
    // DRC
    let _ = write!(j, "  \"drc_violations\": [");
    for (i, v) in s.drc_blocking.iter().enumerate() {
        if i > 0 { j.push(','); }
        let _ = write!(j, "\n    {{\"rule\":\"{}\",\"kind\":\"{}\",\"layer\":\"{}\",\"measured\":{},\"limit\":{},\"x\":{},\"y\":{}}}",
            v.rule_id, v.kind, v.layer, v.measured, v.limit, v.x, v.y);
    }
    j.push_str("\n  ],\n");
    // LVS
    let _ = write!(j, "  \"lvs\": {{\"matched\":{},\"reason\":\"{}\",\"nmos\":{},\"pmos\":{},\"floating_nets\":{}}},\n",
        s.lvs.matched, s.lvs.reason.replace('"', "\\\""), s.lvs.nmos, s.lvs.pmos, s.lvs.floating_nets.len());
    // PEX summary
    let total_r_met1 = s.pex.total_resistance("met1");
    let total_c = s.pex.total_cap();
    let _ = write!(j, "  \"pex\": {{\"r_met1_ohm\":{total_r_met1:.2},\"total_cap_af\":{total_c:.2}}}\n");
    j.push('}');
    j
}

#[cfg(test)]
mod tests {
    use super::*;
    use gdsverify::read_gds;
    use std::collections::HashMap;

    fn table() -> LayerTable {
        let defs: HashMap<String, gdsverify::LayerDef> = serde_json::from_str(
            r#"{"met1": {"layer": 68, "datatype": 20}, "poly": {"layer": 66, "datatype": 20}}"#,
        )
        .unwrap();
        LayerTable::from_defs(&defs)
    }

    #[test]
    fn roundtrips_through_gdsverify() {
        let lt = table();
        let met1 = lt.id("met1").unwrap();
        let poly = lt.id("poly").unwrap();
        let mut store = GeometryStore::new();
        store.add_rect(met1, 0, 0, 1000, 290);
        store.add_rect(poly, 250, -130, 150, 560);
        store.add_polygon(met1, &[(0, 0), (500, 0), (500, 500), (250, 500), (250, 250), (0, 250)]);

        let bytes = gds_bytes(&store, &lt, "top", &[]);
        let layout = read_gds(&bytes, &lt).expect("read back");
        let cell = layout.cells.get("top").expect("cell present");
        assert_eq!(cell.poly_count(), 3);
        let b = cell.poly_bbox[0];
        assert_eq!((b.xmin, b.ymin, b.xmax, b.ymax), (0, 0, 1000, 290));
        assert_eq!(cell.poly_layer[1], poly);
    }

    #[test]
    fn text_labels_roundtrip() {
        let lt = table();
        let met1 = lt.id("met1").unwrap();
        let mut store = GeometryStore::new();
        store.add_rect(met1, 0, 0, 1000, 500);
        let labels = vec![
            TextLabel { x: 500, y: 250, layer: 236, datatype: 0, text: "XM1".into() },
            TextLabel { x: 100, y: 100, layer: 236, datatype: 1, text: "diff_pair".into() },
        ];
        let bytes = gds_bytes(&store, &lt, "labeled", &labels);
        let layout = read_gds(&bytes, &lt).expect("read back");
        let cell = layout.cells.get("labeled").expect("cell present");
        assert_eq!(cell.poly_count(), 1);
        assert_eq!(cell.text_count(), 2);
        assert_eq!(cell.text_string[0], "XM1");
        assert_eq!((cell.text_x[0], cell.text_y[0]), (500, 250));
        assert_eq!(cell.text_string[1], "diff_pair");
        assert_eq!(cell.text_layer[1], 236);
    }
}
