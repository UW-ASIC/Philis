//! # `library` — the pipeline as one reusable, deterministic function.
//!
//! `.sp` + PDK go in, a placed-and-routed solution comes out:
//!
//! ```text
//! parse ─▶ Netlist ─into_bipartite_hypergraph()─▶ (carrier)
//!                     │
//!            annotate(netlist)  ── the annotator attaches the applied rule-sets
//!                     ▼          (placement/routing Requirements) + the group DAG
//!                  Problem
//!         ┌──────────┼─────────────────────────────┐
//!         ▼          ▼                              ▼
//!   cells::draw   gp → dp (placement)          gr → dr (routing)
//!  (macros, PDK)  reqs.hard/cost + layers      reqs + layers
//!                     │                              │
//!                     └──────────────┬───────────────┘
//!                          verify (in-loop DRC → Hard rules)  ──▶ signoff
//! ```
//!
//! The **only** place a concrete stage algorithm is named is the hot-swap point
//! inside [`run`] — change those four lines to swap a placer/router. Everything
//! else is algorithm-agnostic (it talks to the `gp/dp/gr/dr` traits) and
//! analog-agnostic (the theory rides in through `reqs`).

#![allow(dead_code)]

mod cellgen;
mod geometry;
mod parse;

/// GDSII stream writer — flat [`pnr_core::Shape`]s → conformant GDSII bytes, for
/// tape-out artifacts and SVG rendering via [`visualizer::export_svg`].
pub mod gds;
/// Constraint-budget reporting: met, met-without-margin, or violated per family.
pub mod metadata;
/// DC operating point via ngspice — the per-device power and bias current the
/// thermal and electromigration rules need.
pub mod oppoint;

use analog::RuleBatch;
use annotator::{annotate, AnnotationConfig, NoInference};
pub use macro_master::Macros;
/// The GPU GDS/geometry viewer, re-exported so callers get it through the one
/// front-door crate (`show_macro`, `run_viewer`, `export_svg`, `Probe`, …).
pub use visualizer;
use dp::DetailedPlacer;
use dr::DetailedRouter;
use gp::GlobalPlacer;
use gr::GlobalRouter;
use pnr_core::hypergraph::IntoBipartiteHypergraph;
use pnr_core::{Layout, Macro, Routes};
use verify::Pdk;

/// Repeatable-run configuration.
pub struct Config {
    /// Base RNG seed. Each feedback iteration derives a fresh seed from it.
    pub seed: u64,
    /// **Middle-tier** budget: feasibility epochs at a *fixed* variant assignment
    /// (place → route → in-loop DRC → fold back). `1` disables feedback.
    ///
    /// This is the tier that drives positions to feasibility. It cannot fix a
    /// circuit whose chosen variants make the constraints unsatisfiable at *any*
    /// position — that is what the outer tier is for.
    pub feedback_iters: u32,
    /// **Outer-tier** budget: variant/topology escalations (PLAN §2).
    ///
    /// A variant change relocates pins, so it changes routability and extracted
    /// parasitics discontinuously — it is a jump between search spaces, not a step
    /// within one. Each outer iteration re-runs the whole middle tier. `1` fixes the
    /// variant assignment at whatever pricing chose and reproduces the old
    /// single-tier behaviour exactly.
    pub outer_iters: u32,
    /// Recognition overrides: force-skip devices (`do_not_identify`) or patterns
    /// (`do_not_use`), or tag extra rail/clock nets. The user's channel to override
    /// the annotator's automatic block decisions (ALIGN `DoNotIdentify`).
    pub annotation: AnnotationConfig,
    /// Steady-state dissipation per device, µW, indexed like `netlist.devices`.
    ///
    /// This is the **thermal solver's input**, and the one piece of data that
    /// makes `ThermalGradient` bite: with it, the placer computes a real ΔT per
    /// matched pair and moves partners onto a shared isotherm; without it the die
    /// is uniform, every ΔT is `0`, and the rule passes trivially.
    ///
    /// Left empty by default because an operating point cannot be recovered from
    /// a netlist without simulating it — a fabricated estimate would be worse
    /// than an honest zero. Supply it directly, or set [`Config::op`] and let
    /// ngspice solve for it.
    pub device_power_uw: Vec<i32>,
    /// Solve the netlist's DC operating point with ngspice and use it as the
    /// thermal power map.
    ///
    /// This is what turns `ThermalGradient` from a rule that passes trivially
    /// into one that actually constrains placement. Takes precedence over
    /// [`Config::device_power_uw`] for every device the simulation resolves. If
    /// ngspice is missing or the solve fails the flow continues on zero power,
    /// and the metadata report states the bias is unknown rather than implying a
    /// thermal pass.
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

/// A finished placement + routing (pre-signoff). `macros` are kept so signoff and
/// GDS emission can rebuild the flat geometry.
pub struct Solution {
    pub layout: Layout,
    pub routes: Routes,
    pub macros: Vec<Macro>,
    /// The parsed schematic — kept so signoff (LVS) has its reference.
    pub netlist: pnr_core::Netlist,
    /// The LVS reference netlist, built **once** in [`run`] (it is a pure
    /// function of the schematic) and reused by the per-epoch LVS feedback and
    /// by [`signoff`] — which used to rebuild it per call.
    pub reference: verify::lvs::RefNetlist,
    /// Feedback-loop telemetry for the *winning* iteration (for benchmarking).
    pub stats: RunStats,
    /// Per-family budget status (met / met-without-margin / violated) plus the
    /// electrical bias it was judged under.
    pub metadata: metadata::MetadataReport,
}

/// Feedback-loop telemetry: how the run converged and the winning iteration's
/// per-stage legality. `run` hides the loop, so this is how a caller (e.g. the
/// benchmark) sees convergence quality without re-running anything.
#[derive(Clone, Copy, Debug, Default)]
pub struct RunStats {
    /// Middle-tier epochs actually executed, summed over every outer iteration.
    pub iterations: u32,
    /// 0-based index of the iteration whose solution was kept.
    pub best_iteration: u32,
    /// `true` if the loop stopped on the stall/patience criterion (converged),
    /// `false` if it exhausted the iteration budget.
    pub converged: bool,
    /// Outer-tier iterations executed (`1..=cfg.outer_iters`).
    pub outer_iterations: u32,
    /// How many times the middle tier stalled while still infeasible and the outer
    /// tier answered by moving the variant assignment.
    ///
    /// This is PLAN §5's central diagnostic made observable: a nonzero count means
    /// the run hit a **variant-space binding**, i.e. no arrangement of the
    /// previously-chosen variants could satisfy the constraints. That is a different
    /// failure from a placement local minimum and the two are otherwise
    /// indistinguishable from the outside.
    pub variant_escalations: u32,
    /// Winning iteration's Θ — total budget residual. `0.0` with a clean
    /// `hard_violations` count is the feasibility half of termination.
    pub theta: f64,
    /// `‖λ_{k+1} − λ_k‖` at exit. Near zero ⇒ constraint prices are stationary ⇒ no
    /// budget is being actively traded (PLAN §5). A run that is *feasible* but whose
    /// prices are still drifting has not converged, it is mid-negotiation.
    pub price_drift: f64,
    /// Σ `h_n` at exit — accumulated routing negotiation pressure. Still climbing ⇒
    /// resources are still contested.
    pub route_pressure: f64,
    /// Winning iteration: detailed-placement hard-violation count.
    pub place_hard: usize,
    /// Winning iteration: detailed-routing hard-violation count.
    pub route_hard: usize,
    /// Winning iteration: live-DRC feedback hard rules folded forward.
    pub drc_hard: usize,
    /// Winning iteration: residual routing track overuse (from the router report).
    pub route_overuse: i64,
}

/// Anything that stops the flow.
#[derive(Debug)]
pub enum FlowError {
    Parse(String),
    /// Zero feedback iterations produced nothing.
    Empty,
}

/// Run place-and-route for a SPICE netlist against a PDK. **Deterministic** for
/// `cfg.seed`.
///
/// The bipartite hypergraph is the carrier: [`annotate`] walks it (via each
/// rule's `extract`) and returns the applied `Requirements` + block-DAG + group
/// table, which the stages consume. `verify` runs in the loop — its DRC findings
/// return as **Hard** rules merged into the next iteration's placement
/// requirements — so verification *drives* convergence.
///
/// `injected` is the user's `macro_master` registry: any part of the circuit with
/// a macro registered under its instance name is used **as-is** instead of
/// auto-drawn (and its DAG block is flagged `injected`). Pass
/// `&Macros::default()` for a fully auto-generated run.
pub fn run(spice: &str, pdk: &Pdk, injected: &Macros, cfg: &Config) -> Result<Solution, FlowError> {
    let netlist = parse::spice(spice).map_err(FlowError::Parse)?;

    // The bipartite hypergraph is the pipeline's carrier; `annotate` walks it (via
    // each rule's `extract`) to attach the applied rule-sets.
    let _hypergraph = netlist.into_bipartite_hypergraph();

    // The problem is a **pure function of the netlist**, so derive it once per run.
    //
    // It used to be re-derived every epoch, but not for any reason of its own: the
    // in-loop DRC feedback was appended into `problem.placement.hard`, which is a
    // mutation, and `Requirements` cannot be cloned (boxed trait objects). The loop
    // below instead truncates the feedback batches off the end after each epoch, so
    // the base rule-set is reused untouched and the annotator runs once instead of
    // once per epoch plus once more at the end.
    let mut problem = annotate(&netlist, &NoInference, &cfg.annotation);
    for u in &problem.constraints.unitization {
        eprintln!(
            "[unit] devices={:?} unit_w={} unit_l={} dev_nf={:?} kind={:?}",
            u.devices.iter().map(|d| d.0).collect::<Vec<_>>(),
            u.unit_w, u.unit_l, u.dev_nf, u.device_type
        );
    }

    // Offload "which parts are user-provided" onto the DAG: a block whose devices
    // are all covered by injected macros is a pre-drawn fixed node. Loop-invariant
    // (it depends on the netlist and the user's registry, neither of which moves),
    // so it is decided once here rather than re-derived per epoch.
    for b in &mut problem.blocks {
        b.injected = !b.devices.is_empty()
            && b.devices
                .iter()
                .all(|d| injected.get(&netlist.devices[d.0 as usize].name).is_some());
    }

    // Pinned cells = those in injected (user-macro) blocks. `dp` keeps them at their
    // coarse position and never reshapes them — a user's geometry is the user's.
    // Device-indexed here; mapped to cell length after the collapse below (injected
    // devices never merge, so any-member-injected == all-members-injected).
    let mut fixed_dev = vec![false; netlist.devices.len()];
    for b in &problem.blocks {
        if b.injected {
            for d in &b.devices {
                if let Some(f) = fixed_dev.get_mut(d.0 as usize) {
                    *f = true;
                }
            }
        }
    }

    // ---- electrical bias -------------------------------------------------
    // Solve the DC operating point once: it depends on the schematic, not on
    // where anything is placed, so it is loop-invariant. This is the only source
    // of per-device power, and therefore the only thing that makes the thermal
    // gradient a real constraint rather than a vacuous one.
    let op = cfg.op.as_ref().and_then(|oc| match oppoint::extract(spice, &netlist, oc) {
        Ok(o) => {
            eprintln!(
                "[op] {} of {} devices solved, {} µW total ({})",
                o.resolved, netlist.devices.len(), o.total_power_uw(), o.provenance
            );
            Some(o)
        }
        Err(e) => {
            // Never fatal: an unknown bias means a uniform die, which the report
            // will state plainly.
            eprintln!("[op] operating point unavailable ({e}); continuing with zero power");
            None
        }
    });
    let power = op
        .as_ref()
        .map(|o| o.power_uw.clone())
        .unwrap_or_else(|| cfg.device_power_uw.clone());
    let bias = op.as_ref().map(|o| {
        let hottest = o
            .power_uw
            .iter()
            .enumerate()
            .max_by_key(|(_, &p)| p)
            .filter(|(_, &p)| p > 0)
            .map(|(i, &p)| (netlist.devices[i].name.clone(), p));
        metadata::BiasSummary {
            provenance: o.provenance.clone(),
            resolved: o.resolved,
            devices: netlist.devices.len(),
            total_power_uw: o.total_power_uw(),
            hottest,
        }
    });

    // The layers the routers may use — the routable metal stack only. The full
    // deck table would put wires on nwell/diff/poly (see `Pdk::routing_layers`).
    // `cuts` is the matching via stack; without it a layer change draws no cut and
    // the deck's `lvs_cut_required` leaves each metal layer an isolated island.
    //
    // The stack stops at the first layer whose `min_width` exceeds the router's
    // single global wire width: drawing a 290 nm wire on met3 (which wants 300)
    // is a `min_width` violation on every segment. Upper layers are unreachable
    // rather than illegal — for these circuit sizes li/met1/met2 is ample.
    //
    // ponytail: one wire width for the whole stack. Lift this by giving `dr` a
    // per-layer width/pitch, which means a per-layer track lattice in
    // `gr::TrackGrid`; worth it only when a design actually needs the upper metals.
    let wire_w = dr::DetailedCfg::default().wire_width;
    let layers: Vec<_> = pdk
        .routing_layers()
        .into_iter()
        .take_while(|l| pdk.min_width(l.0).is_none_or(|w| w <= wire_w))
        .collect();
    let mut cuts = pdk.routing_vias();
    cuts.truncate(layers.len().saturating_sub(1));

    // Everything below indexes these two tables by track-layer index, so state
    // what they must satisfy here rather than discovering it as DRC noise later.
    assert!(
        !layers.is_empty(),
        "no routable layer: the deck declares no conductor that is not also a \
         device-formation layer, so there is nowhere legal to put a wire"
    );
    assert_eq!(
        cuts.len(),
        layers.len() - 1,
        "every adjacent pair of routing layers needs the cut that joins them; \
         without it the deck's lvs_cut_required leaves those layers isolated"
    );
    for (i, &(cut, size, below, above)) in cuts.iter().enumerate() {
        assert!(
            size <= below && size <= above,
            "cut {cut:?} is {size} nm but its landing pads are {below}/{above} nm — \
             a pad smaller than its cut cannot enclose it"
        );
        assert!(
            !layers.contains(&cut),
            "cut {cut:?} (joining {:?} and {:?}) is also in the routing stack; a \
             layer cannot be both wire metal and a via cut",
            layers[i],
            layers[i + 1]
        );
    }

    // ── THE hot-swap point: the concrete algorithms, named directly. Change a
    //    line here (e.g. `gp::Analytical` → your placer) to swap; nothing else in
    //    the codebase names an algorithm. ──
    let placer = gp::Analytical::default();
    let refiner = dp::Annealer::default();
    let g_router = gr::GlobalRoute::default();
    // Track pitch comes from the deck, not the ported default: one global pitch
    // has to clear the *worst* layer's spacing or that layer cannot be routed
    // legally at all (see `Pdk::routing_pitch`).
    let d_router = {
        let mut cfg = dr::DetailedCfg::default();
        let stack: Vec<_> = layers.iter().map(|l| l.0).collect();
        cfg.pitch = cfg.pitch.max(pdk.routing_pitch(cfg.wire_width, &stack));
        // Two wires on adjacent tracks are `pitch - wire_width` apart. If that is
        // under any routed layer's min_spacing, those tracks are illegal *by
        // construction* and no amount of rerouting can clear the DRC.
        for l in &layers {
            let need = pdk.min_spacing(l.0).unwrap_or(0);
            assert!(
                cfg.pitch - cfg.wire_width >= need,
                "track pitch {} with {} nm wires leaves {} nm between adjacent \
                 tracks, but layer {:?} needs {need} nm",
                cfg.pitch,
                cfg.wire_width,
                cfg.pitch - cfg.wire_width,
                l
            );
            assert!(
                pdk.min_width(l.0).is_none_or(|w| cfg.wire_width >= w),
                "wire width {} is under layer {:?}'s min_width — every segment \
                 drawn there is a violation",
                cfg.wire_width,
                l
            );
        }
        dr::DetailedRoute { cfg }
    };

    // ---- the variant space, drawn once ----------------------------------
    // Every legal joint variant of every cell, drawn up front. `(group, variant) ->
    // Macro` is pure and byte-deterministic, so this table is built once and reused
    // for every trial move of every epoch — see `docs/API-WISH.md` D7 for why this
    // is data rather than a redraw callback.
    //
    // This is also where the group collapse lands (PLAN §2): a matched unitization
    // is ONE cell from here on, so everything below runs in **cell space**
    // (`n_cells ≤ n_devices`) and every device-indexed table is translated through
    // `cell_of` / `devices_of` once, right here — loop-invariant, like the tables
    // themselves.
    let cellgen::Cells { spaces: variants, cell_of, devices_of } =
        cellgen::enumerate(&netlist, injected, &problem.constraints, pdk);

    // Re-point every placement rule at the collapsed cell space. Feedback batches
    // are appended *after* this (inside the loop) and are already cell-indexed —
    // they come from drawn geometry — so retargeting once here, before `base_hard`
    // is read, is what keeps them out of the sweep.
    for b in problem
        .placement
        .hard
        .iter_mut()
        .chain(problem.placement.budget.iter_mut())
        .chain(problem.placement.cost.iter_mut())
    {
        b.retarget(&cell_of);
    }
    #[cfg(debug_assertions)]
    debug_check_retargeted(&problem, variants.len(), &cell_of);

    // Injected-block pins, now cell-length. Injected devices never merge, so
    // any-member == all-members.
    let fixed: Vec<bool> =
        devices_of.iter().map(|m| m.iter().any(|d| fixed_dev[d.0 as usize])).collect();

    // Both group tables rewritten into cell space: GroupIds stay valid (rules keep
    // pointing at the same rows), members become cells, duplicates collapse. A
    // matched group fully merged into one macro shrinks to one member — which
    // grants no abutment and boxes a single cell, both correct: the abutment is
    // now drawn inside the macro.
    //
    // `problem.abutment` is consumed as the annotator hands it (D16): what a group
    // *means* — and which subset may share diffusion — is recognition semantics,
    // and the local single-kind filter this used to recompute was exactly how the
    // two tables came to be conflated.
    let abutment_groups: Vec<Vec<pnr_core::DeviceId>> =
        problem.abutment.iter().map(|g| remap_members(g, &cell_of)).collect();
    let cell_groups: Vec<Vec<pnr_core::DeviceId>> =
        problem.groups.iter().map(|g| remap_members(g, &cell_of)).collect();

    // Guard-ring requirements land on cells too: rewrite each requester through
    // `cell_of` and drop duplicates that collapse onto one cell (two members of a
    // merged pair asking for the same ring is one ring around one macro).
    let constraints_cells = {
        let mut c = problem.constraints.clone();
        for r in &mut c.guard_rings {
            if let Some(&ci) = cell_of.get(r.device.0 as usize) {
                r.device = pnr_core::DeviceId(ci);
            }
        }
        let mut seen: Vec<u16> = Vec::new();
        c.guard_rings.retain(|r| {
            if seen.contains(&r.device.0) {
                false
            } else {
                seen.push(r.device.0);
                true
            }
        });
        c
    };

    // The LVS reference is a pure function of the schematic: build it once, use
    // it for the per-epoch LVS feedback below and hand it to `signoff` through
    // the `Solution` (it used to be rebuilt there per call).
    let reference = cellgen::reference(&netlist);

    // The live oracle (PLAN §1's oracle service, D4-revised): `dp` consumes it
    // per accepted move / per epoch — risk-gated, bbox-scoped, budget-capped.
    // Handed as a flat parameter (D1); rules stay pure functions of `Layout`.
    let oracle = verify::LiveOracle { pdk };

    // Seed the assignment by *pricing*: draw each hypothesis and measure it, rather
    // than guessing from footprint area. PLAN §2 recommends the hybrid — price up
    // front to seed, then allow variant moves once placement feedback shows the
    // priced choice is dominated. Pricing alone is myopic (it cannot know where the
    // device lands); moves alone waste effort on obviously dominated variants.
    let mut assignment = cellgen::seed_assignment(&variants, &layers, pdk);

    // ---- cross-epoch search state, owned here ---------------------------
    // These three are the reason the loop converges rather than oscillating, and
    // they all live at this level because the *terminator* has to read them
    // (PLAN §5). A stage that owned its own copy would reset it every call, which is
    // precisely the bug: `gr`'s PathFinder history was being discarded every epoch.
    let mut neg = gr::Negotiation::new();
    let mut prices = gp::Prices::new();
    // In-loop DRC feedback, carried forward as extra Hard rules.
    let mut drc_hard: Vec<Box<dyn RuleBatch<Layout>>> = Vec::new();

    // The base rule count, so each epoch's feedback batches can be truncated back
    // off without re-deriving the whole problem.
    let base_hard = problem.placement.hard.len();

    // Keep the lexicographically-best solution seen (PLAN §3b: V, then Θ, then PEX)
    // and stop when it stalls — convergence, not just "first DRC-clean".
    let mut best: Option<(LexKey, Layout, Routes, Vec<Macro>, RunStats)> = None;
    let patience = 8u32; // epochs without improvement before the middle tier gives up
    let mut converged = false;
    let mut iterations = 0u32;
    let mut outer_iterations = 0u32;
    let mut variant_escalations = 0u32;

    // ═══ OUTER TIER: discrete search over the variant assignment ═══════════
    //
    // Each outer iteration fixes an assignment and hands it to the middle tier. The
    // tiers are separated by a **separation-of-timescales** argument (PLAN §1):
    // variant choices are discrete and low-cardinality after constraint collapse, so
    // they change rarely; position feasibility is a medium-frequency problem; and
    // parasitic polishing is high-frequency and local, which is why the third tier
    // lives *inside* `dp` as its move loop rather than as a loop here.
    'outer: for outer in 0..cfg.outer_iters.max(1) {
        outer_iterations = outer + 1;
        // Middle-tier stall counter. Reset per outer iteration: a fresh assignment
        // is a fresh search space and inherits none of the previous one's staleness.
        let mut stall = 0u32;

        // ═══ MIDDLE TIER: drive positions to feasibility at fixed V ════════
        for iter in 0..cfg.feedback_iters.max(1) {
            iterations += 1;

            // Fold in the previous epoch's DRC findings as Hard legality the placer
            // must now respect, then (at the end of the epoch) truncate them back off.
            // This is D4-revised's epoch half: the oracle measures, we bake the
            // measurement into a rule batch, and the stage consumes it as an
            // ordinary requirement. The fold **stays** alongside dp's live oracle
            // handle — the in-epoch veto prevents *new* findings, this is the
            // cross-epoch repair pressure on the ones that predate the epoch.
            problem.placement.hard.truncate(base_hard);
            problem.placement.hard.append(&mut drc_hard);

            // New seed per epoch so a stuck run doesn't replay one trajectory. Mixed
            // with `outer` too, or every assignment would retrace the same trajectory.
            let seed = cfg.seed ^ u64::from(iter) ^ (u64::from(outer) << 32);

            // The geometry this epoch actually uses: the chosen alternative per cell.
            let macros = cellgen::realize(&variants, &assignment);

            // Placement: coarse (analytical) → detailed (legalising SA + reshape).
            let (mut coarse, _) =
                placer.place(&macros, &variants, &problem.placement, &layers, &mut prices, seed);
            coarse.debug_check("gp::place");
            // Thread the group table and thermal source data *before* detailed
            // placement, not after. `dp` needs both while it anneals: groups tell it
            // which overlaps are intentional abutment and let `Target::Group` rules
            // resolve, and the power map is what turns `ThermalGradient` from a
            // tautology into a live constraint. Assigning them to the finished layout
            // would be too late to affect any of it.
            // `Layout::groups` has TWO consumers with different semantics, and each
            // side of the `dp` call gets the table it means:
            //  - pre-dp (here): `problem.abutment` remapped — diffusion-sharing
            //    permission. `dp`'s legalizer preserves in-group overlap as
            //    intentional abutment and `rotatable` refuses to turn a grouped
            //    device, so this table must never contain a mixed-polarity
            //    composite (implants would merge; DRC-clean, LVS-fatal).
            //  - post-dp (below): `problem.groups` remapped — the recognition
            //    table, composites included, so `Target::Group` rules can score a
            //    whole OTA core like a device downstream. Nothing moves geometry
            //    after `dp`, so granting no abutment there costs nothing.
            coarse.groups = abutment_groups.clone();
            // Axis ids are per *block*, and a circuit can have more blocks than
            // devices, so the axis table has to be sized by the block count rather
            // than inherited at device length.
            if coarse.axis.len() < problem.blocks.len() {
                let centre = coarse.centre_x_estimate();
                coarse.axis.resize(problem.blocks.len(), centre);
            }
            coarse.power_uw = cell_power(&power, &devices_of);
            coarse.refresh_temps();
            coarse.debug_check("gp::place + group/power threading");

            // `macros` is threaded for its **pins**: `dp` builds its net table from
            // them, so this is what gives detailed placement a wirelength signal and
            // what makes a reshape priced on where its pins land rather than on the new
            // footprint's area (PLAN §2). `dp` re-resolves the geometry through
            // `layout.variant`, so handing it this epoch's assignment is safe even
            // though the reshape move changes that assignment underneath.
            let (mut layout, place_report) = refiner.place(
                &coarse,
                &macros,
                &variants,
                &problem.placement,
                &layers,
                &fixed,
                &mut prices,
                &oracle,
                seed,
            );
            // `dp` is the last stage that may move or reshape a device, so this is
            // where a stacked pair stops being fixable and starts being drawn geometry.
            layout.debug_check_placed("dp::place");
            // The other half of the two-consumer contract (see `coarse.groups`
            // above): recognition semantics from here on — `dp` was the last stage
            // that read groups as abutment permission.
            layout.groups = cell_groups.clone();

            // `dp` may have reshaped: adopt its assignment and redraw. Everything
            // below this line must see the geometry the placer actually legalised, not
            // the geometry it was handed.
            let macros = if layout.variant == assignment {
                macros
            } else {
                cellgen::realize(&variants, &layout.variant)
            };

            // Guard rings: drawn now, around the *placed* group bboxes, before routing
            // — a ring is both an obstacle and a net target, and it cannot exist
            // earlier because it encloses a placed bbox (PLAN §4e: a constraint
            // generated by the decision it constrains). Recomputed every epoch.
            let rings = cells::post_cell::guard_rings(&layout, &constraints_cells, pdk);

            // Routing: global (gcell PathFinder) → detailed (track realisation). Both
            // borrow `neg`, so this epoch's negotiation starts from the accumulated
            // history rather than from zero — that permanence is what stops the
            // rip-up/reroute cycle from looping (PLAN §4e).
            let (global, _) = g_router.route(
                &layout,
                &macros,
                &rings,
                &problem.routing,
                &layers,
                &mut neg,
                seed,
            );
            // Self-connectivity only. `debug_check_connected` is deliberately *not*
            // run here: a gcell plan snaps to cell centres, so it lands near a pin
            // rather than on it (measured 31 nm off on `pair`) and closing that last
            // gap is `dr`'s pin-access job, not a defect in the plan.
            global.debug_check("gr::route");
            // `dr` lands its terminals on these rects. Without them it derives
            // terminals from the coarse route's gcell-snapped corners and never
            // touches a pin.
            let pin_rects: Vec<(pnr_core::NetId, pnr_core::Rect)> =
                gr::place_macros(&macros, &layout)
                    .iter()
                    .flat_map(|m| m.pins.iter().map(|p| (p.net, p.at)))
                    .collect();
            let (routes, route_report) = d_router.route(
                &global,
                &pin_rects,
                &rings,
                &problem.routing,
                &layers,
                &cuts,
                &mut neg,
                seed,
            );
            routes.debug_check("dr::route");
            // The LVS precondition, checked here rather than inferred from an
            // `unconnected_pin` count at signoff: if a pin is not reached, its device
            // extracts onto its own island of nets and no downstream stage can repair
            // it, because nothing is *illegal* — the layout is merely disconnected.
            geometry::debug_check_connected(&macros, &layout, &routes);

            // In-loop DRC over the drawn geometry (device macros + guard rings) → Hard
            // rules for the next epoch.
            let mut shapes = geometry::collect(&macros, &layout, &routes);
            for r in &rings {
                shapes.extend(r.shapes.iter().cloned());
            }
            // The reference enables per-epoch LVS alongside the DRC/ERC pass.
            // PLAN §1 wants LVS "after any move that can change device
            // structure" — a variant escalation always precedes an epoch (the
            // outer tier re-enters the middle tier), and dp's in-epoch reshapes
            // land here too, so per-epoch coverage is exactly that cadence. A
            // mismatch folds forward as the same hard-rule capture DRC uses.
            let feedback = verify::drc_feedback(&shapes, pdk, Some(&reference));

            // Θ for this epoch: the budget residuals, measured on the layout we just
            // produced. This used to run once, after the loop, on the winner only —
            // which made it a *report* rather than a search signal. Inside the loop it
            // becomes the middle tier of the objective.
            let budgets = metadata::build(
                &problem.placement,
                &layout,
                &problem.routing,
                &routes,
                None, // bias is loop-invariant; attached once to the final report
                &problem.net_classes,
            );

            // This epoch's quality as PLAN §3b's lexicographic key, summed across
            // stages: (|V|, Θ, PEX). Strictly ordered — no amount of parasitic
            // improvement can buy past a violated budget, and no budget slack can buy
            // past a hard violation. Summing the tuples (rather than scalarising with
            // weights) is the whole point: a finite penalty is a bribe the optimizer
            // will accept.
            let key = lex_key(&place_report, &route_report, feedback.hard.len(), &budgets);
            drc_hard = feedback.hard;

            let improved = best.as_ref().is_none_or(|(bk, ..)| key < *bk);
            if improved {
                let stats = RunStats {
                    best_iteration: iter,
                    place_hard: place_report.hard_violations.len(),
                    route_hard: route_report.hard_violations.len(),
                    drc_hard: drc_hard.len(),
                    route_overuse: route_report
                        .budget_violations
                        .iter()
                        .map(|v| v.margin)
                        .sum(),
                    theta: key.1,
                    // The rest are patched after the loop, once they are known.
                    ..RunStats::default()
                };
                best = Some((key, layout, routes, rings, stats));
                stall = 0;
            } else {
                stall += 1;
            }

            if stall >= patience {
                break;
            }
        }

        // ═══ The outer-tier decision (PLAN §5's central diagnostic) ════════
        //
        // The middle tier has stopped improving. Two very different things look
        // identical from here, and telling them apart is the whole reason the outer
        // tier exists:
        //
        //   (a) We are feasible and the prices have settled → done. Feasibility alone
        //       is NOT termination: a run can be legal while still actively
        //       re-negotiating, and stopping there banks a solution that was still
        //       trading margin. Multiplier stationarity is what certifies no budget is
        //       being traded.
        //
        //   (b) We are stalled but still infeasible, and no *position* move helps →
        //       the binding constraint is the **variant space itself**. No arrangement
        //       of the currently-chosen variants can satisfy the constraints, so more
        //       placement effort is wasted effort. The response is an outer move: a
        //       different aspect ratio or metal-stack construction, which relocates
        //       pins and changes the problem rather than re-solving it.
        //
        // This separation is only cleanly diagnosable because the middle tier can
        // certify "no local position move helps" — the stall *is* that certificate.
        let feasible = best.as_ref().is_some_and(|(k, ..)| k.0 == 0 && k.1 <= 0.0);
        if feasible && prices.drift() < PRICE_STATIONARY {
            converged = true;
            break 'outer;
        }

        // Case (b): escalate. `None` means the collapsed variant space is exhausted —
        // every legal joint assignment has been tried and none is feasible. That is a
        // real answer (the circuit as specified cannot be laid out under these
        // constraints), not a failure to converge, so stop rather than spin.
        match cellgen::escalate(&variants, &assignment, cfg.seed ^ u64::from(outer)) {
            Some(next) => {
                variant_escalations += 1;
                eprintln!(
                    "[variant] middle tier stalled while infeasible after {iterations} epochs \
                     — variant-space binding, escalating (outer {outer})"
                );
                assignment = next;
            }
            None => break 'outer,
        }
    }

    let (_, layout, routes, rings, mut stats) = best.ok_or(FlowError::Empty)?;
    stats.iterations = iterations;
    stats.converged = converged;
    stats.outer_iterations = outer_iterations;
    stats.variant_escalations = variant_escalations;
    stats.price_drift = prices.drift();
    stats.route_pressure = neg.pressure();
    // The winning epoch's geometry is the one its own `layout.variant` selects — not
    // the last assignment the outer loop happened to try. Then the guard rings ride
    // along as extra absolute-coord macros (indices past the device count →
    // `collect` leaves them at absolute position), so signoff and GDS see them.
    let mut macros = cellgen::realize(&variants, &layout.variant);
    macros.extend(rings);

    // Budget report for the winning solution: the "how close to the edge" view a
    // hard-violation count cannot give. The base rule-set still carries the last
    // epoch's DRC feedback batches, so truncate them off first — they are measured
    // findings, not constraints the circuit declared, and leaving them in would
    // attribute them to a rule family in the table.
    problem.placement.hard.truncate(base_hard);
    let metadata = metadata::build(
        &problem.placement,
        &layout,
        &problem.routing,
        &routes,
        bias,
        &problem.net_classes,
    );

    Ok(Solution { layout, routes, macros, netlist, reference, stats, metadata })
}

/// PLAN §3b's lexicographic key: `(|V|, Θ, PEX)`, summed across every stage.
///
/// Summed, not scalarised. With finite weights the optimizer trades matching,
/// symmetry, and thermal margin for parasitic reduction — the "silently consuming
/// margin" failure PLAN §3b warns about — because a finite penalty is a bribe it
/// will accept. Tuple ordering makes the trade impossible to express instead of
/// merely expensive.
///
/// Note what is deliberately *not* here: a Pareto blend. Matching is a hard budget
/// that must hold, not a competing objective, so trading it against parasitics is
/// the wrong model. Pareto is only meaningful *within* the PEX term (R vs C vs
/// coupling), below the feasibility frontier.
type LexKey = (usize, f64, f32);

fn lex_key(
    place: &pnr_core::Report,
    route: &pnr_core::Report,
    drc_hard: usize,
    budgets: &metadata::MetadataReport,
) -> LexKey {
    let (pv, pt, pc) = place.lex();
    let (rv, rt, rc) = route.lex();
    // The analog Θ contribution: metadata's budget-arm residual sum in milli-budgets
    // (see `MetadataReport::theta` for the deliberate ~×2 double-weighing with the
    // pt/rt terms — the stage reports carry the same residuals; monotone-safe,
    // cleanup deferred).
    (pv + rv + drc_hard, pt + rt + budgets.theta(), pc + rc)
}

/// `‖λ_{k+1} − λ_k‖` below which constraint prices count as stationary (PLAN §5).
///
/// ponytail: one global threshold rather than per-family. Lift it to per-batch if a
/// real circuit turns out to have budgets whose natural price scales differ by orders
/// of magnitude — which is likely eventually (a thermal gradient in milli-°C and a
/// coupling sum in aF do not share units), but is not worth a knob before it bites.
const PRICE_STATIONARY: f64 = 1e-3;

impl Solution {
    /// The flat, placed geometry — every macro translated to its device centre
    /// plus all routed wires. This is exactly what signoff runs over and what
    /// [`gds::emit`] serialises; exposed so callers can write GDS/SVG artifacts.
    #[must_use]
    pub fn geometry(&self) -> Vec<pnr_core::Shape> {
        geometry::collect(&self.macros, &self.layout, &self.routes)
    }
}

/// Per-**cell** power for the thermal solver: the saturating sum of each cell's
/// member device powers (`power` is device-indexed — the operating point is a
/// property of the schematic, and `oppoint`/`BiasSummary` stay device-indexed).
///
/// An empty `power` means "no operating point supplied": every cell becomes a
/// pure heat sensor, the die is uniform, and thermal rules pass honestly rather
/// than by accident. A merged pair dissipates what its members dissipate, at one
/// centre — physically right, since the members share one footprint now.
fn cell_power(power: &[i32], devices_of: &[Vec<pnr_core::DeviceId>]) -> Vec<i32> {
    devices_of
        .iter()
        .map(|members| {
            members
                .iter()
                .map(|d| power.get(d.0 as usize).copied().unwrap_or(0))
                .fold(0i32, i32::saturating_add)
        })
        .collect()
}

/// Rewrite a group's members through `cell_of`, deduplicating while preserving
/// first-seen order. Never empties a non-empty group (dedup keeps ≥ 1 member), so
/// `Layout::bbox`'s "group has no members" invariant survives the collapse.
fn remap_members(members: &[pnr_core::DeviceId], cell_of: &[u16]) -> Vec<pnr_core::DeviceId> {
    let mut out: Vec<pnr_core::DeviceId> = Vec::with_capacity(members.len());
    for d in members {
        let c = match cell_of.get(d.0 as usize) {
            Some(&c) => pnr_core::DeviceId(c),
            None => *d,
        };
        if !out.contains(&c) {
            out.push(c);
        }
    }
    out
}

/// Debug tripwire for the collapse: after retargeting, no placement batch may
/// still name a device id at or past `n_cells` — an id in the old device space
/// means a rule kind is missing its `retarget` override.
///
/// The sweep scores every batch against a probe layout whose cells sit at
/// pairwise-distinct positions, so exact equalities (the rules most likely to be
/// violated at an arbitrary state) push their ids; a batch that indexes past the
/// probe panics on the spot, which is the same tripwire with a louder bell.
#[cfg(debug_assertions)]
fn debug_check_retargeted(problem: &annotator::Problem, n_cells: usize, cell_of: &[u16]) {
    let n = n_cells;
    let probe = Layout {
        x: (0..n).map(|i| i as i32 * 10_007 + 13).collect(),
        y: (0..n).map(|i| i as i32 * 7_919 + 29).collect(),
        hw: vec![50; n],
        hh: vec![50; n],
        axis: vec![0; problem.blocks.len().max(1)],
        groups: problem.groups.iter().map(|g| remap_members(g, cell_of)).collect(),
        orient: vec![pnr_core::Orient::default(); n],
        variant: vec![0; n],
        branch: Vec::new(),
        power_uw: vec![0; n],
        temp_mc: vec![0; n],
    };
    let mut ids: Vec<u32> = Vec::new();
    for b in problem
        .placement
        .hard
        .iter()
        .chain(problem.placement.budget.iter())
        .chain(problem.placement.cost.iter())
    {
        ids.clear();
        b.violating_ids(&probe, &mut ids);
        debug_assert!(
            ids.iter().all(|&i| (i as usize) < n),
            "batch {:?} still touches device ids {:?} after retargeting to {n} cells — \
             its rule kind is missing a `retarget` override",
            b.kind(),
            ids.iter().filter(|&&i| i as usize >= n).collect::<Vec<_>>()
        );
    }
}

/// Parse a SPICE netlist into the internal [`pnr_core::Netlist`] — the same
/// front-end [`run`] uses. Exposed so a caller can size-gate or inspect a
/// circuit without committing to a full flow.
///
/// # Errors
/// Propagates the parser's error string.
pub fn parse(spice: &str) -> Result<pnr_core::Netlist, String> {
    parse::spice(spice)
}

/// Final signoff on a solution: full DRC/LVS/PEX/ERC via `verify` (GPU DRC/ERC
/// under `--features verify/gpu`). Separate from [`run`] because it needs the
/// schematic reference and is a gate, not part of convergence.
#[must_use]
pub fn signoff(sol: &Solution, pdk: &Pdk) -> pnr_core::Report {
    let shapes = sol.geometry();
    let (report, _timings) =
        verify::signoff(&shapes, &sol.reference, &verify::erc::SignoffConfig::default(), pdk);
    report
}
