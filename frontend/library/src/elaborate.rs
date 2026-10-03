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
        Some(crate::signoff_shapes(&verify::Intent::default(),
            &self.geometry(),
            &self.macros,
            &self.nets,
            schematic,
            None,
            pdk,
        ).report)
    }

    /// The net labels [`Elaborated::signoff`] puts on the geometry, so an
    /// independent extractor can be handed the same naming.
    #[must_use]
    pub fn net_labels(&self, pdk: &Pdk) -> Vec<verify::LabeledPin> {
        crate::labeled_pins(&self.macros, &self.nets, pdk, &self.geometry())
    }

    /// Geometric DRC only — runs without a schematic. Margin is [`verify::shortfall`]:
    /// nm for a length rule, ‰ of the limit otherwise. Deck warnings are left
    /// out, as [`verify::Signoff::warnings`] keeps them out of the report.
    #[must_use]
    pub fn signoff_drc(&self, pdk: &Pdk) -> Vec<pnr_core::Violation> {
        verify::drc(&self.geometry(), &[], pdk)
            .into_iter()
            .filter(|f| !f.warning)
            .map(|f| pnr_core::Violation {
                rule: format!("drc/{}:{}", f.rule, f.layer),
                margin: f.margin,
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
        units: Default::default(),
    };

    let (layers, cuts, pin_access) = routing_stack(pdk);
    let d_router = detailed_router(pdk, &layers, &cuts, pin_access);
    // The netlist shares the pins' NetId numbering, so its routing rules key
    // the right nets with no remap.
    let reqs = built
        .netlist
        .as_ref()
        .map_or_else(Requirements::<Routes>::default, |nl| {
            annotate(nl, &crate::annotation(pdk, &AnnotationConfig::default())).routing
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

/// Each routing metal's and cut's deck EM limit, derated to `temp_k` (the
/// operating-point temperature; `None` = the rating temperature). Hastings
/// eqs. 15.24–15.25.
///
/// ponytail: one die temperature; local self-heating of a conductor (the
/// electrothermal loop, Lienig ch. 3) is not fed back.
pub(crate) fn em_limits(pdk: &Pdk, layers: &[LayerId], cuts: &[Cut], temp_k: Option<f32>) -> Vec<(LayerId, analog::routing::em::Limit)> {
    layers
        .iter()
        .copied()
        .chain(cuts.iter().map(|&(c, ..)| c))
        .filter_map(|l| {
            let e = pdk.em_limit(l)?;
            let f = match (e.derating, temp_k) {
                (Some((t_ref, ea, n)), Some(t)) => analog::routing::em::derate(t, t_ref, ea, n),
                _ => 1.0,
            };
            Some((l, analog::routing::em::Limit { ua_per_um: e.ua_per_um, ua_per_cut: e.ua_per_cut, blech: e.blech }.derated(f)))
        })
        .collect()
}

/// The deck's routing stack, bottom-up, metals and cuts interleaved: `pex`
/// ground and lateral C, and each etch stage's antenna rule.
pub(crate) fn stack(pdk: &Pdk) -> analog::routing::Stack {
    let mut order = Vec::new();
    for (i, &m) in pdk.routing_metals.iter().enumerate() {
        order.push(m);
        order.extend(pdk.routing_cuts.get(i));
    }
    let rules: Vec<_> = order.iter().map(|&l| pdk.antenna_rule(l)).collect();
    analog::routing::Stack {
        layers: order
            .iter()
            .zip(&rules)
            .map(|(&l, rule)| analog::routing::stack::Layer {
                id: l.0,
                area_af_um2: pdk.pex_f32(l, "area_cap_af_um2").unwrap_or(0.0),
                fringe_af_um: pdk.pex_f32(l, "fringe_cap_af_um").unwrap_or(0.0),
                lateral: pdk.lateral_af_per_um(l, 1).unwrap_or(0.0),
                antenna_ratio: rule.map_or(0.0, |r| r.0),
                antenna_sidewall_nm: rule.map_or(0.0, |r| r.1),
                sheet_ohm: pdk.pex_f32(l, "sheet_res_ohm_sq").unwrap_or(0.0),
                cut: pdk.routing_cuts.contains(&l),
            })
            .collect(),
        antenna_cumulative: rules.iter().flatten().any(|r| r.2),
        diode: pdk.antenna_diode_credit().map(|(l, bonus)| analog::routing::DiodeCredit { layer: l.0, bonus }),
    }
}

/// Design intent for signoff's EM/IR rules: every Supply/Ground-class net at
/// the operating point's `vdd_mv`, with the DC current it carries (the larger
/// of what its terminals draw and supply). Empty without an operating point,
/// and no current for a net with an unresolved device on it: those rules then
/// skip, and say so, instead of checking a wrong number.
pub(crate) fn intent(
    netlist: &pnr_core::Netlist,
    classes: &[analog::metadata::NetClassification],
    draws: Option<&[Option<Vec<(String, f64)>>]>,
    vdd_mv: f64,
) -> verify::Intent {
    use analog::metadata::NetClass;
    let Some(draws) = draws else { return verify::Intent::default() };
    let mut out = verify::Intent::default();
    for c in classes.iter().filter(|c| matches!(c.class, NetClass::Supply | NetClass::Ground)) {
        let name = netlist.nets[c.net.0 as usize].name.clone();
        let (mut inn, mut outg, mut known) = (0.0f64, 0.0f64, true);
        for (dev, d) in netlist.devices.iter().zip(draws) {
            for (t, _) in dev.terminals.iter().filter(|(_, n)| *n == c.net) {
                known &= d.is_some();
                let ua = d.iter().flatten().find(|(x, _)| x == t).map_or(0.0, |&(_, ua)| ua);
                if ua > 0.0 { inn += ua } else { outg -= ua }
            }
        }
        out.supplies.push((name.clone(), vdd_mv, c.class == NetClass::Ground));
        if known {
            out.currents.push((name, inn.max(outg)));
        }
    }
    out
}

/// Antenna diodes (Hastings pp. 228–229; MFG-04), for the gate nets `routing`'s
/// antenna rules still find over their limit after dr's jumper repair: per
/// net, the deck's diode drawn in free space beside the net's first gate pin
/// (else its first pin) — cathode `N` on the net, anode `P` on `ground` —
/// with the schematic device it adds. Empty when the deck credits no diode
/// ([`Pdk::antenna_diode_credit`]: one would fix nothing at signoff and add an
/// LVS device), when the deck's diode cannot be extracted by LVS
/// ([`Pdk::diode_marker`]), or when there is no ground net.
///
/// ponytail: one minimum diode per net, at the first free spot within 50 µm;
/// `clearance` is the placer's cell-to-cell gap.
pub(crate) fn antenna_diodes(
    pdk: &Pdk,
    routing: &Requirements<Routes>,
    routes: &Routes,
    placed: &[Macro],
    rings: &[Macro],
    ground: Option<pnr_core::NetId>,
    clearance: i32,
) -> Vec<(pnr_core::Device, Macro)> {
    use cells::Cell;
    let (Some(ground), Some(_), Some(_)) = (ground, pdk.diode_marker(), pdk.antenna_diode_credit()) else { return Vec::new() };
    let mut nets = Vec::new();
    for b in routing.hard.iter().filter(|b| b.repair_kind() == analog::RepairKind::Antenna) {
        b.violating_ids(routes, &mut nets);
    }
    nets.sort_unstable();
    nets.dedup();
    if nets.is_empty() {
        return Vec::new();
    }
    let one = pnr_core::DeviceGroup { devices: vec![pnr_core::DeviceId(0)] };
    let template = cells::diode::Diode { rows: 1, cols: 1 }.draw(&one, &analog::Constraints::default(), pdk);
    let mut obstacles: Vec<pnr_core::Rect> = placed.iter().chain(rings).map(|m| m.bbox).collect();
    let mut out = Vec::new();
    for net in nets.into_iter().map(|n| pnr_core::NetId(n as u16)) {
        let pins = || placed.iter().flat_map(|m| &m.pins).filter(|p| p.net == net);
        let Some(pin) = pins().find(|p| p.name.ends_with('G')).or_else(|| pins().next()) else { continue };
        let near = (pin.at.x + pin.at.w / 2, pin.at.y + pin.at.h / 2);
        let Some(mut m) = dr::place_near(&template, near, &obstacles, clearance, pdk.grid, 50_000) else { continue };
        for p in &mut m.pins {
            p.net = if p.name.ends_with('N') { net } else { ground };
        }
        obstacles.push(m.bbox);
        let dim = |k: &str, d: i32| (k.to_string(), i64::from(pnr_core::Process::rule(pdk, k, d)));
        let device = pnr_core::Device {
            name: format!("XDANT{}", net.0),
            kind: pnr_core::DeviceKind::Diode, model: String::new(),
            terminals: vec![("P".into(), ground), ("N".into(), net)],
            params: vec![(dim("diode_w", 0).0.replace("diode_", ""), dim("diode_w", 0).1), (dim("diode_l", 0).0.replace("diode_", ""), dim("diode_l", 0).1)],
        };
        out.push((device, m));
    }
    out
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
    let mut cfg = dr::DetailedCfg { grid: pdk.grid, ..dr::DetailedCfg::default() };
    let stack: Vec<_> = layers.iter().map(|l| l.0).collect();
    // Wires are drawn at pad width (a pad wider than its wire leaves notches
    // beside every via) and never under a layer's min_width.
    let pad_extent = cuts.iter().map(|&(.., b, a)| b.max(a)).max().unwrap_or(0);
    let min_w = stack.iter().filter_map(|&l| pdk.min_width(l)).max().unwrap_or(0);
    // Wires at the pin-access pad (a larger via pad up the stack is drawn
    // at the via only); the lattice pitch clears the largest pad, so every
    // track stays legal wherever a via lands.
    cfg.wire_width = access_pad(pdk).max(min_w);
    cfg.pitch = pdk.routing_pitch(cfg.wire_width.max(pad_extent), &stack);
    cfg.spacing = layers
        .iter()
        .copied()
        .chain(cuts.iter().map(|&(c, ..)| c))
        .map(|l| (l, pdk.route_spacing(l.0).unwrap_or(0), pdk.wide_spacing(l.0)))
        .collect();
    cfg.min_width = layers
        .iter()
        .copied()
        .chain(pin_access.map(|(l, _)| l))
        .filter_map(|l| pdk.min_width(l.0).map(|w| (l, w)))
        .collect();
    // Each cut's required enclosure by the metal below and above it.
    let stack_below: Vec<LayerId> = pin_access.map(|(l, _)| l).into_iter().chain(layers.iter().copied()).collect();
    let stack_above = &layers[usize::from(pin_access.is_none()).min(layers.len())..];
    let all_cuts: Vec<Cut> = pin_access.map(|(_, c)| c).into_iter().chain(cuts.iter().copied()).collect();
    cfg.cut_enclosure = all_cuts
        .iter()
        .zip(&stack_below)
        .zip(stack_above)
        .map(|((&(c, ..), &lo), &hi)| (c, pdk.cut_enclosure(lo, c), pdk.cut_enclosure(hi, c)))
        .collect();
    cfg.array_spacing = cuts
        .iter()
        .filter_map(|&(c, ..)| pdk.via_array_spacing(c.0).map(|(n, s)| (c, n, s)))
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
    // Electrical path cost (gr::Elec): per track step, ground C and the lateral
    // C to an occupied neighbour track, over the cheapest layer's ground C.
    let per_step = |af_per_um: Option<f32>| af_per_um.map(|c| c * cfg.pitch as f32 / 1_000.0);
    let ground: Vec<Option<f32>> = layers.iter().map(|&l| per_step(pdk.wire_af_per_um(l, cfg.wire_width))).collect();
    let side: Vec<Option<f32>> = layers.iter().map(|&l| per_step(pdk.lateral_af_per_um(l, cfg.pitch - cfg.wire_width))).collect();
    if let Some(cheapest) = ground.iter().flatten().copied().reduce(f32::min).filter(|&c| c > 0.0) {
        cfg.layer_c = ground.iter().map(|c| c.unwrap_or(cheapest) / cheapest).collect();
        cfg.beside_c = side.iter().map(|c| c.unwrap_or(0.0) / cheapest).collect();
    }
    // Series R per track step (sheet · pitch / width) and per via cut, over the
    // least resistive layer's step.
    let r_step: Vec<Option<f32>> = layers.iter().map(|&l| pdk.pex_f32(l, "sheet_res_ohm_sq").map(|r| r * cfg.pitch as f32 / cfg.wire_width.max(1) as f32)).collect();
    if let Some(least) = r_step.iter().flatten().copied().reduce(f32::min).filter(|&r| r > 0.0) {
        cfg.layer_r = r_step.iter().map(|r| r.unwrap_or(least) / least).collect();
        cfg.via_r = cuts.iter().map(|&(c, ..)| pdk.pex_f32(c, "sheet_res_ohm_sq").unwrap_or(0.0) / least).collect();
    }
    cfg.pin_access = pin_access;
    let (pad_layer, pad_cut) = match pin_access {
        Some((l, (c, ..))) => (Some(l), Some(c)),
        None => (layers.first().copied(), cuts.first().map(|&(c, ..)| c)),
    };
    cfg.pin_access_spacing = pad_layer.and_then(|l| pdk.min_spacing(l.0)).unwrap_or(0);
    cfg.pin_access_cut_spacing = pad_cut.and_then(|l| pdk.min_spacing(l.0)).unwrap_or(0);
    dr::DetailedRoute { cfg }
}

#[cfg(test)]
mod tests {
    /// A rail with an unresolved device on it states no current (signoff's EM
    /// rule then skips it); a rail with only known devices keeps its current.
    #[test]
    fn a_supply_with_an_unresolved_device_has_no_current() {
        use analog::metadata::{NetClass, NetClassification};
        use pnr_core::{Device, DeviceKind, Net, NetId, Netlist};
        let dev = |name: &str, kind, terminals: &[(&str, u16)]| Device {
            name: name.into(),
            kind,
            model: String::new(),
            terminals: terminals.iter().map(|&(t, n)| (t.into(), NetId(n))).collect(),
            params: vec![],
        };
        // nets: 0 vdd, 1 vss, 2 x
        let nl = Netlist {
            devices: vec![dev("M0", DeviceKind::Nmos, &[("D", 0), ("G", 2), ("S", 1), ("B", 1)]), dev("R1", DeviceKind::Resistor, &[("P", 0), ("N", 2)])],
            nets: ["vdd", "vss", "x"].iter().map(|n| Net { name: (*n).into() }).collect(),
            ..Default::default()
        };
        let class = |n: u16, class| NetClassification { net: NetId(n), class, c_budget_af: None, max_coupling_af: None };
        let draws = [Some(vec![("D".into(), 10.0), ("G".into(), 0.0), ("S".into(), -10.0), ("B".into(), 0.0)]), None];
        let i = super::intent(&nl, &[class(0, NetClass::Supply), class(1, NetClass::Ground)], Some(&draws), 1_800.0);
        assert_eq!(i.supplies.len(), 2, "both rails still declared");
        assert_eq!(i.currents, vec![("vss".to_string(), 10.0)], "vdd unknown: no current, never a partial sum");
    }
}
