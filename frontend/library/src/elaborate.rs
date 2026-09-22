//! # `elaborate` — substrate3's "PDK on the fly" entry.
//!
//! A [`Composition`] already *places* itself (`place`/`place_by` against the
//! bound process), so elaboration is: **build → bind nets → route → report**.
//! No `gp`/`dp` — the generator's relative placement decisions are the
//! placement; only their nm values are recomputed per PDK. Routing is the real
//! flow's `gr`+`dr` with the same negotiated PathFinder, so the routed result
//! is signoff-grade, not a sketch.
//!
//! ```ignore
//! let sol = elaborate(&MyOta { .. }, &sky130, &ElabConfig::default())?;
//! let sol = elaborate(&MyOta { .. }, &gf180,  &ElabConfig::default())?;  // swap PDK
//! ```

use analog::Requirements;
use annotator::{annotate, AnnotationConfig, NoInference};
use dr::DetailedRouter;
use gr::GlobalRouter;
use macro_master::{build_composition, Composition};
use pnr_core::{LayerId, Layout, Macro, Netlist, Orient, Report, Routes, Shape};
use verify::Pdk;

/// Elaboration configuration.
pub struct ElabConfig {
    /// RNG seed (routing is negotiation-driven; the seed keys epoch mixing).
    pub seed: u64,
    /// Routing negotiation epochs. Each epoch re-routes with the accumulated
    /// PathFinder history; the lexicographically best result is kept.
    pub epochs: u32,
}

impl Default for ElabConfig {
    fn default() -> Self {
        Self { seed: 42, epochs: 4 }
    }
}

/// A routed elaboration — the substrate3 analogue of [`crate::Solution`].
pub struct Elaborated {
    /// One placed cell per instance, `layout` indices matching `macros`.
    pub layout: Layout,
    /// Per-instance placed macros (absolute coordinates; the layout stamp is
    /// the identity for them by construction).
    pub macros: Vec<Macro>,
    /// Routed wires/vias per [`pnr_core::NetId`].
    pub routes: Routes,
    /// Net names, indexed by `NetId` — from the generator's `connect` graph.
    pub nets: Vec<String>,
    /// The generator's declared io port names, in declaration order.
    ///
    /// A subset of [`Elaborated::nets`]: the rest are internal nodes the
    /// `connect` graph synthesised (`net7`). Kept apart because a netlist that
    /// promotes every internal node to a subcircuit pin is a different circuit
    /// from the one the generator declared.
    pub ports: Vec<String>,
    /// The winning routing epoch's report (hard violations + cost).
    pub report: Report,
    /// The composition read as a circuit, on the same [`pnr_core::NetId`]
    /// numbering as [`Elaborated::nets`] and [`Elaborated::routes`].
    ///
    /// This is what makes an elaboration more than geometry: the annotator
    /// extracts the analog routing rules from it (so a hand-placed composition
    /// is held to the same differential/crosstalk/antenna/parasitic budgets as
    /// an auto-placed one), and [`Elaborated::signoff`] uses it as the LVS
    /// reference. `None` when any instance is an opaque
    /// [`macro_master::DeviceGen`] — see [`macro_master::BuiltComp::netlist`].
    pub schematic: Option<Netlist>,
}

impl Elaborated {
    /// Flat placed geometry — device shapes plus routed wires — for signoff
    /// and [`crate::gds::emit`].
    #[must_use]
    pub fn geometry(&self) -> Vec<Shape> {
        let mut shapes: Vec<Shape> =
            self.macros.iter().flat_map(|m| m.shapes.iter().cloned()).collect();
        for wires in &self.routes.wires {
            shapes.extend(wires.iter().cloned());
        }
        shapes
    }

    /// One [`verify::LabeledPin`] per declared io port.
    ///
    /// Only [`Elaborated::ports`] are labelled: a label is what makes a net a
    /// pin of the extracted subcircuit, so labelling internal nodes too would
    /// export a cell with `net7` on its port list. They still extract — the
    /// SPICE writer numbers an anonymous net — they just stay internal.
    ///
    /// Exactly one label per net, deliberately: the extractor's short detection
    /// fires on two labels binding one connected component, and a second label
    /// of the same name would bind the same port twice. The first pin reached
    /// in macro order wins, which is canonical because `macros` is.
    #[must_use]
    fn port_labels(&self) -> Vec<verify::LabeledPin> {
        let mut seen = vec![false; self.nets.len()];
        let mut out = Vec::new();
        for m in &self.macros {
            for p in &m.pins {
                let idx = p.net.0 as usize;
                let Some(name) = self.nets.get(idx) else { continue };
                if !self.ports.contains(name) {
                    continue;
                }
                if core::mem::replace(&mut seen[idx], true) {
                    continue;
                }
                out.push(verify::LabeledPin {
                    name: name.clone(),
                    layer: p.layer.0,
                    x: p.at.x + p.at.w / 2,
                    y: p.at.y + p.at.h / 2,
                });
            }
        }
        out
    }

    /// The routed result as a SPICE `.subckt`, read back out of the drawn
    /// geometry alone.
    ///
    /// [`verify::Detail::WithParasitics`] runs PEX and splices per-net R/C in,
    /// making this a post-layout netlist; [`verify::Detail::Schematic`] gives
    /// devices and connectivity only. Device models are the deck's own
    /// (`sky130_fd_pr__nfet_01v8` and friends), so retargeting the generator
    /// retargets the netlist — there is no model-name mapping table to keep in
    /// step by hand.
    ///
    /// # Errors
    /// A deck the checker refuses, or geometry the extractor faults on.
    pub fn netlist(&self, pdk: &Pdk, detail: verify::Detail) -> Result<String, String> {
        verify::extract_spice(&self.geometry(), &self.port_labels(), pdk, detail)
    }

    /// Full signoff — DRC, LVS, PEX, ERC — against `pdk`, the same gate
    /// [`crate::signoff`] is for a searched [`crate::Solution`]. The LVS
    /// reference is [`Elaborated::schematic`], the circuit the composition
    /// declared, so this answers "does the drawn geometry extract as the
    /// circuit I wrote" and not merely "is it rule-clean".
    ///
    /// `None` when the composition has no schematic (an opaque
    /// [`macro_master::DeviceGen`] somewhere) — there is nothing to check the
    /// extraction against. Fall back to [`Elaborated::signoff_drc`], which
    /// always runs.
    #[must_use]
    pub fn signoff(&self, pdk: &Pdk) -> Option<Report> {
        let schematic = self.schematic.as_ref()?;
        let shapes = self.geometry();
        // Every provable net, not just the io ports — [`crate::labeled_pins`]
        // explains why. `macros` is already at absolute coordinates (the layout
        // stamp is the identity for an elaboration), so no `place_macros` here.
        let pins = crate::labeled_pins(&self.macros, &self.nets, pdk, &shapes);
        let mut reference = crate::cellgen::reference(schematic);
        // `verify` requires the reference's port list and the labels on the
        // geometry to name the same nets.
        reference.ports = pins.iter().map(|p| p.name.clone()).collect();
        Some(verify::signoff(&shapes, &pins, &reference, pdk).0)
    }

    /// The net labels [`Elaborated::signoff`] puts on the geometry — one per
    /// net it can *prove* (centre lies on drawn conductor of the layer that
    /// names it; `verify::resolve_labels` fails closed otherwise).
    ///
    /// Exposed so an independent extractor can be handed the same naming. A
    /// cross-check that labels the geometry its own way is comparing two
    /// different questions, and the difference reads as a checker disagreement
    /// when it is really a harness one.
    #[must_use]
    pub fn net_labels(&self, pdk: &Pdk) -> Vec<verify::LabeledPin> {
        crate::labeled_pins(&self.macros, &self.nets, pdk, &self.geometry())
    }

    /// Full geometric DRC of the routed result against `pdk` — one violation
    /// per finding, nm shortfall as the margin. The DRC half of
    /// [`Elaborated::signoff`], and the gate an elaboration can *always* run:
    /// geometry needs no schematic to be rule-checked.
    #[must_use]
    pub fn signoff_drc(&self, pdk: &Pdk) -> Vec<pnr_core::Violation> {
        verify::drc(&self.geometry(), &[], pdk)
            .into_iter()
            .map(|f| pnr_core::Violation {
                rule: format!("drc/{}:{}", f.rule, f.layer),
                margin: f.margin_nm,
            })
            .collect()
    }
}

/// Anything that stops an elaboration.
#[derive(Debug)]
pub enum ElabError {
    /// The generator itself failed (off-grid, overlap, missing layer, …).
    Gen(macro_master::GenError),
    /// Routing produced nothing (zero epochs).
    Empty,
}

/// Build `comp` against `pdk`, route its declared nets, return the routed
/// solution. Deterministic for `cfg.seed`. Call again with a different `pdk`
/// to retarget — that is the whole point.
///
/// # Errors
/// [`ElabError::Gen`] if the generator fails against this process.
pub fn elaborate<C: Composition>(
    comp: &C,
    pdk: &Pdk,
    cfg: &ElabConfig,
) -> Result<Elaborated, ElabError> {
    let built = build_composition(comp, pdk).map_err(ElabError::Gen)?;
    route_built(built, pdk, cfg)
}

/// Route an already-built composition — the half of [`elaborate`] shared with
/// the IR interpreter (`crate::emit`), whose build side goes through
/// `macro_master::build_with` instead of the [`Composition`] trait.
pub(crate) fn route_built(
    built: macro_master::BuiltComp,
    pdk: &Pdk,
    cfg: &ElabConfig,
) -> Result<Elaborated, ElabError> {
    // One layout slot per instance. `place_macro` stamps at `centre − hw −
    // bbox.x`, so choosing `centre = bbox.x + hw` makes the stamp the identity
    // — the instance macros stay exactly where the generator placed them.
    let n = built.instances.len();
    let macros: Vec<Macro> = built.instances.iter().map(|(_, m)| m.clone()).collect();
    let layout = Layout {
        x: macros.iter().map(|m| m.bbox.x + m.bbox.w / 2).collect(),
        y: macros.iter().map(|m| m.bbox.y + m.bbox.h / 2).collect(),
        hw: macros.iter().map(|m| m.bbox.w / 2).collect(),
        hh: macros.iter().map(|m| m.bbox.h / 2).collect(),
        axis: vec![0],
        groups: Vec::new(),
        orient: vec![Orient::default(); n],
        variant: vec![0; n],
        branch: Vec::new(),
        power_uw: vec![0; n],
        temp_mc: vec![0; n],
    };

    // Same stack derivation and router validation as the search flow.
    let (layers, cuts, pin_access) = routing_stack(pdk);
    let g_router = gr::GlobalRoute::default();
    let d_router = detailed_router(pdk, &layers, &cuts, pin_access);
    // Same *rules* as the search flow too. `build_with` resolved the netlist on
    // this build's own NetId numbering, so a rule the annotator keys to net `i`
    // and `Routes.wires[i]` are the same net — no remap, and none possible: the
    // one thing that would make these rules silently wrong is a numbering skew,
    // which is why the netlist is built from the binding the pins used rather
    // than joined to a separate schematic by name.
    //
    // An opaque instance leaves `netlist` None and the rules empty. That is the
    // honest outcome — geometry whose devices were never declared cannot be
    // recognised — but it is a real downgrade, so it is worth knowing about.
    let reqs = built.netlist.as_ref().map_or_else(Requirements::<Routes>::default, |nl| {
        annotate(nl, &NoInference, &AnnotationConfig::default()).routing
    });
    let mut neg = gr::Negotiation::new();

    let mut best: Option<(usize, f64, f32, Routes, Report)> = None;
    for epoch in 0..cfg.epochs.max(1) {
        let seed = cfg.seed ^ u64::from(epoch);
        let (global, _) =
            g_router.route(&layout, &macros, &[], &reqs, &layers, &mut neg, seed);
        let placed = gr::place_macros(&macros, &layout);
        let pin_rects: Vec<(pnr_core::NetId, pnr_core::Rect, LayerId)> = placed
            .iter()
            .flat_map(|m| m.pins.iter().map(|p| (p.net, p.at, p.layer)))
            .collect();
        let (routes, report) = d_router.route(
            &global, &pin_rects, &placed, &[], &reqs, &layers, &cuts, &mut neg, seed,
        );
        let key = report.lex();
        if best.as_ref().is_none_or(|(v, t, c, ..)| key < (*v, *t, *c)) {
            best = Some((key.0, key.1, key.2, routes, report));
        }
    }
    let (.., routes, report) = best.ok_or(ElabError::Empty)?;

    Ok(Elaborated {
        layout,
        macros,
        routes,
        nets: built.nets,
        ports: built.ports,
        report,
        schematic: built.netlist,
    })
}

/// Derive the routable metal stack from the deck: routing metals capped at the
/// router's wire width, matching via cuts, and the reserved pin-access pair
/// (li). Shared by [`crate::run`] and [`elaborate`] — one derivation, one
/// truth. See the long comments at the call site in `run` for the *why* of
/// each step.
#[allow(clippy::type_complexity)]
pub(crate) fn routing_stack(
    pdk: &Pdk,
) -> (
    Vec<LayerId>,
    Vec<(LayerId, i32, i32, i32)>,
    Option<(LayerId, (LayerId, i32, i32, i32))>,
) {
    let wire_w = dr::DetailedCfg::default().wire_width;
    let mut layers: Vec<_> = pdk
        .routing_layers()
        .into_iter()
        .take_while(|l| pdk.min_width(l.0).is_none_or(|w| w <= wire_w))
        .collect();
    let mut cuts = pdk.routing_vias();
    cuts.truncate(layers.len().saturating_sub(1));

    // Reserve the bottom conductor (li) for the cells; route met1 and up.
    let pin_access = (layers.len() > 1).then(|| {
        let pin_layer = layers.remove(0);
        (pin_layer, cuts.remove(0))
    });

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
    (layers, cuts, pin_access)
}

/// Build the detailed router with deck-derived pitch and pin-access config —
/// the validation block shared by [`crate::run`] and [`elaborate`].
pub(crate) fn detailed_router(
    pdk: &Pdk,
    layers: &[LayerId],
    cuts: &[(LayerId, i32, i32, i32)],
    pin_access: Option<(LayerId, (LayerId, i32, i32, i32))>,
) -> dr::DetailedRoute {
    let mut cfg = dr::DetailedCfg::default();
    let stack: Vec<_> = layers.iter().map(|l| l.0).collect();
    cfg.pitch = cfg.pitch.max(pdk.routing_pitch(cfg.wire_width, &stack));
    for l in layers {
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
    cfg.pin_access = pin_access;
    // The pitch has to clear the widest feature a track carries. That is the
    // via landing pad, not the wire: `routing_vias` sizes pads for the
    // asymmetric one-side enclosure (320 nm for via1), and a 320 nm pad on a
    // 430 nm pitch leaves 110 nm to the neighbouring track's wire, under
    // met1/met2's 140 nm — a spacing violation drawn at every via that sits
    // next to a through-route.
    let pad_extent = cuts.iter().map(|&(.., b, a)| b.max(a)).max().unwrap_or(0);
    cfg.pitch = cfg.pitch.max(pdk.routing_pitch(cfg.wire_width.max(pad_extent), &stack));
    // …and the wire is *drawn* at that width too, not under it. A pad wider than
    // its wire protrudes `(pad − wire)/2` on both sides — 15 nm for a 320 nm
    // via1 pad on a 290 nm wire — and each ledge becomes a concave step in the
    // net's own union as soon as another leg of the same net lands beside it,
    // which is where the PLL's `met1_notch` population came from. The pitch
    // above already pays for `pad_extent`, so matching the wire to it costs no
    // track and no area; it only stops the router drawing a conductor narrower
    // than the pads that conductor has to carry.
    cfg.wire_width = cfg.wire_width.max(pad_extent);
    let (pad_layer, pad_cut) = match pin_access {
        Some((l, (c, ..))) => (Some(l), Some(c)),
        None => (layers.first().copied(), cuts.first().map(|&(c, ..)| c)),
    };
    cfg.pin_access_spacing = pad_layer.and_then(|l| pdk.min_spacing(l.0)).unwrap_or(0);
    cfg.pin_access_cut_spacing = pad_cut.and_then(|l| pdk.min_spacing(l.0)).unwrap_or(0);
    dr::DetailedRoute { cfg }
}
