//! # `dr` — detailed routing.
//!
//! [`DetailedRoute`] rebuilds a capacity-1 track lattice over the die, lands one
//! track node per placed pin, runs `gr`'s negotiated-congestion PathFinder, then
//! draws the geometry: track wires and vias, an L-shaped access jog from each
//! landed node to its pin (the pitch is far coarser than a pin, so nodes never
//! sit on pins), and same-net sliver/notch fillers.
//!
//! Ring and cell metal on a lattice layer is a hard obstacle to other nets.
//! What it cannot rule out structurally it measures and reports as hard
//! violations: open nets, unlanded pins or pin access deleted to break a short,
//! drawn shorts (between nets, and to cell or ring metal) and unresolved
//! congestion.

use std::collections::{HashMap, HashSet};

use analog::{RepairKind, Requirements};
use pnr_core::geom::{LayerId, Rect, Shape};
use pnr_core::report::Violation;
use pnr_core::routes::{conductor_layers_meet, open_components, Join};
use pnr_core::{GatePin, Macro, NetId, Report, Routes};

use gr::{extract_geometry, run_pathfinder, to_shapes, Dij, RGraph, RouteCtx, RouteHot, TrackGrid, NONE};

/// A via: `(cut layer, cut size, pad below, pad above)`, nm.
pub type Cut = (LayerId, i32, i32, i32);

const VIA_COST: f32 = 4.0;
const P_FAC: f32 = 2.0;
const HIST_INC: f32 = 0.5;
const MAX_ITERS: u32 = 150;
/// Most tracks one connection takes on a layer (tuning default).
const K_MAX: u8 = 8;
/// EM verify/repair rounds after geometry (tuning default): each bumps the
/// tracks of the branches `Electromigration::failing` names and reroutes.
const EM_ROUNDS: u32 = 3;
/// Repair rounds on the drawn routes after fill (RTE-23).
const POST_ROUNDS: u32 = 2;
/// Rip-up rounds for nets named by violated hard rules, each at doubled `p_fac`.
const HARD_ROUNDS: u32 = 4;
/// History on a node another net owns under a laid access jog: that net's trunk
/// detours rather than short the jog (soft, unlike `reserved`: it may still pass).
const JOG_HIST: f32 = 64.0;
/// `reserved` owner of a node no net's trunk may use (no net has this id).
const CONTESTED: u32 = NONE - 1;
/// `reserved` owner of a node within spacing of foreign cell or ring metal:
/// no net's trunk may use it, and unlike [`CONTESTED`] no pin lands there.
const BLOCKED: u32 = NONE - 2;

/// A region routed metal must respect (absolute nm; built per layout by the
/// library from the cells' keep-outs and match classes, RTE-16).
#[derive(Clone, Copy, Debug)]
pub struct Blockage {
    /// The protected area as drawn (the gate, the body, the plate).
    pub rect: Rect,
    /// How far routed metal keeps off `rect`, nm (the library's: the lowest
    /// routed metal's spacing). The `metal over gate` check reads `rect` alone.
    pub halo: i32,
    /// Bit `l` = lattice layer `l`.
    pub layers: u16,
    /// Hard: no node inside for the nets it applies to; soft: [`gr::KEEPOUT_COST`]
    /// per node for nets without a pin in `cell` (soft always exempts its own).
    pub hard: bool,
    /// Hard only: the nets with a pin in `cell` may pass.
    pub own_exempt: bool,
    /// Hard only: applies to aggressor nets ([`DetailedCfg::aggressor`]) alone.
    pub only_aggressors: bool,
    /// Index of the placed cell it protects.
    pub cell: u32,
    /// A matched gate: any routed metal over `rect` is a `metal over gate` V.
    pub gate: bool,
}

/// Track-lattice configuration (set from the deck by `frontend/library`).
#[derive(Debug, Clone)]
pub struct DetailedCfg {
    /// Base lattice pitch `p0`, nm: every layer's tracks lie on it.
    pub pitch: i32,
    /// The deck's manufacturing grid, nm: everything routed snaps to it.
    pub grid: i32,
    /// Layer 0's wire, nm: what landing and pin access draw.
    pub wire_width: i32,
    /// Per routing layer (parallel to `route`'s `layers`), its track stride,
    /// wire, spacing, oriented via pad and halos. Empty = the uniform lattice:
    /// stride 1, every wire `wire_width`, square pads from the `Cut`s, no halos.
    pub layers: Vec<gr::LayerSpec>,
    /// `min_spacing` of the layer landing pads sit on; `0` disables the check.
    pub pin_access_spacing: i32,
    /// `min_spacing` of the pin-access cut; `0` disables the check.
    pub pin_access_cut_spacing: i32,
    /// Conductor below the routing stack that cells put pins on (li), and the cut
    /// up to `layers[0]`. Reserving it keeps tracks off the cells' own li. `None`:
    /// pins sit on a stack layer.
    pub pin_access: Option<(LayerId, Cut)>,
    /// DC current each member terminal draws from its net, µA (operating
    /// point): per placed cell (index of `placed`), `(pin name, current)`;
    /// `None` = unknown, and a net with any unknown terminal gets no EM sizing.
    /// A pin carries this times its [`Self::pin_share`]; segments carry the sum
    /// on one side of them. Empty = unknown (no EM sizing).
    pub pin_ua: Vec<Vec<(String, Option<i32>)>>,
    /// Per placed cell, parallel to its `pins`: the fraction of its terminal's
    /// current each pin carries ([`pnr_core::pin_shares`] of the unplaced
    /// macro). Empty (outer or inner) = 1.0 for every pin.
    pub pin_share: Vec<Vec<f32>>,
    /// Per placed cell (index of `placed`): `(gate pin name "d{k}:G", schematic
    /// device id, gate-oxide area W·L·m nm²)`, so the antenna rule charges a
    /// piece only the gates it reaches. A gate pin missing here has unknown
    /// device and area (`u32::MAX`, 0).
    pub gate_nm2: Vec<Vec<(String, u32, i64)>>,
    /// Derated DC EM limits per routing metal and cut, from the deck; a layer
    /// absent here is unknown (no EM sizing — never a guessed constant).
    pub em: Vec<(LayerId, analog::routing::em::Limit)>,
    /// Count a via group by its front row ([`analog::routing::Electromigration::front_row`];
    /// sidecar `em_front_row_cuts`, default `true`).
    pub em_front_row: bool,
    /// Supply/ground nets. No special width since RTE-12 (a supply's tracks
    /// come from its EM current like any net's); kept for IR repair (RTE-21).
    pub supply_nets: Vec<NetId>,
    /// Per routing layer: `(min_spacing, [(width threshold, spacing)])` — the
    /// deck's width-dependent spacing: a `k`-track run at or over the first
    /// threshold is drawn as parallel wires with guard tracks.
    pub spacing: Vec<(LayerId, i32, Vec<(i32, i32)>)>,
    /// Per cut layer: `(cut count, spacing)` — an array of at least that many
    /// cuts needs the wider spacing.
    pub array_spacing: Vec<(LayerId, i32, i32)>,
    /// Per layer the router draws on (pin access included): the deck's
    /// minimum width, which no narrowed jog goes under.
    pub min_width: Vec<(LayerId, i32)>,
    /// Per cut: how far the metal below, and the metal above, must pass it
    /// on every side. A pin hosts a cut (no landing pad drawn) only with the
    /// room below around it; a pad above may slide by what exceeds the room
    /// above.
    pub cut_enclosure: Vec<(LayerId, i32, i32)>,
    /// Per routing cut: `(cut, below (across, along), above (across, along))`
    /// enclosure by its metals ([`Self::cut_enclosure`] is the larger of each
    /// pair): what a via array keeps from its overlap's edges, per axis.
    /// A cut absent here keeps its square pads' enclosure on both axes.
    pub cut_enclosure_pair: Vec<(LayerId, (i32, i32), (i32, i32))>,
    /// Parasitic sensitivity per net, `[0, 1]`, by `NetId` (`gr::Elec::weight`);
    /// empty = route on length and congestion alone.
    pub net_weight: Vec<f32>,
    /// Per lattice layer: ground C of one track step, and lateral C to one
    /// occupied adjacent track per step, both over the cheapest layer's ground
    /// C (`gr::Elec`).
    pub layer_c: Vec<f32>,
    pub beside_c: Vec<f32>,
    /// Per lower lattice layer: the crossing C of one node to the layer
    /// above, same scale (`gr::Elec::cross_c`); empty = none.
    pub cross_c: Vec<f32>,
    /// Aggressor weight per net, `[0, 1]`, by `NetId`
    /// (`CouplingBudget::default_weights`); empty = 1.0 each.
    pub aggressor_weight: Vec<f32>,
    /// Per lattice layer: series R of one track step, and of one via up from
    /// it, over the least resistive layer's step (`gr::Elec`); empty = none.
    pub layer_r: Vec<f32>,
    pub via_r: Vec<f32>,
    /// Matched pairs' shared source nets (placed pins, absolute), priced and
    /// repaired as [`analog::routing::CommonNodes`] on `stack`; empty = none.
    pub common: Vec<analog::routing::CommonNode>,
    pub stack: Option<&'static analog::routing::Stack>,
    /// Nets in the netlist: `Routes` is sized to at least this, so a rule on a
    /// net with no pin reads empty shapes. `0` = the pins' highest net + 1.
    pub n_nets: usize,
    /// Keep-outs over matched gates, resistor bodies and cap plates; empty = none.
    pub blockages: Vec<Blockage>,
    /// Per `NetId`, an aggressor-class net (`Clock` today); empty = none.
    pub aggressor: Vec<bool>,
    /// Capacitor sets whose plate leads are routed apart and equalised
    /// (RTE-20; absolute array rects); empty = none. Needs `stack`.
    pub plates: Vec<analog::routing::PlateSet>,
}

impl Default for DetailedCfg {
    fn default() -> Self {
        Self {
            pitch: 0,
            grid: 1,
            wire_width: 0,
            layers: Vec::new(),
            pin_access_spacing: 0,
            pin_access_cut_spacing: 0,
            pin_access: None,
            pin_ua: Vec::new(),
            pin_share: Vec::new(),
            gate_nm2: Vec::new(),
            em: Vec::new(),
            em_front_row: true,
            supply_nets: Vec::new(),
            spacing: Vec::new(),
            array_spacing: Vec::new(),
            min_width: Vec::new(),
            cut_enclosure: Vec::new(),
            cut_enclosure_pair: Vec::new(),
            net_weight: Vec::new(),
            layer_c: Vec::new(),
            beside_c: Vec::new(),
            cross_c: Vec::new(),
            aggressor_weight: Vec::new(),
            layer_r: Vec::new(),
            via_r: Vec::new(),
            common: Vec::new(),
            stack: None,
            n_nets: 0,
            blockages: Vec::new(),
            aggressor: Vec::new(),
            plates: Vec::new(),
        }
    }
}

impl DetailedCfg {
    /// Spacing two same-layer shapes of widths `a`, `b` need on `layer`; `fallback`
    /// when the layer has no entry.
    fn space(&self, layer: LayerId, a: i32, b: i32, fallback: i32) -> i32 {
        let Some((_, min, steps)) = self.spacing.iter().find(|(l, ..)| *l == layer) else {
            return fallback;
        };
        let w = a.max(b);
        steps.iter().filter(|&&(t, _)| w >= t).map(|&(_, s)| s).fold(*min, i32::max)
    }

    /// The first wide-metal threshold on `layer`, nm (`i32::MAX` = none).
    fn wide(&self, layer: LayerId) -> i32 {
        self.spacing.iter().find(|(l, ..)| *l == layer).and_then(|(.., s)| s.iter().map(|&(t, _)| t).min()).unwrap_or(i32::MAX)
    }

    /// Track stride on lattice layer `l` (1 without specs).
    fn stride(&self, l: usize) -> i32 {
        self.layers.get(l).map_or(1, |s| s.stride.max(1) as i32)
    }

    /// Wire width on lattice layer `l`: its spec's, else `wire_width`.
    fn wire(&self, l: usize) -> i32 {
        self.layers.get(l).map_or(self.wire_width, |s| s.wire)
    }

    /// The layer's derated EM limit, if the deck has one.
    fn em_limit(&self, layer: LayerId) -> Option<analog::routing::em::Limit> {
        self.em.iter().find(|(l, _)| *l == layer).map(|&(_, lim)| lim)
    }

    /// EM-safe width of a segment on `layer` carrying `ua` over a diffusion
    /// domain of `domain_nm`, nm, on the manufacturing grid (at least
    /// `wire_width`).
    fn em_width(&self, layer: LayerId, ua: f32, domain_nm: f32) -> i32 {
        let need = self.em_limit(layer).map_or(0.0, |lim| lim.width_nm(ua, domain_nm)).ceil() as i32;
        let step = 2 * self.grid;
        (need.max(self.wire_width) + step - 1) / step * step
    }
}

/// What one `route` call measured: wall time per stage, µs (landing,
/// negotiation, repair + shields, geometry through via arrays, fill), search
/// heap pops (`expanded`), and whether the lattice was coarsened. Fields an
/// item has not landed yet stay zero: `width_fallbacks` (RTE-22).
/// `congestion` is per-region demand after repair (absolute nm; see
/// `congestion`).
#[derive(Default, Clone, Debug)]
pub struct RouteStats {
    pub us_landing: u64,
    pub us_negotiate: u64,
    pub us_repair: u64,
    pub us_geometry: u64,
    pub us_fill: u64,
    pub expanded: u64,
    /// Constraint-repair trials run.
    pub trials: u32,
    pub pf_iters: u32,
    /// Residual track overuse after negotiation, repair and shields.
    pub overuse: f32,
    pub coarsened: bool,
    pub width_fallbacks: u32,
    /// Stack vias left with one cut after RTE-27's doubling (exact-pair
    /// images included). Pin-access cuts are not stack vias.
    pub single_cut_vias: u32,
    /// Stack vias drawn, each array or single cut once; RTE-27's denominator.
    pub stack_vias: u32,
    /// Inner-corner support squares (GAP-12) of EM-critical nets' flush Ls
    /// left out because they would not clear foreign metal (an exact pair's
    /// leader counts twice).
    pub corners_unsupported: u32,
    pub congestion: Vec<(Rect, f32)>,
    /// Matched pairs routed as exact images (RTE-15), `(leader, image)`, and
    /// the rest as `(pos, neg, reason)`: `no pin map`, `axis off lattice`,
    /// `wide pair`, `landing failed`.
    pub pairs_exact: Vec<(u32, u32)>,
    pub pairs_fallback: Vec<(u32, u32, &'static str)>,
    /// Per capacitor set (RTE-20), its lead-C spread after equalising,
    /// percent of one unit's C; `None` = unknown (no unit C).
    pub plate_spread_pct: Vec<Option<f32>>,
    /// Leader fillers whose image on an exact pair's other net would sit
    /// within spacing of foreign metal, dropped from both nets (RTE-23).
    pub pair_fillers_dropped: u32,
    /// Repair rounds run on the drawn routes after fill (RTE-23).
    pub post_rounds: u32,
    /// Distinct vias drawn (RTE-28 reports the drop).
    pub vias: u32,
}

/// The detailed router.
#[derive(Default)]
pub struct DetailedRoute {
    pub cfg: DetailedCfg,
}

/// A landed terminal and the access jog that will connect it to its pin
/// (routing frame).
struct Access {
    ci: usize,
    pin: Rect,
    pin_layer: LayerId,
    node: (i32, i32),
    node_layer: u32,
    /// `(width, flip)` chosen at landing, when the joint search found a clean jog.
    choice: Option<(i32, bool)>,
    /// DC current the pin carries, µA (`None` = unknown): as many pin-end
    /// cuts as it needs.
    ua: Option<f32>,
    /// The largest current at the landed node, µA: the pin's own, or at a
    /// junction the largest MST edge meeting there (the jog lands on that
    /// metal, so the EM rule bounds it by the junction's current). No jog
    /// is narrower than its EM width.
    node_ua: Option<f32>,
}

impl DetailedRoute {
    /// Route `pins` (plus the pins of `placed` and `rings`) over the metal stack
    /// `layers` (even index = horizontal), joined by `cuts[i]` between
    /// `layers[i]` and `layers[i + 1]`.
    ///
    /// `pins` are placed pin rects; pins of `placed` (device macros) and `rings`
    /// are folded in, deduplicated. Their metal on a lattice layer, grown by
    /// spacing, is a hard obstacle to every net but its own (cell metal a pin
    /// reaches, by [`cell_metal`]); unattributed metal, and every shape without
    /// a `cfg.stack`, blocks all nets. Both are consulted for landing-pad
    /// spacing. `neg` carries PathFinder history across calls.
    #[allow(clippy::too_many_arguments)]
    pub fn route(
        &self,
        pins: &[(NetId, Rect, LayerId)],
        placed: &[Macro],
        rings: &[Macro],
        reqs: &Requirements<Routes>,
        layers: &[LayerId],
        cuts: &[Cut],
        neg: &mut gr::Negotiation,
    ) -> (Routes, Report, RouteStats) {
        let cfg = &self.cfg;

        // Terminals, deduped by (net, rect) in first-seen order: landing claims nodes
        // in this order, so the order decides contested sites.
        let mut seen = HashSet::new();
        let all_pins: Vec<(NetId, Rect, LayerId)> = pins
            .iter()
            .copied()
            .chain(placed.iter().chain(rings).flat_map(|m| m.pins.iter().map(|p| (p.net, p.at, p.layer))))
            .filter(|&(n, r, _)| seen.insert((n.0, r.x, r.y, r.w, r.h)))
            .collect();
        let n_nets = cfg.n_nets.max(all_pins.iter().map(|(n, ..)| n.0 as usize + 1).max().unwrap_or(0));
        // Climbing to a layer with no cut to reach it buys only an island.
        let n_layers = (layers.len() as u32).min(cuts.len() as u32 + 1);

        // Routing frame over pins and placed/ring bboxes: `TrackGrid` starts at
        // (0,0) and placed geometry reaches negative coordinates, so shift. The
        // frame keeps a claim-free halo (so jog reservations can never wall off
        // a whole band) plus a free margin (the cheapest trunk highways).
        let (halo, margin) = (2 * cfg.pitch, 3 * cfg.pitch);
        let low = |f: fn(&Rect) -> i32| {
            let pins = all_pins.iter().map(|(_, r, _)| f(r));
            pins.chain(placed.iter().chain(rings).map(|m| f(&m.bbox))).min().unwrap_or(0).min(0)
        };
        // On a multiple of the lattice period, so every layer's tracks sit at
        // the same absolute coordinates whatever the frame.
        let origin = frame_origin((low(|r| r.x), low(|r| r.y)), halo + margin, lattice_period(cfg));
        let shift = |r: Rect| Rect { x: r.x - origin.0, y: r.y - origin.1, ..r };
        // RTE-20: the capacitor sets' plate rules, arrays moved by `-off`
        // (`origin` for the routing frame, `(0, 0)` for absolute routes).
        let plate_rules = |off: (i32, i32)| {
            let space_nm = layers.first().map_or(0, |&l| cfg.space(l, 0, 0, cfg.pitch - cfg.wire_width));
            analog::routing::PlateRatios(match cfg.stack {
                Some(stack) => cfg
                    .plates
                    .iter()
                    .map(|p| analog::routing::PlateRatio {
                        set: analog::routing::PlateSet { array: Rect { x: p.array.x - off.0, y: p.array.y - off.1, ..p.array }, ..p.clone() },
                        tol_pct10: PLATE_TOL_PCT10,
                        stack,
                        space_nm,
                    })
                    .collect(),
                None => Vec::new(),
            })
        };
        // The cells' metal and gate pins per net, for the rules that score the
        // whole conductor (antenna): absolute, and in the routing frame.
        let (cell_abs, gates_abs) = cell_metal(placed, n_nets, cfg.stack, &cfg.gate_nm2);
        let cell_f: Vec<Vec<Shape>> = cell_abs.iter().map(|v| v.iter().map(|s| Shape { rect: shift(s.rect), ..*s }).collect()).collect();
        let gates_f: Vec<Vec<GatePin>> = gates_abs.iter().map(|v| v.iter().map(|&g| GatePin { at: shift(g.at), ..g }).collect()).collect();
        // Every ring and cell shape (absolute) with the net whose pin reaches it
        // ([`cell_metal`] over the rings too); `None` = no pin reaches it, or no
        // stack to tell.
        let foreign_metal: Vec<(Option<u32>, Shape)> = {
            let rings_abs = cell_metal(rings, n_nets, cfg.stack, &[]).0;
            let mut owner: HashMap<(LayerId, Rect), u32> = HashMap::new();
            for per_net in [&cell_abs, &rings_abs] {
                for (n, shapes) in per_net.iter().enumerate() {
                    owner.extend(shapes.iter().map(|s| ((s.layer, s.rect), n as u32)));
                }
            }
            placed.iter().chain(rings).flat_map(|m| &m.shapes).map(|s| (owner.get(&(s.layer, s.rect)).copied(), *s)).collect()
        };

        // DC current a terminal rect draws, µA: over the cell's pins there (two
        // members sharing a region draw one pin each at one rect), each pin's
        // terminal current (by pin name) times its share. No table entry
        // (ring, diode) draws 0; any `None`, or no table at all (no operating
        // point), is unknown.
        let pin_ua = |net: NetId, r: Rect| -> Option<f32> {
            if cfg.pin_ua.is_empty() {
                return None;
            }
            placed.iter().zip(&cfg.pin_ua).enumerate().find_map(|(c, (m, table))| {
                let mut hit = None;
                for (p, pin) in m.pins.iter().enumerate().filter(|(_, p)| p.net == net && p.at == r) {
                    let Some(&(_, ua)) = table.iter().find(|(n, _)| *n == pin.name) else { continue };
                    let share = cfg.pin_share.get(c).and_then(|s| s.get(p)).copied().unwrap_or(1.0);
                    hit = Some(hit.unwrap_or(Some(0.0)).zip(ua).map(|(sum, ua)| sum + ua as f32 * share));
                }
                hit
            }).unwrap_or(Some(0.0))
        };
        let mut term_rects: Vec<Vec<(Rect, LayerId, Option<f32>)>> = vec![Vec::new(); n_nets];
        for &(net, r, l) in &all_pins {
            term_rects[net.0 as usize].push((shift(r), l, pin_ua(net, r)));
        }
        let (mut hi_x, mut hi_y) = (2, 2);
        for m in placed.iter().chain(rings) {
            hi_x = hi_x.max(m.bbox.x + m.bbox.w);
            hi_y = hi_y.max(m.bbox.y + m.bbox.h);
        }
        (hi_x, hi_y) = (hi_x - origin.0 + margin, hi_y - origin.1 + margin);
        for (r, ..) in term_rects.iter().flatten() {
            hi_x = hi_x.max(r.x + r.w);
            hi_y = hi_y.max(r.y + r.h);
        }
        let die = (hi_x + halo, hi_y + halo);

        let mut compact: Vec<usize> = (0..n_nets).filter(|&i| !term_rects[i].is_empty()).collect();
        if compact.is_empty() {
            let routes = Routes { wires: vec![Vec::new(); n_nets], ..Default::default()  };
            let report = score(&routes, reqs, 0.0, &[], &[], &[], 1);
            return (routes, report, RouteStats::default());
        }
        let mut ci_of = vec![usize::MAX; n_nets];
        for (ci, &ni) in compact.iter().enumerate() {
            ci_of[ni] = ci;
        }
        let mut n_compact = compact.len();

        let grid = if cfg.layers.is_empty() {
            TrackGrid::with_layers(die, cfg.pitch, VIA_COST, n_layers)
        } else {
            // A short `cfg.layers` would quietly route on fewer metals than `layers`.
            debug_assert!(cfg.layers.len() >= n_layers as usize, "cfg.layers has {} of {n_layers} layers", cfg.layers.len());
            TrackGrid::new(die, cfg.pitch, cfg.layers[..(n_layers as usize).min(cfg.layers.len())].to_vec(), VIA_COST)
        };

        let mut claimed = vec![false; grid.nodes()];
        let mut reserved = vec![NONE; grid.nodes()];
        let mut c_terms: Vec<Vec<u32>> = vec![Vec::new(); n_compact];
        // `(landed node, µA)` per pin, per compact net; empty when any of the
        // net's terminals is unknown (no EM sizing, never a guessed zero).
        let mut node_ua: Vec<Vec<(u32, f32)>> = vec![Vec::new(); n_compact];
        let mut access: Vec<Access> = Vec::new();
        // Pins that got no node, per compact net: an open the geometry cannot show.
        let mut unlanded = vec![0usize; n_compact];

        // Stage timers (RouteStats::us_*), and the spatial index bucket.
        let mut stats = RouteStats::default();
        let us = |t: std::time::Instant| t.elapsed().as_micros() as u64;
        let side = 4 * cfg.pitch;
        let t_landing = std::time::Instant::now();
        // Every pin grows a stitch pad; a foreign jog through that zone is a short.
        let zones: Vec<(u32, i32, i32)> = all_pins
            .iter()
            .map(|&(n, r, _)| (ci_of[n.0 as usize] as u32, r.x + r.w / 2 - origin.0, r.y + r.h / 2 - origin.1))
            .collect();
        let mut laid_legs: Vec<(usize, Rect)> = Vec::new();
        // Nodes another net owns under a laid jog: history [`JOG_HIST`] for this run.
        let mut jog_hist: Vec<u32> = Vec::new();

        // Landing-pad legality: a pad (or its cut) that stops closer than
        // min_spacing to cell li — or to the cut at the pin end — without merging
        // is a violation no reroute can fix.
        let pad_layer = cfg.pin_access.map_or_else(|| layers.first().copied(), |(l, _)| Some(l));
        let (access_cut, access_pad) = cfg
            .pin_access
            .map(|(_, (_, size, below, above))| (size, below.max(above)))
            .or_else(|| cuts.first().map(|&(_, size, below, above)| (size, below.max(above))))
            .unwrap_or((0, 0));
        let access_obstacles: Vec<Rect> = match pad_layer {
            Some(l0) if access_pad > 0 && cfg.pin_access_spacing > 0 => placed
                .iter()
                .chain(rings)
                .flat_map(|m| &m.shapes)
                .filter(|s| s.layer == l0)
                .map(|s| shift(s.rect))
                .collect(),
            _ => Vec::new(),
        };
        let good_site = |pin_c: (i32, i32), px: i32, py: i32| -> bool {
            let d = (px - pin_c.0).abs().max((py - pin_c.1).abs());
            if access_cut > 0 && cfg.pin_access_cut_spacing > 0 && d >= access_pad {
                let gap = d - access_cut;
                if gap > 0 && gap < cfg.pin_access_cut_spacing {
                    return false;
                }
            }
            let pad = Rect { x: px - access_pad / 2, y: py - access_pad / 2, w: access_pad, h: access_pad };
            !access_obstacles.iter().any(|&o| {
                let g = rect_gap(pad, o);
                g > 0 && g < cfg.pin_access_spacing
            })
        };
        // The default (unflipped, narrow) jog from a candidate must clear every
        // foreign pin's stitch zone.
        let stitch = cfg.wire_width.max(access_pad);
        // The widest pad a layer-0 node carries (a trunk, or the via up), and
        // the spacing the lattice keeps between pads.
        let pad0 = cfg.wire_width.max(cuts.first().map_or(0, |c| c.2));
        let pad_space = (cfg.pitch - cfg.wire_width.max(cuts.iter().map(|c| c.2.max(c.3)).max().unwrap_or(0))).max(0);
        let short_free = |ci: usize, pin: Rect, px: i32, py: i32| -> bool {
            let (cx, cy) = (pin.x + pin.w / 2, pin.y + pin.h / 2);
            let half = cfg.wire_width.min(pin.w).min(pin.h).max(1) / 2;
            let legs = [
                Rect { x: px.min(cx) - half, y: py - half, w: (px - cx).abs() + 2 * half, h: 2 * half },
                Rect { x: cx - half, y: py.min(cy) - half, w: 2 * half, h: (py - cy).abs() + 2 * half },
            ];
            zones.iter().all(|&(zci, zx, zy)| {
                let zone = Rect { x: zx - stitch / 2, y: zy - stitch / 2, w: stitch, h: stitch };
                zci as usize == ci || legs.iter().all(|&l| rect_gap(l, zone) > 0)
            })
        };

        // Reserve each pin's layer-0 stitch footprint for its net before anything
        // else claims nodes: a foreign trunk grazing the stitch pad is a drawn
        // short. Pins race first-come among themselves.
        // Footprint plus the wire spacing: a foreign trunk — or its via's pad,
        // the widest thing a layer-0 node carries — on a node just past the
        // pad would still sit closer than min spacing to it.
        // A node in reach of two nets' pins crowds whichever pad it does not
        // carry: no trunk may use it (`CONTESTED`), though a pin may still
        // land there as a last resort.
        let reach = (pad0 + stitch) / 2 + pad_space;
        for &(n, r, _) in &all_pins {
            let (pcx, pcy) = (r.x + r.w / 2 - origin.0, r.y + r.h / 2 - origin.1);
            let ci = ci_of[n.0 as usize] as u32;
            for iy in ((pcy - reach) / cfg.pitch).max(0)..=((pcy + reach) / cfg.pitch).min(grid.ny as i32 - 1) {
                for ix in ((pcx - reach) / cfg.pitch).max(0)..=((pcx + reach) / cfg.pitch).min(grid.nx as i32 - 1) {
                    let node = grid.node(ix as u32, iy as u32, 0);
                    let (px, py, _) = grid.pos(node);
                    if (px - pcx).abs() < reach && (py - pcy).abs() < reach {
                        let o = &mut reserved[node as usize];
                        *o = if *o == NONE || *o == ci { ci } else { CONTESTED };
                    }
                }
            }
        }
        // Ring and cell metal on a lattice layer, grown by the spacing a wire
        // needs from it plus half a wire: a node inside is a short or spacing
        // error for any trunk but the metal's own net's (it may merge with its
        // strap). Pin stitch nodes, reserved above, stay their pins'.
        let min_space = cfg.pitch - cfg.wire_width;
        let pre: Vec<bool> = reserved.iter().map(|&o| o == NONE).collect();
        for &(owner, s) in &foreign_metal {
            let Some(l) = layers.iter().position(|&l| l == s.layer).filter(|&l| (l as u32) < n_layers) else { continue };
            let own = owner.map(|n| ci_of[n as usize]).filter(|&c| c != usize::MAX).map_or(BLOCKED, |c| c as u32);
            let g = cfg.space(s.layer, s.rect.w.min(s.rect.h), cfg.wire(l), min_space) + cfg.wire(l) / 2;
            let r = shift(s.rect);
            let (x0, y0, x1, y1) = (r.x - g, r.y - g, r.x + r.w + g, r.y + r.h + g);
            for iy in grid.bin_y(y0)..=grid.bin_y(y1) {
                for ix in grid.bin_x(x0)..=grid.bin_x(x1) {
                    let n = grid.node(ix, iy, l as u32);
                    let (px, py, _) = grid.pos(n);
                    if pre[n as usize] && x0 < px && px < x1 && y0 < py && py < y1 {
                        let o = &mut reserved[n as usize];
                        *o = if *o == NONE || *o == own { own } else { BLOCKED };
                    }
                }
            }
        }
        // Hard blockages (RTE-16): a node of a blocked layer whose centre lies
        // inside the rect grown by its halo and half a wire is closed to every net
        // (`BLOCKED`), or, when the blockage spares the cell's own nets or
        // binds aggressors only, to the nets it applies to (`blocked_for`).
        // Per net, the rects (with layer bits) its access jogs keep out of.
        let applies = |b: &Blockage, ni: usize| {
            let own = placed.get(b.cell as usize).is_some_and(|m| m.pins.iter().any(|p| p.net.0 as usize == ni));
            !(b.own_exempt && own) && (!b.only_aggressors || cfg.aggressor.get(ni).copied().unwrap_or(false))
        };
        let mut blocked_for: Vec<Vec<u32>> = vec![Vec::new(); n_compact];
        let mut jog_blocks: Vec<Vec<(Rect, u16)>> = vec![Vec::new(); n_compact];
        for b in cfg.blockages.iter().filter(|b| b.hard) {
            let everyone = !b.own_exempt && !b.only_aggressors;
            let nets: Vec<usize> = (0..n_compact).filter(|&ci| everyone || applies(b, compact[ci])).collect();
            let r = grown(shift(b.rect), b.halo);
            for &ci in &nets {
                jog_blocks[ci].push((r, b.layers));
            }
            for l in (0..n_layers as usize).filter(|&l| b.layers >> l & 1 == 1) {
                let g = cfg.wire(l) / 2;
                let (x0, y0, x1, y1) = (r.x - g, r.y - g, r.x + r.w + g, r.y + r.h + g);
                for iy in grid.bin_y(y0)..=grid.bin_y(y1) {
                    for ix in grid.bin_x(x0)..=grid.bin_x(x1) {
                        let n = grid.node(ix, iy, l as u32);
                        let (px, py, _) = grid.pos(n);
                        if !(x0 < px && px < x1 && y0 < py && py < y1) {
                            continue;
                        }
                        // A pin's stitch node inside stays its pin's, but no
                        // net it binds may land or run there.
                        if everyone && pre[n as usize] {
                            reserved[n as usize] = BLOCKED;
                        } else {
                            nets.iter().for_each(|&ci| blocked_for[ci].push(n));
                        }
                    }
                }
            }
        }
        for v in &mut blocked_for {
            v.sort_unstable();
            v.dedup();
        }
        // The rects of `jog_blocks[ci]` that bind lattice layer `l`.
        let blocks_on = |ci: usize, l: u32| -> Vec<Rect> { jog_blocks[ci].iter().filter(|b| b.1 >> l & 1 == 1).map(|b| b.0).collect() };

        // RTE-15: exact matched pairs land jointly, before every other pin.
        // Per pair `(a = pos, b = neg)` from the `Mirror` batches, the lattice
        // map carrying a's pins exactly onto b's ([`pair_map`]); each a pin
        // (by `(y, x)`) lands on a node `n` clean for a whose image is clean
        // for b's partner pin, on a's side, with both jogs clean; both claim.
        // A pair whose map or landing fails routes alone (`pairs_fallback`).
        let clean_at = |reserved: &[u32], ci: usize, r: Rect, n: u32| -> bool {
            let (px, py, _) = grid.pos(n);
            let pad = Rect { x: px - pad0 / 2, y: py - pad0 / 2, w: pad0, h: pad0 };
            [NONE, CONTESTED, ci as u32].contains(&reserved[n as usize])
                && blocked_for[ci].binary_search(&n).is_err()
                && short_free(ci, r, px, py)
                && good_site((r.x + r.w / 2, r.y + r.h / 2), px, py)
                && zones.iter().all(|&(zci, zx, zy)| zci as usize == ci || rect_gap(pad, Rect { x: zx - stitch / 2, y: zy - stitch / 2, w: stitch, h: stitch }) >= pad_space)
        };
        let mut jointly: HashSet<(usize, i32, i32, i32, i32)> = HashSet::new();
        // The node each pin landed on, by `(compact net, pin rect)`.
        let mut landed_at: HashMap<(usize, i32, i32, i32, i32), u32> = HashMap::new();
        let mut exact: Vec<(usize, usize, gr::LatticeMap)> = Vec::new();
        let mut mirror_ids = Vec::new();
        for b in reqs.hard.iter().chain(&reqs.budget).filter(|b| b.repair_kind() == RepairKind::Mirror) {
            b.touched(&mut mirror_ids);
        }
        for p in mirror_ids.chunks_exact(2) {
            let (pa, pb) = (p[0] as usize, p[1] as usize);
            let (Some(&a), Some(&b)) = (ci_of.get(pa), ci_of.get(pb)) else { continue };
            if a == usize::MAX || b == usize::MAX || a == b || exact.iter().any(|e| [e.0, e.1].contains(&a) || [e.0, e.1].contains(&b)) {
                continue;
            }
            let map = match pair_map(&term_rects[pa], &term_rects[pb], &grid, cfg.grid) {
                Ok(m) => m,
                Err(why) => {
                    stats.pairs_fallback.push((pa as u32, pb as u32, why));
                    continue;
                }
            };
            let side = |n: u32, m: u32| match map {
                gr::LatticeMap::MirrorX { .. } => (grid.pos(m).0 - grid.pos(n).0).signum(),
                gr::LatticeMap::Shift { .. } => 0,
            };
            // A mirror keeps a's metal on a's side of the axis (toward b, the image).
            let want_side = {
                let mean = |v: &[(Rect, LayerId, Option<f32>)]| v.iter().map(|t| i64::from(t.0.x) * 2 + i64::from(t.0.w)).sum::<i64>() / v.len().max(1) as i64;
                match map {
                    gr::LatticeMap::MirrorX { .. } => (mean(&term_rects[pb]) - mean(&term_rects[pa])).signum() as i32,
                    gr::LatticeMap::Shift { .. } => 0,
                }
            };
            let (known_a, known_b) = (term_rects[pa].iter().all(|t| t.2.is_some()), term_rects[pb].iter().all(|t| t.2.is_some()));
            let mut pins_a = term_rects[pa].clone();
            pins_a.sort_by_key(|t| (t.0.y, t.0.x));
            let mut ok = true;
            for (ra, la, ua_a) in pins_a {
                let image = map_rect(ra, map, grid.pitch);
                let Some(&(rb, lb, ua_b)) = term_rects[pb].iter().find(|t| t.1 == la && (t.0.x - image.x).abs() <= cfg.grid && (t.0.y - image.y).abs() <= cfg.grid && t.0.w == image.w && t.0.h == image.h) else {
                    ok = false;
                    break;
                };
                let (ca, cb) = ((ra.x + ra.w / 2, ra.y + ra.h / 2), (rb.x + rb.w / 2, rb.y + rb.h / 2));
                let jog_l = jog_layer(cfg, layers, cuts, grid.n_layers, la);
                let (blk_a, blk_b) = (blocks_on(a, jog_l), blocks_on(b, jog_l));
                let metal = layers.get(jog_l as usize);
                let need = metal.map_or(0, |&m| access_need(cfg, m, ua_a.filter(|_| known_a)).max(access_need(cfg, m, ua_b.filter(|_| known_b))));
                let full = cfg.wire_width.max(1).max(need);
                let floor = cfg.min_width.iter().find(|&&(l, _)| Some(&l) == metal).map_or(1, |&(_, w)| w);
                let narrow = full.min(ra.w).min(ra.h).max(floor).max(need);
                let ok_n = |n: u32| {
                    clean_at(&reserved, a, ra, n) && grid.map(map, n).is_some_and(|m| m != n && !claimed[m as usize] && side(n, m) == want_side && clean_at(&reserved, b, rb, m))
                };
                let cands = grid.candidates(ca.0, ca.1, &claimed, ok_n, 8, 12);
                let spaced = (stitch, (cfg.pitch - cfg.wire_width).max(1));
                let pick = [spaced, (full, 1)].into_iter().find_map(|(zone, gap)| {
                    cands.iter().copied().find_map(|n| {
                        let m = grid.map(map, n)?;
                        let ((nx, ny, _), (mx, my, _)) = (grid.pos(n), grid.pos(m));
                        [full, narrow].into_iter().find_map(|w| {
                            let (la_, lb_) = (jog_legs(nx, ny, ca.0, ca.1, w), jog_legs(mx, my, cb.0, cb.1, w));
                            let f = (0..2).find(|&f| {
                                let mut laid = laid_legs.clone();
                                laid.extend(la_[f].iter().map(|&l| (a, l)));
                                jog_clean(&la_[f], a, zone, gap, &zones, &laid_legs, &blk_a) && jog_clean(&lb_[f], b, zone, gap, &zones, &laid, &blk_b)
                            })?;
                            Some((n, m, (w, f == 1), la_[f], lb_[f]))
                        })
                    })
                });
                let Some((n, m, choice, legs_a, legs_b)) = pick else {
                    ok = false;
                    break;
                };
                for (ci, r, rl, ua, node, legs, known) in [(a, ra, la, ua_a, n, legs_a, known_a), (b, rb, lb, ua_b, m, legs_b, known_b)] {
                    let ua = ua.filter(|_| known);
                    laid_legs.extend(legs.iter().map(|&l| (ci, l)));
                    jog_hist.extend(jog_hist_nodes(&grid, cfg, &legs, jog_l, &reserved, ci));
                    claimed[node as usize] = true;
                    landed_at.insert((ci, r.x, r.y, r.w, r.h), node);
                    reserved[node as usize] = ci as u32;
                    if let Some(ua) = ua {
                        node_ua[ci].push((node, ua));
                    }
                    if !c_terms[ci].contains(&node) {
                        c_terms[ci].push(node);
                    }
                    let (px, py, node_layer) = grid.pos(node);
                    let acc = Access { ci, pin: r, pin_layer: rl, node: (px, py), node_layer, choice: Some(choice), ua, node_ua: ua };
                    claim_jog_sweep(&grid, cfg, layers, cuts, &acc, &mut claimed, &mut reserved);
                    access.push(acc);
                    jointly.insert((ci, r.x, r.y, r.w, r.h));
                }
            }
            if ok && term_rects[pa].len() == term_rects[pb].len() {
                exact.push((a, b, map));
            } else {
                stats.pairs_fallback.push((pa as u32, pb as u32, "landing failed"));
            }
        }

        for (ci, &ni) in compact.iter().enumerate() {
            let known = term_rects[ni].iter().all(|t| t.2.is_some());
            for &(r, r_layer, ua) in &term_rects[ni] {
                if jointly.contains(&(ci, r.x, r.y, r.w, r.h)) {
                    continue;
                }
                // A net with any unknown terminal gets no EM sizing at all.
                let ua = ua.filter(|_| known);
                let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2);
                // Never land inside another net's stitch reservation.
                let mine = |n: u32| [NONE, CONTESTED, ci as u32].contains(&reserved[n as usize]) && blocked_for[ci].binary_search(&n).is_err();
                let at = |n: u32| {
                    let (px, py, _) = grid.pos(n);
                    (px, py)
                };
                let short_ok = |n: u32| mine(n) && short_free(ci, r, at(n).0, at(n).1);
                // A node whose pad would crowd another net's pin pad is a last
                // resort: that pin's own pad lands there regardless.
                let apart = |n: u32| {
                    let (px, py) = at(n);
                    let pad = Rect { x: px - pad0 / 2, y: py - pad0 / 2, w: pad0, h: pad0 };
                    zones.iter().all(|&(zci, zx, zy)| zci as usize == ci || rect_gap(pad, Rect { x: zx - stitch / 2, y: zy - stitch / 2, w: stitch, h: stitch }) >= pad_space)
                };
                let clean = |n: u32| short_ok(n) && good_site((cx, cy), at(n).0, at(n).1) && apart(n);
                // Joint node + jog choice first: the nearest candidate whose jog
                // (full or narrow width, either orientation) clears every foreign
                // zone and every jog already laid.
                let jog_l = jog_layer(cfg, layers, cuts, grid.n_layers, r_layer);
                let metal = layers.get(jog_l as usize);
                let blocks = blocks_on(ci, jog_l);
                // No candidate jog narrower than its EM width.
                let need = metal.map_or(0, |&m| access_need(cfg, m, ua));
                let full = cfg.wire_width.max(1).max(need);
                let floor = cfg.min_width.iter().find(|&&(l, _)| Some(&l) == metal).map_or(1, |&(_, w)| w);
                let narrow = full.min(r.w).min(r.h).max(floor).max(need);
                // A point terminal (1×1 rect) is its own node: nothing to jog to.
                let point = r.w <= 1 && r.h <= 1;
                let cands = if point { Vec::new() } else { grid.candidates(cx, cy, &claimed, clean, 8, 12) };
                // Spaced from every foreign pin's future pad first, then merely
                // not touching it.
                let spaced = (stitch, (cfg.pitch - cfg.wire_width).max(1));
                let joint = [spaced, (full, 1)].into_iter().find_map(|(zone, gap)| {
                    cands.iter().copied().find_map(|n| {
                        let (px, py, _) = grid.pos(n);
                        [full, narrow].into_iter().find_map(|w| {
                            let both = jog_legs(px, py, cx, cy, w);
                            let f = (0..2).find(|&f| jog_clean(&both[f], ci, zone, gap, &zones, &laid_legs, &blocks))?;
                            Some((n, (w, f == 1), both[f]))
                        })
                    })
                });
                // Fallback tiers, tight radii (a far node means a long blind jog):
                // clean, then short-free, then anything. Not landing beats a short.
                let landed = match joint {
                    Some((n, choice, legs)) => {
                        laid_legs.extend(legs.iter().map(|&l| (ci, l)));
                        // A jog over a node another net owns (its stitch reach,
                        // landing or jog) is where that net's trunk may run: a
                        // short `break_shorts` settles only by deleting this access.
                        // Only a node whose wire would touch a leg: the swept bins
                        // reach up to a pitch past it.
                        jog_hist.extend(jog_hist_nodes(&grid, cfg, &legs, jog_l, &reserved, ci));
                        Some((n, Some(choice)))
                    }
                    None => grid
                        .candidates(cx, cy, &claimed, clean, 2, 1)
                        .first()
                        .or(grid.candidates(cx, cy, &claimed, short_ok, 3, 1).first())
                        .or(grid.candidates(cx, cy, &claimed, mine, 4, 1).first())
                        .map(|&n| (n, None)),
                };
                let Some((n, choice)) = landed else {
                    unlanded[ci] += 1;
                    continue;
                };
                claimed[n as usize] = true;
                landed_at.insert((ci, r.x, r.y, r.w, r.h), n);
                reserved[n as usize] = ci as u32;
                if let Some(ua) = ua {
                    node_ua[ci].push((n, ua));
                }
                if !c_terms[ci].contains(&n) {
                    c_terms[ci].push(n);
                }
                if !point {
                    let (px, py, node_layer) = grid.pos(n);
                    let a = Access { ci, pin: r, pin_layer: r_layer, node: (px, py), node_layer, choice, ua, node_ua: ua };
                    claim_jog_sweep(&grid, cfg, layers, cuts, &a, &mut claimed, &mut reserved);
                    access.push(a);
                }
            }
        }

        // RTE-17: each group of a star node routes as its own pseudo-net (a
        // compact net past the real ones, `compact[pseudo]` = the parent's
        // `NetId`, so `build_routes` draws it into the parent): its pins'
        // landed nodes plus one root node of its own inside the feeds' halo
        // (`p0`); the parent keeps its other pins and joins the roots. The
        // pseudo-nets repel each other like any two nets, so the branches
        // meet only at the root. A halo with too few free nodes is a V.
        let mut owner: Vec<u32> = Vec::new();
        let mut star_v: Vec<Violation> = Vec::new();
        // Per star, its branches' pseudo-nets: kept a track apart (below).
        let mut star_sibs: Vec<Vec<usize>> = Vec::new();
        for node in cfg.common.iter().filter(|n| n.star && n.groups.len() > 1) {
            let n = node.net.0 as usize;
            let Some(ci) = ci_of.get(n).copied().filter(|&c| c != usize::MAX) else { continue };
            let pins: Vec<Vec<u32>> = node.groups.iter().map(|g| g.iter().filter_map(|&r| { let r = shift(r); landed_at.get(&(ci, r.x, r.y, r.w, r.h)).copied() }).collect()).collect();
            let p = grid.pitch;
            let mut roots: Vec<(i64, u32)> = Vec::new();
            for f in node.feeds.iter().map(|&f| grown(shift(f), p)) {
                let c = (f.x + f.w / 2, f.y + f.h / 2);
                for l in 0..grid.n_layers.min(2) {
                    for iy in grid.bin_y(f.y)..=grid.bin_y(f.y + f.h) {
                        for ix in grid.bin_x(f.x)..=grid.bin_x(f.x + f.w) {
                            let m = grid.node(ix, iy, l);
                            let (x, y, _) = grid.pos(m);
                            let inside = (f.x..=f.x + f.w).contains(&x) && (f.y..=f.y + f.h).contains(&y);
                            if inside && grid.on_track(m) && !claimed[m as usize] && [NONE, ci as u32].contains(&reserved[m as usize]) && blocked_for[ci].binary_search(&m).is_err() {
                                roots.push((i64::from((x - c.0).abs() + (y - c.1).abs()) + i64::from(l) * i64::from(p), m));
                            }
                        }
                    }
                }
            }
            roots.sort_unstable();
            roots.dedup_by_key(|r| r.1);
            if roots.len() < pins.len() || pins.iter().any(Vec::is_empty) {
                star_v.push(Violation { rule: format!("star root too small net {n}"), margin: (pins.len() - roots.len().min(pins.len())) as i64 });
                continue;
            }
            if owner.is_empty() {
                owner = (0..n_compact as u32).collect();
            }
            star_sibs.push((n_compact..n_compact + pins.len()).collect());
            for (g, &(_, root)) in pins.iter().zip(&roots) {
                claimed[root as usize] = true;
                reserved[root as usize] = ci as u32;
                c_terms[ci].retain(|t| !g.contains(t));
                c_terms[ci].push(root);
                let mut terms = vec![root];
                terms.extend(g.iter().copied().filter(|t| *t != root));
                terms.dedup();
                let ua: Vec<(u32, f32)> = node_ua[ci].iter().copied().filter(|e| g.contains(&e.0)).collect();
                node_ua[ci].retain(|e| !g.contains(&e.0));
                c_terms.push(terms);
                node_ua.push(ua);
                blocked_for.push(blocked_for[ci].clone());
                unlanded.push(0);
                compact.push(n);
                owner.push(ci as u32);
                n_compact += 1;
            }
        }
        stats.us_landing = us(t_landing);
        let weight: Vec<f32> = compact.iter().map(|&n| cfg.net_weight.get(n).copied().unwrap_or(0.0)).collect();
        let counts: Vec<usize> = c_terms.iter().map(Vec::len).collect();
        let net_ids: Vec<u32> = compact.iter().map(|&n| n as u32).collect();
        // Tracks per layer from EM current (C18), per branch: a net's
        // terminals join in a Prim MST over Manhattan distance rooted at the
        // largest |I| ([`prim`]); the branch to each terminal gets the
        // [`tracks`] of the current on its MST edge, and the targets route in
        // MST order ([`gr::NetSearch::term_k`]). A net whose `I_max` (the
        // larger of what its terminals draw and supply, which KCL bounds every
        // segment by) fits one wire on every limited layer keeps one track, as
        // does one with no limit or no current. Guard tracks are the net's,
        // at `I_max`.
        // ponytail: the MST can differ from the routed tree; the EM rounds
        // after geometry close that gap on the drawn shapes.
        let n_l = (grid.n_layers as usize).min(gr::MAX_LAYERS);
        let (mut ks, mut guards) = (Vec::new(), Vec::new());
        let mut term_k: Vec<Vec<[u8; gr::MAX_LAYERS]>> = vec![Vec::new(); n_compact];
        if node_ua.iter().any(|t| !t.is_empty()) {
            ks = vec![[1u8; gr::MAX_LAYERS]; n_compact];
            guards = vec![[0u8; gr::MAX_LAYERS]; n_compact];
            let i_min = (0..n_l)
                .filter_map(|l| cfg.em_limit(layers[l]).map(|lim| lim.ua_per_um).filter(|&j| j > 0.0).map(|j| j * cfg.wire(l) as f32 / 1_000.0))
                .fold(f32::INFINITY, f32::min);
            for (ci, t) in node_ua.iter().enumerate().filter(|(_, t)| !t.is_empty()) {
                let (inn, out) = t.iter().fold((0.0f32, 0.0f32), |(i, o), &(_, ua)| if ua > 0.0 { (i + ua, o) } else { (i, o - ua) });
                let i_max = inn.max(out);
                if i_max <= i_min {
                    continue;
                }
                let ua: Vec<f32> = c_terms[ci].iter().map(|&n| t.iter().filter(|e| e.0 == n).map(|e| e.1).sum()).collect();
                let pos: Vec<(i32, i32)> = c_terms[ci].iter().map(|&n| (grid.pos(n).0, grid.pos(n).1)).collect();
                let (visit, edge, parent) = prim(&pos, &ua);
                for (i, &n) in c_terms[ci].iter().enumerate() {
                    let junction = (0..pos.len()).filter(|&j| j == i || (parent[j] == i && j != visit[0])).map(|j| edge[j]).fold(0.0, f32::max);
                    let (x, y, l) = grid.pos(n);
                    for a in access.iter_mut().filter(|a| a.ci == ci && a.node == (x, y) && a.node_layer == l) {
                        a.node_ua = a.ua.map(|u| u.abs().max(junction));
                        // The junction can widen the jog past what landing
                        // swept: claim the wider corridor (first come still).
                        claim_jog_sweep(&grid, cfg, layers, cuts, a, &mut claimed, &mut reserved);
                    }
                }
                c_terms[ci] = visit.iter().map(|&i| c_terms[ci][i]).collect();
                let mut tk: Vec<[u8; gr::MAX_LAYERS]> = visit
                    .iter()
                    .map(|&i| std::array::from_fn(|l| if l < n_l { tracks(cfg, l, layers[l], edge[i]).0 } else { 1 }))
                    .collect();
                tk[0] = max_k(&tk[1..]);
                ks[ci] = tk[0];
                for l in 0..n_l {
                    guards[ci][l] = tracks(cfg, l, layers[l], i_max).1;
                }
                term_k[ci] = tk;
            }
        }
        let wide: Vec<bool> = (0..n_compact).map(|ci| ks.get(ci).is_some_and(|k: &[u8; gr::MAX_LAYERS]| k.iter().any(|&t| t > 1))).collect();
        let mut cold = RouteCtx {
            order: gr::order_by_priority(&counts, &net_ids, reqs, &weight, &wide),
            graph: grid,
            terms: c_terms,
            reserved,
            weight,
            layer_c: cfg.layer_c.clone(),
            beside_c: cfg.beside_c.clone(),
            cross_c: cfg.cross_c.clone(),
            sep: Vec::new(),
            // Series R is priced only once a drop budget is broken (below).
            current: Vec::new(),
            layer_r: cfg.layer_r.clone(),
            via_r: cfg.via_r.clone(),
            mirror: Vec::new(),
            owner,
            keepout: Vec::new(),
            own_cells: Vec::new(),
            blocked_for,
            k: ks,
            guard: guards,
            term_k,
        };
        // Exact pairs: b's terminals are a's images, in a's order, at tracks
        // tied to the larger of the two per terminal; a vertical-layer pair
        // wider than one track falls back (its image would grow toward −x).
        for (a, b, map) in exact {
            if tie_pair(&mut cold, a, b, map) {
                stats.pairs_exact.push((compact[a] as u32, compact[b] as u32));
            } else {
                stats.pairs_fallback.push((compact[a] as u32, compact[b] as u32, "wide pair"));
            }
        }
        // Soft blockages: nets without a pin in the cell pay `KEEPOUT_COST`
        // per node inside the rect; its own nets (finger straps, drains) do not.
        let soft: Vec<&Blockage> = cfg.blockages.iter().filter(|b| !b.hard).collect();
        if !soft.is_empty() {
            let mut keep = vec![NONE; cold.graph.nodes()];
            for b in &soft {
                let r = grown(shift(b.rect), b.halo);
                for l in (0..n_layers).filter(|&l| b.layers >> l & 1 == 1) {
                    for y in cold.graph.bin_y(r.y)..=cold.graph.bin_y(r.y + r.h) {
                        for x in cold.graph.bin_x(r.x)..=cold.graph.bin_x(r.x + r.w) {
                            keep[cold.graph.node(x, y, l) as usize] = b.cell;
                        }
                    }
                }
            }
            cold.keepout = keep;
            let mut cells: Vec<u32> = soft.iter().map(|b| b.cell).collect();
            cells.sort_unstable();
            cells.dedup();
            cold.own_cells = compact
                .iter()
                .map(|&n| cells.iter().copied().filter(|&c| placed.get(c as usize).is_some_and(|m| m.pins.iter().any(|p| p.net.0 as usize == n))).collect())
                .collect();
        }
        // RTE-19: a shield's victim routes with a guard track each side on
        // every layer whose edge gap `s·p0 − (W(k) + wire)/2` is within the
        // shield's `max_gap`: foreign metal keeps off them during the search,
        // and the shield stage hands them to the reference. No such layer:
        // a budget V, and no shield.
        let mut asks = Vec::new();
        for b in reqs.hard.iter().chain(&reqs.budget) {
            b.shield_pairs(&mut asks);
        }
        let mut shield_v = Vec::new();
        let mut shielded: Vec<(usize, usize, [u8; gr::MAX_LAYERS])> = Vec::new();
        for (v, rf, max_gap) in asks {
            let ci = |n: u32| ci_of.get(n as usize).copied().filter(|&c| c != usize::MAX);
            let (Some(v), Some(rf)) = (ci(v), ci(rf)) else { continue };
            if cold.guard.is_empty() {
                cold.guard = vec![[0; gr::MAX_LAYERS]; n_compact];
            }
            let (k, old) = (cold.k.get(v).copied().unwrap_or([1; gr::MAX_LAYERS]), cold.guard[v]);
            let (mut nearest, mut any) = (i32::MAX, false);
            for l in 0..n_l {
                let step = cfg.stride(l) * cold.graph.pitch;
                let edge = step - (cfg.wire(l) + (i32::from(k[l]) - 1) * step + cfg.wire(l)) / 2;
                nearest = nearest.min(edge);
                if edge <= max_gap {
                    cold.guard[v][l] = cold.guard[v][l].max(1);
                    any = true;
                }
            }
            if !any {
                shield_v.push(Violation { rule: format!("shield gap off lattice net {}", compact[v]), margin: i64::from(nearest - max_gap) });
            } else {
                shielded.push((v, rf, old));
            }
        }
        let mut hot = RouteHot::new(cold.graph.nodes(), n_compact);
        let agg_w: Vec<f32> = if cfg.aggressor_weight.is_empty() { Vec::new() } else { compact.iter().map(|&n| cfg.aggressor_weight.get(n).copied().unwrap_or(1.0)).collect() };
        hot.set_weights(cold.weight.clone(), agg_w);
        // Hard separations (RTE-18): per net, the tracks across it keeps from
        // the other, per layer, `⌈max(0, lateral − (s·p0 − wire)) / (s·p0)⌉`;
        // both nets carry the entry, the later-routed one avoids.
        let mut seps = Vec::new();
        for b in reqs.hard.iter().chain(&reqs.budget) {
            b.separations(&mut seps);
        }
        analog::RuleBatch::separations(&plate_rules((0, 0)), &mut seps);
        // A star's branches keep a free track between them: adjacent same-net
        // runs would be merged by the same-net fill, joining the branches.
        for sibs in &star_sibs {
            for (i, &a) in sibs.iter().enumerate() {
                for &b in &sibs[i + 1..] {
                    if cold.sep.is_empty() {
                        cold.sep = vec![Vec::new(); n_compact];
                    }
                    cold.sep[a].push((b as u32, [1; gr::MAX_LAYERS], false));
                    cold.sep[b].push((a as u32, [1; gr::MAX_LAYERS], false));
                }
            }
        }
        if !seps.is_empty() {
            if cold.sep.is_empty() {
                cold.sep = vec![Vec::new(); n_compact];
            }
            let p = cold.graph.pitch;
            for (a, b, lateral, no_cross) in seps {
                let (Some(a), Some(b)) = (ci_of.get(a as usize).copied().filter(|&c| c != usize::MAX), ci_of.get(b as usize).copied().filter(|&c| c != usize::MAX)) else { continue };
                let tracks: [u8; gr::MAX_LAYERS] = std::array::from_fn(|l| {
                    let step = cfg.stride(l) * p;
                    ((lateral - (step - cfg.wire(l))).max(0) + step - 1).div_euclid(step.max(1)).min(i32::from(u8::MAX)) as u8
                });
                cold.sep[a].push((b as u32, tracks, no_cross));
                cold.sep[b].push((a as u32, tracks, no_cross));
            }
        }
        // History is keyed by absolute position: add the frame shift back.
        let abs = |n: u32| {
            let (x, y, l) = cold.graph.pos(n);
            (x + origin.0, y + origin.1, l)
        };
        neg.seed(&mut hot.hist, abs);
        jog_hist.iter().for_each(|&n| hot.hist[n as usize] += JOG_HIST);
        // One search scratch for the whole call: negotiation, repair, shields.
        let mut dij = Dij::new(cold.graph.nodes());
        let t = std::time::Instant::now();
        (_, stats.pf_iters) = run_pathfinder(&mut hot, &cold, P_FAC, HIST_INC, MAX_ITERS, &mut dij);
        stats.us_negotiate = us(t);
        let t_repair = std::time::Instant::now();

        // A net over its IR-drop budget reroutes pricing its series R, weighted
        // by the DC current it carries (the larger of what its pins draw and
        // supply) over the heaviest such net's. A net within budget does not
        // pay: extra length or vias spent on R that buys nothing is only C.
        let broken = {
            let routes = build_routes(&hot, &cold, cfg, &compact, n_nets, layers, cuts);
            let mut ids = Vec::new();
            for b in reqs.budget.iter().filter(|b| b.repair_kind() == RepairKind::Ir && b.residual(&routes) > 0.0) {
                b.violating_ids(&routes, &mut ids);
            }
            ids
        };
        if !broken.is_empty() {
            let carry: Vec<f32> = node_ua
                .iter()
                .zip(&compact)
                .map(|(t, &n)| {
                    let (inn, out) = t.iter().fold((0.0f32, 0.0f32), |(i, o), &(_, ua)| if ua > 0.0 { (i + ua, o) } else { (i, o - ua) });
                    if broken.contains(&(n as u32)) { inn.max(out) } else { 0.0 }
                })
                .collect();
            let top = carry.iter().copied().fold(0.0, f32::max);
            cold.current = carry.iter().map(|&c| if top > 0.0 { c / top } else { 0.0 }).collect();
        }

        // Constraint repair: the analog rules never make a net congestion-dirty, so
        // PathFinder alone ignores them. Rip up what they name, reroute steered by
        // the rule's own data, keep only what improves the rules.
        // The probe draws pin access too (RTE-23): a rule its jogs break is
        // seen by repair. Frame coordinates; access violations are reported
        // on the shipped drawing.
        let joins = joins(layers, cuts, cfg.pin_access);
        let probe = |hot: &RouteHot, cold: &RouteCtx<TrackGrid>| {
            let mut r = Routes { cell: cell_f.clone(), gates: gates_f.clone(), ..build_routes(hot, cold, cfg, &compact, n_nets, layers, cuts) };
            add_pin_access(&mut r, &access, &compact, cfg, layers, cuts, &joins, &zones, origin, &jog_blocks);
            r
        };
        // Common-node balance, in the routing frame.
        let common: Vec<analog::routing::CommonNode> = cfg
            .common
            .iter()
            .map(|n| analog::routing::CommonNode {
                groups: n.groups.iter().map(|g| g.iter().map(|&r| shift(r)).collect()).collect(),
                feeds: n.feeds.iter().map(|&r| shift(r)).collect(),
                ..n.clone()
            })
            .collect();
        let mut extra: Vec<Box<dyn analog::RuleBatch<Routes>>> = match cfg.stack {
            Some(stack) if !common.is_empty() => vec![Box::new(analog::routing::CommonNodes { nodes: common.clone(), stack, halo_nm: cfg.pitch, joins: joins.clone() })],
            _ => Vec::new(),
        };
        if !cfg.plates.is_empty() {
            extra.push(Box::new(plate_rules(origin)));
        }
        // Per net, for the antenna lift: the top lattice layer of its cells'
        // metal (a plate), and each gate's landed node and pin.
        let lift: Vec<Option<(u32, Vec<((i32, i32), Rect)>)>> = compact
            .iter()
            .enumerate()
            .map(|(ci, &ni)| {
                let top = cell_f[ni].iter().filter_map(|s| layers.iter().position(|&l| l == s.layer)).max()? as u32;
                let sites: Vec<_> = access.iter().filter(|a| a.ci == ci && gates_f[ni].iter().any(|g| g.at == a.pin)).map(|a| (a.node, a.pin)).collect();
                (top + 1 < n_layers && !sites.is_empty()).then_some((top, sites))
            })
            .collect();
        stats.trials = repair_constraints(&mut hot, &cold, reqs, &extra, &common, &ci_of, &lift, &probe, &mut dij);

        // The jog price is this layout's, not negotiation history.
        jog_hist.iter().for_each(|&n| hot.hist[n as usize] -= JOG_HIST);
        stats.congestion = congestion(&hot, &cold.graph, origin);
        neg.accumulate(&hot.hist, abs);
        // Shields: the victim gives up its guard tracks (recommitted, so the
        // search's free side tracks are free), then the reference claims
        // them, tied in by rerouting it to them.
        for (v, rf, old) in shielded {
            cold.guard[v] = old;
            let tree = hot.trees[v].clone();
            cold.commit(&mut hot, v, tree);
            add_shield(&mut hot, &mut cold, v, rf, &mut dij);
        }
        stats.us_repair = us(t_repair);
        // Post-fill repair (RTE-23): a hard rule the probe saw satisfied but
        // the drawing (arrays, fill, stubs, terminals) breaks reroutes the nets
        // it names and redraws, up to `POST_ROUNDS`; a round is kept only when
        // the drawing's `(hard violations, Σ residual)` improves without
        // raising overuse, else every tree and the previous drawing return.
        // dr's own drawn rows (opens, shorts, sacrificed or failed access)
        // count as hard violations there: a round never trades a rule for a
        // short.
        let pre: Vec<u32> = {
            let r = probe(&hot, &cold);
            reqs.hard.iter().map(|b| b.violations(&r)).collect()
        };
        let metals: Vec<LayerId> = layers[..n_layers as usize].to_vec();
        let plates = plate_rules((0, 0)).0;
        let none = Requirements::<Routes>::default();
        let drawn_key = |r: &Routes, hot: &RouteHot, sacrificed: &[usize], access_v: &[Violation]| {
            let own = score(r, &none, 0.0, &joins, &foreign_metal, sacrificed, side).hard_violations.len() + access_v.len();
            (reqs.hard.iter().map(|b| b.violations(r)).sum::<u32>() + own as u32, reqs.hard.iter().chain(&reqs.budget).map(|b| b.residual(r)).sum::<f64>(), overuse(hot))
        };
        #[allow(clippy::type_complexity)]
        let mut prev: Option<((u32, f64, f32), Vec<Vec<Vec<u32>>>, Vec<[u8; gr::MAX_LAYERS]>, Vec<Vec<[u8; gr::MAX_LAYERS]>>, Vec<Option<(u32, gr::LatticeMap, bool)>>, RouteStats, (Routes, Vec<usize>, Vec<Violation>))> = None;
        let mut post = 0;
        let (routes, sacrificed, access_v) = loop {
            stats.plate_spread_pct.clear();
            let mut round = 0;
            let (mut routes, sacrificed, access_v) = loop {
                let t_geometry = std::time::Instant::now();
                // Counts the drawing this pass makes, not every redraw.
                stats.pair_fillers_dropped = 0;
                (stats.single_cut_vias, stats.stack_vias, stats.corners_unsupported) = (0, 0, 0);
                let mut routes = build_routes(&hot, &cold, cfg, &compact, n_nets, layers, cuts);
                // Shapes at or past this index per net are access geometry — the only
                // shapes the short resolver may sacrifice.
                let pre_access: Vec<usize> = routes.wires.iter().map(Vec::len).collect();
                let access_v = add_pin_access(&mut routes, &access, &compact, cfg, layers, cuts, &joins, &zones, origin, &jog_blocks);

                // A drawn short must never ship silently. Break each by deleting access
                // geometry (an open is reported), counted per net so `score` reports pins
                // that may now float.
                let mut sacrificed = vec![0usize; n_nets];
                for (ci, &n) in unlanded.iter().enumerate() {
                    sacrificed[compact[ci]] += n;
                }
                let mut rank = vec![usize::MAX; n_nets];
                for (k, &ci) in cold.order.iter().enumerate() {
                    rank[compact[ci as usize]] = k;
                }
                break_shorts(&mut routes.wires, &pre_access, &rank, &joins, &mut sacrificed, side);

                // A net's own pin rects, and the cell metal touching them on their
                // layer (the pin's lead), join it for the fill — a trunk stopping short
                // of its pin's metal is a sliver or notch too — then leave: they are
                // the cell's. All other cell metal is foreign.
                // The pin-access conductor too: its landing pads meet the cells' own
                // straps and rails there.
                let fill_layers: Vec<LayerId> = layers.iter().copied().chain(cfg.pin_access.map(|(l, _)| l)).collect();
                // Cell metal on every fill layer: on a deck whose pin-access layer is
                // not routed (metal1 on ihp) it is still a net's own lead or foreign.
                let fill_metal: Vec<Shape> = placed
                    .iter()
                    .chain(rings)
                    .flat_map(|m| &m.shapes)
                    .filter(|s| fill_layers.contains(&s.layer))
                    .map(|s| Shape { rect: shift(s.rect), ..*s })
                    .collect();
                // Each hard blockage, as drawn, per blocked layer, with the compact nets it binds.
                let hard_fill: Vec<(Vec<usize>, Shape)> = cfg
                    .blockages
                    .iter()
                    .filter(|b| b.hard)
                    .flat_map(|b| {
                        let nets: Vec<usize> = (0..n_compact).filter(|&ci| (!b.own_exempt && !b.only_aggressors) || applies(b, compact[ci])).collect();
                        (0..n_layers as usize).filter(move |&l| b.layers >> l & 1 == 1).map(move |l| (nets.clone(), Shape { layer: layers[l], rect: shift(b.rect) }))
                    })
                    .collect();
                let touch = |a: Rect, b: Rect| rect_gap(a, b) == 0;
                let cell_of = |n: usize| -> Vec<Shape> {
                    // The pins, then every cell shape joined to them on their layer
                    // (a pin, its rail, the strap off the rail), by flood fill.
                    let mut out: Vec<Shape> = term_rects.get(n).into_iter().flatten().map(|&(r, l, _)| Shape { layer: l, rect: r }).collect();
                    let mut taken = vec![false; fill_metal.len()];
                    let mut i = 0;
                    while i < out.len() {
                        let a = out[i];
                        for (k, c) in fill_metal.iter().enumerate() {
                            if !taken[k] && c.layer == a.layer && touch(a.rect, c.rect) {
                                taken[k] = true;
                                out.push(*c);
                            }
                        }
                        i += 1;
                    }
                    out
                };
                // Per layer: its own spacing (met3's is not met1's).
                let fill_space = |l: LayerId| {
                    let floor = if cfg.pin_access.is_some_and(|(p, _)| p == l) { min_space.max(cfg.pin_access_spacing) } else { min_space };
                    cfg.space(l, 0, 0, floor)
                };
                // The non-route metal each net keeps off: other nets' pins, cell metal
                // not its own lead, and the hard blockages binding it.
                let statics: Vec<Vec<Shape>> = (0..n_nets)
                    .map(|ni| {
                        let mine = cell_of(ni);
                        (0..n_nets)
                            .filter(|&n| n != ni)
                            .flat_map(|n| term_rects[n].iter().map(|&(r, l, _)| Shape { layer: l, rect: r }))
                            .chain(fill_metal.iter().copied().filter(|c| !mine.contains(c)))
                            .chain(hard_fill.iter().filter(|(c, _)| ci_of.get(ni).is_some_and(|&c0| c0 != usize::MAX && c.contains(&c0))).map(|&(_, s)| s))
                            .collect()
                    })
                    .collect();
                // Metal added after search (redundant cuts, corner stubs and
                // squares) goes where it clears: a cut every cut but itself by cut
                // spacing (same-net too), metal the other nets' wires and its
                // statics by the fill's spacing, its own net's metal by that spacing
                // unless it touches (no new notch or sliver).
                //
                // ponytail: linear scan per candidate shape (O(sites × shapes)).
                // Switch to a `ShapeIndex` if `us_fill` rises visibly on ota.
                let md = |r: Rect| r.w.min(r.h);
                let clear = |n: usize, s: Shape, wires: &[Vec<Shape>]| -> bool {
                    if let Some(&(_, size, ..)) = cuts.iter().find(|c| c.0 == s.layer) {
                        let sp = cfg.space(s.layer, 0, 0, size);
                        return wires.iter().flatten().all(|g| g.layer != s.layer || *g == s || rect_gap(g.rect, s.rect) >= sp);
                    }
                    let ok = |g: &Shape| g.layer != s.layer || rect_gap(g.rect, s.rect) >= cfg.space(s.layer, md(s.rect), md(g.rect), fill_space(s.layer));
                    statics[n].iter().all(ok)
                        && wires.iter().enumerate().filter(|&(m, _)| m != n).flat_map(|(_, w)| w).all(ok)
                        && wires.get(n).is_none_or(|w| w.iter().all(|g| g.layer != s.layer || rect_gap(g.rect, s.rect) == 0 || ok(g)))
                };
                let mut image: Vec<Option<(usize, gr::LatticeMap)>> = vec![None; n_nets];
                for (a, m) in cold.mirror.iter().enumerate() {
                    if let Some((b, map, true)) = *m {
                        image[compact[a]] = Some((compact[b as usize], map));
                    }
                }
                let pitch = cold.graph.pitch;
                // EM-critical nets (RTE-14: a current spread gives branch widths).
                let crit: Vec<bool> = (0..n_nets).map(|n| ci_of.get(n).and_then(|&ci| cold.term_k.get(ci)).is_some_and(|t| !t.is_empty())).collect();

                // GAP-12 (H15-39 recipe (b)): a critical net's cut in a merged
                // corner block moves onto the straight run beside it, with a
                // lower-metal stub under it, so the array below fills a straight
                // overlap. No crowding credit. Skipped where the stub or cut
                // would not clear.
                {
                    let stubs: Vec<Vec<(Rect, [Shape; 2])>> =
                        routes.wires.iter().enumerate().map(|(n, w)| if crit[n] { corner_stubs(w, cfg, layers, cuts) } else { Vec::new() }).collect();
                    let sites: Vec<Vec<Vec<Vec<Shape>>>> = stubs.iter().map(|v| v.iter().map(|(_, alt)| vec![alt.to_vec()]).collect()).collect();
                    let taken = add_where_clear(&mut routes.wires, &sites, &image, pitch, &clear);
                    for (n, t) in taken.iter().enumerate() {
                        for (&(c, [_, cut]), _) in stubs[n].iter().zip(t).filter(|(_, k)| k.is_some()) {
                            routes.wires[n].retain(|s| !(s.layer == cut.layer && s.rect == c));
                            if let Some((b, map)) = image[n] {
                                let m = map_rect(c, map, pitch);
                                routes.wires[b].retain(|s| !(s.layer == cut.layer && s.rect == m));
                            }
                        }
                    }
                }

                // Via arrays: a cut between two multi-track runs becomes as many cuts as fit
                // in their overlap (deck cut size, cut spacing, per-axis enclosure). The
                // hard `Electromigration` rule checks the count (Lienig eq. 3.25) on the
                // drawn routes; a short group bumps its branches' tracks below.
                //
                // ponytail: equal sharing across an array's cuts; crowding at a turn
                // (Lienig §4.6.4) is unchecked.
                let all_cuts: Vec<(usize, Shape)> = routes
                    .wires
                    .iter()
                    .enumerate()
                    .flat_map(|(n, w)| w.iter().filter(|c| cuts.iter().any(|&(l, ..)| l == c.layer)).map(move |c| (n, *c)))
                    .collect();
                let cut_index = ShapeIndex::new(side, all_cuts.iter().map(|(_, c)| (c.layer, c.rect)));
                // Per net, the stack vias left with one cut: (lattice index, cut).
                let mut singles: Vec<Vec<(usize, Rect)>> = vec![Vec::new(); n_nets];
                for (net, wires) in routes.wires.iter_mut().enumerate() {
                    // The pair of same-net rects on `lo` and `hi` that both fully
                    // cover `r` with the largest overlap.
                    let best = |wires: &[Shape], lo: LayerId, hi: LayerId, r: Rect| {
                        let on = |l: LayerId| wires.iter().filter(move |m| m.layer == l && contains(m.rect, r)).map(|m| m.rect);
                        let area = |(a, b): &(Rect, Rect)| {
                            let w = (a.x + a.w).min(b.x + b.w) - a.x.max(b.x);
                            let h = (a.y + a.h).min(b.y + b.h) - a.y.max(b.y);
                            i64::from(w) * i64::from(h)
                        };
                        on(lo).flat_map(|a| on(hi).map(move |b| (a, b))).max_by_key(area)
                    };
                    let mut out = Vec::with_capacity(wires.len());
                    // Array cuts placed for earlier cuts of this net.
                    let mut placed: Vec<Shape> = Vec::new();
                    for c in wires.iter() {
                        let Some(i) = cuts.iter().position(|&(l, ..)| l == c.layer) else {
                            out.push(*c);
                            continue;
                        };
                        let (_, size, below, above) = cuts[i];
                        // Per axis, the larger of what the two metals need there: a
                        // horizontal metal's along-wire enclosure is on x.
                        let (ex, ey) = match cfg.cut_enclosure_pair.iter().find(|e| e.0 == c.layer) {
                            Some(&(_, lo, hi)) => {
                                let horiz = |k: usize| cfg.layers.get(k).map_or(k % 2 == 0, |s| s.horizontal);
                                let xy = |k: usize, (across, along): (i32, i32)| if horiz(k) { (along, across) } else { (across, along) };
                                let (a, b) = (xy(i, lo), xy(i + 1, hi));
                                (a.0.max(b.0), a.1.max(b.1))
                            }
                            None => ((below.max(above) - size) / 2, (below.max(above) - size) / 2),
                        };
                        let Some((a, b)) = best(wires, layers[i], layers[i + 1], c.rect) else {
                            out.push(*c);
                            stats.stack_vias += 1;
                            singles[net].push((i, c.rect));
                            continue;
                        };
                        // A cell cut of this net within cut spacing whose own metal on
                        // both layers this cut's wires overlap (a cap array's tie stack
                        // under its pin lead) already joins them: this cut would only
                        // break the spacing, which binds same-net cuts too.
                        let space = cfg.space(c.layer, 0, 0, size);
                        let mine = cell_f.get(net).map_or(&[][..], Vec::as_slice);
                        let hosts = |l: LayerId, w: Rect, f: Rect| mine.iter().any(|m| m.layer == l && contains(m.rect, f) && rect_gap(m.rect, w) == 0);
                        if mine.iter().any(|f| f.layer == c.layer && (1..space).contains(&rect_gap(f.rect, c.rect)) && hosts(layers[i], a, f.rect) && hosts(layers[i + 1], b, f.rect)) {
                            continue;
                        }
                        stats.stack_vias += 1;
                        let (x, y) = (a.x.max(b.x) + ex, a.y.max(b.y) + ey);
                        let w = (a.x + a.w).min(b.x + b.w) - ex - x;
                        let h = (a.y + a.h).min(b.y + b.h) - ey - y;
                        let fit = |space: i32| {
                            let pitch = size + space;
                            (pitch, (w - size) / pitch + 1, (h - size) / pitch + 1)
                        };
                        let (mut pitch, mut nx, mut ny) = fit(cfg.space(c.layer, 0, 0, size));
                        if let Some(&(_, count, space)) = cfg.array_spacing.iter().find(|(l, ..)| *l == c.layer) {
                            if nx * ny >= count {
                                (pitch, nx, ny) = fit(space);
                            }
                        }
                        let cut_space = pitch - size;
                        // An overlap narrower than one enclosed cut holds no array
                        // (`(w − size)/pitch + 1` truncates a negative to 1).
                        if nx * ny < 2 || w < size || h < size {
                            out.push(*c);
                            singles[net].push((i, c.rect));
                            continue;
                        }
                        // Centred in the overlap, snapped to the nearest grid
                        // point, a tie toward the original cut's centre: both
                        // commute with a mirror or shift of the whole net, so an
                        // exact pair's arrays stay images (RTE-15; flooring a
                        // halved width does not).
                        let g = cfg.grid.max(1);
                        let snap = |lo: i32, room: i32, span: i32, orig2: i32| {
                            let l2 = 2 * lo + room - span;
                            let floor = l2.div_euclid(2 * g) * g;
                            let (d_lo, d_hi) = (l2 - 2 * floor, 2 * (floor + g) - l2);
                            let off = |v: i32| (2 * v + span - orig2).abs();
                            if d_lo < d_hi || (d_lo == d_hi && off(floor) <= off(floor + g)) { floor } else { floor + g }
                        };
                        let x0 = snap(x, w, (nx - 1) * pitch + size, 2 * c.rect.x + c.rect.w);
                        let y0 = snap(y, h, (ny - 1) * pitch + size, 2 * c.rect.y + c.rect.h);
                        let before = out.len();
                        for ix in 0..nx {
                            for iy in 0..ny {
                                let r = Rect { x: x0 + ix * pitch, y: y0 + iy * pitch, w: size, h: size };
                                // Cut spacing binds same-net cuts too: every other original cut,
                                // and every array cut already placed.
                                let ok = |f: &Shape| f.layer != c.layer || rect_gap(f.rect, r) >= cut_space;
                                let clear = cut_index.near(c.layer, r, cut_space).all(|k| {
                                    let (n, f) = all_cuts[k as usize];
                                    (n == net && f.rect == c.rect) || ok(&f)
                                })
                                    && out[before..].iter().chain(&placed).all(ok);
                                if clear {
                                    out.push(Shape { layer: c.layer, rect: r });
                                }
                            }
                        }
                        if out.len() == before {
                            out.push(*c);
                        }
                        if out.len() - before == 1 {
                            singles[net].push((i, out[before].rect));
                        }
                        placed.extend_from_slice(&out[before..]);
                    }
                    *wires = out;
                }

                // RTE-27: a single-cut stack via gets a second cut one cut pitch
                // along its lower metal's direction (either side), else along its
                // upper metal's, with that cut's pads joined to the first's, where
                // they clear. Along the lower
                // wire's current, so REL-12's front row gains nothing: this is
                // redundancy against voiding, not EM capacity.
                // A single within cut pitch of another same-net cut (a pin-access
                // cut beside an array) is already grouped: no site, not counted.
                let sites: Vec<Vec<Vec<Vec<Shape>>>> = singles
                    .iter()
                    .enumerate()
                    .map(|(n, v)| {
                        v.iter()
                            .filter(|&&(i, c)| {
                                let (cut, size, ..) = cuts[i];
                                let sp = cfg.space(cut, 0, 0, size);
                                !routes.wires[n].iter().any(|g| g.layer == cut && g.rect != c && rect_gap(g.rect, c) <= sp)
                            })
                            .map(|&(i, c)| {
                                let (cut, size, ..) = cuts[i];
                                let p = size + cfg.space(cut, 0, 0, size);
                                let horiz = cfg.layers.get(i).map_or(i % 2 == 0, |s| s.horizontal);
                                let (lo, hi) = via_pads(cfg, layers, cuts[i], i, c).into();
                                // Along the lower wire first (spec); across it, along the
                                // upper wire, where the upper pad would reach a
                                // neighbour's track (most misses on ota, met2 beside met2).
                                [(horiz, p), (horiz, -p), (!horiz, p), (!horiz, -p)]
                                    .into_iter()
                                    .map(|(along_x, d)| {
                                        let c2 = if along_x { Rect { x: c.x + d, ..c } } else { Rect { y: c.y + d, ..c } };
                                        let (lo2, hi2) = via_pads(cfg, layers, cuts[i], i, c2).into();
                                        let joined = |a: Option<Shape>, b: Option<Shape>| a.zip(b).map(|(a, b)| Shape { layer: a.layer, rect: bbox(a.rect, b.rect) });
                                        std::iter::once(Shape { layer: cut, rect: c2 }).chain(joined(lo, lo2)).chain(joined(hi, hi2)).collect()
                                    })
                                    .collect()
                            })
                            .collect()
                    })
                    .collect();
                let taken = add_where_clear(&mut routes.wires, &sites, &image, pitch, &clear);
                stats.single_cut_vias = taken.iter().enumerate().map(|(n, t)| t.iter().filter(|k| k.is_none()).count() as u32 * (1 + u32::from(image[n].is_some()))).sum();

                // GAP-12 (EM-27): every flush L of a critical net gets its
                // inner-corner support square where it clears.
                let sites: Vec<Vec<Vec<Vec<Shape>>>> = routes
                    .wires
                    .iter()
                    .enumerate()
                    .map(|(n, w)| {
                        let squares = if crit[n] { support_squares(w, &metals) } else { Vec::new() };
                        squares.into_iter().filter(|q| !w.iter().any(|s| s.layer == q.layer && contains(s.rect, q.rect))).map(|q| vec![vec![q]]).collect()
                    })
                    .collect();
                let taken = add_where_clear(&mut routes.wires, &sites, &image, pitch, &clear);
                stats.corners_unsupported = taken.iter().enumerate().map(|(n, t)| t.iter().filter(|k| k.is_none()).count() as u32 * (1 + u32::from(image[n].is_some()))).sum();

                stats.us_geometry += us(t_geometry);
                let t_fill = std::time::Instant::now();
                // Same-net sliver and notch filling (never within spacing of foreign metal).
                let flat: Vec<(usize, Shape)> =
                    routes.wires.iter().enumerate().flat_map(|(i, w)| w.iter().map(move |s| (i, *s))).collect();
                // An exact image (RTE-15) fills after its leader and first takes
                // the image of each leader filler that clears its foreign metal by
                // the fill's spacing; one that does not is dropped from both sides
                // (DRC beats symmetry). Cell metal is not mirrored: the image's own
                // fill runs after, as for any net (RTE-23).
                //
                // ponytail: a dropped pair can leave a sliver on one side; signoff
                // DRC reports it. Re-fill with the pair frozen if it shows up.
                let mut lead: Vec<Option<(usize, gr::LatticeMap)>> = vec![None; n_nets];
                for (a, m) in cold.mirror.iter().enumerate() {
                    if let Some((b, map, true)) = *m {
                        lead[compact[b as usize]] = Some((compact[a], map));
                    }
                }
                let mut order: Vec<usize> = (0..n_nets).collect();
                order.sort_by_key(|&ni| lead[ni].is_some());
                let mut fillers: Vec<Vec<Shape>> = vec![Vec::new(); n_nets];
                for ni in order {
                    let mine = cell_of(ni);
                    let foreign: Vec<Shape> = flat.iter().filter(|&&(i, _)| i != ni).map(|&(_, s)| s).chain(statics[ni].iter().copied()).collect();
                    if let Some((a, map)) = lead[ni] {
                        for f in std::mem::take(&mut fillers[a]) {
                            let img = Shape { layer: f.layer, rect: map_rect(f.rect, map, cold.graph.pitch) };
                            let space = fill_space(f.layer);
                            if foreign.iter().all(|g| g.layer != f.layer || rect_gap(g.rect, img.rect) >= space) {
                                routes.wires[ni].push(img);
                            } else {
                                routes.wires[a].retain(|s| *s != f);
                                stats.pair_fillers_dropped += 1;
                            }
                        }
                    }
                    let wires = &mut routes.wires[ni];
                    let before = wires.clone();
                    let own = wires.len();
                    wires.extend(mine);
                    let pins = wires.len() - own;
                    // Per layer: its own spacing and min width.
                    for &l in &fill_layers {
                        let space = fill_space(l);
                        let feat = cfg.min_width.iter().find(|&&(m, _)| m == l).map_or(cfg.wire_width, |&(_, w)| w);
                        heal_same_net_slivers(wires, &[l], space, feat, &foreign, cfg.grid);
                        fill_same_net_notches(wires, &[l], space, feat, &foreign, cfg.grid);
                    }
                    wires.drain(own..own + pins);
                    fillers[ni] = wires.iter().filter(|s| !before.contains(s)).copied().collect();
                    drop_contained(wires);
                }
                for s in routes.wires.iter_mut().flatten() {
                    s.rect.x += origin.0;
                    s.rect.y += origin.1;
                }

                stats.us_fill += us(t_fill);
                (routes.cell, routes.gates) = (cell_abs.clone(), gates_abs.clone());
                // The terminals and their currents, for the EM rule. Final routes only:
                // the repair probes carry none, so EM reads unknown there; repair
                // spends no trial on `RepairKind::Em` either (width is not a search
                // resource).
                routes.terms = vec![Vec::new(); n_nets];
                for &(net, r, _) in &all_pins {
                    routes.terms[net.0 as usize].push(pnr_core::Terminal { at: r, ua: pin_ua(net, r) });
                }
                // EM verify/repair (C18): the drawn routes against the hard rule.
                // Each failing shape bumps by one track, per layer, the narrowest
                // of the branches it may belong to (a branch is keyed by the
                // terminal it ends at, `RouteCtx::branch_k`; a metal shape holds
                // the branch nodes on its layer inside it, a cut those on its two
                // metals within a pitch, where its array spreads); the bumped nets
                // reroute and negotiation clears what they crowd. A net whose
                // reroute fails keeps its tree; what still fails is the rule's V.
                let Some(stack) = cfg.stack.filter(|_| round < EM_ROUNDS) else { break (routes, sacrificed, access_v) };
                let mut limits = [(u16::MAX, analog::routing::em::Limit::default()); analog::routing::em::MAX_LAYERS];
                for (slot, &(l, lim)) in limits.iter_mut().zip(&cfg.em) {
                    *slot = (l.0, lim);
                }
                let mut bumped = Vec::new();
                for ci in 0..n_compact {
                    if cold.term_k[ci].is_empty() {
                        continue;
                    }
                    let net = NetId(compact[ci] as u16);
                    let rule = analog::routing::Electromigration { net, limits, stack: Some(stack), front_row: cfg.em_front_row };
                    let Some(bad) = rule.failing(&routes) else { continue };
                    let mut hit: Vec<(usize, usize)> = Vec::new();
                    for i in bad {
                        let s = routes.shapes(net)[i];
                        let (ls, grow) = match (layers.iter().position(|&l| l == s.layer), cuts.iter().position(|c| c.0 == s.layer)) {
                            (Some(li), _) => ([li, li], 0),
                            (None, Some(k)) => ([k, k + 1], cfg.pitch),
                            _ => continue,
                        };
                        let r = s.rect;
                        let inside = |n: &u32| {
                            let (x, y, l) = cold.graph.pos(*n);
                            let (x, y) = (x + origin.0, y + origin.1);
                            ls.contains(&(l as usize)) && (r.x - grow..=r.x + r.w + grow).contains(&x) && (r.y - grow..=r.y + r.h + grow).contains(&y)
                        };
                        // The root's own branch draws nothing: the next one starts
                        // there. `None`: a branch ending off every terminal (a tree
                        // edit), drawn at the net's `k`.
                        let ts: Vec<Option<usize>> = hot.trees[ci]
                            .iter()
                            .filter(|b| b.iter().any(inside))
                            .map(|b| b.last().and_then(|n| cold.terms[ci].iter().position(|x| x == n)))
                            .filter(|&t| t != Some(0))
                            .collect();
                        // A metal shape is a branch's only at that branch's drawn
                        // width (a run, its corner block, or one wire of a
                        // bundle): an access jog or pad is no track's to widen.
                        // Of those, the narrowest on the layer drew it (a wider
                        // branch through the same spot draws its own wider metal).
                        for &l in ls.iter().filter(|&&l| l < n_l) {
                            let drawn = |t: &&usize| {
                                let k = i32::from(cold.term_k[ci][**t][l]);
                                grow > 0 || [cfg.wire(l), cfg.wire(l) + (k - 1) * cfg.stride(l) * cfg.pitch].contains(&r.w.min(r.h))
                            };
                            // An off-terminal branch bumps every terminal at the
                            // net's `k` on this layer: those set its width.
                            let top = cold.k[ci][l];
                            let cands: Vec<usize> = ts
                                .iter()
                                .flat_map(|&t| t.map_or_else(|| (1..cold.term_k[ci].len()).filter(|&t| cold.term_k[ci][t][l] == top).collect(), |t| vec![t]))
                                .collect();
                            let own: Vec<usize> = cands.iter().filter(drawn).copied().collect();
                            let min = own.iter().map(|&t| cold.term_k[ci][t][l]).min();
                            hit.extend(own.iter().filter(|&&t| Some(cold.term_k[ci][t][l]) == min).map(|&t| (t, l)));
                        }
                    }
                    hit.sort_unstable();
                    hit.dedup();
                    let tk = &mut cold.term_k[ci];
                    let mut any = false;
                    for (t, l) in hit {
                        if tk[t][l] < K_MAX {
                            tk[t][l] += 1;
                            any = true;
                        }
                    }
                    if any {
                        tk[0] = max_k(&tk[1..]);
                        cold.k[ci] = tk[0];
                        bumped.push(ci);
                    }
                }
                // An exact pair keeps its tracks tied; one now too wide falls back.
                for a in 0..n_compact {
                    let Some((b, map, true)) = cold.mirror.get(a).copied().flatten() else { continue };
                    let b = b as usize;
                    if bumped.contains(&a) || bumped.contains(&b) {
                        if !tie_pair(&mut cold, a, b, map) {
                            (cold.mirror[a], cold.mirror[b]) = (None, None);
                            stats.pairs_exact.retain(|&p| p != (compact[a] as u32, compact[b] as u32));
                            stats.pairs_fallback.push((compact[a] as u32, compact[b] as u32, "wide pair"));
                        }
                        bumped.extend([a, b]);
                    }
                }
                if bumped.is_empty() {
                    break (routes, sacrificed, access_v);
                }
                for &ci in &bumped {
                    if let Some(t) = cold.reroute(&hot, ci, P_FAC, &[], &mut dij) {
                        cold.commit(&mut hot, ci, t);
                    }
                }
                run_pathfinder(&mut hot, &cold, P_FAC, HIST_INC, MAX_ITERS, &mut dij);
                round += 1;
            };
            // RTE-20: equalise the bits' lead C per unit with dead-end stubs
            // before scoring, so every row (coupling, differential, antenna) sees
            // them; then report what spread remains and any bit crossing the top
            // plate outside the array.
            for rule in &plates {
                equalize_leads(&mut routes, rule, &metals, &foreign_metal, &|l| cfg.space(l, 0, 0, cfg.pitch - cfg.wire_width), cfg.grid);
                stats.plate_spread_pct.push(rule.spread_pct(&routes));
            }
            let key = drawn_key(&routes, &hot, &sacrificed, &access_v);
            if let Some((before, trees, k, term_k, mirror, st, drawing)) = prev.take() {
                if !improves(key, before) {
                    for (ci, t) in trees.into_iter().enumerate() {
                        cold.commit(&mut hot, ci, t);
                    }
                    (cold.k, cold.term_k, cold.mirror) = (k, term_k, mirror);
                    (stats.pairs_exact, stats.pairs_fallback, stats.plate_spread_pct, stats.pair_fillers_dropped) = (st.pairs_exact, st.pairs_fallback, st.plate_spread_pct, st.pair_fillers_dropped);
                    break drawing;
                }
            }
            let newly: Vec<usize> = (0..reqs.hard.len()).filter(|&i| pre[i] == 0 && reqs.hard[i].violations(&routes) > 0).collect();
            if newly.is_empty() || post == POST_ROUNDS {
                break (routes, sacrificed, access_v);
            }
            let mut ids = Vec::new();
            for &i in &newly {
                reqs.hard[i].violating_ids(&routes, &mut ids);
            }
            ids.sort_unstable();
            ids.dedup();
            prev = Some((key, hot.trees.clone(), cold.k.clone(), cold.term_k.clone(), cold.mirror.clone(), stats.clone(), (routes, sacrificed, access_v)));
            stats.post_rounds += 1;
            post += 1;
            // `reroute` returns only its own net's tree: an exact pair goes
            // both sides, as in the EM round, or it ships unmirrored.
            let mut cis: Vec<usize> = ids.iter().filter_map(|&n| ci_of.get(n as usize).copied().filter(|&c| c != usize::MAX)).collect();
            let partners: Vec<usize> = cis.iter().filter_map(|&ci| cold.mirror.get(ci).copied().flatten().filter(|m| m.2).map(|m| m.0 as usize)).collect();
            cis.extend(partners);
            cis.sort_unstable();
            cis.dedup();
            for ci in cis {
                if let Some(t) = cold.reroute(&hot, ci, 2.0 * P_FAC, &[], &mut dij) {
                    cold.commit(&mut hot, ci, t);
                }
            }
            run_pathfinder(&mut hot, &cold, P_FAC, HIST_INC, MAX_ITERS, &mut dij);
        };
        let overuse = overuse(&hot);
        let mut report = score(&routes, reqs, overuse, &joins, &foreign_metal, &sacrificed, side);
        report.hard_violations.extend(star_v);
        for (rule, spread) in plates.iter().zip(&stats.plate_spread_pct) {
            if let Some(spread) = *spread {
                let tol = rule.tol_pct10 as f32 / 10.0;
                if spread > tol {
                    report.budget_violations.push(Violation::from_residual(format!("plate ratio net {}", rule.set.top.0), f64::from((spread - tol) / tol)));
                }
            }
            let a = rule.set.array;
            let top = routes.shapes(rule.set.top);
            for &(bit, _) in &rule.set.bits {
                // Adjacent metals overlapping where the overlap is not wholly
                // inside the array (the plates' own stack): a crossing.
                let crossed = routes.shapes(bit).iter().any(|p| {
                    top.iter().any(|q| {
                        let (lp, lq) = (metals.iter().position(|&l| l == p.layer), metals.iter().position(|&l| l == q.layer));
                        let (x0, y0) = (p.rect.x.max(q.rect.x), p.rect.y.max(q.rect.y));
                        let (x1, y1) = ((p.rect.x + p.rect.w).min(q.rect.x + q.rect.w), (p.rect.y + p.rect.h).min(q.rect.y + q.rect.h));
                        let inside = x0 >= a.x && y0 >= a.y && x1 <= a.x + a.w && y1 <= a.y + a.h;
                        lp.zip(lq).is_some_and(|(x, y)| x.abs_diff(y) == 1) && overlaps(p.rect, q.rect) && !inside
                    })
                });
                if crossed {
                    report.hard_violations.push(Violation { rule: format!("plate crossing net {}", bit.0), margin: 1 });
                }
            }
        }
        report.budget_violations.extend(shield_v);
        report.hard_violations.extend(access_v);
        let mut metals = [u16::MAX; analog::routing::metal_over_gate::MAX_METALS];
        for (m, l) in metals.iter_mut().zip(&layers[..n_layers as usize]) {
            *m = l.0;
        }
        for b in cfg.blockages.iter().filter(|b| b.gate) {
            let um2 = analog::routing::MetalOverGate { rect: b.rect, cell: b.cell, metals }.overlap_um2(&routes);
            if um2 > 0.0 {
                // 10⁻³ µm²: a sliver is not 0.
                report.hard_violations.push(Violation { rule: format!("metal over gate cell {}", b.cell), margin: (um2 * 1_000.0).ceil() as i64 });
            }
        }
        (stats.overuse, stats.expanded, stats.coarsened) = (overuse, dij.pops, cold.graph.coarsened);
        // Lower node of each layer-changing step, deduplicated as `extract_geometry` does.
        let via_at: HashSet<u32> = hot.trees.iter().flatten().flat_map(|b| b.windows(2)).filter(|w| cold.graph.ixy(w[0]).2 != cold.graph.ixy(w[1]).2).map(|w| w[0].min(w[1])).collect();
        stats.vias = via_at.len() as u32;
        (routes, report, stats)
    }
}

/// The track lattice `route` builds from a config, for the placer to snap
/// to (PLC-28): base pitch, per-layer stride, and the multiple every frame
/// origin sits on ([`lattice_period`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LatticeSpec {
    pub p0: i32,
    /// Per routed layer (empty = uniform, stride 1).
    pub strides: Vec<u32>,
    pub origin_multiple: i32,
}

/// The lattice of `cfg` ([`LatticeSpec`]).
#[must_use]
pub fn lattice_spec(cfg: &DetailedCfg) -> LatticeSpec {
    LatticeSpec { p0: cfg.pitch, strides: cfg.layers.iter().map(|s| s.stride).collect(), origin_multiple: lattice_period(cfg) }
}

/// Routing demand per region over a 16×16 split of the frame (the last row
/// and column absorb the remainder): `Σ (usage + hist/P_FAC)` over every
/// on-track node of every layer, over their count. Rects are absolute (`+
/// origin`). After negotiation `usage ≤ 1` almost everywhere; the history
/// term keeps contention that was negotiated away, so `> 1` reads as
/// pressure, not literal overflow (PLC-15). `P_FAC` is fixed, not the final
/// present factor, so epochs compare.
fn congestion(hot: &RouteHot, grid: &TrackGrid, origin: (i32, i32)) -> Vec<(Rect, f32)> {
    const SPLIT: u32 = 16;
    let (rw, rh) = ((grid.nx / SPLIT).max(1), (grid.ny / SPLIT).max(1));
    let (nrx, nry) = ((grid.nx / rw).min(SPLIT), (grid.ny / rh).min(SPLIT));
    let mut sum = vec![(0.0f32, 0u32); (nrx * nry) as usize];
    for n in 0..grid.nodes() as u32 {
        if !grid.on_track(n) {
            continue;
        }
        let (ix, iy, _) = grid.ixy(n);
        let r = &mut sum[((iy / rh).min(nry - 1) * nrx + (ix / rw).min(nrx - 1)) as usize];
        r.0 += f32::from(hot.usage[n as usize]) + hot.hist[n as usize] / P_FAC;
        r.1 += 1;
    }
    let p = grid.pitch;
    sum.iter()
        .enumerate()
        .map(|(k, &(d, c))| {
            let (rx, ry) = (k as u32 % nrx, k as u32 / nrx);
            let x1 = if rx + 1 == nrx { grid.nx } else { (rx + 1) * rw };
            let y1 = if ry + 1 == nry { grid.ny } else { (ry + 1) * rh };
            let (x0, y0) = (rx * rw, ry * rh);
            let rect = Rect { x: origin.0 + (x0 as i32) * p, y: origin.1 + (y0 as i32) * p, w: (x1 - x0) as i32 * p, h: (y1 - y0) as i32 * p };
            (rect, if c > 0 { d / c as f32 } else { 0.0 })
        })
        .collect()
}

/// The lattice's repeat, nm: `p0 · lcm(strides)` (`p0` with no specs).
fn lattice_period(cfg: &DetailedCfg) -> i32 {
    let gcd = |mut a: u32, mut b: u32| {
        while b != 0 {
            (a, b) = (b, a % b);
        }
        a
    };
    let lcm = cfg.layers.iter().map(|s| s.stride.max(1)).fold(1u32, |l, s| l / gcd(l, s) * s);
    cfg.pitch.max(1) * lcm as i32
}

/// The frame origin: `low − pad` rounded down, per axis, to a multiple of `period`.
fn frame_origin(low: (i32, i32), pad: i32, period: i32) -> (i32, i32) {
    let f = |v: i32| (v - pad).div_euclid(period) * period;
    (f(low.0), f(low.1))
}

/// Tracks and guard tracks on lattice layer `l` (`layer`) for a connection
/// carrying `ua` µA: `k = 1 + ⌈max(0, I·1000/J − wire)/(stride·p0)⌉`, at or
/// past the layer's first wide threshold at least `⌈EM width/wire⌉` (drawn as
/// `k` parallel wires), capped at [`K_MAX`]; a run that reaches the threshold
/// keeps guard tracks each side at its wide spacing. `(1, 0)` with no limit
/// or no current.
fn tracks(cfg: &DetailedCfg, l: usize, layer: LayerId, ua: f32) -> (u8, u8) {
    let (wire, pitch) = (cfg.wire(l), cfg.stride(l) * cfg.pitch);
    let j = cfg.em_limit(layer).map_or(0.0, |lim| lim.ua_per_um);
    let ua = ua.abs();
    if j <= 0.0 || ua <= 0.0 {
        return (1, 0);
    }
    let extra = (ua * 1_000.0 / j - wire as f32).max(0.0);
    let mut k = (1 + (extra / pitch as f32).ceil() as i32).min(i32::from(K_MAX));
    let mut w = wire + (k - 1) * pitch;
    if k > 1 && w >= cfg.wide(layer) {
        // Drawn as `k` separate wires: the copper is `k·wire`, not `w`.
        let need = cfg.em_width(layer, ua, 0.0);
        k = k.max((need + wire - 1) / wire).min(i32::from(K_MAX));
        w = wire + (k - 1) * pitch;
    }
    let guard = if k > 1 && w >= cfg.wide(layer) {
        let need = cfg.space(layer, w, 0, 0) - (pitch - wire);
        ((need + pitch - 1) / pitch).max(0) as u8
    } else {
        0
    };
    (k as u8, guard)
}

/// Tie exact pair `a` → `b` under `map` in `cold` (RTE-15): `b`'s terminals
/// become `a`'s images in `a`'s order, both nets' terminal tracks, tracks and
/// guards the elementwise max, and [`RouteCtx::mirror`] links them. `false`
/// (and no link) when an image is missing or the tied pair is wider than one
/// track, or has guard tracks, on a vertical layer.
fn tie_pair(cold: &mut RouteCtx<TrackGrid>, a: usize, b: usize, map: gr::LatticeMap) -> bool {
    let Some(terms_b) = cold.terms[a].iter().map(|&n| cold.graph.map(map, n)).collect::<Option<Vec<u32>>>() else { return false };
    if !terms_b.iter().all(|n| cold.terms[b].contains(n)) || terms_b.len() != cold.terms[b].len() {
        return false;
    }
    let one = [1u8; gr::MAX_LAYERS];
    let get = |v: &[Vec<[u8; gr::MAX_LAYERS]>], net: usize, i: usize| v.get(net).and_then(|t| t.get(i)).copied().unwrap_or(one);
    let tk: Vec<[u8; gr::MAX_LAYERS]> = terms_b
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let j = cold.terms[b].iter().position(|x| x == n).unwrap_or(0);
            let (x, y) = (get(&cold.term_k, a, i), get(&cold.term_k, b, j));
            std::array::from_fn(|l| x[l].max(y[l]))
        })
        .collect();
    let at = |v: &[[u8; gr::MAX_LAYERS]], net: usize, d: u8| v.get(net).copied().unwrap_or([d; gr::MAX_LAYERS]);
    let k: [u8; gr::MAX_LAYERS] = std::array::from_fn(|l| at(&cold.k, a, 1)[l].max(at(&cold.k, b, 1)[l]));
    let guard: [u8; gr::MAX_LAYERS] = std::array::from_fn(|l| at(&cold.guard, a, 0)[l].max(at(&cold.guard, b, 0)[l]));
    let vertical_wide = |t: &[u8; gr::MAX_LAYERS]| (0..cold.graph.n_layers as usize).filter(|l| l % 2 == 1).any(|l| t[l] > 1 || guard[l] > 0);
    if matches!(map, gr::LatticeMap::MirrorX { .. }) && (vertical_wide(&k) || tk.iter().any(vertical_wide)) {
        return false;
    }
    cold.terms[b] = terms_b;
    if !cold.term_k.is_empty() {
        (cold.term_k[a], cold.term_k[b]) = (tk.clone(), tk);
    }
    if !cold.k.is_empty() {
        (cold.k[a], cold.k[b]) = (k, k);
    }
    if !cold.guard.is_empty() {
        (cold.guard[a], cold.guard[b]) = (guard, guard);
    }
    if cold.mirror.is_empty() {
        cold.mirror = vec![None; cold.terms.len()];
    }
    cold.mirror[a] = Some((b as u32, map, true));
    cold.mirror[b] = Some((a as u32, map.inverse(), false));
    true
}

/// Plate-lead spread tolerance, tenths of a percent of one unit's C
/// (tuning default: Hastings §8.2 gives the check, not a number).
const PLATE_TOL_PCT10: i32 = 10;

/// Hastings §8.2 rule C10 (restored from M1's `trim_pair`): per bit whose
/// lead C per unit is under the largest, a same-layer dead-end stub
/// continuing one of its runs outside the array, `L = (target − C_i/n_i)·n_i
/// / c_per_nm` (the layer's area and fringe C at the run's width), snapped
/// to `grid`; kept only when the spread drops and the stub keeps `space`
/// from every foreign shape on its layer (other nets' routes and the cell
/// and ring metal of `foreign` not owned by the bit). Bits are visited most-deficient first, each once.
#[allow(clippy::too_many_arguments)]
fn equalize_leads(routes: &mut Routes, rule: &analog::routing::PlateRatio, metals: &[LayerId], foreign: &[(Option<u32>, Shape)], space: &dyn Fn(LayerId) -> i32, grid: i32) {
    let Some(_) = rule.spread_pct(routes) else { return };
    let mut order: Vec<usize> = (0..rule.set.bits.len()).collect();
    let per = rule.per_unit_af(routes);
    order.sort_by(|&a, &b| per[a].total_cmp(&per[b]));
    let a = rule.set.array;
    let g = grid.max(1);
    for i in order {
        let per = rule.per_unit_af(routes);
        let target = per.iter().copied().fold(0.0, f32::max);
        let (net, n) = rule.set.bits[i];
        let need = (target - per[i]) * n as f32;
        let Some(before) = rule.spread_pct(routes) else { return };
        if need <= 0.0 {
            continue;
        }
        let b = net.0 as usize;
        let runs: Vec<Shape> = routes.wires[b].iter().copied().filter(|s| s.rect.w != s.rect.h && metals.contains(&s.layer) && !overlaps(s.rect, a)).collect();
        'run: for s in runs {
            let Some(l) = rule.stack.layers.iter().find(|l| l.id == s.layer.0) else { continue };
            let width = s.rect.w.min(s.rect.h);
            let per_nm = (l.area_af_um2 * width as f32 / 1_000.0 + 2.0 * l.fringe_af_um) / 1_000.0;
            if per_nm <= 0.0 {
                continue;
            }
            let len = ((need / per_nm / g as f32).round() as i32) * g;
            if len <= 0 {
                continue;
            }
            let r = s.rect;
            let stubs = if r.w > r.h { [Rect { x: r.x + r.w, w: len, ..r }, Rect { x: r.x - len, w: len, ..r }] } else { [Rect { y: r.y + r.h, h: len, ..r }, Rect { y: r.y - len, h: len, ..r }] };
            for stub in stubs {
                let gap = space(s.layer);
                let clear = !overlaps(stub, a)
                    && routes.wires.iter().enumerate().filter(|&(k, _)| k != b).flat_map(|(_, w)| w).chain(foreign.iter().filter(|f| f.0 != Some(b as u32)).map(|f| &f.1)).all(|f| f.layer != s.layer || rect_gap(f.rect, stub) >= gap);
                if !clear {
                    continue;
                }
                routes.wires[b].push(Shape { layer: s.layer, rect: stub });
                if rule.spread_pct(routes).is_some_and(|after| after < before) {
                    break 'run;
                }
                routes.wires[b].pop();
            }
        }
    }
}

/// Elementwise max of `ks` (all ones when empty).
fn max_k(ks: &[[u8; gr::MAX_LAYERS]]) -> [u8; gr::MAX_LAYERS] {
    ks.iter().fold([1; gr::MAX_LAYERS], |m, k| std::array::from_fn(|l| m[l].max(k[l])))
}

/// Prim MST over the terminals at `pos` (Manhattan), rooted at the largest
/// `|ua|`: `(visit order, root first; per terminal, the DC current on the
/// edge into it)`, the root's 0. Cutting an edge splits the terminals in two
/// and it carries one side's sum `S` (Lienig & Thiele 2018 eqs. 3.5–3.7; KCL);
/// when the currents do not sum to zero the net has a port whose attachment
/// is unknown, so the edge carries `max(|S|, |total − S|)` (as `net_flow`).
/// Ties break by index. Also each terminal's MST parent (the root's itself).
fn prim(pos: &[(i32, i32)], ua: &[f32]) -> (Vec<usize>, Vec<f32>, Vec<usize>) {
    let n = pos.len();
    let Some(root) = (0..n).rev().max_by(|&a, &b| ua[a].abs().total_cmp(&ua[b].abs())) else { return (Vec::new(), Vec::new(), Vec::new()) };
    let d = |a: usize, b: usize| i64::from((pos[a].0 - pos[b].0).abs()) + i64::from((pos[a].1 - pos[b].1).abs());
    let (mut best, mut parent, mut done) = (vec![i64::MAX; n], vec![root; n], vec![false; n]);
    best[root] = 0;
    let mut visit = Vec::with_capacity(n);
    while let Some(u) = (0..n).filter(|&i| !done[i]).min_by_key(|&i| (best[i], i)) {
        done[u] = true;
        visit.push(u);
        for v in (0..n).filter(|&v| !done[v]) {
            if d(u, v) < best[v] {
                (best[v], parent[v]) = (d(u, v), u);
            }
        }
    }
    let total: f32 = ua.iter().sum();
    let (mut sub, mut edge) = (ua.to_vec(), vec![0.0; n]);
    for &v in visit[1..].iter().rev() {
        edge[v] = sub[v].abs().max((total - sub[v]).abs());
        sub[parent[v]] += sub[v];
    }
    (visit, edge, parent)
}

/// Width an access jog on `metal` carrying `ua` needs, nm: `⌈|I|·1000/J⌉`
/// snapped up to `2·grid`; `0` when the current or the limit is unknown.
fn access_need(cfg: &DetailedCfg, metal: LayerId, ua: Option<f32>) -> i32 {
    let j = cfg.em_limit(metal).map_or(0.0, |lim| lim.ua_per_um);
    let Some(u) = ua.filter(|_| j > 0.0) else { return 0 };
    let step = 2 * cfg.grid.max(1);
    ((u.abs() * 1_000.0 / j).ceil() as i32 + step - 1) / step * step
}

/// `cell` (drawn at its own origin) moved into free space as close as it fits
/// to `near`: its bbox keeps `clearance` from every `obstacle`, on the `grid`
/// lattice, searched outward ring by ring (at most `reach` nm away). `None`
/// when nothing within reach is free. For a device dr inserts (an antenna
/// diode): the caller routes it as a fixed cell.
#[must_use]
pub fn place_near(cell: &Macro, near: (i32, i32), obstacles: &[Rect], clearance: i32, grid: i32, reach: i32) -> Option<Macro> {
    let b = cell.bbox;
    let step = grid.max(1);
    let snap = |v: i32| v.div_euclid(step) * step;
    let free = |r: Rect| obstacles.iter().all(|&o| rect_gap(r, o) >= clearance.max(1));
    let (cx, cy) = (snap(near.0 - b.w / 2), snap(near.1 - b.h / 2));
    for ring in 0..=reach / step {
        for dy in -ring..=ring {
            for dx in -ring..=ring {
                if dx.abs().max(dy.abs()) != ring {
                    continue;
                }
                let (x, y) = (cx + dx * step, cy + dy * step);
                if free(Rect { x, y, w: b.w, h: b.h }) {
                    let (ox, oy) = (x - b.x, y - b.y);
                    let mv = |r: Rect| Rect { x: r.x + ox, y: r.y + oy, ..r };
                    return Some(Macro {
                        shapes: cell.shapes.iter().map(|s| Shape { rect: mv(s.rect), ..*s }).collect(),
                        pins: cell.pins.iter().map(|p| pnr_core::Pin { at: mv(p.at), ..p.clone() }).collect(),
                        bbox: mv(b),
                        units: cell.units.clone(),
                        dummies: cell.dummies.clone(),
                        drawn: cell.drawn.clone(),
                        keepouts: cell.keepouts.iter().map(|k| pnr_core::Keepout { rect: mv(k.rect), ..*k }).collect(),
                        figures: cell.figures.clone(),
                    });
                }
            }
        }
    }
    None
}

/// Drop any rect another same-layer rect of the net already covers (equal rects:
/// keep the first). Concentric via pads otherwise read as a sliver-gap figure.
fn drop_contained(wires: &mut Vec<Shape>) {
    let covered: Vec<bool> = wires
        .iter()
        .enumerate()
        .map(|(i, s)| {
            wires.iter().enumerate().any(|(j, t)| {
                t.layer == s.layer && contains(t.rect, s.rect) && (j < i || !contains(s.rect, t.rect))
            })
        })
        .collect();
    let mut keep = covered.iter().map(|c| !c);
    wires.retain(|_| keep.next().unwrap_or(true));
}

/// Edge-to-edge gap, `0` when touching or overlapping. Diagonal separation reads
/// as the larger axis gap (an under-estimate: errs toward rejecting).
fn rect_gap(a: Rect, b: Rect) -> i32 {
    let dx = (b.x - (a.x + a.w)).max(a.x - (b.x + b.w));
    let dy = (b.y - (a.y + a.h)).max(a.y - (b.y + b.h));
    dx.max(dy).max(0)
}

/// `outer` covers every point of `inner`.
fn contains(outer: Rect, inner: Rect) -> bool {
    outer.x <= inner.x
        && outer.y <= inner.y
        && outer.x + outer.w >= inner.x + inner.w
        && outer.y + outer.h >= inner.y + inner.h
}

/// The two legs of an access jog of width `w` from node `(nx, ny)` to pin centre
/// `(px, py)`: `[0]` runs horizontal at the node row, `[1]` vertical at the node
/// column. Legs overrun the corner by `w/2` so their union is a flush L.
fn jog_legs(nx: i32, ny: i32, px: i32, py: i32, w: i32) -> [[Rect; 2]; 2] {
    let h = w / 2;
    let horiz = |y: i32| Rect { x: nx.min(px) - h, y: y - h, w: (nx - px).abs() + w, h: w };
    let vert = |x: i32| Rect { x: x - h, y: ny.min(py) - h, w, h: (ny - py).abs() + w };
    [[horiz(ny), vert(px)], [vert(nx), horiz(py)]]
}

/// Lattice layer an access jog off `pin_layer` runs on.
fn jog_layer(cfg: &DetailedCfg, layers: &[LayerId], cuts: &[Cut], n_layers: u32, pin_layer: LayerId) -> u32 {
    let base = layers.iter().position(|&l| l == pin_layer).unwrap_or(0) as u32;
    let l = match cfg.pin_access {
        Some((pl, _)) if pl == pin_layer => 0,
        _ if base + 1 < n_layers && (base as usize) < cuts.len() => base + 1,
        _ => base,
    };
    l.min(n_layers.saturating_sub(1))
}

/// Jog legs clear of every foreign stitch zone (`zone` wide) and every foreign
/// leg already laid.
/// No leg touches another net's pin zone (a `zone`-sided square at the pin,
/// kept `gap` away: a pin landed later still draws its pad there) or a jog
/// already laid, or overlaps a hard blockage in `blocks` (those binding the
/// net on the jog's layer).
#[allow(clippy::too_many_arguments)]
fn jog_clean(legs: &[Rect; 2], ci: usize, zone: i32, gap: i32, zones: &[(u32, i32, i32)], laid: &[(usize, Rect)], blocks: &[Rect]) -> bool {
    zones.iter().all(|&(zci, zx, zy)| {
        let zr = Rect { x: zx - zone / 2, y: zy - zone / 2, w: zone, h: zone };
        zci as usize == ci || legs.iter().all(|&l| rect_gap(l, zr) >= gap.max(1))
    }) && laid.iter().all(|&(lci, lr)| lci == ci || legs.iter().all(|&l| rect_gap(l, lr) > 0))
        && blocks.iter().all(|&b| legs.iter().all(|&l| !overlaps(l, b)))
}

/// `r` grown by `g` on every side.
fn grown(r: Rect, g: i32) -> Rect {
    Rect { x: r.x - g, y: r.y - g, w: r.w + 2 * g, h: r.h + 2 * g }
}

/// `a` and `b` share area (touching edges do not).
fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

/// Reserve for `a.ci` the lattice nodes its jog will occupy (legs inflated by half
/// a wire, both orientations when the jog is still undecided) and the stitch-pad
/// nodes on the pin's own layer when the jog runs on a different one. PathFinder
/// cannot price geometry drawn after it, so it must be kept out up front. First
/// come: already claimed nodes keep their owner.
fn claim_jog_sweep(
    grid: &TrackGrid,
    cfg: &DetailedCfg,
    layers: &[LayerId],
    cuts: &[Cut],
    a: &Access,
    claimed: &mut [bool],
    reserved: &mut [u32],
) {
    let (px, py) = a.node;
    let (cx, cy) = (a.pin.x + a.pin.w / 2, a.pin.y + a.pin.h / 2);
    let base_l = layers.iter().position(|&l| l == a.pin_layer).unwrap_or(0) as u32;
    let jog_l = jog_layer(cfg, layers, cuts, grid.n_layers, a.pin_layer);
    // At least the EM width `add_pin_access` draws it at (`node_ua`).
    let need = layers.get(jog_l as usize).map_or(0, |&m| access_need(cfg, m, a.node_ua));
    let width = a.choice.map_or_else(|| cfg.wire_width.min(a.pin.w).min(a.pin.h).max(1), |(w, _)| w).max(need);
    let both = jog_legs(px, py, cx, cy, width);
    let legs: Vec<Rect> = match a.choice {
        Some((_, flip)) => both[usize::from(flip)].to_vec(),
        None => both.concat(),
    };
    let mut claim = |n: u32| {
        let free = reserved[n as usize] == NONE || reserved[n as usize] == a.ci as u32;
        if !claimed[n as usize] && free {
            claimed[n as usize] = true;
            reserved[n as usize] = a.ci as u32;
        }
    };
    jog_nodes(grid, cfg, &legs, jog_l).for_each(&mut claim);
    if jog_l != base_l {
        for (ex, ey) in [(px, py), (cx, cy)] {
            claim(grid.node(grid.bin_x(ex), grid.bin_y(ey), base_l));
        }
    }
}

/// The nodes another net owns (its stitch reach, landing or jog) that a jog
/// of `ci` crosses, where that net's trunk may run: a short `break_shorts`
/// settles only by deleting this access, so they get [`JOG_HIST`]. Only a
/// node whose wire would touch a leg: the swept bins reach up to a pitch past it.
fn jog_hist_nodes(grid: &TrackGrid, cfg: &DetailedCfg, legs: &[Rect; 2], jog_l: u32, reserved: &[u32], ci: usize) -> Vec<u32> {
    let infl = cfg.wire_width / 2;
    let touches = |n: &u32| {
        let (px, py, _) = grid.pos(*n);
        legs.iter().any(|l| (l.x - infl..=l.x + l.w + infl).contains(&px) && (l.y - infl..=l.y + l.h + infl).contains(&py))
    };
    let foreign = |n: &u32| reserved[*n as usize] != ci as u32 && reserved[*n as usize] < BLOCKED;
    jog_nodes(grid, cfg, legs, jog_l).filter(foreign).filter(touches).collect()
}

/// `r` carried by `m` on `grid` (frame nm): a mirror reflects it about the
/// axis `x = (k·p + p)/2`, a shift moves it by `(dx, dy)` pitches.
fn map_rect(r: Rect, m: gr::LatticeMap, p: i32) -> Rect {
    match m {
        gr::LatticeMap::MirrorX { k } => Rect { x: k * p + p - r.x - r.w, ..r },
        gr::LatticeMap::Shift { dx, dy } => Rect { x: r.x + dx * p, y: r.y + dy * p, ..r },
    }
}

/// `w × h` resized about `r`'s centre.
fn centred(r: Rect, w: i32, h: i32) -> Rect {
    Rect { x: r.x + (r.w - w) / 2, y: r.y + (r.h - h) / 2, w, h }
}

/// The landing pads of stack cut `cut` (lattice index `i`, between
/// `layers[i]` and `layers[i + 1]`) centred on `r`: per layer its spec's
/// `pad_along × pad_across` along its direction, else the cut's
/// `below`/`above` square. `None` past the routed layers.
fn via_pads(cfg: &DetailedCfg, layers: &[LayerId], cut: Cut, i: usize, r: Rect) -> [Option<Shape>; 2] {
    let (_, _, below, above) = cut;
    [(i, below), (i + 1, above)].map(|(k, square)| {
        let (w, h) = match cfg.layers.get(k) {
            Some(s) if s.horizontal => (s.pad_along, s.pad_across),
            Some(s) => (s.pad_across, s.pad_along),
            None => (square, square),
        };
        Some(Shape { layer: *layers.get(k)?, rect: centred(r, w, h) })
    })
}

/// Smallest rect covering `a` and `b`.
fn bbox(a: Rect, b: Rect) -> Rect {
    let (x, y) = (a.x.min(b.x), a.y.min(b.y));
    Rect { x, y, w: (a.x + a.w).max(b.x + b.w) - x, h: (a.y + a.h).max(b.y + b.h) - y }
}

/// Adds to each net, per site, the first alternative (a set of shapes) whose
/// every shape clears `clear(net, shape, wires)`. For an exact pair
/// (`image[a] = Some((b, map))`, leader → image), an alternative counts only
/// if its `map` image (`pitch` = lattice pitch) also clears for `b`, checked
/// with the leader's shapes already in. Both are added or neither is, and
/// `b`'s own sites are ignored (its leader decides them). Works on live
/// `wires`: a shape added for one net is seen by the next. Returns, per net
/// and site, the alternative taken; an image net's entries are empty.
fn add_where_clear(
    wires: &mut [Vec<Shape>],
    sites: &[Vec<Vec<Vec<Shape>>>],
    image: &[Option<(usize, gr::LatticeMap)>],
    pitch: i32,
    clear: &dyn Fn(usize, Shape, &[Vec<Shape>]) -> bool,
) -> Vec<Vec<Option<usize>>> {
    let mut is_image = vec![false; wires.len()];
    for &(b, _) in image.iter().flatten() {
        is_image[b] = true;
    }
    let mut out = vec![Vec::new(); wires.len()];
    for (n, net_sites) in sites.iter().enumerate().filter(|&(n, _)| !is_image[n]) {
        let img = image.get(n).copied().flatten();
        for site in net_sites {
            let pick = site.iter().position(|alt| {
                if !alt.iter().all(|&s| clear(n, s, wires)) {
                    return false;
                }
                let Some((b, map)) = img else {
                    wires[n].extend_from_slice(alt);
                    return true;
                };
                let len = wires[n].len();
                wires[n].extend_from_slice(alt);
                let mapped: Vec<Shape> = alt.iter().map(|s| Shape { rect: map_rect(s.rect, map, pitch), ..*s }).collect();
                if mapped.iter().all(|&s| clear(b, s, wires)) {
                    wires[b].extend(mapped);
                    true
                } else {
                    wires[n].truncate(len);
                    false
                }
            });
            out[n].push(pick);
        }
    }
    out
}

/// GAP-12's array off the corner, per stack cut of one net's `wires` that
/// sits in a merged corner block `B` (a rect on both of its metals, not a
/// pad) where the upper metal's run `R` (⟂ to the lower layer's direction)
/// leaves `B` on one side only: `(c, [stub S, cut c'])`, `S` the lower-metal
/// rect of `B`'s size beside `B` on that side and inside `R`, `c'` = `c`
/// moved by `B`'s length into `S`. The via array then fills the straight
/// overlap `S` instead of the corner (Lienig Fig. 4.29). None where `B` is
/// narrower across than it is long (the stub would neck the lower run).
fn corner_stubs(wires: &[Shape], cfg: &DetailedCfg, layers: &[LayerId], cuts: &[Cut]) -> Vec<(Rect, [Shape; 2])> {
    let mut out = Vec::new();
    for c in wires {
        let Some(i) = cuts.iter().position(|&(l, ..)| l == c.layer) else { continue };
        let (Some(&lo), Some(&hi)) = (layers.get(i), layers.get(i + 1)) else { continue };
        // A cut drawn twice (stack and pin access) moves once.
        if out.iter().any(|(r, _)| *r == c.rect) {
            continue;
        }
        let pads = via_pads(cfg, layers, cuts[i], i, c.rect);
        let is_pad = |r: Rect| pads.iter().flatten().any(|p| p.rect == r);
        let on = |l: LayerId| wires.iter().filter(move |s| s.layer == l && contains(s.rect, c.rect)).map(|s| s.rect);
        let Some(b) = on(lo).find(|&b| !is_pad(b) && on(hi).any(|h| h == b)) else { continue };
        // Along R's axis: x when the lower layer runs vertical.
        let lower_h = cfg.layers.get(i).map_or(i % 2 == 0, |s| s.horizontal);
        let span = |r: Rect| if lower_h { (r.y, r.y + r.h) } else { (r.x, r.x + r.w) };
        let (b0, b1) = span(b);
        let len = b1 - b0;
        // The stub carries the lower run's current along R: narrower than that
        // run (B's length) it would fail EM where the corner did not.
        if (if lower_h { b.w } else { b.h }) < len {
            continue;
        }
        let stub = |d: i32| {
            let at = if d > 0 { b1 } else { b0 - len };
            if lower_h { Rect { y: at, ..b } } else { Rect { x: at, ..b } }
        };
        let hit = on(hi).filter(|&r| r != b && (if lower_h { r.h > r.w } else { r.w > r.h })).find_map(|r| {
            let (r0, r1) = span(r);
            let d = match (r0 < b0, r1 > b1) {
                (true, false) => -1,
                (false, true) => 1,
                _ => return None,
            };
            Some((d, stub(d))).filter(|&(_, s)| contains(r, s))
        });
        let Some((d, s)) = hit else { continue };
        let moved = if lower_h { Rect { y: c.rect.y + d * len, ..c.rect } } else { Rect { x: c.rect.x + d * len, ..c.rect } };
        out.push((c.rect, [Shape { layer: lo, rect: s }, Shape { layer: c.layer, rect: moved }]));
    }
    out
}

/// Inner-corner support squares (EM-27) of every flush L in `wires`: a
/// horizontal and a vertical rect on one metal, `a.h == b.w == w`, meeting in
/// a `w × w` square at an end of each. Each square is the `w × w` square
/// diagonally inside the L, touching both legs. A T or + (the square not at
/// an end of both) gives none. Built from the wires' own edges, so on grid
/// when they are (the spec's snap is a no-op).
///
/// ponytail: flush Ls of one width only; a merged `k > 1` wrong-way run
/// meeting a narrower leg gets none. Widen the match if EM shows such
/// corners failing.
fn support_squares(wires: &[Shape], metals: &[LayerId]) -> Vec<Shape> {
    let mut out = Vec::new();
    for a in wires.iter().filter(|s| metals.contains(&s.layer) && s.rect.w > s.rect.h) {
        let w = a.rect.h;
        for b in wires.iter().filter(|s| s.layer == a.layer && s.rect.h > s.rect.w && s.rect.w == w) {
            let (x, y) = (b.rect.x, a.rect.y);
            let meets = (x == a.rect.x || x + w == a.rect.x + a.rect.w) && (y == b.rect.y || y + w == b.rect.y + b.rect.h);
            if !meets || x < a.rect.x || x + w > a.rect.x + a.rect.w || y < b.rect.y || y + w > b.rect.y + b.rect.h {
                continue;
            }
            let dx = if x == a.rect.x { w } else { -w };
            let dy = if y == b.rect.y { w } else { -w };
            out.push(Shape { layer: a.layer, rect: Rect { x: x + dx, y: y + dy, w, h: w } });
        }
    }
    out.sort_by_key(|s| (s.layer.0, s.rect.x, s.rect.y));
    out.dedup();
    out
}

/// The lattice map carrying pin set `a` exactly (within `snap` nm) onto `b`
/// (frame nm, RTE-15): a mirror about the axis between the two sets' x
/// extents when it reflects every `a` pin onto a `b` pin, else the shift by
/// the centroid difference when that does. `Err` names why there is none:
/// `no pin map` (neither carries the pins), `axis off lattice` (the map is
/// not track to track, or the lattice was coarsened).
fn pair_map(a: &[(Rect, LayerId, Option<f32>)], b: &[(Rect, LayerId, Option<f32>)], grid: &TrackGrid, snap: i32) -> Result<gr::LatticeMap, &'static str> {
    if a.is_empty() || a.len() != b.len() {
        return Err("no pin map");
    }
    let hits = |f: &dyn Fn(Rect) -> Rect| {
        a.iter().all(|&(r, l, _)| {
            let t = f(r);
            b.iter().any(|&(q, lq, _)| lq == l && (q.x - t.x).abs() < snap.max(1) && (q.y - t.y).abs() < snap.max(1) && q.w == t.w && q.h == t.h)
        })
    };
    let ext = |v: &[(Rect, LayerId, Option<f32>)]| (v.iter().map(|t| t.0.x).min().unwrap_or(0), v.iter().map(|t| t.0.x + t.0.w).max().unwrap_or(0));
    let ((a0, a1), (b0, b1)) = (ext(a), ext(b));
    let sum = i64::from(a0) + i64::from(a1) + i64::from(b0) + i64::from(b1);
    let p = grid.pitch;
    let candidate = if sum % 2 == 0 && hits(&|r: Rect| Rect { x: (sum / 2) as i32 - r.x - r.w, ..r }) {
        let axis2 = (sum / 2) as i32;
        ((axis2 - p) % p == 0).then_some(gr::LatticeMap::MirrorX { k: (axis2 - p) / p })
    } else {
        let mean = |v: &[(Rect, LayerId, Option<f32>)], f: fn(&Rect) -> i64| v.iter().map(|t| f(&t.0)).sum::<i64>() / v.len() as i64;
        let (dx, dy) = ((mean(b, |r| i64::from(r.x)) - mean(a, |r| i64::from(r.x))) as i32, (mean(b, |r| i64::from(r.y)) - mean(a, |r| i64::from(r.y))) as i32);
        if !hits(&|r: Rect| Rect { x: r.x + dx, y: r.y + dy, ..r }) {
            return Err("no pin map");
        }
        (dx % p == 0 && dy % p == 0).then_some(gr::LatticeMap::Shift { dx: dx / p, dy: dy / p })
    };
    candidate.filter(|&m| !grid.coarsened && grid.lattice_map(m)).ok_or("axis off lattice")
}

/// The lattice nodes on layer `jog_l` that jog `legs` cover, legs inflated by
/// half a wire.
fn jog_nodes<'a>(grid: &'a TrackGrid, cfg: &DetailedCfg, legs: &'a [Rect], jog_l: u32) -> impl Iterator<Item = u32> + 'a {
    let infl = cfg.wire_width / 2;
    legs.iter().flat_map(move |r| {
        (grid.bin_y(r.y - infl)..=grid.bin_y(r.y + r.h + infl))
            .flat_map(move |gy| (grid.bin_x(r.x - infl)..=grid.bin_x(r.x + r.w + infl)).map(move |gx| grid.node(gx, gy, jog_l)))
    })
}

/// Draw each access jog: an L on the jog metal from the landed node to the pin,
/// with cut + per-layer pads where it must descend to the pin (and to the node,
/// when the route arrives one layer below), and a via up when the node sits on
/// `layers[1]`.
///
/// Orientation/width: landing's choice first, then {full, narrow} × {flip}; the
/// first candidate clear of foreign zones/jogs and `pitch − wire` from foreign
/// routed metal wins, then the first merely short-free one, then landing's choice,
/// then the first zone-clean one. Leftover contact is the short resolver's job.
#[allow(clippy::too_many_arguments)]
fn add_pin_access(
    routes: &mut Routes,
    access: &[Access],
    compact: &[usize],
    cfg: &DetailedCfg,
    layers: &[LayerId],
    cuts: &[Cut],
    joins: &[Join],
    zones: &[(u32, i32, i32)],
    origin: (i32, i32),
    jog_blocks: &[Vec<(Rect, u16)>],
) -> Vec<Violation> {
    let spacing = (cfg.pitch - cfg.wire_width).max(0);
    let mut laid: Vec<(usize, Rect)> = Vec::new();
    let mut out = Vec::new();
    for a in access {
        let net = compact[a.ci];
        // (jog metal's base, climb = (jog metal, cut, size, pads, stitch node end)).
        let (base, climb) = match cfg.pin_access {
            // Pin on the reserved conductor: jog on layers[0]; the route arrives on
            // that same layer, so only the pin end descends.
            Some((pl, (cut, size, below, above))) if pl == a.pin_layer => {
                (pl, Some((layers[0], cut, size, below, above, false)))
            }
            _ => {
                let i = layers.iter().position(|&l| l == a.pin_layer).unwrap_or(0);
                let climb = match (layers.get(i + 1), cuts.get(i)) {
                    (Some(&up), Some(&(cut, size, below, above))) => Some((up, cut, size, below, above, true)),
                    _ => None,
                };
                (layers[i], climb)
            }
        };
        let metal = climb.map_or(base, |(up, ..)| up);
        let bit = layers.iter().position(|&l| l == metal).map_or(0, |l| 1u16 << l);
        let blocks: Vec<Rect> = jog_blocks[a.ci].iter().filter(|b| b.1 & bit != 0).map(|b| b.0).collect();
        let (nx, ny) = a.node;
        let (px, py) = (a.pin.x + a.pin.w / 2, a.pin.y + a.pin.h / 2);
        // No jog narrower than its EM width: narrow survives only where it meets it.
        let need = access_need(cfg, metal, a.node_ua);
        let full = cfg.wire_width.max(1).max(need);
        let floor = cfg.min_width.iter().find(|&&(l, _)| l == metal).map_or(1, |&(_, w)| w);
        let narrow = full.min(a.pin.w).min(a.pin.h).max(floor).max(need);
        let legs_of = |(w, flip): (i32, bool)| jog_legs(nx, ny, px, py, w)[usize::from(flip)];
        let options = [(full, false), (full, true), (narrow, false), (narrow, true)];
        let choice = a.choice.filter(|c| c.0 >= need);
        let cands: Vec<(i32, bool)> = choice.into_iter().chain(options).collect();
        let clear = |legs: &[Rect; 2], gap: i32| {
            let probe = Shape { layer: metal, rect: legs[0] };
            routes.wires.iter().enumerate().all(|(n, w)| {
                n == net
                    || w.iter().all(|s| {
                        !conductor_layers_meet(s, &probe, joins) || legs.iter().all(|&l| rect_gap(l, s.rect) >= gap)
                    })
            })
        };
        // A foreign pin's zone is the pad its own access will draw there.
        let pad_zone = climb.map_or(full, |(_, _, _, _, above, _)| above.max(full));
        let pick = |gap: i32| {
            cands.iter().copied().find(|&c| {
                let legs = legs_of(c);
                jog_clean(&legs, a.ci, pad_zone, gap, zones, &laid, &blocks) && clear(&legs, gap)
            })
        };
        let (w, flip) = pick(spacing).or_else(|| pick(1)).or(choice).unwrap_or_else(|| {
            options.into_iter().find(|&c| jog_clean(&legs_of(c), a.ci, full, 1, zones, &laid, &blocks)).unwrap_or(options[3])
        });
        let legs = legs_of((w, flip));
        laid.extend(legs.iter().map(|&l| (a.ci, l)));
        let foreign: Vec<Rect> =
            routes.wires.iter().enumerate().filter(|&(n, _)| n != net).flat_map(|(_, w)| w).filter(|s| s.layer == metal).map(|s| s.rect).collect();
        let wires = &mut routes.wires[net];
        let pad = |l: LayerId, (x, y): (i32, i32), s: i32| Shape { layer: l, rect: Rect { x: x - s / 2, y: y - s / 2, w: s, h: s } };
        wires.push(Shape { layer: metal, rect: legs[0] });
        if (if flip { nx - px } else { ny - py }) != 0 {
            wires.push(Shape { layer: metal, rect: legs[1] });
        }
        // The layers the jog's node end already reaches: its own metal, and the
        // climb's lower layer — through the cut stitched at the node, or, when
        // the node sits inside the pin's pad, through the pin end's own cut.
        let mut reached = vec![metal];
        if let Some((_, cut, size, below, above, stitch_node)) = climb {
            // Node inside the pin's own pad: the pads merge, so a second cut would
            // only be a cut-spacing violation.
            let merged = (nx - px).abs().max((ny - py).abs()) < below.min(above);
            let node_end = (stitch_node && !merged).then_some((nx, ny));
            reached.push(base);
            // The pin end's cuts carry the pin's current: `n = ⌈I/I_cut⌉` in
            // one row along the pin's long axis, centred on it, each inside
            // the pin by the below enclosure (the pin hosts them), under one
            // pad above. What does not fit is V: the cell must widen the
            // strap (EM-19).
            let n = a.ua.zip(cfg.em_limit(cut)).map_or(1, |(u, lim)| lim.cuts(u));
            let array = (n > 1 && a.pin_layer == base).then(|| {
                let eb = cfg.cut_enclosure.iter().find(|&&(c, ..)| c == cut).map_or(0, |&(_, e, _)| e);
                let space = match cfg.array_spacing.iter().find(|(l, ..)| *l == cut) {
                    Some(&(_, count, s)) if n as i32 >= count => s,
                    _ => cfg.space(cut, 0, 0, size),
                };
                let pitch = size + space;
                let along_x = a.pin.w >= a.pin.h;
                let (long, short) = if along_x { (a.pin.w, a.pin.h) } else { (a.pin.h, a.pin.w) };
                let fit = if short - 2 * eb >= size { ((long - 2 * eb - size) / pitch + 1).max(0) } else { 0 };
                let m = (n as i32).min(fit).max(1);
                let g = cfg.grid.max(1);
                let start = ((if along_x { px } else { py }) - ((m - 1) * pitch + size) / 2).div_euclid(g) * g;
                let across = ((if along_x { py } else { px }) - size / 2).div_euclid(g) * g;
                let rects: Vec<Rect> = (0..m)
                    .map(|j| if along_x { Rect { x: start + j * pitch, y: across, w: size, h: size } } else { Rect { x: across, y: start + j * pitch, w: size, h: size } })
                    .collect();
                (rects, n as i32 - m)
            });
            if let Some((_, missing)) = array.as_ref().filter(|(_, m)| *m > 0) {
                out.push(Violation { rule: format!("em access net {net} pin {},{}", px + origin.0, py + origin.1), margin: i64::from(*missing) });
            }
            for end in node_end.into_iter().chain([(px, py)]) {
                if let Some((rects, _)) = array.as_ref().filter(|_| end == (px, py)) {
                    for &r in rects {
                        let crowded = wires.iter().any(|w| w.layer == cut && rect_gap(w.rect, r) < cfg.space(cut, 0, 0, size));
                        if !crowded {
                            wires.push(Shape { layer: cut, rect: r });
                        }
                    }
                    let grow = (above - size) / 2;
                    let (x0, y0) = (rects[0].x - grow, rects[0].y - grow);
                    let last = rects[rects.len() - 1];
                    wires.push(Shape { layer: metal, rect: Rect { x: x0, y: y0, w: last.x + size + grow - x0, h: last.y + size + grow - y0 } });
                    continue;
                }
                // A same-net cut already within cut spacing (a trunk via at the
                // node, beside the pin) joins these two layers here: another cut
                // would only break the spacing, and the pads below overlap it.
                let c = pad(cut, end, size);
                let crowded = wires.iter().any(|w| w.layer == cut && w.rect != c.rect && rect_gap(w.rect, c.rect) < cfg.space(cut, 0, 0, size));
                if !crowded {
                    wires.push(c);
                }
                // A cut that fits inside the pin is hosted by the cell's own
                // conductor; a base pad there only adds li spacing violations.
                // Only when that conductor *is* the cut's lower layer: a pin on
                // a layer below the stack (poly) encloses nothing on `base`.
                // Hosted only when the pin also gives the cut the deck's
                // enclosure all round.
                let e = cfg.cut_enclosure.iter().find(|&&(c, ..)| c == cut).map_or(0, |&(_, e, _)| e);
                let (cl, ct) = (end.0 - size / 2 - e, end.1 - size / 2 - e);
                let hosted = end == (px, py)
                    && a.pin_layer == base
                    && cl >= a.pin.x
                    && cl + size + 2 * e <= a.pin.x + a.pin.w
                    && ct >= a.pin.y
                    && ct + size + 2 * e <= a.pin.y + a.pin.h;
                if !hosted {
                    wires.push(pad(base, end, below));
                }
                // The pin end's pad slides toward the node along the leg that
                // reaches the pin, as far as the cut stays enclosed (a
                // neighbouring pin's pad sits the other way) and no closer to
                // foreign pads and routed metal than spacing.
                let at = if end == (px, py) {
                    let e = cfg.cut_enclosure.iter().find(|&&(c, ..)| c == cut).map_or(0, |&(.., e)| e);
                    let g = cfg.grid.max(1);
                    let slide = ((above - size - 2 * e) / 2).max(0).div_euclid(g) * g;
                    let (dx, dy) = ((nx - px).signum(), (ny - py).signum());
                    let along_x = if flip { dx != 0 } else { dy == 0 };
                    let slid = |s: i32| if along_x { (px + dx * s, py) } else { (px, py + dy * s) };
                    let room = |c: (i32, i32)| {
                        let r = pad(metal, c, above).rect;
                        let zone = |&(zci, zx, zy): &(u32, i32, i32)| (zci as usize != a.ci).then(|| rect_gap(r, pad(metal, (zx, zy), pad_zone).rect));
                        zones.iter().filter_map(zone).chain(foreign.iter().map(|&f| rect_gap(r, f))).fold(spacing, i32::min)
                    };
                    // Farthest slide among those with the most room.
                    (0..=slide / g).map(|k| slid(k * g)).max_by_key(|&c| room(c)).unwrap_or(end)
                } else {
                    end
                };
                wires.push(pad(metal, at, above));
            }
        }
        // The landed node's layer to the jog: extraction draws nothing for a
        // one-node run, and a jog on another metal (a pin on met2 landed on
        // met1) touches the node through no cut, so stack cuts between them.
        let node_layer = layers.get(a.node_layer as usize).copied();
        if let (Some(nl), Some(ml)) = (node_layer.filter(|l| !reached.contains(l)).map(|_| a.node_layer as usize), layers.iter().position(|&l| l == metal)) {
            for k in nl.min(ml)..nl.max(ml) {
                if let Some(&(cut, size, below, above)) = cuts.get(k) {
                    wires.push(pad(cut, (nx, ny), size));
                    wires.push(pad(layers[k], (nx, ny), below));
                    wires.push(pad(layers[k + 1], (nx, ny), above));
                }
            }
        }
    }
    out
}

/// Current routing state as per-net shapes on real PDK layers: track wires on
/// `layers[i]` at the layer's width, vias as `cuts[i]` resized to the deck's
/// exact cut size, then a landing pad on each side of every cut (sized to
/// satisfy enclosure alone): `pad_along × pad_across` along the layer's wire
/// with specs, else the `Cut`'s square.
fn build_routes(
    hot: &RouteHot,
    cold: &RouteCtx<TrackGrid>,
    cfg: &DetailedCfg,
    compact: &[usize],
    n_nets: usize,
    layers: &[LayerId],
    cuts: &[Cut],
) -> Routes {
    let grid = &cold.graph;
    let widths: Vec<i32> = (0..grid.n_layers as usize).map(|l| cfg.wire(l)).collect();
    let wide: Vec<i32> = layers.iter().take(grid.n_layers as usize).map(|&l| cfg.wide(l)).collect();
    let (wires, vias) = extract_geometry(hot, grid, &widths, &|net, b| cold.branch_k(net, b), &wide);
    let mut out = vec![Vec::new(); n_nets];
    let wire_shapes = to_shapes(compact.len(), &wires, &[]);
    let via_shapes = to_shapes(compact.len(), &[], &vias);
    for (ci, (ws, vs)) in wire_shapes.into_iter().zip(via_shapes).enumerate() {
        let dst = &mut out[compact[ci]];
        dst.extend(ws.into_iter().filter_map(|s| Some(Shape { layer: *layers.get(s.layer.0 as usize)?, ..s })));
        let mut pads = Vec::new();
        for v in &vs {
            let i = v.layer.0 as usize;
            let Some(&cut) = cuts.get(i) else { continue };
            dst.push(Shape { layer: cut.0, rect: centred(v.rect, cut.1, cut.1) });
            pads.extend(via_pads(cfg, layers, cut, i, v.rect).into_iter().flatten());
        }
        dst.append(&mut pads);
    }
    assert_on_stack(&out, layers, cuts);
    Routes { wires: out, ..Default::default()  }
}

/// Every emitted shape is routing metal or an exactly-sized cut. A wrong
/// index→layer table otherwise draws wires on wells and diffusion silently.
/// `assert!` because the flow runs in release.
fn assert_on_stack(out: &[Vec<Shape>], layers: &[LayerId], cuts: &[Cut]) {
    for s in out.iter().flatten() {
        match cuts.iter().find(|(c, ..)| *c == s.layer) {
            Some(&(_, size, ..)) => assert!(
                s.rect.w == size && s.rect.h == size,
                "cut {:?} drawn {}x{}, deck requires {size}x{size}",
                s.layer,
                s.rect.w,
                s.rect.h
            ),
            None => assert!(layers.contains(&s.layer), "routed shape on non-routing layer {:?}", s.layer),
        }
    }
}

/// Residual track overuse `Σ over` ([`RouteHot::over`], halos included).
fn overuse(hot: &RouteHot) -> f32 {
    (0..hot.usage.len()).map(|n| f32::from(hot.over(n, 1))).sum()
}

/// Extra cost per node for a mirrored net off its partner's mirror image.
const GUIDE_COST: f32 = 1.0;
/// Extra cost per node adjacent (same layer, one track) to an aggressor.
const COUPLE_COST: f32 = 2.0;
/// Extra cost per node on a layer an antenna repair steers off: two vias'
/// worth over a run makes hopping to another metal the cheaper path.
const JUMP_COST: f32 = VIA_COST / 2.0;

/// Extra cost per node beside a lifted gate ([`lift_field`]): dearer than
/// climbing the whole stack and back.
const LIFT_COST: f32 = 16.0 * VIA_COST;

/// Rip-up/reroute trials driven by violated routing rules, up to `HARD_ROUNDS`
/// rounds while a trial is accepted. Per [`RepairKind`] (never the kind string):
///
/// * `Mirror` (`Differential`): copy one side's whole tree onto the other when a
///   translation or mirror carries its terminals exactly onto the other's
///   (identical route signature by construction); else reroute one side
///   along the mirror image of the other;
/// * `Balance` (`CommonNodes`): reroute the skewed shared node along the
///   members' bisector ([`balance_field`]);
/// * `KeepAway`, victim only (`CouplingBudget`): reroute each victim priced
///   away from all foreign tracks (pairs named by `CrosstalkExclusion` are
///   kept apart in the search itself);
/// * `Antenna`: reroute the net with one layer priced up (a jumper), per layer,
///   then with its gates lifted over its cells' metal ([`lift_field`]), then
///   plainly;
/// * `Em`, `None`: no trial;
/// * `Reroute`, `Shield`, `Ir`, `Budget`: reroute the nets it names at doubled
///   `p_fac`.
///
/// Returns the number of trials run (each a rip-up/reroute or a tree copy).
///
/// A trial is kept only if `(hard violations, Σ residual)` improves
/// lexicographically without raising overuse. `probe` draws the current state
/// with pin access (what ships, less arrays and fill). The key and the
/// violated list cover `reqs.hard ∪ reqs.budget ∪ extra` less
/// `RepairKind::Em` (probes carry no terminals; EM is repaired on the drawn
/// routes). `IrDrop` stays in until IR repair moves out (RTE-21). Each
/// trial probes once: the key before it is the last accepted one, and
/// [`rescore`] reuses unchanged local batches (RTE-23).
#[allow(clippy::too_many_arguments)]
fn repair_constraints(
    hot: &mut RouteHot,
    cold: &RouteCtx<TrackGrid>,
    reqs: &Requirements<Routes>,
    extra: &[Box<dyn analog::RuleBatch<Routes>>],
    common: &[analog::routing::CommonNode],
    ci_of: &[usize],
    lift: &[Option<(u32, Vec<((i32, i32), Rect)>)>],
    probe: &impl Fn(&RouteHot, &RouteCtx<TrackGrid>) -> Routes,
    dij: &mut Dij,
) -> u32 {
    let not_em = |b: &&Box<dyn analog::RuleBatch<Routes>>| b.repair_kind() != RepairKind::Em;
    let hard: Vec<&dyn analog::RuleBatch<Routes>> = reqs.hard.iter().filter(not_em).map(|b| &**b).collect();
    let n_hard = hard.len();
    let batches: Vec<&dyn analog::RuleBatch<Routes>> = hard.into_iter().chain(reqs.budget.iter().chain(extra).filter(not_em).map(|b| &**b)).collect();
    let measure = |hot: &RouteHot, prev: Option<&Probed>| {
        let routes = probe(hot, cold);
        let scores = rescore(&batches, &routes, prev.map(|p| (&p.routes, &p.scores[..])));
        let key = (scores[..n_hard].iter().map(|s| s.0).sum(), scores.iter().map(|s| s.1).sum(), overuse(hot));
        Probed { routes, scores, key }
    };
    let ci = |n: u32| ci_of.get(n as usize).copied().filter(|&c| c != usize::MAX);
    let (mut p_fac, mut n) = (P_FAC, 0);
    let mut cur = measure(hot, None);
    for _ in 0..HARD_ROUNDS {
        p_fac *= 2.0;
        let routes = cur.routes.clone();
        let violated: Vec<_> = batches
            .iter()
            .zip(&cur.scores)
            .enumerate()
            .filter(|&(i, (_, s))| if i < n_hard { s.0 > 0 } else { s.1 > 0.0 })
            .map(|(_, (b, _))| *b)
            .collect();
        let mut accepted = false;
        for batch in violated {
            let mut ids = Vec::new();
            batch.touched(&mut ids);
            let pairs: Vec<(usize, usize)> =
                ids.chunks_exact(2).filter_map(|p| Some((ci(p[0])?, ci(p[1])?))).collect();
            let mut trials: Vec<Vec<(usize, Vec<f32>)>> = Vec::new();
            match batch.repair_kind() {
                RepairKind::Mirror => {
                    // An exact pair is mirrored by construction (RTE-15).
                    for &(a, b) in pairs.iter().filter(|&&(a, _)| cold.mirror.get(a).is_none_or(Option::is_none)) {
                        for (from, to) in [(a, b), (b, a)] {
                            if let Some(tree) = copy_tree(hot, cold, from, to) {
                                n += 1;
                                accepted |= commit_trial(hot, cold, to, tree, &mut cur, &measure);
                            }
                        }
                        trials.extend(mirror_guide(hot, &cold.graph, &cold.terms, a, b).map(|g| vec![(b, g)]));
                        trials.extend(mirror_guide(hot, &cold.graph, &cold.terms, b, a).map(|g| vec![(a, g)]));
                    }
                }
                // Reroute a skewed shared source along the members' bisector:
                // nodes cost by how unequal their distances to the two sides'
                // pins are, so the trunk splits into mirrored branches.
                RepairKind::Balance => {
                    ids.clear();
                    batch.violating_ids(&routes, &mut ids);
                    for n in common.iter().filter(|n| ids.contains(&u32::from(n.net.0))) {
                        if let Some(c) = ci(u32::from(n.net.0)) {
                            if let [a, b] = n.groups.as_slice() {
                                trials.push(vec![(c, balance_field(&cold.graph, a, b))]);
                            }
                        }
                    }
                }
                // A victim-only rule (`CouplingBudget`): reroute away from all
                // foreign tracks. A rule naming its pairs (`CrosstalkExclusion`)
                // is enforced in the search (`RouteCtx::sep`, RTE-18): no trial.
                RepairKind::KeepAway => {
                    let mut named = Vec::new();
                    batch.keepaway_pairs(&mut named);
                    if named.is_empty() {
                        for v in ids.iter().filter_map(|&n| ci(n)) {
                            let others: Vec<usize> = (0..hot.trees.len()).filter(|&o| o != v).collect();
                            trials.push(vec![(v, keep_away(hot, &cold.graph, &others))]);
                        }
                    }
                }
                // Jumpers first (Hastings pp. 228–229): price one layer so the
                // net's long runs on it hop to another metal, which splits that
                // stage's conductor; then the lift over its cells' metal, then
                // the plain reroute. What routing cannot fix gets a diode
                // (`library`'s `elaborate::antenna_diodes`) where the deck
                // extracts and credits one.
                RepairKind::Antenna => {
                    ids.clear();
                    batch.violating_ids(&routes, &mut ids);
                    for n in ids.iter().filter_map(|&n| ci(n)) {
                        for l in 0..cold.graph.n_layers {
                            trials.push(vec![(n, jumper(&cold.graph, l))]);
                        }
                        if let Some((top, sites)) = &lift[n] {
                            trials.push(vec![(n, lift_field(hot, &cold.graph, n, *top, sites))]);
                        }
                        trials.push(vec![(n, Vec::new())]);
                    }
                }
                // A reroute cannot change width (AT-03, until RTE-12/14): EM
                // spends no trial (AT-24, REL-03 step 7).
                RepairKind::Em | RepairKind::None => {}
                RepairKind::Reroute | RepairKind::Shield | RepairKind::Ir | RepairKind::Budget => {
                    ids.clear();
                    batch.violating_ids(&routes, &mut ids);
                    ids.sort_unstable();
                    ids.dedup();
                    trials.push(ids.iter().filter_map(|&n| ci(n)).map(|n| (n, Vec::new())).collect());
                }
            }
            for t in trials {
                n += u32::from(!t.is_empty());
                accepted |= trial(hot, cold, t, p_fac, &mut cur, &measure, dij);
            }
        }
        if !accepted {
            break;
        }
    }
    n
}

/// The last accepted repair probe: its routes, per-batch `(violations,
/// residual)`, and key `(Σ hard violations, Σ residual, overuse)`.
struct Probed {
    routes: Routes,
    scores: Vec<(u32, f64)>,
    key: (u32, f64, f32),
}

/// Per batch `(violations, residual)` on `r`. With `prev` (an earlier probe
/// and its scores), a [`analog::RuleBatch::local`] batch none of whose
/// touched nets' wires differ between `prev` and `r` keeps its score; every
/// other batch is rescored. Probes share their cell metal and gates, so
/// wires are the only state a local batch can see change.
fn rescore(batches: &[&dyn analog::RuleBatch<Routes>], r: &Routes, prev: Option<(&Routes, &[(u32, f64)])>) -> Vec<(u32, f64)> {
    let mut ids = Vec::new();
    batches
        .iter()
        .enumerate()
        .map(|(i, b)| {
            if let Some((p, s)) = prev.filter(|_| b.local()) {
                ids.clear();
                b.touched(&mut ids);
                if ids.iter().all(|&n| p.wires.get(n as usize) == r.wires.get(n as usize)) {
                    return s[i];
                }
            }
            (b.violations(r), b.residual(r))
        })
        .collect()
}

/// `after` beats `before`: `(hard, residual)` lower without raising overuse.
fn improves(after: (u32, f64, f32), before: (u32, f64, f32)) -> bool {
    after.2 <= before.2 && (after.0, after.1) < (before.0, before.1)
}

/// Rip up every net in `reroutes`, reroute each with its penalty field, and keep
/// the result only if it [`improves`] on `cur` (the state's last accepted
/// probe); otherwise restore the old trees. Returns whether it was kept.
fn trial(
    hot: &mut RouteHot,
    cold: &RouteCtx<TrackGrid>,
    reroutes: Vec<(usize, Vec<f32>)>,
    p_fac: f32,
    cur: &mut Probed,
    measure: &impl Fn(&RouteHot, Option<&Probed>) -> Probed,
    dij: &mut Dij,
) -> bool {
    if reroutes.is_empty() {
        return false;
    }
    let old: Vec<(usize, Vec<Vec<u32>>)> = reroutes.iter().map(|(n, _)| (*n, hot.trees[*n].clone())).collect();
    for &(n, _) in &old {
        cold.commit(hot, n, Vec::new());
    }
    for ((n, field), (_, prev)) in reroutes.into_iter().zip(&old) {
        let tree = cold.reroute(hot, n, p_fac, &field, dij).unwrap_or_else(|| prev.clone());
        cold.commit(hot, n, tree);
    }
    let after = measure(hot, Some(cur));
    let better = improves(after.key, cur.key);
    if better {
        *cur = after;
    } else {
        for (n, tree) in old {
            cold.commit(hot, n, tree);
        }
    }
    better
}

/// Shield `victim` with `reference` tracks: every straight run of the victim
/// (≥ 2 nodes on one layer) claims the free stretches (≥ 2 nodes) of the
/// parallel track on each side, and the reference net is rerouted with one node of each claim
/// as an extra terminal, so each shield is tied in by real routing. The claims
/// join the reference tree. If the reroute fails or adds overuse, the
/// shortest claim is dropped and the rest retried ([`try_shield`]).
fn add_shield(hot: &mut RouteHot, cold: &mut RouteCtx<TrackGrid>, victim: usize, reference: usize, dij: &mut Dij) {
    let g = &cold.graph;
    let free = |n: u32| {
        let i = n as usize;
        hot.usage[i] == 0 && cold.reserved.get(i).is_none_or(|&o| o == NONE || o == reference as u32)
    };
    let mut claims: Vec<Vec<u32>> = Vec::new();
    for branch in &hot.trees[victim] {
        // Maximal same-layer runs.
        let mut runs: Vec<Vec<u32>> = Vec::new();
        for &n in branch {
            match runs.last_mut() {
                Some(run) if g.ixy(run[0]).2 == g.ixy(n).2 => run.push(n),
                _ => runs.push(vec![n]),
            }
        }
        for run in runs.into_iter().filter(|r| r.len() >= 2) {
            let horiz = g.ixy(run[0]).2 % 2 == 0;
            let stride = i64::from(g.stride(g.ixy(run[0]).2));
            for side in [-stride, stride] {
                // The parallel track, split into maximal free stretches (a pin's
                // access or another net may block part of it).
                let mut stretch: Vec<u32> = Vec::new();
                for &n in &run {
                    let (x, y, l) = g.ixy(n);
                    let (x, y) = if horiz { (i64::from(x), i64::from(y) + side) } else { (i64::from(x) + side, i64::from(y)) };
                    let m = (x >= 0 && y >= 0 && x < i64::from(g.nx) && y < i64::from(g.ny))
                        .then(|| g.node(x as u32, y as u32, l))
                        .filter(|&m| free(m));
                    match m {
                        Some(m) => stretch.push(m),
                        None if stretch.len() >= 2 => claims.push(std::mem::take(&mut stretch)),
                        None => stretch.clear(),
                    }
                }
                claims.extend((stretch.len() >= 2).then_some(stretch));
            }
        }
    }
    // Longest first: a claim the reference cannot reach without crossing
    // foreign metal costs the whole set, so the shortest is dropped and the
    // rest retried (RTE-19).
    claims.sort_by_key(|c| std::cmp::Reverse(c.len()));
    while !claims.is_empty() {
        if try_shield(hot, cold, reference, &claims, dij) {
            return;
        }
        claims.pop();
    }
}

/// Reroute `reference` with one node of each claim as an extra terminal and
/// the claims joined to its tree; kept only when overuse does not rise, else
/// everything is restored. Returns whether it was kept.
fn try_shield(hot: &mut RouteHot, cold: &mut RouteCtx<TrackGrid>, reference: usize, claims: &[Vec<u32>], dij: &mut Dij) -> bool {
    let (old_tree, old_terms, over0) = (hot.trees[reference].clone(), cold.terms[reference].clone(), overuse(hot));
    let old_reserved: Vec<(usize, u32)> = claims.iter().flatten().map(|&n| (n as usize, cold.reserved.get(n as usize).copied().unwrap_or(NONE))).collect();
    for &n in claims.iter().flatten() {
        if let Some(o) = cold.reserved.get_mut(n as usize) {
            *o = reference as u32;
        }
    }
    cold.terms[reference].extend(claims.iter().map(|c| c[0]));
    let routed = cold.reroute(hot, reference, P_FAC, &[], dij);
    if let Some(mut tree) = routed {
        tree.extend(claims.iter().cloned());
        cold.commit(hot, reference, tree);
        if overuse(hot) <= over0 {
            return true;
        }
        cold.commit(hot, reference, old_tree);
    }
    cold.terms[reference] = old_terms;
    for (i, o) in old_reserved {
        if let Some(r) = cold.reserved.get_mut(i) {
            *r = o;
        }
    }
    false
}

/// Keep `tree` as `net`'s route iff it [`improves`] on `cur`; else restore.
/// Returns whether it was kept.
fn commit_trial(hot: &mut RouteHot, cold: &RouteCtx<TrackGrid>, net: usize, tree: Vec<Vec<u32>>, cur: &mut Probed, measure: &impl Fn(&RouteHot, Option<&Probed>) -> Probed) -> bool {
    let old = hot.trees[net].clone();
    cold.commit(hot, net, tree);
    let after = measure(hot, Some(cur));
    let better = improves(after.key, cur.key);
    if better {
        *cur = after;
    } else {
        cold.commit(hot, net, old);
    }
    better
}

/// `from`'s tree carried onto `to` by the track transform (a translation, else
/// a mirror about a vertical axis; layers and directions kept) that maps
/// `from`'s terminals exactly onto `to`'s. The copy has the same per-layer
/// lengths and vias as its template — a matched pair by construction. `None`
/// when no transform matches the terminals, a node falls off the grid, or it
/// enters a node another net occupies or has reserved.
fn copy_tree(hot: &RouteHot, cold: &RouteCtx<TrackGrid>, from: usize, to: usize) -> Option<Vec<Vec<u32>>> {
    let g = &cold.graph;
    let (ta, tb) = (&cold.terms[from], &cold.terms[to]);
    if ta.is_empty() || ta.len() != tb.len() || hot.trees[from].is_empty() {
        return None;
    }
    let bins = |t: &[u32]| t.iter().map(|&n| g.ixy(n)).collect::<Vec<_>>();
    let (ba, bb) = (bins(ta), bins(tb));
    // Anchor on each side's terminal extremes, per axis.
    let lo = |v: &[(u32, u32, u32)], f: fn(&(u32, u32, u32)) -> u32| v.iter().map(f).min().map_or(0, i64::from);
    let hi = |v: &[(u32, u32, u32)], f: fn(&(u32, u32, u32)) -> u32| v.iter().map(f).max().map_or(0, i64::from);
    let (x_of, y_of): (fn(&(u32, u32, u32)) -> u32, fn(&(u32, u32, u32)) -> u32) = (|p| p.0, |p| p.1);
    let dx = lo(&bb, x_of) - lo(&ba, x_of);
    let dy = lo(&bb, y_of) - lo(&ba, y_of);
    let shift = move |(x, y, l): (u32, u32, u32)| (x as i64 + dx, y as i64 + dy, l);
    // Mirror: the leftmost of `from` lands on the rightmost of `to`.
    let axis2 = lo(&ba, x_of) + hi(&bb, x_of);
    let flip = move |(x, y, l): (u32, u32, u32)| (axis2 - x as i64, y as i64 + dy, l);
    let mine = hot.tree_nodes(to);
    for map in [&shift as &dyn Fn((u32, u32, u32)) -> (i64, i64, u32), &flip] {
        let mut mapped: Vec<u32> = Vec::new();
        let node = |p: (u32, u32, u32)| {
            let (x, y, l) = map(p);
            (x >= 0 && y >= 0 && x < i64::from(g.nx) && y < i64::from(g.ny)).then(|| g.node(x as u32, y as u32, l))
        };
        let Some(mut tt) = ba.iter().map(|&p| node(p)).collect::<Option<Vec<u32>>>() else { continue };
        tt.sort_unstable();
        let mut want = tb.clone();
        want.sort_unstable();
        if tt != want {
            continue;
        }
        let tree: Option<Vec<Vec<u32>>> = hot.trees[from]
            .iter()
            .map(|br| br.iter().map(|&n| node(g.ixy(n))).collect())
            .collect();
        let Some(tree) = tree else { continue };
        mapped.extend(tree.iter().flatten().copied());
        let free = mapped.iter().all(|&n| {
            let i = n as usize;
            let owner = cold.reserved.get(i).copied().unwrap_or(NONE);
            (owner == NONE || owner == to as u32) && (hot.usage[i] == 0 || mine.binary_search(&n).is_ok())
        });
        if free {
            return Some(tree);
        }
    }
    None
}

/// Guide field for net `b`: zero on the mirror image of `a`'s tree about the
/// vertical axis between the two nets' terminal centroids, `GUIDE_COST`
/// elsewhere. `None` when `b`'s terminals are not (within a track) the mirror
/// of `a`'s — the pair is not placed symmetrically, so no mirror route exists.
fn mirror_guide(hot: &RouteHot, grid: &TrackGrid, terms: &[Vec<u32>], a: usize, b: usize) -> Option<Vec<f32>> {
    let xs = |n: usize| terms[n].iter().map(|&t| grid.pos(t)).collect::<Vec<_>>();
    let (ta, tb) = (xs(a), xs(b));
    if ta.is_empty() || ta.len() != tb.len() || hot.trees[a].is_empty() {
        return None;
    }
    let mean = |v: &[(i32, i32, u32)]| v.iter().map(|p| i64::from(p.0)).sum::<i64>() / v.len() as i64;
    let axis2 = (mean(&ta) + mean(&tb)) as i32; // twice the axis x
    let tol = grid.pitch;
    let mirrored = ta.iter().all(|&(x, y, _)| {
        tb.iter().any(|&(bx, by, _)| ((axis2 - x) - bx).abs() <= tol && (y - by).abs() <= tol)
    });
    if !mirrored {
        return None;
    }
    let mut field = vec![GUIDE_COST; grid.nodes()];
    for n in hot.tree_nodes(a) {
        let (x, y, l) = grid.pos(n);
        field[grid.node(grid.bin_x(axis2 - x), grid.bin_y(y), l) as usize] = 0.0;
    }
    Some(field)
}

/// Per net, the placed cells' metal its pins reach (each connected piece on
/// `stack` holding a shape on the pin's layer — the stack's lowest for a pin
/// below it — over the pin), and its gate pins (`…:G`) with their device and
/// gate area from `gate_nm2` ([`DetailedCfg::gate_nm2`]). Empty without a stack.
/// A cut inside a capacitor plate keep-out (a MIM's via3 on capm) joins no
/// two stack layers: it contacts the plate, and the deck's metal via is the
/// cut off it (sky130 `via3_m3 = via3 not capm`).
///
/// ponytail: O(k²) per cell ([`analog::routing::Stack::connected`]); a cap
/// array's thousands of cuts are the worst case.
fn cell_metal(placed: &[Macro], n_nets: usize, stack: Option<&analog::routing::Stack>, gate_nm2: &[Vec<(String, u32, i64)>]) -> (Vec<Vec<Shape>>, Vec<Vec<GatePin>>) {
    let (mut cell, mut gates) = (vec![Vec::new(); n_nets], vec![Vec::new(); n_nets]);
    let Some(stack) = stack else { return (cell, gates) };
    let on_stack = |l: LayerId| stack.layers.iter().any(|x| x.id == l.0);
    let lowest = stack.layers.first().map(|l| LayerId(l.id));
    let cut = |l: LayerId| stack.layers.iter().any(|x| x.id == l.0 && x.cut);
    for (c, m) in placed.iter().enumerate() {
        let plates: Vec<Rect> = m.keepouts.iter().filter(|k| matches!(k.why, pnr_core::KeepWhy::CapPlate { .. })).map(|k| k.rect).collect();
        let inside = |r: Rect| plates.iter().any(|p| r.x >= p.x && r.y >= p.y && r.x + r.w <= p.x + p.w && r.y + r.h <= p.y + p.h);
        let on: Vec<usize> = (0..m.shapes.len()).filter(|&k| !(cut(m.shapes[k].layer) && inside(m.shapes[k].rect))).collect();
        let shapes: Vec<Shape> = on.iter().map(|&k| m.shapes[k]).collect();
        let pieces: Vec<Vec<usize>> = stack.connected(&shapes).into_iter().map(|p| p.into_iter().map(|k| on[k]).collect()).collect();
        let mut taken = vec![false; pieces.len()];
        for p in &m.pins {
            let Some(net) = cell.get_mut(p.net.0 as usize) else { continue };
            if p.name.ends_with(":G") {
                let known = gate_nm2.get(c).and_then(|t| t.iter().find(|(n, ..)| *n == p.name));
                let (dev, nm2) = known.map_or((u32::MAX, 0), |&(_, d, a)| (d, a));
                gates[p.net.0 as usize].push(GatePin { at: p.at, dev, nm2 });
            }
            let layer = if on_stack(p.layer) { Some(p.layer) } else { lowest };
            for (i, piece) in pieces.iter().enumerate() {
                if !taken[i] && piece.iter().any(|&k| Some(m.shapes[k].layer) == layer && m.shapes[k].rect.touches(&p.at)) {
                    taken[i] = true;
                    net.extend(piece.iter().map(|&k| m.shapes[k]));
                }
            }
        }
    }
    (cell, gates)
}

/// `JUMP_COST` on every node of lattice layer `layer`.
fn jumper(grid: &TrackGrid, layer: u32) -> Vec<f32> {
    let mut field = vec![0.0; grid.nodes()];
    let size = grid.layer_size() as usize;
    let start = layer as usize * size;
    field[start..start + size].fill(JUMP_COST);
    field
}

/// Antenna lift over a cell's plate: [`LIFT_COST`] on every node of layers
/// `0..=top` within two pitches of a gate's access (landed node to pin),
/// save the node's own column. The gate climbs straight past `top` and the
/// net comes back down only away from it, so at every stage up to `top` the
/// gate's conductor is its column and jog alone: the plate charges no gate
/// there. Two pitches, so fattening and same-net fill cannot bridge the gap.
/// A node another net holds costs twice that: a lift that buys overuse is
/// refused anyway (cell metal is reserved, never entered).
fn lift_field(hot: &RouteHot, grid: &TrackGrid, net: usize, top: u32, sites: &[((i32, i32), Rect)]) -> Vec<f32> {
    let mine = hot.tree_nodes(net);
    let mut field: Vec<f32> = (0..grid.nodes())
        .map(|i| if hot.usage[i] > u16::from(mine.binary_search(&(i as u32)).is_ok()) { 2.0 * LIFT_COST } else { 0.0 })
        .collect();
    let m = 2 * grid.pitch;
    for &((x, y), pin) in sites {
        let own = (grid.bin_x(x), grid.bin_y(y));
        for l in 0..=top {
            for gy in grid.bin_y(y.min(pin.y) - m)..=grid.bin_y(y.max(pin.y + pin.h) + m) {
                for gx in grid.bin_x(x.min(pin.x) - m)..=grid.bin_x(x.max(pin.x + pin.w) + m) {
                    if (gx, gy) != own {
                        field[grid.node(gx, gy, l) as usize] += LIFT_COST;
                    }
                }
            }
        }
    }
    field
}

/// Per node, [`BALANCE_COST`] per pitch of difference between its distances
/// to the nearest `a` pin and the nearest `b` pin (Manhattan, pin centres).
fn balance_field(grid: &TrackGrid, a: &[Rect], b: &[Rect]) -> Vec<f32> {
    let centre = |r: &Rect| (r.x + r.w / 2, r.y + r.h / 2);
    let (a, b): (Vec<_>, Vec<_>) = (a.iter().map(centre).collect(), b.iter().map(centre).collect());
    let near = |p: &[(i32, i32)], x: i32, y: i32| p.iter().map(|&(px, py)| (px - x).abs() + (py - y).abs()).min().unwrap_or(0);
    (0..grid.nodes() as u32)
        .map(|n| {
            let (x, y, _) = grid.pos(n);
            BALANCE_COST * (near(&a, x, y) - near(&b, x, y)).abs() as f32 / grid.pitch.max(1) as f32
        })
        .collect()
}

/// Cost per pitch of imbalance in [`balance_field`].
const BALANCE_COST: f32 = 0.5;

/// `COUPLE_COST` on every node within one track (same layer) of `nets`' trees.
fn keep_away(hot: &RouteHot, grid: &TrackGrid, nets: &[usize]) -> Vec<f32> {
    let mut field = vec![0.0; grid.nodes()];
    for &net in nets {
        for n in hot.tree_nodes(net) {
            let (ix, iy, l) = grid.ixy(n);
            for (dx, dy) in [(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)] {
                let (jx, jy) = (ix as i32 + dx, iy as i32 + dy);
                if jx >= 0 && jy >= 0 && jx < grid.nx as i32 && jy < grid.ny as i32 {
                    field[grid.node(jx as u32, jy as u32, l) as usize] = COUPLE_COST;
                }
            }
        }
    }
    field
}

/// Report: analog tiers; open nets (margin = pieces past the first),
/// sacrificed/unlanded pins, drawn shorts (between nets, and to `foreign` cell
/// or ring metal, absolute, with its owning net) and unresolved congestion
/// (margin = residual track overuse) in V (EM is the hard rule's alone, C18);
/// cost = raw overuse + criticality-weighted analog cost.
fn score(
    routes: &Routes,
    reqs: &Requirements<Routes>,
    overuse: f32,
    joins: &[Join],
    foreign: &[(Option<u32>, Shape)],
    sacrificed: &[usize],
    side: i32,
) -> Report {
    let (mut hard, budget) = gr::analog_tiers(routes, reqs);
    for (net, shapes) in routes.wires.iter().enumerate() {
        let open = open_components(shapes, joins);
        if open > 0 {
            hard.push(Violation { rule: format!("open net {net}"), margin: open as i64 });
        }
    }
    // Counted, not proven: the pin may still be tied through the cell, so this
    // over-reports and never under-reports.
    for (net, &n) in sacrificed.iter().enumerate().filter(|(_, &n)| n > 0) {
        hard.push(Violation { rule: format!("pin access sacrificed on net {net}"), margin: n as i64 });
    }
    let (nets, cell) = cross_net_shorts(&routes.wires, joins, foreign, side);
    for (a, b) in nets {
        hard.push(Violation { rule: format!("drawn short nets {a}/{b}"), margin: 1 });
    }
    for n in cell {
        hard.push(Violation { rule: format!("drawn short net {n} to cell metal"), margin: 1 });
    }
    if overuse > 0.0 {
        hard.push(Violation { rule: "unresolved congestion".into(), margin: overuse as i64 });
    }
    let analog_cost: f32 = reqs.cost.iter().map(|b| b.criticality(routes) * b.cost(routes)).sum();
    Report { hard_violations: hard, budget_violations: budget, cost: overuse + analog_cost }
}

/// Snap `r` outward onto the manufacturing grid.
fn snap_out(r: Rect, grid: i32) -> Rect {
    let x = r.x.div_euclid(grid) * grid;
    let y = r.y.div_euclid(grid) * grid;
    let w = ((r.x + r.w) - x + grid - 1).div_euclid(grid) * grid;
    let h = ((r.y + r.h) - y + grid - 1).div_euclid(grid) * grid;
    Rect { x, y, w, h }
}

/// Bridge sub-`min_space` gaps between pairs of one net's same-layer pieces:
/// aligned gaps with a full-hull-width span (else their parallel run), corner
/// gaps with a patch reaching `min_feat` into both. Skips gaps already covered
/// (corner pairs already bridged) and fillers that would come within
/// `min_space` of `foreign`. Up to four passes.
fn heal_same_net_slivers(shapes: &mut Vec<Shape>, layers: &[LayerId], min_space: i32, min_feat: i32, foreign: &[Shape], grid: i32) {
    if min_space <= 0 {
        return;
    }
    let touches = |a: Rect, b: Rect| rect_gap(a, b) == 0;
    for _ in 0..4 {
        let mut fillers: Vec<Shape> = Vec::new();
        for i in 0..shapes.len() {
            let layer = shapes[i].layer;
            if !layers.contains(&layer) {
                continue;
            }
            for j in (i + 1)..shapes.len() {
                if shapes[j].layer != layer {
                    continue;
                }
                let (a, b) = (shapes[i].rect, shapes[j].rect);
                let dx = (b.x - (a.x + a.w)).max(a.x - (b.x + b.w));
                let dy = (b.y - (a.y + a.h)).max(a.y - (b.y + b.h));
                if dx.max(dy) <= 0 || dx.max(dy) >= min_space {
                    continue;
                }
                let (hx0, hy0) = (a.x.min(b.x), a.y.min(b.y));
                let (hx1, hy1) = ((a.x + a.w).max(b.x + b.w), (a.y + a.h).max(b.y + b.h));
                // Aligned gaps: the hull-wide span, else just the pieces'
                // parallel run (a third piece touching both connects them but
                // leaves the gap beside it a notch, so only a gap already
                // covered counts as healed).
                let fouls = |r: Rect| foreign.iter().any(|f| f.layer == layer && rect_gap(r, f.rect) < min_space);
                let covered = |r: Rect| shapes.iter().chain(&fillers).any(|s| s.layer == layer && contains(s.rect, r));
                let aligned = if dx <= 0 {
                    let (g0, g1) = ((a.y + a.h).min(b.y + b.h), a.y.max(b.y));
                    let (r0, r1) = (a.x.max(b.x), (a.x + a.w).min(b.x + b.w));
                    Some([span_fill(hx0, hx1, g0, g1, min_feat, false), span_fill(r0, r1, g0, g1, min_feat, false)])
                } else if dy <= 0 {
                    let (g0, g1) = ((a.x + a.w).min(b.x + b.w), a.x.max(b.x));
                    let (r0, r1) = (a.y.max(b.y), (a.y + a.h).min(b.y + b.h));
                    Some([span_fill(hy0, hy1, g0, g1, min_feat, true), span_fill(r0, r1, g0, g1, min_feat, true)])
                } else {
                    None
                };
                if let Some(spans) = aligned {
                    // The gap itself, which a filler must cover.
                    let gap = if dx <= 0 {
                        Rect { x: a.x.max(b.x), y: (a.y + a.h).min(b.y + b.h), w: (a.x + a.w).min(b.x + b.w) - a.x.max(b.x), h: dy }
                    } else {
                        Rect { x: (a.x + a.w).min(b.x + b.w), y: a.y.max(b.y), w: dx, h: (a.y + a.h).min(b.y + b.h) - a.y.max(b.y) }
                    };
                    if !covered(gap) {
                        if let Some(f) = spans.into_iter().map(|r| snap_out(r, grid)).find(|&r| r.w > 0 && r.h > 0 && !fouls(r)) {
                            fillers.push(Shape { layer, rect: f });
                        }
                    }
                    continue;
                }
                let x0 = ((a.x + a.w).min(b.x + b.w) - min_feat).max(hx0);
                let x1 = (a.x.max(b.x) + min_feat).min(hx1);
                let y0 = ((a.y + a.h).min(b.y + b.h) - min_feat).max(hy0);
                let y1 = (a.y.max(b.y) + min_feat).min(hy1);
                let rect = Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
                let bridged = shapes.iter().chain(&fillers).any(|s| {
                    s.layer == layer && s.rect != a && s.rect != b && touches(s.rect, a) && touches(s.rect, b)
                });
                let filler = snap_out(rect, grid);
                if !bridged && !fouls(filler) && rect.w > 0 && rect.h > 0 {
                    fillers.push(Shape { layer, rect: filler });
                }
            }
        }
        if fillers.is_empty() {
            return;
        }
        shapes.append(&mut fillers);
    }
}

/// Aligned-gap filler: span `[p0, p1]` (grown about its centre to `min_feat`:
/// a filler narrower than that is a width violation of its own), across gap
/// `[g0, g1]` extended `min_feat` into each side. `swap` puts the gap on x.
fn span_fill(p0: i32, p1: i32, g0: i32, g1: i32, min_feat: i32, swap: bool) -> Rect {
    let (mut p0, mut p1) = (p0, p1);
    if p1 - p0 < min_feat {
        p0 -= (min_feat - (p1 - p0)) / 2;
        p1 = p0 + min_feat;
    }
    let (g0, g1) = (g0 - min_feat, g1 + min_feat);
    if swap {
        Rect { x: g0, y: p0, w: g1 - g0, h: p1 - p0 }
    } else {
        Rect { x: p0, y: g0, w: p1 - p0, h: g1 - g0 }
    }
}

/// Fill sub-`min_space` concave gaps and holes in one net's same-layer union.
///
/// Coordinate-compressed occupancy over the net's own rect edges: an empty cell
/// narrower than `min_space` with material on both sides along that axis is
/// filled (grown to `min_feat`, since the checker measures min_width per drawn
/// rect). Unlike the pairwise healer this sees notches a third piece creates.
/// Fillers within `min_space` of `foreign` are skipped. Two passes.
fn fill_same_net_notches(shapes: &mut Vec<Shape>, layers: &[LayerId], min_space: i32, min_feat: i32, foreign: &[Shape], grid: i32) {
    if min_space <= 0 {
        return;
    }
    let mut present: Vec<LayerId> = shapes.iter().map(|s| s.layer).filter(|l| layers.contains(l)).collect();
    present.sort_unstable_by_key(|l| l.0);
    present.dedup();
    let snap = |v: i32| v.div_euclid(grid) * grid;
    let grow = |lo: i32, len: i32| if len >= min_feat { (lo, len) } else { (snap(lo - (min_feat - len) / 2), min_feat) };
    for _ in 0..2 {
        let mut fillers: Vec<Shape> = Vec::new();
        for &layer in &present {
            let rects: Vec<Rect> = shapes.iter().chain(&fillers).filter(|s| s.layer == layer).map(|s| s.rect).collect();
            if rects.len() < 2 {
                continue;
            }
            let axis = |f: fn(&Rect) -> [i32; 2]| {
                let mut v: Vec<i32> = rects.iter().flat_map(f).collect();
                v.sort_unstable();
                v.dedup();
                v
            };
            let xs = axis(|r| [r.x, r.x + r.w]);
            let ys = axis(|r| [r.y, r.y + r.h]);
            let (nx, ny) = (xs.len() - 1, ys.len() - 1);
            let mut full = vec![false; nx * ny];
            for r in &rects {
                let (i0, i1) = (xs.partition_point(|&v| v < r.x), xs.partition_point(|&v| v < r.x + r.w));
                let (j0, j1) = (ys.partition_point(|&v| v < r.y), ys.partition_point(|&v| v < r.y + r.h));
                for i in i0..i1 {
                    full[i * ny + j0..i * ny + j1].fill(true);
                }
            }
            for i in 0..nx {
                for j in 0..ny {
                    if full[i * ny + j] {
                        continue;
                    }
                    let (w, h) = (xs[i + 1] - xs[i], ys[j + 1] - ys[j]);
                    let across_x = w < min_space && i > 0 && i + 1 < nx && full[(i - 1) * ny + j] && full[(i + 1) * ny + j];
                    let across_y = h < min_space && j > 0 && j + 1 < ny && full[i * ny + j - 1] && full[i * ny + j + 1];
                    if !(across_x || across_y) {
                        continue;
                    }
                    // Grown about the gap, else flush with either end of it
                    // (away from foreign metal on the other side).
                    let spans = |lo: i32, len: i32| {
                        let hi = lo + len;
                        let s = snap(hi - min_feat);
                        if len >= min_feat { vec![(lo, len)] } else { vec![grow(lo, len), (s, hi - s), (lo, min_feat)] }
                    };
                    let (sx, sy) = (spans(xs[i], w), spans(ys[j], h));
                    let fits = |&(fx, fw): &(i32, i32), &(fy, fh): &(i32, i32)| {
                        // Growth landing wholly on the net's own metal adds none:
                        // only the gap itself is new, so only it can foul.
                        let own = (|| {
                            let (i0, i1) = (xs.partition_point(|&v| v <= fx).checked_sub(1)?, xs.partition_point(|&v| v < fx + fw));
                            let (j0, j1) = (ys.partition_point(|&v| v <= fy).checked_sub(1)?, ys.partition_point(|&v| v < fy + fh));
                            let inside = i1 <= nx && j1 <= ny && xs[i1] >= fx + fw && ys[j1] >= fy + fh;
                            Some(inside && (i0..i1).all(|a| (j0..j1).all(|b| (a, b) == (i, j) || full[a * ny + b])))
                        })()
                        .unwrap_or(false);
                        let new = if own { Rect { x: xs[i], y: ys[j], w, h } } else { Rect { x: fx, y: fy, w: fw, h: fh } };
                        !foreign.iter().any(|f| f.layer == layer && rect_gap(new, f.rect) < min_space)
                    };
                    let pick = sx.iter().flat_map(|x| sy.iter().map(move |y| (x, y))).find(|&(x, y)| fits(x, y));
                    if let Some((&(fx, fw), &(fy, fh))) = pick {
                        fillers.push(Shape { layer, rect: Rect { x: fx, y: fy, w: fw, h: fh } });
                    }
                }
            }
        }
        if fillers.is_empty() {
            return;
        }
        shapes.append(&mut fillers);
    }
}

/// Every cut this stage draws and the two conductors it joins — the routing vias
/// and the pin-access cut down to the reserved pin layer: the relation dr's
/// `open net` and `drawn short` entries are measured under.
#[must_use]
pub fn joins(layers: &[LayerId], cuts: &[Cut], pin_access: Option<(LayerId, Cut)>) -> Vec<Join> {
    let stack = cuts.iter().zip(layers.windows(2)).map(|(&(c, ..), w)| (c, w[0], w[1]));
    let access = pin_access.zip(layers.first()).map(|((pl, (c, ..)), &l0)| (c, pl, l0));
    stack.chain(access).collect()
}

/// A uniform-bucket spatial index over `(layer, rect)` items, by item index
/// (buckets of `side` nm; `4·pitch` in `route`).
struct ShapeIndex {
    side: i32,
    cells: HashMap<(LayerId, i32, i32), Vec<u32>>,
}

impl ShapeIndex {
    fn new(side: i32, items: impl IntoIterator<Item = (LayerId, Rect)>) -> Self {
        let mut idx = Self { side: side.max(1), cells: HashMap::new() };
        for (k, (l, r)) in items.into_iter().enumerate() {
            let (x0, x1, y0, y1) = idx.span(r, 0);
            for cx in x0..=x1 {
                for cy in y0..=y1 {
                    idx.cells.entry((l, cx, cy)).or_default().push(k as u32);
                }
            }
        }
        idx
    }

    /// Bucket range of `r` grown by `halo`, edges inclusive (touching rects share a bucket).
    fn span(&self, r: Rect, halo: i32) -> (i32, i32, i32, i32) {
        let b = |v: i32| v.div_euclid(self.side);
        (b(r.x - halo), b(r.x + r.w + halo), b(r.y - halo), b(r.y + r.h + halo))
    }

    /// Items on `layer` in a bucket `r` grown by `halo` reaches: a superset of
    /// those within `halo` of `r`, an item possibly more than once.
    fn near(&self, layer: LayerId, r: Rect, halo: i32) -> impl Iterator<Item = u32> + '_ {
        let (x0, x1, y0, y1) = self.span(r, halo);
        (x0..=x1).flat_map(move |cx| (y0..=y1).filter_map(move |cy| self.cells.get(&(layer, cx, cy)))).flatten().copied()
    }
}

/// The layers a shape on `layer` can meet ([`conductor_layers_meet`]): its
/// own, the cuts landing on it, and a cut's two conductors.
fn meeting(layer: LayerId, joins: &[Join]) -> Vec<LayerId> {
    let mut out = vec![layer];
    for &(c, lo, hi) in joins {
        if lo == layer || hi == layer {
            out.push(c);
        }
        if c == layer {
            out.extend([lo, hi]);
        }
    }
    out
}

/// Break drawn shorts between nets by deleting access shapes (index
/// `>= pre_access[net]`). Every touching cross-net pair `(a, b, i, j)` (a cut
/// counts on both layers it joins) is listed once, then taken in order, skipping
/// a pair whose shape is already gone: an access shape of the net later in
/// `rank` (position in the routing order) goes first, else the other net's,
/// each counted in `sacrificed`. A trunk-vs-trunk pair deletes nothing; `score`
/// reports it. Candidates come from a [`ShapeIndex`] of `side` nm buckets.
fn break_shorts(wires: &mut [Vec<Shape>], pre_access: &[usize], rank: &[usize], joins: &[Join], sacrificed: &mut [usize], side: i32) {
    let flat: Vec<(usize, usize)> = wires.iter().enumerate().flat_map(|(n, w)| (0..w.len()).map(move |i| (n, i))).collect();
    let idx = ShapeIndex::new(side, flat.iter().map(|&(n, i)| (wires[n][i].layer, wires[n][i].rect)));
    let mut pairs = Vec::new();
    for &(a, i) in &flat {
        let sa = &wires[a][i];
        for l in meeting(sa.layer, joins) {
            for k in idx.near(l, sa.rect, 0) {
                let (b, j) = flat[k as usize];
                let sb = &wires[b][j];
                if b > a && conductor_layers_meet(sa, sb, joins) && rect_gap(sa.rect, sb.rect) == 0 {
                    pairs.push((a, b, i, j));
                }
            }
        }
    }
    pairs.sort_unstable();
    pairs.dedup();
    let mut gone: HashSet<(usize, usize)> = HashSet::new();
    for (a, b, i, j) in pairs {
        if gone.contains(&(a, i)) || gone.contains(&(b, j)) {
            continue;
        }
        let (first, second) = if rank[b] >= rank[a] { ((b, j), (a, i)) } else { ((a, i), (b, j)) };
        if let Some(&(n, k)) = [first, second].iter().find(|&&(n, k)| k >= pre_access[n]) {
            gone.insert((n, k));
            sacrificed[n] += 1;
        }
    }
    for (n, w) in wires.iter_mut().enumerate() {
        let mut k = 0;
        w.retain(|_| {
            k += 1;
            !gone.contains(&(n, k - 1))
        });
    }
}

/// Every net pair whose geometry touches on one conductor (a cut counts on both
/// layers it joins), and every net touching, on a shared conductor, a `foreign`
/// cell or ring shape not owned by it. Candidates come from [`ShapeIndex`]es
/// of `side` nm buckets.
fn cross_net_shorts(wires: &[Vec<Shape>], joins: &[Join], foreign: &[(Option<u32>, Shape)], side: i32) -> (Vec<(usize, usize)>, Vec<usize>) {
    let footprint = |s: &Shape| -> Vec<(LayerId, Rect)> {
        match joins.iter().find(|j| j.0 == s.layer) {
            Some(&(_, lo, hi)) => vec![(lo, s.rect), (hi, s.rect)],
            None => vec![(s.layer, s.rect)],
        }
    };
    let nets: Vec<(usize, LayerId, Rect)> = wires.iter().enumerate().flat_map(|(n, w)| w.iter().flat_map(footprint).map(move |(l, r)| (n, l, r))).collect();
    let idx = ShapeIndex::new(side, nets.iter().map(|&(_, l, r)| (l, r)));
    let mut pairs = Vec::new();
    for &(a, l, r) in &nets {
        for k in idx.near(l, r, 0) {
            let (b, lb, rb) = nets[k as usize];
            if b > a && lb == l && rect_gap(r, rb) == 0 {
                pairs.push((a, b));
            }
        }
    }
    pairs.sort_unstable();
    pairs.dedup();
    // Only the conductors the routes draw on can be touched.
    let drawn: HashSet<LayerId> = nets.iter().map(|f| f.1).collect();
    let cell: Vec<(Option<u32>, LayerId, Rect)> = foreign
        .iter()
        .flat_map(|(o, s)| footprint(s).into_iter().map(move |(l, r)| (*o, l, r)))
        .filter(|f| drawn.contains(&f.1))
        .collect();
    let cells = ShapeIndex::new(side, cell.iter().map(|&(_, l, r)| (l, r)));
    let mut to_cell: Vec<usize> = nets
        .iter()
        .filter(|&&(n, l, r)| cells.near(l, r, 0).any(|k| {
            let (o, lc, rc) = cell[k as usize];
            o != Some(n as u32) && l == lc && rect_gap(r, rc) == 0
        }))
        .map(|f| f.0)
        .collect();
    to_cell.dedup();
    (pairs, to_cell)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// sky130-like lattice: the unit tests draw fixed geometry against it.
    fn test_cfg() -> DetailedCfg {
        DetailedCfg { pitch: 430, grid: 5, wire_width: 290, ..DetailedCfg::default() }
    }

    /// LAYERS joined by CUTS' layer: `cell_metal` attributes cell and ring
    /// metal only with a stack; without one, every such shape blocks. Sheet
    /// resistances (sky130-like) let `net_flow` split current for EM.
    fn test_stack() -> &'static analog::routing::Stack {
        use analog::routing::{stack::Layer, Stack};
        Box::leak(Box::new(Stack {
            layers: [0, 2, 1].map(|id| Layer { id, cut: id == 2, sheet_ohm: if id == 2 { 4.5 } else { 0.125 }, ..Layer::default() }).to_vec(),
            antenna_cumulative: false,
            diode: None,
        }))
    }

    const LAYERS: [LayerId; 2] = [LayerId(0), LayerId(1)];
    const CUTS: [Cut; 1] = [(LayerId(2), 100, 140, 140)];

    fn pin(net: u16, x: i32, y: i32) -> (NetId, Rect, LayerId) {
        (NetId(net), Rect { x, y, w: 170, h: 170 }, LAYERS[0])
    }

    fn touches(s: &Shape, r: Rect) -> bool {
        s.rect.touches(&r)
    }

    fn route(
        cfg: DetailedCfg,
        pins: &[(NetId, Rect, LayerId)],
        placed: &[Macro],
        rings: &[Macro],
        neg: &mut gr::Negotiation,
    ) -> (Routes, Report) {
        let reqs = Requirements::<Routes>::default();
        let (routes, report, _) = DetailedRoute { cfg }.route(pins, placed, rings, &reqs, &LAYERS, &CUTS, neg);
        (routes, report)
    }

    /// The hard EM rule on `net` with `cfg.em`'s limits over [`test_stack`].
    fn em_rule(cfg: &DetailedCfg, net: u16) -> analog::routing::Electromigration {
        let mut limits = [(u16::MAX, analog::routing::em::Limit::default()); analog::routing::em::MAX_LAYERS];
        for (slot, &(l, lim)) in limits.iter_mut().zip(&cfg.em) {
            // A cut is limited per cut only (`Pdk::em_limit`).
            *slot = (l.0, if CUTS.iter().any(|c| c.0 == l) { analog::routing::em::Limit { ua_per_um: 0.0, ..lim } } else { lim });
        }
        analog::routing::Electromigration { net: NetId(net), limits, stack: Some(test_stack()), front_row: cfg.em_front_row }
    }

    fn rules(r: &Report) -> Vec<&String> {
        r.hard_violations.iter().map(|v| &v.rule).collect()
    }

    #[test]
    fn detailed_realises_a_two_pin_net() {
        let pins = [pin(0, 1_000, 1_000), pin(0, 18_000, 18_000)];
        let (routes, report) = route(test_cfg(), &pins, &[], &[], &mut gr::Negotiation::new());
        assert_eq!(routes.wires.len(), 1);
        assert!(!routes.wires[0].is_empty());
        assert!(report.hard_violations.is_empty(), "{:?}", rules(&report));
        assert!(routes.wires[0].iter().all(|s| s.rect.w > 0 && s.rect.h > 0));
    }

    #[test]
    fn no_pins_give_empty_routes() {
        let cfg = DetailedCfg { n_nets: 3, ..test_cfg() };
        let (routes, report) = route(cfg, &[], &[], &[], &mut gr::Negotiation::new());
        assert_eq!(routes.wires.len(), 3);
        assert!(routes.wires.iter().all(Vec::is_empty));
        assert!(report.hard_violations.is_empty());
    }

    /// The frame follows the pins wherever they are, negative coordinates included.
    #[test]
    fn negative_coordinate_pins_route() {
        let pins = [pin(0, -20_000, -5_000), pin(0, 5_000, 3_000)];
        let (routes, report) = route(test_cfg(), &pins, &[], &[], &mut gr::Negotiation::new());
        assert!(!routes.wires[0].is_empty());
        assert!(report.hard_violations.is_empty(), "{:?}", rules(&report));
    }

    /// Every pin rect is touched by its own net, with pins deliberately centred a
    /// half-pitch off every track.
    #[test]
    fn every_pin_is_touched_by_its_own_net() {
        let pins = [pin(0, 1_805, 1_805), pin(0, 15_035, 13_145), pin(1, 3_695, 15_035), pin(1, 13_145, 3_695)];
        let cfg = DetailedCfg { pitch: 1_890, ..test_cfg() };
        let (routes, _) = route(cfg, &pins, &[], &[], &mut gr::Negotiation::new());
        for &(net, r, _) in &pins {
            let hit = routes.wires[net.0 as usize].iter().any(|s| touches(s, r));
            assert!(hit, "net {} pin at ({}, {}) untouched", net.0, r.x, r.y);
        }
    }

    /// A ring is an obstacle and a target: its pins get routed through their
    /// own stitch nodes, and with a stack `cell_metal` gives its bands to its
    /// net, so a route over them is no short to cell metal.
    #[test]
    fn ring_pins_are_routed() {
        let band = |x, y, w, h| Shape { layer: LayerId(0), rect: Rect { x, y, w, h } };
        let ring_pin = |x, y| pnr_core::geom::Pin {
            name: "ring".into(),
            net: NetId(0),
            at: Rect { x, y, w: 170, h: 170 },
            layer: LayerId(0),
        };
        let ring = Macro {
            bbox: Rect { x: 4_000, y: 4_000, w: 12_000, h: 12_000 },
            shapes: vec![band(4_000, 4_000, 12_000, 800), band(4_000, 15_200, 12_000, 800)],
            pins: vec![ring_pin(4_431, 4_207), ring_pin(15_113, 15_411)],
            units: Vec::new(),
            dummies: Vec::new(),
            ..Default::default()
        };
        let (routes, report) =
            route(DetailedCfg { stack: Some(test_stack()), ..test_cfg() }, &[], &[], &[ring.clone()], &mut gr::Negotiation::new());
        for p in &ring.pins {
            assert!(routes.wires[0].iter().any(|s| touches(s, p.at)), "ring pin at ({}, {}) untouched", p.at.x, p.at.y);
        }
        assert!(report.budget_violations.is_empty());
        assert!(report.hard_violations.is_empty(), "the ring's own net on its own band: {:?}", rules(&report));
    }

    /// Placed cells do not block routing under their (possibly inflated) bbox.
    #[test]
    fn inflated_bbox_does_not_block_routing() {
        let pins = [pin(0, 1_000, 7_000), pin(0, 15_000, 7_000)];
        let wall = Macro {
            shapes: vec![Shape { layer: LayerId(0), rect: Rect { x: 7_000, y: 200, w: 500, h: 500 } }],
            pins: Vec::new(),
            bbox: Rect { x: 6_000, y: -5_000, w: 3_000, h: 30_000 },
            units: Vec::new(),
            dummies: Vec::new(),
            ..Default::default()
        };
        let (routes, report) =
            route(test_cfg(), &pins, &[wall], &[], &mut gr::Negotiation::new());
        assert!(!rules(&report).iter().any(|r| r.starts_with("open net")), "{:?}", rules(&report));
        for &(_, r, _) in &pins {
            assert!(routes.wires[0].iter().any(|s| touches(s, r)));
        }
    }

    /// `score` reads `reqs.budget` on `gr`'s scale.
    #[test]
    fn budget_residual_reaches_theta() {
        let routes =
            Routes { wires: vec![vec![Shape { layer: LayerId(0), rect: Rect { x: 0, y: 0, w: 1_500, h: 1 } }]], ..Default::default()  };
        let mut reqs = Requirements::<Routes>::default();
        reqs.budget.push(Box::new(vec![analog::routing::ParasiticBudget {
            net: NetId(0),
            max_len_nm: 1_000,
            max_c_af: 0,
            margin_pct: 10,
            stack: None,
        }]));
        let report = score(&routes, &reqs, 0.0, &[], &[], &[], 1_000);
        assert_eq!(report.budget_violations[0].margin, 500);
    }

    /// A shared source fed beside member A: with its common node declared,
    /// the router rebalances the branches (R to A vs R to B) toward equal.
    #[test]
    fn a_skewed_common_node_is_rebalanced() {
        use analog::routing::{stack::Layer, CommonNode, CommonNodes, Stack};
        use analog::RuleBatch;
        let stack: &'static Stack = Box::leak(Box::new(Stack {
            layers: vec![
                Layer { id: 0, sheet_ohm: 0.125, ..Layer::default() },
                Layer { id: 2, sheet_ohm: 4.5, cut: true, ..Layer::default() },
                Layer { id: 1, sheet_ohm: 0.125, ..Layer::default() },
            ],
            antenna_cumulative: false,
            diode: None,
        }));
        let (feed, a, b) = (pin(0, 1_000, 2_000), pin(0, 12_000, 2_000), pin(0, 12_000, 14_000));
        let node = CommonNode { net: NetId(0), groups: vec![vec![a.1], vec![b.1]], feeds: vec![feed.1], max_delta_ohm: 0.2, star: false };
        let skew = |cfg: DetailedCfg| {
            let (routes, _) = route(cfg, &[feed, a, b], &[], &[], &mut gr::Negotiation::new());
            CommonNodes { nodes: vec![node.clone()], stack, halo_nm: 420, joins: joins(&LAYERS, &CUTS, None) }.worst_usage(&routes).unwrap()
        };
        let plain = skew(test_cfg());
        let balanced = skew(DetailedCfg { common: vec![node.clone()], stack: Some(stack), ..test_cfg() });
        assert!(balanced < plain * 0.5, "skew {plain} → {balanced}");
    }

    /// A matched cell's soft blockage is priced for foreign nets only: net 0,
    /// straight across the cell otherwise, goes around it; net 1, with a pin
    /// in the cell, still runs over it.
    #[test]
    fn foreign_nets_route_around_a_matched_cell() {
        let cell = Rect { x: 6_000, y: 3_000, w: 4_000, h: 4_000 };
        let matched = Macro {
            shapes: Vec::new(),
            pins: vec![pnr_core::Pin { name: "d0:D".into(), net: NetId(1), at: Rect { x: 8_000, y: 5_000, w: 1, h: 1 }, layer: LAYERS[0] }],
            bbox: cell,
            ..Default::default()
        };
        let pins = [pin(0, 1_000, 5_000), pin(0, 15_000, 5_000), pin(1, 8_000, 12_000)];
        let soft = Blockage { rect: cell, halo: 0, layers: 0b11, hard: false, own_exempt: true, only_aggressors: false, cell: 0, gate: false };
        let cfg = DetailedCfg { blockages: vec![soft], ..test_cfg() };
        let (routes, _) = route(cfg, &pins, &[matched], &[], &mut gr::Negotiation::new());
        let over = |n: usize| routes.wires[n].iter().any(|s| s.rect.x < cell.x + cell.w && cell.x < s.rect.x + s.rect.w && s.rect.y < cell.y + cell.h && cell.y < s.rect.y + s.rect.h);
        assert!(!over(0), "net 0 crossed the matched cell: {:?}", routes.wires[0]);
        assert!(over(1), "net 1 must reach its pin inside the cell");
    }

    /// Routed metal of `net` over `rect` on the test lattice's metals, µm².
    fn metal_over(routes: &Routes, net: usize, rect: Rect) -> f32 {
        let mut metals = [u16::MAX; analog::routing::metal_over_gate::MAX_METALS];
        (metals[0], metals[1]) = (LAYERS[0].0, LAYERS[1].0);
        let one = Routes { wires: vec![routes.wires[net].clone()], ..Default::default() };
        analog::routing::MetalOverGate { rect, cell: 0, metals }.overlap_um2(&one)
    }

    /// Four gate strips, 300 × 4 000 nm, 1 µm apart, in a 4 µm row across y = 5 000.
    fn gate_row(hard: bool) -> Vec<Blockage> {
        (0..4)
            .map(|i| Blockage { rect: Rect { x: 6_000 + i * 1_000, y: 3_000, w: 300, h: 4_000 }, halo: 0, layers: 0b11, hard, own_exempt: !hard, only_aggressors: false, cell: 0, gate: true })
            .collect()
    }

    /// A blank cell standing in for the matched transistor (blockage `cell 0`).
    fn blank_cell(pins: Vec<pnr_core::Pin>) -> Macro {
        Macro { bbox: Rect { x: 5_500, y: 3_000, w: 4_500, h: 4_000 }, pins, ..Default::default() }
    }

    /// Ring bands on both lattice layers over `rects`: no net passes them.
    fn walls(rects: &[Rect]) -> Macro {
        let shapes = rects.iter().flat_map(|&rect| LAYERS.map(|layer| Shape { layer, rect })).collect();
        Macro { shapes, bbox: rects[0], ..Default::default() }
    }

    /// Hard gate blockages (Moderate): a foreign net whose straight route
    /// crosses the row goes around it; no `metal over gate` row.
    #[test]
    fn no_route_crosses_a_moderately_matched_gate() {
        let pins = [pin(0, 1_000, 5_000), pin(0, 15_000, 5_000)];
        let cfg = DetailedCfg { blockages: gate_row(true), ..test_cfg() };
        let (routes, report) = route(cfg, &pins, &[blank_cell(Vec::new())], &[], &mut gr::Negotiation::new());
        assert!(!rules(&report).iter().any(|r| r.starts_with("open net") || r.starts_with("metal over gate")), "{:?}", rules(&report));
        for b in gate_row(true) {
            assert_eq!(metal_over(&routes, 0, b.rect), 0.0, "{:?}", b.rect);
        }
    }

    /// At Moderate the cell's own drain may not cross its gates either
    /// (`own_exempt = false`): pins either side of one gate join around it.
    #[test]
    fn own_drain_does_not_cross_its_gates_at_mod() {
        let at = |x| pnr_core::Pin { name: "d0:D".into(), net: NetId(0), at: Rect { x, y: 4_915, w: 170, h: 170 }, layer: LAYERS[0] };
        let gate = Blockage { rect: Rect { x: 7_000, y: 3_000, w: 300, h: 4_000 }, halo: 0, layers: 0b11, hard: true, own_exempt: false, only_aggressors: false, cell: 0, gate: true };
        let cfg = DetailedCfg { blockages: vec![gate], ..test_cfg() };
        let (routes, report) = route(cfg, &[], &[blank_cell(vec![at(6_000), at(8_200)])], &[], &mut gr::Negotiation::new());
        assert!(!routes.wires[0].is_empty());
        assert!(!rules(&report).iter().any(|r| r.starts_with("open net") || r.starts_with("metal over gate")), "{:?}", rules(&report));
        assert_eq!(metal_over(&routes, 0, gate.rect), 0.0);
    }

    /// Minimal class: soft gate blockages. With the detour walled off far
    /// out, crossing at `KEEPOUT_COST` a node is cheaper: the route crosses
    /// and the gate check reports it; the hard set-up detours instead.
    #[test]
    fn a_min_class_pair_allows_crossing_at_a_cost() {
        let pins = [pin(0, 1_000, 5_000), pin(0, 15_000, 5_000)];
        let ring = walls(&[Rect { x: 5_500, y: 7_000, w: 4_500, h: 30_000 }, Rect { x: 5_500, y: -27_000, w: 4_500, h: 30_000 }]);
        let run = |hard: bool| {
            let cfg = DetailedCfg { blockages: gate_row(hard), ..test_cfg() };
            route(cfg, &pins, &[blank_cell(Vec::new())], &[ring.clone()], &mut gr::Negotiation::new())
        };
        let (routes, report) = run(false);
        let over: f32 = gate_row(false).iter().map(|b| metal_over(&routes, 0, b.rect)).sum();
        assert!(over > 0.0, "the soft route detours: {:?}", rules(&report));
        assert!(rules(&report).iter().any(|r| r.starts_with("metal over gate")), "{:?}", rules(&report));
        assert!(!rules(&report).iter().any(|r| r.starts_with("open net")), "{:?}", rules(&report));
        let (routes, report) = run(true);
        let over: f32 = gate_row(true).iter().map(|b| metal_over(&routes, 0, b.rect)).sum();
        assert_eq!(over, 0.0);
        assert!(!rules(&report).iter().any(|r| r.starts_with("metal over gate")), "{:?}", rules(&report));
    }

    /// A Minimal resistor body: soft for every foreign net plus a hard copy
    /// for aggressors. Walls leave only the body and a head gap: the signal
    /// (net 0) crosses the body, the clock (net 1) the head.
    #[test]
    fn a_clock_never_crosses_a_resistor_body() {
        let body = Rect { x: 7_000, y: 3_000, w: 1_000, h: 6_000 };
        let soft = Blockage { rect: body, halo: 0, layers: 0b11, hard: false, own_exempt: true, only_aggressors: false, cell: 0, gate: false };
        let hard = Blockage { hard: true, only_aggressors: true, ..soft };
        let ring = walls(&[Rect { x: 7_000, y: 11_000, w: 1_000, h: 30_000 }, Rect { x: 7_000, y: -27_000, w: 1_000, h: 30_000 }]);
        let pins = [pin(0, 1_000, 5_000), pin(0, 15_000, 5_000), pin(1, 1_000, 7_000), pin(1, 15_000, 7_000)];
        let cfg = DetailedCfg { blockages: vec![soft, hard], aggressor: vec![false, true], ..test_cfg() };
        let cell = Macro { bbox: body, ..Default::default() };
        let (routes, report) = route(cfg, &pins, &[cell], &[ring], &mut gr::Negotiation::new());
        assert!(!rules(&report).iter().any(|r| r.starts_with("open net")), "{:?}", rules(&report));
        assert!(metal_over(&routes, 0, body) > 0.0, "the signal crosses the body");
        assert_eq!(metal_over(&routes, 1, body), 0.0, "the clock crosses a head");
    }

    /// Minimum same-layer edge gap between two nets' shapes, nm.
    fn min_gap(r: &Routes, a: usize, b: usize) -> i32 {
        r.wires[a].iter().flat_map(|p| r.wires[b].iter().filter(move |q| q.layer == p.layer).map(move |q| rect_gap(p.rect, q.rect))).min().unwrap_or(i32::MAX)
    }

    /// A hard `CrosstalkExclusion` of 1 000 nm is kept in the search: net 1's
    /// pins sit two pitches above net 0's straight line, so one of them
    /// leaves the other's tracks; the rule holds and no same-layer gap is
    /// under 1 000 nm.
    #[test]
    fn separated_nets_never_share_adjacent_tracks() {
        use analog::routing::CrosstalkExclusion;
        use analog::Rule;
        let rule = CrosstalkExclusion { a: NetId(0), b: NetId(1), min_spacing_nm: 1_000, margin_pct: 0 };
        let mut reqs = Requirements::<Routes>::default();
        reqs.hard.push(Box::new(vec![rule]));
        let pins = [pin(0, 1_000, 5_000), pin(0, 15_000, 5_000), pin(1, 4_000, 5_860), pin(1, 12_000, 5_860)];
        let (routes, report, _) = DetailedRoute { cfg: test_cfg() }.route(&pins, &[], &[], &reqs, &LAYERS, &CUTS, &mut gr::Negotiation::new());
        assert!(!rules(&report).iter().any(|r| r.starts_with("open net")), "{:?}", rules(&report));
        assert!(rule.satisfied(&routes), "gap {}", min_gap(&routes, 0, 1));
        assert!(min_gap(&routes, 0, 1) >= 1_000, "{}", min_gap(&routes, 0, 1));
        // Without the rule the straight runs sit 570 nm apart.
        let (plain, _, _) = DetailedRoute { cfg: test_cfg() }.route(&pins, &[], &[], &Requirements::default(), &LAYERS, &CUTS, &mut gr::Negotiation::new());
        assert!(min_gap(&plain, 0, 1) < 1_000, "the set-up does not force the pair together");
    }

    /// A no-cross separation: net 1's vertical run would cross net 0's
    /// horizontal one; it goes around net 0's end instead, and no shape of
    /// one overlaps the other's in xy on the adjacent layer.
    #[test]
    fn a_no_cross_pair_never_stacks() {
        struct NoCross;
        impl analog::RuleBatch<Routes> for NoCross {
            fn cost(&self, _: &Routes) -> f32 {
                0.0
            }
            fn violations(&self, _: &Routes) -> u32 {
                0
            }
            fn separations(&self, out: &mut Vec<(u32, u32, i32, bool)>) {
                out.push((0, 1, 0, true));
            }
        }
        let mut reqs = Requirements::<Routes>::default();
        reqs.hard.push(Box::new(NoCross));
        let pins = [pin(0, 6_000, 6_000), pin(0, 10_000, 6_000), pin(1, 8_000, 1_000), pin(1, 8_000, 11_000)];
        let route = |reqs: &Requirements<Routes>| DetailedRoute { cfg: test_cfg() }.route(&pins, &[], &[], reqs, &LAYERS, &CUTS, &mut gr::Negotiation::new());
        let stacked = |r: &Routes| r.wires[0].iter().filter(|p| LAYERS.contains(&p.layer)).any(|p| r.wires[1].iter().any(|q| q.layer != p.layer && LAYERS.contains(&q.layer) && overlaps(p.rect, q.rect)));
        let (routes, report, _) = route(&reqs);
        assert!(!rules(&report).iter().any(|r| r.starts_with("open net")), "{:?}", rules(&report));
        assert!(!stacked(&routes));
        assert!(stacked(&route(&Requirements::default()).0), "the set-up does not force a crossing");
    }

    /// RTE-15 set-up: p0 = 420, axis x = 10 290 (a track centre), nets 0/1
    /// under a `Differential` (no length tolerance); net 1's pins are net
    /// 0's mirrored, its second pin shifted by `off` nm.
    fn pair_run(off: i32, rings: &[Macro]) -> (Routes, Report, RouteStats, Requirements<Routes>) {
        use analog::routing::Differential;
        let cfg = DetailedCfg { pitch: 420, grid: 5, wire_width: 260, ..test_cfg() };
        let mut reqs = Requirements::<Routes>::default();
        reqs.budget.push(Box::new(vec![Differential { pos: NetId(0), neg: NetId(1), max_len_delta_pct10: 0, same_layer_required: true, stack: Some(test_stack()), aggressor_weight: None }]));
        let a = [(6_000, 3_000), (8_000, 9_000), (5_000, 12_000)];
        let mut pins: Vec<(NetId, Rect, LayerId)> = a.iter().map(|&(x, y)| pin(0, x, y)).collect();
        pins.extend(a.iter().enumerate().map(|(i, &(x, y))| pin(1, 2 * 10_290 - x - 170 + if i == 1 { off } else { 0 }, y)));
        let (routes, report, stats) = DetailedRoute { cfg }.route(&pins, &[], rings, &reqs, &LAYERS, &CUTS, &mut gr::Negotiation::new());
        (routes, report, stats, reqs)
    }

    /// Per layer: (Σ long side, Σ area, shape count) of `net`.
    fn signature(r: &Routes, net: usize) -> Vec<(u16, i64, i64, usize)> {
        let mut by: std::collections::BTreeMap<u16, (i64, i64, usize)> = std::collections::BTreeMap::new();
        for s in &r.wires[net] {
            let e = by.entry(s.layer.0).or_default();
            *e = (e.0 + i64::from(s.rect.w.max(s.rect.h)), e.1 + i64::from(s.rect.w) * i64::from(s.rect.h), e.2 + 1);
        }
        by.into_iter().map(|(l, (a, b, c))| (l, a, b, c)).collect()
    }

    /// Mirrored pins route as exact images: an obstacle on net 0's side only
    /// makes both detour; equal per-layer signatures, every net-1 shape a
    /// net-0 shape mirrored about 10 290, Differential residual 0.
    #[test]
    fn mirrored_pins_route_as_exact_mirrors() {
        let ring = Macro {
            shapes: LAYERS.map(|layer| Shape { layer, rect: Rect { x: 5_500, y: 5_500, w: 3_500, h: 1_000 } }).to_vec(),
            bbox: Rect { x: 5_500, y: 5_500, w: 3_500, h: 1_000 },
            ..Default::default()
        };
        let (routes, report, stats, reqs) = pair_run(0, &[ring]);
        assert_eq!(stats.pairs_exact.len(), 1, "{:?}", stats.pairs_fallback);
        assert!(!rules(&report).iter().any(|r| r.starts_with("open net")), "{:?}", rules(&report));
        assert_eq!(signature(&routes, 0), signature(&routes, 1));
        for s in &routes.wires[1] {
            let m = Rect { x: 2 * 10_290 - s.rect.x - s.rect.w, ..s.rect };
            assert!(routes.wires[0].iter().any(|t| t.layer == s.layer && t.rect == m), "net 1 {s:?} has no mirror in net 0: {:?} vs {:?}", routes.wires[0].iter().filter(|t| t.layer == s.layer).collect::<Vec<_>>(), routes.wires[1].iter().filter(|t| t.layer == s.layer).collect::<Vec<_>>());
        }
        assert_eq!(reqs.budget[0].residual(&routes), 0.0);
    }

    /// Without the obstacle neither net uses the axis track on the vertical layer.
    #[test]
    fn the_axis_track_is_never_used_by_a_pair() {
        let (routes, _, stats, _) = pair_run(0, &[]);
        assert_eq!(stats.pairs_exact.len(), 1, "{:?}", stats.pairs_fallback);
        for n in 0..2 {
            assert!(!routes.wires[n].iter().any(|s| s.layer == LAYERS[1] && s.rect.x <= 10_290 && 10_290 <= s.rect.x + s.rect.w), "net {n} covers the axis");
        }
    }

    /// One of net 1's pins one grid step (5 nm) off the mirror (all of them
    /// off would be an exact mirror about an off-lattice axis): no map, both nets
    /// route alone (guide fallback), and no metal shape is a dead end: each
    /// one off every pin touches at least two other shapes of its net.
    #[test]
    fn unmatched_pins_fall_back_to_the_guide() {
        let (routes, report, stats, _) = pair_run(5, &[]);
        assert!(stats.pairs_exact.is_empty());
        assert_eq!(stats.pairs_fallback, vec![(0, 1, "no pin map")]);
        assert!(!rules(&report).iter().any(|r| r.starts_with("open net")), "{:?}", rules(&report));
        for n in 0..2 {
            let w = &routes.wires[n];
            for (i, s) in w.iter().enumerate().filter(|(_, s)| s.layer == LAYERS[0] || s.layer == LAYERS[1]) {
                let touches = w.iter().enumerate().filter(|&(j, t)| j != i && rect_gap(s.rect, t.rect) == 0 && (t.layer == s.layer || t.layer == CUTS[0].0)).count();
                let at_pin = w.iter().any(|t| t.layer != s.layer && t.layer != CUTS[0].0 && rect_gap(s.rect, t.rect) == 0) || touches >= 2;
                assert!(at_pin, "net {n}: dead-end shape {s:?}");
            }
        }
    }

    /// Kelvin: force and sense groups of one net, the tap the feed, a star.
    /// The sense lead starts at the tap: every shape of its branch (the
    /// piece holding the sense pin once the tap's halo is cut out) carries
    /// 0 µA by `net_flow`, and the star check passes. Without the star the
    /// sense pin joins the force trunk, whose current then flows through it.
    #[test]
    fn a_kelvin_sense_branch_starts_at_the_tap() {
        use analog::routing::{current::net_flow, CommonNode, CommonNodes};
        use analog::RuleBatch;
        let (tap, force, sense) = (pin(0, 8_000, 2_000), pin(0, 8_000, 14_000), pin(0, 9_000, 14_000));
        let node = |star| CommonNode { net: NetId(0), groups: vec![vec![force.1], vec![sense.1]], feeds: vec![tap.1], max_delta_ohm: 0.0, star };
        let run = |star: bool| {
            let cfg = DetailedCfg { common: vec![node(star)], stack: Some(test_stack()), ..test_cfg() };
            let (routes, report) = route(cfg, &[tap, force, sense], &[], &[], &mut gr::Negotiation::new());
            assert!(!rules(&report).iter().any(|r| r.starts_with("open net") || r.starts_with("star root")), "{:?}", rules(&report));
            let terms = [(tap.1, -1_000.0), (force.1, 1_000.0), (sense.1, 0.0)].map(|(at, ua)| pnr_core::Terminal { at, ua: Some(ua) });
            let flow = net_flow(test_stack(), &routes.wires[0], &terms).expect("flow");
            // The sense branch: the tap's halo cut out, flood from the sense pin.
            let p = test_cfg().pitch;
            let halo = Rect { x: tap.1.x - p, y: tap.1.y - p, w: tap.1.w + 2 * p, h: tap.1.h + 2 * p };
            let w = &routes.wires[0];
            let keep: Vec<usize> = (0..w.len()).filter(|&i| !w[i].rect.touches(&halo)).collect();
            let j = joins(&LAYERS, &CUTS, None);
            let mut branch: Vec<usize> = keep.iter().copied().filter(|&i| w[i].rect.touches(&sense.1)).collect();
            let mut k = 0;
            while k < branch.len() {
                let a = branch[k];
                for &b in &keep {
                    if !branch.contains(&b) && w[a].rect.touches(&w[b].rect) && conductor_layers_meet(&w[a], &w[b], &j) {
                        branch.push(b);
                    }
                }
                k += 1;
            }
            let star_v = CommonNodes { nodes: vec![node(true)], stack: test_stack(), halo_nm: p, joins: j }.violations(&routes);
            (branch.iter().map(|&i| flow.shape_ua[i]).fold(0.0f32, f32::max), star_v, branch.is_empty())
        };
        let (ua, star_v, empty) = run(true);
        assert!(!empty, "the sense pin has a branch");
        assert_eq!(ua, 0.0, "force current in the sense lead");
        assert_eq!(star_v, 0);
        let (ua, star_v, _) = run(false);
        assert!(ua > 0.0 || star_v > 0, "without the star the sense lead is not separate");
    }

    /// Bits of 1/2/4/8 units with equal 10 µm leads (ground C only): the
    /// stubs bring every bit's lead C per unit within 10 aF of the mean.
    #[test]
    fn equalize_leads_matches_per_unit_lead_c() {
        use analog::routing::{stack::Layer, PlateRatio, PlateSet, Stack};
        let stack: &'static Stack = Box::leak(Box::new(Stack { layers: vec![Layer { id: LAYERS[1].0, area_af_um2: 30.0, lateral: 1e-9, ..Layer::default() }], antenna_cumulative: false, diode: None }));
        let set = PlateSet { top: NetId(0), bits: [1, 2, 4, 8].iter().enumerate().map(|(i, &n)| (NetId(i as u16 + 1), n)).collect(), c_unit_af: 1_000.0, array: Rect { x: 0, y: 0, w: 100_000, h: 1_000 } };
        let rule = PlateRatio { set, tol_pct10: 10, stack, space_nm: 140 };
        let mut wires = vec![Vec::new()];
        for i in 0..4 {
            wires.push(vec![Shape { layer: LAYERS[1], rect: Rect { x: i * 20_000, y: 1_000, w: 260, h: 10_000 } }]);
        }
        let mut r = Routes { wires, ..Default::default() };
        assert!(rule.spread_pct(&r).unwrap() > 1.0);
        equalize_leads(&mut r, &rule, &LAYERS, &[], &|_| 140, 5);
        let per = rule.per_unit_af(&r);
        let mean = per.iter().sum::<f32>() / 4.0;
        assert!(per.iter().all(|c| (c - mean).abs() <= 10.0), "{per:?}");
    }

    /// A 4-bit bank: the top plate (net 0) leaves the array downward, the
    /// bits (nets 1–4) upward, so they would cross; the no-cross separation
    /// keeps every bit off the top on adjacent layers and the top a spacing
    /// away on its own.
    #[test]
    fn bottom_plates_never_cross_the_top_plate() {
        use analog::routing::PlateSet;
        let array = Rect { x: 8_000, y: 8_000, w: 5_000, h: 4_000 };
        let at = |net: u16, x, y| pnr_core::Pin { name: format!("c{net}"), net: NetId(net), at: Rect { x, y, w: 170, h: 170 }, layer: LAYERS[0] };
        let cell = Macro { pins: [at(0, 10_400, 11_600)].into_iter().chain((1..5).map(|i| at(i, 8_400 + i32::from(i) * 900, 8_200))).collect(), bbox: array, ..Default::default() };
        let mut pins = vec![pin(0, 10_400, 2_000)];
        pins.extend((1..5).map(|i| pin(i, 8_400 + i32::from(i) * 900, 18_000)));
        let set = PlateSet { top: NetId(0), bits: (1..5).map(|i| (NetId(i), 1)).collect(), c_unit_af: f32::NAN, array };
        let run = |plates: Vec<PlateSet>| {
            let cfg = DetailedCfg { plates, stack: Some(test_stack()), ..test_cfg() };
            route(cfg, &pins, &[cell.clone()], &[], &mut gr::Negotiation::new())
        };
        let crossed = |r: &Routes| (1..5).any(|b| r.wires[b].iter().any(|p| r.wires[0].iter().any(|q| p.layer != q.layer && LAYERS.contains(&p.layer) && LAYERS.contains(&q.layer) && overlaps(p.rect, q.rect))));
        let (routes, report) = run(vec![set]);
        assert!(!rules(&report).iter().any(|r| r.starts_with("open net") || r.starts_with("plate crossing")), "{:?}", rules(&report));
        assert!(!crossed(&routes));
        assert!((1..5).all(|b| min_gap(&routes, 0, b) >= 140), "{:?}", (1..5).map(|b| min_gap(&routes, 0, b)).collect::<Vec<_>>());
        assert!(crossed(&run(Vec::new()).0), "the set-up does not force a crossing");
    }

    /// History survives the call and changes the next one.
    #[test]
    fn negotiation_persists_across_calls() {
        let cfg = DetailedCfg { pitch: 1_300, ..test_cfg() };
        // Six nets whose terminals all sit within one pitch of the same two rows
        // (five no longer jam once nets jog one row over, RTE-28):
        // every net wants the same horizontal track.
        let pins: Vec<(NetId, Rect, LayerId)> = (0..6u16)
            .flat_map(|n| {
                let off = i32::from(n) * 104;
                [
                    (NetId(n), Rect { x: 1_000, y: 7_000 + off, w: 1, h: 1 }, LAYERS[0]),
                    (NetId(n), Rect { x: 15_000, y: 7_000 + off, w: 1, h: 1 }, LAYERS[0]),
                ]
            })
            .collect();
        let flat = |r: &Routes| -> Vec<(u16, i32, i32, i32, i32)> {
            r.wires.iter().flatten().map(|s| (s.layer.0, s.rect.x, s.rect.y, s.rect.w, s.rect.h)).collect()
        };
        let mut neg = gr::Negotiation::new();
        let (first, report) = route(cfg.clone(), &pins, &[], &[], &mut neg);
        let jam = report.hard_violations.iter().find(|v| v.rule == "unresolved congestion");
        assert!(jam.is_some_and(|v| v.margin > 0), "residual overuse is V: {:?}", rules(&report));
        let p1 = neg.pressure();
        assert!(p1 > 0.0, "contested tracks must accumulate history");
        let second = route(cfg, &pins, &[], &[], &mut neg).0;
        assert!(neg.pressure() > p1, "history must keep climbing");
        assert_ne!(flat(&first), flat(&second), "second call was not seeded");
    }

    /// Width is sized per branch from its MST edge current, in whole tracks
    /// (`W = 290 + (k−1)·430`): a 1 mA source feeding 600 µA and 400 µA loads
    /// gets a 3-track trunk at the source (1 µm) and a 2-track stub at the
    /// lighter load (400 nm); a net with no current keeps `wire_width`.
    #[test]
    fn segments_are_sized_by_their_branch_current() {
        use analog::routing::em::Limit;
        let pins = [pin(0, 1_000, 1_000), pin(0, 12_000, 1_000), pin(0, 12_000, 9_000), pin(1, 1_000, 9_000), pin(1, 6_000, 9_000)];
        let names = ["S", "a", "b", "g1", "g2"];
        let cell_of = || Macro {
            shapes: Vec::new(),
            pins: pins.iter().zip(names).map(|(&(net, at, layer), n)| pnr_core::Pin { name: n.into(), net, at, layer }).collect(),
            bbox: Rect { x: 0, y: 0, w: 13_000, h: 10_000 },
            units: Vec::new(),
            dummies: Vec::new(),
            ..Default::default()
        };
        let lim = Limit { ua_per_um: 1_000.0, ua_per_cut: 10_000.0, blech: 0.0 };
        let cfg_of = || DetailedCfg {
            pin_ua: vec![vec![("S".into(), Some(-1_000)), ("a".into(), Some(600)), ("b".into(), Some(400))]],
            em: vec![(LAYERS[0], lim), (LAYERS[1], lim), (CUTS[0].0, lim)],
            ..test_cfg()
        };
        let (routes, report) = route(cfg_of(), &pins, &[cell_of()], &[], &mut gr::Negotiation::new());
        let trunks = |n: usize| -> Vec<i32> {
            routes.wires[n].iter().filter(|s| s.rect.w != s.rect.h && LAYERS.contains(&s.layer)).map(|s| s.rect.w.min(s.rect.h)).collect()
        };
        assert_eq!(trunks(0).iter().max(), Some(&1_150), "{:?}", trunks(0));
        assert!(trunks(0).contains(&720), "the 400 µA branch is not sized for 1 mA: {:?}", trunks(0));
        assert_eq!(trunks(1).iter().max(), Some(&290));
        assert!(report.budget_violations.is_empty(), "{:?}", report.budget_violations.iter().map(|v| &v.rule).collect::<Vec<_>>());
        // The final routes carry each pin and its current for the EM rule;
        // without an operating point every current is unknown, never 0.
        let terms = routes.terminals(NetId(0));
        assert_eq!(terms.iter().map(|t| (t.at, t.ua)).collect::<Vec<_>>(), [(pins[0].1, Some(-1_000.0)), (pins[1].1, Some(600.0)), (pins[2].1, Some(400.0))]);
        let (bare, _) = route(DetailedCfg { pin_ua: Vec::new(), ..cfg_of() }, &pins, &[cell_of()], &[], &mut gr::Negotiation::new());
        assert!(bare.terminals(NetId(0)).len() == 3 && bare.terminals(NetId(0)).iter().all(|t| t.ua.is_none()));
        // A 1 µA cut limit asks ~1000 cuts of the source's via: the hard EM
        // rule (the only EM measure, C18) says so.
        let starved = DetailedCfg { em: vec![(LAYERS[0], lim), (LAYERS[1], lim), (CUTS[0].0, Limit { ua_per_cut: 1.0, ..lim })], ..cfg_of() };
        let rule = em_rule(&starved, 0);
        let (routes, _) = route(starved, &pins, &[cell_of()], &[], &mut gr::Negotiation::new());
        use analog::Rule;
        assert!(rule.known(&routes) && !rule.satisfied(&routes));
    }

    /// Four fingers S D S D S (`pnr_core::pin_shares`' fixture): the two
    /// `d0:D` pins each carry half of the drain's 400 µA, so the trunk to the
    /// −400 µA sink is sized for 400 µA (2 tracks, 720 nm, at 1 mA/µm), not
    /// 800 (3 tracks, 1150 nm).
    #[test]
    fn a_multi_finger_terminal_is_not_counted_per_pin() {
        use analog::routing::em::Limit;
        let pins = [pin(0, 2_000 - 85, -85), pin(0, 6_000 - 85, -85), pin(0, 12_000, 9_000)];
        let cell = Macro {
            shapes: Vec::new(),
            pins: pins.iter().zip(["d0:D", "d0:D", "sink"]).map(|(&(net, at, layer), n)| pnr_core::Pin { name: n.into(), net, at, layer }).collect(),
            bbox: Rect { x: 0, y: -85, w: 13_000, h: 10_000 },
            units: [(1_000, 1), (3_000, -1), (5_000, 1), (7_000, -1)]
                .map(|(x, phi)| pnr_core::Unit { owner: 0, x, y: 0, weight: 1, phi: (phi, 0), sa: 0, sb: 0 })
                .to_vec(),
            dummies: Vec::new(),
            ..Default::default()
        };
        let lim = Limit { ua_per_um: 1_000.0, ua_per_cut: 10_000.0, blech: 0.0 };
        let cfg = DetailedCfg {
            pin_ua: vec![vec![("d0:D".into(), Some(400)), ("sink".into(), Some(-400))]],
            pin_share: vec![pnr_core::pin_shares(&cell)],
            em: vec![(LAYERS[0], lim), (LAYERS[1], lim), (CUTS[0].0, lim)],
            ..test_cfg()
        };
        let (routes, _) = route(cfg, &pins, &[cell], &[], &mut gr::Negotiation::new());
        let trunks: Vec<i32> = routes.wires[0].iter().filter(|s| s.rect.w != s.rect.h && LAYERS.contains(&s.layer)).map(|s| s.rect.w.min(s.rect.h)).collect();
        assert_eq!(trunks.iter().max(), Some(&720), "{trunks:?}");
    }

    /// Two members sharing a source region draw one pin each at one rect. A
    /// row A1 — A2 — sink: each of A1, A2 carries both members' halves (300
    /// µA), so the A1–A2 run carries 300 and A2–sink 600. Counting only the
    /// first pin at a rect gives 150 per rect and a 450 µA A1–A2 run.
    #[test]
    fn a_shared_region_carries_both_members_current() {
        use analog::routing::em::Limit;
        let pins = [pin(0, 1_000, 1_000), pin(0, 1_000, 1_000), pin(0, 6_000, 1_000), pin(0, 6_000, 1_000), pin(0, 12_000, 1_000)];
        let cell = Macro {
            shapes: Vec::new(),
            pins: pins.iter().zip(["d0:S", "d1:S", "d0:S", "d1:S", "t"]).map(|(&(net, at, layer), n)| pnr_core::Pin { name: n.into(), net, at, layer }).collect(),
            bbox: Rect { x: 0, y: 0, w: 13_000, h: 2_000 },
            units: Vec::new(),
            dummies: Vec::new(),
            ..Default::default()
        };
        let lim = Limit { ua_per_um: 1_000.0, ua_per_cut: 10_000.0, blech: 0.0 };
        let cfg = DetailedCfg {
            pin_ua: vec![vec![("d0:S".into(), Some(-300)), ("d1:S".into(), Some(-300)), ("t".into(), Some(600))]],
            pin_share: vec![vec![0.5, 0.5, 0.5, 0.5, 1.0]],
            em: vec![(LAYERS[0], lim), (LAYERS[1], lim), (CUTS[0].0, lim)],
            ..test_cfg()
        };
        let (routes, _) = route(cfg, &pins, &[cell], &[], &mut gr::Negotiation::new());
        let mut trunks: Vec<i32> = routes.wires[0].iter().filter(|s| s.rect.w != s.rect.h && LAYERS.contains(&s.layer)).map(|s| s.rect.w.min(s.rect.h)).collect();
        trunks.sort_unstable();
        trunks.dedup();
        assert!(trunks.contains(&300) && trunks.contains(&600) && !trunks.contains(&450), "{trunks:?}");
    }

    /// One unknown terminal leaves its whole net unsized: a known 1 mA on the
    /// other pin is not a branch current when the rest of the net is unknown.
    #[test]
    fn a_net_with_an_unknown_terminal_gets_no_em_sizing() {
        use analog::routing::em::Limit;
        let pins = [pin(0, 1_000, 1_000), pin(0, 12_000, 9_000)];
        let cell = Macro {
            shapes: Vec::new(),
            pins: pins.iter().zip(["S", "D"]).map(|(&(net, at, layer), n)| pnr_core::Pin { name: n.into(), net, at, layer }).collect(),
            bbox: Rect { x: 0, y: 0, w: 13_000, h: 10_000 },
            units: Vec::new(),
            dummies: Vec::new(),
            ..Default::default()
        };
        let lim = Limit { ua_per_um: 1_000.0, ua_per_cut: 10_000.0, blech: 0.0 };
        let run = |table: Vec<(String, Option<i32>)>| {
            let cfg = DetailedCfg { pin_ua: vec![table], em: vec![(LAYERS[0], lim), (LAYERS[1], lim), (CUTS[0].0, lim)], ..test_cfg() };
            let routes = route(cfg, &pins, std::slice::from_ref(&cell), &[], &mut gr::Negotiation::new()).0;
            routes.wires[0].iter().map(|s| (s.layer, s.rect)).collect::<Vec<_>>()
        };
        let widest = |r: &[(LayerId, Rect)]| r.iter().filter(|(l, r)| r.w != r.h && LAYERS.contains(l)).map(|(_, r)| r.w.min(r.h)).max();
        assert_eq!(widest(&run(vec![("S".into(), Some(-1_000)), ("D".into(), Some(1_000))])), Some(1_150), "known: 3 tracks for 1 mA");
        assert_eq!(run(vec![("S".into(), None), ("D".into(), Some(1_000))]), run(Vec::new()), "unknown: routed as with no currents at all");
    }

    /// A per-stage antenna rule the lower metal fails: the repair jumps the
    /// run up to the next same-direction metal (a jumper), which splits the
    /// lower stage's conductor, and the violation clears.
    #[test]
    fn antenna_repair_jumps_to_a_higher_metal() {
        use analog::routing::stack::{Layer, Stack};
        use analog::Rule;
        // Declares `RepairKind::Antenna` so repair takes the antenna arm;
        // checks the stack's per-stage ratio (m0's limit only).
        #[derive(Clone, Copy)]
        struct Antenna(&'static Stack);
        impl Rule for Antenna {
            type On = Routes;
            const REPAIR: RepairKind = RepairKind::Antenna;
            fn cost(self, r: &Routes) -> f32 {
                self.residual(r)
            }
            fn satisfied(self, r: &Routes) -> bool {
                self.0.antenna(r.shapes(NetId(0)), &[], &[], &[], 1_000_000).is_none_or(|(x, l)| x <= l)
            }
            fn residual(self, r: &Routes) -> f32 {
                self.0.antenna(r.shapes(NetId(0)), &[], &[], &[], 1_000_000).map_or(0.0, |(x, l)| (x / l - 1.0).max(0.0))
            }
            fn touches(self, out: &mut Vec<u32>) {
                out.push(0);
            }
        }
        let layers = [LayerId(0), LayerId(1), LayerId(3)];
        let cuts: [Cut; 2] = [(LayerId(2), 100, 140, 140), (LayerId(4), 100, 140, 140)];
        let stack: &'static Stack = Box::leak(Box::new(Stack {
            layers: [0, 2, 1, 4, 3].map(|id| Layer { id, antenna_ratio: if id == 0 { 4.0 } else { 0.0 }, ..Layer::default() }).to_vec(),
            antenna_cumulative: false,
        diode: None,
        }));
        let mut reqs = Requirements::<Routes>::default();
        reqs.hard.push(Box::new(vec![Antenna(stack)]));
        let pins = [pin(0, 1_000, 1_000), pin(0, 30_000, 1_000)];
        let route = |reqs: &Requirements<Routes>| DetailedRoute { cfg: test_cfg() }.route(&pins, &[], &[], reqs, &layers, &cuts, &mut gr::Negotiation::new());
        // Unconstrained, the straight run sits on m0: 29 µm × 0.29 µm ≫ 4 µm².
        let (free, _, _) = route(&Requirements::default());
        assert!(!Antenna(stack).satisfied(&free));
        let (fixed, report, _) = route(&reqs);
        assert!(Antenna(stack).satisfied(&fixed), "residual {}", Antenna(stack).residual(&fixed));
        assert!(fixed.wires[0].iter().any(|s| s.layer == LayerId(3)), "the run jumped to the upper metal");
        assert!(report.hard_violations.is_empty(), "{:?}", rules(&report));
    }

    /// A violated EM rule names its net, but no reroute changes width: repair
    /// runs no trial for it. The control (the same rule declaring `Reroute`)
    /// shows the count is live. The rule is a stand-in that always fails:
    /// the real `Electromigration` reads unknown on repair probes (no
    /// terminals), so it would never reach the dispatch this test checks.
    #[test]
    fn an_em_violation_spends_no_repair_trial() {
        use analog::Rule;
        #[derive(Clone, Copy)]
        struct Stuck<const EM: bool>;
        impl<const EM: bool> Rule for Stuck<EM> {
            type On = Routes;
            const REPAIR: RepairKind = if EM { RepairKind::Em } else { RepairKind::Reroute };
            fn cost(self, _: &Routes) -> f32 {
                1.0
            }
            fn satisfied(self, _: &Routes) -> bool {
                false
            }
            fn touches(self, out: &mut Vec<u32>) {
                out.push(0);
            }
        }
        let pins = [pin(0, 1_000, 1_000), pin(0, 30_000, 1_000)];
        let trials = |reqs: &Requirements<Routes>| {
            let dr = DetailedRoute { cfg: test_cfg() };
            let (routes, report, stats) = dr.route(&pins, &[], &[], reqs, &LAYERS, &CUTS, &mut gr::Negotiation::new());
            assert!(!routes.wires[0].is_empty(), "the net routed");
            assert_eq!(rules(&report).len(), 1, "the stuck rule stays violated: {:?}", rules(&report));
            stats.trials
        };
        let mut em = Requirements::<Routes>::default();
        em.hard.push(Box::new(vec![Stuck::<true>]));
        assert_eq!(trials(&em), 0);
        let mut reroute = Requirements::<Routes>::default();
        reroute.hard.push(Box::new(vec![Stuck::<false>]));
        assert!(trials(&reroute) > 0, "a Reroute rule on the same net does spend trials");
    }

    /// dr's rows are the measurement of the routes it returns (RTE-23):
    /// every hard batch has a row iff it is violated there, every budget
    /// batch iff its residual is positive, each with that residual's margin.
    #[test]
    fn the_report_is_the_measurement_of_the_drawn_routes() {
        use analog::routing::CouplingBudget;
        let check = |routes: &Routes, report: &Report, reqs: &Requirements<Routes>| {
            let row = |rows: &[Violation], tier: &str, i: usize| rows.iter().find(|v| v.rule == format!("{}routing {tier} {i}", Violation::BATCH)).map(|v| v.margin);
            for (i, b) in reqs.hard.iter().enumerate() {
                let want = (b.violations(routes) > 0).then(|| Violation::from_residual("", b.residual(routes)).margin);
                assert_eq!(row(&report.hard_violations, "hard", i), want, "hard {i}");
            }
            for (i, b) in reqs.budget.iter().enumerate() {
                let r = b.residual(routes);
                assert_eq!(row(&report.budget_violations, "budget", i), (r > 0.0).then(|| Violation::from_residual("", r).margin), "budget {i}");
            }
        };
        // A victim beside an aggressor, over budget.
        let pins = [pin(0, 1_000, 5_000), pin(0, 21_000, 5_000), pin(1, 1_000, 5_430), pin(1, 21_000, 5_430)];
        let mut reqs = Requirements::<Routes>::default();
        reqs.budget.push(Box::new(vec![CouplingBudget { net: NetId(0), max_coupling_af: 1, margin_pct: 0, stack: Some(test_stack()), exclude: None, aggressor_weight: None }]));
        let (routes, report, _) = DetailedRoute { cfg: test_cfg() }.route(&pins, &[], &[], &reqs, &LAYERS, &CUTS, &mut gr::Negotiation::new());
        assert!(reqs.budget[0].residual(&routes) > 0.0, "the coupling row is live");
        check(&routes, &report, &reqs);
        let (routes, report, shield) = shield_run();
        let mut reqs = Requirements::<Routes>::default();
        reqs.budget.push(Box::new(vec![shield]));
        check(&routes, &report, &reqs);
        let (routes, report, _, reqs) = pair_run(300, &[]);
        check(&routes, &report, &reqs);
    }

    /// Step 1: the repair probe draws pin access, so a hard rule its jogs
    /// break is violated before fill already: no post-fill round runs for it,
    /// and the row ships. The stand-in rule fails when a net-0 shape on any
    /// layer overlaps net 0's cell metal in plan (its pins, a half-pitch off
    /// every track: only the access jogs and fill reach them).
    #[test]
    fn a_rule_broken_by_pin_access_is_seen_by_repair() {
        use analog::Rule;
        #[derive(Clone, Copy)]
        struct Untouched;
        impl Rule for Untouched {
            type On = Routes;
            fn cost(self, _: &Routes) -> f32 {
                1.0
            }
            fn satisfied(self, r: &Routes) -> bool {
                !r.shapes(NetId(0)).iter().any(|s| r.cell_metal(NetId(0)).iter().any(|c| s.rect.touches(&c.rect)))
            }
            fn touches(self, out: &mut Vec<u32>) {
                out.push(0);
            }
        }
        let pins = [pin(0, 1_805, 1_805), pin(0, 15_035, 13_145), pin(1, 3_695, 15_035), pin(1, 13_145, 3_695)];
        let cell = Macro {
            shapes: pins.iter().map(|&(_, rect, layer)| Shape { layer, rect }).collect(),
            pins: pins.iter().enumerate().map(|(i, &(net, at, layer))| pnr_core::Pin { name: format!("P{i}"), net, at, layer }).collect(),
            bbox: Rect { x: 1_805, y: 1_805, w: 13_400, h: 13_400 },
            ..Default::default()
        };
        let cfg = DetailedCfg { pitch: 1_890, stack: Some(test_stack()), ..test_cfg() };
        let mut reqs = Requirements::<Routes>::default();
        reqs.hard.push(Box::new(vec![Untouched]));
        let (routes, report, stats) = DetailedRoute { cfg }.route(&pins, &[cell], &[], &reqs, &LAYERS, &CUTS, &mut gr::Negotiation::new());
        assert!(!Untouched.satisfied(&routes), "the jogs reach the pins");
        assert_eq!(stats.post_rounds, 0, "the probe saw the jogs");
        assert!(rules(&report).contains(&&format!("{}routing hard 0", Violation::BATCH)), "{:?}", rules(&report));
    }

    /// Step 4: a hard rule only the drawing breaks (the stand-in fails once
    /// terminals are attached, which probes never carry) gets one post-fill
    /// round; it cannot improve, so the round is rejected and the drawing is
    /// the one without the rule. A rule already broken on the probe gets none.
    #[test]
    fn a_rule_broken_after_fill_gets_a_post_round() {
        use analog::Rule;
        #[derive(Clone, Copy)]
        struct NoTerms<const EVER: bool>;
        impl<const EVER: bool> Rule for NoTerms<EVER> {
            type On = Routes;
            fn cost(self, _: &Routes) -> f32 {
                1.0
            }
            fn satisfied(self, r: &Routes) -> bool {
                EVER && r.terms.iter().all(Vec::is_empty)
            }
            fn touches(self, out: &mut Vec<u32>) {
                out.push(0);
            }
        }
        let pins = [pin(0, 1_000, 1_000), pin(0, 12_000, 9_000), pin(1, 1_000, 9_000), pin(1, 12_000, 1_000)];
        let run = |reqs: &Requirements<Routes>| DetailedRoute { cfg: test_cfg() }.route(&pins, &[], &[], reqs, &LAYERS, &CUTS, &mut gr::Negotiation::new());
        let (plain, _, _) = run(&Requirements::default());
        let mut reqs = Requirements::<Routes>::default();
        reqs.hard.push(Box::new(vec![NoTerms::<true>]));
        let (routes, report, stats) = run(&reqs);
        assert_eq!(stats.post_rounds, 1);
        assert_eq!(rules(&report), [&format!("{}routing hard 0", Violation::BATCH)]);
        assert_eq!(routes.wires, plain.wires, "the rejected round restored the drawing");
        let mut reqs = Requirements::<Routes>::default();
        reqs.hard.push(Box::new(vec![NoTerms::<false>]));
        assert_eq!(run(&reqs).2.post_rounds, 0, "broken on the probe: repair's, not a post round's");
    }

    /// Step 2: an unchanged local batch keeps its score, a non-local one is
    /// always rescored, and a local one whose net changed is rescored; the
    /// incremental scores equal a full rescore.
    #[test]
    fn rescore_reuses_only_unchanged_local_batches() {
        use analog::Rule;
        use std::sync::atomic::{AtomicU32, Ordering::SeqCst};
        static CALLS: [AtomicU32; 2] = [AtomicU32::new(0), AtomicU32::new(0)];
        #[derive(Clone, Copy)]
        struct Count<const L: bool>;
        impl<const L: bool> Rule for Count<L> {
            type On = Routes;
            const LOCAL: bool = L;
            fn cost(self, _: &Routes) -> f32 {
                1.0
            }
            fn satisfied(self, r: &Routes) -> bool {
                CALLS[usize::from(L)].fetch_add(1, SeqCst);
                r.wires[0].len() < 2
            }
            fn touches(self, out: &mut Vec<u32>) {
                out.push(0);
            }
        }
        let (local, global) = (vec![Count::<true>], vec![Count::<false>]);
        let b: [&dyn analog::RuleBatch<Routes>; 2] = [&local, &global];
        let w = |x| Shape { layer: LAYERS[0], rect: Rect { x, y: 0, w: 100, h: 100 } };
        let r = |a: Vec<Shape>, c: Vec<Shape>| Routes { wires: vec![a, c], ..Routes::default() };
        let (r1, r2, r3) = (r(vec![w(0)], vec![w(500)]), r(vec![w(0)], vec![w(900)]), r(vec![w(0), w(300)], vec![w(500)]));
        let n = |i: usize| CALLS[i].load(SeqCst);
        let s1 = rescore(&b, &r1, None);
        let (l0, g0) = (n(1), n(0));
        let inc = rescore(&b, &r2, Some((&r1, &s1)));
        assert_eq!(n(1), l0, "net 0 unchanged: the local batch kept its score");
        assert!(n(0) > g0, "the non-local batch is rescored");
        assert_eq!(inc, rescore(&b, &r2, None));
        let l1 = n(1);
        let s3 = rescore(&b, &r3, Some((&r1, &s1)));
        assert!(n(1) > l1, "net 0 changed: the local batch is rescored");
        assert_eq!(s3, rescore(&b, &r3, None));
        assert_eq!(s3[0].0, 1);
    }

    /// Step 3: on an exact pair the leader's fillers are mirrored onto its
    /// image. Net 0 has two pins 100 nm apart (a sliver its fill closes);
    /// without an obstacle both nets fill it and stay images. A foreign
    /// shape within spacing of the image's filler site only: the filler is
    /// dropped from both, so the pair stays shape-for-shape mirrored.
    #[test]
    fn fill_on_a_pair_is_mirrored() {
        use analog::routing::Differential;
        let cfg = DetailedCfg { pitch: 420, grid: 5, wire_width: 260, ..test_cfg() };
        let mut reqs = Requirements::<Routes>::default();
        reqs.budget.push(Box::new(vec![Differential { pos: NetId(0), neg: NetId(1), max_len_delta_pct10: 0, same_layer_required: true, stack: Some(test_stack()), aggressor_weight: None }]));
        let a = [(6_000, 3_000), (6_270, 3_000), (8_000, 9_000), (5_000, 12_000)];
        let mut pins: Vec<(NetId, Rect, LayerId)> = a.iter().map(|&(x, y)| pin(0, x, y)).collect();
        pins.extend(a.iter().map(|&(x, y)| pin(1, 2 * 10_290 - x - 170, y)));
        let gap = Rect { x: 6_170, y: 3_000, w: 100, h: 170 };
        let mirror = |r: Rect| Rect { x: 2 * 10_290 - r.x - r.w, ..r };
        let run = |rings: &[Macro]| DetailedRoute { cfg: cfg.clone() }.route(&pins, &[], rings, &reqs, &LAYERS, &CUTS, &mut gr::Negotiation::new());
        let imaged = |r: &Routes| {
            let set = |n: usize, f: &dyn Fn(Rect) -> Rect| {
                let mut v: Vec<(LayerId, i32, i32, i32, i32)> = r.wires[n].iter().map(|s| f(s.rect)).zip(&r.wires[n]).map(|(q, s)| (s.layer, q.x, q.y, q.w, q.h)).collect();
                v.sort_unstable_by_key(|t| (t.0 .0, t.1, t.2, t.3, t.4));
                v
            };
            set(0, &mirror) == set(1, &|q| q)
        };
        let (routes, _, stats) = run(&[]);
        assert_eq!(stats.pairs_exact.len(), 1, "{:?}", stats.pairs_fallback);
        assert!(routes.wires[0].iter().any(|s| s.layer == LAYERS[0] && contains(s.rect, gap)), "net 0's fill closes the gap");
        assert!(imaged(&routes), "{:?}\n{:?}", routes.wires[0], routes.wires[1]);
        assert_eq!(stats.pair_fillers_dropped, 0);
        let site = mirror(gap);
        let ring = Macro { shapes: vec![Shape { layer: LAYERS[0], rect: Rect { x: site.x, y: site.y - 60 - 50, w: 100, h: 50 } }], bbox: Rect { x: site.x, y: site.y - 110, w: 100, h: 50 }, ..Default::default() };
        let (routes, _, stats) = run(&[ring]);
        assert_eq!(stats.pairs_exact.len(), 1, "{:?}", stats.pairs_fallback);
        assert_eq!(stats.pair_fillers_dropped, 1, "one filler faces the ring, counted once");
        assert!(imaged(&routes), "{:?}\n{:?}", routes.wires[0], routes.wires[1]);
    }

    /// Series R is priced only for a net over its IR-drop budget: a heavy
    /// net within budget routes exactly as with no budget at all.
    #[test]
    fn series_r_is_priced_only_past_the_drop_budget() {
        use analog::routing::{stack::Layer, IrDrop, Stack};
        let stack: &'static Stack = Box::leak(Box::new(Stack {
            layers: vec![
                Layer { id: 0, sheet_ohm: 0.1, ..Layer::default() },
                Layer { id: 2, sheet_ohm: 5.0, cut: true, ..Layer::default() },
                Layer { id: 1, sheet_ohm: 0.1, ..Layer::default() },
            ],
            antenna_cumulative: false,
        diode: None,
        }));
        let pins = [pin(0, 1_000, 1_000), pin(0, 12_000, 9_000)];
        let cell = Macro {
            shapes: Vec::new(),
            pins: pins.iter().zip(["S", "D"]).map(|(&(net, at, layer), n)| pnr_core::Pin { name: n.into(), net, at, layer }).collect(),
            bbox: Rect { x: 0, y: 0, w: 13_000, h: 10_000 },
            units: Vec::new(),
            dummies: Vec::new(),
            ..Default::default()
        };
        let cfg = || DetailedCfg { pin_ua: vec![vec![("S".into(), Some(-1_000)), ("D".into(), Some(1_000))]], layer_r: vec![1.0, 1.0], via_r: vec![20.0], ..test_cfg() };
        let run = |max_drop_uv: Option<i64>| {
            let mut reqs = Requirements::<Routes>::default();
            if let Some(max_drop_uv) = max_drop_uv {
                reqs.budget.push(Box::new(vec![IrDrop { net: NetId(0), current_ua: 1_000, max_drop_uv, margin_pct: 20, stack: Some(stack) }]));
            }
            DetailedRoute { cfg: cfg() }.route(&pins, &[cell.clone()], &[], &reqs, &LAYERS, &CUTS, &mut gr::Negotiation::new()).0
        };
        let rects = |r: &Routes| r.wires[0].iter().map(|s| (s.layer, s.rect)).collect::<Vec<_>>();
        assert_eq!(rects(&run(Some(1_000_000_000))), rects(&run(None)), "within budget: no R pricing");
    }

    /// A pin on the upper metal whose node lands on the lower one is stitched
    /// by a cut at the node: the jog on the pin's metal otherwise floats.
    #[test]
    fn a_pin_on_the_upper_metal_is_stitched_to_its_node() {
        let up = |x, y| (NetId(0), Rect { x, y, w: 170, h: 170 }, LAYERS[1]);
        let (routes, report) = route(test_cfg(), &[up(1_000, 1_000), up(12_000, 1_000)], &[], &[], &mut gr::Negotiation::new());
        assert!(report.hard_violations.is_empty(), "{:?}", rules(&report));
        assert!(routes.wires[0].iter().any(|s| s.layer == CUTS[0].0), "a cut joins the node to the jog");
    }

    /// dac4's notch: two fat same-net trunks 100 nm apart, joined by a third
    /// piece across part of their run. Connected is not healed — the gap
    /// beside the joining piece is a notch until it is filled.
    #[test]
    fn a_bridged_gap_between_same_net_trunks_is_still_filled() {
        let m = |x, y, w, h| Shape { layer: LAYERS[0], rect: Rect { x, y, w, h } };
        let (a, b) = (m(27_995, 38_250, 8_735, 645), m(32_555, 38_995, 13_550, 1_590));
        let mut shapes = vec![a, b, m(31_675, 38_575, 2_890, 2_430)];
        heal_same_net_slivers(&mut shapes, &LAYERS, 140, 290, &[], 5);
        let gap = Rect { x: 34_565, y: 38_895, w: 36_730 - 34_565, h: 100 };
        assert!(shapes.iter().any(|s| contains(s.rect, gap)), "{:?}", &shapes[3..]);
    }

    /// dac4's via1 pair: a pin just beside a trunk via at its landed node. The
    /// access climb must not add its own cut within cut spacing of that via —
    /// the via and the overlapping pads already join the two layers.
    #[test]
    fn an_access_cut_beside_a_same_net_via_is_not_duplicated() {
        let cfg = test_cfg();
        let (cut, size, below, above) = CUTS[0];
        let at = |x: i32, y: i32, l: LayerId, s: i32| Shape { layer: l, rect: Rect { x: x - s / 2, y: y - s / 2, w: s, h: s } };
        // Net 0's trunk via at the node (1000, 1190), with its pads.
        let mut routes = Routes { wires: vec![vec![at(1_000, 1_190, cut, size), at(1_000, 1_190, LAYERS[0], below), at(1_000, 1_190, LAYERS[1], above)]], ..Default::default()  };
        let access = [Access { ci: 0, pin: Rect { x: 915, y: 915, w: 170, h: 170 }, pin_layer: LAYERS[0], node: (1_000, 1_190), node_layer: 0, choice: None, ua: None, node_ua: None }];
        let joins = joins(&LAYERS, &CUTS, None);
        add_pin_access(&mut routes, &access, &[0], &cfg, &LAYERS, &CUTS, &joins, &[], (0, 0), &[Vec::new()]);
        let cuts: Vec<Rect> = routes.wires[0].iter().filter(|s| s.layer == cut).map(|s| s.rect).collect();
        for (i, a) in cuts.iter().enumerate() {
            for b in &cuts[i + 1..] {
                assert!(a == b || rect_gap(*a, *b) >= size, "cuts {a:?} and {b:?} closer than spacing");
            }
        }
    }

    /// tq_chain's via.2: a cap array's tie ends in a via1 under its met1 pin
    /// lead and met2 pad; the net's climb at the pin lands beside it. That
    /// cell cut joins the route's two layers, so the route's own cut, within
    /// cut spacing of it, is dropped rather than shipped.
    #[test]
    fn a_route_cut_beside_its_own_cell_cut_is_dropped() {
        let (cut, size, ..) = CUTS[0];
        let pin_at = Rect { x: 1_000, y: 1_000, w: 170, h: 170 };
        let cell_cut = Rect { x: 1_135 + 40, y: 1_035, w: size, h: size };
        let s = |layer, x, y, w, h| Shape { layer, rect: Rect { x, y, w, h } };
        let cell = Macro {
            shapes: vec![s(LAYERS[0], 1_000, 1_000, 400, 170), s(cut, cell_cut.x, cell_cut.y, size, size), s(LAYERS[1], 1_150, 1_010, 150, 150)],
            pins: vec![pnr_core::Pin { name: "d0:N".into(), net: NetId(0), at: pin_at, layer: LAYERS[0] }],
            bbox: Rect { x: 1_000, y: 1_000, w: 400, h: 170 },
            ..Default::default()
        };
        let pins = [(NetId(0), pin_at, LAYERS[0]), (NetId(0), Rect { x: 12_000, y: 1_000, w: 170, h: 170 }, LAYERS[1])];
        let cfg = DetailedCfg { stack: Some(test_stack()), ..test_cfg() };
        let (routes, _) = route(cfg, &pins, &[cell], &[], &mut gr::Negotiation::new());
        for c in routes.wires[0].iter().filter(|s| s.layer == cut) {
            let g = rect_gap(c.rect, cell_cut);
            assert!(g == 0 || g >= size, "route cut {:?} {g} nm from the cell cut {cell_cut:?}", c.rect);
        }
    }

    /// An inserted cell lands in the nearest free spot that keeps clearance
    /// from every obstacle, with its pins moved alongside.
    #[test]
    fn place_near_finds_the_closest_free_spot() {
        let cell = Macro {
            shapes: vec![Shape { layer: LAYERS[0], rect: Rect { x: 0, y: 0, w: 1_000, h: 1_000 } }],
            pins: vec![pnr_core::Pin { name: "d0:N".into(), net: NetId(0), at: Rect { x: 100, y: 100, w: 170, h: 170 }, layer: LAYERS[0] }],
            bbox: Rect { x: 0, y: 0, w: 1_000, h: 1_000 },
            units: Vec::new(),
            dummies: Vec::new(),
            ..Default::default()
        };
        let blocker = Rect { x: 4_000, y: 4_000, w: 2_000, h: 2_000 };
        let m = place_near(&cell, (5_000, 5_000), &[blocker], 500, 100, 20_000).unwrap();
        assert!(rect_gap(m.bbox, blocker) >= 500, "{:?}", m.bbox);
        assert!(rect_gap(m.bbox, blocker) < 700, "as close as the grid allows: {:?}", m.bbox);
        assert_eq!((m.pins[0].at.x - m.bbox.x, m.pins[0].at.y - m.bbox.y), (100, 100), "pins move with the cell");
        assert!(place_near(&cell, (5_000, 5_000), &[Rect { x: -1_000_000, y: -1_000_000, w: 2_000_000, h: 2_000_000 }], 500, 100, 20_000).is_none());
    }

    /// KCL on the Prim tree (theory ch. 20: a 3.5 mA source feeding 2 + 1.5
    /// mA): each edge carries the sum beyond it; with an unplaced port the
    /// larger side.
    #[test]
    fn prim_tree_carries_the_far_side() {
        assert_eq!(prim(&[(0, 0), (10, 0), (20, 0)], &[-3_500.0, 2_000.0, 1_500.0]), (vec![0, 1, 2], vec![0.0, 3_500.0, 1_500.0], vec![0, 0, 1]));
        let (_, e, p) = prim(&[(0, 0), (10, 0), (0, 20)], &[-3_500.0, 2_000.0, 1_500.0]);
        assert_eq!((&e[1..], p), (&[2_000.0, 1_500.0][..], vec![0, 0, 0]), "b and c both hang off the root");
        // The source missing (a port elsewhere): b roots, and the stub to c
        // carries its own 1.5 mA or, fed from c's side, everything else.
        assert_eq!(prim(&[(10, 0), (20, 0)], &[2_000.0, 1_500.0]).1, vec![0.0, 2_000.0]);
    }

    /// The guide for `b` is free exactly on the mirror image of `a`'s tree, and
    /// absent when the terminals are not mirror images.
    #[test]
    fn mirror_guide_follows_the_partner() {
        let grid = TrackGrid::with_layers((10_000, 10_000), 1_000, VIA_COST, 2);
        let n = |ix, iy| grid.node(ix, iy, 0);
        let terms = vec![vec![n(1, 2), n(3, 2)], vec![n(8, 2), n(6, 2)], vec![n(8, 5), n(6, 2)]];
        let mut hot = RouteHot::new(grid.nodes(), 3);
        hot.commit(0, vec![vec![n(1, 2), n(2, 2), n(3, 2)]], Vec::new(), Vec::new());
        let g = mirror_guide(&hot, &grid, &terms, 0, 1).expect("mirror-placed pair");
        let free: Vec<u32> = (0..grid.nodes() as u32).filter(|&i| g[i as usize] == 0.0).collect();
        assert_eq!(free, vec![n(6, 2), n(7, 2), n(8, 2)]);
        assert!(mirror_guide(&hot, &grid, &terms, 0, 2).is_none());
        let away = keep_away(&hot, &grid, &[0]);
        assert_eq!(away[n(2, 3) as usize], COUPLE_COST);
        assert_eq!(away[n(2, 4) as usize], 0.0);
    }

    /// No two nets may overlap on a layer.
    #[test]
    fn no_two_nets_share_geometry_on_a_layer() {
        let pins: Vec<(NetId, Rect, LayerId)> = (0..4u16)
            .flat_map(|n| {
                let x = 2_000 + i32::from(n) * 300;
                [pin(n, x, 2_000), pin(n, x, 9_000)]
            })
            .collect();
        let (routes, _) = route(test_cfg(), &pins, &[], &[], &mut gr::Negotiation::new());
        let hits = |a: Rect, b: Rect| a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
        for (i, na) in routes.wires.iter().enumerate() {
            for nb in routes.wires.iter().skip(i + 1) {
                for wa in na {
                    assert!(nb.iter().all(|wb| wa.layer != wb.layer || !hits(wa.rect, wb.rect)));
                }
            }
        }
    }

    /// Two nets' pins a pitch and a half apart: the lattice nodes between
    /// them crowd whichever pad they do not carry, so no trunk runs there and
    /// the nets' metal keeps its spacing.
    #[test]
    fn a_node_between_two_nets_pins_carries_neither() {
        // Pins on a pin-access conductor: their pads are on the lattice's layer.
        let cfg = DetailedCfg { pin_access: Some((LayerId(3), (LayerId(4), 100, 140, 290))), ..test_cfg() };
        let space = cfg.pitch - cfg.wire_width;
        let pin = |n: u16, x: i32, y: i32| (NetId(n), Rect { x, y, w: 170, h: 170 }, LayerId(3));
        // Net 0's trunk east would pass the node between the two pins.
        let pins = [pin(0, 2_000, 2_000), pin(1, 2_645, 2_000), pin(0, 6_000, 2_000), pin(1, 2_645, 6_000)];
        let (routes, _) = route(cfg, &pins, &[], &[], &mut gr::Negotiation::new());
        for a in routes.wires[0].iter().filter(|s| s.layer == LAYERS[0]) {
            for b in routes.wires[1].iter().filter(|s| s.layer == LAYERS[0]) {
                assert!(rect_gap(a.rect, b.rect) >= space, "{:?} {:?}", a.rect, b.rect);
            }
        }
    }

    /// A pair whose terminals are a translated copy gets the template's tree
    /// translated node for node; a mirrored pair its mirror image; a pair
    /// whose terminal sets do not correspond, or whose copy lands on another
    /// net, gets nothing.
    #[test]
    fn copy_tree_reproduces_the_template_exactly() {
        let g = TrackGrid::with_layers((20_000, 20_000), 200, VIA_COST, 2);
        let n = |x: u32, y: u32, l: u32| g.node(x, y, l);
        // Net 0: (2,2) → (6,2) on layer 0, then up to layer 1 and to (6,5).
        let t0 = vec![n(2, 2, 0), n(6, 5, 1)];
        let tree0: Vec<Vec<u32>> = vec![(2..=6).map(|x| n(x, 2, 0)).chain((2..=5).map(|y| n(6, y, 1))).collect()];
        let run = |t1: Vec<u32>, blocker: Option<u32>| {
            let cold = RouteCtx::new(TrackGrid::with_layers((20_000, 20_000), 200, VIA_COST, 2), vec![t0.clone(), t1, vec![]], vec![0, 1, 2]);
            let mut hot = RouteHot::new(cold.graph.nodes(), 3);
            hot.commit(0, tree0.clone(), Vec::new(), Vec::new());
            if let Some(b) = blocker {
                hot.commit(2, vec![vec![b]], Vec::new(), Vec::new());
            }
            copy_tree(&hot, &cold, 0, 1)
        };
        // Translated by (+10, +3).
        let got = run(vec![n(12, 5, 0), n(16, 8, 1)], None).expect("translation");
        let want: Vec<Vec<u32>> = vec![(12..=16).map(|x| n(x, 5, 0)).chain((5..=8).map(|y| n(16, y, 1))).collect()];
        assert_eq!(got, want);
        // Mirrored about x = 9: (2,2)↔(16,2), (6,5)↔(12,5).
        let got = run(vec![n(16, 2, 0), n(12, 5, 1)], None).expect("mirror");
        assert!(got[0].contains(&n(12, 2, 0)) && got[0].contains(&n(12, 5, 1)));
        // Terminals that no transform matches, and a copy blocked by another net.
        assert!(run(vec![n(12, 5, 0), n(17, 8, 1)], None).is_none());
        assert!(run(vec![n(12, 5, 0), n(16, 8, 1)], Some(n(14, 5, 0))).is_none());
    }

    /// A shielded victim routes between reference tracks, the shields are tied
    /// to the reference net (one connected net, no shorts), and without the
    /// request nothing is added.
    #[test]
    fn a_shield_request_draws_tied_reference_tracks_both_sides() {
        use analog::routing::Shield;
        // Victim: one long horizontal run. Reference: two pins well away.
        let pins = [pin(0, 2_000, 9_000), pin(0, 16_000, 9_000), pin(1, 2_000, 2_000), pin(1, 16_000, 2_000)];
        // 75%: beside the victim's end pads (wider than a wire) the shield
        // keeps min spacing, which leaves the pads' stretch unshielded.
        let shield = Shield { victim: NetId(0), reference: NetId(1), min_coverage_pct: 75, max_gap_nm: 430 };
        let mut reqs = Requirements::<Routes>::default();
        reqs.budget.push(Box::new(vec![shield]));
        let (routes, report, _) =
            DetailedRoute { cfg: test_cfg() }.route(&pins, &[], &[], &reqs, &LAYERS, &CUTS, &mut gr::Negotiation::new());
        assert!(report.hard_violations.is_empty(), "{:?}", rules(&report));
        let cov = analog::Rule::usage(shield, &routes).expect("victim routed");
        assert!(cov >= 0.75, "coverage {cov}");

        let (plain, _) = route(test_cfg(), &pins, &[], &[], &mut gr::Negotiation::new());
        assert!(analog::Rule::usage(shield, &plain).unwrap() < 0.1, "no request, no shield");
        assert!(routes.wires[1].len() > plain.wires[1].len());
    }

    /// RTE-19 set-up: a 20 µm two-pin victim (net 0) under a 430 nm shield
    /// by net 1, and a foreign net 2 whose natural run is the victim's
    /// adjacent track.
    fn shield_run() -> (Routes, Report, analog::routing::Shield) {
        use analog::routing::Shield;
        let pins = [pin(0, 1_000, 5_000), pin(0, 21_000, 5_000), pin(1, 1_000, 1_000), pin(1, 21_000, 1_000), pin(2, 3_000, 5_430), pin(2, 19_000, 5_430)];
        let shield = Shield { victim: NetId(0), reference: NetId(1), min_coverage_pct: 80, max_gap_nm: 430 };
        let mut reqs = Requirements::<Routes>::default();
        reqs.budget.push(Box::new(vec![shield]));
        let (routes, report, _) = DetailedRoute { cfg: test_cfg() }.route(&pins, &[], &[], &reqs, &LAYERS, &CUTS, &mut gr::Negotiation::new());
        (routes, report, shield)
    }

    /// The victim's side tracks are kept free during the search, so the
    /// foreign net detours and the reference takes both: coverage ≥ 0.95.
    #[test]
    fn a_shielded_victim_gets_both_side_tracks() {
        let (routes, report, shield) = shield_run();
        assert!(!rules(&report).iter().any(|r| r.starts_with("open net")), "{:?}", rules(&report));
        let cov = analog::Rule::usage(shield, &routes).expect("victim routed");
        // Measured 0.768 (RTE-19): the access jogs and pads at the victim's
        // ends count in its length and no track runs beside them; the trunk
        // alone is 0.93 covered. Not lowered: reported in m2-routing-report.
        assert!(cov >= 0.95, "coverage {cov}");
    }

    /// The shield is drawn and adds no coupling of its own: excluded, the
    /// reference (shields included) counts nothing; not excluded, it does.
    /// It screens the foreign net: the victim's coupling is strictly lower
    /// with the shield shapes than without them.
    #[test]
    fn a_shield_does_not_count_as_an_aggressor() {
        use analog::routing::CouplingBudget;
        let (routes, _, shield) = shield_run();
        assert!(analog::Rule::usage(shield, &routes).is_some_and(|u| u > 0.0), "no shield drawn");
        let b = CouplingBudget { net: NetId(0), max_coupling_af: 1, margin_pct: 0, stack: None, exclude: Some(NetId(1)), aggressor_weight: None };
        let total = |b: CouplingBudget, r: &Routes| analog::Rule::usage(b, r).unwrap();
        let without = |r: &Routes| Routes { wires: vec![r.wires[0].clone(), Vec::new(), r.wires[2].clone()], ..Default::default() };
        let quiet = |r: &Routes| Routes { wires: vec![r.wires[0].clone(), r.wires[1].clone(), Vec::new()], ..Default::default() };
        assert_eq!(total(b, &quiet(&routes)), 0.0, "an excluded reference counts");
        assert!(total(CouplingBudget { exclude: None, ..b }, &quiet(&routes)) > 0.0, "an included reference counts nothing");
        assert!(total(b, &routes) < total(b, &without(&routes)), "the shield screens nothing");
    }

    /// A ring band on layer 0 between two pins of another net at its rows:
    /// the net still connects, through layer 1, and none of its layer-0
    /// metal comes within spacing of the band (no tree node inside the
    /// grown band). Without the block the straight layer-0 run crosses it.
    #[test]
    fn a_ring_band_is_a_hard_obstacle() {
        let band = Rect { x: 5_000, y: 7_000, w: 6_000, h: 800 };
        let ring = Macro { bbox: band, shapes: vec![Shape { layer: LAYERS[0], rect: band }], ..Default::default() };
        let pins = [pin(0, 1_000, 7_300), pin(0, 15_000, 7_300)];
        let cfg = test_cfg();
        let space = cfg.pitch - cfg.wire_width;
        let (routes, report) = route(cfg, &pins, &[], &[ring], &mut gr::Negotiation::new());
        assert!(report.hard_violations.is_empty(), "{:?}", rules(&report));
        for &(_, r, _) in &pins {
            assert!(routes.wires[0].iter().any(|s| touches(s, r)));
        }
        assert!(routes.wires[0].iter().any(|s| s.layer == LAYERS[1]), "the net never left layer 0");
        for s in routes.wires[0].iter().filter(|s| s.layer == LAYERS[0]) {
            assert!(rect_gap(s.rect, band) >= space, "layer-0 metal {:?} within spacing of the band", s.rect);
        }
    }

    /// A routed wire touching cell metal of another net, on its layer, is a
    /// short; touching its own net's cell metal is not.
    #[test]
    fn a_route_touching_foreign_cell_metal_is_a_short() {
        let met1 = LAYERS[0];
        let wires = vec![vec![Shape { layer: met1, rect: Rect { x: 0, y: 0, w: 2_000, h: 260 } }]];
        let cell = Shape { layer: met1, rect: Rect { x: 1_000, y: 0, w: 170, h: 170 } };
        assert_eq!(cross_net_shorts(&wires, &[], &[(Some(1), cell)], 1_000), (vec![], vec![0]));
        assert_eq!(cross_net_shorts(&wires, &[], &[(Some(0), cell)], 1_000), (vec![], vec![]));
    }

    /// Overlapping access jogs: the net later in the routing order loses its
    /// jog; the other is intact. Against the later net's trunk, the earlier
    /// net's jog goes. Trunks touching delete nothing (`score` reports them).
    #[test]
    fn resolver_sacrifices_the_later_access_shape() {
        let m = |x, w| Shape { layer: LAYERS[0], rect: Rect { x, y: 0, w, h: 290 } };
        let wires = || vec![vec![m(0, 2_000), m(2_000, 300)], vec![m(5_000, 2_000), m(2_200, 2_800)]];
        let joins = joins(&LAYERS, &CUTS, None);
        let (mut w, mut sacrificed) = (wires(), vec![0; 2]);
        break_shorts(&mut w, &[1, 1], &[0, 1], &joins, &mut sacrificed, 1_000);
        assert_eq!(w, vec![wires()[0].clone(), vec![m(5_000, 2_000)]]);
        assert_eq!(sacrificed, [0, 1]);
        let (mut w, mut sacrificed) = (wires(), vec![0; 2]);
        break_shorts(&mut w, &[1, 1], &[1, 0], &joins, &mut sacrificed, 1_000);
        assert_eq!(w, vec![vec![m(0, 2_000)], wires()[1].clone()]);
        assert_eq!(sacrificed, [1, 0]);
        // The later net's side is trunk: the earlier net's jog goes instead.
        let (mut w, mut sacrificed) = (wires(), vec![0; 2]);
        break_shorts(&mut w, &[1, 2], &[0, 1], &joins, &mut sacrificed, 1_000);
        assert_eq!(w, vec![vec![m(0, 2_000)], wires()[1].clone()]);
        assert_eq!(sacrificed, [1, 0]);
        let (mut w, mut sacrificed) = (vec![vec![m(0, 2_000)], vec![m(1_000, 2_000)]], vec![0; 2]);
        break_shorts(&mut w, &[1, 1], &[0, 1], &joins, &mut sacrificed, 1_000);
        assert_eq!((w[1].len(), sacrificed), (1, vec![0, 0]));
    }

    /// Metals 0 and 2 overlapping in xy with no cut between them are two
    /// pieces; the via stack through metal 1 joins them.
    #[test]
    fn an_xy_overlap_on_non_adjacent_layers_is_open() {
        let joins = [(LayerId(10), LayerId(0), LayerId(1)), (LayerId(11), LayerId(1), LayerId(2))];
        let at = |l| Shape { layer: LayerId(l), rect: Rect { x: 0, y: 0, w: 500, h: 500 } };
        assert_eq!(open_components(&[at(0), at(2)], &joins), 1);
        assert_eq!(open_components(&[at(0), at(10), at(1), at(11), at(2)], &joins), 0);
    }

    /// A cell strap on layer 0 across the straight run, attributed by
    /// `cell_metal` (with a stack) to the net whose pin sits on it: that net's
    /// layer-0 trunk merges with it (a wire covers the strap well away from
    /// the pin's stitch nodes) and is no short. Blocked, it would detour.
    #[test]
    fn a_trunk_merges_with_its_own_cell_strap() {
        let strap = Rect { x: 4_000, y: 7_000, w: 8_000, h: 800 };
        let cell = Macro {
            bbox: strap,
            shapes: vec![Shape { layer: LAYERS[0], rect: strap }],
            pins: vec![pnr_core::Pin { name: "c:D".into(), net: NetId(0), at: Rect { x: 7_900, y: 7_300, w: 170, h: 170 }, layer: LAYERS[0] }],
            ..Default::default()
        };
        let pins = [pin(0, 1_000, 7_300), pin(0, 15_000, 7_300)];
        let cfg = DetailedCfg { stack: Some(test_stack()), ..test_cfg() };
        let (routes, report) = route(cfg, &pins, &[cell], &[], &mut gr::Negotiation::new());
        assert!(report.hard_violations.is_empty(), "{:?}", rules(&report));
        let away = Rect { x: 5_000, y: strap.y, w: 1, h: strap.h };
        assert!(
            routes.wires[0].iter().any(|s| s.layer == LAYERS[0] && touches(s, away)),
            "net 0 kept off its own strap: {:?}",
            routes.wires[0]
        );
    }

    /// The frame origin sits on the lattice period, at or below `low − pad`.
    #[test]
    fn origin_is_a_multiple_of_the_lattice_period() {
        let (x, y) = frame_origin((-12_345, 7), 2_100, 840);
        for (v, low) in [(x, -12_345), (y, 7)] {
            assert_eq!(v.rem_euclid(840), 0);
            assert!(v <= low - 2_100 && v > low - 2_100 - 840, "{v}");
        }
    }

    /// Two nets each climbing a layer from pins one `p0` apart along a
    /// layer-0 track: the via halo pushes one via off, so no two foreign
    /// same-layer shapes come closer than the 140 nm spacing (pads 320 nm
    /// along the track, 400 nm apart, would leave 80 nm).
    #[test]
    fn adjacent_foreign_vias_keep_the_along_track_halo() {
        fn run(halo_via: u8) -> (Routes, Report) {
            let spec = |horizontal| gr::LayerSpec { horizontal, wire: 260, space: 140, pad_across: 260, pad_along: 320, halo_via, ..gr::LayerSpec::default() };
            let cfg = DetailedCfg { pitch: 400, wire_width: 260, layers: vec![spec(true), spec(false)], ..test_cfg() };
            // Pin centres on nodes (absolute x ≡ 200 mod 400): one up, one down.
            let p = |net, x: i32, y: i32| pin(net, x - 85, y - 85);
            let pins = [p(0, 4_200, 4_200), p(0, 4_200, 8_200), p(1, 4_600, 4_200), p(1, 4_600, 200)];
            route(cfg, &pins, &[], &[], &mut gr::Negotiation::new())
        }
        let gaps = |r: &Routes| {
            let mut worst = i32::MAX;
            for a in 0..r.wires.len() {
                for b in a + 1..r.wires.len() {
                    for sa in &r.wires[a] {
                        for sb in r.wires[b].iter().filter(|s| s.layer == sa.layer) {
                            worst = worst.min(rect_gap(sa.rect, sb.rect));
                        }
                    }
                }
            }
            worst
        };
        let (with, report) = run(1);
        assert!(report.hard_violations.is_empty(), "{:?}", rules(&report));
        assert!(gaps(&with) >= 140, "{}", gaps(&with));
        let without = run(0).0;
        assert!(gaps(&without) < 140, "without the halo the pads crowd: {}", gaps(&without));
    }

    /// A placed cell holding `net` 0's pins, the first drawing `ua` µA and the
    /// last sourcing it, with 1 mA/µm on `layers`: the EM current that sets
    /// `k` ([`DetailedCfg::pin_ua`]).
    fn em_cell(pins: &[(i32, i32)], ua: i32, layers: &[LayerId]) -> (Macro, DetailedCfg) {
        let pin = |i: usize, &(x, y): &(i32, i32)| pnr_core::Pin { name: format!("p{i}"), net: NetId(0), at: Rect { x: x - 85, y: y - 85, w: 170, h: 170 }, layer: LAYERS[0] };
        let cell = Macro { pins: pins.iter().enumerate().map(|(i, p)| pin(i, p)).collect(), bbox: Rect { x: 0, y: 0, w: 100, h: 100 }, ..Default::default() };
        let n = pins.len() - 1;
        let table = (0..=n).map(|i| (format!("p{i}"), Some(if i == 0 { ua } else if i == n { -ua } else { 0 }))).collect();
        let lim = analog::routing::em::Limit { ua_per_um: 1_000.0, ua_per_cut: 0.0, blech: 0.0 };
        let cfg = DetailedCfg { pin_ua: vec![table], em: layers.iter().map(|&l| (l, lim)).collect(), ..test_cfg() };
        (cell, cfg)
    }

    /// Net-0 pins `(rect, layer, µA)` of one cell, named `p{i}`, and the
    /// sky130 met1 lattice (pitch 420, wire 260) with `lim` on both LAYERS
    /// and CUTS[0] (per cut only, as `Pdk::em_limit` gives a cut) and that
    /// operating point.
    fn sky_cell(pins: &[(Rect, LayerId, i32)], lim: analog::routing::em::Limit) -> (Vec<(NetId, Rect, LayerId)>, Macro, DetailedCfg) {
        use analog::routing::em::Limit;
        let cell = Macro {
            pins: pins.iter().enumerate().map(|(i, &(at, layer, _))| pnr_core::Pin { name: format!("p{i}"), net: NetId(0), at, layer }).collect(),
            bbox: Rect { x: 0, y: 0, w: 13_000, h: 10_000 },
            ..Default::default()
        };
        let table = pins.iter().enumerate().map(|(i, p)| (format!("p{i}"), Some(p.2))).collect();
        let cfg = DetailedCfg { pitch: 420, wire_width: 260, grid: 5, pin_ua: vec![table], em: vec![(LAYERS[0], lim), (LAYERS[1], lim), (CUTS[0].0, Limit { ua_per_um: 0.0, ..lim })], ..DetailedCfg::default() };
        (pins.iter().map(|&(at, l, _)| (NetId(0), at, l)).collect(), cell, cfg)
    }

    const SKY_LIM: analog::routing::em::Limit = analog::routing::em::Limit { ua_per_um: 2_800.0, ua_per_cut: 1e6, blech: 0.0 };

    /// The feed's MST edge carries both loads: S→a (2 mA, 714 nm: 3 tracks,
    /// 260 + 2·420) is wider than a→b (1 mA, 357 nm: 2 tracks). With the
    /// stack the EM rounds check the drawn net and it passes the hard rule:
    /// here a's access climbs onto b's met2 run, so that run's shape carries
    /// all 2 mA and the rounds widen it.
    #[test]
    fn a_trunk_carrying_two_branches_is_wider() {
        let sq = |x, y| Rect { x, y, w: 170, h: 170 };
        let (pins, cell, cfg) = sky_cell(&[(sq(1_000, 1_000), LAYERS[0], -2_000), (sq(9_000, 1_000), LAYERS[0], 1_000), (sq(9_000, 7_000), LAYERS[0], 1_000)], SKY_LIM);
        let em = em_rule(&cfg, 0);
        let run = |cfg: DetailedCfg| route(cfg, &pins, std::slice::from_ref(&cell), &[], &mut gr::Negotiation::new()).0;
        let trunks = |r: &Routes| -> Vec<i32> { r.wires[0].iter().filter(|s| s.rect.w != s.rect.h && LAYERS.contains(&s.layer)).map(|s| s.rect.w.min(s.rect.h)).collect() };
        let mst = trunks(&run(cfg.clone()));
        assert_eq!(mst.iter().max(), Some(&1_100), "{mst:?}");
        assert!(mst.contains(&680), "{mst:?}");
        use analog::Rule;
        let routes = run(DetailedCfg { stack: Some(test_stack()), ..cfg });
        assert!(em.known(&routes) && em.satisfied(&routes), "{:?} {:?}", em.failing(&routes), trunks(&routes));
    }

    /// li pins reached through mcon (`pin_access`): a ±1 mA access jog on met1
    /// is never narrower than its 357 nm EM width (snapped to 360); with no
    /// operating point a jog stays at wire width.
    #[test]
    fn access_jog_never_necks_below_em() {
        let li = LayerId(3);
        let sq = |x, y| Rect { x, y, w: 170, h: 170 };
        let lim = Limit { ua_per_um: 2_800.0, ua_per_cut: 1e6, blech: 0.0 };
        use analog::routing::em::Limit;
        let (pins, cell, cfg) = sky_cell(&[(sq(1_000, 1_000), li, 1_000), (sq(12_000, 9_000), li, -1_000)], lim);
        let cfg = DetailedCfg { pin_access: Some((li, (LayerId(4), 170, 170, 260))), em: [cfg.em.clone(), vec![(li, lim)]].concat(), ..cfg };
        let jogs = |cfg: DetailedCfg| -> Vec<i32> {
            let (routes, _) = route(cfg, &pins, std::slice::from_ref(&cell), &[], &mut gr::Negotiation::new());
            routes.wires[0].iter().filter(|s| s.layer == LAYERS[0] && s.rect.w != s.rect.h).map(|s| s.rect.w.min(s.rect.h)).collect()
        };
        let with = jogs(cfg.clone());
        assert!(!with.is_empty() && with.iter().all(|&w| w >= 360), "{with:?}");
        let without = jogs(DetailedCfg { pin_ua: Vec::new(), ..cfg });
        assert!(without.iter().any(|&w| w < 360), "{without:?}");
    }

    /// A foreign wire 70 nm beside the full-width (360 nm) jog, 165 nm beside
    /// a 170 nm pin-wide one: only the narrow jog keeps spacing, but at 1 mA
    /// it would neck below its EM width, so the jog stays full width.
    #[test]
    fn a_blocked_full_jog_never_falls_back_below_em() {
        let cfg = DetailedCfg { em: vec![(LAYERS[1], SKY_LIM)], ..test_cfg() };
        let foreign = Shape { layer: LAYERS[1], rect: Rect { x: 1_250, y: 800, w: 290, h: 900 } };
        let mut routes = Routes { wires: vec![Vec::new(), vec![foreign]], ..Default::default() };
        let access = [Access { ci: 0, pin: Rect { x: 915, y: 915, w: 170, h: 170 }, pin_layer: LAYERS[0], node: (1_000, 1_430), node_layer: 1, choice: None, ua: None, node_ua: Some(1_000.0) }];
        add_pin_access(&mut routes, &access, &[0], &cfg, &LAYERS, &CUTS, &joins(&LAYERS, &CUTS, None), &[], (0, 0), &[Vec::new()]);
        let jogs: Vec<i32> = routes.wires[0].iter().filter(|s| s.layer == LAYERS[1] && s.rect.w != s.rect.h).map(|s| s.rect.w.min(s.rect.h)).collect();
        assert!(!jogs.is_empty() && jogs.iter().all(|&w| w >= 360), "{jogs:?}");
    }

    /// An 800 µA li pin gets ⌈800/360⌉ = 3 mcon cuts in a row along its
    /// 1 µm side, at cut spacing; 1.3 mA needs 4 (4·170 + 3·190 = 1250 nm >
    /// 1000) and the shortfall is V naming the pin.
    #[test]
    fn access_cuts_follow_current() {
        use analog::routing::em::Limit;
        let li = LayerId(3);
        let mcon = LayerId(4);
        let tall = |x, y| Rect { x, y, w: 170, h: 1_000 };
        let run = |ua: i32| {
            let (pins, cell, cfg) = sky_cell(&[(tall(1_000, 1_000), li, ua), (tall(12_000, 8_000), li, -ua)], SKY_LIM);
            let cfg = DetailedCfg {
                pin_access: Some((li, (mcon, 170, 170, 260))),
                em: [cfg.em.clone(), vec![(mcon, Limit { ua_per_um: 0.0, ua_per_cut: 360.0, blech: 0.0 })]].concat(),
                spacing: vec![(mcon, 190, vec![])],
                ..cfg
            };
            let (routes, report) = route(cfg, &pins, &[cell], &[], &mut gr::Negotiation::new());
            (routes, report, pins[0].1)
        };
        let (routes, report, pin0) = run(800);
        let cuts: Vec<Rect> = routes.wires[0].iter().filter(|s| s.layer == mcon && contains(pin0, s.rect)).map(|s| s.rect).collect();
        assert_eq!(cuts.len(), 3, "{cuts:?}");
        assert!(cuts.iter().enumerate().all(|(i, &a)| cuts[i + 1..].iter().all(|&b| rect_gap(a, b) >= 190)), "{cuts:?}");
        assert!(!rules(&report).iter().any(|r| r.starts_with("em access")), "{:?}", rules(&report));
        let (_, report, _) = run(1_300);
        assert!(rules(&report).iter().any(|r| r.starts_with("em access net 0")), "{:?}", rules(&report));
    }

    /// Smallest gap between two nets' same-layer shapes.
    fn foreign_gap(r: &Routes) -> i32 {
        let mut worst = i32::MAX;
        for a in 0..r.wires.len() {
            for b in a + 1..r.wires.len() {
                for sa in &r.wires[a] {
                    for sb in r.wires[b].iter().filter(|s| s.layer == sa.layer) {
                        worst = worst.min(rect_gap(sa.rect, sb.rect));
                    }
                }
            }
        }
        worst
    }

    /// sky130 met1/met2 at `k = 2`: runs drawn merged at
    /// `W(2) = 260 + 420` and `280 + 420`, and a bend whose 700×680 corner
    /// block would hold the 2×2 via array (85 nm along-wire enclosure, 170 nm
    /// cut spacing). Returns the routes, report and the cell.
    fn two_by_two_corner() -> (Routes, Report, Macro) {
        let via = LayerId(2);
        let spec = |horizontal, wire| gr::LayerSpec { horizontal, wire, space: 140, pad_across: wire, pad_along: if horizontal { 320 } else { 370 }, halo_via: 1, ..gr::LayerSpec::default() };
        // 680 µA: 680 nm at 1 mA/µm on met1 and 700 nm at 0.9715 mA/µm on met2, each
        // exactly its `W(2)`, so EM narrows neither run.
        let (cell, cfg) = em_cell(&[(2_310, 2_310), (12_390, 9_870)], 680, &LAYERS);
        let mut em = cfg.em.clone();
        em[1].1.ua_per_um = 971.5;
        let cfg = DetailedCfg {
            em,
            pitch: 420,
            wire_width: 260,
            layers: vec![spec(true, 260), spec(false, 280)],
            spacing: vec![(via, 170, Vec::new())],
            cut_enclosure_pair: vec![(via, (55, 85), (55, 85))],
            ..cfg
        };
        let cuts = [(via, 150, 320, 370)];
        let reqs = Requirements::<Routes>::default();
        let (routes, report, _) = DetailedRoute { cfg }.route(&[], &[cell.clone()], &[], &reqs, &LAYERS, &cuts, &mut gr::Negotiation::new());
        (routes, report, cell)
    }

    /// The fixture's 700×680 corner blocks on LAYERS[0] holding no pin centre.
    fn corner_blocks(w: &[Shape], cell: &Macro) -> Vec<Rect> {
        let pins: Vec<(i32, i32)> = cell.pins.iter().map(|p| (p.at.x + 85, p.at.y + 85)).collect();
        let inside = |r: Rect, (x, y): (i32, i32)| x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h;
        // A merged corner block is drawn on both metals (a stub only below).
        w.iter()
            .filter(|s| s.layer == LAYERS[0] && (s.rect.w, s.rect.h) == (700, 680) && w.contains(&Shape { layer: LAYERS[1], rect: s.rect }))
            .map(|s| s.rect)
            .filter(|&r| !pins.iter().any(|&p| inside(r, p)))
            .collect()
    }

    /// Same-size rect sharing an edge with `b` (GAP-12's stub).
    fn beside(r: Rect, b: Rect) -> bool {
        r != b && (r.w, r.h) == (b.w, b.h) && rect_gap(r, b) == 0 && (r.x == b.x || r.y == b.y)
    }

    #[test]
    fn a_two_by_two_corner_gets_four_cuts() {
        let via = LayerId(2);
        let (routes, report, cell) = two_by_two_corner();
        assert!(report.hard_violations.is_empty(), "{:?}", rules(&report));
        let w = &routes.wires[0];
        let widest = |l: LayerId| w.iter().filter(|s| s.layer == l && s.rect.w != s.rect.h).map(|s| s.rect.w.min(s.rect.h)).max();
        assert_eq!((widest(LAYERS[0]), widest(LAYERS[1])), (Some(680), Some(700)));
        let bends = corner_blocks(w, &cell);
        assert!(!bends.is_empty(), "no corner block");
        // GAP-12: the array sits in the stub beside the corner, not in it.
        for b in bends {
            let stub = w.iter().find(|s| s.layer == LAYERS[0] && beside(s.rect, b)).unwrap_or_else(|| panic!("no stub beside {b:?}"));
            let n = w.iter().filter(|s| s.layer == via && contains(stub.rect, s.rect)).count();
            assert_eq!(n, 4, "cuts in the stub beside {b:?}");
        }
    }

    /// GAP-12 step 2: the corner array moves onto the straight run beside the
    /// corner block, inside a lower stub and the upper run, EM still clean.
    #[test]
    fn a_corner_via_array_moves_onto_the_straight_run() {
        let via = LayerId(2);
        let (routes, report, cell) = two_by_two_corner();
        assert!(report.hard_violations.is_empty(), "{:?}", rules(&report));
        let w = &routes.wires[0];
        let cuts: Vec<Rect> = w.iter().filter(|s| s.layer == via).map(|s| s.rect).collect();
        let bends = corner_blocks(w, &cell);
        assert!(!bends.is_empty(), "no corner block");
        for b in bends {
            assert!(!cuts.iter().any(|&c| contains(b, c)), "a cut in the corner {b:?}: {cuts:?}");
            let hosts = w
                .iter()
                .filter(|s| s.layer == LAYERS[0] && beside(s.rect, b))
                .filter(|s| {
                    let inn: Vec<Rect> = cuts.iter().copied().filter(|&c| contains(s.rect, c)).collect();
                    inn.len() == 4 && inn.iter().all(|&c| w.iter().any(|r| r.layer == LAYERS[1] && r.rect.w != r.rect.h && contains(r.rect, c)))
                })
                .count();
            assert_eq!(hosts, 1, "stubs beside {b:?} holding 4 cuts inside an upper run");
        }
    }

    /// GAP-12 step 1: a flush L's inner corner square; none for a T or
    /// unequal widths.
    #[test]
    fn a_critical_corner_gets_a_support_square() {
        let sh = |rect| Shape { layer: LAYERS[0], rect };
        let a = sh(Rect { x: 0, y: 0, w: 1_000, h: 260 });
        let sq = |b: Rect| support_squares(&[a, sh(b)], &LAYERS);
        assert_eq!(sq(Rect { x: 740, y: 0, w: 260, h: 1_000 }), vec![sh(Rect { x: 480, y: 260, w: 260, h: 260 })]);
        assert_eq!(sq(Rect { x: 0, y: 0, w: 260, h: 1_000 }), vec![sh(Rect { x: 260, y: 260, w: 260, h: 260 })]);
        assert_eq!(sq(Rect { x: 370, y: 0, w: 260, h: 1_000 }), vec![]);
        assert_eq!(sq(Rect { x: 700, y: 0, w: 300, h: 1_000 }), vec![]);
    }

    /// RTE-27: every stack via of free nets gets a second cut one cut pitch
    /// (100 + 100 fallback spacing) along its horizontal lower metal.
    #[test]
    fn every_free_via_is_doubled() {
        let mut pins = Vec::new();
        for n in 0..3u16 {
            let x = 1_000 + 4_000 * i32::from(n);
            pins.push(pin(n, x, 1_000));
            pins.push((NetId(n), Rect { x: x + 4_000, y: 5_000, w: 170, h: 170 }, LAYERS[1]));
        }
        let reqs = Requirements::<Routes>::default();
        let (routes, report, stats) = DetailedRoute { cfg: test_cfg() }.route(&pins, &[], &[], &reqs, &LAYERS, &CUTS, &mut gr::Negotiation::new());
        assert!(report.hard_violations.is_empty(), "{:?}", rules(&report));
        assert_eq!(stats.single_cut_vias, 0, "{stats:?}");
        assert!(stats.stack_vias >= 3, "{stats:?}");
        for (n, w) in routes.wires.iter().enumerate().take(3) {
            let cuts: Vec<Rect> = w.iter().filter(|s| s.layer == CUTS[0].0).map(|s| s.rect).collect();
            assert!(!cuts.is_empty(), "net {n} has no cut");
            for c in &cuts {
                let partners = cuts.iter().filter(|d| (d.x - c.x).abs() == 200 && d.y == c.y).count();
                assert_eq!(partners, 1, "net {n} cut {c:?} among {cuts:?}");
            }
        }
    }

    /// RTE-27 on an exact pair: the leader's second cut is taken only with its
    /// image, and both or neither are added.
    #[test]
    fn pair_vias_double_symmetrically() {
        let map = gr::LatticeMap::MirrorX { k: 10 };
        let c = Rect { x: 1_000, y: 1_000, w: 100, h: 100 };
        let pad = |r: Rect| Rect { x: r.x - 20, y: r.y - 20, w: 140, h: 140 };
        let alt = |d: i32| {
            let c2 = Rect { x: c.x + d, ..c };
            let b = bbox(pad(c), pad(c2));
            vec![Shape { layer: CUTS[0].0, rect: c2 }, Shape { layer: LAYERS[0], rect: b }, Shape { layer: LAYERS[1], rect: b }]
        };
        let net0 = vec![Shape { layer: CUTS[0].0, rect: c }, Shape { layer: LAYERS[0], rect: pad(c) }, Shape { layer: LAYERS[1], rect: pad(c) }];
        let img = |v: &[Shape]| v.iter().map(|s| Shape { rect: map_rect(s.rect, map, 400), ..*s }).collect::<Vec<_>>();
        let wires0 = vec![net0.clone(), img(&net0)];
        let sites = vec![vec![vec![alt(200), alt(-200)]], Vec::new()];
        let image = vec![Some((1, map)), None];
        // Above both upper bboxes (top y 1 120), 80 nm off; net 1 is past x 3 000.
        let foreign = Rect { x: 700, y: 1_200, w: 700, h: 100 };
        let blocked = |_: usize, s: Shape, _: &[Vec<Shape>]| s.layer != LAYERS[1] || rect_gap(s.rect, foreign) >= 140;
        let mut wires = wires0.clone();
        assert_eq!(add_where_clear(&mut wires, &sites, &image, 400, &blocked), vec![vec![None], vec![]]);
        assert_eq!(wires, wires0);
        let free = |_: usize, _: Shape, _: &[Vec<Shape>]| true;
        assert_eq!(add_where_clear(&mut wires, &sites, &image, 400, &free), vec![vec![Some(0)], vec![]]);
        assert_eq!(wires[1][3..], img(&alt(200))[..]);
        assert_eq!(wires[1].len(), 6);
    }

    /// A differential pair with no current keeps one track (no pair member
    /// is widened).
    #[test]
    fn a_differential_pair_keeps_one_track() {
        use analog::routing::Differential;
        let pins = [pin(0, 1_000, 1_000), pin(0, 12_000, 1_000), pin(1, 1_000, 5_000), pin(1, 12_000, 5_000), pin(2, 1_000, 9_000), pin(2, 12_000, 9_000)];
        let mut reqs = Requirements::<Routes>::default();
        reqs.budget.push(Box::new(vec![Differential { pos: NetId(0), neg: NetId(1), max_len_delta_pct10: 50, same_layer_required: true, stack: None, aggressor_weight: None }]));
        let (routes, _, _) = DetailedRoute { cfg: test_cfg() }.route(&pins, &[], &[], &reqs, &LAYERS, &CUTS, &mut gr::Negotiation::new());
        let widest = |n: usize| routes.wires[n].iter().filter(|s| s.rect.w != s.rect.h).map(|s| s.rect.w.min(s.rect.h)).max();
        assert_eq!((widest(0), widest(1)), (Some(290), Some(290)), "the pair keeps wire width");
    }

    /// Net 0 at `k = 2` on layer 0 (720 nm drawn, 500 nm of EM width) claims the track beside its anchor during
    /// the search: net 1, whose pins sit on that track inside net 0's span,
    /// never ends up closer than spacing to it.
    #[test]
    fn a_two_track_net_reserves_both_tracks() {
        // 500 µA at 1 mA/µm: k = 1 + ⌈(500 − 290)/430⌉ = 2 on layer 0 only.
        let (cell, cfg) = em_cell(&[(1_075, 5_375), (15_265, 5_375)], 500, &LAYERS[..1]);
        let pins = [pin(1, 4_755 - 85, 5_805 - 85), pin(1, 10_775 - 85, 5_805 - 85)];
        let reqs = Requirements::<Routes>::default();
        let (routes, report, _) = DetailedRoute { cfg }.route(&pins, &[cell], &[], &reqs, &LAYERS, &CUTS, &mut gr::Negotiation::new());
        assert!(report.hard_violations.is_empty(), "{:?}", rules(&report));
        let widest = routes.wires[0].iter().filter(|s| s.layer == LAYERS[0] && s.rect.w != s.rect.h).map(|s| s.rect.w.min(s.rect.h)).max();
        assert_eq!(widest, Some(720), "drawn across the two tracks it reserved");
        assert!(foreign_gap(&routes) >= 430 - 290, "{}", foreign_gap(&routes));
    }

    /// A 5-track bundle (`W = 290 + 4·430 = 2010` past a 1000 nm threshold)
    /// is drawn as 5 wires whose copper (`5·290`) carries the current, and its guard track keeps a foreign wire at the
    /// 500 nm wide spacing, not the 140 nm a single track gets.
    #[test]
    fn a_wide_bundle_keeps_wide_spacing() {
        // 1250 µA at 1 mA/µm: 1 + ⌈(1250 − 290)/430⌉ = 4 tracks pass the
        // threshold, so k = ⌈1260/290⌉ = 5 parallel wires.
        let (cell, cfg) = em_cell(&[(1_075, 5_375), (15_265, 5_375)], 1_250, &LAYERS[..1]);
        let cfg = DetailedCfg { spacing: vec![(LAYERS[0], 140, vec![(1_000, 500)])], ..cfg };
        // Net 1 on the track just past the bundle's fifth, along its span.
        let pins = [pin(1, 4_755 - 85, 5_375 + 5 * 430 - 85), pin(1, 10_775 - 85, 5_375 + 5 * 430 - 85)];
        let reqs = Requirements::<Routes>::default();
        let (routes, report, _) = DetailedRoute { cfg }.route(&pins, &[cell], &[], &reqs, &LAYERS, &CUTS, &mut gr::Negotiation::new());
        assert!(report.hard_violations.is_empty(), "{:?}", rules(&report));
        assert!(report.budget_violations.is_empty(), "{:?}", report.budget_violations.iter().map(|v| &v.rule).collect::<Vec<_>>());
        let long = routes.wires[0].iter().filter(|s| s.layer == LAYERS[0] && s.rect.h == 290 && s.rect.w > 2_000).count();
        assert!(long >= 5, "parallel wires: {long}");
        assert!(!routes.wires[0].iter().any(|s| s.layer == LAYERS[0] && s.rect.w.min(s.rect.h) > 290 && s.rect.w.min(s.rect.h) >= 1_000 && s.rect.w.max(s.rect.h) > 2_000), "no merged wide run");
        let gap = routes.wires[0].iter().filter(|s| s.layer == LAYERS[0]).flat_map(|a| routes.wires[1].iter().filter(|b| b.layer == LAYERS[0]).map(move |b| rect_gap(a.rect, b.rect))).min();
        assert!(gap.is_none_or(|g| g >= 500), "{gap:?}");
    }

    /// A route fills its search count and stage timers.
    #[test]
    fn route_fills_stage_timers() {
        let pins = [pin(0, 1_000, 1_000), pin(0, 18_000, 18_000)];
        let reqs = Requirements::<Routes>::default();
        let (_, _, stats) = DetailedRoute { cfg: test_cfg() }.route(&pins, &[], &[], &reqs, &LAYERS, &CUTS, &mut gr::Negotiation::new());
        assert!(stats.expanded > 0);
        assert!(stats.us_negotiate + stats.us_geometry > 0, "{stats:?}");
    }

    /// The region holding ten used nodes is the most congested, and its rect
    /// is where those nodes are.
    #[test]
    fn congestion_marks_the_crowded_region() {
        let g = TrackGrid::with_layers((64 * 100, 64 * 100), 100, VIA_COST, 2);
        let mut hot = RouteHot::new(g.nodes(), 1);
        // Regions are 4×4 nodes: (5, 5) spans nodes 20..24.
        for k in 0..10 {
            hot.usage[g.node(20 + k % 4, 20 + k / 4, 0) as usize] = 1;
        }
        hot.usage[g.node(49, 13, 1) as usize] = 1;
        let map = congestion(&hot, &g, (1_000, -500));
        let (top, _) = map.iter().copied().max_by(|a, b| a.1.total_cmp(&b.1)).unwrap();
        let centre = (1_000 + 22 * 100, -500 + 22 * 100);
        assert!(centre.0 > top.x && centre.0 < top.x + top.w && centre.1 > top.y && centre.1 < top.y + top.h, "{top:?}");
    }

    /// With no history, demand times the region's on-track node count sums
    /// to the total usage (remainder rows and columns included).
    #[test]
    fn congestion_without_history_counts_usage() {
        let g = TrackGrid::with_layers((70 * 100, 50 * 100), 100, VIA_COST, 2);
        let mut hot = RouteHot::new(g.nodes(), 1);
        let mut seed = 7u64;
        for u in &mut hot.usage {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            *u = (seed >> 63) as u16;
        }
        let total: f32 = hot.usage.iter().map(|&u| f32::from(u)).sum();
        let map = congestion(&hot, &g, (0, 0));
        let nodes = |r: &Rect| (r.w / 100 * (r.h / 100) * 2) as f32;
        let sum: f32 = map.iter().map(|(r, d)| d * nodes(r)).sum();
        assert!((sum - total).abs() < 1e-3 * total, "{sum} vs {total}");
        assert_eq!(map.iter().map(|(r, _)| nodes(r)).sum::<f32>(), g.nodes() as f32, "regions tile the frame");
    }
}
