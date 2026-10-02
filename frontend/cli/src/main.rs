//! `philis` — read a netlist and a PDK deck, run the flow, sign off.
//!
//! ```text
//! philis <netlist.sp> <pdk>               # solve + signoff
//! philis emit <netlist.sp> <pdk> <out.rs> # also decompile to generator source
//! ```
//!
//! `<pdk>` is a sidecar `*.json`, or the name of one compiled in
//! (`sky130`, `gf180mcu`, `ihp_sg13g2`, `generic_finfet`).

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

/// `Ok(true)` when signoff is clean.
fn cli() -> Result<bool, String> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let emit_to = if args.first().is_some_and(|a| a == "emit") {
        args.remove(0);
        Some(
            args.get(2)
                .cloned()
                .ok_or("usage: philis emit <netlist.sp> <pdk> <out.rs>")?,
        )
    } else {
        None
    };
    let [netlist, deck, ..] = args.as_slice() else {
        return Err("usage: philis [emit] <netlist.sp> <pdk.json | sky130 | gf180mcu | ihp_sg13g2 | generic_finfet> [out.rs]".into());
    };
    let read = |p: &str| std::fs::read_to_string(p).map_err(|e| format!("read {p}: {e}"));
    let spice = read(netlist)?;
    let pdk = if std::path::Path::new(deck).is_file() { verify::Pdk::from_json(&read(deck)?) } else { verify::Pdk::builtin(deck) }
        .map_err(|e| format!("pdk: {e}"))?;

    let cfg = Config::default();
    let sol =
        library::run(&spice, &pdk, &Macros::default(), &cfg).map_err(|e| format!("flow: {e:?}"))?;

    if let Some(out) = emit_to {
        let ir = library::emit::emit(&sol.netlist, &sol.layout, &pdk, &cfg)
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
    if report.hard_violations.is_empty() {
        println!("signoff CLEAN — cost {:.3}", report.cost);
    } else {
        println!(
            "signoff: {} hard violation(s)",
            report.hard_violations.len()
        );
    }
    Ok(report.hard_violations.is_empty())
}
