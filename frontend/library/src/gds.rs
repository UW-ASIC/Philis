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

/// A net-name label: one GDS TEXT element.
pub struct Text {
    /// The label string (the net name), written as the STRING record.
    pub name: String,
    /// GDS `(layer, texttype)` the label is written on.
    pub gds: (u16, u16),
    /// Anchor x, nm (database units).
    pub x: i32,
    /// Anchor y, nm (database units).
    pub y: i32,
}

/// Returns `shapes` and `texts` as a GDSII byte stream with one structure
/// named `top`: every shape a closed 5-point BOUNDARY, every text a TEXT
/// element, database unit 1 nm, fixed timestamps so equal input gives equal
/// bytes. `layer_gds` maps a Philis [`pnr_core::LayerId`] (index =
/// `LayerId.0`) to its `(gds_layer, gds_datatype)` numbers; layer and
/// datatype are written as their 16-bit pattern.
///
/// # Errors
/// Names every shape layer id outside `layer_gds` or mapped to `(0, 0)` (a
/// layer the deck derives rather than draws): writing it anyway would put the
/// shape on a layer nothing reads. Also refuses a `top` or text name longer
/// than one GDS record holds (65530 bytes), which would otherwise
/// corrupt the stream.
pub fn emit(top: &str, shapes: &[Shape], layer_gds: &[(u16, u16)], texts: &[Text]) -> Result<Vec<u8>, String> {
    let mut bad: Vec<u16> = shapes
        .iter()
        .map(|s| s.layer.0)
        .filter(|&l| layer_gds.get(l as usize).is_none_or(|&g| g == (0, 0)))
        .collect();
    bad.sort_unstable();
    bad.dedup();
    if !bad.is_empty() {
        return Err(format!("no GDS stream number for layer ids {bad:?} (derived or unmapped)"));
    }
    // A string record's padded payload must fit the u16 length field.
    if let Some(name) = std::iter::once(top).chain(texts.iter().map(|t| t.name.as_str())).find(|n| n.len() + n.len() % 2 > MAX_PAYLOAD) {
        return Err(format!("name of {} bytes exceeds one GDS record ({MAX_PAYLOAD} bytes)", name.len()));
    }
    // 4 records of 4–6 bytes plus a 44-byte XY per boundary: 68 bytes.
    let mut out = Vec::with_capacity(256 + 68 * shapes.len() + 64 * texts.len());

    rec_i16(&mut out, HEADER, &[GDS_VERSION]);
    rec_i16(&mut out, BGNLIB, &TIMESTAMPS);
    rec_str(&mut out, LIBNAME_R, LIBNAME);
    // UNITS — (user-units per db-unit, db-unit in metres): 1 nm database grid.
    rec_real(&mut out, UNITS, &[1e-3, 1e-9]);

    // One structure holding every shape.
    rec_i16(&mut out, BGNSTR, &TIMESTAMPS);
    rec_str(&mut out, STRNAME_R, top);

    for s in shapes {
        let (gl, gd) = layer_gds[s.layer.0 as usize];
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
    Ok(out)
}

// ── record writers ─────────────────────────────────────────────────────────
// Every record is [u16 total-len][u8 rec-type][u8 data-type][payload], BE.

/// Writes a record header for a payload of `payload_len` bytes, which must be
/// even and at most [`MAX_PAYLOAD`] (debug-asserted).
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

/// Writes an ASCII-string record, null-padded to even length (GDS rule).
fn rec_str(out: &mut Vec<u8>, rec_datatype: u16, s: &str) {
    let odd = s.len() % 2;
    header(out, rec_datatype, s.len() + odd);
    out.extend_from_slice(s.as_bytes());
    if odd == 1 {
        out.push(0);
    }
}

fn rec_real(out: &mut Vec<u8>, rec_datatype: u16, vals: &[f64]) {
    header(out, rec_datatype, vals.len() * 8);
    for &v in vals {
        out.extend_from_slice(&gds_real(v));
    }
}

/// Encodes an `f64` as an 8-byte GDS real: base-16 exponent in excess-64, sign
/// in bit 7 of byte 0, then a 7-byte fractional mantissa in `[1/16, 1)`,
/// truncated (not rounded) to 56 bits. Inverse of the reader in `visualizer`
/// to within one mantissa ulp.
///
/// # Panics
/// On a non-finite `v`, or a magnitude outside the representable
/// `[16^-65, 16^63)`; only the two UNITS constants are ever encoded.
fn gds_real(v: f64) -> [u8; 8] {
    assert!(v.is_finite(), "GDS real of a non-finite {v}");
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
    assert!((-64..64).contains(&exp), "GDS real {v:e} outside the excess-64 exponent range");
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
        let bytes = emit("TOP", &shapes, &[(68, 20), (69, 20)], &[]).unwrap();
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
        )
        .unwrap();
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
        let bytes = emit("strongarm", &[shape], &[(68, 20)], &[Text { name: "vinp".into(), gds: (68, 5), x: 5, y: 5 }]).unwrap();
        let has = |s: &[u8]| bytes.windows(s.len()).any(|w| w == s);
        assert!(has(b"strongarm"), "STRNAME is the top name");
        assert!(!has(b"TOP"), "no fixed TOP cell");
        assert!(has(&[0x00, 0x06, 0x16, 0x02, 0x00, 0x05]), "TEXTTYPE 5");
        assert!(has(b"vinp"), "the label string");
        let (polys, _) = visualizer::parse_gds(&bytes);
        assert_eq!(polys.len(), 1, "the boundary still parses with a text present");
    }

    // A shape on a layer with no stream number is refused, not drawn on
    // `(id, 0)` or `(0, 0)` where no reader looks.
    #[test]
    fn unmapped_layer_is_an_error() {
        let on = |l| [Shape { layer: LayerId(l), rect: Rect { x: 0, y: 0, w: 10, h: 10 } }];
        let err = emit("t", &on(5), &[(68, 20), (69, 20)], &[]).unwrap_err();
        assert!(err.contains('5'), "{err}");
        let err = emit("t", &on(1), &[(68, 20), (0, 0)], &[]).unwrap_err();
        assert!(err.contains("[1]"), "{err}");
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

    /// Splits a stream into `(record type, payload)` pairs, asserting each
    /// length is even, at least 4 and inside the stream.
    fn records(bytes: &[u8]) -> Vec<(u16, &[u8])> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            let len = u16::from_be_bytes([bytes[i], bytes[i + 1]]) as usize;
            assert!(len >= 4 && len % 2 == 0 && i + len <= bytes.len(), "bad record at {i}");
            out.push((u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]), &bytes[i + 4..i + len]));
            i += len;
        }
        out
    }

    fn i32s(p: &[u8]) -> Vec<i32> {
        p.chunks(4).map(|c| i32::from_be_bytes([c[0], c[1], c[2], c[3]])).collect()
    }

    fn rect(layer: u16, x: i32, y: i32, w: i32, h: i32) -> Shape {
        Shape { layer: LayerId(layer), rect: Rect { x, y, w, h } }
    }

    // Empty input is still a complete library: the fixed header records, one
    // empty structure, ENDLIB last, in GDSII order.
    #[test]
    fn empty_input_is_a_complete_library() {
        let bytes = emit("top", &[], &[], &[]).unwrap();
        let kinds: Vec<u16> = records(&bytes).iter().map(|r| r.0).collect();
        assert_eq!(kinds, [HEADER, BGNLIB, LIBNAME_R, UNITS, BGNSTR, STRNAME_R, ENDSTR, ENDLIB]);
        let recs = records(&bytes);
        assert_eq!(recs[0].1, &600i16.to_be_bytes(), "version 600");
        assert_eq!(recs[1].1.len(), 24, "12 timestamp shorts");
        assert_eq!(recs[2].1, b"PHILIS.db\0", "odd name padded with one null");
        assert_eq!(recs[3].1.len(), 16, "two 8-byte reals");
    }

    // One boundary is BOUNDARY, LAYER, DATATYPE, XY, ENDEL, and XY is the
    // closed counter-clockwise rectangle starting at the lower-left corner.
    #[test]
    fn boundary_records_are_exact() {
        let bytes = emit("t", &[rect(0, -5, 7, 10, 20)], &[(66, 20)], &[]).unwrap();
        let recs = records(&bytes);
        let at = recs.iter().position(|r| r.0 == BOUNDARY).unwrap();
        assert_eq!(recs[at + 1], (LAYER, &66i16.to_be_bytes()[..]));
        assert_eq!(recs[at + 2], (DATATYPE, &20i16.to_be_bytes()[..]));
        assert_eq!(recs[at + 3].0, XY);
        assert_eq!(i32s(recs[at + 3].1), [-5, 7, 5, 7, 5, 27, -5, 27, -5, 7]);
        assert_eq!(recs[at + 4].0, ENDEL);
    }

    // A text is TEXT, LAYER, TEXTTYPE, XY (one point), STRING, ENDEL.
    #[test]
    fn text_records_are_exact() {
        let t = Text { name: "out".into(), gds: (70, 5), x: -3, y: 9 };
        let bytes = emit("t", &[], &[], &[t]).unwrap();
        let recs = records(&bytes);
        let at = recs.iter().position(|r| r.0 == TEXT).unwrap();
        assert_eq!(recs[at + 1], (LAYER, &70i16.to_be_bytes()[..]));
        assert_eq!(recs[at + 2], (TEXTTYPE, &5i16.to_be_bytes()[..]));
        assert_eq!(i32s(recs[at + 3].1), [-3, 9]);
        assert_eq!(recs[at + 4], (STRING, &b"out\0"[..]));
        assert_eq!(recs[at + 5].0, ENDEL);
    }

    // Layer numbers past i16::MAX keep their 16-bit pattern.
    #[test]
    fn high_layer_numbers_keep_their_bits() {
        let bytes = emit("t", &[rect(0, 0, 0, 1, 1)], &[(40_000, 65_535)], &[]).unwrap();
        let recs = records(&bytes);
        let at = recs.iter().position(|r| r.0 == LAYER).unwrap();
        assert_eq!(recs[at].1, &40_000u16.to_be_bytes());
        assert_eq!(recs[at + 1].1, &65_535u16.to_be_bytes());
    }

    // Strings: even length unpadded, odd padded by one null, empty empty.
    #[test]
    fn strings_are_padded_to_even_length() {
        for (name, payload) in [("ab", &b"ab"[..]), ("abc", &b"abc\0"[..]), ("", &b""[..])] {
            let bytes = emit(name, &[], &[], &[]).unwrap();
            let rec = records(&bytes).into_iter().find(|r| r.0 == STRNAME_R).unwrap();
            assert_eq!(rec.1, payload, "{name:?}");
        }
    }

    // Same input, same bytes: timestamps are fixed.
    #[test]
    fn output_is_deterministic() {
        let s = [rect(0, 1, 2, 3, 4)];
        assert_eq!(emit("t", &s, &[(1, 0)], &[]).unwrap(), emit("t", &s, &[(1, 0)], &[]).unwrap());
    }

    // The error lists each bad layer id once, ascending.
    #[test]
    fn unmapped_layers_are_listed_sorted_once() {
        let s = [rect(7, 0, 0, 1, 1), rect(3, 0, 0, 1, 1), rect(7, 0, 0, 1, 1), rect(0, 0, 0, 1, 1)];
        let err = emit("t", &s, &[(1, 0)], &[]).unwrap_err();
        assert!(err.contains("[3, 7]"), "{err}");
    }

    // A name one record cannot hold is refused, not written with a wrapped
    // length that desynchronises every reader.
    #[test]
    fn oversized_names_are_refused() {
        let long = "x".repeat(MAX_PAYLOAD + 1);
        assert!(emit(&long, &[], &[], &[]).is_err());
        let t = Text { name: long, gds: (1, 0), x: 0, y: 0 };
        assert!(emit("t", &[], &[], &[t]).is_err());
        let fits = "x".repeat(MAX_PAYLOAD);
        let bytes = emit(&fits, &[], &[], &[]).unwrap();
        assert_eq!(records(&bytes).into_iter().find(|r| r.0 == STRNAME_R).unwrap().1.len(), MAX_PAYLOAD);
    }

    // Known encodings: 1.0 = 0x41 10 00…, 1/16 = 0x40 10 00…, sign bit for
    // a negative.
    #[test]
    fn gds_real_known_encodings() {
        assert_eq!(gds_real(1.0), [0x41, 0x10, 0, 0, 0, 0, 0, 0]);
        assert_eq!(gds_real(1.0 / 16.0), [0x40, 0x10, 0, 0, 0, 0, 0, 0]);
        assert_eq!(gds_real(-1.0), [0xC1, 0x10, 0, 0, 0, 0, 0, 0]);
        assert_eq!(gds_real(0.5), [0x40, 0x80, 0, 0, 0, 0, 0, 0]);
    }

    // 56 mantissa bits hold every f64 mantissa, so decoding is exact, over a
    // spread of magnitudes and signs.
    #[test]
    fn gds_real_is_lossless_for_f64() {
        fn read(b: [u8; 8]) -> f64 {
            let sign = if b[0] & 0x80 != 0 { -1.0 } else { 1.0 };
            let exp = i32::from(b[0] & 0x7f) - 64;
            let mant = b[1..].iter().enumerate().map(|(k, &x)| f64::from(x) / 256f64.powi(k as i32 + 1)).sum::<f64>();
            sign * mant * 16f64.powi(exp)
        }
        let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
        for _ in 0..2_000 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            let mant = 1.0 + (x >> 11) as f64 / (1u64 << 53) as f64;
            let exp = ((x % 120) as i32) - 60;
            let v = if x & 1 == 0 { mant } else { -mant } * 2f64.powi(exp);
            assert_eq!(read(gds_real(v)), v, "{v:e}");
        }
        for v in [1e-3, 1e-9] {
            assert_eq!(read(gds_real(v)), v);
        }
    }

    #[test]
    #[should_panic]
    fn gds_real_refuses_infinity() {
        let _ = gds_real(f64::INFINITY);
    }

    #[test]
    #[should_panic]
    fn gds_real_refuses_nan() {
        let _ = gds_real(f64::NAN);
    }

    // 1e-80 is below 16^-65 ≈ 5.3e-79: its exponent cannot be written.
    #[test]
    #[should_panic]
    fn gds_real_refuses_an_unrepresentable_exponent() {
        let _ = gds_real(1e-80);
    }
}
