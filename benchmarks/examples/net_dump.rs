//! Diagnostic: run a fixture through the flow, print signoff grouped by rule,
//! then the extraction net by net (devices, polygons per layer, bbox) and
//! which extracted net each routed net's wires land on — a routed net split
//! across two extracted nets is an open. Labels are not applied here, so every
//! net is anonymous and bulk-only rails show as device-less.
//!
//!     cargo run --release -p benchmark --example net_dump [fixture]
//!     NET=VDD cargo run ...   # also list that net's wires

use std::collections::{BTreeMap, HashMap};

use gdsverify::check::topology::NetId;
use library::{Config, Macros};
use verify::{Checker, Checks, Pdk};

fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let name = std::env::args().nth(1).unwrap_or_else(|| "ota".into());
    let text = std::fs::read_to_string(root.join(format!("benchmarks/fixtures/{name}.spice")))
        .expect("fixture");
    let pdk = Pdk::from_json(&std::fs::read_to_string(root.join("pdks/sky130.json")).unwrap())
        .expect("deck");
    let cfg = Config { seed: 1, feedback_iters: 5, ..Config::default() };
    let sol = library::run(&text, &pdk, &Macros::default(), &cfg).expect("flow");

    let mut by_rule: BTreeMap<String, usize> = BTreeMap::new();
    for v in &library::signoff(&sol, &pdk).hard_violations {
        *by_rule.entry(v.rule.clone()).or_default() += 1;
    }
    println!("=== {name} signoff ===");
    for (rule, n) in &by_rule {
        println!("  {n:6}  {rule}");
    }

    let shapes = sol.geometry();
    let mut c = Checker::new(&pdk, false).unwrap();
    c.run(&shapes, &[], Checks { drc: false, erc: true, lvs: false, pex: false }).expect("run");
    println!("not run: {:?}", c.skipped_rules());
    let ex = c.extracted();
    let store = &c.loaded.store;
    // (layer, bbox) → extracted net, to map routed wires back.
    let mut at: HashMap<(u16, i64, i64, i64, i64), u32> = HashMap::new();
    for n in 0..ex.nets.net_count() {
        let net = NetId(n as u32);
        let mut layers: BTreeMap<&str, usize> = BTreeMap::new();
        let (mut x0, mut y0, mut x1, mut y1) = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
        for &p in ex.nets.polys_of(net) {
            *layers.entry(c.layer_name(store.poly_layer(p))).or_default() += 1;
            let b = store.poly_bbox(p);
            let key = (b.xlo.raw(), b.ylo.raw(), b.xhi.raw(), b.yhi.raw());
            at.insert((store.poly_layer(p).0, key.0, key.1, key.2, key.3), n as u32);
            (x0, y0, x1, y1) = (x0.min(key.0), y0.min(key.1), x1.max(key.2), y1.max(key.3));
        }
        println!(
            "net {n} devices={} polys={layers:?} bbox=({x0},{y0})..({x1},{y1})",
            ex.devices.devices_on(net).len()
        );
    }

    let show = std::env::var("NET").ok();
    for (i, wires) in sol.routes.wires.iter().enumerate() {
        let net_name = &sol.netlist.nets[i].name;
        let mut hit: BTreeMap<Option<u32>, usize> = BTreeMap::new();
        for w in wires {
            let r = w.rect;
            let key = (w.layer.0, i64::from(r.x), i64::from(r.y), i64::from(r.x + r.w), i64::from(r.y + r.h));
            *hit.entry(at.get(&key).copied()).or_default() += 1;
        }
        println!("route {net_name}: {} wires -> extracted nets {hit:?}", wires.len());
        if show.as_deref() == Some(net_name.as_str()) {
            for w in wires {
                println!("   {} {:?}", c.layer_name(gdsverify::geom::LayerId(w.layer.0)), w.rect);
            }
        }
    }
}
