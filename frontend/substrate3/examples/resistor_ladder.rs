//! Resistor ladder: series chain of matched unit resistors, serpentine folded.
//!
//! Shows: polygon() for non-rectangular shapes, hierarchical composition
//! with MY orientation for serpentine routing, unit-cell pattern.
//!
//! Run: `cargo run --example resistor_ladder -p substrate3`

use substrate3::*;

// ---------------------------------------------------------------------------
//  Unit resistor segment
// ---------------------------------------------------------------------------

struct UnitResistor {
    w: i32,   // body width (nm)
    l: i32,   // body length (nm)
}

impl CellGenerator for UnitResistor {
    fn generate(&self, b: &mut CellBuilder) -> Result<(), CellError> {
        let ct = 170;
        let head_l = 300; // head region for contacts
        let total_l = head_l + self.l + head_l;

        // Resistor body (poly for poly resistor, diff for diffusion resistor)
        b.rect("poly", 0, 0, self.w, total_l)?;

        // RPO (resistor protection oxide) — marks the resistive region
        // Only covers the body, not the heads
        b.rect("li", 0, head_l, self.w, self.l)?;

        // Head contacts (left/bottom and right/top)
        let cx = self.w / 2 - ct / 2;
        b.rect("li", cx, head_l / 2 - ct / 2, ct, ct)?;
        b.pin("A", "li", cx, head_l / 2 - ct / 2, ct, ct)?;

        b.rect("li", cx, head_l + self.l + head_l / 2 - ct / 2, ct, ct)?;
        b.pin("B", "li", cx, head_l + self.l + head_l / 2 - ct / 2, ct, ct)?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
//  Resistor ladder: N units in series, serpentine folded
// ---------------------------------------------------------------------------

struct ResistorLadder {
    w: i32,
    l: i32,
    n_units: u32,
    cols: u32, // units per row before folding
}

impl CellGenerator for ResistorLadder {
    fn ports(&self) -> Vec<PortDef> {
        // Taps at every junction + endpoints
        let mut ports = vec![PortDef::inout("T0")];
        for i in 1..=self.n_units {
            ports.push(PortDef::inout(format!("T{i}")));
        }
        ports
    }

    fn generate(&self, b: &mut CellBuilder) -> Result<(), CellError> {
        b.set_device_type(DeviceType::Res);
        b.set_pattern(PatternType::Single);

        let unit = UnitResistor { w: self.w, l: self.l };

        // Trial for unit dimensions
        let mut trial = CellBuilder::new(b.deck(), MatchingTier::None, 0);
        unit.generate(&mut trial)?;
        let ub = trial.compute_bbox();
        let uw = ub.width();
        let uh = ub.height();
        let col_pitch = uw + 200; // spacing between columns

        for i in 0..self.n_units {
            let col = i % self.cols;
            let row = i / self.cols;
            let x = col as i32 * col_pitch;
            let y = row as i32 * (uh + 400); // row spacing

            // Serpentine: odd rows are MY-flipped so B of row N
            // aligns with A of row N+1 for series connection
            let orient = if row % 2 == 0 {
                Orientation::R0
            } else {
                Orientation::MY
            };

            b.instance(&format!("R{i}"), &unit, x, y, orient)?;
        }

        b.set_electrical(
            self.w,
            self.l * self.n_units as i32,
            self.n_units as u16,
            self.w,
        );

        Ok(())
    }
}

fn main() {
    let deck = example_deck();

    // 8-unit ladder, 4 columns wide (2 rows serpentine)
    let ladder = ResistorLadder {
        w: 500,
        l: 2000,
        n_units: 8,
        cols: 4,
    };

    println!("Ports: {:?}", ladder.ports());

    let mut b = CellBuilder::new(&deck, MatchingTier::Moderate, 0);
    ladder.generate(&mut b).expect("generate ladder");
    let out = b.finish();

    println!("8-unit resistor ladder (4x2 serpentine):");
    println!("  bbox: {}x{} nm", out.bbox.width(), out.bbox.height());
    println!("  polygons: {}", out.store.poly_count());
    println!("  pins: {}", out.pins.len());
    println!("  total resistance segments: {}", out.meta.finger_count);
}

fn example_deck() -> Deck {
    let json = r#"{
        "layers": {
            "diff":  {"layer": 65, "datatype": 20},
            "poly":  {"layer": 66, "datatype": 20},
            "li":    {"layer": 67, "datatype": 20},
            "met1":  {"layer": 68, "datatype": 20}
        },
        "drc": {
            "off_grid": {"grid": 5}
        }
    }"#;
    Deck::from_json(json).expect("deck")
}
