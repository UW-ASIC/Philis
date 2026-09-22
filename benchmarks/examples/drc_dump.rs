//! Throwaway diagnostic: run a fixture through the real flow, then dump every
//! signoff violation grouped by rule, so a count can be attributed instead of
//! guessed at. `cargo run --release -p benchmark --example drc_dump [fixture]`.

use std::collections::BTreeMap;

use library::{Config, Macros};
use verify::Pdk;

fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let name = std::env::args().nth(1).unwrap_or_else(|| "ota".into());
    let text = std::fs::read_to_string(root.join(format!("benchmarks/fixtures/{name}.spice")))
        .expect("fixture");
    let deck_json = std::fs::read_to_string(root.join("pdks/sky130.json")).expect("deck");
    let pdk = Pdk::from_json(&deck_json).expect("deck parse");

    let iters = std::env::var("ITERS").ok().and_then(|v| v.parse().ok()).unwrap_or(5);
    let cfg = Config { seed: 1, feedback_iters: iters, ..Config::default() };
    let sol = library::run(&text, &pdk, &Macros::default(), &cfg).expect("flow");
    let report = library::signoff(&sol, &pdk);

    let mut by_rule: BTreeMap<String, usize> = BTreeMap::new();
    for v in &report.hard_violations {
        *by_rule.entry(v.rule.clone()).or_default() += 1;
    }
    println!("=== {name}: {} hard violations ===", report.hard_violations.len());
    for (rule, n) in &by_rule {
        println!("  {n:6}  {rule}");
    }

    // Grid alignment of the drawn geometry: an off-grid stamp shows up as a
    // flood of per-vertex violations, so check it directly rather than inferring.
    let grid = pdk.grid;
    let shapes = sol.geometry();
    let off = shapes
        .iter()
        .filter(|s| {
            s.rect.x % grid != 0
                || s.rect.y % grid != 0
                || s.rect.w % grid != 0
                || s.rect.h % grid != 0
        })
        .count();
    println!("grid = {grid} nm; {off}/{} shapes off-grid", shapes.len());
    println!(
        "stats: iters {} best {} conv {} outer {} esc {} place_hard {} route_hard {} drc_hard {} overuse {}",
        sol.stats.iterations,
        sol.stats.best_iteration,
        sol.stats.converged,
        sol.stats.outer_iterations,
        sol.stats.variant_escalations,
        sol.stats.place_hard,
        sol.stats.route_hard,
        sol.stats.drc_hard,
        sol.stats.route_overuse
    );

    // Rough LiveOracle::drc per-call cost on this cell's geometry.
    {
        use pnr_core::Oracle as _;
        let oracle = verify::LiveOracle::new(&pdk).expect("oracle");
        let n = 20;
        let t0 = std::time::Instant::now();
        for _ in 0..n {
            let _ = oracle.drc(&shapes);
        }
        println!(
            "LiveOracle::drc: {:.2} ms/call ({n} calls, {} shapes)",
            t0.elapsed().as_secs_f64() * 1000.0 / f64::from(n),
            shapes.len()
        );
    }

    // TEMP: extracted devices + label bindings, for LVS pairing failures.
    if std::env::var("DUMP_DEVICES").is_ok() {
        let mut checker = verify::Checker::new(&pdk, true).expect("checker");
        match checker.debug_devices(&shapes, &[]) {
            Ok(lines) => {
                for l in lines {
                    println!("{l}");
                }
            }
            Err(e) => println!("debug_devices failed: {e}"),
        }
    }

    // TEMP: dump raw DRC/ERC findings with coordinates
    if std::env::var("DUMP_FINDINGS").is_ok() {
        for f in verify::drc(&shapes, &[], &pdk) {
            println!("finding {}:{} margin={} at ({}, {})", f.rule, f.layer, f.margin_nm, f.x, f.y);
        }
        for f in verify::erc(&shapes, &[], &pdk) {
            println!("erc {}:{} at ({}, {})", f.rule, f.layer, f.x, f.y);
        }
    }

    // TEMP: dump conductor/cut shapes for connectivity tracing
    if std::env::var("DUMP_SHAPES").is_ok() {
        for s in &shapes {
            let name = pdk
                .layers
                .iter()
                .find(|(_, id)| *id == s.layer)
                .map(|(n, _)| n.as_str())
                .unwrap_or("?");
            println!(
                "shape {name} x={} y={} w={} h={}",
                s.rect.x, s.rect.y, s.rect.w, s.rect.h
            );
        }
        // Placed pins with their schematic net names, same frame as the shapes.
        for m in pnr_core::place_macros(&sol.macros, &sol.layout) {
            for p in &m.pins {
                let net = sol
                    .netlist
                    .nets
                    .get(p.net.0 as usize)
                    .map(|n| n.name.as_str())
                    .unwrap_or("?");
                let lname = pdk
                    .layers
                    .iter()
                    .find(|(_, id)| id.0 == p.layer.0)
                    .map(|(n, _)| n.as_str())
                    .unwrap_or("?");
                println!(
                    "pin {net} layer={lname} x={} y={} w={} h={}",
                    p.at.x, p.at.y, p.at.w, p.at.h
                );
            }
        }
        // Per-net route shapes: which net drew which wire.
        for (ni, wires) in sol.routes.wires.iter().enumerate() {
            let net = sol
                .netlist
                .nets
                .get(ni)
                .map(|n| n.name.as_str())
                .unwrap_or("?");
            for s in wires {
                let name = pdk
                    .layers
                    .iter()
                    .find(|(_, id)| *id == s.layer)
                    .map(|(n, _)| n.as_str())
                    .unwrap_or("?");
                println!(
                    "wire {net} {name} x={} y={} w={} h={}",
                    s.rect.x, s.rect.y, s.rect.w, s.rect.h
                );
            }
        }
    }
}
