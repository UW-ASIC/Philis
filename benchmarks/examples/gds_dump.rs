//! Emit a fixture's auto-flow GDS for an independent extractor to read.
//! `cargo run -p benchmark --example gds_dump -- <fixture> <out.gds>`
fn main() {
    let mut a = std::env::args().skip(1);
    let name = a.next().expect("fixture name");
    let out = a.next().expect("out path");
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let pdk = verify::Pdk::from_json(
        &std::fs::read_to_string(root.join("pdks/sky130.json")).unwrap()).unwrap();
    let spice = std::fs::read_to_string(
        root.join(format!("benchmarks/fixtures/{name}.spice"))).unwrap();
    let cfg = library::Config { feedback_iters: 1, ..Default::default() };
    let sol = library::run(&spice, &pdk, &library::Macros::default(), &cfg).expect("flow");
    let shapes = sol.geometry();
    std::fs::write(&out, library::gds::emit(&shapes, &pdk.layer_gds())).unwrap();
    // How much nwell is drawn, and does it sit over diff?
    let (nw, df) = (pdk.layer("nwell"), pdk.layer("diff"));
    use pnr_core::Process as _;
    let nwells: Vec<_> = shapes.iter().filter(|s| Some(s.layer) == nw).map(|s| s.rect).collect();
    let diffs: Vec<_> = shapes.iter().filter(|s| Some(s.layer) == df).map(|s| s.rect).collect();
    let covered = diffs.iter().filter(|d| nwells.iter().any(|n|
        n.x <= d.x && n.y <= d.y && n.x + n.w >= d.x + d.w && n.y + n.h >= d.y + d.h)).count();
    println!("{name}: {} nwell rects, {} diff rects, {covered} diff rects fully inside an nwell",
        nwells.len(), diffs.len());
}
