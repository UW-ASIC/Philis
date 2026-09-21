//! Benchmark harness: discover fixture circuits, run the FULL flow via the same
//! `library` entry points the `philis` CLI uses (`library::run` → `library::
//! signoff`) on each, print a summary table, and emit GDS + SVG artifacts.
//!
//!   cargo run --release -p benchmark --bin bench [local|align|magical|tinytapeout|all]
//!
//! Env: `PNR_BENCH_SEED` (default 1), `PNR_BENCH_PDK` (deck override, see
//! `pdk_path`).
//!
//! Debug artifacts (`<name>.gds`, `signoff.txt`) land in
//! `target/bench_debug/<name>/`; SVGs in `assets/`.
//!
//! Migrated from `tools/benchmark`: fixture discovery + SPICE preprocessing are
//! unchanged; the per-circuit run now drives `library` (the CLI's flow) and the
//! metrics table is rebuilt from the public `Solution` + `RunStats` + `signoff`
//! report. Two capabilities the old harness had are gone with the new API and
//! noted where they mattered: constrained-interface sidecars (`library::run`
//! takes no `InterfaceSpec`) and the plan-utilisation column (`Layout` no longer
//! carries a die / cell-margin).

mod fixtures;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use annotator::{annotate, AnnotationConfig, NoInference};
use fixtures::{
    cleanup_fixtures, clone_repos_if_needed, discover_all, preprocess_spice, BenchmarkCircuit,
    Suite,
};
use library::visualizer;
use library::{gds, Config, Macros};
use pnr_core::{Layout, NetId};
use verify::Pdk;

/// Guard against pathological fixtures (SA cost scales with cell count even with
/// the bucket index; raise as the annealer earns it).
const MAX_CELLS: usize = 800;

/// Feedback-loop cap: 5 iterations keeps the full suite tractable (the hardened
/// per-iter pipeline is minutes on big analog blocks). Mirrors the old harness's
/// `max_feedback_iters = 5`.
const FEEDBACK_ITERS: u32 = 5;

/// Repo root: one level up from benchmarks/.
fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

/// Deck per suite: the finfet-flavoured suites default to `generic_finfet`, the
/// rest to `sky130`. `PNR_BENCH_PDK` overrides for every suite — either a deck
/// name under `pdks/` (`sky130`, `generic_finfet`) or a path — so the same
/// fixtures can be run against both decks and compared.
fn pdk_path(suite: Suite) -> PathBuf {
    let root = repo_root();
    if let Ok(o) = std::env::var("PNR_BENCH_PDK") {
        let p = PathBuf::from(&o);
        return if p.is_file() { p } else { root.join(format!("pdks/{o}.json")) };
    }
    match suite {
        Suite::Align | Suite::Magical => root.join("pdks/generic_finfet.json"),
        _ => root.join("pdks/sky130.json"),
    }
}

/// Per-PDK render tables: `layer_gds[LayerId.0] = (gds_layer, gds_datatype)` for
/// the GDS writer, and `layer_names[(gds_layer, gds_datatype)] = name` for SVG
/// legends. Both are derived by joining the deck's `layers` block (name →
/// gds numbers) with the `Pdk`'s name → `LayerId` table.
fn layer_tables(pdk_json: &str, pdk: &Pdk) -> (Vec<(u16, u16)>, visualizer::LayerMap) {
    let mut by_name: HashMap<String, (u16, u16)> = HashMap::new();
    let mut names = visualizer::LayerMap::new();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(pdk_json) {
        if let Some(layers) = v.get("layers").and_then(|l| l.as_object()) {
            for (name, val) in layers {
                // Deck spells a layer as `[gds_layer, datatype]`; the older
                // `{"layer": .., "datatype": ..}` object form is kept readable
                // so foreign decks don't silently emit everything on (0, 0) —
                // which is exactly what happened when the schema moved: every
                // GDS artifact went out on one layer and external tools
                // (KLayout, magic) saw a blank design while in-memory signoff
                // stayed correct.
                let pair = match (val.as_array(), val.get("layer"), val.get("datatype")) {
                    (Some(a), ..) if a.len() >= 2 => {
                        a[0].as_i64().zip(a[1].as_i64())
                    }
                    (_, Some(l), Some(d)) => l.as_i64().zip(d.as_i64()),
                    _ => None,
                };
                if let Some((l, d)) = pair {
                    by_name.insert(name.clone(), (l as u16, d as u16));
                    names.insert((l as i32, d as i32), name.clone());
                }
            }
        }
    }
    let max_id = pdk.layers.iter().map(|(_, id)| id.0 as usize).max().unwrap_or(0);
    let mut layer_gds = vec![(0u16, 0u16); max_id + 1];
    for (name, id) in &pdk.layers {
        if let Some(&(l, d)) = by_name.get(name) {
            layer_gds[id.0 as usize] = (l, d);
        }
    }
    (layer_gds, names)
}

struct Row {
    name: String,
    suite: String,
    outcome: String,
    ms: u128,
    /// Per-constraint-type satisfaction, evaluated against the placed layout.
    contracts: Vec<ContractStat>,
}

/// One placement-constraint batch's satisfaction at the final layout.
struct ContractStat {
    kind: String,
    total: usize,
    violated: usize,
    /// Worst per-rule cost among the violating rules (severity proxy).
    worst: f32,
}

#[derive(Debug)]
struct Compactness {
    area_um2: f64,
    drawn_util_pct: f64,
    bbox_fill_pct: f64,
}

impl Compactness {
    /// Footprint metrics from device half-extents. Without a die/cell-margin on
    /// the new `Layout`, the envelope is the device bounding box; `drawn_util`
    /// and `bbox_fill` therefore coincide (both drawn-area / bbox-area) — kept as
    /// two columns for table-shape parity with the old harness.
    fn measure(l: &Layout) -> Self {
        let n = l.x.len();
        let drawn_area: f64 = (0..n)
            .map(|i| f64::from(2 * l.hw[i]) * f64::from(2 * l.hh[i]))
            .sum();
        let bbox_area = (0..n)
            .fold(None::<(i64, i64, i64, i64)>, |b, i| {
                let cell = (
                    i64::from(l.x[i] - l.hw[i]),
                    i64::from(l.y[i] - l.hh[i]),
                    i64::from(l.x[i] + l.hw[i]),
                    i64::from(l.y[i] + l.hh[i]),
                );
                Some(match b {
                    None => cell,
                    Some((x0, y0, x1, y1)) => {
                        (x0.min(cell.0), y0.min(cell.1), x1.max(cell.2), y1.max(cell.3))
                    }
                })
            })
            .map_or(0.0, |(x0, y0, x1, y1)| {
                (x1 - x0).max(0) as f64 * (y1 - y0).max(0) as f64
            });
        let pct = |a: f64, env: f64| if env > 0.0 { 100.0 * a / env } else { 0.0 };
        Self {
            area_um2: bbox_area / 1_000_000.0,
            drawn_util_pct: pct(drawn_area, bbox_area),
            bbox_fill_pct: pct(drawn_area, bbox_area),
        }
    }
}

/// Trim a rule's fully-qualified type name to its final path segment.
fn short_kind(kind: &str) -> String {
    kind.rsplit("::").next().unwrap_or(kind).trim_end_matches('>').to_string()
}

fn run_circuit(
    c: &BenchmarkCircuit,
    pdk: &Pdk,
    pdk_json_path: &Path,
    layer_gds: &[(u16, u16)],
    layer_names: &visualizer::LayerMap,
    seed: u64,
) -> (String, Vec<ContractStat>) {
    let raw = match std::fs::read_to_string(&c.spice_path) {
        Ok(t) => t,
        Err(e) => return (format!("read failed: {e}"), Vec::new()),
    };
    // Generic-netlist preprocessing (backslash joins, bare R/C, nfin->W).
    let text = preprocess_spice(&raw, pdk_json_path).unwrap_or(raw);

    // Pre-parse only to size-gate before committing to the full flow. Uses the
    // exact parser `run` uses, so the count is authoritative.
    let g = match library::parse(&text) {
        Ok(g) => g,
        Err(e) => return (format!("parse failed: {e}"), Vec::new()),
    };
    if g.devices.is_empty() {
        return ("no PDK devices resolved".into(), Vec::new());
    }
    if g.devices.len() > MAX_CELLS {
        return (format!("skipped ({} cells > {MAX_CELLS})", g.devices.len()), Vec::new());
    }
    // NOTE: constrained-interface sidecars (`<stem>.interface.json`) are not
    // applied — `library::run` exposes no `InterfaceSpec` hook. Present sidecars
    // are ignored; the circuit still runs fully auto-generated.

    let cfg = Config { seed, feedback_iters: FEEDBACK_ITERS, ..Config::default() };
    let sol = match library::run(&text, pdk, &Macros::default(), &cfg) {
        Ok(s) => s,
        Err(e) => return (format!("flow failed: {e:?}"), Vec::new()),
    };

    let report = library::signoff(&sol, pdk);

    // Signoff findings are structured by rule-name prefix: `{domain}/{rule}:{layer}`
    // with domains drc/erc/lvs, plus `engine/…` for a stage that could not run
    // (see `verify::signoff`).
    let mut drc = 0usize;
    let mut erc = 0usize;
    let mut lvs_mismatch = false;
    let mut engine = 0usize;
    for v in &report.hard_violations {
        if v.rule.starts_with("drc/") {
            drc += 1;
        } else if v.rule.starts_with("lvs/") {
            lvs_mismatch = true;
        } else if v.rule.starts_with("erc/") {
            erc += 1;
        } else if v.rule.starts_with("engine/") {
            engine += 1;
        }
    }

    // Routing quality, derived from the public `Routes` geometry.
    let n_nets = sol.netlist.nets.len();
    let wl: i64 = (0..sol.routes.wires.len())
        .map(|i| sol.routes.length(NetId(i as u16)))
        .sum();
    let unrouted = (0..n_nets)
        .filter(|&i| sol.routes.wires.get(i).map_or(true, |w| w.is_empty()))
        .count();

    let comp = Compactness::measure(&sol.layout);
    let s = &sol.stats;

    // `overuse` is `RunStats::route_overuse`: milli-budget normalised residual margins
    // since the Θ-realness step, not a raw track count — compare runs, not absolutes.
    // `esc` is D12's diagnostic and the reason `variant_escalations` exists: a nonzero
    // count means the run hit a **variant-space binding** (no arrangement of the chosen
    // variants was feasible), which is a different failure from a placement local
    // minimum and indistinguishable from it without this number.
    let outcome = format!(
        "{} cells, {} nets | WL {} nm, unrouted {} | overuse {} | DRC {} | LVS {} | ERC {}{} | C {:.1} fF | area {:.1} um2 | util drawn {:.1}%, bbox {:.1}% | best {}/{}{} | outer {}, esc {} | seed {}",
        sol.netlist.devices.len(),
        n_nets,
        wl,
        unrouted,
        s.route_overuse,
        drc,
        if lvs_mismatch { "MISMATCH" } else { "MATCH" },
        erc,
        if engine > 0 { format!(" | engine fails {engine}") } else { String::new() },
        report.cost,
        comp.area_um2,
        comp.drawn_util_pct,
        comp.bbox_fill_pct,
        s.best_iteration + 1,
        s.iterations,
        if s.converged { " converged" } else { " budget" },
        s.outer_iterations,
        s.variant_escalations,
        seed,
    );

    // Per-constraint-type satisfaction: re-annotate (pure) and evaluate each
    // placement batch against the final layout. These are the analog placement
    // constraints (the old "contracts") — DRC-feedback rules are not included.
    let problem = annotate(&sol.netlist, &NoInference, &AnnotationConfig::default());
    let mut contracts = Vec::new();
    for batch in problem.placement.hard.iter().chain(problem.placement.cost.iter()) {
        let total = batch.count();
        if total == 0 {
            continue;
        }
        contracts.push(ContractStat {
            kind: short_kind(batch.kind()),
            total,
            violated: batch.violations(&sol.layout) as usize,
            worst: batch.worst_cost(&sol.layout),
        });
    }

    // Emit GDS (target/bench_debug/<name>/) + SVG (assets/) for the solution.
    let shapes = sol.geometry();
    let debug_dir = Path::new("target/bench_debug").join(&c.name);
    let _ = std::fs::create_dir_all(&debug_dir);
    let gds_bytes = gds::emit(&shapes, layer_gds);
    let _ = std::fs::write(debug_dir.join(format!("{}.gds", c.name)), &gds_bytes);
    let _ = std::fs::write(debug_dir.join("signoff.txt"), &outcome);
    // Every hard violation verbatim — the summary counts alone can't say which rule fired.
    let detail: String =
        report.hard_violations.iter().map(|v| format!("{}\t{}\n", v.rule, v.margin)).collect();
    let _ = std::fs::write(debug_dir.join("violations.txt"), detail);
    // DRC again, unsummarised: `signoff` keeps only (rule, margin), and without the
    // representative x/y there is no way to tell a cell-internal violation from one
    // the router created.
    let located: String = verify::drc(&shapes, &[], pdk)
        .iter()
        .map(|f| {
            format!("{}\t{}\tmargin={} nm\t({}, {})\n", f.rule, f.layer, f.margin_nm, f.x, f.y)
        })
        .collect();
    let _ = std::fs::write(debug_dir.join("drc_located.txt"), located);
    let assets = repo_root().join("assets");
    let _ = std::fs::create_dir_all(&assets);
    let svg = visualizer::export_svg(&gds_bytes, layer_names);
    let _ = std::fs::write(assets.join(format!("{}.svg", c.name)), &svg);

    (outcome, contracts)
}

// ── Per-constraint-type satisfaction summary ──

struct TypeStats {
    count: usize,
    satisfied: usize,
    violated: usize,
    worst_metric: f32,
}

fn print_constraint_summary(all: &[&ContractStat]) {
    if all.is_empty() {
        return;
    }
    let mut by_kind: HashMap<&str, TypeStats> = HashMap::new();
    for c in all {
        let e = by_kind.entry(&c.kind).or_insert(TypeStats {
            count: 0,
            satisfied: 0,
            violated: 0,
            worst_metric: 0.0,
        });
        e.count += c.total;
        e.satisfied += c.total - c.violated;
        e.violated += c.violated;
        if c.worst > e.worst_metric {
            e.worst_metric = c.worst;
        }
    }
    println!(
        "\n  {:<24} {:>5} {:>5} {:>5} {:>6}  {:>12}",
        "Constraint type", "total", "sat", "viol", "rate", "worst(cost)"
    );
    println!("  {}", "-".repeat(72));
    let mut kinds: Vec<&&str> = by_kind.keys().collect();
    kinds.sort();
    let (mut total, mut total_sat) = (0usize, 0usize);
    for kind in kinds {
        let s = &by_kind[kind];
        let rate = if s.count > 0 { s.satisfied as f64 / s.count as f64 } else { 0.0 };
        let worst = if s.violated > 0 { format!("{:.3}", s.worst_metric) } else { "-".into() };
        println!(
            "  {:<24} {:>5} {:>5} {:>5} {:>5.0}%  {:>12}",
            kind, s.count, s.satisfied, s.violated, rate * 100.0, worst
        );
        total += s.count;
        total_sat += s.satisfied;
    }
    let overall = if total > 0 { total_sat as f64 / total as f64 } else { 0.0 };
    println!("  {}", "-".repeat(72));
    println!(
        "  {:<24} {:>5} {:>5} {:>5} {:>5.0}%",
        "OVERALL", total, total_sat, total - total_sat, overall * 100.0
    );
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
    // Extra args = name substrings (filter) or a single number (take first N).
    let filters: Vec<String> = std::env::args().skip(2).collect();
    if let [n] = filters.as_slice() {
        if let Ok(n) = n.parse::<usize>() {
            circuits.truncate(n);
        }
    }
    if !filters.is_empty()
        && !circuits.is_empty()
        && filters.iter().any(|f| f.parse::<usize>().is_err())
    {
        circuits.retain(|c| filters.iter().any(|f| c.name == *f));
    }
    let seed = std::env::var("PNR_BENCH_SEED")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(1);
    println!("Discovered {} circuits (seed {seed})", circuits.len());

    // Cache loaded PDKs + render tables by path (sky130 vs generic_finfet).
    let mut pdk_cache: HashMap<PathBuf, (Pdk, Vec<(u16, u16)>, visualizer::LayerMap)> =
        HashMap::new();

    let mut rows = Vec::new();
    for c in &circuits {
        let pdk_json_path = pdk_path(c.suite);
        if !pdk_cache.contains_key(&pdk_json_path) {
            let deck = match std::fs::read_to_string(&pdk_json_path) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("PDK load failed ({}): {e}", pdk_json_path.display());
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
            let (lg, ln) = layer_tables(&deck, &p);
            pdk_cache.insert(pdk_json_path.clone(), (p, lg, ln));
        }
        let (pdk, layer_gds, layer_names) = &pdk_cache[&pdk_json_path];

        let t = Instant::now();
        let (outcome, contracts) =
            run_circuit(c, pdk, &pdk_json_path, layer_gds, layer_names, seed);
        rows.push(Row {
            name: c.name.clone(),
            suite: format!("{:?}", c.suite),
            outcome,
            ms: t.elapsed().as_millis(),
            contracts,
        });
        let r = rows.last().unwrap();
        let deck = pdk_json_path.file_stem().unwrap_or_default().to_string_lossy();
        println!("  [{:>11}/{deck}] {:32} {:6} ms  {}", r.suite, r.name, r.ms, r.outcome);
    }

    let ok = rows.iter().filter(|r| r.outcome.contains("cells,")).count();
    println!("{ok}/{} circuits placed+routed; debug in target/bench_debug/", rows.len());

    let all_contracts: Vec<&ContractStat> =
        rows.iter().flat_map(|r| r.contracts.iter()).collect();
    if !all_contracts.is_empty() {
        println!("\n── Constraint satisfaction ──");
        print_constraint_summary(&all_contracts);
    }

    let exported = rows.iter().filter(|r| r.outcome.contains("cells,")).count();
    if exported > 0 {
        println!("{exported} SVGs exported to assets/");
    }

    if suite != Suite::Local {
        cleanup_fixtures();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_kind_trims_path() {
        assert_eq!(short_kind("philis::placement::ThermalGradient"), "ThermalGradient");
        assert_eq!(short_kind("Foo"), "Foo");
    }

    // Compactness of a single 1×1 µm device at origin: 1 µm² bbox, 100% fill.
    #[test]
    fn compactness_single_device() {
        let l = Layout {
            x: vec![0],
            y: vec![0],
            hw: vec![500],
            hh: vec![500],
            axis: vec![],
            groups: vec![],
            orient: vec![pnr_core::Orient::default()],
            variant: vec![0],
            branch: vec![],
            power_uw: vec![0],
            temp_mc: vec![0],
        };
        let c = Compactness::measure(&l);
        assert!((c.area_um2 - 1.0).abs() < 1e-9);
        assert!((c.drawn_util_pct - 100.0).abs() < 1e-9);
        assert!((c.bbox_fill_pct - 100.0).abs() < 1e-9);
    }
}
