//! # `library` — the whole flow as one deterministic function.
//!
//! [`run`] reads top-down: parse → annotate → cells → place (`gp` → `dp`) →
//! route (`gr` → `dr`) → signoff, repeated until the best epoch stops
//! improving. [`signoff`] is the final DRC/ERC/LVS/PEX gate.

mod cellgen;
mod fill;
mod geometry;
mod parse;
pub use parse::SizeConvention;

/// Substrate3 elaboration: build a `macro_master::Composition` against a PDK
/// and route its declared nets — the "PDK on the fly" entry.
pub mod elaborate;
pub use elaborate::{elaborate, ElabConfig, Elaborated};

/// Decompile a solved [`Solution`] into a PDK-agnostic generator.
pub mod emit;
/// GDSII stream writer.
pub mod gds;
/// Constraint-budget report: met, met-without-margin, or violated per family.
pub mod metadata;
/// DC operating point via ngspice — the per-device power the thermal rules need.
pub mod oppoint;
pub mod perf;

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
    /// A failed solve falls back to zero power and the report says so.
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
    /// Supplies and their currents, for signoff's EM/IR rules (empty without an
    /// operating point).
    pub intent: verify::Intent,
    /// The fold table the cells were drawn at ([`cellgen::folds`]); LVS
    /// expands the schematic by it.
    pub folds: Vec<(u16, i32)>,
    /// The deck's nwell: bridged wells merge into one rect in [`Solution::geometry`].
    pub well_layer: Option<LayerId>,
}

/// How the search went, and the winning epoch's per-stage legality.
#[derive(Clone, Copy, Debug, Default)]
pub struct RunStats {
    /// Epochs executed, over every variant assignment.
    pub iterations: u32,
    /// Index of the winning epoch within its assignment.
    pub best_iteration: u32,
    /// Stopped feasible with stationary constraint prices, none saturated.
    pub converged: bool,
    /// Dual steps on the constraint prices: one per epoch, so it equals
    /// `iterations` (T6).
    pub dual_steps: u32,
    /// Variant assignments tried.
    pub outer_iterations: u32,
    /// Times an assignment stalled infeasible and the variants were changed.
    pub variant_escalations: u32,
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
    /// Post-layout simulations that could not run ([`perf::evaluate`] `Err`),
    /// over every epoch of every start and cell topology, not just the
    /// winner's; each scored its epoch as every spec unmeasured.
    pub sim_failures: u32,
}

/// Anything that stops the flow.
#[derive(Debug)]
pub enum FlowError {
    Parse(String),
    /// An injected macro for a FET instance does not extract to exactly one
    /// device: `(instance, devices extracted, None = extraction failed)`. It
    /// would unpair LVS for the whole circuit, so it is refused up front.
    InjectedNotADevice(String, Option<usize>),
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

/// Place and route a SPICE netlist against a PDK. Deterministic for `cfg.seed`.
///
/// `injected` maps instance names to user-drawn macros: those devices are used
/// as drawn, never reshaped or moved by `dp`.
pub fn run(spice: &str, pdk: &Pdk, injected: &Macros, cfg: &Config) -> Result<Solution, FlowError> {
    // 1. Parse, naming each device by the deck's model.
    let mut netlist = parse::spice_with(spice, &parse::ParseOptions { size: cfg.size_convention }).map_err(FlowError::Parse)?;
    deck_models(&mut netlist, pdk);

    check_injected(&netlist, injected, pdk)?;

    // 2. Bias: per-device power and per-net current. Placement-independent,
    //    so solved once.
    let (power, bias, currents, net_headroom_mv, gm_us) = bias(&netlist, cfg);
    let bias = Bias { power, summary: bias, currents, net_headroom_mv, gm_us };
    let (perf_rows, perf_bounds) = performance_rows(&netlist, pdk, cfg);

    // 3–7 per cell topology. A distinct-gate pair merged as ABBA cancels a
    // linear gradient but splits one drain across the row ends (asymmetric
    // routing); apart, it routes as translated copies. Neither dominates in
    // general, so when a merge like that exists both are solved and the
    // lexicographically better kept.
    let start = |j: u32| -> (Solution, LexKey) {
        let seed = cfg.seed.wrapping_add(u64::from(j).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let (merged, key, distinct) = solve(&netlist, injected, pdk, cfg, &bias, &perf_rows, true, seed);
        if !distinct {
            return (merged, key);
        }
        let (apart, apart_key, _) = solve(&netlist, injected, pdk, cfg, &bias, &perf_rows, false, seed);
        // Failures count over both topologies, whichever wins.
        let failed = merged.stats.sim_failures + apart.stats.sim_failures;
        let (mut sol, key) = if key_lt(&apart_key, &key) { (apart, apart_key) } else { (merged, key) };
        sol.stats.sim_failures = failed;
        (sol, key)
    };
    // Multi-start: the lex-best start wins; ties go to the earliest.
    let start = &start;
    let runs: Vec<(Solution, LexKey)> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..cfg.starts.max(1)).map(|j| s.spawn(move || start(j))).collect();
        handles.into_iter().map(|h| h.join().expect("a search start panicked")).collect()
    });
    // Failures count over every start, not just the winner's: a failed
    // simulation scores its epoch unmeasured, so selection would hide them.
    let sim_failures = runs.iter().map(|r| r.0.stats.sim_failures).sum();
    let mut sol = runs.into_iter().reduce(|best, r| if key_lt(&r.1, &best.1) { r } else { best }).expect("one start at least").0;
    (sol.stats.sim_failures, sol.metadata.sim_failures) = (sim_failures, sim_failures);
    sol.metadata.budget_rows = perf_bounds;
    Ok(sol)
}

/// Each spec bound as a shared routing budget from schematic sensitivities
/// (see [`perf::budget_rows`]): one baseline plus one run per signal net, in
/// parallel, once per run. Empty without performance scoring, a simulator, or
/// the deck's wire capacitance. Also, per declared bound (`"{metric}:min"` /
/// `":max"`), its row or why it has none ([`metadata::MetadataReport::budget_rows`]).
fn performance_rows(
    netlist: &pnr_core::Netlist,
    pdk: &Pdk,
    cfg: &Config,
) -> (Vec<analog::routing::PerformanceBudget>, Vec<String>) {
    use analog::metadata::NetClass;
    let Some(p) = &cfg.performance else { return (Vec::new(), Vec::new()) };
    let bounds = || {
        p.specs.iter().flat_map(|s| {
            [(s.min, "min"), (s.max, "max")].into_iter().filter(|b| b.0.is_some_and(f64::is_finite)).map(move |(_, side)| format!("{}:{side}", s.metric))
        })
    };
    let notes = |rows: &[analog::routing::PerformanceBudget], why: &str| -> Vec<String> {
        let notes: Vec<String> = bounds()
            .map(|b| match rows.iter().find(|r| r.metric == b) {
                Some(r) if r.nets.is_empty() => format!("{b}: no row (no net sensitivity measured)"),
                Some(r) if r.limit > 0.0 => format!("{b}: row ({} nets)", r.nets.len()),
                Some(_) => format!("{b}: do-not-worsen row (the schematic misses it)"),
                None => format!("{b}: no row ({why})"),
            })
            .collect();
        notes.iter().for_each(|n| eprintln!("[perf] {n}"));
        notes
    };
    let ann = annotation(pdk, &cfg.annotation);
    let Some(af_per_um) = ann.process.wire_af_per_um else {
        return (Vec::new(), notes(&[], "deck has no wire capacitance"));
    };
    let classes = annotate(netlist, &ann).net_classes;
    let nets: Vec<pnr_core::NetId> = classes
        .iter()
        .filter(|c| matches!(c.class, NetClass::Signal | NetClass::Sensitive | NetClass::Clock))
        .map(|c| c.net)
        .collect();
    let names: Vec<String> = nets.iter().map(|n| netlist.nets[n.0 as usize].name.clone()).collect();
    // 10 fF: well above solver noise, small enough to stay linear.
    match perf::sensitivities(netlist, p, &names, 10_000.0) {
        Ok(s) => {
            let rows = perf::budget_rows(p, &s, &nets, af_per_um / 1000.0);
            let notes = notes(&rows, "not measured at the schematic");
            (rows, notes)
        }
        Err(e) => (Vec::new(), notes(&[], &format!("sensitivities unavailable: {e}"))),
    }
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
}

/// Annotate, draw cells and search at one cell topology: `merge_distinct_gates`
/// lets a matched pair on different gate nets merge into one cell. Returns the
/// winner, its key, and whether any such merge happened.
#[allow(clippy::too_many_arguments)]
fn solve(
    netlist: &pnr_core::Netlist,
    injected: &Macros,
    pdk: &Pdk,
    cfg: &Config,
    bias: &Bias,
    perf_rows: &[analog::routing::PerformanceBudget],
    merge_distinct_gates: bool,
    seed: u64,
) -> (Solution, LexKey, bool) {
    let netlist = netlist.clone();
    let currents = bias.currents.clone();
    // Annotate: placement/routing rules + cell constraints, device-indexed.
    let ann = annotation(pdk, &cfg.annotation);
    let mut problem = annotate(&netlist, &ann);
    if cfg.min_utilization > 0.0 {
        problem.placement.budget.push(Box::new(analog::placement::utilization::Utilization { u_min: cfg.min_utilization }));
    }
    for row in perf_rows {
        problem.routing.budget.push(Box::new(row.clone()));
    }

    // Cells: every legal variant drawn once; matched groups collapse to one
    // cell and every device-indexed table moves to cell space.
    // One fold table for the cells and every LVS reference of this run.
    let fold = cellgen::folds(&netlist, pdk, &bias.gm_us);
    let cells = CellSpace::new(&netlist, injected, &mut problem, pdk, &bias.power, merge_distinct_gates, &fold);
    let distinct = cells.distinct_gate_merges > 0;

    // 5. Stages. The metal stack and router config come from the deck.
    let (layers, cuts, pin_access) = elaborate::routing_stack(pdk);
    // EM limits on the pin-access layer and cut too (sky130 mcon 0.36 mA/cut):
    // the access jogs and pin cuts carry their terminal's current.
    let em_layers: Vec<LayerId> = layers.iter().copied().chain(pin_access.map(|p| p.0)).collect();
    let em_cuts: Vec<elaborate::Cut> = cuts.iter().copied().chain(pin_access.map(|p| p.1)).collect();
    let em = elaborate::em_limits(pdk, &em_layers, &em_cuts, cfg.op.as_ref().map(|o| o.temp_c as f32 + 273.15));
    em_rules(&mut problem, &netlist, &em, &em_layers, &em_cuts, ann.process.stack);
    // IR-drop budgets (PWR-02) on nets carrying op current (`annotator::ir`).
    if let (Some(c), Some(h)) = (&bias.currents, &bias.net_headroom_mv) {
        let vdd_mv = cfg.op.as_ref().map_or(1_800.0, |o| o.vdd * 1e3);
        let i = oppoint::net_current_ua(&netlist, c);
        let rules: Vec<analog::routing::IrDrop> = annotator::ir::budgets(&problem.net_classes, &i, h, vdd_mv)
            .into_iter()
            .map(|(net, current_ua, max_drop_uv)| analog::routing::IrDrop { net, current_ua, max_drop_uv, margin_pct: 20, stack: ann.process.stack })
            .collect();
        problem.routing.budget.push(Box::new(rules));
    } else {
        problem.missing.push(("IrDrop", "operating point"));
    }
    let sens: Vec<(pnr_core::NetId, f32)> =
        perf_rows.iter().flat_map(|r| r.nets.iter().copied().zip(r.weights.iter().copied())).collect();
    let net_weight = gp::net_weights(&problem.net_classes, &sens);
    let intent = elaborate::intent(&netlist, &problem.net_classes, currents.as_deref(), cfg.op.as_ref().map_or(0.0, |o| o.vdd * 1_000.0));
    let flow = Flow {
        pdk,
        netlist: &netlist,
        net_names: netlist.nets.iter().map(|n| n.name.clone()).collect(),
        d_router: {
            let mut r = elaborate::detailed_router(pdk, &layers, &cuts, pin_access);
            r.cfg.supply_nets = problem
                .net_classes
                .iter()
                .filter(|c| matches!(c.class, analog::metadata::NetClass::Supply | analog::metadata::NetClass::Ground))
                .map(|c| c.net)
                .collect();
            r.cfg.pin_ua = currents.as_deref().map_or_else(Vec::new, |c| pin_currents(&netlist, &cells.devices_of, c));
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
        perf: cfg.performance.as_ref(),
        intent: intent.clone(),
        net_weight,
        fold: fold.clone(),
        stack: ann.process.stack.expect("`annotation` always carries the stack"),
        id_ua: currents
            .as_ref()
            .map(|c| c.iter().map(|d| d.as_ref().and_then(|t| t.iter().find(|(n, _)| n == "D").map(|&(_, i)| i))).collect())
            .unwrap_or_default(),
        avt_mv_um: ann.process.avt_mv_um,
        offset_sigma_mv: ann.offset_sigma_mv,
        gp_mode: cfg.gp_mode,
    };

    // 6. Search. Outer: variant assignment. Middle: epochs at that assignment,
    //    keeping the best [`LexKey`], whose V includes the epoch's own signoff
    //    errors. Prices and routing history persist across epochs.
    let mut assignment = cellgen::seed_assignment(&flow.cells.variants, &flow.layers, pdk);
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
        let Some(next) = cellgen::escalate(&flow.cells.variants, &assignment) else {
            break;
        };
        stats.variant_escalations += 1;
        assignment = next;
    }

    // 7. The winner, redrawn from its own variant choice, with its guard rings.
    let best = best.expect("at least one epoch ran");
    stats.dual_steps = prices.steps();
    stats = RunStats {
        best_iteration: best.iteration,
        ..best.stats.merge(stats)
    };
    let mut macros = cellgen::realize(&flow.cells.variants, &best.layout.variant);
    // Only a winner claiming zero hard violations must be fully connected; an
    // infeasible winner's opens are already counted and reported at signoff.
    if best.key.0 == 0 {
        best.routes.debug_check("dr::route (winner)");
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
        bias.summary.clone(),
        &flow.problem.net_classes,
        &flow.problem.missing,
        &pdk.unverified(),
    );
    let mut metadata = metadata;
    metadata.binding = prices.saturated().iter().map(|k| (*k).to_string()).collect();
    metadata.coverage = best.coverage;
    metadata.add_routing(&[Box::new(flow.common_nodes(&best.layout)), Box::new(flow.environment(&best.layout, &best.rings))], &best.routes);
    if let (Some(cfg), Some(result)) = (flow.perf, &best.perf) {
        metadata.performance = cfg
            .specs
            .iter()
            .zip(&result.metrics)
            .map(|(s, (m, v))| (m.clone(), *v, s.min, s.max, perf::miss(s, *v)))
            .collect();
        metadata.sim_failures = stats.sim_failures;
    }
    let placement = flow.problem.placement;
    let key = best.key;
    // Inserted devices (antenna diodes) join the schematic LVS reads.
    let mut netlist = netlist;
    netlist.devices.extend(best.extra);
    let solution = Solution {
        layout: best.layout,
        routes: best.routes,
        macros,
        netlist,
        stats,
        metadata,
        placement,
        intent,
        folds: fold,
        well_layer: pnr_core::Process::layer(pdk, "nwell"),
    };
    (solution, key, distinct)
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
    /// Post-layout performance scoring, when configured.
    perf: Option<&'a perf::PerfConfig>,
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
    /// Deck `A_VT`, mV·µm, `[nmos, pmos]`.
    avt_mv_um: [Option<f32>; 2],
    /// The pair offset budget the matching rules allocate from.
    offset_sigma_mv: Option<f32>,
    gp_mode: GpMode,
}

/// `base` plus what the annotator needs from the deck.
#[must_use]
pub fn annotation(pdk: &Pdk, base: &AnnotationConfig) -> AnnotationConfig {
    use pnr_core::Process;
    let (layers, ..) = elaborate::routing_stack(pdk);
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
        svt_uv_per_um: pos("svt_uv_per_um"),
        vt_tc_uv_per_k: [pos("vt_tc_uv_per_k"), pos("vt_tc_uv_per_k_p")],
        lod_kvth0_mv_um: [pos("lod_kvth0_n_mv_um"), pos("lod_kvth0_p_mv_um")],
        epi_nm: pos("epi_thickness_nm").map(|v| v as i32),
        // Rules are `Copy`, so they borrow the stack for 'static.
        // ponytail: leaked once per `annotation` call (twice per run, a few
        // hundred bytes each); cache by deck if runs ever loop in one process.
        stack: Some(Box::leak(Box::new(elaborate::stack(pdk)))),
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
        let placement = &self.problem.placement;
        let cells = &self.cells;
        let layers = &self.layers;

        // Place: coarse analytical, then legalising anneal (which may reshape).
        let macros = cellgen::realize(&cells.variants, assignment);
        let (mut coarse, _) = gp::place(&macros, &cells.variants, assignment, placement, prices, place_rules(self.pdk), &self.net_weight, seed, self.gp_mode == GpMode::Analytic);
        coarse.debug_check("gp::place");
        // dp reads groups as abutment permission, so it gets the diffusion-sharing
        // table; after dp, groups are the recognition table for `Target::Group`.
        coarse.groups = cells.abutment.clone();
        if coarse.axis.len() < self.problem.blocks.len() {
            let centre = coarse.centre_x_estimate();
            coarse.axis.resize(self.problem.blocks.len(), centre);
        }
        coarse.power_uw = cells.power.clone();
        coarse.units = cells.units.clone();
        coarse.refresh_temps();
        let (mut layout, place_report, dp_stats) = dp::place(
            &coarse,
            &macros,
            if reshape { &cells.variants } else { &[] },
            placement,
            &cells.fixed,
            prices,
            place_rules(self.pdk),
            &self.net_weight,
            seed,
        );
        layout.debug_check_placed("dp::place");
        layout.groups = cells.groups.clone();
        // The epoch's one dual step, on the layout it is scored on (T6).
        prices.settle(placement, &layout);
        let macros = if layout.variant == assignment {
            macros
        } else {
            cellgen::realize(&cells.variants, &layout.variant)
        };
        // Measured on the macros dp's variants draw, so `lattice_off` stamps what is drawn.
        let lattice = cells::builder::cut_lattice(self.pdk);
        let place = geometry::placement_metrics(&macros, &layout, lattice, place_rules(self.pdk).clearance, placement);

        // Guard rings enclose placed cells, so they are drawn now, before routing.
        let mut rings = cells::post_cell::guard_rings(&layout, &cells.guard_rings, self.pdk, ring_cut_ohm(self.pdk));
        // Same-bulk PMOS cells facing each other share one well.
        let bridges = cells::post_cell::well_bridges(&gr::place_macros(&macros, &layout), &rings, self.pdk);
        rings.extend(bridges);
        // Same-type implants of neighbours closer than their spacing merge.
        let placed_now: Vec<Macro> = gr::place_macros(&macros, &layout).into_iter().chain(rings.iter().cloned()).collect();
        rings.extend(cells::post_cell::implant_bridges(&placed_now, self.pdk));

        // Route: global gcell plan, then track realisation onto the real pins.
        let routing = &self.problem.routing;
        let (global, _) =
            gr::GlobalRoute { net_weight: self.d_router.cfg.net_weight.clone(), ..Default::default() }.route(&layout, &macros, &rings, routing, layers, neg);
        global.debug_check("gr::route");
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
                stack: Some(self.stack),
                pin_share: macros.iter().map(pnr_core::pin_shares).collect(),
                ..self.d_router.cfg.clone()
            },
        };
        let (mut routes, mut route_report) = router.route(
            &global, &pins, &placed, &rings, routing, layers, &self.cuts, neg,
        );
        // Antenna nets the jumper could not fix get a diode each, routed in as
        // a fixed cell; its shape on the deck's credited diode layer joins the
        // net's routes (the rule's credit, `Stack::diode`).
        let ground = self.problem.net_classes.iter().find(|c| c.class == analog::metadata::NetClass::Ground).map(|c| c.net);
        let diodes = elaborate::antenna_diodes(self.pdk, routing, &routes, &placed, &rings, ground, place_rules(self.pdk).clearance);
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
            (routes, route_report) = router.route(&global, &pins, &placed, &rings, routing, layers, &self.cuts, neg);
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
        let signoff = signoff_shapes(&self.intent, &shapes, &labelled, &self.net_names, &netlist, Some(&self.fold), self.pdk);
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

        let (key, stats) = epoch_score(&place_report, &route_report, &signoff, &budgets, layout.footprint_nm2());
        let stats = RunStats { place, dp: dp_stats, ..stats };
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
        }
    }
}

impl Flow<'_> {
    /// Promote `epoch` to post-layout simulation and fold the spec miss into
    /// its key. No extraction or no simulator: every spec unknown (a full
    /// miss each), never passed.
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
                let Some((k, t)) = p.name.strip_prefix('d').and_then(|r| r.split_once(':')) else { continue };
                let Some(&d) = k.parse::<usize>().ok().and_then(|k| members.get(k)) else { continue };
                if let Some(v) = pins.get_mut(p.net.0 as usize) {
                    v.push((d.0 as usize, t.to_string(), p.at));
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
        perf::Parasitics { caps: epoch.caps.clone(), series, lod_inv_um }
    }

    /// Each recognised matched pair on one source net, with its members'
    /// source pins and the net's other pins (feeds) as placed, budgeted
    /// `ΔR ≤ η·σ_rand / I_D` with the matching rules' `η`
    /// ([`annotator::emit::systematic_allowance_mv`]).
    fn common_nodes(&self, layout: &Layout) -> analog::routing::CommonNodes {
        use annotator::BlockKind::{CurrentMirror, DiffPair, Load};
        let placed = gr::place_macros(&cellgen::realize(&self.cells.variants, &layout.variant), layout);
        let term = |d: DeviceId, t: &str| self.netlist.devices[d.0 as usize].terminals.iter().find(|(n, _)| n == t).map(|&(_, n)| n);
        // (device, terminal, rect) of every placed pin, per net.
        let mut on_net: Vec<Vec<(DeviceId, String, pnr_core::Rect)>> = vec![Vec::new(); self.netlist.nets.len()];
        for (m, members) in placed.iter().zip(&self.cells.devices_of) {
            for p in &m.pins {
                let Some((k, t)) = p.name.strip_prefix('d').and_then(|r| r.split_once(':')) else { continue };
                let Some(&d) = k.parse::<usize>().ok().and_then(|k| members.get(k)) else { continue };
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
            // σ_rand = A_VT/√(W_total·L·m) of member a, over its drain current.
            let dev = &self.netlist.devices[a.0 as usize];
            let avt = if dev.kind == pnr_core::DeviceKind::Nmos { self.avt_mv_um[0] } else { self.avt_mv_um[1] };
            let gate_um2 = dev.gate_area_um2() as f32;
            let i_ua = self.id_ua.get(a.0 as usize).copied().flatten().map(|i| i.abs() as f32).filter(|&i| i > 0.0);
            let max_delta_ohm = match (annotator::emit::systematic_allowance_mv(avt, gate_um2, self.offset_sigma_mv), i_ua) {
                (Some(mv), Some(i)) => mv / i * 1e3,
                _ => 0.0,
            };
            nodes.push(analog::routing::CommonNode { net, a: pins(a), b: pins(b), feeds, max_delta_ohm });
        }
        analog::routing::CommonNodes { nodes, stack: self.stack }
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
                wpe_min_nm: self.pdk.rule("wpe_clearance_moderate", 0) as f32,
                ose_range_nm: self.pdk.rule("lod_moat_ext_moderate", 0) as f32,
            });
        }
        analog::placement::Environment(out)
    }

    /// Simulate `epoch` on its parasitics; a simulator that cannot run counts
    /// in `stats.sim_failures` and scores every spec unmeasured.
    fn score_perf(&self, epoch: &mut Epoch, stats: &mut RunStats) {
        let Some(p) = self.perf else { return };
        let unknown = || perf::PerfResult {
            metrics: p.specs.iter().map(|s| (s.metric.clone(), None)).collect(),
            residual: p.specs.len() as f64,
        };
        let result = if epoch.caps.is_empty() {
            unknown()
        } else {
            perf::evaluate(self.netlist, &self.parasitics(epoch), p).unwrap_or_else(|e| {
                eprintln!("[perf] {e}");
                stats.sim_failures += 1;
                unknown()
            })
        };
        epoch.key.1 = result.residual;
        epoch.perf = Some(result);
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
            ..run
        }
    }
}

/// `(|V|, spec miss, Θ, extracted C, footprint nm²)`, compared by [`key_lt`]:
/// no parasitic gain buys past a budget residual, no budget slack past a missed
/// circuit spec, nothing past a hard violation. V counts violated hard rules
/// ([`metadata::MetadataReport::hard_violated`]), the stages' own non-batch
/// rows, and signoff errors (deck warnings are never in that report). Θ is
/// [`metadata::MetadataReport::theta`] plus dr's own non-batch budget rows, all
/// in milli-budgets. The spec miss is the post-layout simulation's Σ normalised
/// miss (`0` without performance scoring); C is signoff's extracted total (fF).
type LexKey = (usize, f64, f64, f32, f64);

/// Relative extracted-C difference read as a tie, which area then breaks.
///
/// ponytail: a flat 2%, about the seed-to-seed C spread; a declared spec on C
/// or area would replace it.
const C_TIE: f32 = 0.02;

/// `a` beats `b`: `|V|`, spec miss, Θ lexicographically, then extracted C —
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
    footprint_nm2: f64,
) -> (LexKey, RunStats) {
    let key = lex_key(place, route, &signoff.report, budgets, None, footprint_nm2);
    let stats = RunStats {
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
    footprint_nm2: f64,
) -> LexKey {
    let own = |r: &Report| r.hard_violations.iter().filter(|v| !v.is_batch_row()).count();
    // `lvs-coverage/` rows (devices no deck recogniser extracts) are the same
    // every epoch: no layout fixes them, so they stay out of |V| or no design
    // with a BJT/MOM could ever read feasible, converge or debug-check its
    // winner. They stay in the report, so signoff and bench still see them.
    let checked = signoff.hard_violations.iter().filter(|v| !v.rule.starts_with("lvs-coverage/")).count();
    let v = budgets.hard_violated() + own(place) + own(route) + checked;
    let theta = budgets.theta()
        + route.budget_violations.iter().filter(|x| !x.is_batch_row()).map(|x| x.margin as f64).sum::<f64>();
    (v, perf.map_or(0.0, |p| p.residual), theta, signoff.cost, footprint_nm2)
}

/// Edge-to-edge gap `dp` keeps between cells: the deck's widest same-layer
/// spacing, so no two cells' layers can merge. (`dp`'s default is a sky130
/// guess; measured: chain4 ERC 93 → 74 with the deck value.)
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
/// once as a missing input: its shapes go unchecked.
fn em_rules(
    problem: &mut Problem,
    netlist: &pnr_core::Netlist,
    em: &[(pnr_core::LayerId, analog::routing::em::Limit)],
    metals: &[LayerId],
    cuts: &[elaborate::Cut],
    stack: Option<&'static analog::routing::Stack>,
) {
    use analog::routing::em::{Limit, MAX_LAYERS};
    let mut limits = [(u16::MAX, Limit::default()); MAX_LAYERS];
    for (slot, &(l, lim)) in limits.iter_mut().zip(em) {
        *slot = (l.0, lim);
    }
    let has = |l: LayerId, f: fn(&Limit) -> f32| limits.iter().any(|(x, lim)| *x == l.0 && f(lim) > 0.0);
    if !metals.iter().all(|&l| has(l, |e| e.ua_per_um)) || !cuts.iter().all(|&(c, ..)| has(c, |e| e.ua_per_cut)) {
        problem.missing.push(("Electromigration", "a routed or pin-access layer has no deck EM limit (unchecked)"));
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
#[allow(clippy::type_complexity)]
fn bias(
    netlist: &pnr_core::Netlist,
    cfg: &Config,
) -> (Vec<i32>, Option<metadata::BiasSummary>, Option<Vec<Option<Vec<(String, f64)>>>>, Option<Vec<Option<f64>>>, Vec<Option<f64>>) {
    let op = cfg
        .op
        .as_ref()
        .and_then(|oc| match oppoint::extract(netlist, oc) {
            Ok(o) => Some(o),
            Err(e) => {
                eprintln!("[op] operating point unavailable ({e}); continuing with zero power");
                None
            }
        });
    let Some(o) = op else {
        return (cfg.device_power_uw.clone(), None, None, None, Vec::new());
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
    };
    let currents = o.terminal_ua(netlist);
    let headroom = o.net_headroom_mv(netlist);
    (o.power_uw, Some(summary), Some(currents), Some(headroom), o.gm_us)
}

/// The collapsed cell table and every device-indexed input translated to it.
struct CellSpace {
    /// Pre-drawn alternatives per cell — what `gp`/`dp` search over.
    variants: Vec<gp::VariantSpace>,
    /// Injected (user-macro) cells: `dp` never moves or reshapes them.
    fixed: Vec<bool>,
    /// `Layout::groups` for `dp`: groups that may share diffusion.
    abutment: Vec<Vec<DeviceId>>,
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
            abutment: problem.abutment.iter().map(to_cells).collect(),
            groups,
            variants: spaces,
            guard_rings,
            units: std::sync::Arc::default(),
            distinct_gate_merges,
            devices_of: Vec::new(),
        };
        // Reserve each ring's halo in the requester's bbox so the placer keeps
        // neighbours out of it; the ring is drawn back inside the reservation.
        for r in &cells.guard_rings.guard_rings {
            let ext = round_up(cells::post_cell::ring_halo(r, pdk, ring_cut_ohm(pdk)), pdk.grid.max(1));
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
        axis: vec![0; problem.blocks.len().max(1)],
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
    // A device this process has no construction for (an NPN without a deep
    // well, a poly resistor on a fin process) is drawn as nothing: a hard
    // finding, never hidden behind an LVS that cannot see it either.
    for (i, m) in sol.macros.iter().take(sol.layout.x.len()).enumerate() {
        if m.shapes.is_empty() {
            let (name, model) = sol.netlist.devices.get(i).map_or(("?", "?"), |d| (d.name.as_str(), d.model.as_str()));
            s.report.hard_violations.push(pnr_core::report::Violation { rule: format!("cell/undrawable: {name} ({model}) has no construction on this process"), margin: 1 });
        }
    }
    s
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
    /// Parallel starts stay deterministic: the same seed and start count give
    /// the same layout, however the threads interleave.
    #[test]
    fn multi_start_is_deterministic() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let Ok(json) = std::fs::read_to_string(root.join("pdks/sky130.json")) else { return };
        let pdk = verify::Pdk::from_json(&json).expect("sky130 loads");
        let spice = ".subckt p d g VSS\nXM1 d g VSS VSS nfet_01v8 W=2u L=0.5u\nXM2 d x VSS VSS nfet_01v8 W=2u L=0.5u\n.ends p\n";
        let cfg = crate::Config { seed: 7, feedback_iters: 2, outer_iters: 1, starts: 3, ..Default::default() };
        let run = || crate::run(spice, &pdk, &Default::default(), &cfg).expect("flow");
        let (a, b) = (run(), run());
        assert_eq!((a.layout.x, a.layout.y), (b.layout.x, b.layout.y));
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
            testbench: String::new(),
            specs: vec![crate::perf::Spec { metric: "gain".into(), min: Some(20.0), max: Some(60.0) }],
        };
        let cfg = crate::Config { feedback_iters: 2, outer_iters: 1, starts: 1, performance: Some(performance), ..Default::default() };
        let sol = crate::run(&spice, &pdk, &Default::default(), &cfg).expect("flow");
        assert!(sol.stats.sim_failures >= 1, "{:?}", sol.stats);
        assert_eq!(sol.metadata.sim_failures, sol.stats.sim_failures);
        let rows = &sol.metadata.budget_rows;
        assert_eq!(rows.len(), 2, "{rows:?}");
        assert!(rows[0].starts_with("gain:min: no row (sensitivities unavailable"), "{rows:?}");
        assert!(rows[1].starts_with("gain:max: no row (sensitivities unavailable"), "{rows:?}");
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
            testbench: String::new(),
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
        let nl = Netlist { devices: vec![fet("M0"), fet("M1"), fet("M2")], nets: ["a", "b", "c", "d"].iter().map(|n| Net { name: (*n).into() }).collect() };
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
        let lib = std::env::var_os("PDK_ROOT")
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| std::path::Path::new(&h).join(".volare")))
            .map(|r| r.join("sky130A/libs.tech/ngspice/sky130.lib.spice"))
            .filter(|l| l.is_file());
        let (Some(lib), true) = (lib, std::process::Command::new("ngspice").arg("--version").output().is_ok()) else {
            eprintln!("SKIP fixture_nets_without_a_resistor_keep_em_sizing: needs ngspice and the sky130 ngspice models");
            return;
        };
        let pdk = verify::Pdk::from_json(&std::fs::read_to_string(root.join("pdks/sky130.json")).unwrap()).unwrap();
        let cfg = crate::Config { op: Some(crate::oppoint::OpConfig { model_lib: Some(lib), ..Default::default() }), ..Default::default() };
        for fixture in ["rc_filter", "dac4"] {
            let mut nl = crate::parse(&std::fs::read_to_string(root.join(format!("benchmarks/fixtures/{fixture}.spice"))).unwrap()).unwrap();
            crate::deck_models(&mut nl, &pdk);
            let currents = crate::bias(&nl, &cfg).2.expect("operating point");
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
        BudgetStatus { kind: "K".into(), arm, total, satisfied, unknown: 0, criticality: 0.0, residual, usage: None, violated: Vec::new() }
    }

    fn rows(rules: &[&str]) -> Vec<Violation> {
        rules.iter().map(|r| Violation { rule: (*r).into(), margin: 1 }).collect()
    }

    fn key(place: &Report, signoff: &Report, budgets: &MetadataReport) -> crate::LexKey {
        crate::lex_key(place, &Report::default(), signoff, budgets, None, 1.0)
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
            crate::epoch_score(&Report::default(), &Report::default(), &signoff, &MetadataReport::default(), 1.0);
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
            let fold = crate::cellgen::folds(&netlist, &pdk, &[]);
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
                let card = crate::oppoint::flat_circuit(&netlist, &crate::oppoint::OpConfig::default());
                let xm5 = card.lines().find(|l| l.starts_with("XM5 ")).expect("XM5 card");
                assert!(xm5.contains("W=40 L=2 nf=1 m=4"), "{xm5}");
            }
        }
        // ota ×3: 5 each; pair 2, quad 4, chain4 4, rc_filter 2, dac4 9.
        assert_eq!(checked, 36, "every fixture MOS checked");
    }
}
