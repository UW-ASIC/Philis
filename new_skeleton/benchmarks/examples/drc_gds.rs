//! Run GPurify DRC/ERC over a GDS file and print located findings, one per
//! line, in `drc_located.txt` format (`rule\tlayer\tmargin=..\t(x, y)`).
//!
//!     cargo run --release -p benchmark --example drc_gds -- <file.gds> [pdk.json]
//!
//! The differential harness (`benchmarks/xcheck.py --selftest`) feeds this a
//! KLayout-synthesised layout with known violations, so GPurify and KLayout
//! are validated against each other on geometry neither produced.
//!
//! Own 60-line reader on purpose: the visualizer's `parse_gds` drops the GDS
//! datatype, and this deck distinguishes layers by it (65/20 diff vs 65/44
//! tap). Rect-only — both the Philis writer and the self-test emit rects.

use verify::Pdk;

fn be16(b: &[u8]) -> u16 {
    u16::from_be_bytes([b[0], b[1]])
}
fn be32(b: &[u8]) -> i32 {
    i32::from_be_bytes([b[0], b[1], b[2], b[3]])
}

/// Flat rect list from a GDS stream: `(layer, datatype, x, y, w, h)`.
/// Bounding box of each BOUNDARY/BOX element; fine for rectilinear rects.
fn read_rects(data: &[u8]) -> Vec<(u16, u16, i32, i32, i32, i32)> {
    let mut out = Vec::new();
    let (mut layer, mut dtype) = (0u16, 0u16);
    let mut i = 0usize;
    while i + 4 <= data.len() {
        let len = be16(&data[i..]) as usize;
        if len < 4 || i + len > data.len() {
            break;
        }
        let (rec, body) = (be16(&data[i + 2..]), &data[i + 4..i + len]);
        match rec {
            0x0D02 => layer = be16(body),                     // LAYER
            0x0E02 | 0x1602 => dtype = be16(body),            // DATATYPE/TEXTTYPE
            0x1003 => {
                // XY — take the bbox.
                let n = body.len() / 8;
                if n >= 2 {
                    let (mut x0, mut y0) = (i32::MAX, i32::MAX);
                    let (mut x1, mut y1) = (i32::MIN, i32::MIN);
                    for k in 0..n {
                        let x = be32(&body[k * 8..]);
                        let y = be32(&body[k * 8 + 4..]);
                        x0 = x0.min(x);
                        y0 = y0.min(y);
                        x1 = x1.max(x);
                        y1 = y1.max(y);
                    }
                    if x1 > x0 && y1 > y0 {
                        out.push((layer, dtype, x0, y0, x1 - x0, y1 - y0));
                    }
                }
            }
            _ => {}
        }
        i += len;
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: drc_gds <file.gds> [pdk.json]");
        std::process::exit(2);
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let deck_path = args
        .get(2)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.join("pdks/sky130.json"));
    let deck_json = std::fs::read_to_string(&deck_path).expect("deck");
    let pdk = Pdk::from_json(&deck_json).expect("deck parse");

    // (gds layer, datatype) -> deck LayerId, straight from the deck's own map.
    let table: serde_json::Value = serde_json::from_str(&deck_json).expect("json");
    let mut ld: Vec<(u16, u16, u16)> = Vec::new(); // (layer, dtype, LayerId.0)
    for (name, v) in table["layers"].as_object().expect("layers").iter() {
        let l = v[0].as_u64().unwrap() as u16;
        let d = v[1].as_u64().unwrap() as u16;
        if let Some(id) = pdk.layers.iter().find(|(n, _)| n == name).map(|(_, id)| id.0) {
            ld.push((l, d, id));
        }
    }

    let bytes = std::fs::read(&args[1]).expect("gds");
    let mut shapes = Vec::new();
    let mut unknown = 0usize;
    for (l, d, x, y, w, h) in read_rects(&bytes) {
        match ld.iter().find(|&&(ll, dd, _)| ll == l && dd == d) {
            Some(&(_, _, id)) => shapes.push(pnr_core::Shape {
                layer: pnr_core::LayerId(id),
                rect: pnr_core::Rect { x, y, w, h },
            }),
            None => unknown += 1,
        }
    }
    eprintln!("drc_gds: {} shapes ({} on unknown layers)", shapes.len(), unknown);

    for f in verify::drc(&shapes, &[], &pdk) {
        println!("{}\t{}\tmargin={} nm\t({}, {})", f.rule, f.layer, f.margin_nm, f.x, f.y);
    }
    for f in verify::erc(&shapes, &[], &pdk) {
        println!("{}\t{}\tmargin={} nm\t({}, {})", f.rule, f.layer, f.margin_nm, f.x, f.y);
    }
}
