//! M6 acceptance: one hand-written five-transistor OTA generator, elaborated
//! against **two** decks — sky130 and the synthetic generic deck. Zero literal
//! nm anywhere except device W/L params; every spacing is a `rule(..)` lookup.
//! This is the substrate3 claim made checkable: same source, PDK swapped on
//! the fly.

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

/// Five-transistor OTA: mirrored NMOS diff pair, mirrored PMOS current-mirror
/// load above it, NMOS tail below.
struct Ota5T;

impl Block for Ota5T {
    type Io = OtaIo;
    fn name(&self) -> String {
        "ota5t".into()
    }
}

impl Composition for Ota5T {
    fn build<P: Process>(&self, c: &mut CompBuilder<P>) -> Result<(), GenError> {
        // Every dimension from the deck — no literal spacings. `device_gap` is
        // the deck's own inter-device construction dimension; separate nwells
        // must clear the deck's well-spacing rule.
        // ponytail: "NWELL.2" is a deck rule id, not a role name — add a
        // role-aliased construction dim (nwell_spacing) to the cell section
        // when a deck with different rule ids shows up.
        let sep = c.process().rule("device_gap", 600);
        let wellsep = sep.max(c.process().rule("NWELL.2", 1270));
        let vgap = sep.max(2 * c.process().rule("well_enclosure", 180));

        let nmos = || Mos::new(DeviceKind::Nmos, 420, 150, 1);
        let pmos = || Mos::new(DeviceKind::Pmos, 840, 150, 1);

        // Diff pair: m1 | axis | m2 (exact mirror symmetry).
        let i1 = c.instantiate("m1", &nmos())?;
        let m1 = c.place(i1)?;
        let i2 = c.instantiate("m2", &nmos())?;
        let m2 = c.place_mirrored(i2, &m1, sep)?;

        // Mirror load above, same axis discipline.
        let i3 = c.instantiate("m3", &pmos())?;
        let m3 = c.place_by(i3, AlignMode::Above, &m1, vgap)?;
        let i4 = c.instantiate("m4", &pmos())?;
        let m4 = c.place_mirrored(i4, &m3, wellsep)?;

        // Tail below the pair.
        let i5 = c.instantiate("m5", &nmos())?;
        let m5 = c.place_by(i5, AlignMode::Beneath, &m1, vgap)?;

        // Diff pair inputs and internal nodes.
        c.connect(&m1.term("g"), "inp");
        c.connect(&m2.term("g"), "inn");
        c.connect(&m1.term("s"), &m5.term("d")); // tail node (internal)
        c.connect(&m2.term("s"), &m1.term("s"));
        // Mirror load: m3 diode-connected, gates shared, m4 drives the output.
        c.connect(&m3.term("g"), &m3.term("d"));
        c.connect(&m4.term("g"), &m3.term("g"));
        c.connect(&m3.term("d"), &m1.term("d")); // left branch
        c.connect(&m4.term("d"), "vout");
        c.connect(&m2.term("d"), "vout");
        c.connect(&m3.term("s"), "vdd");
        c.connect(&m4.term("s"), "vdd");
        // Tail bias + rails.
        c.connect(&m5.term("g"), "vbias");
        c.connect(&m5.term("s"), "vss");
        // Bodies to their rails. Omitting these used to be invisible — nothing
        // read the composition as a circuit — but an untied body is a floating
        // net, and now that `schematic` exists LVS says so.
        for n in [&m1, &m2, &m5] {
            c.connect(&n.term("b"), "vss");
        }
        for p in [&m3, &m4] {
            c.connect(&p.term("b"), "vdd");
        }
        Ok(())
    }
}

/// Read the elaboration back out as a circuit and check it describes the OTA.
///
/// The netlist half of the claim the DRC half makes: the geometry is not just
/// rule-clean, it *extracts* as the circuit the generator declared.
/// `expect_model` is the deck's own nfet model name where the deck names a real
/// process — that is what makes the netlist PDK-correct by construction instead
/// of by a mapping table kept in step by hand.
fn check_netlist(
    sol: &library::Elaborated,
    pdk: &verify::Pdk,
    label: &str,
    expect_model: Option<&str>,
) {
    let spice = sol
        .netlist(pdk, verify::Detail::WithParasitics)
        .unwrap_or_else(|e| panic!("{label}: netlist export failed: {e}"));

    let subckt = spice
        .lines()
        .find(|l| l.starts_with(".subckt"))
        .unwrap_or_else(|| panic!("{label}: no .subckt line in:\n{spice}"));
    // With parasitics a port node is `vdd:0` — the net's first sub-node — so
    // match the stem, not the whole token.
    for name in ["inp", "inn", "vout", "vdd", "vss", "vbias"] {
        let found = subckt
            .split_whitespace()
            .any(|t| t == name || t.starts_with(&format!("{name}:")));
        assert!(found, "{label}: port {name} missing from {subckt:?}");
    }

    // ...and nothing else. An internal node on the port list is a different
    // cell from the one the generator declared, and it simulates, so nothing
    // downstream would catch it.
    let pins: Vec<&str> = subckt.split_whitespace().skip(2).collect();
    assert_eq!(
        pins.len(),
        6,
        "{label}: expected six pins, got {pins:?}"
    );

    let mos = spice.lines().filter(|l| l.starts_with('M')).count();
    assert_eq!(mos, 5, "{label}: expected five transistors, got:\n{spice}");

    assert!(
        spice.lines().any(|l| l.starts_with('C')),
        "{label}: asked for parasitics but got no capacitance card:\n{spice}"
    );

    if let Some(model) = expect_model {
        assert!(
            spice.contains(model),
            "{label}: device cards do not name the deck's model {model}:\n{spice}"
        );
    }

    let caps = spice.lines().filter(|l| l.starts_with('C')).count();
    let res = spice.lines().filter(|l| l.starts_with('R')).count();
    eprintln!("{label}: netlist {mos} devices, {res} R, {caps} C, {} lines", spice.lines().count());
    if std::env::var_os("PHILIS_DUMP_NETLIST").is_some() {
        let path = std::env::temp_dir().join(format!("{label}_ota5t.spice"));
        std::fs::write(&path, &spice).expect("dump netlist");
        eprintln!("{label}: wrote {}", path.display());
    }
}

fn elaborate_on(deck: &str, label: &str, expect_model: Option<&str>) {
    let pdk = verify::Pdk::from_json(deck).unwrap_or_else(|e| panic!("{label} deck parses: {e:?}"));
    let sol = elaborate(&Ota5T, &pdk, &ElabConfig::default())
        .unwrap_or_else(|e| panic!("{label}: elaboration failed: {e:?}"));
    assert_eq!(sol.macros.len(), 5, "{label}: five placed devices");
    // Every multi-terminal net must have drawn wires.
    for name in ["inp", "inn", "vout", "vdd", "vss", "vbias"] {
        let id = sol.nets.iter().position(|n| n == name);
        let id = id.unwrap_or_else(|| panic!("{label}: net {name} missing, nets: {:?}", sol.nets));
        assert!(
            !sol.routes.shapes(pnr_core::NetId(id as u16)).is_empty(),
            "{label}: net {name} unrouted ({} hard route violations)",
            sol.report.hard_violations.len()
        );
    }
    let drc = sol.signoff_drc(&pdk);
    // Gate what substrate3 *owns*: device-formation layers — geometry the
    // generator placed. Wire layers (li/met*/cuts) carry known `dr` debt (the
    // search flow shows the same class: suite chain4 DRC 44, all routing), and
    // met1 density needs fill, a signoff post-pass. Tighten to
    // `drc.is_empty()` when the dr pin-access/notch fixes land.
    const OWNED: &[&str] = &[":diff", ":poly", ":nwell", ":nsdm", ":psdm", ":tap", ":licon"];
    let own: Vec<(String, i64)> = drc
        .iter()
        .filter(|v| OWNED.iter().any(|l| v.rule.ends_with(l)))
        .map(|v| (v.rule.clone(), v.margin))
        .collect();
    assert!(own.is_empty(), "{label}: device-layer DRC violations: {own:?}");
    // The pin-access conductor is gated too: dr draws its li pads per-layer and
    // only at pin ends now, so a li violation is a regression, not debt. met1
    // notch/spacing/density stay excluded above (separate debt).
    let li: Vec<(String, i64)> = drc
        .iter()
        .filter(|v| v.rule.ends_with(":li"))
        .map(|v| (v.rule.clone(), v.margin))
        .collect();
    assert!(li.is_empty(), "{label}: pin-access-layer DRC violations: {li:?}");
    eprintln!("{label}: device layers clean; {} routing-layer violations (dr debt)", drc.len());
    check_netlist(&sol, &pdk, label, expect_model);
}

/// The elaboration reads back as the circuit that was written, on the *same*
/// net numbering as its routes — which is what lets the annotator's rules and
/// the LVS reference both apply to a hand-placed composition.
///
/// The load-bearing case is the tail node: `build` joins it terminal-to-terminal
/// (`m1.s` ↔ `m5.d`) and never names it, so it is an anonymous `net{i}`. Any
/// scheme that recovered the schematic by *matching names* against a separate
/// reference would drop exactly this net — and dropping it silently is worse
/// than having no rules, because the rules that remain look complete.
fn dev_names(n: &pnr_core::Netlist) -> Vec<&str> {
    n.devices.iter().map(|d| d.name.as_str()).collect()
}

#[test]
fn ota5t_schematic_is_the_circuit_on_the_routes_numbering() {
    let deck = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../pdks/sky130.json"
    ))
    .expect("sky130 deck present");
    let pdk = verify::Pdk::from_json(&deck).expect("deck parses");
    let sol = elaborate(&Ota5T, &pdk, &ElabConfig::default()).expect("elaboration");

    let schem = sol.schematic.as_ref().expect("every instance is a variants::Mos, so not opaque");
    assert_eq!(schem.devices.len(), 5, "five transistors: {:?}", dev_names(schem));

    // The invariant everything else rests on: one NetId space.
    let names: Vec<&str> = schem.nets.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(names, sol.nets, "schematic and routes must share one NetId space");

    let net = |dev: &str, term: &str| {
        let d = schem
            .devices
            .iter()
            .find(|d| d.name == dev)
            .unwrap_or_else(|| panic!("no device {dev} in {:?}", dev_names(schem)));
        d.terminals
            .iter()
            .find(|(t, _)| t == term)
            .unwrap_or_else(|| panic!("{dev} has no {term}: {:?}", d.terminals))
            .1
    };
    let port = |name: &str| {
        pnr_core::NetId(sol.nets.iter().position(|n| n == name).expect("declared port") as u16)
    };
    assert_eq!(net("m1", "G"), port("inp"), "m1 gate is the inp port");
    assert_eq!(net("m5", "S"), port("vss"), "m5 source is the vss rail");
    // The anonymous internal node, resolved through the connect binding.
    assert_eq!(net("m1", "S"), net("m5", "D"), "tail node joins m1.s and m5.d");
    assert_eq!(net("m2", "S"), net("m1", "S"), "…and m2.s with them");
    assert!(
        !sol.ports.contains(&sol.nets[net("m1", "S").0 as usize]),
        "the tail is internal, not a port"
    );

    // The netlist is recognisable: the annotator extracts real routing rules
    // from it, which is the whole reason elaborate() carries one.
    let reqs = annotator::annotate(
        schem,
        &annotator::NoInference,
        &annotator::AnnotationConfig::default(),
    )
    .routing;
    let batches = reqs.hard.len() + reqs.budget.len() + reqs.cost.len();
    assert!(batches > 0, "annotator extracted no routing rules from the elaborated netlist");

    // …and signoff runs against it rather than falling back to bare DRC.
    let report = sol.signoff(&pdk).expect("schematic present, so signoff runs");
    let lvs = report.hard_violations.iter().filter(|v| v.rule.contains("lvs")).count();
    eprintln!(
        "ota5t signoff: {} hard ({lvs} lvs), {} budget, cost {:.3}; {batches} routing rule batches",
        report.hard_violations.len(),
        report.budget_violations.len(),
        report.cost
    );
    assert_eq!(lvs, 0, "LVS must match the declared schematic");
}

#[test]
fn ota5t_clean_on_sky130() {
    let deck = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../pdks/sky130.json"
    ))
    .expect("sky130 deck present");
    elaborate_on(&deck, "sky130", Some("sky130_fd_pr__nfet_01v8"));
}

#[test]
fn ota5t_clean_on_generic_finfet() {
    let deck = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../pdks/generic_finfet.json"
    ))
    .expect("generic_finfet deck present");
    // The synthetic deck names no real foundry models, so only the structure is
    // gated here; sky130 is where the model-name claim is checkable.
    elaborate_on(&deck, "generic_finfet", None);
}

