//! `philis` — read a netlist and a PDK deck, run the flow, sign off.
//!
//! ```text
//! philis <netlist.sp> <deck.json>                     # solve + signoff
//! philis run <netlist.sp> <deck.json> [-o DIR] [--seed N] [--max-iters N] [--starts N]
//! philis emit <netlist.sp> <deck.json> <out.rs>       # also decompile to generator source
//! philis --version
//! ```
//!
//! `run -o DIR` writes `<top>.gds` (the `.subckt` ports as labels on the
//! deck's text layers), `<top>_ref.spice` (the LVS reference signoff used,
//! dummies included), `signoff.txt` and `signoff.json`. `<top>` is the
//! netlist's `.subckt` name, else the file stem. The exit code is 0 only
//! when signoff is clean.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use library::{Config, Macros};

fn main() -> ExitCode {
    match cli() {
        Ok(clean) => ExitCode::from(u8::from(!clean)),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

const USAGE: &str = "usage: philis [run|emit] <netlist.sp> <deck.json> [out.rs] \
                     [-o DIR] [--seed N] [--max-iters N] [--starts N]";

/// `Ok(true)` when signoff is clean.
fn cli() -> Result<bool, String> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("philis {} ({})", env!("CARGO_PKG_VERSION"), env!("PHILIS_GIT_REV"));
        return Ok(true);
    }
    let mut cfg = Config::default();
    let mut out: Option<PathBuf> = None;
    // Flags, anywhere after the subcommand; what is left is positional.
    let mut pos = Vec::new();
    let mut it = args.drain(..);
    while let Some(a) = it.next() {
        let mut val = || it.next().ok_or_else(|| format!("{a} needs a value\n{USAGE}"));
        match a.as_str() {
            "-o" | "--out" => out = Some(PathBuf::from(val()?)),
            "--seed" => cfg.seed = num(&a, &val()?)?,
            "--max-iters" => cfg.feedback_iters = num(&a, &val()?)?,
            "--starts" => cfg.starts = num(&a, &val()?)?,
            // ponytail: no fixed die or boundary pins in this flow; the run
            // places freely and labels nets at their drawn terminals.
            "--interface" => {
                let f = val()?;
                eprintln!("warning: --interface {f} ignored: fixed die and boundary pins are not supported");
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
    let emit_to = match sub.as_deref() {
        Some("emit") => Some(pos.get(2).cloned().ok_or(USAGE)?),
        _ => None,
    };
    let [netlist, deck, ..] = pos.as_slice() else {
        return Err(USAGE.into());
    };
    let read = |p: &str| std::fs::read_to_string(p).map_err(|e| format!("read {p}: {e}"));
    let spice = read(netlist)?;
    let pdk = verify::Pdk::from_json(&read(deck)?).map_err(|e| format!("pdk: {e}"))?;

    let sol =
        library::run(&spice, &pdk, &Macros::default(), &cfg).map_err(|e| format!("flow: {e:?}"))?;

    if let Some(out) = emit_to {
        let ir = library::emit::emit(&sol.netlist, &sol.layout, &pdk, &cfg)
            .map_err(|e| format!("emit: {e:?}"))?;
        std::fs::write(&out, library::emit::to_rust(&ir))
            .map_err(|e| format!("write {out}: {e}"))?;
        println!("emitted generator → {out}");
    }

    let report = library::signoff(&sol, &pdk);
    let clean = report.hard_violations.is_empty();
    let summary = if clean {
        format!("signoff CLEAN — cost {:.3}", report.cost)
    } else {
        format!("signoff: {} hard violation(s)", report.hard_violations.len())
    };
    println!("{summary}");

    if let Some(dir) = out {
        let (top, ports) = interface(&spice, Path::new(netlist));
        write_outputs(&dir, &top, &ports, &sol, &pdk, &report, &summary)?;
        println!("wrote {}/{{{top}.gds, {top}_ref.spice, signoff.txt, signoff.json}}", dir.display());
    }
    Ok(clean)
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
) -> Result<(), String> {
    let write = |name: String, bytes: &[u8]| {
        let p = dir.join(name);
        std::fs::write(&p, bytes).map_err(|e| format!("write {}: {e}", p.display()))
    };
    std::fs::create_dir_all(dir).map_err(|e| format!("mkdir {}: {e}", dir.display()))?;
    write(format!("{top}.gds"), &library::export_gds(sol, pdk, top, ports))?;
    write(format!("{top}_ref.spice"), library::reference_spice(sol, pdk, top, ports).as_bytes())?;

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
        report.hard_violations.is_empty(),
        json_num(f64::from(report.cost)),
        arr(&report.hard_violations),
        arr(&report.budget_violations),
    );
    write("signoff.json".into(), json.as_bytes())
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
