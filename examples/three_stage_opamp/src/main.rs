//! # Side-by-side example: user macro + auto place-and-route
//!
//! A 3-stage MOSFET op-amp. The user hands the pipeline a **pre-drawn macro** for
//! one part of the design — the big output PMOS `M8` — via the `macro_master`
//! registry, and the library auto-generates and places/routes everything else.
//! Downstream can't tell the injected macro from a generated one; the only
//! difference is that `M8`'s DAG block is flagged `injected`.
//!
//! Run: `cargo run -p three_stage_opamp -- <deck.json>`
//! (the deck is a gdsverify PDK JSON — the same one the `philis` CLI takes).

use library::{run, signoff, Config, Macros};
use pnr_core::Macro;
use verify::Pdk;

/// A 3-stage op-amp, MOSFETs only: input diff pair + active load (stage 1), a
/// common-source gain stage (stage 2), a push-pull output (stage 3), tail/bias.
const OPAMP_3STAGE: &str = r"
.subckt opamp3 vin_p vin_n vout vdd vss vbias
* stage 1 — input differential pair + tail current source
M1 n1 vin_p tail vss nfet w=4u l=0.5u
M2 n2 vin_n tail vss nfet w=4u l=0.5u
M3 tail vbias vss vss nfet w=8u l=0.5u
* stage 1 — active load (PMOS current mirror)
M4 n1 n1 vdd vdd pfet w=6u l=0.5u
M5 n2 n1 vdd vdd pfet w=6u l=0.5u
* stage 2 — common-source gain
M6 n3 n2 vdd vdd pfet w=12u l=0.5u
M7 n3 vbias vss vss nfet w=6u l=0.5u
* stage 3 — output driver (M8 is user-provided below)
M8 vout n3 vdd vdd pfet w=40u l=0.5u
M9 vout vbias vss vss nfet w=20u l=0.5u
.ends
";

fn main() {
    let Some(deck_path) = std::env::args().nth(1) else {
        eprintln!("usage: three_stage_opamp <deck.json>");
        std::process::exit(2);
    };
    let deck = std::fs::read_to_string(&deck_path).expect("read deck json");
    let pdk = Pdk::from_json(&deck).expect("parse pdk deck");

    // ── The part of the design the *user* does themselves ──
    // Register a pre-drawn macro for the output PMOS `M8` under its instance name.
    // It is drawn by a `cells::Cell` generator (`cells::mosfet::Mosfet`); a
    // `macro_master::Composition` could assemble it from parts too. The library uses it
    // verbatim and auto-generates M1–M7, M9.
    let mut macros = Macros::default();
    macros.register("M8", user_output_pmos(&pdk));

    let cfg = Config::default();
    let sol = run(OPAMP_3STAGE, &pdk, &macros, &cfg).expect("place-and-route");

    println!(
        "placed {} devices ({} routed nets); M8 was user-injected, the rest auto-drawn",
        sol.macros.len(),
        sol.routes.wires.len()
    );
    // PLC-01: the winner's placement metrics and dp counters (T7's second fixture).
    println!("{:?}\n{:?}", sol.stats.place, sol.stats.dp);

    let report = signoff(&sol, &pdk).report;
    if report.hard_violations.is_empty() {
        println!("signoff CLEAN — cost {:.3}", report.cost);
    } else {
        println!(
            "signoff — {} hard violation(s)",
            report.hard_violations.len()
        );
    }

    // Show the placed-and-routed layout (blocks until the window closes; a no-op
    // on a headless host — `Probe::open` reports "no display" and returns).
    let polys = library::visualizer::polys_from_shapes(&sol.geometry(), &pdk.layer_gds());
    let mut probe = library::visualizer::Probe::open("three_stage_opamp");
    probe.send(&polys, Some("three_stage_opamp: placed + routed"));
    probe.wait();
}

/// The user's output PMOS, drawn outside the flow with the same MOSFET
/// generator (`cells::mosfet::Mosfet`) the flow's own cells use: one 40/0.5 µm device with
/// contacts, implant, well and bulk tap — a real transistor, so LVS pairs it.
/// (A hand-drawn bar of diff and poly with no implant or contacts extracts to
/// nothing and unpairs the whole circuit.)
///
/// Pins are renamed `d0:G` → `G`: the library binds an injected macro's pins
/// to the instance's terminals by name.
fn user_output_pmos(pdk: &Pdk) -> Macro {
    use analog::cell::{SeriesParallel, Unitization};
    use cells::{mosfet::Mosfet, Cell};
    use pnr_core::{DeviceGroup, DeviceId, DeviceKind};

    let group = DeviceGroup { devices: vec![DeviceId(0)] };
    let mut c = analog::Constraints::default();
    c.unitization.push(Unitization {
        devices: group.devices.clone(),
        device_type: DeviceKind::Pmos,
        dev_nf: vec![1],
        target_ratio: vec![1],
        unit_w: 40_000,
        unit_l: 500,
        series_parallel: SeriesParallel::Parallel,
        dummy_required: false,
        route_matching_required: false,
        class: None,
    });
    let variant = Mosfet::enumerate(&group, &c, pdk).into_iter().next().expect("a PMOS variant");
    let mut m = variant.draw(&group, &c, pdk);
    for pin in &mut m.pins {
        pin.name = pin.name.trim_start_matches("d0:").to_string();
    }
    m
}
