use pnr_core::backend::ConstraintRecord;
use pnr_core::orchestrator::{run_flow, FlowConfig};
use std::path::PathBuf;

const MINV: &str = "\
.subckt minv in out mid VDD VSS
XM1 mid in VSS VSS nfet_01v8 W=0.42u L=0.15u
XM2 mid in VDD VDD pfet_01v8 W=0.42u L=0.15u
XR1 mid out res_generic_po W=0.33u L=0.1u
.ends minv
";

fn main() {
    let deck = std::fs::read_to_string(format!(
        "{}/../../pdks/sky130.json",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let dir = PathBuf::from("/tmp/pnr_label_demo");
    let _ = std::fs::create_dir_all(&dir);
    let cfg = FlowConfig {
        debug_dir: Some(dir.clone()),
        ..Default::default()
    };
    let r = run_flow(MINV, &deck, &ConstraintRecord::default(), &cfg).unwrap();
    println!("{r}");
    if let Some(p) = &r.gds_path {
        println!("GDS written to: {}", p.display());
    }
}
