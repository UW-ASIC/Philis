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

    let cfg = library::Config {
        feedback_iters: 3,
        op: Some(library::oppoint::OpConfig {
            model_lib: models.exists().then_some(models),
            ..Default::default()
        }),
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
