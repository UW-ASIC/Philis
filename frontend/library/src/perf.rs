//! Post-layout performance: the circuit simulated with its extracted
//! capacitance, measured against its specifications.
//!
//! Total extracted C says nothing about which node carries it; two layouts with
//! equal totals can differ in phase margin or offset. So an epoch is scored by
//! the metrics a testbench measures on the schematic plus the extracted
//! capacitance matrix (ground and coupling, per net), and a failed spec makes
//! the layout infeasible, whatever it saves elsewhere.

use std::process::Command;

use pnr_core::Netlist;

use crate::oppoint::{check_exit, flat_circuit_with, node_name, OpConfig};

/// What a layout adds to the schematic for simulation.
#[derive(Clone, Debug, Default)]
pub struct Parasitics {
    /// Extracted capacitance matrix (ground and coupling, per net).
    pub caps: verify::CapMatrix,
    /// Per device, each terminal's routed branch R, Ω (`(terminal, Ω)`; a
    /// terminal absent here connects straight to its net).
    pub series: Vec<Vec<(String, f32)>>,
    /// Per device, the layout's mean LOD stress term `1/(SA+L/2) +
    /// 1/(SB+L/2)` over its fingers, 1/µm; `None` = not drawn / unknown.
    pub lod_inv_um: Vec<Option<f32>>,
    /// Built from a drawn layout (its capacitor plates are in `caps`): schematic
    /// capacitor cards are left out so they are not counted twice.
    pub extracted: bool,
}

/// `SA = SB = S` (µm) at which BSIM4's multi-finger average `(1/nf)·Σᵢ
/// 1/(S + L/2 + i·L)` (SD = 0) gives `target/2`: an equivalent card for the
/// layout's measured stress. `None` when no finite `S` reaches it.
fn equivalent_sa_um(target_inv_um: f64, l_um: f64, nf: i64) -> Option<f64> {
    let f = |s: f64| (0..nf.max(1)).map(|i| 1.0 / (s + l_um / 2.0 + i as f64 * l_um)).sum::<f64>() / nf.max(1) as f64;
    let want = target_inv_um / 2.0;
    let (mut lo, mut hi) = (1e-3, 1e4);
    if !(f(hi)..=f(lo)).contains(&want) {
        return None;
    }
    for _ in 0..80 {
        let mid = (lo * hi).sqrt();
        if f(mid) > want {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some((lo * hi).sqrt())
}

/// One requirement on a measured metric (a `.measure` name).
#[derive(Clone, Debug)]
pub struct Spec {
    pub metric: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

/// How to score a layout electrically.
#[derive(Clone, Debug)]
pub struct PerfConfig {
    /// Models, corner and ngspice binary (the circuit card format is shared
    /// with the operating point).
    pub sim: OpConfig,
    /// Sources, loads, analyses and `.measure` statements over the schematic's
    /// net names. Must not contain `.end`.
    pub testbench: String,
    pub specs: Vec<Spec>,
}

/// The measured metrics of one layout and how far they miss their specs.
#[derive(Clone, Debug, Default)]
pub struct PerfResult {
    /// Every spec's metric, `None` when the testbench did not produce it.
    pub metrics: Vec<(String, Option<f64>)>,
    /// Σ normalised miss over the specs (`0` = all met). A metric that could
    /// not be measured counts a full `1.0`: unknown never passes.
    pub residual: f64,
}

impl PerfResult {
    /// Every spec met and measured.
    #[must_use]
    pub fn met(&self) -> bool {
        self.residual <= 0.0
    }
}

/// Normalised miss of `v` against `spec`: the overshoot past the violated
/// bound over that bound's magnitude (`1` when the bound is zero).
#[must_use]
pub fn miss(spec: &Spec, v: Option<f64>) -> f64 {
    let Some(v) = v else { return 1.0 };
    let over = |excess: f64, bound: f64| (excess / if bound == 0.0 { 1.0 } else { bound.abs() }).max(0.0);
    spec.min.map_or(0.0, |lo| over(lo - v, lo)) + spec.max.map_or(0.0, |hi| over(v - hi, hi))
}

/// The deck: models, flat circuit, one capacitor per extracted matrix entry,
/// the testbench.
///
/// # Errors
/// A device the circuit cannot express (`flat_circuit_with`).
fn deck(netlist: &Netlist, par: &Parasitics, cfg: &PerfConfig) -> Result<String, String> {
    let caps = &par.caps;
    let node = |name: &str| {
        netlist
            .nets
            .iter()
            .position(|n| n.name == name)
            .map(|i| node_name(netlist, pnr_core::NetId(i as u16)))
    };
    let mut pex = String::new();
    for (i, (a, b, c)) in caps.iter().enumerate() {
        let (Some(a), Some(b)) = (node(a), b.as_deref().map_or(Some("0".to_string()), node)) else { continue };
        if a != b && *c > 0.0 {
            pex.push_str(&format!("Cpex{i} {a} {b} {c:.6e}f\n"));
        }
    }
    let lib = cfg.sim.lib_lines();
    // Branch resistors: a terminal with routed R gets its own node,
    // `<net>__<device>_<terminal>`, joined to the net through it.
    let branch = |di: usize, t: &str| par.series.get(di).and_then(|v| v.iter().find(|(n, _)| n == t)).map(|&(_, r)| r).filter(|&r| r > 0.0);
    let mut rs = String::new();
    for (di, dev) in netlist.devices.iter().enumerate() {
        for (t, id) in &dev.terminals {
            if let Some(r) = branch(di, t) {
                let net = node_name(netlist, *id);
                rs.push_str(&format!("Rpex_{di}_{t} {net}__{di}_{t} {net} {r:.4}\n"));
            }
        }
    }
    let circuit = flat_circuit_with(
        netlist,
        &cfg.sim,
        |di, t, n| if branch(di, t).is_some() { format!("{n}__{di}_{t}") } else { n },
        |di| {
            let Some(sz) = netlist.devices[di].mos_size() else { return String::new() };
            par.lod_inv_um
                .get(di)
                .copied()
                .flatten()
                .and_then(|t| equivalent_sa_um(f64::from(t), sz.l_nm as f64 / 1e3, i64::from(sz.nf)))
                .map_or(String::new(), |s| format!(" sa={:.4e} sb={:.4e}", s * 1e-6, s * 1e-6))
        },
        !par.extracted,
    )?;
    Ok(format!(
        "* Philis post-layout performance (generated)\n{lib}{circuit}\n* extracted capacitance\n{pex}\n* routed branch resistance\n{rs}\n{}\n.end\n",
        cfg.testbench
    ))
}

/// `name = value` lines ngspice prints for `.measure` results.
fn parse_measures(text: &str) -> Vec<(String, f64)> {
    text.lines()
        .filter_map(|l| {
            let (k, v) = l.split_once('=')?;
            let k = k.trim().to_ascii_lowercase();
            let v: f64 = v.split_whitespace().next()?.parse().ok()?;
            (!k.is_empty() && !k.contains(char::is_whitespace)).then_some((k, v))
        })
        .collect()
}

/// Simulate and score. `Err` when the circuit cannot be simulated at all; a
/// failed measurement is a `None` metric (and a full miss), not an error.
///
/// # Errors
/// The deck cannot be built ("cannot simulate: …") or written, ngspice cannot
/// be started, or it exits with an error ("cannot simulate: ngspice exit …").
pub fn evaluate(netlist: &Netlist, par: &Parasitics, cfg: &PerfConfig) -> Result<PerfResult, String> {
    static RUN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!("philis_perf_{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| format!("deck io: {e}"))?;
    // One file per run: sensitivity runs go in parallel.
    let path = dir.join(format!("perf{}.spice", RUN.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
    let text = deck(netlist, par, cfg).map_err(|e| format!("cannot simulate: {e}"))?;
    std::fs::write(&path, text).map_err(|e| format!("deck io: {e}"))?;
    let out = Command::new(&cfg.sim.ngspice).arg("-b").arg(&path).output().map_err(|e| format!("ngspice unavailable: {e}"))?;
    check_exit(&out)?;
    let measured = parse_measures(&String::from_utf8_lossy(&out.stdout));
    let metrics: Vec<(String, Option<f64>)> = cfg
        .specs
        .iter()
        .map(|s| {
            let key = s.metric.to_ascii_lowercase();
            (s.metric.clone(), measured.iter().rev().find(|(k, _)| *k == key).map(|&(_, v)| v))
        })
        .collect();
    let residual = cfg.specs.iter().zip(&metrics).map(|(s, (_, v))| miss(s, *v)).sum();
    Ok(PerfResult { metrics, residual })
}

/// Finite-difference sensitivities at the schematic operating point: the
/// baseline (no parasitics), and `∂f/∂C` per spec per net from one run with
/// `delta_af` of ground capacitance added to that net alone. Runs in parallel.
pub struct Sensitivity {
    pub base: PerfResult,
    /// `[spec][net]`, metric units per aF; `None` where a run did not measure.
    pub d_per_af: Vec<Vec<Option<f64>>>,
}

/// # Errors
/// As [`evaluate`], for the baseline run.
pub fn sensitivities(netlist: &Netlist, cfg: &PerfConfig, nets: &[String], delta_af: f64) -> Result<Sensitivity, String> {
    let base = evaluate(netlist, &Parasitics::default(), cfg)?;
    let runs: Vec<Option<PerfResult>> = std::thread::scope(|scope| {
        let handles: Vec<_> = nets
            .iter()
            .map(|n| {
                let caps = vec![(n.clone(), None, delta_af / 1000.0)];
                scope.spawn(move || evaluate(netlist, &Parasitics { caps, ..Parasitics::default() }, cfg).ok())
            })
            .collect();
        handles.into_iter().map(|h| h.join().ok().flatten()).collect()
    });
    let d_per_af = (0..cfg.specs.len())
        .map(|j| {
            runs.iter()
                .map(|r| {
                    let (f0, f1) = (base.metrics[j].1?, r.as_ref()?.metrics[j].1?);
                    Some((f1 - f0) / delta_af)
                })
                .collect()
        })
        .collect();
    Ok(Sensitivity { base, d_per_af })
}

/// One [`analog::routing::PerformanceBudget`] row per finite bound, metric
/// `"{metric}:min"` / `"{metric}:max"`: `w_i = sign·(∂f/∂C_i) / headroom`,
/// `limit = 1`, with floor headroom `f0 − lo` (`sign = −1`) and ceiling `hi −
/// f0` (`sign = +1`). A bound the schematic already misses (`headroom ≤ 0`)
/// keeps a do-not-worsen row: `limit = 0`, `w_i = sign·(∂f/∂C_i) / |bound|`
/// (`miss`'s unit-free scale, `1` for a zero bound). A spec the schematic does
/// not measure finitely, or a non-finite bound, has no row (reported by the
/// caller); a net whose run did not measure is left out of the row.
#[must_use]
pub fn budget_rows(
    cfg: &PerfConfig,
    s: &Sensitivity,
    nets: &[pnr_core::NetId],
    af_per_nm: f32,
) -> Vec<analog::routing::PerformanceBudget> {
    cfg.specs
        .iter()
        .enumerate()
        .flat_map(|(j, spec)| {
            let f0 = s.base.metrics[j].1;
            [(spec.min, -1.0, "min"), (spec.max, 1.0, "max")].into_iter().filter_map(move |(bound, sign, side)| {
                let (f0, bound) = (f0.filter(|v| v.is_finite())?, bound.filter(|v| v.is_finite())?);
                let headroom = sign * (bound - f0);
                let (scale, limit) = if headroom > 0.0 { (headroom, 1.0) } else { (if bound == 0.0 { 1.0 } else { bound.abs() }, 0.0) };
                let (nets, weights): (Vec<_>, Vec<_>) = nets
                    .iter()
                    .zip(&s.d_per_af[j])
                    .filter_map(|(&n, d)| Some((n, (sign * d.as_ref()? / scale) as f32)))
                    .unzip();
                Some(analog::routing::PerformanceBudget { metric: format!("{}:{side}", spec.metric), nets, weights, af_per_nm, limit })
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(min: Option<f64>, max: Option<f64>) -> Spec {
        Spec { metric: "m".into(), min, max }
    }

    #[test]
    fn a_miss_is_normalised_by_its_bound_and_unknown_is_a_full_miss() {
        let pm = spec(Some(60.0), None);
        assert_eq!(miss(&pm, Some(66.0)), 0.0);
        assert!((miss(&pm, Some(54.0)) - 0.1).abs() < 1e-12, "6° short of 60°");
        let power = spec(None, Some(100.0));
        assert!((miss(&power, Some(150.0)) - 0.5).abs() < 1e-12);
        assert_eq!(miss(&pm, None), 1.0, "unmeasured never passes");
        assert_eq!(miss(&spec(None, Some(0.0)), Some(0.25)), 0.25, "a zero bound uses unit scale");
    }

    #[test]
    fn measures_parse_from_ngspice_batch_output() {
        let out = "Circuit: x\n\nugf                 =  1.234560e+07 at=  1.0e+00\npm = 6.1e+01\nNote: blah = no\n";
        let m = parse_measures(out);
        assert!(m.contains(&("ugf".into(), 1.23456e7)));
        assert!(m.contains(&("pm".into(), 61.0)));
    }

    #[test]
    fn the_deck_carries_every_matrix_entry_on_schematic_nodes() {
        use pnr_core::{Device, DeviceKind, Net, NetId};
        let nl = Netlist {
            devices: vec![Device {
                name: "M1".into(),
                kind: DeviceKind::Nmos, model: String::new(),
                terminals: vec![("D".into(), NetId(0)), ("G".into(), NetId(1)), ("S".into(), NetId(2)), ("B".into(), NetId(2))],
                params: vec![("w".into(), 1000), ("l".into(), 150)],
            }],
            nets: ["out", "in", "vss"].iter().map(|n| Net { name: (*n).into() }).collect(),
            ..Default::default()
        };
        let caps = vec![("out".to_string(), None, 2.5), ("in".to_string(), Some("out".to_string()), 0.4), ("ghost".to_string(), None, 9.0)];
        let cfg = PerfConfig { sim: OpConfig::default(), testbench: ".measure tran x avg v(out)".into(), specs: vec![] };
        let d = deck(&nl, &Parasitics { caps, ..Parasitics::default() }, &cfg).unwrap();
        assert!(d.contains("out 0 2.500000e0f"), "{d}");
        assert!(d.contains("in out 4.000000e-1f"), "{d}");
        assert!(!d.contains("ghost"), "a net the schematic lacks has no node");
        assert!(d.trim_end().ends_with(".end"));
    }

    /// Branch R gives the terminal its own node through a resistor; the
    /// layout's stress term becomes an equivalent `sa`/`sb` that reproduces
    /// it under BSIM4's multi-finger average.
    #[test]
    fn the_deck_carries_branch_resistance_and_layout_stress() {
        use pnr_core::{Device, DeviceKind, Net, NetId};
        let nl = Netlist {
            devices: vec![Device {
                name: "M1".into(),
                kind: DeviceKind::Nmos, model: "sky130_fd_pr__nfet_01v8".into(),
                terminals: vec![("D".into(), NetId(0)), ("G".into(), NetId(1)), ("S".into(), NetId(2)), ("B".into(), NetId(2))],
                params: vec![("w".into(), 1000), ("l".into(), 500), ("nf".into(), 2)],
            }],
            nets: ["out", "in", "vss"].iter().map(|n| Net { name: (*n).into() }).collect(),
            ..Default::default()
        };
        let par = Parasitics { caps: Vec::new(), series: vec![vec![("D".into(), 12.5)]], lod_inv_um: vec![Some(1.0)], extracted: false };
        let cfg = PerfConfig { sim: OpConfig::default(), testbench: String::new(), specs: vec![] };
        let d = deck(&nl, &par, &cfg).unwrap();
        assert!(d.contains("Rpex_0_D out__0_D out 12.5000"), "{d}");
        assert!(d.contains("XM1 out__0_D in"), "{d}");
        // SA = SB = S with (1/2)(1/(S+0.25) + 1/(S+0.75)) = 0.5.
        let s = equivalent_sa_um(1.0, 0.5, 2).unwrap();
        assert!(((1.0 / (s + 0.25) + 1.0 / (s + 0.75)) / 2.0 - 0.5).abs() < 1e-9, "{s}");
        assert!(d.contains(" sa="), "{d}");
    }

    #[test]
    fn rows_turn_sensitivities_into_shares_of_the_headroom() {
        let cfg = PerfConfig {
            sim: OpConfig::default(),
            testbench: String::new(),
            specs: vec![spec(Some(300e6), None), spec(None, Some(1.0)), spec(Some(10.0), None)],
        };
        let s = Sensitivity {
            // UGF 400 MHz (100 MHz headroom); power 0.5 (0.5 headroom); gain 5 (already short).
            base: PerfResult { metrics: vec![("a".into(), Some(400e6)), ("b".into(), Some(0.5)), ("c".into(), Some(5.0))], residual: 0.0 },
            // UGF drops 1 MHz per aF on net 0, not at all on net 1.
            d_per_af: vec![vec![Some(-1e6), Some(0.0)], vec![Some(0.01), None], vec![Some(0.0), Some(0.0)]],
        };
        let rows = budget_rows(&cfg, &s, &[pnr_core::NetId(0), pnr_core::NetId(1)], 0.1);
        assert_eq!(rows.len(), 3, "one row per bound, the missed one included");
        assert!((rows[0].weights[0] - 0.01).abs() < 1e-7, "1 MHz/aF of 100 MHz = 1% per aF");
        assert_eq!(rows[0].weights[1], 0.0);
        assert_eq!(rows[1].nets, vec![pnr_core::NetId(0)], "an unmeasured net is left out");
        assert!((rows[1].weights[0] - 0.02).abs() < 1e-7);
        assert_eq!((rows[2].metric.as_str(), rows[2].limit), ("m:min", 0.0), "the missed spec keeps a do-not-worsen row");
    }

    fn one_net(specs: Vec<Spec>, f0: f64, d: f64) -> Vec<analog::routing::PerformanceBudget> {
        let cfg = PerfConfig { sim: OpConfig::default(), testbench: String::new(), specs };
        let s = Sensitivity {
            base: PerfResult { metrics: vec![("m".into(), Some(f0))], residual: 0.0 },
            d_per_af: vec![vec![Some(d)]],
        };
        budget_rows(&cfg, &s, &[pnr_core::NetId(0)], 1.0)
    }

    /// A window spec is two features (GRAEB-04): the floor row and the
    /// ceiling row, each over its own headroom, with opposite signs.
    #[test]
    fn rows_cover_both_bounds_of_a_window_spec() {
        let rows = one_net(vec![spec(Some(10.0), Some(20.0))], 15.0, -1.0);
        assert_eq!(rows.len(), 2);
        assert_eq!((rows[0].metric.as_str(), rows[0].limit), ("m:min", 1.0));
        assert!((rows[0].weights[0] - 0.2).abs() < 1e-7, "{:?}", rows[0].weights);
        assert_eq!((rows[1].metric.as_str(), rows[1].limit), ("m:max", 1.0));
        assert!((rows[1].weights[0] + 0.2).abs() < 1e-7, "{:?}", rows[1].weights);
    }

    /// A floor the schematic already misses (GRAEB-11): a zero-limit row in
    /// units of the bound, so any adverse C is a residual.
    #[test]
    fn a_missed_bound_keeps_a_do_not_worsen_row() {
        let rows = one_net(vec![spec(Some(10.0), None)], 5.0, -1.0);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].limit, 0.0);
        assert!((rows[0].weights[0] - 0.1).abs() < 1e-7, "{:?}", rows[0].weights);
    }

    /// Only a finite bound against a finite schematic value gets a row: a NaN
    /// weight would never violate.
    #[test]
    fn a_non_finite_bound_or_value_has_no_row() {
        assert!(one_net(vec![spec(Some(f64::NAN), Some(f64::INFINITY))], 5.0, -1.0).is_empty());
        assert!(one_net(vec![spec(Some(10.0), None)], f64::NAN, -1.0).is_empty());
    }
}
