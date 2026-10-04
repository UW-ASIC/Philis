//! The post-layout performance loop against real ngspice and the installed
//! sky130 models: the OTA fixture's gain with its layout's branch resistance
//! and stress, and a full flow scored on it. Without ngspice or the models
//! each test prints why and returns; `PHILIS_REQUIRE_TOOLS=1` (CI's nightly
//! job) turns that skip into a panic, so a missing tool is never a green run.

use library::oppoint::OpConfig;
use library::perf::{evaluate, Parasitics, PerfConfig, Scenario, Spec};
use library::tools::{sky130_models as models, tool_or_skip};

/// The cargo running this build reads as present (so detection cannot
/// silently skip every ngspice test); a binary that is on no PATH skips
/// (returns `false`) in a plain run and panics when the run requires tools.
#[test]
fn tool_or_skip_tells_present_from_missing() {
    assert!(
        tool_or_skip(env!("CARGO")),
        "the building cargo must read as present"
    );
    let bin = "philis-no-such-binary-7f3a";
    if std::env::var_os("PHILIS_REQUIRE_TOOLS").is_some_and(|v| v == "1") {
        assert!(
            std::panic::catch_unwind(|| tool_or_skip(bin)).is_err(),
            "a required tool that is absent must panic"
        );
    } else {
        assert!(
            !tool_or_skip(bin),
            "an absent binary must not read as present"
        );
    }
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
        testbenches: vec![BENCH.into()],
        specs: vec![Spec { metric: "gain".into(), min: Some(20.0), max: None }],
        scenarios: Vec::new(),
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
    let Some(lib) = models() else { return };
    let nl = ota();
    let gain = |p: &Parasitics| evaluate(&nl, p, &cfg(lib.clone()), &[0]).unwrap().metrics[0].1;
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
    // (0.012 dB before XM5's m=4 reached the simulator, 0.0099 dB since),
    // on M3 alone it moves the gain by 0.53 dB.
    let stressed = Parasitics { lod_inv_um: (0..n).map(|d| (d == 2).then_some(2.0)).collect(), ..Parasitics::default() };
    let s = gain(&stressed).expect("measured");
    assert!((s - plain).abs() > 0.01, "stress reached the models: {plain} dB vs {s} dB");
}

/// A full run scored on post-layout gain: the epoch's parasitics (extracted
/// C, routed branch R, drawn stress) simulate and the spec is met.
#[test]
fn a_flow_scores_its_layout_in_simulation() {
    let Some(lib) = models() else { return };
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
    // Real models measure the schematic, so the floor gets a row with nets
    // (`row (N nets)`; an empty row reads `row with no measured nets`).
    let rows = &sol.metadata.budget_rows;
    assert!(rows.len() == 1 && rows[0].starts_with("gain:min: row ("), "{rows:?}");
    assert_eq!(sol.stats.sim_failures, 0, "{:?}", sol.stats);
}

fn scenario(name: &str, corner: &str, temp_c: f64) -> Scenario {
    Scenario { name: name.into(), corner: corner.into(), temp_c, params: Vec::new() }
}

/// PERF-10: two testbenches that both measure a spec's metric are refused,
/// not resolved by order.
#[test]
fn a_metric_from_two_testbenches_is_refused() {
    let Some(lib) = models() else { return };
    let p = PerfConfig { testbenches: vec![BENCH.into(), BENCH.into()], ..cfg(lib) };
    let e = evaluate(&ota(), &Parasitics::default(), &p, &[0]).unwrap_err();
    assert!(e.contains("testbenches 0 and 1"), "{e}");
}

/// PERF-10: the perf deck sets `.temp`, so the scenario's temperature moves
/// the schematic's gain.
#[test]
fn ota_gain_moves_with_temperature() {
    let Some(lib) = models() else { return };
    let p = PerfConfig { scenarios: vec![scenario("tt27", "tt", 27.0), scenario("tt125", "tt", 125.0)], ..cfg(lib) };
    let nl = ota();
    let g = |i: usize| evaluate(&nl, &Parasitics::default(), &p, &[i]).unwrap().metrics[0].1.expect("gain measured");
    let (g27, g125) = (g(0), g(1));
    assert!((g27 - g125).abs() > 0.1, "{g27} dB at 27 °C vs {g125} dB at 125 °C");
}

/// PERF-10 acceptance: a flow over three corners reports, per bound, the
/// scenario the schematic is worst at, and simulates epochs only at the
/// active ones (nominal plus each bound's worst).
#[test]
fn a_flow_reports_the_worst_scenario_per_bound() {
    let Some(lib) = models() else { return };
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let pdk = verify::Pdk::from_json(&std::fs::read_to_string(root.join("pdks/sky130.json")).unwrap()).unwrap();
    let spice = std::fs::read_to_string(root.join("benchmarks/fixtures/ota.spice")).unwrap();
    let p = PerfConfig {
        scenarios: vec![scenario("tt_27", "tt", 27.0), scenario("ss_125", "ss", 125.0), scenario("ff_m40", "ff", -40.0)],
        ..cfg(lib.clone())
    };
    let nl = ota();
    let gains: Vec<f64> = (0..3)
        .map(|i| evaluate(&nl, &Parasitics::default(), &p, &[i]).unwrap().metrics[0].1.expect("gain measured"))
        .collect();
    let lowest = (0..3).min_by(|&a, &b| gains[a].total_cmp(&gains[b])).unwrap();
    let c = library::Config {
        feedback_iters: 2,
        outer_iters: 1,
        starts: 1,
        op: Some(OpConfig { model_lib: Some(lib), ..OpConfig::default() }),
        performance: Some(p.clone()),
        ..library::Config::default()
    };
    let sol = library::run(&spice, &pdk, &library::Macros::default(), &c).expect("flow");
    let worst = &sol.metadata.performance_worst;
    assert_eq!(worst.len(), 1, "{worst:?}");
    assert!(worst[0].starts_with("gain:min worst "), "{worst:?}");
    assert!(worst[0].contains(&format!(" at {} (", p.scenarios[lowest].name)), "gains {gains:?}: {worst:?}");
    let k: usize = worst[0].split("(over ").nth(1).and_then(|t| t.split(' ').next()).and_then(|k| k.parse().ok()).expect("active count");
    assert!(k <= 2 && worst[0].contains(" active of 3 scenarios)"), "{worst:?}");
}

/// A fixture parsed as the flow parses it: the deck's model table, then its
/// model names.
fn fixture(name: &str) -> pnr_core::Netlist {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sky130 = verify::Pdk::from_json(&std::fs::read_to_string(root.join("pdks/sky130.json")).unwrap()).unwrap();
    let text = std::fs::read_to_string(root.join(format!("benchmarks/fixtures/{name}.spice"))).unwrap();
    let opts = library::ParseOptions { models: library::model_table(&sky130), ..Default::default() };
    let mut nl = library::spice_with(&text, &opts).unwrap();
    library::deck_models(&mut nl, &sky130);
    nl
}

fn op_cfg(lib: std::path::PathBuf) -> OpConfig {
    OpConfig {
        model_lib: Some(lib),
        params: library::oppoint::SKY130_NPN_NOMINAL.iter().map(|&(k, v)| (k.to_string(), v)).collect(),
        ..OpConfig::default()
    }
}

/// The simulated circuit is the drawn one: every device of a resistor and a
/// BJT fixture reaches the operating point (M1 PERF criterion).
#[test]
fn fixtures_resolve_every_device() {
    let Some(lib) = models() else { return };
    for name in ["rc_filter", "bjt_mirror"] {
        let nl = fixture(name);
        let op = library::oppoint::extract(&nl, &op_cfg(lib.clone())).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(op.resolved, nl.devices.len(), "{name}");
    }
}

/// The poly resistor carries the inverter's output into a load, and its
/// current closes KCL on `vmid` with the two FETs.
#[test]
fn rc_filter_resistor_carries_current() {
    let Some(lib) = models() else { return };
    let nl = fixture("rc_filter");
    // The probe leaves `vout` unloaded, so the resistor would carry 0.
    let cfg = OpConfig { testbench: Some("Vdd vdd 0 1.8\nVin vin 0 0.9\nRload vout 0 10k\n".into()), ..op_cfg(lib) };
    let op = library::oppoint::extract(&nl, &cfg).unwrap();
    let t = op.terminal_ua(&nl);
    let r = nl.devices.iter().position(|d| d.name == "XR1").unwrap();
    let p = t[r].as_ref().unwrap().iter().find(|(n, _)| n == "P").unwrap().1;
    assert!(p.abs() > 1.0, "XR1 carries {p} µA");
    let vmid = nl.nets.iter().position(|n| n.name == "vmid").unwrap();
    let sum: f64 = nl
        .devices
        .iter()
        .zip(&t)
        .flat_map(|(d, c)| d.terminals.iter().zip(c.as_ref().unwrap()).filter(|((_, n), _)| n.0 as usize == vmid).map(|(_, (_, i))| *i))
        .sum();
    assert!(sum.abs() < 1e-3, "KCL on vmid: {sum} µA {t:?}");
}

#[test]
fn an_inductor_netlist_cannot_simulate() {
    let Some(lib) = models() else { return };
    let nl = library::parse("XM1 d g 0 0 nfet_01v8 W=1u L=1u\nL1 d 0 1n\n").unwrap();
    let e = library::oppoint::extract(&nl, &op_cfg(lib)).err().expect("an inductor is refused");
    assert!(e.contains("cannot simulate"), "{e}");
}

/// rc_filter routed once and extracted as a post-layout `.subckt rc_filter`.
fn rc_filter_pex() -> (library::Solution, String) {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let pdk = verify::Pdk::from_json(&std::fs::read_to_string(root.join("pdks/sky130.json")).unwrap()).unwrap();
    let spice = std::fs::read_to_string(root.join("benchmarks/fixtures/rc_filter.spice")).unwrap();
    let c = library::Config { feedback_iters: 1, starts: 1, ..library::Config::default() };
    let sol = library::run(&spice, &pdk, &library::Macros::default(), &c).expect("flow");
    let pex = library::post_layout_spice(&sol, &pdk, "rc_filter").unwrap();
    (sol, pex)
}

/// `v(vout)` of `deck` under `ngspice -b` in `dir`; panics on a failed run or
/// an `Error` line.
fn vout(dir: &std::path::Path, file: &str, deck: &str) -> f64 {
    std::fs::write(dir.join(file), deck).unwrap();
    let o = std::process::Command::new("ngspice").arg("-b").arg(file).current_dir(dir).output().expect("ngspice runs");
    let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert!(o.status.success(), "{file}: ngspice failed\n{text}");
    assert!(!text.lines().any(|l| l.trim_start().starts_with("Error") || l.trim_start().starts_with("ERROR")), "{file}:\n{text}");
    text.lines()
        .find_map(|l| l.trim().strip_prefix("v(vout) = ")?.trim().parse().ok())
        .unwrap_or_else(|| panic!("{file}: no v(vout)\n{text}"))
}

/// The extracted layout simulates and its DC output matches the schematic's
/// within 1 % (M1 PERF criterion). `vin = 0` keeps the PMOS in triode, so the
/// comparison does not ride the inverter's gain; `Rload` draws ~170 µA through
/// the PMOS, the resistor and the routing, so their extracted sizes and
/// parasitics move `v(vout)` (unloaded it sits at VDD for any deck).
#[test]
fn post_layout_rc_filter_simulates() {
    let Some(lib) = models() else { return };
    let (_, pex) = rc_filter_pex();
    let dir = std::env::temp_dir().join(format!("philis_pex_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let bench = |body: &str| {
        format!(
            "* pex\n.lib {} tt\n{body}\nXdut vin vout VDD VSS rc_filter\nVdd VDD 0 1.8\nVss VSS 0 0\nVin vin 0 0\nRload vout 0 10k\n.control\nop\nprint v(vout)\n.endc\n.end\n",
            lib.display()
        )
    };
    let sch = ".subckt rc_filter vin vout VDD VSS
XM1 vmid vin VDD VDD sky130_fd_pr__pfet_01v8 W=1 L=0.15 nf=1 m=1
XM2 vmid vin VSS VSS sky130_fd_pr__nfet_01v8 W=0.5 L=0.15 nf=1 m=1
RXR1 vmid vout sky130_fd_pr__res_generic_po w=0.5 l=2
.ends";
    // Removed even when `vout` panics.
    struct Scratch(std::path::PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let _scratch = Scratch(dir.clone());
    let v_pex = vout(&dir, "pex.spice", &bench(&pex));
    let v_sch = vout(&dir, "sch.spice", &bench(sch));
    assert!((v_pex - v_sch).abs() <= 0.01 * v_sch.abs(), "post-layout {v_pex} V vs schematic {v_sch} V\n{pex}");
}

/// The extractor reports no BJT card: bjt_mirror's post-layout netlist would
/// hold no device, so it is refused, naming a BJT.
#[test]
fn post_layout_refuses_a_device_it_does_not_extract() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let pdk = verify::Pdk::from_json(&std::fs::read_to_string(root.join("pdks/sky130.json")).unwrap()).unwrap();
    let spice = std::fs::read_to_string(root.join("benchmarks/fixtures/bjt_mirror.spice")).unwrap();
    let c = library::Config { feedback_iters: 1, starts: 1, ..library::Config::default() };
    let sol = library::run(&spice, &pdk, &library::Macros::default(), &c).expect("flow");
    let e = library::post_layout_spice(&sol, &pdk, "bjt_mirror").expect_err("BJTs are not extracted");
    assert!(e.contains("Npn") || e.contains("Pnp"), "{e}");
}

#[test]
fn post_layout_ports_are_the_schematic_ports() {
    if models().is_none() {
        return;
    }
    let (_, pex) = rc_filter_pex();
    let subckt = pex.lines().find(|l| l.starts_with(".subckt")).unwrap();
    assert_eq!(subckt, ".subckt rc_filter vin vout VDD VSS");
}

/// FR-7: every MOS keeps its bulk, on the rail of its type.
#[test]
fn every_mos_card_has_four_nodes() {
    if models().is_none() {
        return;
    }
    let (_, pex) = rc_filter_pex();
    // The model is the positional token naming a FET, wherever it sits: a
    // 3-node card puts it at t[4].
    let model = |s: &&str| !s.contains('=') && s.contains("fet_");
    let mos: Vec<Vec<&str>> =
        pex.lines().map(|l| l.split_whitespace().collect::<Vec<_>>()).filter(|t| t[0].starts_with('X') && t.iter().any(model)).collect();
    assert!(!mos.is_empty(), "{pex}");
    for t in mos {
        assert_eq!(t.iter().position(|s| model(&s)), Some(5), "four nodes, then the model: {t:?}");
        let bulk = t[4].rsplit_once(':').map_or(t[4], |(n, _)| n);
        let rail = if t[5].contains("nfet") { "VSS" } else { "VDD" };
        assert!(bulk.eq_ignore_ascii_case(rail), "{t:?}");
    }
}
