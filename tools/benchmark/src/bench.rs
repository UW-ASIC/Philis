//! Benchmark harness: discover fixture circuits, run the FULL flow
//! (preprocess -> parse -> generate cells -> place -> route -> DRC/LVS/PEX ->
//! GDS) on each, print a summary table.
//!
//!   cargo run --release -p pnr-benchmark [local|align|magical|tinytapeout|all]
//!
//! Debug artifacts (traces, contracts, <top>.gds, signoff.txt) land in
//! target/bench_debug/<name>/.

mod fixtures;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use fixtures::{
    clone_repos_if_needed, cleanup_fixtures, discover_all, preprocess_spice, BenchmarkCircuit,
    Suite,
};
use pnr_core::backend::{ConstraintContract, ConstraintRecord, ConstraintStatus, InterfaceSpec};
use pnr_core::orchestrator::{run_flow, FlowConfig};
use pnr_core::frontend::{parse_spice, Pdk};

/// Guard against pathological fixtures (SA cost scales with cell count even
/// with the bucket index; raise as the annealer earns it).
const MAX_CELLS: usize = 800;

/// Repo root: two levels up from tools/benchmark.
fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap()
}

fn pdk_path(suite: Suite) -> PathBuf {
    let root = repo_root();
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
    /// Placement contracts for per-type satisfaction reporting (empty on failure).
    contracts: Vec<ConstraintContract>,
}

#[derive(Debug)]
struct Compactness {
    die_area_um2: f64,
    drawn_util_pct: f64,
    planning_util_pct: f64,
    bbox_fill_pct: f64,
}

impl Compactness {
    fn measure(
        x: &[i32],
        y: &[i32],
        sizes: &[(i32, i32)],
        die: (i32, i32),
        cell_margin: i32,
    ) -> Self {
        let drawn_area: f64 = sizes
            .iter()
            .map(|&(w, h)| f64::from(w) * f64::from(h))
            .sum();
        let margin = f64::from(cell_margin.max(0));
        let planning_area: f64 = sizes
            .iter()
            .map(|&(w, h)| (f64::from(w) + margin) * (f64::from(h) + margin))
            .sum();
        let die_area = f64::from(die.0.max(0)) * f64::from(die.1.max(0));

        let bbox_area = x
            .iter()
            .zip(y)
            .zip(sizes)
            .fold(None::<(i64, i64, i64, i64)>, |bounds, ((&cx, &cy), &(w, h))| {
                let cell = (
                    i64::from(cx - w / 2),
                    i64::from(cy - h / 2),
                    i64::from(cx + w / 2),
                    i64::from(cy + h / 2),
                );
                Some(match bounds {
                    None => cell,
                    Some((xmin, ymin, xmax, ymax)) => (
                        xmin.min(cell.0),
                        ymin.min(cell.1),
                        xmax.max(cell.2),
                        ymax.max(cell.3),
                    ),
                })
            })
            .map_or(0.0, |(xmin, ymin, xmax, ymax)| {
                (xmax - xmin).max(0) as f64 * (ymax - ymin).max(0) as f64
            });
        let pct = |area: f64, envelope: f64| {
            if envelope > 0.0 {
                100.0 * area / envelope
            } else {
                0.0
            }
        };

        Self {
            die_area_um2: die_area / 1_000_000.0,
            drawn_util_pct: pct(drawn_area, die_area),
            planning_util_pct: pct(planning_area, die_area),
            bbox_fill_pct: pct(drawn_area, bbox_area),
        }
    }
}

fn run_circuit(
    c: &BenchmarkCircuit,
    pdk: &Pdk,
    deck_json: &str,
    pdk_json: &Path,
    seed: u64,
) -> (String, Vec<ConstraintContract>) {
    let raw = match std::fs::read_to_string(&c.spice_path) {
        Ok(t) => t,
        Err(e) => return (format!("read failed: {e}"), Vec::new()),
    };
    // Generic-netlist preprocessing (backslash joins, bare R/C, nfin->W).
    let text = preprocess_spice(&raw, pdk_json).unwrap_or(raw);

    // pre-parse only to size-gate before committing to the full flow
    let g = match parse_spice(&text, pdk, &HashSet::new()) {
        Ok(g) => g,
        Err(e) => return (format!("parse failed: {e}"), Vec::new()),
    };
    if g.cells.is_empty() {
        return ("no PDK devices resolved".into(), Vec::new());
    }
    if g.cells.len() > MAX_CELLS {
        return (format!("skipped ({} cells > {MAX_CELLS})", g.cells.len()), Vec::new());
    }

    let mut cfg = FlowConfig {
        debug_dir: Some(Path::new("target/bench_debug").join(&c.name)),
        // Benchmark cap: 5 feedback iterations keeps the full suite tractable
        // (the hardened per-iter pipeline is minutes on big analog blocks).
        max_feedback_iters: 5,
        ..Default::default()
    };
    cfg.placement.seed = seed;
    // Constrained-interface sidecar: <stem>.interface.json beside the fixture.
    let iface_path = c.spice_path.with_extension("interface.json");
    if iface_path.exists() {
        let parsed = std::fs::read_to_string(&iface_path)
            .map_err(|e| e.to_string())
            .and_then(|t| serde_json::from_str::<InterfaceSpec>(&t).map_err(|e| e.to_string()))
            .and_then(|s| s.validate().map(|()| s));
        match parsed {
            Ok(spec) => cfg.interface = Some(spec),
            Err(e) => return (format!("interface spec invalid: {e}"), Vec::new()),
        }
    }
    let r = match run_flow(&text, deck_json, &ConstraintRecord::default(), &cfg) {
        Ok(r) => r,
        Err(e) => return (format!("flow failed: {e}"), Vec::new()),
    };
    let s = &r.signoff;
    let p = &r.placement.placement;
    let compactness = Compactness::measure(&p.x, &p.y, &p.sizes, p.die, p.cell_margin);
    let outcome = format!(
        "{} cells, {} nets | WL {} nm, unrouted {}, overuse {} | DRC {} (+{} density waived) | LVS {} | C {:.1} fF | area {:.1} um2 | util drawn {:.1}%, plan {:.1}%, bbox {:.1}% | best {}/{}{} | seed {}",
        r.graph.cells.len(),
        r.graph.nets.len(),
        r.routing.report.wirelength_nm,
        r.routing.report.unrouted.len(),
        r.routing.report.overuse,
        s.drc_blocking.len(),
        s.drc_waived_density,
        if s.lvs.matched { "MATCH" } else { "MISMATCH" },
        s.pex.total_cap() / 1000.0,
        compactness.die_area_um2,
        compactness.drawn_util_pct,
        compactness.planning_util_pct,
        compactness.bbox_fill_pct,
        r.best_iteration + 1,
        r.iterations,
        if r.converged { " converged" } else { " budget" },
        seed,
    );
    (outcome, r.placement.report.contracts)
}

// ── Per-constraint-type satisfaction summary ──

struct TypeStats {
    count: usize,
    satisfied: usize,
    violated: usize,
    worst_metric: f64,
}

/// Aggregate placement contracts by kind and print per-type satisfaction.
fn print_constraint_summary(all_contracts: &[&ConstraintContract]) {
    if all_contracts.is_empty() { return; }
    let mut by_kind: HashMap<&str, TypeStats> = HashMap::new();
    for c in all_contracts {
        let e = by_kind.entry(&c.kind).or_insert(TypeStats {
            count: 0, satisfied: 0, violated: 0, worst_metric: 0.0,
        });
        e.count += 1;
        match c.status() {
            ConstraintStatus::Satisfied => e.satisfied += 1,
            ConstraintStatus::Violated => {
                e.violated += 1;
                if let Some(m) = c.violation_metric() {
                    if m > e.worst_metric { e.worst_metric = m; }
                }
            }
            _ => {}
        }
    }
    println!("\n  {:<22} {:>5} {:>5} {:>5} {:>6}  {:>12}",
        "Constraint type", "total", "sat", "viol", "rate", "worst(um)");
    println!("  {}", "-".repeat(70));
    let mut kinds: Vec<&&str> = by_kind.keys().collect();
    kinds.sort();
    let (mut total, mut total_sat) = (0usize, 0usize);
    for kind in kinds {
        let s = &by_kind[kind];
        let rate = if s.count > 0 { s.satisfied as f64 / s.count as f64 } else { 0.0 };
        let worst = if s.violated > 0 { format!("{:.3}", s.worst_metric) } else { "-".into() };
        println!("  {:<22} {:>5} {:>5} {:>5} {:>5.0}%  {:>12}",
            kind, s.count, s.satisfied, s.violated, rate * 100.0, worst);
        total += s.count;
        total_sat += s.satisfied;
    }
    let overall = if total > 0 { total_sat as f64 / total as f64 } else { 0.0 };
    println!("  {}", "-".repeat(70));
    println!("  {:<22} {:>5} {:>5} {:>5} {:>5.0}%",
        "OVERALL", total, total_sat, total - total_sat, overall * 100.0);
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

    let mut circuits = discover_all(suite);
    // ponytail: extra args = name substrings (filter) or a single number (take first N)
    let filters: Vec<String> = std::env::args().skip(2).collect();
    if let [n] = filters.as_slice() {
        if let Ok(n) = n.parse::<usize>() {
            circuits.truncate(n);
        }
    }
    if !filters.is_empty() && !circuits.is_empty() && filters.iter().any(|f| f.parse::<usize>().is_err()) {
        circuits.retain(|c| filters.iter().any(|f| c.name == *f));
    }
    let seed = std::env::var("PNR_BENCH_SEED")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1);
    println!("Discovered {} circuits (seed {seed})", circuits.len());

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
        let (outcome, contracts) = run_circuit(c, pdk, deck_json, &pdk_json, seed);
        rows.push(Row {
            name: c.name.clone(),
            suite: format!("{:?}", c.suite),
            outcome,
            ms: t.elapsed().as_millis(),
            contracts,
        });
        let r = rows.last().unwrap();
        println!("  [{:>11}] {:32} {:6} ms  {}",
            r.suite, r.name, r.ms, r.outcome);
    }

    let ok = rows.iter().filter(|r| r.outcome.contains("cells,")).count();
    println!("{ok}/{} circuits placed+routed; debug in target/bench_debug/", rows.len());

    // Per-constraint-type satisfaction summary across all circuits
    let all_contracts: Vec<&ConstraintContract> = rows.iter()
        .flat_map(|r| r.contracts.iter())
        .collect();
    if !all_contracts.is_empty() {
        println!("\n── Constraint satisfaction ──");
        print_constraint_summary(&all_contracts);
    }

    // Export SVGs for successful circuits
    let assets = repo_root().join("assets");
    let _ = std::fs::create_dir_all(&assets);
    let mut exported = 0;
    for r in &rows {
        if !r.outcome.contains("cells,") { continue; }
        let gds = Path::new("target/bench_debug").join(&r.name).join(format!("{}.gds", r.name));
        if let Ok(data) = std::fs::read(&gds) {
            let pdk_json = std::fs::read_to_string(
                repo_root().join("pdks/sky130.json")
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

#[cfg(test)]
mod tests {
    use super::Compactness;

    #[test]
    fn compactness_separates_drawn_and_planning_footprints() {
        let result = Compactness::measure(
            &[1_000],
            &[1_000],
            &[(1_000, 1_000)],
            (2_000, 2_000),
            1_000,
        );
        assert!((result.die_area_um2 - 4.0).abs() < 1e-9);
        assert!((result.drawn_util_pct - 25.0).abs() < 1e-9);
        assert!((result.planning_util_pct - 100.0).abs() < 1e-9);
        assert!((result.bbox_fill_pct - 100.0).abs() < 1e-9);
    }
}
