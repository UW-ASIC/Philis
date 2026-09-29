// TEMPORARY diagnostics (routing agent): router DRC on gf180/IHP.
#[test]
#[ignore]
fn dump() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for deck in ["ihp_sg13g2"] {
        let pdk = verify::Pdk::from_json(&std::fs::read_to_string(root.join(format!("pdks/{deck}.json"))).unwrap()).unwrap();
        for name in ["ota"] {
            let spice = std::fs::read_to_string(root.join(format!("benchmarks/fixtures/{name}.spice"))).unwrap();
            let cfg = library::Config { feedback_iters: 1, ..Default::default() };
            let sol = library::run(&spice, &pdk, &library::Macros::default(), &cfg).unwrap();
            let shapes = sol.geometry();
            for f in verify::drc(&shapes, &[], &pdk) {
                if f.rule.contains("density") { continue; }
                println!("{deck} {name} {} {} m={} at ({},{})", f.rule, f.layer, f.margin_nm, f.x, f.y);
                for s in shapes.iter().filter(|s| (s.rect.x as i64) - 400 <= f.x && f.x <= (s.rect.x + s.rect.w) as i64 + 400 && (s.rect.y as i64) - 400 <= f.y && f.y <= (s.rect.y + s.rect.h) as i64 + 400) {
                    println!("    L{} {} {} {} {}", s.layer.0, s.rect.x, s.rect.y, s.rect.x + s.rect.w, s.rect.y + s.rect.h);
                }
                for (n, w) in sol.routes.wires.iter().enumerate() {
                    for s in w.iter().filter(|s| (s.rect.x as i64) - 400 <= f.x && f.x <= (s.rect.x + s.rect.w) as i64 + 400 && (s.rect.y as i64) - 400 <= f.y && f.y <= (s.rect.y + s.rect.h) as i64 + 400) {
                        println!("    route net{n} L{} {} {} {} {}", s.layer.0, s.rect.x, s.rect.y, s.rect.x + s.rect.w, s.rect.y + s.rect.h);
                    }
                }
            }
        }
    }
}
