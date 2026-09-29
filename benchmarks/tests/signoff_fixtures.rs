//! Full signoff (DRC + LVS + ERC + PEX) over every checked-in fixture, with
//! DRC findings attributed by layer: a routing layer carries only what `dr`
//! drew, a device layer only what a cell generator drew (lone cells are pinned
//! clean by `cells/tests/cell_selfcheck.rs`, so a device-layer finding here is
//! assembly). Every fixture must sign off clean: DRC 0 is a hard ceiling,
//! never raised to admit a regression.

use std::collections::BTreeMap;
use std::path::PathBuf;

#[path = "../src/fixtures.rs"]
#[allow(dead_code)]
mod fixtures;

/// `(name, max DRC, max ERC, LVS clean?)`, counted from `library::signoff`
/// (labelled, so a bulk-only port rail like chain4's VSS is not floating).
const BASELINE: &[(&str, usize, usize, bool)] = &[
    // name             DRC  ERC  LVS clean
    ("pair",              0,   0, true),
    ("quad",              0,   0, true),
    ("rc_filter",         0,   0, true),
    ("bjt_mirror",        0,   0, true),
    ("bgr_core",          0,   0, true),
    ("chain4",            0,   0, true),
    ("dac4",              0,   0, true),
];

/// Fixtures big enough that a full flow dominates the suite runtime. Same
/// checks, run with `cargo test --release -- --ignored`.
const SLOW: &[(&str, usize, usize, bool)] = &[
    ("ota",               0,   0, true),
    ("ota_constrained",   0,   0, true),
    ("tt_ota",            0,   0, true),
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn pdk() -> verify::Pdk {
    let text = std::fs::read_to_string(root().join("pdks/sky130.json")).expect("read sky130 deck");
    // `from_json` validates: a deck that cannot describe a legal layout is
    // rejected here rather than producing quietly wrong geometry downstream.
    verify::Pdk::from_json(&text).expect("sky130 deck is complete")
}

/// Where a violation came from, decided by the layer it is on.
///
/// A routing layer carries only what `dr` drew (wires, cuts, landing pads); a
/// device layer carries only what a cell generator drew. Nothing draws on both.
fn origin(pdk: &verify::Pdk, layer: &str) -> &'static str {
    let routing: Vec<String> = pdk
        .routing_metals
        .iter()
        .chain(&pdk.routing_cuts)
        .filter_map(|id| pdk.layers.iter().find(|(_, l)| l == id).map(|(n, _)| n.clone()))
        .collect();
    if routing.iter().any(|n| n == layer) {
        "routing"
    } else {
        "cells/assembly"
    }
}

fn check(name: &str, max_drc: usize, max_erc: usize, lvs_must_match: bool) {
    let pdk = pdk();
    let spice = std::fs::read_to_string(root().join(format!("benchmarks/fixtures/{name}.spice")))
        .unwrap_or_else(|e| panic!("{name}: read fixture: {e}"));

    // One feedback iteration: this is a correctness check, not a quality one, and
    // convergence quality is what `bench` measures.
    let cfg = library::Config { feedback_iters: 1, ..Default::default() };
    let sol = library::run(&spice, &pdk, &library::Macros::default(), &cfg)
        .unwrap_or_else(|e| panic!("{name}: flow failed: {e:?}"));

    let report = library::signoff(&sol, &pdk);
    let shapes = sol.geometry();
    let raw = verify::drc(&shapes, &[], &pdk);

    let mut by_origin: BTreeMap<&str, usize> = BTreeMap::new();
    let mut by_rule: BTreeMap<String, usize> = BTreeMap::new();
    for f in &raw {
        *by_origin.entry(origin(&pdk, &f.layer)).or_default() += 1;
        *by_rule.entry(format!("{}:{}", f.rule, f.layer)).or_default() += 1;
    }

    let erc: Vec<&str> = report
        .hard_violations
        .iter()
        .filter(|v| v.rule.starts_with("erc/"))
        .map(|v| v.rule.as_str())
        .collect();
    let lvs = report.hard_violations.iter().find(|v| v.rule.starts_with("lvs/"));

    // Zero capacitance on a routed design means PEX declined the geometry.
    let cap = report.cost;
    assert!(
        cap.is_finite() && cap > 0.0,
        "{name}: PEX extracted {cap} fF from {} shapes — it did not run",
        shapes.len()
    );

    // `--nocapture` prints the numbers the ceilings are read from.
    println!(
        "{name:16} DRC {:3} (routing {:2}, cells/assembly {:2})  ERC {:3}  PEX {cap:.1} fF  {}\n\
         {:16}   rules: {}",
        raw.len(),
        by_origin.get("routing").copied().unwrap_or(0),
        by_origin.get("cells/assembly").copied().unwrap_or(0),
        erc.len(),
        lvs.map_or("LVS clean".into(), |v| v.rule.clone()),
        "",
        by_rule.iter().map(|(k, n)| format!("{k}={n}")).collect::<Vec<_>>().join(", "),
    );

    let detail = || {
        let rules: Vec<String> = by_rule.iter().map(|(k, n)| format!("{k}={n}")).collect();
        format!(
            "\n  origin: {by_origin:?}\n  rules: {}\n  at: {}\n  erc: {}\n  lvs: {}",
            rules.join(", "),
            raw.iter().map(|f| format!("{}@({}, {})", f.rule, f.x, f.y)).collect::<Vec<_>>().join(", "),
            erc.join(", "),
            lvs.map_or("clean".into(), |v| v.rule.clone())
        )
    };

    assert!(
        raw.len() <= max_drc,
        "{name}: DRC {} exceeds baseline {max_drc}{}",
        raw.len(),
        detail()
    );
    assert!(
        erc.len() <= max_erc,
        "{name}: ERC {} exceeds baseline {max_erc}{}",
        erc.len(),
        detail()
    );
    if lvs_must_match {
        assert!(lvs.is_none(), "{name}: LVS must match{}", detail());
    }
}

#[test]
fn fixtures_sign_off_within_baseline() {
    for &(name, drc, erc, lvs) in BASELINE {
        check(name, drc, erc, lvs);
    }
}

#[test]
#[ignore = "full flow on the OTA-class fixtures; run with --ignored"]
fn large_fixtures_sign_off_within_baseline() {
    for &(name, drc, erc, lvs) in SLOW {
        check(name, drc, erc, lvs);
    }
}

/// The netlist the flow parsed must be the netlist the file describes. If a
/// fixture silently loses devices at parse time, every downstream count is
/// measured against the wrong circuit — which looks like a tool bug and is not.
#[test]
fn fixtures_parse_to_a_non_empty_circuit() {
    for &(name, ..) in BASELINE.iter().chain(SLOW) {
        let path = root().join(format!("benchmarks/fixtures/{name}.spice"));
        let spice = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
        let netlist = library::parse(&spice)
            .unwrap_or_else(|e| panic!("{name}: fixture does not parse: {e}"));
        assert!(!netlist.devices.is_empty(), "{name}: parsed to zero devices");
        assert!(!netlist.nets.is_empty(), "{name}: parsed to zero nets");
    }
}

/// A device the deck has no construction for (ASAP7 has no bipolar and no
/// resistor) is one `cell/undrawable` finding, not a silent gap.
#[test]
fn an_undrawable_device_is_one_finding() {
    let deck = root().join("pdks/generic_finfet.json");
    let pdk = verify::Pdk::from_json(&std::fs::read_to_string(&deck).expect("read deck")).expect("deck loads");
    for name in ["bjt_mirror", "rc_filter"] {
        let raw = std::fs::read_to_string(root().join(format!("benchmarks/fixtures/{name}.spice"))).expect("read fixture");
        let spice = fixtures::preprocess_spice(&raw, &deck).expect("retarget");
        let cfg = library::Config { feedback_iters: 1, ..Default::default() };
        let sol = library::run(&spice, &pdk, &library::Macros::default(), &cfg).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let report = library::signoff(&sol, &pdk);
        let undrawable: Vec<&str> = report.hard_violations.iter().map(|v| v.rule.as_str()).filter(|r| r.starts_with("cell/undrawable")).collect();
        assert_eq!(undrawable.len(), 1, "{name}: {undrawable:?}");
    }
}
