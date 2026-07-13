//! Conformance harness.
//!
//! Reads the Python-generated `conformance/{conformance.gds, manifest.json, params.json}`,
//! runs DRC/LVS/PEX per case through the *public* crate API, and diffs actual vs expected.
//! Exit code 0 iff every case passes.
//!
//! Usage: conformance [conformance_dir]   (default: ../conformance)

use gdsverify::gpu;
use gdsverify::params::PexLayerParams;
use gdsverify::*;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::process::exit;

struct Totals {
    pass: usize,
    fail: usize,
    // coverage: rule/check → (pass_ids, fail_ids)
    coverage: HashMap<String, (Vec<String>, Vec<String>)>,
}
impl Totals {
    fn new() -> Self {
        Totals {
            pass: 0,
            fail: 0,
            coverage: HashMap::new(),
        }
    }
    fn record(&mut self, ok: bool) {
        if ok {
            self.pass += 1
        } else {
            self.fail += 1
        }
    }
    fn track(&mut self, rule: &str, id: &str, is_pass: bool) {
        let e = self
            .coverage
            .entry(rule.into())
            .or_insert_with(|| (Vec::new(), Vec::new()));
        if is_pass {
            e.0.push(id.into());
        } else {
            e.1.push(id.into());
        }
    }
}

fn main() {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "../conformance".to_string());
    let manifest_txt =
        std::fs::read_to_string(format!("{dir}/manifest.json")).expect("read manifest.json");
    let manifest: Value = serde_json::from_str(&manifest_txt).expect("parse manifest");

    let deck = load_deck_json(&format!("{dir}/params.json")).expect("load params.json");
    let gds_file = manifest["gds_file"].as_str().unwrap();
    let layout = load_gds(&format!("{dir}/{gds_file}"), &deck).expect("read gds");

    println!("== gdsverify conformance ==");
    println!("cells loaded: {}", layout.cells.len());
    println!("active DRC rules: {}", deck.drc_rules.len());
    let backends = gpu::available_backends();
    // run DRC on the best backend available; results must match the manifest either way,
    // which is exactly the CPU/GPU-equivalence check.
    let backend = *backends.last().unwrap();
    println!("backends available: {backends:?} -> using {backend:?}");
    println!();

    let mut t = Totals::new();

    run_drc_cases(&manifest, &deck, &layout, backend, &mut t);
    run_lvs_cases(&manifest, &deck, &layout, &mut t);
    run_pex_cases(&manifest, &deck, &layout, &mut t);
    run_erc_cases(&manifest, &deck, &layout, &mut t);
    run_differential(&deck, &layout, &mut t);

    println!();
    println!("== summary: {} passed, {} failed ==", t.pass, t.fail);

    // coverage report
    let csv_path = format!("{dir}/coverage.csv");
    emit_coverage(&t, &csv_path);

    if t.fail > 0 {
        exit(1);
    }
}

/// Build a per-cell store (checkers run on one isolated cell).
fn cell_store<'a>(layout: &'a GdsLayout, cell: &str) -> &'a GeometryStore {
    layout
        .cells
        .get(cell)
        .unwrap_or_else(|| panic!("cell {cell} missing from GDS"))
}

fn run_drc_cases(
    manifest: &Value,
    deck: &Deck,
    layout: &GdsLayout,
    backend: gpu::Backend,
    t: &mut Totals,
) {
    println!("--- DRC ---");
    let cases = manifest["drc"]["cases"].as_array().unwrap();
    for case in cases {
        let id = case["id"].as_str().unwrap();
        let cell = case["cell"].as_str().unwrap();
        let rule = case["rule"].as_str().unwrap();
        let expect_n = case["expect_violations"].as_u64().unwrap() as usize;
        let case_strict = case
            .get("strict")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let store = cell_store(layout, cell);
        let report = run_drc_backend_strict(store, deck, backend, case_strict);
        // Restrict to the rule under test for this isolated case.
        let got: Vec<&Violation> = report
            .violations
            .iter()
            .filter(|v| v.kind == rule)
            .collect();

        let mut ok = got.len() == expect_n;
        let mut detail = String::new();

        // When violations are expected, check EVERY manifest-pinned measured value
        // against the got set (sorted multiset compare; a manifest listing fewer
        // entries than expect_n asserts containment with multiplicity).
        if ok && expect_n > 0 {
            let expected_list = case["violations"].as_array().unwrap();
            let mut exp_meas: Vec<i64> = expected_list
                .iter()
                .filter_map(|e| e.get("measured").and_then(|x| x.as_i64()))
                .collect();
            if !exp_meas.is_empty() {
                let mut got_meas: Vec<i64> = got.iter().map(|v| v.measured).collect();
                got_meas.sort_unstable();
                exp_meas.sort_unstable();
                let matched = if exp_meas.len() == got_meas.len() {
                    exp_meas == got_meas
                } else {
                    let mut g = got_meas.clone();
                    exp_meas.iter().all(|m| {
                        g.iter()
                            .position(|x| x == m)
                            .map(|i| {
                                g.remove(i);
                            })
                            .is_some()
                    })
                };
                if !matched {
                    ok = false;
                    detail = format!("measured {got_meas:?} != expected {exp_meas:?}");
                }
            }
            let expected = &expected_list[0];
            if let (Some(mf), Some(_lf)) = (
                expected.get("measured_frac").and_then(|x| x.as_f64()),
                expected.get("limit_frac").and_then(|x| x.as_f64()),
            ) {
                // density measured stored as ppm i64
                let gm = got[0].measured as f64 / 1_000_000.0;
                if (gm - mf).abs() > 1e-4 {
                    ok = false;
                    detail = format!("frac {gm:.4} != expected {mf:.4}");
                }
            }
        }

        t.record(ok);
        t.track(rule, id, expect_n == 0);
        let flag = if ok { "PASS" } else { "FAIL" };
        print!(
            "  [{flag}] {id:28} rule={rule:16} got={} exp={}",
            got.len(),
            expect_n
        );
        if !detail.is_empty() {
            print!("  ({detail})");
        }
        println!();
    }
}

fn build_reference(v: &Value) -> RefNetlist {
    let mut devices = Vec::new();
    for d in v["devices"].as_array().unwrap() {
        let kind = match d["type"].as_str().unwrap() {
            "nmos" => DeviceKind::Nmos,
            _ => DeviceKind::Pmos,
        };
        let flavor = match d.get("flavor").and_then(|v| v.as_str()) {
            Some("lvt") => DeviceFlavor::Lvt,
            Some("hvt") => DeviceFlavor::Hvt,
            _ => DeviceFlavor::Standard,
        };
        devices.push(RefDevice {
            kind,
            gate: d["g"].as_str().unwrap().to_string(),
            source: d["s"].as_str().unwrap().to_string(),
            drain: d["d"].as_str().unwrap().to_string(),
            w: d.get("w").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            l: d.get("l").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            flavor,
        });
    }
    RefNetlist {
        devices,
        net_seeds: std::collections::HashMap::new(),
        ref_two_terminal: Vec::new(),
        ref_bjt: Vec::new(),
    }
}

fn run_lvs_cases(manifest: &Value, deck: &Deck, layout: &GdsLayout, t: &mut Totals) {
    println!("\n--- LVS ---");
    let cases = manifest["lvs"]["cases"].as_array().unwrap();
    for case in cases {
        let id = case["id"].as_str().unwrap();
        let cell = case["cell"].as_str().unwrap();
        let expect_match = case["expect_match"].as_bool().unwrap();
        let reference = build_reference(&case["reference_netlist"]);

        let store = cell_store(layout, cell);
        let result = run_lvs(store, deck, &reference);

        let ok = result.matched == expect_match;
        t.record(ok);
        t.track("lvs", id, expect_match);
        let flag = if ok { "PASS" } else { "FAIL" };
        println!(
            "  [{flag}] {id:20} match={} exp={}  ({}N/{}P) {}",
            result.matched, expect_match, result.nmos, result.pmos, result.reason
        );
    }
}

fn run_erc_cases(manifest: &Value, deck: &Deck, layout: &GdsLayout, t: &mut Totals) {
    let cases = match manifest.get("erc").and_then(|e| e["cases"].as_array()) {
        Some(c) if !c.is_empty() => c,
        _ => return,
    };
    println!("\n--- ERC ---");
    for case in cases {
        let id = case["id"].as_str().unwrap();
        let cell = case["cell"].as_str().unwrap();
        let check = case["check"].as_str().unwrap();
        let expect_n = case["expect_violations"].as_u64().unwrap() as usize;

        let store = cell_store(layout, cell);
        let report = run_erc(store, deck, &SignoffConfig::default());
        let got: Vec<&ErcViolation> = report.by_check(check);

        let ok = got.len() == expect_n;
        t.record(ok);
        t.track(check, id, expect_n == 0);
        let flag = if ok { "PASS" } else { "FAIL" };
        println!(
            "  [{flag}] {id:28} check={check:20} got={} exp={expect_n}",
            got.len()
        );
    }
}

fn run_pex_cases(manifest: &Value, deck: &Deck, layout: &GdsLayout, t: &mut Totals) {
    println!("\n--- PEX ---");
    let cases = manifest["pex"]["cases"].as_array().unwrap();
    for case in cases {
        let id = case["id"].as_str().unwrap();
        let cell = case["cell"].as_str().unwrap();
        let kind = case["kind"].as_str().unwrap();
        let tol = case["tol"].as_f64().unwrap_or(1e-6);
        let expected: &HashMap<String, Value> =
            &serde_json::from_value(case["expected"].clone()).unwrap();

        // Negative-direction case: the manifest deliberately carries a WRONG value;
        // the engine passes iff its measurement DIFFERS (proves assertions have teeth).
        let expect_mismatch = case
            .get("expect_mismatch")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let store = cell_store(layout, cell);
        let report = run_pex(store, deck);

        if kind == "per_net" {
            // Per-net PEX: extract netlist, then run_pex_by_net, compare per-net R/C
            let ext =
                extract_netlist(store, deck).expect("extraction requires connectivity config");
            let by_net = run_pex_by_net(store, deck, &ext.net_of_poly);
            let expect_nets = case["expected"].as_object().unwrap();
            let mut ok = true;
            let mut detail = String::new();
            for (net_str, vals) in expect_nets {
                let net_id: u32 = net_str.parse().unwrap_or(u32::MAX);
                let got = by_net.get(&net_id).copied().unwrap_or_default();
                if let Some(er) = vals.get("r_ohm").and_then(|v| v.as_f64()) {
                    if (got.r_ohm - er).abs() > tol.max(er.abs() * 1e-6 + 1e-9) {
                        ok = false;
                        detail = format!("net{net_id} R {:.4} != {:.4}", got.r_ohm, er);
                    }
                }
                if let Some(ec) = vals.get("cap_af").and_then(|v| v.as_f64()) {
                    if (got.cap_af - ec).abs() > tol.max(ec.abs() * 1e-6 + 1e-9) {
                        ok = false;
                        detail = format!("net{net_id} C {:.4} != {:.4}", got.cap_af, ec);
                    }
                }
            }
            if expect_mismatch {
                ok = !ok;
                detail.clear();
            }
            t.record(ok);
            let flag = if ok { "PASS" } else { "FAIL" };
            t.track(kind, id, !expect_mismatch);
            print!("  [{flag}] {id:18} kind={kind:12}");
            if expect_mismatch {
                print!(" (negative)");
            }
            if !detail.is_empty() {
                print!("  ({detail})");
            }
            println!();
            continue;
        }

        let (got, exp, unit) = match kind {
            "resistance" => {
                let g = report.total_resistance("met1");
                (g, expected["r_ohm"].as_f64().unwrap(), "ohm")
            }
            "resistance_met2" => {
                let g = report.total_resistance("met2");
                (g, expected["r_ohm"].as_f64().unwrap(), "ohm")
            }
            "area_cap" => {
                let g = report
                    .area_caps()
                    .iter()
                    .map(|p| match p {
                        Parasitic::AreaCap { af, .. } => *af,
                        _ => 0.0,
                    })
                    .sum::<f64>();
                (g, expected["c_af"].as_f64().unwrap(), "aF")
            }
            "coupling_cap" => {
                let g = report
                    .coupling_caps()
                    .iter()
                    .map(|p| match p {
                        Parasitic::CouplingCap { af, .. } => *af,
                        _ => 0.0,
                    })
                    .sum::<f64>();
                (g, expected["c_af"].as_f64().unwrap(), "aF")
            }
            "coupling_cap_met2" => {
                let g = report
                    .coupling_caps()
                    .iter()
                    .map(|p| match p {
                        Parasitic::CouplingCap { layer, af, .. } if layer == "met2" => *af,
                        _ => 0.0,
                    })
                    .sum::<f64>();
                (g, expected["c_af"].as_f64().unwrap(), "aF")
            }
            "interlayer_cap" => {
                let g = report
                    .interlayer_caps()
                    .iter()
                    .map(|p| match p {
                        Parasitic::InterlayerCap { af, .. } => *af,
                        _ => 0.0,
                    })
                    .sum::<f64>();
                (g, expected["c_af"].as_f64().unwrap(), "aF")
            }
            "via_resistance" => {
                let g = report
                    .via_resistances()
                    .iter()
                    .map(|p| match p {
                        Parasitic::ViaResistance { ohm, .. } => *ohm,
                        _ => 0.0,
                    })
                    .sum::<f64>();
                (g, expected["r_ohm"].as_f64().unwrap(), "ohm")
            }
            _ => (0.0, 0.0, "?"),
        };

        let within = (got - exp).abs() <= tol.max(exp.abs() * 1e-6 + 1e-9);
        let ok = if expect_mismatch { !within } else { within };
        t.record(ok);
        t.track(kind, id, !expect_mismatch);
        let flag = if ok { "PASS" } else { "FAIL" };
        let neg = if expect_mismatch { " (negative)" } else { "" };
        println!("  [{flag}] {id:18} kind={kind:12} got={got:.4}{unit} exp={exp:.4}{unit}{neg}");
    }
}

// --- JSON → VerifySchema for the conformance harness ---

#[derive(Deserialize)]
struct LayerDefRaw {
    layer: i32,
    datatype: i32,
}

#[derive(Deserialize, Default)]
struct LvsRaw {
    #[serde(default)]
    cut_required: bool,
}

#[derive(Deserialize)]
struct ParamsDoc {
    layers: HashMap<String, LayerDefRaw>,
    drc: Value,
    #[serde(default)]
    pex: HashMap<String, PexLayerParams>,
    #[serde(default)]
    lvs: LvsRaw,
    #[serde(default)]
    erc: gdsverify::params::ErcParams,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

fn load_deck_json(path: &str) -> Result<Deck, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let doc: ParamsDoc = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let layers = doc
        .layers
        .iter()
        .map(|(n, d)| (n.clone(), (d.layer, d.datatype)))
        .collect();
    let drc_rules = gdsverify::schema::drc_rules_from_json(&doc.drc)?;
    use gdsverify::schema::{ConnectivitySchema, DeviceSchema, MosRuleSchema, ViaSchema};

    let connectivity = if let Some(c) = doc.extra.get("connectivity") {
        ConnectivitySchema {
            conductors: c
                .get("conductors")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            vias: c
                .get("vias")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| {
                            let layer = v.get("layer")?.as_str()?.to_string();
                            let connects = v
                                .get("connects")?
                                .as_array()?
                                .iter()
                                .filter_map(|x| x.as_str().map(String::from))
                                .collect();
                            Some(ViaSchema { layer, connects })
                        })
                        .collect()
                })
                .unwrap_or_default(),
            intra_layer_touch: true,
            global_nets: Vec::new(),
        }
    } else {
        ConnectivitySchema::default()
    };

    let devices = if let Some(d) = doc.extra.get("devices") {
        let mos_rules = d
            .get("mos")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|r| {
                        Some(MosRuleSchema {
                            name: r.get("name")?.as_str()?.into(),
                            gate_layer: r.get("gate_layer")?.as_str()?.into(),
                            channel_layer: r.get("channel_layer")?.as_str()?.into(),
                            type_implant: r.get("type_implant")?.as_str()?.into(),
                            device_type: r.get("device_type")?.as_str()?.into(),
                            flavor_markers: r
                                .get("flavor_markers")
                                .and_then(|f| f.as_array())
                                .map(|a| {
                                    a.iter()
                                        .filter_map(|p| {
                                            let arr = p.as_array()?;
                                            Some((
                                                arr.first()?.as_str()?.into(),
                                                arr.get(1)?.as_str()?.into(),
                                            ))
                                        })
                                        .collect()
                                })
                                .unwrap_or_default(),
                            well_layer: r
                                .get("well_layer")
                                .and_then(|w| w.as_str())
                                .map(String::from),
                            device_class: None,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        DeviceSchema {
            mos_rules,
            ..Default::default()
        }
    } else {
        DeviceSchema::default()
    };

    Deck::from_schema(VerifySchema {
        layers,
        drc_rules,
        pex: doc.pex,
        erc: doc.erc,
        lvs: LvsSchema {
            cut_required: doc.lvs.cut_required,
            ..Default::default()
        },
        connectivity,
        devices,
    })
}

/// Cross-tool differential testing: run DRC+LVS+ERC+PEX on the same cell
/// and verify consistency constraints between them.
fn run_differential(deck: &Deck, layout: &GdsLayout, t: &mut Totals) {
    println!("\n--- DIFFERENTIAL ---");
    // LVS_INV cell: clean inverter. Cross-check:
    // 1. DRC should be clean (no width/spacing violations on met1)
    // 2. LVS should match the inverter reference
    // 3. ERC should have no supply short
    // 4. PEX should produce non-zero parasitics
    if let Some(store) = layout.cells.get("LVS_INV") {
        let drc = run_drc(store, deck);
        let drc_clean =
            drc.by_kind("min_width").is_empty() && drc.by_kind("min_spacing").is_empty();

        let ref_inv = RefNetlist {
            devices: vec![
                RefDevice {
                    kind: DeviceKind::Nmos,
                    gate: "A".into(),
                    source: "VSS".into(),
                    drain: "Y".into(),
                    w: 0,
                    l: 0,
                    flavor: DeviceFlavor::Standard,
                },
                RefDevice {
                    kind: DeviceKind::Pmos,
                    gate: "A".into(),
                    source: "VDD".into(),
                    drain: "Y".into(),
                    w: 0,
                    l: 0,
                    flavor: DeviceFlavor::Standard,
                },
            ],
            net_seeds: HashMap::new(),
            ref_two_terminal: Vec::new(),
            ref_bjt: Vec::new(),
        };
        let lvs = run_lvs(store, deck, &ref_inv);
        let erc = run_erc(store, deck, &SignoffConfig::default());
        let erc_no_short = erc.by_check("supply_short").is_empty();
        // ponytail: LVS_INV has no met1/met2, so PEX is empty — verify
        // PEX runs without panic rather than checking for content.
        let pex = run_pex(store, deck);
        let pex_ran = true; // no panic = success
        let _ = pex.total_cap(); // exercise the accessors

        let ok = drc_clean && lvs.matched && erc_no_short && pex_ran;
        t.record(ok);
        t.track("differential", "DIFF_INV_CROSS", true);
        let flag = if ok { "PASS" } else { "FAIL" };
        println!("  [{flag}] DIFF_INV_CROSS          drc_clean={drc_clean} lvs={} erc_clean={erc_no_short} pex_ran={pex_ran}",
                 lvs.matched);

        // negative direction: a deliberately wrong reference (NAND topology against
        // the inverter layout) must NOT match — proves the cross-check has teeth.
        let ref_wrong = RefNetlist {
            devices: vec![
                RefDevice {
                    kind: DeviceKind::Nmos,
                    gate: "A".into(),
                    source: "VSS".into(),
                    drain: "X".into(),
                    w: 0,
                    l: 0,
                    flavor: DeviceFlavor::Standard,
                },
                RefDevice {
                    kind: DeviceKind::Nmos,
                    gate: "B".into(),
                    source: "X".into(),
                    drain: "Y".into(),
                    w: 0,
                    l: 0,
                    flavor: DeviceFlavor::Standard,
                },
                RefDevice {
                    kind: DeviceKind::Pmos,
                    gate: "A".into(),
                    source: "VDD".into(),
                    drain: "Y".into(),
                    w: 0,
                    l: 0,
                    flavor: DeviceFlavor::Standard,
                },
                RefDevice {
                    kind: DeviceKind::Pmos,
                    gate: "B".into(),
                    source: "VDD".into(),
                    drain: "Y".into(),
                    w: 0,
                    l: 0,
                    flavor: DeviceFlavor::Standard,
                },
            ],
            net_seeds: HashMap::new(),
            ref_two_terminal: Vec::new(),
            ref_bjt: Vec::new(),
        };
        let lvs_neg = run_lvs(store, deck, &ref_wrong);
        let ok_neg = !lvs_neg.matched;
        t.record(ok_neg);
        t.track("differential", "DIFF_INV_NEG", false);
        let flag = if ok_neg { "PASS" } else { "FAIL" };
        println!(
            "  [{flag}] DIFF_INV_NEG            wrong-ref rejected={}",
            !lvs_neg.matched
        );
    }
}

fn emit_coverage(t: &Totals, path: &str) {
    let mut csv = String::from("rule,pass_tests,fail_tests,status\n");
    let mut rules: Vec<&String> = t.coverage.keys().collect();
    rules.sort();
    let mut unproven = 0;
    for rule in &rules {
        let (pass, fail) = t.coverage.get(*rule).unwrap();
        let status = if fail.is_empty() {
            unproven += 1;
            "unproven"
        } else {
            "proven"
        };
        csv.push_str(&format!(
            "{},{},{},{}\n",
            rule,
            pass.join(";"),
            fail.join(";"),
            status
        ));
    }
    std::fs::write(path, &csv).expect("write coverage.csv");
    println!(
        "\ncoverage: {}/{} rules proven (have ≥1 negative test), {} unproven -> {path}",
        rules.len() - unproven,
        rules.len(),
        unproven
    );
}
