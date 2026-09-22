//! Hierarchy acceptance: `CompBuilder::instantiate_comp` inside a real
//! elaboration, with the `DeviceGen::devices()` netlist path exercised.
//!
//! The existing `macro_master` unit test covers the *pin/net* half of
//! hierarchy; nothing covered the half that matters to the annotator and to
//! LVS: `BuiltComp::netlist` built from a child's forwarded `devices`. The
//! load-bearing property is that the schematic and `nets` share one `NetId`
//! numbering (`schematic.nets[i].name == nets[i]`) — a skew there silently
//! applies a routing rule to the wrong net.
//!
//! The leaf deliberately has an **internal** node (the cascode mid-point),
//! because two instances of the same leaf sharing it would be a short that no
//! other assertion here would see.

use library::{elaborate, ElabConfig};
use macro_master::{
    variants::Mos, AlignMode, Block, CompBuilder, Composition, GenError, InOut, Input, Io,
    Output, PortInfo, Process, Signal,
};
use pnr_core::{Device, DeviceKind, Netlist};

// ── leaf: a cascoded common-source leg ─────────────────────────────────────

#[derive(Default)]
struct LegIo {
    vin: Input<Signal>,
    vb: Input<Signal>,
    vout: Output<Signal>,
    vss: InOut<Signal>,
}
impl Io for LegIo {
    fn ports(&self) -> Vec<PortInfo> {
        vec![self.vin.port("vin"), self.vb.port("vb"), self.vout.port("vout"), self.vss.port("vss")]
    }
}

/// `m1` (input device) with `m2` cascoded on top. `m1.d`–`m2.s` is the mid
/// node: internal to the leaf, named by no port.
struct Leg;
impl Block for Leg {
    type Io = LegIo;
    fn name(&self) -> String {
        "leg".into()
    }
}
impl Composition for Leg {
    fn build<P: Process>(&self, c: &mut CompBuilder<P>) -> Result<(), GenError> {
        let vgap = c.process().rule("device_gap", 600);
        let mos = || Mos::new(DeviceKind::Nmos, 420, 150, 1);
        let i1 = c.instantiate("m1", &mos())?;
        let m1 = c.place(i1)?;
        let i2 = c.instantiate("m2", &mos())?;
        let m2 = c.place_by(i2, AlignMode::Above, &m1, vgap)?;
        c.connect(&m1.term("g"), "vin");
        c.connect(&m1.term("s"), "vss");
        c.connect(&m1.term("b"), "vss");
        c.connect(&m1.term("d"), &m2.term("s")); // the internal mid node
        c.connect(&m2.term("g"), "vb");
        c.connect(&m2.term("d"), "vout");
        c.connect(&m2.term("b"), "vss");
        Ok(())
    }
}

// ── parent: two legs, wired by the parent ──────────────────────────────────

#[derive(Default)]
struct TwinIo {
    inp: Input<Signal>,
    inn: Input<Signal>,
    outp: Output<Signal>,
    outn: Output<Signal>,
    vb: Input<Signal>,
    vss: InOut<Signal>,
}
impl Io for TwinIo {
    fn ports(&self) -> Vec<PortInfo> {
        vec![
            self.inp.port("inp"),
            self.inn.port("inn"),
            self.outp.port("outp"),
            self.outn.port("outn"),
            self.vb.port("vb"),
            self.vss.port("vss"),
        ]
    }
}

struct Twin;
impl Block for Twin {
    type Io = TwinIo;
    fn name(&self) -> String {
        "twin".into()
    }
}
impl Composition for Twin {
    fn build<P: Process>(&self, c: &mut CompBuilder<P>) -> Result<(), GenError> {
        let sep = c.process().rule("device_gap", 600);
        let i1 = c.instantiate_comp("x1", &Leg)?;
        let x1 = c.place(i1)?;
        let i2 = c.instantiate_comp("x2", &Leg)?;
        let x2 = c.place_by(i2, AlignMode::ToTheRight, &x1, sep)?;
        c.connect(&x1.term("vin"), "inp");
        c.connect(&x2.term("vin"), "inn");
        c.connect(&x1.term("vout"), "outp");
        c.connect(&x2.term("vout"), "outn");
        // Shared by the *parent* — both children's `vb`/`vss` must land on one
        // NetId each.
        c.connect(&x1.term("vb"), "vb");
        c.connect(&x2.term("vb"), "vb");
        c.connect(&x1.term("vss"), "vss");
        c.connect(&x2.term("vss"), "vss");
        Ok(())
    }
}

// ── the test ───────────────────────────────────────────────────────────────

fn net_of<'a>(nl: &Netlist, nets: &'a [String], dev: &str, term: &str) -> &'a str {
    let d: &Device = nl
        .devices
        .iter()
        .find(|d| d.name == dev)
        .unwrap_or_else(|| {
            panic!("no device {dev}, have {:?}", nl.devices.iter().map(|d| &d.name).collect::<Vec<_>>())
        });
    let id = d
        .terminals
        .iter()
        .find(|(t, _)| t == term)
        .unwrap_or_else(|| panic!("{dev} has no terminal {term}"))
        .1;
    &nets[id.0 as usize]
}

#[test]
fn hierarchical_composition_elaborates_with_a_correct_schematic() {
    let deck =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../pdks/sky130.json"))
            .expect("sky130 deck present");
    let pdk = verify::Pdk::from_json(&deck).expect("deck parses");

    let sol = elaborate(&Twin, &pdk, &ElabConfig::default()).expect("elaboration succeeds");
    assert_eq!(sol.macros.len(), 2, "two sub-composition instances");

    let nl = sol.schematic.as_ref().expect("hierarchy must still yield a schematic");

    // 1. Device count and qualified names — two leaves x two devices each.
    let mut names: Vec<&str> = nl.devices.iter().map(|d| d.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, ["x1.m1", "x1.m2", "x2.m1", "x2.m2"]);

    // 2. The NetId invariant: one numbering for the schematic, the nets and
    //    therefore `Routes`. A skew here mis-applies every routing rule.
    assert_eq!(nl.nets.len(), sol.nets.len(), "schematic and nets are the same numbering");
    for (i, n) in nl.nets.iter().enumerate() {
        assert_eq!(n.name, sol.nets[i], "NetId {i} names two different nets");
    }

    // 3. Ports the *parent* wired reach both children as the same net.
    let n = |dev: &str, t: &str| net_of(nl, &sol.nets, dev, t);
    assert_eq!(n("x1.m1", "G"), "inp");
    assert_eq!(n("x2.m1", "G"), "inn");
    assert_eq!(n("x1.m2", "D"), "outp");
    assert_eq!(n("x2.m2", "D"), "outn");
    assert_eq!(n("x1.m2", "G"), "vb", "child io port binds to the parent's net");
    assert_eq!(n("x2.m2", "G"), "vb", "...and is shared across both instances");
    assert_eq!(n("x1.m1", "S"), "vss");
    assert_eq!(n("x2.m1", "S"), "vss");

    // 4. The leaf's *internal* mid node: whole within an instance, private to
    //    it. Sharing it across x1/x2 would be a short.
    assert_eq!(n("x1.m1", "D"), n("x1.m2", "S"), "x1's mid node is one net");
    assert_eq!(n("x2.m1", "D"), n("x2.m2", "S"), "x2's mid node is one net");
    assert_ne!(n("x1.m1", "D"), n("x2.m1", "D"), "two instances must not share internals");

    // 5. Signoff runs against that schematic.
    let report = sol.signoff(&pdk).expect("signoff runs when a schematic exists");
    let lvs: Vec<&str> =
        report.hard_violations.iter().map(|v| v.rule.as_str()).filter(|r| r.contains("lvs")).collect();
    eprintln!(
        "hier signoff: {} hard violations, {} lvs: {lvs:?}",
        report.hard_violations.len(),
        lvs.len()
    );
}

