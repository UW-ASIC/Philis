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
            supply_nets: Vec::new(),
            spacing: Vec::new(),
            array_spacing: Vec::new(),
            min_width: Vec::new(),
            cut_enclosure: Vec::new(),
            cut_enclosure_pair: Vec::new(),
            net_weight: Vec::new(),
            layer_c: Vec::new(),
            beside_c: Vec::new(),
            layer_r: Vec::new(),
            via_r: Vec::new(),
            common: Vec::new(),
            stack: None,
            n_nets: 0,
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
/// item has not landed yet stay zero: `width_fallbacks` (RTE-22),
/// `single_cut_vias` (RTE-27). `congestion` is per-region demand after
/// repair (absolute nm; see `congestion`).
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
    pub single_cut_vias: u32,
    pub congestion: Vec<(Rect, f32)>,
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

        let compact: Vec<usize> = (0..n_nets).filter(|&i| !term_rects[i].is_empty()).collect();
        if compact.is_empty() {
            let routes = Routes { wires: vec![Vec::new(); n_nets], ..Default::default()  };
            let report = score(&routes, reqs, 0.0, &[], &[], &[], 1);
            return (routes, report, RouteStats::default());
        }
        let mut ci_of = vec![usize::MAX; n_nets];
        for (ci, &ni) in compact.iter().enumerate() {
            ci_of[ni] = ci;
        }
        let n_compact = compact.len();

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

        for (ci, &ni) in compact.iter().enumerate() {
            let known = term_rects[ni].iter().all(|t| t.2.is_some());
            for &(r, r_layer, ua) in &term_rects[ni] {
                // A net with any unknown terminal gets no EM sizing at all.
                let ua = ua.filter(|_| known);
                let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2);
                // Never land inside another net's stitch reservation.
                let mine = |n: u32| [NONE, CONTESTED, ci as u32].contains(&reserved[n as usize]);
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
                            let f = (0..2).find(|&f| jog_clean(&both[f], ci, zone, gap, &zones, &laid_legs))?;
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
                        let infl = cfg.wire_width / 2;
                        let touches = |n: &u32| {
                            let (px, py, _) = grid.pos(*n);
                            legs.iter().any(|l| (l.x - infl..=l.x + l.w + infl).contains(&px) && (l.y - infl..=l.y + l.h + infl).contains(&py))
                        };
                        let foreign = |n: &u32| reserved[*n as usize] != ci as u32 && reserved[*n as usize] < BLOCKED;
                        jog_hist.extend(jog_nodes(&grid, cfg, &legs, jog_l).filter(foreign).filter(touches));
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
            // Series R is priced only once a drop budget is broken (below).
            current: Vec::new(),
            layer_r: cfg.layer_r.clone(),
            via_r: cfg.via_r.clone(),
            plain: {
                let sym = gr::symmetric_nets(reqs);
                net_ids.iter().map(|n| sym.contains(n)).collect()
            },
            keepout: Vec::new(),
            own_cells: Vec::new(),
            k: ks,
            guard: guards,
            term_k,
        };
        // Matched cells (units of more than one member): foreign nets pay to
        // cross them, their own nets (finger straps, drains) do not.
        let matched: Vec<usize> = (0..placed.len())
            .filter(|&c| placed[c].units.iter().any(|u| u.owner != placed[c].units[0].owner))
            .collect();
        if !matched.is_empty() {
            let mut keep = vec![NONE; cold.graph.nodes()];
            for &c in &matched {
                let r = shift(placed[c].bbox);
                for l in 0..n_layers {
                    for y in cold.graph.bin_y(r.y)..=cold.graph.bin_y(r.y + r.h) {
                        for x in cold.graph.bin_x(r.x)..=cold.graph.bin_x(r.x + r.w) {
                            keep[cold.graph.node(x, y, l) as usize] = c as u32;
                        }
                    }
                }
            }
            cold.keepout = keep;
            cold.own_cells = compact
                .iter()
                .map(|&n| matched.iter().filter(|&&c| placed[c].pins.iter().any(|p| p.net.0 as usize == n)).map(|&c| c as u32).collect())
                .collect();
        }
        let mut hot = RouteHot::new(cold.graph.nodes(), n_compact);
        hot.set_weights(cold.weight.clone());
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
        let probe = |hot: &RouteHot| Routes { cell: cell_f.clone(), gates: gates_f.clone(), ..build_routes(hot, &cold, cfg, &compact, n_nets, layers, cuts) };
        // Common-node balance, in the routing frame.
        let common: Vec<analog::routing::CommonNode> = cfg
            .common
            .iter()
            .map(|n| analog::routing::CommonNode {
                a: n.a.iter().map(|&r| shift(r)).collect(),
                b: n.b.iter().map(|&r| shift(r)).collect(),
                feeds: n.feeds.iter().map(|&r| shift(r)).collect(),
                ..n.clone()
            })
            .collect();
        let extra: Vec<Box<dyn analog::RuleBatch<Routes>>> = match cfg.stack {
            Some(stack) if !common.is_empty() => vec![Box::new(analog::routing::CommonNodes { nodes: common.clone(), stack })],
            _ => Vec::new(),
        };
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
        stats.trials = repair_constraints(&mut hot, &cold, reqs, &extra, &common, &ci_of, &lift, probe, &mut dij);

        // The jog price is this layout's, not negotiation history.
        jog_hist.iter().for_each(|&n| hot.hist[n as usize] -= JOG_HIST);
        stats.congestion = congestion(&hot, &cold.graph, origin);
        neg.accumulate(&hot.hist, abs);
        // Shields: requested nets get reference tracks alongside, tied in by
        // rerouting the reference to them.
        let mut asks = Vec::new();
        for b in reqs.hard.iter().chain(&reqs.budget) {
            b.shield_pairs(&mut asks);
        }
        let ci = |n: u32| ci_of.get(n as usize).copied().filter(|&c| c != usize::MAX);
        for (v, rf) in asks {
            if let (Some(v), Some(rf)) = (ci(v), ci(rf)) {
                add_shield(&mut hot, &mut cold, v, rf, &mut dij);
            }
        }
        stats.us_repair = us(t_repair);
        let joins = joins(layers, cuts, cfg.pin_access);
        let mut round = 0;
        let (routes, sacrificed, access_v) = loop {
            let t_geometry = std::time::Instant::now();
            let mut routes = build_routes(&hot, &cold, cfg, &compact, n_nets, layers, cuts);
            // Shapes at or past this index per net are access geometry — the only
            // shapes the short resolver may sacrifice.
            let pre_access: Vec<usize> = routes.wires.iter().map(Vec::len).collect();
            let access_v = add_pin_access(&mut routes, &access, &compact, cfg, layers, cuts, &joins, &zones, origin);

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
                    if nx * ny < 2 {
                        out.push(*c);
                        continue;
                    }
                    // Centred in the overlap, snapped to the manufacturing grid.
                    let snap = |v: i32| v.div_euclid(cfg.grid) * cfg.grid;
                    let x0 = snap(x + (w - (nx - 1) * pitch - size) / 2);
                    let y0 = snap(y + (h - (ny - 1) * pitch - size) / 2);
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
                    placed.extend_from_slice(&out[before..]);
                }
                *wires = out;
            }

            stats.us_geometry += us(t_geometry);
            let t_fill = std::time::Instant::now();
            // Same-net sliver and notch filling (never within spacing of foreign metal).
            let flat: Vec<(usize, Shape)> =
                routes.wires.iter().enumerate().flat_map(|(i, w)| w.iter().map(move |s| (i, *s))).collect();
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
            for (ni, wires) in routes.wires.iter_mut().enumerate() {
                let mine = cell_of(ni);
                let foreign: Vec<Shape> = flat
                    .iter()
                    .filter(|&&(i, _)| i != ni)
                    .map(|&(_, s)| s)
                    .chain((0..n_nets).filter(|&n| n != ni).flat_map(|n| term_rects[n].iter().map(|&(r, l, _)| Shape { layer: l, rect: r })))
                    .chain(fill_metal.iter().copied().filter(|c| !mine.contains(c)))
                    .collect();
                let own = wires.len();
                wires.extend(mine);
                let pins = wires.len() - own;
                // Per layer: its own spacing (met3's is not met1's) and min width.
                for &l in &fill_layers {
                    let floor = if cfg.pin_access.is_some_and(|(p, _)| p == l) { min_space.max(cfg.pin_access_spacing) } else { min_space };
                    let space = cfg.space(l, 0, 0, floor);
                    let feat = cfg.min_width.iter().find(|&&(m, _)| m == l).map_or(cfg.wire_width, |&(_, w)| w);
                    heal_same_net_slivers(wires, &[l], space, feat, &foreign, cfg.grid);
                    fill_same_net_notches(wires, &[l], space, feat, &foreign, cfg.grid);
                }
                wires.drain(own..own + pins);
                drop_contained(wires);
                for s in wires.iter_mut() {
                    s.rect.x += origin.0;
                    s.rect.y += origin.1;
                }
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
                let rule = analog::routing::Electromigration { net, limits, stack: Some(stack) };
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
                    // The root's own branch draws nothing: the next one starts there.
                    let ts: Vec<usize> = hot.trees[ci]
                        .iter()
                        .filter(|b| b.iter().any(inside))
                        .filter_map(|b| b.last().and_then(|n| cold.terms[ci].iter().position(|x| x == n)).filter(|&t| t > 0))
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
                        let own: Vec<usize> = ts.iter().filter(drawn).copied().collect();
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
        let overuse = overuse(&hot);
        let mut report = score(&routes, reqs, overuse, &joins, &foreign_metal, &sacrificed, side);
        report.hard_violations.extend(access_v);
        (stats.overuse, stats.expanded, stats.coarsened) = (overuse, dij.pops, cold.graph.coarsened);
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
                        figures: cell.figures,
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
/// already laid.
fn jog_clean(legs: &[Rect; 2], ci: usize, zone: i32, gap: i32, zones: &[(u32, i32, i32)], laid: &[(usize, Rect)]) -> bool {
    zones.iter().all(|&(zci, zx, zy)| {
        let zr = Rect { x: zx - zone / 2, y: zy - zone / 2, w: zone, h: zone };
        zci as usize == ci || legs.iter().all(|&l| rect_gap(l, zr) >= gap.max(1))
    }) && laid.iter().all(|&(lci, lr)| lci == ci || legs.iter().all(|&l| rect_gap(l, lr) > 0))
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
    let width = a.choice.map_or_else(|| cfg.wire_width.min(a.pin.w).min(a.pin.h).max(1), |(w, _)| w);
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
                jog_clean(&legs, a.ci, pad_zone, gap, zones, &laid) && clear(&legs, gap)
            })
        };
        let (w, flip) = pick(spacing).or_else(|| pick(1)).or(choice).unwrap_or_else(|| {
            options.into_iter().find(|&c| jog_clean(&legs_of(c), a.ci, full, 1, zones, &laid)).unwrap_or(options[3])
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
        // Resized about the stamped rect's centre.
        let centred = |r: Rect, w: i32, h: i32| Rect { x: r.x + (r.w - w) / 2, y: r.y + (r.h - h) / 2, w, h };
        let mut pads = Vec::new();
        for v in &vs {
            let i = v.layer.0 as usize;
            let Some(&(cut, size, below, above)) = cuts.get(i) else { continue };
            dst.push(Shape { layer: cut, rect: centred(v.rect, size, size) });
            for (k, square) in [(i, below), (i + 1, above)] {
                let Some(&metal) = layers.get(k) else { continue };
                let (w, h) = match cfg.layers.get(k) {
                    Some(s) if s.horizontal => (s.pad_along, s.pad_across),
                    Some(s) => (s.pad_across, s.pad_along),
                    None => (square, square),
                };
                pads.push(Shape { layer: metal, rect: centred(v.rect, w, h) });
            }
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
/// * `KeepAway`, pairs named (`CrosstalkExclusion`): reroute the victim, else
///   the aggressor, priced away from the other's tracks; victim only
///   (`CouplingBudget`): reroute each victim priced away from all foreign tracks;
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
/// lexicographically without raising overuse. `probe` draws the current state.
#[allow(clippy::too_many_arguments)]
fn repair_constraints(
    hot: &mut RouteHot,
    cold: &RouteCtx<TrackGrid>,
    reqs: &Requirements<Routes>,
    extra: &[Box<dyn analog::RuleBatch<Routes>>],
    common: &[analog::routing::CommonNode],
    ci_of: &[usize],
    lift: &[Option<(u32, Vec<((i32, i32), Rect)>)>],
    probe: impl Fn(&RouteHot) -> Routes,
    dij: &mut Dij,
) -> u32 {
    let key = |hot: &RouteHot| {
        let r = probe(hot);
        let hard: u32 = reqs.hard.iter().map(|b| b.violations(&r)).sum();
        let residual: f64 = reqs.hard.iter().chain(&reqs.budget).chain(extra).map(|b| b.residual(&r)).sum();
        (hard, residual, overuse(hot))
    };
    let ci = |n: u32| ci_of.get(n as usize).copied().filter(|&c| c != usize::MAX);
    let (mut p_fac, mut n) = (P_FAC, 0);
    for _ in 0..HARD_ROUNDS {
        p_fac *= 2.0;
        let routes = probe(hot);
        let violated: Vec<_> = reqs
            .hard
            .iter()
            .filter(|b| b.violations(&routes) > 0)
            .chain(reqs.budget.iter().chain(extra).filter(|b| b.residual(&routes) > 0.0))
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
                    for &(a, b) in &pairs {
                        for (from, to) in [(a, b), (b, a)] {
                            if let Some(tree) = copy_tree(hot, cold, from, to) {
                                n += 1;
                                accepted |= commit_trial(hot, cold, to, tree, &key);
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
                            trials.push(vec![(c, balance_field(&cold.graph, &n.a, &n.b))]);
                        }
                    }
                }
                // A rule naming its pairs (`CrosstalkExclusion`): reroute the
                // victim, else the aggressor, away from the other. A victim-only
                // rule (`CouplingBudget`): away from all foreign tracks.
                RepairKind::KeepAway => {
                    let mut named = Vec::new();
                    batch.keepaway_pairs(&mut named);
                    if named.is_empty() {
                        for v in ids.iter().filter_map(|&n| ci(n)) {
                            let others: Vec<usize> = (0..hot.trees.len()).filter(|&o| o != v).collect();
                            trials.push(vec![(v, keep_away(hot, &cold.graph, &others))]);
                        }
                    }
                    for (a, b) in named.into_iter().filter_map(|(a, b)| Some((ci(a)?, ci(b)?))) {
                        trials.push(vec![(a, keep_away(hot, &cold.graph, &[b]))]);
                        trials.push(vec![(b, keep_away(hot, &cold.graph, &[a]))]);
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
                accepted |= trial(hot, cold, t, p_fac, &key, dij);
            }
        }
        if !accepted {
            break;
        }
    }
    n
}

/// Rip up every net in `reroutes`, reroute each with its penalty field, and keep
/// the result only if `key` improves on (hard, budget) without raising overuse;
/// otherwise restore the old trees. Returns whether it was kept.
fn trial(
    hot: &mut RouteHot,
    cold: &RouteCtx<TrackGrid>,
    reroutes: Vec<(usize, Vec<f32>)>,
    p_fac: f32,
    key: &impl Fn(&RouteHot) -> (u32, f64, f32),
    dij: &mut Dij,
) -> bool {
    if reroutes.is_empty() {
        return false;
    }
    let before = key(hot);
    let old: Vec<(usize, Vec<Vec<u32>>)> = reroutes.iter().map(|(n, _)| (*n, hot.trees[*n].clone())).collect();
    for &(n, _) in &old {
        cold.commit(hot, n, Vec::new());
    }
    for ((n, field), (_, prev)) in reroutes.into_iter().zip(&old) {
        let tree = cold.reroute(hot, n, p_fac, &field, dij).unwrap_or_else(|| prev.clone());
        cold.commit(hot, n, tree);
    }
    let after = key(hot);
    let better = after.2 <= before.2 && (after.0, after.1) < (before.0, before.1);
    if !better {
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
/// join the reference tree. All-or-nothing: if the reroute fails or adds
/// overuse, nothing changes.
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
    if claims.is_empty() {
        return;
    }
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
        tree.extend(claims);
        cold.commit(hot, reference, tree);
        if overuse(hot) <= over0 {
            return;
        }
        cold.commit(hot, reference, old_tree);
    }
    cold.terms[reference] = old_terms;
    for (i, o) in old_reserved {
        if let Some(r) = cold.reserved.get_mut(i) {
            *r = o;
        }
    }
}

/// Keep `tree` as `net`'s route iff `key` improves on (hard, budget) without
/// raising overuse; else restore. Returns whether it was kept.
fn commit_trial(hot: &mut RouteHot, cold: &RouteCtx<TrackGrid>, net: usize, tree: Vec<Vec<u32>>, key: &impl Fn(&RouteHot) -> (u32, f64, f32)) -> bool {
    let before = key(hot);
    let old = hot.trees[net].clone();
    cold.commit(hot, net, tree);
    let after = key(hot);
    let better = after.2 <= before.2 && (after.0, after.1) < (before.0, before.1);
    if !better {
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
        analog::routing::Electromigration { net: NetId(net), limits, stack: Some(test_stack()) }
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
        let node = CommonNode { net: NetId(0), a: vec![a.1], b: vec![b.1], feeds: vec![feed.1], max_delta_ohm: 0.2 };
        let skew = |cfg: DetailedCfg| {
            let (routes, _) = route(cfg, &[feed, a, b], &[], &[], &mut gr::Negotiation::new());
            CommonNodes { nodes: vec![node.clone()], stack }.worst_usage(&routes).unwrap()
        };
        let plain = skew(test_cfg());
        let balanced = skew(DetailedCfg { common: vec![node.clone()], stack: Some(stack), ..test_cfg() });
        assert!(balanced < plain * 0.5, "skew {plain} → {balanced}");
    }

    /// A matched cell (units of two members) is priced for foreign nets only:
    /// net 0, straight across the cell otherwise, goes around it; net 1, with
    /// a pin in the cell, still runs over it.
    #[test]
    fn foreign_nets_route_around_a_matched_cell() {
        let cell = Rect { x: 6_000, y: 3_000, w: 4_000, h: 4_000 };
        let unit = |owner| pnr_core::Unit { owner, x: 0, y: 0, weight: 1, phi: (1, 0), sa: 0, sb: 0 };
        let matched = Macro {
            shapes: Vec::new(),
            pins: vec![pnr_core::Pin { name: "d0:D".into(), net: NetId(1), at: Rect { x: 8_000, y: 5_000, w: 1, h: 1 }, layer: LAYERS[0] }],
            bbox: cell,
            units: vec![unit(0), unit(1)],
            dummies: Vec::new(),
            ..Default::default()
        };
        let pins = [pin(0, 1_000, 5_000), pin(0, 15_000, 5_000), pin(1, 8_000, 12_000)];
        let (routes, _) = route(test_cfg(), &pins, &[matched], &[], &mut gr::Negotiation::new());
        let over = |n: usize| routes.wires[n].iter().any(|s| s.rect.x < cell.x + cell.w && cell.x < s.rect.x + s.rect.w && s.rect.y < cell.y + cell.h && cell.y < s.rect.y + s.rect.h);
        assert!(!over(0), "net 0 crossed the matched cell: {:?}", routes.wires[0]);
        assert!(over(1), "net 1 must reach its pin inside the cell");
    }

    /// History survives the call and changes the next one.
    #[test]
    fn negotiation_persists_across_calls() {
        let cfg = DetailedCfg { pitch: 1_300, ..test_cfg() };
        // Five nets whose terminals all sit within one pitch of the same two rows:
        // every net wants the same horizontal track.
        let pins: Vec<(NetId, Rect, LayerId)> = (0..5u16)
            .flat_map(|n| {
                let off = i32::from(n) * 130;
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
                self.0.antenna(r.shapes(NetId(0)), &[], &[], 1_000_000).is_none_or(|(x, l)| x <= l)
            }
            fn residual(self, r: &Routes) -> f32 {
                self.0.antenna(r.shapes(NetId(0)), &[], &[], 1_000_000).map_or(0.0, |(x, l)| (x / l - 1.0).max(0.0))
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
        add_pin_access(&mut routes, &access, &[0], &cfg, &LAYERS, &CUTS, &joins, &[], (0, 0));
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
    /// `W(2) = 260 + 420` and `280 + 420`, and the bend's 700×680 corner holds
    /// a 2×2 via array (85 nm along-wire enclosure, 170 nm cut spacing).
    #[test]
    fn a_two_by_two_corner_gets_four_cuts() {
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
        assert!(report.hard_violations.is_empty(), "{:?}", rules(&report));
        let w = &routes.wires[0];
        let widest = |l: LayerId| w.iter().filter(|s| s.layer == l && s.rect.w != s.rect.h).map(|s| s.rect.w.min(s.rect.h)).max();
        assert_eq!((widest(LAYERS[0]), widest(LAYERS[1])), (Some(680), Some(700)));
        // The bend: the corner block holding no pin centre.
        let pins: Vec<(i32, i32)> = cell.pins.iter().map(|p| (p.at.x + 85, p.at.y + 85)).collect();
        let inside = |r: Rect, (x, y): (i32, i32)| x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h;
        let bends: Vec<Rect> = w.iter().filter(|s| s.layer == LAYERS[0] && (s.rect.w, s.rect.h) == (700, 680)).map(|s| s.rect).filter(|&r| !pins.iter().any(|&p| inside(r, p))).collect();
        assert!(!bends.is_empty(), "no corner block");
        for b in bends {
            let n = w.iter().filter(|s| s.layer == via && contains(b, s.rect)).count();
            assert_eq!(n, 4, "cuts in the corner {b:?}");
        }
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
