//! `gds2svg` — render a GDS file to SVG via the `visualizer` (re-exported through
//! `library`).
//!
//!   cargo run --release -p benchmark --bin gds2svg -- <file.gds> [pdk.json] [out.svg]
//!
//! The deck JSON's `layers` block (name → `{layer, datatype}`) supplies layer
//! names for the SVG group ids; colours come from the layer number regardless.

use library::visualizer;

/// Build the `(gds_layer, gds_datatype) → name` map the SVG exporter labels with,
/// from a deck JSON's `layers` object.
fn layer_names(pdk_json: &str) -> visualizer::LayerMap {
    let mut names = visualizer::LayerMap::new();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(pdk_json) {
        if let Some(layers) = v.get("layers").and_then(|l| l.as_object()) {
            for (name, val) in layers {
                let l = val.get("layer").and_then(|x| x.as_i64());
                let d = val.get("datatype").and_then(|x| x.as_i64());
                if let (Some(l), Some(d)) = (l, d) {
                    names.insert((l as i32, d as i32), name.clone());
                }
            }
        }
    }
    names
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: gds2svg <file.gds> [pdk.json] [out.svg]");
        std::process::exit(1);
    }
    let pdk_path = args.get(2).map(|s| s.as_str()).unwrap_or("pdks/sky130.json");
    let pdk_json = std::fs::read_to_string(pdk_path).unwrap_or_default();
    let names = layer_names(&pdk_json);
    let data = std::fs::read(&args[1]).expect("read GDS");
    let svg = visualizer::export_svg(&data, &names);
    let out = args
        .get(3)
        .map(|s| s.to_string())
        .unwrap_or_else(|| args[1].replace(".gds", ".svg"));
    std::fs::write(&out, &svg).expect("write SVG");
    println!("{out} ({} KB)", svg.len() / 1024);
}
