//! `philis` — the CLI. Thin: read the netlist + PDK, call `library`, sign off.
//! All logic lives in `library`; this only does I/O and argument handling.
//!
//! ```text
//! philis <netlist.sp> <deck.json>              # solve + signoff
//! philis emit <netlist.sp> <deck.json> <out.rs> # solve + decompile to generator source
//! ```

use std::process::ExitCode;

use library::{run, signoff, Config};
use verify::Pdk;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    // `philis emit a.sp deck.json out.rs` — strip the subcommand, remember it.
    let (emit_out, args): (Option<String>, Vec<String>) =
        if args.len() >= 2 && args[1] == "emit" {
            if args.len() < 5 {
                eprintln!("usage: philis emit <netlist.sp> <deck.json> <out.rs>");
                return ExitCode::FAILURE;
            }
            (Some(args[4].clone()), {
                let mut a = args.clone();
                a.remove(1);
                a
            })
        } else {
            (None, args)
        };
    if args.len() < 3 {
        eprintln!("usage: philis [emit] <netlist.sp> <deck.json> [out.rs]");
        return ExitCode::FAILURE;
    }

    let spice = match std::fs::read_to_string(&args[1]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("read {}: {e}", args[1]);
            return ExitCode::FAILURE;
        }
    };
    let deck = match std::fs::read_to_string(&args[2]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("read {}: {e}", args[2]);
            return ExitCode::FAILURE;
        }
    };
    let pdk = match Pdk::from_json(&deck) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("pdk: {e}");
            return ExitCode::FAILURE;
        }
    };

    let cfg = Config::default();
    // The CLI does a fully auto-generated run — no user-injected macros.
    let sol = match run(&spice, &pdk, &library::Macros::default(), &cfg) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("flow: {e:?}");
            return ExitCode::FAILURE;
        }
    };

    if let Some(out) = emit_out {
        // Decompile the solved layout into PDK-agnostic generator source.
        let ir = match library::emit::emit_solution(&sol, &pdk, &cfg) {
            Ok(ir) => ir,
            Err(e) => {
                eprintln!("emit: {e:?}");
                return ExitCode::FAILURE;
            }
        };
        if let Err(e) = std::fs::write(&out, library::emit::to_rust(&ir)) {
            eprintln!("write {out}: {e}");
            return ExitCode::FAILURE;
        }
        println!("emitted generator → {out}");
    }

    let report = signoff(&sol, &pdk);
    if report.hard_violations.is_empty() {
        println!("signoff CLEAN — cost {:.3}", report.cost);
        ExitCode::SUCCESS
    } else {
        println!("signoff: {} hard violation(s)", report.hard_violations.len());
        ExitCode::FAILURE
    }
}
