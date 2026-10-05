//! An antenna the jumper cannot fix gets a diode (Hastings pp. 228–229): dr's
//! epoch inserts it beside the gate, routes it in, and the schematic adopts it
//! so LVS still matches.

use pnr_core::{DeviceKind, Process};

/// sky130 with every antenna rule of its deck made impossibly tight: no
/// route, jumpered or not, stays under it, so only a diode can clear the
/// gate net. Each row becomes an `antenna_electrical` crediting the deck's
/// diode marker (`diode_credit: 0`, a bonus no ratio reaches), so the diode is
/// one the deck credits and the flow inserts it.
fn tight_sky130() -> Option<verify::Pdk> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sidecar = std::fs::read_to_string(root.join("pdks/sky130.json")).ok()?;
    let deck = verify::Pdk::deck_text(&sidecar).ok()?;
    let base = verify::Pdk::load(&deck, &sidecar).ok()?;
    let marker = base.layer("diom")?;
    let diom = &base.layers.iter().find(|(_, l)| *l == marker)?.0;
    let tight: Vec<String> = deck
        .lines()
        .map(|l| match (l.find(" antenna("), l.find("max_ratio: ")) {
            (Some(k), Some(at)) => format!(
                "{} antenna_electrical({}max_ratio: 0.001, diode: {diom}, diode_credit: 0, diode_bonus: 1000000)",
                &l[..k],
                &l[k + " antenna(".len()..at]
            ),
            _ => l.to_string(),
        })
        .collect();
    let pdk = verify::Pdk::load(&tight.join("\n"), &sidecar).ok()?;
    assert_eq!(pdk.antenna_diode_credit(), Some((marker, 1_000_000.0)), "the rewritten deck credits its diode");
    Some(pdk)
}

#[test]
fn an_antenna_the_jumper_cannot_fix_gets_a_diode_and_lvs_matches() {
    let pdk = tight_sky130().expect("sky130 with tightened antenna stages loads");
    let spice = ".subckt ant d g VSS\nXM1 d g VSS VSS nfet_01v8 W=2u L=0.5u\nXM2 d g VSS VSS nfet_01v8 W=2u L=0.5u\n.ends ant\n";
    let cfg = library::Config { feedback_iters: 1, outer_iters: 1, ..Default::default() };
    let sol = library::run(spice, &pdk, &library::Macros::default(), &cfg).expect("flow");
    let net = |name: &str| sol.netlist.nets.iter().position(|n| n.name == name).unwrap();

    // The schematic adopted one diode: cathode on the gate net, anode on ground.
    let diode = sol.netlist.devices.iter().find(|d| d.kind == DeviceKind::Diode).expect("a diode was inserted");
    let term = |t: &str| diode.terminals.iter().find(|x| x.0 == t).map(|x| usize::from(x.1 .0));
    assert_eq!((term("N"), term("P")), (Some(net("g")), Some(net("VSS"))));

    // Polarity from the geometry (LVS does not check it): the cathode pin sits
    // on n+ diffusion, the anode pin on the p+ tap.
    let cell = sol.macros[sol.layout.x.len()..].iter().find(|m| m.pins.iter().any(|p| p.name.ends_with(":N"))).expect("the diode's macro");
    let on = |suffix: &str, implant: &str| {
        let p = cell.pins.iter().find(|p| p.name.ends_with(suffix)).unwrap();
        let imp = pdk.layer(implant).unwrap();
        let (x, y) = (p.at.x + p.at.w / 2, p.at.y + p.at.h / 2);
        (usize::from(p.net.0), cell.shapes.iter().any(|s| s.layer == imp && (s.rect.x..s.rect.x + s.rect.w).contains(&x) && (s.rect.y..s.rect.y + s.rect.h).contains(&y)))
    };
    assert_eq!(on(":N", "nsdm"), (net("g"), true), "cathode: n+ on the gate net");
    assert_eq!(on(":P", "psdm"), (net("VSS"), true), "anode: p+ tap on ground");

    // dr scored before the diode's marker joined the gate net: the rows that
    // ship are re-derived on the shipped routes (RTE-23), and the tight deck
    // leaves at least one, so the comparison is not empty on both sides.
    // ponytail: here the marker touches no routed piece, so the re-derived row
    // equals dr's; a fixture whose marker lowers a routed ratio would make
    // this fail without the re-derive.
    let rows = |v: &mut dyn Iterator<Item = &pnr_core::Violation>| v.filter(|v| v.is_batch_row()).map(|v| (v.rule.clone(), v.margin)).collect::<Vec<_>>();
    let (hard, budget) = gr::analog_tiers(&sol.routes, &sol.routing);
    let got = rows(&mut sol.route.hard_violations.iter().chain(&sol.route.budget_violations));
    assert_eq!(got, rows(&mut hard.iter().chain(&budget)), "dr rows describe the routes that ship");
    assert!(!got.is_empty(), "no batch row: the comparison checks nothing");
    let report = library::signoff(&sol, &pdk).report;
    let lvs: Vec<&String> = report.hard_violations.iter().map(|v| &v.rule).filter(|r| r.starts_with("lvs")).collect();
    assert!(lvs.is_empty(), "the inserted diode matches: {lvs:?}");
}
