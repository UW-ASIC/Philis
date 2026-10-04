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
    /// Per device, a DC source in series with the gate, V (`V_gate −
    /// V_net`); missing or `0` = none. Sensitivity runs only.
    pub gate_offset_v: Vec<f64>,
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
    // A gate offset sits between the gate and its net (or branch-R) node.
    let offset = |di: usize| par.gate_offset_v.get(di).copied().filter(|&v| v != 0.0);
    let prev = |di: usize, t: &str, n: String| if branch(di, t).is_some() { format!("{n}__{di}_{t}") } else { n };
    let mut rs = String::new();
    for (di, dev) in netlist.devices.iter().enumerate() {
        for (t, id) in &dev.terminals {
            let net = node_name(netlist, *id);
            if let Some(r) = branch(di, t) {
                rs.push_str(&format!("Rpex_{di}_{t} {net}__{di}_{t} {net} {r:.4}\n"));
            }
            if let (Some(v), "G") = (offset(di), t.as_str()) {
                let p = prev(di, t, net);
                rs.push_str(&format!("Vgo{di} {p}__o{di} {p} {v:.6e}\n"));
            }
        }
    }
    let circuit = flat_circuit_with(
        netlist,
        &cfg.sim,
        |di, t, n| {
            let p = prev(di, t, n);
            if t == "G" && offset(di).is_some() { format!("{p}__o{di}") } else { p }
        },
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

/// `f` over `jobs` on a bounded pool: `available_parallelism()` workers (1
/// when unknown) take the next job index from one counter, so nested sweeps
/// never spawn more threads than cores. Results in `jobs` order; a panicking
/// job panics the caller.
pub(crate) fn run_jobs<J: Sync, T: Send>(jobs: &[J], f: impl Fn(&J) -> T + Sync) -> Vec<T> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let workers = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get).min(jobs.len());
    let next = AtomicUsize::new(0);
    let out = std::sync::Mutex::new((0..jobs.len()).map(|_| None).collect::<Vec<Option<T>>>());
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(j) = jobs.get(i) else { break };
                    let r = f(j);
                    out.lock().expect("a job panicked")[i] = Some(r);
                }
            });
        }
    });
    out.into_inner().expect("a job panicked").into_iter().map(|r| r.expect("every job ran")).collect()
}

/// One deck's `.measure` results. One dir per run (`run_deck`): decks go in
/// parallel and must not share a deck or the ngspice cwd (`bsim4v5.out`). A
/// panicking deck is `Err` (a sim failure), not a panic through [`run_jobs`].
fn measure(netlist: &Netlist, par: &Parasitics, cfg: &PerfConfig, tb: usize, sc: &Scenario) -> Result<Vec<(String, f64)>, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let text = deck(netlist, par, cfg, &cfg.testbenches[tb], sc).map_err(|e| format!("cannot simulate: {e}"))?;
        let out = run_deck(&cfg.sim.ngspice, "perf", &text)?;
        Ok(parse_measures(&String::from_utf8_lossy(&out.stdout)))
    }))
    .unwrap_or_else(|_| Err("cannot simulate: a deck panicked".into()))
}

/// Every spec's metric from one scenario's per-testbench measures.
///
/// # Errors
/// A metric measured by two testbenches.
fn assemble(cfg: &PerfConfig, per_tb: &[Vec<(String, f64)>]) -> Result<Vec<Option<f64>>, String> {
    let mut row = Vec::with_capacity(cfg.specs.len());
    for s in &cfg.specs {
        let key = s.metric.to_ascii_lowercase();
        let found: Vec<(usize, f64)> =
            per_tb.iter().enumerate().filter_map(|(t, m)| m.iter().rev().find(|(k, _)| *k == key).map(|&(_, v)| (t, v))).collect();
        if let [(a, _), (b, _), ..] = found[..] {
            return Err(format!("metric {} measured by testbenches {a} and {b}", s.metric));
        }
        row.push(found.first().map(|&(_, v)| v));
    }
    Ok(row)
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
    let outs = run_jobs(&jobs, |&(t, sc)| measure(netlist, par, cfg, t, sc)).into_iter().collect::<Result<Vec<_>, _>>()?;
    let n_tb = cfg.testbenches.len();
    let mut measured = outs.chunks(n_tb.max(1)).map(|per_sc| assemble(cfg, per_sc)).collect::<Result<Vec<_>, _>>()?;
    if n_tb == 0 {
        measured = vec![vec![None; cfg.specs.len()]; scenarios.len()];
    }
    Ok(score(&cfg.specs, &measured, scenarios))
}

/// One perturbed quantity. Gate offset sign: a source in series with the
/// gate, `δ = V_gate − V_net`; an NMOS threshold shift ΔV_T acts as `δ =
/// −ΔV_T`, a PMOS one as `δ = +Δ|V_T|`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Param {
    /// Capacitance from the net to ground, aF.
    GroundC { net: pnr_core::NetId },
    /// Capacitance between two nets, aF; `a < b`.
    CouplingC { a: pnr_core::NetId, b: pnr_core::NetId },
    /// Branch R in series with one terminal, Ω; `terminal` indexes the
    /// device's `terminals`.
    SeriesR { device: u16, terminal: u8 },
    /// Gate offset source, V.
    GateOffset { device: u16 },
}

/// One parameter's derivative of every spec's metric.
#[derive(Clone, Debug)]
pub struct SensRow {
    pub param: Param,
    /// Δ, in the parameter's unit.
    pub step: f64,
    /// Per spec, metric units per parameter unit; `None` where a run did not
    /// measure it.
    pub d: Vec<Option<f64>>,
    /// Both difference quotients agree within [`StepPolicy::lin_tol`] on every
    /// measured spec.
    pub linear: bool,
}

/// Finite-difference sensitivities at one scenario around `at`.
#[derive(Clone, Debug)]
pub struct SensTable {
    /// Index into [`PerfConfig::scenarios`].
    pub scenario: usize,
    pub at: Parasitics,
    /// [`score`] of the unperturbed run at `scenario` alone.
    pub base: PerfResult,
    pub rows: Vec<SensRow>,
    /// ngspice decks run for this table.
    pub sims: u32,
}

/// Perturbation sizes. C: `Δ = max(c_min_af, c_frac·C_est)` with `C_est` the
/// gate capacitance on the net; R: `r_ohm`; gate offset: the device's σ, 1 mV
/// when unknown. Central differences when the base value is at least Δ
/// (always for gate offsets), else forward at Δ and Δ/2; `linear` is
/// `|s₁ − s₂| ≤ lin_tol·max(|s₁|, |s₂|)` over the two quotients.
#[derive(Clone, Copy, Debug)]
pub struct StepPolicy {
    pub c_min_af: f64,
    pub c_frac: f64,
    /// Gate capacitance, aF/µm²; `0` = unknown (every C step is `c_min_af`).
    pub gate_af_um2: f64,
    pub r_ohm: f64,
    pub lin_tol: f64,
}

impl Default for StepPolicy {
    fn default() -> Self {
        Self { c_min_af: 1000.0, c_frac: 0.1, gate_af_um2: 0.0, r_ohm: 100.0, lin_tol: 0.2 }
    }
}

/// `at` with `v` added to `p` (aF / Ω / V; may be negative).
fn perturb(netlist: &Netlist, at: &Parasitics, p: Param, v: f64) -> Parasitics {
    let mut out = at.clone();
    let name = |n: pnr_core::NetId| netlist.nets[n.0 as usize].name.clone();
    let mut cap = |a: String, b: Option<String>| match out.caps.iter_mut().find(|(x, y, _)| (*x == a && *y == b) || (Some(x) == b.as_ref() && y.as_ref() == Some(&a))) {
        Some(e) => e.2 += v / 1000.0,
        None => out.caps.push((a, b, v / 1000.0)),
    };
    match p {
        Param::GroundC { net } => cap(name(net), None),
        Param::CouplingC { a, b } => cap(name(a), Some(name(b))),
        Param::SeriesR { device, terminal } => {
            let t = &netlist.devices[device as usize].terminals[terminal as usize].0;
            out.series.resize(netlist.devices.len().max(out.series.len()), Vec::new());
            let s = &mut out.series[device as usize];
            match s.iter_mut().find(|(n, _)| n == t) {
                Some(e) => e.1 += v as f32,
                None => s.push((t.clone(), v as f32)),
            }
        }
        Param::GateOffset { device } => {
            out.gate_offset_v.resize(netlist.devices.len().max(out.gate_offset_v.len()), 0.0);
            out.gate_offset_v[device as usize] += v;
        }
    }
    out
}

/// `p`'s value in `at` (aF / Ω / V), `0` when absent.
fn base_value(netlist: &Netlist, at: &Parasitics, p: Param) -> f64 {
    let name = |n: pnr_core::NetId| netlist.nets[n.0 as usize].name.as_str();
    let cap = |a: &str, b: Option<&str>| {
        at.caps.iter().find(|(x, y, _)| (x == a && y.as_deref() == b) || (Some(x.as_str()) == b && y.as_deref() == Some(a))).map_or(0.0, |e| e.2 * 1000.0)
    };
    match p {
        Param::GroundC { net } => cap(name(net), None),
        Param::CouplingC { a, b } => cap(name(a), Some(name(b))),
        Param::SeriesR { device, terminal } => {
            let t = &netlist.devices[device as usize].terminals[terminal as usize].0;
            at.series.get(device as usize).and_then(|s| s.iter().find(|(n, _)| n == t)).map_or(0.0, |e| f64::from(e.1))
        }
        Param::GateOffset { device } => at.gate_offset_v.get(device as usize).copied().unwrap_or(0.0),
    }
}

/// Δ for `p` under `steps` ([`StepPolicy`]).
fn step(netlist: &Netlist, p: Param, sigma_v: &[Option<f64>], steps: &StepPolicy) -> f64 {
    let c = |net: pnr_core::NetId| {
        let gates: f64 = netlist.devices.iter().filter(|d| d.terminals.iter().any(|(t, n)| t == "G" && *n == net)).map(pnr_core::Device::gate_area_um2).sum();
        steps.c_min_af.max(steps.c_frac * gates * steps.gate_af_um2)
    };
    match p {
        Param::GroundC { net } | Param::CouplingC { a: net, .. } => c(net),
        Param::SeriesR { .. } => steps.r_ohm,
        Param::GateOffset { device } => sigma_v.get(device as usize).copied().flatten().unwrap_or(1e-3),
    }
}

/// The rows of `params` around `at` at `scenario`, and the base metrics
/// (simulated unless given). Returns `(base, rows, sims)`.
#[allow(clippy::too_many_arguments)]
fn sens_rows(
    netlist: &Netlist,
    cfg: &PerfConfig,
    scenario: usize,
    params: &[Param],
    sigma_v: &[Option<f64>],
    steps: &StepPolicy,
    at: &Parasitics,
    base: Option<Vec<Option<f64>>>,
) -> Result<(Vec<Option<f64>>, Vec<SensRow>, u32), String> {
    let all = cfg.scenarios();
    let sc = all.get(scenario).ok_or(format!("cannot simulate: no scenario {scenario}"))?;
    let plan: Vec<(f64, bool)> = params
        .iter()
        .map(|&p| {
            let s = step(netlist, p, sigma_v, steps);
            (s, matches!(p, Param::GateOffset { .. }) || base_value(netlist, at, p) >= s)
        })
        .collect();
    // Points: [base?] then per row (+Δ, −Δ) central or (Δ, Δ/2) forward.
    let mut points: Vec<Parasitics> = if base.is_none() { vec![at.clone()] } else { Vec::new() };
    for (&p, &(s, central)) in params.iter().zip(&plan) {
        points.extend([s, if central { -s } else { s / 2.0 }].map(|v| perturb(netlist, at, p, v)));
    }
    let n_tb = cfg.testbenches.len();
    let jobs: Vec<(usize, usize)> = (0..points.len()).flat_map(|k| (0..n_tb).map(move |t| (k, t))).collect();
    let outs = run_jobs(&jobs, |&(k, t)| measure(netlist, &points[k], cfg, t, sc));
    let mut values = outs.chunks(n_tb.max(1)).map(|c| c.iter().cloned().collect::<Result<Vec<_>, _>>().and_then(|m| assemble(cfg, &m)));
    let none = vec![None; cfg.specs.len()];
    let base = match base {
        Some(b) => b,
        None if n_tb == 0 => none.clone(),
        None => values.next().expect("the base point")?,
    };
    let mut values: Vec<Vec<Option<f64>>> = values.map(|v| v.unwrap_or_else(|_| none.clone())).collect();
    if n_tb == 0 {
        values = vec![none; points.len()];
    }
    let rows = params
        .iter()
        .zip(&plan)
        .zip(values.chunks(2))
        .map(|((&param, &(s, central)), v)| {
            let tol = |a: f64, b: f64| (a - b).abs() <= steps.lin_tol * a.abs().max(b.abs());
            let quot: Vec<Option<(f64, f64)>> = (0..cfg.specs.len())
                .map(|j| {
                    let (f0, f1, f2) = (base[j]?, v[0][j]?, v[1][j]?);
                    Some(if central { ((f1 - f0) / s, (f0 - f2) / s) } else { ((f1 - f0) / s, (f2 - f0) / (s / 2.0)) })
                })
                .collect();
            let d = quot.iter().map(|q| q.map(|(a, b)| if central { (a + b) / 2.0 } else { b })).collect();
            SensRow { param, step: s, d, linear: quot.iter().flatten().all(|&(a, b)| tol(a, b)) }
        })
        .collect();
    Ok((base, rows, jobs.len() as u32))
}

/// Finite-difference sensitivities of every spec's metric to each of
/// `params` at `scenario`, around `at` ([`StepPolicy`]): one base run plus
/// two per row, every deck on one [`run_jobs`] pool. `sigma_v` is the gate
/// offset step per device (empty / `None` = 1 mV). A perturbed run that
/// fails reads as unmeasured (`d = None`).
///
/// # Errors
/// As [`evaluate`], for the base run.
pub fn sensitivities(
    netlist: &Netlist,
    cfg: &PerfConfig,
    scenario: usize,
    params: &[Param],
    sigma_v: &[Option<f64>],
    steps: &StepPolicy,
    at: &Parasitics,
) -> Result<SensTable, String> {
    let (base, rows, sims) = sens_rows(netlist, cfg, scenario, params, sigma_v, steps, at, None)?;
    Ok(SensTable { scenario, at: at.clone(), base: score(&cfg.specs, &[base], &[scenario]), rows, sims })
}

/// Appends `CouplingC` rows for the `max` best-screened pairs
/// ([`coupling_params`]) to `t`, reusing `t.base`.
///
/// # Errors
/// As [`sensitivities`].
pub fn add_coupling(t: &mut SensTable, netlist: &Netlist, cfg: &PerfConfig, nets: &[pnr_core::NetId], steps: &StepPolicy, max: usize) -> Result<(), String> {
    let params = coupling_params(t, netlist, nets, max);
    let base = t.base.metrics.iter().map(|m| m.1).collect();
    let (_, rows, sims) = sens_rows(netlist, cfg, t.scenario, &params, &[], steps, &t.at, Some(base))?;
    t.rows.extend(rows);
    t.sims += sims;
    Ok(())
}

/// Steps 1–3 of the selection: `GroundC` per `nets`; `SeriesR` for FET S/D,
/// BJT E/C, resistor P/N terminals on nets with ≥ 2 device pins;
/// `GateOffset` per FET.
#[must_use]
pub fn default_params(netlist: &Netlist, nets: &[pnr_core::NetId]) -> Vec<Param> {
    use pnr_core::DeviceKind as K;
    let pins = |n: pnr_core::NetId| netlist.devices.iter().flat_map(|d| &d.terminals).filter(|(_, x)| *x == n).count();
    let mut out: Vec<Param> = nets.iter().map(|&net| Param::GroundC { net }).collect();
    for (di, d) in netlist.devices.iter().enumerate() {
        let names: &[&str] = match d.kind {
            K::Nmos | K::Pmos => &["S", "D"],
            K::Npn | K::Pnp => &["E", "C"],
            K::Resistor => &["P", "N"],
            _ => &[],
        };
        for (ti, (t, n)) in d.terminals.iter().enumerate() {
            if names.contains(&t.as_str()) && pins(*n) >= 2 {
                out.push(Param::SeriesR { device: di as u16, terminal: ti as u8 });
            }
        }
    }
    let fets = netlist.devices.iter().enumerate().filter(|(_, d)| matches!(d.kind, K::Nmos | K::Pmos));
    out.extend(fets.map(|(di, _)| Param::GateOffset { device: di as u16 }));
    out
}

/// Pairs `(a ∈ nets, b ≠ a)` over every net but the ground node (its
/// coupling is the `GroundC` row), unordered, ranked by `max_j (|∂f_j/∂C_a| +
/// |∂f_j/∂C_b|) / s_j` from `t`'s `GroundC` rows (a net without one counts
/// 0), `s_j` = |base metric j| (1 when 0 or unmeasured) so no spec's unit
/// dominates; ties by `(a, b)`; at most `max`.
#[must_use]
pub fn coupling_params(t: &SensTable, netlist: &Netlist, nets: &[pnr_core::NetId], max: usize) -> Vec<Param> {
    let scale: Vec<f64> = t.base.metrics.iter().map(|m| m.1.map_or(1.0, f64::abs)).map(|s| if s == 0.0 { 1.0 } else { s }).collect();
    let d = |n: u16| t.rows.iter().find(|r| r.param == Param::GroundC { net: pnr_core::NetId(n) }).map(|r| &r.d);
    let mut pairs: Vec<(f64, u16, u16)> = Vec::new();
    for a in nets.iter().map(|n| n.0) {
        for b in (0..netlist.nets.len() as u16).filter(|&b| b != a && node_name(netlist, pnr_core::NetId(b)) != "0") {
            let (a, b) = (a.min(b), a.max(b));
            if pairs.iter().any(|p| (p.1, p.2) == (a, b)) {
                continue;
            }
            let g = |n: u16, j: usize| d(n).and_then(|d| d[j]).map_or(0.0, f64::abs);
            let score = scale.iter().enumerate().map(|(j, s)| (g(a, j) + g(b, j)) / s).fold(0.0, f64::max);
            pairs.push((score, a, b));
        }
    }
    pairs.sort_by(|x, y| y.0.total_cmp(&x.0).then((x.1, x.2).cmp(&(y.1, y.2))));
    pairs.into_iter().take(max).map(|(_, a, b)| Param::CouplingC { a: pnr_core::NetId(a), b: pnr_core::NetId(b) }).collect()
}

/// One [`analog::routing::PerformanceBudget`] row per bound of `start`,
/// metric `"{metric}:min"` / `"{metric}:max"`: `w_i = sign·(∂f/∂C_i) /
/// headroom`, `limit = 1`, with floor headroom `f0 − lo` (`sign = −1`) and
/// ceiling `hi − f0` (`sign = +1`), `f0` the bound's worst value over the
/// scenarios and `∂f/∂C_i` the `GroundC` row of the table at that bound's
/// scenario. A bound the schematic already misses (`headroom ≤ 0`) keeps a
/// do-not-worsen row: `limit = 0`, `w_i = sign·(∂f/∂C_i) / |bound|` (`miss`'s
/// unit-free scale, `1` for a zero bound). A bound the schematic does not
/// measure finitely has no row (reported by the caller); a net without a
/// measured, linear row is left out of the row.
#[must_use]
pub fn budget_rows(
    cfg: &PerfConfig,
    start: &PerfResult,
    tables: &[SensTable],
    nets: &[pnr_core::NetId],
    af_per_nm: f32,
) -> Vec<analog::routing::PerformanceBudget> {
    start
        .bounds
        .iter()
        .filter_map(|b| {
            let spec = &cfg.specs[b.spec];
            let (bound, sign, side) = if b.upper { (spec.max?, 1.0, "max") } else { (spec.min?, -1.0, "min") };
            let f0 = b.value.filter(|v| v.is_finite())?;
            let headroom = sign * (bound - f0);
            let (scale, limit) = if headroom > 0.0 { (headroom, 1.0) } else { (if bound == 0.0 { 1.0 } else { bound.abs() }, 0.0) };
            let table = tables.iter().find(|t| t.scenario == b.scenario);
            let d = |n: pnr_core::NetId| table?.rows.iter().find(|r| r.param == Param::GroundC { net: n } && r.linear)?.d[b.spec];
            let (nets, weights): (Vec<_>, Vec<_>) = nets.iter().filter_map(|&n| Some((n, (sign * d(n)? / scale) as f32))).unzip();
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
        let par = Parasitics { series: vec![vec![("D".into(), 12.5)]], lod_inv_um: vec![Some(1.0)], ..Parasitics::default() };
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
        // UGF 400 MHz (100 MHz headroom); power 0.5 (0.5 headroom); gain 5 (already short).
        let start = score(&cfg.specs, &[vec![Some(400e6), Some(0.5), Some(5.0)]], &[0]);
        // UGF drops 1 MHz per aF on net 0, not at all on net 1; net 2's row is nonlinear.
        let rows = vec![
            ground_row(0, vec![Some(-1e6), Some(0.01), Some(0.0)], true),
            ground_row(1, vec![Some(0.0), None, Some(0.0)], true),
            ground_row(2, vec![Some(-5e6), Some(0.1), Some(0.0)], false),
        ];
        let t = SensTable { scenario: 0, at: Parasitics::default(), base: start.clone(), rows, sims: 0 };
        let nets = [pnr_core::NetId(0), pnr_core::NetId(1), pnr_core::NetId(2)];
        let rows = budget_rows(&cfg, &start, &[t], &nets, 0.1);
        assert_eq!(rows.len(), 3, "one row per bound, the missed one included");
        assert_eq!(rows[0].nets, nets[..2], "a nonlinear row is left out");
        assert!((rows[0].weights[0] - 0.01).abs() < 1e-7, "1 MHz/aF of 100 MHz = 1% per aF");
        assert_eq!(rows[0].weights[1], 0.0);
        assert_eq!(rows[1].nets, vec![pnr_core::NetId(0)], "an unmeasured net is left out");
        assert!((rows[1].weights[0] - 0.02).abs() < 1e-7);
        assert_eq!((rows[2].metric.as_str(), rows[2].limit), ("m:min", 0.0), "the missed spec keeps a do-not-worsen row");
    }

    fn ground_row(net: u16, d: Vec<Option<f64>>, linear: bool) -> SensRow {
        SensRow { param: Param::GroundC { net: pnr_core::NetId(net) }, step: 1000.0, d, linear }
    }

    fn one_net(specs: Vec<Spec>, f0: f64, d: f64) -> Vec<analog::routing::PerformanceBudget> {
        let cfg = PerfConfig { sim: OpConfig::default(), testbenches: Vec::new(), specs, scenarios: Vec::new() };
        let base = score(&cfg.specs, &[vec![Some(f0)]], &[0]);
        let t = SensTable { scenario: 0, at: Parasitics::default(), base: base.clone(), rows: vec![ground_row(0, vec![Some(d)], true)], sims: 0 };
        budget_rows(&cfg, &base, &[t], &[pnr_core::NetId(0)], 1.0)
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
    fn nets(names: &[&str]) -> Vec<pnr_core::Net> {
        names.iter().map(|n| pnr_core::Net { name: (*n).into() }).collect()
    }

    #[test]
    fn run_jobs_never_exceeds_the_pool() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let (live, max) = (AtomicUsize::new(0), AtomicUsize::new(0));
        let jobs: Vec<usize> = (0..64).collect();
        let out = run_jobs(&jobs, |&i| {
            let now = live.fetch_add(1, Ordering::SeqCst) + 1;
            max.fetch_max(now, Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(5));
            live.fetch_sub(1, Ordering::SeqCst);
            i * 2
        });
        let pool = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
        assert!(max.load(Ordering::SeqCst) <= pool, "{} > {pool}", max.load(Ordering::SeqCst));
        assert_eq!(out, (0..64).map(|i| i * 2).collect::<Vec<_>>());
    }

    #[test]
    fn the_deck_inserts_a_gate_offset_source() {
        use pnr_core::{Device, DeviceKind, NetId};
        let nl = Netlist {
            devices: vec![Device {
                name: "M1".into(),
                kind: DeviceKind::Nmos, model: String::new(),
                terminals: vec![("D".into(), NetId(0)), ("G".into(), NetId(1)), ("S".into(), NetId(2)), ("B".into(), NetId(2))],
                params: vec![("w".into(), 1000), ("l".into(), 150)],
            }],
            nets: nets(&["out", "in", "vss"]),
            ..Default::default()
        };
        let cfg = PerfConfig { sim: OpConfig::default(), testbenches: vec![String::new()], specs: vec![], scenarios: Vec::new() };
        let par = Parasitics { gate_offset_v: vec![1e-3], ..Parasitics::default() };
        let d = deck(&nl, &par, &cfg, "", &cfg.scenarios()[0]).unwrap();
        assert!(d.contains("Vgo0 in__o0 in 1.000000e-3"), "{d}");
        let card: Vec<&str> = d.lines().find(|l| l.starts_with("XM1 ")).expect("FET card").split_whitespace().collect();
        assert_eq!(card[2], "in__o0", "{d}");
    }

    fn sim_cfg(bench: &str, metric: &str) -> PerfConfig {
        PerfConfig {
            sim: OpConfig { model_lib: None, nmos_model: "nch".into(), ..OpConfig::default() },
            testbenches: vec![bench.into()],
            specs: vec![Spec { metric: metric.into(), min: None, max: None }],
            scenarios: Vec::new(),
        }
    }

    /// f3dB = 1/(2πRC), so ∂f/∂C = −f/C: at C = 1 pF = 1e6 aF, −f0/1e6 per aF.
    #[test]
    fn rc_lowpass_f3db_derivative_matches_analytic() {
        if !crate::tools::tool_or_skip("ngspice") {
            return;
        }
        let nl = Netlist { nets: nets(&["in", "out"]), ..Default::default() };
        let cfg = sim_cfg("Vin in 0 dc 0 ac 1\nR1 in out 1k\nC1 out 0 1p\n.ac dec 1000 1e6 1e10\n.control\nrun\nmeas ac f3db when vdb(out)=-3\n.endc", "f3db");
        let out = pnr_core::NetId(1);
        let t = sensitivities(&nl, &cfg, 0, &[Param::GroundC { net: out }], &[], &StepPolicy::default(), &Parasitics::default()).unwrap();
        let f0 = t.base.metrics[0].1.expect("f3db measured");
        let d = t.rows[0].d[0].expect("derivative measured");
        let want = -f0 / 1e6;
        assert!((d / want - 1.0).abs() < 0.05, "{d} vs {want}");
        assert!(t.rows[0].linear);
        assert_eq!(t.sims, 3);
    }

    /// A positive offset on one input of a resistively loaded pair moves the
    /// differential output one way, on the other input the other way, by the
    /// same amount.
    #[test]
    fn gate_offset_moves_a_diff_pair_output_both_ways() {
        if !crate::tools::tool_or_skip("ngspice") {
            return;
        }
        use pnr_core::{Device, DeviceKind, NetId};
        // o1 o2 t in vdd vss
        let fet = |name: &str, d: u16| Device {
            name: name.into(),
            kind: DeviceKind::Nmos, model: String::new(),
            terminals: vec![("D".into(), NetId(d)), ("G".into(), NetId(3)), ("S".into(), NetId(2)), ("B".into(), NetId(5))],
            params: vec![("w".into(), 10_000), ("l".into(), 1000)],
        };
        let nl = Netlist { devices: vec![fet("M1", 0), fet("M2", 1)], nets: nets(&["o1", "o2", "t", "in", "vdd", "vss"]), ..Default::default() };
        // The card is `X… nch W= L= nf= m=` (µm): a level-1 model behind a
        // subckt. Batch `.measure` cannot read `vdb`/`v(a,b)`: `.control` can.
        let bench = ".subckt nch d g s b W=1 L=1 nf=1 m=1\nM0 d g s b nchm W={W*1e-6} L={L*1e-6}\n.ends\n\
            .model nchm nmos level=1 vto=0.5 kp=200u\n\
            Vdd vdd 0 1.8\nVg in 0 1\nRl1 vdd o1 10k\nRl2 vdd o2 10k\nI1 t 0 100u\n\
            .dc Vdd 1.7 1.9 0.1\n.control\nrun\nlet d = v(o1)-v(o2)\nmeas dc vod find d at=1.8\n.endc";
        let cfg = sim_cfg(bench, "vod");
        let params = [Param::GateOffset { device: 0 }, Param::GateOffset { device: 1 }];
        let t = sensitivities(&nl, &cfg, 0, &params, &[], &StepPolicy::default(), &Parasitics::default()).unwrap();
        let (d1, d2) = (t.rows[0].d[0].expect("measured"), t.rows[1].d[0].expect("measured"));
        assert!(d1 * d2 < 0.0, "{d1} {d2}");
        assert!((d1 + d2).abs() <= 0.1 * d1.abs().max(d2.abs()), "{d1} {d2}");
        assert!(t.rows.iter().all(|r| r.linear));
    }

    #[test]
    fn coupling_rows_only_for_screened_pairs() {
        let names: Vec<String> = (0..20).map(|i| format!("n{i}")).collect();
        let nl = Netlist { nets: names.iter().map(|n| pnr_core::Net { name: n.clone() }).collect(), ..Default::default() };
        let ids: Vec<pnr_core::NetId> = (0..20).map(pnr_core::NetId).collect();
        let specs = [spec(Some(0.0), None)];
        let t = SensTable {
            scenario: 0,
            at: Parasitics::default(),
            base: score(&specs, &[vec![Some(1.0)]], &[0]),
            rows: (0..20).map(|i| ground_row(i, vec![Some(f64::from(i))], true)).collect(),
            sims: 0,
        };
        let p = coupling_params(&t, &nl, &ids, 64);
        assert!(p.len() <= 64 && !p.is_empty(), "{}", p.len());
        assert_eq!(p[0], Param::CouplingC { a: pnr_core::NetId(18), b: pnr_core::NetId(19) });
        let mut seen = std::collections::HashSet::new();
        for q in &p {
            let Param::CouplingC { a, b } = *q else { panic!("{q:?}") };
            assert!(a.0 < b.0, "{q:?}");
            assert!(seen.insert((a.0, b.0)), "duplicate {q:?}");
        }
    }
}
