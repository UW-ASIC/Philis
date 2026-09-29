//! A device a later stage inserts (an antenna diode) signs off through
//! [`library::adopt_devices`]: its LVS entry comes from the schematic, its
//! shapes from the macro.

use pnr_core::{DeviceKind, Macro, Process, Rect};

fn pdk() -> Option<verify::Pdk> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    verify::Pdk::from_json(&std::fs::read_to_string(root.join("pdks/sky130.json")).ok()?).ok()
}

fn lvs(sol: &library::Solution, pdk: &verify::Pdk) -> Vec<String> {
    library::signoff(sol, pdk).hard_violations.into_iter().map(|v| v.rule).filter(|r| r.starts_with("lvs")).collect()
}

/// The flow draws and routes a gate-to-ground diode; lifted out and adopted
/// back as an inserted device (cathode = gate net, anode = ground) it keeps
/// LVS at MATCH, and its macro without the reference entry does not.
#[test]
fn an_adopted_antenna_diode_keeps_lvs_matched() {
    let Some(pdk) = pdk() else { return };
    let spice = ".subckt pd d g VSS\nXM1 d g VSS VSS nfet_01v8 W=2u L=0.5u\nXM2 d g VSS VSS nfet_01v8 W=2u L=0.5u\n\
                 XD1 VSS g sky130_fd_pr__diode_pw2nd_05v5 W=0.5u L=1u\n.ends pd\n";
    let cfg = library::Config { feedback_iters: 1, ..Default::default() };
    let mut sol = library::run(spice, &pdk, &library::Macros::default(), &cfg).expect("flow");
    assert!(lvs(&sol, &pdk).is_empty(), "the routed diode matches: {:?}", lvs(&sol, &pdk));

    // Lift it out: its cell keeps its slot (empty), the schematic loses it.
    let cell = (0..sol.layout.x.len())
        .find(|&c| sol.macros[c].pins.iter().any(|p| p.name.ends_with(":N")))
        .expect("the diode's cell");
    let placed = pnr_core::place_macros(&sol.macros, &sol.layout).swap_remove(cell);
    sol.macros[cell] = Macro { shapes: vec![], pins: vec![], bbox: Rect { w: 0, h: 0, ..placed.bbox }, units: vec![], dummies: Vec::new() };
    let at = sol.netlist.devices.iter().position(|d| d.kind == DeviceKind::Diode).unwrap();
    let diode = sol.netlist.devices.remove(at);

    // Macro alone: the layout has a diode the reference lacks.
    sol.macros.push(placed.clone());
    assert!(!lvs(&sol, &pdk).is_empty(), "an unreferenced diode must not match");
    sol.macros.pop();

    // LVS does not check diode polarity (gpurify, fail-open), so check it from
    // the geometry: the cathode pin sits on n+ diffusion and carries the gate
    // net, the anode pin on the p+ tap and carries ground.
    let net = |name: &str| sol.netlist.nets.iter().position(|n| n.name == name).unwrap();
    let doped = |term: &str, implant: &str| {
        let p = placed.pins.iter().find(|p| p.name.ends_with(term)).unwrap();
        let imp = pdk.layer(implant).unwrap();
        let (x, y) = (p.at.x + p.at.w / 2, p.at.y + p.at.h / 2);
        let under = placed.shapes.iter().any(|s| s.layer == imp && (s.rect.x..s.rect.x + s.rect.w).contains(&x) && (s.rect.y..s.rect.y + s.rect.h).contains(&y));
        (usize::from(p.net.0), under)
    };
    assert_eq!(doped(":N", "nsdm"), (net("g"), true), "cathode: n+ on the gate net");
    assert_eq!(doped(":P", "psdm"), (net("VSS"), true), "anode: p+ tap on ground");

    library::adopt_devices(&mut sol.netlist, &mut sol.macros, vec![(diode, placed)]);
    assert!(lvs(&sol, &pdk).is_empty(), "adopted: {:?}", lvs(&sol, &pdk));
}
