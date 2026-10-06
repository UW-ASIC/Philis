//! MOS flavours (CELL-16): an lvt and an hvt device draw their recogniser's
//! Vt marker, so they extract as their own models and sign off.

fn pdk() -> verify::Pdk {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let json = std::fs::read_to_string(root.join("pdks/sky130.json")).expect("pdks/sky130.json is in the repo");
    verify::Pdk::from_json(&json).expect("pdks/sky130.json loads")
}

#[test]
fn an_lvt_and_an_hvt_device_sign_off_with_their_models() {
    let pdk = pdk();
    let spice = ".subckt f VDD VSS a b\nXM1 a b VSS VSS sky130_fd_pr__nfet_01v8_lvt w=1u l=0.15u\nXM2 b a VDD VDD sky130_fd_pr__pfet_01v8_hvt w=1u l=0.15u\n.ends f\n";
    let cfg = library::Config { feedback_iters: 1, ..Default::default() };
    let sol = library::run(spice, &pdk, &library::Macros::default(), &cfg).expect("flow");
    let (shapes, pins, reference) = library::signoff_inputs(&sol, &pdk);
    let rows: Vec<String> = verify::signoff_with_intent(&shapes, &pins, &reference, &sol.intent, &pdk).0.hard_violations.into_iter().map(|v| v.rule).collect();
    assert!(!rows.iter().any(|r| r.starts_with("engine/")), "{rows:?}");
    let bad: Vec<&String> = rows.iter().filter(|r| r.starts_with("lvs/") || r.starts_with("drc/")).collect();
    assert!(bad.is_empty(), "{bad:?}");
    // LVS may ignore model names: the extract must name the flavours.
    let spice = verify::extract_spice(&shapes, &[], &pdk, verify::Detail::Schematic).expect("extracts");
    for m in ["nfet_01v8_lvt", "pfet_01v8_hvt"] {
        assert!(spice.contains(m), "{m} missing:\n{spice}");
    }
}

/// ASAP7 (field report): every flavour, n and p × rvt/lvt/slvt, is a device
/// the deck's LVS recognises, drawn with its Vt marker and extracted as its
/// own model; none is left LVS-unverified.
#[test]
fn every_asap7_flavour_is_recognised_by_lvs() {
    let pdk = verify::Pdk::builtin("asap7").expect("asap7 loads");
    let models = ["nmos_rvt", "nmos_lvt", "nmos_slvt", "pmos_rvt", "pmos_lvt", "pmos_slvt"];
    let cards: String = models.iter().enumerate().map(|(i, m)| format!("M{i} a b {s} {s} {m} nfin=2 l=20n\n", s = if m.starts_with('p') { "VDD" } else { "VSS" })).collect();
    let spice = format!(".subckt six a b VDD VSS\n{cards}.ends six\n");
    let cfg = library::Config { feedback_iters: 1, ..Default::default() };
    let sol = library::run(&spice, &pdk, &library::Macros::default(), &cfg).expect("flow");
    let signoff = library::signoff(&sol, &pdk);
    assert!(signoff.coverage.unverified.is_empty(), "{:?}", signoff.coverage.unverified);
    let rows: Vec<&str> = signoff.report.hard_violations.iter().map(|v| v.rule.as_str()).filter(|r| r.starts_with("lvs") || r.starts_with("engine/")).collect();
    assert!(rows.is_empty(), "{rows:?}");
    let (shapes, _, _) = library::signoff_inputs(&sol, &pdk);
    let extracted = verify::extract_spice(&shapes, &[], &pdk, verify::Detail::Schematic).expect("extracts");
    for m in models {
        assert_eq!(extracted.matches(&format!(" {m} ")).count(), 1, "{m}:\n{extracted}");
    }
}
