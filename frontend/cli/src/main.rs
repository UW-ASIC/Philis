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
//! (die and boundary pins, checked against the ports), `--out-rs FILE` (emit).
//! `--constraints`, `--hierarchy` other than `flat` and `--max-wall` are
//! refused (exit 2) until EXT-26 / FLOW-11 / FLOW-08.
//!
//! Every run writes `<top>.gds` (the `.subckt` ports as labels on the
//! deck's text layers), `<top>_ref.spice` (the LVS reference signoff used,
//! dummies included), `<top>_pex.spice` when extraction allows,
//! `signoff.txt`, `signoff.json` and `report.txt` (constraint budgets, the
//! signoff hard rows and the run's stats). `<top>` is the netlist's `.subckt`
//! name, else the file stem. Exit code: 0 signoff clean, 1 not clean, 2 error.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use library::{Config, Macros};

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
                     [--op-lib PATH [--corner C] [--vdd V] [--temp C] [--testbench FILE]] [--perf SPECS.json] [--interface FILE] [--out-rs FILE]";

/// `Ok(true)` when signoff is clean: no errors and every device LVS-compared.
fn cli() -> Result<bool, String> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("philis {} ({})", env!("CARGO_PKG_VERSION"), env!("PHILIS_GIT_REV"));
        return Ok(true);
    }
    let read = |p: &str| std::fs::read_to_string(p).map_err(|e| format!("read {p}: {e}"));
    let mut cfg = Config::default();
    let mut out: Option<PathBuf> = None;
    let mut pdk_arg: Option<String> = None;
    let mut out_rs: Option<String> = None;
    let mut perf: Option<String> = None;
    let mut op = library::oppoint::OpConfig::default();
    let mut op_set = false;
    // Flags, anywhere after the subcommand; what is left is positional.
    let mut pos = Vec::new();
    let mut it = args.drain(..);
    while let Some(a) = it.next() {
        let mut val = || it.next().ok_or_else(|| format!("{a} needs a value\n{USAGE}"));
        match a.as_str() {
            "-o" | "--out" => out = Some(PathBuf::from(val()?)),
            "--pdk" => pdk_arg = Some(val()?),
            "--seed" => cfg.seed = num(&a, &val()?)?,
            "--max-iters" | "--iters" => cfg.feedback_iters = num(&a, &val()?)?,
            "--outer" => cfg.outer_iters = num(&a, &val()?)?,
            "--starts" => cfg.starts = num(&a, &val()?)?,
            "--size" => {
                cfg.size_convention = match val()?.as_str() {
                    "spice" => library::SizeConvention::Spice,
                    "per-finger" => library::SizeConvention::PerFinger,
                    v => return Err(format!("--size: {v:?} is not spice|per-finger")),
                }
            }
            "--top" => cfg.top = Some(val()?),
            "--op-lib" => (op.model_lib, op_set) = (Some(PathBuf::from(val()?)), true),
            "--corner" => (op.corner, op_set) = (val()?, true),
            "--vdd" => (op.vdd, op_set) = (num(&a, &val()?)?, true),
            "--temp" => (op.temp_c, op_set) = (num(&a, &val()?)?, true),
            "--testbench" => (op.testbench, op_set) = (Some(read(&val()?)?), true),
            "--perf" => perf = Some(val()?),
            "--interface" => cfg.interface = Some(library::Interface::from_json(&read(&val()?)?)?),
            "--out-rs" => out_rs = Some(val()?),
            "--constraints" => return Err("`--constraints` needs EXT-26 (constraint file format)".into()),
            "--max-wall" => return Err("`--max-wall` needs FLOW-08 (wall-clock stop)".into()),
            "--hierarchy" => {
                let h = val()?;
                if h != "flat" {
                    return Err(format!("`--hierarchy {h}` needs FLOW-11 (only `flat`)"));
                }
            }
            _ if a.starts_with('-') => return Err(format!("unknown flag {a}\n{USAGE}")),
            _ => pos.push(a),
        }
    }
    drop(it);
    let sub = match pos.first().map(String::as_str) {
        Some("run") | Some("emit") => Some(pos.remove(0)),
        _ => None,
    };
    let netlist = pos.first().cloned().ok_or(USAGE)?;
    // The PDK is `--pdk`, else the second positional.
    let rest = &pos[1..];
    let (deck, rest) = match pdk_arg {
        Some(p) => (p, rest),
        None => (rest.first().cloned().ok_or(USAGE)?, &rest[1..]),
    };
    let emit_to = match sub.as_deref() {
        Some("emit") => Some(out_rs.or_else(|| rest.first().cloned()).ok_or(USAGE)?),
        _ => None,
    };
    let spice = read(&netlist)?;
    let pdk = if Path::new(&deck).is_file() { verify::Pdk::from_json(&read(&deck)?) } else { verify::Pdk::builtin(&deck) }
        .map_err(|e| format!("pdk: {e}"))?;
    if op_set {
        cfg.op = Some(op);
    }
    if let Some(path) = perf {
        cfg.performance = Some(perf_config(&path, cfg.op.clone().unwrap_or_default())?);
    }

    let sol =
        library::run(&spice, &pdk, &Macros::default(), &cfg).map_err(|e| format!("flow: {e:?}"))?;

    if let Some(out) = emit_to {
        let ir = library::emit::emit_solution(&sol, &pdk)
            .map_err(|e| format!("emit: {e:?}"))?;
        std::fs::write(&out, library::emit::to_rust(&ir))
            .map_err(|e| format!("write {out}: {e}"))?;
        println!("emitted generator → {out}");
    }

    if !sol.metadata.assumed.is_empty() {
        println!("assumed (UNVERIFIED sidecar values): {}", sol.metadata.assumed.join(", "));
    }
    let signoff = library::signoff(&sol, &pdk);
    let report = signoff.report;
    if !signoff.warnings.is_empty() {
        println!("signoff: {} deck warning(s), not violations", signoff.warnings.len());
    }
    print!("{}", signoff.coverage);
    // Unknown never passes: an LVS-unverified device keeps it from CLEAN.
    let unverified: usize = signoff.coverage.unverified.iter().map(|u| u.2).sum();
    let clean = report.hard_violations.is_empty() && unverified == 0;
    let summary = if clean {
        format!("signoff CLEAN — cost {:.3}", report.cost)
    } else {
        format!(
            "signoff: {} hard violation(s), {unverified} device(s) LVS-unverified — not clean",
            report.hard_violations.len()
        )
    };
    println!("{summary}");

    let path = Path::new(&netlist);
    let dir = out.unwrap_or_else(|| Path::new("philis_out").join(path.file_stem().unwrap_or_default()));
    let (top, ports) = interface(&spice, path);
    let pex = if write_outputs(&dir, &top, &ports, &sol, &pdk, &report, &summary, clean)? { format!(" {top}_pex.spice,") } else { String::new() };
    println!("wrote {}/{{{top}.gds, {top}_ref.spice,{pex} signoff.txt, signoff.json, report.txt}}", dir.display());
    Ok(clean)
}

/// `--perf SPECS.json`: `{"testbench": "tb.spice", "specs": [{"metric": "gain_db",
/// "min": 40, "max": null}]}`, the testbench path relative to the JSON.
fn perf_config(path: &str, sim: library::oppoint::OpConfig) -> Result<library::perf::PerfConfig, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("read {path}: {e}"))?;
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
    let tb = v.get("testbench").and_then(serde_json::Value::as_str).ok_or_else(|| format!("{path}: `testbench` must be a path"))?;
    let tb = Path::new(path).parent().unwrap_or(Path::new("")).join(tb);
    let testbench = std::fs::read_to_string(&tb).map_err(|e| format!("read {}: {e}", tb.display()))?;
    let bound = |s: &serde_json::Value, k: &str| -> Result<Option<f64>, String> {
        match s.get(k) {
            None | Some(serde_json::Value::Null) => Ok(None),
            Some(x) => x.as_f64().map(Some).ok_or_else(|| format!("{path}: spec `{k}` must be a number or null")),
        }
    };
    let specs = v.get("specs").and_then(serde_json::Value::as_array).ok_or_else(|| format!("{path}: `specs` must be an array"))?;
    let specs = specs
        .iter()
        .map(|s| {
            let metric = s.get("metric").and_then(serde_json::Value::as_str).ok_or_else(|| format!("{path}: spec `metric` must be a string"))?;
            Ok(library::perf::Spec { metric: metric.to_string(), min: bound(s, "min")?, max: bound(s, "max")? })
        })
        .collect::<Result<_, String>>()?;
    Ok(library::perf::PerfConfig { sim, testbench, specs })
}

fn num<T: std::str::FromStr>(flag: &str, v: &str) -> Result<T, String> {
    v.parse().map_err(|_| format!("{flag}: not a number: {v}"))
}

/// The first `.subckt`'s name and ports (`k=v` params dropped), else the
/// file stem and no ports (every labelled net): the GDS top cell, its pin
/// labels and the reference header, so external LVS pairs them by name.
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

fn write_outputs(
    dir: &Path,
    top: &str,
    ports: &[String],
    sol: &library::Solution,
    pdk: &verify::Pdk,
    report: &pnr_core::report::Report,
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
    // A previous run's file in `dir` would read as this layout's extraction: removed when none is written.
    let pex = match library::post_layout_spice(sol, pdk, top) {
        Ok(s) => write(format!("{top}_pex.spice"), s.as_bytes()).map(|()| true)?,
        Err(e) => {
            eprintln!("{top}_pex.spice not written: {e}");
            let _ = std::fs::remove_file(dir.join(format!("{top}_pex.spice")));
            false
        }
    };

    let lines = |vs: &[pnr_core::report::Violation]| -> String {
        vs.iter().map(|v| format!("{}\t{}\n", v.rule, v.margin)).collect()
    };
    let txt = format!(
        "{summary}\n\n# hard ({})\n{}\n# budget ({})\n{}",
        report.hard_violations.len(),
        lines(&report.hard_violations),
        report.budget_violations.len(),
        lines(&report.budget_violations),
    );
    write("signoff.txt".into(), txt.as_bytes())?;

    let arr = |vs: &[pnr_core::report::Violation]| -> String {
        let items: Vec<String> = vs
            .iter()
            .map(|v| format!("{{\"rule\":{},\"margin\":{}}}", json_str(&v.rule), json_num(v.margin as f64)))
            .collect();
        format!("[{}]", items.join(","))
    };
    let json = format!(
        "{{\"clean\":{},\"cost\":{},\"hard\":{},\"budget\":{}}}\n",
        clean,
        json_num(f64::from(report.cost)),
        arr(&report.hard_violations),
        arr(&report.budget_violations),
    );
    write("signoff.json".into(), json.as_bytes())?;

    let st = &sol.stats;
    let stages: String = library::STAGES.iter().zip(st.stage_ms).map(|(n, ms)| format!(" {n}={ms:.0}")).collect();
    let rep = format!(
        "{}\n# signoff hard ({})\n{}\n# run\nconverged\t{}\niterations\t{}\nouter_iterations\t{}\nsim_failures\t{}\nwarnings\t{}\nstage_ms\t{}\n",
        sol.metadata,
        report.hard_violations.len(),
        lines(&report.hard_violations),
        st.converged,
        st.iterations,
        st.outer_iterations,
        st.sim_failures,
        st.warnings,
        stages.trim_start(),
    );
    write("report.txt".into(), rep.as_bytes())?;
    Ok(pex)
}

fn json_str(s: &str) -> String {
    let mut o = String::from('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// JSON has no NaN/inf: those become `null`.
fn json_num(v: f64) -> String {
    if v.is_finite() { format!("{v}") } else { "null".into() }
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
