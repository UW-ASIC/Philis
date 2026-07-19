//! Philis CLI: SPICE netlist + PDK deck in, placed/routed/verified GDS out.
//!
//!   philis <netlist.spice> <pdk.json> [out_dir]
//!
//! Thin driver over [`pnr_core::orchestrator::run_flow`]. All physical-design
//! work lives in the backend; this only reads files, sets the output dir (which
//! is what makes the flow emit `<top>.gds`), and prints the signoff summary.

use std::path::PathBuf;
use std::process::ExitCode;

use pnr_core::backend::ConstraintRecord;
use pnr_core::orchestrator::{run_flow, FlowConfig};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let (Some(spice_path), Some(pdk_path)) = (args.next(), args.next()) else {
        eprintln!("usage: philis <netlist.spice> <pdk.json> [out_dir]");
        return ExitCode::FAILURE;
    };
    let out_dir = PathBuf::from(args.next().unwrap_or_else(|| "out".into()));

    let spice = match std::fs::read_to_string(&spice_path) {
        Ok(s) => s,
        Err(e) => return fail(format!("read {spice_path}: {e}")),
    };
    let deck = match std::fs::read_to_string(&pdk_path) {
        Ok(s) => s,
        Err(e) => return fail(format!("read {pdk_path}: {e}")),
    };
    if let Err(e) = std::fs::create_dir_all(&out_dir) {
        return fail(format!("mkdir {}: {e}", out_dir.display()));
    }

    // debug_dir is the switch that makes the flow write <top>.gds + signoff.
    let config = FlowConfig {
        debug_dir: Some(out_dir),
        ..FlowConfig::default()
    };

    match run_flow(&spice, &deck, &ConstraintRecord::default(), &config) {
        Ok(r) => {
            let s = &r.signoff;
            println!(
                "drc: {} blocking ({} waived) | lvs: {} | unrouted: {} | gds: {}",
                s.drc_blocking.len(),
                s.drc_waived_density,
                if s.lvs.matched { "match" } else { &s.lvs.reason },
                r.routing.report.unrouted.len(),
                r.gds_path.as_ref().map_or("<none>".into(), |p| p.display().to_string()),
            );
            let clean = s.drc_blocking.is_empty() && s.lvs.matched && r.routing.report.unrouted.is_empty();
            if clean { ExitCode::SUCCESS } else { ExitCode::FAILURE }
        }
        Err(e) => fail(e),
    }
}

fn fail(msg: String) -> ExitCode {
    eprintln!("error: {msg}");
    ExitCode::FAILURE
}
