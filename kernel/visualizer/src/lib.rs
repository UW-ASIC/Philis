//! GDS viewer: a wgpu window over a GDS file (reloads on change), an
//! in-process [`Probe`] window fed polygons over a channel, and headless
//! [`export_svg`].

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

/// One GPU vertex: world (or, for the overlay, NDC) position and RGBA.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 2],
    color: [f32; 4],
}

/// The camera uniform: NDC = `(world + offset) · scale`, x divided by
/// `aspect` (width / height).
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct CamUni {
    /// World translation applied before scaling, database units.
    offset: [f32; 2],
    /// NDC per database unit.
    scale: f32,
    /// Window width / height.
    aspect: f32,
}

/// One flattened polygon on a GDS `(layer, datatype)`.
#[derive(Clone)]
pub struct Poly {
    /// GDS layer number.
    pub layer: u16,
    /// GDS datatype.
    pub datatype: u16,
    /// Vertices in database units, open (the closing vertex is not repeated).
    pub pts: Vec<[i32; 2]>,
}

/// One flattened GDS TEXT label.
#[derive(Clone)]
pub struct TextEntry {
    /// Anchor x, database units.
    pub x: i32,
    /// Anchor y, database units.
    pub y: i32,
    /// The label string.
    pub text: String,
}

/// GDS (layer, datatype) → layer name.
pub type LayerMap = HashMap<(i32, i32), String>;

/// Layer names from `(name, (gds layer, datatype))` pairs — a caller with a
/// deck builds them from `Pdk::layers` and `Pdk::layer_gds` (this crate does
/// not depend on `verify`). A later pair on the same GDS key wins.
#[must_use]
pub fn layer_names(pairs: &[(String, (u16, u16))]) -> LayerMap {
    pairs.iter().map(|(n, (l, d))| ((i32::from(*l), i32::from(*d)), n.clone())).collect()
}

/// The name for GDS `(layer, datatype)`, else `L{layer}/{datatype}`. Datatype
/// matters: sky130 poly and licon are both layer 66.
fn layer_label(names: &LayerMap, (l, d): (u16, u16)) -> String {
    names.get(&(i32::from(l), i32::from(d))).cloned().unwrap_or_else(|| format!("L{l}/{d}"))
}

/// `(x0, y0, x1, y1)` over every vertex; `None` when there are none.
fn bounds(polys: &[Poly]) -> Option<(i32, i32, i32, i32)> {
    let mut pts = polys.iter().flat_map(|p| p.pts.iter());
    let &[x, y] = pts.next()?;
    Some(pts.fold((x, y, x, y), |(x0, y0, x1, y1), &[x, y]| {
        (x0.min(x), y0.min(y), x1.max(x), y1.max(y))
    }))
}

// ── GDS parser: BOUNDARY, BOX, PATH, SREF, AREF, TEXT ──

/// One GDS structure, unflattened (its own frame).
struct GdsCell {
    polys: Vec<Poly>,
    srefs: Vec<SRef>,
    texts: Vec<TextEntry>,
}

/// An SREF, or an AREF as a `cols × rows` lattice of SREFs.
struct SRef {
    /// Referenced structure name.
    name: String,
    /// Origin of the first placement, parent frame.
    x: i32,
    y: i32,
    /// Mirror about x before rotating.
    mirror_x: bool,
    /// Counter-clockwise rotation, degrees.
    angle_deg: f64,
    /// Lattice size; 1 × 1 for an SREF.
    cols: u16,
    rows: u16,
    /// Per-column and per-row step, parent frame.
    col_vec: (i32, i32),
    row_vec: (i32, i32),
}

/// The GDS element whose records are being read.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Element {
    None,
    /// BOUNDARY or BOX.
    Boundary,
    Path,
    Sref,
    Aref,
    Text,
}

/// Decodes an 8-byte GDS excess-64 base-16 real.
///
/// # Panics
/// When `b` is shorter than 8 bytes.
fn gds_real(b: &[u8]) -> f64 {
    let sign = if b[0] & 0x80 != 0 { -1.0 } else { 1.0 };
    let exp = (b[0] & 0x7F) as i32 - 64;
    let mut mant: u64 = 0;
    for &byte in &b[1..8] {
        mant = (mant << 8) | byte as u64;
    }
    sign * (mant as f64 / (1u64 << 56) as f64) * 16.0f64.powi(exp)
}

/// Big-endian i16 at `off`. Panics past the end.
fn read_i16(data: &[u8], off: usize) -> i16 {
    i16::from_be_bytes([data[off], data[off + 1]])
}

/// Big-endian i32 at `off`. Panics past the end.
fn read_i32(data: &[u8], off: usize) -> i32 {
    i32::from_be_bytes(data[off..off + 4].try_into().unwrap())
}

/// `len` bytes at `off` as text, cut at the first NUL (GDS pads to even).
fn read_string(data: &[u8], off: usize, len: usize) -> String {
    let s = &data[off..off + len];
    let end = s.iter().position(|&b| b == 0).unwrap_or(len);
    String::from_utf8_lossy(&s[..end]).into_owned()
}

/// A PATH as one quad per segment, `hw` either side of the centre line;
/// ends are flush (PATHTYPE 0).
fn path_to_polys(layer: u16, datatype: u16, hw: i32, pts: &[[i32; 2]]) -> Vec<Poly> {
    let mut out = Vec::with_capacity(pts.len().saturating_sub(1));
    for seg in pts.windows(2) {
        let [x0, y0] = seg[0];
        let [x1, y1] = seg[1];
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
        out.push(Poly { layer, datatype, pts: rect });
    }
    out
}

/// Every structure in a GDS stream by name. Reading stops at the first
/// truncated or malformed record header; unknown records are skipped.
fn parse_gds_cells(data: &[u8]) -> HashMap<String, GdsCell> {
    let mut cells: HashMap<String, GdsCell> = HashMap::new();
    let mut i = 0;

    let mut cur_name: Option<String> = None;
    let mut cur_polys: Vec<Poly> = Vec::new();
    let mut cur_srefs: Vec<SRef> = Vec::new();
    let mut cur_texts: Vec<TextEntry> = Vec::new();

    let mut el = Element::None;
    let mut layer: u16 = 0;
    let mut datatype: u16 = 0;
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
                el = Element::Boundary;
                layer = 0;
                datatype = 0;
            }
            0x09 => {
                el = Element::Path;
                layer = 0;
                datatype = 0;
                path_width = 0;
            }
            0x0A => {
                el = Element::Sref;
                sref_mirror = false;
                sref_angle = 0.0;
            }
            0x0B => {
                el = Element::Aref;
                sref_mirror = false;
                sref_angle = 0.0;
                aref_cols = 1;
                aref_rows = 1;
            }
            0x0C => {
                // TEXT
                el = Element::Text;
                text_string.clear();
                text_xy = (0, 0);
            }

            // LAYER
            0x0D if matches!(el, Element::Boundary | Element::Path) && len >= 6 => {
                layer = read_i16(data, i + 4) as u16;
            }

            // DATATYPE
            0x0E if matches!(el, Element::Boundary | Element::Path) && len >= 6 => {
                datatype = read_i16(data, i + 4) as u16;
            }

            // WIDTH (path)
            0x0F if el == Element::Path && len >= 8 => {
                path_width = read_i32(data, i + 4);
            }

            // STRING (text content)
            0x19 if el == Element::Text => {
                text_string = read_string(data, i + 4, len - 4);
            }

            // SNAME
            0x12 if matches!(el, Element::Sref | Element::Aref) => {
                sref_name = read_string(data, i + 4, len - 4);
            }

            // COLROW (aref)
            0x13 if el == Element::Aref && len >= 8 => {
                aref_cols = read_i16(data, i + 4) as u16;
                aref_rows = read_i16(data, i + 6) as u16;
            }

            // STRANS
            0x1A if matches!(el, Element::Sref | Element::Aref) && len >= 6 => {
                let flags = u16::from_be_bytes([data[i + 4], data[i + 5]]);
                sref_mirror = flags & 0x8000 != 0;
            }

            // ANGLE
            0x1C if matches!(el, Element::Sref | Element::Aref) && len >= 12 => {
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

                match el {
                    Element::Boundary => {
                        if pts.len() > 1 && pts.first() == pts.last() { pts.pop(); }
                        if pts.len() >= 3 {
                            cur_polys.push(Poly { layer, datatype, pts });
                        }
                    }
                    Element::Path if pts.len() >= 2 => {
                        let hw = (path_width / 2).max(1);
                        cur_polys.extend(path_to_polys(layer, datatype, hw, &pts));
                    }
                    Element::Sref if !pts.is_empty() => cur_srefs.push(SRef {
                        name: sref_name.clone(),
                        x: pts[0][0], y: pts[0][1],
                        mirror_x: sref_mirror, angle_deg: sref_angle,
                        cols: 1, rows: 1,
                        col_vec: (0, 0), row_vec: (0, 0),
                    }),
                    Element::Aref if pts.len() >= 3 => {
                        let (x0, y0) = (pts[0][0], pts[0][1]);
                        let (c, r) = (i32::from(aref_cols.max(1)), i32::from(aref_rows.max(1)));
                        cur_srefs.push(SRef {
                            name: sref_name.clone(),
                            x: x0, y: y0,
                            mirror_x: sref_mirror, angle_deg: sref_angle,
                            cols: aref_cols, rows: aref_rows,
                            col_vec: ((pts[1][0] - x0) / c, (pts[1][1] - y0) / c),
                            row_vec: ((pts[2][0] - x0) / r, (pts[2][1] - y0) / r),
                        });
                    }
                    Element::Text if !pts.is_empty() => text_xy = (pts[0][0], pts[0][1]),
                    _ => {}
                }
            }

            // ENDEL
            0x11 => {
                if el == Element::Text && !text_string.is_empty() {
                    cur_texts.push(TextEntry {
                        x: text_xy.0, y: text_xy.1,
                        text: std::mem::take(&mut text_string),
                    });
                }
                el = Element::None;
            }
            _ => {}
        }
        i += len;
    }
    cells
}

/// `(x, y)` mirrored about x (when `mirror_x`), then rotated `angle_deg`
/// counter-clockwise, rounded to the nearest unit.
fn transform_point(x: i32, y: i32, mirror_x: bool, angle_deg: f64) -> (i32, i32) {
    let (px, mut py) = (x as f64, y as f64);
    if mirror_x { py = -py; }
    let rad = angle_deg.to_radians();
    let (s, c) = (rad.sin(), rad.cos());
    ((px * c - py * s).round() as i32, (px * s + py * c).round() as i32)
}

/// Appends `name`'s polygons and labels, and those of everything it
/// references, under the transform (mirror, rotate, then translate by
/// `(tx, ty)`). Unknown names and references deeper than 64 are dropped.
#[allow(clippy::too_many_arguments)]
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
        out.push(Poly { layer: poly.layer, datatype: poly.datatype, pts });
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

/// A structure nothing references, else any structure; `None` when empty.
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

/// Parses a GDS stream and returns the top cell's polygons and text labels,
/// hierarchy flattened, in database units. Malformed input yields what was
/// read before the damage (possibly nothing); it never panics.
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

// ── Colours, triangulation, stroke text ──

/// Fill colour and whether to fill at all. Known layer names get the
/// conventional colours; wells, implants and marker layers are outline-only so
/// they never hide the devices under them; anything else hashes (layer, datatype).
fn layer_style(names: &LayerMap, key: (u16, u16)) -> ([f32; 4], bool) {
    let name = names.get(&(i32::from(key.0), i32::from(key.1))).map(String::as_str).unwrap_or("");
    let c = |r: f32, g: f32, b: f32| [r, g, b, 0.55];
    let known = match name {
        "diff" | "comp" | "activ" => Some(c(0.20, 0.75, 0.25)),
        "tap" => Some(c(0.60, 0.75, 0.20)),
        "poly" | "poly2" | "gatpoly" => Some(c(0.90, 0.20, 0.20)),
        "licon" | "mcon" | "via" | "via1" | "via2" | "via3" | "via4" | "cont" | "contact" => Some([0.95, 0.95, 0.95, 0.85]),
        "li1" | "li" => Some(c(0.60, 0.45, 0.90)),
        "met1" | "m1" | "metal1" => Some(c(0.25, 0.50, 1.00)),
        "met2" | "m2" | "metal2" => Some(c(0.90, 0.40, 0.85)),
        "met3" | "m3" | "metal3" => Some(c(0.20, 0.85, 0.85)),
        "met4" | "m4" | "metal4" => Some(c(1.00, 0.65, 0.20)),
        "met5" | "m5" | "metal5" => Some(c(0.85, 0.85, 0.30)),
        "rpoly" | "poly_res" => Some(c(1.00, 0.55, 0.22)),
        _ => None,
    };
    if let Some(col) = known {
        return (col, true);
    }
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
    let outline_only = name.contains("well")
        || name.contains("sdm")
        || name.ends_with("_mk")
        || matches!(name, "nplus" | "pplus" | "nsd" | "psd" | "npc" | "hvtp" | "lvtn" | "rpm" | "diom" | "dualgate" | "sab" | "salblock");
    (P[(usize::from(key.0) * 7 + usize::from(key.1)) % P.len()], !outline_only)
}

impl Poly {
    fn key(&self) -> (u16, u16) {
        (self.layer, self.datatype)
    }
}

/// The stroke colour for a fill: 1.5× brighter, opaque.
fn outline_color(fill: [f32; 4]) -> [f32; 4] {
    [(fill[0] * 1.5).min(1.0), (fill[1] * 1.5).min(1.0), (fill[2] * 1.5).min(1.0), 1.0]
}

/// Filled layers' polygons as a triangle list (ear clipping); polygons
/// earcut rejects are skipped.
fn triangulate(polys: &[Poly], names: &LayerMap) -> Vec<Vertex> {
    let mut verts = Vec::new();
    for p in polys {
        let (color, fill) = layer_style(names, p.key());
        if !fill {
            continue;
        }
        let coords: Vec<f64> = p.pts.iter().flat_map(|v| [f64::from(v[0]), f64::from(v[1])]).collect();
        for i in earcutr::earcut(&coords, &[], 2).unwrap_or_default() {
            verts.push(Vertex { pos: [p.pts[i][0] as f32, p.pts[i][1] as f32], color });
        }
    }
    verts
}

/// Every polygon's closed outline as a line list.
fn outline_vertices(polys: &[Poly], names: &LayerMap) -> Vec<Vertex> {
    let mut verts = Vec::new();
    for p in polys {
        let color = outline_color(layer_style(names, p.key()).0);
        for (i, a) in p.pts.iter().enumerate() {
            let b = p.pts[(i + 1) % p.pts.len()];
            verts.push(Vertex { pos: [a[0] as f32, a[1] as f32], color });
            verts.push(Vertex { pos: [b[0] as f32, b[1] as f32], color });
        }
    }
    verts
}

/// Glyphs on a 5×7 grid as line-segment endpoint pairs; a `PEN_UP` pair skips.
const PEN_UP: (u8, u8) = (255, 255);

/// The stroke pairs for `ch`; unknown characters draw an X.
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
        ',' => &[(2,1),(1,0)],
        '.' => &[(2,0),(2,1),(2,1),(2,0)],
        ' ' => &[],
        _ => &[(0,0),(4,7), PEN_UP,PEN_UP, (0,7),(4,0)], // fallback: X
    }
}

/// A thick line as two triangles.
fn stroke_quad(verts: &mut Vec<Vertex>, a: [f32; 2], b: [f32; 2], half_w: f32, color: [f32; 4]) {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len = (dx * dx + dy * dy).sqrt().max(1e-6);
    let (nx, ny) = (-dy / len * half_w, dx / len * half_w);
    for pos in [
        [a[0] - nx, a[1] - ny],
        [b[0] - nx, b[1] - ny],
        [b[0] + nx, b[1] + ny],
        [a[0] - nx, a[1] - ny],
        [b[0] + nx, b[1] + ny],
        [a[0] + nx, a[1] + ny],
    ] {
        verts.push(Vertex { pos, color });
    }
}

/// `text` as stroked glyphs from `(x, y)`, one glyph unit = `s`.
fn push_text(verts: &mut Vec<Vertex>, text: &str, x: f32, y: f32, s: f32, color: [f32; 4]) {
    for (ci, ch) in text.chars().enumerate() {
        let cx = x + ci as f32 * 6.0 * s;
        for seg in glyph(ch).chunks_exact(2) {
            if seg.contains(&PEN_UP) {
                continue;
            }
            let at = |(gx, gy): (u8, u8)| [cx + f32::from(gx) * s, y + f32::from(gy) * s];
            stroke_quad(verts, at(seg[0]), at(seg[1]), s * 0.25, color);
        }
    }
}

/// A solid axis-aligned rectangle as two triangles.
fn push_rect(verts: &mut Vec<Vertex>, x0: f32, y0: f32, x1: f32, y1: f32, color: [f32; 4]) {
    for pos in [[x0, y0], [x1, y0], [x1, y1], [x0, y0], [x1, y1], [x0, y1]] {
        verts.push(Vertex { pos, color });
    }
}

/// World-space text labels, sized to the layout's span.
fn text_vertices(texts: &[TextEntry], polys: &[Poly]) -> Vec<Vertex> {
    let span = bounds(polys).map_or(7000.0, |(x0, y0, x1, y1)| (x1 - x0).max(y1 - y0).max(1) as f32);
    let mut verts = Vec::new();
    for t in texts {
        push_text(&mut verts, &t.text, t.x as f32, t.y as f32, span * 0.025 / 7.0, [1.0, 1.0, 1.0, 0.95]);
    }
    verts
}

/// Screen-space (NDC) legend in the top-right corner: a swatch and name per layer.
fn build_legend(polys: &[Poly], layer_names: &LayerMap) -> Vec<Vertex> {
    let mut layers: Vec<(u16, u16)> = polys.iter().map(Poly::key).collect();
    layers.sort_unstable();
    layers.dedup();
    if layers.is_empty() {
        return Vec::new();
    }
    let (row, margin) = (0.04_f32, 0.02_f32);
    let (x0, y_top) = (1.0 - margin - 0.3, 1.0 - margin);
    let mut verts = Vec::new();
    let bottom = y_top - layers.len() as f32 * row - margin;
    push_rect(&mut verts, x0 - margin, bottom, 1.0 - margin / 2.0, y_top + margin / 2.0, [0.08, 0.08, 0.10, 0.75]);
    for (i, &layer) in layers.iter().enumerate() {
        let y = y_top - (i as f32 + 0.5) * row;
        let c = layer_style(layer_names, layer).0;
        push_rect(&mut verts, x0, y - row * 0.35, x0 + 0.03, y + row * 0.35, [c[0], c[1], c[2], 1.0]);
        let label = layer_label(layer_names, layer);
        push_text(&mut verts, &label, x0 + 0.05, y - row * 0.35, row * 0.11, [0.9, 0.9, 0.9, 1.0]);
    }
    verts
}

/// The camera that fits every polygon in 90 % of the shorter NDC axis,
/// centred; the identity when there are none.
fn fit_view(polys: &[Poly], aspect: f32) -> CamUni {
    let Some((x0, y0, x1, y1)) = bounds(polys) else {
        return CamUni { offset: [0.0; 2], scale: 1.0, aspect };
    };
    let span = (x1 - x0).max(y1 - y0).max(1) as f32;
    CamUni { offset: [-(x0 + x1) as f32 / 2.0, -(y0 + y1) as f32 / 2.0], scale: 1.8 / span, aspect }
}

// ── SVG export (headless) ──

/// Renders a GDS stream as a standalone 800 × 800 SVG in database units
/// (y flipped so the layout reads upright), one `<g id="{layer}">` per GDS
/// layer and the labels on top. An empty layout is an empty `<svg/>`.
#[must_use]
pub fn export_svg(gds_bytes: &[u8], layer_names: &LayerMap) -> String {
    let (polys, texts) = parse_gds(gds_bytes);
    let Some((x0, y0, x1, y1)) = bounds(&polys) else {
        return String::from(r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#);
    };
    let pad = (x1 - x0).max(y1 - y0) / 40;
    let (vx, vy, vw, vh) = (x0 - pad, y0 - pad, x1 - x0 + 2 * pad, y1 - y0 + 2 * pad);
    let mut layers: Vec<(u16, u16)> = polys.iter().map(Poly::key).collect();
    layers.sort_unstable();
    layers.dedup();

    let byte = |v: f32| (v * 255.0) as u8;
    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{vx} {vy} {vw} {vh}" width="800" height="800" style="background:#14141a">"#
    );
    // GDS is y-up, SVG y-down.
    let flip = vy * 2 + vh;
    svg.push_str(&format!(r#"<g transform="translate(0,{flip}) scale(1,-1)">"#));
    for &layer in &layers {
        let (c, fill) = layer_style(layer_names, layer);
        let o = outline_color(c);
        let fill = if fill {
            format!("rgba({},{},{},{})", byte(c[0]), byte(c[1]), byte(c[2]), c[3])
        } else {
            "none".into()
        };
        svg.push_str(&format!(
            r#"<g id="{}" fill="{fill}" stroke="rgba({},{},{},1)" stroke-width="{}">"#,
            layer_label(layer_names, layer),
            byte(o[0]),
            byte(o[1]),
            byte(o[2]),
            (vw.max(vh) as f32 * 0.001) as i32,
        ));
        for p in polys.iter().filter(|p| p.key() == layer) {
            let pts: Vec<String> = p.pts.iter().map(|[x, y]| format!("{x},{y}")).collect();
            svg.push_str(&format!(r#"<polygon points="{}"/>"#, pts.join(" ")));
        }
        svg.push_str("</g>");
    }
    svg.push_str("</g>");
    // Labels outside the flip so the glyphs read upright.
    let size = vw.max(vh) / 60;
    for t in &texts {
        svg.push_str(&format!(
            r#"<text x="{}" y="{}" font-size="{size}" font-family="monospace" fill="white">{}</text>"#,
            t.x,
            flip - t.y,
            t.text.replace('&', "&amp;").replace('<', "&lt;"),
        ));
    }
    svg.push_str("</svg>");
    svg
}

// ── GPU ──

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

/// Screen-space: positions are already NDC.
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

/// A vertex buffer and its vertex count.
struct Mesh {
    buf: Option<wgpu::Buffer>,
    n: u32,
}

/// The window's wgpu state and the three meshes it draws.
struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    fill_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    overlay_pipeline: wgpu::RenderPipeline,
    cam_buf: wgpu::Buffer,
    cam_bg: wgpu::BindGroup,
    fill: Mesh,
    line: Mesh,
    overlay: Mesh,
}

/// An alpha-blended pipeline over [`Vertex`] with `vs`/`fs` entry points.
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
                attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4],
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

impl Gpu {
    /// Opens a surface on `win`.
    ///
    /// # Panics
    /// When no adapter or device is available.
    fn new(win: std::sync::Arc<Window>) -> Self {
        let sz = win.inner_size();
        let inst = wgpu::Instance::default();
        let surface = inst.create_surface(win).unwrap();
        let adapter = pollster::block_on(inst.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .expect("no GPU adapter found");
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))
                .unwrap();

        let caps = surface.get_capabilities(&adapter);
        let fmt = caps.formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: fmt,
            width: sz.width.max(1),
            height: sz.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let module = |src: &str| {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(src.into()),
            })
        };
        let (shader, overlay_shader) = (module(SHADER), module(OVERLAY_SHADER));
        let cam_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: std::mem::size_of::<CamUni>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let cam_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: cam_buf.as_entire_binding() }],
        });
        let layout = |bgls: &[&wgpu::BindGroupLayout]| {
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: bgls,
                push_constant_ranges: &[],
            })
        };
        let (pll, overlay_pll) = (layout(&[&bgl]), layout(&[]));
        use wgpu::PrimitiveTopology::{LineList, TriangleList};
        Self {
            fill_pipeline: make_pipeline(&device, &shader, &pll, fmt, TriangleList),
            line_pipeline: make_pipeline(&device, &shader, &pll, fmt, LineList),
            overlay_pipeline: make_pipeline(&device, &overlay_shader, &overlay_pll, fmt, TriangleList),
            device,
            queue,
            surface,
            config,
            cam_buf,
            cam_bg,
            fill: Mesh { buf: None, n: 0 },
            line: Mesh { buf: None, n: 0 },
            overlay: Mesh { buf: None, n: 0 },
        }
    }

    /// Reconfigures the surface; a zero extent (minimised) is ignored.
    fn resize(&mut self, w: u32, h: u32) {
        if w > 0 && h > 0 {
            self.config.width = w;
            self.config.height = h;
            self.surface.configure(&self.device, &self.config);
        }
    }

    /// Uploads `verts`; an empty slice is a mesh with no buffer.
    fn mesh(&self, verts: &[Vertex]) -> Mesh {
        if verts.is_empty() {
            return Mesh { buf: None, n: 0 };
        }
        let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: std::mem::size_of_val(verts) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&buf, 0, bytemuck::cast_slice(verts));
        Mesh { buf: Some(buf), n: verts.len() as u32 }
    }

    /// Draws one frame; a lost surface is reconfigured and the frame skipped.
    fn draw(&mut self, cam: &CamUni) {
        self.queue.write_buffer(&self.cam_buf, 0, bytemuck::bytes_of(cam));
        let Ok(frame) = self.surface.get_current_texture() else {
            self.surface.configure(&self.device, &self.config);
            return;
        };
        let view = frame.texture.create_view(&Default::default());
        let mut enc = self.device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.08, g: 0.08, b: 0.10, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            for (mesh, pipeline, world) in [
                (&self.fill, &self.fill_pipeline, true),
                (&self.line, &self.line_pipeline, true),
                (&self.overlay, &self.overlay_pipeline, false),
            ] {
                let Some(buf) = &mesh.buf else { continue };
                rp.set_pipeline(pipeline);
                if world {
                    rp.set_bind_group(0, &self.cam_bg, &[]);
                }
                rp.set_vertex_buffer(0, buf.slice(..));
                rp.draw(0..mesh.n, 0..1);
            }
        }
        self.queue.submit([enc.finish()]);
        frame.present();
    }
}

// ── App: a watched GDS file, or a probe channel ──

/// Where the window's geometry comes from.
enum Source {
    /// A GDS file, polled for a new mtime every 500 ms.
    File { path: PathBuf, mtime: Option<SystemTime>, last_poll: Instant },
    Channel(mpsc::Receiver<ProbeMsg>),
}

/// One update sent to a [`Probe`] window.
struct ProbeMsg {
    polys: Vec<Poly>,
    title: Option<String>,
}

/// The winit application: window, GPU, camera and source.
struct App {
    win: Option<std::sync::Arc<Window>>,
    gpu: Option<Gpu>,
    cam: CamUni,
    cursor: [f64; 2],
    drag: bool,
    fit: bool,
    title: String,
    source: Source,
    layer_names: LayerMap,
    /// The last geometry shown, re-uploaded once the window exists.
    shown: (Vec<Poly>, Vec<TextEntry>),
}

impl App {
    fn new(title: String, source: Source, layer_names: LayerMap) -> Self {
        Self {
            win: None,
            gpu: None,
            cam: CamUni { offset: [0.0; 2], scale: 1.0, aspect: 1.0 },
            cursor: [0.0; 2],
            drag: false,
            fit: true,
            title,
            source,
            layer_names,
            shown: (Vec::new(), Vec::new()),
        }
    }

    /// Displays `polys` (sorted by layer) and `texts`; refits the camera
    /// when a fit is pending and there is geometry.
    fn show(&mut self, mut polys: Vec<Poly>, texts: Vec<TextEntry>) {
        polys.sort_by_key(Poly::key);
        if self.fit && !polys.is_empty() {
            self.cam = fit_view(&polys, self.cam.aspect);
            self.fit = false;
        }
        if let Some(gpu) = &mut self.gpu {
            let mut fill = triangulate(&polys, &self.layer_names);
            fill.extend(text_vertices(&texts, &polys));
            gpu.fill = gpu.mesh(&fill);
            gpu.line = gpu.mesh(&outline_vertices(&polys, &self.layer_names));
            gpu.overlay = gpu.mesh(&build_legend(&polys, &self.layer_names));
        }
        self.shown = (polys, texts);
    }

    /// Re-reads the source file; a read error is reported and the view kept.
    fn load_gds(&mut self) {
        let Source::File { path, mtime, .. } = &mut self.source else { return };
        let data = match std::fs::read(&*path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("read {}: {e}", path.display());
                return;
            }
        };
        *mtime = std::fs::metadata(&*path).and_then(|m| m.modified()).ok();
        let (polys, texts) = parse_gds(&data);
        self.show(polys, texts);
    }

    /// The cursor in NDC (`[-1, 1]`, y up); the origin without a window.
    fn cursor_ndc(&self) -> [f32; 2] {
        self.win.as_ref().map_or([0.0; 2], |w| {
            let s = w.inner_size();
            [
                2.0 * self.cursor[0] as f32 / s.width as f32 - 1.0,
                -(2.0 * self.cursor[1] as f32 / s.height as f32 - 1.0),
            ]
        })
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.win.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title(&self.title)
            .with_inner_size(LogicalSize::new(1280u32, 720u32));
        let w = std::sync::Arc::new(el.create_window(attrs).unwrap());
        let s = w.inner_size();
        self.cam.aspect = s.width as f32 / s.height.max(1) as f32;
        self.gpu = Some(Gpu::new(w.clone()));
        self.win = Some(w);
        if matches!(self.source, Source::File { .. }) {
            self.load_gds();
        } else {
            let (polys, texts) = std::mem::take(&mut self.shown);
            self.show(polys, texts);
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, ev: WindowEvent) {
        match ev {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(s) => {
                self.cam.aspect = s.width as f32 / s.height.max(1) as f32;
                if let Some(g) = &mut self.gpu {
                    g.resize(s.width, s.height);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(g) = &mut self.gpu {
                    g.draw(&self.cam);
                }
            }
            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                self.drag = state == ElementState::Pressed;
            }
            WindowEvent::CursorMoved { position, .. } => {
                let (dx, dy) = (position.x - self.cursor[0], position.y - self.cursor[1]);
                self.cursor = [position.x, position.y];
                if let (true, Some(w)) = (self.drag, &self.win) {
                    let s = w.inner_size();
                    self.cam.offset[0] +=
                        (2.0 * dx as f32 / s.width as f32) * self.cam.aspect / self.cam.scale;
                    self.cam.offset[1] -= (2.0 * dy as f32 / s.height as f32) / self.cam.scale;
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let d = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 100.0,
                };
                // Zoom about the cursor.
                let ndc = self.cursor_ndc();
                let inv_old = 1.0 / self.cam.scale;
                self.cam.scale *= zoom_factor(d);
                let diff = 1.0 / self.cam.scale - inv_old;
                self.cam.offset[0] += ndc[0] * self.cam.aspect * diff;
                self.cam.offset[1] += ndc[1] * diff;
            }
            WindowEvent::KeyboardInput { event: ev, .. } if ev.state == ElementState::Pressed => {
                let step = 0.1 / self.cam.scale;
                match ev.logical_key {
                    Key::Named(NamedKey::Escape) => el.exit(),
                    Key::Named(NamedKey::ArrowUp) => self.cam.offset[1] += step,
                    Key::Named(NamedKey::ArrowDown) => self.cam.offset[1] -= step,
                    Key::Named(NamedKey::ArrowLeft) => self.cam.offset[0] -= step,
                    Key::Named(NamedKey::ArrowRight) => self.cam.offset[0] += step,
                    Key::Character(ref c) => match c.as_str() {
                        "r" => {
                            self.fit = true;
                            self.load_gds();
                        }
                        "=" | "+" => self.cam.scale *= 1.2,
                        "-" => self.cam.scale /= 1.2,
                        _ => {}
                    },
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _el: &ActiveEventLoop) {
        match &mut self.source {
            Source::File { path, mtime, last_poll } => {
                if last_poll.elapsed() >= Duration::from_millis(500) {
                    *last_poll = Instant::now();
                    let mt = std::fs::metadata(&*path).and_then(|m| m.modified()).ok();
                    if mt.is_some() && mt != *mtime {
                        self.load_gds();
                    }
                }
            }
            Source::Channel(rx) => {
                if let Some(msg) = rx.try_iter().last() {
                    if let (Some(t), Some(win)) = (&msg.title, &self.win) {
                        win.set_title(&format!("{} \u{2014} {t}", self.title));
                    }
                    self.show(msg.polys, Vec::new());
                }
            }
        }
        if let Some(w) = &self.win {
            w.request_redraw();
        }
    }
}

/// The scale multiplier for one wheel step of `d` (positive zooms in):
/// 1.1 per step in, 1 / 1.1 out, 1 for no movement.
fn zoom_factor(d: f32) -> f32 {
    if d > 0.0 { 1.1 } else { 1.0 / 1.1 }
}

/// Opens a window on a GDS file, reloading it whenever it changes on disk,
/// and blocks until the window closes. Without a display server the process
/// exits with status 1.
///
/// # Panics
/// When the event loop fails while running.
pub fn run_viewer(path: PathBuf, layer_names: LayerMap) {
    let el = EventLoop::new().unwrap_or_else(|e| {
        eprintln!("cannot create window (no display server?): {e}");
        std::process::exit(1);
    });
    let title = format!("GDS \u{2014} {}", path.display());
    let source = Source::File { path, mtime: None, last_poll: Instant::now() };
    el.run_app(&mut App::new(title, source, layer_names)).unwrap();
}

/// An event loop that runs off the main thread (the probe's window thread).
#[cfg(target_os = "linux")]
fn make_event_loop() -> Result<EventLoop<()>, winit::error::EventLoopError> {
    let mut builder = EventLoop::builder();
    winit::platform::x11::EventLoopBuilderExtX11::with_any_thread(&mut builder, true);
    winit::platform::wayland::EventLoopBuilderExtWayland::with_any_thread(&mut builder, true);
    builder.build()
}

#[cfg(not(target_os = "linux"))]
fn make_event_loop() -> Result<EventLoop<()>, winit::error::EventLoopError> {
    EventLoop::new()
}

/// A window on a background thread showing the latest polygons sent to it.
/// Stays open after the sender is dropped; a no-op without a display.
pub struct Probe {
    tx: Option<mpsc::Sender<ProbeMsg>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Probe {
    /// Spawns the window thread titled `title`; a missing display is
    /// reported on stderr and leaves a probe whose sends are dropped.
    #[must_use]
    pub fn open(title: &str) -> Self {
        let (tx, rx) = mpsc::channel();
        let title = title.to_string();
        let handle = std::thread::spawn(move || {
            let el = match make_event_loop() {
                Ok(el) => el,
                Err(e) => {
                    eprintln!("probe: no display: {e}");
                    return;
                }
            };
            el.run_app(&mut App::new(title, Source::Channel(rx), LayerMap::new())).unwrap();
        });
        Probe { tx: Some(tx), handle: Some(handle) }
    }

    /// Stops sending and blocks until the user closes the window; later
    /// [`Probe::send`]s are no-ops.
    pub fn wait(&mut self) {
        self.tx.take();
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }

    /// Replaces the shown geometry with a copy of `polys`, appending `title`
    /// to the window title when given. Never blocks; a closed window drops it.
    pub fn send(&self, polys: &[Poly], title: Option<&str>) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(ProbeMsg { polys: polys.to_vec(), title: title.map(String::from) });
        }
    }
}

/// `pnr_core` shapes as 4-point polygons on their GDS `(layer, datatype)`:
/// `gds[LayerId.0]` (`Pdk::layer_gds`); a missing row keeps the layer id
/// with datatype 0.
#[must_use]
pub fn polys_from_shapes(shapes: &[pnr_core::Shape], gds: &[(u16, u16)]) -> Vec<Poly> {
    shapes
        .iter()
        .map(|s| {
            let r = s.rect;
            let (layer, datatype) = gds.get(s.layer.0 as usize).copied().unwrap_or((s.layer.0, 0));
            Poly {
                layer,
                datatype,
                pts: vec![[r.x, r.y], [r.x + r.w, r.y], [r.x + r.w, r.y + r.h], [r.x, r.y + r.h]],
            }
        })
        .collect()
}

/// Show a macro's geometry (GDS table as in [`polys_from_shapes`]) and block
/// until the window closes.
pub fn show_macro(mac: &pnr_core::Macro, title: &str, gds: &[(u16, u16)]) {
    let mut probe = Probe::open(title);
    probe.send(&polys_from_shapes(&mac.shapes, gds), Some(title));
    probe.wait();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_names_and_bounds() {
        let names = layer_names(&[("met1".into(), (68, 20)), ("via".into(), (68, 44))]);
        assert_eq!(names[&(68, 20)], "met1");
        assert_eq!(layer_label(&names, (68, 44)), "via");
        assert_eq!(layer_label(&names, (68, 5)), "L68/5");
        assert!(layer_style(&names, (68, 20)) != layer_style(&names, (68, 44)));
        let p = |pts: Vec<[i32; 2]>| Poly { layer: 0, datatype: 0, pts };
        assert_eq!(bounds(&[]), None);
        assert_eq!(bounds(&[p(vec![[1, 5], [3, -2]]), p(vec![[0, 0]])]), Some((0, -2, 3, 5)));
        let s = pnr_core::Shape { layer: pnr_core::LayerId(1), rect: pnr_core::Rect { x: 0, y: 0, w: 1, h: 1 } };
        let q = &polys_from_shapes(&[s], &[(0, 0), (68, 20)])[0];
        assert_eq!((q.layer, q.datatype), (68, 20));
        let q = &polys_from_shapes(&[s], &[])[0];
        assert_eq!((q.layer, q.datatype), (1, 0), "no table row: layer id, datatype 0");
    }

    // ── GDS stream builders ──

    fn rec(t: u8, dt: u8, payload: &[u8]) -> Vec<u8> {
        let len = u16::try_from(4 + payload.len()).unwrap();
        let mut v = len.to_be_bytes().to_vec();
        v.extend([t, dt]);
        v.extend(payload);
        v
    }
    fn s_rec(t: u8, s: &str) -> Vec<u8> {
        let mut b = s.as_bytes().to_vec();
        if b.len() % 2 == 1 {
            b.push(0);
        }
        rec(t, 6, &b)
    }
    fn i16_rec(t: u8, v: &[i16]) -> Vec<u8> {
        rec(t, 2, &v.iter().flat_map(|x| x.to_be_bytes()).collect::<Vec<_>>())
    }
    fn xy(pts: &[[i32; 2]]) -> Vec<u8> {
        rec(0x10, 3, &pts.iter().flat_map(|p| p.iter().flat_map(|c| c.to_be_bytes())).collect::<Vec<_>>())
    }
    fn bgn(name: &str) -> Vec<u8> {
        [rec(0x05, 2, &[]), s_rec(0x06, name)].concat()
    }
    fn end() -> Vec<u8> {
        rec(0x07, 0, &[])
    }
    fn boundary(layer: i16, dt: i16, pts: &[[i32; 2]]) -> Vec<u8> {
        [rec(0x08, 0, &[]), i16_rec(0x0D, &[layer]), i16_rec(0x0E, &[dt]), xy(pts), rec(0x11, 0, &[])].concat()
    }
    fn path(layer: i16, width: i32, pts: &[[i32; 2]]) -> Vec<u8> {
        [rec(0x09, 0, &[]), i16_rec(0x0D, &[layer]), rec(0x0F, 3, &width.to_be_bytes()), xy(pts), rec(0x11, 0, &[])]
            .concat()
    }
    /// SREF with optional mirror and angle (90 or 180 only: exact reals).
    fn sref(name: &str, at: [i32; 2], mirror: bool, angle: u16) -> Vec<u8> {
        let mut v = [rec(0x0A, 0, &[]), s_rec(0x12, name)].concat();
        if mirror || angle != 0 {
            v.extend(rec(0x1A, 1, &(if mirror { 0x8000u16 } else { 0 }).to_be_bytes()));
        }
        match angle {
            0 => {}
            90 => v.extend(rec(0x1C, 5, &[0x42, 0x5A, 0, 0, 0, 0, 0, 0])),
            180 => v.extend(rec(0x1C, 5, &[0x42, 0xB4, 0, 0, 0, 0, 0, 0])),
            _ => unreachable!(),
        }
        [v, xy(&[at]), rec(0x11, 0, &[])].concat()
    }
    fn square(x: i32, y: i32, s: i32) -> Vec<[i32; 2]> {
        vec![[x, y], [x + s, y], [x + s, y + s], [x, y + s], [x, y]]
    }
    fn sorted_pts(p: &Poly) -> Vec<[i32; 2]> {
        let mut v = p.pts.clone();
        v.sort_unstable();
        v
    }

    // ── Decoders ──

    #[test]
    fn gds_real_decodes_excess_64_hex_floats() {
        assert_eq!(gds_real(&[0x41, 0x10, 0, 0, 0, 0, 0, 0]), 1.0);
        assert_eq!(gds_real(&[0x42, 0x5A, 0, 0, 0, 0, 0, 0]), 90.0);
        assert_eq!(gds_real(&[0xC0, 0x80, 0, 0, 0, 0, 0, 0]), -0.5);
        assert_eq!(gds_real(&[0; 8]), 0.0);
        // 1e-3 (the usual user unit) to within f64 rounding of 56-bit mantissa.
        let mm = gds_real(&[0x3E, 0x41, 0x89, 0x37, 0x4B, 0xC6, 0xA7, 0xF0]);
        assert!((mm - 1e-3).abs() < 1e-15, "{mm}");
    }

    #[test]
    fn read_string_stops_at_nul_padding() {
        assert_eq!(read_string(b"ab\0\0", 0, 4), "ab");
        assert_eq!(read_string(b"xabc", 1, 3), "abc");
        assert_eq!(read_string(b"", 0, 0), "");
    }

    // ── Parser ──

    #[test]
    fn malformed_streams_parse_to_nothing_without_panicking() {
        let cases: [&[u8]; 5] = [&[], &[0, 4], &[0, 2, 0x05, 0], &[0, 200, 0x05, 0, 1, 2], &[0xFF; 3]];
        for data in cases {
            let (p, t) = parse_gds(data);
            assert!(p.is_empty() && t.is_empty());
        }
        // A structure cut off before ENDSTR is not emitted.
        let cut = [bgn("A"), boundary(1, 0, &square(0, 0, 10))].concat();
        assert!(parse_gds(&cut).0.is_empty());
        // XY with an odd trailing word reads only whole points.
        let mut odd = bgn("A");
        odd.extend([rec(0x08, 0, &[]), rec(0x10, 3, &[0, 0, 0, 1, 0, 0, 0, 2, 0, 0])].concat());
        odd.extend([rec(0x11, 0, &[]), end()].concat());
        assert!(parse_gds(&odd).0.is_empty(), "one point is no polygon");
    }

    #[test]
    fn boundary_drops_closing_vertex_and_degenerate_polygons() {
        let gds = [
            bgn("TOP"),
            boundary(68, 20, &square(0, 0, 10)),
            boundary(68, 20, &[[0, 0], [5, 5], [0, 0]]),
            boundary(68, 20, &[[0, 0], [5, 5]]),
            end(),
        ]
        .concat();
        let (polys, _) = parse_gds(&gds);
        assert_eq!(polys.len(), 1);
        assert_eq!((polys[0].layer, polys[0].datatype), (68, 20));
        assert_eq!(polys[0].pts, vec![[0, 0], [10, 0], [10, 10], [0, 10]]);
    }

    #[test]
    fn box_record_is_read_as_a_boundary() {
        let el = [rec(0x2D, 0, &[]), i16_rec(0x0D, &[5]), xy(&square(0, 0, 4)), rec(0x11, 0, &[])].concat();
        let (polys, _) = parse_gds(&[bgn("T"), el, end()].concat());
        assert_eq!(polys.len(), 1);
        assert_eq!(polys[0].layer, 5);
    }

    #[test]
    fn paths_become_one_quad_per_segment() {
        let gds = [bgn("T"), path(3, 10, &[[0, 0], [100, 0], [100, 50]]), end()].concat();
        let (polys, _) = parse_gds(&gds);
        assert_eq!(polys.len(), 2);
        assert_eq!(sorted_pts(&polys[0]), vec![[0, -5], [0, 5], [100, -5], [100, 5]]);
        assert_eq!(sorted_pts(&polys[1]), vec![[95, 0], [95, 50], [105, 0], [105, 50]]);
        // Reversed direction draws the same rectangle.
        let back = parse_gds(&[bgn("T"), path(3, 10, &[[100, 0], [0, 0]]), end()].concat()).0;
        assert_eq!(sorted_pts(&back[0]), sorted_pts(&polys[0]));
    }

    #[test]
    fn path_width_zero_still_draws_and_negative_width_is_absolute() {
        let zero = parse_gds(&[bgn("T"), path(3, 0, &[[0, 0], [10, 0]]), end()].concat()).0;
        assert_eq!(sorted_pts(&zero[0]), vec![[0, -1], [0, 1], [10, -1], [10, 1]]);
        // GDS: a negative WIDTH is an absolute (unscaled) width of |w|.
        let neg = parse_gds(&[bgn("T"), path(3, -10, &[[0, 0], [10, 0]]), end()].concat()).0;
        assert_eq!(sorted_pts(&neg[0]), vec![[0, -5], [0, 5], [10, -5], [10, 5]]);
    }

    #[test]
    fn diagonal_path_quads_are_offset_perpendicular() {
        let quads = path_to_polys(1, 0, 10, &[[0, 0], [100, 100]]);
        assert_eq!(quads.len(), 1);
        // Normal (−7, 7) rounded from (−7.07, 7.07).
        assert_eq!(quads[0].pts, vec![[-7, 7], [93, 107], [107, 93], [7, -7]]);
        assert!(path_to_polys(1, 0, 10, &[[0, 0]]).is_empty());
    }

    #[test]
    fn sref_translates_mirrors_and_rotates() {
        let child = [bgn("C"), boundary(1, 0, &[[0, 0], [10, 0], [10, 20], [0, 20], [0, 0]]), end()].concat();
        let top = |r: Vec<u8>| [child.clone(), bgn("TOP"), r, end()].concat();

        let p = parse_gds(&top(sref("C", [100, 200], false, 0))).0;
        assert_eq!(sorted_pts(&p[0]), vec![[100, 200], [100, 220], [110, 200], [110, 220]]);

        let p = parse_gds(&top(sref("C", [0, 0], true, 0))).0;
        assert_eq!(sorted_pts(&p[0]), vec![[0, -20], [0, 0], [10, -20], [10, 0]]);

        // 90° CCW: (x, y) ↦ (−y, x).
        let p = parse_gds(&top(sref("C", [0, 0], false, 90))).0;
        assert_eq!(sorted_pts(&p[0]), vec![[-20, 0], [-20, 10], [0, 0], [0, 10]]);

        // Mirror then rotate 90: (x, y) ↦ (y, x).
        let p = parse_gds(&top(sref("C", [0, 0], true, 90))).0;
        assert_eq!(sorted_pts(&p[0]), vec![[0, 0], [0, 10], [20, 0], [20, 10]]);
    }

    /// Nested transforms compose as parent ∘ child: a mirrored parent
    /// reverses the child's rotation sense.
    #[test]
    fn nested_transforms_compose_parent_after_child() {
        let gds = [
            bgn("LEAF"),
            boundary(1, 0, &[[10, 0], [20, 0], [20, 5], [10, 5]]),
            end(),
            bgn("MID"),
            sref("LEAF", [0, 0], false, 90),
            end(),
            bgn("TOP"),
            sref("MID", [1000, 0], true, 0),
            end(),
        ]
        .concat();
        let p = parse_gds(&gds).0;
        // LEAF point (10, 0) → R90 → (0, 10) → Mx → (0, −10) → +1000.
        assert!(p[0].pts.contains(&[1000, -10]), "{:?}", p[0].pts);
        // (20, 5) → (−5, 20) → (−5, −20) → (995, −20).
        assert!(p[0].pts.contains(&[995, -20]), "{:?}", p[0].pts);
    }

    #[test]
    fn aref_expands_to_a_cols_by_rows_lattice() {
        let aref = [
            rec(0x0B, 0, &[]),
            s_rec(0x12, "C"),
            i16_rec(0x13, &[2, 3]),
            xy(&[[0, 0], [200, 0], [0, 900]]),
            rec(0x11, 0, &[]),
        ]
        .concat();
        let gds = [bgn("C"), boundary(1, 0, &square(0, 0, 10)), end(), bgn("TOP"), aref, end()].concat();
        let polys = parse_gds(&gds).0;
        assert_eq!(polys.len(), 6);
        let mut origins: Vec<[i32; 2]> = polys.iter().map(|p| sorted_pts(p)[0]).collect();
        origins.sort_unstable();
        assert_eq!(origins, vec![[0, 0], [0, 300], [0, 600], [100, 0], [100, 300], [100, 600]]);
    }

    #[test]
    fn texts_follow_their_instance_transform() {
        let text = [rec(0x0C, 0, &[]), s_rec(0x19, "vout"), xy(&[[5, 7]]), rec(0x11, 0, &[])].concat();
        let empty_text = [rec(0x0C, 0, &[]), xy(&[[1, 1]]), rec(0x11, 0, &[])].concat();
        let gds = [bgn("C"), text, empty_text, end(), bgn("TOP"), sref("C", [100, 0], false, 180), end()].concat();
        let (_, texts) = parse_gds(&gds);
        assert_eq!(texts.len(), 1, "an empty TEXT is dropped");
        assert_eq!((texts[0].x, texts[0].y, texts[0].text.as_str()), (95, -7, "vout"));
    }

    #[test]
    fn unknown_references_are_dropped_and_cycles_terminate() {
        let gds = [bgn("TOP"), boundary(1, 0, &square(0, 0, 1)), sref("NOPE", [0, 0], false, 0), end()].concat();
        assert_eq!(parse_gds(&gds).0.len(), 1);
        // A ↔ B cycle: neither is a top cell, so either is shown; depth caps it.
        let cyc = [
            bgn("A"),
            boundary(1, 0, &square(0, 0, 1)),
            sref("B", [1, 0], false, 0),
            end(),
            bgn("B"),
            sref("A", [1, 0], false, 0),
            end(),
        ]
        .concat();
        let n = parse_gds(&cyc).0.len();
        assert!(n > 0 && n <= 65, "{n}");
    }

    /// With several unreferenced structures the last one defined is the top
    /// (GDS writers emit the top cell last), every time.
    #[test]
    fn the_last_unreferenced_structure_is_the_top() {
        let gds = [
            bgn("FIRST"),
            boundary(1, 0, &square(0, 0, 1)),
            end(),
            bgn("SECOND"),
            boundary(2, 0, &square(0, 0, 1)),
            end(),
        ]
        .concat();
        for _ in 0..16 {
            let p = parse_gds(&gds).0;
            assert_eq!(p.len(), 1);
            assert_eq!(p[0].layer, 2);
        }
    }

    // ── Styling and meshes ──

    #[test]
    fn layer_style_fills_devices_and_outlines_wells_and_implants() {
        let names = layer_names(&[
            ("met1".into(), (68, 20)),
            ("nwell".into(), (64, 20)),
            ("nsdm".into(), (93, 44)),
            ("areaid_mk".into(), (81, 4)),
            ("psd".into(), (94, 20)),
        ]);
        assert!(layer_style(&names, (68, 20)).1);
        for k in [(64, 20), (93, 44), (81, 4), (94, 20)] {
            assert!(!layer_style(&names, k).1, "{k:?} is outline-only");
        }
        // Unknown: hashed, filled, deterministic.
        let s = layer_style(&names, (7, 3));
        assert!(s.1);
        assert_eq!(s, layer_style(&names, (7, 3)));
    }

    #[test]
    fn outline_color_is_brighter_and_opaque() {
        assert_eq!(outline_color([0.5, 0.8, 0.0, 0.25]), [0.75, 1.0, 0.0, 1.0]);
    }

    #[test]
    fn meshes_count_vertices_per_polygon() {
        let names = layer_names(&[("nwell".into(), (64, 20))]);
        let sq = Poly { layer: 1, datatype: 0, pts: vec![[0, 0], [10, 0], [10, 10], [0, 10]] };
        let well = Poly { layer: 64, datatype: 20, ..sq.clone() };
        assert_eq!(triangulate(&[sq.clone()], &names).len(), 6, "two triangles");
        assert!(triangulate(&[well.clone()], &names).is_empty(), "outline-only layers do not fill");
        let lines = outline_vertices(&[sq, well], &names);
        assert_eq!(lines.len(), 16, "four closed edges each");
        assert_eq!(lines[7].pos, [0.0, 0.0], "the last edge closes on the first vertex");
        assert!(triangulate(&[], &names).is_empty() && outline_vertices(&[], &names).is_empty());
    }

    #[test]
    fn glyphs_are_well_formed_stroke_pairs() {
        for ch in (' '..='~').chain(['é']) {
            let g = glyph(ch);
            assert_eq!(g.len() % 2, 0, "{ch:?}");
            for seg in g.chunks_exact(2) {
                let up = seg.iter().filter(|&&p| p == PEN_UP).count();
                assert!(up == 0 || up == 2, "{ch:?}: half a pen-up");
                if up == 0 {
                    assert!(seg.iter().all(|&(x, y)| x <= 5 && y <= 7), "{ch:?} leaves the cell");
                }
            }
        }
        assert!(glyph(' ').is_empty());
        assert_eq!(glyph('a'), glyph('A'));
    }

    #[test]
    fn push_text_strokes_each_segment_as_two_triangles() {
        let mut v = Vec::new();
        push_text(&mut v, " ", 0.0, 0.0, 1.0, [1.0; 4]);
        assert!(v.is_empty());
        push_text(&mut v, "I", 0.0, 0.0, 1.0, [1.0; 4]);
        assert_eq!(v.len(), 3 * 6, "I is three strokes");
        // The second glyph starts six units right.
        let mut w = Vec::new();
        push_text(&mut w, " I", 0.0, 0.0, 1.0, [1.0; 4]);
        assert!(w.iter().zip(&v).all(|(a, b)| (a.pos[0] - b.pos[0] - 6.0).abs() < 1e-6));
    }

    #[test]
    fn a_zero_length_stroke_has_no_nan() {
        let mut v = Vec::new();
        stroke_quad(&mut v, [1.0, 1.0], [1.0, 1.0], 0.5, [1.0; 4]);
        assert_eq!(v.len(), 6);
        assert!(v.iter().all(|x| x.pos.iter().all(|c| c.is_finite())));
    }

    #[test]
    fn legend_has_a_backdrop_and_one_row_per_layer() {
        let names = LayerMap::new();
        assert!(build_legend(&[], &names).is_empty());
        let p = |layer| Poly { layer, datatype: 0, pts: vec![[0, 0], [1, 0], [1, 1]] };
        let one = build_legend(&[p(1), p(1)], &names);
        let two = build_legend(&[p(1), p(2)], &names);
        assert!(two.len() > one.len(), "duplicate layers collapse to one row");
        assert!(one.iter().all(|v| v.pos.iter().all(|c| (-1.0..=1.0).contains(c))), "legend stays on screen");
    }

    #[test]
    fn text_vertices_scale_with_the_layout() {
        let t = [TextEntry { x: 0, y: 0, text: "I".into() }];
        let big = Poly { layer: 1, datatype: 0, pts: vec![[0, 0], [70_000, 0], [0, 70_000]] };
        let height = |v: &[Vertex]| v.iter().map(|x| x.pos[1]).fold(f32::MIN, f32::max);
        let default = height(&text_vertices(&t, &[]));
        let scaled = height(&text_vertices(&t, &[big]));
        assert!((scaled / default - 10.0).abs() < 0.1, "{default} {scaled}");
        assert!(text_vertices(&[], &[]).is_empty());
    }

    #[test]
    fn fit_view_centres_and_fills_the_view() {
        let id = fit_view(&[], 2.0);
        assert_eq!((id.offset, id.scale, id.aspect), ([0.0; 2], 1.0, 2.0));
        let p = Poly { layer: 1, datatype: 0, pts: vec![[100, 0], [300, 0], [300, 100]] };
        let c = fit_view(&[p], 1.0);
        assert_eq!(c.offset, [-200.0, -50.0]);
        assert!((c.scale * 200.0 - 1.8).abs() < 1e-6);
        // A single point: span floors at one unit.
        let dot = Poly { layer: 1, datatype: 0, pts: vec![[5, 5]] };
        assert!((fit_view(&[dot], 1.0).scale - 1.8).abs() < 1e-6);
    }

    #[test]
    fn extreme_coordinates_do_not_overflow() {
        let p = Poly { layer: 1, datatype: 0, pts: vec![[i32::MIN, i32::MIN], [i32::MAX, i32::MIN], [i32::MAX, i32::MAX]] };
        let c = fit_view(std::slice::from_ref(&p), 1.0);
        assert!(c.scale > 0.0 && c.scale.is_finite());
        assert!(c.offset.iter().all(|o| o.abs() <= 1.0), "{:?}", c.offset);
        assert!(!text_vertices(&[TextEntry { x: 0, y: 0, text: "A".into() }], &[p]).is_empty());
        let gds = [bgn("T"), boundary(1, 0, &[[i32::MIN, i32::MIN], [i32::MAX, i32::MIN], [i32::MAX, i32::MAX]]), end()].concat();
        assert!(export_svg(&gds, &LayerMap::new()).contains("<polygon"));
    }

    #[test]
    fn zoom_steps_by_ten_percent_and_ignores_no_movement() {
        assert!((zoom_factor(1.0) - 1.1).abs() < 1e-6);
        assert!((zoom_factor(-3.0) - 1.0 / 1.1).abs() < 1e-6);
        assert_eq!(zoom_factor(0.0), 1.0, "a horizontal scroll must not zoom");
        assert_eq!(zoom_factor(-0.0), 1.0);
    }

    // ── SVG ──

    #[test]
    fn svg_groups_layers_and_flips_labels_upright() {
        let text = [rec(0x0C, 0, &[]), s_rec(0x19, "a&b<c"), xy(&[[0, 100]]), rec(0x11, 0, &[])].concat();
        let gds = [bgn("T"), boundary(68, 20, &square(0, 0, 400)), boundary(64, 20, &square(0, 0, 800)), text, end()].concat();
        let names = layer_names(&[("met1".into(), (68, 20)), ("nwell".into(), (64, 20))]);
        let svg = export_svg(&gds, &names);
        assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
        // pad = 800 / 40 = 20.
        assert!(svg.contains(r#"viewBox="-20 -20 840 840""#), "{svg}");
        assert!(svg.contains(r#"<g id="met1" fill="rgba("#));
        assert!(svg.contains(r#"<g id="nwell" fill="none""#), "wells are outline-only");
        // flip = 2·vy + vh = 800; label y = 800 − 100.
        assert!(svg.contains(r#"<text x="0" y="700""#), "{svg}");
        assert!(svg.contains(">a&amp;b&lt;c</text>"));
        assert_eq!(svg.matches("<polygon").count(), 2);
    }

    #[test]
    fn svg_attributes_are_escaped_and_never_degenerate() {
        // A tiny layout still strokes, and a zero-width one keeps a viewBox.
        let gds = [bgn("T"), boundary(1, 0, &[[0, 0], [0, 10], [0, 20]]), end()].concat();
        let names = layer_names(&[("a\"b<".into(), (1, 0))]);
        let svg = export_svg(&gds, &names);
        assert!(!svg.contains(r#"stroke-width="0""#), "{svg}");
        assert!(svg.contains(r#"viewBox="-1 -1 2 22""#), "{svg}");
        assert!(svg.contains(r#"id="a&quot;b&lt;""#), "{svg}");
    }

    #[test]
    fn empty_gds_is_an_empty_svg() {
        assert_eq!(export_svg(&[], &LayerMap::new()), r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#);
    }
}
