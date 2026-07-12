//! GDSII reader — boundaries, paths, boxes, text labels, and full hierarchy
//! (SREF/AREF with reflection/magnification/rotation), flattened into per-cell
//! `GeometryStore`s.
//!
//! GDSII is a big-endian, record-based binary format. A record is:
//!   [u16 length][u8 rec_type][u8 data_type][payload...]
//!
//! Hierarchy is resolved leaf-up: cells are topologically sorted by reference,
//! each cell's flattened store is built once, then parents copy the child's
//! already-flat geometry through the instance transform. Circular or dangling
//! references are hard errors, not silent drops.

use crate::geometry::GeometryStore;
use crate::params::LayerTable;
use std::collections::HashMap;

const HEADER: u8 = 0x00;
const BGNLIB: u8 = 0x01;
const LIBNAME: u8 = 0x02;
const UNITS: u8 = 0x03;
const ENDLIB: u8 = 0x04;
const BGNSTR: u8 = 0x05;
const STRNAME: u8 = 0x06;
const ENDSTR: u8 = 0x07;
const BOUNDARY: u8 = 0x08;
const PATH: u8 = 0x09;
const SREF: u8 = 0x0A;
const AREF: u8 = 0x0B;
const TEXT: u8 = 0x0C;
const LAYER: u8 = 0x0D;
const DATATYPE: u8 = 0x0E;
const WIDTH: u8 = 0x0F;
const XY: u8 = 0x10;
const ENDEL: u8 = 0x11;
const SNAME: u8 = 0x12;
const COLROW: u8 = 0x13;
const NODE: u8 = 0x15;
const TEXTTYPE: u8 = 0x16;
const STRING: u8 = 0x19;
const STRANS: u8 = 0x1A;
const MAG: u8 = 0x1B;
const ANGLE: u8 = 0x1C;
const PATHTYPE: u8 = 0x21;
const BOX: u8 = 0x2D;
const BOXTYPE: u8 = 0x2E;
const BGNEXTN: u8 = 0x30;
const ENDEXTN: u8 = 0x31;

const DT_NONE: u8 = 0x00;
const DT_BIT_ARRAY: u8 = 0x01;
const DT_I16: u8 = 0x02;
const DT_I32: u8 = 0x03;
const DT_REAL8: u8 = 0x05;
const DT_ASCII: u8 = 0x06;

/// Unit metadata from the GDS `UNITS` record. Geometry coordinates remain the
/// exact signed database-unit integers stored in the file; the reader never
/// rescales or rounds them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GdsUnits {
    /// Size of one database unit expressed in the library's user unit.
    pub user_units_per_database_unit: f64,
    /// Size of one database unit in meters.
    pub meters_per_database_unit: f64,
}

impl GdsUnits {
    /// Size of one database unit in nanometers.
    pub fn database_unit_nm(self) -> f64 {
        self.meters_per_database_unit * 1.0e9
    }
}

/// Geometry records whose GDS `(layer, datatype)` pair was not present in the
/// supplied [`LayerTable`]. Entries are sorted by `(layer, datatype)` so callers
/// can deterministically reject, waive, or report them. TEXT records remain raw
/// metadata in each [`GeometryStore`] and are not counted here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GdsUnmappedLayer {
    pub layer: i32,
    pub datatype: i32,
    pub element_count: usize,
}

/// Parsed GDS: one flattened `GeometryStore` per cell (structure), keyed by name.
pub struct GdsLayout {
    pub cells: HashMap<String, GeometryStore>,
    /// Cells not referenced by any other cell (usually exactly one on real layouts).
    pub top_cells: Vec<String>,
    /// Parsed `UNITS` metadata. `None` preserves compatibility with legacy
    /// record streams that omit the otherwise-standard library record.
    pub units: Option<GdsUnits>,
    /// Counted BOUNDARY/BOX/PATH records on layer pairs absent from the supplied
    /// layer table. Unmapped geometry is never silently invisible to callers.
    pub unmapped_geometry: Vec<GdsUnmappedLayer>,
}

/// One SREF/AREF instance. SREF is an AREF with cols = rows = 1 and zero steps.
#[derive(Clone)]
struct RawRef {
    cell: String,
    origin: (i32, i32),
    mirror_x: bool, // STRANS bit 15: reflect about x-axis before rotation
    mag: f64,
    angle_deg: f64,
    cols: u32,
    rows: u32,
    col_step: (f64, f64), // per-column displacement
    row_step: (f64, f64), // per-row displacement
}

#[derive(Default)]
struct RawCell {
    store: GeometryStore,
    refs: Vec<RawRef>,
}

#[derive(PartialEq, Clone, Copy)]
enum El {
    None,
    Boundary,
    Path,
    Box,
    Text,
    Sref,
    Aref,
    Node,
}

pub fn read_gds(bytes: &[u8], lt: &LayerTable) -> Result<GdsLayout, String> {
    let mut raw: HashMap<String, RawCell> = HashMap::new();
    let mut pos = 0usize;
    let mut in_structure = false;
    let mut saw_endlib = false;
    let mut units = None;
    let mut unmapped_geometry: HashMap<(i32, i32), usize> = HashMap::new();

    let mut cur_name: Option<String> = None;
    let mut cur = RawCell::default();

    // element-in-progress state
    let mut el = El::None;
    let mut el_layer: Option<i32> = None;
    let mut el_datatype: Option<i32> = None;
    let mut el_xy: Vec<(i32, i32)> = Vec::new();
    let mut el_xy_seen = false;
    let mut el_string = String::new();
    let mut el_sname = String::new();
    let mut el_width: i32 = 0;
    let mut el_pathtype: i16 = 0;
    let mut el_bgnextn: i32 = 0;
    let mut el_endextn: i32 = 0;
    let mut el_mirror = false;
    let mut el_mag: f64 = 1.0;
    let mut el_angle: f64 = 0.0;
    let mut el_cols: u32 = 1;
    let mut el_rows: u32 = 1;
    let mut el_colrow_seen = false;

    while pos < bytes.len() {
        if saw_endlib {
            return Err(format!("GDS: data follows ENDLIB at byte offset {pos}"));
        }
        if bytes.len() - pos < 4 {
            return Err(format!(
                "GDS: truncated record header at byte offset {pos} ({} byte(s) remain)",
                bytes.len() - pos,
            ));
        }
        let len = u16::from_be_bytes([bytes[pos], bytes[pos + 1]]) as usize;
        if len < 4 {
            return Err(format!(
                "GDS: invalid record length {len} at byte offset {pos}; minimum is 4"
            ));
        }
        if len % 2 != 0 {
            return Err(format!(
                "GDS: invalid odd record length {len} at byte offset {pos}"
            ));
        }
        let end = pos
            .checked_add(len)
            .ok_or_else(|| format!("GDS: record length overflow at byte offset {pos}"))?;
        if end > bytes.len() {
            return Err(format!(
                "GDS: truncated record at byte offset {pos}: declares {len} bytes, only {} remain",
                bytes.len() - pos,
            ));
        }
        let rec = bytes[pos + 2];
        let data_type = bytes[pos + 3];
        let payload = &bytes[pos + 4..end];
        validate_record(rec, data_type, payload, pos)?;

        match rec {
            HEADER | BGNLIB | LIBNAME => {}
            UNITS => {
                if units.is_some() {
                    return Err("GDS: duplicate UNITS record".into());
                }
                let user_units_per_database_unit = read_real8(&payload[..8]);
                let meters_per_database_unit = read_real8(&payload[8..]);
                if !user_units_per_database_unit.is_finite()
                    || user_units_per_database_unit <= 0.0
                    || !meters_per_database_unit.is_finite()
                    || meters_per_database_unit <= 0.0
                {
                    return Err(format!(
                        "GDS: UNITS values must be finite and positive, got \
                         {user_units_per_database_unit} and {meters_per_database_unit}"
                    ));
                }
                units = Some(GdsUnits {
                    user_units_per_database_unit,
                    meters_per_database_unit,
                });
            }
            ENDLIB => {
                if in_structure {
                    return Err("GDS: ENDLIB encountered before ENDSTR".into());
                }
                saw_endlib = true;
            }
            BGNSTR => {
                if in_structure {
                    return Err("GDS: nested BGNSTR before previous ENDSTR".into());
                }
                in_structure = true;
                cur = RawCell::default();
                cur_name = None;
            }
            STRNAME => {
                if !in_structure {
                    return Err("GDS: STRNAME outside a structure".into());
                }
                if cur_name.is_some() {
                    return Err("GDS: structure has more than one STRNAME".into());
                }
                let name = read_ascii(payload)?;
                if name.is_empty() {
                    return Err("GDS: STRNAME must not be empty".into());
                }
                cur_name = Some(name);
            }
            BOUNDARY | PATH | BOX | TEXT | SREF | AREF | NODE => {
                if !in_structure {
                    return Err(format!(
                        "GDS: element record 0x{rec:02X} outside a structure"
                    ));
                }
                if el != El::None {
                    return Err("GDS: new element started before previous ENDEL".into());
                }
                el = match rec {
                    BOUNDARY => El::Boundary,
                    PATH => El::Path,
                    BOX => El::Box,
                    TEXT => El::Text,
                    SREF => El::Sref,
                    AREF => El::Aref,
                    _ => El::Node,
                };
                el_layer = None;
                el_datatype = None;
                el_xy.clear();
                el_xy_seen = false;
                el_string.clear();
                el_sname.clear();
                el_width = 0;
                el_pathtype = 0;
                el_bgnextn = 0;
                el_endextn = 0;
                el_mirror = false;
                el_mag = 1.0;
                el_angle = 0.0;
                el_cols = 1;
                el_rows = 1;
                el_colrow_seen = false;
            }
            LAYER => {
                if el == El::None {
                    return Err("GDS: LAYER outside an element".into());
                }
                el_layer = Some(read_i16(payload) as i32);
            }
            // BOXTYPE/TEXTTYPE play datatype's role for their element kinds
            DATATYPE | BOXTYPE | TEXTTYPE => {
                if el == El::None {
                    return Err("GDS: datatype record outside an element".into());
                }
                el_datatype = Some(read_i16(payload) as i32);
            }
            WIDTH => {
                if el != El::Path {
                    return Err("GDS: WIDTH outside a PATH element".into());
                }
                el_width = read_i32(payload);
            }
            PATHTYPE => {
                if el != El::Path {
                    return Err("GDS: PATHTYPE outside a PATH element".into());
                }
                el_pathtype = read_i16(payload);
            }
            BGNEXTN => {
                if el != El::Path {
                    return Err("GDS: BGNEXTN outside a PATH element".into());
                }
                el_bgnextn = read_i32(payload);
            }
            ENDEXTN => {
                if el != El::Path {
                    return Err("GDS: ENDEXTN outside a PATH element".into());
                }
                el_endextn = read_i32(payload);
            }
            SNAME => {
                if !matches!(el, El::Sref | El::Aref) {
                    return Err("GDS: SNAME outside an SREF/AREF element".into());
                }
                el_sname = read_ascii(payload)?;
            }
            COLROW => {
                if el != El::Aref {
                    return Err("GDS: COLROW outside an AREF element".into());
                }
                let cols = i16::from_be_bytes([payload[0], payload[1]]);
                let rows = i16::from_be_bytes([payload[2], payload[3]]);
                if cols <= 0 || rows <= 0 {
                    return Err(format!(
                        "GDS: AREF COLROW values must be positive, got {cols} x {rows}"
                    ));
                }
                el_cols = cols as u32;
                el_rows = rows as u32;
                el_colrow_seen = true;
            }
            STRANS => {
                if !matches!(el, El::Sref | El::Aref | El::Text) {
                    return Err("GDS: STRANS outside a reference/text element".into());
                }
                // bit 15 = reflect about x-axis. Absolute-mag/angle bits (1, 2) are
                // vanishingly rare in layout GDS; treated as relative.
                el_mirror = payload[0] & 0x80 != 0;
            }
            MAG => {
                if !matches!(el, El::Sref | El::Aref | El::Text) {
                    return Err("GDS: MAG outside a reference/text element".into());
                }
                el_mag = read_real8(payload);
                if !el_mag.is_finite() || el_mag <= 0.0 {
                    return Err(format!(
                        "GDS: MAG must be finite and positive, got {el_mag}"
                    ));
                }
            }
            ANGLE => {
                if !matches!(el, El::Sref | El::Aref | El::Text) {
                    return Err("GDS: ANGLE outside a reference/text element".into());
                }
                el_angle = read_real8(payload);
                if !el_angle.is_finite() {
                    return Err(format!("GDS: ANGLE must be finite, got {el_angle}"));
                }
            }
            STRING => {
                if el != El::Text {
                    return Err("GDS: STRING outside a TEXT element".into());
                }
                el_string = read_ascii(payload)?;
            }
            XY => {
                if el == El::None {
                    return Err("GDS: XY outside an element".into());
                }
                if el_xy_seen {
                    return Err("GDS: element has more than one XY record".into());
                }
                el_xy.clear();
                let mut i = 0;
                while i + 8 <= payload.len() {
                    let x = i32::from_be_bytes([
                        payload[i],
                        payload[i + 1],
                        payload[i + 2],
                        payload[i + 3],
                    ]);
                    let y = i32::from_be_bytes([
                        payload[i + 4],
                        payload[i + 5],
                        payload[i + 6],
                        payload[i + 7],
                    ]);
                    el_xy.push((x, y));
                    i += 8;
                }
                el_xy_seen = true;
            }
            ENDEL => {
                match el {
                    El::Boundary | El::Box => {
                        // GDS repeats the first point at the end; drop it.
                        let mut pts = std::mem::take(&mut el_xy);
                        if pts.len() < 4 {
                            return Err("GDS: BOUNDARY/BOX needs at least 4 XY points".into());
                        }
                        if pts.first() != pts.last() {
                            return Err("GDS: BOUNDARY/BOX XY loop is not closed".into());
                        }
                        pts.pop();
                        let layer = el_layer.ok_or("GDS: BOUNDARY/BOX missing LAYER")?;
                        let datatype =
                            el_datatype.ok_or("GDS: BOUNDARY/BOX missing DATATYPE/BOXTYPE")?;
                        if let Some(lid) = lt.from_gds(layer, datatype) {
                            cur.store.add_polygon(lid, &pts);
                        } else {
                            *unmapped_geometry.entry((layer, datatype)).or_default() += 1;
                        }
                    }
                    El::Path => {
                        if el_xy.len() < 2 {
                            return Err("GDS: PATH needs at least 2 XY points".into());
                        }
                        let layer = el_layer.ok_or("GDS: PATH missing LAYER")?;
                        let datatype = el_datatype.ok_or("GDS: PATH missing DATATYPE")?;
                        let width = el_width
                            .checked_abs()
                            .ok_or("GDS: PATH WIDTH cannot be i32::MIN")?;
                        if width == 0 {
                            return Err(
                                "GDS: zero-width PATH cannot be represented as area geometry"
                                    .into(),
                            );
                        }
                        if !matches!(el_pathtype, 0 | 1 | 2 | 4) {
                            return Err(format!("GDS: unsupported PATHTYPE {el_pathtype}"));
                        }
                        if let Some(lid) = lt.from_gds(layer, datatype) {
                            path_to_polys(
                                &el_xy,
                                width,
                                el_pathtype,
                                el_bgnextn,
                                el_endextn,
                                lid,
                                &mut cur.store,
                            );
                        } else {
                            *unmapped_geometry.entry((layer, datatype)).or_default() += 1;
                        }
                    }
                    El::Text => {
                        if !el_string.is_empty() && !el_xy.is_empty() {
                            let layer = el_layer.ok_or("GDS: TEXT missing LAYER")?;
                            let datatype = el_datatype.ok_or("GDS: TEXT missing TEXTTYPE")?;
                            let (x, y) = el_xy[0];
                            cur.store.add_text(
                                layer,
                                datatype,
                                x,
                                y,
                                std::mem::take(&mut el_string),
                            );
                        }
                    }
                    El::Sref | El::Aref => {
                        if el_sname.is_empty() || el_xy.is_empty() {
                            return Err("GDS: SREF/AREF missing SNAME or XY".into());
                        }
                        let origin = el_xy[0];
                        let (mut col_step, mut row_step) = ((0.0, 0.0), (0.0, 0.0));
                        let (cols, rows) = if el == El::Aref {
                            if !el_colrow_seen {
                                return Err("GDS: AREF missing COLROW".into());
                            }
                            if el_xy.len() != 3 {
                                return Err("GDS: AREF needs exactly 3 XY points".into());
                            }
                            // point 2 = origin + cols·col_spacing, point 3 = origin + rows·row_spacing
                            col_step = (
                                (el_xy[1].0 - origin.0) as f64 / el_cols as f64,
                                (el_xy[1].1 - origin.1) as f64 / el_cols as f64,
                            );
                            row_step = (
                                (el_xy[2].0 - origin.0) as f64 / el_rows as f64,
                                (el_xy[2].1 - origin.1) as f64 / el_rows as f64,
                            );
                            (el_cols, el_rows)
                        } else {
                            if el_xy.len() != 1 {
                                return Err("GDS: SREF needs exactly 1 XY point".into());
                            }
                            (1, 1)
                        };
                        cur.refs.push(RawRef {
                            cell: std::mem::take(&mut el_sname),
                            origin,
                            mirror_x: el_mirror,
                            mag: el_mag,
                            angle_deg: el_angle,
                            cols,
                            rows,
                            col_step,
                            row_step,
                        });
                    }
                    // NODE carries no geometry the checkers consume.
                    El::Node => {}
                    El::None => return Err("GDS: ENDEL without an active element".into()),
                }
                el = El::None;
            }
            ENDSTR => {
                if !in_structure {
                    return Err("GDS: ENDSTR without BGNSTR".into());
                }
                if el != El::None {
                    return Err("GDS: ENDSTR encountered before element ENDEL".into());
                }
                let name = cur_name.take().ok_or("GDS: structure missing STRNAME")?;
                if raw.contains_key(&name) {
                    return Err(format!("GDS: duplicate structure name `{name}`"));
                }
                raw.insert(name, std::mem::take(&mut cur));
                in_structure = false;
            }
            _ => {}
        }
        pos = end;
    }

    if in_structure {
        let name = cur_name.as_deref().unwrap_or("<unnamed>");
        return Err(format!("GDS: structure `{name}` is missing ENDSTR"));
    }

    flatten(raw, units, unmapped_geometry)
}

// ---------------------------------------------------------------------------
// Hierarchy flattening
// ---------------------------------------------------------------------------

/// 2x2 linear map + translation. Column vectors: p' = M·p + d.
#[derive(Clone, Copy)]
struct Trans {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    dx: f64,
    dy: f64,
}

impl Trans {
    /// GDS instance transform order: reflect about x-axis, rotate CCW, magnify, translate.
    fn instance(mirror_x: bool, mag: f64, angle_deg: f64, dx: f64, dy: f64) -> Trans {
        let (s, co) = if angle_deg == 0.0 {
            (0.0, 1.0)
        } else {
            angle_deg.to_radians().sin_cos()
        };
        let fy = if mirror_x { -1.0 } else { 1.0 };
        Trans {
            a: mag * co,
            b: mag * -s * fy,
            c: mag * s,
            d: mag * co * fy,
            dx,
            dy,
        }
    }
    #[inline]
    fn apply(&self, x: i32, y: i32) -> (i32, i32) {
        let (xf, yf) = (f64::from(x), f64::from(y));
        (
            (self.a * xf + self.b * yf + self.dx).round() as i32,
            (self.c * xf + self.d * yf + self.dy).round() as i32,
        )
    }
    /// Does the map reverse polygon orientation? (mirror, or negative mag)
    #[inline]
    fn flips(&self) -> bool {
        self.a * self.d - self.b * self.c < 0.0
    }
}

fn flatten(
    raw: HashMap<String, RawCell>,
    units: Option<GdsUnits>,
    unmapped_geometry_counts: HashMap<(i32, i32), usize>,
) -> Result<GdsLayout, String> {
    // children-before-parents order, with cycle + dangling-ref detection
    let order = topo_order(&raw)?;

    let referenced: std::collections::HashSet<&str> = raw
        .values()
        .flat_map(|c| c.refs.iter().map(|r| r.cell.as_str()))
        .collect();
    let mut top_cells: Vec<String> = raw
        .keys()
        .filter(|n| !referenced.contains(n.as_str()))
        .cloned()
        .collect();
    top_cells.sort();

    let mut flat: HashMap<String, GeometryStore> = HashMap::new();
    let mut pts_buf: Vec<(i32, i32)> = Vec::new();
    for name in order {
        let rc = &raw[&name];
        let mut store = rc.store.clone();
        for r in &rc.refs {
            // topo order guarantees the child is already flattened
            let child = &flat[&r.cell];
            for ci in 0..r.cols {
                for ri in 0..r.rows {
                    let dx = f64::from(r.origin.0)
                        + f64::from(ci) * r.col_step.0
                        + f64::from(ri) * r.row_step.0;
                    let dy = f64::from(r.origin.1)
                        + f64::from(ci) * r.col_step.1
                        + f64::from(ri) * r.row_step.1;
                    let t = Trans::instance(r.mirror_x, r.mag, r.angle_deg, dx, dy);
                    append_transformed(&mut store, child, &t, &mut pts_buf);
                }
            }
        }
        flat.insert(name, store);
    }
    let mut unmapped_geometry: Vec<GdsUnmappedLayer> = unmapped_geometry_counts
        .into_iter()
        .map(|((layer, datatype), element_count)| GdsUnmappedLayer {
            layer,
            datatype,
            element_count,
        })
        .collect();
    unmapped_geometry.sort_by_key(|entry| (entry.layer, entry.datatype));
    Ok(GdsLayout {
        cells: flat,
        top_cells,
        units,
        unmapped_geometry,
    })
}

fn append_transformed(
    dst: &mut GeometryStore,
    src: &GeometryStore,
    t: &Trans,
    pts: &mut Vec<(i32, i32)>,
) {
    let flip = t.flips();
    for p in 0..src.poly_count() {
        let s = src.poly_vert_start[p] as usize;
        let n = src.poly_vert_len[p] as usize;
        pts.clear();
        pts.extend((0..n).map(|i| t.apply(src.verts_x[s + i], src.verts_y[s + i])));
        if flip {
            pts.reverse(); // keep winding consistent for area/inside tests
        }
        dst.add_polygon(src.poly_layer[p], pts);
    }
    for i in 0..src.text_count() {
        let (x, y) = t.apply(src.text_x[i], src.text_y[i]);
        dst.add_text(
            src.text_layer[i],
            src.text_datatype[i],
            x,
            y,
            src.text_string[i].clone(),
        );
    }
}

/// DFS topological sort of the cell-reference DAG; errors on cycles and
/// references to cells the file never defines.
fn topo_order(raw: &HashMap<String, RawCell>) -> Result<Vec<String>, String> {
    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        White,
        Gray,
        Black,
    }
    let mut mark: HashMap<&str, Mark> = raw.keys().map(|k| (k.as_str(), Mark::White)).collect();
    let mut order = Vec::with_capacity(raw.len());
    // iterative DFS: (cell, next-ref-index)
    for start in raw.keys() {
        if mark[start.as_str()] != Mark::White {
            continue;
        }
        let mut stack: Vec<(&str, usize)> = vec![(start, 0)];
        mark.insert(start, Mark::Gray);
        while let Some(&mut (cell, ref mut idx)) = stack.last_mut() {
            let refs = &raw[cell].refs;
            if *idx < refs.len() {
                let child = refs[*idx].cell.as_str();
                *idx += 1;
                match mark.get(child) {
                    None => return Err(format!("GDS: reference to undefined cell '{child}'")),
                    Some(Mark::Gray) => {
                        return Err(format!("GDS: circular cell reference through '{child}'"));
                    }
                    Some(Mark::Black) => {}
                    Some(Mark::White) => {
                        // borrow of `raw` key outlives the loop; fetch owned key ref
                        let (k, _) = raw.get_key_value(child).unwrap();
                        mark.insert(k, Mark::Gray);
                        stack.push((k, 0));
                    }
                }
            } else {
                mark.insert(cell, Mark::Black);
                order.push(cell.to_string());
                stack.pop();
            }
        }
    }
    Ok(order)
}

// ---------------------------------------------------------------------------
// PATH expansion
// ---------------------------------------------------------------------------

/// Expand a PATH centerline into one quad per segment. Interior joints are
/// squared off by extending both adjacent segments half a width — exact for
/// 90° bends; for other angles the squares overlap and merged-group semantics
/// absorb the seam. Endcaps follow PATHTYPE: 0 flush, 1 round (approximated
/// square — max error (√2−1)·w/2 at the cap), 2 square, 4 custom extensions.
fn path_to_polys(
    pts: &[(i32, i32)],
    width: i32,
    pathtype: i16,
    bgnextn: i32,
    endextn: i32,
    lid: crate::geometry::LayerId,
    store: &mut GeometryStore,
) {
    if pts.len() < 2 || width <= 0 {
        return;
    }
    let half = f64::from(width) / 2.0;
    let cap = |custom: i32| -> f64 {
        match pathtype {
            0 => 0.0,
            4 => f64::from(custom),
            _ => half, // 1 (round → square approx) and 2 (square)
        }
    };
    let last_seg = pts.len() - 2;
    for i in 0..=last_seg {
        let (x0, y0) = (f64::from(pts[i].0), f64::from(pts[i].1));
        let (x1, y1) = (f64::from(pts[i + 1].0), f64::from(pts[i + 1].1));
        let (dx, dy) = (x1 - x0, y1 - y0);
        let len = (dx * dx + dy * dy).sqrt();
        if len == 0.0 {
            continue;
        }
        let (ux, uy) = (dx / len, dy / len); // along
        let (px, py) = (-uy, ux); // perpendicular
        let ext_s = if i == 0 { cap(bgnextn) } else { half }; // joint fill
        let ext_e = if i == last_seg { cap(endextn) } else { half };
        let (sx, sy) = (x0 - ux * ext_s, y0 - uy * ext_s);
        let (ex, ey) = (x1 + ux * ext_e, y1 + uy * ext_e);
        let quad = [
            (
                (sx + px * half).round() as i32,
                (sy + py * half).round() as i32,
            ),
            (
                (ex + px * half).round() as i32,
                (ey + py * half).round() as i32,
            ),
            (
                (ex - px * half).round() as i32,
                (ey - py * half).round() as i32,
            ),
            (
                (sx - px * half).round() as i32,
                (sy - py * half).round() as i32,
            ),
        ];
        store.add_polygon(lid, &quad);
    }
}

// ---------------------------------------------------------------------------
// Primitive readers
// ---------------------------------------------------------------------------

fn validate_record(record: u8, data_type: u8, payload: &[u8], offset: usize) -> Result<(), String> {
    let exact = |expected_type: u8, expected_len: usize| -> Result<(), String> {
        if data_type != expected_type {
            return Err(format!(
                "GDS: record 0x{record:02X} at byte offset {offset} has data type \
                 0x{data_type:02X}; expected 0x{expected_type:02X}"
            ));
        }
        if payload.len() != expected_len {
            return Err(format!(
                "GDS: record 0x{record:02X} at byte offset {offset} has {} payload bytes; \
                 expected {expected_len}",
                payload.len(),
            ));
        }
        Ok(())
    };
    let typed_multiple =
        |expected_type: u8, unit: usize, description: &str| -> Result<(), String> {
            if data_type != expected_type {
                return Err(format!(
                    "GDS: record 0x{record:02X} at byte offset {offset} has data type \
                     0x{data_type:02X}; expected 0x{expected_type:02X}"
                ));
            }
            if payload.len() % unit != 0 {
                return Err(format!(
                    "GDS: record 0x{record:02X} at byte offset {offset} payload length {} \
                     is not a multiple of {unit} ({description})",
                    payload.len(),
                ));
            }
            Ok(())
        };

    match record {
        HEADER => exact(DT_I16, 2),
        BGNLIB | BGNSTR => exact(DT_I16, 24),
        LIBNAME | STRNAME | SNAME | STRING => typed_multiple(DT_ASCII, 2, "ASCII word"),
        UNITS => exact(DT_REAL8, 16),
        ENDLIB | ENDSTR | BOUNDARY | PATH | SREF | AREF | TEXT | ENDEL | NODE | BOX => {
            exact(DT_NONE, 0)
        }
        LAYER | DATATYPE | PATHTYPE | TEXTTYPE | BOXTYPE => exact(DT_I16, 2),
        WIDTH | BGNEXTN | ENDEXTN => exact(DT_I32, 4),
        XY => typed_multiple(DT_I32, 8, "XY coordinate pair"),
        COLROW => exact(DT_I16, 4),
        STRANS => exact(DT_BIT_ARRAY, 2),
        MAG | ANGLE => exact(DT_REAL8, 8),
        // Forward compatibility: records this reader does not consume still
        // receive the universal length/bounds checks in `read_gds`.
        _ => Ok(()),
    }
}

fn read_i16(payload: &[u8]) -> i16 {
    i16::from_be_bytes([payload[0], payload[1]])
}

fn read_i32(payload: &[u8]) -> i32 {
    i32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]])
}

/// GDS REAL8: sign bit, 7-bit excess-64 base-16 exponent, 56-bit mantissa fraction.
fn read_real8(b: &[u8]) -> f64 {
    if b.len() < 8 {
        return 0.0;
    }
    let sign = if b[0] & 0x80 != 0 { -1.0 } else { 1.0 };
    let exp = i32::from(b[0] & 0x7F) - 64;
    let mut mant: u64 = 0;
    for &byte in &b[1..8] {
        mant = (mant << 8) | u64::from(byte);
    }
    if mant == 0 {
        return 0.0;
    }
    sign * (mant as f64 / f64::from(2u32).powi(56)) * 16f64.powi(exp)
}

fn read_ascii(payload: &[u8]) -> Result<String, String> {
    let end = payload
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(payload.len());
    let bytes = &payload[..end];
    if !bytes.is_ascii() {
        return Err("GDS: ASCII record contains a non-ASCII byte".into());
    }
    Ok(std::str::from_utf8(bytes)
        .expect("ASCII is valid UTF-8")
        .trim()
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{LayerDef, LayerTable};

    fn record(out: &mut Vec<u8>, record: u8, data_type: u8, payload: &[u8]) {
        let len = u16::try_from(payload.len() + 4).unwrap();
        out.extend_from_slice(&len.to_be_bytes());
        out.push(record);
        out.push(data_type);
        out.extend_from_slice(payload);
    }

    fn no_data(out: &mut Vec<u8>, record_type: u8) {
        record(out, record_type, DT_NONE, &[]);
    }

    fn i16_record(out: &mut Vec<u8>, record_type: u8, values: &[i16]) {
        let payload: Vec<u8> = values
            .iter()
            .flat_map(|value| value.to_be_bytes())
            .collect();
        record(out, record_type, DT_I16, &payload);
    }

    fn i32_record(out: &mut Vec<u8>, record_type: u8, values: &[i32]) {
        let payload: Vec<u8> = values
            .iter()
            .flat_map(|value| value.to_be_bytes())
            .collect();
        record(out, record_type, DT_I32, &payload);
    }

    fn string_record(out: &mut Vec<u8>, record_type: u8, value: &str) {
        let mut payload = value.as_bytes().to_vec();
        if payload.len() % 2 != 0 {
            payload.push(0);
        }
        record(out, record_type, DT_ASCII, &payload);
    }

    fn gds_real(value: f64) -> [u8; 8] {
        if value == 0.0 {
            return [0; 8];
        }
        let (mut mantissa, mut exponent) = (value.abs(), 64i32);
        while mantissa >= 1.0 {
            mantissa /= 16.0;
            exponent += 1;
        }
        while mantissa < 1.0 / 16.0 {
            mantissa *= 16.0;
            exponent -= 1;
        }
        let bits = (mantissa * 2f64.powi(56)) as u64;
        let mut bytes = bits.to_be_bytes();
        bytes[0] = (if value < 0.0 { 0x80 } else { 0 }) | exponent as u8;
        bytes
    }

    fn begin_structure(out: &mut Vec<u8>, name: &str) {
        i16_record(out, BGNSTR, &[0; 12]);
        string_record(out, STRNAME, name);
    }

    fn rectangle(out: &mut Vec<u8>, layer: i16, datatype: i16) {
        no_data(out, BOUNDARY);
        i16_record(out, LAYER, &[layer]);
        i16_record(out, DATATYPE, &[datatype]);
        i32_record(out, XY, &[0, 0, 100, 0, 100, 100, 0, 100, 0, 0]);
        no_data(out, ENDEL);
    }

    fn layer_table() -> LayerTable {
        let defs = [(
            "met1".into(),
            LayerDef {
                layer: 7,
                datatype: 0,
            },
        )]
        .into_iter()
        .collect();
        LayerTable::from_defs(&defs)
    }

    #[test]
    fn rejects_truncated_and_invalid_records() {
        let table = layer_table();
        let truncated = [0, 8, HEADER, DT_I16, 0, 6];
        let error = read_gds(&truncated, &table)
            .err()
            .expect("declared record extending past EOF must fail");
        assert!(error.contains("truncated record"), "{error}");

        let invalid_length = [0, 2, HEADER, DT_I16];
        let error = read_gds(&invalid_length, &table)
            .err()
            .expect("record shorter than header must fail");
        assert!(error.contains("invalid record length"), "{error}");

        let mut wrong_type = Vec::new();
        record(&mut wrong_type, HEADER, DT_I32, &[0, 6]);
        let error = read_gds(&wrong_type, &table)
            .err()
            .expect("wrong record data type must fail");
        assert!(error.contains("data type"), "{error}");

        let trailing_header = [0u8; 3];
        let error = read_gds(&trailing_header, &table)
            .err()
            .expect("partial header must fail");
        assert!(error.contains("truncated record header"), "{error}");
    }

    #[test]
    fn rejects_duplicate_structures_and_missing_endstr() {
        let table = layer_table();
        let mut duplicate = Vec::new();
        begin_structure(&mut duplicate, "top");
        no_data(&mut duplicate, ENDSTR);
        begin_structure(&mut duplicate, "top");
        no_data(&mut duplicate, ENDSTR);
        let error = read_gds(&duplicate, &table)
            .err()
            .expect("duplicate structure name must fail");
        assert!(error.contains("duplicate structure name `top`"), "{error}");

        let mut missing = Vec::new();
        begin_structure(&mut missing, "unfinished");
        let error = read_gds(&missing, &table)
            .err()
            .expect("missing ENDSTR must fail");
        assert!(error.contains("missing ENDSTR"), "{error}");
    }

    #[test]
    fn exposes_units_without_rescaling_geometry() {
        let table = layer_table();
        let mut bytes = Vec::new();
        let mut unit_payload = Vec::new();
        unit_payload.extend_from_slice(&gds_real(1.0e-3));
        unit_payload.extend_from_slice(&gds_real(1.0e-9));
        record(&mut bytes, UNITS, DT_REAL8, &unit_payload);
        begin_structure(&mut bytes, "top");
        no_data(&mut bytes, BOUNDARY);
        i16_record(&mut bytes, LAYER, &[7]);
        i16_record(&mut bytes, DATATYPE, &[0]);
        i32_record(
            &mut bytes,
            XY,
            &[100, 200, 1100, 200, 1100, 700, 100, 700, 100, 200],
        );
        no_data(&mut bytes, ENDEL);
        no_data(&mut bytes, ENDSTR);

        let layout = read_gds(&bytes, &table).expect("valid layout");
        let units = layout.units.expect("UNITS metadata");
        assert!((units.user_units_per_database_unit - 1.0e-3).abs() < 1.0e-15);
        assert!((units.meters_per_database_unit - 1.0e-9).abs() < 1.0e-21);
        assert!((units.database_unit_nm() - 1.0).abs() < 1.0e-12);

        let bbox = layout.cells["top"].poly_bbox[0];
        assert_eq!(
            (bbox.xmin, bbox.ymin, bbox.xmax, bbox.ymax),
            (100, 200, 1100, 700)
        );
    }

    #[test]
    fn reports_unmapped_geometry_by_layer_pair() {
        let table = layer_table();
        let mut bytes = Vec::new();
        begin_structure(&mut bytes, "top");
        rectangle(&mut bytes, 99, 7);
        rectangle(&mut bytes, 99, 7);
        rectangle(&mut bytes, 98, 0);
        no_data(&mut bytes, ENDSTR);

        let layout = read_gds(&bytes, &table).expect("well-formed unmapped geometry");
        assert_eq!(layout.cells["top"].poly_count(), 0);
        assert_eq!(
            layout.unmapped_geometry,
            [
                GdsUnmappedLayer {
                    layer: 98,
                    datatype: 0,
                    element_count: 1
                },
                GdsUnmappedLayer {
                    layer: 99,
                    datatype: 7,
                    element_count: 2
                },
            ]
        );
    }
}
