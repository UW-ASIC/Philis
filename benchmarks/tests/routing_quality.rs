//! Routing-quality acceptance on the checked-in fixtures (RTE-15…RTE-20):
//! what `dr` reports and what the routed geometry measures, over the same
//! one-iteration flow as `signoff_fixtures::check`. Full flows: run with
//! `cargo test --release -p benchmark --test routing_quality -- --include-ignored`.

use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn pdk() -> verify::Pdk {
    let text = std::fs::read_to_string(root().join("pdks/sky130.json")).expect("read sky130 deck");
    verify::Pdk::from_json(&text).expect("sky130 deck is complete")
}

/// The fixture `name` through the flow at one feedback iteration.
fn run(name: &str, pdk: &verify::Pdk) -> library::Solution {
    let spice = std::fs::read_to_string(root().join(format!("benchmarks/fixtures/{name}.spice"))).unwrap_or_else(|e| panic!("{name}: read fixture: {e}"));
    let cfg = library::Config { feedback_iters: 1, ..Default::default() };
    library::run(&spice, pdk, &library::Macros::default(), &cfg).unwrap_or_else(|e| panic!("{name}: flow failed: {e:?}"))
}

/// dr's hard rows starting with `prefix`.
fn rows<'a>(sol: &'a library::Solution, prefix: &str) -> Vec<&'a str> {
    sol.route.hard_violations.iter().filter(|v| v.rule.starts_with(prefix)).map(|v| v.rule.as_str()).collect()
}

/// RTE-16: no routed metal over a matched gate, and no open net.
#[test]
#[ignore = "full flow on the OTA-class fixtures; run with --include-ignored"]
fn no_metal_over_gates() {
    let pdk = pdk();
    for name in ["ota", "tt_ota"] {
        let sol = run(name, &pdk);
        println!("{name}: metal over gate {:?}, open {:?}", rows(&sol, "metal over gate"), rows(&sol, "open net"));
        assert!(rows(&sol, "metal over gate").is_empty(), "{name}: {:?}", rows(&sol, "metal over gate"));
        assert!(rows(&sol, "open net").is_empty(), "{name}: {:?}", rows(&sol, "open net"));
    }
}

/// Every fixture the signoff suite runs.
const FIXTURES: [&str; 12] = ["pair", "quad", "rc_filter", "res_m2", "bjt_mirror", "bgr_core", "chain4", "dac4", "mirror_ratio", "ota", "ota_constrained", "tt_ota"];

/// The batches of `reqs` (hard then budget) whose kind ends with `kind`.
fn batches<'a>(reqs: &'a analog::Requirements<pnr_core::Routes>, kind: &str) -> Vec<&'a dyn analog::RuleBatch<pnr_core::Routes>> {
    reqs.hard.iter().chain(&reqs.budget).filter(|b| b.kind().ends_with(kind)).map(|b| b.as_ref()).collect()
}

/// RTE-18: every `CrosstalkExclusion` holds on every fixture (kept in the search).
#[test]
#[ignore = "full flow on every fixture; run with --include-ignored"]
fn crosstalk_exclusions_hold() {
    let pdk = pdk();
    let mut bad = Vec::new();
    for name in FIXTURES {
        let sol = run(name, &pdk);
        let n: u32 = batches(&sol.routing, "CrosstalkExclusion").iter().map(|b| b.count() as u32).sum();
        let v: u32 = batches(&sol.routing, "CrosstalkExclusion").iter().map(|b| b.violations(&sol.routes)).sum();
        println!("{name:16} CrosstalkExclusion {v}/{n} violated, open {:?}", rows(&sol, "open net"));
        if v > 0 {
            bad.push(name);
        }
    }
    assert!(bad.is_empty(), "violated on {bad:?}");
}

/// RTE-18 calibration: per `CouplingBudget` victim on `ota`, the unweighted
/// screened coupling over routed and cell metal against signoff PEX's
/// victim–net rows (PERF-16 owns PEX fidelity; a miss is printed).
#[test]
#[ignore = "full flow on ota; run with --include-ignored"]
fn coupling_tracks_pex() {
    let pdk = pdk();
    let sol = run("ota", &pdk);
    let stack = library::parasitic_stack(&pdk);
    let caps = library::signoff(&sol, &pdk).caps;
    let shapes = |n: usize| -> Vec<pnr_core::Shape> { sol.routes.wires.get(n).into_iter().flatten().chain(sol.routes.cell.get(n).into_iter().flatten()).copied().collect() };
    let n_nets = sol.routes.wires.len();
    let mut victims = Vec::new();
    for b in batches(&sol.routing, "CouplingBudget") {
        b.touched(&mut victims);
    }
    let mut worst = 0.0f64;
    for v in victims.into_iter().map(|v| v as usize) {
        let name = &sol.netlist.nets[v].name;
        let mine = shapes(v);
        let model: f64 = (0..n_nets)
            .filter(|&o| o != v)
            .map(|o| {
                let screens: Vec<pnr_core::Shape> = (0..n_nets).filter(|&x| x != v && x != o).flat_map(shapes).collect();
                f64::from(analog::routing::coupling::net_pair_af(Some(&stack), &mine, &shapes(o), &screens))
            })
            .sum();
        let pex: f64 = caps.iter().filter(|(a, b, _)| b.is_some() && (a == name || b.as_deref() == Some(name.as_str()))).map(|r| r.2 * 1_000.0).sum();
        let err = if pex > 0.0 { (model - pex).abs() / pex } else { f64::INFINITY };
        println!("calibration {name:12} model {model:8.1} aF  pex {pex:8.1} aF  error {:5.1} %", err * 100.0);
        worst = worst.max(err);
    }
    assert!(worst <= 0.20, "coupling model off PEX by {:.1} %", worst * 100.0);
}

/// `v` with every net but `a` and `b` emptied.
fn pair_only<T: Clone>(v: &[Vec<T>], a: u32, b: u32) -> Vec<Vec<T>> {
    v.iter().enumerate().map(|(n, x)| if n == a as usize || n == b as usize { x.clone() } else { Vec::new() }).collect()
}

/// RTE-15: per fixture, the matched pairs dr routed as exact images and why
/// the rest fell back; every exact pair's Differential mismatch is 0.0 on its
/// own two nets (the rest of the layout taken out: foreign coupling is not
/// the image's to match).
#[test]
#[ignore = "full flow on the OTA-class fixtures; run with --include-ignored"]
fn pairs_report() {
    let pdk = pdk();
    for name in ["ota", "ota_constrained", "tt_ota"] {
        let sol = run(name, &pdk);
        let diff = batches(&sol.routing, "Differential");
        let mut ids = Vec::new();
        for b in &diff {
            b.touched(&mut ids);
        }
        let st = &sol.route_stats;
        println!("{name:16} pairs exact {:?} of {}  fallback {:?}", st.pairs_exact, ids.len() / 2, st.pairs_fallback);
        for &(a, b) in &st.pairs_exact {
            let r = &sol.routes;
            let only = pnr_core::Routes { wires: pair_only(&r.wires, a, b), cell: pair_only(&r.cell, a, b), gates: pair_only(&r.gates, a, b), terms: pair_only(&r.terms, a, b) };
            let worst = diff.iter().filter_map(|d| d.worst_usage(&only)).reduce(f32::max);
            assert_eq!(worst, Some(0.0), "{name}: exact pair {a}/{b} Differential usage");
        }
    }
}

/// RTE-17: on `ota` every common node with a ΔR allowance meets it (the
/// winner's metadata row; unknown = no allowance). Star/Kelvin nodes wait on
/// EXT-24's producers.
#[test]
#[ignore = "full flow on ota; run with --include-ignored"]
fn common_nodes_meet_allowance() {
    let pdk = pdk();
    let sol = run("ota", &pdk);
    let row = sol.metadata.routing.iter().find(|r| r.kind == "CommonNode").expect("a CommonNode row");
    println!("ota CommonNode total {} violated {} unknown {}", row.total, row.violations, row.unknown);
    assert_eq!(row.violations, 0);
}

/// RTE-20 on `dac4`: the capacitor set's bit leads never cross its top plate
/// outside the array, and their lead C per unit is within tolerance.
#[test]
#[ignore = "acceptance row; run with --include-ignored"]
fn dac4_plates() {
    let pdk = pdk();
    let sol = run("dac4", &pdk);
    println!("dac4 plate sets {:?}  crossing {:?}  ratio {:?}", sol.route_stats.plate_spread_pct, rows(&sol, "plate crossing"), sol.route.budget_violations.iter().filter(|v| v.rule.starts_with("plate ratio")).map(|v| &v.rule).collect::<Vec<_>>());
    assert!(!sol.route_stats.plate_spread_pct.is_empty(), "no capacitor set reached dr");
    assert!(rows(&sol, "plate crossing").is_empty());
    assert!(!sol.route.budget_violations.iter().any(|v| v.rule.starts_with("plate ratio")));
}
