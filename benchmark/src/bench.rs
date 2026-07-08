//! Benchmark harness: discover fixture circuits, run the FULL flow
//! (preprocess -> parse -> generate cells -> place -> route -> DRC/LVS/PEX ->
//! GDS) on each, print a summary table.
//!
//!   cargo run --release -p pnr-benchmark [local|align|magical|tinytapeout|all]
//!
//! Debug artifacts (traces, contracts, <top>.gds, signoff.txt) land in
//! target/bench_debug/<name>/.

mod fixtures;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use fixtures::{
    clone_repos_if_needed, cleanup_fixtures, discover_all, preprocess_spice, BenchmarkCircuit,
    Suite,
};
use pnr_core::backend::ConstraintRecord;
use pnr_core::orchestrator::{run_flow, FlowConfig};
use pnr_core::frontend::{parse_spice, Pdk};

/// Guard against pathological fixtures (SA cost scales with cell count even
/// with the bucket index; raise as the annealer earns it).
const MAX_CELLS: usize = 800;

fn pdk_path(suite: Suite) -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    match suite {
        Suite::Align | Suite::Magical => root.join("pdks/generic_finfet.json"),
        _ => root.join("pdks/sky130.json"),
    }
}

struct Row {
    name: String,
    suite: String,
    outcome: String,
    ms: u128,
}

fn run_circuit(c: &BenchmarkCircuit, pdk: &Pdk, deck_json: &str, pdk_json: &Path) -> String {
    let raw = match std::fs::read_to_string(&c.spice_path) {
        Ok(t) => t,
        Err(e) => return format!("read failed: {e}"),
    };
    // Generic-netlist preprocessing (backslash joins, bare R/C, nfin->W).
    let text = preprocess_spice(&raw, pdk_json).unwrap_or(raw);

    // pre-parse only to size-gate before committing to the full flow
    let g = match parse_spice(&text, pdk, &HashSet::new()) {
        Ok(g) => g,
        Err(e) => return format!("parse failed: {e}"),
    };
    if g.cells.is_empty() {
        return "no PDK devices resolved".into();
    }
    if g.cells.len() > MAX_CELLS {
        return format!("skipped ({} cells > {MAX_CELLS})", g.cells.len());
    }

    let cfg = FlowConfig {
        debug_dir: Some(Path::new("target/bench_debug").join(&c.name)),
        ..Default::default()
    };
    let r = match run_flow(&text, deck_json, &ConstraintRecord::default(), &cfg) {
        Ok(r) => r,
        Err(e) => return format!("flow failed: {e}"),
    };
    let s = &r.signoff;
    format!(
        "{} cells, {} nets | WL {} nm, unrouted {}, overuse {} | DRC {} (+{} density waived) | LVS {} | C {:.1} fF",
        r.graph.cells.len(),
        r.graph.nets.len(),
        r.routing.report.wirelength_nm,
        r.routing.report.unrouted.len(),
        r.routing.report.overuse,
        s.drc_blocking.len(),
        s.drc_waived_density,
        if s.lvs.matched { "MATCH" } else { "MISMATCH" },
        s.pex.total_cap() / 1000.0,
    )
}

fn main() {
    let suite = std::env::args()
        .nth(1)
        .map(|s| Suite::from_str(&s))
        .unwrap_or(Suite::Local);

    if let Err(e) = clone_repos_if_needed(suite) {
        eprintln!("Clone failed: {e}");
        std::process::exit(1);
    }

    let circuits = discover_all(suite);
    println!("Discovered {} circuits", circuits.len());

    // Cache loaded PDKs by path (sky130 vs generic_finfet).
    let mut pdk_cache: std::collections::HashMap<PathBuf, (String, Pdk)> = std::collections::HashMap::new();

    let mut rows = Vec::new();
    for c in &circuits {
        let pdk_json = pdk_path(c.suite);
        let (deck_json, pdk) = match pdk_cache.get(&pdk_json) {
            Some(pair) => (&pair.0, &pair.1),
            None => {
                let deck = match std::fs::read_to_string(&pdk_json) {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!("PDK load failed ({}): {e}", pdk_json.display());
                        std::process::exit(1);
                    }
                };
                let p = match Pdk::from_json(&deck) {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("PDK parse failed: {e}");
                        std::process::exit(1);
                    }
                };
                pdk_cache.insert(pdk_json.clone(), (deck, p));
                let pair = pdk_cache.get(&pdk_json).unwrap();
                (&pair.0, &pair.1)
            }
        };
        let t = Instant::now();
        let outcome = run_circuit(c, pdk, deck_json, &pdk_json);
        rows.push(Row {
            name: c.name.clone(),
            suite: format!("{:?}", c.suite),
            outcome,
            ms: t.elapsed().as_millis(),
        });
        let r = rows.last().unwrap();
        println!("  [{:>11}] {:32} {:6} ms  {}", r.suite, r.name, r.ms, r.outcome);
    }

    let ok = rows.iter().filter(|r| r.outcome.contains("cells,")).count();
    println!("{ok}/{} circuits placed+routed; debug in target/bench_debug/", rows.len());

    // Export SVGs for successful circuits
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("assets");
    let _ = std::fs::create_dir_all(&assets);
    let mut exported = 0;
    for r in &rows {
        if !r.outcome.contains("cells,") { continue; }
        let gds = Path::new("target/bench_debug").join(&r.name).join(format!("{}.gds", r.name));
        if let Ok(data) = std::fs::read(&gds) {
            let pdk_json = std::fs::read_to_string(
                Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("pdks/sky130.json")
            ).unwrap_or_default();
            let layer_names = pnr_visualizer::parse_layer_names(&pdk_json);
            let svg = pnr_visualizer::export_svg(&data, &layer_names);
            let out = assets.join(format!("{}.svg", r.name));
            let _ = std::fs::write(&out, &svg);
            exported += 1;
        }
    }
    if exported > 0 { println!("{exported} SVGs exported to assets/"); }

    if suite != Suite::Local {
        cleanup_fixtures();
    }
}
