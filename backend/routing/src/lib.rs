//! pnr-routing — analog routing on top of the pure pnr-engine.
//!
//! `run_routing` takes the hypergraph, the [`Placement`] from pnr-placement,
//! and the shared [`ConstraintRecord`]; runs global (gcell PathFinder) then
//! detailed (track-grid PathFinder, capacity 1) routing; closes routing-stage
//! `ConstraintContract`s (crosstalk exclusion); and emits wire/via geometry
//! plus debug artifacts.

use std::fmt;
use std::path::PathBuf;

use pnr_cells::netlist::BipartiteHypergraph;
use pnr_constraints::contract::ContractValidation;
use pnr_constraints::{
    validate_constraint_record, ConstraintContract, ConstraintRecord, ConstraintStatus,
    ConstraintStrength, Contractable, GROUND_NAMES, SUPPLY_NAMES,
};
use pnr_engine::routing::{
    self as route, build_track_grid, eol_violations, net_pair_clearance, prl_violations, GcellGrid,
    RGraph, RouteCtx, RouteHot, Term, TrackGrid,
};
use pnr_engine::{SplitMix64, Telemetry};
use pnr_placement::Placement;

pub use pnr_engine::routing::{
    extract_geometry_minarea, wire_rect, DetailedRouteCfg, GlobalRouteCfg, PinAccessFailure,
    PinLanding, Via, Wire,
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
    /// Per-layer PEX parameters for post-route R/C contract closure
    /// (parasitic budgets, differential-pair matching). `None` leaves those
    /// contracts Consumed — never judged on invented numbers.
    pub wire_params: Option<pnr_constraints::WireParasiticParams>,
    /// Extra keep-away points for landing claims (nm): met1 features the
    /// router cannot see, e.g. in-cell dummy-gate contact pads.
    pub extra_obstacles: Vec<(i32, i32)>,
    /// Negotiated-congestion memory carried across feedback iterations.
    pub global_history: Vec<f32>,
    pub detailed_history: Vec<f32>,
    pub history_decay: f32,
    /// Per-layer drawn wire width (nm), indexed by routing conductor.
    /// Empty → uniform `detailed.wire_width` (legacy behavior).
    pub layer_widths: Vec<i32>,
    /// Per-layer minimum same-direction track center-to-center distance (nm):
    /// worst same-layer feature (wire or via landing pad) + spacing. The
    /// uniform-pitch grid enforces it by masking tracks to a stride of
    /// ceil(step/pitch). Empty → legacy parity mask on layers >= met3.
    pub layer_track_steps: Vec<i32>,
    /// Per-layer min-area (nm²) and spacing (nm) for upper-metal island
    /// extension. Empty → built-in sky130 met3 constants.
    pub layer_min_areas: Vec<i64>,
    pub layer_spacings: Vec<i32>,
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            seed: 1,
            global: GlobalRouteCfg::default(),
            detailed: DetailedRouteCfg::default(),
            debug_dir: None,
            net_priority_overrides: std::collections::HashMap::new(),
            wire_params: None,
            extra_obstacles: Vec::new(),
            global_history: Vec::new(),
            detailed_history: Vec::new(),
            history_decay: 0.65,
            layer_widths: Vec::new(),
            layer_track_steps: Vec::new(),
            layer_min_areas: Vec::new(),
            layer_spacings: Vec::new(),
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
    pub no_path: Vec<String>,
    pub missing_pin_count: usize,
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
        writeln!(
            f,
            "  wirelength {} nm, {} vias",
            self.wirelength_nm, self.via_count
        )?;
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
    pub landing_failures: Vec<PinAccessFailure>,
    pub hotspots: Vec<RoutingHotspot>,
    pub global_history: Vec<f32>,
    pub detailed_history: Vec<f32>,
    pub track_pitch: i32,
}

impl RoutingResult {
    /// Write artifacts for this owned routing result. Safe to call after the
    /// feedback engine has selected a non-final iteration as its best result.
    pub fn write_debug(&self, dir: &std::path::Path) -> std::io::Result<()> {
        dump_debug(dir, &self.net_names, &self.report, &self.wires)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RoutingHotspot {
    pub x: i32,
    pub y: i32,
    pub layer: u32,
    pub pressure: f64,
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
    /// Per-net estimated resistance (ohm) from routing wire geometry.
    pub net_r_ohm: Vec<f64>,
    /// Per-net estimated capacitance (fF) from routing wire geometry.
    pub net_c_ff: Vec<f64>,
    /// Matched-pair parasitic deltas (populated when symmetry groups provided).
    pub matched_deltas: Vec<MatchedDelta>,
    /// True only for capacity/no-path failure. Pin-access failures are handled
    /// locally and must not trigger destructive global utilization halving.
    pub spread_required: bool,
}

/// Parasitic delta between matched nets of a symmetry pair.
#[derive(Debug, Clone)]
pub struct MatchedDelta {
    pub device_a: String,
    pub device_b: String,
    pub net_a: String,
    pub net_b: String,
    pub r_a: f64,
    pub r_b: f64,
    pub c_a: f64,
    pub c_b: f64,
    pub r_delta_pct: f64,
    pub c_delta_pct: f64,
}

/// Estimate per-net resistance (ohm) and capacitance (fF) from routing wires.
/// Same analytical model as `verify/src/pex/mod.rs` but on routing geometry.
fn estimate_net_rc(
    wires: &[Wire],
    vias: &[Via],
    n_nets: usize,
    p: &pnr_constraints::WireParasiticParams,
) -> (Vec<f64>, Vec<f64>) {
    let mut r = vec![0.0; n_nets];
    let mut c = vec![0.0; n_nets];
    for w in wires {
        let ni = w.net as usize;
        if ni >= n_nets {
            continue;
        }
        let li = w.layer as usize;
        let len_nm = ((w.x1 - w.x0).abs() + (w.y1 - w.y0).abs()) as f64;
        let width_nm = w.width.max(1) as f64;
        if let Some(&sr) = p.sheet_r.get(li) {
            r[ni] += sr * len_nm / width_nm;
        }
        let len_um = len_nm / 1000.0;
        let width_um = width_nm / 1000.0;
        if let Some(&ac) = p.area_cap.get(li) {
            c[ni] += ac * len_um * width_um;
        }
        if let Some(&fc) = p.fringe_cap.get(li) {
            c[ni] += fc * 2.0 * (len_um + width_um);
        }
    }
    for v in vias {
        let ni = v.net as usize;
        if ni < n_nets {
            r[ni] += p.via_r;
        }
    }
    // aF → fF
    for ci in c.iter_mut() {
        *ci /= 1000.0;
    }
    (r, c)
}

/// Compute matched-pair parasitic deltas from per-net R/C.
///
/// `device_names` maps DeviceId index → name (from hypergraph).
/// `net_of_device` maps (device_idx, pin_name) → net index in `net_names`.
fn compute_matched_deltas(
    net_r: &[f64],
    net_c: &[f64],
    net_names: &[String],
    symmetry: &[pnr_constraints::SymmetryGroup],
    device_names: &[String],
    net_of_device: &dyn Fn(u32, &str) -> Option<usize>,
) -> Vec<MatchedDelta> {
    let mut out = Vec::new();
    let pins = ["G", "D", "S"];
    for sg in symmetry {
        for mp in &sg.pairs {
            if mp.tier < pnr_constraints::MatchingTier::Moderate {
                continue;
            }
            let (a, b) = (mp.device_a.0, mp.device_b.0);
            for pin in &pins {
                let (ia, ib) = match (net_of_device(a, pin), net_of_device(b, pin)) {
                    (Some(a), Some(b)) if a != b => (a, b),
                    _ => continue,
                };
                let (ra, rb) = (net_r[ia], net_r[ib]);
                let (ca, cb) = (net_c[ia], net_c[ib]);
                let avg_r = (ra + rb) / 2.0;
                let avg_c = (ca + cb) / 2.0;
                let r_delta = if avg_r > 0.0 {
                    ((ra - rb).abs() / avg_r) * 100.0
                } else {
                    0.0
                };
                let c_delta = if avg_c > 0.0 {
                    ((ca - cb).abs() / avg_c) * 100.0
                } else {
                    0.0
                };
                if r_delta < 0.1 && c_delta < 0.1 {
                    continue;
                }
                let na = net_names.get(ia).cloned().unwrap_or_default();
                let nb = net_names.get(ib).cloned().unwrap_or_default();
                let da = device_names.get(a as usize).cloned().unwrap_or_default();
                let db = device_names.get(b as usize).cloned().unwrap_or_default();
                out.push(MatchedDelta {
                    device_a: da,
                    device_b: db,
                    net_a: na,
                    net_b: nb,
                    r_a: ra,
                    r_b: rb,
                    c_a: ca,
                    c_b: cb,
                    r_delta_pct: r_delta,
                    c_delta_pct: c_delta,
                });
            }
        }
    }
    out
}

/// Extract placement feedback from a completed routing.
pub fn extract_feedback(
    result: &RoutingResult,
    placement: &Placement,
    parasitic_budgets: &[pnr_constraints::ParasiticBudget],
    weight_cap: f64,
    inflation_cap: f64,
) -> RoutingFeedback {
    extract_feedback_with_prior(
        result,
        placement,
        parasitic_budgets,
        weight_cap,
        inflation_cap,
        None,
        None,
        &[],
        &[],
        &|_, _| None,
    )
}

/// Extract feedback with optional prior weights, wire parasitic params, and
/// symmetry groups for matched-pair delta computation.
pub fn extract_feedback_with_prior(
    result: &RoutingResult,
    placement: &Placement,
    parasitic_budgets: &[pnr_constraints::ParasiticBudget],
    weight_cap: f64,
    inflation_cap: f64,
    prior: Option<&std::collections::HashMap<String, f64>>,
    wire_params: Option<&pnr_constraints::WireParasiticParams>,
    symmetry: &[pnr_constraints::SymmetryGroup],
    device_names: &[String],
    net_of_device: &dyn Fn(u32, &str) -> Option<usize>,
) -> RoutingFeedback {
    let n = result.net_names.len();
    let n_cells = placement.x.len();

    // -- per-net weights from wirelength ratio --
    let mut actual: Vec<i64> = vec![0; n];
    for w in &result.wires {
        actual[w.net as usize] += i64::from((w.x1 - w.x0).abs() + (w.y1 - w.y0).abs());
    }
    let mut weights: Vec<f64> = (0..n)
        .map(|i| {
            let h = result.net_hpwl[i] as f64;
            if h > 0.0 {
                (actual[i] as f64 / h).clamp(1.0, weight_cap)
            } else {
                1.0
            }
        })
        .collect();

    // -- per-net R/C estimate from routing wires --
    let (net_r, net_c) = wire_params
        .map(|wp| estimate_net_rc(&result.wires, &result.vias, n, wp))
        .unwrap_or_else(|| (vec![0.0; n], vec![0.0; n]));

    // ponytail: proportional parasitic boost replaces blind 2× — weight scaled
    // by how far over budget the net is, so near-budget nets get mild pressure
    // and heavily-over-budget nets get strong pressure.
    for budget in parasitic_budgets {
        if let Some(idx) = result.net_names.iter().position(|n| n == &budget.net_name) {
            let r_ratio = if budget.max_r > 0.0 && net_r[idx] > 0.0 {
                (net_r[idx] / budget.max_r).max(1.0)
            } else {
                1.0
            };
            let c_ratio = if budget.max_c > 0.0 && net_c[idx] > 0.0 {
                (net_c[idx] / budget.max_c).max(1.0)
            } else {
                1.0
            };
            let boost = r_ratio.max(c_ratio);
            // ponytail: fall back to 2× when no wire params available
            let boost = if wire_params.is_some() { boost } else { 2.0 };
            weights[idx] = (weights[idx] * boost).min(weight_cap);
        }
    }

    // ponytail: EMA decay — lets resolved nets relax instead of locking high
    if let Some(prev) = prior {
        for (i, name) in result.net_names.iter().enumerate() {
            let old = prev.get(name).copied().unwrap_or(1.0);
            weights[i] = (0.7 * weights[i] + 0.3 * old).clamp(1.0, weight_cap);
        }
    }

    // -- per-cell directional pressure from actual PathFinder history and
    // pin-access displacement, not from the midpoint of already-routed wires. --
    let mut pressure_x = vec![0.0f64; n_cells];
    let mut pressure_y = vec![0.0f64; n_cells];
    let pitch = result.track_pitch.max(1);
    let nearest_cell = |x: i32, y: i32| -> Option<usize> {
        (0..n_cells).min_by_key(|&ci| {
            let (w, h) = placement.sizes[ci];
            let dx = (x - placement.x[ci]).abs().saturating_sub(w / 2);
            let dy = (y - placement.y[ci]).abs().saturating_sub(h / 2);
            i64::from(dx.max(0)).pow(2) + i64::from(dy.max(0)).pow(2)
        })
    };
    for hs in &result.hotspots {
        for ci in 0..n_cells {
            let (w, h) = placement.sizes[ci];
            let dx = (hs.x - placement.x[ci]).abs().saturating_sub(w / 2).max(0);
            let dy = (hs.y - placement.y[ci]).abs().saturating_sub(h / 2).max(0);
            let dist = dx + dy;
            if dist <= 3 * pitch {
                let local = hs.pressure / (1.0 + f64::from(dist) / f64::from(pitch));
                if hs.layer % 2 == 0 {
                    pressure_x[ci] += local;
                } else {
                    pressure_y[ci] += local;
                }
            }
        }
    }
    for l in &result.landings {
        if let Some(ci) = nearest_cell(l.pin.0, l.pin.1) {
            pressure_x[ci] += f64::from((l.node.0 - l.pin.0).abs()) / f64::from(pitch);
            pressure_y[ci] += f64::from((l.node.1 - l.pin.1).abs()) / f64::from(pitch);
        }
    }
    for f in &result.landing_failures {
        if let Some(ci) = nearest_cell(f.pin.0, f.pin.1) {
            pressure_x[ci] += 4.0;
            pressure_y[ci] += 4.0;
        }
    }
    let inflate = |pressure: &[f64]| -> Vec<f64> {
        let peak = pressure.iter().copied().fold(0.0f64, f64::max);
        let mean = pressure.iter().sum::<f64>() / pressure.len().max(1) as f64;
        let threshold = (1.5 * mean).max(1.0);
        pressure
            .iter()
            .map(|&p| {
                if peak > threshold && p > threshold {
                    (1.0 + 0.30 * (p - threshold) / (peak - threshold)).clamp(1.0, inflation_cap)
                } else {
                    1.0
                }
            })
            .collect()
    };
    let cell_inflation_x = inflate(&pressure_x);
    let cell_inflation_y = inflate(&pressure_y);

    // -- net ordering priority from detour ratio --
    let mut net_order_priority: Vec<(String, f64)> = (0..n)
        .map(|i| {
            let h = result.net_hpwl[i] as f64;
            let ratio = if h > 0.0 { actual[i] as f64 / h } else { 1.0 };
            (result.net_names[i].clone(), ratio)
        })
        .collect();
    net_order_priority.sort_by(|a, b| b.1.total_cmp(&a.1));

    // -- matched-pair parasitic deltas --
    let matched_deltas = compute_matched_deltas(
        &net_r,
        &net_c,
        &result.net_names,
        symmetry,
        device_names,
        net_of_device,
    );

    let max_weight = weights.iter().copied().fold(1.0f64, f64::max);
    let clean = result.report.unrouted.is_empty() && result.report.overuse == 0;
    RoutingFeedback {
        net_weights: weights,
        cell_inflation_x,
        cell_inflation_y,
        max_weight,
        clean,
        net_order_priority,
        constraint_adjustments: Vec::new(),
        net_r_ohm: net_r,
        net_c_ff: net_c,
        matched_deltas,
        spread_required: result.report.overuse > 0 || !result.report.no_path.is_empty(),
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

struct ParaCheck {
    contract: usize,
    net: u32,
    max_r: f64,
    max_c: f64,
}

struct DiffCheck {
    contract: usize,
    net_a: u32,
    net_b: u32,
    max_len_pct: f64,
    max_r_pct: f64,
    max_c_pct: f64,
    same_layer: bool,
}

struct StraightCheck {
    contract: usize,
    net: u32,
    vertical: bool,
}

struct RouteLedger {
    contracts: Vec<ConstraintContract>,
    checks: Vec<XtalkCheck>,
    para: Vec<ParaCheck>,
    diff: Vec<DiffCheck>,
    straight: Vec<StraightCheck>,
    open: usize,
}

impl RouteLedger {
    fn build(g: &BipartiteHypergraph, rec: &ConstraintRecord, net_of: &[Option<u32>]) -> Self {
        let names: Vec<String> = g.cells.iter().map(|c| c.name.clone()).collect();
        let mut contracts = Vec::new();
        let mut checks = Vec::new();
        let mut para = Vec::new();
        let mut diff = Vec::new();
        let mut straight = Vec::new();
        let net_idx = |name: &str| g.net_id(name).and_then(|n| net_of[n as usize]);
        for x in &rec.crosstalk {
            let mut c = x.to_contract(&names);
            if let (Some(a), Some(b)) = (net_idx(&x.net_a), net_idx(&x.net_b)) {
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
            let mut c = p.to_contract(&names);
            if let Some(ni) = net_idx(&p.net_name) {
                c.consume("routing");
                para.push(ParaCheck {
                    contract: contracts.len(),
                    net: ni,
                    max_r: p.max_r,
                    max_c: p.max_c,
                });
            }
            contracts.push(c);
        }
        for d in &rec.differential {
            let mut c = d.to_contract(&names);
            if let (Some(a), Some(b)) = (net_idx(&d.net_pos), net_idx(&d.net_neg)) {
                c.consume("routing");
                diff.push(DiffCheck {
                    contract: contracts.len(),
                    net_a: a,
                    net_b: b,
                    max_len_pct: d.max_length_delta_pct,
                    max_r_pct: d.max_r_delta_pct,
                    max_c_pct: d.max_c_delta_pct,
                    same_layer: d.same_layer_required,
                });
            }
            contracts.push(c);
        }
        for s in &rec.straight {
            let mut c = s.to_contract(&names);
            if let Some(ni) = net_idx(&s.net) {
                c.consume("routing");
                straight.push(StraightCheck {
                    contract: contracts.len(),
                    net: ni,
                    vertical: s.vertical,
                });
            }
            contracts.push(c);
        }
        Self {
            contracts,
            checks,
            para,
            diff,
            straight,
            open: usize::MAX,
        }
    }

    /// Post-route contract closure on final geometry: parasitic budgets,
    /// differential-pair matching, straight-net spread. R/C checks need PEX
    /// params; without them those contracts stay Consumed (never judged on
    /// invented numbers). Length/spread checks are pure geometry — always run.
    fn reconcile_geometry(
        &mut self,
        wires: &[Wire],
        vias: &[Via],
        n_nets: usize,
        wire_params: Option<&pnr_constraints::WireParasiticParams>,
        straight_tol_nm: i32,
    ) {
        let mut len = vec![0i64; n_nets];
        // Per-net sorted layer sets. This cold-path representation scales with
        // the PDK stack instead of truncating at an arbitrary bit width.
        let mut layers_used = vec![Vec::<u32>::new(); n_nets];
        for w in wires {
            len[w.net as usize] += i64::from((w.x1 - w.x0).abs() + (w.y1 - w.y0).abs());
            layers_used[w.net as usize].push(w.layer);
        }
        for layers in &mut layers_used {
            layers.sort_unstable();
            layers.dedup();
        }
        let rc = wire_params.map(|wp| estimate_net_rc(wires, vias, n_nets, wp));

        for ch in &self.para {
            let Some((r, c)) = rc.as_ref() else { continue };
            let (rn, cn) = (r[ch.net as usize], c[ch.net as usize]);
            let over_r = if ch.max_r > 0.0 {
                rn / ch.max_r - 1.0
            } else {
                0.0
            };
            let over_c = if ch.max_c > 0.0 {
                cn / ch.max_c - 1.0
            } else {
                0.0
            };
            let worst = over_r.max(over_c);
            let ct = &mut self.contracts[ch.contract];
            if worst <= 0.0 {
                ct.satisfy(
                    "routing",
                    &format!("R {rn:.2} ohm, C {cn:.3} fF within budget"),
                );
            } else {
                ct.violate("routing", worst * 100.0, "% over budget");
            }
        }

        let delta_pct = |a: f64, b: f64| {
            let avg = (a + b) / 2.0;
            if avg > 0.0 {
                (a - b).abs() / avg * 100.0
            } else {
                0.0
            }
        };
        for ch in &self.diff {
            let (la, lb) = (len[ch.net_a as usize] as f64, len[ch.net_b as usize] as f64);
            if la == 0.0 || lb == 0.0 {
                continue; // one side unrouted — unrouted-net reporting covers it
            }
            let mut worst = delta_pct(la, lb) - ch.max_len_pct;
            if let Some((r, c)) = rc.as_ref() {
                worst = worst
                    .max(delta_pct(r[ch.net_a as usize], r[ch.net_b as usize]) - ch.max_r_pct)
                    .max(delta_pct(c[ch.net_a as usize], c[ch.net_b as usize]) - ch.max_c_pct);
            }
            let layers_differ =
                ch.same_layer && layers_used[ch.net_a as usize] != layers_used[ch.net_b as usize];
            let ct = &mut self.contracts[ch.contract];
            if worst <= 0.0 && !layers_differ {
                ct.satisfy("routing", &format!("len delta {:.1}%", delta_pct(la, lb)));
            } else if layers_differ {
                let a = &layers_used[ch.net_a as usize];
                let b = &layers_used[ch.net_b as usize];
                let diff_layers = a.iter().filter(|layer| !b.contains(layer)).count()
                    + b.iter().filter(|layer| !a.contains(layer)).count();
                ct.violate("routing", diff_layers as f64, "asymmetric layers");
            } else {
                ct.violate("routing", worst, "% over budget");
            }
        }

        for ch in &self.straight {
            if len[ch.net as usize] == 0 {
                continue;
            }
            // Spread perpendicular to the required direction; pin-landing stubs
            // on other layers count, so tolerance is one track pitch.
            let (mut lo, mut hi) = (i32::MAX, i32::MIN);
            for w in wires.iter().filter(|w| w.net == ch.net) {
                let (a, b) = if ch.vertical {
                    (w.x0, w.x1)
                } else {
                    (w.y0, w.y1)
                };
                lo = lo.min(a.min(b));
                hi = hi.max(a.max(b));
            }
            let spread = hi - lo;
            let ct = &mut self.contracts[ch.contract];
            if spread <= straight_tol_nm {
                ct.satisfy("routing", &format!("spread {spread} nm within one track"));
            } else {
                ct.violate(
                    "routing",
                    f64::from(spread - straight_tol_nm) / 1000.0,
                    "um",
                );
            }
        }
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
                c.status()
            ));
        }
        s
    }

    /// Extract violated crosstalk pairs as (net_a, net_b, shortfall_nm).
    fn violated_pairs(&self, net_names: &[String]) -> Vec<(String, String, f64)> {
        let mut out = Vec::new();
        for ch in &self.checks {
            let c = &self.contracts[ch.contract];
            if c.status() == ConstraintStatus::Violated {
                let shortfall = c.violation_metric().unwrap_or(0.0) * 1000.0; // um → nm
                let na = net_names
                    .get(ch.net_a as usize)
                    .cloned()
                    .unwrap_or_default();
                let nb = net_names
                    .get(ch.net_b as usize)
                    .cloned()
                    .unwrap_or_default();
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
            if hot.trees[ch.net_a as usize].is_empty() || hot.trees[ch.net_b as usize].is_empty() {
                continue;
            }
            let d = net_pair_clearance(hot, &cold.graph, ch.net_a, ch.net_b);
            let ok = d >= ch.min_nm;
            let c = &mut self.contracts[ch.contract];
            let want = if ok {
                ConstraintStatus::Satisfied
            } else {
                ConstraintStatus::Violated
            };
            if c.status() != want {
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
        if hpins.is_empty() {
            continue;
        }
        // NOTE: single-hpin nets are NOT skipped here — a one-pin net can
        // still expand to many physical finger pads that need strapping.
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
        let mut w =
            if SUPPLY_NAMES.contains(&lower.as_str()) || GROUND_NAMES.contains(&lower.as_str()) {
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
    let mut nets = build_nets(g, p, pin_pos, rec, &cfg.net_priority_overrides);
    nets.obstacles.extend_from_slice(&cfg.extra_obstacles);
    let n_nets = nets.names.len();
    eprintln!(
        "[routing] {n_nets} nets ({} obstacle-only pins), die {}x{} nm",
        nets.obstacles.len(),
        p.die.0,
        p.die.1
    );

    // ---- Stage 3: global (gcell) ----
    let ggrid = GcellGrid::new(p.die, cfg.global.gcells_per_side, cfg.global.gcell_capacity);
    let gterms: Vec<Vec<u32>> = nets
        .pins
        .iter()
        .map(|pts| {
            let mut t: Vec<u32> = pts
                .iter()
                .map(|&(x, y)| ggrid.at(x as f32, y as f32))
                .collect();
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
            rec.parasitic.iter().find(|b| b.net_name == *name).map(|b| {
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
    if cfg.global_history.len() == ghot.hist.len() {
        for (dst, &src) in ghot.hist.iter_mut().zip(&cfg.global_history) {
            *dst = src * cfg.history_decay.clamp(0.0, 1.0);
        }
    }
    eprintln!(
        "[routing] stage 1/2: global ({}x{} gcells, cap {})",
        gcold.graph.nx, gcold.graph.ny, cfg.global.gcell_capacity
    );
    let gt = route::run_global_route(&mut ghot, &gcold, &cfg.global, &mut rng);
    eprintln!(
        "[routing] stage 1/2 done: {} iters, residual overflow {}",
        gt.iters, gt.overflow
    );

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
            (
                p.x[i] - w / 2,
                p.y[i] - h / 2,
                p.x[i] + w / 2,
                p.y[i] + h / 2,
            )
        })
        .collect();
    let terms: Vec<Term> = nets
        .pins
        .iter()
        .enumerate()
        .flat_map(|(ni, pts)| {
            pts.iter().map(move |&(x, y)| Term {
                net: ni as u32,
                x,
                y,
            })
        })
        .collect();
    let (mut tgrid, tterms, missing_pins, landings, landing_failures, reserved) = build_track_grid(
        p.die,
        &footprints,
        &terms,
        &nets.obstacles,
        n_nets,
        &cfg.detailed,
        pin_pos.is_some(),
    );
    // Upper metals carry larger real width/spacing rules than the uniform
    // track gap (pitch - wire_width). The grid cannot express per-layer pitch,
    // so thin the track density instead: same-direction tracks `stride` apart
    // give stride*pitch >= worst feature + spacing clearance by construction.
    // With no per-layer steps configured, fall back to the legacy parity mask
    // on layers >= met3.
    for layer in 0..tgrid.n_layers {
        let stride = if cfg.layer_track_steps.is_empty() {
            i32::from(layer >= 2) + 1
        } else {
            let step = cfg
                .layer_track_steps
                .get(layer as usize)
                .copied()
                .unwrap_or(0);
            track_stride(step, cfg.detailed.pitch)
        };
        if stride <= 1 {
            continue;
        }
        for iy in 0..tgrid.ny {
            for ix in 0..tgrid.nx {
                // Even layers run horizontally: distinct tracks differ in y.
                let track = if layer % 2 == 0 { iy } else { ix };
                if track % stride as u32 != 0 {
                    let n = tgrid.node(ix, iy, layer) as usize;
                    tgrid.allowed[n] = false;
                }
            }
        }
    }
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
    if cfg.detailed_history.len() == dhot.hist.len() {
        for (dst, &src) in dhot.hist.iter_mut().zip(&cfg.detailed_history) {
            *dst = src * cfg.history_decay.clamp(0.0, 1.0);
        }
    }
    let mut ledger = RouteLedger::build(g, rec, &nets.net_of);
    if std::env::var("PNR_DEBUG_LANDINGS").is_ok() {
        for t in &terms {
            eprintln!("[dbg] term net={} ({},{})", t.net, t.x, t.y);
        }
        for &(ox, oy) in &nets.obstacles {
            eprintln!("[dbg] obstacle ({ox},{oy})");
        }
        for l in &landings {
            eprintln!(
                "[dbg] landing net={} pin=({},{}) node=({},{}) layer={}",
                l.net, l.pin.0, l.pin.1, l.node.0, l.node.1, l.layer
            );
        }
    }
    eprintln!(
        "[routing] stage 2/2: detailed ({} track nodes, {} pin landings, {} missing pins)",
        dcold.graph.nodes(),
        landings.len(),
        missing_pins.iter().map(|&m| u32::from(m)).sum::<u32>()
    );
    let dt = route::run_detailed_route(&mut dhot, &dcold, &mut ledger, &cfg.detailed, &mut rng);
    eprintln!(
        "[routing] stage 2/2 done: {} iters, {} proposed, {} accepted",
        dt.iters, dt.proposed, dt.accepted
    );

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
            eprintln!(
                "[routing] EOL repair: {} nets ripped up, {} nodes blocked",
                affected.len(),
                blocked.len()
            );
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
                let corridor: &[u32] = dcold.corridors.get(ni).map_or(&[], std::vec::Vec::as_slice);
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
                SUPPLY_NAMES.contains(&lower.as_str()) || GROUND_NAMES.contains(&lower.as_str())
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
                eprintln!(
                    "[routing] PRL repair: {} nets ripped up, {} nodes blocked",
                    affected.len(),
                    blocked.len()
                );
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
                    let corridor: &[u32] =
                        dcold.corridors.get(ni).map_or(&[], std::vec::Vec::as_slice);
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
    let overuse: u32 = dhot
        .usage
        .iter()
        .map(|&u| u32::from(u.saturating_sub(cap)))
        .sum();
    let unrouted: Vec<String> = (0..n_nets)
        .filter(|&i| dhot.trees[i].is_empty() || missing_pins[i] > 0)
        .map(|i| nets.names[i].clone())
        .collect();
    let no_path: Vec<String> = (0..n_nets)
        .filter(|&i| dhot.trees[i].is_empty() && missing_pins[i] == 0)
        .map(|i| nets.names[i].clone())
        .collect();
    let max_hist = dhot.hist.iter().copied().fold(0.0f32, f32::max);
    let hotspots: Vec<RoutingHotspot> = if max_hist > 0.0 {
        dhot.hist
            .iter()
            .enumerate()
            .filter_map(|(node, &hist)| {
                (hist >= 0.05 * max_hist).then(|| {
                    let (x, y, layer) = dcold.graph.pos(node as u32);
                    RoutingHotspot {
                        x,
                        y,
                        layer,
                        pressure: f64::from(hist),
                    }
                })
            })
            .collect()
    } else {
        Vec::new()
    };
    let (mut wires, vias) = extract_geometry_minarea(
        &dhot,
        &dcold.graph,
        cfg.detailed.wire_width,
        cfg.detailed.min_area,
    );
    // Per-layer wire widths: widen each wire to its conductor's real drawn
    // width (>= max(layer min width, EM min width), flow-derived from deck
    // rules) before min-area repair and RC estimation. Via cut/pad geometry
    // is drawn by the flow from its own per-layer via dims; `Via::size` is
    // advisory and left as extracted.
    for w in &mut wires {
        if let Some(&lw) = cfg.layer_widths.get(w.layer as usize) {
            w.width = w.width.max(lw);
        }
    }
    extend_upper_metal_islands(&mut wires, &cfg.layer_min_areas, &cfg.layer_spacings);
    ledger.reconcile_geometry(
        &wires,
        &vias,
        n_nets,
        cfg.wire_params.as_ref(),
        cfg.detailed.pitch,
    );
    let wirelength_nm: i64 = wires
        .iter()
        .map(|w| i64::from((w.x1 - w.x0).abs() + (w.y1 - w.y0).abs()))
        .sum();

    let net_hpwl: Vec<i64> = nets
        .pins
        .iter()
        .map(|pts| {
            if pts.len() < 2 {
                return 0;
            }
            let (mut xn, mut xx, mut yn, mut yx) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
            for &(x, y) in pts {
                xn = xn.min(x);
                xx = xx.max(x);
                yn = yn.min(y);
                yx = yx.max(y);
            }
            i64::from(xx - xn) + i64::from(yx - yn)
        })
        .collect();

    let report = RoutingReport {
        global: gt,
        detailed: dt,
        wirelength_nm,
        via_count: vias.len(),
        overuse,
        unrouted,
        no_path,
        missing_pin_count: landing_failures.len(),
        validation: ledger.validation(),
        contract_lines: ledger.summary(),
    };

    let crosstalk_violations = ledger.violated_pairs(&nets.names);

    let v = &report.validation;
    eprintln!("[routing] done: WL {} nm, {} vias, {} unrouted, overuse {} | contracts {}/{} satisfied, {} violated",
        report.wirelength_nm, report.via_count, report.unrouted.len(), report.overuse,
        v.satisfied, v.total, v.violated);
    if !report.unrouted.is_empty() {
        eprintln!("[routing] UNROUTED: {}", report.unrouted.join(", "));
    }

    if let Some(dir) = &cfg.debug_dir {
        if let Err(e) = dump_debug(dir, &nets.names, &report, &wires) {
            eprintln!("[routing] debug artifact write failed: {e}");
        }
    }

    RoutingResult {
        wires,
        vias,
        landings,
        net_names: nets.names,
        net_hpwl,
        report,
        crosstalk_violations,
        landing_failures,
        hotspots,
        global_history: ghot.hist.clone(),
        detailed_history: dhot.hist.clone(),
        track_pitch: cfg.detailed.pitch,
    }
}

/// Minimum same-direction track stride for a layer whose worst feature +
/// spacing needs `step` nm center-to-center on a uniform `pitch` grid.
fn track_stride(step: i32, pitch: i32) -> i32 {
    let pitch = pitch.max(1);
    ((step + pitch - 1) / pitch).max(1)
}

/// Upper-metal (>= met3) min-area: a one-hop jog plus its via pads is a
/// (pitch + width) x width island — under sky130's met3 minimum area (M3.AREA,
/// 240000 nm^2). Extend each short run symmetrically along its track unless
/// the grown rect would come within the upper-metal spacing of a foreign wire
/// on the same layer. Per-layer min-area/spacing come from the deck via
/// RoutingConfig; the constants are the legacy sky130 met3 fallback.
const UPPER_MIN_AREA: i64 = 240_000;
const UPPER_SPACING: i32 = 300;

fn extend_upper_metal_islands(wires: &mut [Wire], min_areas: &[i64], spacings: &[i32]) {
    let rects: Vec<(u32, u32, (i32, i32, i32, i32))> = wires
        .iter()
        .map(|w| (w.net, w.layer, wire_rect(w)))
        .collect();
    for i in 0..wires.len() {
        if wires[i].layer < 2 {
            continue;
        }
        let w = &wires[i];
        let min_area = min_areas
            .get(w.layer as usize)
            .copied()
            .unwrap_or(UPPER_MIN_AREA);
        let spacing = spacings
            .get(w.layer as usize)
            .copied()
            .unwrap_or(UPPER_SPACING);
        let len = i64::from((w.x1 - w.x0).abs() + (w.y1 - w.y0).abs());
        let width = i64::from(w.width.max(1));
        if min_area <= 0 || (len + width) * width >= min_area {
            continue;
        }
        // extra length needed, split across both ends, snapped up to 5nm grid
        let need = (min_area + width - 1) / width - width - len;
        #[allow(clippy::cast_possible_truncation)]
        let ext = ((((need + 1) / 2 + 4) / 5 * 5) as i32).max(5);
        let horiz = w.y0 == w.y1;
        let (rx0, ry0, rx1, ry1) = wire_rect(w);
        let grown = if horiz {
            (rx0 - ext - spacing, ry0, rx1 + ext + spacing, ry1)
        } else {
            (rx0, ry0 - ext - spacing, rx1, ry1 + ext + spacing)
        };
        let conflict = rects.iter().enumerate().any(|(j, &(net, layer, r))| {
            j != i
                && layer == wires[i].layer
                && net != wires[i].net
                && grown.0 < r.2
                && r.0 < grown.2
                && grown.1 < r.3
                && r.1 < grown.3
        });
        if conflict {
            continue;
        }
        let w = &mut wires[i];
        if horiz {
            if w.x0 <= w.x1 {
                w.x0 -= ext;
                w.x1 += ext;
            } else {
                w.x1 -= ext;
                w.x0 += ext;
            }
        } else if w.y0 <= w.y1 {
            w.y0 -= ext;
            w.y1 += ext;
        } else {
            w.y1 -= ext;
            w.y0 += ext;
        }
    }
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
        let axis_x = if gi < p.axes.len() {
            p.axes[gi]
        } else {
            continue;
        };

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
                        // Mirrored node must be routable (stride-masked upper
                        // tracks are off-limits) and free of foreign nets.
                        if !grid.allowed[mn as usize]
                            || (hot.usage[mn as usize] >= grid.cap()
                                && !cold.terms[dst].contains(&mn))
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
    net_names: &[String],
    r: &RoutingReport,
    wires: &[Wire],
) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let w = |name: &str, content: String| -> std::io::Result<()> {
        std::fs::write(dir.join(name), content)
    };
    w("global_route_trace.csv", r.global.trace_csv())?;
    w("detailed_route_trace.csv", r.detailed.trace_csv())?;
    w("route_report.txt", r.to_string())?;
    w("route_contracts.txt", r.contract_lines.clone())?;
    let mut txt = String::from("net layer x0 y0 x1 y1\n");
    for wire in wires {
        txt.push_str(&format!(
            "{} met{} {} {} {} {}\n",
            net_names[wire.net as usize],
            wire.layer + 1,
            wire.x0,
            wire.y0,
            wire.x1,
            wire.y1
        ));
    }
    w("routes.txt", txt)?;
    Ok(())
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
    use pnr_placement::{estimate_sizes, run_placement, PlacementConfig};

    fn flow(rec: &ConstraintRecord) -> (BipartiteHypergraph, Placement, RoutingResult) {
        let g = pnr_cells::fixtures::ota();
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
        let g = pnr_cells::fixtures::ota();
        let rec = ota_record(&g);
        let (_g, _p, r) = flow(&rec);
        assert!(
            r.report.unrouted.is_empty(),
            "unrouted: {:?}",
            r.report.unrouted
        );
        assert_eq!(r.report.overuse, 0, "track overuse (shorts) remain");
        assert!(!r.wires.is_empty());
        assert!(r.report.wirelength_nm > 0);
        for w in &r.wires {
            assert!(w.x0 >= 0 && w.x1 <= _p.die.0 && w.y0 >= 0 && w.y1 <= _p.die.1);
        }
    }

    #[test]
    fn crosstalk_contract_reconciled() {
        let g = pnr_cells::fixtures::ota();
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
    fn differential_and_parasitic_contracts_reconciled() {
        let g = pnr_cells::fixtures::ota();
        let mut rec = ota_record(&g);
        rec.differential.push(pnr_constraints::DifferentialPair {
            net_pos: "vout1".into(),
            net_neg: "vout2".into(),
            // Generous budgets: the check must judge, not necessarily pass tight ones.
            max_length_delta_pct: 200.0,
            max_r_delta_pct: 200.0,
            max_c_delta_pct: 200.0,
            same_layer_required: false,
        });
        rec.parasitic.push(pnr_constraints::ParasiticBudget {
            net_name: "vtail".into(),
            max_r: 1e6,
            max_c: 1e6,
        });
        let g2 = pnr_cells::fixtures::ota();
        let sizes = estimate_sizes(&g2);
        let p = run_placement(&g2, &sizes, &rec, &PlacementConfig::default(), &[]).placement;
        let cfg = RoutingConfig {
            wire_params: Some(pnr_constraints::WireParasiticParams {
                sheet_r: vec![0.1, 0.1, 0.1],
                area_cap: vec![25.0, 25.0, 25.0],
                fringe_cap: vec![40.0, 40.0, 40.0],
                via_r: 5.0,
            }),
            ..Default::default()
        };
        let r = run_routing(&g2, &p, &rec, &cfg);
        // Both contracts must leave Emitted (consumed) and, with both nets
        // routed + params present, close to Satisfied/Violated.
        assert_eq!(r.report.validation.total, 2);
        assert_eq!(
            r.report.validation.emitted, 0,
            "{}",
            r.report.contract_lines
        );
        assert!(
            r.report.validation.satisfied + r.report.validation.violated == 2,
            "diff/parasitic contracts must be judged post-route:\n{}",
            r.report.contract_lines
        );
    }

    #[test]
    fn deterministic_given_seed() {
        let g = pnr_cells::fixtures::ota();
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
        let g = pnr_cells::fixtures::ota();
        let rec = ota_record(&g);
        let sizes = estimate_sizes(&g);
        let p = run_placement(&g, &sizes, &rec, &PlacementConfig::default(), &[]).placement;
        let dir = std::env::temp_dir().join("pnr_routing_debug_test");
        let _ = std::fs::remove_dir_all(&dir);
        let cfg = RoutingConfig {
            debug_dir: Some(dir.clone()),
            ..Default::default()
        };
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

    #[test]
    fn estimate_net_rc_basic() {
        // Two wires on net 0 (layer 0, met1): 2000nm long, 200nm wide each
        let wires = vec![
            Wire {
                net: 0,
                layer: 0,
                x0: 0,
                y0: 0,
                x1: 2000,
                y1: 0,
                width: 200,
            },
            Wire {
                net: 0,
                layer: 0,
                x0: 0,
                y0: 500,
                x1: 1000,
                y1: 500,
                width: 200,
            },
        ];
        let vias = vec![Via {
            net: 0,
            x: 500,
            y: 250,
            size: 170,
            layer: 0,
        }];
        let params = pnr_constraints::WireParasiticParams {
            sheet_r: vec![0.1],     // 0.1 ohm/sq for met1
            area_cap: vec![25.0],   // 25 aF/um²
            fringe_cap: vec![40.0], // 40 aF/um
            via_r: 5.0,             // 5 ohm per via
        };
        let (r, c) = estimate_net_rc(&wires, &vias, 1, &params);
        // Wire 1: 2000nm/200nm = 10 squares → 1.0 ohm
        // Wire 2: 1000nm/200nm = 5 squares → 0.5 ohm
        // Via: 5.0 ohm
        // Total R = 6.5 ohm
        assert!((r[0] - 6.5).abs() < 0.01, "R={}", r[0]);
        // C: wire 1: area = 2.0*0.2 = 0.4 um² → 10 aF, fringe = 2*(2.0+0.2)*40 = 176 aF
        //    wire 2: area = 1.0*0.2 = 0.2 um² → 5 aF, fringe = 2*(1.0+0.2)*40 = 96 aF
        // Total = (10+176+5+96) aF = 287 aF = 0.287 fF
        assert!((c[0] - 0.287).abs() < 0.01, "C={}", c[0]);
    }

    #[test]
    fn track_stride_covers_wide_upper_layers() {
        // met1/met2: feature+spacing fits within one pitch → full density.
        assert_eq!(track_stride(440, 440), 1);
        // sky130 met3: 490 pad + 300 spacing on a 440 pitch → every 2nd track.
        assert_eq!(track_stride(790, 440), 2);
        // sky130 met4: 1180 via4 pad + 300 spacing → every 4th track.
        assert_eq!(track_stride(1480, 440), 4);
        // sky130 met5: 1600 wire + 1600 spacing → every 8th track.
        assert_eq!(track_stride(3200, 440), 8);
        // Degenerate inputs stay sane.
        assert_eq!(track_stride(0, 440), 1);
    }

    #[test]
    fn per_layer_widths_and_min_area_extension() {
        // A short met5-index wire must pick up its layer width and be extended
        // to its layer min-area, honoring per-layer tables over the constants.
        let mut wires = vec![Wire {
            net: 0,
            layer: 4,
            x0: 0,
            y0: 0,
            x1: 440,
            y1: 0,
            width: 1600,
        }];
        let min_areas = vec![83_000, 67_600, 240_000, 240_000, 1_600_000];
        let spacings = vec![140, 140, 300, 300, 1600];
        extend_upper_metal_islands(&mut wires, &min_areas, &spacings);
        let w = &wires[0];
        let len = i64::from((w.x1 - w.x0).abs());
        let rect_len = len + i64::from(w.width);
        assert!(
            rect_len * i64::from(w.width) >= 1_600_000,
            "met5 island below M5.AREA: {rect_len} x {}",
            w.width
        );
    }

    #[test]
    fn feedback_with_parasitic_params() {
        let g = pnr_cells::fixtures::ota();
        let rec = ota_record(&g);
        let (_, p, r) = flow(&rec);
        let params = pnr_constraints::WireParasiticParams {
            sheet_r: vec![0.1, 0.1],
            area_cap: vec![25.0, 25.0],
            fringe_cap: vec![40.0, 40.0],
            via_r: 0.0,
        };
        let device_names: Vec<String> = g.cells.iter().map(|c| c.name.clone()).collect();
        let net_of_device = |dev: u32, pin: &str| -> Option<usize> {
            let cell = g.cells.get(dev as usize)?;
            let (_, net_id) = cell.pins.iter().find(|(n, _)| n == pin)?;
            let net_name = g.nets.get(*net_id as usize)?;
            r.net_names.iter().position(|n| n == net_name)
        };
        let fb = extract_feedback_with_prior(
            &r,
            &p,
            &rec.parasitic,
            5.0,
            1.5,
            None,
            Some(&params),
            &rec.symmetry,
            &device_names,
            &net_of_device,
        );
        // net_r and net_c should be populated (nonzero for routed nets)
        assert_eq!(fb.net_r_ohm.len(), r.net_names.len());
        assert_eq!(fb.net_c_ff.len(), r.net_names.len());
        let total_r: f64 = fb.net_r_ohm.iter().sum();
        assert!(total_r > 0.0, "total R should be positive for routed OTA");
    }
}
