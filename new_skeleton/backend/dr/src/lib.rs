//! # `dr` — detailed routing (an Algorithm-contract stage)
//!
//! [`DetailedRouter`] realises the global routes as concrete track/via geometry,
//! honouring the same routing [`Requirements`]. Drop-in; the active algorithm is
//! selected in `frontend/library`'s `algorithms` module.
//!
//! ## What this stage does (docs/routing/model-and-algorithms.md: Track graph,
//! Pin landing, Detailed PathFinder, Geometry extraction)
//!
//! Take the coarse [`Routes`] from `gr`, rebuild the fine track lattice over the
//! same die, land one track node per coarse terminal, restrict early PathFinder
//! epochs to the corridor implied by the coarse route, run the same negotiated-
//! congestion loop at capacity 1, then extract DRC-clean centre-line wire + via
//! `Shape`s (min-area-extended). This reuses `gr`'s shared mechanics
//! (`grid`/`pathfinder`/`geom`) so both stages share one PathFinder core.
//!
//! ## Contract note (terminals)
//!
//! `dr` takes `pins: &[(NetId, Rect)]` — the **placed** pin rects — alongside the
//! coarse route. It used to take only `global: &Routes`, and derived terminals
//! from the corners of the coarse wire rects, which are gcell-snapped
//! approximations of a pin, not the pin. The module doc called that a deliberate
//! contract; it was the reason nothing ever connected. Measured on `pair` net 0:
//! the coarse plan reached within 31 nm of the pin and the realised route ended
//! 1265 nm away, so every device extracted onto its own island of nets and LVS
//! could never match.
//!
//! Terminals are now landed from the pin **outward** — `TrackGrid::claim_minarea`
//! walks Chebyshev rings around the pin centre — and the residual node-to-pin
//! offset is closed by a drawn access jog (see [`add_pin_access`]). Landing
//! *inside* the rect is not achievable at these numbers and demanding it only made
//! terminals unlandable: the pitch is set by the worst routed layer's spacing while
//! a pin is one cut wide, so a track node essentially never falls within a pin. The
//! coarse route keeps its two other jobs: the die extent and the corridor that
//! restricts early PathFinder epochs.
//!
//! `pins` empty ⇒ the old coarse-corner derivation, so a caller with no placement
//! (and this crate's own unit tests) still routes something.
//!
//! ## Objective / legality (ANALOG-BLIND)
//!
//! [`Report`] cost = built-in mechanics (residual track overuse) **plus**
//! `reqs.cost.iter().map(|b| b.cost(&routes)).sum()`; legality (V) counts
//! `reqs.hard` violations, and residual track overuse is a **budget** (Θ) carrying
//! its real residual. No analog term is hardcoded.

#![allow(dead_code)]

use analog::Requirements;
use pnr_core::geom::{LayerId, Rect, Shape};
use pnr_core::{Macro, NetId, Report, Routes};

use gr::{
    eol_violations, extract_geometry_minarea, prl_violations, route_net, run_pathfinder, to_shapes,
    Dij, GcellGrid, RGraph, RouteCtx, RouteHot, TrackGrid, NONE,
};

/// Track-lattice tuning (docs: Track graph / Detailed PathFinder). Defaults mirror
/// the ported `DetailedRouteCfg`.
#[derive(Debug, Clone, Copy)]
pub struct DetailedCfg {
    pub pitch: i32,
    pub wire_width: i32,
    pub via_cost: f32,
    pub max_iters: u32,
    pub p_fac: f32,
    pub hist_inc: f32,
    pub min_area: i64,
    pub eol_spacing: i32,
    pub wide_net_extra_spacing: i32,
    pub n_layers: u32,
    /// Gcells per side used to rebuild the corridor regions — must match `gr`.
    pub gcells_per_side: u32,
    /// Extra negotiation rounds run when an **analog hard** routing rule
    /// (crosstalk, antenna, differential) is still violated after the first
    /// PathFinder convergence. Each round re-negotiates at higher present-cost
    /// pressure on the *accumulated* history. `0` disables the loop.
    pub hard_rounds: u32,
    /// Present-cost multiplier applied per hard round.
    pub hard_escalation: f32,
}

impl Default for DetailedCfg {
    fn default() -> Self {
        Self {
            // 430/290 keeps met1 DRC-clean by construction (ported default).
            pitch: 430,
            wire_width: 290,
            via_cost: 4.0,
            max_iters: 150,
            p_fac: 2.0,
            hist_inc: 0.5,
            min_area: 0,
            eol_spacing: 0,
            wide_net_extra_spacing: 0,
            n_layers: 2,
            gcells_per_side: 16,
            hard_rounds: 4,
            hard_escalation: 2.0,
        }
    }
}

/// A detailed-routing algorithm.
pub trait DetailedRouter {
    /// Realise `global` routes into final geometry over the PDK-permitted metal
    /// stack `layers`, deterministically for `seed`. The track lattice uses one
    /// track layer per entry in `layers` (even = horizontal, odd = vertical).
    ///
    /// Wire `Shape`s carry the real [`LayerId`] from `layers`; a layer change
    /// emits a cut on `cuts[i]`, the via joining `layers[i]` to `layers[i + 1]`.
    /// `cuts` shorter than `layers.len() - 1` means the geometry cannot be made
    /// electrically whole, so the layer change is not taken (see
    /// `Pdk::routing_vias`).
    ///
    /// `pins` are the **placed** pin rects (`gr::place_macros` output), which is
    /// what a terminal actually has to land on. Pass `&[]` to fall back to the
    /// coarse route's own corners.
    ///
    /// `rings` are this epoch's guard rings — obstacle (shapes block tracks) and
    /// target (pins carry real `NetId`s). Separate from `pins` because a ring is
    /// drawn geometry that also blocks, which a bare `(NetId, Rect)` cannot say.
    /// Placement-derived, so handed in fresh every epoch.
    ///
    /// `neg` is the negotiation history, carried across outer epochs and owned by
    /// the orchestrator — see [`gr::Negotiation`]. `dr` runs its own negotiation
    /// over track resources; sharing the type with `gr` is what lets the caller
    /// read one `pressure()` for the whole routing tier.
    fn route(
        &self,
        global: &Routes,
        pins: &[(NetId, Rect)],
        rings: &[Macro],
        reqs: &Requirements<Routes>,
        layers: &[LayerId],
        cuts: &[(LayerId, i32, i32, i32)],
        neg: &mut gr::Negotiation,
        seed: u64,
    ) -> (Routes, Report);
}

/// The default drop-in; replaced at the single selection point in `frontend/library`.
#[derive(Default)]
pub struct Placeholder;

impl DetailedRouter for Placeholder {
    fn route(
        &self,
        global: &Routes,
        pins: &[(NetId, Rect)],
        rings: &[Macro],
        reqs: &Requirements<Routes>,
        layers: &[LayerId],
        cuts: &[(LayerId, i32, i32, i32)],
        neg: &mut gr::Negotiation,
        seed: u64,
    ) -> (Routes, Report) {
        DetailedRoute::default().route(global, pins, rings, reqs, layers, cuts, neg, seed)
    }
}

/// The real detailed router.
pub struct DetailedRoute {
    pub cfg: DetailedCfg,
}

impl Default for DetailedRoute {
    fn default() -> Self {
        Self {
            cfg: DetailedCfg::default(),
        }
    }
}

impl DetailedRouter for DetailedRoute {
    fn route(
        &self,
        global: &Routes,
        pins: &[(NetId, Rect)],
        rings: &[Macro],
        reqs: &Requirements<Routes>,
        layers: &[LayerId],
        cuts: &[(LayerId, i32, i32, i32)],
        neg: &mut gr::Negotiation,
        _seed: u64,
    ) -> (Routes, Report) {
        // PathFinder is deterministic for identical inputs (docs: determinism);
        // `seed` selects nothing. `neg` does: the same corridors with more history
        // realise onto different tracks.
        let cfg = self.cfg;

        // Rings as TARGET (D5): a band's `ring` pin carries a real `NetId` and has
        // to be reached like any other pin, or the ring is an island in the
        // extraction. Folded into `pins` because from here on a terminal is a
        // `(NetId, Rect)` and a ring pin is nothing else; rings are already
        // absolute, so there is no placement step to skip or repeat.
        let all_pins: Vec<(NetId, Rect)> = pins
            .iter()
            .copied()
            .chain(rings.iter().flat_map(|m| m.pins.iter().map(|p| (p.net, p.at))))
            .collect();
        let pins: &[(NetId, Rect)] = &all_pins;

        // A net can own pins the coarse router never planned a wire for; sizing by
        // `global.wires` alone would drop them off the end of every table here.
        let n_nets = global
            .wires
            .len()
            .max(pins.iter().map(|(n, _)| n.0 as usize + 1).max().unwrap_or(0));

        // Drive the track lattice height from the PDK metal stack: one track layer
        // per permitted routing layer (the grid's even/odd = horizontal/vertical
        // convention is preserved by index). A taller `layers` slice yields routing
        // on more layers. Fall back to the cfg default only when no stack is given.
        //
        // Cap the stack at the number of cuts we can actually draw: climbing to a
        // layer we cannot via up to buys nothing but an isolated island, since the
        // deck requires an explicit cut for connectivity.
        let n_layers = if layers.is_empty() {
            cfg.n_layers
        } else {
            (layers.len() as u32).min(cuts.len() as u32 + 1)
        };

        // ---- Die extent + per-net corridor seeds, from the coarse Routes. ----
        // The coarse route's rect corners are gcell-snapped, so they are good for
        // "which gcells does this net thread" and useless as terminals.
        let mut coarse_pts: Vec<Vec<(i32, i32)>> = vec![Vec::new(); n_nets];
        let (mut hi_x, mut hi_y) = (2i32, 2i32);
        for (ni, shapes) in global.wires.iter().enumerate() {
            let pts = &mut coarse_pts[ni];
            for s in shapes {
                // Coarse segment endpoints are the two ends of the rect diagonal.
                let (x0, y0) = (s.rect.x, s.rect.y);
                let (x1, y1) = (s.rect.x + s.rect.w, s.rect.y + s.rect.h);
                pts.push((x0, y0));
                pts.push((x1, y1));
                hi_x = hi_x.max(x1);
                hi_y = hi_y.max(y1);
            }
            pts.sort_unstable();
            pts.dedup();
        }

        // ---- Routing frame ----
        // `TrackGrid` nodes start at (0,0), so a pin at negative coordinates is
        // unrepresentable: `claim_minarea` clamps it onto the origin and the net
        // routes to the wrong side of the die. Placed macro geometry does reach
        // negative coordinates (a `Macro`'s bbox is local and its origin is not its
        // corner), so route in a shifted frame and shift the geometry back.
        let origin = {
            let px = pins.iter().map(|(_, r)| r.x).min().unwrap_or(0);
            let py = pins.iter().map(|(_, r)| r.y).min().unwrap_or(0);
            let cx = global.wires.iter().flatten().map(|s| s.rect.x).min().unwrap_or(0);
            let cy = global.wires.iter().flatten().map(|s| s.rect.y).min().unwrap_or(0);
            (px.min(cx).min(0), py.min(cy).min(0))
        };
        for pts in &mut coarse_pts {
            for p in pts.iter_mut() {
                *p = (p.0 - origin.0, p.1 - origin.1);
            }
        }
        hi_x -= origin.0;
        hi_y -= origin.1;

        // ---- Terminals: the placed pin rects. ----
        // A terminal must land *inside* its pin or the wire never touches it, so a
        // terminal carries its rect, not just a point. With no pins supplied, fall
        // back to the coarse corners as degenerate 1x1 rects (this crate's unit
        // tests, and any caller routing without a placement).
        let mut term_rects: Vec<Vec<Rect>> = vec![Vec::new(); n_nets];
        for &(net, r) in pins {
            let ni = net.0 as usize;
            let r = Rect { x: r.x - origin.0, y: r.y - origin.1, ..r };
            if ni < n_nets && !term_rects[ni].contains(&r) {
                term_rects[ni].push(r);
                hi_x = hi_x.max(r.x + r.w);
                hi_y = hi_y.max(r.y + r.h);
            }
        }
        if pins.is_empty() {
            for (ni, pts) in coarse_pts.iter().enumerate() {
                term_rects[ni] =
                    pts.iter().map(|&(x, y)| Rect { x, y, w: 1, h: 1 }).collect();
            }
        }
        let die = (hi_x, hi_y);

        // Compact nets: only those with ≥1 terminal.
        let compact: Vec<usize> = (0..n_nets).filter(|&i| !term_rects[i].is_empty()).collect();
        let n_compact = compact.len();
        if n_compact == 0 {
            let routes = Routes {
                wires: vec![Vec::new(); n_nets],
            };
            let report = score(&routes, reqs, 0.0, 0.0);
            return (routes, report);
        }

        // ---- Build track grid, land terminals, set corridor regions. ----
        let ggrid = GcellGrid::new(die, cfg.gcells_per_side, 1);
        let mut grid = TrackGrid::with_layers(die, cfg.pitch, cfg.via_cost, n_layers);
        grid.set_regions(&ggrid);


        let mut claimed = vec![false; grid.nodes()];
        let mut reserved = vec![NONE; grid.nodes()];
        let mut c_terms: Vec<Vec<u32>> = vec![Vec::new(); n_compact];
        let mut corridors: Vec<Vec<u32>> = vec![Vec::new(); n_compact];

        // Pin rects that got no track node, reported rather than silently dropped:
        // a dropped terminal is an open net, and an open net is what LVS reports
        // five stages later as an `unconnected_pin`.
        let mut unlanded: Vec<(usize, Rect)> = Vec::new();
        // (compact net, pin rect, landed node position). The track pitch is a PDK
        // quantity (1890 nm on sky130's worst routed layer) and a pin is one cut
        // wide (170 nm), so a node essentially never falls *inside* a pin. The
        // route therefore reaches the node and a short jog reaches the pin — that
        // jog is pin access, and its absence is why nothing connected.
        let mut access: Vec<(usize, Rect, (i32, i32))> = Vec::new();

        for (ci, &ni) in compact.iter().enumerate() {
            // Land each terminal on a unique track node (docs: Pin landing).
            for &r in &term_rects[ni] {
                let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2);
                match grid.claim_minarea(cx, cy, &mut claimed, |_, _| true, cfg.min_area) {
                    Some(n) => {
                        reserved[n as usize] = ci as u32;
                        if !c_terms[ci].contains(&n) {
                            c_terms[ci].push(n);
                        }
                        // Degenerate 1x1 rects are the no-pins fallback: there is no
                        // real pin to reach, so no jog.
                        if r.w > 1 || r.h > 1 {
                            let (px, py, _) = grid.pos(n);
                            access.push((ci, r, (px, py)));
                        }
                    }
                    None => unlanded.push((ni, r)),
                }
            }
            // Corridor: gcell of each coarse terminal + its 8 neighbours (docs:
            // Detailed corridors). The coarse route already threads these gcells.
            // Seeded from the coarse route *and* the pins: a pin outside every
            // gcell the coarse route threads would otherwise be unreachable inside
            // the corridor, and PathFinder would leave the net open.
            let mut seeds: Vec<(i32, i32)> = coarse_pts[ni].clone();
            seeds.extend(term_rects[ni].iter().map(|r| (r.x + r.w / 2, r.y + r.h / 2)));
            let mut corr: Vec<u32> = Vec::new();
            for &(x, y) in &seeds {
                let g = ggrid.at(x as f32, y as f32);
                let (gx, gy) = (g % ggrid.nx, g / ggrid.nx);
                for dy in -1..=1i64 {
                    for dx in -1..=1i64 {
                        let (tx, ty) = (gx as i64 + dx, gy as i64 + dy);
                        if tx >= 0 && ty >= 0 && tx < ggrid.nx as i64 && ty < ggrid.ny as i64 {
                            corr.push((ty * ggrid.nx as i64 + tx) as u32);
                        }
                    }
                }
            }
            corr.sort_unstable();
            corr.dedup();
            corridors[ci] = corr;
        }
        debug_assert!(
            unlanded.is_empty(),
            "dr: {} pin rect(s) got no track node inside them at pitch {} — the net \
             will come out open. First few: {:?}",
            unlanded.len(),
            cfg.pitch,
            &unlanded[..unlanded.len().min(4)]
        );

        // ---- Detailed PathFinder ----
        let cold = RouteCtx {
            graph: grid,
            terms: c_terms,
            net_w: vec![1.0; n_compact],
            order: (0..n_compact as u32).collect(),
            corridors,
            reserved,
            max_len: vec![None; n_compact],
        };
        let mut hot = RouteHot::new(cold.graph.nodes(), n_compact);
        // Carry the negotiation in. The key is the node's ABSOLUTE position, so the
        // frame shift has to be added back: `dr` routes in a shifted frame and the
        // shift changes with the placement, which would make a track index — and a
        // shifted coordinate — a different resource every epoch.
        let abs = |n: u32| {
            let (x, y, l) = cold.graph.pos(n);
            (x + origin.0, y + origin.1, l)
        };
        neg.seed(gr::Tier::Detailed, &mut hot.hist, abs);

        // Rings as OBSTACLE: a band is drawn metal, so its track nodes are already
        // spoken for. Charged as *usage*, not `allowed = false`, and that is not a
        // stylistic choice — a hard block seals the ring. The stack this flow actually
        // uses is two layers (li horizontal, met1 vertical), a ring's left/right bands
        // are vertical, and li is then the only layer anything can move horizontally
        // on: forbidding it across a vertical band leaves no way into or out of the
        // enclosure at all, so every enclosed device comes out open. That is strictly
        // worse than the missing blockage.
        //
        // A charge at exactly capacity makes the band full-but-crossable: the search
        // pays `p_fac` to enter, prefers any real detour, and a crossing it cannot
        // avoid becomes residual overuse — a Θ residual naming the real problem
        // instead of a silent open. Charging *below* capacity, which is what `gr` does
        // over gcells, is not available here: track capacity IS 1.
        //
        // ponytail: `p_fac` is the entry price and the negotiation raises it from
        // there. A per-layer table — hard block on a layer that has a same-direction
        // alternative, priced on one that does not — is the upgrade when the permitted
        // stack is deeper than two.
        for m in rings {
            for s in &m.shapes {
                let Some(li) = layers.iter().position(|&l| l == s.layer) else {
                    continue; // implant/tap/cut band: not a routing resource at all
                };
                if li as u32 >= n_layers {
                    continue;
                }
                charge_rect(
                    &mut hot.usage,
                    &cold.reserved,
                    &cold.graph,
                    li as u32,
                    Rect {
                        x: s.rect.x - origin.0,
                        y: s.rect.y - origin.1,
                        ..s.rect
                    },
                );
            }
        }
        let mut tel = run_pathfinder(
            &mut hot,
            &cold,
            &mut (),
            cfg.p_fac,
            cfg.hist_inc,
            cfg.max_iters,
        );

        // ---- Post-route one-shot repairs (docs: EOL / wide-net PRL repair). ----
        if cfg.eol_spacing > 0 {
            let (probe, _) =
                extract_geometry_minarea(&hot, &cold.graph, cfg.wire_width, cfg.min_area);
            let (blocked, affected) = eol_violations(&probe, &cold.graph, cfg.eol_spacing);
            reroute_affected(&mut hot, &cold, &blocked, &affected, cfg.p_fac);
        }
        if cfg.wide_net_extra_spacing > 0 {
            // No net-name classification here (analog-blind) → no power nets to
            // key PRL on. Left inert unless a power-net set is derivable.
            let (probe, _) =
                extract_geometry_minarea(&hot, &cold.graph, cfg.wire_width, cfg.min_area);
            let power_nets: Vec<u32> = Vec::new();
            let (blocked, affected) =
                prl_violations(&probe, &cold.graph, &power_nets, cfg.wide_net_extra_spacing);
            reroute_affected(&mut hot, &cold, &blocked, &affected, cfg.p_fac);
        }

        // ---- Negotiated repair of analog HARD routing rules -----------------
        // Crosstalk / antenna / differential violations do not make a net
        // congestion-"dirty", so `run_pathfinder` alone will never revisit them:
        // it reroutes only nets sitting on an over-capacity node. Rip up exactly
        // the nets the violated rules name and re-route them under escalating
        // present-cost pressure, on the history PathFinder has already
        // accumulated (`hot.hist` persists across rounds — the stateful pricing
        // of backend/TODO.md §3a, without which an unchanged topology just
        // reproduces its own costs).
        let mut p_fac = cfg.p_fac;
        for _ in 0..cfg.hard_rounds {
            let probe = build_routes(&hot, &cold, &cfg, &compact, n_nets, layers, cuts);
            let mut ids: Vec<u32> = Vec::new();
            for batch in &reqs.hard {
                batch.violating_ids(&probe, &mut ids);
            }
            if ids.is_empty() {
                break; // clean, or nothing that can name its targets
            }
            // Rule ids are original NetIds; the router works in compact indices.
            ids.sort_unstable();
            ids.dedup();
            let affected: Vec<u32> = ids
                .iter()
                .filter_map(|&n| compact.iter().position(|&c| c as u32 == n))
                .map(|ci| ci as u32)
                .collect();
            if affected.is_empty() {
                break;
            }
            p_fac *= cfg.hard_escalation;
            reroute_affected(&mut hot, &cold, &[], &affected, p_fac);
        }

        // Hand the negotiation back, after the repair rounds — those reroute on the
        // same `hot.hist`, so folding it in earlier would lose everything they paid.
        neg.accumulate(gr::Tier::Detailed, &hot.hist, abs);

        // Residual overuse (docs: Final classification).
        let cap = cold.graph.cap();
        let overuse: f32 = hot
            .usage
            .iter()
            .map(|&u| f32::from(u.saturating_sub(cap)))
            .sum();
        tel.overflow = overuse;

        // ---- Extract final DRC-clean geometry → per-net Shapes. ----
        let mut routes = build_routes(&hot, &cold, &cfg, &compact, n_nets, layers, cuts);
        add_pin_access(&mut routes, &access, &compact, cfg.wire_width, layers, cuts);
        // Back out of the routing frame.
        if origin != (0, 0) {
            for net in &mut routes.wires {
                for s in net {
                    s.rect.x += origin.0;
                    s.rect.y += origin.1;
                }
            }
        }

        // Overuse normalised by the lattice's total track capacity, so its Θ entry
        // is a residual rather than a raw track-use count.
        let cap_total = f32::from(cap) * cold.graph.nodes() as f32;
        let report = score(&routes, reqs, tel.overflow, cap_total);
        (routes, report)
    }
}

/// Charge every track node of `layer` that `r` covers up to capacity — a soft
/// obstacle (see the ring comment in [`DetailedRoute::route`]).
fn charge_rect(usage: &mut [u16], reserved: &[u32], grid: &TrackGrid, layer: u32, r: Rect) {
    let ix = |v: i32| (v / grid.pitch).clamp(0, grid.nx as i32 - 1) as u32;
    let iy = |v: i32| (v / grid.pitch).clamp(0, grid.ny as i32 - 1) as u32;
    let cap = grid.cap();
    for y in iy(r.y)..=iy(r.y + r.h) {
        for x in ix(r.x)..=ix(r.x + r.w) {
            let n = grid.node(x, y, layer) as usize;
            // A node already reserved as somebody's landing pad is exempt. A ring's
            // own `ring` pins sit ON its bands, and a terminal is used whatever it
            // costs, so charging them would hand every ring a permanent Θ residual
            // for the crime of being connected to itself.
            if reserved.get(n).copied().unwrap_or(NONE) != NONE {
                continue;
            }
            usage[n] = usage[n].max(cap);
        }
    }
}

/// Draw the jog from each landed track node to its pin rect.
///
/// The track pitch is set by the *worst* routed layer's spacing (1890 nm on the
/// sky130 deck) while a pin is one contact wide (170 nm), so a track node lands
/// near a pin, never on it. Without this the route stops at the node and the pin
/// stays open — which is the whole reason `pair`'s two devices extracted onto
/// separate net islands.
///
/// The jog is an L of two rects. It runs on **`layers[1]`** (met1) with a cut at
/// each end when the stack has one, and only falls back to `layers[0]` when it
/// does not. `layers[0]` is li — the *device* interconnect layer, which the cells
/// have already filled with S/D pads and tap chains — so crawling to a pin along
/// it is what `min_spacing:li` kept reporting (17 of chain4's 22). Climbing to
/// met1, which the cells barely touch, is both the DRC-cheaper route and what a
/// real pin-access stage does.
///
/// ponytail: always the same layer, always an L, no search. Upgrade to a real
/// per-pin access search when a `Pin` carries its own layer — it has no layer
/// field today, which is what forces guessing that a pin sits on `layers[0]`.
fn add_pin_access(
    routes: &mut Routes,
    access: &[(usize, Rect, (i32, i32))],
    compact: &[usize],
    width: i32,
    layers: &[LayerId],
    cuts: &[(LayerId, i32, i32, i32)],
) {
    let Some(&base) = layers.first() else {
        return; // no stack given: caller kept raw track indices, nothing to draw on
    };
    // (jog layer, the cut joining it to `base`, cut size, pad size on each side).
    let climb = match (layers.get(1), cuts.first()) {
        (Some(&up), Some(&(cut, size, below, above))) => Some((up, cut, size, below.max(above))),
        _ => None,
    };
    for &(ci, pin, (nx, ny)) in access {
        let net = compact[ci];
        let metal = climb.map_or(base, |(up, ..)| up);
        // The jog is the *pin's* width, not the track wire width. The cell already
        // drew a legal feature at that size on this layer, so it clears min_width
        // by construction, and it is the narrowest thing that still covers the pin
        // — a full-width 290 nm jog shorted neighbouring nets (chain4 extracted its
        // four devices as one).
        let width = width.min(pin.w).min(pin.h).max(1);
        let half = width / 2;
        let (px, py) = (pin.x + pin.w / 2, pin.y + pin.h / 2);
        let wires = &mut routes.wires[net];
        // Horizontal leg along the node's row, vertical leg down the pin's column.
        // Both legs run `half` PAST the junction and past their own end, so the
        // union is a clean L: flush corners, no step for a `notch` rule to find.
        // (Butting them exactly at the corner instead cost 13 `notch:li` on `pair`.)
        let (x0, x1) = (nx.min(px) - half, nx.max(px) + half);
        let (y0, y1) = (ny.min(py) - half, ny.max(py) + half);
        wires.push(Shape {
            layer: metal,
            rect: Rect { x: x0, y: ny - half, w: x1 - x0, h: width },
        });
        if y1 - y0 > width {
            wires.push(Shape {
                layer: metal,
                rect: Rect { x: px - half, y: y0, w: width, h: y1 - y0 },
            });
        }
        // Stitch the jog down to `base` at both ends: at the node so it joins the
        // route, at the pin so it joins the cell. A cut with no pad enclosing it on
        // both sides is an open, so each end gets a pad on each layer.
        if let Some((_, cut, size, pad)) = climb {
            for (cx, cy) in [(nx, ny), (px, py)] {
                wires.push(Shape {
                    layer: cut,
                    rect: Rect { x: cx - size / 2, y: cy - size / 2, w: size, h: size },
                });
                for l in [base, metal] {
                    wires.push(Shape {
                        layer: l,
                        rect: Rect { x: cx - pad / 2, y: cy - pad / 2, w: pad, h: pad },
                    });
                }
            }
        }
    }
}

/// Extract the current routing state as per-net [`Routes`] geometry.
///
/// Shared by the final emission and the hard-rule repair loop, which needs to
/// *score* an intermediate state: analog rules read drawn shapes, so the only way
/// to ask "is this legal yet" is to extract.
///
/// Remaps compact net indices back to their original `NetId` slots and rewrites
/// the grid's internal layer index (`0..n_layers`) to the PDK's real [`LayerId`],
/// so downstream verify/GDS sees actual metal.
fn build_routes(
    hot: &RouteHot,
    cold: &RouteCtx<TrackGrid>,
    cfg: &DetailedCfg,
    compact: &[usize],
    n_nets: usize,
    layers: &[LayerId],
    cuts: &[(LayerId, i32, i32, i32)],
) -> Routes {
    let (wires, vias) = extract_geometry_minarea(hot, &cold.graph, cfg.wire_width, cfg.min_area);

    // Wires and vias share one track-layer index space, so they must be mapped
    // through *different* tables: a wire on track layer `i` is metal `layers[i]`,
    // a via stamped at `i` is the cut `cuts[i]` joining `layers[i]`/`layers[i+1]`.
    // Mapping both through `layers` (as this once did) turns every via into a
    // small square of metal and emits no cut layer at all — the geometry then has
    // no electrical path between layers.
    let mut out = vec![Vec::new(); n_nets];
    let wire_shapes = to_shapes(compact.len(), &wires, &[]);
    let via_shapes = to_shapes(compact.len(), &[], &vias);
    for (ci, (mut ws, mut vs)) in wire_shapes.into_iter().zip(via_shapes).enumerate() {
        remap(&mut ws, layers);
        let mut pads = landing_pads(&vs, layers, cuts);
        remap_cuts(&mut vs, cuts);
        ws.append(&mut vs);
        ws.append(&mut pads);
        out[compact[ci]] = ws;
    }
    assert_geometry_is_on_the_stack(&out, layers, cuts);
    Routes { wires: out }
}

/// Post-condition: every shape this stage emits is on a layer the PDK said we may
/// route on, and every cut is exactly the size the PDK said a cut must be.
///
/// This is the invariant whose violation caused ~330 DRC errors and a dead LVS
/// extraction: the router speaks in track-layer *indices*, and if the table it
/// maps them through is wrong, the geometry is silently drawn on wells and
/// diffusion. Nothing downstream reports that as a layer error — it surfaces as
/// hundreds of unrelated-looking spacing and width violations.
///
/// Deliberately `assert!`, not `debug_assert!`: the flow's feedback loop
/// (`op_demo`, `bench`) is a `--release` build, where `debug_assert!` is compiled
/// out and this would never run. One pass over the emitted shapes is free next to
/// the route itself.
fn assert_geometry_is_on_the_stack(
    out: &[Vec<Shape>],
    layers: &[LayerId],
    cuts: &[(LayerId, i32, i32, i32)],
) {
    if layers.is_empty() {
        return; // no stack given: caller kept the ported defaults, indices are raw
    }
    for shapes in out {
        for s in shapes {
            if let Some(&(_, size, ..)) = cuts.iter().find(|(c, ..)| *c == s.layer) {
                assert!(
                    s.rect.w == size && s.rect.h == size,
                    "cut on layer {:?} drawn {}x{}, PDK requires exactly {size}x{size} \
                     — a cut is a fixed-size contact, not a wire",
                    s.layer,
                    s.rect.w,
                    s.rect.h
                );
                continue;
            }
            assert!(
                layers.contains(&s.layer),
                "routed shape on layer {:?}, which is neither routing metal {:?} nor a \
                 cut {:?} — the track-index-to-LayerId mapping is wrong, so this wire is \
                 being drawn on whatever deck layer sits at that index",
                s.layer,
                layers,
                cuts.iter().map(|(c, ..)| *c).collect::<Vec<_>>()
            );
        }
    }
}

/// A square of metal on the layers immediately below and above each cut, so the
/// cut is enclosed even where no wire run happens to cover it.
///
/// A run of fewer than two nodes emits no wire (`gr::emit_run`), so a route that
/// climbs two layers in consecutive steps leaves the layer it passes through bare
/// and every cut there measures **zero** enclosure. Pads are same-net, same-layer
/// squares that merge with any wire already present, so drawing them
/// unconditionally is cheaper than detecting the case.
///
/// Takes vias **before** [`remap_cuts`], while `s.layer` is still the track index.
fn landing_pads(vias: &[Shape], layers: &[LayerId], cuts: &[(LayerId, i32, i32, i32)]) -> Vec<Shape> {
    let mut pads = Vec::new();
    for v in vias {
        let i = v.layer.0 as usize;
        let Some(&(_, _, below, above)) = cuts.get(i) else {
            continue;
        };
        for (metal, pad) in [(layers.get(i), below), (layers.get(i + 1), above)] {
            let Some(&metal) = metal else { continue };
            pads.push(Shape {
                layer: metal,
                rect: pnr_core::geom::Rect {
                    x: v.rect.x + (v.rect.w - pad) / 2,
                    y: v.rect.y + (v.rect.h - pad) / 2,
                    w: pad,
                    h: pad,
                },
            });
        }
    }
    pads
}

/// Rewrite each shape's track-layer index into the real PDK layer at that index.
/// An index with no entry is dropped: emitting it under its raw index would put
/// geometry on whatever deck layer happens to sit there.
fn remap(shapes: &mut Vec<Shape>, table: &[LayerId]) {
    if table.is_empty() {
        return;
    }
    shapes.retain_mut(|s| match table.get(s.layer.0 as usize) {
        Some(&real) => {
            s.layer = real;
            true
        }
        None => false,
    });
}

/// As [`remap`], but a cut is also **resized to the deck's exact size**, kept
/// centred where it was stamped.
///
/// Extraction stamps a via at `wire_width`, which is right for a wire and wrong
/// for a contact: sky130's `mcon` is exactly 170 nm, so a 290 nm cut breaks
/// `max_width` *and* leaves the enclosing metal 0 nm of enclosure where the deck
/// wants 30. Shrinking to the true size restores the enclosure for free, since
/// the wire carrying the via is wider than the cut.
fn remap_cuts(shapes: &mut Vec<Shape>, cuts: &[(LayerId, i32, i32, i32)]) {
    if cuts.is_empty() {
        return;
    }
    shapes.retain_mut(|s| match cuts.get(s.layer.0 as usize) {
        Some(&(real, size, _, _)) => {
            s.layer = real;
            s.rect.x += (s.rect.w - size) / 2;
            s.rect.y += (s.rect.h - size) / 2;
            s.rect.w = size;
            s.rect.h = size;
            true
        }
        None => false,
    });
}

/// One-shot rip-up/reroute of `affected` nets with a temporary usage bump on
/// `blocked` nodes (docs: repair passes use a usage bump, not an absolute block).
/// No unrestricted fallback beyond the stored corridor (docs: EOL/PRL repair).
fn reroute_affected(
    hot: &mut RouteHot,
    cold: &RouteCtx<TrackGrid>,
    blocked: &[u32],
    affected: &[u32],
    p_fac: f32,
) {
    if affected.is_empty() {
        return;
    }
    for &net in affected {
        let ni = net as usize;
        for &n in &hot.tree_nodes(ni) {
            hot.usage[n as usize] = hot.usage[n as usize].saturating_sub(1);
        }
        hot.trees[ni].clear();
    }
    for &node in blocked {
        hot.usage[node as usize] = hot.usage[node as usize].saturating_add(1);
    }
    let mut dij = Dij::new(cold.graph.nodes());
    for &net in affected {
        let ni = net as usize;
        let corridor: &[u32] = cold.corridors.get(ni).map_or(&[], Vec::as_slice);
        let ml = cold.max_len.get(ni).copied().flatten();
        if let Some((branches, _)) = route_net(
            &cold.graph,
            &hot.usage,
            &hot.hist,
            &[],
            &cold.terms[ni],
            corridor,
            net,
            &cold.reserved,
            p_fac,
            ml,
            &mut dij,
        ) {
            hot.trees[ni] = branches;
            for &n in &hot.tree_nodes(ni) {
                hot.usage[n as usize] += 1;
            }
        }
    }
    for &node in blocked {
        hot.usage[node as usize] = hot.usage[node as usize].saturating_sub(1);
    }
}

/// Assemble the [`Report`]: legality = analog `hard` violations + residual
/// `overuse`; cost = built-in mechanic (`overuse`) + analog `cost` sum.
/// **Analog-blind**.
///
/// Residual `overuse` sits in `budget_violations`, not `hard_violations` — see
/// `gr::score`, same reasoning. That also retires the string-match on
/// `v.rule == "routing overuse"` that `frontend/library` used to recover the number,
/// since it is now a Θ contribution the orchestrator sums generically.
///
/// The two analog tiers come from [`gr::analog_tiers`] rather than being rebuilt here:
/// `frontend/library::lex_key` sums this stage's Θ with `gr`'s and with placement's, so
/// the three must scale a residual identically or the sum is meaningless. The scale has
/// one home: `Violation::from_residual` (pnr_core).
///
/// `cap_total` is the lattice's total track capacity (per-node capacity × node count)
/// — overuse's denominator, same shape as `gr::score`'s overflow: `Σ(usage − cap) /
/// Σcap` in milli-budgets, so a contested track and a blown coupling budget weigh on
/// one scale (D17). `Report::cost` deliberately keeps the **raw** overuse — rescaling
/// it would silently re-weight the PEX tier.
fn score(routes: &Routes, reqs: &Requirements<Routes>, overuse: f32, cap_total: f32) -> Report {
    let (hard_violations, mut budget_violations) = gr::analog_tiers(routes, reqs);
    debug_assert!(
        cap_total > 0.0 || overuse == 0.0,
        "dr::score: positive overuse with no capacity denominator"
    );
    if overuse > 0.0 && cap_total > 0.0 {
        // `from_residual` ceils, so a real overuse can never round to a margin of 0,
        // which `Report::lex` reads as satisfied.
        budget_violations.push(pnr_core::report::Violation::from_residual(
            "routing overuse",
            f64::from(overuse / cap_total),
        ));
    }
    // Slack-blended, exactly as placement weights its analog cost: a net with
    // budget to spare weighs ~0, one at its spec weighs 1 (backend/TODO.md §3).
    let analog_cost: f32 = reqs
        .cost
        .iter()
        .map(|b| b.criticality(routes) * b.cost(routes))
        .sum();
    Report {
        hard_violations,
        budget_violations,
        cost: overuse + analog_cost,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_core::geom::{LayerId, Rect, Shape};

    const LAYERS: [LayerId; 2] = [LayerId(0), LayerId(1)];
    const CUTS: [(LayerId, i32, i32, i32); 1] = [(LayerId(2), 100, 140, 140)];

    // A coarse Routes: net 0 with one gcell segment from (1000,1000) to (18000,18000).
    fn coarse() -> Routes {
        let seg = Shape {
            layer: LayerId(0),
            rect: Rect { x: 1_000, y: 1_000, w: 17_000, h: 17_000 },
        };
        Routes { wires: vec![vec![seg]] }
    }

    #[test]
    fn detailed_realises_coarse_route() {
        let g = coarse();
        let reqs = Requirements::<Routes>::default();
        let (routes, report) = DetailedRoute::default().route(
            &g,
            &[],
            &[],
            &reqs,
            &LAYERS,
            &CUTS,
            &mut gr::Negotiation::new(),
            42,
        );
        assert_eq!(routes.wires.len(), 1);
        assert!(!routes.wires[0].is_empty(), "detailed route should draw shapes");
        assert!(report.hard_violations.is_empty(), "clean route: {:?}", report.hard_violations.iter().map(|v| &v.rule).collect::<Vec<_>>());
        // Every drawn shape has positive area.
        for s in &routes.wires[0] {
            assert!(s.rect.w > 0 && s.rect.h > 0);
        }
    }

    #[test]
    fn empty_coarse_gives_empty_routes() {
        let g = Routes { wires: vec![Vec::new(); 3] };
        let reqs = Requirements::<Routes>::default();
        let (routes, report) = DetailedRoute::default().route(
            &g,
            &[],
            &[],
            &reqs,
            &LAYERS,
            &CUTS,
            &mut gr::Negotiation::new(),
            0,
        );
        assert_eq!(routes.wires.len(), 3);
        assert!(routes.wires.iter().all(|w| w.is_empty()));
        assert!(report.hard_violations.is_empty());
    }

    /// The one invariant this stage exists to satisfy: **every pin rect is touched
    /// by a wire of its own net**.
    ///
    /// This is the cheap half of `frontend/library`'s `geometry::debug_check_connected`
    /// (xy-touch, per net) reproduced here because the full flow does not build. If
    /// it fails, `dr` is back to the measured `pair` failure — the route ending 1265
    /// nm from the pin, every device extracting onto its own island of nets, and LVS
    /// unable to match no matter what the placer does.
    ///
    /// The pin rects are deliberately **off-track**: one cut wide (170 nm), centred
    /// exactly on a pitch boundary so the nearest track node is a full half-pitch
    /// away in both axes. Measured with the access jog removed, every one of the four
    /// pins is missed by **715 nm** — the same order as the 1265 nm miss on `pair`
    /// — and by **0 nm** with it.
    #[test]
    fn every_pin_is_touched_by_its_own_net() {
        // Two nets, two pins each, all off-pitch and one-cut wide.
        let pin = |net: u16, x: i32, y: i32| {
            (NetId(net), Rect { x, y, w: 170, h: 170 })
        };
        // Each centre sits exactly on a pitch *boundary*, i.e. a full half-pitch
        // (945 nm) from the nearest track node in both axes — the worst case, and the
        // same order as the 1265 nm miss measured on `pair`.
        let pins = [
            pin(0, 1_805, 1_805),
            pin(0, 15_035, 13_145),
            pin(1, 3_695, 15_035),
            pin(1, 13_145, 3_695),
        ];
        // Coarse plan: one diagonal-bbox segment per net, gcell-snapped — i.e. near
        // the pins and on none of them, which is all `gr` ever promises.
        let seg = |x: i32, y: i32, w: i32, h: i32| Shape {
            layer: LayerId(0),
            rect: Rect { x, y, w, h },
        };
        let global = Routes {
            wires: vec![
                vec![seg(1_100, 1_000, 16_500, 15_200)],
                vec![seg(2_300, 2_700, 13_700, 12_400)],
            ],
        };

        let reqs = Requirements::<Routes>::default();
        let cfg = DetailedCfg { pitch: 1_890, ..DetailedCfg::default() };
        let (routes, _) = DetailedRoute { cfg }.route(
            &global,
            &pins,
            &[],
            &reqs,
            &LAYERS,
            &CUTS,
            &mut gr::Negotiation::new(),
            0,
        );

        for &(net, r) in &pins {
            let wires = &routes.wires[net.0 as usize];
            let touches = |s: &Shape| {
                s.rect.x <= r.x + r.w
                    && r.x <= s.rect.x + s.rect.w
                    && s.rect.y <= r.y + r.h
                    && r.y <= s.rect.y + s.rect.h
            };
            assert!(
                wires.iter().any(touches),
                "net {} pin at ({}, {}) is not touched by any of its {} shapes — \
                 nearest is {} nm away, so this net extracts as two islands",
                net.0,
                r.x,
                r.y,
                wires.len(),
                wires
                    .iter()
                    .map(|s| {
                        let dx = (s.rect.x - (r.x + r.w)).max(r.x - (s.rect.x + s.rect.w)).max(0);
                        let dy = (s.rect.y - (r.y + r.h)).max(r.y - (s.rect.y + s.rect.h)).max(0);
                        dx.max(dy)
                    })
                    .min()
                    .unwrap_or(i32::MAX),
            );
        }
    }

    /// A ring is an obstacle *and* a target (D5). The target half is the one that
    /// silently failed: the ring reached signoff as geometry the router never saw,
    /// so its bands were an unconnected island on the well/substrate net.
    #[test]
    fn ring_pins_are_routed() {
        let band = |x: i32, y: i32, w: i32, h: i32| Shape {
            layer: LayerId(0),
            rect: Rect { x, y, w, h },
        };
        let ring_pin = |x: i32, y: i32| pnr_core::geom::Pin {
            name: "ring".into(),
            net: NetId(0),
            at: Rect { x, y, w: 170, h: 170 },
        };
        let ring = Macro {
            bbox: Rect { x: 4_000, y: 4_000, w: 12_000, h: 12_000 },
            // Bottom and top bands only — enough to be an obstacle and carry pins.
            shapes: vec![
                band(4_000, 4_000, 12_000, 800),
                band(4_000, 15_200, 12_000, 800),
            ],
            pins: vec![ring_pin(4_431, 4_207), ring_pin(15_113, 15_411)],
        };
        let reqs = Requirements::<Routes>::default();
        let (routes, report) = DetailedRoute::default().route(
            &Routes { wires: Vec::new() },
            &[],
            &[ring.clone()],
            &reqs,
            &LAYERS,
            &CUTS,
            &mut gr::Negotiation::new(),
            0,
        );
        assert!(
            !routes.wires[0].is_empty(),
            "the ring's own net got no geometry: its pins never became terminals"
        );
        for p in &ring.pins {
            assert!(
                routes.wires[0].iter().any(|s| {
                    s.rect.x <= p.at.x + p.at.w
                        && p.at.x <= s.rect.x + s.rect.w
                        && s.rect.y <= p.at.y + p.at.h
                        && p.at.y <= s.rect.y + s.rect.h
                }),
                "ring pin at ({}, {}) untouched — the band is an island",
                p.at.x,
                p.at.y
            );
        }
        // The band charge must not price the ring's *own* landing pads, or a ring
        // reports a Θ residual for being connected to itself and the search can
        // never reach Θ = 0.
        assert!(
            report.budget_violations.is_empty(),
            "ring fabricated a budget residual: {:?}",
            report
                .budget_violations
                .iter()
                .map(|v| (&v.rule, v.margin))
                .collect::<Vec<_>>()
        );
    }

    /// **J1**, `dr`'s half: `score` must read `reqs.budget`, on the same scale `gr` and
    /// `gp` use. `gr::analog_tiers` is tested there; this guards the *delegation*, which
    /// is the part a per-crate test in `gr` cannot see — drop the call and `dr` silently
    /// stops contributing to Θ while every `gr` test still passes.
    #[test]
    fn budget_residual_reaches_theta() {
        let routes = Routes {
            wires: vec![vec![Shape {
                layer: LayerId(0),
                rect: Rect { x: 0, y: 0, w: 1_500, h: 1 },
            }]],
        };
        let mut reqs = Requirements::<Routes>::default();
        reqs.budget.push(Box::new(vec![analog::routing::ParasiticBudget {
            net: NetId(0),
            max_r_mohm: 1_000_000,
            max_c_af: 100_000_000,
            max_len_nm: 1_000,
            margin_pct: 10,
        }]));
        let report = score(&routes, &reqs, 0.0, 0.0);
        // 1500/1000 - 1 = 0.5 over ⇒ 500 milli-budgets.
        assert_eq!(report.budget_violations.len(), 1);
        assert_eq!(report.budget_violations[0].margin, 500);
        assert!(report.lex().1 > 0.0, "Θ must move when a budget is exceeded");
    }

    /// `Negotiation` must survive the call and change the next one, exactly as in
    /// `gr` — `dr` runs its own negotiation over track resources and D3 says both
    /// feed one `pressure()`.
    #[test]
    fn negotiation_persists_across_calls() {
        // Track capacity is 1, so nets crossing the same narrow frame are
        // unavoidably contested. Everything is squeezed into a small die at a coarse
        // pitch to guarantee it.
        // 1300 nm pitch over a ~15 µm die is 11 tracks a side for five nets with ten
        // reserved landing nodes: contention is unavoidable (so history keeps
        // climbing) while enough of the lattice stays free that the priced-up nodes
        // can actually be avoided (so the second call routes differently). Both
        // halves are needed — a lattice tight enough to force overuse *everywhere*
        // negotiates nothing, and a loose one never contests anything.
        let cfg = DetailedCfg { pitch: 1_300, max_iters: 40, ..DetailedCfg::default() };
        let router = DetailedRoute { cfg };
        let pins: Vec<(NetId, Rect)> = (0..5u16)
            .flat_map(|n| {
                let off = i32::from(n) * 1_100;
                [
                    (NetId(n), Rect { x: 1_000 + off, y: 1_000, w: 170, h: 170 }),
                    (NetId(n), Rect { x: 15_000 - off, y: 15_000, w: 170, h: 170 }),
                ]
            })
            .collect();
        let global = Routes { wires: vec![Vec::new(); 5] };
        let reqs = Requirements::<Routes>::default();
        let flat = |r: &Routes| -> Vec<(u16, i32, i32, i32, i32)> {
            r.wires
                .iter()
                .flatten()
                .map(|s| (s.layer.0, s.rect.x, s.rect.y, s.rect.w, s.rect.h))
                .collect()
        };

        let mut neg = gr::Negotiation::new();
        let run = |neg: &mut gr::Negotiation| {
            router
                .route(&global, &pins, &[], &reqs, &LAYERS, &CUTS, neg, 0)
                .0
        };
        let first = run(&mut neg);
        let p1 = neg.pressure();
        assert!(p1 > 0.0, "contested tracks must accumulate history");
        let second = run(&mut neg);
        let p2 = neg.pressure();
        assert!(p2 > p1, "history must keep climbing: {p1} -> {p2}");
        assert_ne!(
            flat(&first),
            flat(&second),
            "second call saw history and produced the identical route — not seeded"
        );
    }
}
