fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: visualizer <file.gds> [pdk.json]");
        std::process::exit(1);
    }
    let layer_names = args
        .get(2)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|json| visualizer::parse_layer_names(&json))
        .unwrap_or_default();
    visualizer::run_viewer(args[1].clone().into(), layer_names);
}
