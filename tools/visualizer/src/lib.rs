//! GPU-accelerated GDS / GeometryStore visualizer.
//!
//! Two modes:
//! - **Standalone binary**: `pnr-visualizer <file.gds>` — watches file, auto-reloads.
//! - **In-process probe**: `Probe::open()` — spawns a window thread, accepts SoA
//!   snapshots from the P&R iteration loop via a channel.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant, SystemTime},
};

use bytemuck::{Pod, Zeroable};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

// ═══════════════════════════════════════════════════════════════════════
//  Public types
// ═══════════════════════════════════════════════════════════════════════

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Vertex {
    pub pos: [f32; 2],
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct CamUni {
    pub offset: [f32; 2],
    pub scale: f32,
    pub aspect: f32,
}

#[derive(Clone)]
pub struct Poly {
    pub layer: u16,
    pub pts: Vec<[i32; 2]>,
}

#[derive(Clone)]
pub struct TextEntry {
    pub x: i32,
    pub y: i32,
    pub text: String,
}

/// GDS (layer, datatype) → human-readable name.
pub type LayerMap = HashMap<(i32, i32), String>;

/// Parse layer names from PDK JSON. Expects `{"layers": {"met1": [68, 20], ...}}`.
pub fn parse_layer_names(json: &str) -> LayerMap {
    let v: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return LayerMap::new(),
    };
    let mut map = LayerMap::new();
    if let Some(layers) = v.get("layers").and_then(|l| l.as_object()) {
        for (name, val) in layers {
            if let Some(arr) = val.as_array() {
                if arr.len() >= 2 {
                    if let (Some(l), Some(d)) = (arr[0].as_i64(), arr[1].as_i64()) {
                        map.insert((l as i32, d as i32), name.clone());
                    }
                }
            }
        }
    }
    map
}

/// Parsed signoff sidecar data for visualization.
#[derive(Default, Clone)]
pub struct SignoffData {
    pub drc: Vec<DrcMarker>,
    pub lvs_matched: bool,
    pub lvs_reason: String,
    pub lvs_nmos: usize,
    pub lvs_pmos: usize,
    pub pex_r_ohm: f64,
    pub pex_c_af: f64,
}

#[derive(Clone)]
pub struct DrcMarker {
    pub x: i32,
    pub y: i32,
    pub rule: String,
    pub kind: String,
    pub measured: i64,
    pub limit: i64,
}

/// Parse signoff.json sidecar.
pub fn parse_signoff(json: &str) -> SignoffData {
    let v: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return SignoffData::default(),
    };
    let mut sd = SignoffData::default();
    if let Some(arr) = v.get("drc_violations").and_then(|a| a.as_array()) {
        for item in arr {
            sd.drc.push(DrcMarker {
                x: item.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                y: item.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                rule: item.get("rule").and_then(|v| v.as_str()).unwrap_or("").into(),
                kind: item.get("kind").and_then(|v| v.as_str()).unwrap_or("").into(),
                measured: item.get("measured").and_then(|v| v.as_i64()).unwrap_or(0),
                limit: item.get("limit").and_then(|v| v.as_i64()).unwrap_or(0),
            });
        }
    }
    if let Some(lvs) = v.get("lvs") {
        sd.lvs_matched = lvs.get("matched").and_then(|v| v.as_bool()).unwrap_or(false);
        sd.lvs_reason = lvs.get("reason").and_then(|v| v.as_str()).unwrap_or("").into();
        sd.lvs_nmos = lvs.get("nmos").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        sd.lvs_pmos = lvs.get("pmos").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    }
    if let Some(pex) = v.get("pex") {
        sd.pex_r_ohm = pex.get("r_met1_ohm").and_then(|v| v.as_f64()).unwrap_or(0.0);
        sd.pex_c_af = pex.get("total_cap_af").and_then(|v| v.as_f64()).unwrap_or(0.0);
    }
    sd
}

// ═══════════════════════════════════════════════════════════════════════
//  GDS parser — full: BOUNDARY, BOX, PATH, SREF, AREF
// ═══════════════════════════════════════════════════════════════════════

struct GdsCell {
    polys: Vec<Poly>,
    srefs: Vec<SRef>,
    texts: Vec<TextEntry>,
}

struct SRef {
    name: String,
    x: i32,
    y: i32,
    mirror_x: bool,
    angle_deg: f64,
    cols: u16,
    rows: u16,
    col_vec: (i32, i32),
    row_vec: (i32, i32),
}

fn gds_real(b: &[u8]) -> f64 {
    let sign = if b[0] & 0x80 != 0 { -1.0 } else { 1.0 };
    let exp = (b[0] & 0x7F) as i32 - 64;
    let mut mant: u64 = 0;
    for &byte in &b[1..8] {
        mant = (mant << 8) | byte as u64;
    }
    sign * (mant as f64 / (1u64 << 56) as f64) * 16.0f64.powi(exp)
}

fn read_i16(data: &[u8], off: usize) -> i16 {
    i16::from_be_bytes([data[off], data[off + 1]])
}

fn read_i32(data: &[u8], off: usize) -> i32 {
    i32::from_be_bytes(data[off..off + 4].try_into().unwrap())
}

fn read_string(data: &[u8], off: usize, len: usize) -> String {
    let s = &data[off..off + len];
    let end = s.iter().position(|&b| b == 0).unwrap_or(len);
    String::from_utf8_lossy(&s[..end]).into_owned()
}

fn path_to_polys(layer: u16, half_w: i32, pts: &[[i32; 2]]) -> Vec<Poly> {
    let mut out = Vec::new();
    for seg in pts.windows(2) {
        let [x0, y0] = seg[0];
        let [x1, y1] = seg[1];
        let hw = half_w;
        let rect = if y0 == y1 {
            let (lx, rx) = (x0.min(x1), x0.max(x1));
            vec![[lx, y0 - hw], [rx, y0 - hw], [rx, y0 + hw], [lx, y0 + hw]]
        } else if x0 == x1 {
            let (by, ty) = (y0.min(y1), y0.max(y1));
            vec![[x0 - hw, by], [x0 + hw, by], [x0 + hw, ty], [x0 - hw, ty]]
        } else {
            let dx = (x1 - x0) as f64;
            let dy = (y1 - y0) as f64;
            let len = (dx * dx + dy * dy).sqrt();
            let nx = (-dy / len * hw as f64) as i32;
            let ny = (dx / len * hw as f64) as i32;
            vec![
                [x0 + nx, y0 + ny], [x1 + nx, y1 + ny],
                [x1 - nx, y1 - ny], [x0 - nx, y0 - ny],
            ]
        };
        out.push(Poly { layer, pts: rect });
    }
    out
}

fn parse_gds_cells(data: &[u8]) -> HashMap<String, GdsCell> {
    let mut cells: HashMap<String, GdsCell> = HashMap::new();
    let mut i = 0;

    let mut cur_name: Option<String> = None;
    let mut cur_polys: Vec<Poly> = Vec::new();
    let mut cur_srefs: Vec<SRef> = Vec::new();
    let mut cur_texts: Vec<TextEntry> = Vec::new();

    // Element state
    let mut in_boundary = false;
    let mut in_path = false;
    let mut in_sref = false;
    let mut in_aref = false;
    let mut in_text = false;
    let mut layer: u16 = 0;
    let mut path_width: i32 = 0;
    let mut sref_name = String::new();
    let mut sref_mirror = false;
    let mut sref_angle = 0.0f64;
    let mut aref_cols: u16 = 1;
    let mut aref_rows: u16 = 1;
    let mut text_string = String::new();
    let mut text_xy: (i32, i32) = (0, 0);

    while i + 4 <= data.len() {
        let len = u16::from_be_bytes([data[i], data[i + 1]]) as usize;
        if len < 4 || i + len > data.len() {
            break;
        }
        let rec = data[i + 2];

        match rec {
            // Structure
            0x05 => {
                // BGNSTR
                cur_polys.clear();
                cur_srefs.clear();
                cur_texts.clear();
            }
            0x06 => {
                // STRNAME
                cur_name = Some(read_string(data, i + 4, len - 4));
            }
            0x07 => {
                // ENDSTR
                if let Some(name) = cur_name.take() {
                    cells.insert(name, GdsCell {
                        polys: std::mem::take(&mut cur_polys),
                        srefs: std::mem::take(&mut cur_srefs),
                        texts: std::mem::take(&mut cur_texts),
                    });
                }
            }

            // Element begin
            0x08 | 0x2D => {
                in_boundary = true;
                layer = 0;
            }
            0x09 => {
                in_path = true;
                layer = 0;
                path_width = 0;
            }
            0x0A => {
                in_sref = true;
                sref_mirror = false;
                sref_angle = 0.0;
            }
            0x0B => {
                in_aref = true;
                sref_mirror = false;
                sref_angle = 0.0;
                aref_cols = 1;
                aref_rows = 1;
            }
            0x0C => {
                // TEXT
                in_text = true;
                text_string.clear();
                text_xy = (0, 0);
            }

            // LAYER
            0x0D if (in_boundary || in_path) && len >= 6 => {
                layer = read_i16(data, i + 4) as u16;
            }

            // WIDTH (path)
            0x0F if in_path && len >= 8 => {
                path_width = read_i32(data, i + 4);
            }

            // STRING (text content)
            0x19 if in_text => {
                text_string = read_string(data, i + 4, len - 4);
            }

            // SNAME
            0x12 if in_sref || in_aref => {
                sref_name = read_string(data, i + 4, len - 4);
            }

            // COLROW (aref)
            0x13 if in_aref && len >= 8 => {
                aref_cols = read_i16(data, i + 4) as u16;
                aref_rows = read_i16(data, i + 6) as u16;
            }

            // STRANS
            0x1A if (in_sref || in_aref) && len >= 6 => {
                let flags = u16::from_be_bytes([data[i + 4], data[i + 5]]);
                sref_mirror = flags & 0x8000 != 0;
            }

            // ANGLE
            0x1C if (in_sref || in_aref) && len >= 12 => {
                sref_angle = gds_real(&data[i + 4..i + 12]);
            }

            // XY
            0x10 => {
                let np = (len - 4) / 8;
                let mut pts: Vec<[i32; 2]> = Vec::with_capacity(np);
                for j in 0..np {
                    let o = i + 4 + j * 8;
                    if o + 8 > data.len() { break; }
                    pts.push([read_i32(data, o), read_i32(data, o + 4)]);
                }

                if in_boundary {
                    if pts.len() > 1 && pts.first() == pts.last() { pts.pop(); }
                    if pts.len() >= 3 {
                        cur_polys.push(Poly { layer, pts });
                    }
                } else if in_path {
                    if pts.len() >= 2 {
                        let hw = (path_width / 2).max(1);
                        cur_polys.extend(path_to_polys(layer, hw, &pts));
                    }
                } else if in_sref && !pts.is_empty() {
                    cur_srefs.push(SRef {
                        name: sref_name.clone(),
                        x: pts[0][0], y: pts[0][1],
                        mirror_x: sref_mirror, angle_deg: sref_angle,
                        cols: 1, rows: 1,
                        col_vec: (0, 0), row_vec: (0, 0),
                    });
                } else if in_aref && pts.len() >= 3 {
                    let (x0, y0) = (pts[0][0], pts[0][1]);
                    cur_srefs.push(SRef {
                        name: sref_name.clone(),
                        x: x0, y: y0,
                        mirror_x: sref_mirror, angle_deg: sref_angle,
                        cols: aref_cols, rows: aref_rows,
                        col_vec: ((pts[1][0] - x0) / aref_cols.max(1) as i32,
                                  (pts[1][1] - y0) / aref_cols.max(1) as i32),
                        row_vec: ((pts[2][0] - x0) / aref_rows.max(1) as i32,
                                  (pts[2][1] - y0) / aref_rows.max(1) as i32),
                    });
                } else if in_text && !pts.is_empty() {
                    text_xy = (pts[0][0], pts[0][1]);
                }
            }

            // ENDEL
            0x11 => {
                if in_text && !text_string.is_empty() {
                    cur_texts.push(TextEntry {
                        x: text_xy.0, y: text_xy.1,
                        text: text_string.clone(),
                    });
                }
                in_boundary = false;
                in_path = false;
                in_sref = false;
                in_aref = false;
                in_text = false;
            }
            _ => {}
        }
        i += len;
    }
    cells
}

fn transform_point(x: i32, y: i32, mirror_x: bool, angle_deg: f64) -> (i32, i32) {
    let (px, mut py) = (x as f64, y as f64);
    if mirror_x { py = -py; }
    let rad = angle_deg.to_radians();
    let (s, c) = (rad.sin(), rad.cos());
    ((px * c - py * s).round() as i32, (px * s + py * c).round() as i32)
}

fn flatten_cell(
    cells: &HashMap<String, GdsCell>,
    name: &str,
    tx: i32, ty: i32,
    mirror: bool, angle: f64,
    out: &mut Vec<Poly>,
    out_texts: &mut Vec<TextEntry>,
    depth: usize,
) {
    if depth > 64 { return; }
    let cell = match cells.get(name) {
        Some(c) => c,
        None => return,
    };
    for poly in &cell.polys {
        let pts: Vec<[i32; 2]> = poly.pts.iter().map(|&[x, y]| {
            let (rx, ry) = transform_point(x, y, mirror, angle);
            [rx + tx, ry + ty]
        }).collect();
        out.push(Poly { layer: poly.layer, pts });
    }
    for t in &cell.texts {
        let (rx, ry) = transform_point(t.x, t.y, mirror, angle);
        out_texts.push(TextEntry { x: rx + tx, y: ry + ty, text: t.text.clone() });
    }
    for sref in &cell.srefs {
        for row in 0..sref.rows {
            for col in 0..sref.cols {
                let sx = sref.x + col as i32 * sref.col_vec.0 + row as i32 * sref.row_vec.0;
                let sy = sref.y + col as i32 * sref.col_vec.1 + row as i32 * sref.row_vec.1;
                let (px, py) = transform_point(sx, sy, mirror, angle);
                let m = mirror ^ sref.mirror_x;
                let a = if mirror { angle - sref.angle_deg } else { angle + sref.angle_deg };
                flatten_cell(cells, &sref.name, tx + px, ty + py, m, a, out, out_texts, depth + 1);
            }
        }
    }
}

fn find_top_cell(cells: &HashMap<String, GdsCell>) -> Option<String> {
    let mut referenced = std::collections::HashSet::new();
    for cell in cells.values() {
        for sref in &cell.srefs {
            referenced.insert(sref.name.clone());
        }
    }
    cells.keys().find(|k| !referenced.contains(*k)).cloned()
        .or_else(|| cells.keys().next().cloned())
}

/// Parse a GDS file and return all polygons + text labels (flattened hierarchy).
pub fn parse_gds(data: &[u8]) -> (Vec<Poly>, Vec<TextEntry>) {
    let cells = parse_gds_cells(data);
    let top = match find_top_cell(&cells) {
        Some(t) => t,
        None => return (Vec::new(), Vec::new()),
    };
    let mut out = Vec::new();
    let mut texts = Vec::new();
    flatten_cell(&cells, &top, 0, 0, false, 0.0, &mut out, &mut texts, 0);
    (out, texts)
}

// ═══════════════════════════════════════════════════════════════════════
//  Dump file parser (SoA text format, multiple frames)
// ═══════════════════════════════════════════════════════════════════════

/// One frame from a dump file.
pub struct DumpFrame {
    pub label: String,
    pub polys: Vec<Poly>,
    pub texts: Vec<TextEntry>,
}

/// Parse a dump file with one or more frames.
///
/// Format:
/// ```text
/// # frame 0 wl=36.9 overuse=0
/// 68 100,200 300,200 300,400 100,400
/// 69 500,100 700,100 700,300
/// # frame 1 wl=35.2 overuse=0
/// 68 110,210 310,210 310,410 110,410
/// ```
pub fn parse_dump(text: &str) -> Vec<DumpFrame> {
    let mut frames = Vec::new();
    let mut label = String::new();
    let mut polys = Vec::new();
    let mut texts = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("# frame") {
            if !polys.is_empty() || !texts.is_empty() {
                frames.push(DumpFrame { label: std::mem::take(&mut label), polys: std::mem::take(&mut polys), texts: std::mem::take(&mut texts) });
            }
            label = line.strip_prefix("# ").unwrap_or(line).to_string();
        } else if line.is_empty() || line.starts_with('#') {
            continue;
        } else if line.starts_with("T ") {
            // Text label: T x,y label text
            let rest = &line[2..];
            if let Some((coord, text)) = rest.split_once(' ') {
                if let Some((xs, ys)) = coord.split_once(',') {
                    if let (Ok(x), Ok(y)) = (xs.parse(), ys.parse()) {
                        texts.push(TextEntry { x, y, text: text.to_string() });
                    }
                }
            }
        } else {
            let mut parts = line.split_whitespace();
            let layer: u16 = match parts.next().and_then(|s| s.parse().ok()) {
                Some(l) => l,
                None => continue,
            };
            let pts: Vec<[i32; 2]> = parts.filter_map(|s| {
                let (x, y) = s.split_once(',')?;
                Some([x.parse().ok()?, y.parse().ok()?])
            }).collect();
            if pts.len() >= 3 {
                polys.push(Poly { layer, pts });
            }
        }
    }
    if !polys.is_empty() || !texts.is_empty() {
        frames.push(DumpFrame { label, polys, texts });
    }
    frames
}

// ═══════════════════════════════════════════════════════════════════════
//  GeometryStore → Poly (behind "probe" feature)
// ═══════════════════════════════════════════════════════════════════════

#[cfg(feature = "probe")]
pub fn store_to_polys(store: &gdsverify::GeometryStore) -> Vec<Poly> {
    let n = store.poly_count();
    let mut polys = Vec::with_capacity(n);
    for i in 0..n {
        let start = store.poly_vert_start[i] as usize;
        let len = store.poly_vert_len[i] as usize;
        let pts: Vec<[i32; 2]> = (0..len)
            .map(|j| [store.verts_x[start + j], store.verts_y[start + j]])
            .collect();
        if pts.len() >= 3 {
            polys.push(Poly { layer: store.poly_layer[i], pts });
        }
    }
    polys
}

// ═══════════════════════════════════════════════════════════════════════
//  Colors & triangulation
// ═══════════════════════════════════════════════════════════════════════

pub fn layer_color(l: u16) -> [f32; 4] {
    const P: [[f32; 4]; 10] = [
        [0.22, 0.60, 1.00, 0.60],
        [1.00, 0.33, 0.33, 0.60],
        [0.30, 0.85, 0.40, 0.60],
        [1.00, 0.80, 0.20, 0.60],
        [0.75, 0.35, 0.85, 0.60],
        [0.30, 0.88, 0.88, 0.60],
        [1.00, 0.55, 0.22, 0.60],
        [0.55, 0.78, 0.25, 0.60],
        [0.85, 0.45, 0.55, 0.60],
        [0.50, 0.50, 0.80, 0.60],
    ];
    P[l as usize % P.len()]
}

fn outline_color(fill: [f32; 4]) -> [f32; 4] {
    [
        (fill[0] * 1.5).min(1.0),
        (fill[1] * 1.5).min(1.0),
        (fill[2] * 1.5).min(1.0),
        1.0,
    ]
}

pub fn triangulate(polys: &[Poly]) -> Vec<Vertex> {
    let mut verts = Vec::new();
    for p in polys {
        let c = layer_color(p.layer);
        let coords: Vec<f64> = p.pts.iter().flat_map(|v| [v[0] as f64, v[1] as f64]).collect();
        let idx = earcutr::earcut(&coords, &[], 2).unwrap_or_default();
        for i in idx {
            if i < p.pts.len() {
                verts.push(Vertex { pos: [p.pts[i][0] as f32, p.pts[i][1] as f32], color: c });
            }
        }
    }
    verts
}

pub fn outline_vertices(polys: &[Poly]) -> Vec<Vertex> {
    let mut verts = Vec::new();
    for p in polys {
        let c = outline_color(layer_color(p.layer));
        let n = p.pts.len();
        for i in 0..n {
            let j = (i + 1) % n;
            verts.push(Vertex { pos: [p.pts[i][0] as f32, p.pts[i][1] as f32], color: c });
            verts.push(Vertex { pos: [p.pts[j][0] as f32, p.pts[j][1] as f32], color: c });
        }
    }
    verts
}

// ═══════════════════════════════════════════════════════════════════════
//  Stroke font — minimal line-segment glyphs for annotation labels
// ═══════════════════════════════════════════════════════════════════════

// Each glyph is defined on a 5-wide, 7-tall grid as pairs of (x,y) line endpoints.
// A (255,255) pair marks a pen-up (move without drawing).
const PEN_UP: (u8, u8) = (255, 255);

fn glyph(ch: char) -> &'static [(u8, u8)] {
    match ch {
        'A' | 'a' => &[(0,0),(0,5),(0,5),(2,7),(2,7),(4,7),(4,7),(4,0), PEN_UP,PEN_UP, (0,3),(4,3)],
        'B' | 'b' => &[(0,0),(0,7),(0,7),(3,7),(3,7),(4,6),(4,6),(4,5),(4,5),(3,4),(3,4),(0,4), PEN_UP,PEN_UP, (3,4),(4,3),(4,3),(4,1),(4,1),(3,0),(3,0),(0,0)],
        'C' | 'c' => &[(4,1),(3,0),(3,0),(1,0),(1,0),(0,1),(0,1),(0,6),(0,6),(1,7),(1,7),(3,7),(3,7),(4,6)],
        'D' | 'd' => &[(0,0),(0,7),(0,7),(3,7),(3,7),(4,6),(4,6),(4,1),(4,1),(3,0),(3,0),(0,0)],
        'E' | 'e' => &[(4,0),(0,0),(0,0),(0,7),(0,7),(4,7), PEN_UP,PEN_UP, (0,4),(3,4)],
        'F' | 'f' => &[(0,0),(0,7),(0,7),(4,7), PEN_UP,PEN_UP, (0,4),(3,4)],
        'G' | 'g' => &[(4,6),(3,7),(3,7),(1,7),(1,7),(0,6),(0,6),(0,1),(0,1),(1,0),(1,0),(3,0),(3,0),(4,1),(4,1),(4,3),(4,3),(2,3)],
        'H' | 'h' => &[(0,0),(0,7), PEN_UP,PEN_UP, (4,0),(4,7), PEN_UP,PEN_UP, (0,4),(4,4)],
        'I' | 'i' => &[(1,0),(3,0), PEN_UP,PEN_UP, (2,0),(2,7), PEN_UP,PEN_UP, (1,7),(3,7)],
        'J' | 'j' => &[(1,1),(2,0),(2,0),(3,0),(3,0),(4,1),(4,1),(4,7)],
        'K' | 'k' => &[(0,0),(0,7), PEN_UP,PEN_UP, (4,7),(0,3), PEN_UP,PEN_UP, (1,4),(4,0)],
        'L' | 'l' => &[(0,7),(0,0),(0,0),(4,0)],
        'M' | 'm' => &[(0,0),(0,7),(0,7),(2,4),(2,4),(4,7),(4,7),(4,0)],
        'N' | 'n' => &[(0,0),(0,7),(0,7),(4,0),(4,0),(4,7)],
        'O' | 'o' => &[(1,0),(0,1),(0,1),(0,6),(0,6),(1,7),(1,7),(3,7),(3,7),(4,6),(4,6),(4,1),(4,1),(3,0),(3,0),(1,0)],
        'P' | 'p' => &[(0,0),(0,7),(0,7),(3,7),(3,7),(4,6),(4,6),(4,4),(4,4),(3,3),(3,3),(0,3)],
        'Q' | 'q' => &[(1,0),(0,1),(0,1),(0,6),(0,6),(1,7),(1,7),(3,7),(3,7),(4,6),(4,6),(4,1),(4,1),(3,0),(3,0),(1,0), PEN_UP,PEN_UP, (3,2),(5,0)],
        'R' | 'r' => &[(0,0),(0,7),(0,7),(3,7),(3,7),(4,6),(4,6),(4,5),(4,5),(3,4),(3,4),(0,4), PEN_UP,PEN_UP, (2,4),(4,0)],
        'S' | 's' => &[(4,6),(3,7),(3,7),(1,7),(1,7),(0,6),(0,6),(0,5),(0,5),(1,4),(1,4),(3,3),(3,3),(4,2),(4,2),(4,1),(4,1),(3,0),(3,0),(1,0),(1,0),(0,1)],
        'T' | 't' => &[(0,7),(4,7), PEN_UP,PEN_UP, (2,0),(2,7)],
        'U' | 'u' => &[(0,7),(0,1),(0,1),(1,0),(1,0),(3,0),(3,0),(4,1),(4,1),(4,7)],
        'V' | 'v' => &[(0,7),(2,0),(2,0),(4,7)],
        'W' | 'w' => &[(0,7),(1,0),(1,0),(2,3),(2,3),(3,0),(3,0),(4,7)],
        'X' | 'x' => &[(0,0),(4,7), PEN_UP,PEN_UP, (0,7),(4,0)],
        'Y' | 'y' => &[(0,7),(2,4),(2,4),(4,7), PEN_UP,PEN_UP, (2,4),(2,0)],
        'Z' | 'z' => &[(0,7),(4,7),(4,7),(0,0),(0,0),(4,0)],
        '0' => &[(1,0),(0,1),(0,1),(0,6),(0,6),(1,7),(1,7),(3,7),(3,7),(4,6),(4,6),(4,1),(4,1),(3,0),(3,0),(1,0), PEN_UP,PEN_UP, (1,1),(3,6)],
        '1' => &[(1,6),(2,7),(2,7),(2,0), PEN_UP,PEN_UP, (1,0),(3,0)],
        '2' => &[(0,6),(1,7),(1,7),(3,7),(3,7),(4,6),(4,6),(4,5),(4,5),(0,1),(0,1),(0,0),(0,0),(4,0)],
        '3' => &[(0,6),(1,7),(1,7),(3,7),(3,7),(4,6),(4,6),(4,5),(4,5),(3,4),(3,4),(2,4), PEN_UP,PEN_UP, (3,4),(4,3),(4,3),(4,1),(4,1),(3,0),(3,0),(1,0),(1,0),(0,1)],
        '4' => &[(0,7),(0,3),(0,3),(4,3), PEN_UP,PEN_UP, (3,7),(3,0)],
        '5' => &[(4,7),(0,7),(0,7),(0,4),(0,4),(3,4),(3,4),(4,3),(4,3),(4,1),(4,1),(3,0),(3,0),(1,0),(1,0),(0,1)],
        '6' => &[(3,7),(1,7),(1,7),(0,6),(0,6),(0,1),(0,1),(1,0),(1,0),(3,0),(3,0),(4,1),(4,1),(4,3),(4,3),(3,4),(3,4),(0,4)],
        '7' => &[(0,7),(4,7),(4,7),(1,0)],
        '8' => &[(1,4),(0,5),(0,5),(0,6),(0,6),(1,7),(1,7),(3,7),(3,7),(4,6),(4,6),(4,5),(4,5),(3,4),(3,4),(1,4), PEN_UP,PEN_UP, (1,4),(0,3),(0,3),(0,1),(0,1),(1,0),(1,0),(3,0),(3,0),(4,1),(4,1),(4,3),(4,3),(3,4)],
        '9' => &[(4,3),(1,3),(1,3),(0,4),(0,4),(0,6),(0,6),(1,7),(1,7),(3,7),(3,7),(4,6),(4,6),(4,1),(4,1),(3,0)],
        '_' => &[(0,0),(4,0)],
        '-' => &[(1,3),(3,3)],
        '(' => &[(3,0),(1,1),(1,1),(1,6),(1,6),(3,7)],
        ')' => &[(1,0),(3,1),(3,1),(3,6),(3,6),(1,7)],
        ',' => &[(2,0),(1,255)], // short descender
        '.' => &[(2,0),(2,1),(2,1),(2,0)],
        ' ' => &[],
        _ => &[(0,0),(4,7), PEN_UP,PEN_UP, (0,7),(4,0)], // fallback: X
    }
}

/// Emit a thick line as two triangles (a quad). Appends 6 vertices.
fn stroke_quad(verts: &mut Vec<Vertex>, ax: f32, ay: f32, bx: f32, by: f32, half_w: f32, color: [f32; 4]) {
    let dx = bx - ax;
    let dy = by - ay;
    let len = (dx * dx + dy * dy).sqrt().max(1e-6);
    let nx = -dy / len * half_w;
    let ny = dx / len * half_w;
    verts.push(Vertex { pos: [ax - nx, ay - ny], color });
    verts.push(Vertex { pos: [bx - nx, by - ny], color });
    verts.push(Vertex { pos: [bx + nx, by + ny], color });
    verts.push(Vertex { pos: [ax - nx, ay - ny], color });
    verts.push(Vertex { pos: [bx + nx, by + ny], color });
    verts.push(Vertex { pos: [ax + nx, ay + ny], color });
}

/// Render text labels as filled quads (triangle list). Thick strokes visible at any zoom.
pub fn text_vertices(texts: &[TextEntry], polys: &[Poly]) -> Vec<Vertex> {
    if texts.is_empty() { return Vec::new(); }
    let scale = if polys.is_empty() {
        100.0_f32
    } else {
        let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for p in polys {
            for &[x, y] in &p.pts {
                x0 = x0.min(x); y0 = y0.min(y);
                x1 = x1.max(x); y1 = y1.max(y);
            }
        }
        let span = ((x1 - x0) as f32).max((y1 - y0) as f32).max(1.0);
        span * 0.025 / 7.0
    };
    let color: [f32; 4] = [1.0, 1.0, 1.0, 0.95];
    let char_w = 6.0 * scale;
    let lw = scale * 0.22; // stroke half-width — thick enough to read
    let mut verts = Vec::new();

    for t in texts {
        let ox = t.x as f32;
        let oy = t.y as f32;
        for (ci, ch) in t.text.chars().enumerate() {
            let g = glyph(ch);
            let cx = ox + ci as f32 * char_w;
            let mut i = 0;
            while i + 1 < g.len() {
                let (gx0, gy0) = g[i];
                let (gx1, gy1) = g[i + 1];
                i += 2;
                if (gx0, gy0) == PEN_UP || (gx1, gy1) == PEN_UP { continue; }
                stroke_quad(&mut verts,
                    cx + gx0 as f32 * scale, oy + gy0 as f32 * scale,
                    cx + gx1 as f32 * scale, oy + gy1 as f32 * scale,
                    lw, color);
            }
        }
    }
    verts
}

/// Build legend overlay vertices in NDC space (top-right corner).
/// Returns triangle-list vertices for colored squares + line-list vertices for text.
/// Render DRC violations as red X markers in world space (triangle list).
pub fn drc_marker_vertices(markers: &[DrcMarker], polys: &[Poly]) -> Vec<Vertex> {
    if markers.is_empty() { return Vec::new(); }
    let span = if polys.is_empty() { 1000.0 } else {
        let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for p in polys { for &[x, y] in &p.pts { x0 = x0.min(x); y0 = y0.min(y); x1 = x1.max(x); y1 = y1.max(y); } }
        ((x1 - x0) as f32).max((y1 - y0) as f32).max(1.0)
    };
    let arm = span * 0.015; // X arm length
    let lw = arm * 0.25;    // stroke width
    let color: [f32; 4] = [1.0, 0.15, 0.15, 0.95];
    let mut verts = Vec::new();
    for m in markers {
        let cx = m.x as f32;
        let cy = m.y as f32;
        stroke_quad(&mut verts, cx - arm, cy - arm, cx + arm, cy + arm, lw, color);
        stroke_quad(&mut verts, cx - arm, cy + arm, cx + arm, cy - arm, lw, color);
    }
    verts
}

/// Build signoff summary overlay in NDC (top-left corner). Returns triangle-list vertices.
pub fn signoff_overlay(sd: &SignoffData) -> Vec<Vertex> {
    if sd.lvs_reason.is_empty() && sd.drc.is_empty() && sd.pex_r_ohm == 0.0 { return Vec::new(); }

    let ch = 0.035_f32;
    let margin = 0.02_f32;
    let glyph_scale = ch * 0.11;
    let char_w = glyph_scale * 6.0;
    let lw = glyph_scale * 0.3;

    let mut lines: Vec<(String, [f32; 4])> = Vec::new();
    // LVS
    let lvs_color = if sd.lvs_matched { [0.3, 0.9, 0.4, 1.0] } else { [1.0, 0.3, 0.3, 1.0] };
    lines.push((format!("LVS {} {}N {}P", if sd.lvs_matched { "OK" } else { "FAIL" }, sd.lvs_nmos, sd.lvs_pmos), lvs_color));
    // DRC
    let drc_color = if sd.drc.is_empty() { [0.3, 0.9, 0.4, 1.0] } else { [1.0, 0.3, 0.3, 1.0] };
    lines.push((format!("DRC {} VIOLATIONS", sd.drc.len()), drc_color));
    // PEX
    let pex_color: [f32; 4] = [0.7, 0.8, 1.0, 1.0];
    lines.push((format!("PEX R {:.1} C {:.1}FF", sd.pex_r_ohm, sd.pex_c_af / 1000.0), pex_color));

    let n = lines.len();
    let total_h = n as f32 * ch + margin;
    let x0 = -1.0 + margin;
    let y_top = 1.0 - margin;

    let mut verts = Vec::new();
    let bg: [f32; 4] = [0.08, 0.08, 0.10, 0.8];

    // background
    let bx1 = x0 + 0.42;
    let by0 = y_top - total_h;
    for &[px, py] in &[[x0 - margin / 2.0, by0], [bx1, by0], [bx1, y_top + margin / 2.0],
                        [x0 - margin / 2.0, by0], [bx1, y_top + margin / 2.0], [x0 - margin / 2.0, y_top + margin / 2.0]] {
        verts.push(Vertex { pos: [px, py], color: bg });
    }

    for (i, (text, color)) in lines.iter().enumerate() {
        let y = y_top - (i as f32 + 0.5) * ch;
        for (ci, c) in text.chars().enumerate() {
            let g = glyph(c);
            let cx = x0 + ci as f32 * char_w;
            let mut gi = 0;
            while gi + 1 < g.len() {
                let (gx0, gy0) = g[gi];
                let (gx1, gy1) = g[gi + 1];
                gi += 2;
                if (gx0, gy0) == PEN_UP || (gx1, gy1) == PEN_UP { continue; }
                stroke_quad(&mut verts,
                    cx + gx0 as f32 * glyph_scale, y - ch * 0.35 + gy0 as f32 * glyph_scale,
                    cx + gx1 as f32 * glyph_scale, y - ch * 0.35 + gy1 as f32 * glyph_scale,
                    lw, *color);
            }
        }
    }
    verts
}

pub fn build_legend(polys: &[Poly], layer_names: &LayerMap, _aspect: f32) -> Vec<Vertex> {
    let mut layers: Vec<u16> = polys.iter().map(|p| p.layer).collect();
    layers.sort_unstable();
    layers.dedup();
    if layers.is_empty() { return Vec::new(); }

    let n = layers.len();
    let ch = 0.04_f32; // row height in NDC
    let sw = 0.03_f32; // color swatch width
    let margin = 0.02_f32;
    let total_h = n as f32 * ch + margin;
    // top-right anchor
    let x0 = 1.0 - margin - 0.3; // left edge of legend box
    let y_top = 1.0 - margin;

    let mut verts = Vec::new();
    let bg_color: [f32; 4] = [0.08, 0.08, 0.10, 0.75];

    // background quad (two triangles)
    let bx0 = x0 - margin;
    let bx1 = 1.0 - margin / 2.0;
    let by0 = y_top - total_h;
    let by1 = y_top + margin / 2.0;
    for &[px, py] in &[[bx0,by0],[bx1,by0],[bx1,by1],[bx0,by0],[bx1,by1],[bx0,by1]] {
        verts.push(Vertex { pos: [px, py], color: bg_color });
    }

    let glyph_scale = ch * 0.11;
    let char_w = glyph_scale * 6.0;

    for (i, &layer) in layers.iter().enumerate() {
        let y = y_top - (i as f32 + 0.5) * ch;
        let c = layer_color(layer);
        let sc = [c[0], c[1], c[2], 1.0]; // full alpha for swatch

        // color swatch (two triangles)
        let sx0 = x0;
        let sx1 = x0 + sw;
        let sy0 = y - ch * 0.35;
        let sy1 = y + ch * 0.35;
        for &[px, py] in &[[sx0,sy0],[sx1,sy0],[sx1,sy1],[sx0,sy0],[sx1,sy1],[sx0,sy1]] {
            verts.push(Vertex { pos: [px, py], color: sc });
        }

        let name = layer_names.iter()
            .find(|(&(l, _d), _)| l == layer as i32)
            .map(|(_, n)| n.as_str())
            .unwrap_or("");
        let label = if name.is_empty() { format!("L{layer}") } else { name.to_string() };
        let tx = x0 + sw + margin;
        let text_color: [f32; 4] = [0.9, 0.9, 0.9, 1.0];
        let lw = glyph_scale * 0.3;

        for (ci, ch_c) in label.chars().enumerate() {
            let g = glyph(ch_c);
            let cx = tx + ci as f32 * char_w;
            let mut gi = 0;
            while gi + 1 < g.len() {
                let (gx0, gy0) = g[gi];
                let (gx1, gy1) = g[gi + 1];
                gi += 2;
                if (gx0, gy0) == PEN_UP || (gx1, gy1) == PEN_UP { continue; }
                stroke_quad(&mut verts,
                    cx + gx0 as f32 * glyph_scale, y - ch * 0.35 + gy0 as f32 * glyph_scale,
                    cx + gx1 as f32 * glyph_scale, y - ch * 0.35 + gy1 as f32 * glyph_scale,
                    lw, text_color);
            }
        }
    }
    verts
}

pub fn fit_view(polys: &[Poly]) -> CamUni {
    if polys.is_empty() {
        return CamUni { offset: [0.0; 2], scale: 1.0, aspect: 1.0 };
    }
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for p in polys {
        for &[x, y] in &p.pts {
            x0 = x0.min(x); y0 = y0.min(y);
            x1 = x1.max(x); y1 = y1.max(y);
        }
    }
    let cx = (x0 + x1) as f32 / 2.0;
    let cy = (y0 + y1) as f32 / 2.0;
    let span = ((x1 - x0) as f32).max((y1 - y0) as f32).max(1.0);
    CamUni { offset: [-cx, -cy], scale: 1.8 / span, aspect: 1.0 }
}

// ═══════════════════════════════════════════════════════════════════════
//  SVG export (headless, no GPU)
// ═══════════════════════════════════════════════════════════════════════

pub fn export_svg(gds_bytes: &[u8], layer_names: &LayerMap) -> String {
    let (polys, _texts) = parse_gds(gds_bytes);
    if polys.is_empty() {
        return String::from(r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#);
    }
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for p in &polys {
        for &[x, y] in &p.pts {
            x0 = x0.min(x); y0 = y0.min(y);
            x1 = x1.max(x); y1 = y1.max(y);
        }
    }
    let pad = ((x1 - x0).max(y1 - y0)) / 40;
    let (vx, vy, vw, vh) = (x0 - pad, y0 - pad, x1 - x0 + 2 * pad, y1 - y0 + 2 * pad);

    let mut layers: Vec<u16> = polys.iter().map(|p| p.layer).collect();
    layers.sort_unstable();
    layers.dedup();

    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{vx} {vy} {vw} {vh}" width="800" height="800" style="background:#14141a">"#
    );
    // flip y: GDS has y-up, SVG has y-down
    svg.push_str(&format!(r#"<g transform="translate(0,{}) scale(1,-1)">"#, vy * 2 + vh));

    for &layer in &layers {
        let c = layer_color(layer);
        let r = (c[0] * 255.0) as u8;
        let g = (c[1] * 255.0) as u8;
        let b = (c[2] * 255.0) as u8;
        let a = c[3];
        let name = layer_names.iter()
            .find(|(&(l, _), _)| l == layer as i32)
            .map(|(_, n)| n.as_str())
            .unwrap_or("");
        let group_id = if name.is_empty() { format!("L{layer}") } else { name.to_string() };
        svg.push_str(&format!(r#"<g id="{group_id}" fill="rgba({r},{g},{b},{a})" stroke="rgba({},{},{},1)" stroke-width="{}">"#,
            ((c[0] * 1.5).min(1.0) * 255.0) as u8,
            ((c[1] * 1.5).min(1.0) * 255.0) as u8,
            ((c[2] * 1.5).min(1.0) * 255.0) as u8,
            (vw.max(vh) as f32 * 0.001) as i32,
        ));
        for p in polys.iter().filter(|p| p.layer == layer) {
            svg.push_str(r#"<polygon points=""#);
            for (i, &[x, y]) in p.pts.iter().enumerate() {
                if i > 0 { svg.push(' '); }
                svg.push_str(&format!("{x},{y}"));
            }
            svg.push_str(r#""/>"#);
        }
        svg.push_str("</g>");
    }
    svg.push_str("</g></svg>");
    svg
}

// ═══════════════════════════════════════════════════════════════════════
//  WGSL shader
// ═══════════════════════════════════════════════════════════════════════

const SHADER: &str = "
struct Camera { offset: vec2<f32>, scale: f32, aspect: f32 }
@group(0) @binding(0) var<uniform> cam: Camera;

struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) color: vec4<f32> }

@vertex fn vs(@location(0) pos: vec2<f32>, @location(1) color: vec4<f32>) -> VsOut {
    var out: VsOut;
    let p = (pos + cam.offset) * cam.scale;
    out.pos = vec4<f32>(p.x / cam.aspect, p.y, 0.0, 1.0);
    out.color = color;
    return out;
}

@fragment fn fs(in: VsOut) -> @location(0) vec4<f32> { return in.color; }
";

const OVERLAY_SHADER: &str = "
struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) color: vec4<f32> }

@vertex fn vs(@location(0) pos: vec2<f32>, @location(1) color: vec4<f32>) -> VsOut {
    var out: VsOut;
    out.pos = vec4<f32>(pos.x, pos.y, 0.0, 1.0);
    out.color = color;
    return out;
}

@fragment fn fs(in: VsOut) -> @location(0) vec4<f32> { return in.color; }
";

// ═══════════════════════════════════════════════════════════════════════
//  GPU state
// ═══════════════════════════════════════════════════════════════════════

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    fill_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    overlay_fill_pipeline: wgpu::RenderPipeline,
    #[allow(dead_code)]
    overlay_line_pipeline: wgpu::RenderPipeline,
    fill_buf: wgpu::Buffer,
    line_buf: wgpu::Buffer,
    overlay_buf: wgpu::Buffer,
    cam_buf: wgpu::Buffer,
    cam_bg: wgpu::BindGroup,
    n_fill: u32,
    n_line: u32,
    n_overlay: u32,
}

fn make_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
    topology: wgpu::PrimitiveTopology,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs"),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 0, shader_location: 0 },
                    wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 8, shader_location: 1 },
                ],
            }],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState { topology, ..Default::default() },
        depth_stencil: None,
        multisample: Default::default(),
        multiview: None,
        cache: None,
    })
}

fn dummy_buf(device: &wgpu::Device) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: None, size: 64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

impl Gpu {
    fn new(win: std::sync::Arc<Window>) -> Self {
        let sz = win.inner_size();
        let inst = wgpu::Instance::default();
        let surface = inst.create_surface(win).unwrap();
        let adapter = pollster::block_on(inst.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface), ..Default::default()
        })).expect("no GPU adapter found");
        let (device, queue) = pollster::block_on(
            adapter.request_device(&wgpu::DeviceDescriptor::default(), None)
        ).unwrap();

        let caps = surface.get_capabilities(&adapter);
        let fmt = caps.formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: fmt, width: sz.width.max(1), height: sz.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![], desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None, source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let cam_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: None, size: std::mem::size_of::<CamUni>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0, visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false, min_binding_size: None,
                }, count: None,
            }],
        });
        let cam_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None, layout: &bgl,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: cam_buf.as_entire_binding() }],
        });
        let pll = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None, bind_group_layouts: &[&bgl], push_constant_ranges: &[],
        });

        let fill_pipeline = make_pipeline(&device, &shader, &pll, fmt, wgpu::PrimitiveTopology::TriangleList);
        let line_pipeline = make_pipeline(&device, &shader, &pll, fmt, wgpu::PrimitiveTopology::LineList);

        let overlay_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None, source: wgpu::ShaderSource::Wgsl(OVERLAY_SHADER.into()),
        });
        let overlay_pll = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None, bind_group_layouts: &[], push_constant_ranges: &[],
        });
        let overlay_fill_pipeline = make_pipeline(&device, &overlay_shader, &overlay_pll, fmt, wgpu::PrimitiveTopology::TriangleList);
        let overlay_line_pipeline = make_pipeline(&device, &overlay_shader, &overlay_pll, fmt, wgpu::PrimitiveTopology::LineList);

        Self {
            fill_buf: dummy_buf(&device), line_buf: dummy_buf(&device),
            overlay_buf: dummy_buf(&device),
            device, queue, surface, config,
            fill_pipeline, line_pipeline,
            overlay_fill_pipeline, overlay_line_pipeline,
            cam_buf, cam_bg, n_fill: 0, n_line: 0, n_overlay: 0,
        }
    }

    fn resize(&mut self, w: u32, h: u32) {
        if w > 0 && h > 0 {
            self.config.width = w;
            self.config.height = h;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn upload(&mut self, fill: &[Vertex], outline: &[Vertex]) {
        self.n_fill = fill.len() as u32;
        self.n_line = outline.len() as u32;
        if !fill.is_empty() {
            self.fill_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: None, size: (fill.len() * std::mem::size_of::<Vertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.queue.write_buffer(&self.fill_buf, 0, bytemuck::cast_slice(fill));
        }
        if !outline.is_empty() {
            self.line_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: None, size: (outline.len() * std::mem::size_of::<Vertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.queue.write_buffer(&self.line_buf, 0, bytemuck::cast_slice(outline));
        }
    }

    fn upload_overlay(&mut self, verts: &[Vertex]) {
        self.n_overlay = verts.len() as u32;
        if !verts.is_empty() {
            self.overlay_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: None, size: (verts.len() * std::mem::size_of::<Vertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.queue.write_buffer(&self.overlay_buf, 0, bytemuck::cast_slice(verts));
        }
    }

    fn draw(&mut self, cam: &CamUni) {
        self.queue.write_buffer(&self.cam_buf, 0, bytemuck::bytes_of(cam));
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(_) => { self.surface.configure(&self.device, &self.config); return; }
        };
        let view = frame.texture.create_view(&Default::default());
        let mut enc = self.device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view, resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.08, g: 0.08, b: 0.10, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            if self.n_fill > 0 {
                rp.set_pipeline(&self.fill_pipeline);
                rp.set_bind_group(0, &self.cam_bg, &[]);
                rp.set_vertex_buffer(0, self.fill_buf.slice(..));
                rp.draw(0..self.n_fill, 0..1);
            }
            if self.n_line > 0 {
                rp.set_pipeline(&self.line_pipeline);
                rp.set_bind_group(0, &self.cam_bg, &[]);
                rp.set_vertex_buffer(0, self.line_buf.slice(..));
                rp.draw(0..self.n_line, 0..1);
            }
            // overlay: screen-space legend (no camera transform)
            if self.n_overlay > 0 {
                rp.set_pipeline(&self.overlay_fill_pipeline);
                rp.set_vertex_buffer(0, self.overlay_buf.slice(..));
                rp.draw(0..self.n_overlay, 0..1);
            }
        }
        self.queue.submit([enc.finish()]);
        frame.present();
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Unified App (file-watcher mode OR probe-channel mode)
// ═══════════════════════════════════════════════════════════════════════

enum Source {
    File {
        path: PathBuf,
        mtime: Option<SystemTime>,
        last_poll: Instant,
    },
    Channel {
        rx: mpsc::Receiver<ProbeMsg>,
    },
    Dump {
        path: PathBuf,
        mtime: Option<SystemTime>,
        last_poll: Instant,
        frames: Vec<DumpFrame>,
        current: usize,
        playing: bool,
        last_advance: Instant,
    },
}

pub(crate) struct ProbeMsg {
    pub fill: Vec<Vertex>,
    pub outline: Vec<Vertex>,
    pub title: Option<String>,
}

struct App {
    win: Option<std::sync::Arc<Window>>,
    gpu: Option<Gpu>,
    cam: CamUni,
    cursor: [f64; 2],
    drag: bool,
    first_fit: bool,
    frame_count: u32,
    title: String,
    source: Source,
    layer_names: LayerMap,
    signoff: SignoffData,
}

impl App {
    fn for_file(path: PathBuf, layer_names: LayerMap) -> Self {
        let title = format!("GDS \u{2014} {}", path.display());
        Self {
            win: None, gpu: None,
            cam: CamUni { offset: [0.0; 2], scale: 1.0, aspect: 1.0 },
            cursor: [0.0; 2], drag: false, first_fit: true, frame_count: 0,
            title, layer_names, signoff: SignoffData::default(),
            source: Source::File { path, mtime: None, last_poll: Instant::now() },
        }
    }

    fn for_dump(path: PathBuf, layer_names: LayerMap) -> Self {
        let title = format!("Dump \u{2014} {}", path.display());
        Self {
            win: None, gpu: None,
            cam: CamUni { offset: [0.0; 2], scale: 1.0, aspect: 1.0 },
            cursor: [0.0; 2], drag: false, first_fit: true, frame_count: 0,
            title, layer_names, signoff: SignoffData::default(),
            source: Source::Dump { path, mtime: None, last_poll: Instant::now(), frames: Vec::new(), current: 0, playing: true, last_advance: Instant::now() },
        }
    }

    fn for_probe(title: String, rx: mpsc::Receiver<ProbeMsg>) -> Self {
        Self {
            win: None, gpu: None,
            cam: CamUni { offset: [0.0; 2], scale: 1.0, aspect: 1.0 },
            cursor: [0.0; 2], drag: false, first_fit: true, frame_count: 0,
            title, layer_names: LayerMap::new(), signoff: SignoffData::default(),
            source: Source::Channel { rx },
        }
    }

    fn apply_polys(&mut self, polys: &[Poly], texts: &[TextEntry]) {
        let mut sorted: Vec<&Poly> = polys.iter().collect();
        sorted.sort_by_key(|p| p.layer);
        let sorted_polys: Vec<Poly> = sorted.into_iter()
            .map(|p| Poly { layer: p.layer, pts: p.pts.clone() })
            .collect();

        if self.first_fit {
            let mut c = fit_view(&sorted_polys);
            c.aspect = self.cam.aspect;
            self.cam = c;
            self.first_fit = false;
        }

        let mut fill = triangulate(&sorted_polys);
        fill.extend(text_vertices(texts, &sorted_polys));
        fill.extend(drc_marker_vertices(&self.signoff.drc, &sorted_polys));
        let outline = outline_vertices(&sorted_polys);
        let mut overlay = build_legend(&sorted_polys, &self.layer_names, self.cam.aspect);
        overlay.extend(signoff_overlay(&self.signoff));
        if let Some(gpu) = &mut self.gpu {
            gpu.upload(&fill, &outline);
            gpu.upload_overlay(&overlay);
        }
    }

    fn apply_msg(&mut self, msg: ProbeMsg) {
        if let Some(title) = &msg.title {
            if let Some(win) = &self.win {
                win.set_title(&format!("{} \u{2014} {title}", self.title));
            }
        }
        if self.first_fit && !msg.fill.is_empty() {
            let mut cam = CamUni { offset: [0.0; 2], scale: 1.0, aspect: self.cam.aspect };
            let (mut x0, mut y0) = (f32::MAX, f32::MAX);
            let (mut x1, mut y1) = (f32::MIN, f32::MIN);
            for v in &msg.fill {
                x0 = x0.min(v.pos[0]); y0 = y0.min(v.pos[1]);
                x1 = x1.max(v.pos[0]); y1 = y1.max(v.pos[1]);
            }
            let cx = (x0 + x1) / 2.0;
            let cy = (y0 + y1) / 2.0;
            let span = (x1 - x0).max(y1 - y0).max(1.0);
            cam.offset = [-cx, -cy];
            cam.scale = 1.8 / span;
            self.cam = cam;
            self.first_fit = false;
        }
        if let Some(gpu) = &mut self.gpu {
            gpu.upload(&msg.fill, &msg.outline);
        }
    }

    fn load_dump(&mut self) {
        if let Source::Dump { ref path, ref mut mtime, ref mut last_poll, ref mut frames, ref mut current, ref mut playing, ref mut last_advance } = self.source {
            let text = match std::fs::read_to_string(path) {
                Ok(t) => t,
                Err(e) => { eprintln!("read: {e}"); return; }
            };
            *mtime = std::fs::metadata(path).and_then(|m| m.modified()).ok();
            let first_load = frames.is_empty();
            *frames = parse_dump(&text);
            if first_load {
                *current = 0;
                *playing = true;
                *last_advance = Instant::now();
                *last_poll = Instant::now();
            } else if *current >= frames.len() {
                *current = frames.len().saturating_sub(1);
            }
            eprintln!("{} frames loaded", frames.len());
        }
        self.show_frame();
    }

    fn show_frame(&mut self) {
        let info = if let Source::Dump { ref frames, current, .. } = self.source {
            frames.get(current).map(|f| (f.polys.clone(), f.texts.clone(), current + 1, frames.len(), f.label.clone()))
        } else { None };
        if let Some((polys, texts, idx, total, label)) = info {
            self.apply_polys(&polys, &texts);
            if let Some(win) = &self.win {
                win.set_title(&format!("{} \u{2014} [{idx}/{total}] {label}", self.title));
            }
        }
    }

    fn load_gds(&mut self) {
        if let Source::File { ref path, ref mut mtime, .. } = self.source {
            let data = match std::fs::read(path) {
                Ok(d) => d,
                Err(e) => { eprintln!("read: {e}"); return; }
            };
            *mtime = std::fs::metadata(path).and_then(|m| m.modified()).ok();
            // load signoff sidecar if present
            let signoff_path = path.with_extension("").with_file_name("signoff.json");
            self.signoff = std::fs::read_to_string(&signoff_path)
                .map(|j| parse_signoff(&j))
                .unwrap_or_default();
            if !self.signoff.drc.is_empty() || !self.signoff.lvs_reason.is_empty() {
                eprintln!("signoff: LVS {} | {} DRC violations | R {:.1}Ω C {:.1}fF",
                    if self.signoff.lvs_matched { "MATCH" } else { "MISMATCH" },
                    self.signoff.drc.len(), self.signoff.pex_r_ohm, self.signoff.pex_c_af / 1000.0);
            }
            let (polys, texts) = parse_gds(&data);
            let mut layers: Vec<u16> = polys.iter().map(|p| p.layer).collect();
            layers.sort_unstable(); layers.dedup();
            eprintln!("{} polygons, {} layers, {} labels", polys.len(), layers.len(), texts.len());
            self.apply_polys(&polys, &texts);
        }
    }

    fn cursor_ndc(&self) -> [f32; 2] {
        self.win.as_ref().map(|w| {
            let s = w.inner_size();
            [2.0 * self.cursor[0] as f32 / s.width as f32 - 1.0,
             -(2.0 * self.cursor[1] as f32 / s.height as f32 - 1.0)]
        }).unwrap_or([0.0; 2])
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.win.is_some() { return; }
        let w = std::sync::Arc::new(el.create_window(
            Window::default_attributes()
                .with_title(&self.title)
                .with_inner_size(LogicalSize::new(1280u32, 720u32)),
        ).unwrap());
        let s = w.inner_size();
        self.cam.aspect = s.width as f32 / s.height.max(1) as f32;
        self.gpu = Some(Gpu::new(w.clone()));
        self.win = Some(w);
        if matches!(self.source, Source::File { .. }) {
            self.load_gds();
        }
        if matches!(self.source, Source::Dump { .. }) {
            self.load_dump();
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, ev: WindowEvent) {
        match ev {
            WindowEvent::CloseRequested => el.exit(),

            WindowEvent::Resized(s) => {
                self.cam.aspect = s.width as f32 / s.height.max(1) as f32;
                if let Some(g) = &mut self.gpu { g.resize(s.width, s.height); }
            }

            WindowEvent::RedrawRequested => {
                if let Some(g) = &mut self.gpu {
                    g.draw(&self.cam);
                    self.frame_count = self.frame_count.saturating_add(1);
                    // ponytail: second RedrawRequested = compositor's frame callback,
                    // meaning the first frame was actually displayed on screen
                    if self.frame_count == 60 {
                        if let Source::Dump { ref mut last_advance, .. } = self.source {
                            *last_advance = Instant::now();
                        }
                    }
                }
            }

            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                self.drag = state == ElementState::Pressed;
            }

            WindowEvent::CursorMoved { position, .. } => {
                let (dx, dy) = (position.x - self.cursor[0], position.y - self.cursor[1]);
                self.cursor = [position.x, position.y];
                if self.drag {
                    if let Some(w) = &self.win {
                        let s = w.inner_size();
                        self.cam.offset[0] +=
                            (2.0 * dx as f32 / s.width as f32) * self.cam.aspect / self.cam.scale;
                        self.cam.offset[1] -=
                            (2.0 * dy as f32 / s.height as f32) / self.cam.scale;
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let d = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 100.0,
                };
                let factor = if d > 0.0 { 1.1 } else { 1.0 / 1.1 };
                let ndc = self.cursor_ndc();
                let inv_old = 1.0 / self.cam.scale;
                self.cam.scale *= factor;
                let diff = 1.0 / self.cam.scale - inv_old;
                self.cam.offset[0] += ndc[0] * self.cam.aspect * diff;
                self.cam.offset[1] += ndc[1] * diff;
            }

            WindowEvent::KeyboardInput { event: ev, .. } if ev.state == ElementState::Pressed => {
                let step = 0.1 / self.cam.scale;
                match ev.logical_key {
                    Key::Named(NamedKey::Escape) => el.exit(),
                    Key::Character(ref c) => match c.as_str() {
                        "r" => {
                            self.first_fit = true;
                            self.load_gds();
                            self.load_dump();
                        }
                        "=" | "+" => self.cam.scale *= 1.2,
                        "-" => self.cam.scale /= 1.2,
                        "[" => {
                            if let Source::Dump { ref mut current, ref mut playing, .. } = self.source {
                                *playing = false;
                                if *current > 0 { *current -= 1; }
                            }
                            self.first_fit = true;
                            self.show_frame();
                        }
                        "]" => {
                            if let Source::Dump { ref mut current, ref frames, ref mut playing, .. } = self.source {
                                *playing = false;
                                if *current + 1 < frames.len() { *current += 1; }
                            }
                            self.first_fit = true;
                            self.show_frame();
                        }
                        " " => {
                            if let Source::Dump { ref mut playing, ref mut last_advance, ref mut current, ref frames, .. } = self.source {
                                *playing = !*playing;
                                if *playing {
                                    *last_advance = Instant::now();
                                    if *current + 1 >= frames.len() { *current = 0; }
                                }
                            }
                        }
                        _ => {}
                    },
                    Key::Named(NamedKey::ArrowUp) => self.cam.offset[1] += step,
                    Key::Named(NamedKey::ArrowDown) => self.cam.offset[1] -= step,
                    Key::Named(NamedKey::ArrowLeft) => {
                        if let Source::Dump { ref mut current, ref mut playing, .. } = self.source {
                            *playing = false;
                            if *current > 0 { *current -= 1; }
                            self.first_fit = true;
                            self.show_frame();
                        } else {
                            self.cam.offset[0] -= step;
                        }
                    }
                    Key::Named(NamedKey::ArrowRight) => {
                        if let Source::Dump { ref mut current, ref frames, ref mut playing, .. } = self.source {
                            *playing = false;
                            if *current + 1 < frames.len() { *current += 1; }
                            self.first_fit = true;
                            self.show_frame();
                        } else {
                            self.cam.offset[0] += step;
                        }
                    }
                    _ => {}
                }
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _el: &ActiveEventLoop) {
        match &mut self.source {
            Source::File { ref path, ref mut mtime, ref mut last_poll } => {
                if last_poll.elapsed() >= Duration::from_millis(500) {
                    *last_poll = Instant::now();
                    let mt = std::fs::metadata(path).and_then(|m| m.modified()).ok();
                    if mt.is_some() && mt != *mtime {
                        self.load_gds();
                    }
                }
            }
            Source::Dump { ref path, ref mut mtime, ref mut last_poll, ref mut current, ref frames, ref mut playing, ref mut last_advance } => {
                if last_poll.elapsed() >= Duration::from_millis(500) {
                    *last_poll = Instant::now();
                    let mt = std::fs::metadata(path).and_then(|m| m.modified()).ok();
                    if mt.is_some() && mt != *mtime {
                        drop((current, frames, playing, last_advance));
                        self.load_dump();
                        return; // re-enter on next tick
                    }
                }
                if *playing && self.frame_count >= 60 && last_advance.elapsed() >= Duration::from_millis(75) {
                    *last_advance = Instant::now();
                    if *current + 1 < frames.len() {
                        *current += 1;
                        drop((current, frames, playing, last_advance));
                        self.first_fit = true;
                        self.show_frame();
                        return;
                    } else {
                        *playing = false;
                    }
                }
            }
            Source::Channel { rx } => {
                let mut latest = None;
                loop {
                    match rx.try_recv() {
                        Ok(msg) => latest = Some(msg),
                        Err(mpsc::TryRecvError::Empty) => break,
                        Err(mpsc::TryRecvError::Disconnected) => break,
                    }
                }
                if let Some(msg) = latest {
                    self.apply_msg(msg);
                }
            }
        }
        if let Some(w) = &self.win {
            w.request_redraw();
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Public: standalone viewer
// ═══════════════════════════════════════════════════════════════════════

pub fn run_viewer(path: PathBuf, layer_names: LayerMap) {
    let el = EventLoop::new().unwrap_or_else(|e| {
        eprintln!("cannot create window (no display server?): {e}");
        std::process::exit(1);
    });
    let is_dump = path.extension().map_or(false, |e| e == "txt" || e == "dump");
    let mut app = if is_dump { App::for_dump(path, layer_names) } else { App::for_file(path, layer_names) };
    el.run_app(&mut app).unwrap();
}

// ═══════════════════════════════════════════════════════════════════════
//  Event loop helpers
// ═══════════════════════════════════════════════════════════════════════

/// Build an event loop that works on any thread (needed for the probe's spawned thread).
#[cfg(target_os = "linux")]
fn make_event_loop() -> Result<EventLoop<()>, winit::error::EventLoopError> {
    let mut builder = EventLoop::builder();
    // Set any_thread on both backends — only the active one takes effect.
    winit::platform::x11::EventLoopBuilderExtX11::with_any_thread(&mut builder, true);
    winit::platform::wayland::EventLoopBuilderExtWayland::with_any_thread(&mut builder, true);
    builder.build()
}

#[cfg(not(target_os = "linux"))]
fn make_event_loop() -> Result<EventLoop<()>, winit::error::EventLoopError> {
    EventLoop::new()
}

// ═══════════════════════════════════════════════════════════════════════
//  Public: in-process probe
// ═══════════════════════════════════════════════════════════════════════

/// Live probe window — spawns a GPU window on a background thread.
/// Accepts geometry snapshots from the P&R iteration loop.
/// Stays open after the sender is dropped so you can inspect the final state.
pub struct Probe {
    tx: Option<mpsc::Sender<ProbeMsg>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Probe {
    pub fn open(title: &str) -> Self {
        let (tx, rx) = mpsc::channel();
        let title = title.to_string();
        let handle = std::thread::spawn(move || {
            let el = match make_event_loop() {
                Ok(el) => el,
                Err(e) => { eprintln!("probe: no display: {e}"); return; }
            };
            el.run_app(&mut App::for_probe(title, rx)).unwrap();
        });
        Probe { tx: Some(tx), handle: Some(handle) }
    }

    /// Block until the user closes the probe window.
    /// Drops the sender (no more updates), then joins the window thread.
    pub fn wait(&mut self) {
        self.tx.take(); // drop sender — window stays open, no more updates
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }

    /// Send pre-built polygon list.
    pub fn send(&self, polys: &[Poly], title: Option<&str>) {
        let mut sorted: Vec<&Poly> = polys.iter().collect();
        sorted.sort_by_key(|p| p.layer);
        let sorted_owned: Vec<Poly> = sorted.into_iter()
            .map(|p| Poly { layer: p.layer, pts: p.pts.clone() })
            .collect();
        let fill = triangulate(&sorted_owned);
        let outline = outline_vertices(&sorted_owned);
        if let Some(ref tx) = self.tx {
            let _ = tx.send(ProbeMsg { fill, outline, title: title.map(String::from) });
        }
    }

    /// Send raw pre-triangulated vertices (skip retriangulation).
    pub fn send_raw(&self, fill: Vec<Vertex>, outline: Vec<Vertex>, title: Option<&str>) {
        if let Some(ref tx) = self.tx {
            let _ = tx.send(ProbeMsg { fill, outline, title: title.map(String::from) });
        }
    }

    /// Send a GeometryStore snapshot directly (reads SoA arrays).
    #[cfg(feature = "probe")]
    pub fn send_store(&self, store: &gdsverify::GeometryStore, title: Option<&str>) {
        self.send(&store_to_polys(store), title);
    }
}
