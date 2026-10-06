//! FLOW-11: bottom-up hierarchy on `two_ota` (two instances of one 5T OTA).
//! Full flows, so `#[ignore]`d: run with `--release -- --ignored`.

use library::{Config, Hierarchy, Solution};

fn run(cfg: &Config) -> (Solution, verify::Pdk) {
    let pdk = verify::Pdk::builtin("sky130").expect("sky130 deck");
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let spice = std::fs::read_to_string(format!("{root}/benchmarks/fixtures/two_ota.spice")).expect("fixture present");
    let sol = library::run(&spice, &pdk, &library::Macros::default(), cfg).unwrap_or_else(|e| panic!("flow failed: {e:?}"));
    (sol, pdk)
}

fn cfg(hierarchy: Hierarchy) -> Config {
    // One feedback iteration, as `signoff_fixtures`: a correctness check.
    Config { feedback_iters: 1, top: Some("two_ota".into()), hierarchy, ..Default::default() }
}

#[test]
#[ignore = "full flow: --release -- --ignored"]
fn bottom_up_solves_each_definition_once_and_signs_off_clean() {
    let (sol, pdk) = run(&cfg(Hierarchy::BottomUp { min_devices: 5 }));
    assert_eq!(sol.stats.block_solves, 1);
    assert_eq!(sol.blocks.iter().map(|b| b.0.as_str()).collect::<Vec<_>>(), ["ota"]);
    assert_eq!(sol.devices_of.iter().filter(|m| m.len() == 5).count(), 2, "two block cells: {:?}", sol.devices_of);
    let s = library::signoff(&sol, &pdk);
    let bad: Vec<&String> = s.report.hard_violations.iter().map(|v| &v.rule).filter(|r| r.starts_with("lvs/") || r.starts_with("drc/")).collect();
    assert!(bad.is_empty(), "signoff rows: {bad:#?}");
}

#[test]
#[ignore = "full flow: --release -- --ignored"]
fn flat_and_bottom_up_both_run() {
    for h in [Hierarchy::Flat, Hierarchy::BottomUp { min_devices: 5 }] {
        let t = std::time::Instant::now();
        let (sol, _) = run(&cfg(h));
        let s = sol.stats;
        println!(
            "{h:?}: place_hard {} route_hard {} drc_hard {} route_overuse {} c_tier {:.3} wall {:.1}s",
            s.place_hard,
            s.route_hard,
            s.drc_hard,
            s.route_overuse,
            s.c_tier,
            t.elapsed().as_secs_f64()
        );
    }
}
