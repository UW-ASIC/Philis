//! Benchmark harness: discover fixture circuits, run the FULL flow via the same
//! `library` entry points the `philis` CLI uses (`library::run` → `library::
//! signoff`) on each, print a summary table, and emit GDS + SVG artifacts.
//!
//!   cargo run --release -p benchmark --bin bench [local|align|magical|tinytapeout|all]
//!   cargo run --release -p benchmark --bin bench -- --pex-cal   # target/bench/pex_cal.json
//!
//! Env: `PNR_BENCH_SEED` (default 1), `PNR_BENCH_STARTS` (search starts,
//! default the library's), `PNR_BENCH_PDK` (deck override, see
//! `pdk_path`).
//!
//! Debug artifacts (`<name>.gds`, `signoff.txt`, `violations.txt`,
//! `drc_located.txt`, and for `signoff_xcheck.py` `<name>.lvs.gds`,
//! `<name>.ref.spice`, `caps.json`) land in `target/bench_debug/<name>/`; SVGs in `assets/`.
//! `<stem>.interface.json` sidecars are ignored (`library::run` takes none).

mod fixtures;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use fixtures::{
    cleanup_fixtures, clone_repos_if_needed, discover_all, preprocess_spice, BenchmarkCircuit,
    Suite,
};
use library::visualizer;
use library::{Config, Macros};
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

struct Row {
    name: String,
    suite: String,
    outcome: String,
    ms: u128,
    /// Per-constraint-type satisfaction, evaluated against the placed layout.
    contracts: Vec<ContractStat>,
}

/// One constraint batch's satisfaction at the final layout / routes.
struct ContractStat {
    kind: String,
    /// Enforcement arm: `hard`, `budget`, or `cost` (cost-only kinds).
    arm: &'static str,
    circuit: String,
    total: usize,
    violated: usize,
    /// Rules with nothing to check (e.g. DTI on a trench-less process).
    na: usize,
    /// Rules whose inputs are missing ([`analog::Rule::known`]): neither
    /// satisfied nor violated. Takes precedence over `na` (an unpowered die is
    /// unknown power, not a verified-cool one).
    unk: usize,
    /// Largest spent fraction of a rule's budget (`1.0` = at the spec).
    usage: Option<f32>,
    /// Σ residual — overshoot in budgets' worth.
    theta: f64,
}

fn stat<S>(arm: &'static str, circuit: &str, b: &dyn analog::RuleBatch<S>, s: &S) -> Option<ContractStat> {
    let total = b.count();
    (total > 0).then(|| ContractStat {
        kind: short_kind(b.kind()),
        arm,
        circuit: circuit.to_string(),
        total,
        violated: b.violations(s) as usize,
        na: (b.inapplicable(s) as usize).saturating_sub(b.unknown(s) as usize),
        unk: b.unknown(s) as usize,
        usage: b.worst_usage(s),
        theta: b.residual(s),
    })
}

/// `(bbox area µm², device area / bbox area %)` from device half-extents.
fn footprint(l: &Layout) -> (f64, f64) {
    let n = l.x.len();
    let drawn: f64 = (0..n).map(|i| f64::from(2 * l.hw[i]) * f64::from(2 * l.hh[i])).sum();
    if n == 0 {
        return (0.0, 0.0);
    }
    let x0 = (0..n).map(|i| l.x[i] - l.hw[i]).min().unwrap();
    let y0 = (0..n).map(|i| l.y[i] - l.hh[i]).min().unwrap();
    let x1 = (0..n).map(|i| l.x[i] + l.hw[i]).max().unwrap();
    let y1 = (0..n).map(|i| l.y[i] + l.hh[i]).max().unwrap();
    let bbox = f64::from(x1 - x0) * f64::from(y1 - y0);
    (bbox / 1e6, if bbox > 0.0 { 100.0 * drawn / bbox } else { 0.0 })
}

/// Device diffusion (the placed cells' `diff`-role shapes, rings and fill
/// excluded) as % of the block area: what `util`, which counts ring halos
/// as device, hides.
fn active(sol: &library::Solution, pdk: &Pdk, area_um2: f64) -> f64 {
    use pnr_core::Process;
    let Some(diff) = pdk.layer("diff") else { return 0.0 };
    let cells = pnr_core::place_macros(&sol.macros[..sol.layout.x.len()], &sol.layout);
    let drawn: f64 = cells.iter().flat_map(|m| &m.shapes).filter(|s| s.layer == diff).map(|s| f64::from(s.rect.w) * f64::from(s.rect.h)).sum();
    if area_um2 > 0.0 { 100.0 * drawn / (area_um2 * 1e6) } else { 0.0 }
}

/// Trim a rule's fully-qualified type name to its final path segment.
fn short_kind(kind: &str) -> String {
    kind.rsplit("::").next().unwrap_or(kind).trim_end_matches('>').to_string()
}

/// The operating point the bench biases with (ENV-01/02, PWR-01): the
/// synthesised mid-rail probe (`oppoint`) on the installed sky130 models, for
/// sky130 decks only. `None` (unbiased: thermal and EM read unknown) without
/// the models; ngspice missing is reported by the flow itself.
///
/// ponytail: no fixture ships a testbench, so every bias is the probe, which
/// its provenance says is not a sign-off condition.
fn op_config(pdk_json: &Path) -> Option<library::oppoint::OpConfig> {
    if !pdk_json.file_name()?.to_str()?.starts_with("sky130") {
        return None;
    }
    let root = std::env::var_os("PDK_ROOT")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".volare")))?;
    let lib = root.join("sky130A/libs.tech/ngspice/sky130.lib.spice");
    lib.is_file().then(|| library::oppoint::OpConfig {
        model_lib: Some(lib),
        params: library::oppoint::SKY130_NPN_NOMINAL.iter().map(|&(k, v)| (k.to_string(), v)).collect(),
        ..Default::default()
    })
}

/// Parse, size-gate and run the full flow on `c`; `Err` is the outcome text.
fn solve(c: &BenchmarkCircuit, pdk: &Pdk, pdk_json_path: &Path, seed: u64) -> Result<library::Solution, String> {
    let raw = std::fs::read_to_string(&c.spice_path).map_err(|e| format!("read failed: {e}"))?;
    // Generic-netlist preprocessing (backslash joins, bare R/C, nfin->W).
    let text = preprocess_spice(&raw, pdk_json_path).unwrap_or(raw);

    // Pre-parse only to size-gate before committing to the full flow. Uses the
    // exact parser and deck model table `run` uses, so the count is authoritative.
    let opts = library::ParseOptions { models: library::model_table(pdk), ..Default::default() };
    let g = library::spice_with(&text, &opts).map_err(|e| format!("parse failed: {e}"))?;
    if g.devices.is_empty() {
        return Err("no PDK devices resolved".into());
    }
    if g.devices.len() > MAX_CELLS {
        return Err(format!("skipped ({} cells > {MAX_CELLS})", g.devices.len()));
    }
    let mut cfg = Config { seed, feedback_iters: FEEDBACK_ITERS, op: op_config(pdk_json_path), ..Config::default() };
    // FLOW-08's T10 sweep: `PNR_BENCH_COLD_EVERY` overrides the default schedule.
    if let Some(n) = std::env::var("PNR_BENCH_COLD_EVERY").ok().and_then(|v| v.parse().ok()) {
        cfg.cold_every = n;
    }
    if let Some(k) = std::env::var("PNR_BENCH_STARTS").ok().and_then(|v| v.parse().ok()) {
        cfg.starts = k;
    }
    if let Ok(n) = std::env::var("LOCAL_ESD") { cfg.esd = Some(library::EsdSpec { hbm_v: 2000.0, nets: vec![n] }); }
    library::run(&text, pdk, &Macros::default(), &cfg).map_err(|e| format!("flow failed: {e:?}"))
}

fn run_circuit(
    c: &BenchmarkCircuit,
    pdk: &Pdk,
    pdk_json_path: &Path,
    layer_names: &visualizer::LayerMap,
    seed: u64,
) -> (String, Vec<ContractStat>) {
    let sol = match solve(c, pdk, pdk_json_path, seed) {
        Ok(s) => s,
        Err(e) => return (e, Vec::new()),
    };

    let signoff = library::signoff(&sol, pdk);
    let report = &signoff.report;

    // Signoff errors are structured by rule-name prefix: `{domain}/{rule}:{layer}`
    // with domains drc/erc/lvs, plus `engine/…` for a stage that could not run
    // (see `verify::signoff_checked`). Deck warnings are `signoff.warnings`,
    // never counted here.
    let mut drc = 0usize;
    let mut erc = 0usize;
    let mut lvs_mismatch = false;
    let mut unverified = 0i64;
    let mut engine = 0usize;
    for v in &report.hard_violations {
        if v.rule.starts_with("drc/") {
            drc += 1;
        } else if v.rule.starts_with("lvs/") {
            lvs_mismatch = true;
        } else if v.rule.starts_with("lvs-coverage/") {
            unverified += v.margin;
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
    // A net no drawn cell has a pin on (its devices are all undrawable, a
    // `cell/undrawable` finding) has nothing to route: not unrouted.
    let pinned: std::collections::HashSet<NetId> =
        sol.macros.iter().filter(|m| !m.shapes.is_empty()).flat_map(|m| &m.pins).map(|p| p.net).collect();
    let unrouted = (0..n_nets)
        .filter(|&i| pinned.contains(&NetId(i as u16)) && sol.routes.wires.get(i).map_or(true, |w| w.is_empty()))
        .count();
    let undrawable = report.hard_violations.iter().filter(|v| v.rule.starts_with("cell/undrawable")).count();

    for r in sol.metadata.routing.iter().filter(|r| r.kind.contains("Esd")) { eprintln!("LOCAL {} total {} viol {} unknown {} use {:?}", r.kind, r.total, r.violations, r.unknown, r.usage); }
    if std::env::var("LOCAL_CENTRES").is_ok() { for (i, d) in sol.netlist.devices.iter().enumerate() { eprintln!("LOCAL centre {} {:?}", d.name, sol.layout.centre(pnr_core::Target::Device(pnr_core::DeviceId(i as _)))); } }
    let (area_um2, util_pct) = footprint(&sol.layout);
    let active_pct = active(&sol, pdk, area_um2);
    let s = &sol.stats;
    let problem = annotator::annotate(&sol.netlist, &library::annotation(pdk, &annotator::AnnotationConfig::default()));
    let names: Vec<String> = sol.netlist.nets.iter().map(|n| n.name.clone()).collect();

    // `overuse` is milli-budget normalised residual margin, not a track count.
    // `esc` > 0 means a variant-space binding (no arrangement of the chosen
    // variants was feasible), not a placement local minimum.
    // A counter its owning item has not built yet prints `n/a`, never a 0.
    let na = |v: Option<u64>| v.map_or_else(|| "n/a".to_string(), |v| v.to_string());
    let outcome = format!(
        "{} cells, {} nets | WL {} nm, unrouted {}{} | route hard {} | overuse {} | DRC {} | LVS {} | ERC {}{} | warnings {} | skipped [{}] | EM/IR ran {}/{} | C total {:.1} fF, sig {:.1} fF | key tier {:.1} | area {:.1} um2 | util {:.1}% | active {:.1}% | best {}/{}{} | outer {}, esc {}, pruned {} | seed {} | bias {} | EM {} | usage {:.3} | lattice off {} | overlap {:.0} nm2 | clr residue {:.0} nm2 | matched mismatch {} | islands extra {} | clusters extra {} | dp temps {}, proposals {}, accepted {}, decode fail {}, matched incompat {} | axes on lattice {}/{}",
        sol.netlist.devices.len(),
        n_nets,
        wl,
        unrouted,
        if undrawable > 0 { format!(" | undrawable {undrawable}") } else { String::new() },
        s.route_hard,
        s.route_overuse,
        drc,
        // MATCH only when every device was compared; PARTIAL(n): n devices
        // no deck recogniser extracts, the rest matched.
        match (lvs_mismatch, unverified) {
            (true, _) => "MISMATCH".to_string(),
            (false, 0) => "MATCH".to_string(),
            (false, n) => format!("PARTIAL({n})"),
        },
        erc,
        if engine > 0 { format!(" | engine fails {engine}") } else { String::new() },
        signoff.warnings.len(),
        signoff.coverage.skipped_rules.iter().map(|(r, _)| r.as_str()).collect::<Vec<_>>().join(", "),
        signoff.coverage.em_ir.0,
        signoff.coverage.em_ir.1,
        // Total (every net, coupling on both ends, rails included) and
        // signal-class C (NaN on a label short), both from this signoff of the final (filled)
        // layout; then the search key's C tier (`RunStats::c_tier`), from
        // the winning epoch's own signoff before fill.
        report.cost,
        library::signoff_c_tier(&signoff, &names, &problem.net_classes, &[]),
        s.c_tier,
        area_um2,
        util_pct,
        active_pct,
        s.best_iteration + 1,
        s.iterations,
        format!(" {:?} warm {}", s.stop, s.warm_epochs),
        s.outer_iterations,
        s.variant_escalations,
        s.pruned,
        seed,
        sol.metadata.bias.as_ref().map_or_else(
            || "none".to_string(),
            |b| format!(
                "{} uW, {}, EM at {:.3} K ({}), aging pairs {} (max dVds {:.1} mV), V-rating unknown {}",
                b.total_power_uw,
                if b.provenance.starts_with("SYNTH") { "probe" } else { "testbench" },
                b.em_temp_k,
                b.em_derate,
                sol.metadata.aging.len(),
                sol.metadata.aging.iter().map(|a| a.2).fold(0.0, f64::max),
                sol.metadata.voltage_unknown
            )
        ),
        // Per fixture (REL T3/T4): nets checked, of them violated, unknown, and
        // the worst known net's need/have (T3's `min(w/need) ≥ 1` is `use ≤ 1`).
        sol.metadata.routing.iter().find(|r| r.kind == "Electromigration").map_or_else(
            || "none".to_string(),
            |r| format!(
                "known {} (viol {}), unknown {}, max use {}",
                r.total - r.unknown,
                r.violations,
                r.unknown,
                r.usage.map_or_else(|| "none".to_string(), |u| format!("{u:.3}"))
            )
        ),
        s.place.area_usage,
        s.place.lattice_off,
        s.place.overlap_nm2,
        s.place.clearance_residue_nm2,
        s.place.matched_geometry_mismatch,
        na(s.place.islands_extra.map(u64::from)),
        na(s.place.clusters_extra.map(u64::from)),
        s.dp.temps,
        s.dp.proposals,
        s.dp.accepted,
        na(s.dp.decode_fail),
        na(s.dp.matched_incompatible.map(u64::from)),
        s.dp.axes_on_lattice,
        s.dp.axes,
    );

    // Per-constraint-type satisfaction: the run's own cell-space placement
    // rules against the final layout (a cost copy of a hard/budget batch is
    // skipped), and the routing rules against the final routes.
    let (p, l) = (&sol.placement, &sol.layout);
    let enforced: Vec<&str> = p.hard.iter().chain(&p.budget).map(|b| b.kind()).collect();
    let mut contracts: Vec<ContractStat> = p.hard.iter().map(|b| ("hard", b))
        .chain(p.budget.iter().map(|b| ("budget", b)))
        .chain(p.cost.iter().filter(|b| !enforced.contains(&b.kind())).map(|b| ("cost", b)))
        .filter_map(|(arm, b)| stat(arm, &c.name, b.as_ref(), l))
        .collect();
    let routing = &problem.routing;
    contracts.extend(
        routing.hard.iter().map(|b| ("hard", b))
            .chain(routing.budget.iter().map(|b| ("budget", b)))
            .filter_map(|(arm, b)| stat(arm, &c.name, b.as_ref(), &sol.routes)),
    );
    // Families the library adds from the op point (EM, IR drop) exist only in
    // the run's own report: take those rows from its metadata.
    let seen: Vec<String> = contracts.iter().map(|k| k.kind.clone()).collect();
    contracts.extend(sol.metadata.routing.iter().filter(|r| !seen.contains(&r.kind)).map(|r| ContractStat {
        kind: r.kind.clone(),
        arm: if r.arm == library::metadata::Arm::Hard { "hard" } else { "budget" },
        circuit: c.name.clone(),
        total: r.total,
        violated: r.violations,
        na: 0,
        unk: r.unknown,
        usage: r.usage,
        theta: r.residual,
    }));

    // Emit GDS (target/bench_debug/<name>/) + SVG (assets/) for the solution.
    let shapes = sol.geometry();
    let debug_dir = Path::new("target/bench_debug").join(&c.name);
    let _ = std::fs::create_dir_all(&debug_dir);
    let gds_bytes = library::export_gds(&sol, pdk, &c.name, &[]).unwrap_or_else(|e| panic!("{}: {e}", c.name));
    let _ = std::fs::write(debug_dir.join(format!("{}.gds", c.name)), &gds_bytes);
    // Foundry LVS inputs for `signoff_xcheck.py`: a GDS labelling only the schematic
    // ports (magic/KLayout then see the same pins as the `.subckt`) and the matching
    // reference with dummies. `<name>.gds` stays all-labelled (DRC, magic C per net).
    let ports: Vec<String> = sol.netlist.ports.iter().map(|n| sol.netlist.nets[n.0 as usize].name.clone()).collect();
    let lvs_gds = debug_dir.join(format!("{}.lvs.gds", c.name));
    match library::export_gds(&sol, pdk, &c.name, &ports) {
        Ok(b) => drop(std::fs::write(&lvs_gds, b)),
        Err(e) => {
            eprintln!("{}.lvs.gds not written: {e}", c.name);
            let _ = std::fs::remove_file(&lvs_gds);
        }
    }
    let _ = std::fs::write(debug_dir.join(format!("{}.ref.spice", c.name)), library::reference_spice(&sol, pdk, &c.name, &ports));
    // `[net, other | null, fF]` rows of the signoff extraction.
    let _ = std::fs::write(debug_dir.join("caps.json"), serde_json::to_string(&signoff.caps).expect("caps serialise"));
    match library::post_layout_spice(&sol, pdk, &c.name) {
        Ok(s) => drop(std::fs::write(debug_dir.join(format!("{}_pex.spice", c.name)), s)),
        Err(e) => {
            eprintln!("{}_pex.spice not written: {e}", c.name);
            // A previous run's file would read as this layout's extraction.
            let _ = std::fs::remove_file(debug_dir.join(format!("{}_pex.spice", c.name)));
        }
    }
    let _ = std::fs::write(debug_dir.join("signoff.txt"), format!("{outcome}\n{}", signoff.coverage));
    // Every hard violation verbatim — the summary counts alone can't say which rule fired.
    // Then each routing hard rule the run itself scored, per net it violates,
    // with its own `Rule::residual` (EM: `(need − have)/need`; per net: REL-03),
    // then every deck warning, marked as such.
    let net_name = |n: u32| sol.netlist.nets.get(n as usize).map_or_else(|| format!("#{n}"), |x| x.name.clone());
    let detail: String = report
        .hard_violations
        .iter()
        .map(|v| format!("{}\t{}\n", v.rule, v.margin))
        .chain(sol.metadata.routing.iter().filter(|r| r.arm == library::metadata::Arm::Hard).flat_map(|r| {
            r.violated.iter().map(move |&(n, res)| format!("route/{}: net {}\t{res}\n", r.kind, net_name(n)))
        }))
        .chain(signoff.warnings.iter().map(|v| format!("warning {}\t{}\n", v.rule, v.margin)))
        .collect();
    let _ = std::fs::write(debug_dir.join("violations.txt"), detail);
    // DRC again, unsummarised: `signoff` keeps only (rule, margin), and without the
    // representative x/y there is no way to tell a cell-internal violation from one
    // the router created.
    let located: String = verify::drc(&shapes, &[], pdk)
        .iter()
        .map(|f| {
            format!("{}{}\t{}\tmargin={} {}\t({}, {})\n", if f.warning { "warning " } else { "" }, f.rule, f.layer, f.margin, f.unit, f.x, f.y)
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

/// One row per (kind, arm): counts, the worst budget usage (and where), Σ Θ.
fn print_constraint_summary(all: &[&ContractStat]) {
    if all.is_empty() {
        return;
    }
    let mut rows: Vec<(&str, &str)> = all.iter().map(|c| (c.kind.as_str(), c.arm)).collect();
    rows.sort_unstable();
    rows.dedup();
    println!(
        "\n  {:<20} {:<6} {:>5} {:>5} {:>5} {:>4} {:>4} {:>6}  {:>9} {:<16} {:>8}",
        "Constraint type", "arm", "total", "sat", "viol", "unk", "n/a", "rate", "max use", "(at)", "Θ"
    );
    println!("  {}", "-".repeat(101));
    let (mut total, mut sat, mut na, mut unk) = (0usize, 0usize, 0usize, 0usize);
    // Rate over the rules actually checked: unknown and n/a are neither.
    let rate = |s: usize, checked: usize| {
        if checked > 0 { format!("{:>5.0}%", 100.0 * s as f64 / checked as f64) } else { "     -".into() }
    };
    for (kind, arm) in rows {
        let cs: Vec<&&ContractStat> = all.iter().filter(|c| c.kind == kind && c.arm == arm).collect();
        let n: usize = cs.iter().map(|c| c.total).sum();
        let v: usize = cs.iter().map(|c| c.violated).sum();
        let a: usize = cs.iter().map(|c| c.na).sum();
        let u: usize = cs.iter().map(|c| c.unk).sum();
        let s = n.saturating_sub(v + a + u);
        let theta: f64 = cs.iter().map(|c| c.theta).sum();
        let worst = cs.iter().filter_map(|c| c.usage.map(|u| (u, c.circuit.as_str()))).max_by(|x, y| x.0.total_cmp(&y.0));
        let (use_s, at) = worst.map_or(("-".into(), ""), |(u, c)| (format!("{:.3}", u), c));
        let r = rate(s, s + v);
        println!(
            "  {kind:<20} {arm:<6} {n:>5} {s:>5} {v:>5} {u:>4} {a:>4} {r}  {use_s:>9} {at:<16} {theta:>8.3}"
        );
        total += n;
        sat += s;
        na += a;
        unk += u;
    }
    let viol = total - sat - na - unk;
    println!("  {}", "-".repeat(101));
    println!(
        "  {:<20} {:<6} {:>5} {:>5} {:>5} {:>4} {:>4} {}",
        "OVERALL", "", total, sat, viol, unk, na, rate(sat, sat + viol)
    );
    println!("  (max use = spent fraction of the tightest rule's budget; 1.000 = at spec. unk = inputs missing, never a pass; n/a = nothing to check. rate = sat / checked.)");
}

/// `--pex-cal` row (PERF-16): ground C per labelled signal net (supplies
/// excluded) unmerged, merged and field-solved (every such net), fF, plus
/// the analytical and field PEX-only wall times — the measurement gating
/// field solve on promoted epochs (kept only at ≤ 2× analytical).
fn pex_cal(sol: &library::Solution, pdk: &Pdk) -> serde_json::Value {
    let (shapes, pins, _) = library::signoff_inputs(sol, pdk);
    let mut nets: Vec<String> = pins.iter().map(|p| p.name.clone()).filter(|n| !sol.intent.supplies.iter().any(|s| &s.0 == n)).collect();
    nets.sort();
    nets.dedup();
    let run = |o: verify::ExtractOptions| library::signoff_with(sol, pdk, &o);
    let unmerged = run(verify::ExtractOptions { unmerged: true, ..Default::default() });
    let merged = run(verify::ExtractOptions::default());
    let field = run(verify::ExtractOptions { field_solve: nets.clone(), ..Default::default() });
    let mut checker = verify::Checker::new(pdk, false).expect("deck loads");
    let t = Instant::now();
    let pex_only = verify::Checks { drc: false, erc: false, lvs: false, pex: true };
    let analytic_ok = checker.run(&shapes, &pins, pex_only).is_ok();
    let t_pex = t.elapsed();
    let g = |s: &verify::Signoff, n: &str| s.caps.iter().find(|(a, o, _)| a == n && o.is_none()).map(|r| r.2);
    let rows: Vec<serde_json::Value> = nets
        .iter()
        .map(|n| serde_json::json!({ "net": n, "c_unmerged": g(&unmerged, n), "c_merged": g(&merged, n), "c_field": g(&field, n) }))
        .collect();
    let refused: Vec<&String> = field.coverage.skipped_rules.iter().filter(|(r, _)| r == "pex.field").map(|(_, w)| w).collect();
    serde_json::json!({
        "nets": rows,
        "c_total": { "unmerged": unmerged.report.cost, "merged": merged.report.cost, "field": field.report.cost },
        "t_pex_ms": analytic_ok.then(|| t_pex.as_secs_f64() * 1e3),
        "t_field_ms": field.pex_field.as_secs_f64() * 1e3,
        "field_refused": refused,
    })
}

fn main() {
    let pex_cal_mode = std::env::args().nth(1).as_deref() == Some("--pex-cal");
    let suite = std::env::args()
        .nth(1)
        .filter(|_| !pex_cal_mode)
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
    let mut pdk_cache: HashMap<PathBuf, (Pdk, visualizer::LayerMap)> =
        HashMap::new();

    let mut rows = Vec::new();
    let mut cal = serde_json::Map::new();
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
            // Drawn layers only: a derived layer's `(0, 0)` row names nothing.
            let gds = p.layer_gds();
            let pairs: Vec<_> = p.layers.iter().map(|(n, id)| (n.clone(), gds[id.0 as usize])).filter(|(_, g)| *g != (0, 0)).collect();
            let ln = visualizer::layer_names(&pairs);
            pdk_cache.insert(pdk_json_path.clone(), (p, ln));
        }
        let (pdk, layer_names) = &pdk_cache[&pdk_json_path];
        if pex_cal_mode {
            match solve(c, pdk, &pdk_json_path, seed) {
                Ok(sol) => {
                    let row = pex_cal(&sol, pdk);
                    println!("  {}: t_pex {} ms, t_field {} ms", c.name, row["t_pex_ms"], row["t_field_ms"]);
                    cal.insert(c.name.clone(), row);
                }
                Err(e) => println!("  {}: {e}", c.name),
            }
            continue;
        }

        let t = Instant::now();
        let (outcome, contracts) =
            run_circuit(c, pdk, &pdk_json_path, layer_names, seed);
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

    if pex_cal_mode {
        let out = Path::new("target/bench");
        let _ = std::fs::create_dir_all(out);
        let text = serde_json::to_string_pretty(&cal).expect("pex_cal serialises");
        std::fs::write(out.join("pex_cal.json"), &text).expect("write target/bench/pex_cal.json");
        println!("{text}");
        return;
    }
    let ok = rows.iter().filter(|r| r.outcome.contains("cells,")).count();
    println!(
        "{ok}/{} circuits placed+routed; debug in target/bench_debug/, SVGs in assets/",
        rows.len()
    );

    let all_contracts: Vec<&ContractStat> =
        rows.iter().flat_map(|r| r.contracts.iter()).collect();
    if !all_contracts.is_empty() {
        println!("\n── Constraint satisfaction ──");
        print_constraint_summary(&all_contracts);
    }

    if suite != Suite::Local {
        cleanup_fixtures();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A rule without its inputs is counted unknown, never satisfied.
    #[test]
    fn unknown_rules_are_not_counted_as_satisfied() {
        #[derive(Clone, Copy)]
        struct NoInput;
        impl analog::Rule for NoInput {
            type On = ();
            fn cost(self, _: &()) -> f32 {
                0.0
            }
            fn known(self, _: &()) -> bool {
                false
            }
        }
        let c = stat("cost", "x", &vec![NoInput; 3], &()).unwrap();
        assert_eq!((c.total, c.violated, c.unk, c.na), (3, 0, 3, 0));
    }

    #[test]
    fn short_kind_trims_path() {
        assert_eq!(short_kind("philis::placement::ThermalGradient"), "ThermalGradient");
        assert_eq!(short_kind("Foo"), "Foo");
    }

    // A single 1×1 µm device: 1 µm² bbox, 100% fill.
    #[test]
    fn footprint_single_device() {
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
            units: Default::default(),
        };
        let (area, util) = footprint(&l);
        assert!((area - 1.0).abs() < 1e-9);
        assert!((util - 100.0).abs() < 1e-9);
    }
}
