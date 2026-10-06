//! # `library` — the whole flow as one deterministic function.
//!
//! [`run`] reads top-down: parse → annotate → cells → place (`gp` → `dp`) →
//! route (`gr` → `dr`) → signoff, repeated until the best epoch stops
//! improving. [`signoff`] is the final DRC/ERC/LVS/PEX gate.

mod cellgen;
mod fill;
mod geometry;
mod hier;
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

use annotator::{AnnotationConfig, Problem};
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
    /// Power threshold, µW, past which a cell is a heat source a Moderate or
    /// Exceptional matched set keeps ≥ 1 µm/mW from (PLC-14). Policy: Hastings
    /// rule 14 exempts "small power devices" without a number.
    pub heat_source_uw: i32,
    /// What a MOS card's `W` means; [`run`] stores it as the SPICE total.
    /// Only [`run`] reads it: [`parse`] is always [`SizeConvention::Spice`].
    pub size_convention: SizeConvention,
    /// What gp does before dp; [`GpMode::Pile`] measures gp's contribution.
    pub gp_mode: GpMode,
    /// Which detailed placer runs ([`dp::DpMode::Sp`]: sequence-pair anneal, PLC-09).
    pub dp_mode: dp::DpMode,
    /// Fixed die and boundary pins. [`run`] checks each pin names a port;
    /// nothing else reads it yet (PLC/RTE consume it).
    pub interface: Option<Interface>,
    /// The top sub-circuit ([`ParseOptions::top`]); `None`: the parser's choice.
    pub top: Option<String>,
    /// User constraint sidecar, JSON text (EXT-26, `annotator::sidecar`):
    /// [`run`] merges it over `annotation` (lists extend, scalars from the base).
    pub constraints: Option<String>,
    /// ESD pad nets (REL-17); `None` = no ESD width floor.
    pub esd: Option<EsdSpec>,
    /// FLOW-08: with an incumbent from the current assignment, every
    /// `cold_every`-th epoch (counted over the whole search) is cold (gp +
    /// [`dp::Schedule::cold`], fresh routing history); the rest anneal the
    /// incumbent warm ([`epoch_kind`]). `1` = every epoch cold: T10's
    /// cold-only baseline, not the pre-FLOW-08 loop (which kept routing
    /// history across all epochs and ran on past a feasible incumbent).
    pub cold_every: u32,
    /// A warm epoch's dp schedule.
    pub warm: dp::Schedule,
    /// Wall budget over the whole run, starts included: once it is spent, a
    /// search that has an incumbent stops before its next epoch
    /// ([`StopReason::WallBudget`]). `None` = unbounded.
    pub max_wall: Option<std::time::Duration>,
    /// Flat, or each eligible sub-circuit definition solved once as a block (FLOW-11).
    pub hierarchy: Hierarchy,
    /// PLC-16: price each performance row's MST ground-C estimate on
    /// placement ([`analog::placement::PlacePerf`]); the T5 A/B switch.
    pub place_perf: bool,
}

/// Bottom-up hierarchy (FLOW-11). Opt-in: bottom-up is not globally optimal
/// (Balasa–Graeb L3068–3075): a child is solved without its parent's context.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Hierarchy {
    /// One flat solve of every device.
    #[default]
    Flat,
    /// Every eligible definition with at least `min_devices` devices
    /// (own + nested) solved once, children first, and instanced as a cell.
    BottomUp { min_devices: usize },
    /// `BottomUp { min_devices: 2 }` when the flattened top has more than 100
    /// devices, else `Flat` (policy bound, Balasa–Graeb L1604–1605).
    Auto,
}

/// [`Config::hierarchy`] resolved on `nl`: `Some(min_devices)`, `None` = flat.
fn hierarchy(cfg: &Config, nl: &pnr_core::Netlist) -> Option<usize> {
    match cfg.hierarchy {
        Hierarchy::Flat => None,
        Hierarchy::BottomUp { min_devices } => Some(min_devices),
        Hierarchy::Auto => (nl.devices.len() > 100).then_some(2),
    }
}

/// A die edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// Top edge (`+y`).
    North,
    /// Bottom edge (`−y`).
    South,
    /// Right edge (`+x`).
    East,
    /// Left edge (`−x`).
    West,
}

/// One boundary pin: `frac` ∈ [0, 1] along `side` (from its low end).
#[derive(Clone, Debug, PartialEq)]
pub struct IoPin {
    /// Net name; [`run`] refuses one that is not a top-cell port.
    pub net: String,
    /// The die edge the pin sits on.
    pub side: Side,
    /// Position along `side` from its low end (west or south), ∈ [0, 1].
    pub frac: f32,
    /// Pin width along the edge, nm (> 0).
    pub width_nm: i32,
    /// Deck layer name (`met3`).
    pub layer: String,
}

/// A block's fixed outline and boundary pins.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Interface {
    /// `(w, h)`; `None`: the placer sizes the die.
    pub die_nm: Option<(i32, i32)>,
    /// Boundary pins, in file order.
    pub pins: Vec<IoPin>,
}

impl Interface {
    /// `{"die": {"w": nm, "h": nm}, "pins": [{"net", "side": "north"|"south"|"east"|"west",
    /// "frac", "width": nm, "layer"}]}` (`benchmarks/fixtures/ota_constrained.interface.json`).
    /// A missing or `null` `die` leaves the die to the placer; missing or
    /// `null` `pins` means none.
    ///
    /// # Errors
    /// Malformed JSON; a `die` or `pins` of the wrong shape; a die extent or
    /// pin width that is not a positive `i32`; a `side` outside
    /// north|south|east|west; a `frac` outside [0, 1]. The message names the
    /// offending field (`pins[2].frac`).
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
    /// gp skipped: under [`dp::DpMode::Sp`] dp starts from
    /// [`dp::sp::Tree::seed_constructive`]; under `Flat` (which needs
    /// coordinates) it starts from the pile, as [`GpMode::Pile`].
    Constructive,
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
            heat_source_uw: 1000,
            size_convention: SizeConvention::Spice,
            gp_mode: GpMode::default(),
            dp_mode: dp::DpMode::default(),
            interface: None,
            top: None,
            constraints: None,
            esd: None,
            // ponytail: T10 (`bench local` over cold_every ∈ {1,2,4,8,∞}) has
            // not been measured, so the default stays all-cold (history reset
            // every epoch; not the pre-FLOW-08 loop, which kept it).
            cold_every: 1,
            warm: dp::Schedule::warm(),
            max_wall: None,
            hierarchy: Hierarchy::Flat,
            place_perf: true,
        }
    }
}

/// ESD pad nets and their HBM rating (REL-17): each named net gets a hard
/// [`analog::routing::EsdWidth`] floor on its routed metal.
#[derive(Clone, Debug, PartialEq)]
pub struct EsdSpec {
    /// Human-body-model rating, V.
    pub hbm_v: f32,
    /// Pad net names; one absent from the netlist is a missing-input row.
    pub nets: Vec<String>,
}

/// A finished placement + routing (pre-signoff).
pub struct Solution {
    /// The winner's placement, indexed by cell (not device: see `devices_of`).
    pub layout: Layout,
    /// The winner's routed wires, indexed by net.
    pub routes: Routes,
    /// Placed cells indexed like `layout`, then guard rings (absolute coords).
    pub macros: Vec<Macro>,
    /// The parsed schematic — signoff's LVS reference — plus every device a
    /// stage inserted (antenna diodes), appended after the parsed ones.
    pub netlist: pnr_core::Netlist,
    /// How the search went and the winner's per-stage legality.
    pub stats: RunStats,
    /// The winner's constraint-budget report.
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
    /// dr's counters for `routes` (congestion, exact pairs, …).
    pub route_stats: dr::RouteStats,
    /// Annotator findings (EXT-26 sidecar entries it could not apply,
    /// ambiguous symmetry, conflicts), in annotator order; the CLI writes one
    /// line each to `report.txt`.
    pub diagnostics: Vec<analog::intent::Diagnostic>,
    /// The shipped geometry's PEX matrix: post-fill when `metadata.post_fill`,
    /// else the winning epoch's (PERF-15/22 read it).
    pub caps: verify::CapMatrix,
    /// Per sub-circuit definition solved as a block (FLOW-11), children first:
    /// its name and its own solve's report. Empty when flat.
    pub blocks: Vec<(String, metadata::MetadataReport)>,
    /// The LVS side of the placed blocks ([`hier::BlockRef`]).
    pub(crate) block_ref: hier::BlockRef,
    /// Nets to field-solve (PERF-16, [`field_nets`]); [`signoff`] does not yet
    /// (deferred, owner decision: docs/plans/cards/m2-perf-5.md).
    pub field_nets: Vec<String>,
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
    /// Warm epochs ([`epoch_kind`]) over the winning start's search.
    pub warm_epochs: u32,
    /// Why the winning start's search ended.
    pub stop: StopReason,
    /// Sub-circuit definitions solved as blocks (FLOW-11); 0 when flat.
    pub block_solves: u32,
}

/// Why a search ended (FLOW-08).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StopReason {
    /// Feasible with stationary, unsaturated prices.
    Converged,
    /// Feasible, prices still moving or saturated: a feasible incumbent is
    /// never escalated, so the search ends here.
    FeasibleNotStationary,
    /// Infeasible when the last variant assignment stalled.
    #[default]
    OuterBudget,
    /// Infeasible and every allowed assignment tried.
    EscalationExhausted,
    /// [`Config::max_wall`] spent.
    WallBudget,
}

/// An epoch's start: gp + cold dp, or a warm dp from the incumbent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Cold,
    Warm,
}

/// Epoch `k` (counted over the whole search) is cold without an incumbent
/// from the current assignment (none yet, or the incumbent predates the last
/// escalation) or every `cold_every`-th epoch; else warm.
fn epoch_kind(k: u32, current_incumbent: bool, cold_every: u32) -> Kind {
    if !current_incumbent || k % cold_every.max(1) == 0 {
        Kind::Cold
    } else {
        Kind::Warm
    }
}

/// After a middle loop: the search's stop, or `None` = escalate. A feasible
/// incumbent is never escalated.
fn next_action(feasible: bool, stationary: bool, last_outer: bool) -> Option<StopReason> {
    match (feasible, stationary, last_outer) {
        (true, true, _) => Some(StopReason::Converged),
        (true, false, _) => Some(StopReason::FeasibleNotStationary),
        (false, _, true) => Some(StopReason::OuterBudget),
        (false, _, false) => None,
    }
}

/// [`RunStats::stage_ms`]'s stages: `route` is gr+dr, `reroute` dr again
/// after antenna diodes, `perf` the post-layout simulation.
pub const STAGES: [&str; 9] = ["gp", "dp", "rings", "route", "reroute", "diodes", "signoff", "metadata", "perf"];

/// Anything that stops the flow.
#[derive(Debug)]
pub enum FlowError {
    /// The SPICE front end's message.
    Parse(String),
    /// An injected macro for a FET instance does not extract to exactly one
    /// device: `(instance, devices extracted, None = extraction failed)`. It
    /// would unpair LVS for the whole circuit, so it is refused up front.
    InjectedNotADevice(String, Option<usize>),
    /// [`Config::interface`] names a net that is not a port of the top cell.
    Interface(String),
    /// The deck's routing stack is unusable (`elaborate::routing_stack`).
    Deck(String),
}

/// A power rail or the substrate: `Supply`, `Ground` or `Substrate`. Rails
/// carry no signal, so no performance row, C tier or field solve weighs them.
fn is_rail(class: analog::metadata::NetClass) -> bool {
    use analog::metadata::NetClass::{Ground, Substrate, Supply};
    matches!(class, Supply | Ground | Substrate)
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
///
/// Cost: `cfg.starts` threads, each up to `outer_iters × feedback_iters`
/// epochs of place → route → signoff (bounded by `cfg.max_wall`).
///
/// # Errors
/// [`FlowError::Parse`] for SPICE the parser rejects;
/// [`FlowError::Interface`] for interface pins on non-port nets or a
/// malformed constraint sidecar; [`FlowError::InjectedNotADevice`] for an
/// injected FET macro that does not extract to exactly one device;
/// [`FlowError::Deck`] for an unusable routing stack.
///
/// # Panics
/// When a search thread panics (a broken internal invariant).
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
    // 2. Bias: per-device power and per-net current. Placement-independent,
    //    so solved once, before annotation: its op point and testbench are the
    //    annotator's evidence (EXT-17).
    let bias = bias(&netlist, cfg);
    // FLOW-11: blocks are solved first, each from its first instance, and
    // enter this solve as one cell per outermost instance.
    let (blocks, placed) = match hierarchy(cfg, &netlist) {
        Some(k) => {
            let b = hier::solve_blocks(&netlist, pdk, cfg, &bias, k)?;
            let p = hier::instantiate(&netlist, &b);
            (b, p)
        }
        None => Default::default(),
    };
    let mut sol = solve(&netlist, pdk, injected, cfg, bias, &placed)?;
    sol.stats.block_solves = blocks.len() as u32;
    sol.blocks = blocks.into_iter().map(|b| (b.subckt, b.metadata)).collect();
    Ok(sol)
}

/// Steps 3–7 of [`run`] on a parsed, biased `netlist`: annotate, draw cells
/// (each of `blocks.cells` one fixed-geometry cell), search, finish. A block
/// child is solved here too ([`hier::solve_blocks`]).
fn solve(netlist: &pnr_core::Netlist, pdk: &Pdk, injected: &Macros, cfg: &Config, bias: Bias, blocks: &hier::Placed) -> Result<Solution, FlowError> {
    // Annotated once here for the sensitivity rows; each topology annotates
    // its own `Problem` with the same `ann` (one leaked stack per run).
    let stack: &'static analog::routing::Stack = Box::leak(Box::new(elaborate::stack(pdk)));
    let mut ann = annotation_with(pdk, &cfg.annotation, stack);
    ann.process.die_temp_k = cfg.op.as_ref().map(|o| o.temp_c as f32 + 273.15);
    if let Some(text) = &cfg.constraints {
        let (side, diags) = AnnotationConfig::from_json(text, netlist).map_err(FlowError::Interface)?;
        ann.supply_nets.extend(side.supply_nets);
        ann.ground_nets.extend(side.ground_nets);
        ann.clock_nets.extend(side.clock_nets);
        ann.do_not_identify.extend(side.do_not_identify);
        ann.seeds.extend(side.seeds);
        ann.symmetry_dir = ann.symmetry_dir.or(side.symmetry_dir);
        ann.groups.extend(side.groups);
        ann.classes.extend(side.classes);
        ann.net_classes.extend(side.net_classes);
        ann.offset_budgets.extend(side.offset_budgets);
        ann.loads.extend(side.loads);
        ann.kelvins.extend(side.kelvins);
        ann.tubs.extend(side.tubs);
        ann.sidecar_diags.extend(diags);
    }
    let ev = bias.op.as_ref().map_or_else(Default::default, |o| {
        let mut e = o.evidence(netlist, bias.summary.as_ref().is_some_and(|s| s.probe));
        if let Some(tb) = cfg.op.as_ref().and_then(|c| c.testbench.as_deref()) {
            (e.switching_nets, e.dc_sources) = oppoint::testbench_sources(netlist, tb);
        }
        e
    });
    let base = annotator::annotate_with(netlist, &ann, &ev);
    let plan = performance_rows(netlist, cfg, &ann, &base.net_classes);
    let ev = annotator::Evidence { sens: plan.evidence.clone(), ..ev };

    // 3–7 per cell topology. A distinct-gate pair merged as ABBA cancels a
    // linear gradient but splits one drain across the row ends (asymmetric
    // routing); apart, it routes as translated copies. Neither dominates in
    // general, so when a merge like that exists both are solved and the
    // lexicographically better kept.
    // Both topologies are built once on this thread (pricing every variant
    // once); the starts only search them.
    let merged = topology(netlist, injected, pdk, cfg, &bias, &ann, &ev, &plan, blocks, true)?;
    let apart = if merged.distinct { Some(topology(netlist, injected, pdk, cfg, &bias, &ann, &ev, &plan, blocks, false)?) } else { None };
    let tops: Vec<&Topology> = std::iter::once(&merged).chain(apart.as_ref()).collect();
    let tops = &tops;
    let deadline = cfg.max_wall.map(|d| std::time::Instant::now() + d);
    let runs: Vec<Vec<Searched>> = std::thread::scope(|s| {
        let handles: Vec<_> = (0..cfg.starts.max(1))
            .map(|j| {
                s.spawn(move || {
                    let seed = cfg.seed.wrapping_add(u64::from(j).wrapping_mul(0x9E37_79B9_7F4A_7C15));
                    tops.iter().map(|t| search(t, cfg, seed, deadline)).collect()
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
    let mut pareto = Vec::new();
    for p in runs.iter().flatten().flat_map(|r| r.pareto.iter().cloned()) {
        metadata::pareto_insert(&mut pareto, p);
    }
    let searched = runs.into_iter().nth(win.0).expect("one start at least").swap_remove(win.1);
    let t = if win.1 == 0 { merged } else { apart.expect("index 1 is `apart`") };
    let mut sol = finish(t, searched, &bias, pdk);
    (sol.stats.sim_failures, sol.metadata.sim_failures) = (sim_failures, sim_failures);
    sol.stats.stage_ms = stage_ms;
    sol.stats.sims = sims;
    sol.metadata.budget_rows = plan.notes;
    sol.metadata.sensitivity = plan.sens;
    sol.metadata.pareto = pareto;
    sol.block_ref = blocks.refs.clone();
    Ok(sol)
}

/// Each spec bound as a shared routing budget from schematic sensitivities
/// (see [`annotator::budget::rows`]): one baseline plus one run per signal net, in
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
    let Some(p) = &cfg.performance else { return PerfPlan::schematic_only(Vec::new(), vec![0]) };
    let all: Vec<usize> = (0..p.scenarios().len()).collect();
    let bounds = || {
        p.specs.iter().flat_map(|s| {
            [(s.min, "min"), (s.max, "max")].into_iter().filter(|b| b.0.is_some_and(f64::is_finite)).map(move |(_, side)| format!("{}:{side}", s.metric))
        })
    };
    let notes = |rows: &[analog::routing::PerformanceBudget], why: &str| -> Vec<String> {
        let notes: Vec<String> = bounds()
            .map(|b| match rows.iter().find(|r| r.metric == b) {
                Some(r) if r.nets.is_empty() => format!("{b}: row with no adverse measured nets"),
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
        return PerfPlan::schematic_only(out, all);
    };
    let nets: Vec<pnr_core::NetId> = classes
        .iter()
        .filter(|c| !is_rail(c.class))
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
        Ok((start, active, tables, mut sens_notes)) => {
            // PERF-12: EXT-17's evidence and RTE-21's router weights, each
            // bound scaled by its statistical headroom (PERF-06's scale).
            let stats = robust::bound_stats(&tables, &sigma_v, &start, &p.specs, &[], &[]);
            let evidence = perf::to_evidence(p, &tables, &start, &stats, netlist);
            // EXT-25: R/C classes and `no_layout_margin` are the annotator's,
            // emitted from the same evidence; dropped here as duplicates.
            let (rows, _, _) = annotator::budget::rows(&evidence, af_per_um / 1000.0, ann.process.wire_ohm_per_um, &ann.policy);
            // PERF-14: decided once per run so every epoch is keyed alike.
            let beta_key = stats.iter().all(|s| s.sigma_f.is_some());
            let h_bounds: Vec<(usize, f64, usize)> = start
                .bounds
                .iter()
                .zip(&stats)
                .filter_map(|(b, st)| {
                    let spec = &p.specs[b.spec];
                    let (bound, sign) = if b.upper { (spec.max?, 1.0) } else { (spec.min?, -1.0) };
                    let plain = sign * (bound - b.value?);
                    let h = match st.headroom_stat {
                        Some(h) if h > 0.0 => h,
                        None if plain > 0.0 => plain,
                        _ if bound == 0.0 => 1.0,
                        _ => bound.abs(),
                    };
                    Some((b.spec, h, b.scenario))
                })
                .collect();
            let (r_weight, pair_weight) = perf::router_weights(&tables, &h_bounds, netlist);
            let unknown = || "unknown".to_string();
            for e in &evidence.specs {
                let sigma = e.sigma_f.map_or_else(unknown, |v| format!("{v:.3e}"));
                sens_notes.push(format!("evidence {}: d_c {}, d_r {}, d_vt {}, d_cc {}, σ_f {sigma}", e.metric, e.d_c.len(), e.d_r.len(), e.d_vt.len(), e.d_cc.len()));
            }
            let net_name = |n: pnr_core::NetId| &netlist.nets[n.0 as usize].name;
            for (n, w) in r_weight.iter().enumerate().filter(|(_, &w)| w != 0.0) {
                sens_notes.push(format!("r_weight {} {w:.3}", netlist.nets[n].name));
            }
            sens_notes.extend(pair_weight.iter().filter(|p| p.2 != 0.0).map(|&(a, b, w)| format!("pair_weight {}-{} {w:.3e}", net_name(a), net_name(b))));
            let mut out = scenario_notes(&active);
            out.extend(notes(&rows, "not measured at the schematic"));
            sens_notes.iter().for_each(|n| eprintln!("[perf] sens {n}"));
            let sims = (all.len() * p.testbenches.len()) as u32 + tables.iter().map(|t| t.sims).sum::<u32>();
            PerfPlan { rows, notes: out, active, tables, sigma_v, sens: sens_notes, sims, evidence: Some(evidence), r_weight, pair_weight, beta_key }
        }
        Err(e) => {
            let mut out = scenario_notes(&all);
            out.extend(notes(&[], &format!("sensitivities unavailable: {e}")));
            PerfPlan::schematic_only(out, all)
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
    /// EXT-17's spec sensitivities ([`perf::to_evidence`]); `None` without tables.
    evidence: Option<annotator::evidence::Sensitivities>,
    /// RTE-21's per-net R weight ([`perf::router_weights`]); computed and
    /// reported only until `DetailedCfg` takes it.
    #[allow(dead_code)] // RTE-21 step 1 writes it into `DetailedCfg` (not landed)
    r_weight: Vec<f32>,
    /// RTE-21's coupling pair weight; as `r_weight`.
    #[allow(dead_code)] // as `r_weight`
    pair_weight: Vec<(pnr_core::NetId, pnr_core::NetId, f32)>,
    /// PERF-14: every schematic bound has a σ_f, so epochs are keyed on β
    /// ([`robust::key_tiers`]); else on the spec miss.
    beta_key: bool,
}

impl PerfPlan {
    /// A plan with no rows, tables or evidence: `notes` and the `active`
    /// scenarios only (geometry-only scoring, or sensitivities unavailable).
    fn schematic_only(notes: Vec<String>, active: Vec<usize>) -> PerfPlan {
        PerfPlan { rows: Vec::new(), notes, active, tables: Vec::new(), sigma_v: Vec::new(), sens: Vec::new(), sims: 0, evidence: None, r_weight: Vec::new(), pair_weight: Vec::new(), beta_key: false }
    }
}

/// The operating point, solved once per run.
struct Bias {
    /// Per device dissipation, µW, indexed like `netlist.devices`; empty =
    /// a uniform die.
    power: Vec<i32>,
    /// The report's bias provenance; `None` without an operating point.
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

impl Bias {
    /// No operating point: `power` as declared ([`Config::device_power_uw`]),
    /// no currents, headroom, gm or summary.
    fn uniform(power: Vec<i32>) -> Bias {
        Bias { power, summary: None, currents: None, net_headroom_mv: None, gm_us: Vec::new(), op: None }
    }
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
    ev: &annotator::Evidence,
    perf: &'a PerfPlan,
    blocks: &hier::Placed,
    merge_distinct_gates: bool,
) -> Result<Topology<'a>, FlowError> {
    #[cfg(test)]
    if !merge_distinct_gates {
        APART_BUILDS.with(|c| c.set(c.get() + 1));
    }
    let currents = &bias.currents;
    // Annotate: placement/routing rules + cell constraints, device-indexed.
    // Per topology: `CellSpace::new` mutates the problem, which is not `Clone`.
    let mut problem = annotator::annotate_with(netlist, ann, ev);
    let (perf_rows, perf_active) = (&perf.rows[..], &perf.active[..]);
    for row in perf_rows {
        problem.routing.budget.push(Box::new(row.clone()));
    }

    // Cells: every legal variant drawn once; matched groups collapse to one
    // cell and every device-indexed table moves to cell space.
    // One fold table for the cells and every LVS reference of this run.
    let unit_cells: Vec<(Vec<DeviceId>, bool)> = problem.constraints.unitization.iter().map(|u| (u.devices.clone(), u.route_matching_required)).collect();
    let fold = cellgen::folds(&netlist, pdk, &bias.gm_us, &unit_cells);
    let cells = CellSpace::new(netlist, injected, &mut problem, pdk, &bias.power, merge_distinct_gates, &fold, &blocks.cells);
    // Already cell-indexed: pushed after the retarget.
    let env = live_environment(&problem, &cells, pdk);
    if !env.pairs.is_empty() {
        problem.placement.budget.push(Box::new(env.clone()));
    }
    // PLC-14: matched sets keep their distance from hot cells.
    let sets: Vec<(pnr_core::MatchClass, Vec<u16>)> = problem
        .intent
        .sets
        .iter()
        .map(|s| {
            let mut c: Vec<u16> = s.members.iter().filter_map(|m| cells.units.cell_of.get(m.device.0 as usize).copied()).collect();
            c.sort_unstable();
            c.dedup();
            (s.class, c)
        })
        .collect();
    let heat = analog::placement::heat::separations(&sets, &cells.power, cfg.heat_source_uw);
    if !heat.is_empty() {
        problem.placement.budget.push(Box::new(heat.clone()));
        problem.placement.cost.push(Box::new(heat));
    }
    // PLC-21: a Mirror pair stays Mirror only if no variant of either cell
    // leaves a net φx (Mx180 would reverse its current). Every variant, since
    // dp reshapes.
    let mirror_ok = |c: u32| {
        cells.variants.get(c as usize).is_some_and(|v| v.alternatives.iter().all(|m| analog::matching::moments::mirror_allowed_units(&m.units)))
    };
    let keep = |a: u32, b: u32| a != b && mirror_ok(a) && mirror_ok(b);
    for b in problem.placement.hard.iter_mut().chain(&mut problem.placement.budget).chain(&mut problem.placement.cost) {
        b.demote_mirrors(&keep);
    }
    let locks = dp::locks::locks(&problem.placement, cells.variants.len(), &cells.variants);
    let rules = place_rules(pdk, &cells, &locks, &match_class(&problem, &cells));
    // PLC-24: the floor is pushed only now, capped by what these cells and gaps can reach.
    if cfg.min_utilization > 0.0 {
        let dims: Vec<(i32, i32)> =
            cells.variants.iter().map(|v| v.alternatives.first().map_or((0, 0), |m| (m.bbox.w, m.bbox.h))).collect();
        let u_min = u_eff(cfg.min_utilization, &dims, median_x_gap(&rules));
        problem.placement.budget.push(Box::new(analog::placement::utilization::Utilization { u_min }));
    }
    // PLC-12: each symmetry axis's cells form one island.
    for island in symmetry_islands(&problem.placement, &rules) {
        problem.placement.budget.push(Box::new(island.clone()));
        problem.placement.cost.push(Box::new(island));
    }
    // PLC-16: each performance row's ground C, estimated on placement.
    if cfg.place_perf && !perf_rows.is_empty() {
        let alts: Vec<&[Macro]> = cells.variants.iter().map(|v| &v.alternatives[..]).collect();
        for row in perf_rows {
            problem.placement.budget.push(Box::new(analog::placement::PlacePerf::new(row, &alts, analog::placement::perf::RESERVE)));
        }
    }
    let distinct = cells.distinct_gate_merges > 0;

    // 5. Stages. The metal stack and router config come from the deck.
    let stack = elaborate::routing_stack(pdk, None).map_err(FlowError::Deck)?;
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
    em_rules(&mut problem, netlist, &em, &em_layers, &em_cuts, ann.process.stack, em_front_row(pdk), pdk);
    if let (Some(esd), Some(stack)) = (&cfg.esd, ann.process.stack) {
        let (rho, cv) = analog::routing::em::metal_family(pdk.cell_str("metal_family"));
        let area_um2 = analog::routing::em::esd_area_um2(esd.hbm_v, rho, cv);
        let mut rules = Vec::new();
        for name in &esd.nets {
            match netlist.nets.iter().position(|n| n.name == *name) {
                Some(k) => rules.push(analog::routing::EsdWidth { net: pnr_core::NetId(k as u16), area_um2, stack }),
                // ponytail: leaked once per run, as `em_rules`'s names are.
                None => problem.missing.push(("EsdWidth", Box::leak(format!("net {name} (Config::esd) not in the netlist").into_boxed_str()))),
            }
        }
        problem.routing.hard.push(Box::new(rules));
    }
    // IR-drop budgets (PWR-02) on nets carrying op current (`annotator::ir`).
    let net_ua = bias.currents.as_ref().map_or_else(Vec::new, |c| oppoint::net_current_ua(netlist, c));
    let ir = if let (Some(_), Some(h)) = (&bias.currents, &bias.net_headroom_mv) {
        let vdd_mv = cfg.op.as_ref().map_or(1_800.0, |o| o.vdd * 1e3);
        let ir = annotator::ir::budgets(&problem.net_classes, &net_ua, h, vdd_mv, &ann.policy);
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
    let net_weight = gp::net_weights(&problem.net_classes, &sens, &net_ua);
    let intent = elaborate::intent(netlist, &problem.net_classes, currents.as_deref(), cfg.op.as_ref().map_or(0.0, |o| o.vdd * 1_000.0), &ir);
    let weights = Weights { route: route_weights(&net_weight, &problem.net_classes, &sens), place: net_weight };
    let mut flow = Flow {
        pdk,
        netlist,
        net_names: netlist.nets.iter().map(|n| n.name.clone()).collect(),
        block_ref: blocks.refs.clone(),
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
            r
        },
        layers,
        cuts,
        problem,
        cells,
        env,
        locks,
        perf: cfg.performance.as_ref(),
        perf_rows,
        perf_active,
        perf_plan: perf,
        intent,
        weights,
        warm: cfg.warm,
        fold,
        stack: ann.process.stack.expect("`annotation` always carries the stack"),
        id_ua: currents
            .as_ref()
            .map(|c| c.iter().map(|d| d.as_ref().and_then(|t| t.iter().find(|(n, _)| n == "D").map(|&(_, i)| i))).collect())
            .unwrap_or_default(),
        gp_mode: cfg.gp_mode,
        dp_mode: cfg.dp_mode,
        rules,
        halo_st: Vec::new(),
        halo_pins: Vec::new(),
    };
    // PLC-15: J and w_min of the lowest layer with a wire EM limit.
    if let Some(&(l, lim)) = flow.d_router.cfg.em.iter().find(|(_, lim)| lim.ua_per_um > 0.0) {
        let w_min = pdk.min_width(l.0).unwrap_or(0);
        flow.halo_st = static_halos(&flow.cells.variants, &flow.d_router.cfg.pin_ua, lim.ua_per_um / 1000.0, w_min, flow.rules.grid);
    }
    flow.halo_pins = face_pins(&flow.cells.variants);
    // Matched cells keep every alternative: a merged group, or a member of a
    // 2-device leaf that emits a `MatchedSet` (annotator emit.rs table).
    let matched = matched_cells(&flow.problem.blocks, &flow.cells.devices_of);
    // GAP-18: a cell inside an Exceptional unitization lists its alternatives best-matching first.
    let ranked: Vec<bool> = flow
        .cells
        .devices_of
        .iter()
        .map(|m| flow.problem.constraints.unitization.iter().any(|u| u.class == Some(pnr_core::MatchClass::Exceptional) && m.iter().all(|d| u.devices.contains(d))))
        .collect();
    let (assignment0, allowed) = cellgen::seed_assignment(&flow.cells.variants, &matched, &ranked, pdk);
    // PLC-28: symmetry axes on the router's track centrelines.
    flow.rules.axis_grid = axis_grid(&dr::lattice_spec(&flow.d_router.cfg), flow.rules.grid);
    Ok(Topology { flow, assignment0, allowed, distinct, t_em_k, em_derate })
}

/// One start's search on a shared topology: the winning epoch, the start's
/// stats, the saturated prices (`metadata.binding`) and the winner's key.
struct Searched {
    /// The winning epoch.
    best: Epoch,
    /// The start's counters merged with the winner's legality.
    stats: RunStats,
    /// Prices held at their cap ([`metadata::MetadataReport::binding`]).
    binding: Vec<String>,
    /// `best.key`, kept apart because `best` moves into [`finish`].
    key: LexKey,
    /// [`metadata::MetadataReport::pareto`] of this search.
    pareto: Vec<metadata::ParetoPoint>,
    /// [`metadata::MetadataReport::epochs`].
    epochs: Vec<metadata::ParetoPoint>,
    /// [`metadata::MetadataReport::candidates`].
    candidates: Vec<metadata::Candidate>,
}

/// 6. Search. Outer: variant assignment. Middle: epochs at that assignment,
///    keeping the best [`LexKey`], whose V includes the epoch's own signoff
///    errors. Prices persist across epochs; routing history is reset at every
///    cold epoch and kept across warm ones ([`epoch_kind`]); both are this
///    start's own. A feasible incumbent stops the search; an infeasible stall
///    escalates by blame ([`cellgen::escalate_blamed`]), never revisiting an
///    assignment. [`RunStats::stop`] says which way it ended.
fn search(t: &Topology, cfg: &Config, seed: u64, deadline: Option<std::time::Instant>) -> Searched {
    let flow = &t.flow;
    let mut assignment = t.assignment0.clone();
    let mut prices = gp::Prices::new();
    let mut neg = gr::Negotiation::new();
    let weights = flow.weights.clone();
    let mut best: Option<Epoch> = None;
    let mut stats = RunStats::default();
    let (mut pareto, mut epochs) = (Vec::new(), Vec::new());
    // Every unified assignment an epoch ran: escalation never revisits one.
    let mut tried: std::collections::BTreeSet<Vec<u16>> = std::collections::BTreeSet::new();
    // `current`: `best` came from an epoch of the current assignment (its
    // variants are that assignment's, maybe dp-reshaped); after an escalation
    // a warm start from it would anneal the old assignment instead.
    let (mut k, mut current) = (0u32, false);
    // PLC-15: congestion halo per cell (placed faces), smoothed across this
    // start's epochs; search-local because `Flow` is shared by the starts.
    let mut halo_dyn = vec![[0; 4]; flow.cells.variants.len()];
    let mut candidates = Vec::new();

    let n_outer = cfg.outer_iters.max(1);
    'search: for outer in 0..n_outer {
        stats.outer_iterations += 1;
        let mut stall = 0;
        for iter in 0..cfg.feedback_iters.max(1) {
            if best.is_some() && deadline.is_some_and(|d| std::time::Instant::now() >= d) {
                stats.stop = StopReason::WallBudget;
                break 'search;
            }
            stats.iterations += 1;
            let seed = seed ^ u64::from(iter) ^ (u64::from(outer) << 32);
            let kind = epoch_kind(k, best.is_some() && current, cfg.cold_every);
            k += 1;
            let start = match kind {
                Kind::Warm => best.as_ref().map(|b| &b.layout),
                Kind::Cold => {
                    neg = gr::Negotiation::new();
                    None
                }
            };
            let mut ran = start.map_or_else(|| assignment.clone(), |l| l.variant.clone());
            flow.locks.unify(&mut ran);
            tried.insert(ran);
            // Alternate: dp's reshape trades a squarer variant for HPWL, which
            // the utilization floor cannot see mid-anneal (a 1-row tail cost
            // ota 40% area); the epoch key picks between the two.
            let mut epoch = flow.epoch(&assignment, iter % 2 == 1, start, &weights, &mut prices, &mut neg, seed, &mut halo_dyn);
            stats.warm_epochs += epoch.stats.warm_epochs;
            let wl_nm: i64 = epoch.routes.wires.iter().flatten().map(|s| i64::from(s.rect.w.max(s.rect.h))).sum();
            candidates.push(metadata::Candidate { outer, iteration: iter, wl_um: wl_nm as f64 / 1e3, area_um2: epoch.key.5 / 1e6, clean: epoch.key.0 == 0 });
            // Promotion: simulate only a candidate whose hard count can still
            // beat the incumbent; its spec miss then decides against it.
            if best.as_ref().is_none_or(|b| epoch.key.0 <= b.key.0) {
                flow.score_perf(&mut epoch, &mut stats);
                let k = &epoch.key;
                let point = metadata::ParetoPoint {
                    outer,
                    iteration: iter,
                    v: k.0,
                    residual: epoch.perf.as_ref().map_or(0.0, |p| p.residual),
                    min_beta: epoch.min_beta,
                    theta: k.3,
                    c_tier: k.4,
                    area_um2: k.5 / 1e6,
                };
                if k.0 == 0 && k.1 == 0 {
                    metadata::pareto_insert(&mut pareto, point.clone());
                }
                epochs.push(point);
            }
            for (s, e) in stats.stage_ms.iter_mut().zip(epoch.stats.stage_ms) {
                *s += e;
            }
            if best.as_ref().is_none_or(|b| key_lt(&epoch.key, &b.key)) {
                best = Some(Epoch {
                    iteration: iter,
                    ..epoch
                });
                (stall, current) = (0, true);
            } else {
                stall += 1;
                if stall >= PATIENCE {
                    break;
                }
            }
        }

        // Feasible: stop, converged or not (a feasible incumbent is never
        // escalated). A price held at its cap reads as settled in `drift` but
        // is still binding, so it blocks convergence.
        let feasible = best
            .as_ref()
            .is_some_and(|b| b.key.0 == 0 && b.key.1 == 0 && b.key.3 <= 0.0);
        let stationary = prices.drift() < PRICE_STATIONARY && prices.saturated().is_empty();
        if let Some(stop) = next_action(feasible, stationary, outer + 1 == n_outer) {
            stats.stop = stop;
            stats.converged = stop == StopReason::Converged;
            break;
        }
        // Infeasible stall: move the most-blamed cells first.
        let b = best.as_ref().expect("an epoch ran");
        let blame = flow.blame(b);
        let next = loop {
            let Some(raw) = cellgen::escalate_blamed(&flow.cells.variants, &t.allowed, &assignment, &blame, &tried) else {
                break None;
            };
            let mut next = raw.clone();
            flow.locks.unify(&mut next);
            let fresh = !tried.contains(&next);
            // The raw pick is spent either way, so the loop terminates.
            tried.insert(raw.clone());
            assignment = raw;
            if fresh {
                break Some(next);
            }
        };
        let Some(next) = next else {
            stats.stop = StopReason::EscalationExhausted;
            break;
        };
        stats.variant_escalations += 1;
        assignment = next;
        current = false;
        halo_dyn.fill([0; 4]);
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
    Searched { key: best.key, best, stats, binding, pareto, epochs, candidates }
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
    let wires_of = |classes: &[analog::metadata::NetClass]| -> Vec<pnr_core::Shape> {
        flow.problem.net_classes.iter().filter(|c| classes.contains(&c.class))
            .flat_map(|c| best.routes.wires.get(c.net.0 as usize).into_iter().flatten().copied()).collect()
    };
    // Matched cells: more than one member owns its units.
    let matched: Vec<pnr_core::Rect> = pnr_core::place_macros(&macros, &best.layout).iter()
        .filter(|m| m.units.iter().any(|u| u.owner != m.units[0].owner)).map(|m| m.bbox).collect();
    use analog::metadata::NetClass::{Bias, Ground, Reference, Sensitive};
    let filled = fill::fill(&drawn, &wires_of(&[Ground]), &wires_of(&[Sensitive, Bias, Reference]), &matched, pdk);
    let post_fill = filled.is_some();
    macros.extend(filled);
    let mut metadata = metadata::build(
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
    metadata.binding = s.binding;
    metadata.epochs = s.epochs;
    metadata.post_fill = post_fill;
    metadata.audit = flow.problem.intent.diagnostics.iter().filter(|d| annotator::audit::KINDS.contains(&d.kind)).map(|d| format!("{}: {}", d.kind, d.message)).collect();
    metadata.candidates = s.candidates;
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
    if flow.perf.is_some() && best.perf.is_some() {
        let par = flow.parasitics(&best);
        let fets: Vec<usize> = (0..flow.netlist.devices.len()).filter(|&d| flow.netlist.devices[d].mos_size().is_some()).collect();
        let k = fets.iter().filter(|&&d| par.junction[d].is_none()).count();
        if k > 0 {
            eprintln!("[perf] junction geometry: not drawn-derived for {k} of {} FETs", fets.len());
        }
    }
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
    let field_nets = field_nets(&flow);
    let mut sol = Solution {
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
        diagnostics: flow.problem.intent.diagnostics,
        caps: best.caps,
        blocks: Vec::new(),
        block_ref: hier::BlockRef::default(),
        field_nets,
    };
    // FLOW-10: the certificate is of what ships. Budgets are not re-scored:
    // fill moves no cell and adds no route.
    if post_fill {
        let s = drawn_signoff(&sol, pdk, &verify::ExtractOptions::default());
        sol.stats.drc_hard = s.report.hard_violations.len();
        sol.stats.warnings = s.warnings.len() as u32;
        sol.caps = s.caps;
    }
    sol
}

/// Sensitivity-ranked nets [`field_nets`] adds on top of the
/// structural ones. ponytail: a fixed policy count.
const FIELD_RANKED: usize = 8;

/// The nets to field-solve (PERF-16): every non-rail net of a
/// `Differential` routing row (the compounds' mirrored net pairs) and of a
/// `CommonNode` row, then the [`FIELD_RANKED`] nets of largest
/// Σ_b |d_b|/scale_b over the `GroundC` rows of every sensitivity table
/// (scale_b = |bound|, 1 for a zero bound, as [`perf::miss`]). Until PERF-11
/// (M4) refines the ranking, these rows are the ranking.
fn field_nets(flow: &Flow) -> Vec<String> {
    let rail = |n: pnr_core::NetId| flow.problem.net_classes.get(n.0 as usize).is_some_and(|c| is_rail(c.class));
    let intent = &flow.problem.intent;
    let mut nets: Vec<pnr_core::NetId> = intent
        .compounds
        .iter()
        .flat_map(|c| c.net_pairs.iter().filter(|(x, y)| x != y).flat_map(|&(x, y)| [x, y]))
        .chain(intent.common_nodes.iter().map(|c| c.net))
        .filter(|&n| !rail(n))
        .collect();
    if let Some(p) = flow.perf {
        let mut score: Vec<(pnr_core::NetId, f64)> = Vec::new();
        for row in flow.perf_plan.tables.iter().flat_map(|t| &t.rows) {
            let perf::Param::GroundC { net } = row.param else { continue };
            let s: f64 = p
                .specs
                .iter()
                .zip(&row.d)
                .filter_map(|(spec, d)| d.map(|d| (spec, d.abs())))
                .flat_map(|(spec, d)| [spec.min, spec.max].into_iter().flatten().map(move |b| d / if b == 0.0 { 1.0 } else { b.abs() }))
                .sum();
            match score.iter_mut().find(|(n, _)| *n == net) {
                Some(e) => e.1 += s,
                None => score.push((net, s)),
            }
        }
        score.retain(|&(n, s)| s > 0.0 && !rail(n) && !nets.contains(&n));
        score.sort_by(|a, b| b.1.total_cmp(&a.1));
        nets.extend(score.iter().take(FIELD_RANKED).map(|&(n, _)| n));
    }
    let mut names: Vec<String> = Vec::new();
    for n in nets {
        let name = &flow.net_names[n.0 as usize];
        if !names.contains(name) {
            names.push(name.clone());
        }
    }
    names
}

/// Everything an epoch reads that is fixed for the run.
struct Flow<'a> {
    pdk: &'a Pdk,
    /// The schematic (LVS reference) and its net names (label text).
    netlist: &'a pnr_core::Netlist,
    net_names: Vec<String>,
    /// Placed blocks' LVS cards and the schematic devices they replace (FLOW-11).
    block_ref: hier::BlockRef,
    /// Rules and constraints; placement rules retargeted to cell ids.
    problem: Problem,
    cells: CellSpace,
    /// Matched pairs' live WPE/OSE (PLC-29); also the report's, plus ring wells.
    env: analog::placement::LiveEnvironment,
    /// Placement spacing and grid ([`place_rules`]), fixed for the run.
    rules: gp::PlaceRules,
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
    /// Net weights; [`search`] clones them per start.
    weights: Weights,
    /// A warm epoch's dp schedule ([`Config::warm`]).
    warm: dp::Schedule,
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
    dp_mode: dp::DpMode,
    /// PLC-15 static halo per cell and variant, R0 faces ([`static_halos`]);
    /// empty without an op point or an EM-limited layer.
    halo_st: Vec<Vec<[i32; 4]>>,
    /// Pins per R0 face per cell and variant ([`face_pins`]).
    halo_pins: Vec<Vec<[i32; 4]>>,
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
        wire_ohm_per_um: wire.and_then(|l| pdk.pex_f32(l, "sheet_res_ohm_sq")).filter(|&r| r > 0.0 && width > 0).map(|r| r * 1000.0 / width as f32),
        route_space_nm: wire.and_then(|l| pdk.min_spacing(l.0)).unwrap_or(0),
        dti: opt("dti_max_spacing").zip(opt("dti_width")),
        avt_mv_um: [pos("avt_n_mv_um"), pos("avt_p_mv_um")],
        abeta_pct_um: [pos("abeta_n_pct_um"), pos("abeta_p_pct_um")],
        svt_uv_per_um: pos("svt_uv_per_um"),
        svt_fit: pos("svt_a_uv2_per_um2").zip(pos("svt_b_uv2")),
        vt_tc_uv_per_k: [pos("vt_tc_uv_per_k"), pos("vt_tc_uv_per_k_p")],
        lod_kvth0_mv_um: [pos("lod_kvth0_n_mv_um"), pos("lod_kvth0_p_mv_um")],
        bjt_ka_pct_um: pos("bjt_ka_pct_um"),
        vbe_tc_uv_per_k: pos("vbe_tc_uv_per_k"),
        lattice_nm: cells::builder::cut_lattice(pdk),
        substrate: pnr_core::SubstrateKind::from_key(pdk.cell_str("substrate_kind")),
        epi_nm: pos("epi_thickness_nm").map(|v| v as i32),
        stack: Some(stack),
        min_ring_width_nm: pdk.rule("min_guard_ring_width", 0),
        ecgr_min_width_nm: opt("ecgr_min_width_nm"),
        ecgr_drawable: cells::post_cell::drawable(analog::cell::GuardRingType::Ecgr, pdk),
        hcgr_drawable: cells::post_cell::drawable(analog::cell::GuardRingType::Hcgr, pdk),
        tub_drawable: cells::post_cell::drawable(analog::cell::GuardRingType::Tub { id: 0 }, pdk),
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

/// Placement's process numbers, built once per run after the cells are
/// drawn. Origins snap to the cells' cut lattice so every cut stays on it.
/// Cell pairs are spaced per facing edge (PLC-07): each (cell, variant)'s
/// edge profile ([`gp::spacing::profile`], bulk = its first `:B` pin's net)
/// against the deck's role × role table plus the sidecar's
/// `cell.placement_space`. `fallback` is the old scalar, the widest spacing
/// of any device layer: unmapped layers and same-role pairs the deck leaves
/// open still get it.
/// PLC-13: a matched cell (`class[c]`) owes foreign cells outside its orient
/// set (`locks.orient_of`) the deck's `wpe_clearance_nm` tier and
/// [`gp::spacing::FOREIGN_POLY_NM`].
fn place_rules(pdk: &Pdk, cells: &CellSpace, locks: &dp::locks::Locks, class: &[Option<pnr_core::MatchClass>]) -> gp::PlaceRules {
    use gp::spacing::{profile, Profiles, SpacingTable, DECK_ROLE, N, ROLES};
    use pnr_core::Process;
    let fallback = ["nwell", "diff", "tap", "poly", "nsdm", "psdm", "li"]
        .iter()
        .filter_map(|&r| pdk.layer(r))
        .filter_map(|l| pdk.min_spacing(l.0))
        .max()
        .unwrap_or(0);
    let lattice = cells::builder::cut_lattice(pdk);
    let mut table = SpacingTable::new(pdk, &placement_space(pdk), fallback, lattice);
    use pnr_core::MatchClass::{Exceptional, Minimal, Moderate};
    table.wpe = [Minimal, Moderate, Exceptional].map(|c| analog::matching::class::mos_env(c, pdk).wpe_nm);
    table.foreign_poly = gp::spacing::FOREIGN_POLY_NM;
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let open: Vec<_> = (0..N - 1)
            .flat_map(|i| (i..N - 1).map(move |j| (i, j)))
            .filter(|&(i, j)| DECK_ROLE[i] == DECK_ROLE[j] && table.src[i][j] == gp::spacing::Src::Fallback)
            .map(|(i, j)| format!("{}/{}", ROLES[i], ROLES[j]))
            .collect();
        if !open.is_empty() {
            eprintln!("placement spacing: no deck value, fallback {fallback} nm for {}", open.join(", "));
        }
    });
    let of = cells
        .variants
        .iter()
        .enumerate()
        .map(|(c, s)| {
            s.alternatives
                .iter()
                .map(|m| {
                    let bulk = m.pins.iter().find(|p| p.name.ends_with(":B")).map(|p| p.net);
                    let p = gp::spacing::Profile {
                        matched: class.get(c).copied().flatten(),
                        set: locks.orient_of.get(c).copied().flatten(),
                        ..profile(m, pdk, bulk)
                    };
                    Profiles::orients(&p)
                })
                .collect()
        })
        .collect();
    gp::PlaceRules::new(lattice, table, Profiles { of })
}

/// The utilization floor these cells can reach: `min(u_min, 0.9 · ΣA / Σ(w+g)(h+g))`,
/// each cell (first variant, `w × h`) owing gap `g` on one side per axis. The
/// 0.9 is policy (room for routing halos). `u_min` when there are no cells.
fn u_eff(u_min: f32, cells: &[(i32, i32)], g: i32) -> f32 {
    let (a, padded) = cells.iter().fold((0.0f64, 0.0f64), |(a, p), &(w, h)| {
        let (w, h, g) = (f64::from(w), f64::from(h), f64::from(g));
        (a + w * h, p + (w + g) * (h + g))
    });
    if padded <= 0.0 {
        return u_min;
    }
    u_min.min((0.9 * a / padded) as f32)
}

/// Median over ordered cell pairs `a ≠ b` of the x gap `a`'s R face owes `b`
/// (variant 0, R0); `spacing.fallback` with fewer than 2 profiled cells.
/// ponytail: O(n²) once per topology (~10⁴ for 100 cells); sample if n grows.
fn median_x_gap(r: &gp::PlaceRules) -> i32 {
    let p: Vec<_> = r.profiles.of.iter().filter_map(|v| v.first().map(|o| &o[0])).collect();
    let mut g: Vec<i32> = p
        .iter()
        .enumerate()
        .flat_map(|(i, a)| p.iter().enumerate().filter(move |&(j, _)| j != i).map(move |(_, b)| r.spacing.gap(a, gp::spacing::Face::R, b).min))
        .collect();
    if g.is_empty() {
        return r.spacing.fallback;
    }
    let mid = g.len() / 2;
    *g.select_nth_unstable(mid).1
}

/// Matched cells: a merged group, or a member of a 2-device leaf that emits a
/// `MatchedSet` (annotator emit.rs table).
fn matched_cells(blocks: &[annotator::Block], devices_of: &[Vec<DeviceId>]) -> Vec<bool> {
    use annotator::BlockKind::{CascodePair, CurrentMirror, DiffPair, Load};
    let paired: Vec<DeviceId> = annotator::block::leaves(blocks)
        .into_iter()
        .filter(|l| l.devices.len() == 2 && matches!(l.kind, DiffPair | CurrentMirror | Load | CascodePair))
        .flat_map(|l| l.devices.iter().copied())
        .collect();
    devices_of.iter().map(|m| m.len() > 1 || m.iter().any(|d| paired.contains(d))).collect()
}

/// Each cell's matching class: `Moderate` for a [`matched_cells`] cell, raised
/// to the highest class of any placement batch pairing it
/// ([`analog::RuleBatch::matched_class`]); `None` when unmatched.
fn match_class(problem: &Problem, cells: &CellSpace) -> Vec<Option<pnr_core::MatchClass>> {
    let mut class: Vec<_> =
        matched_cells(&problem.blocks, &cells.devices_of).into_iter().map(|m| m.then_some(pnr_core::MatchClass::Moderate)).collect();
    let p = &problem.placement;
    let mut pairs = Vec::new();
    for b in p.hard.iter().chain(&p.budget).chain(&p.cost) {
        let Some(k) = b.matched_class() else { continue };
        pairs.clear();
        b.matched_pairs(&mut pairs);
        for &(x, y) in &pairs {
            for c in [x, y] {
                if let Some(slot) = class.get_mut(c as usize) {
                    *slot = Some(slot.map_or(k, |v| v.max(k)));
                }
            }
        }
    }
    class
}

/// The sidecar's `cell.placement_space`: `{"role_a,role_b": [nm, "source"]}`.
///
/// # Panics
/// On a malformed entry, naming its key: a typo must not silently drop a rule
/// (an unknown role panics in [`gp::spacing::SpacingTable::new`]).
fn placement_space(pdk: &Pdk) -> Vec<(String, String, i32)> {
    let Some(obj) = pdk.cell.get("placement_space").filter(|v| !v.is_null()) else { return Vec::new() };
    let obj = obj.as_object().expect("cell.placement_space: an object of \"role_a,role_b\": [nm, \"source\"]");
    obj.iter()
        .map(|(k, v)| {
            let (a, b) = k.split_once(',').unwrap_or_else(|| panic!("cell.placement_space.{k}: key is \"role_a,role_b\""));
            let nm = v.get(0).and_then(|n| n.as_i64()).and_then(|n| i32::try_from(n).ok());
            let nm = nm.unwrap_or_else(|| panic!("cell.placement_space.{k}: value is [nm, \"source\"], got {v}"));
            (a.trim().to_owned(), b.trim().to_owned(), nm)
        })
        .collect()
}

/// One [`analog::placement::SymmetryIsland`] per symmetry axis of `reqs.hard`
/// (cell ids) with ≥ 2 distinct cells. `touch_nm` = the largest gap any two
/// members owe across any face, at any variant and orient (a mirror partner
/// drawn MY/MX180 meets with the face its R0 profile calls the same side),
/// + one lattice step.
fn symmetry_islands(reqs: &analog::Requirements<Layout>, rules: &gp::PlaceRules) -> Vec<analog::placement::SymmetryIsland> {
    use gp::spacing::Face;
    let mut pairs = Vec::new();
    for b in &reqs.hard {
        b.mirror_pairs(&mut pairs);
    }
    let mut axes: Vec<u16> = pairs.iter().map(|p| p.2).collect();
    axes.sort_unstable();
    axes.dedup();
    let profiles = |c: u32| rules.profiles.of.get(c as usize).map(|v| v.iter().flat_map(|o| o.iter()).collect::<Vec<_>>()).unwrap_or_default();
    axes.into_iter()
        .filter_map(|ax| {
            let mut cells: Vec<u32> = pairs.iter().filter(|p| p.2 == ax).flat_map(|p| [p.0, p.1]).collect();
            cells.sort_unstable();
            cells.dedup();
            if cells.len() < 2 {
                return None;
            }
            let mut gap = 0;
            for (i, &a) in cells.iter().enumerate() {
                for &b in &cells[i + 1..] {
                    let (pa, pb) = (profiles(a), profiles(b));
                    if pa.is_empty() || pb.is_empty() {
                        gap = gap.max(rules.spacing.fallback);
                    }
                    for p in &pa {
                        for q in &pb {
                            for f in [Face::L, Face::B, Face::R, Face::T] {
                                gap = gap.max(rules.spacing.gap(p, f, q).min);
                            }
                        }
                    }
                }
            }
            let members = cells.iter().map(|&c| pnr_core::ids::Target::Device(DeviceId(c as u16))).collect();
            Some(analog::placement::SymmetryIsland { members, touch_nm: gap + rules.grid })
        })
        .collect()
}

/// `(p0, P)` for `gp::PlaceRules::axis_grid` (PLC-28): the router maps a pair
/// mirror-exactly (`dr` `pair_map`) when `2·axis − p0 ≡ 0 (mod p0)` relative to
/// its frame and the mirror shift is a multiple of every vertical (odd-index)
/// layer's stride, i.e. `axis ≡ p0/2 (mod P)` with `S = lcm(odd strides)` (1
/// if none) and `P = p0·lcm(2, S)/2`. Frame origins are multiples of
/// `p0·lcm(all strides)`, which `P` divides, so absolute = frame-relative mod
/// `P`. `None` when `p0` is not a multiple of `2·lattice` (an axis there would
/// leave the placement lattice).
fn axis_grid(spec: &dr::LatticeSpec, lattice: i32) -> Option<(i32, i32)> {
    fn gcd(a: u32, b: u32) -> u32 {
        if b == 0 { a } else { gcd(b, a % b) }
    }
    let lcm = |a: u32, b: u32| a / gcd(a, b) * b;
    let s = spec.strides.iter().skip(1).step_by(2).fold(1, |acc, &st| lcm(acc, st.max(1)));
    let p = spec.p0 * (lcm(2, s) / 2) as i32;
    (spec.p0 > 0 && spec.p0 % (2 * lattice.max(1)) == 0).then_some((spec.p0, p))
}

/// Per-net weights an epoch places and routes with (FLOW-08 plumbing: a
/// search owns its copy; PERF-15/GAP-08 will update it between epochs).
#[derive(Clone)]
pub(crate) struct Weights {
    /// Placement HPWL weight ([`gp::net_weights`]).
    place: Vec<f32>,
    /// dr's parasitic weight ([`route_weights`]).
    route: Vec<f32>,
}

/// dr prices parasitics only on nets something budgets (a C budget or a
/// sensitivity row), `place` scaled to [0, 1]; every other net 0.
fn route_weights(place: &[f32], classes: &[analog::metadata::NetClassification], sens: &[(pnr_core::NetId, f32)]) -> Vec<f32> {
    let budgeted = |n: usize| classes.get(n).is_some_and(|c| c.c_budget_af.is_some()) || sens.iter().any(|(s, _)| usize::from(s.0) == n);
    let w: Vec<f32> = place.iter().enumerate().map(|(n, &w)| if budgeted(n) { w } else { 0.0 }).collect();
    let top = w.iter().copied().fold(0.0f32, f32::max);
    w.iter().map(|&x| if top > 0.0 { x / top } else { 0.0 }).collect()
}

/// One scored epoch.
struct Epoch {
    key: LexKey,
    /// Smallest β over the bounds after [`Flow::score_perf`] ([`robust::min_beta`]).
    min_beta: Option<f64>,
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
    /// The code `dp::place_sp` returned under [`dp::DpMode::Sp`] (PLC-10):
    /// FLOW-08's warm start resumes from it (`dp::Start::Warm`).
    #[allow(dead_code)] // read once FLOW-08 lands
    tree: Option<dp::sp::Tree>,
}

impl Flow<'_> {
    /// Place → route → signoff at `assignment`, scored. `reshape` lets dp
    /// swap variants; without it the assignment is kept as drawn. `start`
    /// `None` = cold (gp, then [`dp::Schedule::cold`]); `Some(incumbent)` =
    /// warm: the incumbent's variants and positions annealed at
    /// [`Flow::warm`], no gp. Routing history accumulates once per epoch.
    #[allow(clippy::too_many_arguments)]
    fn epoch(
        &self,
        assignment: &[u16],
        reshape: bool,
        start: Option<&Layout>,
        w: &Weights,
        prices: &mut gp::Prices,
        neg: &mut gr::Negotiation,
        seed: u64,
        halo_dyn: &mut [[i32; 4]],
    ) -> Epoch {
        let unified = {
            let mut a = start.map_or(assignment, |l| &l.variant[..]).to_vec();
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
            rules: &self.rules,
            net_weight: &w.place,
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
        // Its own stream (AP-19): gp and dp drawing the same sequence correlate their moves.
        let dp_seed = seed ^ 0xD1B5_4A32_D192_ED03;
        let dp_variants = if reshape { &cells.variants[..] } else { &[] };
        let mut tree = None;
        let cold;
        let (mut layout, place_report, dp_stats) = if self.dp_mode == dp::DpMode::Sp {
            // ponytail: FLOW-08 warm starts cover the flat path only; the SP
            // path always starts cold until PLC-10 step 1 (dp::Start::Warm).
            let coarse = (self.gp_mode != GpMode::Constructive).then(|| gp::place(&inp, prices, seed).0);
            lap(0);
            let sp_in = dp::PlaceInput {
                macros: &macros,
                variants: dp_variants,
                assignment,
                reqs: placement,
                fixed: &cells.fixed,
                blocks: &cells.groups,
                rules: self.rules.clone(),
                locks: &self.locks,
                halo: &self.halo_st,
                halo_dyn: &*halo_dyn,
                net_weight: &w.place,
                n_axes: self.problem.axis_count,
                power_uw: &cells.power,
                units: cells.units.clone(),
            };
            let start = coarse.as_ref().map_or(dp::Start::Constructive, dp::Start::Cold);
            let (l, t, rep, st) = dp::place_sp(&sp_in, start, dp::Schedule::cold(), prices, dp_seed);
            tree = Some(t);
            (l, rep, st)
        } else {
            let (coarse, schedule) = match start {
                Some(inc) => (inc, self.warm),
                None => {
                    cold = gp::place(&inp, prices, seed).0;
                    cold.debug_check("gp::place");
                    (&cold, dp::Schedule::cold())
                }
            };
            lap(0);
            dp::place(coarse, &macros, dp_variants, placement, &cells.fixed, &self.locks, prices, &self.rules, &w.place, dp_seed, schedule)
        };
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
        let place = geometry::placement_metrics(&macros, &layout, lattice, &self.rules, placement, &self.locks, &cells.groups);
        debug_assert_eq!(place.lattice_off, 0, "dp::place: cell origin off the cut lattice");

        // Guard rings enclose placed cells, so they are drawn now, before routing.
        let mut rings = cells::post_cell::guard_rings(&layout, &cells.guard_rings, self.pdk, ring_cut_ohm(self.pdk));
        // Same-bulk PMOS cells facing each other share one well.
        // Never across REL-16's forbidden pairs (CELL-22).
        let flags = cell_flags(&self.problem.intent, &self.cells.devices_of);
        let bridges = cells::post_cell::well_bridges(&gr::place_macros(&macros, &layout), &rings, self.pdk, &|i, j| cells::post_cell::may_share_well(flags[i], flags[j]));
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
                plates: self.plate_sets(&placed),
                aggressor_weight: analog::routing::CouplingBudget::default_weights(&self.problem.net_classes, self.netlist.nets.len()),
                stack: Some(self.stack),
                pin_share: macros.iter().map(pnr_core::pin_shares).collect(),
                n_nets: self.netlist.nets.len(),
                net_weight: w.route.clone(),
                ..self.d_router.cfg.clone()
            },
        };
        // AT-34: a diode reroute replays from here, so history counts once.
        let before = neg.clone();
        let (mut routes, mut route_report, mut route_stats) =
            router.route(&pins, &placed, &rings, routing, layers, &self.cuts, neg);
        lap(3);
        // Antenna nets the jumper could not fix get a diode each, routed in as
        // a fixed cell; its shape on the deck's credited diode layer joins the
        // net's routes (the rule's credit, `Stack::diode`).
        let ground = self.problem.net_classes.iter().find(|c| c.class == analog::metadata::NetClass::Ground).map(|c| c.net);
        let diodes = elaborate::antenna_diodes(self.pdk, routing, &routes, &placed, &rings, ground, self.rules.spacing.fallback);
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
            *neg = before;
            (routes, route_report, route_stats) = router.route(&pins, &placed, &rings, routing, layers, &self.cuts, neg);
            lap(4);
            let marked = !marks.is_empty();
            for (net, shape) in marks {
                if let Some(w) = routes.wires.get_mut(net.0 as usize) {
                    w.push(shape);
                }
            }
            // dr scored the routes before the markers joined them: re-derive
            // its rule rows on the routes that ship (RTE-23).
            if marked {
                let (hard, budget) = gr::analog_tiers(&routes, routing);
                route_report.hard_violations.retain(|v| !v.is_batch_row());
                route_report.budget_violations.retain(|v| !v.is_batch_row());
                route_report.hard_violations.extend(hard);
                route_report.budget_violations.extend(budget);
            }
        }
        let netlist = with_extra(self.netlist, &extra);

        // Measure: signoff over the drawn geometry, budget residuals over the result.
        let mut shapes = geometry::collect(&macros, &layout, &routes);
        shapes.extend(rings.iter().flat_map(|r| r.shapes.iter().copied()));
        if let Some(l) = pnr_core::Process::layer(self.pdk, "nwell") {
            geometry::merge_rects(&mut shapes, l);
        }
        // The epoch is scored by the same DRC/ERC/LVS gate as the final result.
        let mut labelled = placed;
        labelled.extend(rings.iter().cloned());
        let mut signoff = signoff_shapes(&self.intent, &shapes, &labelled, &self.net_names, &netlist, Some(&self.fold), &self.block_ref, self.pdk);
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
        budgets.add_routing(&[Box::new(self.common_nodes(&layout))], &routes);

        let c = signoff_c_tier(&signoff, &self.net_names, &self.problem.net_classes, self.perf_rows);
        // PLC-15: this epoch's congestion steps the next epoch's halo.
        update_dyn_halo(halo_dyn, &layout, &route_stats.congestion, &self.halo_pins, dr::lattice_spec(&self.d_router.cfg).p0, self.rules.grid);
        let (key, stats) = epoch_score(&place_report, &route_report, &signoff, &budgets, c, layout.footprint_nm2());
        lap(7);
        let stats = RunStats { place, dp: dp_stats, stage_ms: ms, warm_epochs: u32::from(start.is_some()), ..stats };
        Epoch {
            key,
            min_beta: None,
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
            tree,
        }
    }

    /// Per cell, how much an epoch's failures point at it: +1 per id a
    /// violated placement hard or budget rule touches ([`analog::RuleBatch::violating_ids`],
    /// cell ids), +1 per DRC finding (label-free, cells + routes + rings)
    /// inside the cell's placed bbox. [`cellgen::escalate_blamed`] moves the
    /// most-blamed cell first.
    fn blame(&self, e: &Epoch) -> Vec<u32> {
        let n = e.layout.x.len();
        let mut blame = vec![0u32; n];
        let mut ids = Vec::new();
        for b in self.problem.placement.hard.iter().chain(&self.problem.placement.budget) {
            b.violating_ids(&e.layout, &mut ids);
        }
        for i in ids {
            if let Some(c) = blame.get_mut(i as usize) {
                *c += 1;
            }
        }
        let macros = cellgen::realize(&self.cells.variants, &e.layout.variant);
        let mut shapes = geometry::collect(&macros, &e.layout, &e.routes);
        shapes.extend(e.rings.iter().flat_map(|r| r.shapes.iter().copied()));
        let placed = pnr_core::place_macros(&macros, &e.layout);
        for f in verify::drc(&shapes, &[], self.pdk) {
            let inside = |r: &pnr_core::Rect| (i64::from(r.x)..=i64::from(r.x) + i64::from(r.w)).contains(&f.x) && (i64::from(r.y)..=i64::from(r.y) + i64::from(r.h)).contains(&f.y);
            for (c, m) in blame.iter_mut().zip(&placed) {
                if inside(&m.bbox) {
                    *c += 1;
                }
            }
        }
        blame
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

/// `netlist` with dr's inserted devices appended (borrowed when there are none).
fn with_extra<'a>(netlist: &'a pnr_core::Netlist, extra: &[pnr_core::Device]) -> std::borrow::Cow<'a, pnr_core::Netlist> {
    if extra.is_empty() {
        return std::borrow::Cow::Borrowed(netlist);
    }
    let mut n = netlist.clone();
    n.devices.extend(extra.iter().cloned());
    std::borrow::Cow::Owned(n)
}

/// Per device `[AS, AD, PS, PD]` per SPICE instance from the placed cells' `Figures.sd` (CELL-19):
/// owner `o` of cell `c` is `devices_of[c][o]`; S/D entries summed per device, ÷ the device's `m`
/// (the card has `m` copies), nm² → µm² ÷ 1e6, nm → µm ÷ 1e3. `None` for a device no figure names.
fn junctions(placed: &[Macro], devices_of: &[Vec<DeviceId>], netlist: &pnr_core::Netlist) -> Vec<Option<[f64; 4]>> {
    let mut out: Vec<Option<[f64; 4]>> = vec![None; netlist.devices.len()];
    for (m, members) in placed.iter().zip(devices_of) {
        for &(o, t, area, perim) in &m.figures.sd {
            let Some(&d) = members.get(usize::from(o)) else { continue };
            let Some(slot) = out.get_mut(usize::from(d.0)) else { continue };
            let j = slot.get_or_insert([0.0; 4]);
            let k = usize::from(t == "D");
            j[k] += area as f64;
            j[2 + k] += perim as f64;
        }
    }
    for (j, dev) in out.iter_mut().zip(&netlist.devices) {
        if let Some(j) = j {
            let m = f64::from(dev.mos_size().map_or(1, |s| s.m).max(1));
            *j = [j[0] / m / 1e6, j[1] / m / 1e6, j[2] / m / 1e3, j[3] / m / 1e3];
        }
    }
    out
}

/// Per device the drawn gate's R, Ω, from the placed cells' `Figures.gate_ohm` (owner as in
/// [`junctions`]); the owner's whole gate over all copies, so not ÷ m. `None` where none is drawn.
fn gate_ohms(placed: &[Macro], devices_of: &[Vec<DeviceId>], n: usize) -> Vec<Option<f64>> {
    let mut out = vec![None; n];
    for (m, members) in placed.iter().zip(devices_of) {
        for &(o, r) in &m.figures.gate_ohm {
            if let Some(slot) = members.get(usize::from(o)).and_then(|d| out.get_mut(usize::from(d.0))) {
                *slot = Some(f64::from(r));
            }
        }
    }
    out
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

    /// Per net, `(device, terminal, pin rect)` of every pin of `placed` (cells
    /// indexed like `devices_of`) that names a member terminal
    /// ([`Flow::member_pin`]); a pin on a net outside the netlist is dropped.
    fn pins_by_net(&self, placed: &[Macro]) -> Vec<Vec<(DeviceId, String, pnr_core::Rect)>> {
        let mut on_net: Vec<Vec<(DeviceId, String, pnr_core::Rect)>> = vec![Vec::new(); self.netlist.nets.len()];
        for (m, members) in placed.iter().zip(&self.cells.devices_of) {
            for p in &m.pins {
                let Some((d, t)) = self.member_pin(members, &p.name) else { continue };
                if let Some(v) = on_net.get_mut(p.net.0 as usize) {
                    v.push((d, t.to_string(), p.at));
                }
            }
        }
        on_net
    }

    /// What the epoch's layout adds to the schematic: its extracted C, each
    /// device terminal's routed branch R (parallel pins of one terminal
    /// combine), and each device's mean LOD stress over its drawn fingers.
    /// Every per-device vector spans `netlist ∪ epoch.extra` (inserted devices get `None`/empty), and
    /// AS/AD/PS/PD and the gate R come from the placed cells' `Figures` (CELL-19).
    fn parasitics(&self, epoch: &Epoch) -> perf::Parasitics {
        let n = self.netlist.devices.len() + epoch.extra.len();
        let placed = gr::place_macros(&cellgen::realize(&self.cells.variants, &epoch.layout.variant), &epoch.layout);
        let pins = self.pins_by_net(&placed);
        let mut conductance: Vec<Vec<(String, f32)>> = vec![Vec::new(); n];
        for (net, list) in pins.iter().enumerate() {
            let shapes = epoch.routes.wires.get(net).map_or(&[][..], Vec::as_slice);
            if list.len() < 2 || shapes.is_empty() {
                continue;
            }
            let rects: Vec<pnr_core::Rect> = list.iter().map(|&(_, _, r)| r).collect();
            for ((d, t, _), r) in list.iter().zip(self.stack.terminal_resistance_ohm(shapes, &rects)) {
                let Some(r) = r.filter(|&r| r > 0.0) else { continue };
                let v = &mut conductance[usize::from(d.0)];
                match v.iter_mut().find(|(n, _)| n == t) {
                    Some((_, g)) => *g += 1.0 / r,
                    None => v.push((t.clone(), 1.0 / r)),
                }
            }
        }
        let series = conductance.into_iter().map(|v| v.into_iter().map(|(t, g)| (t, 1.0 / g)).collect()).collect();
        let lod_inv_um = (0..self.netlist.devices.len())
            .map(|d| {
                let (s, k) = epoch.layout.units.of_device(&epoch.layout, pnr_core::DeviceId(d as u16)).filter(|u| u.lod.is_finite()).fold((0.0f32, 0u32), |(s, k), u| (s + u.lod, k + 1));
                (k > 0).then(|| s / k as f32)
            })
            .chain(std::iter::repeat(None).take(epoch.extra.len()))
            .collect();
        let mut junction = junctions(&placed, &self.cells.devices_of, self.netlist);
        junction.resize(n, None);
        let gate_ohm = gate_ohms(&placed, &self.cells.devices_of, n);
        perf::Parasitics { caps: epoch.caps.clone(), series, lod_inv_um, extracted: true, gate_offset_v: Vec::new(), junction, gate_ohm }
    }

    /// RTE-20: each capacitor `Unitization` of three or more members (EXT-19's
    /// sets) as placed: `top` the members' `P` net, `bits` the `(N net, units)`
    /// of every member whose `N` is no rail (the terminated unit's is),
    /// `c_unit_af` a one-unit member's `c_af` (`NAN` without one), `array` the
    /// bbox of the cell drawing the members.
    fn plate_sets(&self, placed: &[Macro]) -> Vec<analog::routing::PlateSet> {
        let rail = |n: pnr_core::NetId| self.problem.net_classes.iter().any(|c| c.net == n && is_rail(c.class));
        let term = |d: DeviceId, t: &str| self.netlist.devices[d.0 as usize].terminals.iter().find(|(n, _)| n == t).map(|&(_, n)| n);
        let mut out = Vec::new();
        for u in self.problem.constraints.unitization.iter().filter(|u| u.device_type == pnr_core::DeviceKind::Capacitor && u.devices.len() >= 3) {
            let Some(top) = term(u.devices[0], "P") else { continue };
            let Some(c) = self.cells.devices_of.iter().position(|m| !m.is_empty() && m.iter().all(|d| u.devices.contains(d))) else { continue };
            let Some(array) = placed.get(c).map(|m| m.bbox) else { continue };
            let units = |i: usize| u32::from(u.dev_nf.get(i).copied().unwrap_or(1));
            let bits: Vec<(pnr_core::NetId, u32)> = u.devices.iter().enumerate().filter_map(|(i, &d)| term(d, "N").filter(|&n| !rail(n)).map(|n| (n, units(i)))).collect();
            let c_unit_af = u
                .devices
                .iter()
                .enumerate()
                .filter(|&(i, _)| units(i) == 1)
                .find_map(|(_, &d)| self.netlist.devices[d.0 as usize].params.iter().find(|(k, _)| k == "c_af").map(|&(_, v)| v as f32))
                .unwrap_or(f32::NAN);
            out.push(analog::routing::PlateSet { top, bits, c_unit_af, array });
        }
        out
    }

    /// Each extracted common node (EXT-24 `Intent.common_nodes`), with its
    /// halves' pins on the node and the net's other pins (feeds) as placed,
    /// budgeted `ΔR ≤ (allowance − placement spend) / I_D` from the first
    /// pair's `MatchedSet` ledger.
    fn common_nodes(&self, layout: &Layout) -> analog::routing::CommonNodes {
        let mut left = Vec::new();
        for b in &self.problem.placement.budget {
            b.offset_allowances(layout, &mut left);
        }
        let placed = gr::place_macros(&cellgen::realize(&self.cells.variants, &layout.variant), layout);
        let on_net = self.pins_by_net(&placed);
        let mut nodes = Vec::new();
        for req in &self.problem.intent.common_nodes {
            let (Some(&a), Some(&b)) = (req.a.first(), req.b.first()) else { continue };
            let list = &on_net[req.net.0 as usize];
            let term = format!("{:?}", req.term);
            let pins = |ds: &[DeviceId]| list.iter().filter(|p| ds.contains(&p.0) && p.1 == term).map(|p| p.2).collect::<Vec<_>>();
            let feeds = list.iter().filter(|p| !req.a.contains(&p.0) && !req.b.contains(&p.0)).map(|p| p.2).collect();
            let i_ua = self.id_ua.get(a.0 as usize).copied().flatten().map(|i| i.abs() as f32).filter(|&i| i > 0.0);
            let max_delta_ohm = common_node_ohm(&left, a, b, i_ua);
            // RTE-17 step 4: EXT-24's StarReq/KelvinReq (`intent.stars`/`kelvins`)
            // are not mapped yet (a StarReq supersedes the net's node, star = true).
            nodes.push(analog::routing::CommonNode { net: req.net, groups: vec![pins(&req.a), pins(&req.b)], feeds, max_delta_ohm, star: false });
        }
        let joins = dr::joins(&self.layers, &self.cuts, self.d_router.cfg.pin_access);
        analog::routing::CommonNodes { nodes, stack: self.stack, halo_nm: self.d_router.cfg.pitch, joins }
    }

    /// Each recognised matched pair's surroundings on the placed geometry,
    /// `rings`' n-wells joining the cells' ([`live_environment`]).
    fn environment(&self, layout: &Layout, rings: &[Macro]) -> analog::placement::Environment {
        use pnr_core::Process;
        let ring_wells: Vec<pnr_core::Rect> = self
            .pdk
            .layer("nwell")
            .map(|nwell| rings.iter().flat_map(|m| &m.shapes).filter(|s| s.layer == nwell).map(|s| s.rect).collect())
            .unwrap_or_default();
        analog::placement::Environment(self.env.surroundings_with(layout, &ring_wells))
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
            perf::evaluate(&with_extra(self.netlist, &epoch.extra), &self.parasitics(epoch), p, self.perf_active).unwrap_or_else(|e| {
                eprintln!("[perf] {e}");
                stats.sim_failures += 1;
                unknown()
            })
        };
        let plan = self.perf_plan;
        let stats_b = robust::bound_stats(&plan.tables, &plan.sigma_v, &result, &p.specs, &[], &[]);
        (epoch.key.1, epoch.key.2) = robust::key_tiers(&stats_b, &result, &p.specs, plan.beta_key);
        epoch.min_beta = robust::min_beta(&stats_b);
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

/// `(|V|, failed bounds, spec shortfall, Θ, C tier, footprint nm²)`, compared
/// by [`key_lt`]: no parasitic gain buys past a budget residual, no budget
/// slack past a failed bound or a β short of 3, nothing past a hard violation. V counts violated hard rules
/// ([`metadata::MetadataReport::hard_violated`]), the stages' own non-batch
/// rows, and signoff errors (deck warnings are never in that report). Θ is
/// [`metadata::MetadataReport::theta`] plus dr's own non-batch budget rows, all
/// in milli-budgets. The spec tiers are [`robust::key_tiers`] of the post-layout
/// simulation (PERF-14: on β when every schematic bound has a σ_f, else on the
/// Σ normalised miss; `(0, 0)` without performance scoring); the C tier is [`c_tier`] over
/// signoff's extracted matrix, not its total (`Report::cost`), which on ota is
/// 79 % supply-related (AV-06).
type LexKey = (usize, u32, f64, f64, f32, f64);

/// Relative [`c_tier`] difference read as a tie, which area then breaks.
///
/// ponytail: a flat 2%, calibrated on the seed-to-seed spread of total
/// extracted C and not re-measured for the tier; a declared spec on C or area
/// would replace it.
const C_TIE: f32 = 0.02;

/// `a` beats `b`: `|V|`, failed bounds, spec shortfall, Θ lexicographically, then the C tier —
/// except that C within [`C_TIE`] is a tie decided by footprint. A feasible
/// optimum is a vector (area, C, …) and a scalarisation must be a declared
/// policy (Graeb 2007 ch.1); this is ours. Not transitive inside a C band;
/// callers only ever compare a candidate against the incumbent. A NaN tier
/// reads as +∞, so it loses to any finite value and never sticks as incumbent.
fn key_lt(a: &LexKey, b: &LexKey) -> bool {
    let nan_last = |x: f64| if x.is_nan() { f64::INFINITY } else { x };
    let c = |k: &LexKey| if k.4.is_nan() { f32::INFINITY } else { k.4 };
    let (a4, b4) = (c(a), c(b));
    let head = |k: &LexKey| (k.0, k.1, nan_last(k.2), nan_last(k.3));
    if head(a) != head(b) {
        return head(a) < head(b);
    }
    if (a4 - b4).abs() <= C_TIE * a4.abs().min(b4.abs()) {
        nan_last(a.5) < nan_last(b.5)
    } else {
        a4 < b4
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
    let key = lex_key(place, route, &signoff.report, budgets, c_tier, footprint_nm2);
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

/// The [`LexKey`] of one epoch before simulation: |V| over violated hard
/// rules, the stages' own non-batch rows and signoff errors (`lvs-coverage/`
/// rows excluded), Θ over the budgets plus dr's own non-batch budget rows;
/// spec tiers `(0, 0.0)`.
fn lex_key(
    place: &Report,
    route: &Report,
    signoff: &Report,
    budgets: &metadata::MetadataReport,
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
    (v, 0, 0.0, theta, c_tier, footprint_nm2)
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
    let w = |name: &str| -> f64 {
        let Some(id) = names.iter().position(|n| n == name) else { return 0.0 };
        if rows.is_empty() {
            let signal = classes
                .iter()
                .any(|c| usize::from(c.net.0) == id && !is_rail(c.class));
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

/// The [`gp::spacing::Face`] (L, B, R, T) nearest pin rect `r`'s centre in
/// `m`'s R0 frame; ties go to the earlier face.
fn nearest_face(m: &Macro, r: pnr_core::geom::Rect) -> usize {
    let (cx, cy, b) = (r.x + r.w / 2, r.y + r.h / 2, m.bbox);
    let d = [cx - b.x, cy - b.y, b.x + b.w - cx, b.y + b.h - cy];
    (0..4).min_by_key(|&f| d[f]).expect("four faces")
}

/// PLC-15 static halo per cell, variant and R0 face: a pin drawing
/// `I_p = table(pin) × pin_shares` µA past what a minimum-width wire carries
/// (`I_p > j·w_min`) reserves the extra width, `round_up(⌈I_p/j⌉ − w_min)`,
/// on its nearest face. Only width beyond one track: minimum-width signals
/// route over cells (policy; LAMP's static term reserves every terminal
/// wire's full width, L4012–4024). Empty `pin_ua` = no halo.
fn static_halos(variants: &[gp::VariantSpace], pin_ua: &[Vec<(String, Option<i32>)>], j_ua_per_nm: f32, w_min: i32, lattice: i32) -> Vec<Vec<[i32; 4]>> {
    variants
        .iter()
        .enumerate()
        .map(|(c, v)| {
            v.alternatives
                .iter()
                .map(|m| {
                    let mut h = [0; 4];
                    let Some(table) = pin_ua.get(c) else { return h };
                    for (pin, share) in m.pins.iter().zip(pnr_core::pin_shares(m)) {
                        let Some(ua) = table.iter().find(|(n, _)| *n == pin.name).and_then(|t| t.1) else { continue };
                        let i = ua as f32 * share;
                        if j_ua_per_nm > 0.0 && i > j_ua_per_nm * w_min as f32 {
                            h[nearest_face(m, pin.at)] += round_up((i / j_ua_per_nm).ceil() as i32 - w_min, lattice);
                        }
                    }
                    h
                })
                .collect()
        })
        .collect()
}

/// Pins per R0 face, per cell and variant ([`nearest_face`]): `n_f` of the
/// congestion halo.
fn face_pins(variants: &[gp::VariantSpace]) -> Vec<Vec<[i32; 4]>> {
    variants
        .iter()
        .map(|v| {
            v.alternatives
                .iter()
                .map(|m| {
                    let mut n = [0; 4];
                    for pin in &m.pins {
                        n[nearest_face(m, pin.at)] += 1;
                    }
                    n
                })
                .collect()
        })
        .collect()
}

/// Congestion halo target for one face: `round_up((d − 1)⁺ · n · p0)`, one
/// track per pin per unit of demand past capacity (PLC-15, policy).
fn halo_target(d: f32, n: i32, p0: i32, lattice: i32) -> i32 {
    round_up(((d - 1.0).max(0.0) * n as f32 * p0 as f32).ceil() as i32, lattice)
}

/// One smoothing step toward `target`: `round_up(0.5·h + 0.5·target)`, capped
/// at `4·p0`. β = 0.5 and the cap are policy: BAL2-19 warns that naive
/// raise/lower updates "could oscillate indefinitely" (L9401–9405).
fn halo_step(h: i32, target: i32, p0: i32, lattice: i32) -> i32 {
    round_up(((h as f32 + target as f32) * 0.5).ceil() as i32, lattice).min(4 * p0)
}

/// Steps each placed face's congestion halo ([`halo_step`]) toward the
/// demand `d_f` read in the `p0`-wide band just outside it (max over the
/// regions overlapping the band, 0 if none); `n_pins` per cell and variant,
/// R0 ([`face_pins`]), turned to the placed frame.
fn update_dyn_halo(halo: &mut [[i32; 4]], l: &Layout, congestion: &[(pnr_core::geom::Rect, f32)], n_pins: &[Vec<[i32; 4]>], p0: i32, lattice: i32) {
    use pnr_core::geom::Rect;
    for (c, h) in halo.iter_mut().enumerate() {
        let (x, y, hw, hh) = (l.x[c], l.y[c], l.hw[c], l.hh[c]);
        let bands = [
            Rect { x: x - hw - p0, y: y - hh, w: p0, h: 2 * hh },
            Rect { x: x - hw, y: y - hh - p0, w: 2 * hw, h: p0 },
            Rect { x: x + hw, y: y - hh, w: p0, h: 2 * hh },
            Rect { x: x - hw, y: y + hh, w: 2 * hw, h: p0 },
        ];
        let v = usize::from(l.variant[c]);
        let n = gp::spacing::oriented_faces(n_pins.get(c).and_then(|p| p.get(v)).copied().unwrap_or_default(), l.orient[c]);
        for f in 0..4 {
            let b = bands[f];
            let d = congestion
                .iter()
                .filter(|(r, _)| r.x < b.x + b.w && b.x < r.x + r.w && r.y < b.y + b.h && b.y < r.y + r.h)
                .map(|&(_, d)| d)
                .fold(0.0f32, f32::max);
            h[f] = halo_step(h[f], halo_target(d, n[f].max(1), p0, lattice), p0, lattice);
        }
    }
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

/// Sidecar `em_front_row_cuts` (default `true`): EM counts a via group by its
/// front row (REL-12), in `em_rules` and the detailed router alike.
fn em_front_row(pdk: &Pdk) -> bool {
    pdk.cell.get("em_front_row_cuts").and_then(serde_json::Value::as_bool).unwrap_or(true)
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
    front_row: bool,
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
        .map(|k| analog::routing::Electromigration { net: pnr_core::NetId(k as u16), limits, stack, front_row })
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

/// Per placed cell, its members' substrate tags OR-ed (EXT-23, for REL-16's
/// [`cells::post_cell::may_share_well`]); a device in no cell is skipped.
fn cell_flags(intent: &analog::intent::Intent, devices_of: &[Vec<DeviceId>]) -> Vec<cells::post_cell::CellFlags> {
    use analog::intent::Inject::{MinorityElectron, MinorityHole};
    let mut flags = vec![cells::post_cell::CellFlags::default(); devices_of.len()];
    let cell = |d: DeviceId| devices_of.iter().position(|m| m.contains(&d));
    for a in &intent.aggressors {
        if let Some(k) = cell(a.device) {
            flags[k].noisy = true;
            flags[k].injector |= matches!(a.inject, MinorityElectron | MinorityHole);
        }
    }
    for v in &intent.victims {
        if let Some(k) = cell(v.device) {
            flags[k].sensitive = true;
        }
    }
    flags
}

#[cfg(test)]
mod cell_flags_tests {
    use analog::intent::{Aggressor, Inject, Intent, Victim};
    use cells::post_cell::CellFlags;
    use pnr_core::DeviceId;

    /// CELL-22: a cell's tags are the OR over its members; a device in no cell adds nothing.
    #[test]
    fn cell_flags_or_over_members() {
        let intent = Intent {
            aggressors: vec![Aggressor { device: DeviceId(1), inject: Inject::MinorityHole, reason: "" }, Aggressor { device: DeviceId(9), inject: Inject::Switching, reason: "" }],
            victims: vec![Victim { device: DeviceId(2), weight: 1.0, reason: "" }],
            ..Default::default()
        };
        let flags = super::cell_flags(&intent, &[vec![DeviceId(0), DeviceId(1)], vec![DeviceId(2)]]);
        assert_eq!(flags, vec![CellFlags { injector: true, noisy: true, sensitive: false }, CellFlags { sensitive: true, ..Default::default() }]);
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
        return Bias::uniform(cfg.device_power_uw.clone());
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

/// The matched pairs (2-device `DiffPair`/`CurrentMirror`/`Load` leaves) and
/// every cell variant's n-well and diff rects, for the live WPE/OSE batch.
/// Ranges: the deck's moderate WPE clearance and its minimal-tier LOD moat
/// extension. No `nwell` or `diff` layer: no pairs (nothing measurable).
fn live_environment(problem: &Problem, cells: &CellSpace, pdk: &Pdk) -> analog::placement::LiveEnvironment {
    use annotator::BlockKind::{CurrentMirror, DiffPair, Load};
    use pnr_core::Process;
    let mut env = analog::placement::LiveEnvironment {
        pairs: Vec::new(),
        geo: std::sync::Arc::default(),
        wpe_min_nm: analog::matching::class::mos_env(pnr_core::MatchClass::Moderate, pdk).wpe_nm as f32,
        ose_range_nm: pdk.tier("lod_moat_ext_nm", pnr_core::MatchClass::Minimal).unwrap_or(0) as f32,
    };
    let (Some(nwell), Some(diff)) = (pdk.layer("nwell"), pdk.layer("diff")) else { return env };
    let on = |m: &Macro, layer| m.shapes.iter().filter(|s| s.layer == layer).map(|s| s.rect).collect::<Vec<_>>();
    let alts = || cells.variants.iter().map(|v| &v.alternatives);
    env.geo = std::sync::Arc::new(analog::placement::EnvGeo {
        bbox: alts().map(|a| a.iter().map(|m| m.bbox).collect()).collect(),
        wells: alts().map(|a| a.iter().map(|m| on(m, nwell)).collect()).collect(),
        diffs: alts().map(|a| a.iter().map(|m| on(m, diff)).collect()).collect(),
    });
    let cell = |d: DeviceId| cells.devices_of.iter().position(|m| m.contains(&d)).map(|c| c as u16);
    for leaf in annotator::block::leaves(&problem.blocks) {
        let &[a, b] = leaf.devices.as_slice() else { continue };
        if !matches!(leaf.kind, DiffPair | CurrentMirror | Load) {
            continue;
        }
        // A member in no cell reads no OSE (`∞`), as before.
        env.pairs.push((a, b, cell(a).unwrap_or(u16::MAX), cell(b).unwrap_or(u16::MAX)));
    }
    env
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
        blocks: &[(Vec<DeviceId>, Macro)],
    ) -> Self {
        let cellgen::Cells {
            spaces,
            cell_of,
            devices_of,
            aspect_missed,
        } = cellgen::enumerate_folded(netlist, injected, &problem.constraints, pdk, merge_distinct_gates, fold, &problem.net_classes, blocks);
        let note = ("MatchClass", "no variant meets the aspect limit");
        if aspect_missed > 0 && !problem.missing.contains(&note) {
            problem.missing.push(note);
        }
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
        // A block member's ring was drawn in the block (FLOW-11).
        let mut guard_rings = analog::Constraints::default();
        let in_block = |d: DeviceId| blocks.iter().any(|b| b.0.contains(&d));
        for r in problem.constraints.guard_rings.iter().filter(|r| !in_block(r.device)) {
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
/// PEX is analytical over merged metal. ponytail: [`Solution::field_nets`]
/// are not field-solved here — measured (`bench --pex-cal`, PERF-16) at
/// 10³–10⁴× the analytical time and 3–10× below magic's ground C; pass
/// them to [`signoff_with`] once the solve reads near magic. Step 3
/// deferred pending an owner decision (docs/plans/cards/m2-perf-5.md).
#[must_use]
pub fn signoff(sol: &Solution, pdk: &Pdk) -> verify::Signoff {
    signoff_with(sol, pdk, &verify::ExtractOptions::default())
}

/// [`signoff`] with explicit extraction options; `field_solve` is cut to the
/// nets that carry a label (an unlabelled name would refuse the whole solve).
#[must_use]
pub fn signoff_with(sol: &Solution, pdk: &Pdk, opts: &verify::ExtractOptions) -> verify::Signoff {
    let mut s = drawn_signoff(sol, pdk, opts);
    if let Some(op) = &sol.op {
        let probe = sol.metadata.bias.as_ref().is_some_and(|b| b.probe);
        s.report.hard_violations.extend(reliability::voltage_findings(&sol.netlist, op, &pdk.fet_voltage_limits(), &sol.pairs, probe).0);
    }
    s
}

/// DRC/ERC/LVS over the drawn solution plus undrawable devices — what an
/// epoch's `RunStats::drc_hard` counts (no operating-point rows).
fn drawn_signoff(sol: &Solution, pdk: &Pdk, opts: &verify::ExtractOptions) -> verify::Signoff {
    let (shapes, pins, reference) = signoff_inputs(sol, pdk);
    let field_solve = opts.field_solve.iter().filter(|n| pins.iter().any(|p| &p.name == *n)).cloned().collect();
    let opts = verify::ExtractOptions { field_solve, ..opts.clone() };
    let mut s = verify::signoff_extract(&shapes, &pins, &reference, &sol.intent, &opts, pdk);
    s.report.hard_violations.extend(undrawable(&sol.macros[..sol.layout.x.len()], &sol.devices_of, &sol.netlist));
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
    let (pins, reference) = labels_and_reference(&shapes, &placed, &names, &sol.netlist, Some(&sol.folds), &sol.block_ref, pdk);
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
#[allow(clippy::too_many_arguments)]
pub(crate) fn signoff_shapes(
    intent: &verify::Intent,
    shapes: &[pnr_core::Shape],
    placed: &[Macro],
    nets: &[String],
    schematic: &pnr_core::Netlist,
    fold: Option<&[(u16, i32)]>,
    blocks: &hier::BlockRef,
    pdk: &Pdk,
) -> verify::Signoff {
    let (pins, reference) = labels_and_reference(shapes, placed, nets, schematic, fold, blocks, pdk);
    verify::signoff_checked(shapes, &pins, &reference, intent, pdk)
}

/// Labels every provable net (see [`labeled_pins`]) and builds the LVS
/// reference with exactly those names as ports — `verify` requires they match.
/// A placed block's members are replaced by its own cards (`blocks`, FLOW-11).
#[allow(clippy::too_many_arguments)]
fn labels_and_reference(
    shapes: &[pnr_core::Shape],
    placed: &[Macro],
    nets: &[String],
    schematic: &pnr_core::Netlist,
    fold: Option<&[(u16, i32)]>,
    blocks: &hier::BlockRef,
    pdk: &Pdk,
) -> (Vec<verify::LabeledPin>, verify::RefInput) {
    let pins = labeled_pins(placed, nets, pdk, shapes);
    let (drawn, mut replaced) = cellgen::drawn_cards(placed, nets, schematic, pdk);
    replaced.extend_from_slice(&blocks.members);
    let mut reference = cellgen::reference(schematic, fold, &replaced);
    // A resistor is drawn to its model's recipe: the deck model that names.
    for d in reference.devices.iter_mut().filter(|d| d.kind == verify::reference::RefKind::Resistor) {
        if let Some(r) = pdk.recipe("resistor", d.model.as_deref().unwrap_or("")) {
            d.model = Some(r.model);
        }
    }
    reference.devices.extend(cellgen::dummy_cards(placed, nets, &reference.devices));
    reference.devices.extend(drawn);
    reference.devices.extend(blocks.cards.iter().cloned());
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
    #[test]
    fn axis_period_from_strides() {
        let spec = |strides: Vec<u32>| dr::LatticeSpec { p0: 420, strides, origin_multiple: 0 };
        assert_eq!(super::axis_grid(&spec(vec![1, 1, 2, 2]), 10), Some((420, 420)));
        assert_eq!(super::axis_grid(&spec(vec![1, 3]), 10), Some((420, 1260)));
        assert_eq!(super::axis_grid(&spec(vec![1, 1, 2, 2]), 25), None);
    }

    use crate::{Kind, StopReason};

    const THREE_FET: &str = ".subckt three a b c g VSS\nXM1 a g VSS VSS nfet_01v8 W=2u L=0.5u\nXM2 b a VSS VSS nfet_01v8 W=4u L=0.15u\nXM3 c b VSS VSS nfet_01v8 W=1u L=1u\n.ends three\n";
    const ONE_FET: &str = ".subckt one d g VSS\nXM1 d g VSS VSS nfet_01v8 W=2u L=0.5u\n.ends one\n";

    /// FLOW-08: cold first, without a current-assignment incumbent (after an
    /// escalation) and every `cold_every`-th epoch; `1` is all cold,
    /// `u32::MAX` warm after the first.
    #[test]
    fn schedule_is_cold_first_then_periodic() {
        use crate::epoch_kind;
        assert_eq!(epoch_kind(0, false, 4), Kind::Cold);
        assert_eq!(epoch_kind(1, true, 4), Kind::Warm);
        assert_eq!(epoch_kind(4, true, 4), Kind::Cold);
        assert_eq!(epoch_kind(5, false, 4), Kind::Cold);
        assert!((0..8).all(|k| epoch_kind(k, true, 1) == Kind::Cold));
        assert_eq!(epoch_kind(7, true, u32::MAX), Kind::Warm);
    }

    /// FLOW-08: feasible stops (converged or not); infeasible escalates
    /// until the outer budget.
    #[test]
    fn feasible_incumbent_is_never_escalated() {
        use crate::next_action;
        assert_eq!(next_action(true, false, false), Some(StopReason::FeasibleNotStationary));
        assert_eq!(next_action(true, true, false), Some(StopReason::Converged));
        assert_eq!(next_action(true, true, true), Some(StopReason::Converged));
        assert_eq!(next_action(false, false, true), Some(StopReason::OuterBudget));
        assert_eq!(next_action(false, true, true), Some(StopReason::OuterBudget));
        assert_eq!(next_action(false, false, false), None);
    }

    /// FLOW-08: a spent wall budget stops after the first epoch (one
    /// incumbent is always kept).
    #[test]
    fn wall_budget_stops_with_its_reason() {
        let pdk = verify::Pdk::builtin("sky130").expect("sky130 loads");
        let cfg = crate::Config { max_wall: Some(std::time::Duration::ZERO), feedback_iters: 5, outer_iters: 2, starts: 1, ..Default::default() };
        let sol = crate::run(ONE_FET, &pdk, &Default::default(), &cfg).expect("flow");
        assert_eq!((sol.stats.iterations, sol.stats.stop), (1, StopReason::WallBudget));
    }

    /// FLOW-08: a warm epoch with a no-op schedule keeps its incumbent's
    /// positions and variants, where a cold epoch at its seed places elsewhere
    /// (so the match is the warm start's); `cold_every: 1` never runs warm.
    #[test]
    fn warm_epoch_starts_from_the_incumbent() {
        let pdk = verify::Pdk::builtin("sky130").expect("sky130 loads");
        let cfg = |cold_every| crate::Config { cold_every, feedback_iters: 2, outer_iters: 1, starts: 1, warm: dp::Schedule { range0: 0.0, max_temps: 0, t0_scale: 0.0, ..dp::Schedule::warm() }, ..Default::default() };
        let mut nl = crate::parse(THREE_FET).unwrap();
        crate::deck_models(&mut nl, &pdk);
        let cfg_warm = cfg(u32::MAX);
        let bias = crate::Bias::uniform(Vec::new());
        let ann = crate::annotation_with(&pdk, &cfg_warm.annotation, Box::leak(Box::new(crate::elaborate::stack(&pdk))));
        let plan = crate::PerfPlan::schematic_only(Vec::new(), vec![0]);
        let t = crate::topology(&nl, &Default::default(), &pdk, &cfg_warm, &bias, &ann, &Default::default(), &plan, &Default::default(), true).unwrap();
        let f = &t.flow;
        let epoch = |start: Option<&pnr_core::Layout>, seed: u64| f.epoch(&t.assignment0, false, start, &f.weights, &mut gp::Prices::new(), &mut gr::Negotiation::new(), seed, &mut vec![[0; 4]; f.cells.variants.len()]).layout;
        let pos = |l: &pnr_core::Layout| (l.x.clone(), l.y.clone(), l.variant.clone());
        let inc = epoch(None, 1);
        assert_ne!(pos(&epoch(None, 2)), pos(&inc), "seed 2 places elsewhere cold");
        assert_eq!(pos(&epoch(Some(&inc), 2)), pos(&inc));
        let run = |c| crate::run(THREE_FET, &pdk, &Default::default(), &cfg(c)).expect("flow").stats.warm_epochs;
        assert_eq!((run(u32::MAX), run(1)), (1, 0));
    }

    /// FLOW-10 (T7): the shipped `drc_hard` is the signoff of the shipped
    /// geometry. Without a covered density rule nothing is filled; with a
    /// 10 µm metal1 floor fill is kept and the stats and caps are re-measured
    /// on it.
    /// 80 %, not 40 %: fill counts a 2 µm tile any metal1 touches as covered,
    /// which reads ota's metal1 as 69 % (291/420 tiles), so 40 % fills nothing.
    #[test]
    fn winner_signoff_matches_its_certificate() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        // pair's block is narrower than the window, so ota, on gf180's 3.3 V FETs.
        let spice = std::fs::read_to_string(root.join("benchmarks/fixtures/ota.spice")).expect("fixture");
        let spice = spice.replace("fet_01v8", "fet_03v3");
        let cfg = crate::Config { feedback_iters: 2, outer_iters: 1, starts: 1, ..Default::default() };
        let pdk = verify::Pdk::builtin("gf180mcu").expect("gf180 loads");
        let sol = crate::run(&spice, &pdk, &Default::default(), &cfg).expect("flow");
        assert!(!sol.metadata.post_fill);
        assert_eq!(crate::signoff(&sol, &pdk).report.hard_violations.len(), sol.stats.drc_hard);
        let sidecar = std::fs::read_to_string(root.join("pdks/gf180mcu.json")).expect("sidecar");
        let text = verify::Pdk::deck_text(&sidecar).expect("deck");
        let text = format!("{text}\nrule TEST.m1_density density(metal1; window: 10um, step: 5um) >= 80%\n");
        let pdk = verify::Pdk::load(&text, &sidecar).expect("deck loads");
        let mut sol = crate::run(&spice, &pdk, &Default::default(), &cfg).expect("flow");
        assert!(sol.metadata.post_fill, "ota's block is filled at a 10 µm window");
        let post = crate::signoff(&sol, &pdk);
        assert_eq!(post.report.hard_violations.len(), sol.stats.drc_hard);
        assert_eq!((&post.caps, post.warnings.len() as u32), (&sol.caps, sol.stats.warnings));
        // The fill macro is appended last. Fill clears no density window here
        // (the hard count is the same either way), but grounded fill moves the
        // extracted caps: the winner's own pre-fill caps would not match.
        sol.macros.pop();
        let pre = crate::signoff(&sol, &pdk);
        assert_ne!(pre.caps, sol.caps, "the shipped caps are re-extracted with fill");
    }

    /// PLC-24: the floor is capped by what tiny cells plus their gaps can fill.
    #[test]
    fn utilization_floor_is_reachable_for_tiny_cells() {
        let c = [(2000, 2000); 4];
        assert!((crate::u_eff(0.6, &c, 270) - 0.6).abs() < 1e-6);
        assert!((crate::u_eff(0.6, &c, 1270) - 0.3367).abs() < 1e-3);
        assert_eq!(crate::u_eff(0.6, &[], 0), 0.6);
    }

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
        let problem = || annotator::annotate(&nl, &crate::annotation(&pdk, &Default::default()));
        let (name, id) = pdk.layers[0].clone();
        let lim = Limit { ua_per_um: 1.0, ua_per_cut: 1.0, ..Limit::default() };
        let full: Vec<_> = (0..=MAX_LAYERS as u16).map(|l| (LayerId(l), lim)).collect();

        let mut p = problem();
        crate::em_rules(&mut p, &nl, &full, &[LayerId(MAX_LAYERS as u16)], &[], None, true, &pdk);
        let rows: Vec<&str> = p.missing.iter().filter(|m| m.0 == "Electromigration").map(|m| m.1).collect();
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert!(rows[0].contains("MAX_LAYERS"), "{rows:?}");

        let mut p = problem();
        crate::em_rules(&mut p, &nl, &[], &[id], &[], None, true, &pdk);
        let rows: Vec<&str> = p.missing.iter().filter(|m| m.0 == "Electromigration").map(|m| m.1).collect();
        assert_eq!(rows, [format!("deck EM limit on every routed and pin-access layer ({name} unchecked)")]);
    }

    /// C within the tie band goes to the smaller footprint; outside it, C wins.
    #[test]
    fn close_c_is_decided_by_area_and_far_c_by_c() {
        let k = |c: f32, area: f64| (0usize, 0u32, 0.0, 0.0, c, area);
        assert!(crate::key_lt(&k(30.0, 265.0), &k(29.6, 385.0)), "1.3% more C, 31% less area");
        assert!(crate::key_lt(&k(29.0, 385.0), &k(30.0, 265.0)), "3.3% less C wins outright");
        assert!(!crate::key_lt(&(1usize, 0, 0.0, 0.0, 1.0, 1.0), &k(99.0, 999.0)), "a violation never wins on C");
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
        crate::lex_key(place, &Report::default(), signoff, budgets, 0.0, 1.0)
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
        assert_eq!(key(&place, &Report::default(), &budgets).3, 500.0);
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
        let (nan, finite) = ((0usize, 0u32, f64::NAN, 0.0, 1.0, 1.0), (0usize, 0u32, 5.0, 0.0, 1.0, 1.0));
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
        let rows = [analog::routing::PerformanceBudget::ground_c("gain:min".into(), vec![pnr_core::NetId(0), pnr_core::NetId(1)], vec![0.01, 0.0], 1.0)];
        let cap = |a: &str, b: Option<&str>, c: f64| (a.to_owned(), b.map(str::to_owned), c);
        let a = vec![cap("VSS", Some("vbn"), 89.4), cap("vout1", None, 4.6)];
        let b = vec![cap("VSS", Some("vbn"), 10.0), cap("vout1", None, 5.0)];
        let (ca, cb) = (crate::c_tier(&a, &net_names(), &classes(), &rows), crate::c_tier(&b, &net_names(), &classes(), &rows));
        assert!((ca - 46.0).abs() < 1e-3 && (cb - 50.0).abs() < 1e-3, "c_tier A {ca}, B {cb}");
        let total = |m: &verify::CapMatrix| m.iter().map(|r| r.2).sum::<f64>();
        assert!(total(&a) > 6.0 * total(&b), "A carries 94 fF, B 15 fF");
        assert!(crate::key_lt(&(0, 0, 0.0, 0.0, ca, 1.0), &(0, 0, 0.0, 0.0, cb, 1.0)), "A ranks better");

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
        let (shorted, finite) = ((1usize, 0u32, 0.0, 0.0, tier, 0.5), (1usize, 0u32, 0.0, 0.0, 1000.0, 1.0));
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
            let cells = crate::CellSpace::new(&netlist, &Default::default(), &mut problem, &pdk, &[], true, &fold, &[]);
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

    /// PERF-26: AS/AD/PS/PD per card are the owner's `Figures.sd` totals ÷ the card's `m`, µm(²);
    /// every ota FET is drawn-derived, and XM5 (`m=4`) gets a quarter of its owner's total.
    #[test]
    fn junctions_come_from_the_drawn_figures() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let pdk = verify::Pdk::from_json(&std::fs::read_to_string(root.join("pdks/sky130.json")).unwrap()).unwrap();
        let spice = std::fs::read_to_string(root.join("benchmarks/fixtures/ota.spice")).unwrap();
        let mut netlist = crate::parse::spice(&spice).expect("parses");
        crate::deck_models(&mut netlist, &pdk);
        let mut problem = annotator::annotate(&netlist, &crate::annotation(&pdk, &Default::default()));
        let fold = crate::cellgen::folds(&netlist, &pdk, &[], &[]);
        let cells = crate::CellSpace::new(&netlist, &Default::default(), &mut problem, &pdk, &[], true, &fold, &[]);
        let placed: Vec<pnr_core::Macro> = cells.variants.iter().map(|v| v.alternatives[0].clone()).collect();
        let j = crate::junctions(&placed, &cells.devices_of, &netlist);
        let mut quartered = false;
        for (i, dev) in netlist.devices.iter().enumerate() {
            let Some(s) = dev.mos_size() else { continue };
            let id = pnr_core::DeviceId(i as u16);
            let cell = cells.devices_of.iter().position(|m| m.contains(&id)).expect("device has a cell");
            let owner = cells.devices_of[cell].iter().position(|&d| d == id).unwrap();
            let total = |t: &str| placed[cell].figures.sd.iter().filter(|e| usize::from(e.0) == owner && e.1 == t).map(|e| e.2 as f64).sum::<f64>();
            let jd = j[i].unwrap_or_else(|| panic!("{}: no junction", dev.name));
            let m = f64::from(s.m);
            assert!((jd[1] * m * 1e6 - total("D")).abs() < 1e-6 * total("D").max(1.0), "{} AD {jd:?} vs {}", dev.name, total("D"));
            assert!((jd[0] * m * 1e6 - total("S")).abs() < 1e-6 * total("S").max(1.0), "{} AS {jd:?} vs {}", dev.name, total("S"));
            assert!(total("D") > 0.0 && total("S") > 0.0, "{}: no drawn S/D", dev.name);
            quartered |= s.m == 4 && (jd[1] - total("D") / 4.0 / 1e6).abs() < 1e-9;
        }
        assert!(quartered, "XM5 (m=4) carries a quarter of its owner's AD");
    }

    /// PERF-26: dr's antenna diode reaches the post-layout deck with the deck's diode model.
    #[test]
    fn an_inserted_diode_is_simulated() {
        use pnr_core::{Device, DeviceKind, Net, NetId};
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let pdk = verify::Pdk::from_json(&std::fs::read_to_string(root.join("pdks/sky130.json")).unwrap()).unwrap();
        let nl = pnr_core::Netlist {
            devices: vec![Device {
                name: "M1".into(),
                kind: DeviceKind::Nmos, model: "sky130_fd_pr__nfet_01v8".into(),
                terminals: vec![("D".into(), NetId(0)), ("G".into(), NetId(1)), ("S".into(), NetId(2)), ("B".into(), NetId(2))],
                params: vec![("w".into(), 1000), ("l".into(), 500)],
            }],
            nets: ["out", "in", "vss"].iter().map(|n| Net { name: (*n).into() }).collect(),
            ..Default::default()
        };
        let model = crate::model_table(&pdk).into_iter().find(|(_, k)| *k == DeviceKind::Diode).map(|(m, _)| m).unwrap();
        let extra = [Device {
            name: "XDANT1".into(),
            kind: DeviceKind::Diode, model,
            terminals: vec![("P".into(), NetId(2)), ("N".into(), NetId(1))],
            params: vec![("w".into(), 450), ("l".into(), 450)],
        }];
        let cfg = crate::perf::PerfConfig { sim: crate::oppoint::OpConfig::default(), testbenches: vec![String::new()], specs: vec![], scenarios: Vec::new() };
        let d = crate::perf::deck(&crate::with_extra(&nl, &extra), &crate::perf::Parasitics::default(), &cfg, "", &cfg.scenarios()[0]).unwrap();
        // `vss` is the deck's ground node `0`.
        assert!(d.contains("DXDANT1 0 in sky130_fd_pr__diode_pw2nd_05v5"), "{d}");
        assert!(matches!(crate::with_extra(&nl, &[]), std::borrow::Cow::Borrowed(_)));
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
        let bias = crate::Bias::uniform(Vec::new());
        let ann = crate::annotation_with(&pdk, &cfg.annotation, Box::leak(Box::new(crate::elaborate::stack(&pdk))));
        let plan = crate::PerfPlan::schematic_only(Vec::new(), vec![0]);
        let t = crate::topology(&nl, &injected, &pdk, &cfg, &bias, &ann, &Default::default(), &plan, &Default::default(), true).unwrap();
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

/// PLC-07: per-pair placement spacing on the shipped decks and fixtures.
#[cfg(test)]
mod spacing_tests {
    use gp::spacing::{profile, Face, Src, SpacingTable, N, ROLES};
    use pnr_core::{DeviceId, Layout, Macro, Orient, Process as _};
    use std::collections::BTreeMap;

    fn root() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn pdk(deck: &str) -> verify::Pdk {
        let json = std::fs::read_to_string(root().join(format!("pdks/{deck}.json"))).expect("sidecar");
        verify::Pdk::from_json(&json).unwrap_or_else(|e| panic!("{deck} loads: {e}"))
    }

    /// The `CellSpace` the flow builds for fixture `name` (as `size_tests`).
    fn cells(name: &str, pdk: &verify::Pdk) -> crate::CellSpace {
        built(name, pdk).2
    }

    /// [`cells`] with the device names and the annotated problem.
    fn built(name: &str, pdk: &verify::Pdk) -> (Vec<String>, crate::Problem, crate::CellSpace) {
        let spice = std::fs::read_to_string(root().join(format!("benchmarks/fixtures/{name}.spice"))).expect("fixture");
        let mut netlist = crate::parse::spice(&spice).expect("parses");
        crate::deck_models(&mut netlist, pdk);
        let mut problem = annotator::annotate(&netlist, &crate::annotation(pdk, &Default::default()));
        let fold = crate::cellgen::folds(&netlist, pdk, &[], &[]);
        let cs = crate::CellSpace::new(&netlist, &Default::default(), &mut problem, pdk, &[], true, &fold, &[]);
        (netlist.devices.iter().map(|d| d.name.clone()).collect(), problem, cs)
    }

    /// PLC-13, library side: [`crate::match_class`] marks ota's diff pair and
    /// loads `Moderate` and dac4's cell 0 `Exceptional`; [`crate::place_rules`]
    /// stamps that class and the orient set on every profile, and a matched
    /// cell's halo and gap grow by the keep-out tier. The shipped cells' own
    /// diffusion insets already cover the tiers (keep-outs are redundant on the
    /// fixtures), so the halo/gap checks shrink every inset to 0.
    #[test]
    fn matched_cells_get_keep_outs() {
        use gp::spacing::{Face, Profile};
        use pnr_core::MatchClass::{Exceptional, Moderate};
        let pdk = pdk("sky130");
        let (names, problem, cs) = built("ota", &pdk);
        let class = crate::match_class(&problem, &cs);
        let of = |n: &str| cs.devices_of.iter().position(|d| d.iter().any(|&x| names[usize::from(x.0)].ends_with(n))).expect(n);
        for n in ["M1", "M2", "M3", "M4"] {
            assert_eq!(class[of(n)], Some(Moderate), "ota {n}");
        }
        assert_eq!(class[of("M5")], None, "ota M5 (tail) is unmatched");
        let (_, p, dac) = built("dac4", &pdk);
        assert_eq!(crate::match_class(&p, &dac)[0], Some(Exceptional), "dac4 cell 0");

        let mut locks = dp::locks::Locks::default();
        locks.orient_of = (0..cs.variants.len()).map(|c| (c == of("M1") || c == of("M2")).then_some(7)).collect();
        let (with, without) = (crate::place_rules(&pdk, &cs, &locks, &class), crate::place_rules(&pdk, &cs, &locks, &[]));
        let t = &with.spacing;
        let shrink = |p: &Profile| {
            let mut p = *p;
            for e in &mut p.edge {
                for r in (0..gp::spacing::N).filter(|&r| e.present & (1 << r) != 0) {
                    e.inset[r] = 0;
                }
            }
            p
        };
        let tail = shrink(&without.profiles.of[of("M5")][0][0]);
        for c in 0..cs.variants.len() {
            let (m, u) = (with.profiles.of[c][0][0], without.profiles.of[c][0][0]);
            assert_eq!((m.matched, m.set), (class[c], locks.orient_of[c]), "cell {c} profile");
            assert_eq!((u.matched, u.set), (None, locks.orient_of[c]), "cell {c} unmatched build");
            let Some(k) = class[c] else { continue };
            let (ms, us) = (shrink(&m), shrink(&u));
            let tier = t.wpe[k as usize].max(t.foreign_poly[k as usize]);
            assert!(tier > 0 && t.halo(&ms) >= tier && t.halo(&ms) > t.halo(&us), "cell {c}: halo {} vs unmatched {}, tier {tier}", t.halo(&ms), t.halo(&us));
            let (gm, gu) = (t.gap(&ms, Face::R, &tail), t.gap(&us, Face::R, &tail));
            assert!(gm.min > gu.min, "cell {c}: gap to the tail {gm:?} vs unmatched {gu:?}");
        }
        // Orient-set partners are exempt from each other's keep-outs.
        let s = |r: &gp::PlaceRules, n: &str| shrink(&r.profiles.of[of(n)][0][0]);
        assert_eq!(t.gap(&s(&with, "M1"), Face::R, &s(&with, "M2")), t.gap(&s(&without, "M1"), Face::R, &s(&without, "M2")), "M1/M2 share an orient set");
    }

    const FIXTURES: [&str; 10] = ["ota", "ota_constrained", "tt_ota", "pair", "quad", "chain4", "rc_filter", "dac4", "bjt_mirror", "bgr_core"];

    /// Layers `profile` reads as `other`, by name, over every cell × variant.
    fn unmapped(deck: &str) -> BTreeMap<String, usize> {
        let pdk = pdk(deck);
        let mut out = BTreeMap::new();
        for name in FIXTURES {
            let cells = cells(name, &pdk);
            for m in cells.variants.iter().flat_map(|s| &s.alternatives) {
                let r0 = profile(&Macro { shapes: vec![], ..m.clone() }, &pdk, None);
                assert_eq!(r0.edge[0].present, 0, "an empty macro has no roles");
                for s in &m.shapes {
                    let one = profile(&Macro { shapes: vec![*s], ..m.clone() }, &pdk, None);
                    if one.edge[0].present & (1 << (N - 1)) != 0 {
                        let layer = pdk.layers.iter().find(|(_, l)| *l == s.layer).map_or_else(|| format!("#{}", s.layer.0), |(n, _)| n.clone());
                        *out.entry(layer).or_default() += 1;
                    }
                }
            }
        }
        out
    }

    #[test]
    fn shipped_cells_draw_no_unmapped_layer() {
        for deck in ["gf180mcu", "ihp_sg13g2", "generic_finfet"] {
            // Print only: those decks keep `fallback` for unmapped layers.
            match std::panic::catch_unwind(|| unmapped(deck)) {
                Ok(m) => eprintln!("{deck}: shapes on unmapped layers {m:?}"),
                Err(_) => eprintln!("{deck}: fixtures do not build"),
            }
        }
        let sky = unmapped("sky130");
        assert!(sky.is_empty(), "sky130 cells draw layers no placement role maps: {sky:?}");
    }

    #[test]
    fn table_is_symmetric_and_sidecar_wins_when_larger() {
        let pdk = pdk("sky130");
        let t = SpacingTable::new(&pdk, &crate::placement_space(&pdk), 1270, 10);
        for i in 0..N {
            for j in 0..N {
                assert_eq!(t.rule[i][j], t.rule[j][i], "{}/{}", ROLES[i], ROLES[j]);
            }
        }
        let r = |a: &str| ROLES.iter().position(|&x| x == a).unwrap();
        assert_eq!(t.rule[r("diff_out")][r("nwell")], 340);
        // poly.9 is on derived layers (`poly_res`): only the sidecar sees it from rpm.
        assert_eq!((t.rule[r("rpm")][r("poly")], t.src[r("rpm")][r("poly")]), (480, Src::Sidecar));
        assert_eq!((t.rule[r("nwell")][r("nwell")], t.src[r("nwell")][r("nwell")]), (1270, Src::Deck));
        assert_eq!(t.rule[r("diff_in")][r("diff_out")], 270);
        assert_eq!(t.rule[r("diom")][r("diom")], 0, "a marker owes no spacing");
    }

    /// Drawn shapes of `cells` placed by `l`, plus rings and bridges as `Flow::epoch` adds them.
    fn drawn(macros: &[Macro], l: &Layout, rings: &analog::Constraints, pdk: &verify::Pdk) -> Vec<pnr_core::Shape> {
        let placed = pnr_core::place_macros(macros, l);
        let mut extra = cells::post_cell::guard_rings(l, rings, pdk, crate::ring_cut_ohm(pdk));
        extra.extend(cells::post_cell::well_bridges(&placed, &extra, pdk, &|_, _| true));
        let all: Vec<Macro> = placed.iter().chain(&extra).cloned().collect();
        extra.extend(cells::post_cell::implant_bridges(&all, pdk));
        let mut shapes: Vec<_> = placed.iter().chain(&extra).flat_map(|m| m.shapes.iter().copied()).collect();
        if let Some(nw) = pdk.layer("nwell") {
            crate::geometry::merge_rects(&mut shapes, nw);
        }
        shapes
    }

    fn findings(shapes: &[pnr_core::Shape], pdk: &verify::Pdk) -> BTreeMap<String, usize> {
        let mut out = BTreeMap::new();
        for f in verify::drc(shapes, &[], pdk).into_iter().filter(|f| !f.warning) {
            *out.entry(f.rule).or_default() += 1;
        }
        out
    }

    /// Cells at lower-left corners `at`, unturned.
    fn layout(macros: &[Macro], at: &[(i32, i32)]) -> Layout {
        let n = macros.len();
        let (hw, hh): (Vec<i32>, Vec<i32>) = macros.iter().map(|m| (m.bbox.w / 2, m.bbox.h / 2)).unzip();
        Layout {
            x: (0..n).map(|i| at[i].0 + hw[i]).collect(),
            y: (0..n).map(|i| at[i].1 + hh[i]).collect(),
            hw,
            hh,
            variant: vec![0; n],
            axis: vec![],
            branch: vec![false; n],
            groups: (0..n).map(|i| vec![DeviceId(i as u16)]).collect(),
            orient: vec![Orient::R0; n],
            power_uw: vec![0; n],
            temp_mc: vec![0; n],
            units: Default::default(),
        }
    }

    /// Every ordered cell pair, x- and y-facing, stamped at the table's `min`
    /// (and at 0 when `abut`): no DRC rule fires more often on the pair than on
    /// the two cells alone. Cells' own findings are not this item's.
    #[test]
    fn table_gaps_are_drc_clean_on_sky130() {
        let pdk = pdk("sky130");
        let lattice = cells::builder::cut_lattice(&pdk);
        // (macro, profile, its ring request with `device` cleared)
        let mut uniq: Vec<(Macro, gp::spacing::Profile, Option<analog::cell::GuardRingRequirement>)> = Vec::new();
        for name in ["ota", "dac4", "rc_filter", "chain4", "pair"] {
            let cs = cells(name, &pdk);
            let rules = crate::place_rules(&pdk, &cs, &Default::default(), &[]);
            for (c, s) in cs.variants.iter().enumerate() {
                for (v, m) in s.alternatives.iter().enumerate().take(2) {
                    if uniq.iter().any(|(u, ..)| u.shapes == m.shapes && u.bbox == m.bbox) {
                        continue;
                    }
                    let ring = cs.guard_rings.guard_rings.iter().find(|g| usize::from(g.device.0) == c).cloned();
                    uniq.push((m.clone(), rules.profiles.of[c][v][0], ring));
                }
            }
        }
        let rules = crate::place_rules(&pdk, &cells("pair", &pdk), &Default::default(), &[]);
        let t = &rules.spacing;
        let rings_of = |ids: &[usize]| {
            let mut c = analog::Constraints::default();
            for (k, &i) in ids.iter().enumerate() {
                if let Some(mut r) = uniq[i].2.clone() {
                    r.device = DeviceId(k as u16);
                    c.guard_rings.push(r);
                }
            }
            c
        };
        let single: Vec<BTreeMap<String, usize>> = (0..uniq.len())
            .map(|i| findings(&drawn(&[uniq[i].0.clone()], &layout(&[uniq[i].0.clone()], &[(0, 0)]), &rings_of(&[i]), &pdk), &pdk))
            .collect();
        let (mut probes, mut tight, mut tight_of, mut bad) = (0, 0, 0, Vec::new());
        for a in 0..uniq.len() {
            for b in 0..uniq.len() {
                let (ma, mb) = (&uniq[a].0, &uniq[b].0);
                for f in [Face::R, Face::T] {
                    let g = t.gap(&uniq[a].1, f, &uniq[b].1);
                    let mut gaps = vec![g.min];
                    if g.abut && g.min > 0 {
                        gaps.push(0);
                    }
                    let at = |gap: i32| match f {
                        Face::R => (ma.bbox.w + gap, (ma.bbox.h - mb.bbox.h) / 2 / lattice * lattice),
                        _ => ((ma.bbox.w - mb.bbox.w) / 2 / lattice * lattice, ma.bbox.h + gap),
                    };
                    let run = |gap: i32| {
                        let pair = [ma.clone(), mb.clone()];
                        let mut got = findings(&drawn(&pair, &layout(&pair, &[(0, 0), at(gap)]), &rings_of(&[a, b]), &pdk), &pdk);
                        got.retain(|r, n| *n > single[a].get(r).unwrap_or(&0) + single[b].get(r).unwrap_or(&0));
                        got
                    };
                    for gap in gaps {
                        probes += 1;
                        let extra = run(gap);
                        if !extra.is_empty() {
                            bad.push(format!("cells {a}->{b} {f:?} gap {gap}: {extra:?}"));
                        }
                    }
                    if g.min >= 2 * lattice {
                        tight_of += 1;
                        tight += usize::from(!run(g.min - 2 * lattice).is_empty());
                    }
                }
            }
        }
        eprintln!("{} cells, {probes} probes; tightness: {tight}/{tight_of} fail at min − 2·lattice", uniq.len());
        assert!(bad.is_empty(), "{} of {probes} probes add DRC findings:\n{}", bad.len(), bad.join("\n"));
    }
}

#[cfg(test)]
mod environment_tests {
    use pnr_core::{Layout, Orient, Process as _, Rect};

    fn root() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn pdk() -> verify::Pdk {
        verify::Pdk::from_json(&std::fs::read_to_string(root().join("pdks/sky130.json")).expect("sidecar")).expect("sky130 loads")
    }

    /// The live batch's geometry (`EnvGeo` + `place_rect`) is the placed
    /// macros' own, cell by cell, under every orient and the last variant.
    #[test]
    fn env_geo_places_like_place_macros_on_ota() {
        let pdk = pdk();
        let spice = std::fs::read_to_string(root().join("benchmarks/fixtures/ota.spice")).expect("fixture");
        let mut netlist = crate::parse::spice(&spice).expect("parses");
        crate::deck_models(&mut netlist, &pdk);
        let mut problem = annotator::annotate(&netlist, &crate::annotation(&pdk, &Default::default()));
        let fold = crate::cellgen::folds(&netlist, &pdk, &[], &[]);
        let cells = crate::CellSpace::new(&netlist, &Default::default(), &mut problem, &pdk, &[], true, &fold, &[]);
        let env = crate::live_environment(&problem, &cells, &pdk);
        let n = cells.variants.len();
        assert!(!env.pairs.is_empty(), "ota has a matched pair");
        assert!(env.pairs.iter().all(|&(_, _, a, b)| usize::from(a) < n && usize::from(b) < n));

        const ALL: [Orient; 8] = [Orient::R0, Orient::R90, Orient::R180, Orient::R270, Orient::Mx, Orient::Mx90, Orient::Mx180, Orient::Mx270];
        let variant: Vec<u16> = cells.variants.iter().map(|v| (v.alternatives.len() - 1) as u16).collect();
        let orient: Vec<Orient> = (0..n).map(|c| ALL[c % 8]).collect();
        let half = |c: usize| {
            let r = orient[c].apply_rect(cells.variants[c].alternatives[usize::from(variant[c])].bbox);
            (r.w / 2, r.h / 2)
        };
        let l = Layout {
            x: (0..n).map(|c| 20_000 * c as i32 + 1_005).collect(),
            y: (0..n).map(|c| 7_000 * (c as i32 % 3) - 3_015).collect(),
            hw: (0..n).map(|c| half(c).0).collect(),
            hh: (0..n).map(|c| half(c).1).collect(),
            orient,
            variant,
            axis: vec![],
            branch: vec![],
            groups: vec![],
            power_uw: vec![0; n],
            temp_mc: vec![0; n],
            units: std::sync::Arc::default(),
        };
        let placed = pnr_core::place_macros(&crate::cellgen::realize(&cells.variants, &l.variant), &l);
        let (nwell, diff) = (pdk.layer("nwell").expect("nwell"), pdk.layer("diff").expect("diff"));
        for c in 0..n {
            let v = usize::from(l.variant[c]);
            for (layer, local) in [(nwell, &env.geo.wells[c][v]), (diff, &env.geo.diffs[c][v])] {
                let key = |r: &Rect| (r.x, r.y, r.w, r.h);
                let mut want: Vec<_> = placed[c].shapes.iter().filter(|s| s.layer == layer).map(|s| key(&s.rect)).collect();
                let mut got: Vec<_> = local.iter().map(|&r| key(&pnr_core::place_rect(env.geo.bbox[c][v], r, &l, c))).collect();
                want.sort_unstable();
                got.sort_unstable();
                assert_eq!(got, want, "cell {c}");
            }
        }
    }

    /// The live batch scores in the placement arm; the routing arm keeps only
    /// the final report's ring-inclusive row (T9's judge). `ota`, not `pair`:
    /// `pair`'s two devices share their gate, so no matched pair is recognised.
    /// Red under debug until MAT-07 stops registering `OrientationSet` in both
    /// arms (`annotator/src/emit.rs:145-148` trips `gp::Prices::bind`'s
    /// debug_assert on ota); see `docs/plans/m2-placement-report.md`.
    #[test]
    fn environment_is_in_the_placement_arm_once() {
        let spice = std::fs::read_to_string(root().join("benchmarks/fixtures/ota.spice")).expect("fixture");
        let cfg = crate::Config { feedback_iters: 1, outer_iters: 1, starts: 1, ..Default::default() };
        let sol = crate::run(&spice, &pdk(), &Default::default(), &cfg).expect("flow");
        let rows = |r: &[crate::metadata::BudgetStatus]| r.iter().filter(|b| b.kind == "Environment").count();
        assert_eq!(rows(&sol.metadata.placement), 1);
        assert_eq!(rows(&sol.metadata.routing), 1);
    }

    /// A sidecar `Load` reaches the annotator: ota's drain-only nets miss AA-25's external load
    /// until the sidecar gives one each (the merge once dropped `loads`).
    #[test]
    fn sidecar_loads_reach_the_annotator() {
        let spice = std::fs::read_to_string(root().join("benchmarks/fixtures/ota.spice")).expect("fixture");
        let entry = ("ParasiticBudget", "external load of drain-only nets: sidecar Load (AA-25)");
        let missing = |constraints: Option<&str>| {
            let cfg = crate::Config { feedback_iters: 1, outer_iters: 1, starts: 1, constraints: constraints.map(Into::into), ..Default::default() };
            crate::run(&spice, &pdk(), &Default::default(), &cfg).expect("flow").metadata.missing.contains(&entry)
        };
        assert!(missing(None), "ota without a sidecar must miss the drain-only load");
        let loads = r#"[{"constraint":"Load","net":"vout1","ff":100},{"constraint":"Load","net":"vout2","ff":100},{"constraint":"Load","net":"vtail","ff":100}]"#;
        assert!(!missing(Some(loads)), "sidecar loads must satisfy AA-25");
    }
}

#[cfg(test)]
mod halo_tests {
    use super::{halo_step, halo_target, static_halos, update_dyn_halo};
    use pnr_core::geom::Rect;
    use pnr_core::{LayerId, Macro, NetId, Pin};

    fn cell() -> Macro {
        let pin = |name: &str, x: i32| Pin { name: name.into(), net: NetId(0), at: Rect { x, y: 450, w: 100, h: 100 }, layer: LayerId(0) };
        Macro { pins: vec![pin("D", 1_900), pin("S", 0)], bbox: Rect { x: 0, y: 0, w: 2_000, h: 1_000 }, ..Default::default() }
    }

    /// 5 mA at J = 2.8 µA/nm (sky130 met1) needs ⌈1785.7⌉ nm, 140 of them one
    /// track: 1650 on the right face. 300 µA ≤ I_1 = 392 µA reserves nothing.
    #[test]
    fn static_halo_reserves_only_excess_width() {
        let v = vec![gp::VariantSpace { alternatives: vec![cell()] }];
        let ua = vec![vec![("D".to_string(), Some(5_000)), ("S".to_string(), Some(300))]];
        assert_eq!(static_halos(&v, &ua, 2.8, 140, 10), vec![vec![[0, 0, 1_650, 0]]]);
        assert_eq!(static_halos(&v, &[], 2.8, 140, 10), vec![vec![[0; 4]]]);
    }

    #[test]
    fn dynamic_halo_decays_without_overflow() {
        let h = halo_step(800, halo_target(1.0, 2, 420, 10), 420, 10);
        assert_eq!(h, 400);
        assert_eq!(halo_step(h, halo_target(1.0, 2, 420, 10), 420, 10), 200);
    }

    #[test]
    fn dynamic_halo_is_capped() {
        assert_eq!(halo_step(0, halo_target(10.0, 4, 420, 10), 420, 10), 1_680);
    }

    /// Demand 3 in the region left of the cell, 0.5 everywhere else: only face
    /// L grows, by half of `(3 − 1)·1·420`.
    #[test]
    fn band_reads_only_the_adjacent_region() {
        let l = pnr_core::Layout {
            x: vec![10_000],
            y: vec![10_000],
            hw: vec![1_000],
            hh: vec![500],
            axis: vec![0],
            groups: vec![],
            orient: vec![pnr_core::Orient::R0],
            variant: vec![0],
            branch: Vec::new(),
            power_uw: vec![0],
            temp_mc: vec![0],
            units: Default::default(),
        };
        let congestion = [
            (Rect { x: 0, y: 0, w: 9_000, h: 20_000 }, 3.0),
            (Rect { x: 9_000, y: 0, w: 11_000, h: 20_000 }, 0.5),
        ];
        let mut halo = vec![[0; 4]];
        update_dyn_halo(&mut halo, &l, &congestion, &[], 420, 10);
        assert_eq!(halo, vec![[420, 0, 0, 0]]);
    }
}

/// Step-2 coverage of the flow's own helpers: every branch and corner case
/// (empty input, boundaries, degenerate geometry, error paths) against the
/// doc comments' contracts.
#[cfg(test)]
mod cleanup_tests {
    use crate::{Config, FlowError, Hierarchy, Interface, RunStats};
    use annotator::{Block, BlockKind};
    use pnr_core::geom::Rect;
    use pnr_core::{Device, DeviceId, DeviceKind, LayerId, Macro, Net, NetId, Netlist, Pin, Report, Shape, Violation};

    fn dev(name: &str, kind: DeviceKind, params: &[(&str, i64)]) -> Device {
        Device { name: name.into(), kind, model: String::new(), terminals: Vec::new(), params: params.iter().map(|&(k, v)| (k.into(), v)).collect() }
    }

    fn netlist(devices: Vec<Device>, nets: &[&str]) -> Netlist {
        Netlist { devices, nets: nets.iter().map(|n| Net { name: (*n).into() }).collect(), ..Default::default() }
    }

    fn block(kind: BlockKind, devices: &[u16], sub_blocks: Vec<Block>) -> Block {
        Block { kind, template: "t", devices: devices.iter().map(|&d| DeviceId(d)).collect(), injected: false, sub_blocks, selfs: Vec::new() }
    }

    const ONE_FET: &str = ".subckt one d g VSS\nXM1 d g VSS VSS nfet_01v8 W=2u L=0.5u\n.ends one\n";

    // ---- tools ----

    #[test]
    fn a_present_tool_is_reported_present() {
        assert!(crate::tools::present_or_skip("anything", true));
    }

    // ---- Config / hierarchy ----

    #[test]
    fn hierarchy_resolves_flat_bottom_up_and_auto_at_its_100_device_bound() {
        let nl = |n: usize| netlist(vec![dev("M", DeviceKind::Nmos, &[]); n], &[]);
        let cfg = |hierarchy| Config { hierarchy, ..Default::default() };
        assert_eq!(crate::hierarchy(&cfg(Hierarchy::Flat), &nl(500)), None);
        assert_eq!(crate::hierarchy(&cfg(Hierarchy::BottomUp { min_devices: 5 }), &nl(0)), Some(5));
        assert_eq!(crate::hierarchy(&cfg(Hierarchy::Auto), &nl(100)), None, "exactly 100 stays flat");
        assert_eq!(crate::hierarchy(&cfg(Hierarchy::Auto), &nl(101)), Some(2));
        assert_eq!(crate::hierarchy(&cfg(Hierarchy::Auto), &nl(0)), None);
    }

    #[test]
    fn stages_name_every_stage_ms_slot() {
        assert_eq!(crate::STAGES.len(), RunStats::default().stage_ms.len());
        assert_eq!((crate::STAGES[6], crate::STAGES[8]), ("signoff", "perf"), "Flow::epoch laps 6, score_perf writes 8");
    }

    // ---- Interface::from_json ----

    #[test]
    fn interface_defaults_when_die_and_pins_are_absent_or_null() {
        assert_eq!(Interface::from_json("{}").unwrap(), Interface::default());
        assert_eq!(Interface::from_json(r#"{"die": null, "pins": null}"#).unwrap(), Interface::default());
    }

    #[test]
    fn interface_accepts_frac_at_both_ends() {
        let i = Interface::from_json(r#"{"die":{"w":10,"h":20},"pins":[
            {"net":"a","side":"west","frac":0,"width":5,"layer":"met3"},
            {"net":"b","side":"east","frac":1,"width":5,"layer":"met3"},
            {"net":"c","side":"north","frac":0.5,"width":5,"layer":"met2"}]}"#)
        .unwrap();
        assert_eq!(i.die_nm, Some((10, 20)));
        assert_eq!(i.pins.iter().map(|p| (p.side, p.frac)).collect::<Vec<_>>(), [(crate::Side::West, 0.0), (crate::Side::East, 1.0), (crate::Side::North, 0.5)]);
        assert_eq!(i.pins[2].layer, "met2");
    }

    #[test]
    fn interface_rejects_each_malformed_field_by_name() {
        let err = |t: &str| Interface::from_json(t).expect_err(t);
        assert!(err("not json").starts_with("interface:"));
        assert!(err(r#"{"die":{"w":"10","h":5}}"#).contains("`w`"));
        assert!(err(r#"{"die":{"w":10}}"#).contains("`h`"));
        assert!(err(r#"{"die":{"w":3000000000,"h":5}}"#).contains("`w`"), "past i32");
        assert!(err(r#"{"pins":[{"net":"a","side":"up","frac":0.5,"width":5,"layer":"m"}]}"#).contains("pins[0].side"));
        assert!(err(r#"{"pins":[{"net":"a","side":"north","frac":1.5,"width":5,"layer":"m"}]}"#).contains("pins[0].frac"));
        assert!(err(r#"{"pins":[{"net":"a","side":"north","frac":-0.1,"width":5,"layer":"m"}]}"#).contains("pins[0].frac"));
        assert!(err(r#"{"pins":[{"side":"north","frac":0.5,"width":5,"layer":"m"}]}"#).contains("pins[0].net"));
        assert!(err(r#"{"pins":[{"net":"a","side":"north","frac":0.5,"layer":"m"}]}"#).contains("`width`"));
    }

    /// Degenerate geometry and a mistyped `pins` are errors, not a silent
    /// default (the doc's "not a positive i32" / "wrong shape").
    #[test]
    fn interface_rejects_degenerate_die_widths_and_non_array_pins() {
        assert!(Interface::from_json(r#"{"die":{"w":0,"h":5}}"#).is_err(), "zero-width die");
        assert!(Interface::from_json(r#"{"die":{"w":10,"h":-5}}"#).is_err(), "negative die");
        assert!(Interface::from_json(r#"{"pins":[{"net":"a","side":"north","frac":0.5,"width":0,"layer":"m"}]}"#).is_err(), "zero-width pin");
        assert!(Interface::from_json(r#"{"pins": 5}"#).is_err(), "pins must be an array");
        assert!(Interface::from_json(r#"{"die": 5}"#).is_err(), "die must be an object");
    }

    // ---- search control ----

    #[test]
    fn cold_every_zero_reads_as_every_epoch_cold() {
        assert!((0..6).all(|k| crate::epoch_kind(k, true, 0) == crate::Kind::Cold));
        assert_eq!(crate::epoch_kind(0, true, 3), crate::Kind::Cold, "epoch 0 is cold even with an incumbent");
    }

    #[test]
    fn merge_keeps_the_winners_legality_and_the_runs_counters() {
        let winner = RunStats { place_hard: 1, route_hard: 2, drc_hard: 3, warnings: 4, route_overuse: 5, c_tier: 6.0, iterations: 99, sims: 99, ..Default::default() };
        let run = RunStats { place_hard: 50, iterations: 10, sims: 7, variant_escalations: 2, ..Default::default() };
        let m = winner.merge(run);
        assert_eq!((m.place_hard, m.route_hard, m.drc_hard, m.warnings, m.route_overuse, m.c_tier), (1, 2, 3, 4, 5, 6.0));
        assert_eq!((m.iterations, m.sims, m.variant_escalations), (10, 7, 2));
    }

    // ---- key ----

    #[test]
    fn key_lt_is_irreflexive_and_ranks_v_before_everything() {
        let k = (0usize, 0u32, 1.0, 2.0, 3.0f32, 4.0);
        assert!(!crate::key_lt(&k, &k));
        assert!(crate::key_lt(&(0, 9, 9.0, 9.0, 9.0, 9.0), &(1, 0, 0.0, 0.0, 0.0, 0.0)));
        assert!(crate::key_lt(&(0, 0, 9.0, 9.0, 9.0, 9.0), &(0, 1, 0.0, 0.0, 0.0, 0.0)), "failed bounds before shortfall");
        assert!(crate::key_lt(&(0, 0, 0.0, 9.0, 9.0, 9.0), &(0, 0, 1.0, 0.0, 0.0, 0.0)), "shortfall before Θ");
        assert!(crate::key_lt(&(0, 0, 0.0, 0.0, 9.0, 9.0), &(0, 0, 0.0, 1.0, 0.0, 0.0)), "Θ before C");
        assert!(crate::key_lt(&(0, 0, f64::NAN, 0.0, 0.0, 0.0), &(0, 0, f64::NAN, 1.0, 0.0, 0.0)), "equal NaN shortfalls fall through to Θ");
    }

    /// Equal C tiers are a tie whatever their value, so area decides: zero C
    /// on both sides, and two unknown (NaN, read as +∞) tiers alike.
    #[test]
    fn equal_c_tiers_are_decided_by_area() {
        assert!(crate::key_lt(&(0, 0, 0.0, 0.0, 0.0, 1.0), &(0, 0, 0.0, 0.0, 0.0, 2.0)));
        assert!(crate::key_lt(&(0, 0, 0.0, 0.0, f32::NAN, 1.0), &(0, 0, 0.0, 0.0, f32::NAN, 2.0)), "two NaN tiers tie");
        assert!(!crate::key_lt(&(0, 0, 0.0, 0.0, f32::NAN, 2.0), &(0, 0, 0.0, 0.0, f32::NAN, 1.0)));
        assert!(crate::key_lt(&(0, 0, 0.0, 0.0, 1.0, 1.0), &(0, 0, 0.0, 0.0, 1.0, f64::NAN)), "a NaN footprint loses");
    }

    #[test]
    fn epoch_score_sums_route_margins_and_leaves_spec_tiers_zero() {
        let v = |rule: &str, margin| Violation { rule: rule.into(), margin };
        let route = Report { hard_violations: vec![v("open net", 1)], budget_violations: vec![v("r", 3), v("s", 4)], ..Default::default() };
        let (key, stats) = crate::epoch_score(&Report::default(), &route, &verify::Signoff::default(), &crate::metadata::MetadataReport::default(), 2.5, 9.0);
        assert_eq!((key.0, key.1, key.2, key.3, key.4, key.5), (1, 0, 0.0, 7.0, 2.5, 9.0));
        assert_eq!((stats.route_hard, stats.route_overuse, stats.c_tier, stats.place_hard, stats.drc_hard), (1, 7, 2.5, 0, 0));
    }

    #[test]
    fn c_tier_of_nothing_is_zero_and_adverse_weights_only() {
        let names = ["a".to_string(), "b".to_string()];
        assert_eq!(crate::c_tier(&Vec::new(), &names, &[], &[]), 0.0);
        let row = analog::routing::PerformanceBudget::ground_c("m:min".into(), vec![NetId(0), NetId(1)], vec![-1.0, 0.002], 1.0);
        let caps = vec![("a".to_string(), None, 5.0), ("b".to_string(), None, 1.0)];
        let c = crate::c_tier(&caps, &names, &[], &[row]);
        assert!((c - 2.0).abs() < 1e-4, "a's negative weight clamps to 0, b 2/fF: {c}");
    }

    #[test]
    fn only_a_label_short_makes_the_tier_unknown() {
        let mut s = verify::Signoff { caps: vec![("a".into(), None, 1.0)], ..Default::default() };
        s.report.hard_violations = vec![Violation { rule: "lvs/net mismatch".into(), margin: 1 }];
        let classes = [analog::metadata::NetClassification { net: NetId(0), class: analog::metadata::NetClass::Signal, c_budget_af: None, max_coupling_af: None }];
        assert!((crate::signoff_c_tier(&s, &["a".into()], &classes, &[]) - 1.0).abs() < 1e-6);
    }

    // ---- routing weights and budgets ----

    #[test]
    fn route_weights_keep_budgeted_nets_scaled_to_one() {
        use analog::metadata::{NetClass, NetClassification};
        let class = |n: u16, budget| NetClassification { net: NetId(n), class: NetClass::Signal, c_budget_af: budget, max_coupling_af: None };
        let classes = [class(0, Some(10)), class(1, None), class(2, None)];
        assert_eq!(crate::route_weights(&[2.0, 4.0, 8.0], &classes, &[(NetId(1), 1.0)]), vec![0.5, 1.0, 0.0]);
        assert_eq!(crate::route_weights(&[2.0, 4.0], &[], &[]), vec![0.0, 0.0], "nothing budgeted");
        assert_eq!(crate::route_weights(&[0.0], &classes, &[]), vec![0.0], "all-zero weights stay zero");
        assert!(crate::route_weights(&[], &classes, &[]).is_empty());
    }

    /// No current, or a non-positive one, leaves the budget unknown (0), never ∞.
    #[test]
    fn common_node_budget_is_unknown_without_a_positive_current() {
        let left = [(0u32, 1u32, 0.5f32)];
        assert_eq!(crate::common_node_ohm(&left, DeviceId(0), DeviceId(1), Some(0.0)), 0.0);
        assert_eq!(crate::common_node_ohm(&left, DeviceId(0), DeviceId(1), Some(-10.0)), 0.0);
        assert_eq!(crate::common_node_ohm(&[], DeviceId(0), DeviceId(1), Some(10.0)), 0.0);
    }

    // ---- placement helpers ----

    #[test]
    fn u_eff_never_raises_the_floor_and_survives_empty_cells() {
        assert_eq!(crate::u_eff(0.6, &[(0, 0), (0, 0)], 0), 0.6, "zero padded area");
        assert_eq!(crate::u_eff(0.6, &[(1000, 1000)], 0), 0.6, "0.9 > 0.6: the floor stands");
        assert!(crate::u_eff(0.6, &[(1000, 1000)], 1000) < 0.6);
    }

    #[test]
    fn axis_grid_degenerate_inputs() {
        let spec = |p0: i32, strides: Vec<u32>| dr::LatticeSpec { p0, strides, origin_multiple: 0 };
        assert_eq!(crate::axis_grid(&spec(420, vec![]), 10), Some((420, 420)), "no layers: S = 1");
        assert_eq!(crate::axis_grid(&spec(420, vec![1, 0]), 10), Some((420, 420)), "a 0 stride reads as 1");
        assert_eq!(crate::axis_grid(&spec(420, vec![1, 1]), 0), Some((420, 420)), "lattice 0 reads as 1");
        assert_eq!(crate::axis_grid(&spec(0, vec![1]), 10), None);
        assert_eq!(crate::axis_grid(&spec(-420, vec![1]), 10), None);
    }

    /// A period past i32 is no grid at all, not an overflow panic.
    #[test]
    fn axis_grid_overflow_is_none() {
        let spec = dr::LatticeSpec { p0: 420, strides: vec![1, 4_000_000_000], origin_multiple: 0 };
        assert_eq!(crate::axis_grid(&spec, 10), None);
        let spec = dr::LatticeSpec { p0: 420, strides: vec![1, 4_000_000_007, 1, 4_000_000_009], origin_multiple: 0 };
        assert_eq!(crate::axis_grid(&spec, 10), None, "lcm past u32");
    }

    #[test]
    fn matched_cells_are_merged_groups_or_two_device_pair_leaves() {
        let blocks = [
            block(BlockKind::DiffPair, &[0, 1], vec![]),
            block(BlockKind::Stack, &[2, 3], vec![]),
            block(BlockKind::DiffPair, &[4, 5, 6], vec![]),
            block(BlockKind::Group, &[9, 10], vec![block(BlockKind::CascodePair, &[9, 10], vec![])]),
        ];
        let devices_of: Vec<Vec<DeviceId>> = [&[0][..], &[1], &[2], &[3], &[4], &[7, 8], &[10]].iter().map(|m| m.iter().map(|&d| DeviceId(d)).collect()).collect();
        assert_eq!(crate::matched_cells(&blocks, &devices_of), [true, true, false, false, false, true, true]);
        assert!(crate::matched_cells(&blocks, &[]).is_empty());
    }

    #[test]
    fn matched_pairs_are_two_device_diff_mirror_load_leaves() {
        let blocks = [
            block(BlockKind::Group, &[0, 1, 2, 3], vec![block(BlockKind::Load, &[0, 1], vec![]), block(BlockKind::CascodePair, &[2, 3], vec![])]),
            block(BlockKind::CurrentMirror, &[4, 5], vec![]),
            block(BlockKind::Glue, &[6, 7], vec![]),
            block(BlockKind::DiffPair, &[8, 9, 10], vec![]),
        ];
        assert_eq!(crate::matched_pairs(&blocks), [(DeviceId(0), DeviceId(1)), (DeviceId(4), DeviceId(5))]);
        assert!(crate::matched_pairs(&[]).is_empty());
    }

    #[test]
    fn remap_members_dedups_in_first_seen_order_and_keeps_unmapped_ids() {
        let ids = |v: &[u16]| v.iter().map(|&d| DeviceId(d)).collect::<Vec<_>>();
        assert_eq!(crate::remap_members(&ids(&[2, 0, 1, 3]), &[5, 5, 4]), ids(&[4, 5, 3]));
        assert!(crate::remap_members(&[], &[0]).is_empty());
    }

    // ---- halos ----

    fn cell() -> Macro {
        let pin = |name: &str, x: i32| Pin { name: name.into(), net: NetId(0), at: Rect { x, y: 450, w: 100, h: 100 }, layer: LayerId(0) };
        Macro { pins: vec![pin("D", 1_900), pin("S", 0)], bbox: Rect { x: 0, y: 0, w: 2_000, h: 1_000 }, ..Default::default() }
    }

    #[test]
    fn round_up_boundaries() {
        assert_eq!((crate::round_up(0, 10), crate::round_up(1, 10), crate::round_up(10, 10), crate::round_up(11, 10)), (0, 10, 10, 20));
        assert_eq!(crate::round_up(7, 1), 7);
    }

    #[test]
    fn nearest_face_ties_go_to_the_earlier_face_and_outside_pins_count() {
        let m = Macro { bbox: Rect { x: 0, y: 0, w: 100, h: 100 }, ..Default::default() };
        assert_eq!(crate::nearest_face(&m, Rect { x: 40, y: 40, w: 20, h: 20 }), 0, "centre: L wins the tie");
        assert_eq!(crate::nearest_face(&m, Rect { x: 150, y: 40, w: 20, h: 20 }), 2, "beyond R");
        assert_eq!(crate::nearest_face(&m, Rect { x: 40, y: 95, w: 10, h: 10 }), 3, "T");
        assert_eq!(crate::nearest_face(&m, Rect { x: 40, y: -5, w: 10, h: 10 }), 1, "B");
    }

    #[test]
    fn face_pins_count_each_pin_on_its_nearest_face() {
        let v = vec![gp::VariantSpace { alternatives: vec![cell(), Macro::default()] }];
        assert_eq!(crate::face_pins(&v), vec![vec![[1, 0, 1, 0], [0; 4]]]);
        assert!(crate::face_pins(&[]).is_empty());
    }

    /// The halo starts strictly past one minimum-width wire's current; a
    /// missing or unknown pin current and a zero J reserve nothing.
    #[test]
    fn static_halo_threshold_is_strict() {
        let v = vec![gp::VariantSpace { alternatives: vec![cell()] }];
        let at = |ua: Option<i32>| vec![vec![("D".to_string(), ua)]];
        assert_eq!(crate::static_halos(&v, &at(Some(200)), 2.0, 100, 10), vec![vec![[0; 4]]], "I = j·w_min");
        assert_eq!(crate::static_halos(&v, &at(Some(201)), 2.0, 100, 10), vec![vec![[0, 0, 10, 0]]], "1 nm over, rounded to the lattice");
        assert_eq!(crate::static_halos(&v, &at(None), 2.0, 100, 10), vec![vec![[0; 4]]]);
        assert_eq!(crate::static_halos(&v, &at(Some(1_000_000)), 0.0, 100, 10), vec![vec![[0; 4]]], "no J, no halo");
        assert_eq!(crate::static_halos(&v, &[vec![]], 2.0, 100, 10), vec![vec![[0; 4]]], "pin not in the table");
    }

    #[test]
    fn halo_target_ignores_unknown_and_sub_capacity_demand() {
        assert_eq!(crate::halo_target(f32::NAN, 2, 420, 10), 0);
        assert_eq!(crate::halo_target(0.5, 2, 420, 10), 0);
        assert_eq!(crate::halo_target(2.0, 1, 420, 10), 420);
    }

    /// Any demand, however large, ends at the `4·p0` cap without overflow.
    #[test]
    fn huge_demand_is_capped_without_overflow() {
        let t = crate::halo_target(1e9, 1_000, 420, 10);
        assert!(t >= 1_680);
        assert_eq!(crate::halo_step(0, t, 420, 10), 1_680);
        assert_eq!(crate::halo_step(i32::MAX, i32::MAX, 420, 10), 1_680);
    }

    #[test]
    fn dynamic_halo_decays_with_no_congestion_and_ignores_an_empty_table() {
        let l = pnr_core::Layout {
            x: vec![10_000],
            y: vec![10_000],
            hw: vec![1_000],
            hh: vec![500],
            axis: vec![0],
            groups: vec![],
            orient: vec![pnr_core::Orient::R0],
            variant: vec![0],
            branch: Vec::new(),
            power_uw: vec![0],
            temp_mc: vec![0],
            units: Default::default(),
        };
        let mut halo = vec![[800, 0, 30, 0]];
        crate::update_dyn_halo(&mut halo, &l, &[], &[], 420, 10);
        assert_eq!(halo, vec![[400, 0, 20, 0]]);
        let mut none: Vec<[i32; 4]> = Vec::new();
        crate::update_dyn_halo(&mut none, &l, &[], &[], 420, 10);
        assert!(none.is_empty());
    }

    // ---- per-device tables ----

    /// AS/AD/PS/PD per device: S into 0/2, D into 1/3, ÷ m, nm² → µm²,
    /// nm → µm; a non-MOS divides by 1; owners outside the cell and devices
    /// outside the netlist are skipped; an unnamed device is `None`.
    #[test]
    fn junctions_split_s_and_d_and_divide_by_m() {
        let nl = netlist(
            vec![dev("M0", DeviceKind::Nmos, &[("w", 1000), ("l", 150), ("m", 2)]), dev("R1", DeviceKind::Resistor, &[]), dev("M2", DeviceKind::Nmos, &[("w", 1000), ("l", 150)])],
            &[],
        );
        let mut m = Macro::default();
        m.figures.sd = vec![(0, "S", 2_000_000, 4_000), (0, "D", 1_000_000, 2_000), (1, "S", 3_000_000, 6_000), (5, "D", 9, 9), (2, "D", 9, 9)];
        let devices_of = vec![vec![DeviceId(0), DeviceId(1), DeviceId(40)]];
        let j = crate::junctions(&[m], &devices_of, &nl);
        assert_eq!(j, vec![Some([1.0f64, 0.5, 2.0, 1.0]), Some([3.0, 0.0, 6.0, 0.0]), None]);
        assert_eq!(crate::junctions(&[], &[], &nl), vec![None::<[f64; 4]>; 3]);
    }

    #[test]
    fn gate_ohms_map_owners_and_skip_strays() {
        let mut m = Macro::default();
        m.figures.gate_ohm = vec![(1, 12.5), (0, 3.0), (7, 1.0)];
        let g = crate::gate_ohms(&[m], &[vec![DeviceId(2), DeviceId(0), DeviceId(9)]], 3);
        assert_eq!(g, vec![Some(12.5f64), None, Some(3.0)]);
        assert!(crate::gate_ohms(&[], &[], 0).is_empty());
    }

    #[test]
    fn with_extra_appends_inserted_devices() {
        let nl = netlist(vec![dev("M0", DeviceKind::Nmos, &[])], &[]);
        let n = crate::with_extra(&nl, &[dev("D1", DeviceKind::Diode, &[])]);
        assert!(matches!(n, std::borrow::Cow::Owned(_)));
        assert_eq!(n.devices.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(), ["M0", "D1"]);
    }

    #[test]
    fn pin_member_edge_cases() {
        assert_eq!(crate::pin_member("d:S"), None, "no ordinal");
        assert_eq!(crate::pin_member("d12:G"), Some((12, "G")));
        assert_eq!(crate::pin_member("x1:G"), None, "not a d ordinal");
        assert_eq!(crate::pin_member("ring"), Some((0, "ring")), "a bare name is member 0's (member_pin filters it)");
    }

    #[test]
    fn cell_flags_separate_noise_from_injection() {
        use analog::intent::{Aggressor, Inject, Intent};
        let intent = Intent {
            aggressors: vec![Aggressor { device: DeviceId(0), inject: Inject::Switching, reason: "" }, Aggressor { device: DeviceId(1), inject: Inject::MinorityElectron, reason: "" }],
            ..Default::default()
        };
        let f = crate::cell_flags(&intent, &[vec![DeviceId(0)], vec![DeviceId(1)], vec![]]);
        assert_eq!((f[0].noisy, f[0].injector, f[1].noisy, f[1].injector), (true, false, true, true));
        assert_eq!(f[2], Default::default());
        assert!(crate::cell_flags(&intent, &[]).is_empty());
    }

    /// A non-finite current is unknown (`None`), never a known 0 µA; only
    /// member 0 gets bare names.
    #[test]
    fn pin_currents_treat_a_non_finite_current_as_unknown() {
        let mut a = dev("M0", DeviceKind::Nmos, &[]);
        a.terminals = vec![("D".into(), NetId(0))];
        let b = a.clone();
        let nl = netlist(vec![a, b], &["n"]);
        let draws = [Some(vec![("D".to_string(), f64::NAN)]), Some(vec![("D".to_string(), 2.6)])];
        let pins = crate::pin_currents(&nl, &[vec![DeviceId(0), DeviceId(1)]], &draws);
        assert_eq!(pins, vec![vec![("d0:D".to_string(), None), ("D".to_string(), None), ("d1:D".to_string(), Some(3))]]);
        assert!(crate::pin_currents(&nl, &[], &draws).is_empty());
        assert!(crate::pin_currents(&nl, &[vec![]], &draws)[0].is_empty());
    }

    #[test]
    fn undrawable_skips_drawn_cells_and_names_unknown_members() {
        let nl = netlist(vec![dev("M0", DeviceKind::Npn, &[])], &[]);
        let drawn = Macro { shapes: vec![Shape { layer: LayerId(0), rect: Rect { x: 0, y: 0, w: 1, h: 1 } }], ..Default::default() };
        let rows: Vec<String> = crate::undrawable(&[drawn, Macro::default(), Macro::default()], &[vec![DeviceId(0)], vec![DeviceId(7)]], &nl).map(|v| v.rule).collect();
        assert_eq!(rows.len(), 2);
        assert!(rows[0].starts_with("cell/undrawable: ?"), "{rows:?}");
        assert!(rows[1].starts_with("cell/undrawable: ?"), "cell outside devices_of: {rows:?}");
    }

    #[test]
    fn adopt_devices_appends_in_order() {
        let mut nl = netlist(vec![dev("M0", DeviceKind::Nmos, &[])], &[]);
        let mut macros = vec![Macro::default()];
        crate::adopt_devices(&mut nl, &mut macros, vec![(dev("D1", DeviceKind::Diode, &[]), Macro::default()), (dev("D2", DeviceKind::Diode, &[]), Macro::default())]);
        assert_eq!(nl.devices.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(), ["M0", "D1", "D2"]);
        assert_eq!(macros.len(), 3);
        crate::adopt_devices(&mut nl, &mut macros, Vec::new());
        assert_eq!((nl.devices.len(), macros.len()), (3, 3));
    }

    // ---- deck reads ----

    fn sky130() -> verify::Pdk {
        verify::Pdk::builtin("sky130").expect("sky130 loads")
    }

    #[test]
    fn model_table_files_sky130_fets_by_polarity() {
        let t = crate::model_table(&sky130());
        let kind = |needle: &str| t.iter().find(|(m, _)| m.contains(needle)).map(|e| e.1);
        assert_eq!(kind("nfet_01v8"), Some(DeviceKind::Nmos));
        assert_eq!(kind("pfet_01v8"), Some(DeviceKind::Pmos));
    }

    #[test]
    fn deck_models_keep_an_unknown_model_as_written() {
        let mut nl = netlist(vec![Device { model: "no_such_model_xyz".into(), ..dev("M0", DeviceKind::Nmos, &[]) }], &[]);
        crate::deck_models(&mut nl, &sky130());
        assert_eq!(nl.devices[0].model, "no_such_model_xyz");
    }

    #[test]
    fn ring_cut_ohm_is_never_negative() {
        assert!(crate::ring_cut_ohm(&sky130()) >= 0.0);
    }

    #[test]
    fn placement_space_reads_null_absent_and_entries() {
        let mut pdk = sky130();
        pdk.cell["placement_space"] = serde_json::Value::Null;
        assert!(crate::placement_space(&pdk).is_empty());
        pdk.cell["placement_space"] = serde_json::json!({" a , b ": [120, "src"]});
        assert_eq!(crate::placement_space(&pdk), [("a".to_string(), "b".to_string(), 120)]);
    }

    #[test]
    #[should_panic(expected = "cell.placement_space.ab")]
    fn placement_space_panics_on_a_key_without_a_comma() {
        let mut pdk = sky130();
        pdk.cell["placement_space"] = serde_json::json!({"ab": [1, "x"]});
        crate::placement_space(&pdk);
    }

    #[test]
    #[should_panic(expected = "cell.placement_space.a,b")]
    fn placement_space_panics_on_a_non_integer_value() {
        let mut pdk = sky130();
        pdk.cell["placement_space"] = serde_json::json!({"a,b": ["wide", "x"]});
        crate::placement_space(&pdk);
    }

    #[test]
    fn em_front_row_defaults_to_true() {
        let mut pdk = sky130();
        pdk.cell["em_front_row_cuts"] = serde_json::Value::Null;
        assert!(crate::em_front_row(&pdk));
        pdk.cell["em_front_row_cuts"] = serde_json::json!(false);
        assert!(!crate::em_front_row(&pdk));
    }

    #[test]
    fn em_rules_with_every_layer_limited_add_no_missing_row() {
        let pdk = sky130();
        let nl = crate::parse(ONE_FET).unwrap();
        let mut p = annotator::annotate(&nl, &crate::annotation(&pdk, &Default::default()));
        let rows = |p: &crate::Problem| p.missing.iter().filter(|m| m.0 == "Electromigration").count();
        let before = rows(&p);
        crate::em_rules(&mut p, &nl, &[], &[], &[], None, true, &pdk);
        assert_eq!(rows(&p), before, "{:?}", p.missing);
    }

    /// One label per net, only where the pin centre is on drawn conductor of
    /// its layer, and only for a net that has a name.
    #[test]
    fn labeled_pins_label_each_provable_net_once() {
        let pdk = sky130();
        let Some(&(_, l)) = pdk.layers.iter().find(|(_, l)| verify::geom::label_layer(&pdk.deck, l.0).is_some()) else {
            panic!("sky130 has a label layer");
        };
        let pin = |net: u16, x: i32| Pin { name: "p".into(), net: NetId(net), at: Rect { x, y: 40, w: 20, h: 20 }, layer: l };
        let shapes = [Shape { layer: l, rect: Rect { x: 0, y: 0, w: 100, h: 100 } }];
        let placed = [Macro { pins: vec![pin(0, 40), pin(0, 60), pin(1, 500), pin(5, 40)], ..Default::default() }];
        let got = crate::labeled_pins(&placed, &["a".into(), "b".into()], &pdk, &shapes);
        assert_eq!(got.iter().map(|p| (p.name.as_str(), p.layer, p.x, p.y)).collect::<Vec<_>>(), [("a", l.0, 50, 50)]);
        assert!(crate::labeled_pins(&[], &[], &pdk, &shapes).is_empty());
    }

    // ---- flow error paths ----

    #[test]
    fn an_injected_fet_that_is_not_one_device_is_refused() {
        let pdk = sky130();
        let name = crate::parse(ONE_FET).unwrap().devices[0].name.clone();
        let mut injected = crate::Macros::default();
        injected.register(&name, Macro::default());
        match crate::run(ONE_FET, &pdk, &injected, &Config::default()) {
            Err(FlowError::InjectedNotADevice(n, count)) => {
                assert_eq!(n, name);
                assert_ne!(count, Some(1));
            }
            r => panic!("expected InjectedNotADevice, got {:?}", r.err()),
        }
    }

    #[test]
    fn post_layout_spice_refuses_a_device_it_cannot_extract() {
        let pdk = sky130();
        let cfg = Config { feedback_iters: 1, outer_iters: 1, starts: 1, ..Default::default() };
        let mut sol = crate::run(ONE_FET, &pdk, &Default::default(), &cfg).expect("flow");
        sol.netlist.devices.push(dev("C9", DeviceKind::Capacitor, &[]));
        let e = crate::post_layout_spice(&sol, &pdk, "top").expect_err("a capacitor is not extracted");
        assert!(e.contains("C9") && e.contains("not extracted"), "{e}");
    }
}
