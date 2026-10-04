//! # `library` — the whole flow as one deterministic function.
//!
//! [`run`] reads top-down: parse → annotate → cells → place (`gp` → `dp`) →
//! route (`gr` → `dr`) → signoff, repeated until the best epoch stops
//! improving. [`signoff`] is the final DRC/ERC/LVS/PEX gate.

mod cellgen;
mod fill;
mod geometry;
mod parse;
pub use parse::{spice_report, spice_with, ParseOptions, ParseReport, SizeConvention};

/// Substrate3 elaboration: build a `macro_master::Composition` against a PDK
/// and route its declared nets — the "PDK on the fly" entry.
pub mod elaborate;
pub use elaborate::{elaborate, stack as parasitic_stack, ElabConfig, Elaborated};

/// Decompile a solved [`Solution`] into a PDK-agnostic generator.
pub mod emit;
/// GDSII stream writer.
pub mod gds;
/// Constraint-budget report: met, met-without-margin, or violated per family.
pub mod metadata;
/// DC operating point via ngspice — the per-device power the thermal rules need.
pub mod oppoint;
pub mod perf;
pub mod reliability;
pub mod robust;

/// Test gates for external tools (FLOW-14), shared by the unit and
/// integration tests: a missing tool skips with a printed reason, and under
/// `PHILIS_REQUIRE_TOOLS=1` (CI's nightly job) panics, so a missing tool is
/// never a green run.
#[doc(hidden)]
pub mod tools {
    /// `true` when `what` is present; otherwise a panic under
    /// `PHILIS_REQUIRE_TOOLS=1`, else an `eprintln!` and `false`.
    pub fn present_or_skip(what: &str, present: bool) -> bool {
        if !present {
            assert!(
                std::env::var_os("PHILIS_REQUIRE_TOOLS").is_none_or(|v| v != "1"),
                "PHILIS_REQUIRE_TOOLS=1 and {what} is missing"
            );
            eprintln!("{what} unavailable — skipping");
        }
        present
    }

    /// `bin` counts as present when it spawns at all (`--version` exits either
    /// way); only a failed spawn, i.e. not on PATH, is absent.
    pub fn tool_or_skip(bin: &str) -> bool {
        present_or_skip(bin, std::process::Command::new(bin).arg("--version").output().is_ok())
    }

    /// ngspice and the sky130 ngspice library (`$PDK_ROOT`, else `~/.volare`,
    /// `/sky130A/libs.tech/ngspice/sky130.lib.spice`), gated as [`present_or_skip`].
    pub fn sky130_models() -> Option<std::path::PathBuf> {
        if !tool_or_skip("ngspice") {
            return None;
        }
        let root = std::env::var_os("PDK_ROOT")
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| std::path::Path::new(&h).join(".volare")));
        let lib = root.map(|r| r.join("sky130A/libs.tech/ngspice/sky130.lib.spice"));
        let found = lib.as_ref().is_some_and(|l| l.is_file());
        present_or_skip(&format!("sky130 models ({lib:?})"), found).then(|| lib.unwrap())
    }
}

use annotator::{annotate, AnnotationConfig, Problem};
pub use dp::PlaceStats;
pub use geometry::PlacementMetrics;
pub use macro_master::Macros;
use pnr_core::{DeviceId, LayerId, Layout, Macro, Report, Routes};
use verify::Pdk;
/// The GDS/geometry viewer, re-exported so callers get it through this crate.
pub use visualizer;

/// Repeatable-run configuration.
pub struct Config {
    /// Base RNG seed; every epoch derives its own from it.
    pub seed: u64,
    /// Epochs per variant assignment (place → route → signoff).
    pub feedback_iters: u32,
    /// Variant assignments to try when an assignment stalls infeasible. `1`
    /// keeps the priced seed assignment.
    pub outer_iters: u32,
    /// User overrides for the annotator's recognition.
    pub annotation: AnnotationConfig,
    /// Per-device dissipation, µW, indexed like `netlist.devices` — the thermal
    /// rules' input. Empty means a uniform die (thermal rules pass vacuously).
    pub device_power_uw: Vec<i32>,
    /// Solve the DC operating point with ngspice; overrides `device_power_uw`.
    /// A failed solve falls back to `device_power_uw` and the report says so.
    pub op: Option<oppoint::OpConfig>,
    /// Score every epoch by simulating the extracted circuit against these
    /// specs: a failed spec outranks every budget. `None` = geometry-only.
    pub performance: Option<perf::PerfConfig>,
    /// Independent searches from derived seeds, run in parallel; the
    /// lexicographically best wins. Start 0 uses `seed`, so `1` is the
    /// single-start flow. One SA run certifies nothing (its optimum theorem
    /// needs infinite chains, Lampaert 1999 pp.117–119).
    pub starts: u32,
    /// Declared utilization floor: cells fill at least this share of their
    /// bounding box (a priced budget, so it outranks C; see
    /// `analog::placement::utilization`). `0` disables it.
    pub min_utilization: f32,
    /// What a MOS card's `W` means; [`run`] stores it as the SPICE total.
    /// Only [`run`] reads it: [`parse`] is always [`SizeConvention::Spice`].
    pub size_convention: SizeConvention,
    /// What gp does before dp; [`GpMode::Pile`] measures gp's contribution.
    pub gp_mode: GpMode,
    /// Fixed die and boundary pins. [`run`] checks each pin names a port;
    /// nothing else reads it yet (PLC/RTE consume it).
    pub interface: Option<Interface>,
    /// The top sub-circuit ([`ParseOptions::top`]); `None`: the parser's choice.
    pub top: Option<String>,
}

/// A die edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    North,
    South,
    East,
    West,
}

/// One boundary pin: `frac` ∈ [0, 1] along `side` (from its low end).
#[derive(Clone, Debug, PartialEq)]
pub struct IoPin {
    pub net: String,
    pub side: Side,
    pub frac: f32,
    pub width_nm: i32,
    /// Deck layer name (`met3`).
    pub layer: String,
}

/// A block's fixed outline and boundary pins.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Interface {
    /// `(w, h)`; `None`: the placer sizes the die.
    pub die_nm: Option<(i32, i32)>,
    pub pins: Vec<IoPin>,
}

impl Interface {
    /// `{"die": {"w": nm, "h": nm}, "pins": [{"net", "side": "north"|"south"|"east"|"west",
    /// "frac", "width": nm, "layer"}]}` (`benchmarks/fixtures/ota_constrained.interface.json`).
    pub fn from_json(text: &str) -> Result<Interface, String> {
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("interface: {e}"))?;
        let int = |o: &serde_json::Value, k: &str| -> Result<i32, String> {
            o.get(k).and_then(serde_json::Value::as_i64).and_then(|x| i32::try_from(x).ok()).ok_or_else(|| format!("interface: `{k}` must be an integer, nm"))
        };
        let die_nm = match v.get("die") {
            None | Some(serde_json::Value::Null) => None,
            Some(d) => Some((int(d, "w")?, int(d, "h")?)),
        };
        let pins = v.get("pins").and_then(serde_json::Value::as_array).map_or(&[][..], Vec::as_slice);
        let pins = pins
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let text = |k: &str| p.get(k).and_then(serde_json::Value::as_str).ok_or_else(|| format!("interface: pins[{i}].{k} must be a string"));
                let side = match text("side")? {
                    "north" => Side::North,
                    "south" => Side::South,
                    "east" => Side::East,
                    "west" => Side::West,
                    s => return Err(format!("interface: pins[{i}].side {s:?} is not north|south|east|west")),
                };
                let frac = p.get("frac").and_then(serde_json::Value::as_f64).filter(|f| (0.0..=1.0).contains(f));
                let frac = frac.ok_or_else(|| format!("interface: pins[{i}].frac must be in [0, 1]"))? as f32;
                Ok(IoPin { net: text("net")?.to_string(), side, frac, width_nm: int(p, "width")?, layer: text("layer")?.to_string() })
            })
            .collect::<Result<_, String>>()?;
        Ok(Interface { die_nm, pins })
    }
}

/// Coarse-placement strategy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GpMode {
    /// gp's analytic loop refines the seeded pile.
    #[default]
    Analytic,
    /// dp starts from gp's seeded pile, unrefined.
    Pile,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            seed: 42,
            feedback_iters: 200,
            outer_iters: 4,
            annotation: AnnotationConfig::default(),
            device_power_uw: Vec::new(),
            op: None,
            performance: None,
            starts: 3,
            min_utilization: 0.6,
            size_convention: SizeConvention::Spice,
            gp_mode: GpMode::default(),
            interface: None,
            top: None,
        }
    }
}

/// A finished placement + routing (pre-signoff).
pub struct Solution {
    pub layout: Layout,
    pub routes: Routes,
    /// Placed cells indexed like `layout`, then guard rings (absolute coords).
    pub macros: Vec<Macro>,
    /// The parsed schematic — signoff's LVS reference.
    pub netlist: pnr_core::Netlist,
    pub stats: RunStats,
    pub metadata: metadata::MetadataReport,
    /// The placement rules the search scored, retargeted to `layout`'s cell ids.
    pub placement: analog::Requirements<Layout>,
    /// The routing rules dr scored `routes` against.
    pub routing: analog::Requirements<Routes>,
    /// Supplies and their currents, for signoff's EM/IR rules (empty without an
    /// operating point).
    pub intent: verify::Intent,
    /// The fold table the cells were drawn at ([`cellgen::folds`]); LVS
    /// expands the schematic by it.
    pub folds: Vec<(u16, i32)>,
    /// The deck's nwell: bridged wells merge into one rect in [`Solution::geometry`].
    pub well_layer: Option<LayerId>,
    /// The operating point the run was biased with; `None` without one.
    pub op: Option<oppoint::OpPoint>,
    /// Recognised matched pairs ([`matched_pairs`]): signoff's REL-10 rows.
    pub pairs: Vec<(DeviceId, DeviceId)>,
    /// Schematic devices per cell, indexed like `layout`.
    pub devices_of: Vec<Vec<DeviceId>>,
    /// The winner's detailed-routing report and stats (`dr`'s own rows:
    /// `open net`, `metal over gate`, …; the pairs it routed exactly).
    pub route: Report,
    pub route_stats: dr::RouteStats,
}

/// How the search went, and the winning epoch's per-stage legality.
#[derive(Clone, Copy, Debug, Default)]
pub struct RunStats {
    /// Epochs executed, over every variant assignment.
    pub iterations: u32,
    /// Index of the winning epoch within its assignment.
    pub best_iteration: u32,
    /// Stopped feasible with stationary constraint prices, none saturated.
    /// Feasible excludes the `lvs-coverage/` rows (devices no deck extracts,
    /// the same on every layout), so `converged` does not imply LVS-complete:
    /// [`metadata::MetadataReport::certified`] does. An undrawable device does
    /// count, so it never reads converged.
    pub converged: bool,
    /// Dual steps on the constraint prices: one per epoch, so it equals
    /// `iterations` (T6).
    pub dual_steps: u32,
    /// Variant assignments tried.
    pub outer_iterations: u32,
    /// Times an assignment stalled infeasible and the variants were changed.
    pub variant_escalations: u32,
    /// Alternatives the escalation odometer skips as dominated in
    /// `(DRC+ERC, w, h)` (GAP-16), summed over the cells of the winner's topology.
    pub pruned: u32,
    /// Winner: detailed-placement hard violations.
    pub place_hard: usize,
    /// Winner: detailed-routing hard violations.
    pub route_hard: usize,
    /// Winner: signoff (DRC + ERC + LVS) errors over its drawn geometry.
    pub drc_hard: usize,
    /// Winner: deck warning rows ([`verify::Signoff::warnings`]) — reported,
    /// never in `drc_hard` or |V|.
    pub warnings: u32,
    /// Winner: Σ routing budget margins (milli-budgets, not tracks).
    pub route_overuse: i64,
    /// Winner: its placement measured after dp ([`PlacementMetrics`]).
    pub place: PlacementMetrics,
    /// Winner: dp's anneal counters.
    pub dp: PlaceStats,
    /// Winner: its key's C tier ([`signoff_c_tier`]) over the epoch's own
    /// signoff, before fill: spec-headroom share with sensitivity rows, else
    /// signal-class C, fF. `Report::cost` stays the total.
    pub c_tier: f32,
    /// Post-layout simulations that could not run ([`perf::evaluate`] `Err`),
    /// over every epoch of every start and cell topology, not just the
    /// winner's; each scored its epoch as every spec unmeasured.
    pub sim_failures: u32,
    /// CPU ms per stage [`STAGES`], summed over every epoch of every start and
    /// topology (threads overlap: not wall time).
    pub stage_ms: [f64; 9],
    /// ngspice decks run: sensitivities plus every scored epoch, over every
    /// start and cell topology.
    pub sims: u32,
}

/// [`RunStats::stage_ms`]'s stages: `route` is gr+dr, `reroute` dr again
/// after antenna diodes, `perf` the post-layout simulation.
pub const STAGES: [&str; 9] = ["gp", "dp", "rings", "route", "reroute", "diodes", "signoff", "metadata", "perf"];

/// Anything that stops the flow.
#[derive(Debug)]
pub enum FlowError {
    Parse(String),
    /// An injected macro for a FET instance does not extract to exactly one
    /// device: `(instance, devices extracted, None = extraction failed)`. It
    /// would unpair LVS for the whole circuit, so it is refused up front.
    InjectedNotADevice(String, Option<usize>),
    /// [`Config::interface`] names a net that is not a port of the top cell.
    Interface(String),
}

/// Epochs without improvement before an assignment counts as stalled.
const PATIENCE: u32 = 8;
/// `‖λ_{k+1} − λ_k‖` below which constraint prices count as stationary.
const PRICE_STATIONARY: f64 = 1e-3;

/// Renames each device's model to the deck's (`Pdk::deck_model`), so the
/// simulator's library, LVS and the recipes all read one name. A model the
/// deck lacks stays as written.
pub fn deck_models(netlist: &mut pnr_core::Netlist, pdk: &Pdk) {
    for d in &mut netlist.devices {
        if let Some(m) = pdk.deck_model(&d.model) {
            d.model = m;
        }
    }
}

/// Every model the deck recognises (its `device` rows) and every sidecar
/// resistor recipe's `model`/`aliases`, with the kind the parser files an `X`
/// card naming it under ([`ParseOptions::models`]). A MOS is P-type when its
/// S/D terminal layer, or its marker, is computed from the `psdm` role (the
/// marker for gf180, whose n and p devices share one `sd` layer); a BJT is a
/// PNP when its model has a `pnp` token, else an NPN.
#[must_use]
pub fn model_table(pdk: &Pdk) -> Vec<(String, pnr_core::DeviceKind)> {
    use pnr_core::{DeviceKind as K, Process as _};
    use verify::DeviceKind as D;
    let d = &pdk.deck.devices;
    let psdm = pdk.layer("psdm");
    let p_type = |x| psdm.is_some_and(|l| pdk.reaches(x, l.0));
    let mut out: Vec<(String, K)> = (0..d.kind.len())
        .map(|row| {
            let model = pdk.strings.resolve(d.model[row]).to_string();
            let kind = match d.kind[row] {
                D::Mos if p_type(d.terminal[d.terminal_start[row] as usize + 1]) || p_type(d.marker[row]) => K::Pmos,
                D::Mos => K::Nmos,
                D::Bjt if model.to_ascii_lowercase().split('_').any(|t| t == "pnp") => K::Pnp,
                D::Bjt => K::Npn,
                D::Resistor => K::Resistor,
                D::Capacitor => K::Capacitor,
                D::Diode => K::Diode,
            };
            (model, kind)
        })
        .collect();
    let recipes = pdk.cell.get("resistors").and_then(|t| t.get("recipes")).and_then(|r| r.as_object());
    for r in recipes.into_iter().flat_map(|rs| rs.values()) {
        let aliases = r.get("aliases").and_then(|a| a.as_array()).into_iter().flatten().filter_map(|a| a.as_str());
        out.extend(aliases.chain(r.get("model").and_then(|m| m.as_str())).map(|m| (m.to_string(), K::Resistor)));
    }
    out
}

/// Place and route a SPICE netlist against a PDK. Deterministic for `cfg.seed`.
///
/// `injected` maps instance names to user-drawn macros: those devices are used
/// as drawn, never reshaped or moved by `dp`.
pub fn run(spice: &str, pdk: &Pdk, injected: &Macros, cfg: &Config) -> Result<Solution, FlowError> {
    // 1. Parse, naming each device by the deck's model.
    let opts = ParseOptions { size: cfg.size_convention, models: model_table(pdk), top: cfg.top.clone(), ..Default::default() };
    let mut netlist = parse::spice_with(spice, &opts).map_err(FlowError::Parse)?;
    deck_models(&mut netlist, pdk);
    if let Some(i) = &cfg.interface {
        let is_port = |n: &str| netlist.ports.iter().any(|p| netlist.nets[p.0 as usize].name == n);
        let bad: Vec<&str> = i.pins.iter().map(|p| p.net.as_str()).filter(|n| !is_port(n)).collect();
        if !bad.is_empty() {
            return Err(FlowError::Interface(format!("interface pins on non-port nets {bad:?}")));
        }
    }

    check_injected(&netlist, injected, pdk)?;

    // Annotated once here for the sensitivity rows; each topology annotates
    // its own `Problem` with the same `ann` (one leaked stack per run).
    let stack: &'static analog::routing::Stack = Box::leak(Box::new(elaborate::stack(pdk)));
    let mut ann = annotation_with(pdk, &cfg.annotation, stack);
    ann.process.die_temp_k = cfg.op.as_ref().map(|o| o.temp_c as f32 + 273.15);
    let base = annotate(&netlist, &ann);

    // 2. Bias: per-device power and per-net current. Placement-independent,
    //    so solved once.
    let bias = bias(&netlist, cfg);
    let plan = performance_rows(&netlist, cfg, &ann, &base.net_classes);

    // 3–7 per cell topology. A distinct-gate pair merged as ABBA cancels a
    // linear gradient but splits one drain across the row ends (asymmetric
    // routing); apart, it routes as translated copies. Neither dominates in
    // general, so when a merge like that exists both are solved and the
    // lexicographically better kept.
    // Both topologies are built once on this thread (pricing every variant
    // once); the starts only search them.
    let merged = topology(&netlist, injected, pdk, cfg, &bias, &ann, &plan, true);
    let apart = merged.distinct.then(|| topology(&netlist, injected, pdk, cfg, &bias, &ann, &plan, false));
    let tops: Vec<&Topology> = std::iter::once(&merged).chain(apart.as_ref()).collect();
    let tops = &tops;
    let runs: Vec<Vec<Searched>> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..cfg.starts.max(1))
            .map(|j| {
                s.spawn(move || {
                    let seed = cfg.seed.wrapping_add(u64::from(j).wrapping_mul(0x9E37_79B9_7F4A_7C15));
                    tops.iter().map(|t| search(t, cfg, seed)).collect()
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().expect("a search start panicked")).collect()
    });
    // Failures and stage times count over every start and topology, not just
    // the winner's: a failed simulation scores its epoch unmeasured, so
    // selection would hide them.
    let all = || runs.iter().flatten().map(|r| r.stats);
    let sim_failures = all().map(|s| s.sim_failures).sum();
    let sims = all().map(|s| s.sims).sum::<u32>() + plan.sims;
    let mut stage_ms = [0.0; 9];
    for s in all() {
        stage_ms.iter_mut().zip(s.stage_ms).for_each(|(a, b)| *a += b);
    }
    // Per start, `apart` wins only when strictly better; over starts, the
    // lex-best wins and ties go to the earliest (`key_lt` is not transitive
    // inside a C band, so this order is the contract).
    let pick = |r: &[Searched]| usize::from(r.len() > 1 && key_lt(&r[1].key, &r[0].key));
    let mut win = (0, pick(&runs[0]));
    for (j, r) in runs.iter().enumerate().skip(1) {
        let k = pick(r);
        if key_lt(&r[k].key, &runs[win.0][win.1].key) {
            win = (j, k);
        }
    }
    let searched = runs.into_iter().nth(win.0).expect("one start at least").swap_remove(win.1);
    let t = if win.1 == 0 { merged } else { apart.expect("index 1 is `apart`") };
    let mut sol = finish(t, searched, &bias, pdk);
    (sol.stats.sim_failures, sol.metadata.sim_failures) = (sim_failures, sim_failures);
    sol.stats.stage_ms = stage_ms;
    sol.stats.sims = sims;
    sol.metadata.budget_rows = plan.notes;
    sol.metadata.sensitivity = plan.sens;
    Ok(sol)
}

/// Each spec bound as a shared routing budget from schematic sensitivities
/// (see [`perf::budget_rows`]): one baseline plus one run per signal net, in
/// parallel, once per run. Empty without performance scoring, a simulator, or
/// the deck's wire capacitance. Also, per declared bound (`"{metric}:min"` /
/// `":max"`), its row or why it has none ([`metadata::MetadataReport::budget_rows`]).
/// Also the active scenarios (PERF-10): nominal (index 0) plus each bound's
/// worst scenario on the schematic, in order; every scenario when the
/// schematic cannot be evaluated. Epochs are simulated over these only.
/// The notes open with `"scenario {name}: active"` / `": inactive"` per
/// scenario, ahead of the per-bound notes. Per active scenario a sensitivity
/// table (PERF-11), noted in [`PerfPlan::sens`].
fn performance_rows(netlist: &pnr_core::Netlist, cfg: &Config, ann: &AnnotationConfig, classes: &[analog::metadata::NetClassification]) -> PerfPlan {
    use analog::metadata::NetClass;
    let plan = |rows, notes, active| PerfPlan { rows, notes, active, tables: Vec::new(), sigma_v: Vec::new(), sens: Vec::new(), sims: 0 };
    let Some(p) = &cfg.performance else { return plan(Vec::new(), Vec::new(), vec![0]) };
    let all: Vec<usize> = (0..p.scenarios().len()).collect();
    let bounds = || {
        p.specs.iter().flat_map(|s| {
            [(s.min, "min"), (s.max, "max")].into_iter().filter(|b| b.0.is_some_and(f64::is_finite)).map(move |(_, side)| format!("{}:{side}", s.metric))
        })
    };
    let notes = |rows: &[analog::routing::PerformanceBudget], why: &str| -> Vec<String> {
        let notes: Vec<String> = bounds()
            .map(|b| match rows.iter().find(|r| r.metric == b) {
                Some(r) if r.nets.is_empty() => format!("{b}: row with no measured nets"),
                Some(r) if r.limit > 0.0 => format!("{b}: row ({} nets)", r.nets.len()),
                Some(_) => format!("{b}: do-not-worsen row (the schematic misses it)"),
                None => format!("{b}: no row ({why})"),
            })
            .collect();
        notes.iter().for_each(|n| eprintln!("[perf] {n}"));
        notes
    };
    let scenario_notes = |active: &[usize]| -> Vec<String> {
        let notes: Vec<String> = p
            .scenarios()
            .iter()
            .enumerate()
            .map(|(i, sc)| format!("scenario {}: {}", sc.name, if active.contains(&i) { "active" } else { "inactive" }))
            .collect();
        notes.iter().for_each(|n| eprintln!("[perf] {n}"));
        notes
    };
    let Some(af_per_um) = ann.process.wire_af_per_um else {
        let mut out = scenario_notes(&all);
        out.extend(notes(&[], "deck has no wire capacitance"));
        return plan(Vec::new(), out, all);
    };
    let nets: Vec<pnr_core::NetId> = classes
        .iter()
        .filter(|c| matches!(c.class, NetClass::Signal | NetClass::Sensitive | NetClass::Clock))
        .map(|c| c.net)
        .collect();
    let steps = perf::StepPolicy { gate_af_um2: ann.process.gate_af_per_um2.map_or(0.0, f64::from), ..Default::default() };
    let params = perf::default_params(netlist, &nets);
    let sigma_v = robust::device_sigma_v(netlist, ann.process.avt_mv_um);
    let names = p.scenarios();
    let sens = perf::evaluate(netlist, &perf::Parasitics::default(), p, &all).and_then(|start| {
        let mut active = vec![0];
        for b in &start.bounds {
            if !active.contains(&b.scenario) {
                active.push(b.scenario);
            }
        }
        let mut sens_notes = Vec::new();
        let mut tables = Vec::new();
        for &s in &active {
            let t0 = std::time::Instant::now();
            let mut t = perf::sensitivities(netlist, p, s, &params, &sigma_v, &steps, &perf::Parasitics::default())?;
            perf::add_coupling(&mut t, netlist, p, &nets, &steps, 64)?;
            let name = &names[s].name;
            sens_notes.push(format!("{name}: {} rows, {} sims, {} ms", t.rows.len(), t.sims, t0.elapsed().as_millis()));
            sens_notes.extend(t.rows.iter().filter(|r| !r.linear).map(|r| format!("{name}: nonlinear {:?}", r.param)));
            tables.push(t);
        }
        Ok((start, active, tables, sens_notes))
    });
    match sens {
        Ok((start, active, tables, sens_notes)) => {
            let rows = perf::budget_rows(p, &start, &tables, &nets, af_per_um / 1000.0);
            let mut out = scenario_notes(&active);
            out.extend(notes(&rows, "not measured at the schematic"));
            sens_notes.iter().for_each(|n| eprintln!("[perf] sens {n}"));
            let sims = (all.len() * p.testbenches.len()) as u32 + tables.iter().map(|t| t.sims).sum::<u32>();
            PerfPlan { rows, notes: out, active, tables, sigma_v, sens: sens_notes, sims }
        }
        Err(e) => {
            let mut out = scenario_notes(&all);
            out.extend(notes(&[], &format!("sensitivities unavailable: {e}")));
            plan(Vec::new(), out, all)
        }
    }
}

/// What the run scores performance with, solved once on the schematic
/// ([`performance_rows`]).
struct PerfPlan {
    /// Spec bounds as routing budget rows.
    rows: Vec<analog::routing::PerformanceBudget>,
    /// [`metadata::MetadataReport::budget_rows`].
    notes: Vec<String>,
    /// Scenarios each scored epoch is simulated at.
    active: Vec<usize>,
    /// One sensitivity table per active scenario, in `active` order.
    tables: Vec<perf::SensTable>,
    /// Per device random V_T σ, V ([`robust::device_sigma_v`]); the gate
    /// offset step of the tables.
    sigma_v: Vec<Option<f64>>,
    /// [`metadata::MetadataReport::sensitivity`].
    sens: Vec<String>,
    /// ngspice decks run on the schematic.
    sims: u32,
}

/// The operating point, solved once per run.
struct Bias {
    power: Vec<i32>,
    summary: Option<metadata::BiasSummary>,
    /// Per device, the DC current each terminal draws (`None`: unresolved).
    currents: Option<Vec<Option<Vec<(String, f64)>>>>,
    /// Per net, the smallest saturation headroom on it, mV (IR budgets).
    net_headroom_mv: Option<Vec<Option<f64>>>,
    /// Per device transconductance, µS (fold floor for gate R); empty = none.
    gm_us: Vec<Option<f64>>,
    /// The operating point the run was biased with; `None` without one.
    op: Option<oppoint::OpPoint>,
}

/// What is fixed for the run at one cell topology (`merge_distinct_gates`
/// lets a matched pair on different gate nets merge into one cell): the
/// annotated problem, the cells, EM/IR rules, router config and the seed
/// variant assignment. Built once on the calling thread and shared by every
/// start's [`search`].
struct Topology<'a> {
    flow: Flow<'a>,
    /// [`cellgen::seed_assignment`]: every alternative DRC-priced once.
    assignment0: Vec<u16>,
    /// Per cell, the alternatives [`cellgen::escalate`] may visit.
    allowed: Vec<Vec<u16>>,
    /// A distinct-gate merge happened, so the `apart` topology is worth solving.
    distinct: bool,
    /// REL-05: EM temperature, K; `None` without an op.
    t_em_k: Option<f32>,
    /// Where the EM derating came from (`BiasSummary::em_derate`).
    em_derate: &'static str,
}

// Starts share one `Topology` by reference across threads.
const _: fn() = || {
    fn s<T: Sync>() {}
    s::<Topology<'static>>();
};

#[cfg(test)]
thread_local!(static APART_BUILDS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) });

/// Annotate (with `ann`, whose stack the run leaked once) and draw cells at one
/// cell topology.
#[allow(clippy::too_many_arguments)]
fn topology<'a>(
    netlist: &'a pnr_core::Netlist,
    injected: &Macros,
    pdk: &'a Pdk,
    cfg: &'a Config,
    bias: &Bias,
    ann: &AnnotationConfig,
    perf: &'a PerfPlan,
    merge_distinct_gates: bool,
) -> Topology<'a> {
    #[cfg(test)]
    if !merge_distinct_gates {
        APART_BUILDS.with(|c| c.set(c.get() + 1));
    }
    let currents = &bias.currents;
    // Annotate: placement/routing rules + cell constraints, device-indexed.
    // Per topology: `CellSpace::new` mutates the problem, which is not `Clone`.
    let mut problem = annotate(netlist, ann);
    if cfg.min_utilization > 0.0 {
        problem.placement.budget.push(Box::new(analog::placement::utilization::Utilization { u_min: cfg.min_utilization }));
    }
    let (perf_rows, perf_active) = (&perf.rows[..], &perf.active[..]);
    for row in perf_rows {
        problem.routing.budget.push(Box::new(row.clone()));
    }

    // Cells: every legal variant drawn once; matched groups collapse to one
    // cell and every device-indexed table moves to cell space.
    // One fold table for the cells and every LVS reference of this run.
    let unit_cells: Vec<(Vec<DeviceId>, bool)> = problem.constraints.unitization.iter().map(|u| (u.devices.clone(), u.route_matching_required)).collect();
    let fold = cellgen::folds(&netlist, pdk, &bias.gm_us, &unit_cells);
    let cells = CellSpace::new(&netlist, injected, &mut problem, pdk, &bias.power, merge_distinct_gates, &fold);
    let locks = dp::locks::locks(&problem.placement, cells.variants.len(), &cells.variants);
    let distinct = cells.distinct_gate_merges > 0;

    // 5. Stages. The metal stack and router config come from the deck.
    // ponytail: a bad deck still panics here; FLOW-11 propagates the `Err`.
    let stack = elaborate::routing_stack(pdk, None).unwrap_or_else(|e| panic!("routing stack: {e}"));
    let (layers, cuts, pin_access) = (stack.layers.clone(), stack.cuts.clone(), stack.pin_access);
    // EM limits on the pin-access layer and cut too (sky130 mcon 0.36 mA/cut):
    // the access jogs and pin cuts carry their terminal's current.
    let em_layers: Vec<LayerId> = layers.iter().copied().chain(pin_access.map(|p| p.0)).collect();
    let em_cuts: Vec<elaborate::Cut> = cuts.iter().copied().chain(pin_access.map(|p| p.1)).collect();
    // REL-05: EM is derated at T_amb + θ_JA·P_total + the worst on-die rise
    // any placement can give ([`pnr_core::thermal::rise_bound_mc`]). Per cell,
    // the variant whose eq. 5.6 self term is largest: it bounds every
    // variant's mutual term too.
    // ponytail: a cell's bbox is wider than its heated channel, which
    // understates the rise; read the channel area from `Macro.units` if a
    // bias ever heats a cell enough to matter.
    let t_em_k = cfg.op.as_ref().map(|o| {
        use pnr_core::thermal::{rise_bound_mc, self_rise_mc, K_SI_W_PER_M_K as K};
        let foot: Vec<(i32, i32)> = cells
            .variants
            .iter()
            .zip(&cells.power)
            .map(|(v, &p)| {
                v.alternatives.iter().map(|m| (m.bbox.w, m.bbox.h)).max_by(|a, b| self_rise_mc(p, a.0, a.1, K).total_cmp(&self_rise_mc(p, b.0, b.1, K))).unwrap_or((1, 1))
            })
            .collect();
        let package = o.theta_ja_c_per_w.unwrap_or(0.0) * bias.summary.as_ref().map_or(0, |s| s.total_power_uw) as f64 * 1e-6;
        o.temp_c as f32 + 273.15 + package as f32 + rise_bound_mc(&cells.power, &foot, K) / 1e3
    });
    let em = elaborate::em_limits(pdk, &em_layers, &em_cuts, t_em_k);
    let em_derate = match layers.first().and_then(|&l| pdk.em_limit(l)) {
        Some(verify::EmLimit { derating: Some(_), derating_assumed: true, .. }) => "sidecar+fallback Ea/n",
        Some(verify::EmLimit { derating: Some(_), .. }) => "deck",
        _ => "none",
    };
    em_rules(&mut problem, netlist, &em, &em_layers, &em_cuts, ann.process.stack, pdk);
    // IR-drop budgets (PWR-02) on nets carrying op current (`annotator::ir`).
    let ir = if let (Some(c), Some(h)) = (&bias.currents, &bias.net_headroom_mv) {
        let vdd_mv = cfg.op.as_ref().map_or(1_800.0, |o| o.vdd * 1e3);
        let i = oppoint::net_current_ua(netlist, c);
        let ir = annotator::ir::budgets(&problem.net_classes, &i, h, vdd_mv, &ann.policy);
        let rules: Vec<analog::routing::IrDrop> = ir
            .iter()
            .map(|&(net, current_ua, max_drop_uv)| analog::routing::IrDrop { net, current_ua, max_drop_uv, margin_pct: 20, stack: ann.process.stack })
            .collect();
        problem.routing.budget.push(Box::new(rules));
        ir
    } else {
        problem.missing.push(("IrDrop", "operating point"));
        Vec::new()
    };
    problem.missing.extend(analog::matching::class::missing_tiers(pdk).map(|m| ("MatchClass", m)));
    let sens: Vec<(pnr_core::NetId, f32)> =
        perf_rows.iter().flat_map(|r| r.nets.iter().copied().zip(r.weights.iter().copied())).collect();
    let net_weight = gp::net_weights(&problem.net_classes, &sens);
    let intent = elaborate::intent(netlist, &problem.net_classes, currents.as_deref(), cfg.op.as_ref().map_or(0.0, |o| o.vdd * 1_000.0), &ir);
    let flow = Flow {
        pdk,
        netlist,
        net_names: netlist.nets.iter().map(|n| n.name.clone()).collect(),
        d_router: {
            let mut r = elaborate::detailed_router(pdk, &stack);
            r.cfg.supply_nets = problem
                .net_classes
                .iter()
                .filter(|c| matches!(c.class, analog::metadata::NetClass::Supply | analog::metadata::NetClass::Ground))
                .map(|c| c.net)
                .collect();
            r.cfg.pin_ua = currents.as_deref().map_or_else(Vec::new, |c| pin_currents(netlist, &cells.devices_of, c));
            // Per cell, each member's gate pin, device and `W·L·m` (the
            // annotator's antenna gate area, µm² → nm²).
            r.cfg.gate_nm2 = cells
                .devices_of
                .iter()
                .map(|members| members.iter().enumerate().map(|(k, d)| (format!("d{k}:G"), u32::from(d.0), (netlist.devices[d.0 as usize].gate_area_um2() * 1e6) as i64)).collect())
                .collect();
            r.cfg.em = em;
            // Routing prices parasitics only on nets something budgets, on [0, 1].
            let budgeted = |n: usize| problem.net_classes.get(n).is_some_and(|c| c.c_budget_af.is_some()) || sens.iter().any(|(s, _)| usize::from(s.0) == n);
            let w: Vec<f32> = net_weight.iter().enumerate().map(|(n, &w)| if budgeted(n) { w } else { 0.0 }).collect();
            let top = w.iter().copied().fold(0.0f32, f32::max);
            r.cfg.net_weight = w.iter().map(|&x| if top > 0.0 { x / top } else { 0.0 }).collect();
            r
        },
        layers,
        cuts,
        problem,
        cells,
        locks,
        perf: cfg.performance.as_ref(),
        perf_rows,
        perf_active,
        perf_plan: perf,
        intent,
        net_weight,
        fold,
        stack: ann.process.stack.expect("`annotation` always carries the stack"),
        id_ua: currents
            .as_ref()
            .map(|c| c.iter().map(|d| d.as_ref().and_then(|t| t.iter().find(|(n, _)| n == "D").map(|&(_, i)| i))).collect())
            .unwrap_or_default(),
        gp_mode: cfg.gp_mode,
    };
    // Matched cells keep every alternative: a merged group, or a member of a
    // 2-device leaf that emits a `MatchedSet` (annotator emit.rs table).
    let paired: Vec<DeviceId> = annotator::block::leaves(&flow.problem.blocks)
        .into_iter()
        .filter(|l| l.devices.len() == 2 && matches!(l.kind, annotator::BlockKind::DiffPair | annotator::BlockKind::CurrentMirror | annotator::BlockKind::Load | annotator::BlockKind::CascodePair))
        .flat_map(|l| l.devices.iter().copied())
        .collect();
    let matched: Vec<bool> = flow.cells.devices_of.iter().map(|m| m.len() > 1 || m.iter().any(|d| paired.contains(d))).collect();
    // GAP-18: a cell inside an Exceptional unitization lists its alternatives best-matching first.
    let ranked: Vec<bool> = flow
        .cells
        .devices_of
        .iter()
        .map(|m| flow.problem.constraints.unitization.iter().any(|u| u.class == Some(pnr_core::MatchClass::Exceptional) && m.iter().all(|d| u.devices.contains(d))))
        .collect();
    let (assignment0, allowed) = cellgen::seed_assignment(&flow.cells.variants, &matched, &ranked, pdk);
    Topology { flow, assignment0, allowed, distinct, t_em_k, em_derate }
}

/// One start's search on a shared topology: the winning epoch, the start's
/// stats, the saturated prices (`metadata.binding`) and the winner's key.
struct Searched {
    best: Epoch,
    stats: RunStats,
    binding: Vec<String>,
    key: LexKey,
}

/// 6. Search. Outer: variant assignment. Middle: epochs at that assignment,
///    keeping the best [`LexKey`], whose V includes the epoch's own signoff
///    errors. Prices and routing history persist across epochs and are this
///    start's own.
fn search(t: &Topology, cfg: &Config, seed: u64) -> Searched {
    let flow = &t.flow;
    let mut assignment = t.assignment0.clone();
    let mut prices = gp::Prices::new();
    let mut neg = gr::Negotiation::new();
    let mut best: Option<Epoch> = None;
    let mut stats = RunStats::default();

    let n_outer = cfg.outer_iters.max(1);
    for outer in 0..n_outer {
        stats.outer_iterations += 1;
        let mut stall = 0;
        for iter in 0..cfg.feedback_iters.max(1) {
            stats.iterations += 1;
            let seed = seed ^ u64::from(iter) ^ (u64::from(outer) << 32);
            // Alternate: dp's reshape trades a squarer variant for HPWL, which
            // the utilization floor cannot see mid-anneal (a 1-row tail cost
            // ota 40% area); the epoch key picks between the two.
            let mut epoch = flow.epoch(&assignment, iter % 2 == 1, &mut prices, &mut neg, seed);
            // Promotion: simulate only a candidate whose hard count can still
            // beat the incumbent; its spec miss then decides against it.
            if best.as_ref().is_none_or(|b| epoch.key.0 <= b.key.0) {
                flow.score_perf(&mut epoch, &mut stats);
            }
            for (s, e) in stats.stage_ms.iter_mut().zip(epoch.stats.stage_ms) {
                *s += e;
            }
            if best.as_ref().is_none_or(|b| key_lt(&epoch.key, &b.key)) {
                best = Some(Epoch {
                    iteration: iter,
                    ..epoch
                });
                stall = 0;
            } else {
                stall += 1;
                if stall >= PATIENCE {
                    break;
                }
            }
        }

        // Feasible with settled prices: done. A price held at its cap reads as
        // settled in `drift` but is still binding, so it blocks convergence.
        let feasible = best
            .as_ref()
            .is_some_and(|b| b.key.0 == 0 && b.key.1 <= 0.0 && b.key.2 <= 0.0);
        if feasible && prices.drift() < PRICE_STATIONARY && prices.saturated().is_empty() {
            stats.converged = true;
            break;
        }
        // Not converged: infeasible, or feasible with prices still moving or
        // saturated. Try the next variant assignment, unless the budget or the
        // variant space is exhausted. ponytail: a feasible run that once
        // saturated a λ relaxes it by ρ·slack per epoch, ρ ≥ RHO_FLOOR = 0.25:
        // −64 at slack 0.5 with ρ at the floor is 512 epochs of drift above
        // PRICE_STATIONARY, so it escalates rather than converges; FLOW-08's
        // `RunStats.stop` will show it.
        if outer + 1 == n_outer {
            break;
        }
        let Some(next) = cellgen::escalate(&flow.cells.variants, &t.allowed, &assignment) else {
            break;
        };
        stats.variant_escalations += 1;
        assignment = next;
    }

    let best = best.expect("at least one epoch ran");
    stats.dual_steps = prices.steps();
    let all: usize = flow.cells.variants.iter().map(|v| v.alternatives.len()).sum();
    stats.pruned = (all - t.allowed.iter().map(Vec::len).sum::<usize>()) as u32;
    let stats = RunStats {
        best_iteration: best.iteration,
        ..best.stats.merge(stats)
    };
    let binding = prices.saturated().iter().map(|k| (*k).to_string()).collect();
    Searched { key: best.key, best, stats, binding }
}

/// 7. The winner only, redrawn from its own variant choice, with its guard
/// rings, fill and metadata. Consumes `t`, so the placement rules move into
/// [`Solution::placement`].
fn finish(t: Topology, s: Searched, bias: &Bias, pdk: &Pdk) -> Solution {
    let (flow, best, stats, t_em_k, em_derate) = (t.flow, s.best, s.stats, t.t_em_k, t.em_derate);
    let mut macros = cellgen::realize(&flow.cells.variants, &best.layout.variant);
    // Only a winner claiming zero hard violations must be fully connected; an
    // infeasible winner's opens are already counted and reported at signoff.
    if cfg!(debug_assertions) && best.key.0 == 0 {
        // Under dr's own joins; a diode's shape on the deck's credited layer
        // rides in the routes as the antenna rule's credit (`Flow::epoch`),
        // not as conductor.
        let marker = pdk.antenna_diode_credit().map(|(l, _)| l);
        let wires = best.routes.wires.iter().map(|w| w.iter().filter(|s| Some(s.layer) != marker).copied().collect()).collect();
        let joins = dr::joins(&flow.layers, &flow.cuts, flow.d_router.cfg.pin_access);
        Routes { wires, ..Routes::default() }.debug_check_joined("dr::route (winner)", &joins);
        geometry::debug_check_connected(&macros, &best.layout, &best.routes);
    }
    macros.extend(best.rings.iter().cloned());
    // MFG-01: density fill, once, on the winner; the search never sees it.
    let drawn = geometry::collect(&macros, &best.layout, &best.routes);
    let wires_of = |class: analog::metadata::NetClass| -> Vec<pnr_core::Shape> {
        flow.problem.net_classes.iter().filter(|c| c.class == class)
            .flat_map(|c| best.routes.wires.get(c.net.0 as usize).into_iter().flatten().copied()).collect()
    };
    // Matched cells: more than one member owns its units.
    let matched: Vec<pnr_core::Rect> = pnr_core::place_macros(&macros, &best.layout).iter()
        .filter(|m| m.units.iter().any(|u| u.owner != m.units[0].owner)).map(|m| m.bbox).collect();
    use analog::metadata::NetClass::{Ground, Sensitive};
    macros.extend(fill::fill(&drawn, &wires_of(Ground), &wires_of(Sensitive), &matched, pdk));
    let metadata = metadata::build(
        &flow.problem.placement,
        &best.layout,
        &flow.problem.routing,
        &best.routes,
        bias.summary.clone().map(|mut b| {
            b.em_temp_k = t_em_k.unwrap_or(b.em_temp_k);
            b.em_derate = em_derate;
            b
        }),
        &flow.problem.net_classes,
        &flow.problem.missing,
        &pdk.unverified(),
    );
    let mut metadata = metadata;
    metadata.binding = s.binding;
    let pairs = matched_pairs(&flow.problem.blocks);
    if let Some(op) = &bias.op {
        let (_, aging, unknown) = reliability::voltage_findings(flow.netlist, op, &pdk.fet_voltage_limits(), &pairs, false);
        let name = |d: DeviceId| flow.netlist.devices[d.0 as usize].name.clone();
        metadata.aging = aging.into_iter().map(|a| (name(a.a), name(a.b), a.dvds_mv, a.dvgs_mv, a.dvbs_mv)).collect();
        metadata.voltage_unknown = unknown;
    }
    let mut recognition = std::collections::BTreeMap::new();
    for b in flow.problem.blocks.iter().filter(|b| b.kind != annotator::BlockKind::Glue) {
        *recognition.entry(b.template).or_insert(0) += 1;
    }
    metadata.recognition = recognition.into_iter().collect();
    metadata.unconstrained = flow.problem.coverage.iter()
        .filter_map(|&(d, c)| match c {
            annotator::Coverage::Unconstrained(why) => Some((flow.netlist.devices[d.0 as usize].name.clone(), why)),
            _ => None,
        })
        .collect();
    metadata.coverage = best.coverage;
    metadata.add_routing(&[Box::new(flow.common_nodes(&best.layout)), Box::new(flow.environment(&best.layout, &best.rings))], &best.routes);
    if let (Some(cfg), Some(result)) = (flow.perf, &best.perf) {
        metadata.performance = cfg
            .specs
            .iter()
            .zip(&result.metrics)
            .zip(&result.miss)
            .map(|((s, (m, v)), miss)| (m.clone(), *v, s.min, s.max, *miss))
            .collect();
        let names = cfg.scenarios();
        metadata.performance_worst = result
            .bounds
            .iter()
            .map(|b| {
                let value = b.value.map_or_else(|| "unmeasured".to_string(), |v| format!("{v:.4e}"));
                format!(
                    "{}:{} worst {value} at {} (over {} active of {} scenarios)",
                    cfg.specs[b.spec].metric,
                    if b.upper { "max" } else { "min" },
                    names[b.scenario].name,
                    flow.perf_active.len(),
                    names.len()
                )
            })
            .collect();
        metadata.sim_failures = stats.sim_failures;
        // ponytail: no systematic/gradient terms; MAT's per-pair ledger is not
        // exported at the winner yet.
        let stats = robust::bound_stats(&flow.perf_plan.tables, &flow.perf_plan.sigma_v, result, &cfg.specs, &[], &[]);
        metadata.robustness = stats
            .iter()
            .map(|st| {
                let b = &result.bounds[st.bound];
                let name = format!("{}:{}", cfg.specs[b.spec].metric, if b.upper { "max" } else { "min" });
                let (Some(sf), Some(beta), Some(y)) = (st.sigma_f, st.beta, st.yield_part) else { return format!("{name} UNKNOWN ({})", robust::unknown_reason(&flow.perf_plan.tables, &flow.perf_plan.sigma_v, result, st.bound)) };
                let top: Vec<String> = st.shares.iter().map(|&(d, w)| format!("{} {:.0}%", flow.netlist.devices[d as usize].name, w * 100.0)).collect();
                format!("{name} σ_f {sf:.4e} β {beta:.2} Φ(β) {y:.4} (V_T only) top {}", top.join(", "))
            })
            .collect();
        let y = robust::linear_joint_yield(&flow.perf_plan.tables, &flow.perf_plan.sigma_v, result, &cfg.specs, &[], 100_000, 1);
        metadata.robustness.push(y.map_or_else(|| "joint yield (linear, 1e5) UNKNOWN".into(), |y| format!("joint yield (linear, 1e5) {y:.4}")));
    }
    // Inserted devices (antenna diodes) join the schematic LVS reads.
    let mut netlist = flow.netlist.clone();
    netlist.devices.extend(best.extra);
    Solution {
        layout: best.layout,
        routes: best.routes,
        macros,
        netlist,
        stats,
        metadata,
        placement: flow.problem.placement,
        routing: flow.problem.routing,
        intent: flow.intent,
        folds: flow.fold,
        well_layer: pnr_core::Process::layer(pdk, "nwell"),
        op: bias.op.clone(),
        pairs,
        devices_of: flow.cells.devices_of,
        route: best.route,
        route_stats: best.route_stats,
    }
}

/// Everything an epoch reads that is fixed for the run.
struct Flow<'a> {
    pdk: &'a Pdk,
    /// The schematic (LVS reference) and its net names (label text).
    netlist: &'a pnr_core::Netlist,
    net_names: Vec<String>,
    /// Rules and constraints; placement rules retargeted to cell ids.
    problem: Problem,
    cells: CellSpace,
    /// Matched-cell orient/shape locks (PLC-03), over all variants.
    locks: dp::locks::Locks,
    /// Post-layout performance scoring, when configured.
    perf: Option<&'a perf::PerfConfig>,
    /// Spec bounds as sensitivity rows ([`performance_rows`]); weigh [`c_tier`].
    perf_rows: &'a [analog::routing::PerformanceBudget],
    /// Scenarios each promoted epoch is simulated at ([`performance_rows`]).
    perf_active: &'a [usize],
    /// Sensitivity tables and V_T σ the winner's robustness reads (PERF-13).
    perf_plan: &'a PerfPlan,
    /// Placement HPWL weight per net ([`gp::net_weights`]).
    net_weight: Vec<f32>,
    layers: Vec<LayerId>,
    cuts: Vec<elaborate::Cut>,
    d_router: dr::DetailedRoute,
    /// Signoff's design intent ([`elaborate::intent`]).
    intent: verify::Intent,
    /// The fold table the cells were drawn at ([`cellgen::folds`]).
    fold: Vec<(u16, i32)>,
    /// The routing stack's per-layer R/C (branch resistance).
    stack: &'static analog::routing::Stack,
    /// Per device drain current, µA (`None` = unresolved).
    id_ua: Vec<Option<f64>>,
    gp_mode: GpMode,
}

/// `base` plus what the annotator needs from the deck.
#[must_use]
pub fn annotation(pdk: &Pdk, base: &AnnotationConfig) -> AnnotationConfig {
    annotation_with(pdk, base, Box::leak(Box::new(elaborate::stack(pdk))))
}

/// [`annotation`] on a stack the caller already holds. Rules are `Copy`, so
/// they borrow the stack for 'static. ponytail: [`run`] leaks one per call (a
/// few hundred bytes); cache by deck if runs ever loop in one process.
fn annotation_with(pdk: &Pdk, base: &AnnotationConfig, stack: &'static analog::routing::Stack) -> AnnotationConfig {
    use pnr_core::Process;
    let layers = elaborate::routing_stack(pdk, None).map(|s| s.layers).unwrap_or_default();
    let wire = layers.first().copied();
    let width = wire.and_then(|l| pdk.min_width(l.0)).unwrap_or(0);
    let opt = |key: &str| Some(pdk.rule(key, 0)).filter(|&v| v > 0);
    let pos = |key: &str| pdk.cell_f32(key).filter(|&v| v > 0.0);
    let process = annotator::ProcessNumbers {
        antenna_max_ratio: pdk.antenna_max_ratio(),
        gate_af_per_um2: opt("gate_cap_af_um2").map(|v| v as f32),
        wire_af_per_um: wire.and_then(|l| pdk.wire_af_per_um(l, width)),
        route_space_nm: wire.and_then(|l| pdk.min_spacing(l.0)).unwrap_or(0),
        dti: opt("dti_max_spacing").zip(opt("dti_width")),
        avt_mv_um: [pos("avt_n_mv_um"), pos("avt_p_mv_um")],
        abeta_pct_um: [pos("abeta_n_pct_um"), pos("abeta_p_pct_um")],
        svt_uv_per_um: pos("svt_uv_per_um"),
        svt_fit: pos("svt_a_uv2_per_um2").zip(pos("svt_b_uv2")),
        vt_tc_uv_per_k: [pos("vt_tc_uv_per_k"), pos("vt_tc_uv_per_k_p")],
        lod_kvth0_mv_um: [pos("lod_kvth0_n_mv_um"), pos("lod_kvth0_p_mv_um")],
        lattice_nm: cells::builder::cut_lattice(pdk),
        substrate: pnr_core::SubstrateKind::from_key(pdk.cell_str("substrate_kind")),
        epi_nm: pos("epi_thickness_nm").map(|v| v as i32),
        stack: Some(stack),
        min_ring_width_nm: pdk.rule("min_guard_ring_width", 0),
        ecgr_min_width_nm: opt("ecgr_min_width_nm"),
        ecgr_drawable: cells::post_cell::drawable(analog::cell::GuardRingType::Ecgr, pdk),
        hcgr_drawable: cells::post_cell::drawable(analog::cell::GuardRingType::Hcgr, pdk),
        // Set by the callers from `Config.op` (not a deck key).
        die_temp_k: None,
        unit: annotator::sets::UnitDeck {
            grid_nm: i64::from(pdk.grid()),
            min_w_nm: i64::from(pdk.rule("min_finger_width", 0)),
            max_w_nm: i64::from(pdk.rule("max_finger_width", 0)),
            min_l_nm: pdk.layer("poly").and_then(|l| pdk.min_width(l.0)).map_or(0, i64::from),
            res_min_segment_nm: i64::from(pdk.rule("res_min_segment", 0)),
        },
    };
    AnnotationConfig { process, ..base.clone() }
}

/// One guard-ring tap contact's resistance, ohms: the deck's `pex` value for
/// a `licon` cut on `tap` (the larger of n- and p-tap; 0, no budget check,
/// when the deck has none).
#[must_use]
pub fn ring_cut_ohm(pdk: &Pdk) -> f32 {
    use pnr_core::Process;
    pdk.cut_ohm("licon", "tap").unwrap_or(0.0)
}

/// Placement's process numbers. Origins snap to the cells' cut lattice so
/// every cut stays on it; the gap between cells is the widest spacing of any
/// device layer, so wells and implants of neighbouring cells never merge.
fn place_rules(pdk: &Pdk) -> gp::Rules {
    use pnr_core::Process;
    let clearance = ["nwell", "diff", "tap", "poly", "nsdm", "psdm", "li"]
        .iter()
        .filter_map(|&r| pdk.layer(r))
        .filter_map(|l| pdk.min_spacing(l.0))
        .max()
        .unwrap_or(0);
    gp::Rules { grid: cells::builder::cut_lattice(pdk), clearance }
}

/// One scored epoch.
struct Epoch {
    key: LexKey,
    /// Measured specs, when performance scoring is on and this epoch was
    /// promoted to simulation.
    perf: Option<perf::PerfResult>,
    /// Extracted capacitance, kept for that simulation.
    caps: verify::CapMatrix,
    /// What this epoch's signoff did not check; the winner's goes to
    /// [`metadata::MetadataReport::coverage`].
    coverage: verify::Coverage,
    iteration: u32,
    layout: Layout,
    routes: Routes,
    rings: Vec<Macro>,
    /// Devices dr inserted (antenna diodes), for the schematic; their macros
    /// are in `rings`.
    extra: Vec<pnr_core::Device>,
    stats: RunStats,
    /// dr's report and stats for `routes`.
    route: Report,
    route_stats: dr::RouteStats,
}

impl Flow<'_> {
    /// Place → route → signoff at `assignment`, scored. `reshape` lets dp
    /// swap variants; without it the assignment is kept as drawn.
    fn epoch(
        &self,
        assignment: &[u16],
        reshape: bool,
        prices: &mut gp::Prices,
        neg: &mut gr::Negotiation,
        seed: u64,
    ) -> Epoch {
        let unified = {
            let mut a = assignment.to_vec();
            self.locks.unify(&mut a);
            a
        };
        let assignment = &unified[..];
        let placement = &self.problem.placement;
        let cells = &self.cells;
        let layers = &self.layers;

        // Place: coarse analytical, then legalising anneal (which may reshape).
        let macros = cellgen::realize(&cells.variants, assignment);
        let inp = gp::GpInput {
            macros: &macros,
            variants: &cells.variants,
            assignment,
            reqs: placement,
            rules: place_rules(self.pdk),
            net_weight: &self.net_weight,
            n_axes: self.problem.axis_count,
            power_uw: &cells.power,
            units: cells.units.clone(),
            iterate: self.gp_mode == GpMode::Analytic,
        };
        let mut ms = [0.0; 9];
        let mut clock = std::time::Instant::now();
        let mut lap = |i: usize| {
            ms[i] += clock.elapsed().as_secs_f64() * 1e3;
            clock = std::time::Instant::now();
        };
        let (coarse, _) = gp::place(&inp, prices, seed);
        lap(0);
        coarse.debug_check("gp::place");
        let (mut layout, place_report, dp_stats) = dp::place(
            &coarse,
            &macros,
            if reshape { &cells.variants } else { &[] },
            placement,
            &cells.fixed,
            &self.locks,
            prices,
            place_rules(self.pdk),
            &self.net_weight,
            // Its own stream (AP-19): gp and dp drawing the same sequence correlate their moves.
            seed ^ 0xD1B5_4A32_D192_ED03,
            dp::Schedule::cold(),
        );
        layout.debug_check("dp::place");
        layout.groups = cells.groups.clone();
        // The epoch's one dual step, on the layout it is scored on (T6).
        prices.settle(placement, &layout);
        lap(1);
        let macros = if layout.variant == assignment {
            macros
        } else {
            cellgen::realize(&cells.variants, &layout.variant)
        };
        // Measured on the macros dp's variants draw, so `lattice_off` stamps what is drawn.
        let lattice = cells::builder::cut_lattice(self.pdk);
        let place = geometry::placement_metrics(&macros, &layout, lattice, place_rules(self.pdk).clearance, placement, &self.locks);
        debug_assert_eq!(place.lattice_off, 0, "dp::place: cell origin off the cut lattice");

        // Guard rings enclose placed cells, so they are drawn now, before routing.
        let mut rings = cells::post_cell::guard_rings(&layout, &cells.guard_rings, self.pdk, ring_cut_ohm(self.pdk));
        // Same-bulk PMOS cells facing each other share one well.
        let bridges = cells::post_cell::well_bridges(&gr::place_macros(&macros, &layout), &rings, self.pdk);
        rings.extend(bridges);
        // Same-type implants of neighbours closer than their spacing merge.
        let placed_now: Vec<Macro> = gr::place_macros(&macros, &layout).into_iter().chain(rings.iter().cloned()).collect();
        rings.extend(cells::post_cell::implant_bridges(&placed_now, self.pdk));
        lap(2);

        // Route: track realisation onto the real pins.
        let routing = &self.problem.routing;
        let placed = gr::place_macros(&macros, &layout);
        let pins: Vec<_> = placed
            .iter()
            .flat_map(|m| m.pins.iter().map(|p| (p.net, p.at, p.layer)))
            .collect();
        // The router repairs this placement's shared-source skew itself.
        let router = dr::DetailedRoute {
            // Pin shares from the unplaced macros: `place_macro` leaves units local.
            cfg: dr::DetailedCfg {
                common: self.common_nodes(&layout).nodes,
                blockages: elaborate::blockages(&placed, self.pdk, |c| elaborate::match_class(&self.problem.constraints.unitization, self.cells.devices_of.get(c).map_or(&[][..], Vec::as_slice)), layers),
                aggressor: {
                    let mut a = vec![false; self.netlist.nets.len()];
                    for c in self.problem.net_classes.iter().filter(|c| c.class == analog::metadata::NetClass::Clock) {
                        if let Some(x) = a.get_mut(c.net.0 as usize) {
                            *x = true;
                        }
                    }
                    a
                },
                aggressor_weight: analog::routing::CouplingBudget::default_weights(&self.problem.net_classes, self.netlist.nets.len()),
                stack: Some(self.stack),
                pin_share: macros.iter().map(pnr_core::pin_shares).collect(),
                n_nets: self.netlist.nets.len(),
                ..self.d_router.cfg.clone()
            },
        };
        let (mut routes, mut route_report, mut route_stats) =
            router.route(&pins, &placed, &rings, routing, layers, &self.cuts, neg);
        lap(3);
        // Antenna nets the jumper could not fix get a diode each, routed in as
        // a fixed cell; its shape on the deck's credited diode layer joins the
        // net's routes (the rule's credit, `Stack::diode`).
        let ground = self.problem.net_classes.iter().find(|c| c.class == analog::metadata::NetClass::Ground).map(|c| c.net);
        let diodes = elaborate::antenna_diodes(self.pdk, routing, &routes, &placed, &rings, ground, place_rules(self.pdk).clearance);
        lap(5);
        let mut extra = Vec::new();
        if !diodes.is_empty() {
            let marker = self.pdk.antenna_diode_credit().map(|(l, _)| l);
            let mut marks = Vec::new();
            for (device, m) in diodes {
                let cathode = device.terminals.iter().find(|t| t.0 == "N").map(|t| t.1);
                marks.extend(cathode.zip(m.shapes.iter().find(|s| Some(s.layer) == marker).copied()));
                extra.push(device);
                rings.push(m);
            }
            (routes, route_report, route_stats) = router.route(&pins, &placed, &rings, routing, layers, &self.cuts, neg);
            lap(4);
            for (net, shape) in marks {
                if let Some(w) = routes.wires.get_mut(net.0 as usize) {
                    w.push(shape);
                }
            }
        }
        let netlist = if extra.is_empty() {
            std::borrow::Cow::Borrowed(self.netlist)
        } else {
            let mut n = self.netlist.clone();
            n.devices.extend(extra.iter().cloned());
            std::borrow::Cow::Owned(n)
        };

        // Measure: signoff over the drawn geometry, budget residuals over the result.
        let mut shapes = geometry::collect(&macros, &layout, &routes);
        shapes.extend(rings.iter().flat_map(|r| r.shapes.iter().copied()));
        if let Some(l) = pnr_core::Process::layer(self.pdk, "nwell") {
            geometry::merge_rects(&mut shapes, l);
        }
        // The epoch is scored by the same DRC/ERC/LVS gate as the final result.
        let mut labelled = placed;
        labelled.extend(rings.iter().cloned());
        let mut signoff = signoff_shapes(&self.intent, &shapes, &labelled, &self.net_names, &netlist, Some(&self.fold), self.pdk);
        signoff.report.hard_violations.extend(undrawable(&macros, &self.cells.devices_of, self.netlist));
        lap(6);
        let mut budgets = metadata::build(
            placement,
            &layout,
            routing,
            &routes,
            None,
            &self.problem.net_classes,
            &self.problem.missing,
            &self.pdk.unverified(),
        );
        budgets.add_routing(&[Box::new(self.common_nodes(&layout)), Box::new(self.environment(&layout, &rings))], &routes);

        let c = signoff_c_tier(&signoff, &self.net_names, &self.problem.net_classes, self.perf_rows);
        let (key, stats) = epoch_score(&place_report, &route_report, &signoff, &budgets, c, layout.footprint_nm2());
        lap(7);
        let stats = RunStats { place, dp: dp_stats, stage_ms: ms, ..stats };
        Epoch {
            key,
            perf: None,
            caps: signoff.caps,
            coverage: signoff.coverage,
            iteration: 0,
            layout,
            routes,
            rings,
            extra,
            stats,
            route: route_report,
            route_stats,
        }
    }
}

/// A common node's ΔR budget, Ω: the pair's remaining allowance (mV, either order) over I_D (µA); 0 = unknown.
fn common_node_ohm(left: &[(u32, u32, f32)], a: DeviceId, b: DeviceId, i_ua: Option<f32>) -> f32 {
    let (a, b) = (u32::from(a.0), u32::from(b.0));
    let mv = left.iter().find(|&&(x, y, _)| (x, y) == (a, b) || (x, y) == (b, a)).map(|&(_, _, mv)| mv);
    match (mv, i_ua) {
        (Some(mv), Some(i)) => mv / i * 1e3,
        _ => 0.0,
    }
}

impl Flow<'_> {
    /// The member device and terminal a placed cell pin names ([`pin_member`]),
    /// only when that device has the terminal: a cell's non-terminal pins
    /// (`ring`) name nothing.
    fn member_pin<'n>(&self, members: &[DeviceId], name: &'n str) -> Option<(DeviceId, &'n str)> {
        let (k, t) = pin_member(name)?;
        let &d = members.get(k)?;
        self.netlist.devices[usize::from(d.0)].terminals.iter().any(|(n, _)| n == t).then_some((d, t))
    }

    /// What the epoch's layout adds to the schematic: its extracted C, each
    /// device terminal's routed branch R (parallel pins of one terminal
    /// combine), and each device's mean LOD stress over its drawn fingers.
    fn parasitics(&self, epoch: &Epoch) -> perf::Parasitics {
        let n = self.netlist.devices.len();
        let placed = gr::place_macros(&cellgen::realize(&self.cells.variants, &epoch.layout.variant), &epoch.layout);
        // Per net: (device, terminal, pin rect) of every placed pin.
        let mut pins: Vec<Vec<(usize, String, pnr_core::Rect)>> = vec![Vec::new(); self.netlist.nets.len()];
        for (m, members) in placed.iter().zip(&self.cells.devices_of) {
            for p in &m.pins {
                let Some((d, t)) = self.member_pin(members, &p.name) else { continue };
                if let Some(v) = pins.get_mut(p.net.0 as usize) {
                    v.push((usize::from(d.0), t.to_string(), p.at));
                }
            }
        }
        let mut conductance: Vec<Vec<(String, f32)>> = vec![Vec::new(); n];
        for (net, list) in pins.iter().enumerate() {
            let shapes = epoch.routes.wires.get(net).map_or(&[][..], Vec::as_slice);
            if list.len() < 2 || shapes.is_empty() {
                continue;
            }
            let rects: Vec<pnr_core::Rect> = list.iter().map(|&(_, _, r)| r).collect();
            for ((d, t, _), r) in list.iter().zip(self.stack.terminal_resistance_ohm(shapes, &rects)) {
                let Some(r) = r.filter(|&r| r > 0.0) else { continue };
                let v = &mut conductance[*d];
                match v.iter_mut().find(|(n, _)| n == t) {
                    Some((_, g)) => *g += 1.0 / r,
                    None => v.push((t.clone(), 1.0 / r)),
                }
            }
        }
        let series = conductance.into_iter().map(|v| v.into_iter().map(|(t, g)| (t, 1.0 / g)).collect()).collect();
        let lod_inv_um = (0..n)
            .map(|d| {
                let (s, k) = epoch.layout.units.of_device(&epoch.layout, pnr_core::DeviceId(d as u16)).filter(|u| u.lod.is_finite()).fold((0.0f32, 0u32), |(s, k), u| (s + u.lod, k + 1));
                (k > 0).then(|| s / k as f32)
            })
            .collect();
        perf::Parasitics { caps: epoch.caps.clone(), series, lod_inv_um, extracted: true, gate_offset_v: Vec::new() }
    }

    /// Each recognised matched pair on one source net, with its members'
    /// source pins and the net's other pins (feeds) as placed, budgeted
    /// `ΔR ≤ (allowance − placement spend) / I_D` from the pair's `MatchedSet` ledger.
    fn common_nodes(&self, layout: &Layout) -> analog::routing::CommonNodes {
        use annotator::BlockKind::{CurrentMirror, DiffPair, Load};
        let mut left = Vec::new();
        for b in &self.problem.placement.budget {
            b.offset_allowances(layout, &mut left);
        }
        let placed = gr::place_macros(&cellgen::realize(&self.cells.variants, &layout.variant), layout);
        let term = |d: DeviceId, t: &str| self.netlist.devices[d.0 as usize].terminals.iter().find(|(n, _)| n == t).map(|&(_, n)| n);
        // (device, terminal, rect) of every placed pin, per net.
        let mut on_net: Vec<Vec<(DeviceId, String, pnr_core::Rect)>> = vec![Vec::new(); self.netlist.nets.len()];
        for (m, members) in placed.iter().zip(&self.cells.devices_of) {
            for p in &m.pins {
                let Some((d, t)) = self.member_pin(members, &p.name) else { continue };
                if let Some(v) = on_net.get_mut(p.net.0 as usize) {
                    v.push((d, t.to_string(), p.at));
                }
            }
        }
        let mut nodes = Vec::new();
        for leaf in annotator::block::leaves(&self.problem.blocks) {
            let &[a, b] = leaf.devices.as_slice() else { continue };
            if !matches!(leaf.kind, DiffPair | CurrentMirror | Load) {
                continue;
            }
            let Some(net) = term(a, "S").filter(|&n| term(b, "S") == Some(n)) else { continue };
            let list = &on_net[net.0 as usize];
            let pins = |d: DeviceId| list.iter().filter(|p| p.0 == d && p.1 == "S").map(|p| p.2).collect::<Vec<_>>();
            let feeds = list.iter().filter(|p| p.0 != a && p.0 != b).map(|p| p.2).collect();
            let i_ua = self.id_ua.get(a.0 as usize).copied().flatten().map(|i| i.abs() as f32).filter(|&i| i > 0.0);
            let max_delta_ohm = common_node_ohm(&left, a, b, i_ua);
            // RTE-17 step 4: EXT-24's CommonNodeReq/StarReq/KelvinReq replace
            // this leaf mapping (a StarReq supersedes the net's node, star = true).
            nodes.push(analog::routing::CommonNode { net, groups: vec![pins(a), pins(b)], feeds, max_delta_ohm, star: false });
        }
        let joins = dr::joins(&self.layers, &self.cuts, self.d_router.cfg.pin_access);
        analog::routing::CommonNodes { nodes, stack: self.stack, halo_nm: self.d_router.cfg.pitch, joins }
    }

    /// Each recognised matched pair's surroundings on the placed geometry
    /// (`rings` included): its channels' distance to the nearest nwell-union
    /// edge (WPE) and its cells' diffusion's gap to other cells' (OSE).
    fn environment(&self, layout: &Layout, rings: &[Macro]) -> analog::placement::Environment {
        use annotator::BlockKind::{CurrentMirror, DiffPair, Load};
        use pnr_core::Process;
        let placed = gr::place_macros(&cellgen::realize(&self.cells.variants, &layout.variant), layout);
        let (Some(nwell), Some(diff)) = (self.pdk.layer("nwell"), self.pdk.layer("diff")) else {
            return analog::placement::Environment::default();
        };
        let wells: Vec<pnr_core::Rect> =
            placed.iter().chain(rings).flat_map(|m| &m.shapes).filter(|s| s.layer == nwell).map(|s| s.rect).collect();
        let inside = |r: &pnr_core::Rect, (x, y): (i32, i32)| r.x <= x && x <= r.x + r.w && r.y <= y && y <= r.y + r.h;
        let covered = |p: (i32, i32)| wells.iter().any(|r| inside(r, p));
        let gap = |r: &pnr_core::Rect, (x, y): (i32, i32)| {
            let dx = (r.x - x).max(x - (r.x + r.w)).max(0);
            let dy = (r.y - y).max(y - (r.y + r.h)).max(0);
            f64::from(dx).hypot(f64::from(dy)) as f32
        };
        // Distance from `p` to the nwell union's boundary: to the nearest
        // well outside it, else to the nearest uncovered point just past a
        // well edge (projections onto every edge, and the corners).
        let wpe = |p: (i32, i32)| -> f32 {
            if !covered(p) {
                return wells.iter().map(|r| gap(r, p)).fold(f32::INFINITY, f32::min);
            }
            let mut best = f32::INFINITY;
            for r in &wells {
                let (x0, x1, y0, y1) = (r.x, r.x + r.w, r.y, r.y + r.h);
                let cx = p.0.clamp(x0, x1);
                let cy = p.1.clamp(y0, y1);
                for q in [(x0 - 1, cy), (x1 + 1, cy), (cx, y0 - 1), (cx, y1 + 1), (x0 - 1, y0 - 1), (x1 + 1, y0 - 1), (x0 - 1, y1 + 1), (x1 + 1, y1 + 1)] {
                    if !covered(q) {
                        best = best.min(f64::from(q.0 - p.0).hypot(f64::from(q.1 - p.1)) as f32);
                    }
                }
            }
            best
        };
        let diffs = |c: usize| placed[c].shapes.iter().filter(|s| s.layer == diff).map(|s| s.rect).collect::<Vec<_>>();
        let rect_gap = |a: &pnr_core::Rect, b: &pnr_core::Rect| {
            let dx = (a.x - (b.x + b.w)).max(b.x - (a.x + a.w)).max(0);
            let dy = (a.y - (b.y + b.h)).max(b.y - (a.y + a.h)).max(0);
            f64::from(dx).hypot(f64::from(dy)) as f32
        };
        let mut out = Vec::new();
        for leaf in annotator::block::leaves(&self.problem.blocks) {
            let &[a, b] = leaf.devices.as_slice() else { continue };
            if !matches!(leaf.kind, DiffPair | CurrentMirror | Load) {
                continue;
            }
            let mean_wpe = |d: DeviceId| {
                let (s, n) = layout.units.of_device(layout, d).fold((0.0f32, 0u32), |(s, n), u| (s + wpe((u.x, u.y)).min(1e7), n + 1));
                if n == 0 { f32::INFINITY } else { s / n as f32 }
            };
            let ose = |d: DeviceId| {
                let Some(c) = self.cells.devices_of.iter().position(|m| m.contains(&d)) else { return f32::INFINITY };
                let own = diffs(c);
                (0..placed.len())
                    .filter(|&o| o != c)
                    .flat_map(diffs)
                    .flat_map(|f| own.iter().map(move |m| (f, *m)))
                    .map(|(f, m)| rect_gap(&f, &m))
                    .fold(f32::INFINITY, f32::min)
            };
            out.push(analog::placement::Surroundings {
                wpe_nm: [mean_wpe(a), mean_wpe(b)],
                ose_nm: [ose(a), ose(b)],
                wpe_min_nm: analog::matching::class::mos_env(pnr_core::MatchClass::Moderate, self.pdk).wpe_nm as f32,
                ose_range_nm: self.pdk.tier("lod_moat_ext_nm", pnr_core::MatchClass::Minimal).unwrap_or(0) as f32,
            });
        }
        analog::placement::Environment(out)
    }

    /// Simulate `epoch` on its parasitics; a simulator that cannot run counts
    /// in `stats.sim_failures` and scores every spec unmeasured.
    fn score_perf(&self, epoch: &mut Epoch, stats: &mut RunStats) {
        let Some(p) = self.perf else { return };
        let clock = std::time::Instant::now();
        let unknown = || perf::score(&p.specs, &[vec![None; p.specs.len()]], &[0]);
        let result = if epoch.caps.is_empty() {
            unknown()
        } else {
            stats.sims += (self.perf_active.len() * p.testbenches.len()) as u32;
            perf::evaluate(self.netlist, &self.parasitics(epoch), p, self.perf_active).unwrap_or_else(|e| {
                eprintln!("[perf] {e}");
                stats.sim_failures += 1;
                unknown()
            })
        };
        epoch.key.1 = result.residual;
        epoch.perf = Some(result);
        stats.stage_ms[8] += clock.elapsed().as_secs_f64() * 1e3;
    }
}

impl RunStats {
    /// The winner's per-stage legality with the run-wide counters of `run`.
    fn merge(self, run: RunStats) -> RunStats {
        RunStats {
            place_hard: self.place_hard,
            route_hard: self.route_hard,
            drc_hard: self.drc_hard,
            warnings: self.warnings,
            route_overuse: self.route_overuse,
            place: self.place,
            dp: self.dp,
            c_tier: self.c_tier,
            ..run
        }
    }
}

/// `(|V|, spec miss, Θ, C tier, footprint nm²)`, compared by [`key_lt`]:
/// no parasitic gain buys past a budget residual, no budget slack past a missed
/// circuit spec, nothing past a hard violation. V counts violated hard rules
/// ([`metadata::MetadataReport::hard_violated`]), the stages' own non-batch
/// rows, and signoff errors (deck warnings are never in that report). Θ is
/// [`metadata::MetadataReport::theta`] plus dr's own non-batch budget rows, all
/// in milli-budgets. The spec miss is the post-layout simulation's Σ normalised
/// miss (`0` without performance scoring); the C tier is [`c_tier`] over
/// signoff's extracted matrix, not its total (`Report::cost`), which on ota is
/// 79 % supply-related (AV-06).
type LexKey = (usize, f64, f64, f32, f64);

/// Relative [`c_tier`] difference read as a tie, which area then breaks.
///
/// ponytail: a flat 2%, calibrated on the seed-to-seed spread of total
/// extracted C and not re-measured for the tier; a declared spec on C or area
/// would replace it.
const C_TIE: f32 = 0.02;

/// `a` beats `b`: `|V|`, spec miss, Θ lexicographically, then the C tier —
/// except that C within [`C_TIE`] is a tie decided by footprint. A feasible
/// optimum is a vector (area, C, …) and a scalarisation must be a declared
/// policy (Graeb 2007 ch.1); this is ours. Not transitive inside a C band;
/// callers only ever compare a candidate against the incumbent. A NaN tier
/// reads as +∞, so it loses to any finite value and never sticks as incumbent.
fn key_lt(a: &LexKey, b: &LexKey) -> bool {
    let nan_last = |x: f64| if x.is_nan() { f64::INFINITY } else { x };
    let c = |k: &LexKey| if k.3.is_nan() { f32::INFINITY } else { k.3 };
    let (a3, b3) = (c(a), c(b));
    let head = |k: &LexKey| (k.0, nan_last(k.1), nan_last(k.2));
    if head(a) != head(b) {
        return head(a) < head(b);
    }
    if (a3 - b3).abs() <= C_TIE * a3.abs().min(b3.abs()) {
        nan_last(a.4) < nan_last(b.4)
    } else {
        a3 < b3
    }
}

/// One epoch's key and per-stage counts. Signoff errors only: `signoff.warnings`
/// feed `RunStats::warnings` and nothing else. Spec tier left at 0: the
/// simulation is the expensive step, so the caller runs it (`Flow::score_perf`)
/// only on a candidate that can still win.
fn epoch_score(
    place: &Report,
    route: &Report,
    signoff: &verify::Signoff,
    budgets: &metadata::MetadataReport,
    c_tier: f32,
    footprint_nm2: f64,
) -> (LexKey, RunStats) {
    let key = lex_key(place, route, &signoff.report, budgets, None, c_tier, footprint_nm2);
    let stats = RunStats {
        c_tier,
        place_hard: place.hard_violations.len(),
        route_hard: route.hard_violations.len(),
        drc_hard: signoff.report.hard_violations.len(),
        warnings: signoff.warnings.len() as u32,
        route_overuse: route.budget_violations.iter().map(|v| v.margin).sum(),
        ..RunStats::default()
    };
    (key, stats)
}

fn lex_key(
    place: &Report,
    route: &Report,
    signoff: &Report,
    budgets: &metadata::MetadataReport,
    perf: Option<&perf::PerfResult>,
    c_tier: f32,
    footprint_nm2: f64,
) -> LexKey {
    let own = |r: &Report| r.hard_violations.iter().filter(|v| !v.is_batch_row()).count();
    // `lvs-coverage/` rows (devices no deck recogniser extracts) are the same
    // every epoch: no layout fixes them, so they stay out of |V| or no design
    // with a BJT/MOM could ever read feasible, converge or debug-check its
    // winner. They stay in the report, so signoff and bench still see them.
    // An amendment to PERF-01 step 5 / PERF-02 step 3 (plan-07, master §6.1):
    // `converged` does not imply LVS-complete; `MetadataReport::certified` does.
    let checked = signoff.hard_violations.iter().filter(|v| !v.rule.starts_with("lvs-coverage/")).count();
    let v = budgets.hard_violated() + own(place) + own(route) + checked;
    let theta = budgets.theta()
        + route.budget_violations.iter().filter(|x| !x.is_batch_row()).map(|x| x.margin as f64).sum::<f64>();
    (v, perf.map_or(0.0, |p| p.residual), theta, c_tier, footprint_nm2)
}

/// The epoch key's C tier. With sensitivity rows: Σ_n w⁺_n·C_n(ground) +
/// Σ_(a,b) (w⁺_a + w⁺_b)·C_ab, C in aF, w⁺_n = Σ_rows max(w_jn, 0) — the share
/// of spec headroom the extracted C spends (dimensionless; Lampaert eq. 2.12,
/// adverse side only as in BAL2-16). Without rows: signal-class (`Signal`,
/// `Sensitive`, `Clock`, the nets [`performance_rows`] measures) ground C plus
/// coupling counted once per signal end, fF. A net with no class row, a
/// `Supply`/`Ground`/`Substrate` net, or a name outside `names` weighs 0.
/// `caps` is [`verify::CapMatrix`]; `names` indexes it by `NetId`.
///
/// ponytail: rails are what the name classifier says (`annotator::netrole`).
fn c_tier(
    caps: &verify::CapMatrix,
    names: &[String],
    classes: &[analog::metadata::NetClassification],
    rows: &[analog::routing::PerformanceBudget],
) -> f32 {
    use analog::metadata::NetClass;
    let w = |name: &str| -> f64 {
        let Some(id) = names.iter().position(|n| n == name) else { return 0.0 };
        if rows.is_empty() {
            let signal = classes
                .iter()
                .any(|c| usize::from(c.net.0) == id && matches!(c.class, NetClass::Signal | NetClass::Sensitive | NetClass::Clock));
            return f64::from(u8::from(signal));
        }
        let per_af: f64 = rows
            .iter()
            .flat_map(|r| r.nets.iter().zip(&r.weights))
            .filter(|(n, _)| usize::from(n.0) == id)
            .map(|(_, &w)| f64::from(w.max(0.0)))
            .sum();
        per_af * 1000.0
    };
    caps.iter().map(|(a, b, c)| c * (w(a) + b.as_deref().map_or(0.0, w))).sum::<f64>() as f32
}

/// [`c_tier`] over `signoff`'s extracted matrix, or NaN when signoff reports a
/// label short: extraction then books the shorted nets' C on the one label it
/// keeps (dac4: b0, b1 under VSS, weight 0), so the tier is unknown and
/// [`key_lt`] ranks it last within its |V|. Empty `rows`: signal-class C, fF;
/// `names` are the netlist's net names, `classes` the annotator's.
#[must_use]
pub fn signoff_c_tier(
    signoff: &verify::Signoff,
    names: &[String],
    classes: &[analog::metadata::NetClassification],
    rows: &[analog::routing::PerformanceBudget],
) -> f32 {
    let short = format!("lvs/{}", verify::checker::LABEL_SHORT);
    if signoff.report.hard_violations.iter().any(|v| v.rule.starts_with(&short)) {
        return f32::NAN;
    }
    c_tier(&signoff.caps, names, classes, rows)
}

/// `v` rounded up to a multiple of `grid` (`v ≥ 0`, `grid > 0`).
fn round_up(v: i32, grid: i32) -> i32 {
    (v + grid - 1) / grid * grid
}

/// Every injected FET macro extracts to exactly one device (the one it
/// replaces). Other families are skipped: some (capacitors) are LVS reference
/// skips and legitimately extract to none.
fn check_injected(netlist: &pnr_core::Netlist, injected: &Macros, pdk: &Pdk) -> Result<(), FlowError> {
    let mut checker: Option<verify::Checker> = None;
    for d in &netlist.devices {
        if !matches!(d.kind, pnr_core::DeviceKind::Nmos | pnr_core::DeviceKind::Pmos) {
            continue;
        }
        let Some(m) = injected.get(&d.name) else { continue };
        let c = checker.get_or_insert_with(|| verify::Checker::new(pdk, true).expect("a loaded Pdk re-parses its own deck"));
        match c.device_count(&m.shapes) {
            Some(1) => {}
            n => return Err(FlowError::InjectedNotADevice(d.name.clone(), n)),
        }
    }
    Ok(())
}

/// One hard [`analog::routing::Electromigration`] per routed (≥ 2-terminal)
/// net: every routed segment and via against its layer's derated deck limit,
/// on the currents `dr` records per terminal (`Routes::terms`; unknown
/// without an operating point or with an unresolved device on the net). A
/// routed or pin-access layer (`metals`, `cuts`) with no deck limit is listed
/// once as a missing input naming those layers: their shapes go unchecked.
/// So are deck limits past the rule's `MAX_LAYERS` slots, which it cannot hold.
fn em_rules(
    problem: &mut Problem,
    netlist: &pnr_core::Netlist,
    em: &[(pnr_core::LayerId, analog::routing::em::Limit)],
    metals: &[LayerId],
    cuts: &[elaborate::Cut],
    stack: Option<&'static analog::routing::Stack>,
    pdk: &Pdk,
) {
    use analog::routing::em::{Limit, MAX_LAYERS};
    let mut limits = [(u16::MAX, Limit::default()); MAX_LAYERS];
    for (slot, &(l, lim)) in limits.iter_mut().zip(em) {
        *slot = (l.0, lim);
    }
    if em.len() > MAX_LAYERS {
        problem.missing.push(("Electromigration", "slot for every deck EM limit (more than MAX_LAYERS = 16; the rest unchecked)"));
    }
    let has = |l: LayerId, f: fn(&Limit) -> f32| em.iter().any(|(x, lim)| *x == l && f(lim) > 0.0);
    let name = |l: LayerId| pdk.layers.iter().find(|(_, x)| *x == l).map_or_else(|| format!("layer {}", l.0), |(n, _)| n.clone());
    let unchecked: Vec<String> = metals
        .iter()
        .filter(|&&l| !has(l, |e| e.ua_per_um))
        .chain(cuts.iter().map(|(c, ..)| c).filter(|&&c| !has(c, |e| e.ua_per_cut)))
        .map(|&l| name(l))
        .collect();
    if !unchecked.is_empty() {
        // ponytail: leaked once per run (`missing` holds `&'static str`), as the stack is.
        let input = format!("deck EM limit on every routed and pin-access layer ({} unchecked)", unchecked.join(", "));
        problem.missing.push(("Electromigration", Box::leak(input.into_boxed_str())));
    }
    let mut terminals = vec![0usize; netlist.nets.len()];
    for (_, net) in netlist.devices.iter().flat_map(|d| &d.terminals) {
        terminals[net.0 as usize] += 1;
    }
    let rules: Vec<analog::routing::Electromigration> = (0..terminals.len())
        .filter(|&k| terminals[k] >= 2)
        .map(|k| analog::routing::Electromigration { net: pnr_core::NetId(k as u16), limits, stack })
        .collect();
    problem.routing.hard.push(Box::new(rules));
}

/// Member and terminal a cell pin names: `d{k}:T` is terminal `T` of member
/// `k`, a bare `T` (an injected macro's pin) member 0 — the
/// `cellgen::bind_pins` rule. `GND` (a cell's substrate pin) and malformed
/// ordinals name no member.
fn pin_member(name: &str) -> Option<(usize, &str)> {
    match name.split_once(':') {
        Some((k, t)) => Some((k.strip_prefix('d')?.parse().ok()?, t)),
        None => (name != "GND").then_some((0, name)),
    }
}

/// Per placed cell, `(pin name, µA)` for every terminal of its members: pin
/// `d{k}:T` is terminal `T` of member `k`, a bare `T` member 0 (see
/// `cellgen::bind_pins`). An unresolved device's terminals are `None` (its
/// nets get no EM sizing); every other device keeps its currents.
fn pin_currents(netlist: &pnr_core::Netlist, devices_of: &[Vec<DeviceId>], draws: &[Option<Vec<(String, f64)>>]) -> Vec<Vec<(String, Option<i32>)>> {
    devices_of
        .iter()
        .map(|members| {
            let mut out = Vec::new();
            for (k, d) in members.iter().enumerate() {
                let terms: Vec<(String, Option<i32>)> = match &draws[d.0 as usize] {
                    Some(ts) => ts.iter().map(|(t, ua)| (t.clone(), Some(ua.round() as i32))).collect(),
                    None => netlist.devices[d.0 as usize].terminals.iter().map(|(t, _)| (t.clone(), None)).collect(),
                };
                for (t, ua) in terms {
                    out.push((format!("d{k}:{t}"), ua));
                    if k == 0 {
                        out.push((t, ua));
                    }
                }
            }
            out
        })
        .collect()
}

/// Per-device power (µW), per-terminal current (µA) and the bias provenance for
/// the report: the ngspice operating point when configured and solvable, else
/// `cfg.device_power_uw` and no currents.
fn bias(netlist: &pnr_core::Netlist, cfg: &Config) -> Bias {
    let op = cfg
        .op
        .as_ref()
        .and_then(|oc| match oppoint::extract(netlist, oc) {
            Ok(o) => Some((o, oc.testbench.is_none())),
            Err(e) => {
                eprintln!("[op] operating point unavailable ({e}); continuing with `device_power_uw`");
                None
            }
        });
    let Some((o, probe)) = op else {
        return Bias { power: cfg.device_power_uw.clone(), summary: None, currents: None, net_headroom_mv: None, gm_us: Vec::new(), op: None };
    };
    let hottest = o
        .power_uw
        .iter()
        .enumerate()
        .max_by_key(|(_, &p)| p)
        .filter(|(_, &p)| p > 0)
        .map(|(i, &p)| (netlist.devices[i].name.clone(), p));
    let summary = metadata::BiasSummary {
        provenance: o.provenance.clone(),
        resolved: o.resolved,
        devices: netlist.devices.len(),
        total_power_uw: o.total_power_uw(),
        hottest,
        probe,
        em_temp_k: (cfg.op.as_ref().map_or(27.0, |c| c.temp_c) + 273.15) as f32,
        em_derate: "",
    };
    let currents = o.terminal_ua(netlist);
    let headroom = o.net_headroom_mv(netlist);
    Bias { power: o.power_uw.clone(), summary: Some(summary), currents: Some(currents), net_headroom_mv: Some(headroom), gm_us: o.gm_us.clone(), op: Some(o) }
}

/// The collapsed cell table and every device-indexed input translated to it.
struct CellSpace {
    /// Pre-drawn alternatives per cell — what `gp`/`dp` search over.
    variants: Vec<gp::VariantSpace>,
    /// Injected (user-macro) cells, drawn as given: dp never reshapes or rotates them.
    fixed: Vec<bool>,
    /// `Layout::groups` after `dp`: the recognition table.
    groups: Vec<Vec<DeviceId>>,
    /// Guard-ring requirements, one per requesting cell.
    guard_rings: analog::Constraints,
    /// Per-cell power, µW (sum of member devices).
    power: Vec<i32>,
    /// Every (cell, variant)'s physical units, keyed by schematic device.
    units: std::sync::Arc<pnr_core::UnitLib>,
    /// Merged cells whose members sit on different gate nets.
    distinct_gate_merges: usize,
    /// Schematic devices per cell, in pin-ordinal order.
    devices_of: Vec<Vec<DeviceId>>,
}

impl CellSpace {
    /// Draw every cell's variants and retarget `problem.placement` to cell ids.
    fn new(
        netlist: &pnr_core::Netlist,
        injected: &Macros,
        problem: &mut Problem,
        pdk: &Pdk,
        power: &[i32],
        merge_distinct_gates: bool,
        fold: &[(u16, i32)],
    ) -> Self {
        let cellgen::Cells {
            spaces,
            cell_of,
            devices_of,
        } = cellgen::enumerate_folded(netlist, injected, &problem.constraints, pdk, merge_distinct_gates, fold, {
            let ground = problem.net_classes.iter().find(|c| c.class == analog::metadata::NetClass::Ground);
            ground.map(|c| c.net)
        });
        let gate = |d: &DeviceId| {
            netlist.devices[d.0 as usize].terminals.iter().find(|(t, _)| t == "G").map(|(_, n)| *n)
        };
        let distinct_gate_merges = devices_of
            .iter()
            .filter(|m| m.len() > 1 && m.iter().any(|d| gate(d) != gate(&m[0])))
            .count();
        let p = &mut problem.placement;
        for b in p
            .hard
            .iter_mut()
            .chain(p.budget.iter_mut())
            .chain(p.cost.iter_mut())
        {
            b.retarget(&cell_of);
        }
        let to_cells = |g: &Vec<DeviceId>| remap_members(g, &cell_of);
        let groups: Vec<_> = problem.groups.iter().map(to_cells).collect();
        #[cfg(debug_assertions)]
        debug_check_retargeted(problem, spaces.len(), &groups);

        // Guard rings land on cells; two members of one cell asking is one ring.
        let mut guard_rings = analog::Constraints::default();
        for r in &problem.constraints.guard_rings {
            let mut r = r.clone();
            if let Some(&c) = cell_of.get(r.device.0 as usize) {
                r.device = DeviceId(c);
            }
            if !guard_rings.guard_rings.iter().any(|g| g.device == r.device) {
                guard_rings.guard_rings.push(r);
            }
        }

        let mut cells = CellSpace {
            fixed: devices_of
                .iter()
                .map(|m| {
                    m.iter()
                        .any(|d| injected.get(&netlist.devices[d.0 as usize].name).is_some())
                })
                .collect(),
            power: devices_of
                .iter()
                .map(|m| {
                    m.iter()
                        .map(|d| power.get(d.0 as usize).copied().unwrap_or(0))
                        .fold(0, i32::saturating_add)
                })
                .collect(),
            groups,
            variants: spaces,
            guard_rings,
            units: std::sync::Arc::default(),
            distinct_gate_merges,
            devices_of: Vec::new(),
        };
        // Reserve each ring's halo in the requester's bbox so the placer keeps
        // neighbours out of it; the ring is drawn back inside the reservation.
        let lattice = cells::builder::cut_lattice(pdk);
        for r in &cells.guard_rings.guard_rings {
            let ext = round_up(cells::post_cell::ring_halo(r, pdk, ring_cut_ohm(pdk)), lattice);
            let Some(space) = cells.variants.get_mut(r.device.0 as usize) else {
                continue;
            };
            for m in &mut space.alternatives {
                m.bbox.x -= ext;
                m.bbox.y -= ext;
                m.bbox.w += 2 * ext;
                m.bbox.h += 2 * ext;
            }
        }
        // Origins on the cut lattice need extents on twice it (PLC-02); generated
        // and injected cells alike, before unit frames are taken from the bbox.
        for m in cells.variants.iter_mut().flat_map(|s| s.alternatives.iter_mut()) {
            m.align_bbox(lattice);
        }
        // After the halo: a unit's frame is the bbox `place_macro` anchors on.
        let per_cell: Vec<Vec<(pnr_core::Rect, &[pnr_core::Unit])>> = cells
            .variants
            .iter()
            .map(|s| s.alternatives.iter().map(|m| (m.bbox, &m.units[..])).collect())
            .collect();
        let units = pnr_core::UnitLib::build(cell_of, &devices_of, per_cell.iter().map(Vec::as_slice));
        cells.units = std::sync::Arc::new(units);
        cells.devices_of = devices_of;
        cells
    }
}

/// Rewrite a group's members through `cell_of`, deduplicated in first-seen
/// order. A non-empty group stays non-empty.
fn remap_members(members: &[DeviceId], cell_of: &[u16]) -> Vec<DeviceId> {
    let mut out: Vec<DeviceId> = Vec::with_capacity(members.len());
    for d in members {
        let c = cell_of.get(d.0 as usize).map_or(*d, |&c| DeviceId(c));
        if !out.contains(&c) {
            out.push(c);
        }
    }
    out
}

/// After retargeting, no placement batch may name an id `>= n_cells` — that
/// would be a rule kind missing its `retarget` override.
#[cfg(debug_assertions)]
fn debug_check_retargeted(problem: &Problem, n: usize, groups: &[Vec<DeviceId>]) {
    let probe = Layout {
        x: (0..n).map(|i| i as i32 * 10_007 + 13).collect(),
        y: (0..n).map(|i| i as i32 * 7_919 + 29).collect(),
        hw: vec![50; n],
        hh: vec![50; n],
        axis: vec![0; problem.axis_count.max(1)],
        groups: groups.to_vec(),
        orient: vec![pnr_core::Orient::default(); n],
        variant: vec![0; n],
        branch: Vec::new(),
        power_uw: vec![0; n],
        temp_mc: vec![0; n],
        units: Default::default(),
    };
    let p = &problem.placement;
    let mut ids = Vec::new();
    for b in p.hard.iter().chain(p.budget.iter()).chain(p.cost.iter()) {
        ids.clear();
        b.violating_ids(&probe, &mut ids);
        assert!(
            ids.iter().all(|&i| (i as usize) < n),
            "batch {:?} touches device ids {ids:?} after retargeting to {n} cells",
            b.kind()
        );
    }
}

impl Solution {
    /// The flat placed geometry — every macro stamped at its placement plus all
    /// routed wires. What signoff checks and [`gds::emit`] writes.
    #[must_use]
    pub fn geometry(&self) -> Vec<pnr_core::Shape> {
        let mut shapes = geometry::collect(&self.macros, &self.layout, &self.routes);
        if let Some(l) = self.well_layer {
            geometry::merge_rects(&mut shapes, l);
        }
        shapes
    }
}

/// Parse a SPICE netlist — the same front end [`run`] uses.
///
/// # Errors
/// The parser's message.
pub fn parse(spice: &str) -> Result<pnr_core::Netlist, String> {
    parse::spice(spice)
}

/// Full DRC/ERC/LVS/PEX signoff of a solution against its own schematic:
/// errors in `report`, deck warnings and coverage apart ([`verify::Signoff`]).
#[must_use]
pub fn signoff(sol: &Solution, pdk: &Pdk) -> verify::Signoff {
    let (shapes, pins, reference) = signoff_inputs(sol, pdk);
    let mut s = verify::signoff_checked(&shapes, &pins, &reference, &sol.intent, pdk);
    s.report.hard_violations.extend(undrawable(&sol.macros[..sol.layout.x.len()], &sol.devices_of, &sol.netlist));
    if let Some(op) = &sol.op {
        let probe = sol.metadata.bias.as_ref().is_some_and(|b| b.probe);
        s.report.hard_violations.extend(reliability::voltage_findings(&sol.netlist, op, &pdk.fet_voltage_limits(), &sol.pairs, probe).0);
    }
    s
}

/// Every recognised 2-device `DiffPair` / `CurrentMirror` / `Load` leaf block.
fn matched_pairs(blocks: &[annotator::Block]) -> Vec<(DeviceId, DeviceId)> {
    use annotator::BlockKind::{CurrentMirror, DiffPair, Load};
    annotator::block::leaves(blocks)
        .into_iter()
        .filter(|b| matches!(b.kind, DiffPair | CurrentMirror | Load))
        .filter_map(|b| match *b.devices.as_slice() {
            [a, b] => Some((a, b)),
            _ => None,
        })
        .collect()
}

/// A device this process has no construction for (an NPN without a deep
/// well, a poly resistor on a fin process, any inductor) is drawn as nothing:
/// one hard `cell/undrawable` row per empty cell of `cells`, naming its
/// members (`devices_of`, indexed like `cells`), never hidden
/// behind an LVS that cannot see it either. The epoch counts them in |V| too
/// (the same rows every epoch, so ranking is unchanged), so a run with an
/// undrawn device never reads feasible or converged.
fn undrawable<'a>(cells: &'a [Macro], devices_of: &'a [Vec<DeviceId>], netlist: &'a pnr_core::Netlist) -> impl Iterator<Item = pnr_core::report::Violation> + 'a {
    cells.iter().enumerate().filter(|(_, m)| m.shapes.is_empty()).map(move |(i, _)| {
        let who = devices_of.get(i).map_or_else(
            || "?".to_string(),
            |ds| ds.iter().map(|d| netlist.devices.get(d.0 as usize).map_or("?".to_string(), |d| format!("{} ({})", d.name, d.model))).collect::<Vec<_>>().join(", "),
        );
        pnr_core::report::Violation { rule: format!("cell/undrawable: {who} has no construction on this process"), margin: 1 }
    })
}

/// Adopt devices a later stage inserted (an antenna diode from `dr`): each
/// device joins the schematic, so [`cellgen::reference`] gives LVS its entry,
/// and its macro, already placed and pin-bound (absolute, like a guard ring),
/// joins the geometry. A diode is `P` = anode, `N` = cathode.
pub fn adopt_devices(netlist: &mut pnr_core::Netlist, macros: &mut Vec<Macro>, extra: Vec<(pnr_core::Device, Macro)>) {
    for (device, m) in extra {
        netlist.devices.push(device);
        macros.push(m);
    }
}

/// Exactly what [`signoff`] hands `verify`: the drawn shapes, the net labels
/// placed on them, and the LVS reference whose ports are those labels.
#[must_use]
pub fn signoff_inputs(
    sol: &Solution,
    pdk: &Pdk,
) -> (
    Vec<pnr_core::Shape>,
    Vec<verify::LabeledPin>,
    verify::RefInput,
) {
    let shapes = sol.geometry();
    let placed = pnr_core::place_macros(&sol.macros, &sol.layout);
    let names: Vec<String> = sol.netlist.nets.iter().map(|n| n.name.clone()).collect();
    let (pins, reference) = labels_and_reference(&shapes, &placed, &names, &sol.netlist, Some(&sol.folds), pdk);
    (shapes, pins, reference)
}

/// The solution as a GDSII stream other tools can sign off: one structure
/// named `top`, and each of `ports` (empty: every net signoff labels) written
/// as TEXT on the deck's text layer for its conductor ([`Pdk::label_gds`]).
/// magic makes every top-level label a port, so an internal net labelled
/// here fails pin matching against the schematic's `.subckt`.
/// `Err` when a shape is on a layer with no GDS stream number ([`gds::emit`]).
pub fn export_gds(sol: &Solution, pdk: &Pdk, top: &str, ports: &[String]) -> Result<Vec<u8>, String> {
    let (shapes, pins, _) = signoff_inputs(sol, pdk);
    let texts: Vec<gds::Text> = pins
        .iter()
        .filter(|p| ports.is_empty() || ports.contains(&p.name))
        .filter_map(|p| Some(gds::Text { name: p.name.clone(), gds: pdk.label_gds(p.layer)?, x: p.x, y: p.y }))
        .collect();
    gds::emit(top, &shapes, &pdk.layer_gds(), &texts)
}

/// The LVS reference [`signoff`] compares against, as SPICE `.subckt top`
/// ([`Pdk::reference_spice`]): dummies and per-finger cards included, so an
/// external LVS of [`export_gds`] against it checks the same claim. The
/// user's schematic lacks the dummies, so LVS against it reports them extra.
#[must_use]
pub fn reference_spice(sol: &Solution, pdk: &Pdk, top: &str, ports: &[String]) -> String {
    pdk.reference_spice(&signoff_inputs(sol, pdk).2, top, ports)
}

/// The routed layout extracted with parasitics as `.subckt {top}` over the schematic's ports
/// ([`pnr_core::Netlist::ports`], declaration order), simulatable against the PDK's ngspice library (FR-7):
/// MOS cards become `X` cards in µm (the library's `.option scale=1.0u`, as [`oppoint`] assumes), a
/// modelled resistor an `R` element with the schematic's `w`/`l`, parasitic `Cp`/`Rp` kept. A port is
/// the extractor's `{port}:0` piece; its other `:k` pieces hang off it through the parasitics.
///
/// # Errors
/// A schematic device other than a MOS or resistor (the extractor reports no BJT or capacitor card, so the
/// file would silently lack it), no ports, a port with no label, a MOS card without a bulk node, a resistor
/// matching no single schematic resistor, rewritten resistor cards not one per schematic resistor, any other
/// device card (not rewritten yet), or the extractor's own error.
pub fn post_layout_spice(sol: &Solution, pdk: &Pdk, top: &str) -> Result<String, String> {
    use pnr_core::DeviceKind as K;
    let nl = &sol.netlist;
    if let Some(d) = nl.devices.iter().find(|d| !matches!(d.kind, K::Nmos | K::Pmos | K::Resistor)) {
        return Err(format!("{}: {:?} not extracted (PERF-30)", d.name, d.kind));
    }
    let (shapes, pins, _) = signoff_inputs(sol, pdk);
    let raw = verify::extract_spice(&shapes, &pins, pdk, verify::Detail::WithParasitics)?;
    let ports: Vec<&str> = nl.ports.iter().map(|n| nl.nets[n.0 as usize].name.as_str()).collect();
    if ports.is_empty() {
        return Err("no .subckt ports".into());
    }
    if let Some(p) = ports.iter().find(|p| !pins.iter().any(|q| q.name.eq_ignore_ascii_case(p))) {
        return Err(format!("port {p} has no label"));
    }
    let base = |node: &str| node.rsplit_once(':').map_or(node, |(n, _)| n).to_string();
    let um = |nm: f64| nm / 1000.0;
    let mut out = String::new();
    let mut resistors = 0;
    for line in raw.lines() {
        let t: Vec<&str> = line
            .split_whitespace()
            .map(|tok| ports.iter().find(|p| tok.eq_ignore_ascii_case(&format!("{p}:0"))).copied().unwrap_or(tok))
            .collect();
        let name = t.first().copied().unwrap_or("");
        // Positional tokens (nodes, model) end at the first `k=v`.
        let pos = t.iter().take_while(|s| !s.contains('=')).count();
        let kv = |k: &str| t[pos..].iter().find_map(|s| s.split_once('=').filter(|(n, _)| n.eq_ignore_ascii_case(k))?.1.parse::<f64>().ok());
        let card = match name.chars().next().map(|c| c.to_ascii_uppercase()) {
            None => String::new(),
            Some('.') if name.eq_ignore_ascii_case(".subckt") => format!(".subckt {top} {}", ports.join(" ")),
            Some('.') if name.eq_ignore_ascii_case(".ends") => format!(".ends {top}"),
            Some('*') if line.contains("database units") => "* lengths in um (the model library's .option scale=1.0u)".into(),
            Some('*' | '.') => line.to_string(),
            Some('M') => {
                if pos < 6 {
                    return Err(format!("{name}: MOS card without a bulk node"));
                }
                let (w, l) = kv("w").zip(kv("l")).ok_or_else(|| format!("{name}: MOS card without w/l"))?;
                format!("X{name} {} w={} l={}", t[1..6].join(" "), um(w), um(l))
            }
            Some('R' | 'C') if t.get(3).is_some_and(|v| v.trim_end_matches(char::is_alphabetic).parse::<f64>().is_ok()) => {
                line.to_string()
            }
            Some('R') if pos == 4 => {
                let model = t[3];
                let ends = |a: &str, b: &str| {
                    let (a, b) = (base(a), base(b));
                    let (x, y) = (base(t[1]), base(t[2]));
                    (a.eq_ignore_ascii_case(&x) && b.eq_ignore_ascii_case(&y)) || (a.eq_ignore_ascii_case(&y) && b.eq_ignore_ascii_case(&x))
                };
                let net = |d: &pnr_core::Device, k: &str| d.terminals.iter().find(|(n, _)| n == k).map(|(_, n)| nl.nets[n.0 as usize].name.as_str());
                // The deck model the LVS reference names (as in [`labels_and_reference`]).
                let deck = |m: &str| pdk.recipe("resistor", m).map_or_else(|| m.to_string(), |r| r.model);
                let hits: Vec<&pnr_core::Device> = nl
                    .devices
                    .iter()
                    .filter(|d| d.kind == K::Resistor && deck(&d.model).eq_ignore_ascii_case(model))
                    .filter(|d| net(d, "P").zip(net(d, "N")).is_some_and(|(p, n)| ends(p, n)))
                    .collect();
                // ponytail: a resistor drawn as several segments (CELL-06) extracts several cards and errs here
                let [d] = hits[..] else {
                    return Err(format!("{name}: {} schematic resistors match", hits.len()));
                };
                let p = |k: &str| d.params.iter().find(|(n, _)| n == k).map(|&(_, v)| um(v as f64));
                let (w, l) = p("w").zip(p("l")).ok_or_else(|| format!("{name}: schematic {} has no w/l", d.name))?;
                resistors += 1;
                format!("R{name} {} {} {model} w={w} l={l}", t[1], t[2])
            }
            _ => return Err(format!("{name}: card not rewritten (PERF-30)")),
        };
        out.push_str(&card);
        out.push('\n');
    }
    let want = nl.devices.iter().filter(|d| d.kind == K::Resistor).count();
    if resistors != want {
        return Err(format!("{resistors} resistor cards extracted for {want} schematic resistors"));
    }
    Ok(out)
}

/// Signoff over drawn `shapes`; `fold`: the table the cells were drawn at
/// ([`cellgen::folds`], the flow), `None` for the schematic's own fingers.
pub(crate) fn signoff_shapes(
    intent: &verify::Intent,
    shapes: &[pnr_core::Shape],
    placed: &[Macro],
    nets: &[String],
    schematic: &pnr_core::Netlist,
    fold: Option<&[(u16, i32)]>,
    pdk: &Pdk,
) -> verify::Signoff {
    let (pins, reference) = labels_and_reference(shapes, placed, nets, schematic, fold, pdk);
    verify::signoff_checked(shapes, &pins, &reference, intent, pdk)
}

/// Labels every provable net (see [`labeled_pins`]) and builds the LVS
/// reference with exactly those names as ports — `verify` requires they match.
fn labels_and_reference(
    shapes: &[pnr_core::Shape],
    placed: &[Macro],
    nets: &[String],
    schematic: &pnr_core::Netlist,
    fold: Option<&[(u16, i32)]>,
    pdk: &Pdk,
) -> (Vec<verify::LabeledPin>, verify::RefInput) {
    let pins = labeled_pins(placed, nets, pdk, shapes);
    let (drawn, replaced) = cellgen::drawn_cards(placed, nets, schematic, pdk);
    let mut reference = cellgen::reference(schematic, fold, &replaced);
    // A resistor is drawn to its model's recipe: the deck model that names.
    for d in reference.devices.iter_mut().filter(|d| d.kind == verify::reference::RefKind::Resistor) {
        if let Some(r) = pdk.recipe("resistor", d.model.as_deref().unwrap_or("")) {
            d.model = Some(r.model);
        }
    }
    reference.devices.extend(cellgen::dummy_cards(placed, nets, &reference.devices));
    reference.devices.extend(drawn);
    reference.ports = pins.iter().map(|p| p.name.clone()).collect();
    // PERF-03: only the declared `.subckt` ports leave the cell; with none
    // (no `.subckt` around the top) every labelled net stays exempt.
    reference.external_ports = (!schematic.ports.is_empty())
        .then(|| schematic.ports.iter().map(|n| schematic.nets[n.0 as usize].name.clone()).collect());
    (pins, reference)
}

/// One label per net: the first placed pin on a label layer whose centre lies
/// on drawn conductor of that layer. `verify` fails closed on a label with no
/// shape under it, so only provable labels go in; an unlabelled net stays
/// anonymous, which LVS handles structurally.
pub(crate) fn labeled_pins(
    placed: &[Macro],
    nets: &[String],
    pdk: &Pdk,
    shapes: &[pnr_core::Shape],
) -> Vec<verify::LabeledPin> {
    let mut out: Vec<verify::LabeledPin> = Vec::new();
    let mut labelled: Vec<u16> = Vec::new();
    // ponytail: O(pins × shapes), once per signoff; index shapes per layer if it shows.
    for p in placed.iter().flat_map(|m| &m.pins) {
        if labelled.contains(&p.net.0) {
            continue;
        }
        if verify::geom::label_layer(&pdk.deck, p.layer.0).is_none() {
            continue;
        }
        let (x, y) = (p.at.x + p.at.w / 2, p.at.y + p.at.h / 2);
        let on_drawn = shapes.iter().any(|s| {
            s.layer == p.layer
                && (s.rect.x..=s.rect.x + s.rect.w).contains(&x)
                && (s.rect.y..=s.rect.y + s.rect.h).contains(&y)
        });
        let Some(name) = nets.get(p.net.0 as usize).filter(|_| on_drawn) else {
            continue;
        };
        labelled.push(p.net.0);
        out.push(verify::LabeledPin {
            name: name.clone(),
            layer: p.layer.0,
            x,
            y,
        });
    }
    out
}

#[cfg(test)]
mod start_tests {
    /// GAP-04: the substrate kind comes from the deck's `substrate_kind`; a
    /// misspelt key (here or in the sidecar) would silently read Unknown.
    #[test]
    fn substrate_kind_is_read_from_the_deck() {
        use pnr_core::SubstrateKind::{Bulk, Unknown};
        for (name, kind) in [("sky130", Bulk), ("gf180mcu", Unknown), ("ihp_sg13g2", Unknown), ("generic_finfet", Unknown)] {
            let pdk = verify::Pdk::builtin(name).expect("deck loads");
            assert_eq!(crate::annotation(&pdk, &Default::default()).process.substrate, kind, "{name}");
        }
    }

    /// Parallel starts stay deterministic: the same seed and start count give
    /// the same layout, however the threads interleave.
    #[test]
    fn multi_start_is_deterministic() {
        let pdk = verify::Pdk::builtin("sky130").expect("sky130 loads");
        let spice = ".subckt p d g VSS\nXM1 d g VSS VSS nfet_01v8 W=2u L=0.5u\nXM2 d x VSS VSS nfet_01v8 W=2u L=0.5u\n.ends p\n";
        let cfg = crate::Config { seed: 7, feedback_iters: 2, outer_iters: 1, starts: 3, ..Default::default() };
        let run = || crate::run(spice, &pdk, &Default::default(), &cfg).expect("flow");
        let (a, b) = (run(), run());
        assert_eq!((a.layout.x, a.layout.y), (b.layout.x, b.layout.y));
    }

    /// FLOW-09: topologies (and `seed_assignment`'s pricing) are built once on
    /// the calling thread, not once per start.
    #[test]
    fn hoisting_prices_each_alternative_once() {
        let pdk = verify::Pdk::builtin("sky130").expect("sky130 loads");
        let spice = ".subckt p d g VSS\nXM1 d g VSS VSS nfet_01v8 W=8u L=0.5u\nXM2 d x VSS VSS nfet_01v8 W=8u L=0.5u\n.ends p\n";
        let calls = |starts| {
            let before = (crate::cellgen::price_calls(), crate::APART_BUILDS.with(std::cell::Cell::get));
            let cfg = crate::Config { feedback_iters: 2, outer_iters: 1, starts, ..Default::default() };
            crate::run(spice, &pdk, &Default::default(), &cfg).expect("flow");
            (crate::cellgen::price_calls() - before.0, crate::APART_BUILDS.with(std::cell::Cell::get) - before.1)
        };
        let (n1, n3) = (calls(1), calls(3));
        // The pair merges with distinct gates, so both topologies are hoisted.
        assert_eq!(n1.1, 1, "the apart topology is built once");
        assert!(n1.0 > 0);
        assert_eq!(n3, n1);
    }

    /// Same seed, same bytes: the GDS of two runs is identical with parallel starts.
    #[test]
    fn same_seed_same_gds_bytes() {
        let pdk = verify::Pdk::builtin("sky130").expect("sky130 loads");
        let cfg = crate::Config { seed: 1, feedback_iters: 2, outer_iters: 1, starts: 3, ..Default::default() };
        for spice in [include_str!("../../../benchmarks/fixtures/chain4.spice"), include_str!("../../../benchmarks/fixtures/ota.spice")] {
            let gds = || crate::export_gds(&crate::run(spice, &pdk, &Default::default(), &cfg).expect("flow"), &pdk, "top", &[]).unwrap();
            assert!(gds() == gds());
        }
    }

    #[test]
    fn stage_times_are_reported() {
        let pdk = verify::Pdk::builtin("sky130").expect("sky130 loads");
        let spice = include_str!("../../../benchmarks/fixtures/pair.spice");
        let cfg = crate::Config { feedback_iters: 2, outer_iters: 1, starts: 1, ..Default::default() };
        let ms = crate::run(spice, &pdk, &Default::default(), &cfg).expect("flow").stats.stage_ms;
        assert!(ms.iter().sum::<f64>() > 0.0, "{ms:?}");
        assert!(ms[6] > 0.0, "signoff runs every epoch: {ms:?}");
    }

    #[test]
    fn interface_parses_the_fixture_and_rejects_a_non_port() {
        let i = crate::Interface::from_json(include_str!("../../../benchmarks/fixtures/ota_constrained.interface.json")).expect("parses");
        assert_eq!(i.die_nm, Some((30000, 70000)));
        assert_eq!(i.pins.len(), 6);
        assert_eq!(i.pins[0].side, crate::Side::South);
        assert_eq!(i.pins[0].width_nm, 800);
        let pdk = verify::Pdk::builtin("sky130").expect("sky130 loads");
        let pin = crate::IoPin { net: "nope".into(), side: crate::Side::North, frac: 0.5, width_nm: 800, layer: "met3".into() };
        let cfg = crate::Config { interface: Some(crate::Interface { die_nm: None, pins: vec![pin] }), ..Default::default() };
        match crate::run(include_str!("../../../benchmarks/fixtures/pair.spice"), &pdk, &Default::default(), &cfg) {
            Err(crate::FlowError::Interface(m)) => assert!(m.contains("nope"), "{m}"),
            r => panic!("expected FlowError::Interface, got {:?}", r.err()),
        }
    }

    /// T6: exactly one dual step per epoch, taken by the flow (gp and dp only
    /// bind), so the price step count equals the epoch count.
    #[test]
    fn one_dual_step_per_epoch() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let pdk = verify::Pdk::from_json(&std::fs::read_to_string(root.join("pdks/sky130.json")).unwrap()).expect("sky130 loads");
        let spice = std::fs::read_to_string(root.join("benchmarks/fixtures/pair.spice")).unwrap();
        let cfg = crate::Config { feedback_iters: 3, outer_iters: 1, starts: 1, ..Default::default() };
        let sol = crate::run(&spice, &pdk, &Default::default(), &cfg).expect("flow");
        assert!(sol.stats.iterations > 0);
        assert_eq!(sol.stats.dual_steps, sol.stats.iterations);
    }

    /// PERF-06: a simulator that cannot start is counted, not just logged,
    /// and every declared bound is in the report with its reason for no row.
    #[test]
    fn a_failed_simulation_is_counted_and_every_bound_reported() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let pdk = verify::Pdk::from_json(&std::fs::read_to_string(root.join("pdks/sky130.json")).unwrap()).expect("sky130 loads");
        let spice = std::fs::read_to_string(root.join("benchmarks/fixtures/pair.spice")).unwrap();
        let performance = crate::perf::PerfConfig {
            sim: crate::oppoint::OpConfig { ngspice: "philis-no-such-binary-7f3a".into(), ..Default::default() },
            testbenches: vec![String::new()],
            scenarios: Vec::new(),
            specs: vec![crate::perf::Spec { metric: "gain".into(), min: Some(20.0), max: Some(60.0) }],
        };
        let cfg = crate::Config { feedback_iters: 2, outer_iters: 1, starts: 1, performance: Some(performance), ..Default::default() };
        let sol = crate::run(&spice, &pdk, &Default::default(), &cfg).expect("flow");
        assert!(sol.stats.sim_failures >= 1, "{:?}", sol.stats);
        assert_eq!(sol.metadata.sim_failures, sol.stats.sim_failures);
        let rows = &sol.metadata.budget_rows;
        assert_eq!(rows.len(), 3, "{rows:?}");
        // Unevaluated, every scenario stays active (here the one `sim` implies).
        assert!(rows[0].starts_with("scenario ") && rows[0].ends_with(": active"), "{rows:?}");
        assert!(rows[1].starts_with("gain:min: no row (sensitivities unavailable"), "{rows:?}");
        assert!(rows[2].starts_with("gain:max: no row (sensitivities unavailable"), "{rows:?}");
        let text = sol.metadata.to_string();
        assert!(text.contains(&format!("simulations failed: {}", sol.stats.sim_failures)), "{text}");
        assert!(text.contains("budget gain:max: no row"), "{text}");
    }

    /// PERF-06: failures count over every start, not just the winner's (a
    /// failed run scores its epoch unmeasured, so selection would hide it).
    /// One epoch per solve, the first always scored: each start fails at
    /// least once, the winner alone at most `iterations` times.
    #[test]
    fn failed_simulations_count_over_every_start() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let pdk = verify::Pdk::from_json(&std::fs::read_to_string(root.join("pdks/sky130.json")).unwrap()).expect("sky130 loads");
        let spice = std::fs::read_to_string(root.join("benchmarks/fixtures/pair.spice")).unwrap();
        let performance = crate::perf::PerfConfig {
            sim: crate::oppoint::OpConfig { ngspice: "philis-no-such-binary-7f3a".into(), ..Default::default() },
            testbenches: vec![String::new()],
            scenarios: Vec::new(),
            specs: vec![crate::perf::Spec { metric: "gain".into(), min: Some(20.0), max: None }],
        };
        let cfg = crate::Config { feedback_iters: 1, outer_iters: 1, starts: 2, performance: Some(performance), ..Default::default() };
        let sol = crate::run(&spice, &pdk, &Default::default(), &cfg).expect("flow");
        assert_eq!(sol.stats.iterations, 1, "{:?}", sol.stats);
        assert!(sol.stats.sim_failures >= 2, "{:?}", sol.stats);
        assert_eq!(sol.metadata.sim_failures, sol.stats.sim_failures);
    }

    /// An unresolved device's pins are unknown; the other devices keep their
    /// currents, in its own cell and in every other cell (not all-or-nothing).
    #[test]
    fn one_unresolved_device_keeps_the_others_known() {
        use pnr_core::{Device, DeviceId, DeviceKind, Net, NetId, Netlist};
        let fet = |name: &str| Device {
            name: name.into(),
            kind: DeviceKind::Nmos,
            model: String::new(),
            terminals: ["D", "G", "S", "B"].iter().enumerate().map(|(i, t)| ((*t).into(), NetId(i as u16))).collect(),
            params: vec![],
        };
        let nl = Netlist { devices: vec![fet("M0"), fet("M1"), fet("M2")], nets: ["a", "b", "c", "d"].iter().map(|n| Net { name: (*n).into() }).collect(), ..Default::default() };
        let known = |id: f64| Some(vec![("D".into(), id), ("G".into(), 0.0), ("S".into(), -id), ("B".into(), 0.0)]);
        let cells = [vec![DeviceId(0), DeviceId(1)], vec![DeviceId(2)], vec![DeviceId(1), DeviceId(0)]];
        let pins = crate::pin_currents(&nl, &cells, &[known(10.0), None, known(20.0)]);
        let ua = |cell: usize, pin: &str| pins[cell].iter().find(|(n, _)| n == pin).map(|p| p.1);
        assert_eq!((ua(0, "d0:D"), ua(0, "D"), ua(0, "d0:S")), (Some(Some(10)), Some(Some(10)), Some(Some(-10))), "member 0 known");
        assert_eq!((ua(0, "d1:D"), ua(0, "d1:S"), ua(0, "d1:G")), (Some(None), Some(None), Some(None)), "member 1 unknown, never zero");
        assert_eq!((ua(1, "d0:D"), ua(1, "S")), (Some(Some(20)), Some(Some(-20))), "the other cell known");
        assert_eq!((ua(2, "D"), ua(2, "d0:S"), ua(2, "d1:D")), (Some(None), Some(None), Some(Some(10))), "unresolved member 0: bare names unknown too");
    }

    /// REL-01 acceptance on the real fixtures and ngspice bias: every pin on a
    /// net that touches no resistor carries a known current, so dr sizes it
    /// (all-or-nothing left rc_filter with no pin currents at all).
    #[test]
    fn fixture_nets_without_a_resistor_keep_em_sizing() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let Some(lib) = crate::tools::sky130_models() else { return };
        let pdk = verify::Pdk::from_json(&std::fs::read_to_string(root.join("pdks/sky130.json")).unwrap()).unwrap();
        let cfg = crate::Config { op: Some(crate::oppoint::OpConfig { model_lib: Some(lib), ..Default::default() }), ..Default::default() };
        for fixture in ["rc_filter", "dac4"] {
            let mut nl = crate::parse(&std::fs::read_to_string(root.join(format!("benchmarks/fixtures/{fixture}.spice"))).unwrap()).unwrap();
            crate::deck_models(&mut nl, &pdk);
            let currents = crate::bias(&nl, &cfg).currents.expect("operating point");
            let one_per_cell: Vec<_> = (0..nl.devices.len()).map(|i| vec![pnr_core::DeviceId(i as u16)]).collect();
            let pins = crate::pin_currents(&nl, &one_per_cell, &currents);
            for (k, net) in nl.nets.iter().enumerate() {
                let on = || nl.devices.iter().enumerate().flat_map(|(i, d)| d.terminals.iter().filter(|(_, n)| n.0 as usize == k).map(move |(t, _)| (i, d, t)));
                let resistor = on().any(|(_, d, _)| d.kind == pnr_core::DeviceKind::Resistor);
                let known = on().all(|(i, _, t)| pins[i].iter().any(|(p, ua)| *p == format!("d0:{t}") && ua.is_some()));
                eprintln!("{fixture} {}: resistor={resistor} known={known}", net.name);
                assert!(resistor || known, "{fixture} {} touches no resistor but has an unknown pin current", net.name);
            }
        }
    }

    /// A routed layer the deck does not limit is named in the missing row,
    /// and deck limits past the rule's `MAX_LAYERS` slots are a missing row
    /// of their own, never a silent drop.
    #[test]
    fn em_rules_name_what_goes_unchecked() {
        use analog::routing::em::{Limit, MAX_LAYERS};
        use pnr_core::LayerId;
        let pdk = verify::Pdk::builtin("sky130").expect("sky130 loads");
        let nl = crate::parse(".subckt p d g VSS\nXM1 d g VSS VSS nfet_01v8 W=2u L=0.5u\n.ends p\n").unwrap();
        let problem = || crate::annotate(&nl, &crate::annotation(&pdk, &Default::default()));
        let (name, id) = pdk.layers[0].clone();
        let lim = Limit { ua_per_um: 1.0, ua_per_cut: 1.0, ..Limit::default() };
        let full: Vec<_> = (0..=MAX_LAYERS as u16).map(|l| (LayerId(l), lim)).collect();

        let mut p = problem();
        crate::em_rules(&mut p, &nl, &full, &[LayerId(MAX_LAYERS as u16)], &[], None, &pdk);
        let rows: Vec<&str> = p.missing.iter().filter(|m| m.0 == "Electromigration").map(|m| m.1).collect();
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert!(rows[0].contains("MAX_LAYERS"), "{rows:?}");

        let mut p = problem();
        crate::em_rules(&mut p, &nl, &[], &[id], &[], None, &pdk);
        let rows: Vec<&str> = p.missing.iter().filter(|m| m.0 == "Electromigration").map(|m| m.1).collect();
        assert_eq!(rows, [format!("deck EM limit on every routed and pin-access layer ({name} unchecked)")]);
    }

    /// C within the tie band goes to the smaller footprint; outside it, C wins.
    #[test]
    fn close_c_is_decided_by_area_and_far_c_by_c() {
        let k = |c: f32, area: f64| (0usize, 0.0, 0.0, c, area);
        assert!(crate::key_lt(&k(30.0, 265.0), &k(29.6, 385.0)), "1.3% more C, 31% less area");
        assert!(crate::key_lt(&k(29.0, 385.0), &k(30.0, 265.0)), "3.3% less C wins outright");
        assert!(!crate::key_lt(&(1usize, 0.0, 0.0, 1.0, 1.0), &k(99.0, 999.0)), "a violation never wins on C");
    }

    use crate::metadata::{Arm, BudgetStatus, MetadataReport};
    use pnr_core::{Report, Violation};

    fn row(arm: Arm, total: usize, satisfied: usize, residual: f64) -> BudgetStatus {
        BudgetStatus { kind: "K".into(), arm, total, satisfied, violations: total - satisfied, unknown: 0, criticality: 0.0, residual, usage: None, violated: Vec::new() }
    }

    fn rows(rules: &[&str]) -> Vec<Violation> {
        rules.iter().map(|r| Violation { rule: (*r).into(), margin: 1 }).collect()
    }

    fn key(place: &Report, signoff: &Report, budgets: &MetadataReport) -> crate::LexKey {
        crate::lex_key(place, &Report::default(), signoff, budgets, None, 0.0, 1.0)
    }

    /// |V| counts the 40 violated rules of a hard batch, not its one stage row.
    #[test]
    fn v_counts_rules_not_batches() {
        let budgets = MetadataReport { placement: vec![row(Arm::Hard, 40, 0, 3.0)], ..Default::default() };
        let place = Report { hard_violations: rows(&["batch:analog hard 0 (Symmetry)"]), ..Default::default() };
        assert_eq!(key(&place, &Report::default(), &budgets).0, 40);
    }

    /// A budget residual restated by the placement report is not added twice.
    #[test]
    fn theta_counts_each_budget_once() {
        let budgets = MetadataReport { placement: vec![row(Arm::Budget, 1, 0, 0.5)], ..Default::default() };
        assert_eq!(budgets.theta(), 500.0);
        let place = Report {
            budget_violations: vec![Violation::from_residual(format!("{}analog budget 0", Violation::BATCH), 0.5)],
            ..Default::default()
        };
        assert_eq!(place.budget_violations[0].margin, 500);
        assert_eq!(key(&place, &Report::default(), &budgets).2, 500.0);
    }

    /// A deck warning is reported, not counted as a hard violation: the
    /// epoch's |V| and `drc_hard` see the one error, `warnings` the two
    /// warnings (verify's `split_by_severity` pins the split upstream).
    #[test]
    fn warnings_are_not_violations() {
        let mut signoff = verify::Signoff::default();
        signoff.report.hard_violations = rows(&["drc/m1.1:met1"]);
        signoff.warnings = rows(&["erc/tie_high_low:li", "erc/tie_high_low:li"]);
        let (key, stats) =
            crate::epoch_score(&Report::default(), &Report::default(), &signoff, &MetadataReport::default(), 0.0, 1.0);
        assert_eq!((key.0, stats.drc_hard, stats.warnings), (1, 1, 2));
    }

    /// An uncompared device is out of |V| (it is the same every epoch, and a
    /// design with a BJT must still converge) but stays a signoff row.
    #[test]
    fn coverage_rows_are_not_epoch_violations() {
        let signoff = Report { hard_violations: rows(&["lvs-coverage/unverified:Pnp:-", "drc/m1.1:met1"]), ..Default::default() };
        assert_eq!(key(&Report::default(), &signoff, &MetadataReport::default()).0, 1);
    }

    /// A NaN tier loses to a finite one, whichever side it is on.
    #[test]
    fn nan_loses() {
        let (nan, finite) = ((0usize, f64::NAN, 0.0, 1.0, 1.0), (0usize, 5.0, 0.0, 1.0, 1.0));
        assert!(crate::key_lt(&finite, &nan), "the finite key displaces a NaN incumbent");
        assert!(!crate::key_lt(&nan, &finite), "a NaN candidate never wins");
    }

    fn net_names() -> Vec<String> {
        ["vout1", "vbn", "VSS", "VDD", "x"].map(String::from).to_vec()
    }

    fn classes() -> Vec<analog::metadata::NetClassification> {
        use analog::metadata::{NetClass, NetClassification};
        [NetClass::Signal, NetClass::Signal, NetClass::Ground, NetClass::Supply]
            .into_iter()
            .enumerate()
            .map(|(i, class)| NetClassification { net: pnr_core::NetId(i as u16), class, c_budget_af: None, max_coupling_af: None })
            .collect()
    }

    /// AV-06: a layout that parks C on the vbn–VSS decoupling the specs do not
    /// feel ranks better than one with less total C but more on the output.
    #[test]
    fn supply_decoupling_does_not_rank_layouts() {
        let rows = [analog::routing::PerformanceBudget {
            metric: "gain:min".into(),
            nets: vec![pnr_core::NetId(0), pnr_core::NetId(1)],
            weights: vec![0.01, 0.0],
            af_per_nm: 1.0,
            limit: 1.0,
        }];
        let cap = |a: &str, b: Option<&str>, c: f64| (a.to_owned(), b.map(str::to_owned), c);
        let a = vec![cap("VSS", Some("vbn"), 89.4), cap("vout1", None, 4.6)];
        let b = vec![cap("VSS", Some("vbn"), 10.0), cap("vout1", None, 5.0)];
        let (ca, cb) = (crate::c_tier(&a, &net_names(), &classes(), &rows), crate::c_tier(&b, &net_names(), &classes(), &rows));
        assert!((ca - 46.0).abs() < 1e-3 && (cb - 50.0).abs() < 1e-3, "c_tier A {ca}, B {cb}");
        let total = |m: &verify::CapMatrix| m.iter().map(|r| r.2).sum::<f64>();
        assert!(total(&a) > 6.0 * total(&b), "A carries 94 fF, B 15 fF");
        assert!(crate::key_lt(&(0, 0.0, 0.0, ca, 1.0), &(0, 0.0, 0.0, cb, 1.0)), "A ranks better");

        // Coupling by each end's own w⁺, summed over rows, clamped at 0:
        // vout1 0.01 + max(-0.005, 0) → 10/fF, vbn 0 + 0.002 → 2/fF, VSS 0.
        let mut two = rows.to_vec();
        two.push(analog::routing::PerformanceBudget { metric: "ugf:min".into(), weights: vec![-0.005, 0.002], ..rows[0].clone() });
        let m = vec![cap("VSS", Some("vout1"), 1.0), cap("vbn", Some("vout1"), 2.0), cap("vout1", None, 0.5)];
        let c = crate::c_tier(&m, &net_names(), &classes(), &two);
        assert!((c - 39.0).abs() < 1e-3, "(10 + 0)·1 + (2 + 10)·2 + 10·0.5 = 39, got {c}");
    }

    /// A label short leaves the tier unknown: NaN, which loses to a finite
    /// tier at equal |V| even with the smaller footprint.
    #[test]
    fn label_short_tier_is_nan_and_loses() {
        let mut signoff = verify::Signoff { caps: vec![("vout1".into(), None, 4.6)], ..Default::default() };
        let clean = crate::signoff_c_tier(&signoff, &net_names(), &classes(), &[]);
        assert!((clean - 4.6).abs() < 1e-4, "no short: signal C, got {clean}");
        let short = format!("lvs/{}: labels [\"b0\", \"VSS\"] bind to one extracted net", verify::checker::LABEL_SHORT);
        signoff.report.hard_violations = rows(&[&short]);
        let tier = crate::signoff_c_tier(&signoff, &net_names(), &classes(), &[]);
        assert!(tier.is_nan(), "shorted tier {tier}");
        let (shorted, finite) = ((1usize, 0.0, 0.0, tier, 0.5), (1usize, 0.0, 0.0, 1000.0, 1.0));
        assert!(crate::key_lt(&finite, &shorted) && !crate::key_lt(&shorted, &finite));
    }

    /// No rows: ground C of signal nets plus coupling once per signal end;
    /// rails, unclassified nets and unknown names add nothing.
    #[test]
    fn no_rows_counts_signal_nets_only() {
        let cap = |a: &str, b: Option<&str>, c: f64| (a.to_owned(), b.map(str::to_owned), c);
        let m = vec![
            cap("VDD", None, 100.0),
            cap("VDD", Some("VSS"), 50.0),
            cap("VSS", Some("vbn"), 10.0),
            cap("vbn", Some("vout1"), 2.0),
            cap("vout1", None, 3.0),
            cap("x", None, 7.0),
            cap("nowhere", None, 9.0),
        ];
        let c = crate::c_tier(&m, &net_names(), &classes(), &[]);
        assert!((c - 17.0).abs() < 1e-4, "10 + 2·2 + 3 fF, got {c}");
    }
}




#[cfg(test)]
mod size_tests {
    use pnr_core::Process as _;

    /// One size convention end to end (FLOW-01, plan-08 T1): on every local
    /// fixture, the channel the cells draw for a MOS (Σ unit `W·L / L` over its
    /// units, variant 0) is the `W_total·m` the simulator card asks for, to a
    /// grid step per drawn finger. Before, ota's `XM1` (`W=10u nf=2`) drew
    /// 20 µm and simulated 10 µm, `XM5` (`W=40u m=4`) drew 160 µm and
    /// simulated 40 µm.
    #[test]
    fn drawn_width_equals_simulated_width() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let json = std::fs::read_to_string(root.join("pdks/sky130.json")).expect("pdks/sky130.json");
        let pdk = verify::Pdk::from_json(&json).expect("sky130 loads");
        let fixtures = ["ota", "ota_constrained", "tt_ota", "pair", "quad", "chain4", "rc_filter", "dac4", "bjt_mirror", "bgr_core"];
        let mut checked = 0;
        for name in fixtures {
            let path = root.join(format!("benchmarks/fixtures/{name}.spice"));
            let spice = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let mut netlist = crate::parse::spice(&spice).expect("parses");
            crate::deck_models(&mut netlist, &pdk);
            let mut problem = annotator::annotate(&netlist, &crate::annotation(&pdk, &Default::default()));
            let fold = crate::cellgen::folds(&netlist, &pdk, &[], &[]);
            let cells = crate::CellSpace::new(&netlist, &Default::default(), &mut problem, &pdk, &[], true, &fold);
            for (i, dev) in netlist.devices.iter().enumerate() {
                if !matches!(dev.kind, pnr_core::DeviceKind::Nmos | pnr_core::DeviceKind::Pmos) {
                    continue;
                }
                let s = dev.mos_size().unwrap_or_else(|| panic!("{name} {}: no size", dev.name));
                let id = pnr_core::DeviceId(i as u16);
                let cell = cells.devices_of.iter().position(|m| m.contains(&id)).expect("device has a cell");
                let owner = cells.devices_of[cell].iter().position(|&d| d == id).unwrap();
                let units = &cells.variants[cell].alternatives[0].units;
                let drawn: i64 = units.iter().filter(|u| usize::from(u.owner) == owner).map(|u| u.weight / s.l_nm).sum();
                let want = s.w_total_nm * i64::from(s.m);
                let tol = i64::from(s.fingers()) * i64::from(pdk.grid());
                assert!((drawn - want).abs() <= tol, "{name} {}: drawn {drawn} nm, simulated {want} nm (±{tol})", dev.name);
                checked += 1;
            }
            if name == "ota" {
                let card = crate::oppoint::flat_circuit(&netlist, &crate::oppoint::OpConfig::default()).unwrap();
                let xm5 = card.lines().find(|l| l.starts_with("XM5 ")).expect("XM5 card");
                assert!(xm5.contains("W=40 L=2 nf=1 m=4"), "{xm5}");
            }
        }
        // ota ×3: 5 each; pair 2, quad 4, chain4 4, rc_filter 2, dac4 9.
        assert_eq!(checked, 36, "every fixture MOS checked");
    }
}

#[cfg(test)]
mod common_node_tests {
    use super::common_node_ohm;
    use pnr_core::DeviceId;

    /// A common node's ΔR budget is the pair's remaining allowance (either
    /// member order) over I_D; an absent pair or an unresolved current reads 0.
    #[test]
    fn common_node_budget_is_the_remaining_allowance() {
        let left = [(0u32, 1u32, 0.237f32)];
        assert!((common_node_ohm(&left, DeviceId(0), DeviceId(1), Some(100.0)) - 2.37).abs() < 1e-4);
        assert!((common_node_ohm(&left, DeviceId(1), DeviceId(0), Some(100.0)) - 2.37).abs() < 1e-4);
        assert_eq!(common_node_ohm(&left, DeviceId(2), DeviceId(3), Some(100.0)), 0.0);
        assert_eq!(common_node_ohm(&left, DeviceId(0), DeviceId(1), None), 0.0);
    }

    /// AF-32: cell pins `d{k}:T` and an injected macro's bare `T` both name a
    /// member; `GND` and a malformed ordinal name none.
    #[test]
    fn pin_member_reads_bare_and_ordinal_pins() {
        assert_eq!(crate::pin_member("d1:D"), Some((1, "D")));
        assert_eq!(crate::pin_member("S"), Some((0, "S")));
        assert_eq!(crate::pin_member("GND"), None);
        assert_eq!(crate::pin_member("dx:S"), None);
    }

    const PAIR: &str = ".subckt pair a b g vss\nXM1 a g vss vss nfet_01v8 W=1u L=0.15u\nXM2 b g vss vss nfet_01v8 W=1u L=0.15u\n.ends pair\n";

    /// A pair whose `XM1` is injected (a real transistor with bare `G/D/S/B`
    /// pins) still has both members' source pins on its common node; before,
    /// the injected member's pins were skipped and `a` was empty.
    #[test]
    fn common_node_sees_injected_macro_pins() {
        let pdk = verify::Pdk::builtin("sky130").expect("sky130 loads");
        let mut nl = crate::parse(PAIR).unwrap();
        crate::deck_models(&mut nl, &pdk);
        // XM1 alone, drawn by the generator, its `d0:T` pins renamed `T`.
        let one = crate::cellgen::enumerate(&nl, &Default::default(), &Default::default(), &pdk, true);
        let cell = one.devices_of.iter().position(|m| m == &[DeviceId(0)]).expect("XM1 has its own cell");
        let mut m = one.spaces[cell].alternatives[0].clone();
        m.pins.iter_mut().for_each(|p| p.name = p.name.strip_prefix("d0:").expect("generated pin").to_string());
        let mut injected = crate::Macros::default();
        injected.register(&nl.devices[0].name, m);
        let cfg = crate::Config::default();
        let bias = crate::Bias { power: Vec::new(), summary: None, currents: None, net_headroom_mv: None, gm_us: Vec::new(), op: None };
        let ann = crate::annotation_with(&pdk, &cfg.annotation, Box::leak(Box::new(crate::elaborate::stack(&pdk))));
        let plan = crate::PerfPlan { rows: Vec::new(), notes: Vec::new(), active: vec![0], tables: Vec::new(), sigma_v: Vec::new(), sens: Vec::new(), sims: 0 };
        let t = crate::topology(&nl, &injected, &pdk, &cfg, &bias, &ann, &plan, true);
        let n = t.flow.cells.variants.len();
        let layout = pnr_core::Layout {
            x: (0..n).map(|i| i as i32 * 20_000).collect(),
            y: vec![0; n],
            hw: vec![50; n],
            hh: vec![50; n],
            axis: vec![0; t.flow.problem.blocks.len().max(1)],
            groups: Vec::new(),
            orient: vec![pnr_core::Orient::default(); n],
            variant: vec![0; n],
            branch: Vec::new(),
            power_uw: vec![0; n],
            temp_mc: vec![0; n],
            units: Default::default(),
        };
        let nodes = t.flow.common_nodes(&layout).nodes;
        assert_eq!(nodes.len(), 1, "one common source node");
        assert!(nodes[0].groups.len() == 2 && nodes[0].groups.iter().all(|g| !g.is_empty()), "both members' source pins: {:?}", nodes[0].groups);
    }

    /// AF-31: an empty cell is named by its members, not by its cell index
    /// read as a device index.
    #[test]
    fn undrawable_names_the_cell_members() {
        let mut nl = crate::parse(PAIR).unwrap();
        crate::deck_models(&mut nl, &verify::Pdk::builtin("sky130").unwrap());
        let cells = [pnr_core::Macro::default(), pnr_core::Macro::default()];
        let rows: Vec<String> = crate::undrawable(&cells, &[vec![DeviceId(1)], vec![DeviceId(0)]], &nl).map(|v| v.rule).collect();
        let (d0, d1) = (&nl.devices[0].name, &nl.devices[1].name);
        assert_eq!(rows.len(), 2);
        assert!(rows[0].contains(d1.as_str()) && !rows[0].contains(d0.as_str()), "{rows:?}");
        assert!(rows[1].contains(d0.as_str()) && !rows[1].contains(d1.as_str()), "{rows:?}");
    }
}
