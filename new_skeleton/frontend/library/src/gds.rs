//! GDSII stream writer: a flat shape list → a spec-compliant GDSII byte stream.
//!
//! This is the "GDS writer" the [`crate::geometry::collect`] comment anticipates
//! — the pipeline draws geometry as [`Shape`]s (device-centre coordinates already
//! flattened), and this turns them into the industry GDSII format for tape-out
//! artifacts and for feeding [`visualizer::export_svg`].
//!
//! ## Conformance
//!
//! The output follows the GDSII Stream Format (Calma/Cadence) record layout:
//! every record is `[u16 byte-length][u8 record-type][u8 data-type][payload]`,
//! big-endian, even byte-length. The stream is a complete, self-contained
//! library:
//!
//! ```text
//! HEADER  BGNLIB LIBNAME UNITS
//!   BGNSTR STRNAME
//!     ( BOUNDARY LAYER DATATYPE XY ENDEL )*
//!   ENDSTR
//! ENDLIB
//! ```
//!
//! - **Timestamps** (BGNLIB/BGNSTR) are the real modification/access time as the
//!   six-short GDS calendar tuple `(year, month, day, hour, minute, second)`.
//! - **Strings** (LIBNAME/STRNAME) are ASCII, null-padded to an even length.
//! - **UNITS** declares a 1 nm database unit (user-unit = 1e-3 µm, db-unit = 1e-9
//!   m); shape coordinates are written as their native `nm` integers, so a reader
//!   recovers real dimensions.
//! - **XY** for a BOUNDARY is an explicitly closed polygon (first vertex repeated
//!   as the last), as the spec requires.
//! - A record's payload is length-checked against the GDS 65534-byte record cap;
//!   rectangles are far under it, but the guard keeps the writer honest.

use std::time::{SystemTime, UNIX_EPOCH};

use pnr_core::Shape;

/// Library name written into the LIBNAME record.
const LIBNAME: &str = "PHILIS.db";
/// The single structure (cell) name every shape is written into.
const STRNAME: &str = "TOP";
/// GDS records carry a `u16` byte length, so the payload cap is `65534 - 4`.
const MAX_PAYLOAD: usize = 65534 - 4;

// GDS record type + data type, packed as the `u16` written after the length.
const HEADER: u16 = 0x0002; //  version           (2-byte int)
const BGNLIB: u16 = 0x0102; //  library begin     (2-byte int × 12)
const LIBNAME_R: u16 = 0x0206; //               (ASCII string)
const UNITS: u16 = 0x0305; //  units             (8-byte real × 2)
const BGNSTR: u16 = 0x0502; //  structure begin   (2-byte int × 12)
const STRNAME_R: u16 = 0x0606; //              (ASCII string)
const BOUNDARY: u16 = 0x0800; //  boundary element (no data)
const LAYER: u16 = 0x0D02; //  layer number       (2-byte int)
const DATATYPE: u16 = 0x0E02; //  data type        (2-byte int)
const XY: u16 = 0x1003; //  coordinate list        (4-byte int)
const ENDEL: u16 = 0x1100; //  element end         (no data)
const ENDSTR: u16 = 0x0700; //  structure end      (no data)
const ENDLIB: u16 = 0x0400; //  library end        (no data)

/// GDS stream version. 600 is the widely-accepted modern release number.
const GDS_VERSION: i16 = 600;

/// Emit `shapes` as a GDSII byte stream. `layer_gds` maps a Philis
/// [`pnr_core::LayerId`] (index = `LayerId.0`) to its `(gds_layer, gds_datatype)`
/// numbers; a shape whose layer id is outside the table falls back to `(id, 0)`
/// so it still draws.
#[must_use]
pub fn emit(shapes: &[Shape], layer_gds: &[(u16, u16)]) -> Vec<u8> {
    let ts = gds_timestamp();
    let mut out = Vec::new();

    rec_i16(&mut out, HEADER, &[GDS_VERSION]);
    // BGNLIB — last-modified + last-accessed timestamps (identical here).
    rec_i16(&mut out, BGNLIB, &timestamp_pair(ts));
    rec_str(&mut out, LIBNAME_R, LIBNAME);
    // UNITS — (user-units per db-unit, db-unit in metres): 1 nm database grid.
    rec_real(&mut out, UNITS, &[1e-3, 1e-9]);

    // One structure holding every shape.
    rec_i16(&mut out, BGNSTR, &timestamp_pair(ts));
    rec_str(&mut out, STRNAME_R, STRNAME);

    for s in shapes {
        let (gl, gd) = layer_gds
            .get(s.layer.0 as usize)
            .copied()
            .unwrap_or((s.layer.0, 0));
        let (x, y, w, h) = (s.rect.x, s.rect.y, s.rect.w, s.rect.h);
        rec_empty(&mut out, BOUNDARY);
        rec_i16(&mut out, LAYER, &[gl as i16]);
        rec_i16(&mut out, DATATYPE, &[gd as i16]);
        // Closed rectangle: first vertex repeated as the last (GDS requirement).
        rec_i32(&mut out, XY, &[x, y, x + w, y, x + w, y + h, x, y + h, x, y]);
        rec_empty(&mut out, ENDEL);
    }

    rec_empty(&mut out, ENDSTR);
    rec_empty(&mut out, ENDLIB);
    out
}

// ── record writers ─────────────────────────────────────────────────────────
// Every record is [u16 total-len][u8 rec-type][u8 data-type][payload], BE.

fn header(out: &mut Vec<u8>, rec_datatype: u16, payload_len: usize) {
    debug_assert!(payload_len <= MAX_PAYLOAD, "GDS record payload exceeds 65534-byte cap");
    let len = 4 + payload_len;
    out.extend_from_slice(&(len as u16).to_be_bytes());
    out.extend_from_slice(&rec_datatype.to_be_bytes());
}

fn rec_empty(out: &mut Vec<u8>, rec_datatype: u16) {
    header(out, rec_datatype, 0);
}

fn rec_i16(out: &mut Vec<u8>, rec_datatype: u16, vals: &[i16]) {
    header(out, rec_datatype, vals.len() * 2);
    for v in vals {
        out.extend_from_slice(&v.to_be_bytes());
    }
}

fn rec_i32(out: &mut Vec<u8>, rec_datatype: u16, vals: &[i32]) {
    header(out, rec_datatype, vals.len() * 4);
    for v in vals {
        out.extend_from_slice(&v.to_be_bytes());
    }
}

fn rec_str(out: &mut Vec<u8>, rec_datatype: u16, s: &str) {
    let mut bytes = s.as_bytes().to_vec();
    if bytes.len() % 2 == 1 {
        bytes.push(0); // GDS strings are null-padded to even length.
    }
    header(out, rec_datatype, bytes.len());
    out.extend_from_slice(&bytes);
}

fn rec_real(out: &mut Vec<u8>, rec_datatype: u16, vals: &[f64]) {
    header(out, rec_datatype, vals.len() * 8);
    for &v in vals {
        out.extend_from_slice(&gds_real(v));
    }
}

/// Encode an `f64` as an 8-byte GDS real: base-16 exponent in excess-64, sign in
/// bit 7 of byte 0, then a 7-byte fractional mantissa in `[1/16, 1)`. Exact
/// inverse of the reader in `visualizer`.
fn gds_real(v: f64) -> [u8; 8] {
    if v == 0.0 {
        return [0; 8];
    }
    let sign = v < 0.0;
    let mut mant = v.abs();
    let mut exp: i32 = 0;
    while mant >= 1.0 {
        mant /= 16.0;
        exp += 1;
    }
    while mant < 1.0 / 16.0 {
        mant *= 16.0;
        exp -= 1;
    }
    let mut out = [0u8; 8];
    out[0] = (if sign { 0x80 } else { 0 }) | (((exp + 64) as u8) & 0x7f);
    let mut m = mant;
    for b in out.iter_mut().skip(1) {
        m *= 256.0;
        let byte = m.floor();
        *b = byte as u8;
        m -= byte;
    }
    out
}

// ── timestamps ───────────────────────────────────────────────────────────

/// The GDS six-short calendar tuple `(year, month, day, hour, minute, second)`
/// for "now" (UTC). Falls back to the GDS epoch tuple if the clock predates
/// `UNIX_EPOCH`.
fn gds_timestamp() -> [i16; 6] {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let days = secs.div_euclid(86_400);
    let tod = secs.rem_euclid(86_400);
    let (y, mo, d) = civil_from_days(days);
    [y, mo, d, (tod / 3600) as i16, ((tod % 3600) / 60) as i16, (tod % 60) as i16]
}

/// Two identical timestamps (modification + access) as the 12-short payload
/// BGNLIB and BGNSTR both take.
fn timestamp_pair(ts: [i16; 6]) -> [i16; 12] {
    let mut out = [0i16; 12];
    out[..6].copy_from_slice(&ts);
    out[6..].copy_from_slice(&ts);
    out
}

/// Days-since-Unix-epoch → `(year, month, day)` (proleptic Gregorian, UTC).
/// Howard Hinnant's `civil_from_days`, dependency-free.
fn civil_from_days(z: i64) -> (i16, i16, i16) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let year = if m <= 2 { y + 1 } else { y };
    (year as i16, m as i16, d as i16)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_core::{LayerId, Rect, Shape};

    // A written GDS must round-trip back through the viewer's parser: same
    // rectangle count, same layer numbers, closed 4-corner boundaries.
    #[test]
    fn emit_round_trips_through_parser() {
        let shapes = vec![
            Shape { layer: LayerId(0), rect: Rect { x: 0, y: 0, w: 100, h: 200 } },
            Shape { layer: LayerId(1), rect: Rect { x: 50, y: 50, w: 30, h: 30 } },
        ];
        let bytes = emit(&shapes, &[(68, 20), (69, 20)]);
        let (polys, _) = visualizer::parse_gds(&bytes);
        assert_eq!(polys.len(), 2, "both boundaries parse back");
        let mut layers: Vec<u16> = polys.iter().map(|p| p.layer).collect();
        layers.sort_unstable();
        assert_eq!(layers, vec![68, 69]);
        assert!(polys.iter().all(|p| p.pts.len() == 4), "closing point dropped by parser");
    }

    // Every record must be even-length and the byte length must exactly cover the
    // stream — the invariant a conformant reader relies on to walk records.
    #[test]
    fn record_lengths_are_even_and_cover_the_stream() {
        let bytes = emit(
            &[Shape { layer: LayerId(0), rect: Rect { x: -5, y: -5, w: 10, h: 10 } }],
            &[(66, 20)],
        );
        let mut i = 0;
        let mut saw_endlib = false;
        while i + 4 <= bytes.len() {
            let len = u16::from_be_bytes([bytes[i], bytes[i + 1]]) as usize;
            assert!(len >= 4 && len % 2 == 0, "record at {i} has bad length {len}");
            assert!(i + len <= bytes.len(), "record at {i} overruns stream");
            if u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) == ENDLIB {
                saw_endlib = true;
            }
            i += len;
        }
        assert_eq!(i, bytes.len(), "records exactly tile the stream");
        assert!(saw_endlib, "stream terminates with ENDLIB");
    }

    // The real encoder must invert the documented reader semantics.
    #[test]
    fn gds_real_matches_reader() {
        // Reader: sign·mantissa·16^(exp-64), mantissa = Σ byte[k]/256^k.
        fn read(b: [u8; 8]) -> f64 {
            let sign = if b[0] & 0x80 != 0 { -1.0 } else { 1.0 };
            let exp = (b[0] & 0x7f) as i32 - 64;
            let mut mant = 0.0f64;
            for k in 1..8 {
                mant += b[k] as f64 / 256f64.powi(k as i32);
            }
            sign * mant * 16f64.powi(exp)
        }
        for &v in &[1e-3, 1e-9, 1.0, 42.5, -7.25] {
            let got = read(gds_real(v));
            assert!((got - v).abs() <= v.abs() * 1e-9 + 1e-18, "roundtrip {v} != {got}");
        }
        assert_eq!(gds_real(0.0), [0; 8]);
    }
}
