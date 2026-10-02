//! The post-layout performance loop against real ngspice and the installed
//! sky130 models: the OTA fixture's gain with its layout's branch resistance
//! and stress, and a full flow scored on it. Skips without ngspice or models.

use library::oppoint::OpConfig;
use library::perf::{evaluate, Parasitics, PerfConfig, Spec};

fn models() -> Option<std::path::PathBuf> {
    let root = std::env::var_os("PDK_ROOT")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::Path::new(&h).join(".volare")))?;
    let lib = root.join("sky130A/libs.tech/ngspice/sky130.lib.spice");
    let ngspice = std::process::Command::new("ngspice").arg("--version").output().is_ok();
    (lib.is_file() && ngspice).then_some(lib)
}

/// The fixture as a 5T OTA: `vbias` tied to `vout1` (M3 diode-connected),
/// the tail at 0.5 V, `vinm` fed back from `vout2` at DC through 1 GΩ and
/// held at AC by 1 F (so the loop only sets the bias), AC into `vinp`. Gain at
/// 1 kHz is ~33 dB on the schematic.
const BENCH: &str = "Vdd vdd 0 1.8
Vtie vbias vout1 0
Vbn vbn 0 0.5
Vp vinp 0 0.9 ac 1
Rfb vout2 vinm 1e9
Cfb vinm 0 1
.ac dec 10 10 1e9
.control
run
meas ac gain find vdb(vout2) at=1e3
.endc";

fn cfg(lib: std::path::PathBuf) -> PerfConfig {
    PerfConfig {
        sim: OpConfig { model_lib: Some(lib), ..OpConfig::default() },
        testbench: BENCH.into(),
        specs: vec![Spec { metric: "gain".into(), min: Some(20.0), max: None }],
    }
}

fn ota() -> pnr_core::Netlist {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut nl = library::parse(&std::fs::read_to_string(root.join("benchmarks/fixtures/ota.spice")).unwrap()).unwrap();
    let pdk = verify::Pdk::from_json(&std::fs::read_to_string(root.join("pdks/sky130.json")).unwrap()).unwrap();
    library::deck_models(&mut nl, &pdk);
    nl
}

#[test]
fn branch_resistance_and_stress_reach_the_simulation() {
    let Some(lib) = models() else {
        eprintln!("ngspice or sky130 models unavailable — skipping");
        return;
    };
    let nl = ota();
    let gain = |p: &Parasitics| evaluate(&nl, p, &cfg(lib.clone())).unwrap().metrics[0].1;
    let plain = gain(&Parasitics::default()).expect("the schematic's gain is measured");
    // Source degeneration on the input pair (the tail runs in weak
    // inversion, gm ≈ 10 µS, so 200 kΩ is gm·R ≈ 2): gain drops.
    let n = nl.devices.len();
    let degenerated = Parasitics {
        series: (0..n).map(|d| if d < 2 { vec![("S".to_string(), 200_000.0)] } else { Vec::new() }).collect(),
        ..Parasitics::default()
    };
    let r = gain(&degenerated).expect("measured");
    assert!(plain > 25.0, "the bench biases the OTA: {plain} dB");
    assert!(r < plain - 1.0, "source R must cost gain: {plain} dB vs {r} dB");
    // Short diffusion ends (strong LOD stress) shift VT and mobility. On one
    // mirror half (M3): the same stress on both halves cancels in the gain
    // (0.012 dB with W read per finger, 0.0099 dB at the SPICE sizes of
    // FLOW-01), on M3 alone it moves the gain by 0.53 dB.
    let stressed = Parasitics { lod_inv_um: (0..n).map(|d| (d == 2).then_some(2.0)).collect(), ..Parasitics::default() };
    let s = gain(&stressed).expect("measured");
    assert!((s - plain).abs() > 0.01, "stress reached the models: {plain} dB vs {s} dB");
}

/// A full run scored on post-layout gain: the epoch's parasitics (extracted
/// C, routed branch R, drawn stress) simulate and the spec is met.
#[test]
fn a_flow_scores_its_layout_in_simulation() {
    let Some(lib) = models() else {
        eprintln!("ngspice or sky130 models unavailable — skipping");
        return;
    };
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let pdk = verify::Pdk::from_json(&std::fs::read_to_string(root.join("pdks/sky130.json")).unwrap()).unwrap();
    let spice = std::fs::read_to_string(root.join("benchmarks/fixtures/ota.spice")).unwrap();
    let c = library::Config {
        feedback_iters: 2,
        outer_iters: 1,
        starts: 1,
        op: Some(OpConfig { model_lib: Some(lib.clone()), ..OpConfig::default() }),
        performance: Some(cfg(lib)),
        ..library::Config::default()
    };
    let sol = library::run(&spice, &pdk, &library::Macros::default(), &c).expect("flow");
    let perf = &sol.metadata.performance;
    assert_eq!(perf.len(), 1, "{perf:?}");
    let (metric, value, ..) = &perf[0];
    assert_eq!(metric, "gain");
    assert!(value.is_some_and(|g| g >= 20.0), "post-layout gain misses 20 dB: {perf:?}");
}
