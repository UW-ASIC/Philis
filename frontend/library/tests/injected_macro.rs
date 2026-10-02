//! An injected FET macro must be a real device: one that does not extract to
//! exactly one transistor is refused before it can unpair LVS for the circuit.

use library::{run, Config, FlowError, Macros};
use pnr_core::{Macro, NetId, Pin, Process, Rect, Shape};

const PAIR: &str = r"
.subckt pair a b g vss
M1 a g vss vss nfet w=1u l=0.15u
M2 b g vss vss nfet w=1u l=0.15u
.ends
";

#[test]
fn a_bar_of_diff_and_poly_is_not_a_transistor() {
    let deck = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../pdks/sky130.json")).unwrap();
    let pdk = verify::Pdk::from_json(&deck).unwrap();
    let (diff, poly) = (pdk.layer("diff").unwrap(), pdk.layer("poly").unwrap());
    // No implant, no contacts: the extractor sees no device.
    let bar = Macro {
        shapes: vec![
            Shape { layer: diff, rect: Rect { x: 0, y: 0, w: 4000, h: 1000 } },
            Shape { layer: poly, rect: Rect { x: 1800, y: -200, w: 400, h: 1400 } },
        ],
        pins: vec![Pin { name: "G".into(), net: NetId(0), at: Rect { x: 1800, y: 0, w: 400, h: 400 }, layer: poly }],
        bbox: Rect { x: -200, y: -200, w: 4400, h: 1400 },
        units: Vec::new(),
        dummies: Vec::new(),
        ..Default::default()
    };
    let mut macros = Macros::default();
    macros.register("M1", bar);
    match run(PAIR, &pdk, &macros, &Config::default()) {
        Err(FlowError::InjectedNotADevice(name, n)) => assert_eq!((name.as_str(), n), ("M1", Some(0))),
        other => panic!("expected refusal, got {:?}", other.map(|_| "a solution")),
    }
}
