//! # `library` — the whole flow as one deterministic function.
//!
//! [`run`] reads top-down: parse → annotate → cells → place (`gp` → `dp`) →
//! route (`gr` → `dr`) → in-loop DRC, repeated until the best epoch stops
//! improving. [`signoff`] is the final DRC/ERC/LVS/PEX gate.

mod cellgen;
mod geometry;
mod parse;

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

use analog::Requirements;
use annotator::{annotate, AnnotationConfig, NoInference, Problem};
use dp::DetailedPlacer;
use dr::DetailedRouter;
use gp::GlobalPlacer;
use gr::GlobalRouter;
pub use macro_master::Macros;
use pnr_core::{DeviceId, LayerId, Layout, Macro, Report, Routes};
use verify::Pdk;
/// The GDS/geometry viewer, re-exported so callers get it through this crate.
pub use visualizer;

/// Repeatable-run configuration.
pub struct Config {
    /// Base RNG seed; every epoch derives its own from it.
    pub seed: u64,
    /// Epochs per variant assignment (place → route → DRC → fold back).
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
}

/// How the search went, and the winning epoch's per-stage legality.
#[derive(Clone, Copy, Debug, Default)]
pub struct RunStats {
    /// Epochs executed, over every variant assignment.
    pub iterations: u32,
    /// Index of the winning epoch within its assignment.
    pub best_iteration: u32,
    /// Stopped feasible with stationary constraint prices.
    pub converged: bool,
    /// Variant assignments tried.
    pub outer_iterations: u32,
    /// Times an assignment stalled infeasible and the variants were changed.
    pub variant_escalations: u32,
    /// Winner: detailed-placement hard violations.
    pub place_hard: usize,
    /// Winner: detailed-routing hard violations.
    pub route_hard: usize,
    /// Winner: in-loop DRC violations over its drawn geometry.
    pub drc_hard: usize,
    /// Winner: Σ routing budget margins (milli-budgets, not tracks).
    pub route_overuse: i64,
}

/// Anything that stops the flow.
#[derive(Debug)]
pub enum FlowError {
    Parse(String),
}

/// Epochs without improvement before an assignment counts as stalled.
const PATIENCE: u32 = 8;
/// `‖λ_{k+1} − λ_k‖` below which constraint prices count as stationary.
const PRICE_STATIONARY: f64 = 1e-3;

/// Place and route a SPICE netlist against a PDK. Deterministic for `cfg.seed`.
///
/// `injected` maps instance names to user-drawn macros: those devices are used
/// as drawn, never reshaped or moved by `dp`.
pub fn run(spice: &str, pdk: &Pdk, injected: &Macros, cfg: &Config) -> Result<Solution, FlowError> {
    // 1. Parse.
    let netlist = parse::spice(spice).map_err(FlowError::Parse)?;

    // 2. Annotate: placement/routing rules + cell constraints, device-indexed.
    let mut problem = annotate(&netlist, &NoInference, &cfg.annotation);

    // 3. Bias: per-device power. Placement-independent, so solved once.
    let (power, bias) = bias(&netlist, cfg);

    // 4. Cells: every legal variant drawn once; matched groups collapse to one
    //    cell and every device-indexed table moves to cell space.
    let cells = CellSpace::new(&netlist, injected, &mut problem, pdk, &power);

    // 5. Stages. The metal stack and router config come from the deck.
    let (layers, cuts, pin_access) = elaborate::routing_stack(pdk);
    let mut flow = Flow {
        pdk,
        d_router: elaborate::detailed_router(pdk, &layers, &cuts, pin_access),
        layers,
        cuts,
        oracle: verify::LiveOracle::new(pdk).expect("a loaded Pdk re-parses its own deck"),
        base_hard: problem.placement.hard.len(),
        base_cost: problem.placement.cost.len(),
        problem,
        cells,
    };

    // 6. Search. Outer: variant assignment. Middle: epochs at that assignment,
    //    keeping the lexicographically best (|V|, Θ, PEX). Cross-epoch state
    //    (prices, routing history, DRC findings) lives here so it persists.
    let mut assignment = cellgen::seed_assignment(&flow.cells.variants, &flow.layers, pdk);
    let mut prices = gp::Prices::new();
    let mut neg = gr::Negotiation::new();
    let mut drc_feedback = Requirements::<Layout>::default();
    let mut best: Option<Epoch> = None;
    let mut stats = RunStats::default();

    'outer: for outer in 0..cfg.outer_iters.max(1) {
        stats.outer_iterations += 1;
        let mut stall = 0;
        for iter in 0..cfg.feedback_iters.max(1) {
            stats.iterations += 1;
            let seed = cfg.seed ^ u64::from(iter) ^ (u64::from(outer) << 32);
            let (epoch, feedback) = flow.epoch(&assignment, drc_feedback, &mut prices, &mut neg, seed);
            drc_feedback = feedback;
            if best.as_ref().is_none_or(|b| epoch.key < b.key) {
                best = Some(Epoch { iteration: iter, ..epoch });
                stall = 0;
            } else {
                stall += 1;
                if stall >= PATIENCE {
                    break;
                }
            }
        }

        // Feasible with settled prices: done.
        let feasible = best.as_ref().is_some_and(|b| b.key.0 == 0 && b.key.1 <= 0.0);
        if feasible && prices.drift() < PRICE_STATIONARY {
            stats.converged = true;
            break;
        }
        // Infeasible: the variants themselves bind. Try the next assignment, or
        // stop when the variant space is exhausted.
        match cellgen::escalate(&flow.cells.variants, &assignment) {
            Some(next) => {
                stats.variant_escalations += 1;
                assignment = next;
            }
            None => break 'outer,
        }
    }

    // 7. The winner, redrawn from its own variant choice, with its guard rings.
    let best = best.expect("at least one epoch ran");
    stats = RunStats { best_iteration: best.iteration, ..best.stats.merge(stats) };
    let mut macros = cellgen::realize(&flow.cells.variants, &best.layout.variant);
    // Only a winner claiming zero hard violations must be fully connected; an
    // infeasible winner's opens are already counted and reported at signoff.
    if best.key.0 == 0 {
        best.routes.debug_check("dr::route (winner)");
        geometry::debug_check_connected(&macros, &best.layout, &best.routes);
    }
    macros.extend(best.rings);
    let metadata = metadata::build(
        &flow.problem.placement,
        &best.layout,
        &flow.problem.routing,
        &best.routes,
        bias,
        &flow.problem.net_classes,
    );
    Ok(Solution { layout: best.layout, routes: best.routes, macros, netlist, stats, metadata })
}

/// Everything an epoch reads that is fixed for the run.
struct Flow<'a> {
    pdk: &'a Pdk,
    /// Placement rules are cell-indexed; `placement.hard/cost` grow by the DRC
    /// feedback during an epoch and are truncated back to `base_*` after.
    problem: Problem,
    base_hard: usize,
    base_cost: usize,
    cells: CellSpace,
    layers: Vec<LayerId>,
    cuts: Vec<elaborate::Cut>,
    d_router: dr::DetailedRoute,
    oracle: verify::LiveOracle,
}

/// One scored epoch.
struct Epoch {
    key: LexKey,
    iteration: u32,
    layout: Layout,
    routes: Routes,
    rings: Vec<Macro>,
    stats: RunStats,
}

impl Flow<'_> {
    /// Place → route → DRC at a fixed `assignment`. Returns the scored epoch and
    /// its DRC findings, to be folded into the next epoch's placement rules.
    fn epoch(
        &mut self,
        assignment: &[u16],
        mut drc_feedback: Requirements<Layout>,
        prices: &mut gp::Prices,
        neg: &mut gr::Negotiation,
        seed: u64,
    ) -> (Epoch, Requirements<Layout>) {
        let placement = &mut self.problem.placement;
        placement.hard.append(&mut drc_feedback.hard);
        placement.cost.append(&mut drc_feedback.cost);
        let cells = &self.cells;
        let layers = &self.layers;

        // Place: coarse analytical, then legalising anneal (which may reshape).
        let macros = cellgen::realize(&cells.variants, assignment);
        let (mut coarse, _) =
            gp::Analytical::default().place(&macros, &cells.variants, placement, layers, prices, seed);
        coarse.debug_check("gp::place");
        // dp reads groups as abutment permission, so it gets the diffusion-sharing
        // table; after dp, groups are the recognition table for `Target::Group`.
        coarse.groups = cells.abutment.clone();
        if coarse.axis.len() < self.problem.blocks.len() {
            let centre = coarse.centre_x_estimate();
            coarse.axis.resize(self.problem.blocks.len(), centre);
        }
        coarse.power_uw = cells.power.clone();
        coarse.refresh_temps();
        let (mut layout, place_report) = dp::Annealer::default().place(
            &coarse,
            &macros,
            &cells.variants,
            placement,
            layers,
            &cells.fixed,
            prices,
            &self.oracle,
            seed,
        );
        layout.debug_check_placed("dp::place");
        layout.groups = cells.groups.clone();
        let macros = if layout.variant == assignment {
            macros
        } else {
            cellgen::realize(&cells.variants, &layout.variant)
        };

        // Guard rings enclose placed cells, so they are drawn now, before routing.
        let rings = cells::post_cell::guard_rings(&layout, &cells.guard_rings, self.pdk);

        // Route: global gcell plan, then track realisation onto the real pins.
        let routing = &self.problem.routing;
        let (global, _) =
            gr::GlobalRoute::default().route(&layout, &macros, &rings, routing, layers, neg, seed);
        global.debug_check("gr::route");
        let placed = gr::place_macros(&macros, &layout);
        let pins: Vec<_> =
            placed.iter().flat_map(|m| m.pins.iter().map(|p| (p.net, p.at, p.layer))).collect();
        let (routes, route_report) = self.d_router.route(
            &global, &pins, &placed, &rings, routing, layers, &self.cuts, neg, seed,
        );

        // Measure: DRC over the drawn geometry, budget residuals over the result.
        let mut shapes = geometry::collect(&macros, &layout, &routes);
        shapes.extend(rings.iter().flat_map(|r| r.shapes.iter().copied()));
        let (feedback, drc) = verify::drc_feedback(&self.oracle, &shapes);
        let budgets = metadata::build(
            &self.problem.placement,
            &layout,
            routing,
            &routes,
            None,
            &self.problem.net_classes,
        );
        self.problem.placement.hard.truncate(self.base_hard);
        self.problem.placement.cost.truncate(self.base_cost);

        let drc_hard = drc.violations as usize;
        let key = lex_key(&place_report, &route_report, drc_hard, &budgets);
        let stats = RunStats {
            place_hard: place_report.hard_violations.len(),
            route_hard: route_report.hard_violations.len(),
            drc_hard,
            route_overuse: route_report.budget_violations.iter().map(|v| v.margin).sum(),
            ..RunStats::default()
        };
        (Epoch { key, iteration: 0, layout, routes, rings, stats }, feedback)
    }
}

impl RunStats {
    /// The winner's per-stage legality with the run-wide counters of `run`.
    fn merge(self, run: RunStats) -> RunStats {
        RunStats { place_hard: self.place_hard, route_hard: self.route_hard, drc_hard: self.drc_hard, route_overuse: self.route_overuse, ..run }
    }
}

/// `(|V|, Θ, PEX)` summed over stages and compared as a tuple: no parasitic gain
/// buys past a budget residual, no budget slack buys past a hard violation.
type LexKey = (usize, f64, f32);

fn lex_key(place: &Report, route: &Report, drc: usize, budgets: &metadata::MetadataReport) -> LexKey {
    let (pv, pt, pc) = place.lex();
    let (rv, rt, rc) = route.lex();
    (pv + rv + drc, pt + rt + budgets.theta(), pc + rc)
}

fn round_up(v: i32, grid: i32) -> i32 {
    (v + grid - 1) / grid * grid
}

/// Per-device power (µW) and its provenance for the report: the ngspice
/// operating point when configured and solvable, else `cfg.device_power_uw`.
fn bias(
    netlist: &pnr_core::Netlist,
    cfg: &Config,
) -> (Vec<i32>, Option<metadata::BiasSummary>) {
    let op = cfg.op.as_ref().and_then(|oc| match oppoint::extract(netlist, oc) {
        Ok(o) => Some(o),
        Err(e) => {
            eprintln!("[op] operating point unavailable ({e}); continuing with zero power");
            None
        }
    });
    let Some(o) = op else { return (cfg.device_power_uw.clone(), None) };
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
    (o.power_uw, Some(summary))
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
}

impl CellSpace {
    /// Draw every cell's variants and retarget `problem.placement` to cell ids.
    fn new(
        netlist: &pnr_core::Netlist,
        injected: &Macros,
        problem: &mut Problem,
        pdk: &Pdk,
        power: &[i32],
    ) -> Self {
        let cellgen::Cells { spaces, cell_of, devices_of } =
            cellgen::enumerate(netlist, injected, &problem.constraints, pdk);
        let p = &mut problem.placement;
        for b in p.hard.iter_mut().chain(p.budget.iter_mut()).chain(p.cost.iter_mut()) {
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
                .map(|m| m.iter().any(|d| injected.get(&netlist.devices[d.0 as usize].name).is_some()))
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
        };
        // Reserve each ring's halo in the requester's bbox so the placer keeps
        // neighbours out of it; the ring is drawn back inside the reservation.
        for r in &cells.guard_rings.guard_rings {
            let ext = round_up(cells::post_cell::ring_halo(r, pdk), pdk.grid.max(1));
            let Some(space) = cells.variants.get_mut(r.device.0 as usize) else { continue };
            for m in &mut space.alternatives {
                m.bbox.x -= ext;
                m.bbox.y -= ext;
                m.bbox.w += 2 * ext;
                m.bbox.h += 2 * ext;
            }
        }
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
        geometry::collect(&self.macros, &self.layout, &self.routes)
    }
}

/// Parse a SPICE netlist — the same front end [`run`] uses.
///
/// # Errors
/// The parser's message.
pub fn parse(spice: &str) -> Result<pnr_core::Netlist, String> {
    parse::spice(spice)
}

/// Full DRC/ERC/LVS/PEX signoff of a solution against its own schematic.
#[must_use]
pub fn signoff(sol: &Solution, pdk: &Pdk) -> Report {
    let shapes = sol.geometry();
    let placed = pnr_core::place_macros(&sol.macros, &sol.layout);
    let names: Vec<String> = sol.netlist.nets.iter().map(|n| n.name.clone()).collect();
    signoff_shapes(&shapes, &placed, &names, &sol.netlist, pdk)
}

/// Signoff over drawn `shapes`: labels every provable net (see
/// [`labeled_pins`]) and builds the LVS reference with exactly those ports.
pub(crate) fn signoff_shapes(
    shapes: &[pnr_core::Shape],
    placed: &[Macro],
    nets: &[String],
    schematic: &pnr_core::Netlist,
    pdk: &Pdk,
) -> Report {
    let pins = labeled_pins(placed, nets, pdk, shapes);
    let mut reference = cellgen::reference(schematic);
    reference.ports = pins.iter().map(|p| p.name.clone()).collect();
    verify::signoff(shapes, &pins, &reference, pdk).0
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
    let conn = &pdk.deck.connectivity;
    let conductor_of = |layer: u16| {
        conn.label_layer.iter().position(|l| l.0 == layer).map(|row| conn.label_names[row].0)
    };
    let mut out: Vec<verify::LabeledPin> = Vec::new();
    let mut labelled: Vec<u16> = Vec::new();
    // ponytail: O(pins × shapes), once per signoff; index shapes per layer if it shows.
    for p in placed.iter().flat_map(|m| &m.pins) {
        if labelled.contains(&p.net.0) {
            continue;
        }
        let Some(conductor) = conductor_of(p.layer.0) else { continue };
        let (x, y) = (p.at.x + p.at.w / 2, p.at.y + p.at.h / 2);
        let on_drawn = shapes.iter().any(|s| {
            s.layer.0 == conductor
                && (s.rect.x..=s.rect.x + s.rect.w).contains(&x)
                && (s.rect.y..=s.rect.y + s.rect.h).contains(&y)
        });
        let Some(name) = nets.get(p.net.0 as usize).filter(|_| on_drawn) else { continue };
        labelled.push(p.net.0);
        out.push(verify::LabeledPin { name: name.clone(), layer: p.layer.0, x, y });
    }
    out
}
