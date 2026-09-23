//! # `dr` — detailed routing.
//!
//! [`DetailedRoute`] rebuilds a capacity-1 track lattice over the die, lands one
//! track node per placed pin, runs `gr`'s negotiated-congestion PathFinder (early
//! epochs restricted to the corridor the coarse route threads), then draws the
//! geometry: track wires and vias, an L-shaped access jog from each landed node to
//! its pin (the pitch is far coarser than a pin, so nodes never sit on pins), and
//! same-net sliver/notch fillers.
//!
//! What it cannot rule out structurally it measures and reports as hard
//! violations: open nets, unlanded pins or pin access deleted to break a short,
//! and drawn shorts.

use std::collections::HashSet;

use analog::Requirements;
use pnr_core::geom::{LayerId, Rect, Shape};
use pnr_core::report::Violation;
use pnr_core::{Macro, NetId, Report, Routes};

use gr::{extract_geometry, run_pathfinder, to_shapes, Dij, GcellGrid, RGraph, RouteCtx, RouteHot, TrackGrid, NONE};

/// A via: `(cut layer, cut size, pad below, pad above)`, nm.
pub type Cut = (LayerId, i32, i32, i32);

const VIA_COST: f32 = 4.0;
const P_FAC: f32 = 2.0;
const HIST_INC: f32 = 0.5;
const MAX_ITERS: u32 = 150;
/// Must match `gr::GlobalCfg::gcells_per_side` so corridors line up.
const GCELLS_PER_SIDE: u32 = 16;
/// Rip-up rounds for nets named by violated hard rules, each at doubled `p_fac`.
const HARD_ROUNDS: u32 = 4;
/// Everything routed sits on the 5 nm manufacturing grid.
const MFG_GRID: i32 = 5;
/// DC electromigration limit of the routing metals, µA per µm of wire width
/// (sky130 met1/met2 ≈ 1 mA/µm).
const EM_UA_PER_UM: u64 = 1_000;

/// Track-lattice configuration (set from the deck by `frontend/library`).
#[derive(Debug, Clone)]
pub struct DetailedCfg {
    pub pitch: i32,
    pub wire_width: i32,
    /// `min_spacing` of the layer landing pads sit on; `0` disables the check.
    pub pin_access_spacing: i32,
    /// `min_spacing` of the pin-access cut; `0` disables the check.
    pub pin_access_cut_spacing: i32,
    /// Conductor below the routing stack that cells put pins on (li), and the cut
    /// up to `layers[0]`. Reserving it keeps tracks off the cells' own li. `None`:
    /// pins sit on a stack layer.
    pub pin_access: Option<(LayerId, Cut)>,
    /// DC current each net carries, µA, indexed by `NetId` (operating point).
    /// Trunks are widened to `current / EM_UA_PER_UM`. Empty = unknown.
    pub net_current_ua: Vec<i32>,
    /// Supply/ground nets: fattened first, toward `fat_supply`.
    pub supply_nets: Vec<NetId>,
    /// Widest a signal / supply trunk is fattened to when room allows, nm.
    pub fat_signal: i32,
    pub fat_supply: i32,
    /// Per routing layer: `(min_spacing, [(width threshold, spacing)])` — the
    /// deck's width-dependent spacing, used when fattening.
    pub spacing: Vec<(LayerId, i32, Vec<(i32, i32)>)>,
}

impl Default for DetailedCfg {
    fn default() -> Self {
        Self {
            pitch: 0,
            wire_width: 0,
            pin_access_spacing: 0,
            pin_access_cut_spacing: 0,
            pin_access: None,
            net_current_ua: Vec::new(),
            supply_nets: Vec::new(),
            fat_signal: 0,
            fat_supply: 0,
            spacing: Vec::new(),
        }
    }
}

impl DetailedCfg {
    /// The width `net`'s trunks are fattened toward when there is room.
    fn fat_width(&self, net: usize) -> i32 {
        if self.supply_nets.iter().any(|n| n.0 as usize == net) { self.fat_supply } else { self.fat_signal }
    }

    /// Spacing two same-layer shapes of widths `a`, `b` need on `layer`; `fallback`
    /// when the layer has no entry.
    fn space(&self, layer: LayerId, a: i32, b: i32, fallback: i32) -> i32 {
        let Some((_, min, steps)) = self.spacing.iter().find(|(l, ..)| *l == layer) else {
            return fallback;
        };
        let w = a.max(b);
        steps.iter().filter(|&&(t, _)| w >= t).map(|&(_, s)| s).fold(*min, i32::max)
    }

    /// EM-safe trunk width for `net`, nm, on the manufacturing grid (at least
    /// `wire_width`).
    fn em_width(&self, net: usize) -> i32 {
        let need = match self.net_current_ua.get(net) {
            Some(&ua) => (u64::from(ua.unsigned_abs()) * 1_000).div_ceil(EM_UA_PER_UM) as i32,
            None => 0,
        };
        let step = 2 * MFG_GRID;
        (need.max(self.wire_width) + step - 1) / step * step
    }
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
}

impl DetailedRoute {
    /// Realise `global` over the metal stack `layers` (even index = horizontal),
    /// joined by `cuts[i]` between `layers[i]` and `layers[i + 1]`.
    ///
    /// `pins` are placed pin rects; pins of `placed` (device macros) and `rings`
    /// are folded in, deduplicated. Ring metal is charged as track usage (routed
    /// around); device cells are not (routed to). Both are consulted for landing-pad
    /// spacing. `neg` carries PathFinder history across calls.
    #[allow(clippy::too_many_arguments)]
    pub fn route(
        &self,
        global: &Routes,
        pins: &[(NetId, Rect, LayerId)],
        placed: &[Macro],
        rings: &[Macro],
        reqs: &Requirements<Routes>,
        layers: &[LayerId],
        cuts: &[Cut],
        neg: &mut gr::Negotiation,
    ) -> (Routes, Report) {
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
        let n_nets = global.wires.len().max(all_pins.iter().map(|(n, ..)| n.0 as usize + 1).max().unwrap_or(0));
        // Climbing to a layer with no cut to reach it buys only an island.
        let n_layers = (layers.len() as u32).min(cuts.len() as u32 + 1);

        // Routing frame: `TrackGrid` starts at (0,0) and placed geometry reaches
        // negative coordinates, so shift. The frame keeps a claim-free halo (so jog
        // reservations can never wall off a whole band) plus a free margin (the
        // cheapest trunk highways).
        let (halo, margin) = (2 * cfg.pitch, 3 * cfg.pitch);
        let low = |f: fn(&Rect) -> i32| {
            let pins = all_pins.iter().map(|(_, r, _)| f(r));
            pins.chain(global.wires.iter().flatten().map(|s| f(&s.rect))).min().unwrap_or(0).min(0)
        };
        let origin = (low(|r| r.x) - halo - margin, low(|r| r.y) - halo - margin);
        let shift = |r: Rect| Rect { x: r.x - origin.0, y: r.y - origin.1, ..r };

        let mut term_rects: Vec<Vec<(Rect, LayerId)>> = vec![Vec::new(); n_nets];
        for &(net, r, l) in &all_pins {
            term_rects[net.0 as usize].push((shift(r), l));
        }
        let (mut hi_x, mut hi_y) = (2, 2);
        for s in global.wires.iter().flatten() {
            hi_x = hi_x.max(s.rect.x + s.rect.w);
            hi_y = hi_y.max(s.rect.y + s.rect.h);
        }
        (hi_x, hi_y) = (hi_x - origin.0 + margin, hi_y - origin.1 + margin);
        for (r, _) in term_rects.iter().flatten() {
            hi_x = hi_x.max(r.x + r.w);
            hi_y = hi_y.max(r.y + r.h);
        }
        let die = (hi_x + halo, hi_y + halo);

        let compact: Vec<usize> = (0..n_nets).filter(|&i| !term_rects[i].is_empty()).collect();
        if compact.is_empty() {
            let routes = Routes { wires: vec![Vec::new(); n_nets] };
            let report = score(&routes, reqs, 0.0, 0.0, &[], &[], &[]);
            return (routes, report);
        }
        let mut ci_of = vec![usize::MAX; n_nets];
        for (ci, &ni) in compact.iter().enumerate() {
            ci_of[ni] = ci;
        }
        let n_compact = compact.len();

        let ggrid = GcellGrid::new(die, GCELLS_PER_SIDE, 1);
        let mut grid = TrackGrid::with_layers(die, cfg.pitch, VIA_COST, n_layers);
        grid.set_regions(&ggrid);

        let mut claimed = vec![false; grid.nodes()];
        let mut reserved = vec![NONE; grid.nodes()];
        let mut c_terms: Vec<Vec<u32>> = vec![Vec::new(); n_compact];
        let mut corridors: Vec<Vec<u32>> = vec![Vec::new(); n_compact];
        let mut access: Vec<Access> = Vec::new();
        // Pins that got no node, per compact net: an open the geometry cannot show.
        let mut unlanded = vec![0usize; n_compact];

        // Every pin grows a stitch pad; a foreign jog through that zone is a short.
        let zones: Vec<(u32, i32, i32)> = all_pins
            .iter()
            .map(|&(n, r, _)| (ci_of[n.0 as usize] as u32, r.x + r.w / 2 - origin.0, r.y + r.h / 2 - origin.1))
            .collect();
        let mut laid_legs: Vec<(usize, Rect)> = Vec::new();

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
        let reach = (cfg.wire_width + stitch) / 2;
        for &(n, r, _) in &all_pins {
            let (pcx, pcy) = (r.x + r.w / 2 - origin.0, r.y + r.h / 2 - origin.1);
            for iy in ((pcy - reach) / cfg.pitch).max(0)..=((pcy + reach) / cfg.pitch).min(grid.ny as i32 - 1) {
                for ix in ((pcx - reach) / cfg.pitch).max(0)..=((pcx + reach) / cfg.pitch).min(grid.nx as i32 - 1) {
                    let node = grid.node(ix as u32, iy as u32, 0);
                    let (px, py, _) = grid.pos(node);
                    if (px - pcx).abs() <= reach && (py - pcy).abs() <= reach && reserved[node as usize] == NONE {
                        reserved[node as usize] = ci_of[n.0 as usize] as u32;
                    }
                }
            }
        }

        for (ci, &ni) in compact.iter().enumerate() {
            for &(r, r_layer) in &term_rects[ni] {
                let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2);
                // Never land inside another net's stitch reservation.
                let mine = |n: u32| reserved[n as usize] == NONE || reserved[n as usize] == ci as u32;
                let at = |n: u32| {
                    let (px, py, _) = grid.pos(n);
                    (px, py)
                };
                let short_ok = |n: u32| mine(n) && short_free(ci, r, at(n).0, at(n).1);
                let clean = |n: u32| short_ok(n) && good_site((cx, cy), at(n).0, at(n).1);
                // Joint node + jog choice first: the nearest candidate whose jog
                // (full or narrow width, either orientation) clears every foreign
                // zone and every jog already laid.
                let full = cfg.wire_width.max(1);
                let narrow = full.min(r.w).min(r.h).max(1);
                // A point terminal (1×1 rect) is its own node: nothing to jog to.
                let point = r.w <= 1 && r.h <= 1;
                let cands = if point { Vec::new() } else { grid.candidates(cx, cy, &claimed, clean, 8, 12) };
                let joint = cands.into_iter().find_map(|n| {
                    let (px, py, _) = grid.pos(n);
                    [full, narrow].into_iter().find_map(|w| {
                        let both = jog_legs(px, py, cx, cy, w);
                        let f = (0..2).find(|&f| jog_clean(&both[f], ci, full, &zones, &laid_legs))?;
                        Some((n, (w, f == 1), both[f]))
                    })
                });
                // Fallback tiers, tight radii (a far node means a long blind jog):
                // clean, then short-free, then anything. Not landing beats a short.
                let landed = match joint {
                    Some((n, choice, legs)) => {
                        laid_legs.extend(legs.iter().map(|&l| (ci, l)));
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
                if !c_terms[ci].contains(&n) {
                    c_terms[ci].push(n);
                }
                if !point {
                    let (px, py, node_layer) = grid.pos(n);
                    let a = Access { ci, pin: r, pin_layer: r_layer, node: (px, py), node_layer, choice };
                    claim_jog_sweep(&grid, cfg, layers, cuts, &a, &mut claimed, &mut reserved);
                    access.push(a);
                }
            }
            // Corridor: the 3×3 gcells around every coarse-route point and every pin.
            let coarse = global.wires.get(ni).into_iter().flatten().flat_map(|s| {
                let r = shift(s.rect);
                [(r.x, r.y), (r.x + r.w, r.y + r.h)]
            });
            let pin_c = term_rects[ni].iter().map(|(r, _)| (r.x + r.w / 2, r.y + r.h / 2));
            let mut corr: Vec<u32> = Vec::new();
            for (x, y) in coarse.chain(pin_c) {
                let g = ggrid.at(x as f32, y as f32);
                let (gx, gy) = ((g % ggrid.nx) as i64, (g / ggrid.nx) as i64);
                for (tx, ty) in (-1..=1).flat_map(|dy| (-1..=1).map(move |dx| (gx + dx, gy + dy))) {
                    if tx >= 0 && ty >= 0 && tx < ggrid.nx as i64 && ty < ggrid.ny as i64 {
                        corr.push((ty * ggrid.nx as i64 + tx) as u32);
                    }
                }
            }
            corr.sort_unstable();
            corr.dedup();
            corridors[ci] = corr;
        }

        let cold = RouteCtx {
            graph: grid,
            terms: c_terms,
            order: (0..n_compact as u32).collect(),
            corridors,
            reserved,
        };
        let mut hot = RouteHot::new(cold.graph.nodes(), n_compact);
        // History is keyed by absolute position: add the frame shift back.
        let abs = |n: u32| {
            let (x, y, l) = cold.graph.pos(n);
            (x + origin.0, y + origin.1, l)
        };
        neg.seed(gr::Tier::Detailed, &mut hot.hist, abs);
        // Ring bands are charged to capacity (full but crossable at a price), not
        // blocked: a hard block would seal the enclosure on a two-layer stack.
        for s in rings.iter().flat_map(|m| &m.shapes) {
            if let Some(li) = layers.iter().position(|&l| l == s.layer).filter(|&li| (li as u32) < n_layers) {
                charge_rect(&mut hot.usage, &cold.reserved, &cold.graph, li as u32, shift(s.rect));
            }
        }
        run_pathfinder(&mut hot, &cold, P_FAC, HIST_INC, MAX_ITERS);

        // Constraint repair: the analog rules never make a net congestion-dirty, so
        // PathFinder alone ignores them. Rip up what they name, reroute steered by
        // the rule's own data, keep only what improves the rules.
        let probe = |hot: &RouteHot| build_routes(hot, &cold.graph, cfg.wire_width, &compact, n_nets, layers, cuts);
        repair_constraints(&mut hot, &cold, reqs, &ci_of, probe);
        neg.accumulate(gr::Tier::Detailed, &hot.hist, abs);
        let overuse = overuse(&hot);

        let mut routes = build_routes(&hot, &cold.graph, cfg.wire_width, &compact, n_nets, layers, cuts);
        // Shapes at or past this index per net are access geometry — the only
        // shapes the short resolver may sacrifice.
        let mut pre_access: Vec<usize> = routes.wires.iter().map(Vec::len).collect();
        let joins = joins(layers, cuts, cfg.pin_access);
        add_pin_access(&mut routes, &access, &compact, cfg, layers, cuts, &joins, &zones);

        // A drawn short must never ship. Break each by deleting access geometry (an
        // open is reported; a short is silent), counted per net so `score` reports
        // pins that may now float.
        let mut sacrificed = vec![0usize; n_nets];
        for (ci, &n) in unlanded.iter().enumerate() {
            sacrificed[compact[ci]] += n;
        }
        while let Some((a, b, i, j)) = first_short(&routes.wires, &joins) {
            if j >= pre_access[b] {
                routes.wires[b].remove(j);
                sacrificed[b] += 1;
            } else if i >= pre_access[a] {
                routes.wires[a].remove(i);
                sacrificed[a] += 1;
            } else {
                // Trunk vs trunk: unexpected on a capacity-1 lattice; drop the later
                // net's shape and keep `pre_access` pointing at its access shapes.
                routes.wires[b].remove(j);
                pre_access[b] -= 1;
                sacrificed[b] += 1;
            }
        }

        // Fatten: every trunk grows to the widest width (≤ its net's cap) that keeps
        // `min_space` from foreign metal — other nets' wires and cell/ring metal on
        // the same layer. EM width is the floor; a trunk that cannot reach it goes to
        // Θ. Supply nets widen first, so they win contested room.
        let min_space = cfg.pitch - cfg.wire_width;
        let cell_metal: Vec<Shape> = placed
            .iter()
            .chain(rings)
            .flat_map(|m| &m.shapes)
            .filter(|s| layers.contains(&s.layer))
            .map(|s| Shape { rect: shift(s.rect), ..*s })
            .collect();
        let mut em_shortfall = vec![0.0f64; n_nets];
        let mut order: Vec<usize> = (0..n_nets).collect();
        order.sort_by_key(|&n| std::cmp::Reverse(cfg.fat_width(n)));
        for net in order {
            let (need, cap) = (cfg.em_width(net), cfg.fat_width(net).max(cfg.em_width(net)));
            for i in 0..pre_access[net] {
                let s = routes.wires[net][i];
                let narrow = s.rect.w.min(s.rect.h);
                if narrow >= cap || s.rect.w == s.rect.h || !layers.contains(&s.layer) {
                    continue;
                }
                let narrow_of = |r: Rect| r.w.min(r.h);
                let gap_ok = |f: &Shape, g: &Shape| {
                    rect_gap(f.rect, g.rect) >= cfg.space(g.layer, narrow_of(f.rect), narrow_of(g.rect), min_space)
                };
                let clear = |g: &Shape| {
                    routes.wires.iter().enumerate().all(|(n, w)| {
                        n == net || w.iter().all(|f| !conductor_layers_meet(f, g, &joins) || f.layer != g.layer && rect_gap(f.rect, g.rect) >= min_space || f.layer == g.layer && gap_ok(f, g))
                    }) && cell_metal.iter().all(|c| {
                        // Cell metal the original trunk already touches is its own pin.
                        c.layer != g.layer || rect_gap(c.rect, s.rect) <= 0 || gap_ok(c, g)
                    })
                };
                let step = 2 * MFG_GRID;
                let best = (0..)
                    .map(|k| cap - k * step)
                    .take_while(|&w| w > narrow)
                    .map(|w| Shape { rect: widen(s.rect, w), ..s })
                    .find(|g| clear(g));
                if let Some(g) = best {
                    routes.wires[net][i] = g;
                }
                let got = best.map_or(narrow, |g| g.rect.w.min(g.rect.h));
                if got < need {
                    em_shortfall[net] = em_shortfall[net].max(f64::from(need - got) / f64::from(need));
                }
            }
        }

        // Same-net sliver and notch filling (never within spacing of foreign metal).
        let flat: Vec<(usize, Shape)> =
            routes.wires.iter().enumerate().flat_map(|(i, w)| w.iter().map(move |s| (i, *s))).collect();
        for (ni, wires) in routes.wires.iter_mut().enumerate() {
            let foreign: Vec<Shape> = flat.iter().filter(|&&(i, _)| i != ni).map(|&(_, s)| s).collect();
            heal_same_net_slivers(wires, layers, min_space, cfg.wire_width, &foreign);
            fill_same_net_notches(wires, layers, min_space, cfg.wire_width, &foreign);
            drop_contained(wires);
            for s in wires.iter_mut() {
                s.rect.x += origin.0;
                s.rect.y += origin.1;
            }
        }

        let cap_total = cold.graph.nodes() as f32;
        let report = score(&routes, reqs, overuse, cap_total, &joins, &sacrificed, &em_shortfall);
        (routes, report)
    }
}

/// `r` with its narrow (cross-run) dimension grown to `w` about its centre line.
/// Ends extend by the added half-width so an L-corner of two widened trunks fills.
fn widen(r: Rect, w: i32) -> Rect {
    if r.w > r.h {
        let e = (w - r.h) / 2;
        Rect { x: r.x - e, y: r.y + r.h / 2 - w / 2, w: r.w + 2 * e, h: w }
    } else {
        let e = (w - r.w) / 2;
        Rect { x: r.x + r.w / 2 - w / 2, y: r.y - e, w, h: r.h + 2 * e }
    }
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
fn jog_clean(legs: &[Rect; 2], ci: usize, zone: i32, zones: &[(u32, i32, i32)], laid: &[(usize, Rect)]) -> bool {
    zones.iter().all(|&(zci, zx, zy)| {
        let zr = Rect { x: zx - zone / 2, y: zy - zone / 2, w: zone, h: zone };
        zci as usize == ci || legs.iter().all(|&l| rect_gap(l, zr) > 0)
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
    let infl = cfg.wire_width / 2;
    for r in &legs {
        for gy in grid.bin_y(r.y - infl)..=grid.bin_y(r.y + r.h + infl) {
            for gx in grid.bin_x(r.x - infl)..=grid.bin_x(r.x + r.w + infl) {
                claim(grid.node(gx, gy, jog_l));
            }
        }
    }
    if jog_l != base_l {
        for (ex, ey) in [(px, py), (cx, cy)] {
            claim(grid.node(grid.bin_x(ex), grid.bin_y(ey), base_l));
        }
    }
}

/// Charge every unreserved node of `layer` under `r` to capacity (soft obstacle).
/// Reserved nodes are exempt: a ring's own pins sit on its bands.
fn charge_rect(usage: &mut [u16], reserved: &[u32], grid: &TrackGrid, layer: u32, r: Rect) {
    for y in grid.bin_y(r.y)..=grid.bin_y(r.y + r.h) {
        for x in grid.bin_x(r.x)..=grid.bin_x(r.x + r.w) {
            let n = grid.node(x, y, layer) as usize;
            if reserved[n] == NONE {
                usage[n] = usage[n].max(1);
            }
        }
    }
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
) {
    let spacing = (cfg.pitch - cfg.wire_width).max(0);
    let mut laid: Vec<(usize, Rect)> = Vec::new();
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
        let full = cfg.wire_width.max(1);
        let narrow = full.min(a.pin.w).min(a.pin.h).max(1);
        let legs_of = |(w, flip): (i32, bool)| jog_legs(nx, ny, px, py, w)[usize::from(flip)];
        let options = [(full, false), (full, true), (narrow, false), (narrow, true)];
        let cands: Vec<(i32, bool)> = a.choice.into_iter().chain(options).collect();
        let clear = |legs: &[Rect; 2], gap: i32| {
            let probe = Shape { layer: metal, rect: legs[0] };
            routes.wires.iter().enumerate().all(|(n, w)| {
                n == net
                    || w.iter().all(|s| {
                        !conductor_layers_meet(s, &probe, joins) || legs.iter().all(|&l| rect_gap(l, s.rect) >= gap)
                    })
            })
        };
        let pick = |gap: i32| {
            cands.iter().copied().find(|&c| {
                let legs = legs_of(c);
                jog_clean(&legs, a.ci, full, zones, &laid) && clear(&legs, gap)
            })
        };
        let (w, flip) = pick(spacing).or_else(|| pick(1)).or(a.choice).unwrap_or_else(|| {
            options.into_iter().find(|&c| jog_clean(&legs_of(c), a.ci, full, zones, &laid)).unwrap_or(options[3])
        });
        let legs = legs_of((w, flip));
        laid.extend(legs.iter().map(|&l| (a.ci, l)));
        let wires = &mut routes.wires[net];
        let pad = |l: LayerId, (x, y): (i32, i32), s: i32| Shape { layer: l, rect: Rect { x: x - s / 2, y: y - s / 2, w: s, h: s } };
        wires.push(Shape { layer: metal, rect: legs[0] });
        if (if flip { nx - px } else { ny - py }) != 0 {
            wires.push(Shape { layer: metal, rect: legs[1] });
        }
        if let Some((_, cut, size, below, above, stitch_node)) = climb {
            // Node inside the pin's own pad: the pads merge, so a second cut would
            // only be a cut-spacing violation.
            let merged = (nx - px).abs().max((ny - py).abs()) < below.min(above);
            let node_end = (stitch_node && !merged).then_some((nx, ny));
            for end in node_end.into_iter().chain([(px, py)]) {
                wires.push(pad(cut, end, size));
                // A cut that fits inside the pin is hosted by the cell's own
                // conductor; a base pad there only adds li spacing violations.
                let (cl, ct) = (end.0 - size / 2, end.1 - size / 2);
                let hosted = end == (px, py)
                    && cl >= a.pin.x
                    && cl + size <= a.pin.x + a.pin.w
                    && ct >= a.pin.y
                    && ct + size <= a.pin.y + a.pin.h;
                if !hosted {
                    wires.push(pad(base, end, below));
                }
                wires.push(pad(metal, end, above));
            }
        }
        // Node on layers[1]: extraction draws nothing for a one-node run, so stitch
        // the node up to the route explicitly.
        if a.node_layer == 1 {
            if let (Some(&up), Some(&(cut1, size1, below1, above1))) = (layers.get(1), cuts.first()) {
                wires.push(pad(cut1, (nx, ny), size1));
                wires.push(pad(metal, (nx, ny), below1));
                wires.push(pad(up, (nx, ny), above1));
            }
        }
    }
}

/// Current routing state as per-net shapes on real PDK layers: track wires on
/// `layers[i]`, vias as `cuts[i]` resized to the deck's exact cut size, then a
/// landing pad on each side of every cut (sized to satisfy enclosure alone).
fn build_routes(
    hot: &RouteHot,
    grid: &TrackGrid,
    wire_width: i32,
    compact: &[usize],
    n_nets: usize,
    layers: &[LayerId],
    cuts: &[Cut],
) -> Routes {
    let (wires, vias) = extract_geometry(hot, grid, wire_width);
    let mut out = vec![Vec::new(); n_nets];
    let wire_shapes = to_shapes(compact.len(), &wires, &[]);
    let via_shapes = to_shapes(compact.len(), &[], &vias);
    for (ci, (ws, vs)) in wire_shapes.into_iter().zip(via_shapes).enumerate() {
        let dst = &mut out[compact[ci]];
        dst.extend(ws.into_iter().filter_map(|s| Some(Shape { layer: *layers.get(s.layer.0 as usize)?, ..s })));
        // Resized about the stamped rect's centre.
        let centred = |r: Rect, s: i32| Rect { x: r.x + (r.w - s) / 2, y: r.y + (r.h - s) / 2, w: s, h: s };
        let mut pads = Vec::new();
        for v in &vs {
            let i = v.layer.0 as usize;
            let Some(&(cut, size, below, above)) = cuts.get(i) else { continue };
            dst.push(Shape { layer: cut, rect: centred(v.rect, size) });
            for (metal, pad) in [(layers.get(i), below), (layers.get(i + 1), above)] {
                if let Some(&metal) = metal {
                    pads.push(Shape { layer: metal, rect: centred(v.rect, pad) });
                }
            }
        }
        dst.append(&mut pads);
    }
    assert_on_stack(&out, layers, cuts);
    Routes { wires: out }
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

/// Residual track overuse `Σ max(0, usage − 1)`.
fn overuse(hot: &RouteHot) -> f32 {
    hot.usage.iter().map(|&u| f32::from(u.saturating_sub(1))).sum()
}

/// Extra cost per node for a mirrored net off its partner's mirror image.
const GUIDE_COST: f32 = 1.0;
/// Extra cost per node adjacent (same layer, one track) to an aggressor.
const COUPLE_COST: f32 = 2.0;

/// Rip-up/reroute trials driven by violated routing rules, up to `HARD_ROUNDS`
/// rounds while a trial is accepted. Per rule kind:
///
/// * `Differential`: reroute one side along the mirror image of the other
///   (matched length and layers);
/// * `CrosstalkExclusion`: reroute the victim, else the aggressor, priced away
///   from the other's tracks;
/// * `CouplingBudget`: reroute each victim priced away from all foreign tracks;
/// * anything else: reroute the nets it names at doubled `p_fac`.
///
/// A trial is kept only if `(hard violations, Σ residual)` improves
/// lexicographically without raising overuse. `probe` draws the current state.
fn repair_constraints(
    hot: &mut RouteHot,
    cold: &RouteCtx<TrackGrid>,
    reqs: &Requirements<Routes>,
    ci_of: &[usize],
    probe: impl Fn(&RouteHot) -> Routes,
) {
    let key = |hot: &RouteHot| {
        let r = probe(hot);
        let hard: u32 = reqs.hard.iter().map(|b| b.violations(&r)).sum();
        let residual: f64 = reqs.hard.iter().chain(&reqs.budget).map(|b| b.residual(&r)).sum();
        (hard, residual, overuse(hot))
    };
    let ci = |n: u32| ci_of.get(n as usize).copied().filter(|&c| c != usize::MAX);
    let mut p_fac = P_FAC;
    for _ in 0..HARD_ROUNDS {
        p_fac *= 2.0;
        let routes = probe(hot);
        let violated: Vec<_> = reqs
            .hard
            .iter()
            .filter(|b| b.violations(&routes) > 0)
            .chain(reqs.budget.iter().filter(|b| b.residual(&routes) > 0.0))
            .collect();
        let mut accepted = false;
        for batch in violated {
            let mut ids = Vec::new();
            batch.touched(&mut ids);
            let pairs: Vec<(usize, usize)> =
                ids.chunks_exact(2).filter_map(|p| Some((ci(p[0])?, ci(p[1])?))).collect();
            let mut trials: Vec<Vec<(usize, Vec<f32>)>> = Vec::new();
            match batch.kind().rsplit("::").next().unwrap_or("") {
                "Differential" => {
                    for &(a, b) in &pairs {
                        trials.extend(mirror_guide(hot, &cold.graph, &cold.terms, a, b).map(|g| vec![(b, g)]));
                        trials.extend(mirror_guide(hot, &cold.graph, &cold.terms, b, a).map(|g| vec![(a, g)]));
                    }
                }
                "CrosstalkExclusion" => {
                    for &(a, b) in &pairs {
                        trials.push(vec![(a, keep_away(hot, &cold.graph, &[b]))]);
                        trials.push(vec![(b, keep_away(hot, &cold.graph, &[a]))]);
                    }
                }
                "CouplingBudget" => {
                    for v in ids.iter().filter_map(|&n| ci(n)) {
                        let others: Vec<usize> = (0..hot.trees.len()).filter(|&o| o != v).collect();
                        trials.push(vec![(v, keep_away(hot, &cold.graph, &others))]);
                    }
                }
                _ => {
                    ids.clear();
                    batch.violating_ids(&routes, &mut ids);
                    ids.sort_unstable();
                    ids.dedup();
                    trials.push(ids.iter().filter_map(|&n| ci(n)).map(|n| (n, Vec::new())).collect());
                }
            }
            for t in trials {
                accepted |= trial(hot, cold, t, p_fac, &key);
            }
        }
        if !accepted {
            return;
        }
    }
}

/// Rip up every net in `reroutes`, reroute each with its penalty field (corridor
/// first), and keep the result only if `key` improves on (hard, budget) without
/// raising overuse; otherwise restore the old trees. Returns whether it was kept.
fn trial(
    hot: &mut RouteHot,
    cold: &RouteCtx<TrackGrid>,
    reroutes: Vec<(usize, Vec<f32>)>,
    p_fac: f32,
    key: &impl Fn(&RouteHot) -> (u32, f64, f32),
) -> bool {
    if reroutes.is_empty() {
        return false;
    }
    let before = key(hot);
    let old: Vec<(usize, Vec<Vec<u32>>)> = reroutes.iter().map(|(n, _)| (*n, hot.trees[*n].clone())).collect();
    for &(n, _) in &old {
        hot.commit(n, Vec::new());
    }
    let mut dij = Dij::new(cold.graph.nodes());
    for ((n, field), (_, prev)) in reroutes.into_iter().zip(&old) {
        let tree = cold.reroute(hot, n, true, p_fac, &field, &mut dij).unwrap_or_else(|| prev.clone());
        hot.commit(n, tree);
    }
    let after = key(hot);
    let better = after.2 <= before.2 && (after.0, after.1) < (before.0, before.1);
    if !better {
        for (n, tree) in old {
            hot.commit(n, tree);
        }
    }
    better
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

/// Report: analog tiers; open nets, sacrificed/unlanded pins and drawn shorts in
/// V; EM width shortfall per net and residual overuse over total track capacity
/// in Θ; cost = raw overuse +
/// criticality-weighted analog cost.
fn score(
    routes: &Routes,
    reqs: &Requirements<Routes>,
    overuse: f32,
    cap_total: f32,
    joins: &[Join],
    sacrificed: &[usize],
    em_shortfall: &[f64],
) -> Report {
    let (mut hard, mut budget) = gr::analog_tiers(routes, reqs);
    for (net, shapes) in routes.wires.iter().enumerate() {
        let stranded = unreachable_shapes(shapes);
        if stranded > 0 {
            hard.push(Violation { rule: format!("open net {net}"), margin: stranded as i64 });
        }
    }
    // Counted, not proven: the pin may still be tied through the cell, so this
    // over-reports and never under-reports.
    for (net, &n) in sacrificed.iter().enumerate().filter(|(_, &n)| n > 0) {
        hard.push(Violation { rule: format!("pin access sacrificed on net {net}"), margin: n as i64 });
    }
    for (a, b) in cross_net_shorts(&routes.wires, joins) {
        hard.push(Violation { rule: format!("drawn short nets {a}/{b}"), margin: 1 });
    }
    for (net, &r) in em_shortfall.iter().enumerate().filter(|(_, &r)| r > 0.0) {
        budget.push(Violation::from_residual(format!("em underwidth net {net}"), r));
    }
    if overuse > 0.0 && cap_total > 0.0 {
        budget.push(Violation::from_residual("routing overuse", f64::from(overuse / cap_total)));
    }
    let analog_cost: f32 = reqs.cost.iter().map(|b| b.criticality(routes) * b.cost(routes)).sum();
    Report { hard_violations: hard, budget_violations: budget, cost: overuse + analog_cost }
}

/// Snap `r` outward onto the manufacturing grid.
fn snap_out(r: Rect) -> Rect {
    let x = r.x.div_euclid(MFG_GRID) * MFG_GRID;
    let y = r.y.div_euclid(MFG_GRID) * MFG_GRID;
    let w = ((r.x + r.w) - x + MFG_GRID - 1).div_euclid(MFG_GRID) * MFG_GRID;
    let h = ((r.y + r.h) - y + MFG_GRID - 1).div_euclid(MFG_GRID) * MFG_GRID;
    Rect { x, y, w, h }
}

/// Bridge sub-`min_space` gaps between pairs of one net's same-layer pieces:
/// aligned gaps with a full-hull-width span, corner gaps with a patch reaching
/// `min_feat` into both. Skips pairs already bridged and fillers that would come
/// within `min_space` of `foreign`. Up to four passes.
fn heal_same_net_slivers(shapes: &mut Vec<Shape>, layers: &[LayerId], min_space: i32, min_feat: i32, foreign: &[Shape]) {
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
                let rect = if dx <= 0 {
                    let (g0, g1) = ((a.y + a.h).min(b.y + b.h), a.y.max(b.y));
                    span_fill(hx0, hx1, g0, g1, min_feat, false)
                } else if dy <= 0 {
                    let (g0, g1) = ((a.x + a.w).min(b.x + b.w), a.x.max(b.x));
                    span_fill(hy0, hy1, g0, g1, min_feat, true)
                } else {
                    let x0 = ((a.x + a.w).min(b.x + b.w) - min_feat).max(hx0);
                    let x1 = (a.x.max(b.x) + min_feat).min(hx1);
                    let y0 = ((a.y + a.h).min(b.y + b.h) - min_feat).max(hy0);
                    let y1 = (a.y.max(b.y) + min_feat).min(hy1);
                    Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
                };
                let bridged = shapes.iter().chain(&fillers).any(|s| {
                    s.layer == layer && s.rect != a && s.rect != b && touches(s.rect, a) && touches(s.rect, b)
                });
                let filler = snap_out(rect);
                let fouls = foreign.iter().any(|f| f.layer == layer && rect_gap(filler, f.rect) < min_space);
                if !bridged && !fouls && rect.w > 0 && rect.h > 0 {
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

/// Aligned-gap filler: span `[p0, p1]` (grown to `min_feat` within itself),
/// across gap `[g0, g1]` extended `min_feat` into each side. `swap` puts the gap
/// on x.
fn span_fill(p0: i32, p1: i32, g0: i32, g1: i32, min_feat: i32, swap: bool) -> Rect {
    let (lo, hi) = (p0, p1);
    let (mut p0, mut p1) = (p0, p1);
    if p1 - p0 < min_feat {
        p0 = (p0 - (min_feat - (p1 - p0)) / 2).max(lo);
        p1 = (p0 + min_feat).min(hi);
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
fn fill_same_net_notches(shapes: &mut Vec<Shape>, layers: &[LayerId], min_space: i32, min_feat: i32, foreign: &[Shape]) {
    if min_space <= 0 {
        return;
    }
    let mut present: Vec<LayerId> = shapes.iter().map(|s| s.layer).filter(|l| layers.contains(l)).collect();
    present.sort_unstable_by_key(|l| l.0);
    present.dedup();
    let snap = |v: i32| v.div_euclid(MFG_GRID) * MFG_GRID;
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
                    let (fx, fw) = grow(xs[i], w);
                    let (fy, fh) = grow(ys[j], h);
                    let rect = Rect { x: fx, y: fy, w: fw, h: fh };
                    if !foreign.iter().any(|f| f.layer == layer && rect_gap(rect, f.rect) < min_space) {
                        fillers.push(Shape { layer, rect });
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

/// Two shapes conduct into each other when touching: same layer, or one is a cut
/// joining the other's layer.
fn conductor_layers_meet(a: &Shape, b: &Shape, joins: &[Join]) -> bool {
    let joined = |cut: LayerId, other: LayerId| joins.iter().any(|&(c, lo, hi)| c == cut && (other == lo || other == hi));
    a.layer == b.layer || joined(a.layer, b.layer) || joined(b.layer, a.layer)
}

/// `(cut, layer below, layer above)`.
type Join = (LayerId, LayerId, LayerId);

/// Every cut this stage draws and the two conductors it joins — the routing vias
/// and the pin-access cut down to the reserved pin layer.
fn joins(layers: &[LayerId], cuts: &[Cut], pin_access: Option<(LayerId, Cut)>) -> Vec<Join> {
    let stack = cuts.iter().zip(layers.windows(2)).map(|(&(c, ..), w)| (c, w[0], w[1]));
    let access = pin_access.zip(layers.first()).map(|((pl, (c, ..)), &l0)| (c, pl, l0));
    stack.chain(access).collect()
}

/// First `(net a, net b, shape i of a, shape j of b)` in (a, b, i, j) order whose
/// shapes touch on a shared conductor.
fn first_short(wires: &[Vec<Shape>], joins: &[Join]) -> Option<(usize, usize, usize, usize)> {
    for a in 0..wires.len() {
        for b in (a + 1)..wires.len() {
            for (i, sa) in wires[a].iter().enumerate() {
                for (j, sb) in wires[b].iter().enumerate() {
                    if conductor_layers_meet(sa, sb, joins) && rect_gap(sa.rect, sb.rect) == 0 {
                        return Some((a, b, i, j));
                    }
                }
            }
        }
    }
    None
}

/// Every net pair whose geometry touches on one conductor (a cut counts on both
/// layers it joins).
fn cross_net_shorts(wires: &[Vec<Shape>], joins: &[Join]) -> Vec<(usize, usize)> {
    let footprint = |shapes: &[Shape]| -> Vec<(LayerId, Rect)> {
        let mut out = Vec::with_capacity(shapes.len());
        for s in shapes {
            match joins.iter().find(|j| j.0 == s.layer) {
                Some(&(_, lo, hi)) => out.extend([(lo, s.rect), (hi, s.rect)]),
                None => out.push((s.layer, s.rect)),
            }
        }
        out
    };
    let nets: Vec<Vec<(LayerId, Rect)>> = wires.iter().map(|w| footprint(w)).collect();
    let mut out = Vec::new();
    for a in 0..nets.len() {
        for b in (a + 1)..nets.len() {
            if nets[a].iter().any(|&(la, ra)| nets[b].iter().any(|&(lb, rb)| la == lb && rect_gap(ra, rb) == 0)) {
                out.push((a, b));
            }
        }
    }
    out
}

/// Shapes not reachable from the first by layer-blind xy-touch (`0` = one
/// component), the same relation `Routes::debug_check` asserts.
fn unreachable_shapes(shapes: &[Shape]) -> usize {
    if shapes.len() < 2 {
        return 0;
    }
    let mut seen = vec![false; shapes.len()];
    let mut stack = vec![0usize];
    seen[0] = true;
    while let Some(a) = stack.pop() {
        let ra = shapes[a].rect;
        for (b, s) in shapes.iter().enumerate() {
            let rb = s.rect;
            if !seen[b] && ra.x <= rb.x + rb.w && rb.x <= ra.x + ra.w && ra.y <= rb.y + rb.h && rb.y <= ra.y + ra.h {
                seen[b] = true;
                stack.push(b);
            }
        }
    }
    seen.iter().filter(|&&v| !v).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// sky130-like lattice: the unit tests draw fixed geometry against it.
    fn test_cfg() -> DetailedCfg {
        DetailedCfg { pitch: 430, wire_width: 290, ..DetailedCfg::default() }
    }

    const LAYERS: [LayerId; 2] = [LayerId(0), LayerId(1)];
    const CUTS: [Cut; 1] = [(LayerId(2), 100, 140, 140)];

    fn pin(net: u16, x: i32, y: i32) -> (NetId, Rect, LayerId) {
        (NetId(net), Rect { x, y, w: 170, h: 170 }, LAYERS[0])
    }

    fn touches(s: &Shape, r: Rect) -> bool {
        s.rect.x <= r.x + r.w && r.x <= s.rect.x + s.rect.w && s.rect.y <= r.y + r.h && r.y <= s.rect.y + s.rect.h
    }

    fn route(
        cfg: DetailedCfg,
        global: &Routes,
        pins: &[(NetId, Rect, LayerId)],
        placed: &[Macro],
        rings: &[Macro],
        neg: &mut gr::Negotiation,
    ) -> (Routes, Report) {
        let reqs = Requirements::<Routes>::default();
        DetailedRoute { cfg }.route(global, pins, placed, rings, &reqs, &LAYERS, &CUTS, neg)
    }

    fn rules(r: &Report) -> Vec<&String> {
        r.hard_violations.iter().map(|v| &v.rule).collect()
    }

    #[test]
    fn detailed_realises_a_two_pin_net() {
        let global = Routes {
            wires: vec![vec![Shape { layer: LayerId(0), rect: Rect { x: 1_000, y: 1_000, w: 17_000, h: 17_000 } }]],
        };
        let pins = [pin(0, 1_000, 1_000), pin(0, 18_000, 18_000)];
        let (routes, report) = route(test_cfg(), &global, &pins, &[], &[], &mut gr::Negotiation::new());
        assert_eq!(routes.wires.len(), 1);
        assert!(!routes.wires[0].is_empty());
        assert!(report.hard_violations.is_empty(), "{:?}", rules(&report));
        assert!(routes.wires[0].iter().all(|s| s.rect.w > 0 && s.rect.h > 0));
    }

    #[test]
    fn empty_coarse_gives_empty_routes() {
        let global = Routes { wires: vec![Vec::new(); 3] };
        let (routes, report) = route(test_cfg(), &global, &[], &[], &[], &mut gr::Negotiation::new());
        assert_eq!(routes.wires.len(), 3);
        assert!(routes.wires.iter().all(Vec::is_empty));
        assert!(report.hard_violations.is_empty());
    }

    /// Every pin rect is touched by its own net, with pins deliberately centred a
    /// half-pitch off every track.
    #[test]
    fn every_pin_is_touched_by_its_own_net() {
        let pins = [pin(0, 1_805, 1_805), pin(0, 15_035, 13_145), pin(1, 3_695, 15_035), pin(1, 13_145, 3_695)];
        let seg = |x, y, w, h| Shape { layer: LayerId(0), rect: Rect { x, y, w, h } };
        let global =
            Routes { wires: vec![vec![seg(1_100, 1_000, 16_500, 15_200)], vec![seg(2_300, 2_700, 13_700, 12_400)]] };
        let cfg = DetailedCfg { pitch: 1_890, ..test_cfg() };
        let (routes, _) = route(cfg, &global, &pins, &[], &[], &mut gr::Negotiation::new());
        for &(net, r, _) in &pins {
            let hit = routes.wires[net.0 as usize].iter().any(|s| touches(s, r));
            assert!(hit, "net {} pin at ({}, {}) untouched", net.0, r.x, r.y);
        }
    }

    /// A ring is an obstacle and a target: its pins get routed, and charging its
    /// bands must not price its own landing pads into a Θ residual.
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
        };
        let global = Routes { wires: Vec::new() };
        let (routes, report) =
            route(test_cfg(), &global, &[], &[], &[ring.clone()], &mut gr::Negotiation::new());
        for p in &ring.pins {
            assert!(routes.wires[0].iter().any(|s| touches(s, p.at)), "ring pin at ({}, {}) untouched", p.at.x, p.at.y);
        }
        assert!(report.budget_violations.is_empty());
    }

    /// Placed cells do not block routing under their (possibly inflated) bbox.
    #[test]
    fn inflated_bbox_does_not_block_routing() {
        let pins = [pin(0, 1_000, 7_000), pin(0, 15_000, 7_000)];
        let global = Routes {
            wires: vec![vec![Shape { layer: LayerId(0), rect: Rect { x: 1_000, y: 6_600, w: 14_200, h: 1_000 } }]],
        };
        let wall = Macro {
            shapes: vec![Shape { layer: LayerId(0), rect: Rect { x: 7_000, y: 200, w: 500, h: 500 } }],
            pins: Vec::new(),
            bbox: Rect { x: 6_000, y: -5_000, w: 3_000, h: 30_000 },
        };
        let (routes, report) =
            route(test_cfg(), &global, &pins, &[wall], &[], &mut gr::Negotiation::new());
        assert!(!rules(&report).iter().any(|r| r.starts_with("open net")), "{:?}", rules(&report));
        for &(_, r, _) in &pins {
            assert!(routes.wires[0].iter().any(|s| touches(s, r)));
        }
    }

    /// `score` reads `reqs.budget` on `gr`'s scale.
    #[test]
    fn budget_residual_reaches_theta() {
        let routes =
            Routes { wires: vec![vec![Shape { layer: LayerId(0), rect: Rect { x: 0, y: 0, w: 1_500, h: 1 } }]] };
        let mut reqs = Requirements::<Routes>::default();
        reqs.budget.push(Box::new(vec![analog::routing::ParasiticBudget {
            net: NetId(0),
            max_len_nm: 1_000,
            margin_pct: 10,
        }]));
        let report = score(&routes, &reqs, 0.0, 0.0, &[], &[], &[]);
        assert_eq!(report.budget_violations[0].margin, 500);
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
        let global = Routes { wires: vec![Vec::new(); 5] };
        let flat = |r: &Routes| -> Vec<(u16, i32, i32, i32, i32)> {
            r.wires.iter().flatten().map(|s| (s.layer.0, s.rect.x, s.rect.y, s.rect.w, s.rect.h)).collect()
        };
        let mut neg = gr::Negotiation::new();
        let first = route(cfg.clone(), &global, &pins, &[], &[], &mut neg).0;
        let p1 = neg.pressure();
        assert!(p1 > 0.0, "contested tracks must accumulate history");
        let second = route(cfg, &global, &pins, &[], &[], &mut neg).0;
        assert!(neg.pressure() > p1, "history must keep climbing");
        assert_ne!(flat(&first), flat(&second), "second call was not seeded");
    }

    /// A net carrying 1 mA gets 1 µm trunks; the default net keeps `wire_width`.
    #[test]
    fn high_current_net_is_widened() {
        let global = Routes { wires: vec![Vec::new(); 2] };
        let pins = [pin(0, 1_000, 1_000), pin(0, 12_000, 1_000), pin(1, 1_000, 9_000), pin(1, 12_000, 9_000)];
        let cfg = DetailedCfg { net_current_ua: vec![1_000, 10], ..test_cfg() };
        let (routes, report) = route(cfg, &global, &pins, &[], &[], &mut gr::Negotiation::new());
        let widest = |n: usize| routes.wires[n].iter().filter(|s| s.rect.w != s.rect.h).map(|s| s.rect.w.min(s.rect.h)).max();
        assert_eq!(widest(0), Some(1_000));
        assert_eq!(widest(1), Some(290));
        assert!(report.budget_violations.is_empty(), "{:?}", report.budget_violations.iter().map(|v| &v.rule).collect::<Vec<_>>());
    }

    /// The guide for `b` is free exactly on the mirror image of `a`'s tree, and
    /// absent when the terminals are not mirror images.
    #[test]
    fn mirror_guide_follows_the_partner() {
        let grid = TrackGrid::with_layers((10_000, 10_000), 1_000, VIA_COST, 2);
        let n = |ix, iy| grid.node(ix, iy, 0);
        let terms = vec![vec![n(1, 2), n(3, 2)], vec![n(8, 2), n(6, 2)], vec![n(8, 5), n(6, 2)]];
        let mut hot = RouteHot::new(grid.nodes(), 3);
        hot.commit(0, vec![vec![n(1, 2), n(2, 2), n(3, 2)]]);
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
        let global = Routes { wires: vec![Vec::new(); 4] };
        let (routes, _) = route(test_cfg(), &global, &pins, &[], &[], &mut gr::Negotiation::new());
        let hits = |a: Rect, b: Rect| a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
        for (i, na) in routes.wires.iter().enumerate() {
            for nb in routes.wires.iter().skip(i + 1) {
                for wa in na {
                    assert!(nb.iter().all(|wb| wa.layer != wb.layer || !hits(wa.rect, wb.rect)));
                }
            }
        }
    }
}
