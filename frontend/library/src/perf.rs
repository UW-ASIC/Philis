//! Post-layout performance: the circuit simulated with its extracted
//! capacitance, measured against its specifications.
//!
//! Total extracted C says nothing about which node carries it; two layouts with
//! equal totals can differ in phase margin or offset. So an epoch is scored by
//! the metrics a testbench measures on the schematic plus the extracted
//! capacitance matrix (ground and coupling, per net), and a failed spec makes
//! the layout infeasible, whatever it saves elsewhere.

use pnr_core::Netlist;

use crate::oppoint::{flat_circuit_with, node_name, run_deck, OpConfig};

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

/// One process/environment point: model corner, temperature, and `.param`
/// values the testbenches read (e.g. `Vdd vdd 0 {vdd}`).
#[derive(Clone, Debug)]
pub struct Scenario {
    pub name: String,
    pub corner: String,
    pub temp_c: f64,
    pub params: Vec<(String, f64)>,
}

/// How to score a layout electrically.
#[derive(Clone, Debug)]
pub struct PerfConfig {
    /// Models, corner and ngspice binary (the circuit card format is shared
    /// with the operating point).
    pub sim: OpConfig,
    /// Each a complete bench (sources, loads, analyses, `.measure`) over the
    /// schematic's net names, no `.end`; a spec's metric comes from exactly
    /// one of them.
    pub testbenches: Vec<String>,
    pub specs: Vec<Spec>,
    /// Index 0 is nominal. Empty = one scenario from `sim.corner` /
    /// `sim.temp_c`.
    pub scenarios: Vec<Scenario>,
}

impl PerfConfig {
    /// `scenarios`, or the single one `sim` implies.
    #[must_use]
    pub fn scenarios(&self) -> Vec<Scenario> {
        if self.scenarios.is_empty() {
            vec![Scenario { name: self.sim.corner.clone(), corner: self.sim.corner.clone(), temp_c: self.sim.temp_c, params: Vec::new() }]
        } else {
            self.scenarios.clone()
        }
    }
}

/// A finite bound's worst value over the evaluated scenarios.
#[derive(Clone, Debug, PartialEq)]
pub struct BoundResult {
    pub spec: usize,
    /// The ceiling (`max`); `false` = the floor (`min`).
    pub upper: bool,
    /// `None` = some scenario did not measure it (that scenario is the worst).
    pub value: Option<f64>,
    /// Index into [`PerfConfig::scenarios`].
    pub scenario: usize,
}

/// The measured metrics of one layout and how far they miss their specs.
#[derive(Clone, Debug, Default)]
pub struct PerfResult {
    /// Every spec's metric at the first evaluated scenario (nominal in the
    /// flow), `None` when no testbench produced it there.
    pub metrics: Vec<(String, Option<f64>)>,
    /// One per finite bound, spec order, floor before ceiling.
    pub bounds: Vec<BoundResult>,
    /// Per spec: `1.0` if any scenario did not measure it, else the floor miss
    /// at the worst floor value plus the ceiling miss at the worst ceiling
    /// value (one scenario: [`miss`]).
    pub miss: Vec<f64>,
    /// Σ `miss` (`0` = all met). Unknown never passes.
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

/// Score `measured[i][j]` (spec `j` at `scenarios[i]`): per finite bound the
/// scenario with the smallest margin (`v − lo` / `hi − v`), the first
/// unmeasured one if any; ties go to the earlier scenario.
#[must_use]
pub fn score(specs: &[Spec], measured: &[Vec<Option<f64>>], scenarios: &[usize]) -> PerfResult {
    let metrics = specs.iter().enumerate().map(|(j, s)| (s.metric.clone(), measured.first().and_then(|m| m[j]))).collect();
    let mut bounds = Vec::new();
    let mut misses = Vec::new();
    for (j, s) in specs.iter().enumerate() {
        let col = || measured.iter().zip(scenarios).map(move |(m, &sc)| (m[j], sc));
        // The worst `(value, scenario)` for a bound: `margin` grows with slack.
        let worst = |margin: fn(f64) -> f64| {
            col().find(|(v, _)| v.is_none()).or_else(|| col().reduce(|a, b| if margin(b.0.unwrap()) < margin(a.0.unwrap()) { b } else { a }))
        };
        let lo = worst(|v| v);
        let hi = worst(|v| -v);
        for (bound, upper, w) in [(s.min, false, lo), (s.max, true, hi)] {
            if let (Some(_), Some((value, scenario))) = (bound.filter(|b| b.is_finite()), w) {
                bounds.push(BoundResult { spec: j, upper, value, scenario });
            }
        }
        let floor = Spec { max: None, ..s.clone() };
        let ceiling = Spec { min: None, ..s.clone() };
        misses.push(match (lo, hi) {
            (Some((Some(a), _)), Some((Some(b), _))) => miss(&floor, Some(a)) + miss(&ceiling, Some(b)),
            _ => 1.0,
        });
    }
    PerfResult { metrics, bounds, residual: misses.iter().sum(), miss: misses }
}

/// The deck: models at `sc`'s corner, its `.temp` and `.param`s, flat
/// circuit, one capacitor per extracted matrix entry, the testbench `tb`.
///
/// # Errors
/// A device the circuit cannot express (`flat_circuit_with`).
fn deck(netlist: &Netlist, par: &Parasitics, cfg: &PerfConfig, tb: &str, sc: &Scenario) -> Result<String, String> {
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
    let mut lib = OpConfig { corner: sc.corner.clone(), ..cfg.sim.clone() }.lib_lines();
    lib.push_str(&format!(".temp {}\n", sc.temp_c));
    lib.extend(sc.params.iter().map(|(k, v)| format!(".param {k}={v}\n")));
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
        "* Philis post-layout performance (generated)\n{lib}{circuit}\n* extracted capacitance\n{pex}\n* routed branch resistance\n{rs}\n{tb}\n.end\n"
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

/// Simulate every `(testbench, scenario)` deck of `scenarios` (indices into
/// [`PerfConfig::scenarios`]) and [`score`] them. `Err` when the circuit
/// cannot be simulated at all; a failed measurement is a `None` metric (and a
/// full miss), not an error.
///
/// # Errors
/// A deck cannot be built ("cannot simulate: …") or written, ngspice cannot be
/// started, or it exits with an error ("cannot simulate: ngspice exit …"); a
/// metric measured by two testbenches ("metric {m} measured by testbenches
/// {a} and {b}").
pub fn evaluate(netlist: &Netlist, par: &Parasitics, cfg: &PerfConfig, scenarios: &[usize]) -> Result<PerfResult, String> {
    let all = cfg.scenarios();
    let jobs: Vec<(usize, &Scenario)> = scenarios
        .iter()
        .flat_map(|&i| cfg.testbenches.iter().enumerate().map(move |(t, _)| (t, i)))
        .map(|(t, i)| all.get(i).map(|sc| (t, sc)).ok_or(format!("cannot simulate: no scenario {i}")))
        .collect::<Result<_, _>>()?;
    // ponytail: one thread per deck; PERF-11's run_jobs bounds it
    // One dir per run (`run_deck`): decks go in parallel and must not share a
    // deck or the ngspice cwd (`bsim4v5.out`).
    let outs: Vec<Result<Vec<(String, f64)>, String>> = std::thread::scope(|scope| {
        let handles: Vec<_> = jobs
            .iter()
            .map(|&(t, sc)| {
                scope.spawn(move || {
                    let text = deck(netlist, par, cfg, &cfg.testbenches[t], sc).map_err(|e| format!("cannot simulate: {e}"))?;
                    let out = run_deck(&cfg.sim.ngspice, "perf", &text)?;
                    Ok(parse_measures(&String::from_utf8_lossy(&out.stdout)))
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap_or_else(|_| Err("cannot simulate: a deck panicked".into()))).collect()
    });
    let outs = outs.into_iter().collect::<Result<Vec<_>, _>>()?;
    let n_tb = cfg.testbenches.len();
    let mut measured = Vec::with_capacity(scenarios.len());
    for per_sc in outs.chunks(n_tb.max(1)) {
        let mut row = Vec::with_capacity(cfg.specs.len());
        for s in &cfg.specs {
            let key = s.metric.to_ascii_lowercase();
            let found: Vec<(usize, f64)> =
                per_sc.iter().enumerate().filter_map(|(t, m)| m.iter().rev().find(|(k, _)| *k == key).map(|&(_, v)| (t, v))).collect();
            if let [(a, _), (b, _), ..] = found[..] {
                return Err(format!("metric {} measured by testbenches {a} and {b}", s.metric));
            }
            row.push(found.first().map(|&(_, v)| v));
        }
        measured.push(row);
    }
    if n_tb == 0 {
        measured = vec![vec![None; cfg.specs.len()]; scenarios.len()];
    }
    Ok(score(&cfg.specs, &measured, scenarios))
}

/// Finite-difference sensitivities at the schematic operating point: the
/// baseline (no parasitics), and `∂f/∂C` per bound per net from one run with
/// `delta_af` of ground capacitance added to that net alone, both over the
/// same scenarios. Runs in parallel.
pub struct Sensitivity {
    pub base: PerfResult,
    /// `[bound][net]`, aligned with `base.bounds`: the change of the bound's
    /// worst value over the scenarios, metric units per aF; `None` where a run
    /// did not measure. The worst scenario may switch under the step; the
    /// difference is still of the worst case, the corner-safe margin.
    pub d_per_af: Vec<Vec<Option<f64>>>,
}

/// # Errors
/// As [`evaluate`], for the baseline run.
pub fn sensitivities(netlist: &Netlist, cfg: &PerfConfig, nets: &[String], delta_af: f64, scenarios: &[usize]) -> Result<Sensitivity, String> {
    let base = evaluate(netlist, &Parasitics::default(), cfg, scenarios)?;
    let runs: Vec<Option<PerfResult>> = std::thread::scope(|scope| {
        let handles: Vec<_> = nets
            .iter()
            .map(|n| {
                let caps = vec![(n.clone(), None, delta_af / 1000.0)];
                scope.spawn(move || evaluate(netlist, &Parasitics { caps, ..Parasitics::default() }, cfg, scenarios).ok())
            })
            .collect();
        handles.into_iter().map(|h| h.join().ok().flatten()).collect()
    });
    let d_per_af = (0..base.bounds.len())
        .map(|k| {
            runs.iter()
                .map(|r| {
                    let (f0, f1) = (base.bounds[k].value?, r.as_ref()?.bounds[k].value?);
                    Some((f1 - f0) / delta_af)
                })
                .collect()
        })
        .collect();
    Ok(Sensitivity { base, d_per_af })
}

/// One [`analog::routing::PerformanceBudget`] row per bound of `s.base`,
/// metric `"{metric}:min"` / `"{metric}:max"`: `w_i = sign·(∂f/∂C_i) /
/// headroom`, `limit = 1`, with floor headroom `f0 − lo` (`sign = −1`) and
/// ceiling `hi − f0` (`sign = +1`), `f0` the bound's worst value over the
/// scenarios. A bound the schematic already misses (`headroom ≤ 0`) keeps a
/// do-not-worsen row: `limit = 0`, `w_i = sign·(∂f/∂C_i) / |bound|` (`miss`'s
/// unit-free scale, `1` for a zero bound). A bound the schematic does not
/// measure finitely has no row (reported by the caller); a net whose run did
/// not measure is left out of the row.
#[must_use]
pub fn budget_rows(
    cfg: &PerfConfig,
    s: &Sensitivity,
    nets: &[pnr_core::NetId],
    af_per_nm: f32,
) -> Vec<analog::routing::PerformanceBudget> {
    s.base
        .bounds
        .iter()
        .zip(&s.d_per_af)
        .filter_map(|(b, d)| {
            let spec = &cfg.specs[b.spec];
            let (bound, sign, side) = if b.upper { (spec.max?, 1.0, "max") } else { (spec.min?, -1.0, "min") };
            let f0 = b.value.filter(|v| v.is_finite())?;
            let headroom = sign * (bound - f0);
            let (scale, limit) = if headroom > 0.0 { (headroom, 1.0) } else { (if bound == 0.0 { 1.0 } else { bound.abs() }, 0.0) };
            let (nets, weights): (Vec<_>, Vec<_>) =
                nets.iter().zip(d).filter_map(|(&n, d)| Some((n, (sign * d.as_ref()? / scale) as f32))).unzip();
            Some(analog::routing::PerformanceBudget { metric: format!("{}:{side}", spec.metric), nets, weights, af_per_nm, limit })
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
        let cfg = PerfConfig { sim: OpConfig::default(), testbenches: vec![".measure tran x avg v(out)".into()], specs: vec![], scenarios: Vec::new() };
        let d = deck(&nl, &Parasitics { caps, ..Parasitics::default() }, &cfg, &cfg.testbenches[0], &cfg.scenarios()[0]).unwrap();
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
        let cfg = PerfConfig { sim: OpConfig::default(), testbenches: vec![String::new()], specs: vec![], scenarios: Vec::new() };
        let d = deck(&nl, &par, &cfg, "", &cfg.scenarios()[0]).unwrap();
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
            testbenches: Vec::new(),
            specs: vec![spec(Some(300e6), None), spec(None, Some(1.0)), spec(Some(10.0), None)],
            scenarios: Vec::new(),
        };
        let s = Sensitivity {
            // UGF 400 MHz (100 MHz headroom); power 0.5 (0.5 headroom); gain 5 (already short).
            base: score(&cfg.specs, &[vec![Some(400e6), Some(0.5), Some(5.0)]], &[0]),
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
        let cfg = PerfConfig { sim: OpConfig::default(), testbenches: Vec::new(), specs, scenarios: Vec::new() };
        let base = score(&cfg.specs, &[vec![Some(f0)]], &[0]);
        let s = Sensitivity { d_per_af: vec![vec![Some(d)]; base.bounds.len()], base };
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

    #[test]
    fn the_deck_sets_corner_temperature_and_params() {
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
        let sim = OpConfig { model_lib: Some("/m.lib".into()), ..OpConfig::default() };
        let cfg = PerfConfig { sim, testbenches: vec![String::new()], specs: vec![], scenarios: Vec::new() };
        let sc = Scenario { name: "ss_hot".into(), corner: "ss".into(), temp_c: 125.0, params: vec![("vdd".into(), 1.62)] };
        let d = deck(&nl, &Parasitics::default(), &cfg, "", &sc).unwrap();
        assert!(d.contains(".lib /m.lib ss\n"), "{d}");
        assert!(d.contains(".param vdd=1.62"), "{d}");
        let temp = d.find(".temp 125").expect(".temp line");
        assert!(temp < d.find("XM1").expect("device card"), "{d}");
    }

    #[test]
    fn the_worst_scenario_is_chosen_per_bound() {
        let specs = [spec(Some(10.0), Some(20.0))];
        let r = score(&specs, &[vec![Some(15.0)], vec![Some(11.0)], vec![Some(19.0)]], &[0, 1, 2]);
        assert_eq!(r.bounds[0], BoundResult { spec: 0, upper: false, value: Some(11.0), scenario: 1 });
        assert_eq!(r.bounds[1], BoundResult { spec: 0, upper: true, value: Some(19.0), scenario: 2 });
        assert_eq!(r.residual, 0.0);
        assert_eq!(r.metrics[0].1, Some(15.0));
        let r = score(&specs, &[vec![Some(15.0)], vec![Some(8.0)], vec![Some(19.0)]], &[0, 1, 2]);
        assert!((r.miss[0] - 0.2).abs() < 1e-12, "{:?}", r.miss);
        let r = score(&specs, &[vec![Some(15.0)], vec![None], vec![Some(19.0)]], &[0, 1, 2]);
        assert!(r.bounds.iter().all(|b| b.value.is_none() && b.scenario == 1), "{:?}", r.bounds);
        assert_eq!(r.residual, 1.0);
    }
}
