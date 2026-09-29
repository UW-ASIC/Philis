//! Solve the operating point with ngspice, place-and-route against it, print
//! the constraint-budget report and the signoff breakdown.
use std::collections::BTreeMap;
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let spice = std::fs::read_to_string(root.join("benchmarks/fixtures/ota.spice")).unwrap();
    let pdk_json = std::fs::read_to_string(root.join("pdks/sky130.json")).unwrap();
    let pdk = verify::Pdk::from_json(&pdk_json).expect("pdk");

    // PDK_ROOT/PDK come from the dev shell; fall back to the in-repo .pdk.
    let pdk_root = std::env::var("PDK_ROOT").map_or_else(|_| root.join("../.pdk"), PathBuf::from);
    let pdk_name = std::env::var("PDK").unwrap_or_else(|_| "sky130A".into());
    let models = pdk_root
        .join(&pdk_name)
        .join("libs.tech/ngspice/sky130.lib.spice");

    let sim = library::oppoint::OpConfig {
        model_lib: models.exists().then_some(models),
        ..Default::default()
    };
    // Post-layout specs: the extracted capacitance on vout1/vout2 loads the
    // output pole, so UGF is what a careless layout loses first. The load
    // bias is set where the open-loop outputs balance (no CMFB in this OTA).
    let performance = library::perf::PerfConfig {
        sim: sim.clone(),
        testbench: "\
Vdd vdd 0 1.8
Vbn vbn 0 0.5
Vbias vbias 0 0.8515
Vip vinp 0 0.9 ac 0.5
Vim vinm 0 0.9 ac -0.5
.ac dec 20 1k 10g
.measure ac gain_db find vdb(vout2) at=1e3
.measure ac ugf when vdb(vout2)=0
.control
set ngbehavior=hsa
run
.endc"
            .into(),
        specs: vec![
            library::perf::Spec { metric: "gain_db".into(), min: Some(20.0), max: None },
            library::perf::Spec { metric: "ugf".into(), min: Some(300e6), max: None },
        ],
    };
    let cfg = library::Config {
        feedback_iters: 3,
        op: Some(sim),
        performance: Some(performance),
        ..Default::default()
    };
    let sol = library::run(&spice, &pdk, &library::Macros::default(), &cfg).expect("flow");
    println!("\n{}", sol.metadata);

    let rep = library::signoff(&sol, &pdk);
    let mut by_rule: BTreeMap<&str, usize> = BTreeMap::new();
    for v in &rep.hard_violations {
        *by_rule
            .entry(v.rule.split(':').next().unwrap_or(&v.rule))
            .or_default() += 1;
    }
    println!("\n  signoff: {} violations", rep.hard_violations.len());
    for (rule, n) in &by_rule {
        println!("    {rule:<40} {n}");
    }
}
