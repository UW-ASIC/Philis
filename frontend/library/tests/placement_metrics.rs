//! PLC-01: the placement numbers the flow reports exist, and both gp modes are
//! seed-deterministic (the counters never steer a decision).

use library::{Config, GpMode, Solution};

fn run_ota(cfg: &Config) -> Solution {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let deck = std::fs::read_to_string(format!("{root}/pdks/sky130.json")).expect("sky130 deck present");
    let pdk = verify::Pdk::from_json(&deck).expect("deck parses");
    let spice = std::fs::read_to_string(format!("{root}/benchmarks/fixtures/ota.spice")).expect("ota fixture present");
    library::run(&spice, &pdk, &library::Macros::default(), cfg).expect("ota places")
}

fn small(gp_mode: GpMode) -> Config {
    Config { starts: 1, feedback_iters: 2, outer_iters: 1, gp_mode, ..Default::default() }
}

#[test]
fn metrics_are_populated_on_ota() {
    let stats = run_ota(&small(GpMode::Analytic)).stats;
    eprintln!("{:?}\n{:?}", stats.place, stats.dp);
    assert!(stats.dp.proposals > 0, "dp counted no proposals: {:?}", stats.dp);
    assert!(stats.place.area_usage >= 1.0, "footprint below Σ cell area: {:?}", stats.place);
    // Existence, not legality: AP-14's legalizer can leave overlap today (PLC-09 asserts 0).
    assert!(
        stats.place.overlap_nm2.is_finite() && stats.place.overlap_nm2 >= 0.0,
        "overlap not a measurement: {:?}",
        stats.place
    );
}

#[test]
fn gp_modes_are_deterministic() {
    for mode in [GpMode::Analytic, GpMode::Pile] {
        let (a, b) = (run_ota(&small(mode)).layout, run_ota(&small(mode)).layout);
        assert_eq!(a.x, b.x, "{mode:?}: x differs between identical runs");
        assert_eq!(a.y, b.y, "{mode:?}: y differs between identical runs");
        assert_eq!(a.variant, b.variant, "{mode:?}: variant differs between identical runs");
    }
}
