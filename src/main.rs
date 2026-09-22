use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde::Deserialize;

use pnr_core::backend::{ConstraintRecord, FlowConfig, InterfaceSpec};
use pnr_core::hier::run_flow_hier;
use pnr_core::orchestrator::run_flow;

/// Philis — analog place-and-route.
///
/// SPICE netlist + PDK deck in, placed/routed/verified GDS out.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the full place-and-route flow.
    Run(RunArgs),
    /// Convert a GDS file to SVG.
    Gds2svg {
        /// Input GDS file
        gds: PathBuf,
        /// PDK JSON (for layer names). Defaults to pdks/sky130.json.
        #[arg(short, long)]
        pdk: Option<PathBuf>,
        /// Output SVG path. Defaults to <input>.svg.
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
}

#[derive(Parser)]
struct RunArgs {
    /// SPICE netlist file (.spice)
    netlist: PathBuf,

    /// PDK deck file (.json)
    pdk: PathBuf,

    /// Run configuration JSON file. CLI flags override values in this file.
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Output directory for GDS, signoff, and debug artifacts
    #[arg(short, long)]
    out: Option<PathBuf>,

    // -- seeds & core knobs --------------------------------------------------

    /// RNG seed (placement + routing). Deterministic for a given seed.
    #[arg(short, long)]
    seed: Option<u64>,

    /// Target placement utilization (0.0–1.0)
    #[arg(short, long)]
    utilization: Option<f32>,

    /// Fixed die size in nm, WxH (e.g. 50000x40000). Overrides adaptive sizing.
    #[arg(long, value_parser = parse_die)]
    die: Option<(i32, i32)>,

    /// Interface spec JSON file (die + boundary pins).
    #[arg(long)]
    interface: Option<PathBuf>,

    // -- feedback loop -------------------------------------------------------

    /// Max placement/routing feedback iterations
    #[arg(long)]
    max_iters: Option<u32>,

    /// Feedback net-weight cap
    #[arg(long)]
    feedback_weight_cap: Option<f64>,

    /// Feedback convergence threshold
    #[arg(long)]
    feedback_threshold: Option<f64>,

    /// Feedback cell-inflation cap
    #[arg(long)]
    feedback_inflation_cap: Option<f64>,

    // -- geometry overrides --------------------------------------------------

    /// Via cut size in nm (0 = PDK default)
    #[arg(long)]
    via_size: Option<i32>,

    /// Landing pad width in nm (0 = derived)
    #[arg(long)]
    pad: Option<i32>,

    /// Local-interconnect strap width in nm (0 = PDK default)
    #[arg(long)]
    li_width: Option<i32>,

    // -- net classification --------------------------------------------------

    /// Mark a top-level net as a pad/package connection (repeatable)
    #[arg(long = "pad-net", num_args = 1)]
    pad_nets: Vec<String>,

    // -- placement SA tuning -------------------------------------------------

    /// Placement: global-phase max iterations
    #[arg(long)]
    pg_max_iters: Option<u32>,

    /// Placement: detailed-SA max iterations
    #[arg(long)]
    pd_max_iters: Option<u32>,

    /// Placement: detailed-SA cooling rate (0.0–1.0)
    #[arg(long)]
    pd_alpha: Option<f64>,

    /// Placement: outline-area weight in SA objective
    #[arg(long)]
    pd_outline_weight: Option<f64>,

    /// Placement: refinement max iterations
    #[arg(long)]
    pr_max_iters: Option<u32>,

    /// Placement: cell margin in nm
    #[arg(long)]
    cell_margin: Option<i32>,

    /// Placement: min die side in nm
    #[arg(long)]
    min_side: Option<i32>,

    // -- routing tuning ------------------------------------------------------

    /// Routing: detailed pitch in nm
    #[arg(long)]
    route_pitch: Option<i32>,

    /// Routing: wire width in nm
    #[arg(long)]
    wire_width: Option<i32>,

    /// Routing: via cost weight
    #[arg(long)]
    via_cost: Option<f32>,

    /// Routing: global max iterations
    #[arg(long)]
    rg_max_iters: Option<u32>,

    /// Routing: detailed max iterations
    #[arg(long)]
    rd_max_iters: Option<u32>,

    /// Routing: history decay factor (0.0–1.0)
    #[arg(long)]
    history_decay: Option<f32>,

    // -- output control ------------------------------------------------------

    /// Print detailed signoff report
    #[arg(short, long)]
    verbose: bool,

    /// Suppress the summary line
    #[arg(short, long)]
    quiet: bool,

    /// Hierarchical P&R: split into subckts, P&R each unique block once
    /// (leaves first, independent blocks in parallel), reuse as macro cells.
    #[arg(long)]
    hier: bool,
}

// ---------------------------------------------------------------------------
// Run config JSON — the single file that describes a PNR run.
//
// Every field is optional; omitted fields keep the backend defaults.
// CLI flags override anything in this file.
// ---------------------------------------------------------------------------

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct RunConfig {
    out: Option<String>,
    seed: Option<u64>,
    utilization: Option<f32>,
    die: Option<[i32; 2]>,
    interface: Option<PathBuf>,
    pad_nets: Vec<String>,

    // geometry
    via_size: Option<i32>,
    pad: Option<i32>,
    li_width: Option<i32>,

    // feedback loop
    max_iters: Option<u32>,
    feedback_weight_cap: Option<f64>,
    feedback_threshold: Option<f64>,
    feedback_inflation_cap: Option<f64>,

    // placement
    placement: Option<PlacementCfgJson>,

    // routing
    routing: Option<RoutingCfgJson>,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct PlacementCfgJson {
    cell_margin: Option<i32>,
    min_side: Option<i32>,
    global_max_iters: Option<u32>,
    detailed_max_iters: Option<u32>,
    detailed_alpha: Option<f64>,
    detailed_outline_weight: Option<f64>,
    refinement_max_iters: Option<u32>,
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct RoutingCfgJson {
    pitch: Option<i32>,
    wire_width: Option<i32>,
    via_cost: Option<f32>,
    global_max_iters: Option<u32>,
    detailed_max_iters: Option<u32>,
    history_decay: Option<f32>,
}

// ---------------------------------------------------------------------------

fn parse_die(s: &str) -> Result<(i32, i32), String> {
    let (w, h) = s.split_once('x').ok_or("expected WxH (e.g. 50000x40000)")?;
    Ok((
        w.parse().map_err(|_| "bad width")?,
        h.parse().map_err(|_| "bad height")?,
    ))
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Run(args) => cmd_run(args),
        Command::Gds2svg { gds, pdk, out } => cmd_gds2svg(gds, pdk, out),
    }
}

// ---------------------------------------------------------------------------
// philis run
// ---------------------------------------------------------------------------

fn cmd_run(args: RunArgs) -> ExitCode {
    // 1. Load the config JSON (if any) as the base, then overlay CLI flags.
    let base = match &args.config {
        Some(p) => match load_run_config(p) {
            Ok(c) => c,
            Err(e) => return fail(e),
        },
        None => RunConfig::default(),
    };

    let spice = match std::fs::read_to_string(&args.netlist) {
        Ok(s) => s,
        Err(e) => return fail(format!("read {}: {e}", args.netlist.display())),
    };
    let deck = match std::fs::read_to_string(&args.pdk) {
        Ok(s) => s,
        Err(e) => return fail(format!("read {}: {e}", args.pdk.display())),
    };

    let out_dir = PathBuf::from(
        args.out
            .as_deref()
            .or(base.out.as_deref().map(std::path::Path::new))
            .unwrap_or(std::path::Path::new("out")),
    );
    if let Err(e) = std::fs::create_dir_all(&out_dir) {
        return fail(format!("mkdir {}: {e}", out_dir.display()));
    }

    let interface = match load_interface(&args, &base) {
        Ok(i) => i,
        Err(e) => return fail(e),
    };

    let config = build_flow_config(&args, &base, out_dir, interface);

    let flow = if args.hier {
        run_flow_hier(&spice, &deck, &ConstraintRecord::default(), &config)
    } else {
        run_flow(&spice, &deck, &ConstraintRecord::default(), &config)
    };
    match flow {
        Ok(r) => {
            if args.verbose {
                print_verbose(&r);
            }
            if !args.quiet {
                let s = &r.signoff;
                println!(
                    "drc: {} blocking ({} waived) | lvs: {} | unrouted: {} | gds: {}",
                    s.drc_blocking.len(),
                    s.drc_waived_density,
                    if s.lvs.matched { "match" } else { &s.lvs.reason },
                    r.routing.report.unrouted.len(),
                    r.gds_path
                        .as_ref()
                        .map_or("<none>".into(), |p| p.display().to_string()),
                );
            }
            let clean = r.signoff.drc_blocking.is_empty()
                && r.signoff.lvs.matched
                && r.routing.report.unrouted.is_empty();
            if clean {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(e) => fail(e),
    }
}

/// Merge JSON base + CLI overrides into a FlowConfig.
/// CLI flags (Option::Some) always win; JSON fills in the rest; backend
/// defaults cover anything neither provides.
fn build_flow_config(
    args: &RunArgs,
    base: &RunConfig,
    out_dir: PathBuf,
    interface: Option<InterfaceSpec>,
) -> FlowConfig {
    macro_rules! pick {
        ($cli:expr, $json:expr) => {
            $cli.or($json)
        };
    }

    let seed = pick!(args.seed, base.seed).unwrap_or(1);
    let util = pick!(args.utilization, base.utilization).unwrap_or(0.4);
    let die = args.die.or(base.die.map(|[w, h]| (w, h)));

    let mut pad_nets = args.pad_nets.clone();
    if pad_nets.is_empty() {
        pad_nets = base.pad_nets.clone();
    }

    let bp = base.placement.as_ref();
    let br = base.routing.as_ref();

    let mut config = FlowConfig {
        debug_dir: Some(out_dir),
        via_size: pick!(args.via_size, base.via_size).unwrap_or(0),
        pad: pick!(args.pad, base.pad).unwrap_or(0),
        li_width: pick!(args.li_width, base.li_width).unwrap_or(0),
        max_feedback_iters: pick!(args.max_iters, base.max_iters).unwrap_or(100),
        feedback_weight_cap: pick!(args.feedback_weight_cap, base.feedback_weight_cap)
            .unwrap_or(5.0),
        feedback_threshold: pick!(args.feedback_threshold, base.feedback_threshold).unwrap_or(1.2),
        feedback_inflation_cap: pick!(args.feedback_inflation_cap, base.feedback_inflation_cap)
            .unwrap_or(1.5),
        pad_nets,
        interface,
        ..FlowConfig::default()
    };

    config.placement.seed = seed;
    config.routing.seed = seed;
    config.placement.utilization = util;

    if let Some(v) = die {
        config.placement.fixed_die = Some(v);
    }
    if let Some(v) = pick!(args.pg_max_iters, bp.and_then(|p| p.global_max_iters)) {
        config.placement.global.max_iters = v;
    }
    if let Some(v) = pick!(args.pd_max_iters, bp.and_then(|p| p.detailed_max_iters)) {
        config.placement.detailed.max_iters = v;
    }
    if let Some(v) = pick!(args.pd_alpha, bp.and_then(|p| p.detailed_alpha)) {
        config.placement.detailed.alpha = v;
    }
    if let Some(v) = pick!(args.pd_outline_weight, bp.and_then(|p| p.detailed_outline_weight)) {
        config.placement.detailed.outline_weight = v;
    }
    if let Some(v) = pick!(args.pr_max_iters, bp.and_then(|p| p.refinement_max_iters)) {
        config.placement.refinement.max_iters = v;
    }
    if let Some(v) = pick!(args.cell_margin, bp.and_then(|p| p.cell_margin)) {
        config.placement.cell_margin = v;
    }
    if let Some(v) = pick!(args.min_side, bp.and_then(|p| p.min_side)) {
        config.placement.min_side = v;
    }
    if let Some(v) = pick!(args.route_pitch, br.and_then(|r| r.pitch)) {
        config.routing.detailed.pitch = v;
    }
    if let Some(v) = pick!(args.wire_width, br.and_then(|r| r.wire_width)) {
        config.routing.detailed.wire_width = v;
    }
    if let Some(v) = pick!(args.via_cost, br.and_then(|r| r.via_cost)) {
        config.routing.detailed.via_cost = v;
    }
    if let Some(v) = pick!(args.rg_max_iters, br.and_then(|r| r.global_max_iters)) {
        config.routing.global.max_iters = v;
    }
    if let Some(v) = pick!(args.rd_max_iters, br.and_then(|r| r.detailed_max_iters)) {
        config.routing.detailed.max_iters = v;
    }
    if let Some(v) = pick!(args.history_decay, br.and_then(|r| r.history_decay)) {
        config.routing.history_decay = v;
    }

    config
}

fn load_run_config(path: &PathBuf) -> Result<RunConfig, String> {
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))
}

fn load_interface(args: &RunArgs, base: &RunConfig) -> Result<Option<InterfaceSpec>, String> {
    let iface_path = args.interface.as_ref().or(base.interface.as_ref());
    let die = args.die.or(base.die.map(|[w, h]| (w, h)));

    let Some(path) = iface_path else {
        return Ok(die.map(|(w, h)| InterfaceSpec {
            die: Some(pnr_core::backend::DieSpec { w, h }),
            pins: Vec::new(),
        }));
    };
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut spec: InterfaceSpec =
        serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))?;
    if let Some((w, h)) = die {
        spec.die = Some(pnr_core::backend::DieSpec { w, h });
    }
    spec.validate()?;
    Ok(Some(spec))
}

// ---------------------------------------------------------------------------
// philis gds2svg
// ---------------------------------------------------------------------------

fn cmd_gds2svg(gds: PathBuf, pdk: Option<PathBuf>, out: Option<PathBuf>) -> ExitCode {
    let pdk_path = pdk.unwrap_or_else(|| PathBuf::from("pdks/sky130.json"));
    let pdk_json = std::fs::read_to_string(&pdk_path).unwrap_or_default();
    let layer_names = pnr_visualizer::parse_layer_names(&pdk_json);

    let data = match std::fs::read(&gds) {
        Ok(d) => d,
        Err(e) => return fail(format!("read {}: {e}", gds.display())),
    };

    let svg = pnr_visualizer::export_svg(&data, &layer_names);

    let out_path = out.unwrap_or_else(|| gds.with_extension("svg"));
    if let Err(e) = std::fs::write(&out_path, &svg) {
        return fail(format!("write {}: {e}", out_path.display()));
    }
    println!("{} ({} KB)", out_path.display(), svg.len() / 1024);
    ExitCode::SUCCESS
}

// ---------------------------------------------------------------------------
// output helpers
// ---------------------------------------------------------------------------

fn print_verbose(r: &pnr_core::orchestrator::FlowResult) {
    let s = &r.signoff;

    let drc = &s.drc.violations;
    if !drc.is_empty() {
        eprintln!("--- DRC violations ({}) ---", drc.len());
        for v in drc {
            eprintln!("  {v:?}");
        }
    }

    eprintln!(
        "--- LVS: {} ---",
        if s.lvs.matched { "MATCH" } else { &s.lvs.reason }
    );
    eprintln!(
        "  extracted={} nmos={} pmos={} ambiguous={}",
        s.lvs.extracted_devices, s.lvs.nmos, s.lvs.pmos, s.lvs.ambiguous_classes,
    );
    if !s.lvs.mismatches.is_empty() {
        for m in &s.lvs.mismatches {
            eprintln!("  mismatch: {m:?}");
        }
    }

    if !s.erc_violations.is_empty() {
        eprintln!("--- ERC violations ({}) ---", s.erc_violations.len());
        for v in &s.erc_violations {
            eprintln!("  {v:?}");
        }
    }

    let rr = &r.routing.report;
    eprintln!(
        "--- Routing: {} wl_nm, {} vias, {} unrouted, {} overuse ---",
        rr.wirelength_nm, rr.via_count, rr.unrouted.len(), rr.overuse,
    );
}

fn fail(msg: String) -> ExitCode {
    eprintln!("error: {msg}");
    ExitCode::FAILURE
}
