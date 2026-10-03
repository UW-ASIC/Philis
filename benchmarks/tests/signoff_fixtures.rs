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

/// `(name, max DRC, expected ERC, LVS clean?)`, counted from `library::signoff`
/// (labelled, so a bulk-only port rail like chain4's VSS is not floating).
/// Expected ERC is the exact rule strings (errors and deck warnings alike) the
/// fixture may produce: a row not listed, or a listed row gone, fails (AV-28),
/// so a known-false finding is named, never admitted by count. None today:
/// the sky130 `supply_short` bjt_mirror and bgr_core tripped is scoped to
/// n-taps in a PMOS well since GPurify 4ef439d. LVS clean = no `lvs/` row; the
/// `lvs-coverage/` rows must sum to [`unverified`] of the netlist.
const BASELINE: &[(&str, usize, &[&str], bool)] = &[
    // name             DRC  ERC  LVS clean
    ("pair",              0,  &[], true),
    ("quad",              0,  &[], true),
    ("rc_filter",         0,  &[], true),
    ("res_m2",            0,  &[], true),
    ("bjt_mirror",        0,  &[], true),
    ("bgr_core",          0,  &[], true),
    ("chain4",            0,  &[], true),
    ("dac4",              0,  &[], true),
];

/// Fixtures big enough that a full flow dominates the suite runtime. Same
/// checks, run with `cargo test --release -- --ignored`.
const SLOW: &[(&str, usize, &[&str], bool)] = &[
    ("ota",               0,  &[], true),
    ("ota_constrained",   0,  &[], true),
    ("tt_ota",            0,  &[], true),
];

/// Devices LVS cannot compare on sky130, counted from the netlist alone: the
/// deck recognises no BJT and no MOM capacitor (every capacitor generator
/// draws one), so each drawn unit — `m` per BJT, `max(nf, m)` per capacitor —
/// is one `lvs-coverage/` unit.
fn unverified(netlist: &pnr_core::Netlist) -> i64 {
    use pnr_core::DeviceKind::{Capacitor, Npn, Pnp};
    let p = |d: &pnr_core::Device, k: &str| d.params.iter().find(|(n, _)| n == k).map_or(1, |&(_, v)| v);
    netlist
        .devices
        .iter()
        .map(|d| match d.kind {
            Npn | Pnp => p(d, "m"),
            Capacitor => p(d, "nf").max(p(d, "m")),
            _ => 0,
        })
        .sum()
}

/// Every ERC row of a signoff, errors and deck warnings alike (a deck
/// warning is out of the epoch key's |V| but not out of this baseline).
fn erc_rows<'a>(hard: &'a [pnr_core::Violation], warnings: &'a [pnr_core::Violation]) -> Vec<&'a str> {
    hard.iter().chain(warnings).filter(|v| v.rule.starts_with("erc/")).map(|v| v.rule.as_str()).collect()
}

/// `Err` naming the difference unless `rows` is exactly `expected` (as multisets).
fn erc_matches(rows: &[&str], expected: &[&str]) -> Result<(), String> {
    let (mut got, mut want) = (rows.to_vec(), expected.to_vec());
    got.sort_unstable();
    want.sort_unstable();
    if got == want { Ok(()) } else { Err(format!("ERC rows {got:?}, expected {want:?}")) }
}

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

fn check(name: &str, max_drc: usize, expected_erc: &[&str], lvs_must_match: bool) {
    let pdk = pdk();
    let spice = std::fs::read_to_string(root().join(format!("benchmarks/fixtures/{name}.spice")))
        .unwrap_or_else(|e| panic!("{name}: read fixture: {e}"));

    // One feedback iteration: this is a correctness check, not a quality one, and
    // convergence quality is what `bench` measures.
    let cfg = library::Config { feedback_iters: 1, ..Default::default() };
    let sol = library::run(&spice, &pdk, &library::Macros::default(), &cfg)
        .unwrap_or_else(|e| panic!("{name}: flow failed: {e:?}"));

    let signoff = library::signoff(&sol, &pdk);
    let report = &signoff.report;
    let shapes = sol.geometry();
    // Errors only: a deck warning is `signoff.warnings`, as in the bench's DRC
    // column and the epoch's |V| (PERF-01).
    let raw: Vec<verify::Finding> = verify::drc(&shapes, &[], &pdk).into_iter().filter(|f| !f.warning).collect();

    let mut by_origin: BTreeMap<&str, usize> = BTreeMap::new();
    let mut by_rule: BTreeMap<String, usize> = BTreeMap::new();
    for f in &raw {
        *by_origin.entry(origin(&pdk, &f.layer)).or_default() += 1;
        *by_rule.entry(format!("{}:{}", f.rule, f.layer)).or_default() += 1;
    }

    let erc = erc_rows(&report.hard_violations, &signoff.warnings);
    let lvs = report.hard_violations.iter().find(|v| v.rule.starts_with("lvs/"));
    let uncompared: i64 = report.hard_violations.iter().filter(|v| v.rule.starts_with("lvs-coverage/")).map(|v| v.margin).sum();

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
        lvs.map_or(format!("LVS clean, {uncompared} unverified"), |v| v.rule.clone()),
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

    // Every device is compared or declared: none dropped before LVS. First, so a
    // known DRC/ERC failure (dac4's antenna gate row, RTE-06) cannot mask it.
    let want = unverified(&sol.netlist);
    assert_eq!(uncompared, want, "{name}: lvs-coverage units {uncompared}, the netlist has {want} uncomparable{}", detail());
    assert!(
        raw.len() <= max_drc,
        "{name}: DRC {} exceeds baseline {max_drc}{}",
        raw.len(),
        detail()
    );
    if let Err(e) = erc_matches(&erc, expected_erc) {
        panic!("{name}: {e}{}", detail());
    }
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

/// The in-loop antenna model never passes what signoff fails (REL T2): a
/// fixture with an `erc/ar.*` signoff row has an `Antenna` family with a
/// violated rule. One-directional on purpose — the loop sums per-rect
/// sidewall perimeters and credits a diode only where one is touched, so it
/// may flag what signoff passes.
///
/// ponytail: per fixture, not per net as REL T2 reads: a signoff row carries
/// no net (`verify::Finding` is a point on a layer, here the gate), so a
/// violation on another net would pass this. Per net needs net-attributed ERC
/// findings (M1).
#[test]
fn antenna_in_loop_never_passes_what_signoff_fails() {
    let pdk = pdk();
    for &(name, ..) in BASELINE {
        let spice = std::fs::read_to_string(root().join(format!("benchmarks/fixtures/{name}.spice"))).expect("read fixture");
        let cfg = library::Config { feedback_iters: 1, ..Default::default() };
        let sol = library::run(&spice, &pdk, &library::Macros::default(), &cfg).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let report = library::signoff(&sol, &pdk).report;
        let ar: Vec<&str> = report.hard_violations.iter().map(|v| v.rule.as_str()).filter(|r| r.starts_with("erc/ar.")).collect();
        let fams: Vec<_> = sol.metadata.routing.iter().filter(|b| b.kind.ends_with("Antenna")).collect();
        let counts = || fams.iter().map(|b| (b.total, b.satisfied, b.violations, b.unknown)).collect::<Vec<_>>();
        println!("{name:16} signoff antenna rows {ar:?}  in-loop (total, sat, viol, unk) {:?}", counts());
        if !ar.is_empty() {
            assert!(
                fams.iter().any(|b| b.violations > 0),
                "{name}: signoff fails {ar:?}, the in-loop antenna model passes it: {:?}",
                counts()
            );
        }
    }
}

/// A device LVS cannot compare is a signoff row, not an epoch violation: it
/// is the same on every layout, so bjt_mirror (an NPN and a PNP, neither of
/// which sky130 extracts) still stops converged at bench's 5 epochs, and its
/// signoff still names both units.
#[test]
fn uncompared_devices_do_not_block_convergence() {
    let spice = std::fs::read_to_string(root().join("benchmarks/fixtures/bjt_mirror.spice")).expect("read fixture");
    let cfg = library::Config { feedback_iters: 5, ..Default::default() };
    let pdk = pdk();
    let sol = library::run(&spice, &pdk, &library::Macros::default(), &cfg).unwrap_or_else(|e| panic!("bjt_mirror: {e:?}"));
    let report = library::signoff(&sol, &pdk).report;
    let uncompared: i64 = report.hard_violations.iter().filter(|v| v.rule.starts_with("lvs-coverage/")).map(|v| v.margin).sum();
    assert_eq!(uncompared, unverified(&sol.netlist), "{:?}", report.hard_violations.iter().map(|v| &v.rule).collect::<Vec<_>>());
    assert!(uncompared > 0, "bjt_mirror has no uncompared device: the test checks nothing");
    assert!(sol.stats.converged, "bjt_mirror stopped on budget: {:?}", sol.stats);
}

/// The named-ERC comparison is itself live: one extra row on a real report
/// fails it, and so does a listed row that never appears.
#[test]
fn an_unlisted_erc_row_fails_the_baseline() {
    let (name, _, expected, _) = BASELINE[0];
    let spice = std::fs::read_to_string(root().join(format!("benchmarks/fixtures/{name}.spice"))).expect("read fixture");
    let cfg = library::Config { feedback_iters: 1, ..Default::default() };
    let pdk = pdk();
    let sol = library::run(&spice, &pdk, &library::Macros::default(), &cfg).unwrap_or_else(|e| panic!("{name}: {e:?}"));
    let signoff = library::signoff(&sol, &pdk);
    let (hard, warn) = (&signoff.report.hard_violations, &signoff.warnings);
    assert_eq!(erc_matches(&erc_rows(hard, warn), expected), Ok(()), "{name} must meet its own baseline");
    let mut extra: Vec<pnr_core::Violation> = hard.iter().map(|v| pnr_core::Violation { rule: v.rule.clone(), margin: v.margin }).collect();
    extra.push(pnr_core::Violation { rule: "erc/floating_gate:poly".into(), margin: 0 });
    assert!(erc_matches(&erc_rows(&extra, warn), expected).is_err(), "an unlisted ERC row passed");
    let mut listed = expected.to_vec();
    listed.push("erc/supply_short:ntap");
    assert!(erc_matches(&erc_rows(hard, warn), &listed).is_err(), "a listed ERC row that vanished passed");
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
        let report = library::signoff(&sol, &pdk).report;
        let undrawable: Vec<&str> = report.hard_violations.iter().map(|v| v.rule.as_str()).filter(|r| r.starts_with("cell/undrawable")).collect();
        assert_eq!(undrawable.len(), 1, "{name}: {undrawable:?}");
    }
}

/// An inductor has no recogniser on any deck, so it is drawn as nothing and
/// signoff names it: one `L` card is exactly one `cell/undrawable` row, never
/// a met1 drawing that only looks like a coil (CELL-04).
#[test]
fn an_inductor_is_one_undrawable_finding() {
    let pdk = pdk();
    let spice = "\
.subckt lc vin vout VDD VSS
XM1 vmid vin VDD VDD pfet_01v8 W=1u L=0.15u
XM2 vmid vin VSS VSS nfet_01v8 W=0.5u L=0.15u
L1 vmid vout 1n
.ends lc
";
    let cfg = library::Config { feedback_iters: 1, ..Default::default() };
    let sol = library::run(spice, &pdk, &library::Macros::default(), &cfg).unwrap_or_else(|e| panic!("{e:?}"));
    assert!(sol.netlist.devices.iter().any(|d| d.kind == pnr_core::DeviceKind::Inductor), "L1 parsed away");
    let report = library::signoff(&sol, &pdk).report;
    let undrawable: Vec<&str> = report.hard_violations.iter().map(|v| v.rule.as_str()).filter(|r| r.starts_with("cell/undrawable")).collect();
    assert_eq!(undrawable.len(), 1, "{undrawable:?}");
    assert!(undrawable[0].contains("L1"), "{undrawable:?}");
    // The epoch counts it too: an undrawn device is never a feasible stop.
    assert!(!sol.stats.converged, "converged with L1 undrawn: {:?}", sol.stats);
}

/// PERF-03: with a `.subckt` header, only its declared ports are exempt from
/// the floating-gate check; `ota`'s old (pre-M1) header left `vbias`/`vbn`
/// undeclared, so they were exempt too and their undriven gates went
/// unflagged. The M1 exit criterion is exactly 2 such rows (vbias, vbn).
#[test]
fn ota_old_header_flags_its_undriven_bias_gates() {
    let pdk = pdk();
    let text = std::fs::read_to_string(root().join("benchmarks/fixtures/ota.spice")).expect("read fixture");
    let old = text.replacen(" vbias vbn", "", 1);
    assert_ne!(old, text, "ota.spice lost its vbias/vbn ports");
    let cfg = library::Config { feedback_iters: 1, starts: 1, ..Default::default() };
    let sol = library::run(&old, &pdk, &library::Macros::default(), &cfg).unwrap_or_else(|e| panic!("ota: {e:?}"));
    let signoff = library::signoff(&sol, &pdk);
    let fg = erc_rows(&signoff.report.hard_violations, &signoff.warnings)
        .iter()
        .filter(|r| r.starts_with("erc/floating_gate"))
        .count();
    assert_eq!(fg, 2, "expected 2 floating_gate rows (vbias, vbn) with the old header");
}
