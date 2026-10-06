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
    /// Per device, `[AS, AD, PS, PD]` per SPICE instance: µm², µm², µm, µm (the library's `.option scale=1.0u`
    /// scales them, measured: `ad=100` moves capbd 670×, `ad=1e-10` not at all); `None` = not drawn-derived.
    pub junction: Vec<Option<[f64; 4]>>,
    /// Per device, the drawn gate's poly + cut R, Ω, in series with the card's gate; `None`/0 = none.
    pub gate_ohm: Vec<Option<f64>>,
}

/// `SA = SB = S` (µm) at which BSIM4's multi-finger average `(1/nf)·Σᵢ
/// 1/(S + L/2 + i·L)` (SD = 0) gives `target/2`: an equivalent card for the
/// layout's measured stress. `None` when no `S` in `[1e-3, 1e4]` µm reaches
/// it (a non-positive or NaN target, or stress beyond what abutting fingers
/// give). `nf < 1` reads as 1.
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
    /// The `.measure` name, matched case-insensitively.
    pub metric: String,
    /// Floor, metric units; `None` (or non-finite) = no floor.
    pub min: Option<f64>,
    /// Ceiling, metric units; `None` (or non-finite) = no ceiling.
    pub max: Option<f64>,
}

/// One process/environment point: model corner, temperature, and `.param`
/// values the testbenches read (e.g. `Vdd vdd 0 {vdd}`).
#[derive(Clone, Debug)]
pub struct Scenario {
    /// Label for reports.
    pub name: String,
    /// Corner inside [`OpConfig::model_lib`].
    pub corner: String,
    /// Simulation temperature, °C.
    pub temp_c: f64,
    /// `.param name=value` lines, in order.
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
    /// The requirements, in report order; indices into this are "spec `j`".
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
    /// Index into the scored specs.
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
    /// Per spec, (min, max) of the metric over the evaluated scenarios; None if any did not measure it.
    pub spread: Vec<Option<(f64, f64)>>,
}

impl PerfResult {
    /// Every spec met and measured.
    #[must_use]
    pub fn met(&self) -> bool {
        self.residual <= 0.0
    }
}

/// Normalised miss of `v` against `spec`: the overshoot past the violated
/// bound over that bound's magnitude (`1` when the bound is zero). `≥ 0`;
/// an unmeasured (`None`) or NaN value is a full miss, `1`.
#[must_use]
pub fn miss(spec: &Spec, v: Option<f64>) -> f64 {
    miss_bounds(spec.min, spec.max, v)
}

/// [`miss`] of one side of `spec`: its ceiling when `upper`, else its floor.
#[must_use]
pub(crate) fn side_miss(spec: &Spec, upper: bool, v: Option<f64>) -> f64 {
    if upper {
        miss_bounds(None, spec.max, v)
    } else {
        miss_bounds(spec.min, None, v)
    }
}

/// [`miss`] against a bare floor and ceiling.
fn miss_bounds(min: Option<f64>, max: Option<f64>, v: Option<f64>) -> f64 {
    let Some(v) = v else { return 1.0 };
    let over = |excess: f64, bound: f64| (excess / if bound == 0.0 { 1.0 } else { bound.abs() }).max(0.0);
    min.map_or(0.0, |lo| over(lo - v, lo)) + max.map_or(0.0, |hi| over(v - hi, hi))
}

/// Score `measured[i][j]` (spec `j` at `scenarios[i]`): per finite bound the
/// scenario with the smallest margin (`v − lo` / `hi − v`), the first
/// unmeasured one if any; ties go to the earlier scenario. A NaN measurement
/// is unmeasured. Rows past `scenarios.len()` are ignored, except that
/// `metrics` always reads `measured[0]`; no rows at all leave every spec
/// unmeasured (a full miss).
///
/// # Panics
/// If a row of `measured` is shorter than `specs`.
#[must_use]
pub fn score(specs: &[Spec], measured: &[Vec<Option<f64>>], scenarios: &[usize]) -> PerfResult {
    let metrics = specs.iter().enumerate().map(|(j, s)| (s.metric.clone(), measured.first().and_then(|m| m[j]))).collect();
    let mut bounds = Vec::new();
    let mut misses = Vec::new();
    let mut spread = Vec::new();
    for (j, s) in specs.iter().enumerate() {
        let col = || measured.iter().zip(scenarios).map(move |(m, &sc)| (m[j], sc));
        spread.push(col().map(|(v, _)| v).collect::<Option<Vec<_>>>().and_then(|v| Some((v.iter().copied().reduce(f64::min)?, v.iter().copied().reduce(f64::max)?))));
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
        misses.push(match (lo, hi) {
            (Some((Some(a), _)), Some((Some(b), _))) => side_miss(s, false, Some(a)) + side_miss(s, true, Some(b)),
            _ => 1.0,
        });
    }
    PerfResult { metrics, bounds, residual: misses.iter().sum(), miss: misses, spread }
}

/// The deck: models at `sc`'s corner, its `.temp` and `.param`s, flat
/// circuit, one capacitor per extracted matrix entry, the testbench `tb`.
///
/// # Errors
/// A device the circuit cannot express (`flat_circuit_with`).
pub(crate) fn deck(netlist: &Netlist, par: &Parasitics, cfg: &PerfConfig, tb: &str, sc: &Scenario) -> Result<String, String> {
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
    // The drawn gate R (`gate_ohm`) is in series with the routed G branch: one resistor of their sum.
    let branch = |di: usize, t: &str| {
        let routed = par.series.get(di).and_then(|v| v.iter().find(|(n, _)| n == t)).map_or(0.0, |&(_, r)| f64::from(r));
        let gate = if t == "G" { par.gate_ohm.get(di).copied().flatten().unwrap_or(0.0) } else { 0.0 };
        Some(routed.max(0.0) + gate.max(0.0)).filter(|&r| r > 0.0)
    };
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
                .map_or(String::new(), |s| format!(" sa={s:.4e} sb={s:.4e}"))
                + &par.junction.get(di).copied().flatten().map_or(String::new(), |j| format!(" as={} ad={} ps={} pd={}", j[0], j[1], j[2], j[3]))
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

/// One deck's `.measure` results (testbench `tb` of `cfg` at `sc`). One dir per run (`run_deck`): decks go in
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
/// A metric measured by two testbenches. Within one testbench the last
/// `name = value` line wins.
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
    /// The perturbed quantity.
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
    /// The operating parasitics every perturbation is applied to.
    pub at: Parasitics,
    /// [`score`] of the unperturbed run at `scenario` alone.
    pub base: PerfResult,
    /// One per perturbed parameter, in request order.
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
    /// Smallest C step, aF.
    pub c_min_af: f64,
    /// C step as a fraction of the net's gate capacitance.
    pub c_frac: f64,
    /// Gate capacitance, aF/µm²; `0` = unknown (every C step is `c_min_af`).
    pub gate_af_um2: f64,
    /// R step, Ω.
    pub r_ohm: f64,
    /// Relative agreement of the two quotients for a row to read `linear`.
    pub lin_tol: f64,
}

impl Default for StepPolicy {
    fn default() -> Self {
        Self { c_min_af: 1000.0, c_frac: 0.1, gate_af_um2: 0.0, r_ohm: 100.0, lin_tol: 0.2 }
    }
}

/// Index of the `caps` entry between nets `a` and `b` (`None` = ground), in
/// either order.
fn cap_entry(caps: &verify::CapMatrix, a: &str, b: Option<&str>) -> Option<usize> {
    caps.iter().position(|(x, y, _)| (x == a && y.as_deref() == b) || (Some(x.as_str()) == b && y.as_deref() == Some(a)))
}

/// `at` with `v` added to `p` (aF / Ω / V; may be negative).
///
/// # Panics
/// If `p` names a net, device or terminal `netlist` does not have.
fn perturb(netlist: &Netlist, at: &Parasitics, p: Param, v: f64) -> Parasitics {
    let mut out = at.clone();
    let name = |n: pnr_core::NetId| netlist.nets[n.0 as usize].name.as_str();
    let mut cap = |a: &str, b: Option<&str>| match cap_entry(&out.caps, a, b) {
        Some(k) => out.caps[k].2 += v / 1000.0,
        None => out.caps.push((a.to_string(), b.map(str::to_string), v / 1000.0)),
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
///
/// # Panics
/// As [`perturb`].
fn base_value(netlist: &Netlist, at: &Parasitics, p: Param) -> f64 {
    let name = |n: pnr_core::NetId| netlist.nets[n.0 as usize].name.as_str();
    let cap = |a: &str, b: Option<&str>| cap_entry(&at.caps, a, b).map_or(0.0, |k| at.caps[k].2 * 1000.0);
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

/// Δ for `p` under `steps` ([`StepPolicy`]); a coupling step reads the
/// gate capacitance on its `a` net.
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
/// (simulated unless given). Returns `(base, rows, sims)`, `sims` the decks run.
///
/// # Errors
/// No scenario `scenario`, or the base run fails (as [`evaluate`]).
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
///
/// # Panics
/// If a `param` names a net, device or terminal `netlist` does not have.
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
/// `GateOffset` per FET. In that order, each group in device / terminal order.
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
/// dominates; ties by `(a, b)`; at most `max`. A ground net in `nets` pairs
/// with nothing. Each is `CouplingC { a, b }` with `a < b`.
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

/// EXT-17's input, one `SpecSens` per spec with a finite bound in `start`: the table read is the one at the
/// scenario of the spec's tighter bound (smaller margin, unmeasured tightest; floor on a tie), `f0` its `base`
/// metric (spec skipped when unmeasured or without a table). `proc` = `start.spread[j]` when
/// `cfg.scenarios.len() > 1`; `sigma_f` = max over the spec's bounds of `stats[b].sigma_f` (`None` if any is).
/// `d_c`/`d_cc` per aF from linear `GroundC`/`CouplingC` rows; `d_r` per Ω, linear `SeriesR` rows summed per net
/// of the terminal; `d_vt` per mV from every `GateOffset` row (σ_f's rule, [`crate::robust`]): `−d/1000` NMOS,
/// `+d/1000` PMOS; `d_t` empty.
///
/// # Panics
/// If a row names a device or terminal `netlist` does not have.
#[must_use]
pub fn to_evidence(
    cfg: &PerfConfig,
    tables: &[SensTable],
    start: &PerfResult,
    stats: &[crate::robust::BoundStat],
    netlist: &Netlist,
) -> annotator::evidence::Sensitivities {
    use annotator::evidence::SpecSens;
    use pnr_core::ids::DeviceId;
    let specs = cfg
        .specs
        .iter()
        .enumerate()
        .filter_map(|(j, spec)| {
            let bs: Vec<usize> = (0..start.bounds.len()).filter(|&b| start.bounds[b].spec == j).collect();
            let margin = |b: usize| {
                let r = &start.bounds[b];
                r.value.map_or(f64::NEG_INFINITY, |v| if r.upper { spec.max.unwrap_or(f64::INFINITY) - v } else { v - spec.min.unwrap_or(f64::NEG_INFINITY) })
            };
            let tight = bs.iter().copied().reduce(|a, b| if margin(b) < margin(a) { b } else { a })?;
            let t = tables.iter().find(|t| t.scenario == start.bounds[tight].scenario)?;
            let f0 = t.base.metrics.get(j)?.1?;
            let lin = |r: &&SensRow| r.linear;
            let d = |r: &SensRow| r.d[j];
            let mut d_r: Vec<(pnr_core::NetId, f64)> = Vec::new();
            for r in t.rows.iter().filter(lin) {
                if let (Param::SeriesR { device, terminal }, Some(x)) = (r.param, d(r)) {
                    let net = netlist.devices[device as usize].terminals[terminal as usize].1;
                    match d_r.iter_mut().find(|e| e.0 == net) {
                        Some(e) => e.1 += x,
                        None => d_r.push((net, x)),
                    }
                }
            }
            Some(SpecSens {
                metric: spec.metric.clone(),
                f0,
                lo: spec.min,
                hi: spec.max,
                proc: if cfg.scenarios.len() > 1 { start.spread.get(j).copied().flatten() } else { None },
                sigma_f: bs.iter().map(|&b| stats.get(b)?.sigma_f).collect::<Option<Vec<_>>>().and_then(|v| v.into_iter().reduce(f64::max)),
                d_c: t.rows.iter().filter(lin).filter_map(|r| match r.param { Param::GroundC { net } => Some((net, d(r)?)), _ => None }).collect(),
                d_r,
                d_vt: t
                    .rows
                    .iter()
                    .filter_map(|r| match r.param {
                        Param::GateOffset { device } => {
                            let sign = if netlist.devices[device as usize].kind == pnr_core::DeviceKind::Pmos { 1.0 } else { -1.0 };
                            Some((DeviceId(device), sign * d(r)? / 1000.0))
                        }
                        _ => None,
                    })
                    .collect(),
                d_t: Vec::new(),
                d_cc: t.rows.iter().filter(lin).filter_map(|r| match r.param { Param::CouplingC { a, b } => Some((a, b, d(r)?)), _ => None }).collect(),
            })
        })
        .collect();
    annotator::evidence::Sensitivities { specs }
}

/// RTE-21's router weights. `bounds[k] = (spec, h_b, scenario)`, `h_b > 0`. `r_weight[n]` (len `netlist.nets`)
/// = Σ_b Σ_{linear SeriesR rows on n} |d_b| / h_b, scaled so the max is 1 (all 0 if none);
/// `pair_weight` = `(a, b, Σ_b ½·|d_b| / h_b)` per linear `CouplingC` row, unscaled. Row `d` read in the table
/// whose `scenario` is the bound's. A bound with `h_b ≤ 0` or non-finite, or
/// without a table, contributes nothing.
///
/// # Panics
/// If a row's `d` is shorter than a bound's spec index + 1, or a row names a
/// device or terminal `netlist` does not have.
#[must_use]
pub fn router_weights(
    tables: &[SensTable],
    bounds: &[(usize, f64, usize)],
    netlist: &Netlist,
) -> (Vec<f32>, Vec<(pnr_core::NetId, pnr_core::NetId, f32)>) {
    let mut r = vec![0.0f64; netlist.nets.len()];
    let mut pairs: Vec<(pnr_core::NetId, pnr_core::NetId, f64)> = Vec::new();
    for &(j, h, sc) in bounds {
        let Some(t) = tables.iter().find(|t| t.scenario == sc) else { continue };
        for row in t.rows.iter().filter(|r| r.linear) {
            let Some(x) = row.d[j] else { continue };
            match row.param {
                Param::SeriesR { device, terminal } => r[netlist.devices[device as usize].terminals[terminal as usize].1 .0 as usize] += x.abs() / h,
                Param::CouplingC { a, b } => match pairs.iter_mut().find(|p| (p.0, p.1) == (a, b)) {
                    Some(p) => p.2 += 0.5 * x.abs() / h,
                    None => pairs.push((a, b, 0.5 * x.abs() / h)),
                },
                _ => {}
            }
        }
    }
    let max = r.iter().copied().fold(0.0, f64::max);
    let r_weight = r.iter().map(|&w| if max > 0.0 { (w / max) as f32 } else { 0.0 }).collect();
    (r_weight, pairs.into_iter().map(|(a, b, w)| (a, b, w as f32)).collect())
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
        assert!(d.contains(&format!(" sa={s:.4e}")), "{d}");
    }

    #[test]
    fn junction_params_reach_the_card() {
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
        let mut par = Parasitics { junction: vec![Some([2.0, 1.5, 6.0, 5.0])], gate_ohm: vec![Some(305.8)], ..Parasitics::default() };
        let cfg = PerfConfig { sim: OpConfig::default(), testbenches: vec![String::new()], specs: vec![], scenarios: Vec::new() };
        let d = deck(&nl, &par, &cfg, "", &cfg.scenarios()[0]).unwrap();
        assert!(d.contains(" as=2 ad=1.5 ps=6 pd=5"), "{d}");
        assert!(d.contains("Rpex_0_G in__0_G in 305.8000"), "{d}");
        assert!(d.contains("XM1 out in__0_G"), "{d}");
        par.series = vec![vec![("G".into(), 100.0)]];
        let d = deck(&nl, &par, &cfg, "", &cfg.scenarios()[0]).unwrap();
        assert!(d.contains("Rpex_0_G in__0_G in 405.8000"), "{d}");
        assert_eq!(d.matches("Rpex_0_G").count(), 1, "{d}");
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
        // EXT-25's path: evidence, then the annotator's conservative rows (helpful and nonlinear terms out).
        let ev = to_evidence(&cfg, &[t], &start, &[], &Netlist::default());
        let (rows, _, diags) = annotator::budget::rows(&ev, 0.1, None, &annotator::policy::Policy::default());
        let n0 = pnr_core::NetId(0);
        assert_eq!(rows.len(), 3, "one row per bound, the missed one included");
        assert_eq!(rows[0].nets, vec![n0], "a zero (not adverse) and a nonlinear term are left out");
        assert!((rows[0].weights[0] - 0.01).abs() < 1e-7, "1 MHz/aF of 100 MHz = 1% per aF");
        assert_eq!(rows[1].nets, vec![n0], "an unmeasured net is left out");
        assert!((rows[1].weights[0] - 0.02).abs() < 1e-7);
        assert_eq!((rows[2].metric.as_str(), rows[2].limit), ("m:min", 0.0), "the missed spec keeps a do-not-worsen row");
        assert_eq!(diags.iter().filter(|d| d.kind == "no_layout_margin").count(), 1, "{diags:?}");
    }

    fn ground_row(net: u16, d: Vec<Option<f64>>, linear: bool) -> SensRow {
        SensRow { param: Param::GroundC { net: pnr_core::NetId(net) }, step: 1000.0, d, linear }
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

    /// Nets `[vout1, vtail, x]`; XM1/XM2 NMOS with S on `vtail`, XM3 PMOS.
    fn sens_netlist() -> Netlist {
        use pnr_core::{Device, DeviceKind, NetId};
        let fet = |name: &str, kind| Device {
            name: name.into(),
            kind,
            model: String::new(),
            terminals: vec![("D".into(), NetId(0)), ("G".into(), NetId(2)), ("S".into(), NetId(1)), ("B".into(), NetId(1))],
            params: Vec::new(),
        };
        Netlist { devices: vec![fet("XM1", DeviceKind::Nmos), fet("XM2", DeviceKind::Nmos), fet("XM3", DeviceKind::Pmos)], nets: nets(&["vout1", "vtail", "x"]), ..Default::default() }
    }

    fn row(param: Param, d: f64, linear: bool) -> SensRow {
        SensRow { param, step: 1.0, d: vec![Some(d)], linear }
    }

    fn sens_rows(linear: bool) -> Vec<SensRow> {
        use pnr_core::NetId;
        vec![
            row(Param::GroundC { net: NetId(0) }, -0.002, linear),
            row(Param::SeriesR { device: 0, terminal: 2 }, -0.01, linear),
            row(Param::SeriesR { device: 1, terminal: 2 }, -0.01, linear),
            row(Param::GateOffset { device: 0 }, 1000.0, linear),
            row(Param::GateOffset { device: 2 }, 1000.0, linear),
            row(Param::CouplingC { a: NetId(0), b: NetId(2) }, 0.003, linear),
        ]
    }

    fn gain_cfg() -> (PerfConfig, PerfResult) {
        let cfg = PerfConfig { sim: OpConfig::default(), testbenches: Vec::new(), specs: vec![Spec { metric: "gain".into(), min: Some(40.0), max: None }], scenarios: Vec::new() };
        let start = score(&cfg.specs, &[vec![Some(60.0)]], &[0]);
        (cfg, start)
    }

    fn stat(sigma_f: Option<f64>) -> crate::robust::BoundStat {
        crate::robust::BoundStat { bound: 0, sigma_f, beta: None, yield_part: None, headroom_stat: None, shares: Vec::new() }
    }

    #[test]
    fn evidence_maps_units_and_signs() {
        use pnr_core::{ids::DeviceId, NetId};
        let (cfg, start) = gain_cfg();
        let t = SensTable { scenario: 0, at: Parasitics::default(), base: start.clone(), rows: sens_rows(true), sims: 0 };
        let ev = to_evidence(&cfg, &[t], &start, &[stat(Some(1.5))], &sens_netlist());
        assert_eq!(ev.specs.len(), 1);
        let s = &ev.specs[0];
        assert_eq!((s.metric.as_str(), s.f0, s.lo, s.hi), ("gain", 60.0, Some(40.0), None));
        assert_eq!(s.d_c, vec![(NetId(0), -0.002)]);
        assert_eq!(s.d_r.len(), 1);
        assert!(s.d_r[0].0 == NetId(1) && (s.d_r[0].1 + 0.02).abs() < 1e-12, "{:?}", s.d_r);
        assert_eq!(s.d_vt, vec![(DeviceId(0), -1.0), (DeviceId(2), 1.0)]);
        assert_eq!(s.d_cc, vec![(NetId(0), NetId(2), 0.003)]);
        assert_eq!((s.sigma_f, s.proc), (Some(1.5), None));
        assert!(s.d_t.is_empty());
    }

    #[test]
    fn router_weights_scale_r_to_one() {
        use pnr_core::NetId;
        let mut nl = sens_netlist();
        nl.devices[1].terminals[2].1 = NetId(0);
        let rows = vec![
            row(Param::SeriesR { device: 1, terminal: 2 }, -0.3, true),
            row(Param::SeriesR { device: 0, terminal: 2 }, 0.6, true),
            row(Param::CouplingC { a: NetId(0), b: NetId(1) }, 0.4, true),
        ];
        let t = SensTable { scenario: 0, at: Parasitics::default(), base: PerfResult::default(), rows, sims: 0 };
        let (r, pairs) = router_weights(&[t], &[(0, 2.0, 0)], &nl);
        assert!((r[0] - 0.5).abs() < 1e-6 && (r[1] - 1.0).abs() < 1e-6 && r[2] == 0.0, "{r:?}");
        assert_eq!(pairs.len(), 1);
        assert!((pairs[0].0, pairs[0].1) == (NetId(0), NetId(1)) && (pairs[0].2 - 0.1).abs() < 1e-6, "{pairs:?}");
    }

    #[test]
    fn a_nonlinear_row_is_not_exported() {
        let (cfg, start) = gain_cfg();
        let nl = sens_netlist();
        let t = SensTable { scenario: 0, at: Parasitics::default(), base: start.clone(), rows: sens_rows(false), sims: 0 };
        let ev = to_evidence(&cfg, std::slice::from_ref(&t), &start, &[stat(None)], &nl);
        let s = &ev.specs[0];
        assert!(s.d_c.is_empty() && s.d_r.is_empty() && s.d_cc.is_empty(), "{s:?}");
        assert_eq!(s.d_vt.len(), 2, "gate offset rows follow σ_f's rule: {s:?}");
        assert_eq!(s.sigma_f, None);
        let (r, pairs) = router_weights(&[t], &[(0, 2.0, 0)], &nl);
        assert!(r.iter().all(|&w| w == 0.0) && pairs.is_empty(), "{r:?} {pairs:?}");
    }

    #[test]
    fn score_reports_the_spread() {
        let specs = vec![Spec { metric: "gain".into(), min: Some(40.0), max: None }];
        let r = score(&specs, &[vec![Some(60.0)], vec![Some(55.0)], vec![Some(70.0)]], &[0, 1, 2]);
        assert_eq!(r.spread, vec![Some((55.0, 70.0))]);
        let r = score(&specs, &[vec![Some(60.0)], vec![None], vec![Some(70.0)]], &[0, 1, 2]);
        assert_eq!(r.spread, vec![None]);
    }
}

/// Step-2 coverage: scoring, the deck, the perturbation helpers, parameter
/// selection and the exports, without ngspice. Oracles: the doc contracts,
/// hand-worked values, and round trips (`base_value ∘ perturb`).
#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use pnr_core::{Device, DeviceKind, Net, NetId};

    fn spec(min: Option<f64>, max: Option<f64>) -> Spec {
        Spec { metric: "m".into(), min, max }
    }

    fn nets(names: &[&str]) -> Vec<Net> {
        names.iter().map(|n| Net { name: (*n).into() }).collect()
    }

    /// `M1`: D=out(0) G=in(1) S=B=vss(2).
    fn one_fet() -> Netlist {
        Netlist {
            devices: vec![Device {
                name: "M1".into(),
                kind: DeviceKind::Nmos,
                model: String::new(),
                terminals: vec![("D".into(), NetId(0)), ("G".into(), NetId(1)), ("S".into(), NetId(2)), ("B".into(), NetId(2))],
                params: vec![("w".into(), 2000), ("l".into(), 500)],
            }],
            nets: nets(&["out", "in", "vss"]),
            ..Default::default()
        }
    }

    fn cfg(specs: Vec<Spec>, testbenches: Vec<String>) -> PerfConfig {
        PerfConfig { sim: OpConfig::default(), testbenches, specs, scenarios: Vec::new() }
    }

    #[test]
    fn miss_of_a_window_and_of_nan() {
        let w = spec(Some(10.0), Some(20.0));
        assert_eq!(miss(&w, Some(15.0)), 0.0);
        assert_eq!(miss(&w, Some(10.0)), 0.0, "on the bound is met");
        assert!((miss(&w, Some(5.0)) - 0.5).abs() < 1e-12);
        assert!((miss(&w, Some(30.0)) - 0.5).abs() < 1e-12);
        assert_eq!(miss(&spec(None, None), Some(1e9)), 0.0, "no bound, no miss");
        assert_eq!(miss(&spec(None, None), None), 1.0, "unmeasured is a full miss even unbounded");
        assert!((miss(&spec(Some(-10.0), None), Some(-15.0)) - 0.5).abs() < 1e-12, "a negative floor scales by its magnitude");
        assert_eq!(miss(&w, Some(f64::NAN)), 1.0, "NaN is unmeasured");
    }

    #[test]
    fn side_miss_reads_one_bound() {
        let w = spec(Some(10.0), Some(20.0));
        assert_eq!(side_miss(&w, true, Some(5.0)), 0.0, "the ceiling ignores a floor miss");
        assert!((side_miss(&w, false, Some(5.0)) - 0.5).abs() < 1e-12);
        assert!((side_miss(&w, true, Some(30.0)) - 0.5).abs() < 1e-12);
        assert_eq!(side_miss(&w, false, None), 1.0);
    }

    #[test]
    fn score_of_no_scenarios_is_unmeasured() {
        let specs = [spec(Some(1.0), None), spec(None, None)];
        let r = score(&specs, &[], &[]);
        assert_eq!(r.metrics, vec![("m".to_string(), None), ("m".to_string(), None)]);
        assert!(r.bounds.is_empty());
        assert_eq!(r.miss, vec![1.0, 1.0]);
        assert_eq!(r.residual, 2.0);
        assert!(!r.met());
        assert_eq!(r.spread, vec![None, None]);
    }

    #[test]
    fn an_infinite_bound_is_not_a_bound_result() {
        let r = score(&[spec(Some(f64::NEG_INFINITY), Some(5.0))], &[vec![Some(1.0)]], &[0]);
        assert_eq!(r.bounds.len(), 1);
        assert!(r.bounds[0].upper);
        assert!(r.met());
    }

    #[test]
    fn ties_go_to_the_earlier_scenario() {
        let r = score(&[spec(Some(0.0), Some(10.0))], &[vec![Some(5.0)], vec![Some(5.0)]], &[3, 7]);
        assert_eq!((r.bounds[0].scenario, r.bounds[1].scenario), (3, 3));
    }

    /// A NaN measurement is unmeasured wherever it sits: the bound's worst
    /// is that scenario, the spec a full miss, no spread.
    #[test]
    fn a_nan_measurement_is_unmeasured() {
        for rows in [vec![vec![Some(f64::NAN)], vec![Some(5.0)]], vec![vec![Some(5.0)], vec![Some(f64::NAN)]]] {
            let r = score(&[spec(Some(0.0), None)], &rows, &[0, 1]);
            assert_eq!(r.bounds[0].value, None, "{rows:?}");
            assert_eq!(r.miss, vec![1.0]);
            assert_eq!(r.spread, vec![None]);
        }
    }

    #[test]
    fn met_is_a_zero_residual() {
        assert!(PerfResult::default().met());
        assert!(!PerfResult { residual: 1e-12, ..Default::default() }.met());
    }

    #[test]
    fn scenarios_default_to_the_sim_corner() {
        let mut c = cfg(Vec::new(), Vec::new());
        c.sim.corner = "ff".into();
        c.sim.temp_c = -40.0;
        let s = c.scenarios();
        assert_eq!(s.len(), 1);
        assert_eq!((s[0].name.as_str(), s[0].corner.as_str(), s[0].temp_c), ("ff", "ff", -40.0));
        assert!(s[0].params.is_empty());
        c.scenarios = vec![Scenario { name: "a".into(), corner: "ss".into(), temp_c: 125.0, params: Vec::new() }];
        assert_eq!(c.scenarios()[0].name, "a");
    }

    #[test]
    fn parse_measures_rejects_what_is_not_a_measure() {
        let m = parse_measures("GAIN = 4.0\n = 1\na b = 2\nx = failed\ny=3e-3 at= 1\nno equals here\n");
        assert_eq!(m, vec![("gain".to_string(), 4.0), ("y".to_string(), 3e-3)]);
        assert!(parse_measures("").is_empty());
    }

    #[test]
    fn assemble_takes_the_last_line_and_refuses_two_benches() {
        let c = cfg(vec![Spec { metric: "Gain".into(), min: None, max: None }, Spec { metric: "pm".into(), min: None, max: None }], Vec::new());
        let row = assemble(&c, &[vec![("gain".into(), 1.0), ("gain".into(), 2.0)], Vec::new()]).unwrap();
        assert_eq!(row, vec![Some(2.0), None]);
        let e = assemble(&c, &[vec![("gain".into(), 1.0)], vec![("gain".into(), 2.0)]]).unwrap_err();
        assert!(e.contains("metric Gain measured by testbenches 0 and 1"), "{e}");
        assert_eq!(assemble(&c, &[]).unwrap(), vec![None, None]);
    }

    /// No testbench: nothing runs (no ngspice needed) and every spec is an
    /// unmeasured full miss; an unknown scenario index is an error.
    #[test]
    fn evaluate_without_testbenches_and_with_a_bad_scenario() {
        let c = cfg(vec![spec(Some(1.0), None)], Vec::new());
        let r = evaluate(&one_fet(), &Parasitics::default(), &c, &[0]).unwrap();
        assert_eq!((r.residual, r.bounds.len(), r.bounds[0].value), (1.0, 1, None));
        let r = evaluate(&one_fet(), &Parasitics::default(), &c, &[]).unwrap();
        assert!(r.bounds.is_empty() && r.residual == 1.0);
        let c = cfg(vec![spec(Some(1.0), None)], vec![String::new()]);
        let e = evaluate(&one_fet(), &Parasitics::default(), &c, &[5]).unwrap_err();
        assert!(e.contains("no scenario 5"), "{e}");
    }

    #[test]
    fn sensitivities_without_testbenches_are_unmeasured() {
        let c = cfg(vec![spec(Some(1.0), None)], Vec::new());
        let p = [Param::GateOffset { device: 0 }, Param::GroundC { net: NetId(0) }];
        let t = sensitivities(&one_fet(), &c, 0, &p, &[], &StepPolicy::default(), &Parasitics::default()).unwrap();
        assert_eq!(t.rows.len(), 2);
        assert!(t.rows.iter().all(|r| r.d == vec![None] && r.linear), "no quotient: vacuously linear");
        assert_eq!(t.sims, 0);
        assert_eq!(t.base.metrics[0].1, None);
        assert!(sensitivities(&one_fet(), &c, 2, &p, &[], &StepPolicy::default(), &Parasitics::default()).unwrap_err().contains("no scenario 2"));
    }

    #[test]
    fn equivalent_sa_corners() {
        assert_eq!(equivalent_sa_um(0.0, 0.5, 1), None);
        assert_eq!(equivalent_sa_um(-1.0, 0.5, 1), None);
        assert_eq!(equivalent_sa_um(f64::NAN, 0.5, 1), None);
        assert_eq!(equivalent_sa_um(1e9, 0.5, 1), None, "more stress than any S gives");
        assert_eq!(equivalent_sa_um(1.0, 0.5, 0), equivalent_sa_um(1.0, 0.5, 1), "nf < 1 reads as 1");
        // One finger: 1/(S + L/2) = target/2 ⇒ S = 2/target − L/2.
        let s = equivalent_sa_um(0.5, 1.0, 1).unwrap();
        assert!((s - 3.5).abs() < 1e-9, "{s}");
    }

    #[test]
    fn the_deck_skips_degenerate_capacitors() {
        let caps = vec![
            ("out".to_string(), None, 0.0),
            ("out".to_string(), Some("in".to_string()), -1.0),
            ("out".to_string(), Some("out".to_string()), 1.0),
            ("vss".to_string(), None, 1.0),
            ("in".to_string(), None, f64::NAN),
            ("in".to_string(), Some("vss".to_string()), 2.0),
        ];
        let c = cfg(Vec::new(), vec![String::new()]);
        let d = deck(&one_fet(), &Parasitics { caps, ..Parasitics::default() }, &c, "", &c.scenarios()[0]).unwrap();
        let pex: Vec<&str> = d.lines().filter(|l| l.starts_with("Cpex")).collect();
        assert_eq!(pex, ["Cpex5 in 0 2.000000e0f"], "{d}");
    }

    #[test]
    fn the_deck_drops_non_positive_resistance_and_a_non_finite_offset() {
        let par = Parasitics {
            series: vec![vec![("D".into(), -5.0), ("S".into(), 0.0)]],
            gate_ohm: vec![Some(-3.0)],
            gate_offset_v: vec![f64::NAN],
            ..Parasitics::default()
        };
        let c = cfg(Vec::new(), vec![String::new()]);
        let d = deck(&one_fet(), &par, &c, "", &c.scenarios()[0]).unwrap();
        assert!(!d.contains("Rpex"), "{d}");
        assert!(!d.contains("Vgo"), "a NaN offset is no source: {d}");
        assert!(d.lines().any(|l| l.starts_with("XM1 out in 0 0 ")), "{d}");
    }

    /// A gate with both a branch R and an offset: net → R → offset → gate.
    #[test]
    fn the_deck_chains_gate_resistance_and_offset() {
        let par = Parasitics { gate_ohm: vec![Some(10.0)], gate_offset_v: vec![2e-3], ..Parasitics::default() };
        let c = cfg(Vec::new(), vec![String::new()]);
        let d = deck(&one_fet(), &par, &c, "", &c.scenarios()[0]).unwrap();
        assert!(d.contains("Rpex_0_G in__0_G in 10.0000"), "{d}");
        assert!(d.contains("Vgo0 in__0_G__o0 in__0_G 2.000000e-3"), "{d}");
        assert!(d.lines().any(|l| l.starts_with("XM1 out in__0_G__o0 0 0 ")), "{d}");
    }

    #[test]
    fn the_deck_leaves_out_capacitor_cards_when_extracted() {
        let mut nl = one_fet();
        nl.devices.push(Device { name: "C1".into(), kind: DeviceKind::Capacitor, model: String::new(), terminals: vec![("P".into(), NetId(0)), ("N".into(), NetId(2))], params: vec![("c_af".into(), 500)] });
        let c = cfg(Vec::new(), vec![String::new()]);
        let sc = &c.scenarios()[0];
        assert!(deck(&nl, &Parasitics::default(), &c, "", sc).unwrap().contains("CXC1 out 0 500a"));
        assert!(!deck(&nl, &Parasitics { extracted: true, ..Parasitics::default() }, &c, "", sc).unwrap().contains("CXC1"));
    }

    #[test]
    fn perturb_then_read_back_adds_the_step() {
        let nl = one_fet();
        let at = Parasitics { caps: vec![("in".into(), Some("out".into()), 0.5)], ..Parasitics::default() };
        let params = [
            Param::GroundC { net: NetId(0) },
            Param::CouplingC { a: NetId(0), b: NetId(1) },
            Param::SeriesR { device: 0, terminal: 2 },
            Param::GateOffset { device: 0 },
        ];
        for p in params {
            let before = base_value(&nl, &at, p);
            let after = base_value(&nl, &perturb(&nl, &at, p, 250.0), p);
            assert!((after - before - 250.0).abs() < 1e-9, "{p:?}: {before} → {after}");
            let twice = perturb(&nl, &perturb(&nl, &at, p, 250.0), p, -250.0);
            assert!((base_value(&nl, &twice, p) - before).abs() < 1e-9, "{p:?}");
        }
        // The coupling entry is found in either order: no second entry.
        assert_eq!(base_value(&nl, &at, Param::CouplingC { a: NetId(0), b: NetId(1) }), 500.0);
        assert_eq!(perturb(&nl, &at, Param::CouplingC { a: NetId(0), b: NetId(1) }, 1.0).caps.len(), 1);
    }

    #[test]
    fn steps_follow_the_policy() {
        let nl = one_fet();
        let pol = StepPolicy { c_min_af: 10.0, c_frac: 0.5, gate_af_um2: 100.0, r_ohm: 7.0, lin_tol: 0.1 };
        // Gate area on `in`: 2 µm × 0.5 µm = 1 µm² → 100 aF → half is 50.
        assert!((step(&nl, Param::GroundC { net: NetId(1) }, &[], &pol) - 50.0).abs() < 1e-9);
        assert_eq!(step(&nl, Param::GroundC { net: NetId(0) }, &[], &pol), 10.0, "no gate: the minimum");
        assert_eq!(step(&nl, Param::SeriesR { device: 0, terminal: 0 }, &[], &pol), 7.0);
        assert_eq!(step(&nl, Param::GateOffset { device: 0 }, &[Some(4e-3)], &pol), 4e-3);
        assert_eq!(step(&nl, Param::GateOffset { device: 0 }, &[None], &pol), 1e-3);
        assert_eq!(step(&nl, Param::GateOffset { device: 3 }, &[], &pol), 1e-3);
    }

    #[test]
    fn default_params_in_order() {
        let mut nl = one_fet();
        nl.devices.push(Device { name: "R1".into(), kind: DeviceKind::Resistor, model: String::new(), terminals: vec![("P".into(), NetId(0)), ("N".into(), NetId(3))], params: Vec::new() });
        nl.nets.push(Net { name: "lonely".into() });
        let p = default_params(&nl, &[NetId(1)]);
        assert_eq!(
            p,
            vec![
                Param::GroundC { net: NetId(1) },
                Param::SeriesR { device: 0, terminal: 0 },
                Param::SeriesR { device: 0, terminal: 2 },
                Param::SeriesR { device: 1, terminal: 0 },
                Param::GateOffset { device: 0 },
            ],
            "R1's N sits alone on `lonely`; the FET's S shares vss with its own B"
        );
        assert!(default_params(&Netlist::default(), &[]).is_empty());
    }

    fn empty_table() -> SensTable {
        SensTable { scenario: 0, at: Parasitics::default(), base: PerfResult::default(), rows: Vec::new(), sims: 0 }
    }

    #[test]
    fn coupling_params_skip_ground_and_respect_max() {
        let nl = one_fet(); // out, in, vss (ground)
        let t = empty_table();
        assert!(coupling_params(&t, &nl, &[NetId(0)], 0).is_empty());
        assert_eq!(coupling_params(&t, &nl, &[NetId(0), NetId(1)], 9), vec![Param::CouplingC { a: NetId(0), b: NetId(1) }], "one pair, not two");
        assert!(coupling_params(&t, &nl, &[NetId(2)], 9).is_empty(), "the ground net couples through GroundC only");
    }

    fn row(param: Param, d: Option<f64>, linear: bool) -> SensRow {
        SensRow { param, step: 1.0, d: vec![d], linear }
    }

    #[test]
    fn router_weights_skip_bounds_without_headroom_or_table() {
        let nl = one_fet();
        let t = SensTable { rows: vec![row(Param::SeriesR { device: 0, terminal: 0 }, Some(2.0), true)], ..empty_table() };
        for h in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let (r, pairs) = router_weights(std::slice::from_ref(&t), &[(0, h, 0)], &nl);
            assert!(r.iter().all(|&w| w == 0.0) && pairs.is_empty(), "h = {h}: {r:?}");
        }
        let (r, _) = router_weights(std::slice::from_ref(&t), &[(0, 1.0, 4)], &nl);
        assert!(r.iter().all(|&w| w == 0.0), "no table at scenario 4");
        let (r, _) = router_weights(&[], &[], &nl);
        assert_eq!(r, vec![0.0; 3]);
        let (r, _) = router_weights(std::slice::from_ref(&t), &[(0, 1.0, 0)], &nl);
        assert_eq!(r, vec![1.0, 0.0, 0.0]);
    }

    #[test]
    fn evidence_skips_unbounded_and_unmeasured_specs() {
        let c = PerfConfig { scenarios: vec![Scenario { name: "a".into(), corner: "tt".into(), temp_c: 27.0, params: Vec::new() }; 2], ..cfg(vec![spec(None, None), spec(Some(1.0), None), spec(Some(1.0), None)], Vec::new()) };
        let start = score(&c.specs, &[vec![Some(5.0), Some(5.0), None], vec![Some(6.0), Some(4.0), Some(2.0)]], &[0, 1]);
        let t0 = SensTable { base: score(&c.specs, &[vec![Some(5.0), Some(5.0), None]], &[0]), ..empty_table() };
        let t1 = SensTable { scenario: 1, base: score(&c.specs, &[vec![Some(6.0), Some(4.0), Some(2.0)]], &[1]), ..empty_table() };
        let ev = to_evidence(&c, &[t0, t1], &start, &[], &one_fet());
        // Spec 0 has no bound; spec 2's tightest bound is the unmeasured scenario 0, whose base is None.
        assert_eq!(ev.specs.len(), 1, "{ev:?}");
        let s = &ev.specs[0];
        assert_eq!(s.f0, 4.0, "read at the worst scenario (1)");
        assert_eq!(s.proc, Some((4.0, 5.0)), "two scenarios: the spread is the process term");
        assert_eq!(s.sigma_f, None, "no stats");
    }

    #[test]
    fn run_jobs_of_nothing_is_nothing() {
        let out: Vec<u8> = run_jobs(&[] as &[u8], |&x| x);
        assert!(out.is_empty());
        assert_eq!(run_jobs(&[3u8], |&x| x + 1), vec![4]);
    }
}
