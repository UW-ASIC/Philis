//! # `elaborate` — substrate3's "PDK on the fly" entry.
//!
//! A [`Composition`] places itself against the bound process, so elaboration
//! is build → bind nets → route (the flow's `gr` + `dr`) → report. No `gp`/`dp`.

use analog::Requirements;
use annotator::{annotate, AnnotationConfig};
use macro_master::{build_composition, Composition};
use pnr_core::{LayerId, Layout, Macro, Netlist, Orient, Report, Routes, Shape};
use verify::Pdk;

/// Elaboration configuration.
pub struct ElabConfig {
    /// Routing negotiation epochs; the lexicographically best result is kept.
    pub epochs: u32,
}

impl Default for ElabConfig {
    fn default() -> Self {
        Self { epochs: 4 }
    }
}

/// A routed elaboration — the substrate3 analogue of [`crate::Solution`].
pub struct Elaborated {
    /// Identity placement: one slot per instance, indexed like `macros`.
    pub layout: Layout,
    /// Per-instance macros, already at absolute coordinates.
    pub macros: Vec<Macro>,
    pub routes: Routes,
    /// Net names, indexed by `NetId`.
    pub nets: Vec<String>,
    /// The generator's declared io ports (a subset of `nets`).
    pub ports: Vec<String>,
    /// The winning routing epoch's report.
    pub report: Report,
    /// The composition as a circuit, on the same `NetId` numbering as `nets`:
    /// the annotator's routing rules and the LVS reference come from it. `None`
    /// when any instance is an opaque `macro_master::DeviceGen`.
    pub schematic: Option<Netlist>,
}

impl Elaborated {
    /// Flat placed geometry — device shapes plus routed wires.
    #[must_use]
    pub fn geometry(&self) -> Vec<Shape> {
        let mut shapes: Vec<Shape> = self
            .macros
            .iter()
            .flat_map(|m| m.shapes.iter().copied())
            .collect();
        shapes.extend(self.routes.wires.iter().flatten().copied());
        shapes
    }

    /// The drawn geometry extracted as a SPICE `.subckt` whose pins are exactly
    /// the declared io ports; `Detail::WithParasitics` splices in per-net R/C.
    /// Device models are the deck's own.
    ///
    /// # Errors
    /// A deck the checker refuses, or geometry the extractor faults on.
    pub fn netlist(&self, pdk: &Pdk, detail: verify::Detail) -> Result<String, String> {
        // One label per io port (a second label on one net would bind it twice);
        // internal nets stay unlabelled so they are not exported as pins.
        let mut seen = vec![false; self.nets.len()];
        let mut labels = Vec::new();
        for p in self.macros.iter().flat_map(|m| &m.pins) {
            let i = p.net.0 as usize;
            let Some(name) = self.nets.get(i).filter(|n| self.ports.contains(n)) else {
                continue;
            };
            if !std::mem::replace(&mut seen[i], true) {
                let (x, y) = (p.at.x + p.at.w / 2, p.at.y + p.at.h / 2);
                labels.push(verify::LabeledPin {
                    name: name.clone(),
                    layer: p.layer.0,
                    x,
                    y,
                });
            }
        }
        verify::extract_spice(&self.geometry(), &labels, pdk, detail)
    }

    /// Full DRC/ERC/LVS/PEX signoff against [`Elaborated::schematic`]; `None`
    /// when an opaque instance left no schematic (use [`Elaborated::signoff_drc`]).
    #[must_use]
    pub fn signoff(&self, pdk: &Pdk) -> Option<Report> {
        let schematic = self.schematic.as_ref()?;
        Some(crate::signoff_shapes(
            &self.geometry(),
            &self.macros,
            &self.nets,
            schematic,
            pdk,
        ))
    }

    /// The net labels [`Elaborated::signoff`] puts on the geometry, so an
    /// independent extractor can be handed the same naming.
    #[must_use]
    pub fn net_labels(&self, pdk: &Pdk) -> Vec<verify::LabeledPin> {
        crate::labeled_pins(&self.macros, &self.nets, pdk, &self.geometry())
    }

    /// Geometric DRC only — runs without a schematic. Margin is nm shortfall.
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

/// Build `comp` against `pdk` and route its declared nets. Deterministic; call again with another `pdk` to retarget.
///
/// # Errors
/// The generator failing against this process.
pub fn elaborate<C: Composition>(
    comp: &C,
    pdk: &Pdk,
    cfg: &ElabConfig,
) -> Result<Elaborated, macro_master::GenError> {
    Ok(route_built(build_composition(comp, pdk)?, pdk, cfg))
}

/// Route an already-built composition (shared with `emit::elaborate_ir`).
pub(crate) fn route_built(
    built: macro_master::BuiltComp,
    pdk: &Pdk,
    cfg: &ElabConfig,
) -> Elaborated {
    // `centre = bbox.x + hw` makes the placement stamp the identity.
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

    let (layers, cuts, pin_access) = routing_stack(pdk);
    let d_router = detailed_router(pdk, &layers, &cuts, pin_access);
    // The netlist shares the pins' NetId numbering, so its routing rules key
    // the right nets with no remap.
    let reqs = built
        .netlist
        .as_ref()
        .map_or_else(Requirements::<Routes>::default, |nl| {
            annotate(nl, &AnnotationConfig::default()).routing
        });
    let mut neg = gr::Negotiation::new();
    let placed = gr::place_macros(&macros, &layout);
    let pins: Vec<_> = placed
        .iter()
        .flat_map(|m| m.pins.iter().map(|p| (p.net, p.at, p.layer)))
        .collect();

    let mut best: Option<(Routes, Report)> = None;
    for _ in 0..cfg.epochs.max(1) {
        let (global, _) =
            gr::GlobalRoute::default().route(&layout, &macros, &[], &reqs, &layers, &mut neg);
        let (routes, report) = d_router.route(
            &global,
            &pins,
            &placed,
            &[],
            &reqs,
            &layers,
            &cuts,
            &mut neg,
        );
        if best.as_ref().is_none_or(|(_, b)| report.lex() < b.lex()) {
            best = Some((routes, report));
        }
    }
    let (routes, report) = best.expect("at least one epoch ran");
    Elaborated {
        layout,
        macros,
        routes,
        nets: built.nets,
        ports: built.ports,
        report,
        schematic: built.netlist,
    }
}

/// A via cut and its landing pads: `(cut layer, cut size, pad below, pad above)`, nm.
pub(crate) type Cut = (LayerId, i32, i32, i32);

/// The routable metal stack from the deck: routing metals up to the first one
/// whose `min_width` exceeds the router's wire, their via cuts, and the bottom
/// conductor (li) split off as `pin_access` — cells fill li with pads, so
/// routing on it made the track lattice itself a spacing violation.
pub(crate) fn routing_stack(pdk: &Pdk) -> (Vec<LayerId>, Vec<Cut>, Option<(LayerId, Cut)>) {
    let wire_w = access_pad(pdk);
    let mut layers: Vec<_> = pdk
        .routing_layers()
        .into_iter()
        .take_while(|l| pdk.min_width(l.0).is_none_or(|w| w <= wire_w))
        .collect();
    let mut cuts = pdk.routing_vias();
    cuts.truncate(layers.len().saturating_sub(1));
    let pin_access = (layers.len() > 1).then(|| (layers.remove(0), cuts.remove(0)));

    assert!(!layers.is_empty(), "no routable layer in the deck");
    assert_eq!(
        cuts.len(),
        layers.len() - 1,
        "every adjacent routing-layer pair needs its cut"
    );
    for (i, &(cut, size, below, above)) in cuts.iter().enumerate() {
        assert!(
            size <= below && size <= above,
            "cut {cut:?} is wider than its pads {below}/{above}"
        );
        assert!(
            !layers.contains(&cut),
            "cut {cut:?} (joining {:?} and {:?}) is also a routing layer",
            layers[i],
            layers[i + 1]
        );
    }
    (layers, cuts, pin_access)
}

/// Pad of the first routing via: the narrowest wire every landing needs.
fn access_pad(pdk: &Pdk) -> i32 {
    pdk.routing_vias().first().map_or(0, |&(_, _, b, a)| b.max(a))
}

/// The detailed router configured from the deck: one track pitch that clears
/// the worst layer's spacing *and* the widest via pad, and wires drawn at pad
/// width (a pad wider than its wire leaves notches beside every via).
pub(crate) fn detailed_router(
    pdk: &Pdk,
    layers: &[LayerId],
    cuts: &[Cut],
    pin_access: Option<(LayerId, Cut)>,
) -> dr::DetailedRoute {
    let mut cfg = dr::DetailedCfg::default();
    let stack: Vec<_> = layers.iter().map(|l| l.0).collect();
    // Wires are drawn at pad width (a pad wider than its wire leaves notches
    // beside every via) and never under a layer's min_width.
    let pad_extent = cuts.iter().map(|&(.., b, a)| b.max(a)).max().unwrap_or(0);
    let min_w = stack.iter().filter_map(|&l| pdk.min_width(l)).max().unwrap_or(0);
    cfg.wire_width = access_pad(pdk).max(pad_extent).max(min_w);
    cfg.pitch = pdk.routing_pitch(cfg.wire_width, &stack);
    cfg.spacing = layers
        .iter()
        .map(|&l| (l, pdk.min_spacing(l.0).unwrap_or(0), pdk.wide_spacing(l.0)))
        .collect();
    // Fatten caps: the deck's `cell.route_signal_width`/`route_supply_width`,
    // else 2 wire widths for signals and supply just under the stack's widest
    // wide-metal threshold (4 wire widths when the deck has none).
    let widest_step = layers
        .iter()
        .filter_map(|l| pdk.wide_spacing(l.0).last().map(|&(t, _)| t))
        .min()
        .map(|t| t - 2 * pdk.grid);
    cfg.fat_signal = pnr_core::Process::rule(pdk, "route_signal_width", 2 * cfg.wire_width);
    cfg.fat_supply = pnr_core::Process::rule(pdk, "route_supply_width", widest_step.unwrap_or(4 * cfg.wire_width));
    cfg.pin_access = pin_access;
    let (pad_layer, pad_cut) = match pin_access {
        Some((l, (c, ..))) => (Some(l), Some(c)),
        None => (layers.first().copied(), cuts.first().map(|&(c, ..)| c)),
    };
    cfg.pin_access_spacing = pad_layer.and_then(|l| pdk.min_spacing(l.0)).unwrap_or(0);
    cfg.pin_access_cut_spacing = pad_cut.and_then(|l| pdk.min_spacing(l.0)).unwrap_or(0);
    dr::DetailedRoute { cfg }
}
