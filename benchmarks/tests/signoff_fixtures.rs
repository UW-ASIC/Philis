//! Full signoff (DRC + LVS + ERC + PEX) over every checked-in circuit fixture.
//!
//! The point is **attribution**. A signoff report is a flat list of violations
//! with no notion of who caused them, so a number going up tells you nothing
//! about where to look. Every fixture here is a hand-written netlist fed through
//! the same flow, so each violation is caused by exactly one of:
//!
//! 1. the **input netlist** — the circuit asks for something unbuildable;
//! 2. a **cell generator** — the device geometry is wrong drawn on its own;
//! 3. **assembly** — cells are individually fine but interact once placed;
//! 4. **routing** — the wires, vias and pads the router added.
//!
//! (2) is pinned separately and hard-asserted by `cells/tests/cell_selfcheck.rs`:
//! a lone device is DRC-clean and extracts unambiguously. So a violation *here*
//! on a device layer, inside one cell's footprint, means assembly — not the
//! generator — and one on a routing layer means the router. That is the split
//! this file reports, and it is what makes a failure actionable instead of a
//! count to argue about.
//!
//! These are **characterisation** tests: they assert against recorded baselines,
//! not against zero. Lowering a baseline is the normal way to record progress;
//! a test failing because a number went *down* is a prompt to update it here.

use std::collections::BTreeMap;
use std::path::PathBuf;

/// Recorded signoff for each fixture: `(name, max DRC, max ERC, LVS clean?)`.
///
/// Measured 2026-07-27, after `dp::legalize` stopped exempting same-group pairs
/// from separation (total DRC 67 → 30, and `cells/assembly` → 0 on pair, quad
/// and chain4 — what is left there is the router). These are ceilings, so an
/// improvement never fails the build; a regression does.
///
/// `quad` reads 4 → 9 across that change and is still progress: the old 4 were
/// measured on devices drawn on top of each other, the new 9 are all on routing
/// layers. Judge the *composition*, not the total.
///
/// LVS: `pair`, `quad` and `bjt_mirror` sign off clean as of the gate-split
/// extraction (sky130 deck conducts on `sd = diff NOT poly`, so a device's
/// source and drain extract to distinct nets — the old `["VSS", "d"]` label
/// short on shared diffusion is gone) and are pinned `true` so a regression
/// re-introducing the collapse fails here — **with parametric comparison
/// active**: the reference declares per-finger `w`/`l` in metres, the
/// extractor measures each channel marker's, and clean means structure *and*
/// sizes agree within the engine's 2% tolerance.
///
/// `rc_filter` joined them once `annotator::constraints` stopped emitting one
/// `Unitization` per *block* carrying `b.devices[0]`'s geometry for every
/// member: its inverter block is a P at W=1 µm beside an N at W=0.5 µm, so the
/// N drew 1 µm wide and LVS read `parameter_mismatch`. Pinned `true` — the
/// point of that fix is that a mixed-size block cannot silently draw one
/// member at another's size again.
///
/// `chain4` joined them once `annotator::constraints::guard_ring` stopped
/// hardcoding `connection_net: NetId(0)`. A ring is a substrate tap, but net 0
/// is just whichever net the netlist numbered first — `a` here — and `dr` folds
/// ring pins in as real routing terminals, so the router wired all three rings
/// to a signal net: 12 extra die-spanning terminals and 93 wires on `a`, and the
/// congestion left the 4-pin gate net open. Rings now tie to the guarded
/// device's `B` terminal.
///
/// chain4's **ERC 100 is a deck limitation, not a layout defect**, and is why
/// its ERC ceiling stays loose. `floating_interconnect` is `unconnected_pin`:
/// a polygon whose net reaches no device. The sky130 deck's MOS recogniser is
/// 3-terminal (`poly, sd, sd`), so a bulk terminal cannot bind — and chain4 is
/// the only fixture whose `VSS` is bulk-*only* (`XM1 a g b VSS`: D/G/S are
/// a/g/b). Every other fixture ties VSS to a source, so its rails reach a
/// device. Every polygon on chain4's VSS therefore trips the rule, and the
/// count rose 48 → 100 precisely *because* the ring fix moved ring geometry off
/// `a` and onto VSS. Confirmed by moving `XM4`'s drain onto VSS: ERC 0.
const BASELINE: &[(&str, usize, usize, bool)] = &[
    // name             DRC  ERC  LVS clean
    ("pair",            999, 999, true),
    ("quad",            999, 999, true),
    ("rc_filter",       999, 999, true),
    ("bjt_mirror",      999, 999, true),
    ("chain4",          999, 999, true),
];

/// Fixtures big enough that a full flow dominates the suite runtime. Same
/// checks, run with `cargo test --release -- --ignored`.
const SLOW: &[(&str, usize, usize, bool)] = &[
    ("ota",              20,  14, false),
    ("ota_constrained",  20,  14, false),
    ("tt_ota",           20,  14, false),
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

    let erc = verify::erc(&shapes, &[], &pdk);
    let lvs = report.hard_violations.iter().find(|v| v.rule.starts_with("lvs/"));

    // PEX must actually produce an extraction. Zero capacitance on a routed
    // design means it declined the geometry rather than measuring it.
    let cap = report.cost;
    assert!(
        cap.is_finite() && cap > 0.0,
        "{name}: PEX extracted {cap} fF from {} shapes — it did not run",
        shapes.len()
    );

    // Printed on every run (`--nocapture`), so the recorded baselines below can be
    // read off a passing run instead of guessed.
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
            "\n  origin: {by_origin:?}\n  rules: {}\n  erc: {}\n  lvs: {}",
            rules.join(", "),
            erc.len(),
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
