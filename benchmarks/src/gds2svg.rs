//! `gds2svg` — render a GDS file to SVG, layer names from the deck JSON.
//!
//!   cargo run --release -p benchmark --bin gds2svg -- <file.gds> [pdk.json] [out.svg]

use library::visualizer;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: gds2svg <file.gds> [pdk.json] [out.svg]");
        std::process::exit(1);
    }
    let pdk_path = args.get(2).map_or("pdks/sky130.json", String::as_str);
    let deck = std::fs::read_to_string(pdk_path).unwrap_or_else(|e| panic!("{pdk_path}: {e}"));
    let pdk = verify::Pdk::from_json(&deck).unwrap_or_else(|e| panic!("{pdk_path}: {e}"));
    let gds = pdk.layer_gds();
    let pairs: Vec<_> = pdk.layers.iter().map(|(n, id)| (n.clone(), gds[id.0 as usize])).filter(|(_, g)| *g != (0, 0)).collect();
    let names = visualizer::layer_names(&pairs);
    let svg = visualizer::export_svg(&std::fs::read(&args[1]).expect("read GDS"), &names);
    let out = args.get(3).cloned().unwrap_or_else(|| args[1].replace(".gds", ".svg"));
    std::fs::write(&out, &svg).expect("write SVG");
    println!("{out} ({} KB)", svg.len() / 1024);
}
