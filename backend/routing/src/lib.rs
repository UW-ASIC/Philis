//! pnr-routing — analog routing on top of the pure pnr-engine.
//!
//! `run_routing` takes the hypergraph, the [`Placement`] from pnr-placement,
//! and the shared [`ConstraintRecord`]; runs global (gcell PathFinder) then
//! detailed (track-grid PathFinder, capacity 1) routing; closes routing-stage
//! `ConstraintContract`s (crosstalk exclusion); and emits wire/via geometry
//! plus debug artifacts.

use std::fmt;
use std::path::PathBuf;

use pnr_constraints::contract::ContractValidation;
use pnr_constraints::{
    validate_constraint_record, ConstraintContract, ConstraintStatus, ConstraintStrength,
    Contractable, GROUND_NAMES, SUPPLY_NAMES,
};
use pnr_cells::netlist::BipartiteHypergraph;
use pnr_engine::routing::{
    self as route, build_track_grid, eol_violations, net_pair_clearance, prl_violations,
    GcellGrid, RGraph, RouteCtx, RouteHot, Term, TrackGrid,
};
use pnr_engine::{SplitMix64, Telemetry};
use pnr_placement::{ConstraintRecord, Placement};

pub use pnr_engine::routing::{
    wire_rect, extract_geometry_minarea, DetailedRouteCfg, GlobalRouteCfg, PinLanding, Via, Wire,
};

#[derive(Debug, Clone)]
pub struct RoutingConfig {
    pub seed: u64,
    pub global: GlobalRouteCfg,
    pub detailed: DetailedRouteCfg,
    pub debug_dir: Option<PathBuf>,
    /// Net priority overrides from routing feedback. Nets with higher values
    /// are routed earlier in subsequent iterations.
    pub net_priority_overrides: std::collections::HashMap<String, f64>,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            seed: 1,
            global: GlobalRouteCfg::default(),
            detailed: DetailedRouteCfg::default(),
            debug_dir: None,
            net_priority_overrides: std::collections::HashMap::new(),
        }
    }
}

pub struct RoutingReport {
    pub global: Telemetry,
    pub detailed: Telemetry,
    pub wirelength_nm: i64,
    pub via_count: usize,
    pub overuse: u32,
    pub unrouted: Vec<String>,
    pub validation: ContractValidation,
    pub contract_lines: String,
}

impl fmt::Display for RoutingReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "routing report")?;
        writeln!(
            f,
            "  global:   {} iters, residual overflow {}",
            self.global.iters, self.global.overflow
        )?;
        writeln!(
            f,
            "  detailed: {} iters, residual overuse {}",
            self.detailed.iters, self.overuse
        )?;
        writeln!(f, "  wirelength {} nm, {} vias", self.wirelength_nm, self.via_count)?;
        if self.unrouted.is_empty() {
            writeln!(f, "  all nets routed")?;
        } else {
            writeln!(f, "  UNROUTED: {}", self.unrouted.join(", "))?;
        }
        let v = &self.validation;
        writeln!(
            f,
            "  contracts: {} total | {} satisfied, {} violated, {} waived, {} unconsumed",
            v.total, v.satisfied, v.violated, v.waived, v.emitted
        )?;
        if !v.hard_violations.is_empty() {
            writeln!(f, "  HARD VIOLATIONS: {}", v.hard_violations.join(", "))?;
        }
        Ok(())
    }
}

pub struct RoutingResult {
    pub wires: Vec<Wire>,
    pub vias: Vec<Via>,
    pub landings: Vec<PinLanding>,
    pub net_names: Vec<String>,
    /// Per-net half-perimeter wirelength estimate (nm), from pin positions.
    pub net_hpwl: Vec<i64>,
    pub report: RoutingReport,
    /// Crosstalk violations: (net_a, net_b, shortfall_nm) for pairs that
    /// violated minimum spacing. Used to feed back placement pressure.
    pub crosstalk_violations: Vec<(String, String, f64)>,
}

/// Placement feedback from routing: per-net weight adjustments and per-cell
/// inflation factors. Feed into the next placement iteration.
#[derive(Debug, Clone)]
pub struct RoutingFeedback {
    /// Weight multiplier per net (index matches `RoutingResult::net_names`).
    /// 1.0 = routed at HPWL; higher = detoured or parasitic-critical.
    pub net_weights: Vec<f64>,
    /// Virtual x-size multiplier per cell (index matches placement cells).
    /// > 1.0 in regions congested with horizontal (met1) wires.
    pub cell_inflation_x: Vec<f64>,
    /// Virtual y-size multiplier per cell (index matches placement cells).
    /// > 1.0 in regions congested with vertical (met2) wires.
    pub cell_inflation_y: Vec<f64>,
    /// Largest weight across all nets (convergence signal).
    pub max_weight: f64,
    /// True when all nets routed and no track overuse.
    pub clean: bool,
    /// Suggested net routing order for next iteration: (net_name, detour_ratio)
    /// sorted by detour ratio descending. High-detour nets should route earlier.
    pub net_order_priority: Vec<(String, f64)>,
    /// Constraint-violation pushes: (device_a, device_b, additional_gap_nm)
    /// for pairs that violated crosstalk clearance during routing.
    pub constraint_adjustments: Vec<(String, String, f64)>,
}

/// Extract placement feedback from a completed routing.
///
/// - **net_weights**: actual_wl / hpwl ratio, with 2× boost for
///   parasitic-budgeted nets (they need shorter wires).
/// - **cell_inflation_x/y**: directional wire-density inflation — cells
///   near horizontal wires inflate in x, near vertical in y.
pub fn extract_feedback(
    result: &RoutingResult,
    placement: &Placement,
    parasitic_budgets: &[pnr_constraints::ParasiticBudget],
    weight_cap: f64,
    inflation_cap: f64,
) -> RoutingFeedback {
    extract_feedback_with_prior(result, placement, parasitic_budgets, weight_cap, inflation_cap, None)
}

/// Extract feedback with optional prior weights for EMA decay.
/// `prior` maps net name → previous weight; absent nets default to 1.0.
pub fn extract_feedback_with_prior(
    result: &RoutingResult,
    placement: &Placement,
    parasitic_budgets: &[pnr_constraints::ParasiticBudget],
    weight_cap: f64,
    inflation_cap: f64,
    prior: Option<&std::collections::HashMap<String, f64>>,
) -> RoutingFeedback {
    let n = result.net_names.len();
    let n_cells = placement.x.len();

    // -- per-net weights from wirelength ratio --
    let mut actual: Vec<i64> = vec![0; n];
    for w in &result.wires {
        actual[w.net as usize] +=
            i64::from((w.x1 - w.x0).abs() + (w.y1 - w.y0).abs());
    }
    let mut weights: Vec<f64> = (0..n)
        .map(|i| {
            let h = result.net_hpwl[i] as f64;
            if h > 0.0 { (actual[i] as f64 / h).clamp(1.0, weight_cap) } else { 1.0 }
        })
        .collect();

    // parasitic-budgeted nets get 2× priority — shorter wire = less R/C
    for budget in parasitic_budgets {
        if let Some(idx) = result.net_names.iter().position(|n| n == &budget.net_name) {
            weights[idx] = (weights[idx] * 2.0).min(weight_cap);
        }
    }

    // ponytail: EMA decay — lets resolved nets relax instead of locking high
    if let Some(prev) = prior {
        for (i, name) in result.net_names.iter().enumerate() {
            let old = prev.get(name).copied().unwrap_or(1.0);
            weights[i] = (0.7 * weights[i] + 0.3 * old).clamp(1.0, weight_cap);
        }
    }

    // -- per-cell directional inflation from wire density --
    // ponytail: split by layer — layer 0 (met1, horizontal) → x, layer 1 (met2, vertical) → y
    let mut wire_near_x = vec![0u32; n_cells];
    let mut wire_near_y = vec![0u32; n_cells];
    for w in &result.wires {
        let (x0, y0, x1, y1) = wire_rect(w);
        let (wcx, wcy) = ((x0 + x1) / 2, (y0 + y1) / 2);
        for ci in 0..n_cells {
            let (hw, hh) = (placement.sizes[ci].0, placement.sizes[ci].1);
            if (placement.x[ci] - wcx).abs() < hw
                && (placement.y[ci] - wcy).abs() < hh
            {
                if w.layer == 0 {
                    wire_near_x[ci] += 1;
                } else {
                    wire_near_y[ci] += 1;
                }
            }
        }
    }
    let inflate = |counts: &[u32]| -> Vec<f64> {
        let avg = counts.iter().sum::<u32>() as f64 / n_cells.max(1) as f64;
        counts
            .iter()
            .map(|&c| {
                if avg > 0.0 {
                    (1.0 + 0.1 * (c as f64 / avg - 1.0)).clamp(1.0, inflation_cap)
                } else {
                    1.0
                }
            })
            .collect()
    };
    let cell_inflation_x = inflate(&wire_near_x);
    let cell_inflation_y = inflate(&wire_near_y);

    // -- net ordering priority from detour ratio --
    let mut net_order_priority: Vec<(String, f64)> = (0..n)
        .map(|i| {
            let h = result.net_hpwl[i] as f64;
            let ratio = if h > 0.0 { actual[i] as f64 / h } else { 1.0 };
            (result.net_names[i].clone(), ratio)
        })
        .collect();
    net_order_priority.sort_by(|a, b| b.1.total_cmp(&a.1));

    let max_weight = weights.iter().copied().fold(1.0f64, f64::max);
    let clean = result.report.unrouted.is_empty() && result.report.overuse == 0;
    RoutingFeedback {
        net_weights: weights,
        cell_inflation_x,
        cell_inflation_y,
        max_weight,
        clean,
        net_order_priority,
        // ponytail: constraint_adjustments filled by caller after ledger inspection
        constraint_adjustments: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Crosstalk ledger (routing-stage contract closure)
// ---------------------------------------------------------------------------

struct XtalkCheck {
    contract: usize,
    net_a: u32,
    net_b: u32,
    min_nm: f32,
}

struct RouteLedger {
    contracts: Vec<ConstraintContract>,
    checks: Vec<XtalkCheck>,
    open: usize,
}

impl RouteLedger {
    fn build(g: &BipartiteHypergraph, rec: &ConstraintRecord, net_of: &[Option<u32>]) -> Self {
        let names: Vec<String> = g.cells.iter().map(|c| c.name.clone()).collect();
        let mut contracts = Vec::new();
        let mut checks = Vec::new();
        for x in &rec.crosstalk {
            let mut c = x.to_contract(&names);
            let a = g.net_id(&x.net_a).and_then(|n| net_of[n as usize]);
            let b = g.net_id(&x.net_b).and_then(|n| net_of[n as usize]);
            if let (Some(a), Some(b)) = (a, b) {
                c.consume("routing");
                checks.push(XtalkCheck {
                    contract: contracts.len(),
                    net_a: a,
                    net_b: b,
                    min_nm: (x.min_spacing_um * 1000.0) as f32,
                });
            }
            contracts.push(c);
        }
        for p in &rec.parasitic {
            let _ = p;
        }
        Self { contracts, checks, open: usize::MAX }
    }

    fn validation(&self) -> ContractValidation {
        validate_constraint_record(&self.contracts)
    }

    fn summary(&self) -> String {
        let mut s = String::new();
        for c in &self.contracts {
            s.push_str(&format!(
                "{:60} {:9} {:?}\n",
                c.constraint_id,
                format!("{:?}", c.strength),
                c.status
            ));
        }
        s
    }

    /// Extract violated crosstalk pairs as (net_a, net_b, shortfall_nm).
    fn violated_pairs(&self, net_names: &[String]) -> Vec<(String, String, f64)> {
        let mut out = Vec::new();
        for ch in &self.checks {
            let c = &self.contracts[ch.contract];
            if c.status == ConstraintStatus::Violated {
                let shortfall = c.violation_metric.unwrap_or(0.0) * 1000.0; // um → nm
                let na = net_names.get(ch.net_a as usize).cloned().unwrap_or_default();
                let nb = net_names.get(ch.net_b as usize).cloned().unwrap_or_default();
                out.push((na, nb, shortfall));
            }
        }
        out
    }
}

impl pnr_engine::Ledger<pnr_engine::routing::RouteDomain<TrackGrid>> for RouteLedger {
    fn reconcile(&mut self, hot: &RouteHot, cold: &RouteCtx<TrackGrid>) {
        let mut open = 0usize;
        for ch in &self.checks {
            if hot.trees[ch.net_a as usize].is_empty()
                || hot.trees[ch.net_b as usize].is_empty()
            {
                continue;
            }
            let d = net_pair_clearance(hot, &cold.graph, ch.net_a, ch.net_b);
            let ok = d >= ch.min_nm;
            let c = &mut self.contracts[ch.contract];
            let want = if ok { ConstraintStatus::Satisfied } else { ConstraintStatus::Violated };
            if c.status != want {
                if ok {
                    c.satisfy("routing", &format!("clearance {d:.0} nm"));
                } else {
                    c.violate("routing", f64::from(ch.min_nm - d) / 1000.0, "um");
                }
            }
            if !ok && c.strength == ConstraintStrength::Hard {
                open += 1;
            }
        }
        self.open = open;
    }

    fn open(&self) -> usize {
        self.open
    }
}

// ---------------------------------------------------------------------------
// run_routing
// ---------------------------------------------------------------------------

struct NetSetup {
    net_of: Vec<Option<u32>>,
    names: Vec<String>,
    weights: Vec<f32>,
    pins: Vec<Vec<(i32, i32)>>,
    order: Vec<u32>,
    obstacles: Vec<(i32, i32)>,
}

fn build_nets(
    g: &BipartiteHypergraph,
    p: &Placement,
    pin_pos: Option<&[Vec<Vec<(i32, i32)>>]>,
    rec: &ConstraintRecord,
    priority_overrides: &std::collections::HashMap<String, f64>,
) -> NetSetup {
    let mut s = NetSetup {
        net_of: vec![None; g.nets.len()],
        names: Vec::new(),
        weights: Vec::new(),
        pins: Vec::new(),
        order: Vec::new(),
        obstacles: Vec::new(),
    };
    for (ni, name) in g.nets.iter().enumerate() {
        let hpins = g.pins_on_net(ni as u32);
        if hpins.len() < 2 {
            continue;
        }
        let mut pts = Vec::with_capacity(hpins.len());
        for &(ci, pi) in hpins {
            let c = ci as usize;
            if let Some(pp) = pin_pos {
                if let Some(accs) = pp[c].get(pi as usize) {
                    pts.extend_from_slice(accs);
                    continue;
                }
            }
            let (w, _h) = p.sizes[c];
            let npins = g.cells[c].pins.len().max(1) as i32;
            let px = p.x[c] - w / 2 + (2 * i32::from(pi) + 1) * w / (2 * npins);
            pts.push((px, p.y[c]));
        }
        pts.sort_unstable();
        pts.dedup();
        if pts.len() < 2 {
            s.obstacles.extend_from_slice(&pts);
            continue;
        }
        let lower = name.to_ascii_lowercase();
        let mut w = if SUPPLY_NAMES.contains(&lower.as_str())
            || GROUND_NAMES.contains(&lower.as_str())
        {
            0.5
        } else {
            1.0
        };
        if let Some(nc) = rec.net_class.iter().find(|c| c.net_name == *name) {
            use pnr_constraints::NetClass::*;
            w = match nc.net_class {
                Sensitive => 3.0,
                Clock => 2.0,
                Supply | Ground | Substrate => 0.5,
                Signal => w,
            };
        }
        s.net_of[ni] = Some(s.names.len() as u32);
        s.names.push(name.clone());
        s.weights.push(w);
        s.pins.push(pts);
    }
    let straight: Vec<bool> = s
        .names
        .iter()
        .map(|n| rec.straight.iter().any(|st| st.net == *n))
        .collect();
    // ponytail: priority overrides from routing feedback — high-detour nets route first
    let prio: Vec<f64> = s
        .names
        .iter()
        .map(|n| priority_overrides.get(n).copied().unwrap_or(0.0))
        .collect();
    let mut order: Vec<u32> = (0..s.names.len() as u32).collect();
    order.sort_by(|&a, &b| {
        let (a, b) = (a as usize, b as usize);
        straight[b]
            .cmp(&straight[a])
            .then(prio[b].total_cmp(&prio[a]))
            .then(s.weights[b].total_cmp(&s.weights[a]))
            .then(s.pins[a].len().cmp(&s.pins[b].len()))
    });
    s.order = order;
    s
}

pub fn run_routing(
    g: &BipartiteHypergraph,
    p: &Placement,
    rec: &ConstraintRecord,
    cfg: &RoutingConfig,
) -> RoutingResult {
    run_routing_at(g, p, None, rec, cfg)
}

pub fn run_routing_at(
    g: &BipartiteHypergraph,
    p: &Placement,
    pin_pos: Option<&[Vec<Vec<(i32, i32)>>]>,
    rec: &ConstraintRecord,
    cfg: &RoutingConfig,
) -> RoutingResult {
    let mut rng = SplitMix64::new(cfg.seed);
    let nets = build_nets(g, p, pin_pos, rec, &cfg.net_priority_overrides);
    let n_nets = nets.names.len();

    // ---- Stage 3: global (gcell) ----
    let ggrid = GcellGrid::new(p.die, cfg.global.gcells_per_side, cfg.global.gcell_capacity);
    let gterms: Vec<Vec<u32>> = nets
        .pins
        .iter()
        .map(|pts| {
            let mut t: Vec<u32> =
                pts.iter().map(|&(x, y)| ggrid.at(x as f32, y as f32)).collect();
            t.sort_unstable();
            t.dedup();
            t
        })
        .collect();
    // Compute per-net max wirelength from parasitic budgets.
    // max_len_nm = budget.max_r * wire_width_nm / sheet_r_per_square
    // Default sheet resistance: 0.125 ohm/square for met1 in SKY130.
    let sheet_r: f64 = 0.125;
    let wire_w_nm = f64::from(cfg.detailed.wire_width);
    let net_max_len: Vec<Option<f32>> = nets
        .names
        .iter()
        .map(|name| {
            rec.parasitic
                .iter()
                .find(|b| b.net_name == *name)
                .map(|b| {
                    let max_len_nm = b.max_r * wire_w_nm / sheet_r;
                    // Convert from nm to Dijkstra cost units (grid hops):
                    // each hop = pitch nm
                    (max_len_nm / f64::from(cfg.detailed.pitch)) as f32
                })
        })
        .collect();
    let gcold = RouteCtx {
        graph: ggrid,
        terms: gterms,
        net_w: nets.weights.clone(),
        order: nets.order.clone(),
        corridors: Vec::new(),
        reserved: Vec::new(),
        max_len: vec![None; n_nets], // no length limit for global routing
    };
    let mut ghot = RouteHot::new(gcold.graph.nodes(), n_nets);
    let gt = route::run_global_route(&mut ghot, &gcold, &cfg.global, &mut rng);

    // Detailed-route corridors
    let gnx = gcold.graph.nx as i64;
    let gny = gcold.graph.ny as i64;
    let corridors: Vec<Vec<u32>> = (0..n_nets)
        .map(|net| {
            let mut c: Vec<u32> = ghot
                .tree_nodes(net)
                .iter()
                .flat_map(|&n| {
                    let (x, y) = (i64::from(n) % gnx, i64::from(n) / gnx);
                    let mut v = Vec::with_capacity(9);
                    for dy in -1..=1i64 {
                        for dx in -1..=1i64 {
                            let (tx, ty) = (x + dx, y + dy);
                            if tx >= 0 && ty >= 0 && tx < gnx && ty < gny {
                                v.push((ty * gnx + tx) as u32);
                            }
                        }
                    }
                    v
                })
                .collect();
            c.sort_unstable();
            c.dedup();
            c
        })
        .collect();

    // ---- Stage 4: detailed (tracks) ----
    let footprints: Vec<(i32, i32, i32, i32)> = (0..g.cells.len())
        .map(|i| {
            let (w, h) = p.sizes[i];
            (p.x[i] - w / 2, p.y[i] - h / 2, p.x[i] + w / 2, p.y[i] + h / 2)
        })
        .collect();
    let terms: Vec<Term> = nets
        .pins
        .iter()
        .enumerate()
        .flat_map(|(ni, pts)| {
            pts.iter().map(move |&(x, y)| Term { net: ni as u32, x, y })
        })
        .collect();
    let (mut tgrid, tterms, missing_pins, landings, reserved) = build_track_grid(
        p.die,
        &footprints,
        &terms,
        &nets.obstacles,
        n_nets,
        &cfg.detailed,
        pin_pos.is_some(),
    );
    tgrid.set_regions(&gcold.graph);
    let dcold = RouteCtx {
        graph: tgrid,
        terms: tterms,
        net_w: nets.weights.clone(),
        order: nets.order.clone(),
        corridors,
        reserved,
        max_len: net_max_len,
    };
    let mut dhot = RouteHot::new(dcold.graph.nodes(), n_nets);
    let mut ledger = RouteLedger::build(g, rec, &nets.net_of);
    let dt =
        route::run_detailed_route(&mut dhot, &dcold, &mut ledger, &cfg.detailed, &mut rng);

    // ---- Symmetric route mirroring for matched diff pairs ----
    mirror_symmetric_routes(g, p, rec, &nets, &dcold, &mut dhot);

    // ---- Post-route EOL repair (Item #5) ----
    if cfg.detailed.eol_spacing > 0 {
        let probe_wires = extract_geometry_minarea(
            &dhot,
            &dcold.graph,
            cfg.detailed.wire_width,
            cfg.detailed.min_area,
        );
        let (blocked, affected) =
            eol_violations(&probe_wires.0, &dcold.graph, cfg.detailed.eol_spacing);
        if !affected.is_empty() {
            // Rip up affected nets
            for &net in &affected {
                let ni = net as usize;
                for &n in &dhot.tree_nodes(ni) {
                    dhot.usage[n as usize] = dhot.usage[n as usize].saturating_sub(1);
                }
                dhot.trees[ni].clear();
            }
            // Block EOL-violated nodes via usage bump
            for &node in &blocked {
                dhot.usage[node as usize] = dhot.usage[node as usize].saturating_add(1);
            }
            // Re-route just the affected nets (one-shot repair)
            let mut dij = route::Dij::new(dcold.graph.nodes());
            for &net in &affected {
                let ni = net as usize;
                let old = dhot.tree_nodes(ni);
                let corridor: &[u32] = dcold
                    .corridors
                    .get(ni)
                    .map_or(&[], std::vec::Vec::as_slice);
                let ml = dcold.max_len.get(ni).copied().flatten();
                if let Some((branches, _len)) = route::route_net(
                    &dcold.graph,
                    &dhot.usage,
                    &dhot.hist,
                    &old,
                    &dcold.terms[ni],
                    corridor,
                    net,
                    &dcold.reserved,
                    cfg.detailed.p_fac,
                    ml,
                    &mut dij,
                ) {
                    dhot.trees[ni] = branches;
                    for &n in &dhot.tree_nodes(ni) {
                        dhot.usage[n as usize] += 1;
                    }
                }
            }
            // Remove sentinel blockages
            for &node in &blocked {
                dhot.usage[node as usize] = dhot.usage[node as usize].saturating_sub(1);
            }
        }
    }

    // ---- Post-route PRL / wide-metal repair (Item #7) ----
    if cfg.detailed.wide_net_extra_spacing > 0 {
        let power_nets: Vec<u32> = nets
            .names
            .iter()
            .enumerate()
            .filter(|(_, name)| {
                let lower = name.to_ascii_lowercase();
                SUPPLY_NAMES.contains(&lower.as_str())
                    || GROUND_NAMES.contains(&lower.as_str())
            })
            .map(|(i, _)| i as u32)
            .collect();
        if !power_nets.is_empty() {
            let probe_wires = extract_geometry_minarea(
                &dhot,
                &dcold.graph,
                cfg.detailed.wire_width,
                cfg.detailed.min_area,
            );
            let (blocked, affected) = prl_violations(
                &probe_wires.0,
                &dcold.graph,
                &power_nets,
                cfg.detailed.wide_net_extra_spacing,
            );
            if !affected.is_empty() {
                for &net in &affected {
                    let ni = net as usize;
                    for &n in &dhot.tree_nodes(ni) {
                        dhot.usage[n as usize] = dhot.usage[n as usize].saturating_sub(1);
                    }
                    dhot.trees[ni].clear();
                }
                for &node in &blocked {
                    dhot.usage[node as usize] = dhot.usage[node as usize].saturating_add(1);
                }
                let mut dij = route::Dij::new(dcold.graph.nodes());
                for &net in &affected {
                    let ni = net as usize;
                    let old = dhot.tree_nodes(ni);
                    let corridor: &[u32] = dcold
                        .corridors
                        .get(ni)
                        .map_or(&[], std::vec::Vec::as_slice);
                    let ml = dcold.max_len.get(ni).copied().flatten();
                    if let Some((branches, _len)) = route::route_net(
                        &dcold.graph,
                        &dhot.usage,
                        &dhot.hist,
                        &old,
                        &dcold.terms[ni],
                        corridor,
                        net,
                        &dcold.reserved,
                        cfg.detailed.p_fac,
                        ml,
                        &mut dij,
                    ) {
                        dhot.trees[ni] = branches;
                        for &n in &dhot.tree_nodes(ni) {
                            dhot.usage[n as usize] += 1;
                        }
                    }
                }
                for &node in &blocked {
                    dhot.usage[node as usize] = dhot.usage[node as usize].saturating_sub(1);
                }
            }
        }
    }

    // ---- results ----
    let cap = dcold.graph.cap();
    let overuse: u32 =
        dhot.usage.iter().map(|&u| u32::from(u.saturating_sub(cap))).sum();
    let unrouted: Vec<String> = (0..n_nets)
        .filter(|&i| dhot.trees[i].is_empty() || missing_pins[i] > 0)
        .map(|i| nets.names[i].clone())
        .collect();
    let (wires, vias) = extract_geometry_minarea(&dhot, &dcold.graph, cfg.detailed.wire_width, cfg.detailed.min_area);
    let wirelength_nm: i64 = wires
        .iter()
        .map(|w| i64::from((w.x1 - w.x0).abs() + (w.y1 - w.y0).abs()))
        .sum();

    let net_hpwl: Vec<i64> = nets.pins.iter().map(|pts| {
        if pts.len() < 2 { return 0; }
        let (mut xn, mut xx, mut yn, mut yx) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
        for &(x, y) in pts {
            xn = xn.min(x); xx = xx.max(x);
            yn = yn.min(y); yx = yx.max(y);
        }
        i64::from(xx - xn) + i64::from(yx - yn)
    }).collect();

    let report = RoutingReport {
        global: gt,
        detailed: dt,
        wirelength_nm,
        via_count: vias.len(),
        overuse,
        unrouted,
        validation: ledger.validation(),
        contract_lines: ledger.summary(),
    };

    let crosstalk_violations = ledger.violated_pairs(&nets.names);

    if let Some(dir) = &cfg.debug_dir {
        dump_debug(dir, &nets, &dhot, &dcold, &report, &wires);
    }

    RoutingResult { wires, vias, landings, net_names: nets.names, net_hpwl, report, crosstalk_violations }
}

/// For each symmetry pair, find matched nets (same pin name on mirrored
/// devices). If one net is routed but the other is not, mirror the route
/// tree across the symmetry axis to produce the paired net's route.
fn mirror_symmetric_routes(
    g: &BipartiteHypergraph,
    p: &Placement,
    rec: &ConstraintRecord,
    nets: &NetSetup,
    cold: &RouteCtx<TrackGrid>,
    hot: &mut RouteHot,
) {
    let grid = &cold.graph;
    for (gi, sg) in rec.symmetry.iter().enumerate() {
        let axis_x = if gi < p.axes.len() { p.axes[gi] } else { continue };

        for pair in &sg.pairs {
            let da = pair.device_a.0 as usize;
            let db = pair.device_b.0 as usize;
            if da >= g.cells.len() || db >= g.cells.len() {
                continue;
            }
            // Find matched net pairs: same pin index on mirrored devices
            let pins_a = &g.cells[da].pins;
            let pins_b = &g.cells[db].pins;
            let pin_count = pins_a.len().min(pins_b.len());
            for pi in 0..pin_count {
                let net_a_hyper = pins_a[pi].1;
                let net_b_hyper = pins_b[pi].1;
                if net_a_hyper == net_b_hyper {
                    continue; // same net, nothing to mirror
                }
                let ra = nets.net_of.get(net_a_hyper as usize).and_then(|&v| v);
                let rb = nets.net_of.get(net_b_hyper as usize).and_then(|&v| v);
                let (ra, rb) = match (ra, rb) {
                    (Some(a), Some(b)) => (a as usize, b as usize),
                    _ => continue,
                };
                // If one is routed and the other is not, mirror the routed one
                let (src, dst) = if !hot.trees[ra].is_empty() && hot.trees[rb].is_empty() {
                    (ra, rb)
                } else if !hot.trees[rb].is_empty() && hot.trees[ra].is_empty() {
                    (rb, ra)
                } else {
                    continue;
                };
                // Mirror each branch: x -> 2*axis - x
                let mut mirrored_branches = Vec::new();
                let mut conflict = false;
                for branch in &hot.trees[src] {
                    let mut mbranch = Vec::with_capacity(branch.len());
                    for &node in branch {
                        let (x, y, layer) = grid.pos(node);
                        let mx = 2 * axis_x - x;
                        let mn = grid.nearest(mx, y, layer);
                        // Check if the mirrored node is occupied by another net
                        if hot.usage[mn as usize] >= grid.cap()
                            && !cold.terms[dst].contains(&mn)
                        {
                            conflict = true;
                            break;
                        }
                        mbranch.push(mn);
                    }
                    if conflict {
                        break;
                    }
                    mirrored_branches.push(mbranch);
                }
                if conflict {
                    continue;
                }
                // Install the mirrored route
                hot.trees[dst] = mirrored_branches;
                for &n in &hot.tree_nodes(dst) {
                    hot.usage[n as usize] += 1;
                }
            }
        }
    }
}

fn dump_debug(
    dir: &std::path::Path,
    nets: &NetSetup,
    hot: &RouteHot,
    cold: &RouteCtx<TrackGrid>,
    r: &RoutingReport,
    wires: &[Wire],
) {
    let _ = std::fs::create_dir_all(dir);
    let w = |name: &str, content: String| {
        let _ = std::fs::write(dir.join(name), content);
    };
    w("global_route_trace.csv", r.global.trace_csv());
    w("detailed_route_trace.csv", r.detailed.trace_csv());
    w("route_report.txt", r.to_string());
    w("route_contracts.txt", r.contract_lines.clone());
    let mut txt = String::from("net layer x0 y0 x1 y1\n");
    for wire in wires {
        txt.push_str(&format!(
            "{} met{} {} {} {} {}\n",
            nets.names[wire.net as usize],
            wire.layer + 1,
            wire.x0,
            wire.y0,
            wire.x1,
            wire.y1
        ));
    }
    w("routes.txt", txt);
    let _ = (hot, cold);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_constraints::{
        CrosstalkExclusion, DeviceId, MatchingPair, MatchingTier, MatchingType, SymmetryGroup,
    };
    use pnr_core::frontend::{parse_spice, Pdk};
    use pnr_placement::{estimate_sizes, run_placement, PlacementConfig};
    use std::collections::HashSet;

    const OTA: &str = "\
.subckt ota vinp vinm vout1 vout2 VDD VSS
XM1 vout1 vinp vtail VSS nfet_01v8 W=10u L=1u nf=2
XM2 vout2 vinm vtail VSS nfet_01v8 W=10u L=1u nf=2
XM3 vout1 vbias VDD VDD pfet_01v8 W=20u L=1u
XM4 vout2 vbias VDD VDD pfet_01v8 W=20u L=1u
XM5 vtail vbn VSS VSS nfet_01v8 W=40u L=2u m=4
.ends ota
";

    fn pdk() -> Pdk {
        Pdk::from_json(
            r#"{
            "drc": {"off_grid": {"grid": 5}},
            "devices": {
                "nfet_01v8": {"type": "nmos", "cell": "mosfet", "min_w": 420, "min_l": 150},
                "pfet_01v8": {"type": "pmos", "cell": "mosfet", "min_w": 420, "min_l": 150}
            }}"#,
        )
        .unwrap()
    }

    fn flow(rec: &ConstraintRecord) -> (BipartiteHypergraph, Placement, RoutingResult) {
        let g = parse_spice(OTA, &pdk(), &HashSet::new()).unwrap();
        let sizes = estimate_sizes(&g);
        let p = run_placement(&g, &sizes, rec, &PlacementConfig::default(), &[]).placement;
        let r = run_routing(&g, &p, rec, &RoutingConfig::default());
        (g, p, r)
    }

    fn ota_record(g: &BipartiteHypergraph) -> ConstraintRecord {
        let id = |n: &str| DeviceId(g.cell_id(n).unwrap());
        ConstraintRecord {
            symmetry: vec![SymmetryGroup {
                group_id: "dp".into(),
                axis: None,
                pairs: vec![MatchingPair {
                    device_a: id("XM1"),
                    device_b: id("XM2"),
                    matching_type: MatchingType::DiffPair,
                    tier: MatchingTier::Exceptional,
                    max_dvth_mv: 1.0,
                    max_did_pct: 1.0,
                    w_ratio: None,
                }],
                self_symmetric: vec![id("XM5")],
            }],
            ..Default::default()
        }
    }

    #[test]
    fn ota_routes_clean() {
        let g = parse_spice(OTA, &pdk(), &HashSet::new()).unwrap();
        let rec = ota_record(&g);
        let (_g, _p, r) = flow(&rec);
        assert!(r.report.unrouted.is_empty(), "unrouted: {:?}", r.report.unrouted);
        assert_eq!(r.report.overuse, 0, "track overuse (shorts) remain");
        assert!(!r.wires.is_empty());
        assert!(r.report.wirelength_nm > 0);
        for w in &r.wires {
            assert!(w.x0 >= 0 && w.x1 <= _p.die.0 && w.y0 >= 0 && w.y1 <= _p.die.1);
        }
    }

    #[test]
    fn crosstalk_contract_reconciled() {
        let g = parse_spice(OTA, &pdk(), &HashSet::new()).unwrap();
        let mut rec = ota_record(&g);
        rec.crosstalk.push(CrosstalkExclusion {
            net_a: "vtail".into(),
            net_b: "vout1".into(),
            min_spacing_um: 0.5,
        });
        let (_g, _p, r) = flow(&rec);
        let v = &r.report.validation;
        assert_eq!(v.total, 1);
        assert_eq!(
            v.emitted, 0,
            "crosstalk contract must be consumed by routing:\n{}",
            r.report.contract_lines
        );
    }

    #[test]
    fn deterministic_given_seed() {
        let g = parse_spice(OTA, &pdk(), &HashSet::new()).unwrap();
        let rec = ota_record(&g);
        let run = || {
            let sizes = estimate_sizes(&g);
            let p = run_placement(&g, &sizes, &rec, &PlacementConfig::default(), &[]).placement;
            let r = run_routing(&g, &p, &rec, &RoutingConfig::default());
            (r.report.wirelength_nm, r.wires.len(), r.vias.len())
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn debug_dump_writes_artifacts() {
        let g = parse_spice(OTA, &pdk(), &HashSet::new()).unwrap();
        let rec = ota_record(&g);
        let sizes = estimate_sizes(&g);
        let p = run_placement(&g, &sizes, &rec, &PlacementConfig::default(), &[]).placement;
        let dir = std::env::temp_dir().join("pnr_routing_debug_test");
        let _ = std::fs::remove_dir_all(&dir);
        let cfg = RoutingConfig { debug_dir: Some(dir.clone()), ..Default::default() };
        run_routing(&g, &p, &rec, &cfg);
        for f in [
            "global_route_trace.csv",
            "detailed_route_trace.csv",
            "routes.txt",
            "route_report.txt",
        ] {
            assert!(dir.join(f).exists(), "missing {f}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
