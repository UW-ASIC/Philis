//! Routing-quality acceptance on the checked-in fixtures (RTE-15…RTE-20):
//! what `dr` reports and what the routed geometry measures, over the same
//! one-iteration flow as `signoff_fixtures::check`. Full flows: run with
//! `cargo test --release -p benchmark --test routing_quality -- --include-ignored`.

use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn pdk() -> verify::Pdk {
    let text = std::fs::read_to_string(root().join("pdks/sky130.json")).expect("read sky130 deck");
    verify::Pdk::from_json(&text).expect("sky130 deck is complete")
}

/// The fixture `name` through the flow at one feedback iteration.
fn run(name: &str, pdk: &verify::Pdk) -> library::Solution {
    let spice = std::fs::read_to_string(root().join(format!("benchmarks/fixtures/{name}.spice"))).unwrap_or_else(|e| panic!("{name}: read fixture: {e}"));
    let cfg = library::Config { feedback_iters: 1, ..Default::default() };
    library::run(&spice, pdk, &library::Macros::default(), &cfg).unwrap_or_else(|e| panic!("{name}: flow failed: {e:?}"))
}

/// dr's hard rows starting with `prefix`.
fn rows<'a>(sol: &'a library::Solution, prefix: &str) -> Vec<&'a str> {
    sol.route.hard_violations.iter().filter(|v| v.rule.starts_with(prefix)).map(|v| v.rule.as_str()).collect()
}

/// RTE-16: no routed metal over a matched gate, and no open net.
#[test]
#[ignore = "full flow on the OTA-class fixtures; run with --include-ignored"]
fn no_metal_over_gates() {
    let pdk = pdk();
    for name in ["ota", "tt_ota"] {
        let sol = run(name, &pdk);
        println!("{name}: metal over gate {:?}, open {:?}", rows(&sol, "metal over gate"), rows(&sol, "open net"));
        assert!(rows(&sol, "metal over gate").is_empty(), "{name}: {:?}", rows(&sol, "metal over gate"));
        assert!(rows(&sol, "open net").is_empty(), "{name}: {:?}", rows(&sol, "open net"));
    }
}

#[test]
#[ignore]
fn dbg_quad() {
    let pdk = pdk();
    let sol = run("quad", &pdk);
    let name = |l: pnr_core::LayerId| pdk.layers.iter().find(|(_, x)| *x == l).map(|(n, _)| n.clone()).unwrap_or_default();
    for (n, w) in sol.routes.wires.iter().enumerate() { for s in w { let r = s.rect; if r.x <= 7300 && r.x + r.w >= 6400 && r.y <= -1300 && r.y + r.h >= -2200 { println!("net {n} {} {:?}", name(s.layer), r); } } }
    for m in &sol.macros { for s in &m.shapes { let r = s.rect; if (name(s.layer) == "met1" || name(s.layer)=="via") && r.x <= 7300 && r.x + r.w >= 6400 && r.y <= -1300 && r.y + r.h >= -2200 { println!("cell {} {:?}", name(s.layer), r); } } }
    println!("{:?}", sol.route.hard_violations.iter().map(|v| &v.rule).collect::<Vec<_>>());
}
