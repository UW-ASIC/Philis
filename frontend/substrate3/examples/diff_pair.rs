//! Differential pair: two matched NMOS in common-centroid ABBA layout.
//!
//! Shows: sub-cell composition, MX orientation for gradient cancellation,
//! port declarations, matching metadata.
//!
//! Run: `cargo run --example diff_pair -p substrate3`

use substrate3::*;

// ---------------------------------------------------------------------------
//  Unit finger: one gate stripe with S/D contacts
// ---------------------------------------------------------------------------

struct Finger {
    w: i32,
    l: i32,
}

impl CellGenerator for Finger {
    fn generate(&self, b: &mut CellBuilder) -> Result<(), CellError> {
        // Step 1: read construction dimensions and layer roles from the same
        // validated PDK context used by the backend's built-in generators.
        let pdk = b.pdk()?;
        let ct = pdk.contact;
        let poly_ext = pdk.poly_ext;
        let sd_w = pdk.sd_width;
        let layers = &pdk.layers;
        let pitch = sd_w + self.l + sd_w;

        // Step 2: emit device geometry through role names, never process names.
        b.rect(&layers.diff, 0, 0, pitch, self.w)?;

        // Poly gate (centered, extends beyond diff)
        let gx = sd_w;
        b.rect(&layers.poly, gx, -poly_ext, self.l, self.w + 2 * poly_ext)?;

        // Source contact (left S/D region)
        let cy = self.w / 2 - ct / 2;
        b.rect(&layers.li, sd_w / 2 - ct / 2, cy, ct, ct)?;
        b.pin("S", &layers.li, sd_w / 2 - ct / 2, cy, ct, ct)?;

        // Drain contact (right S/D region)
        let dx = gx + self.l + sd_w / 2 - ct / 2;
        b.rect(&layers.li, dx, cy, ct, ct)?;
        b.pin("D", &layers.li, dx, cy, ct, ct)?;

        // Gate contact (below active)
        b.pin("G", &layers.poly, gx, -poly_ext, self.l, poly_ext)?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
//  Differential pair: ABBA common-centroid
// ---------------------------------------------------------------------------

struct DiffPair {
    w: i32,
    l: i32,
}

impl CellGenerator for DiffPair {
    fn ports(&self) -> Vec<PortDef> {
        vec![
            PortDef::input("INP"),
            PortDef::input("INM"),
            PortDef::inout("S_TAIL"),
            PortDef::output("OUTP"),
            PortDef::output("OUTM"),
        ]
    }

    fn generate(&self, b: &mut CellBuilder) -> Result<(), CellError> {
        b.set_device_type(DeviceType::Nmos);
        b.set_pattern(PatternType::Cc1d);
        b.set_matching_group(0);
        b.set_electrical(self.w * 2, self.l, 4, self.w);

        let finger = Finger {
            w: self.w,
            l: self.l,
        };

        // Finger pitch: need enough room between fingers.
        // Compute from a trial generation.
        let context = b.context().ok_or(CellError::MissingAnalogParams)?;
        let mut trial = CellBuilder::with_context(context, MatchingTier::None, 0);
        finger.generate(&mut trial)?;
        let fb = trial.compute_bbox();
        let fp = fb.width() + 100; // finger pitch = finger width + spacing

        // ABBA common-centroid pattern:
        //   finger 0: device A (R0)
        //   finger 1: device B (R0)
        //   finger 2: device B (MX)  — mirrored for gradient cancellation
        //   finger 3: device A (MX)
        let placements = [
            ("A0", 0, Orientation::R0),
            ("B0", 1, Orientation::R0),
            ("B1", 2, Orientation::MX),
            ("A1", 3, Orientation::MX),
        ];

        for (name, col, orient) in placements {
            b.instance(name, &finger, col * fp, 0, orient)?;
        }

        // Dummy fingers at edges (field poly, no diff underneath)
        let pdk = b.pdk()?;
        let dummy_x_left = -fp;
        b.rect(
            &pdk.layers.poly,
            dummy_x_left + fp / 2 - self.l / 2,
            -pdk.poly_ext,
            self.l,
            self.w + 2 * pdk.poly_ext,
        )?;
        let dummy_x_right = 4 * fp;
        b.rect(
            &pdk.layers.poly,
            dummy_x_right + fp / 2 - self.l / 2,
            -pdk.poly_ext,
            self.l,
            self.w + 2 * pdk.poly_ext,
        )?;

        Ok(())
    }
}

// ---------------------------------------------------------------------------
//  Main
// ---------------------------------------------------------------------------

fn main() {
    let (deck, pdk) = example_process();
    let dp = DiffPair { w: 420, l: 150 };

    println!("Ports: {:?}", dp.ports());

    let mut b =
        CellBuilder::with_context(CellContext::new(&deck, &pdk), MatchingTier::Exceptional, 0);
    dp.generate(&mut b).expect("generate diff pair");
    let out = b.finish();

    println!("Diff pair generated:");
    println!("  bbox: {}x{} nm", out.bbox.width(), out.bbox.height());
    println!("  polygons: {}", out.store.poly_count());
    println!("  pins: {}", out.pins.len());
    println!("  pin names: {:?}", out.pin_map.keys().collect::<Vec<_>>());
    println!("  pattern: {:?}", out.meta.pattern);
    println!("  tier: {:?}", out.meta.tier);
}

fn example_process() -> (Deck, Pdk) {
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
        },
        "cell": {
            "layers": {"diff":"diff", "poly":"poly", "li":"li", "met1":"met1", "nwell":"nwell"},
            "contact": 170,
            "sd_width": 250,
            "poly_ext": 130
        }
    }"#;
    (
        Deck::from_json(json).expect("deck"),
        Pdk::from_json(json).expect("cell PDK"),
    )
}
