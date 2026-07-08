fn main() {
    env_logger::init();
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: pnr-visualizer <file.gds> [pdk.json]");
        std::process::exit(1);
    }
    let pdk_path = args.get(2).map(|s| s.as_str());
    let layer_names = pdk_path
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|json| pnr_visualizer::parse_layer_names(&json))
        .unwrap_or_default();
    pnr_visualizer::run_viewer(args[1].clone().into(), layer_names);
}
