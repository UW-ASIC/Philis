//! Current mirror: 1:2 ratio via unit-finger replication.
//!
//! Shows: ratioed layout, unit-cell reuse with instance(), closure
//! generators for simple sub-cells.
//!
//! Run: `cargo run --example current_mirror -p substrate3`

use substrate3::*;

// ---------------------------------------------------------------------------
//  Unit finger (same pattern as diff_pair, factored for reuse)
// ---------------------------------------------------------------------------

struct UnitFinger {
    w: i32,
    l: i32,
}

impl CellGenerator for UnitFinger {
    fn generate(&self, b: &mut CellBuilder) -> Result<(), CellError> {
        let ct = 170;
        let poly_ext = 130;
        let sd_w = 250;
        let pitch = sd_w + self.l + sd_w;

        b.rect("diff", 0, 0, pitch, self.w)?;
        b.rect("poly", sd_w, -poly_ext, self.l, self.w + 2 * poly_ext)?;

        let cy = self.w / 2 - ct / 2;
        b.rect("li", sd_w / 2 - ct / 2, cy, ct, ct)?;
        b.pin("S", "li", sd_w / 2 - ct / 2, cy, ct, ct)?;

        let dx = sd_w + self.l + sd_w / 2 - ct / 2;
        b.rect("li", dx, cy, ct, ct)?;
        b.pin("D", "li", dx, cy, ct, ct)?;

        b.pin("G", "poly", sd_w, -poly_ext, self.l, poly_ext)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
//  Current mirror: 1 ref + 2 copy fingers, interdigitated
// ---------------------------------------------------------------------------

struct CurrentMirror {
    w: i32,
    l: i32,
    ratio: u32, // copy:ref ratio (e.g. 2 for 1:2)
}

impl CellGenerator for CurrentMirror {
    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef::inout("IREF"),
            PortDef::output("IOUT"),
            PortDef::inout("VDD"),
        ]
    }

    fn generate(&self, b: &mut CellBuilder) -> Result<(), CellError> {
        b.set_device_type(DeviceType::Pmos);
        b.set_pattern(PatternType::Interdig);
        b.set_matching_group(0);

        let uf = UnitFinger { w: self.w, l: self.l };
        let total_fingers = 1 + self.ratio; // ref + copies

        // Trial generation for finger pitch
        let mut trial = CellBuilder::new(b.deck(), MatchingTier::None, 0);
        uf.generate(&mut trial)?;
        let fp = trial.compute_bbox().width() + 100;

        // Interdigitated sequence for 1:2 mirror: REF COPY REF_dummy COPY
        // Simple greedy: alternate ref and copy to distribute.
        // For 1:2: place as COPY REF COPY (centroid-symmetric)
        let sequence: Vec<&str> = match self.ratio {
            1 => vec!["REF", "COPY"],
            2 => vec!["COPY", "REF", "COPY"],
            3 => vec!["COPY", "REF", "COPY", "REF_D"],
            _ => {
                // General: interleave
                let mut s = Vec::new();
                for i in 0..total_fingers {
                    s.push(if i % (self.ratio + 1) == 0 { "REF" } else { "COPY" });
                }
                s
            }
        };

        for (col, _dev) in sequence.iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            let x = col as i32 * fp;
            b.instance(&format!("f{col}"), &uf, x, 0, Orientation::R0)?;
        }

        // Shared gate strap across all fingers (met1)
        let gate_y = -130;
        let total_w = sequence.len() as i32 * fp;
        b.rect("met1", 0, gate_y, total_w, 170)?;
        b.pin("G", "met1", 0, gate_y, total_w, 170)?;

        b.set_electrical(
            self.w * total_fingers as i32,
            self.l,
            total_fingers as u16,
            self.w,
        );

        Ok(())
    }
}

fn main() {
    let deck = example_deck();
    let mirror = CurrentMirror { w: 840, l: 150, ratio: 2 };

    println!("Ports: {:?}", mirror.ports());

    let mut b = CellBuilder::new(&deck, MatchingTier::Moderate, 0);
    mirror.generate(&mut b).expect("generate mirror");
    let out = b.finish();

    println!("1:2 Current mirror generated:");
    println!("  bbox: {}x{} nm", out.bbox.width(), out.bbox.height());
    println!("  polygons: {}", out.store.poly_count());
    println!("  pins: {}", out.pins.len());
    println!("  total fingers: {}", out.meta.finger_count);
    println!("  finger width: {} nm", out.meta.finger_width);
}

fn example_deck() -> Deck {
    let json = r#"{
        "layers": {
            "diff":  {"layer": 65, "datatype": 20},
            "poly":  {"layer": 66, "datatype": 20},
            "li":    {"layer": 67, "datatype": 20},
            "met1":  {"layer": 68, "datatype": 20},
            "nwell": {"layer": 64, "datatype": 20}
        },
        "drc": {
            "off_grid": {"grid": 5}
        }
    }"#;
    Deck::from_json(json).expect("deck")
}
