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
    /// The winning epoch's routed wires, by `NetId`.
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
        let geometry = self.geometry();
        Some(crate::signoff_shapes(&verify::Intent::default(), &geometry, &self.macros, &self.nets, schematic, None, &Default::default(), pdk).report)
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

/// Routes an already-built composition (shared with `emit::elaborate_ir`):
/// identity placement, `cfg.epochs` (at least one) negotiation epochs, the
/// lexicographically best [`Report`] kept.
///
/// # Panics
/// When the deck has no usable routing stack ([`routing_stack`] errs).
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

    // ponytail: a bad deck still panics here; FLOW-11 propagates the `Err`.
    let stack = routing_stack(pdk, None).unwrap_or_else(|e| panic!("routing stack: {e}"));
    let (layers, cuts) = (&stack.layers, &stack.cuts);
    let mut d_router = detailed_router(pdk, &stack);
    d_router.cfg.n_nets = built.netlist.as_ref().map_or(0, |n| n.nets.len());
    let ann = crate::annotation(pdk, &AnnotationConfig::default());
    // dr credits cell metal a pin reaches to that pin's net only through the
    // stack; without it every route landing on cell metal reads as a drawn
    // short (as `Flow` sets it).
    d_router.cfg.stack = ann.process.stack;
    // The netlist shares the pins' NetId numbering, so its routing rules key
    // the right nets with no remap.
    let reqs = built
        .netlist
        .as_ref()
        .map_or_else(Requirements::<Routes>::default, |nl| annotate(nl, &ann).routing);
    let mut neg = gr::Negotiation::new();
    let placed = gr::place_macros(&macros, &layout);
    let pins: Vec<_> = placed
        .iter()
        .flat_map(|m| m.pins.iter().map(|p| (p.net, p.at, p.layer)))
        .collect();

    let mut best: Option<(Routes, Report)> = None;
    for _ in 0..cfg.epochs.max(1) {
        let (routes, report, _) = d_router.route(&pins, &placed, &[], &reqs, layers, cuts, &mut neg);
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

/// The routed stack: metals bottom-up (ids beside their specs, since `dr::route`
/// and every caller take `&[LayerId]`), the cut joining each adjacent pair, the
/// split-off pin-access conductor and its cut, and the lattice ([`layer_specs`]).
///
/// Invariant: `cuts.len() == layers.len() - 1` and `specs.len() == layers.len()`;
/// `layers` is never empty.
pub(crate) struct RoutingStack {
    /// Routed metals, bottom-up.
    pub layers: Vec<LayerId>,
    /// `cuts[i]` joins `layers[i]` and `layers[i + 1]`.
    pub cuts: Vec<Cut>,
    /// A resistive bottom conductor (li) and its cut up to `layers[0]`, when
    /// split off; cells land on it, the router never runs along it.
    pub pin_access: Option<(LayerId, Cut)>,
    /// Base lattice pitch, nm.
    pub p0: i32,
    /// One lattice spec per `layers` entry.
    pub specs: Vec<gr::LayerSpec>,
}

/// A bottom conductor over this many times the next metal's sheet resistance
/// is a local interconnect, not a routing metal (tuning default; sky130 li is
/// 102× met1, every other built-in deck's metal1 under 3× its metal2).
const PIN_ACCESS_SHEET_RATIO: f32 = 10.0;

/// The routable metal stack from the deck: every routing metal, save a
/// resistive bottom conductor (li) split off as `pin_access` — cells fill it
/// with pads, so routing on it makes the lattice itself a spacing violation —
/// and save top metals whose track stride would pass [`gr::MAX_STRIDE`] (each
/// pop recomputes the lattice: the via up no longer lands below). `top` keeps
/// at most that many of the lowest metals.
///
/// # Errors
/// A deck with no routing metal, a missing cut between adjacent metals, a cut
/// wider than its pads, or a cut that is also a routing metal.
pub(crate) fn routing_stack(pdk: &Pdk, top: Option<usize>) -> Result<RoutingStack, String> {
    let mut layers = pdk.routing_layers();
    let mut cuts = pdk.routing_vias();
    if layers.is_empty() {
        return Err("no routable layer in the deck".into());
    }
    if cuts.len() != layers.len() - 1 {
        return Err(format!("every adjacent routing-layer pair needs its cut: {} metals, {} cuts", layers.len(), cuts.len()));
    }
    for (i, &(cut, size, below, above)) in cuts.iter().enumerate() {
        if size > below || size > above {
            return Err(format!("cut {cut:?} is wider than its pads {below}/{above}"));
        }
        if layers.contains(&cut) {
            return Err(format!("cut {cut:?} (joining {:?} and {:?}) is also a routing layer", layers[i], layers[i + 1]));
        }
    }
    let sheet = |l: LayerId| pdk.pex_f32(l, "sheet_res_ohm_sq");
    let resistive = layers.len() > 1 && sheet(layers[0]).zip(sheet(layers[1])).is_some_and(|(a, b)| a > PIN_ACCESS_SHEET_RATIO * b);
    let pin_access = resistive.then(|| (layers.remove(0), cuts.remove(0)));
    let (mut p0, mut specs) = layer_specs(pdk, &layers, &cuts, pin_access);
    let keep = top.unwrap_or(usize::MAX).max(1);
    while layers.len() > 1 && (layers.len() > keep || specs.last().is_some_and(|s| s.stride > gr::MAX_STRIDE)) {
        layers.pop();
        cuts.truncate(layers.len() - 1);
        (p0, specs) = layer_specs(pdk, &layers, &cuts, pin_access);
    }
    Ok(RoutingStack { layers, cuts, pin_access, p0, specs })
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
pub fn stack(pdk: &Pdk) -> analog::routing::Stack {
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
            .map(|(&l, rule)| {
                let cut = pdk.routing_cuts.contains(&l);
                analog::routing::stack::Layer {
                    id: l.0,
                    area_af_um2: pdk.pex_f32(l, "area_cap_af_um2").unwrap_or(0.0),
                    fringe_af_um: pdk.pex_f32(l, "fringe_cap_af_um").unwrap_or(0.0),
                    lateral: pdk.lateral_af_per_um(l, 1).unwrap_or(0.0),
                    antenna_ratio: rule.map_or(0.0, |r| r.0),
                    antenna_sidewall_nm: rule.map_or(0.0, |r| r.1),
                    sheet_ohm: pdk.pex_f32(l, "sheet_res_ohm_sq").unwrap_or(0.0),
                    cut,
                    thickness_nm: pdk.pex_f32(l, "thickness_nm").unwrap_or(0.0),
                    // Metals only: a cut is no lateral conductor.
                    latent_merge_nm: if cut { 0 } else { pdk.cell_f32("latent_merge_nm").map_or_else(|| pdk.min_spacing(l.0).unwrap_or(0), |v| v as i32) },
                    cross_af_um2: pdk.routing_metals.iter().position(|&m| m == l).and_then(|i| pdk.overlap_af_um2(l, *pdk.routing_metals.get(i + 1)?)).unwrap_or(0.0),
                }
            })
            .collect(),
        antenna_cumulative: rules.iter().flatten().any(|r| r.2),
        diode: pdk.antenna_diode_credit().map(|(l, bonus)| analog::routing::DiodeCredit { layer: l.0, bonus }),
    }
}

/// Sheet resistance past which a resistor body is protected whatever its
/// class: Hastings's minimal-matching allowance holds only to 500 Ω/□
/// (§8.2.3; §6.1 names > 1 kΩ/□ as modulated by leads above).
const BODY_SHEET_OHM: f32 = 500.0;
/// How far aggressor-class nets keep off a cap plate, nm (Hastings §8.2.2:
/// "≥ 3–4 µm around the shielded area", lower end).
const PLATE_AGGRESSOR_NM: i32 = 3_000;

/// The match class of the set a cell drawing `members` belongs to: the
/// `Unitization` of more than one device holding every member, its class or
/// `Moderate` when not given (C16); `None` = in no matched set.
pub(crate) fn match_class(units: &[analog::cell::Unitization], members: &[pnr_core::DeviceId]) -> Option<pnr_core::MatchClass> {
    units
        .iter()
        .find(|u| u.devices.len() > 1 && !members.is_empty() && members.iter().all(|d| u.devices.contains(d)))
        .map(|u| u.class.unwrap_or(pnr_core::MatchClass::Moderate))
}

/// RTE-16's routing blockages for the placed cells over the routed `layers`
/// (lattice order). Sources: each cell's keep-outs; a transistor cell with
/// no `Gate` keep-out (no generator emits one yet) gets its gates as the
/// pairwise intersections of its `poly` and `diff` shapes. Every rect keeps
/// the lowest routed metal's spacing as its halo. `class_of(c)` is the match class of
/// the set cell `c` draws, `None` = in no matched set: no blockage (C16).
/// Policy (Hastings §13.3 rule 17, §8.2): a gate at Moderate/Exceptional is
/// hard on every layer, own nets included (marked `gate` for the V check);
/// at Minimal soft. A resistor body at ≥ Moderate or over
/// [`BODY_SHEET_OHM`] is hard for foreign nets, else soft plus a hard copy
/// for aggressors; heads stay crossable. A cap plate at ≥ Moderate is hard
/// for foreign nets below the plate's top conductor (the highest routed
/// metal of the cell over it) and soft above, at Minimal soft; aggressors
/// keep [`PLATE_AGGRESSOR_NM`] off it on every layer.
pub(crate) fn blockages(placed: &[Macro], pdk: &Pdk, class_of: impl Fn(usize) -> Option<pnr_core::MatchClass>, layers: &[LayerId]) -> Vec<dr::Blockage> {
    use pnr_core::{KeepWhy, MatchClass, Process, Rect};
    let all = low_bits(layers.len());
    let s = layers.first().and_then(|l| pdk.route_spacing(l.0)).unwrap_or(0);
    let (poly, diff) = (Process::layer(pdk, "poly"), Process::layer(pdk, "diff"));
    let body_sheet = Process::layer(pdk, "rpoly").and_then(|l| pdk.pex_f32(l, "sheet_res_ohm_sq"));
    let mut out = Vec::new();
    for (c, m) in placed.iter().enumerate() {
        let Some(class) = class_of(c) else { continue };
        let strong = class >= MatchClass::Moderate;
        let b = |rect: Rect, layers: u16, hard: bool, gate: bool| dr::Blockage { rect, halo: s, layers, hard, own_exempt: !(hard && gate), only_aggressors: false, cell: c as u32, gate };
        let mut gates: Vec<Rect> = m.keepouts.iter().filter(|k| matches!(k.why, KeepWhy::Gate { .. })).map(|k| k.rect).collect();
        if gates.is_empty() {
            if let (Some(p), Some(d)) = (poly, diff) {
                let on = |l: LayerId| m.shapes.iter().filter(move |x| x.layer == l).map(|x| x.rect);
                for pr in on(p) {
                    for dr_ in on(d) {
                        let (x0, y0) = (pr.x.max(dr_.x), pr.y.max(dr_.y));
                        let (x1, y1) = ((pr.x + pr.w).min(dr_.x + dr_.w), (pr.y + pr.h).min(dr_.y + dr_.h));
                        if x1 > x0 && y1 > y0 {
                            gates.push(Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 });
                        }
                    }
                }
            }
        }
        out.extend(gates.into_iter().map(|g| b(g, all, strong, true)));
        for k in &m.keepouts {
            match k.why {
                KeepWhy::Gate { .. } => {}
                KeepWhy::ResistorBody { .. } => {
                    if strong || body_sheet.is_some_and(|r| r > BODY_SHEET_OHM) {
                        out.push(b(k.rect, all, true, false));
                    } else {
                        out.push(b(k.rect, all, false, false));
                        out.push(dr::Blockage { only_aggressors: true, hard: true, ..b(k.rect, all, false, false) });
                    }
                }
                KeepWhy::CapPlate { .. } => {
                    let over = |x: &&Shape| x.rect.x < k.rect.x + k.rect.w && k.rect.x < x.rect.x + x.rect.w && x.rect.y < k.rect.y + k.rect.h && k.rect.y < x.rect.y + x.rect.h;
                    let top = m.shapes.iter().filter(over).filter_map(|x| layers.iter().position(|&l| l == x.layer)).max();
                    let below = top.map_or(all, low_bits);
                    if strong && below != 0 {
                        out.push(b(k.rect, below, true, false));
                        if all & !below != 0 {
                            out.push(b(k.rect, all & !below, false, false));
                        }
                    } else {
                        out.push(b(k.rect, all, false, false));
                    }
                    out.push(dr::Blockage { halo: PLATE_AGGRESSOR_NM + s, only_aggressors: true, hard: true, ..b(k.rect, all, true, false) });
                }
            }
        }
    }
    out
}

/// A [`dr::Blockage::layers`] mask of the lowest `n` lattice layers; every
/// layer when `n` ≥ 16.
fn low_bits(n: usize) -> u16 {
    u32::try_from(n).ok().and_then(|n| 1u16.checked_shl(n)).map_or(u16::MAX, |b| b - 1)
}

/// Design intent for signoff's EM/IR rules: every Supply/Ground-class net at
/// the operating point's `vdd_mv`, with the DC current it carries (the larger
/// of what its terminals draw and supply). Empty without an operating point,
/// and no current for a net with an unresolved device on it: those rules then
/// skip, and say so, instead of checking a wrong number. Each such net's IR
/// budget from `ir` (`annotator::ir::budgets`: `(net, µA, max drop µV)`)
/// becomes its `max_drop_mv`, which arms `ir_drop`; a signal net's budget is
/// not written, as GPurify's grid examines only supply nets.
pub(crate) fn intent(
    netlist: &pnr_core::Netlist,
    classes: &[analog::metadata::NetClassification],
    draws: Option<&[Option<Vec<(String, f64)>>]>,
    vdd_mv: f64,
    ir: &[(pnr_core::NetId, i32, i64)],
) -> verify::Intent {
    use analog::metadata::NetClass;
    let Some(draws) = draws else { return verify::Intent::default() };
    let mut out = verify::Intent::default();
    for c in classes.iter().filter(|c| matches!(c.class, NetClass::Supply | NetClass::Ground)) {
        let name = netlist.nets[c.net.0 as usize].name.clone();
        let (mut inn, mut outg, mut known) = (0.0f64, 0.0f64, true);
        for (dev, d) in netlist.devices.iter().zip(draws) {
            for (t, _) in dev.terminals.iter().filter(|(_, n)| *n == c.net) {
                let ua = d.iter().flatten().find(|(x, _)| x == t).map_or(0.0, |&(_, ua)| ua);
                // A non-finite simulated current is as unknown as a missing one.
                known &= d.is_some() && ua.is_finite();
                if ua > 0.0 { inn += ua } else { outg -= ua }
            }
        }
        out.supplies.push((name.clone(), vdd_mv, c.class == NetClass::Ground));
        if let Some(&(_, _, uv)) = ir.iter().find(|b| b.0 == c.net) {
            out.max_drop_mv.push((name.clone(), uv as f64 / 1000.0));
        }
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
        let rule = |k: &str| i64::from(pnr_core::Process::rule(pdk, k, 0));
        let device = pnr_core::Device {
            name: format!("XDANT{}", net.0),
            kind: pnr_core::DeviceKind::Diode,
            // The deck's first diode row: the model `reference_spice` gives LVS, and one ngspice can simulate.
            model: crate::model_table(pdk).into_iter().find(|(_, k)| *k == pnr_core::DeviceKind::Diode).map(|(m, _)| m).unwrap_or_default(),
            terminals: vec![("P".into(), ground), ("N".into(), net)],
            params: vec![("w".into(), rule("diode_w")), ("l".into(), rule("diode_l"))],
        };
        out.push((device, m));
    }
    out
}

/// The per-layer lattice (Hastings Eq 15.19, `P = W_v + 2·E_mv + S_m`): the
/// base pitch `p0` and one [`gr::LayerSpec`] per routed metal, bottom-up,
/// horizontal first. Per metal, over the cuts landing on it (`cuts` below and
/// above, and the pin-access cut for `layers[0]`): `pad_across = size +
/// 2·E_across`, `pad_along = max(size + 2·E_along, ⌈min_area / pad_across⌉)`,
/// each on the manufacturing grid; `wire = max(min_width, pad_across)`; `S =
/// route_spacing`; `P = wire + S`. `p0` is the larger `P` of the two lowest
/// metals on `2·grid`; `stride = ⌈P/p0⌉`, `halo_wire = ⌈(wire + S)/p0⌉ − 1`,
/// `halo_via = ⌈(pad_along/2 + max(pad_along, wire)/2 + S)/p0⌉ − 1`.
pub(crate) fn layer_specs(pdk: &Pdk, layers: &[LayerId], cuts: &[Cut], pin_access: Option<(LayerId, Cut)>) -> (i32, Vec<gr::LayerSpec>) {
    let g = pdk.grid.max(1);
    let up = |v: i64, q: i64| (v + q - 1).div_euclid(q) * q;
    let mut specs: Vec<gr::LayerSpec> = layers
        .iter()
        .enumerate()
        .map(|(i, &l)| {
            let below = if i == 0 { pin_access.map(|(_, c)| c) } else { cuts.get(i - 1).copied() };
            let landing = below.into_iter().chain(cuts.get(i).copied());
            let (mut across, mut along) = (0, 0);
            for (c, size, ..) in landing {
                let (ea, eg) = pdk.cut_enclosure_pair(l, c);
                across = across.max(size + 2 * ea);
                along = along.max(size + 2 * eg);
            }
            let across = up(i64::from(across), i64::from(g)) as i32;
            let area = pdk.min_area(l.0).filter(|_| across > 0).map_or(0, |a| up(a, i64::from(across)) / i64::from(across));
            let along = up(i64::from(along).max(area), i64::from(g)) as i32;
            let wire = pdk.min_width(l.0).unwrap_or(0).max(across);
            gr::LayerSpec {
                id: l,
                horizontal: i % 2 == 0,
                wire,
                space: pdk.route_spacing(l.0).unwrap_or(0),
                pad_across: across,
                pad_along: along,
                ..gr::LayerSpec::default()
            }
        })
        .collect();
    let p0 = up(specs.iter().take(2).map(|s| i64::from(s.wire + s.space)).max().unwrap_or(1).max(1), i64::from(2 * g)) as i32;
    let ceil = |v: i32| ((v + p0 - 1) / p0).max(0);
    for s in &mut specs {
        s.stride = ceil(s.wire + s.space).max(1) as u32;
        // Clamped, never wrapped: a halo past 255 tracks reads as 255.
        s.halo_wire = (ceil(s.wire + s.space) - 1).clamp(0, 255) as u8;
        s.halo_via = (ceil(s.pad_along / 2 + s.pad_along.max(s.wire) / 2 + s.space) - 1).clamp(0, 255) as u8;
    }
    (p0, specs)
}

/// The detailed router configured from the deck: the per-layer lattice
/// ([`layer_specs`]); landing and pin access draw layer 0's wire.
pub(crate) fn detailed_router(pdk: &Pdk, stack: &RoutingStack) -> dr::DetailedRoute {
    let RoutingStack { layers, cuts, pin_access, p0, .. } = stack;
    let (layers, cuts, pin_access, p0, specs) = (&layers[..], &cuts[..], *pin_access, *p0, stack.specs.clone());
    let mut cfg = dr::DetailedCfg { grid: pdk.grid, em_front_row: crate::em_front_row(pdk), ..dr::DetailedCfg::default() };
    cfg.pitch = p0;
    cfg.wire_width = specs.first().map_or(0, |s| s.wire);
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
    let joins: Vec<(LayerId, LayerId, LayerId)> = pin_access.map(|(_, c)| c).into_iter().chain(cuts.iter().copied()).zip(&stack_below).zip(stack_above).map(|(((c, ..), &lo), &hi)| (c, lo, hi)).collect();
    cfg.cut_enclosure = joins.iter().map(|&(c, lo, hi)| (c, pdk.cut_enclosure(lo, c), pdk.cut_enclosure(hi, c))).collect();
    cfg.cut_enclosure_pair = joins.iter().map(|&(c, lo, hi)| (c, pdk.cut_enclosure_pair(lo, c), pdk.cut_enclosure_pair(hi, c))).collect();
    cfg.array_spacing = cuts
        .iter()
        .filter_map(|&(c, ..)| pdk.via_array_spacing(c.0).map(|(n, s)| (c, n, s)))
        .collect();
    // Electrical path cost (gr::Elec): per base-pitch step, ground C at the
    // layer's wire and the lateral C to an occupied neighbour track (one
    // stride over), over the cheapest layer's ground C.
    let per_step = |af_per_um: Option<f32>| af_per_um.map(|c| c * p0 as f32 / 1_000.0);
    let ground: Vec<Option<f32>> = specs.iter().map(|s| per_step(pdk.wire_af_per_um(s.id, s.wire))).collect();
    let side: Vec<Option<f32>> = specs.iter().map(|s| per_step(pdk.lateral_af_per_um(s.id, s.stride as i32 * p0 - s.wire))).collect();
    if let Some(cheapest) = ground.iter().flatten().copied().reduce(f32::min).filter(|&c| c > 0.0) {
        cfg.layer_c = ground.iter().map(|c| c.unwrap_or(cheapest) / cheapest).collect();
        cfg.beside_c = side.iter().map(|c| c.unwrap_or(0.0) / cheapest).collect();
        // A crossing node's overlap with the layer above, `wire_l·wire_{l+1}` (RTE-18).
        cfg.cross_c = specs
            .windows(2)
            .map(|w| pdk.overlap_af_um2(w[0].id, w[1].id).map_or(0.0, |c| c * w[0].wire as f32 * w[1].wire as f32 / 1e6) / cheapest)
            .collect();
    }
    // Series R per track step (sheet · pitch / width) and per via cut, over the
    // least resistive layer's step.
    let r_step: Vec<Option<f32>> = specs.iter().map(|s| pdk.pex_f32(s.id, "sheet_res_ohm_sq").map(|r| r * p0 as f32 / s.wire.max(1) as f32)).collect();
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
    cfg.layers = specs;
    dr::DetailedRoute { cfg }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GAP-06: sky130 met1 (2.8 mA/µm at 90 °C, fallback Ea 0.9 eV, n 1.1)
    /// derates ×0.1004 at 125 °C and is not credited at 27 °C.
    #[test]
    fn sky130_met1_em_limit_derates_at_125c_not_at_27c() {
        let p = verify::Pdk::builtin("sky130").unwrap();
        let m1 = p.layers.iter().find(|(n, _)| n == "met1").unwrap().1;
        let hot = super::em_limits(&p, &[m1], &[], Some(398.15))[0].1.ua_per_um;
        assert!((hot - 281.2).abs() <= 1.0, "{hot}");
        assert_eq!(super::em_limits(&p, &[m1], &[], Some(300.15))[0].1.ua_per_um, 2800.0);
    }

    /// Every cell of plan-05's RTE-10 table (sky130 met1–met4, li pin access).
    #[test]
    fn sky130_layer_specs_match_the_hand_derivation() {
        let pdk = Pdk::builtin("sky130").unwrap();
        let (metals, vias) = (pdk.routing_layers(), pdk.routing_vias());
        let (p0, specs) = layer_specs(&pdk, &metals[1..5], &vias[1..4], Some((metals[0], vias[0])));
        assert_eq!(p0, 420);
        let col = |f: fn(&gr::LayerSpec) -> i64| specs.iter().map(f).collect::<Vec<_>>();
        assert_eq!(col(|s| i64::from(s.pad_across)), [260, 280, 330, 330]);
        assert_eq!(col(|s| i64::from(s.pad_along)), [320, 370, 730, 730]);
        assert_eq!(col(|s| i64::from(s.wire)), [260, 280, 330, 330]);
        assert_eq!(col(|s| i64::from(s.space)), [140, 140, 300, 300]);
        assert_eq!(col(|s| i64::from(s.stride)), [1, 1, 2, 2]);
        assert_eq!(col(|s| i64::from(s.halo_wire)), [0, 0, 1, 1]);
        assert_eq!(col(|s| i64::from(s.halo_via)), [1, 1, 2, 2]);
        assert_eq!(col(|s| i64::from(s.horizontal)), [1, 0, 1, 0]);
    }

    fn names(pdk: &Pdk, ids: &[LayerId]) -> Vec<String> {
        ids.iter().map(|&id| pdk.layers.iter().find(|(_, l)| *l == id).unwrap().0.clone()).collect()
    }

    /// sky130: li splits off as pin access (102× met1's sheet), met5 (stride 8)
    /// drops, met4 recomputed without via4 matches the RTE-10 table row.
    #[test]
    fn sky130_routes_met1_to_met4() {
        let pdk = Pdk::builtin("sky130").unwrap();
        let s = routing_stack(&pdk, None).unwrap();
        assert_eq!(names(&pdk, &s.layers), ["met1", "met2", "met3", "met4"]);
        let (li, mcon) = s.pin_access.unwrap();
        assert_eq!(names(&pdk, &[li, mcon.0]), ["li", "mcon"]);
        assert_eq!(s.p0, 420);
        assert_eq!(s.specs.iter().map(|x| x.stride).collect::<Vec<_>>(), [1, 1, 2, 2]);
        let m4 = &s.specs[3];
        assert_eq!((m4.pad_across, m4.pad_along, m4.wire, m4.space, m4.halo_wire, m4.halo_via, m4.horizontal), (330, 730, 330, 300, 1, 2, false));
    }

    #[test]
    fn gf180_routes_metal1() {
        let pdk = Pdk::builtin("gf180mcu").unwrap();
        let s = routing_stack(&pdk, None).unwrap();
        assert_eq!(s.layers[0], pdk.routing_layers()[0]);
        assert!(s.pin_access.is_none());
    }

    #[test]
    fn ihp_routes_metal1() {
        let pdk = Pdk::builtin("ihp_sg13g2").unwrap();
        let s = routing_stack(&pdk, None).unwrap();
        assert_eq!(names(&pdk, &s.layers[..1]), ["metal1"]);
        assert!(s.pin_access.is_none());
    }

    #[test]
    fn a_top_cap_keeps_the_lowest_metals() {
        let pdk = Pdk::builtin("sky130").unwrap();
        let s = routing_stack(&pdk, Some(2)).unwrap();
        assert_eq!(names(&pdk, &s.layers), ["met1", "met2"]);
        assert_eq!(s.cuts.len(), 1);
    }

    /// A sidecar missing a cut between two metals fails `Pdk::load`, so the
    /// cut-count `Err` is unreachable from a loaded deck (`routing_vias` gives
    /// one cut per adjacent pair); a loaded stack with no metal, or with a cut
    /// that is also a routing metal, is an `Err` from `routing_stack`, never a panic.
    #[test]
    fn a_bad_stack_is_an_error_not_a_panic() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../pdks/");
        let deck = std::fs::read_to_string(format!("{root}decks/sky130.deck")).unwrap();
        let good = std::fs::read_to_string(format!("{root}sky130.json")).unwrap();
        let sidecar = good.replace(",\n        \"via4\"", "");
        assert!(!sidecar.contains("\"via4\""), "the sidecar edit took");
        assert!(Pdk::load(&deck, &sidecar).is_err(), "load rejects a missing cut");
        let load = || Pdk::load(&deck, &good).unwrap();
        assert!(routing_stack(&load(), None).is_ok());
        let mut empty = load();
        empty.routing_metals.clear();
        empty.routing_cuts.clear();
        assert!(routing_stack(&empty, None).is_err());
        let mut metal_cut = load();
        metal_cut.routing_cuts[0] = metal_cut.routing_metals[2];
        assert!(routing_stack(&metal_cut, None).is_err());
    }

    /// A 2-device set with no class is Moderate: its gates (poly ∩ diff) are
    /// hard on every layer, own nets included; Minimal makes them soft; a
    /// cell in no set gets none.
    #[test]
    fn blockages_follow_class() {
        use analog::cell::{SeriesParallel, Unitization};
        use pnr_core::{DeviceId, DeviceKind, MatchClass, Process, Rect};
        let pdk = Pdk::builtin("sky130").unwrap();
        let layers = routing_stack(&pdk, None).unwrap().layers;
        let unit = |devices: Vec<DeviceId>, class| Unitization {
            devices,
            device_type: DeviceKind::Nmos,
            dev_nf: vec![1, 1],
            target_ratio: vec![1, 1],
            unit_w: 1_000,
            unit_l: 150,
            series_parallel: SeriesParallel::Parallel,
            dummy_required: false,
            route_matching_required: true,
            class,
            kind: None,
            series: Vec::new(),
            style: None,
        };
        let units = [unit(vec![DeviceId(0), DeviceId(1)], None), unit(vec![DeviceId(2), DeviceId(3)], Some(MatchClass::Minimal))];
        let members = [vec![DeviceId(0), DeviceId(1)], vec![DeviceId(2), DeviceId(3)], vec![DeviceId(4)]];
        assert_eq!(match_class(&units, &members[0]), Some(MatchClass::Moderate));
        assert_eq!(match_class(&units, &members[1]), Some(MatchClass::Minimal));
        assert_eq!(match_class(&units, &members[2]), None);
        let (poly, diff) = (Process::layer(&pdk, "poly").unwrap(), Process::layer(&pdk, "diff").unwrap());
        let shape = |layer, x, y, w, h| Shape { layer, rect: Rect { x, y, w, h } };
        // Two fingers over one diffusion: two gates.
        let cell = Macro {
            shapes: vec![shape(diff, 0, 0, 2_000, 1_000), shape(poly, 500, -200, 150, 1_400), shape(poly, 1_300, -200, 150, 1_400)],
            ..Default::default()
        };
        let placed = vec![cell.clone(), cell.clone(), cell];
        let b = blockages(&placed, &pdk, |c| match_class(&units, &members[c]), &layers);
        let of = |c: u32| b.iter().filter(move |x| x.cell == c).collect::<Vec<_>>();
        let s = pdk.route_spacing(layers[0].0).unwrap();
        assert_eq!(of(0).len(), 2);
        assert!(of(0).iter().all(|x| x.hard && x.gate && !x.own_exempt && x.layers == (1 << layers.len()) - 1));
        assert_eq!((of(0)[0].rect, of(0)[0].halo), (Rect { x: 500, y: 0, w: 150, h: 1_000 }, s));
        assert!(of(1).len() == 2 && of(1).iter().all(|x| !x.hard && x.gate));
        assert!(of(2).is_empty());
    }

    /// The lattice dr builds for sky130, as the placer reads it (PLC-28).
    #[test]
    fn sky130_lattice_spec() {
        let pdk = Pdk::builtin("sky130").unwrap();
        let cfg = detailed_router(&pdk, &routing_stack(&pdk, None).unwrap()).cfg;
        assert_eq!(dr::lattice_spec(&cfg), dr::LatticeSpec { p0: 420, strides: vec![1, 1, 2, 2], origin_multiple: 840 });
    }

    /// Every built-in deck's lattice clears each layer's spacing between
    /// adjacent tracks, wire to wire and pad to pad (replaces pdk's
    /// `routing_pitch_clears_every_layer_of_its_stack`).
    #[test]
    fn every_layer_pitch_clears_its_spacing() {
        for deck in ["sky130", "gf180mcu", "ihp_sg13g2", "generic_finfet"] {
            let pdk = Pdk::builtin(deck).unwrap();
            let RoutingStack { p0, specs, .. } = routing_stack(&pdk, None).unwrap();
            for s in &specs {
                let need = pdk.min_spacing(s.id.0).unwrap_or(0);
                let pitch = s.stride as i32 * p0;
                assert!(pitch - s.wire >= need, "{deck} {:?}: pitch {pitch} wire {} needs {need}", s.id, s.wire);
                assert!(pitch >= s.pad_across + s.space, "{deck} {:?}: pitch {pitch} pad {} space {}", s.id, s.pad_across, s.space);
            }
        }
    }

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
        let i = super::intent(&nl, &[class(0, NetClass::Supply), class(1, NetClass::Ground)], Some(&draws), 1_800.0, &[]);
        assert_eq!(i.supplies.len(), 2, "both rails still declared");
        assert_eq!(i.currents, vec![("vss".to_string(), 10.0)], "vdd unknown: no current, never a partial sum");
    }
}

#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use analog::cell::{SeriesParallel, Unitization};
    use analog::metadata::{NetClass, NetClassification};
    use pnr_core::{DeviceId, DeviceKind, Keepout, KeepWhy, MatchClass, Net, NetId, Rect};

    fn sky() -> Pdk {
        Pdk::builtin("sky130").unwrap()
    }

    fn unit(devices: &[u16], class: Option<MatchClass>) -> Unitization {
        Unitization {
            devices: devices.iter().map(|&d| DeviceId(d)).collect(),
            device_type: DeviceKind::Nmos,
            dev_nf: vec![1; devices.len()],
            target_ratio: vec![1; devices.len()],
            unit_w: 1_000,
            unit_l: 150,
            series_parallel: SeriesParallel::Parallel,
            dummy_required: false,
            route_matching_required: true,
            class,
            kind: None,
            series: Vec::new(),
            style: None,
        }
    }

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    #[test]
    fn elab_config_defaults_to_four_epochs() {
        assert_eq!(ElabConfig::default().epochs, 4);
    }

    #[test]
    fn low_bits_masks() {
        assert_eq!(low_bits(0), 0);
        assert_eq!(low_bits(1), 1);
        assert_eq!(low_bits(4), 0b1111);
        assert_eq!(low_bits(15), 0x7fff);
        assert_eq!(low_bits(16), u16::MAX);
        assert_eq!(low_bits(40), u16::MAX);
        assert_eq!(low_bits(usize::MAX), u16::MAX);
    }

    #[test]
    fn match_class_needs_a_multi_device_set_holding_every_member() {
        let units = [unit(&[0], Some(MatchClass::Exceptional)), unit(&[1, 2], Some(MatchClass::Exceptional))];
        assert_eq!(match_class(&units, &[DeviceId(0)]), None, "a one-device unitization is no matched set");
        assert_eq!(match_class(&units, &[]), None, "a cell drawing nothing is in no set");
        assert_eq!(match_class(&units, &[DeviceId(1), DeviceId(3)]), None, "partly covered");
        assert_eq!(match_class(&units, &[DeviceId(2)]), Some(MatchClass::Exceptional));
        assert_eq!(match_class(&[], &[DeviceId(0)]), None);
    }

    #[test]
    fn layer_specs_of_no_layers_is_an_empty_lattice() {
        let pdk = sky();
        let (p0, specs) = layer_specs(&pdk, &[], &[], None);
        assert!(specs.is_empty());
        assert!(p0 > 0 && p0 % (2 * pdk.grid.max(1)) == 0, "p0 {p0} stays a positive multiple of 2·grid");
    }

    #[test]
    fn a_zero_or_one_top_keeps_one_metal() {
        let pdk = sky();
        for top in [0, 1] {
            let s = routing_stack(&pdk, Some(top)).unwrap();
            assert_eq!(s.layers.len(), 1, "top {top}");
            assert!(s.cuts.is_empty());
            assert_eq!(s.specs.len(), 1);
        }
    }

    #[test]
    fn routing_stack_invariants_hold_on_every_builtin() {
        for deck in ["sky130", "gf180mcu", "ihp_sg13g2", "generic_finfet"] {
            let pdk = Pdk::builtin(deck).unwrap();
            let s = routing_stack(&pdk, None).unwrap();
            assert!(!s.layers.is_empty(), "{deck}");
            assert_eq!(s.cuts.len(), s.layers.len() - 1, "{deck}");
            assert_eq!(s.specs.len(), s.layers.len(), "{deck}");
            assert!(s.specs.iter().all(|x| x.stride <= gr::MAX_STRIDE), "{deck}");
        }
    }

    #[test]
    fn em_limits_without_a_temperature_are_the_rated_values() {
        let p = sky();
        let m1 = p.layers.iter().find(|(n, _)| n == "met1").unwrap().1;
        assert_eq!(em_limits(&p, &[m1], &[], None)[0].1.ua_per_um, 2800.0);
        assert!(em_limits(&p, &[], &[], Some(400.0)).is_empty());
    }

    #[test]
    fn parasitic_stack_interleaves_metals_and_cuts() {
        let pdk = sky();
        let s = stack(&pdk);
        let want = pdk.routing_metals.len() + pdk.routing_cuts.len().min(pdk.routing_metals.len());
        assert_eq!(s.layers.len(), want);
        assert!(!s.layers[0].cut);
        for l in &s.layers {
            assert_eq!(l.cut, pdk.routing_cuts.iter().any(|c| c.0 == l.id));
            if l.cut {
                assert_eq!(l.latent_merge_nm, 0, "a cut is no lateral conductor");
            }
        }
        let ids: Vec<u16> = s.layers.iter().map(|l| l.id).collect();
        let mut dedup = ids.clone();
        dedup.sort_unstable();
        dedup.dedup();
        assert_eq!(dedup.len(), ids.len(), "ids are unique");
    }

    #[test]
    fn detailed_router_has_one_enclosure_row_per_cut() {
        let pdk = sky();
        let s = routing_stack(&pdk, None).unwrap();
        let cfg = detailed_router(&pdk, &s).cfg;
        let (_, mcon) = s.pin_access.unwrap();
        assert_eq!(cfg.cut_enclosure.len(), s.cuts.len() + 1);
        assert_eq!(cfg.cut_enclosure_pair.len(), s.cuts.len() + 1);
        assert_eq!(cfg.cut_enclosure[0].0, mcon.0, "the pin-access cut comes first");
        assert_eq!(cfg.layers.len(), s.layers.len());
        let gf = Pdk::builtin("gf180mcu").unwrap();
        let s = routing_stack(&gf, None).unwrap();
        let cfg = detailed_router(&gf, &s).cfg;
        assert_eq!(cfg.cut_enclosure.len(), s.cuts.len());
        assert_eq!(cfg.pin_access, None);
    }

    #[test]
    fn antenna_diodes_need_a_ground_and_a_violation() {
        let pdk = sky();
        let reqs = Requirements::<Routes>::default();
        assert!(antenna_diodes(&pdk, &reqs, &Routes::default(), &[], &[], None, 0).is_empty());
        assert!(antenna_diodes(&pdk, &reqs, &Routes::default(), &[], &[], Some(NetId(0)), 0).is_empty());
    }

    fn keepout_cell(why: KeepWhy, extra: Vec<Shape>) -> Macro {
        Macro { shapes: extra, keepouts: vec![Keepout { rect: rect(0, 0, 1_000, 1_000), why }], ..Default::default() }
    }

    #[test]
    fn resistor_body_at_moderate_is_one_hard_foreign_blockage() {
        let pdk = sky();
        let layers = routing_stack(&pdk, None).unwrap().layers;
        let cell = keepout_cell(KeepWhy::ResistorBody { owner: 0 }, Vec::new());
        let b = blockages(&[cell], &pdk, |_| Some(MatchClass::Moderate), &layers);
        assert_eq!(b.len(), 1);
        assert!(b[0].hard && !b[0].gate && b[0].own_exempt && !b[0].only_aggressors);
        assert_eq!(b[0].layers, low_bits(layers.len()));
    }

    #[test]
    fn an_explicit_gate_keepout_replaces_poly_diff_gates() {
        use pnr_core::Process;
        let pdk = sky();
        let layers = routing_stack(&pdk, None).unwrap().layers;
        let (poly, diff) = (Process::layer(&pdk, "poly").unwrap(), Process::layer(&pdk, "diff").unwrap());
        let shapes = vec![Shape { layer: diff, rect: rect(0, 0, 2_000, 1_000) }, Shape { layer: poly, rect: rect(500, -200, 150, 1_400) }];
        let cell = keepout_cell(KeepWhy::Gate { owner: 0 }, shapes);
        let b = blockages(&[cell], &pdk, |_| Some(MatchClass::Exceptional), &layers);
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].rect, rect(0, 0, 1_000, 1_000));
        assert!(b[0].hard && b[0].gate && !b[0].own_exempt);
        assert!(blockages(&[keepout_cell(KeepWhy::Gate { owner: 0 }, Vec::new())], &pdk, |_| None, &layers).is_empty());
    }

    #[test]
    fn cap_plate_blockages_by_class_and_top_conductor() {
        let pdk = sky();
        let layers = routing_stack(&pdk, None).unwrap().layers;
        let s = pdk.route_spacing(layers[0].0).unwrap_or(0);
        let all = low_bits(layers.len());
        // Minimal: soft everywhere, plus the aggressor ring.
        let b = blockages(&[keepout_cell(KeepWhy::CapPlate { owner: 0 }, Vec::new())], &pdk, |_| Some(MatchClass::Minimal), &layers);
        assert_eq!(b.len(), 2);
        assert!(!b[0].hard && b[0].layers == all);
        assert!(b[1].hard && b[1].only_aggressors && b[1].halo == PLATE_AGGRESSOR_NM + s && b[1].layers == all);
        // Moderate, top plate conductor on routed layer 1: hard below it, soft above.
        let over = vec![Shape { layer: layers[1], rect: rect(100, 100, 200, 200) }];
        let b = blockages(&[keepout_cell(KeepWhy::CapPlate { owner: 0 }, over)], &pdk, |_| Some(MatchClass::Moderate), &layers);
        assert_eq!(b.len(), 3);
        assert!(b[0].hard && b[0].layers == 0b1);
        assert!(!b[1].hard && b[1].layers == all & !0b1);
        assert!(b[2].only_aggressors);
    }

    /// More routed layers than the 16-bit mask holds never overflows a shift.
    #[test]
    fn blockages_with_more_than_sixteen_layers_do_not_overflow() {
        let pdk = sky();
        let layers: Vec<LayerId> = (0..20).map(LayerId).collect();
        let over = vec![Shape { layer: LayerId(18), rect: rect(100, 100, 200, 200) }];
        let b = blockages(&[keepout_cell(KeepWhy::CapPlate { owner: 0 }, over)], &pdk, |_| Some(MatchClass::Moderate), &layers);
        assert!(b.iter().all(|x| x.layers != 0));
    }

    fn rails() -> Netlist {
        let dev = |name: &str, terminals: &[(&str, u16)]| pnr_core::Device {
            name: name.into(),
            kind: DeviceKind::Resistor,
            model: String::new(),
            terminals: terminals.iter().map(|&(t, n)| (t.into(), NetId(n))).collect(),
            params: vec![],
        };
        Netlist {
            devices: vec![dev("R0", &[("P", 0), ("N", 2)]), dev("R1", &[("P", 2), ("N", 1)])],
            nets: ["vdd", "vss", "x"].iter().map(|n| Net { name: (*n).into() }).collect(),
            ..Default::default()
        }
    }

    fn class(n: u16, class: NetClass) -> NetClassification {
        NetClassification { net: NetId(n), class, c_budget_af: None, max_coupling_af: None }
    }

    #[test]
    fn intent_without_an_operating_point_is_empty() {
        let i = intent(&rails(), &[class(0, NetClass::Supply)], None, 1_800.0, &[(NetId(0), 1, 5_000)]);
        assert!(i.supplies.is_empty() && i.currents.is_empty() && i.max_drop_mv.is_empty());
    }

    #[test]
    fn intent_declares_rails_currents_and_ir_budgets() {
        let draws = [Some(vec![("P".to_string(), 5.0), ("N".to_string(), -5.0)]), Some(vec![("P".to_string(), 5.0), ("N".to_string(), -5.0)])];
        let classes = [class(0, NetClass::Supply), class(1, NetClass::Ground), class(2, NetClass::Signal)];
        let i = intent(&rails(), &classes, Some(&draws), 1_800.0, &[(NetId(0), 5, 5_000), (NetId(2), 5, 9_000)]);
        assert_eq!(i.supplies, [("vdd".to_string(), 1_800.0, false), ("vss".to_string(), 1_800.0, true)]);
        assert_eq!(i.currents, [("vdd".to_string(), 5.0), ("vss".to_string(), 5.0)]);
        assert_eq!(i.max_drop_mv, [("vdd".to_string(), 5.0)], "a signal net's budget is not written");
    }

    /// A non-finite simulated current is unknown, never a NaN/∞ current.
    #[test]
    fn intent_treats_a_non_finite_draw_as_unknown() {
        for bad in [f64::NAN, f64::INFINITY] {
            let draws = [Some(vec![("P".to_string(), bad), ("N".to_string(), 0.0)]), Some(vec![("P".to_string(), 0.0), ("N".to_string(), 0.0)])];
            let i = intent(&rails(), &[class(0, NetClass::Supply)], Some(&draws), 1_800.0, &[]);
            assert!(i.currents.is_empty(), "{bad}: {:?}", i.currents);
        }
    }

    #[test]
    fn elaborated_geometry_is_cells_then_wires() {
        let pdk = sky();
        let l = routing_stack(&pdk, None).unwrap().layers[0];
        let cell = Shape { layer: l, rect: rect(0, 0, 10, 10) };
        let wire = Shape { layer: l, rect: rect(20, 0, 10, 2) };
        let e = Elaborated {
            layout: Layout {
                x: vec![5],
                y: vec![5],
                hw: vec![5],
                hh: vec![5],
                axis: vec![0],
                groups: Vec::new(),
                orient: vec![Orient::default()],
                variant: vec![0],
                branch: Vec::new(),
                power_uw: vec![0],
                temp_mc: vec![0],
                units: Default::default(),
            },
            macros: vec![Macro { shapes: vec![cell], ..Default::default() }],
            routes: Routes { wires: vec![vec![], vec![wire]], ..Default::default() },
            nets: vec!["a".into(), "b".into()],
            ports: vec![],
            report: Report::default(),
            schematic: None,
        };
        assert_eq!(e.geometry(), [cell, wire]);
        assert!(e.signoff(&pdk).is_none(), "no schematic, no full signoff");
    }
}
