//! The LVS reference lists what the cells drew (`Macro::drawn`), not the
//! schematic card: a segmented resistor extracts one resistor per segment.

use analog::cell::{SeriesParallel, Unitization};
use cells::resistor::Resistor;
use cells::{Cell, Pattern};
use pnr_core::{DeviceGroup, DeviceId, DeviceKind, DrawnKind, Macro, Node};

fn pdk() -> verify::Pdk {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let json = std::fs::read_to_string(root.join("pdks/sky130.json")).expect("pdks/sky130.json is in the repo");
    verify::Pdk::from_json(&json).expect("pdks/sky130.json loads")
}

/// `XR1 a b` alone (two single-pin nets), then its cell forced to the
/// 2-segment variant: redrawn by the generator at the flow's own sizing and
/// recipe, pins and drawn devices bound as the flow bound them, placed
/// unturned at the flow's centre. The router's wires are only access pads on
/// the old variant's pins; they go, so the geometry is the cell alone.
fn two_segment_resistor(pdk: &verify::Pdk) -> library::Solution {
    let spice = ".subckt r a b\nXR1 a b sky130_fd_pr__res_high_po w=0.69u l=40u\n.ends r\n";
    let cfg = library::Config { feedback_iters: 1, ..Default::default() };
    let mut sol = library::run(spice, pdk, &library::Macros::default(), &cfg).expect("flow");
    assert_eq!(sol.layout.x.len(), 1, "one cell");
    let group = DeviceGroup { devices: vec![DeviceId(0)] };
    let c = analog::Constraints {
        unitization: vec![Unitization {
            devices: group.devices.clone(),
            device_type: DeviceKind::Resistor,
            dev_nf: vec![1],
            target_ratio: vec![1],
            unit_w: 690,
            unit_l: 40_000,
            series_parallel: SeriesParallel::Series,
            dummy_required: false,
            route_matching_required: false,
            class: None, series: Vec::new(), style: None,
        }],
        ..Default::default()
    };
    let recipe = pdk.recipe("resistor", &sol.netlist.devices[0].model).expect("high_po recipe");
    let overlay = verify::pdk::Overlay { pdk, recipe };
    let variants = Resistor::enumerate(&group, &c, &overlay);
    let v = variants.iter().position(|r| r.segments == 2 && r.pattern == Pattern::Single).expect("an n=2 variant");
    let mut m: Macro = variants[v].draw(&group, &c, &overlay);
    let flow = &sol.macros[0];
    for p in &mut m.pins {
        p.net = flow.pins.iter().find(|q| q.name == p.name).expect("the flow's pin of that name").net;
    }
    let device = flow.drawn[0].device;
    assert_eq!(device, Some(DeviceId(0)), "the flow bound its drawn segment");
    for d in &mut m.drawn {
        d.device = device;
    }
    sol.layout.orient[0] = pnr_core::Orient::R0;
    sol.layout.hw[0] = m.bbox.w / 2;
    sol.layout.hh[0] = m.bbox.h / 2;
    sol.macros[0] = m;
    sol.routes.wires.iter_mut().for_each(Vec::clear);
    sol
}

fn lvs_rows(sol: &library::Solution, pdk: &verify::Pdk) -> Vec<String> {
    let (shapes, pins, reference) = library::signoff_inputs(sol, pdk);
    let report = verify::signoff_with_intent(&shapes, &pins, &reference, &sol.intent, pdk).0;
    let rows: Vec<String> = report.hard_violations.into_iter().map(|v| v.rule).collect();
    // LVS that never ran is not a match.
    assert!(!rows.iter().any(|r| r.starts_with("engine/lvs") || r.starts_with("engine/reference") || r.starts_with("engine/run")), "{rows:?}");
    rows.into_iter().filter(|r| r.starts_with("lvs/")).collect()
}

#[test]
fn a_segmented_resistor_signs_off_lvs_clean() {
    let pdk = pdk();
    let sol = two_segment_resistor(&pdk);
    assert_eq!(sol.macros[0].drawn.len(), 2);
    let resistors = library::signoff_inputs(&sol, &pdk).2.devices.iter().filter(|d| d.kind == verify::RefKind::Resistor).count();
    assert_eq!(resistors, 2, "one card per segment");
    let rows = lvs_rows(&sol, &pdk);
    assert!(rows.is_empty(), "{rows:?}");
}

#[test]
fn a_missing_segment_is_an_lvs_error() {
    let pdk = pdk();
    let mut sol = two_segment_resistor(&pdk);
    sol.macros[0].drawn.pop();
    assert!(!lvs_rows(&sol, &pdk).is_empty(), "a reference missing a drawn segment must not match");
}

/// CELL-08: dac4 with sky130 MIM capacitors runs the flow and signs off LVS
/// clean with every one of its 16 units a capm card.
#[test]
fn a_mim_dac_signs_off_with_its_capacitors() {
    let pdk = pdk();
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let spice = std::fs::read_to_string(root.join("benchmarks/fixtures/dac4_mim.spice")).expect("dac4_mim fixture");
    let cfg = library::Config { feedback_iters: 1, ..Default::default() };
    let sol = library::run(&spice, &pdk, &library::Macros::default(), &cfg).expect("flow");
    let caps = library::signoff_inputs(&sol, &pdk).2.devices.iter().filter(|d| d.kind == verify::RefKind::Capacitor).count();
    assert_eq!(caps, 16, "one card per unit");
    let rows = lvs_rows(&sol, &pdk);
    assert!(rows.is_empty(), "{rows:?}");
}

/// CELL-18: a decoupling MIM written `VDD g` draws its bottom plate (`N`,
/// the high-parasitic one, the drawn card's second node) on the rail and its
/// top plate on the gate-only net, keeps every pin's name its schematic
/// terminal (so parasitics and currents land on the right one), and still
/// signs off LVS clean.
#[test]
fn a_decoupling_cap_puts_its_bottom_plate_on_the_rail() {
    let pdk = pdk();
    let spice = ".subckt t VDD VSS g\nXM1 VSS g VSS VSS sky130_fd_pr__nfet_01v8 w=1u l=0.15u\nXC1 VDD g sky130_fd_pr__cap_mim_m3_1 W=5u L=5u m=1\n.ends t\n";
    let cfg = library::Config { feedback_iters: 1, ..Default::default() };
    let sol = library::run(spice, &pdk, &library::Macros::default(), &cfg).expect("flow");
    let cap = sol.macros.iter().find(|m| m.drawn.iter().any(|d| d.kind == DrawnKind::Capacitor)).expect("the capacitor's cell");
    let card = cap.drawn.iter().find(|d| d.kind == DrawnKind::Capacitor).expect("its card");
    let dev = &sol.netlist.devices[card.device.expect("bound").0 as usize];
    let pin_net = |t: &str| {
        let p = cap.pins.iter().find(|p| p.name == format!("d{}:{t}", card.owner)).expect("plate pin");
        p.net
    };
    for t in ["P", "N"] {
        let schem = dev.terminals.iter().find(|(n, _)| n == t).expect("terminal").1;
        assert_eq!(pin_net(t), schem, "pin {t} sits on terminal {t}'s net");
    }
    let name = |n: &Node| match *n {
        Node::Pin(t) => sol.netlist.nets[pin_net(t).0 as usize].name.clone(),
        _ => panic!("plate node is a pin"),
    };
    assert_eq!(name(&card.nodes[1]), "VDD", "bottom plate on the rail");
    assert_eq!(name(&card.nodes[0]), "g", "top plate on the gate net");
    let rows = lvs_rows(&sol, &pdk);
    assert!(rows.is_empty(), "{rows:?}");
}
