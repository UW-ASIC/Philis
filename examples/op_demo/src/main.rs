//! End-to-end demo: solve the operating point with ngspice, place-and-route
//! against it, and print the constraint-budget report.
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let spice = std::fs::read_to_string(root.join("benchmarks/fixtures/ota.spice")).unwrap();
    let pdk_json = std::fs::read_to_string(root.join("pdks/sky130.json")).unwrap();
    let pdk = verify::Pdk::from_json(&pdk_json).expect("pdk");

    // PDK_ROOT/PDK are exported by the dev shell (flake.nix fetches sky130 via
    // ciel/volare); fall back to the in-repo .pdk so the demo runs either way.
    let pdk_root = std::env::var("PDK_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| root.join("../.pdk"));
    let pdk_name = std::env::var("PDK").unwrap_or_else(|_| "sky130A".into());
    let models = pdk_root.join(&pdk_name).join("libs.tech/ngspice/sky130.lib.spice");

    let mut cfg = library::Config { feedback_iters: 3, ..Default::default() };
    cfg.op = Some(library::oppoint::OpConfig {
        model_lib: models.exists().then_some(models),
        ..Default::default()
    });

    let sol = library::run(&spice, &pdk, &library::Macros::default(), &cfg).expect("flow");
    println!("\n{}", sol.metadata);
    // ---- empirical signoff breakdown -------------------------------------
    let rep = library::signoff(&sol, &pdk);
    let mut hist: std::collections::BTreeMap<String, usize> = Default::default();
    for v in &rep.hard_violations {
        // Bucket by rule family: "drc/<kind>:<layer>" -> "drc/<kind>"
        let key = v.rule.split(':').next().unwrap_or(&v.rule).to_string();
        *hist.entry(key).or_default() += 1;
    }
    println!("\n  SIGNOFF breakdown ({} violations):", rep.hard_violations.len());
    for (k, n) in &hist {
        println!("    {:<40} {}", k, n);
    }
    println!("  LVS detail:");
    for v in rep.hard_violations.iter().filter(|v| v.rule.starts_with("lvs")) {
        println!("    {}", v.rule);
    }
    let mut full: std::collections::BTreeMap<String, usize> = Default::default();
    for v in rep.hard_violations.iter().filter(|v| v.rule.starts_with("drc")) {
        *full.entry(v.rule.clone()).or_default() += 1;
    }
    let mut rows: Vec<_> = full.into_iter().collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1));
    println!("  DRC by rule:layer (top 12):");
    for (k, n) in rows.iter().take(12) { println!("    {:<34} {}", k, n); }

    // The nm shortfall is what localises a DRC violation: a 550 nm shortfall on
    // `nwell` min_width is a *wire* on the well layer, not a malformed well. The
    // signoff `Violation` keeps only rule+margin, so re-run raw DRC for per-rule
    // margin spreads and locations.
    let shapes = sol.geometry();
    let raw = verify::drc(&shapes, &[], &pdk);
    let mut meas: std::collections::BTreeMap<(&str, &str), Vec<i64>> = Default::default();
    for v in &raw {
        meas.entry((v.rule.as_str(), v.layer.as_str())).or_default().push(v.margin_nm);
    }
    println!("  DRC shortfall (nm) by rule:layer (top rules):");
    let mut mrows: Vec<_> = meas.iter().collect();
    mrows.sort_by_key(|(_, m)| std::cmp::Reverse(m.len()));
    for ((rule, layer), m) in mrows.iter().take(6) {
        println!("    {:<24} n={:<4} shortfall min={} max={}",
            format!("{rule}:{layer}"), m.len(),
            m.iter().min().unwrap_or(&-1), m.iter().max().unwrap_or(&-1));
    }

    // Does a violation sit inside a placed cell? Route-vs-route is fixable by
    // pitch; route-vs-cell-internals means the router has no obstacle model.
    // Guard rings are appended as macros without a layout slot, so `macros` can
    // outrun `layout.x` — zip rather than index.
    let boxes: Vec<_> = sol.macros.iter()
        .zip(sol.layout.x.iter().zip(&sol.layout.y))
        .map(|(m, (&cx, &cy))| (m.bbox.w, m.bbox.h, cx, cy))
        .collect();
    for (rule, layer) in [("li_min_spacing", "li"), ("mcon_min_spacing", "mcon")] {
        let vs: Vec<_> = raw.iter()
            .filter(|v| v.rule == rule && v.layer == layer).collect();
        let inside = vs.iter().filter(|v| boxes.iter().any(|&(w, h, cx, cy)| {
            let (x, y) = (v.x as i32, v.y as i32);
            x >= cx - w / 2 && x <= cx + w / 2 && y >= cy - h / 2 && y <= cy + h / 2
        })).count();
        println!("    {rule}:{layer} n={} inside-a-macro={inside}", vs.len());
    }

    // ERC findings carry a location; the signoff `Violation` keeps only the
    // rule name, so re-run to see where each fires. (The old engine's free-text
    // `detail` is gone from the rewritten report model.)
    let erc = verify::erc(&shapes, &[], &pdk);
    let mut ercs: std::collections::BTreeMap<&str, Vec<(i64, i64)>> = Default::default();
    for v in &erc {
        ercs.entry(v.rule.as_str()).or_default().push((v.x, v.y));
    }
    println!("  ERC by rule:");
    for (rule, locs) in &ercs {
        println!("    {:<24} n={}", rule, locs.len());
        for (x, y) in locs.iter().take(4) {
            println!("        @({x}, {y})");
        }
    }

    // Which layers the router actually put metal on. Routing on a non-metal layer
    // is silent in every other report but shows up instantly here.
    let mut rl: std::collections::BTreeMap<String, usize> = Default::default();
    for net in &sol.routes.wires {
        for s in net {
            let name = pdk.layers.iter().find(|(_, l)| *l == s.layer)
                .map_or("?".into(), |(n, _)| n.clone());
            *rl.entry(name).or_default() += 1;
        }
    }
    println!("  route wire shapes by layer: {rl:?}");

    println!("  routing status per net:");
    for (i, n) in sol.netlist.nets.iter().enumerate() {
        let pins: usize = sol.netlist.devices.iter()
            .flat_map(|d| d.terminals.iter())
            .filter(|(_, id)| id.0 as usize == i)
            .count();
        let len = sol.routes.length(pnr_core::NetId(i as u16));
        println!("    {:<8} terminals={} routed_len={}", n.name, pins, len);
    }
    println!("  device temperatures (mK above ambient):");
    for (i, d) in sol.netlist.devices.iter().enumerate() {
        println!("    {:<5} P={:>4} uW  T=+{:>5} mK",
            d.name, sol.layout.power_uw[i], sol.layout.temp_mc[i]);
    }
}
