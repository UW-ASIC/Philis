//! `philis` — read a netlist and a PDK deck, run the flow, sign off.
//!
//! ```text
//! philis <netlist.sp> <deck.json>               # solve + signoff
//! philis emit <netlist.sp> <deck.json> <out.rs> # also decompile to generator source
//! ```

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
        Some(args.get(2).cloned().ok_or("usage: philis emit <netlist.sp> <deck.json> <out.rs>")?)
    } else {
        None
    };
    let [netlist, deck, ..] = args.as_slice() else {
        return Err("usage: philis [emit] <netlist.sp> <deck.json> [out.rs]".into());
    };
    let read = |p: &str| std::fs::read_to_string(p).map_err(|e| format!("read {p}: {e}"));
    let spice = read(netlist)?;
    let pdk = verify::Pdk::from_json(&read(deck)?).map_err(|e| format!("pdk: {e}"))?;

    let cfg = Config::default();
    let sol = library::run(&spice, &pdk, &Macros::default(), &cfg).map_err(|e| format!("flow: {e:?}"))?;

    if let Some(out) = emit_to {
        let ir = library::emit::emit(&sol.netlist, &sol.layout, &pdk, &cfg)
            .map_err(|e| format!("emit: {e:?}"))?;
        std::fs::write(&out, library::emit::to_rust(&ir)).map_err(|e| format!("write {out}: {e}"))?;
        println!("emitted generator → {out}");
    }

    let report = library::signoff(&sol, &pdk);
    if report.hard_violations.is_empty() {
        println!("signoff CLEAN — cost {:.3}", report.cost);
    } else {
        println!("signoff: {} hard violation(s)", report.hard_violations.len());
    }
    Ok(report.hard_violations.is_empty())
}
