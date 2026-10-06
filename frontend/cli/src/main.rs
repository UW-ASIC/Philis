//! `philis` — read a netlist and a PDK deck, run the flow, sign off.
//!
//! ```text
//! philis [run] <netlist.sp> [<pdk>] [flags]           # solve + signoff, write outputs
//! philis emit <netlist.sp> [<pdk>] [<out.rs>] [flags]  # also decompile to generator source
//! philis --version
//! ```
//!
//! `<pdk>` (or `--pdk`) is a sidecar `*.json`, or the name of one compiled in
//! (`sky130`, `gf180mcu`, `ihp_sg13g2`, `generic_finfet`).
//!
//! Flags: `-o/--out DIR` (default `./philis_out/<netlist stem>/`), `--seed N`,
//! `--iters/--max-iters N` (epochs per assignment), `--outer N` (assignments),
//! `--starts N`, `--size spice|per-finger`, `--top NAME`, `--op-lib PATH
//! [--corner C] [--vdd V] [--temp C] [--testbench FILE]` (operating point),
//! `--perf SPECS.json` (`{"testbench": "tb.spice", "specs": [{"metric", "min",
//! "max"}]}`, the testbench relative to the JSON), `--interface FILE`
//! (die and boundary pins, checked against the ports), `--out-rs FILE` (emit),
//! `--constraints FILE` (ALIGN-style JSON sidecar, `annotator::sidecar`).
//! `--max-wall SECONDS` (wall budget: the search stops before its next epoch
//! once spent, keeping its incumbent). `--hierarchy flat|auto|bottom-up:N`
//! (FLOW-11: each sub-circuit definition with ≥ N devices solved once as a
//! block; `auto` = `bottom-up:2` past 100 devices, else flat).
//!
//! Every run writes `<top>.gds` (the `.subckt` ports as labels on the
//! deck's text layers), `<top>_ref.spice` (the LVS reference signoff used,
//! dummies included), `<top>_pex.spice` when extraction allows,
//! `signoff.txt`, `signoff.json` and `report.txt` (constraint budgets, the
//! signoff hard rows, the run's stats and one `kind\tmessage` line per
//! annotator diagnostic, unknown `--constraints` names included). `<top>` is the netlist's `.subckt`
//! name, else the file stem. Exit code: 0 signoff clean, 1 not clean, 2 error.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use library::{Config, Macros};
use pnr_core::report::{Report, Violation};

fn main() -> ExitCode {
    match cli() {
        Ok(clean) => ExitCode::from(u8::from(!clean)),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}

const USAGE: &str = "usage: philis [run|emit] <netlist.sp> [<pdk.json | sky130 | gf180mcu | ihp_sg13g2 | generic_finfet>] [out.rs] \
                     [--pdk P] [-o|--out DIR] [--seed N] [--iters N] [--outer N] [--starts N] [--size spice|per-finger] [--top NAME] \
                     [--op-lib PATH [--corner C] [--vdd V] [--temp C] [--testbench FILE]] [--perf SPECS.json] [--interface FILE] [--constraints FILE] [--hierarchy flat|auto|bottom-up:N] [--out-rs FILE]";

/// A parsed command line: everything [`cli`] needs before it reads the
/// netlist and the deck.
struct Args {
    /// Flow configuration, operating point (`--op-lib` and friends) folded in.
    cfg: Config,
    /// `-o/--out`; `None` = `philis_out/<netlist stem>/`.
    out: Option<PathBuf>,
    /// The netlist path (first positional after the subcommand).
    netlist: String,
    /// `--pdk`, else the second positional: a sidecar path or a built-in name.
    deck: String,
    /// `Some(path)` for `emit`: `--out-rs`, else the positional after the deck.
    emit_to: Option<String>,
    /// `--perf SPECS.json`, resolved by [`perf_config`] once the deck loads.
    perf: Option<String>,
}

/// Runs the whole command and returns `Ok(true)` when signoff is clean: no
/// hard violation and every device LVS-compared.
///
/// # Errors
/// A usage error, an unreadable input, a deck or flow failure, or an output
/// that cannot be written, as the message printed before exit code 2.
fn cli() -> Result<bool, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("philis {} ({})", env!("CARGO_PKG_VERSION"), env!("PHILIS_GIT_REV"));
        return Ok(true);
    }
    let Args { mut cfg, out, netlist, deck, emit_to, perf } = parse_args(args)?;
    let spice = read(&netlist)?;
    let pdk = if Path::new(&deck).is_file() { verify::Pdk::from_json(&read(&deck)?) } else { verify::Pdk::builtin(&deck) }
        .map_err(|e| format!("pdk: {e}"))?;
    if let Some(path) = perf {
        cfg.performance = Some(perf_config(&path, cfg.op.clone().unwrap_or_default())?);
    }

    let sol = library::run(&spice, &pdk, &Macros::default(), &cfg).map_err(|e| format!("flow: {e:?}"))?;

    if let Some(out) = emit_to {
        let ir = library::emit::emit_solution(&sol, &pdk).map_err(|e| format!("emit: {e:?}"))?;
        std::fs::write(&out, library::emit::to_rust(&ir)).map_err(|e| format!("write {out}: {e}"))?;
        println!("emitted generator → {out}");
    }

    if !sol.metadata.assumed.is_empty() {
        println!("assumed (UNVERIFIED sidecar values): {}", sol.metadata.assumed.join(", "));
    }
    let signoff = library::signoff(&sol, &pdk);
    if !signoff.warnings.is_empty() {
        println!("signoff: {} deck warning(s), not violations", signoff.warnings.len());
    }
    print!("{}", signoff.coverage);
    // Unknown never passes: an LVS-unverified device keeps it from CLEAN.
    let unverified: usize = signoff.coverage.unverified.iter().map(|u| u.2).sum();
    let (clean, summary) = verdict(&signoff.report, unverified);
    println!("{summary}");

    let path = Path::new(&netlist);
    let dir = out.unwrap_or_else(|| Path::new("philis_out").join(path.file_stem().unwrap_or_default()));
    let (top, ports) = interface(&spice, path);
    let pex = if write_outputs(&dir, &top, &ports, &sol, &pdk, &signoff.report, &summary, clean)? {
        format!(" {top}_pex.spice,")
    } else {
        String::new()
    };
    println!("wrote {}/{{{top}.gds, {top}_ref.spice,{pex} signoff.txt, signoff.json, report.txt}}", dir.display());
    Ok(clean)
}

/// Reads a whole text file.
///
/// # Errors
/// `read <path>: <io error>`.
fn read(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))
}

/// Parses the arguments after the program name (`--version` excluded).
/// Flags may sit anywhere; what is left is positional: an optional `run` or
/// `emit` subcommand, the netlist, the deck unless `--pdk` gave it, and for
/// `emit` the output path unless `--out-rs` gave it. `--testbench`,
/// `--interface` and `--constraints` read their file here.
///
/// # Errors
/// An unknown flag, a flag missing its value, an unparseable value, a
/// missing positional, or an unreadable flag file.
fn parse_args(args: Vec<String>) -> Result<Args, String> {
    let mut cfg = Config::default();
    let mut out: Option<PathBuf> = None;
    let mut pdk_arg: Option<String> = None;
    let mut out_rs: Option<String> = None;
    let mut perf: Option<String> = None;
    let mut op = library::oppoint::OpConfig::default();
    let mut op_set = false;
    let mut pos = Vec::new();
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        let mut val = || it.next().ok_or_else(|| format!("{a} needs a value\n{USAGE}"));
        match a.as_str() {
            "-o" | "--out" => out = Some(PathBuf::from(val()?)),
            "--pdk" => pdk_arg = Some(val()?),
            "--seed" => cfg.seed = num(&a, &val()?)?,
            "--max-iters" | "--iters" => cfg.feedback_iters = num(&a, &val()?)?,
            "--outer" => cfg.outer_iters = num(&a, &val()?)?,
            "--starts" => cfg.starts = num(&a, &val()?)?,
            "--size" => cfg.size_convention = size_convention(&val()?)?,
            "--top" => cfg.top = Some(val()?),
            "--op-lib" => (op.model_lib, op_set) = (Some(PathBuf::from(val()?)), true),
            "--corner" => (op.corner, op_set) = (val()?, true),
            "--vdd" => (op.vdd, op_set) = (num(&a, &val()?)?, true),
            "--temp" => (op.temp_c, op_set) = (num(&a, &val()?)?, true),
            "--testbench" => (op.testbench, op_set) = (Some(read(&val()?)?), true),
            "--perf" => perf = Some(val()?),
            "--interface" => cfg.interface = Some(library::Interface::from_json(&read(&val()?)?)?),
            "--out-rs" => out_rs = Some(val()?),
            "--constraints" => cfg.constraints = Some(read(&val()?).map_err(|e| format!("--constraints: {e}"))?),
            "--max-wall" => cfg.max_wall = Some(std::time::Duration::from_secs_f64(num(&a, &val()?)?)),
            "--hierarchy" => cfg.hierarchy = hierarchy(&val()?)?,
            _ if a.starts_with('-') => return Err(format!("unknown flag {a}\n{USAGE}")),
            _ => pos.push(a),
        }
    }
    if op_set {
        cfg.op = Some(op);
    }
    let emit = matches!(pos.first().map(String::as_str), Some("emit"));
    if matches!(pos.first().map(String::as_str), Some("run" | "emit")) {
        pos.remove(0);
    }
    let mut pos = pos.into_iter();
    let netlist = pos.next().ok_or(USAGE)?;
    let deck = match pdk_arg {
        Some(p) => p,
        None => pos.next().ok_or(USAGE)?,
    };
    let emit_to = if emit { Some(out_rs.or_else(|| pos.next()).ok_or(USAGE)?) } else { None };
    Ok(Args { cfg, out, netlist, deck, emit_to, perf })
}

/// `--size` value: `spice` or `per-finger`.
///
/// # Errors
/// Any other word.
fn size_convention(v: &str) -> Result<library::SizeConvention, String> {
    match v {
        "spice" => Ok(library::SizeConvention::Spice),
        "per-finger" => Ok(library::SizeConvention::PerFinger),
        v => Err(format!("--size: {v:?} is not spice|per-finger")),
    }
}

/// `--hierarchy` value: `flat`, `auto`, or `bottom-up:N` with `N` a device
/// count.
///
/// # Errors
/// Any other value, including `bottom-up:` with a non-integer `N`.
fn hierarchy(h: &str) -> Result<library::Hierarchy, String> {
    match h {
        "flat" => Ok(library::Hierarchy::Flat),
        "auto" => Ok(library::Hierarchy::Auto),
        _ => h
            .strip_prefix("bottom-up:")
            .and_then(|n| n.parse().ok())
            .map(|min_devices| library::Hierarchy::BottomUp { min_devices })
            .ok_or_else(|| format!("`--hierarchy {h}`: expected flat, auto or bottom-up:N")),
    }
}

/// Builds the `--perf SPECS.json` configuration: `{"testbench": "tb.spice",
/// "specs": [{"metric": "gain_db", "min": 40, "max": null}]}`, the testbench
/// path relative to the JSON's directory, simulated with `sim`. A missing or
/// `null` bound is no bound.
///
/// # Errors
/// An unreadable JSON or testbench, malformed JSON, `testbench` not a
/// string, `specs` not an array, a spec without a string `metric`, or a
/// bound that is neither a number nor `null`.
fn perf_config(path: &str, sim: library::oppoint::OpConfig) -> Result<library::perf::PerfConfig, String> {
    use serde_json::Value;
    let v: Value = serde_json::from_str(&read(path)?).map_err(|e| format!("{path}: {e}"))?;
    let tb = v.get("testbench").and_then(Value::as_str).ok_or_else(|| format!("{path}: `testbench` must be a path"))?;
    let tb = Path::new(path).parent().unwrap_or(Path::new("")).join(tb);
    let testbench = std::fs::read_to_string(&tb).map_err(|e| format!("read {}: {e}", tb.display()))?;
    let bound = |s: &Value, k: &str| match s.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(x) => x.as_f64().map(Some).ok_or_else(|| format!("{path}: spec `{k}` must be a number or null")),
    };
    let specs = v.get("specs").and_then(Value::as_array).ok_or_else(|| format!("{path}: `specs` must be an array"))?;
    let specs = specs
        .iter()
        .map(|s| {
            let metric = s.get("metric").and_then(Value::as_str).ok_or_else(|| format!("{path}: spec `metric` must be a string"))?;
            Ok(library::perf::Spec { metric: metric.to_string(), min: bound(s, "min")?, max: bound(s, "max")? })
        })
        .collect::<Result<_, String>>()?;
    Ok(library::perf::PerfConfig { sim, testbenches: vec![testbench], specs, scenarios: Vec::new() })
}

/// Parses a flag's numeric value.
///
/// # Errors
/// `<flag>: not a number: <v>`.
fn num<T: std::str::FromStr>(flag: &str, v: &str) -> Result<T, String> {
    v.parse().map_err(|_| format!("{flag}: not a number: {v}"))
}

/// The run's verdict: `(clean, summary line)`. Clean means no hard violation
/// and no LVS-unverified device (`unverified`, a device count).
fn verdict(report: &Report, unverified: usize) -> (bool, String) {
    let clean = report.hard_violations.is_empty() && unverified == 0;
    let summary = if clean {
        format!("signoff CLEAN — cost {:.3}", report.cost)
    } else {
        format!(
            "signoff: {} hard violation(s), {unverified} device(s) LVS-unverified — not clean",
            report.hard_violations.len()
        )
    };
    (clean, summary)
}

/// The first `.subckt`'s name and ports (`k=v` params dropped), else the
/// file stem (`top` without one) and no ports (every labelled net): the GDS
/// top cell, its pin labels and the reference header, so external LVS pairs
/// them by name.
fn interface(spice: &str, path: &Path) -> (String, Vec<String>) {
    let line = spice.lines().map(str::split_whitespace).find_map(|mut w| {
        w.next().filter(|k| k.eq_ignore_ascii_case(".subckt"))?;
        let name = w.next()?.to_string();
        Some((name, w.filter(|t| !t.contains('=')).map(str::to_string).collect()))
    });
    line.unwrap_or_else(|| {
        let stem = path.file_stem().map_or_else(|| "top".into(), |s| s.to_string_lossy().into_owned());
        (stem, Vec::new())
    })
}

/// Writes every output file into `dir` (created if missing) and returns
/// whether `<top>_pex.spice` was written. When extraction declines, a stale
/// `<top>_pex.spice` from an earlier run is removed so it cannot pass for
/// this layout's.
///
/// # Errors
/// `dir` cannot be created, the GDS cannot be exported, or a file cannot be
/// written.
#[allow(clippy::too_many_arguments)]
fn write_outputs(
    dir: &Path,
    top: &str,
    ports: &[String],
    sol: &library::Solution,
    pdk: &verify::Pdk,
    report: &Report,
    summary: &str,
    clean: bool,
) -> Result<bool, String> {
    let write = |name: String, bytes: &[u8]| {
        let p = dir.join(name);
        std::fs::write(&p, bytes).map_err(|e| format!("write {}: {e}", p.display()))
    };
    std::fs::create_dir_all(dir).map_err(|e| format!("mkdir {}: {e}", dir.display()))?;
    write(format!("{top}.gds"), &library::export_gds(sol, pdk, top, ports)?)?;
    write(format!("{top}_ref.spice"), library::reference_spice(sol, pdk, top, ports).as_bytes())?;
    let pex = match library::post_layout_spice(sol, pdk, top) {
        Ok(s) => write(format!("{top}_pex.spice"), s.as_bytes()).map(|()| true)?,
        Err(e) => {
            eprintln!("{top}_pex.spice not written: {e}");
            let _ = std::fs::remove_file(dir.join(format!("{top}_pex.spice")));
            false
        }
    };
    write("signoff.txt".into(), signoff_txt(report, summary).as_bytes())?;
    write("signoff.json".into(), signoff_json(report, clean).as_bytes())?;
    write("report.txt".into(), report_txt(sol, report).as_bytes())?;
    Ok(pex)
}

/// One `rule\tmargin` line per violation.
fn violation_lines(vs: &[Violation]) -> String {
    vs.iter().map(|v| format!("{}\t{}\n", v.rule, v.margin)).collect()
}

/// `signoff.txt`: the summary line, then the hard and budget rows.
fn signoff_txt(report: &Report, summary: &str) -> String {
    format!(
        "{summary}\n\n# hard ({})\n{}\n# budget ({})\n{}",
        report.hard_violations.len(),
        violation_lines(&report.hard_violations),
        report.budget_violations.len(),
        violation_lines(&report.budget_violations),
    )
}

/// `signoff.json`: `{"budget": [{"margin", "rule"}], "clean", "cost",
/// "hard": [...]}` plus a newline. A non-finite cost is `null` (JSON has no
/// NaN or infinity).
fn signoff_json(report: &Report, clean: bool) -> String {
    let rows = |vs: &[Violation]| -> Vec<serde_json::Value> {
        vs.iter().map(|v| serde_json::json!({ "rule": v.rule, "margin": v.margin })).collect()
    };
    let json = serde_json::json!({
        "clean": clean,
        "cost": report.cost,
        "hard": rows(&report.hard_violations),
        "budget": rows(&report.budget_violations),
    });
    format!("{json}\n")
}

/// `report.txt`: the run metadata, the signoff hard rows, the run's stats
/// and one `kind\tmessage` line per annotator diagnostic.
fn report_txt(sol: &library::Solution, report: &Report) -> String {
    let st = &sol.stats;
    let stages: String = library::STAGES.iter().zip(st.stage_ms).map(|(n, ms)| format!(" {n}={ms:.0}")).collect();
    format!(
        "{}\n# signoff hard ({})\n{}\n# run\nconverged\t{}\nstop\t{:?}\nwarm\t{}\niterations\t{}\nouter_iterations\t{}\nsim_failures\t{}\nwarnings\t{}\nstage_ms\t{}\n# diagnostics ({})\n{}",
        sol.metadata,
        report.hard_violations.len(),
        violation_lines(&report.hard_violations),
        st.converged,
        st.stop,
        st.warm_epochs,
        st.iterations,
        st.outer_iterations,
        st.sim_failures,
        st.warnings,
        stages.trim_start(),
        sol.diagnostics.len(),
        sol.diagnostics.iter().map(|d| format!("{}\t{}\n", d.kind, d.message)).collect::<String>(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interface_reads_the_subckt_header_and_falls_back_to_the_stem() {
        let sp = "* c\n.SUBCKT strongarm vinp vinn outp vdd vss W=1\nXM1 a b c d m\n.ends\n";
        assert_eq!(
            interface(sp, Path::new("x/whatever.spice")),
            ("strongarm".into(), vec!["vinp".into(), "vinn".into(), "outp".into(), "vdd".into(), "vss".into()])
        );
        assert_eq!(interface("M1 a b c d n\n", Path::new("x/flat.sp")), ("flat".into(), vec![]));
    }
}
