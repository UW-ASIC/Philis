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
    let names = visualizer::parse_layer_names(&std::fs::read_to_string(pdk_path).unwrap_or_default());
    let svg = visualizer::export_svg(&std::fs::read(&args[1]).expect("read GDS"), &names);
    let out = args.get(3).cloned().unwrap_or_else(|| args[1].replace(".gds", ".svg"));
    std::fs::write(&out, &svg).expect("write SVG");
    println!("{out} ({} KB)", svg.len() / 1024);
}
