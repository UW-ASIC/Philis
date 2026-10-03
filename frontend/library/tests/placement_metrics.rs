//! PLC-01: the placement numbers the flow reports exist, and both gp modes are
//! seed-deterministic (the counters never steer a decision).

use library::{Config, GpMode, Solution};

fn run_fixture(name: &str, cfg: &Config) -> Solution {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let deck = std::fs::read_to_string(format!("{root}/pdks/sky130.json")).expect("sky130 deck present");
    let pdk = verify::Pdk::from_json(&deck).expect("deck parses");
    let spice = std::fs::read_to_string(format!("{root}/benchmarks/fixtures/{name}.spice")).expect("fixture present");
    library::run(&spice, &pdk, &library::Macros::default(), cfg).expect("fixture places")
}

fn run_ota(cfg: &Config) -> Solution {
    run_fixture("ota", cfg)
}

fn small(gp_mode: GpMode) -> Config {
    Config { starts: 1, feedback_iters: 2, outer_iters: 1, gp_mode, ..Default::default() }
}

#[test]
fn metrics_are_populated_on_ota() {
    let sol = run_ota(&small(GpMode::Analytic));
    let stats = sol.stats;
    eprintln!("{:?}\n{:?}", stats.place, stats.dp);
    assert!(stats.dp.proposals > 0, "dp counted no proposals: {:?}", stats.dp);
    assert!(stats.place.area_usage >= 1.0, "footprint below Σ cell area: {:?}", stats.place);
    // Measured on the returned layout, not legality: AP-14's legalizer can
    // leave overlap today (PLC-09 asserts 0).
    assert_eq!(stats.place.overlap_nm2, gp::mechanics::encroachment(&sol.layout, 0), "{:?}", stats.place);
}

/// Also pins the Config -> Flow -> `gp::place` plumbing: with it broken, Pile
/// silently runs Analytic and the two final layouts coincide.
#[test]
fn gp_modes_are_deterministic() {
    let [analytic, pile] = [GpMode::Analytic, GpMode::Pile].map(|mode| {
        let (a, b) = (run_ota(&small(mode)).layout, run_ota(&small(mode)).layout);
        assert_eq!(a.x, b.x, "{mode:?}: x differs between identical runs");
        assert_eq!(a.y, b.y, "{mode:?}: y differs between identical runs");
        assert_eq!(a.variant, b.variant, "{mode:?}: variant differs between identical runs");
        a
    });
    assert!(
        (&analytic.x, &analytic.y) != (&pile.x, &pile.y),
        "Pile placed ota exactly like Analytic: gp_mode not reaching gp::place"
    );
}

/// PLC-02: every cell's placed origin lands on the cut lattice, not just a
/// multiple of the grid.
#[test]
fn every_origin_is_on_the_cut_lattice() {
    for name in ["ota", "dac4", "rc_filter", "chain4", "quad"] {
        for seed in 1..=3u64 {
            let cfg = Config { starts: 1, feedback_iters: 2, outer_iters: 1, seed, ..Default::default() };
            let sol = run_fixture(name, &cfg);
            assert_eq!(sol.stats.place.lattice_off, 0, "{name} seed {seed}: {:?}", sol.stats.place);
        }
    }
}
