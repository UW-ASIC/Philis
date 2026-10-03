//! GDSII stream writer: flat shapes → one named structure of closed
//! rectangular BOUNDARY elements plus net-name TEXT elements, 1 nm database
//! unit, big-endian records.

use pnr_core::Shape;

/// Library name written into the LIBNAME record.
const LIBNAME: &str = "PHILIS.db";
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
const TEXT: u16 = 0x0C00; //  text element         (no data)
const TEXTTYPE: u16 = 0x1602; //  text type         (2-byte int)
const STRING: u16 = 0x1906; //  text string         (ASCII string)
const ENDSTR: u16 = 0x0700; //  structure end      (no data)
const ENDLIB: u16 = 0x0400; //  library end        (no data)

/// Modification + access time, fixed (2000-01-01) so output is reproducible.
const TIMESTAMPS: [i16; 12] = [2000, 1, 1, 0, 0, 0, 2000, 1, 1, 0, 0, 0];

/// GDS stream version. 600 is the widely-accepted modern release number.
const GDS_VERSION: i16 = 600;

/// A net name placed at `(x, y)`, nm, on GDS `(layer, texttype)`.
pub struct Text {
    pub name: String,
    pub gds: (u16, u16),
    pub x: i32,
    pub y: i32,
}

/// Emit `shapes` and `texts` as a GDSII byte stream with one structure named
/// `top`. `layer_gds` maps a Philis [`pnr_core::LayerId`] (index =
/// `LayerId.0`) to its `(gds_layer, gds_datatype)` numbers; a shape whose
/// layer id is outside the table falls back to `(id, 0)` so it still draws.
#[must_use]
pub fn emit(top: &str, shapes: &[Shape], layer_gds: &[(u16, u16)], texts: &[Text]) -> Vec<u8> {
    let mut out = Vec::new();

    rec_i16(&mut out, HEADER, &[GDS_VERSION]);
    rec_i16(&mut out, BGNLIB, &TIMESTAMPS);
    rec_str(&mut out, LIBNAME_R, LIBNAME);
    // UNITS — (user-units per db-unit, db-unit in metres): 1 nm database grid.
    rec_real(&mut out, UNITS, &[1e-3, 1e-9]);

    // One structure holding every shape.
    rec_i16(&mut out, BGNSTR, &TIMESTAMPS);
    rec_str(&mut out, STRNAME_R, top);

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
        rec_i32(
            &mut out,
            XY,
            &[x, y, x + w, y, x + w, y + h, x, y + h, x, y],
        );
        rec_empty(&mut out, ENDEL);
    }
    for t in texts {
        rec_empty(&mut out, TEXT);
        rec_i16(&mut out, LAYER, &[t.gds.0 as i16]);
        rec_i16(&mut out, TEXTTYPE, &[t.gds.1 as i16]);
        rec_i32(&mut out, XY, &[t.x, t.y]);
        rec_str(&mut out, STRING, &t.name);
        rec_empty(&mut out, ENDEL);
    }

    rec_empty(&mut out, ENDSTR);
    rec_empty(&mut out, ENDLIB);
    out
}

// ── record writers ─────────────────────────────────────────────────────────
// Every record is [u16 total-len][u8 rec-type][u8 data-type][payload], BE.

fn header(out: &mut Vec<u8>, rec_datatype: u16, payload_len: usize) {
    debug_assert!(
        payload_len <= MAX_PAYLOAD,
        "GDS record payload exceeds 65534-byte cap"
    );
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

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_core::{LayerId, Rect, Shape};

    // A written GDS must round-trip back through the viewer's parser: same
    // rectangle count, same layer numbers, closed 4-corner boundaries.
    #[test]
    fn emit_round_trips_through_parser() {
        let shapes = vec![
            Shape {
                layer: LayerId(0),
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 100,
                    h: 200,
                },
            },
            Shape {
                layer: LayerId(1),
                rect: Rect {
                    x: 50,
                    y: 50,
                    w: 30,
                    h: 30,
                },
            },
        ];
        let bytes = emit("TOP", &shapes, &[(68, 20), (69, 20)], &[]);
        let (polys, _) = visualizer::parse_gds(&bytes);
        assert_eq!(polys.len(), 2, "both boundaries parse back");
        let mut layers: Vec<u16> = polys.iter().map(|p| p.layer).collect();
        layers.sort_unstable();
        assert_eq!(layers, vec![68, 69]);
        assert!(
            polys.iter().all(|p| p.pts.len() == 4),
            "closing point dropped by parser"
        );
    }

    // Every record must be even-length and the byte length must exactly cover the
    // stream — the invariant a conformant reader relies on to walk records.
    #[test]
    fn record_lengths_are_even_and_cover_the_stream() {
        let bytes = emit(
            "TOP",
            &[Shape {
                layer: LayerId(0),
                rect: Rect {
                    x: -5,
                    y: -5,
                    w: 10,
                    h: 10,
                },
            }],
            &[(66, 20)],
            &[Text { name: "vdd".into(), gds: (68, 5), x: 0, y: 0 }],
        );
        let mut i = 0;
        let mut saw_endlib = false;
        while i + 4 <= bytes.len() {
            let len = u16::from_be_bytes([bytes[i], bytes[i + 1]]) as usize;
            assert!(
                len >= 4 && len % 2 == 0,
                "record at {i} has bad length {len}"
            );
            assert!(i + len <= bytes.len(), "record at {i} overruns stream");
            if u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) == ENDLIB {
                saw_endlib = true;
            }
            i += len;
        }
        assert_eq!(i, bytes.len(), "records exactly tile the stream");
        assert!(saw_endlib, "stream terminates with ENDLIB");
    }

    // The top cell carries the given name and every text its STRING record,
    // and texts do not disturb the boundaries a reader parses.
    #[test]
    fn top_name_and_texts_are_written() {
        let shape = Shape { layer: LayerId(0), rect: Rect { x: 0, y: 0, w: 10, h: 10 } };
        let bytes = emit("strongarm", &[shape], &[(68, 20)], &[Text { name: "vinp".into(), gds: (68, 5), x: 5, y: 5 }]);
        let has = |s: &[u8]| bytes.windows(s.len()).any(|w| w == s);
        assert!(has(b"strongarm"), "STRNAME is the top name");
        assert!(!has(b"TOP"), "no fixed TOP cell");
        assert!(has(&[0x00, 0x06, 0x16, 0x02, 0x00, 0x05]), "TEXTTYPE 5");
        assert!(has(b"vinp"), "the label string");
        let (polys, _) = visualizer::parse_gds(&bytes);
        assert_eq!(polys.len(), 1, "the boundary still parses with a text present");
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
            assert!(
                (got - v).abs() <= v.abs() * 1e-9 + 1e-18,
                "roundtrip {v} != {got}"
            );
        }
        assert_eq!(gds_real(0.0), [0; 8]);
    }
}
