//! Throwaway probe: extract a bench GDS with the current deck and dump devices.
//! cargo run --release -p pnr-backend --example extract_probe <gds> <pdk.json>

use gdsverify::{extract_netlist_opts, Backend, Deck, ExtractOpts};

fn main() {
    let mut args = std::env::args().skip(1);
    let gds_path = args.next().expect("gds path");
    let pdk_path = args.next().expect("pdk json path");
    let deck = Deck::from_json(&std::fs::read_to_string(&pdk_path).unwrap()).unwrap();
    let layout = gdsverify::load_gds(&gds_path, &deck).unwrap();
    let top = layout.top_cells.first().expect("top cell").clone();
    let store = layout.cells.into_iter().find(|(n, _)| *n == top).unwrap().1;
    let ext = extract_netlist_opts(
        &store,
        &deck,
        &ExtractOpts {
            cut_required: deck.lvs_cut_required,
            ..Default::default()
        },
        Backend::Cpu,
    )
    .unwrap();
    println!("devices: {}", ext.devices.len());
    for d in &ext.devices {
        println!(
            "  {:?} {:?} g={} s={} d={} b={} w={} l={}",
            d.kind, d.flavor, d.gate, d.source, d.drain, d.body, d.w, d.l
        );
    }
    println!("two_terminal: {}", ext.two_terminal.len());
    for d in &ext.two_terminal {
        println!(
            "  {:?} {} a={} b={} value={}",
            d.kind, d.name, d.terminal_a, d.terminal_b, d.value
        );
    }
    println!("bjt: {}", ext.bjt_devices.len());
    for d in &ext.bjt_devices {
        println!("  {:?} c={} b={} e={}", d.kind, d.collector, d.base, d.emitter);
    }
    for (net, name) in &ext.net_names {
        println!("net {net} = {name}");
    }
}
