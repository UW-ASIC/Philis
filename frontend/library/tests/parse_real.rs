//! FLOW-07 against real inputs: the deck model tables of gf180mcu and
//! ihp_sg13g2, and every local benchmark fixture.

use std::path::PathBuf;

use library::{model_table, spice_with, ParseOptions};
use pnr_core::DeviceKind;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn pdk(name: &str) -> verify::Pdk {
    let json = std::fs::read_to_string(root().join(format!("pdks/{name}.json"))).expect("sidecar");
    verify::Pdk::from_json(&json).unwrap_or_else(|e| panic!("{name} loads: {e}"))
}

fn kind_with(pdk: &verify::Pdk, card: &str) -> DeviceKind {
    let opts = ParseOptions { models: model_table(pdk), ..Default::default() };
    spice_with(card, &opts).unwrap_or_else(|e| panic!("{card}: {e}")).devices[0].kind
}

#[test]
fn recipe_aliases_classify_gf180_and_ihp_resistors() {
    let gf = pdk("gf180mcu");
    assert_eq!(kind_with(&gf, "XR1 a b ppolyf_u W=1u L=10u"), DeviceKind::Resistor);
    // No token names these: without the table they are errors, not NMOS.
    assert!(library::parse("XR1 a b ppolyf_u W=1u L=10u").is_err());
    let ihp = pdk("ihp_sg13g2");
    assert_eq!(kind_with(&ihp, "XR1 a b rsil W=1u L=10u"), DeviceKind::Resistor);
    assert_eq!(kind_with(&ihp, "XR2 a b rppd W=1u L=10u"), DeviceKind::Resistor);
    // MOS polarity from the deck: IHP by its S/D layer, gf180 (one shared
    // `sd` layer) by its marker, sky130 behind the vendor prefix.
    assert_eq!(kind_with(&ihp, "XM1 d g s b sg13_lv_pmos W=1u L=1u"), DeviceKind::Pmos);
    assert_eq!(kind_with(&ihp, "XM1 d g s b sg13_lv_nmos W=1u L=1u"), DeviceKind::Nmos);
    assert_eq!(kind_with(&gf, "XM1 d g s b pfet_03v3 W=1u L=1u"), DeviceKind::Pmos);
    assert_eq!(kind_with(&gf, "XM1 d g s b nfet_03v3 W=1u L=1u"), DeviceKind::Nmos);
    let sky = pdk("sky130");
    assert_eq!(kind_with(&sky, "XM1 d g s b pfet_01v8_hvt W=1u L=1u"), DeviceKind::Pmos);
    assert_eq!(kind_with(&sky, "XM1 d g s b nfet_01v8_lvt W=1u L=1u"), DeviceKind::Nmos);
    assert_eq!(kind_with(&sky, "XR1 a b res_high_po W=1u L=1u"), DeviceKind::Resistor);
}

#[test]
fn local_fixtures_parse_as_before() {
    // (devices, nets) as the pre-FLOW-07 parser read them (audit-06 G.3).
    let expect = [
        ("bgr_core", 2, 3),
        ("bjt_mirror", 2, 5),
        ("chain4", 4, 7),
        ("dac4", 14, 12),
        ("ota", 5, 9),
        ("ota_constrained", 5, 9),
        ("pair", 2, 3),
        ("quad", 4, 3),
        ("rc_filter", 3, 5),
        ("tt_ota", 5, 9),
    ];
    let sky = pdk("sky130");
    let opts = ParseOptions { models: model_table(&sky), ..Default::default() };
    for (name, devices, nets) in expect {
        let text = std::fs::read_to_string(root().join(format!("benchmarks/fixtures/{name}.spice"))).expect("fixture");
        let nl = spice_with(&text, &opts).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!((nl.devices.len(), nl.nets.len()), (devices, nets), "{name}");
        // Same without the deck table: the model tokens suffice here.
        let bare = library::parse(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!((bare.devices.len(), bare.nets.len()), (devices, nets), "{name}");
        if name == "ota" {
            let ports: Vec<&str> = nl.ports.iter().map(|p| nl.nets[p.0 as usize].name.as_str()).collect();
            assert_eq!(ports, ["vinp", "vinm", "vout1", "vout2", "VDD", "VSS"]);
        }
    }
}
