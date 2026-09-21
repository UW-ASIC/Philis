//! Dump an elaborated composition as GDS + reference SPICE, so an
//! **independent** extractor/comparator (magic + netgen) can render a second
//! LVS verdict on the same layout `verify` judges.
//!
//! The point is not to check the layout — `Elaborated::signoff` does that — but
//! to check the *checker*: a disagreement between gdsverify and magic/netgen is
//! a finding about one of them.
//!
//! `cargo run -p library --example xlvs_dump -- <deck.json> <outdir>`

use library::{elaborate, ElabConfig};
use macro_master::{
    variants::Mos, AlignMode, Block, CompBuilder, Composition, GenError, InOut, Input, Io,
    Output, PortInfo, Process, Signal,
};
use pnr_core::DeviceKind;

#[derive(Default)]
struct OtaIo {
    inp: Input<Signal>,
    inn: Input<Signal>,
    vout: Output<Signal>,
    vbias: Input<Signal>,
    vdd: InOut<Signal>,
    vss: InOut<Signal>,
}
impl Io for OtaIo {
    fn ports(&self) -> Vec<PortInfo> {
        vec![
            self.inp.port("inp"),
            self.inn.port("inn"),
            self.vout.port("vout"),
            self.vbias.port("vbias"),
            self.vdd.port("vdd"),
            self.vss.port("vss"),
        ]
    }
}

/// Byte-for-byte the `Ota5T` of `tests/ota_cross_pdk.rs`.
struct Ota5T;
impl Block for Ota5T {
    type Io = OtaIo;
    fn name(&self) -> String {
        "ota5t".into()
    }
}
impl Composition for Ota5T {
    fn build<P: Process>(&self, c: &mut CompBuilder<P>) -> Result<(), GenError> {
        let sep = c.process().rule("device_gap", 600);
        let wellsep = sep.max(c.process().rule("NWELL.2", 1270));
        let vgap = sep.max(2 * c.process().rule("well_enclosure", 180));
        let nmos = || Mos::new(DeviceKind::Nmos, 420, 150, 1);
        let pmos = || Mos::new(DeviceKind::Pmos, 840, 150, 1);

        let i1 = c.instantiate("m1", &nmos())?;
        let m1 = c.place(i1)?;
        let i2 = c.instantiate("m2", &nmos())?;
        let m2 = c.place_mirrored(i2, &m1, sep)?;
        let i3 = c.instantiate("m3", &pmos())?;
        let m3 = c.place_by(i3, AlignMode::Above, &m1, vgap)?;
        let i4 = c.instantiate("m4", &pmos())?;
        let m4 = c.place_mirrored(i4, &m3, wellsep)?;
        let i5 = c.instantiate("m5", &nmos())?;
        let m5 = c.place_by(i5, AlignMode::Beneath, &m1, vgap)?;

        c.connect(&m1.term("g"), "inp");
        c.connect(&m2.term("g"), "inn");
        c.connect(&m1.term("s"), &m5.term("d"));
        c.connect(&m2.term("s"), &m1.term("s"));
        c.connect(&m3.term("g"), &m3.term("d"));
        c.connect(&m4.term("g"), &m3.term("g"));
        c.connect(&m3.term("d"), &m1.term("d"));
        c.connect(&m4.term("d"), "vout");
        c.connect(&m2.term("d"), "vout");
        c.connect(&m3.term("s"), "vdd");
        c.connect(&m4.term("s"), "vdd");
        c.connect(&m5.term("g"), "vbias");
        c.connect(&m5.term("s"), "vss");
        for n in [&m1, &m2, &m5] {
            c.connect(&n.term("b"), "vss");
        }
        for p in [&m3, &m4] {
            c.connect(&p.term("b"), "vdd");
        }
        Ok(())
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let deck_path = args.next().expect("usage: xlvs_dump <deck.json> <outdir>");
    let out = std::path::PathBuf::from(args.next().expect("outdir"));
    std::fs::create_dir_all(&out).expect("mkdir outdir");

    let deck = std::fs::read_to_string(&deck_path).expect("read deck");
    let pdk = verify::Pdk::from_json(&deck).expect("parse deck");
    let epochs: u32 = std::env::var("XLVS_EPOCHS").ok().and_then(|s| s.parse().ok()).unwrap_or(4);
    let sol = elaborate(&Ota5T, &pdk, &ElabConfig { epochs, ..ElabConfig::default() }).expect("elaborate");

    // GDS on the deck's own layer/datatype numbers, so magic reads it with the
    // stock sky130A techfile and no mapping table in between.
    let layer_gds = pdk.layer_gds();
    let shapes = sol.geometry();
    std::fs::write(out.join("ota5t.gds"), library::gds::emit(&shapes, &layer_gds))
        .expect("write gds");

    // The reference, as SPICE, straight from the declared schematic — one card
    // per device (netgen does its own parallel reduction, so no finger
    // expansion here, unlike gdsverify's RefInput).
    let schem = sol.schematic.as_ref().expect("Mos variants declare their devices");
    let mut sp = String::from("* reference: Ota5T as declared by the composition\n");
    sp.push_str(".subckt ota5t ");
    for p in &sol.ports {
        sp.push_str(p);
        sp.push(' ');
    }
    sp.push('\n');
    for d in &schem.devices {
        let t = |name: &str| {
            let id = d.terminals.iter().find(|(k, _)| k == name).expect("terminal").1;
            sol.nets[id.0 as usize].clone()
        };
        let model = match d.kind {
            DeviceKind::Nmos => "sky130_fd_pr__nfet_01v8",
            DeviceKind::Pmos => "sky130_fd_pr__pfet_01v8",
            k => panic!("unexpected kind {k:?}"),
        };
        let p = |k: &str| d.params.iter().find(|(n, _)| n == k).map_or(0, |&(_, v)| v);
        // nm → µm, the unit sky130 model cards use.
        sp.push_str(&format!(
            "X{} {} {} {} {} {} w={}u l={}u nf={}\n",
            d.name,
            t("D"),
            t("G"),
            t("S"),
            t("B"),
            model,
            p("w") as f64 / 1000.0,
            p("l") as f64 / 1000.0,
            p("nf").max(1),
        ));
    }
    sp.push_str(".ends\n");
    std::fs::write(out.join("ota5t_ref.spice"), &sp).expect("write ref spice");

    // gdsverify's own verdict on the very same geometry, for the comparison.
    eprintln!("ROUTE REPORT: {} hard", sol.report.hard_violations.len());
    for v in &sol.report.hard_violations { eprintln!("  RV {} margin={}", v.rule, v.margin); }
    let report = sol.signoff(&pdk).expect("schematic present");
    let lvs: Vec<&str> =
        report.hard_violations.iter().map(|v| v.rule.as_str()).filter(|r| r.contains("lvs")).collect();
    std::fs::write(
        out.join("gdsverify.txt"),
        format!("lvs findings: {}\n{}\n", lvs.len(), lvs.join("\n")),
    )
    .expect("write verdict");
    // Magic cannot name a net the GDS never labelled. Hand it exactly the
    // labels signoff uses — not raw pin centres, which land in space wherever
    // the drawn pad is not under the pin rect.
    let magic_of = |l: u16| -> &'static str {
        for (deck, magic) in [("li", "li"), ("met1", "m1"), ("met2", "m2"), ("met3", "m3"),
                              ("met4", "m4"), ("met5", "m5"), ("poly", "poly")] {
            if pdk.layer(deck).map(|i| i.0) == Some(l) { return magic; }
        }
        "?"
    };
    let mut tcl = String::new();
    for lp in sol.net_labels(&pdk) {
        tcl.push_str(&format!(
            "box {}um {}um {}um {}um\nlabel {} 0 {}\n",
            lp.x as f64 / 1000.0, lp.y as f64 / 1000.0,
            lp.x as f64 / 1000.0, lp.y as f64 / 1000.0,
            lp.name, magic_of(lp.layer)));
        println!("label {} on {} at {},{}", lp.name, magic_of(lp.layer), lp.x, lp.y);
    }
    std::fs::write(out.join("labels.tcl"), &tcl).expect("write labels");
    // Which pins does a routed wire actually land on?
    let names = ["m1","m2","m3","m4","m5"];
    for (i, m) in sol.macros.iter().enumerate() {
        for p in &m.pins {
            // Layer-agnostic: the router lands on met1+ and drops a via stack,
            // so a wire never shares the li pin's own layer.
            let hit = sol.routes.shapes(p.net).iter().any(|w|
                   w.rect.x <= p.at.x + p.at.w && p.at.x <= w.rect.x + w.rect.w
                && w.rect.y <= p.at.y + p.at.h && p.at.y <= w.rect.y + w.rect.h);
            println!("PIN {} {} net={} at=({},{}) routed={}",
                names.get(i).copied().unwrap_or("?"), p.name,
                sol.nets[p.net.0 as usize], p.at.x, p.at.y, hit);
        }
    }
    println!("wrote {} shapes, {} ref devices, gdsverify lvs findings {}",
        shapes.len(), schem.devices.len(), lvs.len());
}
