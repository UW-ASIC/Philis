use serde_json::{json, Value};

// GDS record types
const REC_HEADER: u8 = 0x00;
const REC_BGNLIB: u8 = 0x01;
const REC_LIBNAME: u8 = 0x02;
const REC_UNITS: u8 = 0x03;
const REC_ENDLIB: u8 = 0x04;
const REC_BGNSTR: u8 = 0x05;
const REC_STRNAME: u8 = 0x06;
const REC_ENDSTR: u8 = 0x07;
const REC_BOUNDARY: u8 = 0x08;
const REC_PATH: u8 = 0x09;
const REC_SREF: u8 = 0x0A;
const REC_AREF: u8 = 0x0B;
const REC_LAYER: u8 = 0x0D;
const REC_DATATYPE: u8 = 0x0E;
const REC_WIDTH: u8 = 0x0F;
const REC_XY: u8 = 0x10;
const REC_ENDEL: u8 = 0x11;
const REC_SNAME: u8 = 0x12;
const REC_COLROW: u8 = 0x13;
const REC_STRANS: u8 = 0x1A;
const REC_MAG: u8 = 0x1B;
const REC_ANGLE: u8 = 0x1C;
const REC_PATHTYPE: u8 = 0x21;

// GDS data types
const DT_NONE: u8 = 0x00;
const DT_INT16: u8 = 0x02;
const DT_INT32: u8 = 0x03;
const DT_REAL8: u8 = 0x05;
const DT_ASCII: u8 = 0x06;

// Layer GDS numbers: (layer, datatype)
pub const NWELL: (i32, i32) = (1, 0);
pub const DIFF: (i32, i32) = (2, 0);
pub const POLY: (i32, i32) = (3, 0);
pub const LICON: (i32, i32) = (4, 0);
pub const LI: (i32, i32) = (5, 0);
pub const MCON: (i32, i32) = (6, 0);
pub const MET1: (i32, i32) = (7, 0);
pub const VIA1: (i32, i32) = (8, 0);
pub const MET2: (i32, i32) = (9, 0);
pub const NSDM: (i32, i32) = (10, 0);
pub const PSDM: (i32, i32) = (11, 0);
pub const LVT: (i32, i32) = (12, 0);
pub const HVT: (i32, i32) = (13, 0);
pub const DIODE: (i32, i32) = (14, 0);

// ---- GDS binary writer ----

pub struct GdsWriter {
    buf: Vec<u8>,
}

impl GdsWriter {
    pub fn new() -> Self {
        Self { buf: Vec::with_capacity(64 * 1024) }
    }

    fn record(&mut self, rec_type: u8, data_type: u8, payload: &[u8]) {
        let len = (4 + payload.len()) as u16;
        self.buf.extend_from_slice(&len.to_be_bytes());
        self.buf.push(rec_type);
        self.buf.push(data_type);
        self.buf.extend_from_slice(payload);
    }

    fn int16_record(&mut self, rec_type: u8, vals: &[i16]) {
        let mut payload = Vec::with_capacity(vals.len() * 2);
        for &v in vals {
            payload.extend_from_slice(&v.to_be_bytes());
        }
        self.record(rec_type, DT_INT16, &payload);
    }

    fn int32_record(&mut self, rec_type: u8, vals: &[i32]) {
        let mut payload = Vec::with_capacity(vals.len() * 4);
        for &v in vals {
            payload.extend_from_slice(&v.to_be_bytes());
        }
        self.record(rec_type, DT_INT32, &payload);
    }

    fn ascii_record(&mut self, rec_type: u8, s: &str) {
        let mut payload = s.as_bytes().to_vec();
        if payload.len() % 2 != 0 {
            payload.push(0);
        }
        self.record(rec_type, DT_ASCII, &payload);
    }

    fn real8_record(&mut self, rec_type: u8, vals: &[f64]) {
        let mut payload = Vec::with_capacity(vals.len() * 8);
        for &v in vals {
            payload.extend_from_slice(&f64_to_gds_real8(v));
        }
        self.record(rec_type, DT_REAL8, &payload);
    }

    fn no_data(&mut self, rec_type: u8) {
        self.record(rec_type, DT_NONE, &[]);
    }

    pub fn begin_lib(&mut self, name: &str) {
        self.int16_record(REC_HEADER, &[600]);
        self.int16_record(REC_BGNLIB, &[0i16; 12]);
        self.ascii_record(REC_LIBNAME, name);
        // units: 1e-3 user units (um), 1e-9 meters (nm database unit)
        self.real8_record(REC_UNITS, &[1e-3, 1e-9]);
    }

    pub fn end_lib(&mut self) {
        self.no_data(REC_ENDLIB);
    }

    pub fn begin_cell(&mut self, name: &str) {
        self.int16_record(REC_BGNSTR, &[0i16; 12]);
        self.ascii_record(REC_STRNAME, name);
    }

    pub fn end_cell(&mut self) {
        self.no_data(REC_ENDSTR);
    }

    /// Write a boundary element (polygon). Points should NOT repeat the first vertex.
    pub fn boundary(&mut self, layer: i32, datatype: i32, pts: &[(i32, i32)]) {
        self.no_data(REC_BOUNDARY);
        self.int16_record(REC_LAYER, &[layer as i16]);
        self.int16_record(REC_DATATYPE, &[datatype as i16]);
        // GDS XY repeats the first point at the end
        let mut xy = Vec::with_capacity((pts.len() + 1) * 2);
        for &(x, y) in pts {
            xy.push(x);
            xy.push(y);
        }
        xy.push(pts[0].0);
        xy.push(pts[0].1);
        self.int32_record(REC_XY, &xy);
        self.no_data(REC_ENDEL);
    }

    /// Convenience: axis-aligned rectangle at (x, y) with size (w, h).
    pub fn rect(&mut self, layer: (i32, i32), x: i32, y: i32, w: i32, h: i32) {
        self.boundary(layer.0, layer.1, &[
            (x, y), (x + w, y), (x + w, y + h), (x, y + h),
        ]);
    }

    /// Write a path element. Points are the centerline; pathtype 0=flush, 2=square.
    pub fn path(&mut self, layer: (i32, i32), width: i32, pathtype: i16, pts: &[(i32, i32)]) {
        self.no_data(REC_PATH);
        self.int16_record(REC_LAYER, &[layer.0 as i16]);
        self.int16_record(REC_DATATYPE, &[layer.1 as i16]);
        self.int16_record(REC_PATHTYPE, &[pathtype]);
        self.int32_record(REC_WIDTH, &[width]);
        let mut xy = Vec::with_capacity(pts.len() * 2);
        for &(x, y) in pts {
            xy.push(x);
            xy.push(y);
        }
        self.int32_record(REC_XY, &xy);
        self.no_data(REC_ENDEL);
    }

    fn strans(&mut self, mirror_x: bool, mag: f64, angle_deg: f64) {
        if mirror_x || mag != 1.0 || angle_deg != 0.0 {
            let bits: u16 = if mirror_x { 0x8000 } else { 0 };
            self.record(REC_STRANS, 0x01, &bits.to_be_bytes());
            if mag != 1.0 { self.real8_record(REC_MAG, &[mag]); }
            if angle_deg != 0.0 { self.real8_record(REC_ANGLE, &[angle_deg]); }
        }
    }

    /// Instance reference with optional mirror (about x-axis, pre-rotation),
    /// magnification, and CCW rotation in degrees.
    pub fn sref(&mut self, cell: &str, x: i32, y: i32, mirror_x: bool, mag: f64, angle_deg: f64) {
        self.no_data(REC_SREF);
        self.ascii_record(REC_SNAME, cell);
        self.strans(mirror_x, mag, angle_deg);
        self.int32_record(REC_XY, &[x, y]);
        self.no_data(REC_ENDEL);
    }

    /// Array reference: cols × rows grid at `origin` with per-step pitches.
    pub fn aref(
        &mut self, cell: &str, origin: (i32, i32), cols: i16, rows: i16,
        col_pitch: (i32, i32), row_pitch: (i32, i32),
    ) {
        self.no_data(REC_AREF);
        self.ascii_record(REC_SNAME, cell);
        self.int16_record(REC_COLROW, &[cols, rows]);
        // XY: origin, origin + cols·col_pitch, origin + rows·row_pitch
        self.int32_record(REC_XY, &[
            origin.0, origin.1,
            origin.0 + col_pitch.0 * cols as i32, origin.1 + col_pitch.1 * cols as i32,
            origin.0 + row_pitch.0 * rows as i32, origin.1 + row_pitch.1 * rows as i32,
        ]);
        self.no_data(REC_ENDEL);
    }

    pub fn finish(&self) -> Vec<u8> {
        self.buf.clone()
    }
}

fn f64_to_gds_real8(val: f64) -> [u8; 8] {
    if val == 0.0 {
        return [0u8; 8];
    }
    let negative = val < 0.0;
    let mut v = val.abs();
    let mut exp: i32 = 64;
    while v >= 1.0 {
        v /= 16.0;
        exp += 1;
    }
    while v < 1.0 / 16.0 {
        v *= 16.0;
        exp -= 1;
    }
    let mantissa = (v * (1u64 << 56) as f64) as u64;
    let mut result = mantissa.to_be_bytes();
    // mantissa < 2^56, so MSB of u64 big-endian is 0 — safe to overwrite with exponent
    result[0] = exp as u8;
    if negative {
        result[0] |= 0x80;
    }
    result
}

// ---- suite builder ----

pub struct Suite {
    pub gds: GdsWriter,
    pub drc_cases: Vec<Value>,
    pub lvs_cases: Vec<Value>,
    pub pex_cases: Vec<Value>,
    pub erc_cases: Vec<Value>,
}

impl Suite {
    pub fn new() -> Self {
        let mut gds = GdsWriter::new();
        gds.begin_lib("conformance");
        Suite {
            gds, drc_cases: Vec::new(), lvs_cases: Vec::new(),
            pex_cases: Vec::new(), erc_cases: Vec::new(),
        }
    }

    pub fn add_drc(&mut self, id: &str, cell: &str, rule: &str, expect: usize) {
        let mut c = json!({
            "id": id, "cell": cell, "rule": rule,
            "expect_violations": expect,
        });
        if expect > 0 {
            c["violations"] = json!([{}]);
        }
        self.drc_cases.push(c);
    }

    pub fn add_drc_measured(&mut self, id: &str, cell: &str, rule: &str, expect: usize, measured: i64) {
        let mut c = json!({
            "id": id, "cell": cell, "rule": rule,
            "expect_violations": expect,
        });
        if expect > 0 {
            // one entry per expected violation: the harness asserts the full multiset
            c["violations"] = json!(vec![json!({"measured": measured}); expect]);
        }
        self.drc_cases.push(c);
    }

    /// Expected violations with per-violation measured values (full multiset assert).
    pub fn add_drc_measured_multi(&mut self, id: &str, cell: &str, rule: &str, measured: &[i64]) {
        let viols: Vec<Value> = measured.iter().map(|m| json!({"measured": m})).collect();
        self.drc_cases.push(json!({
            "id": id, "cell": cell, "rule": rule,
            "expect_violations": measured.len(),
            "violations": viols,
        }));
    }

    pub fn add_drc_strict(&mut self, id: &str, cell: &str, rule: &str, expect: usize) {
        self.drc_cases.push(json!({
            "id": id, "cell": cell, "rule": rule,
            "expect_violations": expect, "strict": true,
        }));
    }

    pub fn add_drc_strict_measured(&mut self, id: &str, cell: &str, rule: &str, expect: usize, measured: i64) {
        let mut c = json!({
            "id": id, "cell": cell, "rule": rule,
            "expect_violations": expect, "strict": true,
        });
        if expect > 0 {
            c["violations"] = json!([{"measured": measured}]);
        }
        self.drc_cases.push(c);
    }

    pub fn add_drc_frac(
        &mut self, id: &str, cell: &str, rule: &str, expect: usize,
        measured_frac: f64, limit_frac: f64,
    ) {
        let mut c = json!({
            "id": id, "cell": cell, "rule": rule,
            "expect_violations": expect,
        });
        if expect > 0 {
            c["violations"] = json!([{
                "measured_frac": measured_frac,
                "limit_frac": limit_frac,
            }]);
        }
        self.drc_cases.push(c);
    }

    pub fn add_lvs(&mut self, id: &str, cell: &str, expect_match: bool, devices: Vec<Value>) {
        self.lvs_cases.push(json!({
            "id": id, "cell": cell, "expect_match": expect_match,
            "reference_netlist": { "devices": devices },
        }));
    }

    pub fn add_pex(&mut self, id: &str, cell: &str, kind: &str, tol: f64, expected: Value) {
        self.pex_cases.push(json!({
            "id": id, "cell": cell, "kind": kind, "tol": tol, "expected": expected,
        }));
    }

    /// Negative-direction PEX case: `expected` is deliberately wrong; the harness
    /// passes iff the engine's measurement differs (proves the assert has teeth).
    pub fn add_pex_mismatch(&mut self, id: &str, cell: &str, kind: &str, tol: f64, expected: Value) {
        self.pex_cases.push(json!({
            "id": id, "cell": cell, "kind": kind, "tol": tol, "expected": expected,
            "expect_mismatch": true,
        }));
    }

    pub fn add_erc(&mut self, id: &str, cell: &str, check: &str, expect: usize) {
        self.erc_cases.push(json!({
            "id": id, "cell": cell, "check": check, "expect_violations": expect,
        }));
    }

    pub fn manifest(&self) -> Value {
        json!({
            "gds_file": "conformance.gds",
            "drc": { "cases": self.drc_cases },
            "lvs": { "cases": self.lvs_cases },
            "pex": { "cases": self.pex_cases },
            "erc": { "cases": self.erc_cases },
        })
    }
}

pub fn params_json() -> Value {
    json!({
        "layers": {
            "nwell": { "layer": NWELL.0, "datatype": NWELL.1 },
            "diff":  { "layer": DIFF.0,  "datatype": DIFF.1 },
            "poly":  { "layer": POLY.0,  "datatype": POLY.1 },
            "licon": { "layer": LICON.0, "datatype": LICON.1 },
            "li":    { "layer": LI.0,    "datatype": LI.1 },
            "mcon":  { "layer": MCON.0,  "datatype": MCON.1 },
            "met1":  { "layer": MET1.0,  "datatype": MET1.1 },
            "via1":  { "layer": VIA1.0,  "datatype": VIA1.1 },
            "met2":  { "layer": MET2.0,  "datatype": MET2.1 },
            "nsdm":  { "layer": NSDM.0,  "datatype": NSDM.1 },
            "psdm":  { "layer": PSDM.0,  "datatype": PSDM.1 },
            "lvt":   { "layer": LVT.0,  "datatype": LVT.1 },
            "hvt":   { "layer": HVT.0,  "datatype": HVT.1 },
            "diode": { "layer": DIODE.0, "datatype": DIODE.1 },
        },
        "drc": {
            "min_width":        { "layer": "met1", "min": 100 },
            "min_spacing":      { "layer": "met1", "min": 100 },
            "min_spacing_diff": { "layer_a": "met1", "layer_b": "met2", "min": 100 },
            "min_enclosure":    { "outer": "met1", "inner": "met2", "min": 50 },
            "min_extension":    { "layer": "poly", "ref": "diff", "min": 50 },
            "min_area":         { "layer": "met1", "min": 40000 },
            "max_width":        { "layer": "met1", "max": 5000 },
            "notch":            { "layer": "met1", "min": 100 },
            "min_edge_length":  { "layer": "met1", "min": 100 },
            "off_grid":         { "grid": 5 },
            "angle":            { "allowed": [0, 90] },
            "min_density":      { "layer": "met1", "window": 2000, "min_frac": 0.3 },
            "max_density":      { "layer": "met1", "window": 2000, "max_frac": 0.8 },
            "overlap":          { "layer_a": "met1", "layer_b": "met2", "min": 50 },
            "corner_to_corner": { "layer": "met1", "min": 150 },
            "antenna":          { "layer": "met1", "ratio": 100.0 },
            "antenna_car":      { "layers": ["li", "met1", "met2"], "ratio": 400.0, "diode_layer": "diode" },
            "eol_spacing":      { "layer": "met1", "eol_width": 200, "eol_spacing": 150 },
            "well_enclosure":   { "outer": "nwell", "inner": "diff", "min": 50 },
            "wide_dependent_spacing": { "layer": "met1", "width_threshold": 500, "wide_spacing": 200 },
            "prl_spacing": { "layer": "met1", "prl_threshold": 500, "prl_spacing": 200 },
            "asymmetric_enclosure": { "outer": "met1", "inner": "via1", "min": 30 },
            "min_enclosed_area": { "layer": "met1", "min": 50000 },
            "cheesing": { "layer": "met1", "max": 2000000 },
            "redundant_via": { "layer": "via1", "min_count": 2, "within": 500 },
            "via_array_spacing": { "layer": "via1", "array_threshold": 3, "array_spacing": 200 },
            "max_distance_to_tap": { "layer": "diff", "layer_b": "li", "max_dist": 3000 },
            "multi_patterning": { "layer": "met1", "num_colors": 2, "min": 100 },
        },
        "pex": {
            "met1": {
                "sheet_res_ohm_sq": 0.1,
                "area_cap_af_um2": 25.0,
                "fringe_cap_af_um": 40.0,
                "coupling_cap_af_um": 100.0,
                "coupling_ref_spacing_nm": 200.0,
            },
            "via1": {
                "sheet_res_ohm_sq": 0.0,
                "area_cap_af_um2": 0.0,
                "fringe_cap_af_um": 0.0,
                "coupling_cap_af_um": 0.0,
                "coupling_ref_spacing_nm": 1.0,
                "via_res_ohm": 5.0
            },
            "met2": {
                "sheet_res_ohm_sq": 0.08,
                "area_cap_af_um2": 15.0,
                "fringe_cap_af_um": 30.0,
                "coupling_cap_af_um": 80.0,
                "coupling_ref_spacing_nm": 200.0,
                "interlayer_cap_af_um2": 10.0
            }
        },
        "erc": {
            "antenna_ratio": 200.0,
            "em_min_width_nm": 200,
            "p2p_r_limit_ohm": 10.0,
            "tie_max_dist_nm": 2000
        },
        "connectivity": {
            "conductors": ["diff", "poly", "li", "met1", "met2"],
            "vias": [
                { "layer": "licon", "connects": ["diff", "poly", "li"] },
                { "layer": "mcon",  "connects": ["li", "met1"] },
                { "layer": "via1",  "connects": ["met1", "met2"] }
            ]
        },
        "devices": {
            "mos": [
                {
                    "name": "nmos", "gate_layer": "poly", "channel_layer": "diff",
                    "type_implant": "nsdm", "device_type": "nmos",
                    "flavor_markers": [["lvt", "lvt"], ["hvt", "hvt"]]
                },
                {
                    "name": "pmos", "gate_layer": "poly", "channel_layer": "diff",
                    "type_implant": "psdm", "device_type": "pmos",
                    "well_layer": "nwell",
                    "flavor_markers": [["lvt", "lvt"], ["hvt", "hvt"]]
                }
            ]
        },
    })
}

pub fn lvs_device(kind: &str, g: &str, s: &str, d: &str) -> Value {
    json!({"type": kind, "g": g, "s": s, "d": d})
}

pub fn lvs_device_wl(kind: &str, g: &str, s: &str, d: &str, w: i32, l: i32) -> Value {
    json!({"type": kind, "g": g, "s": s, "d": d, "w": w, "l": l})
}

pub fn lvs_device_flavor(kind: &str, g: &str, s: &str, d: &str, flavor: &str) -> Value {
    json!({"type": kind, "g": g, "s": s, "d": d, "flavor": flavor})
}
