fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        // No deck loader here (no `verify` dependency): layers read `L{l}/{d}`;
        // `gds2svg` names them from a deck.
        eprintln!("usage: visualizer <file.gds>");
        std::process::exit(1);
    }
    visualizer::run_viewer(args[1].clone().into(), visualizer::LayerMap::new());
}
