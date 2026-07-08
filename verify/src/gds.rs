//! Minimal GDSII reader — loads boundaries and text labels into per-cell `GeometryStore`s.
//!
//! GDSII is a big-endian, record-based binary format. A record is:
//!   [u16 length][u8 rec_type][u8 data_type][payload...]

use crate::geometry::GeometryStore;
use crate::params::LayerTable;
use std::collections::HashMap;

const HEADER: u8 = 0x00;
const BGNLIB: u8 = 0x01;
const BGNSTR: u8 = 0x05;
const STRNAME: u8 = 0x06;
const ENDSTR: u8 = 0x07;
const BOUNDARY: u8 = 0x08;
const TEXT: u8 = 0x0C;
const LAYER: u8 = 0x0D;
const DATATYPE: u8 = 0x0E;
const XY: u8 = 0x10;
const ENDEL: u8 = 0x11;
const STRING: u8 = 0x19;

/// Parsed GDS: one `GeometryStore` per cell (structure), keyed by name.
pub struct GdsLayout {
    pub cells: HashMap<String, GeometryStore>,
}

pub fn read_gds(bytes: &[u8], lt: &LayerTable) -> Result<GdsLayout, String> {
    let mut cells: HashMap<String, GeometryStore> = HashMap::new();
    let mut pos = 0usize;

    let mut cur_name: Option<String> = None;
    let mut cur_store = GeometryStore::new();

    // element-in-progress state
    let mut in_boundary = false;
    let mut in_text = false;
    let mut el_layer: i32 = 0;
    let mut el_datatype: i32 = 0;
    let mut el_xy: Vec<(i32, i32)> = Vec::new();
    let mut el_string = String::new();

    while pos + 4 <= bytes.len() {
        let len = u16::from_be_bytes([bytes[pos], bytes[pos + 1]]) as usize;
        if len < 4 { break; }
        let rec = bytes[pos + 2];
        let _dt = bytes[pos + 3];
        let payload = &bytes[pos + 4..(pos + len).min(bytes.len())];

        match rec {
            HEADER | BGNLIB => {}
            BGNSTR => {
                cur_store = GeometryStore::new();
                cur_name = None;
            }
            STRNAME => {
                cur_name = Some(read_ascii(payload));
            }
            BOUNDARY => {
                in_boundary = true;
                el_xy.clear();
            }
            TEXT => {
                in_text = true;
                el_xy.clear();
                el_string.clear();
            }
            LAYER => {
                if payload.len() >= 2 {
                    el_layer = i16::from_be_bytes([payload[0], payload[1]]) as i32;
                }
            }
            STRING => {
                el_string = read_ascii(payload);
            }
            DATATYPE => {
                if payload.len() >= 2 {
                    el_datatype = i16::from_be_bytes([payload[0], payload[1]]) as i32;
                }
            }
            XY => {
                el_xy.clear();
                let mut i = 0;
                while i + 8 <= payload.len() {
                    let x = i32::from_be_bytes([payload[i], payload[i + 1], payload[i + 2], payload[i + 3]]);
                    let y = i32::from_be_bytes([payload[i + 4], payload[i + 5], payload[i + 6], payload[i + 7]]);
                    el_xy.push((x, y));
                    i += 8;
                }
            }
            ENDEL => {
                if in_boundary {
                    // GDS boundary repeats the first point at the end; drop it.
                    let mut pts = el_xy.clone();
                    if pts.len() >= 2 && pts.first() == pts.last() {
                        pts.pop();
                    }
                    if let Some(lid) = lt.from_gds(el_layer, el_datatype) {
                        if pts.len() >= 3 {
                            cur_store.add_polygon(lid, &pts);
                        }
                    }
                }
                if in_text && !el_string.is_empty() && !el_xy.is_empty() {
                    let (x, y) = el_xy[0];
                    cur_store.add_text(el_layer, el_datatype, x, y, el_string.clone());
                }
                in_boundary = false;
                in_text = false;
            }
            ENDSTR => {
                if let Some(name) = cur_name.take() {
                    cells.insert(name, std::mem::take(&mut cur_store));
                }
            }
            _ => {}
        }
        pos += len;
        // records are word-aligned; length already even in valid GDS
    }

    Ok(GdsLayout { cells })
}

fn read_ascii(payload: &[u8]) -> String {
    let end = payload.iter().position(|&b| b == 0).unwrap_or(payload.len());
    String::from_utf8_lossy(&payload[..end]).trim().to_string()
}
