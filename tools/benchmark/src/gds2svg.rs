fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: gds2svg <file.gds> [pdk.json] [out.svg]");
        std::process::exit(1);
    }
    let pdk_path = args.get(2).map(|s| s.as_str()).unwrap_or("pdks/sky130.json");
    let pdk = std::fs::read_to_string(pdk_path).unwrap_or_default();
    let layer_names = pnr_visualizer::parse_layer_names(&pdk);
    let data = std::fs::read(&args[1]).expect("read GDS");
    let svg = pnr_visualizer::export_svg(&data, &layer_names);
    let out = args.get(3).map(|s| s.to_string())
        .unwrap_or_else(|| args[1].replace(".gds", ".svg"));
    std::fs::write(&out, &svg).expect("write SVG");
    println!("{out} ({} KB)", svg.len() / 1024);
}
