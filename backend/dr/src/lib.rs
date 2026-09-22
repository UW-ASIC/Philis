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
    /// `min_spacing` of the pin-access layer (`layers[0]`, li), in nm.
    ///
    /// A landed node becomes an li pad, and the cells have already drawn li. A pad
    /// that lands *overlapping* cell li merges with it and a pad that clears it by
    /// `min_spacing` is legal — but one that stops in between leaves a sliver that
    /// is a spacing violation no reroute can fix, because the node position, not
    /// the path, is what created it. Landing prefers nodes outside that band.
    ///
    /// `0` disables the check (no deck value ⇒ no rule to enforce).
    pub pin_access_spacing: i32,
    /// `min_spacing` of the pin-access **cut** (`cuts[0]`, mcon), in nm.
    ///
    /// An access jog stitches a cut at each end. Land close enough and the two
    /// cuts are one spacing violation; land closer still and the pads merge, which
    /// is legal again. Same shape of problem as [`Self::pin_access_spacing`], one
    /// layer down. `0` disables.
    pub pin_access_cut_spacing: i32,
    /// The conductor **below** the routing stack that cells put their pins on, plus
    /// the cut joining it up to `layers[0]`: `(li, (mcon, size, below_enc, above_enc))`.
    ///
    /// This is what lets li leave the routing stack. li is a legal conductor, so the
    /// deck rightly lists it in `routing_metals` — but it is also the layer every
    /// cell fills with S/D pads and tap chains, and while it was `layers[0]` the
    /// PathFinder laid horizontal tracks straight across that geometry: 26 of
    /// `chain4`'s 44 DRC violations were `LI.3 min_spacing`. Reserving it for cells
    /// is a *router policy*, not a fact about the process, so it lives here rather
    /// than being edited out of the deck.
    ///
    /// `None` keeps the old behaviour: pins are assumed to sit on `layers[0]`.
    pub pin_access: Option<(LayerId, (LayerId, i32, i32, i32))>,
}

/// Edge-to-edge gap between two rects, `0` when they overlap or touch.
///
/// Diagonal separation reports the larger axis rather than the true Euclidean
/// corner distance — an under-estimate, so a corner case is judged closer than it
/// is. That errs toward rejecting a landing site, which the caller can always
/// fall back from; the opposite error ships a violation.
fn rect_gap(a: Rect, b: Rect) -> i32 {
    let dx = (b.x - (a.x + a.w)).max(a.x - (b.x + b.w));
    let dy = (b.y - (a.y + a.h)).max(a.y - (b.y + b.h));
    if dx <= 0 && dy <= 0 {
        0
    } else {
        dx.max(dy).max(0)
    }
}

/// True when `outer` covers every point of `inner` (equal rects included).
fn contains(outer: Rect, inner: Rect) -> bool {
    outer.x <= inner.x
        && outer.y <= inner.y
        && outer.x + outer.w >= inner.x + inner.w
        && outer.y + outer.h >= inner.y + inner.h
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
            pin_access_spacing: 0,
            pin_access_cut_spacing: 0,
            pin_access: None,
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
    /// `placed` is the **placed device macros**, and `rings` this epoch's guard
    /// rings. Both are absolute and both carry real-`NetId` pins, which are folded
    /// into `pins` (de-duplicated, so a caller may pass the same macros through
    /// both channels without landing a terminal twice).
    ///
    /// They differ in what the router does with the *shapes*, and the difference
    /// is the point of the split:
    ///
    /// * `rings` are **charged as track usage**. A ring is a barrier you route
    ///   *around*, so its bands should cost something to cross.
    /// * `placed` is **not charged**. A device cell is a thing you route *to*;
    ///   pricing its own li made reaching its own pins expensive and cost `chain4`
    ///   38 extra `erc/unconnected_pin` when it was tried.
    ///
    /// Both are consulted for **pin-access legality**: a landing pad that stops a
    /// sliver away from drawn li is a spacing violation no reroute can clear. That
    /// is why `placed` exists at all — device cells used to reach `dr` as pin rects
    /// alone, so the router could not see cell interiors and landed pads 20 nm from
    /// li the cell had already drawn (`LI.3 measured=20`, limit 170).
    ///
    /// Placement-derived, so handed in fresh every epoch.
    ///
    /// `neg` is the negotiation history, carried across outer epochs and owned by
    /// the orchestrator — see [`gr::Negotiation`]. `dr` runs its own negotiation
    /// over track resources; sharing the type with `gr` is what lets the caller
    /// read one `pressure()` for the whole routing tier.
    fn route(
        &self,
        global: &Routes,
        pins: &[(NetId, Rect, LayerId)],
        placed: &[Macro],
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
        pins: &[(NetId, Rect, LayerId)],
        placed: &[Macro],
        rings: &[Macro],
        reqs: &Requirements<Routes>,
        layers: &[LayerId],
        cuts: &[(LayerId, i32, i32, i32)],
        neg: &mut gr::Negotiation,
        seed: u64,
    ) -> (Routes, Report) {
        DetailedRoute::default().route(global, pins, placed, rings, reqs, layers, cuts, neg, seed)
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
        pins: &[(NetId, Rect, LayerId)],
        placed: &[Macro],
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

        // Placed geometry as TARGET (D5): a band's `ring` pin carries a real
        // `NetId` and has to be reached like any other pin, or the ring is an
        // island in the extraction. Folded into `pins` because from here on a
        // terminal is a `(NetId, Rect)` and a ring pin is nothing else; `placed`
        // is already absolute, so there is no placement step to skip or repeat.
        //
        // De-duplicated: the caller passes device macros through *both* `pins`
        // (as rects) and `placed` (as blocking geometry), and landing the same
        // terminal twice burns a second track node and reserves it against the
        // net that already owns it.
        // Order-preserving. Landing walks this list in order and each terminal
        // claims a node the next one can no longer have, so terminal order *is*
        // which net wins a contested site. Sorting to dedupe reshuffled that and
        // re-routed circuits that had no duplicates at all.
        let mut seen = std::collections::HashSet::new();
        let mut all_pins: Vec<(NetId, Rect, LayerId)> = Vec::new();
        for (n, r, l) in pins.iter().copied().chain(
            placed.iter().chain(rings).flat_map(|m| m.pins.iter().map(|p| (p.net, p.at, p.layer))),
        ) {
            if seen.insert((n.0, r.x, r.y, r.w, r.h)) {
                all_pins.push((n, r, l));
            }
        }
        let pins: &[(NetId, Rect, LayerId)] = &all_pins;

        // A net can own pins the coarse router never planned a wire for; sizing by
        // `global.wires` alone would drop them off the end of every table here.
        let n_nets = global
            .wires
            .len()
            .max(pins.iter().map(|(n, ..)| n.0 as usize + 1).max().unwrap_or(0));

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
        // Claim-free perimeter halo. A frame clipped exactly to the pin/route
        // bbox lets the pin-access reservations (`claim_jog_sweep`) of several
        // nets tile the full frame height in one column band of the horizontal
        // layer — and on an H/V lattice such a band is an absolute wall: the
        // vertical layer has no lateral edges, reserved nodes are hard blocks,
        // so a net with terminals on both sides is unroutable no matter how the
        // negotiation prices it (quad's gate net, open every epoch). Two pitches
        // of track outside the pin extent can never be claimed by a jog, so a
        // crossing corridor always survives.
        let halo = 2 * cfg.pitch;
        let origin = {
            let px = pins.iter().map(|(_, r, _)| r.x).min().unwrap_or(0);
            let py = pins.iter().map(|(_, r, _)| r.y).min().unwrap_or(0);
            let cx = global.wires.iter().flatten().map(|s| s.rect.x).min().unwrap_or(0);
            let cy = global.wires.iter().flatten().map(|s| s.rect.y).min().unwrap_or(0);
            (px.min(cx).min(0) - halo, py.min(cy).min(0) - halo)
        };
        // Frame margin: three track pitches of free lattice on every side. The
        // met1 blockage under cells leaves horizontal capacity only in gutters
        // and margins; with zero margin every die-crossing trunk fought for the
        // single boundary row and PathFinder never converged (residual
        // overflow = drawn trunk-on-trunk shorts, chain4). Margins are the
        // cheapest horizontal highways there are: free space, no DRC neighbour.
        let margin = 3 * cfg.pitch;
        let origin = (origin.0 - margin, origin.1 - margin);
        for pts in &mut coarse_pts {
            for p in pts.iter_mut() {
                *p = (p.0 - origin.0, p.1 - origin.1);
            }
        }
        hi_x -= origin.0;
        hi_y -= origin.1;
        hi_x += margin;
        hi_y += margin;

        // ---- Terminals: the placed pin rects. ----
        // A terminal must land *inside* its pin or the wire never touches it, so a
        // terminal carries its rect, not just a point. With no pins supplied, fall
        // back to the coarse corners as degenerate 1x1 rects (this crate's unit
        // tests, and any caller routing without a placement).
        let mut term_rects: Vec<Vec<(Rect, LayerId)>> = vec![Vec::new(); n_nets];
        for &(net, r, l) in pins {
            let ni = net.0 as usize;
            let r = Rect { x: r.x - origin.0, y: r.y - origin.1, ..r };
            if ni < n_nets && !term_rects[ni].iter().any(|&(e, _)| e == r) {
                term_rects[ni].push((r, l));
                hi_x = hi_x.max(r.x + r.w);
                hi_y = hi_y.max(r.y + r.h);
            }
        }
        if pins.is_empty() {
            for (ni, pts) in coarse_pts.iter().enumerate() {
                term_rects[ni] =
                    pts.iter()
                        .map(|&(x, y)| (Rect { x, y, w: 1, h: 1 }, layers.first().copied().unwrap_or(LayerId(0))))
                        .collect();
            }
        }
        // Top/right half of the perimeter halo (the origin shift is the
        // bottom/left half).
        let die = (hi_x + halo, hi_y + halo);

        // Compact nets: only those with ≥1 terminal.
        let compact: Vec<usize> = (0..n_nets).filter(|&i| !term_rects[i].is_empty()).collect();
        let n_compact = compact.len();
        if n_compact == 0 {
            let routes = Routes {
                wires: vec![Vec::new(); n_nets],
            };
            let report = score(&routes, reqs, 0.0, 0.0, layers, cuts, &[]);
            return (routes, report);
        }

        // ---- Build track grid, land terminals, set corridor regions. ----
        let ggrid = GcellGrid::new(die, cfg.gcells_per_side, 1);
        let mut grid = TrackGrid::with_layers(die, cfg.pitch, cfg.via_cost, n_layers);
        grid.set_regions(&ggrid);

        // Every placed cell (and ring) blocks met1 under its DRAWN extent — the
        // wiring of `TrackGrid::block_met1`, whose carve-out machinery
        // (`terminal_only`: landed pins become via-access holes with no lateral
        // wire) existed but had no caller. Without it the PathFinder laid met1
        // trunks straight across cell pin rows, where every pin's off-lattice
        // stitch pad/cut sits, and a trunk touching a foreign pin's mcon is a
        // drawn short no rule reports — rc_filter's pfet extracted Source ≡
        // Drain from exactly that. Layer 0 only: met2 above cells stays
        // routable, and pin access descends by jog + stitch.
        //
        // Drawn extent, NOT `m.bbox`: variant bboxes are deliberately inflated
        // upstream (guard-ring halo, whitespace escalation) to reserve routing
        // ground, and blocking the inflated bbox confiscated every nm of that
        // purchased whitespace right back. Only drawn geometry can short a
        // trunk, so only the shapes' hull blocks — no extra margin (measured
        // harmful). Same rule for rings, where it changes nothing: a ring's
        // bbox IS its shapes' hull (`Builder::finish`, modulo even-grid
        // rounding), so the enclosure interior stays blocked as before, which
        // still covers the enclosed cell's pin rows and the li bands' own
        // mcon-stitched ring pins — the exact short hazard above.
        for m in placed.iter().chain(rings) {
            let mut sh = m.shapes.iter();
            let Some(s0) = sh.next() else { continue }; // nothing drawn, nothing to short
            let (mut x0, mut y0) = (s0.rect.x, s0.rect.y);
            let (mut x1, mut y1) = (s0.rect.x + s0.rect.w, s0.rect.y + s0.rect.h);
            for s in sh {
                x0 = x0.min(s.rect.x);
                y0 = y0.min(s.rect.y);
                x1 = x1.max(s.rect.x + s.rect.w);
                y1 = y1.max(s.rect.y + s.rect.h);
            }
            grid.block_met1(x0 - origin.0, y0 - origin.1, x1 - origin.0, y1 - origin.1);
        }


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
        // (compact net, pin rect, pin layer, landed node position, node LAYER,
        // jog choice decided at landing when the joint search found one).
        // The layer matters: a jog stitches its node end down/up only when the
        // route actually arrives on a different layer than the legs.
        let mut access: Vec<(usize, Rect, LayerId, (i32, i32), u32, Option<(i32, bool)>)> =
            Vec::new();
        // Foreign pin stitch zones, keyed by compact net, route frame — needed
        // AT LANDING now: the node and its jog are chosen jointly.
        let zones: Vec<(u32, i32, i32)> = {
            let mut z = Vec::with_capacity(all_pins.len());
            for &(n, r, _) in all_pins.iter() {
                if let Some(ci) = compact.iter().position(|&c| c == n.0 as usize) {
                    z.push((ci as u32, r.x + r.w / 2 - origin.0, r.y + r.h / 2 - origin.1));
                }
            }
            z
        };
        // Jog legs committed so far (landing order = drawing order).
        let mut laid_legs: Vec<(usize, Rect)> = Vec::new();

        // Pin-access obstacles: the li `placed` has already drawn, in the router's
        // origin-relative frame. Empty when the caller gave no stack, no via
        // enclosure to size a pad from, or no spacing rule to enforce.
        // The layer a landing pad actually lands on: the pin-access conductor when
        // one is reserved, else the stack base.
        let pad_layer = cfg.pin_access.map_or_else(|| layers.first().copied(), |(l, _)| Some(l));
        let access_pad = cfg
            .pin_access
            .map(|(_, (_, _, below, above))| below.max(above))
            .or_else(|| cuts.first().map(|&(_, _, below, above)| below.max(above)))
            .unwrap_or(0);
        let access_obstacles: Vec<Rect> = match pad_layer {
            Some(l0) if access_pad > 0 && cfg.pin_access_spacing > 0 => placed
                .iter()
                .chain(rings)
                .flat_map(|m| m.shapes.iter())
                .filter(|s| s.layer == l0)
                .map(|s| Rect { x: s.rect.x - origin.0, y: s.rect.y - origin.1, ..s.rect })
                .collect(),
            _ => Vec::new(),
        };
        let access_cut = cfg
            .pin_access
            .map(|(_, (_, size, _, _))| size)
            .or_else(|| cuts.first().map(|&(_, size, _, _)| size))
            .unwrap_or(0);
        // A node is a good landing site when neither shape the jog will grow there
        // lands in a "too close to merge, too near to clear" band:
        //
        //   * the li pad, against li the cells already drew — overlap merges, a
        //     full `min_spacing` clears, in between is a sliver;
        //   * its stitch cut, against the cut at the pin end — these are separate
        //     shapes whatever the pads do, so the only legal answer is real
        //     clearance, unless the pads merge and `add_pin_access` drops the
        //     second stitch entirely.
        let good_site = |pin_c: (i32, i32), px: i32, py: i32| -> bool {
            let d = (px - pin_c.0).abs().max((py - pin_c.1).abs());
            if access_cut > 0 && cfg.pin_access_cut_spacing > 0 && d >= access_pad {
                let gap = d - access_cut;
                if gap > 0 && gap < cfg.pin_access_cut_spacing {
                    return false;
                }
            }
            if access_obstacles.is_empty() {
                return true;
            }
            let pad = Rect {
                x: px - access_pad / 2,
                y: py - access_pad / 2,
                w: access_pad,
                h: access_pad,
            };
            !access_obstacles.iter().any(|&o| {
                let g = rect_gap(pad, o);
                g > 0 && g < cfg.pin_access_spacing
            })
        };

        // The jog `add_pin_access` will draw from a candidate node must not
        // cross a FOREIGN pin's stitch zone: every pin grows a metal stitch pad
        // (~wire width) plus its cut, and a jog leg overlapping one is a drawn
        // short — the rc_filter failure, where a VDD leg crossed the adjacent
        // drain pad's mcon and the pfet extracted with Source ≡ Drain. Checked
        // per candidate because it depends on where the node lands, not just on
        // where pins are; the zone is the stitch pad inflated by nothing (a
        // touch already merges).
        let stitch = cfg.wire_width.max(access_pad);
        let short_free = |net: NetId, pin: Rect, px: i32, py: i32| -> bool {
            let (cx, cy) = (pin.x + pin.w / 2, pin.y + pin.h / 2);
            let half = cfg.wire_width.min(pin.w).min(pin.h).max(1) / 2;
            let legs = [
                Rect {
                    x: px.min(cx) - half,
                    y: py - half,
                    w: (px - cx).abs() + 2 * half,
                    h: 2 * half,
                },
                Rect {
                    x: cx - half,
                    y: py.min(cy) - half,
                    w: 2 * half,
                    h: (py - cy).abs() + 2 * half,
                },
            ];
            all_pins.iter().all(|&(n, fr, _)| {
                if n == net {
                    return true;
                }
                let (fx, fy) = (fr.x + fr.w / 2 - origin.0, fr.y + fr.h / 2 - origin.1);
                let zone = Rect {
                    x: fx - stitch / 2,
                    y: fy - stitch / 2,
                    w: stitch,
                    h: stitch,
                };
                legs.iter().all(|&l| rect_gap(l, zone) > 0)
            })
        };

        for (ci, &ni) in compact.iter().enumerate() {
            // Land each terminal on a unique track node (docs: Pin landing).
            for &(r, r_layer) in &term_rects[ni] {
                let (cx, cy) = (r.x + r.w / 2, r.y + r.h / 2);
                let net = NetId(ni as u16);
                // Three tiers: fully clean, then merely short-free, then any
                // site at all. An unlanded terminal is an open net — worse than
                // a spacing violation — but a shorting site is worse than both
                // (LVS silently merges two nets), so the unconditional fallback
                // comes last and the short check never relaxes before it.
                // Tight radii on every tier: a node two rings out is a sub-µm
                // jog inside (or near) the pin's own zone; a node eight rings
                // out is a multi-µm blind leg across foreign columns — the
                // exact geometry the cross-net resolver then has to delete,
                // leaving a permanent open (chain4's net g, every epoch). If
                // even radius 4 finds nothing, NOT landing is better: the
                // terminal drops, the net reports open, and the negotiation
                // can move things — a blind leg cannot be negotiated with.
                // JOINT node+jog selection first: walk near candidates and take
                // the first whose jog (some width × orientation) draws clean
                // against every foreign zone and every jog already committed.
                // This is what replaces landing blind and patching after: the
                // access geometry is decided here, where alternatives still
                // exist. Fall back to the tiered landing (jog decided later,
                // best-effort) only when no candidate offers a clean jog.
                let full = cfg.wire_width.max(1);
                let narrow = full.min(r.w).min(r.h).max(1);
                let mut joint: Option<(u32, (i32, bool), [Rect; 2])> = None;
                if r.w > 1 || r.h > 1 {
                    let cands = grid.claim_candidates(
                        cx,
                        cy,
                        &claimed,
                        |px, py| good_site((cx, cy), px, py) && short_free(net, r, px, py),
                        cfg.min_area,
                        12,
                    );
                    // Deliberately NOT gated on whether another net already
                    // owns the corridor `claim_jog_sweep` will sweep. Tried:
                    // it rejects the near candidates and pushes landing further
                    // out, and the longer jogs collide with more than the
                    // contested cell did — 2 LVS findings became 29 on the 5T
                    // OTA. The corridor conflict is real, but refusing to land
                    // is the wrong lever for it.
                    'cand: for &(n, _) in &cands {
                        let (px, py, _) = grid.pos(n);
                        for &w in &[full, narrow] {
                            for (fi, legs) in jog_legs(px, py, cx, cy, w).iter().enumerate() {
                                if jog_clean(legs, ci, full, &zones, &laid_legs) {
                                    joint = Some((n, (w, fi == 1), *legs));
                                    break 'cand;
                                }
                            }
                        }
                    }
                }
                let landed = match joint {
                    Some((n, choice, legs)) => {
                        grid.claim_node(n, &mut claimed);
                        for &l in &legs {
                            laid_legs.push((ci, l));
                        }
                        Some((n, Some(choice)))
                    }
                    None => grid
                        .claim_minarea_within(
                            cx,
                            cy,
                            &mut claimed,
                            |px, py| good_site((cx, cy), px, py) && short_free(net, r, px, py),
                            cfg.min_area,
                            2,
                        )
                        .or_else(|| {
                            grid.claim_minarea_within(
                                cx,
                                cy,
                                &mut claimed,
                                |px, py| short_free(net, r, px, py),
                                cfg.min_area,
                                3,
                            )
                        })
                        .or_else(|| {
                            grid.claim_minarea_within(
                                cx, cy, &mut claimed, |_, _| true, cfg.min_area, 4,
                            )
                        })
                        .map(|n| (n, None)),
                };
                match landed {
                    Some((n, choice)) => {
                        reserved[n as usize] = ci as u32;
                        if !c_terms[ci].contains(&n) {
                            c_terms[ci].push(n);
                        }
                        // Degenerate 1x1 rects are the no-pins fallback: there is no
                        // real pin to reach, so no jog.
                        if r.w > 1 || r.h > 1 {
                            let (px, py, nl) = grid.pos(n);
                            access.push((ci, r, r_layer, (px, py), nl, choice));
                            if std::env::var("DR_NO_JOG_SWEEP").is_err() {
                                claim_jog_sweep(
                                    &grid, &cfg, layers, cuts, n_layers, ci as u32, r, r_layer,
                                    (px, py), choice, &mut claimed, &mut reserved,
                                );
                            }
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
            seeds.extend(term_rects[ni].iter().map(|(r, _)| (r.x + r.w / 2, r.y + r.h / 2)));
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
        // Reserve every pin's stitch footprint on the lattice: any node whose
        // wire band would overlap a pin's drawn stitch pad belongs to that
        // pin's net — a foreign trunk through it touches the pad and is a
        // drawn short (the boundary-graze class: a pin at a cell edge grows
        // its pad past the met1 blockage, and a trunk hugging the boundary
        // clips it). First-come on conflicts, like every claim here; blanket
        // met1-blockage inflation was tried instead and strangled the
        // inter-cell channels (overuse ×3.6).
        if std::env::var("DR_NO_PIN_RESERVE").is_err() {
            let reach = (cfg.wire_width + cfg.wire_width.max(access_pad)) / 2;
            for &(n, r, _) in all_pins.iter() {
                let Some(ci) = compact.iter().position(|&c| c == n.0 as usize) else {
                    continue;
                };
                let (pcx, pcy) = (r.x + r.w / 2 - origin.0, r.y + r.h / 2 - origin.1);
                // Layer 0 only: the stitch pad is met1, and a met2 wire passing
                // overhead touches nothing (no cut is stamped there). Reserving
                // met2 too walled every vertical highway at every pin row and
                // turned the graze-shorts into unroutable opens.
                for layer in 0..1 {
                    let ix0 = ((pcx - reach) / cfg.pitch).max(0);
                    let ix1 = (pcx + reach) / cfg.pitch;
                    let iy0 = ((pcy - reach) / cfg.pitch).max(0);
                    let iy1 = (pcy + reach) / cfg.pitch;
                    for iy in iy0..=iy1 {
                        for ix in ix0..=ix1 {
                            if ix >= grid.nx as i32 || iy >= grid.ny as i32 {
                                continue;
                            }
                            let node = grid.node(ix as u32, iy as u32, layer) as usize;
                            let (px, py, _) = grid.pos(node as u32);
                            if (px - pcx).abs() > reach || (py - pcy).abs() > reach {
                                continue;
                            }
                            if reserved[node] == NONE {
                                reserved[node] = ci as u32;
                            }
                        }
                    }
                }
            }
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
        // Per-net shape counts BEFORE access drawing: everything at or past
        // this index is jog/stitch geometry, the only shapes the cross-net
        // resolver below is allowed to sacrifice.
        let mut pre_access: Vec<usize> = routes.wires.iter().map(Vec::len).collect();
        let access_pos: Vec<(usize, Rect, LayerId, (i32, i32))> =
            access.iter().map(|&(ci, r, l, p, _, _)| (ci, r, l, p)).collect();
        let node_layers: Vec<u32> = access.iter().map(|&(.., nl, _)| nl).collect();
        let choices: Vec<Option<(i32, bool)>> = access.iter().map(|&(.., c)| c).collect();
        add_pin_access(
            &mut routes, &access_pos, &compact, cfg.wire_width,
            (cfg.pitch - cfg.wire_width).max(0), layers, cuts, cfg.pin_access,
            &zones, &node_layers, &choices,
        );
        // Same-net sliver healing: jog legs, via stitch pads and trunk runs are
        // drawn by three different hands at off-lattice offsets, so one net's
        // own pieces can end up a sub-min_spacing sliver apart — DRC reads a
        // spacing violation between two rects that are electrically one node.
        // Filling the gap is always safe on one net, and no foreign geometry
        // can sit inside it (it would already violate spacing against both
        // sides). `pitch - wire_width` is the lattice's own inter-track gap =
        // the worst spacing any routed layer demands.
        // Cross-net contact is the one thing this stage must never emit: a
        // drawn short extracts two nets as one and surfaces five stages later
        // as an untraceable label conflict. The candidates above dodge what
        // they can; whatever contact remains is resolved by deleting the
        // offending ACCESS shape (jog leg / stitch pad) — turning the short
        // into an open, which is a reported hard violation the search can act
        // on. Trunk-vs-trunk contact never happens (capacity-1 lattice +
        // reservations), so sacrificing access geometry always suffices.
        //
        // "a reported hard violation" was a claim, not a fact, and only held
        // for single-pin nets. The open check below measures whether a net's
        // *wires* form one component — so sacrificing the jog that served ONE
        // pin of a multi-pin net leaves the remainder connected, `stranded`
        // reads 0, and the pin is silently floating diffusion. On the 5T OTA
        // that is 41 deletions a run and zero violations; the first stage that
        // noticed was LVS, five stages downstream, as an unpaired net. So count
        // them here and let `score` report them: the search can only negotiate
        // around damage it is told about.
        let mut sacrificed = vec![0usize; routes.wires.len()];
        loop {
            let mut removed = false;
            'pairs: for a in 0..routes.wires.len() {
                for b in (a + 1)..routes.wires.len() {
                    let hit = {
                        let (wa, wb) = (&routes.wires[a], &routes.wires[b]);
                        let mut found: Option<(usize, usize)> = None;
                        'scan: for (i, sa) in wa.iter().enumerate() {
                            for (j, sb) in wb.iter().enumerate() {
                                if conductor_layers_meet(sa, sb, layers, cuts)
                                    && rect_gap(sa.rect, sb.rect) == 0
                                {
                                    found = Some((i, j));
                                    break 'scan;
                                }
                            }
                        }
                        found
                    };
                    if let Some((i, j)) = hit {
                        if std::env::var("DR_TRACE").is_ok() {
                            eprintln!(
                                "DR_TRACE short-resolve: nets {a}/{b} shapes {:?} vs {:?} (access from {}/{}) pitch={} wire_w={} origin={:?}",
                                routes.wires[a][i], routes.wires[b][j], pre_access[a], pre_access[b],
                                cfg.pitch, cfg.wire_width, origin
                            );
                        }
                        // Prefer sacrificing access geometry; between two
                        // access shapes, the later net loses (deterministic).
                        //
                        // Note this choice is usually *forced*, not chosen: the
                        // other side is a trunk, which is never sacrificeable.
                        // That is how a single-pin net loses its only wire —
                        // its whole route is access (`pre_access` is 0), so it
                        // is the only legal victim however little it can afford
                        // it. Weighing the two sides by remaining geometry was
                        // tried and changed nothing for exactly this reason.
                        if j >= pre_access[b] {
                            routes.wires[b].remove(j);
                            sacrificed[b] += 1;
                        } else if i >= pre_access[a] {
                            routes.wires[a].remove(i);
                            sacrificed[a] += 1;
                        } else {
                            // Both pre-access: genuinely unexpected; drop the
                            // later net's shape rather than ship a short.
                            // `pre_access[b]` has to follow the shape down, or
                            // every access shape after it reads as trunk and
                            // becomes unsacrificeable on the next pass.
                            routes.wires[b].remove(j);
                            pre_access[b] -= 1;
                            sacrificed[b] += 1;
                        }
                        removed = true;
                        break 'pairs;
                    }
                }
            }
            if !removed {
                break;
            }
        }

        {
            // Foreign geometry per net: a filler that closes one net's sliver
            // by touching ANOTHER net is a drawn short — strictly worse than
            // the spacing finding it heals. Routes only: cells draw no stack
            // metal, and healing only ever adds stack-metal fillers.
            let flat: Vec<(usize, Shape)> = routes
                .wires
                .iter()
                .enumerate()
                .flat_map(|(i, w)| w.iter().map(move |s| (i, *s)))
                .collect();
            for ni in 0..routes.wires.len() {
                let foreign: Vec<Shape> = flat
                    .iter()
                    .filter(|&&(i, _)| i != ni)
                    .map(|&(_, s)| s)
                    .collect();
                heal_same_net_slivers(
                    &mut routes.wires[ni],
                    layers,
                    cfg.pitch - cfg.wire_width,
                    cfg.wire_width,
                    &foreign,
                );
                fill_same_net_notches(
                    &mut routes.wires[ni],
                    layers,
                    cfg.pitch - cfg.wire_width,
                    cfg.wire_width,
                    &foreign,
                );
            }
        }
        // Same-net, same-layer *containment*: drop a rect another rect on the
        // same net and layer already covers. A stacked via draws one landing pad
        // per cut it stitches, sized by that cut's enclosure, both centred on the
        // node — so met1 gets mcon's 290 nm pad and via1's 320 nm pad,
        // concentric. Their union is the larger one and the smaller adds nothing,
        // but it is a separate figure strictly inside the larger, which a spacing
        // rule that measures figure-to-figure reads as a 15 nm gap all the way
        // round. Removing it changes no conductor and no connectivity.
        //
        // ponytail: O(n^2) per net over a net's own shapes -- tens, not
        // thousands. Sort by area and sweep if a net ever carries enough wires
        // for this to show up in a profile.
        for wires in &mut routes.wires {
            let covered: Vec<bool> = wires
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    wires.iter().enumerate().any(|(j, t)| {
                        t.layer == s.layer
                            && contains(t.rect, s.rect)
                            // Equal rects cover each other; keep the first.
                            && (j < i || !contains(s.rect, t.rect))
                    })
                })
                .collect();
            let mut keep = covered.iter().map(|c| !c);
            wires.retain(|_| keep.next().unwrap_or(true));
        }
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
        let report = score(&routes, reqs, tel.overflow, cap_total, layers, cuts, &sacrificed);
        (routes, report)
    }
}

/// The two rects of an access jog's L, for one (width, orientation) choice.
/// `flip=false`: horizontal at the node row, vertical down the pin column;
/// `flip=true`: vertical at the node column, horizontal along the pin row.
/// Both legs run half a width past the junction and their own end, so the
/// union is a flush L.
fn jog_legs(nx: i32, ny: i32, px: i32, py: i32, w: i32) -> [[Rect; 2]; 2] {
    let h = w / 2;
    [
        [
            Rect { x: nx.min(px) - h, y: ny - h, w: (nx - px).abs() + w, h: w },
            Rect { x: px - h, y: ny.min(py) - h, w, h: (ny - py).abs() + w },
        ],
        [
            Rect { x: nx - h, y: ny.min(py) - h, w, h: (ny - py).abs() + w },
            Rect { x: nx.min(px) - h, y: py - h, w: (nx - px).abs() + w, h: w },
        ],
    ]
}

/// The track nodes a jog's legs sweep on `jog_l`, inflated by half a wire so a
/// neighbour's via pad cannot be laid beside the run.
///
/// Shared by the landing search — which rejects a corridor another net already
/// owns — and [`claim_jog_sweep`], which takes ownership of it. They have to
/// agree: reserving a corridor the search never checked is what let net2's via
/// pad sit 315 nm from vbias's 320 nm jog on the 5T OTA. The claim skips cells
/// another net took first, so without the check the jog is simply drawn through
/// foreign territory and the short resolver cleans up afterwards by deleting it.
fn jog_sweep_nodes(
    grid: &TrackGrid,
    pitch: i32,
    infl: i32,
    jog_l: u32,
    legs: &[Rect],
    mut f: impl FnMut(u32),
) {
    let bx = |v: i32| (v / pitch).clamp(0, grid.nx as i32 - 1) as u32;
    let by = |v: i32| (v / pitch).clamp(0, grid.ny as i32 - 1) as u32;
    for r in legs {
        for gy in by(r.y - infl)..=by(r.y + r.h + infl) {
            for gx in bx(r.x - infl)..=bx(r.x + r.w + infl) {
                f(grid.node(gx, gy, jog_l));
            }
        }
    }
}

/// Which lattice layer a jog off `pin_layer` runs on — the one derivation the
/// landing check and [`claim_jog_sweep`] both read.
fn jog_layer(
    cfg: &DetailedCfg,
    layers: &[LayerId],
    cuts: &[(LayerId, i32, i32, i32)],
    n_layers: u32,
    pin_layer: LayerId,
) -> u32 {
    let base_l = layers.iter().position(|&l| l == pin_layer).unwrap_or(0) as u32;
    match cfg.pin_access {
        // Reserved-conductor pin: jog runs on `layers[0]`, its pads off-lattice.
        Some((pl, _)) if pl == pin_layer => 0,
        _ if base_l + 1 < n_layers && (base_l as usize) < cuts.len() => base_l + 1,
        _ => base_l,
    }
    .min(n_layers.saturating_sub(1))
}

/// Whether a jog's legs stay clear of every foreign pin stitch zone and every
/// previously laid foreign leg — the joint criterion landing and drawing share.
fn jog_clean(
    legs: &[Rect; 2],
    ci: usize,
    zone: i32,
    zones: &[(u32, i32, i32)],
    laid: &[(usize, Rect)],
) -> bool {
    zones.iter().all(|&(zci, zx, zy)| {
        zci == ci as u32 || legs.iter().all(|&l| {
            let zr = Rect { x: zx - zone / 2, y: zy - zone / 2, w: zone, h: zone };
            rect_gap(l, zr) > 0
        })
    }) && laid
        .iter()
        .all(|&(lci, lr)| lci == ci || legs.iter().all(|&l| rect_gap(l, lr) > 0))
}

/// Claim (and reserve for `net`) the track nodes the access jog will sweep.
///
/// The jog is drawn *after* PathFinder by [`add_pin_access`], so the search
/// cannot price it, and a foreign trunk threads straight under it otherwise —
/// the `pair` short: a same-layer overlap drawn after `overuse` was summed, so
/// the report stayed clean over a hard short. Only the layers the jog actually
/// draws on are claimed: both L-legs run on `metal`, the stitch pads add `base`
/// at the two ends (the reserved-conductor case pads on li, which has no
/// lattice to claim). Each leg is inflated by half a wire width — a node
/// *outside* the leg can still put its own wire's edge inside it.
///
/// First-come: a node already claimed keeps its owner — ponytail: two jogs
/// crossing each other are not priced; a per-pin access search is the upgrade.
#[allow(clippy::too_many_arguments)]
fn claim_jog_sweep(
    grid: &TrackGrid,
    cfg: &DetailedCfg,
    layers: &[LayerId],
    cuts: &[(LayerId, i32, i32, i32)],
    n_layers: u32,
    net: u32,
    pin: Rect,
    pin_layer: LayerId,
    node: (i32, i32),
    // The (width, flip) `add_pin_access` will draw, when landing already chose
    // it. `None` = undecided, so both orientations get reserved.
    choice: Option<(i32, bool)>,
    claimed: &mut [bool],
    reserved: &mut [u32],
) {
    let (px, py) = node;
    let (cx, cy) = (pin.x + pin.w / 2, pin.y + pin.h / 2);
    let base_l = layers.iter().position(|&l| l == pin_layer).unwrap_or(0) as u32;
    let jog_l = jog_layer(cfg, layers, cuts, n_layers, pin_layer);
    // Reserve the jog that will actually be *drawn*, through the same
    // `jog_legs` the landing search and `add_pin_access` use — one definition
    // of where a jog goes, three readers.
    //
    // This used to hardcode the flip=false arms (horizontal along the node row,
    // vertical down the pin column). `add_pin_access` is free to draw flip=true
    // instead — horizontal along the *pin* row — and then the long run was
    // reserved nowhere, so the PathFinder was entitled to lay a via pad beside
    // it. On the 5T OTA it put net2's stitch pad 315 nm from vbias's pin-row
    // leg; both are 320 nm wide, so they overlapped by 5 nm, and the short
    // resolver deleted vbias's only wire to break it.
    let width = choice.map_or_else(|| cfg.wire_width.min(pin.w).min(pin.h).max(1), |(w, _)| w);
    let infl = cfg.wire_width / 2;
    let variants = jog_legs(px, py, cx, cy, width);
    // Orientation still undecided (the fallback landing tiers leave the jog to
    // `add_pin_access`): reserve both arms rather than guess, because guessing
    // wrong is precisely the bug above.
    let legs: Vec<Rect> = match choice {
        Some((_, flip)) => variants[usize::from(flip)].to_vec(),
        None => variants.concat(),
    };
    jog_sweep_nodes(grid, cfg.pitch, infl, jog_l, &legs, |n| {
        let s = n as usize;
        if !claimed[s] {
            claimed[s] = true;
            reserved[s] = net;
        }
    });
    let bx = |v: i32| (v / cfg.pitch).clamp(0, grid.nx as i32 - 1) as u32;
    let by = |v: i32| (v / cfg.pitch).clamp(0, grid.ny as i32 - 1) as u32;
    let mut claim = |gx: u32, gy: u32, l: u32| {
        let s = grid.node(gx, gy, l) as usize;
        if !claimed[s] {
            claimed[s] = true;
            reserved[s] = net;
        }
    };
    // Stitch-pad footprints on `base` at the two ends.
    if jog_l != base_l {
        for &(ex, ey) in &[(px, py), (cx, cy)] {
            claim(bx(ex), by(ey), base_l);
        }
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
#[allow(clippy::too_many_arguments)]
fn add_pin_access(
    routes: &mut Routes,
    access: &[(usize, Rect, LayerId, (i32, i32))],
    compact: &[usize],
    width: i32,
    // Min gap to hold off foreign geometry: the lattice's own inter-track gap.
    spacing: i32,
    layers: &[LayerId],
    cuts: &[(LayerId, i32, i32, i32)],
    pin_access: Option<(LayerId, (LayerId, i32, i32, i32))>,
    zones: &[(u32, i32, i32)],
    node_layers: &[u32],
    choices: &[Option<(i32, bool)>],
) {
    if layers.is_empty() {
        return; // no stack given: caller kept raw track indices, nothing to draw on
    }
    // Jog legs drawn so far, `(compact net, rect)` — each jog dodges earlier
    // foreign jogs (sequential first-come, like the landing order itself).
    let mut laid: Vec<(usize, Rect)> = Vec::new();
    for (ai, &(ci, pin, pin_layer, (nx, ny))) in access.iter().enumerate() {
        let node_layer = node_layers.get(ai).copied().unwrap_or(0);
        let net = compact[ci];
        // Start from the layer the pin is actually drawn on, and climb one step up
        // the stack from *there*. This used to assume `layers[0]` for every pin,
        // because `Pin` carried no layer — which is why li had to stay in the
        // routing stack at all, and why the PathFinder ended up laying tracks on
        // the layer the cells fill with S/D pads.
        //
        // A pin on a layer outside the stack has no declared cut to reach it, so it
        // falls back to the stack base and behaves as before. That is the remaining
        // gap: dropping li from `routing_metals` needs a pin-access stack (the pin
        // layer plus the cut joining it to `layers[0]`) handed in alongside.
        // (jog layer, the cut joining it to `base`, cut size, base pad, jog pad,
        // whether the *node* end needs its own stitch down to `base`).
        let (base, climb) = match pin_access {
            // Pin sits on the reserved conductor below the stack: climb straight to
            // `layers[0]` through its declared cut. This is the case that lets the
            // PathFinder stay off li entirely. The route arrives at the node **on
            // the jog's own layer** (`layers[0]`), so the node end needs no cut —
            // stitching it down to li anyway is what used to leave a li pad at an
            // arbitrary track position, 20–60 nm from li the cells already drew
            // (`LI.3 measured=110..150`, limit 170).
            Some((pl, (cut, size, below, above))) if pl == pin_layer => {
                (pl, layers.first().map(|&up| (up, cut, size, below, above, false)))
            }
            _ => {
                let i = layers.iter().position(|&l| l == pin_layer).unwrap_or(0);
                let base = layers.get(i).copied().unwrap_or(layers[0]);
                let climb = match (layers.get(i + 1), cuts.get(i)) {
                    (Some(&up), Some(&(cut, size, below, above))) => {
                        // The route arrives at the node on `base`, one layer below
                        // the jog, so the node end must stitch down to join it.
                        Some((up, cut, size, below, above, true))
                    }
                    _ => None,
                };
                (base, climb)
            }
        };
        let metal = climb.map_or(base, |(up, ..)| up);
        let (px, py) = (pin.x + pin.w / 2, pin.y + pin.h / 2);
        // The jog's L has four candidate drawings: {wide, narrow} width ×
        // {H-at-node-row, H-at-pin-row} orientation. Wide-flush is best (a
        // narrow leg under 290 nm stitch pads leaves the ledge slivers signoff
        // kept flagging), but a wide leg can reach a foreign pin's stitch zone
        // or a previously drawn foreign jog — the drawn-short class that once
        // fused chain4's devices. Take the first candidate that touches
        // neither; if all four foul, draw the last (narrow, flipped) and let
        // `cross_net_shorts` veto the epoch — a visible loss beats a silent
        // merge.
        let full = width.max(1);
        let narrow = full.min(pin.w).min(pin.h).max(1);
        let legs_of = |w: i32, flip: bool| -> [Rect; 2] {
            let all = jog_legs(nx, ny, px, py, w);
            all[usize::from(flip)]
        };
        // Landing's choice gets tried FIRST but is no longer final. It was
        // validated against pin zones and earlier jogs only — the trunk wires
        // and via pads it has to share `metal` with did not exist yet. The
        // lattice makes that fatal, not unlucky: rows sit every `pitch`, so an
        // off-lattice leg is at most pitch/2 from the nearest row, and two
        // `wire_width` shapes centred pitch/2 apart overlap by
        // wire_width - pitch/2 (90 nm at 320/460). Every off-lattice parallel
        // leg therefore sits inside the pad footprint of a lattice row and
        // survives only where no pad happened to land. When one did, the short
        // resolver deleted the jog and the pin went open (5T OTA net 6). Here,
        // after `build_routes`, the pads are real geometry in `routes.wires`,
        // so the candidates can be judged against them.
        let landing = choices.get(ai).copied().flatten();
        let cands: Vec<(i32, bool)> = landing
            .into_iter()
            .chain([(full, false), (full, true), (narrow, false), (narrow, true)])
            .collect();
        // Foreign geometry on a layer this jog can conduct into, at least `gap`
        // away from both legs.
        let clear = |legs: &[Rect; 2], gap: i32| {
            let probe = Shape { layer: metal, rect: legs[0] };
            routes.wires.iter().enumerate().all(|(n, w)| {
                n == net
                    || w.iter().all(|s| {
                        !conductor_layers_meet(s, &probe, layers, cuts)
                            || legs.iter().all(|&l| rect_gap(l, s.rect) >= gap)
                    })
            })
        };
        let pick = |gap: i32| {
            cands.iter().copied().find(|&(w, f)| {
                let legs = legs_of(w, f);
                jog_clean(&legs, ci, full, zones, &laid) && clear(&legs, gap)
            })
        };
        // Spacing-clean if any candidate is; else merely short-free (a spacing
        // finding the search can price beats an open it cannot see); else
        // today's behaviour — landing's choice, or the blind scan.
        let (width, flip) = pick(spacing).or_else(|| pick(1)).or(landing).unwrap_or_else(|| {
            let mut chosen = (full, false);
            for &(w, flip) in &[(full, false), (full, true), (narrow, false), (narrow, true)] {
                chosen = (w, flip);
                if jog_clean(&legs_of(w, flip), ci, full, zones, &laid) {
                    break;
                }
            }
            chosen
        });
        let legs = legs_of(width, flip);
        for &l in &legs {
            laid.push((ci, l));
        }
        let wires = &mut routes.wires[net];
        // Both legs run `half` PAST the junction and past their own end, so the
        // union is a clean L: flush corners, no step for a `notch` rule to find.
        // (Butting them exactly at the corner instead cost 13 `notch:li` on `pair`.)
        wires.push(Shape { layer: metal, rect: legs[0] });
        let second_span = if flip { (nx - px).abs() } else { (ny - py).abs() };
        if second_span > 0 {
            wires.push(Shape { layer: metal, rect: legs[1] });
        }
        // Stitch the jog down to `base` where a cut is needed: at the pin so it
        // joins the cell, and at the node only when the route arrives on `base`
        // (see `climb`). A cut with no pad enclosing it on both sides is an open,
        // so each stitched end gets a pad on each layer — **sized per layer**.
        // One `below.max(above)` square on both used to put met1's min-area pad
        // (290 nm on sky130) onto li, whose own pad is 230: the 60 nm overhang
        // was every one of the 20–60 nm `LI.3` margins against neighbouring
        // cell li that the cells had drawn at exactly min_spacing.
        if let Some((_, cut, size, below, above, stitch_node)) = climb {
            // When the node sits inside the pin's own pad the two stitches' pads
            // merge into one shape, so the node end is redundant — but its *cut*
            // does not merge, and two cuts that close are a spacing violation. Drop
            // the node stitch: the merged pad still carries the li route through to
            // the pin, so nothing is left open. Judged on the *smaller* pad — both
            // layers' pads must merge or the cut pair is real.
            let merged = (nx - px).abs().max((ny - py).abs()) < below.min(above);
            let ends: &[(i32, i32)] = if stitch_node && !merged {
                &[(nx, ny), (px, py)]
            } else {
                &[(px, py)]
            };
            for &(cx, cy) in ends {
                wires.push(Shape {
                    layer: cut,
                    rect: Rect { x: cx - size / 2, y: cy - size / 2, w: size, h: size },
                });
                // The pin's own drawn conductor hosts the cut when the cut rect
                // fits inside the pin rect — the deck's cover rule (li_covers_mcon)
                // is then met by cell geometry, and a base pad drawn anyway is
                // exactly what kept violating li min_spacing: cells legally draw a
                // gate pin and an S/D pin 50 nm apart diagonally, and any
                // min-area-legal pad centred on both is a violation no placement
                // of the pad can fix. Skip it; the pad exists for cuts that land
                // where nothing is drawn (the node end, or a pin smaller than its
                // cut), not for re-hosting a cut the cell already hosts.
                let hosted = cx == px
                    && cy == py
                    && cx - size / 2 >= pin.x
                    && cx - size / 2 + size <= pin.x + pin.w
                    && cy - size / 2 >= pin.y
                    && cy - size / 2 + size <= pin.y + pin.h;
                for (l, pad) in [(base, below), (metal, above)] {
                    if l == base && hosted {
                        continue;
                    }
                    wires.push(Shape {
                        layer: l,
                        rect: Rect { x: cx - pad / 2, y: cy - pad / 2, w: pad, h: pad },
                    });
                }
            }
        }

        // Layer-1 landing: the route arrives at the node on `layers[1]`
        // (met2), one layer ABOVE the jog's metal, and nothing above ever
        // descends — extraction drops single-node runs, so without this via
        // the whole jog cluster floats one cut short of its trunk. Measured on
        // chain4: three of net g's four pins extracted as isolated islands,
        // every epoch, deterministically. Stitch the node up: stack cut +
        // pads on both layers.
        if node_layer == 1 {
            if let (Some(&up), Some(&(cut1, size1, below1, above1))) =
                (layers.get(1), cuts.first())
            {
                let wires = &mut routes.wires[net];
                wires.push(Shape {
                    layer: cut1,
                    rect: Rect { x: nx - size1 / 2, y: ny - size1 / 2, w: size1, h: size1 },
                });
                for (l, pad) in [(metal, below1), (up, above1)] {
                    wires.push(Shape {
                        layer: l,
                        rect: Rect { x: nx - pad / 2, y: ny - pad / 2, w: pad, h: pad },
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

/// Metal on the layers immediately below and above each cut, so the cut is
/// enclosed even where no wire run happens to cover it.
///
/// A run of fewer than two nodes emits no wire (`gr::emit_run`), so a route that
/// climbs two layers in consecutive steps leaves the layer it passes through bare
/// and every cut there measures **zero** enclosure. Pads are same-net, same-layer
/// shapes that merge with any wire already present, so drawing them
/// unconditionally is cheaper than detecting the case.
///
/// The pad must satisfy every enclosure rule **on its own**: the checker
/// measures enclosure per host polygon, so the wire (whose best margin is
/// `wire_width/2 − cut/2` = 70 nm on sky130, under the 85 nm one-side limit)
/// can never help — and a pad smaller than the wire is doubly wrong, since a
/// boundary-distance checker reads a rect *strictly inside* another as a
/// separate figure a sliver away (`met2_min_spacing measured=15` on every quad
/// via). `Pdk::routing_vias` therefore sizes `below`/`above` to
/// `cut + 2 × max(min_enclosure, min_one_side)`; the pad then protrudes past
/// the wire (boundary contact ⇒ one merged figure), and the track pitch pays
/// for that width — see `elaborate::detailed_router`.
///
/// Takes vias **before** [`remap_cuts`], while `s.layer` is still the track index.
fn landing_pads(
    vias: &[Shape],
    layers: &[LayerId],
    cuts: &[(LayerId, i32, i32, i32)],
) -> Vec<Shape> {
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
    // Keep the ripped-up trees: a repair that cannot re-route must put the old
    // route BACK, not leave the net with no tree at all. Leaving it empty turned
    // a scored hard-rule violation into an open net — strictly worse, and it is
    // exactly how quad's epochs that PathFinder had fully connected came out of
    // the hard-round loop with a four-island net.
    let mut old: Vec<Vec<Vec<u32>>> = affected
        .iter()
        .map(|&net| std::mem::take(&mut hot.trees[net as usize]))
        .collect();
    for branches in &old {
        let mut nodes: Vec<u32> = branches.iter().flatten().copied().collect();
        nodes.sort_unstable();
        nodes.dedup();
        for n in nodes {
            hot.usage[n as usize] = hot.usage[n as usize].saturating_sub(1);
        }
    }
    for &node in blocked {
        hot.usage[node as usize] = hot.usage[node as usize].saturating_add(1);
    }
    let mut dij = Dij::new(cold.graph.nodes());
    for (i, &net) in affected.iter().enumerate() {
        let ni = net as usize;
        let corridor: &[u32] = cold.corridors.get(ni).map_or(&[], Vec::as_slice);
        let ml = cold.max_len.get(ni).copied().flatten();
        let routed = route_net(
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
        )
        // Same corridor→unrestricted fallback as `run_pathfinder`: a corridor
        // too tight to re-route in is not a reason to fail the repair.
        .or_else(|| {
            (!corridor.is_empty()).then(|| {
                route_net(
                    &cold.graph,
                    &hot.usage,
                    &hot.hist,
                    &[],
                    &cold.terms[ni],
                    &[],
                    net,
                    &cold.reserved,
                    p_fac,
                    ml,
                    &mut dij,
                )
            })
            .flatten()
        });
        // Failed both ways: reinstate the pre-repair route.
        hot.trees[ni] = match routed {
            Some((branches, _)) => branches,
            None => std::mem::take(&mut old[i]),
        };
        for &n in &hot.tree_nodes(ni) {
            hot.usage[n as usize] += 1;
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
fn score(
    routes: &Routes,
    reqs: &Requirements<Routes>,
    overuse: f32,
    cap_total: f32,
    layers: &[LayerId],
    cuts: &[(LayerId, i32, i32, i32)],
    // Access shapes the short resolver deleted, per net — a pin that may no
    // longer reach its net. Empty slice where no resolution ran.
    sacrificed: &[usize],
) -> Report {
    let (mut hard_violations, mut budget_violations) = gr::analog_tiers(routes, reqs);
    // An open net is a hard violation of the one contract this stage exists to
    // meet, so it goes in V, where the lexicographic key punishes it above any
    // budget or cost. Reported rather than asserted: mid-search epochs are
    // *allowed* to come out open — the orchestrator scores and discards them —
    // and a panic (`Routes::debug_check`) at that point kills a search that the
    // next negotiation round would have repaired. Only the winning solution has
    // to be connected, and the caller checks that once, after the loop.
    for (net, shapes) in routes.wires.iter().enumerate() {
        let stranded = unreachable_shapes(shapes);
        if stranded > 0 {
            hard_violations.push(pnr_core::report::Violation {
                rule: format!("open net {net}"),
                margin: stranded as i64,
            });
        }
    }
    // The opens the check above cannot see: a pin whose access geometry was
    // deleted to break a short. The net's surviving wires are still one
    // component, so `stranded` is 0 while the pin sits floating. Counted, not
    // measured — proving the pin is *electrically* open needs the deck's
    // derived-layer semantics (sky130's conductor is `diff NOT poly`, so the
    // cell's single diffusion rect is not one node), which belongs to `verify`,
    // not here. So this over-reports where the deletion was redundant (a pin
    // the cell straps to a sibling, e.g. a gate on its poly rail) and never
    // under-reports: every silent open starts as one of these deletions.
    //
    // ponytail: count, not geometry. Narrow it to genuinely-orphaned pins once
    // a `Pin` states which of its siblings the cell already ties it to.
    for (net, &n) in sacrificed.iter().enumerate() {
        if n > 0 {
            hard_violations.push(pnr_core::report::Violation {
                rule: format!("pin access sacrificed on net {net}"),
                margin: n as i64,
            });
        }
    }
    // A drawn short — two different nets' geometry touching on one conductor
    // layer (cuts counted on both layers they join) — is the other violation of
    // this stage's one contract, and the sneakiest: same-layer overlap is DRC-
    // legal, so nothing else reports it, and signoff surfaces it five stages
    // later as an extraction label conflict nobody can trace. Detected here it
    // lands in V, where the lexicographic key makes a shorting epoch unwinnable
    // and the negotiation searches its way around (feasibility-pump pressure,
    // same as opens). Pin-access jogs and stitch pads are drawn off-lattice
    // after the search, so no amount of track pricing can rule this out
    // structurally — measurement is the honest gate.
    for (a, b) in cross_net_shorts(&routes.wires, layers, cuts) {
        hard_violations.push(pnr_core::report::Violation {
            rule: format!("drawn short nets {a}/{b}"),
            margin: 1,
        });
    }
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

/// Fill sub-`min_space` gaps between one net's own same-layer metal pieces.
///
/// Aligned gaps get the shared-span bridge; corner-to-corner near misses get a
/// square over both facing corners. Fillers are clamped inside the pair's hull
/// and floored at `min_feat` so the filler itself clears min_width. Runs to a
/// fixpoint (bounded), since a filler can close one gap and reveal another.
fn heal_same_net_slivers(
    shapes: &mut Vec<Shape>,
    layers: &[LayerId],
    min_space: i32,
    min_feat: i32,
    foreign: &[Shape],
) {
    if min_space <= 0 {
        return;
    }
    // Everything routed is on the 5 nm fabrication grid; a filler must stay on
    // it too or it trades a spacing finding for a manufacturing_grid one.
    const GRID: i32 = 5;
    let snap = |r: Rect| -> Rect {
        let x = r.x.div_euclid(GRID) * GRID;
        let y = r.y.div_euclid(GRID) * GRID;
        let w = ((r.x + r.w) - x + GRID - 1).div_euclid(GRID) * GRID;
        let h = ((r.y + r.h) - y + GRID - 1).div_euclid(GRID) * GRID;
        Rect { x, y, w, h }
    };
    let touches = |a: Rect, b: Rect| rect_gap(a, b) == 0;
    for _ in 0..4 {
        let mut fillers: Vec<Shape> = Vec::new();
        for i in 0..shapes.len() {
            if !layers.contains(&shapes[i].layer) {
                continue;
            }
            for j in (i + 1)..shapes.len() {
                if shapes[j].layer != shapes[i].layer {
                    continue;
                }
                let (a, b) = (shapes[i].rect, shapes[j].rect);
                let dx = (b.x - (a.x + a.w)).max(a.x - (b.x + b.w));
                let dy = (b.y - (a.y + a.h)).max(a.y - (b.y + b.h));
                if dx.max(dy) <= 0 || dx.max(dy) >= min_space {
                    continue; // touching (one conductor) or already clear
                }
                let hx0 = a.x.min(b.x);
                let hy0 = a.y.min(b.y);
                let hx1 = (a.x + a.w).max(b.x + b.w);
                let hy1 = (a.y + a.h).max(b.y + b.h);
                let rect = if dx <= 0 {
                    // Vertical gap: fill the FULL hull width. A filler narrower
                    // than either piece leaves a step the notch rule reads as a
                    // concave slot (measured: one 120 nm notch per bridged jog).
                    let y0 = (a.y + a.h).min(b.y + b.h);
                    let y1 = a.y.max(b.y);
                    span_fill(hx0, hx1, hx0, hx1, y0, y1, min_feat, false)
                } else if dy <= 0 {
                    // Horizontal gap: full hull height, same reasoning.
                    let x0 = (a.x + a.w).min(b.x + b.w);
                    let x1 = a.x.max(b.x);
                    span_fill(hy0, hy1, hy0, hy1, x0, x1, min_feat, true)
                } else {
                    // Corner-to-corner: a patch over both facing corners,
                    // reaching `min_feat` into each rect, clamped to the hull.
                    let x0 = ((a.x + a.w).min(b.x + b.w) - min_feat).max(hx0);
                    let x1 = (a.x.max(b.x) + min_feat).min(hx1);
                    let y0 = ((a.y + a.h).min(b.y + b.h) - min_feat).max(hy0);
                    let y1 = (a.y.max(b.y) + min_feat).min(hy1);
                    Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
                };
                // Already bridged (by an earlier filler or a third piece
                // touching both): the pair is one conductor, the engine unions
                // touching rects into one polygon, and the spacing rule no
                // longer sees a gap. Re-filling every pass is what turned four
                // slivers into sixty-four duplicate fillers.
                let bridged = shapes
                    .iter()
                    .chain(fillers.iter())
                    .any(|s| s.layer == shapes[i].layer
                        && !(s.rect == a) && !(s.rect == b)
                        && touches(s.rect, a) && touches(s.rect, b));
                // A filler that reaches another net is a short, and one that
                // lands within min_space of one is the very violation class it
                // exists to remove — skip both and keep the original (smaller)
                // finding instead.
                let filler = snap(rect);
                let fouls = foreign.iter().any(|f| {
                    f.layer == shapes[i].layer && rect_gap(filler, f.rect) < min_space
                });
                if !bridged && !fouls && rect.w > 0 && rect.h > 0 {
                    fillers.push(Shape { layer: shapes[i].layer, rect: filler });
                }
            }
        }
        if fillers.is_empty() {
            return;
        }
        shapes.append(&mut fillers);
    }
}

/// Round `v` down onto the 5 nm fabrication grid everything routed sits on.
fn snap5(v: i32) -> i32 {
    v.div_euclid(5) * 5
}

/// Fill sub-`min_space` concave gaps and holes in one net's own same-layer
/// union — the cases [`heal_same_net_slivers`] cannot see.
///
/// That function reasons pairwise and bails the moment a third piece bridges the
/// pair, which is right for `min_spacing` (two figures joined by a third are one
/// figure) and wrong for `notch`: joining them is exactly what *creates* the
/// concave slot. A landing pad is wider than the wire that carries it (320 nm
/// against 290 on sky130, because the pad must satisfy the one-side via
/// enclosure alone — see [`landing_pads`]), so every pad that abuts a
/// perpendicular leg leaves a 15 nm ledge, and the next piece along turns that
/// ledge into a notch nothing pairwise will close.
///
/// Coordinate-compressed occupancy instead: lay the net's rects on the lattice
/// of their own edges, then fill an empty cell that is narrower than
/// `min_space` and has material on both sides along that axis. That is one
/// predicate for notches, slivers and min-area holes alike, and it is defined on
/// the union rather than on pairs, so a third piece helps instead of hiding the
/// problem.
///
/// `foreign` is every other net's geometry: a cell can look like a notch because
/// somebody else's wire is sitting in it, and filling that is a drawn short.
/// Fillers within `min_space` of foreign metal are dropped and the smaller
/// finding is kept.
///
/// ponytail: `O(cells)` with cells = (edges)², i.e. quadratic in one net's shape
/// count — tens of shapes, tens of thousands of cells, run twice. Sweep-line it
/// if a net ever carries thousands of wires.
fn fill_same_net_notches(
    shapes: &mut Vec<Shape>,
    layers: &[LayerId],
    min_space: i32,
    min_feat: i32,
    foreign: &[Shape],
) {
    if min_space <= 0 {
        return;
    }
    let mut present: Vec<LayerId> =
        shapes.iter().map(|s| s.layer).filter(|l| layers.contains(l)).collect();
    present.sort_unstable_by_key(|l| l.0);
    present.dedup();

    // A filler can complete one notch and expose the next along the same edge;
    // two passes settles every case measured, and the bound keeps it finite.
    for _ in 0..2 {
        let mut fillers: Vec<Shape> = Vec::new();
        for &layer in &present {
            let rects: Vec<Rect> = shapes
                .iter()
                .chain(fillers.iter())
                .filter(|s| s.layer == layer)
                .map(|s| s.rect)
                .collect();
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
                let i0 = xs.partition_point(|&v| v < r.x);
                let i1 = xs.partition_point(|&v| v < r.x + r.w);
                let j0 = ys.partition_point(|&v| v < r.y);
                let j1 = ys.partition_point(|&v| v < r.y + r.h);
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
                    let across_x = w < min_space
                        && i > 0
                        && i + 1 < nx
                        && full[(i - 1) * ny + j]
                        && full[(i + 1) * ny + j];
                    let across_y = h < min_space
                        && j > 0
                        && j + 1 < ny
                        && full[i * ny + j - 1]
                        && full[i * ny + j + 1];
                    if !(across_x || across_y) {
                        continue;
                    }
                    // Grown to `min_feat` on any axis it is short of. The
                    // checker measures min_width per drawn rectangle, not on
                    // the merged polygon, so a 40 x 10 patch that makes the
                    // union perfectly convex is itself five `min_width`
                    // findings. Growth is centred and almost always lands
                    // inside the material on either side, which adds nothing;
                    // where it does protrude it protrudes at least `min_feat`,
                    // so the bump is legal on its own.
                    let grow = |lo: i32, len: i32| {
                        if len >= min_feat {
                            (lo, len)
                        } else {
                            (snap5(lo - (min_feat - len) / 2), min_feat)
                        }
                    };
                    let (fx, fw) = grow(xs[i], w);
                    let (fy, fh) = grow(ys[j], h);
                    let rect = Rect { x: fx, y: fy, w: fw, h: fh };
                    if foreign
                        .iter()
                        .any(|f| f.layer == layer && rect_gap(rect, f.rect) < min_space)
                    {
                        continue;
                    }
                    fillers.push(Shape { layer, rect });
                }
            }
        }
        if fillers.is_empty() {
            return;
        }
        shapes.append(&mut fillers);
    }
}

/// [`heal_same_net_slivers`]\u{2019} aligned-gap filler: primary span
/// `[p0, p1]` widened to `min_feat` within the hull, bridging the gap
/// `[g0, g1]` extended `min_feat` into each side. `swap` flips axes.
fn span_fill(
    p0: i32,
    p1: i32,
    plo: i32,
    phi: i32,
    g0: i32,
    g1: i32,
    min_feat: i32,
    swap: bool,
) -> Rect {
    let (mut p0, mut p1) = (p0, p1);
    if p1 - p0 < min_feat {
        let grow = min_feat - (p1 - p0);
        p0 = (p0 - grow / 2).max(plo);
        p1 = (p0 + min_feat).min(phi);
    }
    let (g0, g1) = (g0 - min_feat, g1 + min_feat);
    if swap {
        Rect { x: g0, y: p0, w: g1 - g0, h: p1 - p0 }
    } else {
        Rect { x: p0, y: g0, w: p1 - p0, h: g1 - g0 }
    }
}

/// Whether two shapes can conduct into each other when touching: same layer,
/// or one is a cut whose joined pair includes the other's layer.
fn conductor_layers_meet(
    a: &Shape,
    b: &Shape,
    layers: &[LayerId],
    cuts: &[(LayerId, i32, i32, i32)],
) -> bool {
    if a.layer == b.layer {
        return true;
    }
    let joined = |cut: LayerId, other: LayerId| -> bool {
        cuts.iter().position(|&(c, ..)| c == cut).is_some_and(|i| {
            layers.get(i) == Some(&other) || layers.get(i + 1) == Some(&other)
        })
    };
    joined(a.layer, b.layer) || joined(b.layer, a.layer)
}

/// Every pair of nets whose drawn geometry touches on one conductor layer — a
/// cut counts on **both** layers it joins, since a cut landing on a foreign
/// conductor is precisely how extraction merges the two nets. Touch counts
/// (the extractor's own rule). O((Σ shapes)²) worst case over net pairs; at
/// analog net counts this is microseconds and runs once per epoch.
fn cross_net_shorts(
    wires: &[Vec<Shape>],
    layers: &[LayerId],
    cuts: &[(LayerId, i32, i32, i32)],
) -> Vec<(usize, usize)> {
    // Expanded conductor footprints per net: `(layer, rect)`, cuts twice.
    let expand = |shapes: &[Shape]| -> Vec<(LayerId, Rect)> {
        let mut out = Vec::with_capacity(shapes.len() + 4);
        for s in shapes {
            if let Some(i) = cuts.iter().position(|&(c, ..)| c == s.layer) {
                if let Some(&lo) = layers.get(i) {
                    out.push((lo, s.rect));
                }
                if let Some(&hi) = layers.get(i + 1) {
                    out.push((hi, s.rect));
                }
                // The pin-access cut (mcon) joins the reserved conductor below
                // the stack to layers[0]; the reserved layer itself is not in
                // `layers`, so the cut's lower footprint is unrepresentable
                // here — its upper (layers[0]) is what routes can touch.
                if cuts.get(i).is_none() {
                    out.push((s.layer, s.rect));
                }
            } else if layers.contains(&s.layer) {
                out.push((s.layer, s.rect));
            } else {
                // Reserved-conductor geometry (li pads) shorts on its own layer
                // exactly like stack metal — keep it under its real layer.
                out.push((s.layer, s.rect));
            }
        }
        out
    };
    let nets: Vec<Vec<(LayerId, Rect)>> = wires.iter().map(|w| expand(w)).collect();
    let mut out = Vec::new();
    for a in 0..nets.len() {
        if nets[a].is_empty() {
            continue;
        }
        for b in (a + 1)..nets.len() {
            let hit = nets[a].iter().any(|&(la, ra)| {
                nets[b]
                    .iter()
                    .any(|&(lb, rb)| la == lb && rect_gap(ra, rb) == 0)
            });
            if hit {
                out.push((a, b));
            }
        }
    }
    out
}

/// Shapes NOT reachable from the first by the xy-touch relation — `0` means one
/// connected component, i.e. the net is electrically whole in the sense
/// `Routes::debug_check` asserts. Same relation (touching counts, layer-blind)
/// so "0 open-net violations here" and "debug_check passes there" are one fact,
/// not two checks that can drift apart. O(k²) per net, like the original.
fn unreachable_shapes(shapes: &[Shape]) -> usize {
    if shapes.len() < 2 {
        return 0;
    }
    let mut seen = vec![false; shapes.len()];
    let mut stack = vec![0usize];
    seen[0] = true;
    while let Some(a) = stack.pop() {
        for b in 0..shapes.len() {
            if seen[b] {
                continue;
            }
            let (ra, rb) = (shapes[a].rect, shapes[b].rect);
            if ra.x <= rb.x + rb.w
                && rb.x <= ra.x + ra.w
                && ra.y <= rb.y + rb.h
                && rb.y <= ra.y + ra.h
            {
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
            (NetId(net), Rect { x, y, w: 170, h: 170 }, LAYERS[0])
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
            &[],
            &reqs,
            &LAYERS,
            &CUTS,
            &mut gr::Negotiation::new(),
            0,
        );

        for &(net, r, _) in &pins {
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
            layer: LayerId(0),
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

    /// Met1 blocking must follow the DRAWN extent, not `bbox`: upstream inflates
    /// bboxes to buy routing whitespace (guard-ring halo, whitespace escalation),
    /// and blocking the inflated bbox re-confiscates every nm of it. The macro
    /// here is that purchase in miniature — a small drawn shape inside a
    /// full-height wall of bbox. Blocked by bbox, the wall kills every
    /// horizontal met1 edge between the pins (the odd layer is vertical-only)
    /// and the net cannot cross; blocked by drawn extent, the row is free.
    #[test]
    fn inflated_bbox_does_not_block_met1() {
        let pins = [
            (NetId(0), Rect { x: 1_000, y: 7_000, w: 170, h: 170 }, LAYERS[0]),
            (NetId(0), Rect { x: 15_000, y: 7_000, w: 170, h: 170 }, LAYERS[0]),
        ];
        // Coarse plan: one horizontal band over the pin row, straight through
        // the wall's bbox.
        let global = Routes {
            wires: vec![vec![Shape {
                layer: LayerId(0),
                rect: Rect { x: 1_000, y: 6_600, w: 14_200, h: 1_000 },
            }]],
        };
        let wall = Macro {
            // Drawn geometry: one 500x500 block far below the pin row.
            shapes: vec![Shape {
                layer: LayerId(0),
                rect: Rect { x: 7_000, y: 200, w: 500, h: 500 },
            }],
            pins: Vec::new(),
            // Reservation-inflated bbox: full-height wall between the pins.
            bbox: Rect { x: 6_000, y: -5_000, w: 3_000, h: 30_000 },
        };
        let reqs = Requirements::<Routes>::default();
        let (routes, report) = DetailedRoute::default().route(
            &global,
            &pins,
            &[wall],
            &[],
            &reqs,
            &LAYERS,
            &CUTS,
            &mut gr::Negotiation::new(),
            0,
        );
        // Under bbox blocking the wall splits the net into two touched-but-
        // disconnected islands, so pin touch alone cannot discriminate; the
        // open-net report can.
        assert!(
            !report.hard_violations.iter().any(|v| v.rule.starts_with("open net")),
            "net split by the inflated bbox: {:?}",
            report.hard_violations.iter().map(|v| &v.rule).collect::<Vec<_>>()
        );
        for &(_, r, _) in &pins {
            assert!(
                routes.wires[0].iter().any(|s| {
                    s.rect.x <= r.x + r.w
                        && r.x <= s.rect.x + s.rect.w
                        && s.rect.y <= r.y + r.h
                        && r.y <= s.rect.y + s.rect.h
                }),
                "pin at ({}, {}) unreached: the inflated bbox still blocks met1",
                r.x,
                r.y
            );
        }
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
        let report = score(&routes, &reqs, 0.0, 0.0, &[], &[], &[]);
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
        // Track capacity is 1, so nets fighting over the same trunk row are
        // unavoidably contested. All five nets' terminals sit within ONE pitch
        // of the same two rows, so every net wants the same horizontal track
        // for its whole crossing: the first claimant prices it, the rest spill,
        // and the spill itself is contested again — history must climb. (The
        // routing frame grew a claim-free perimeter halo, so scarcity can no
        // longer be manufactured by clipping the die tight; it has to come from
        // the pins, which is the honest version anyway.)
        let cfg = DetailedCfg { pitch: 1_300, max_iters: 40, ..DetailedCfg::default() };
        let router = DetailedRoute { cfg };
        // Degenerate 1x1 pin rects: no access jogs, so no jog-sweep claims. The
        // fixture measures *negotiation over shared trunk nodes*, and the jog
        // claims (a hard, deterministic partition stamped before the search)
        // pre-resolve exactly the contention this test needs to observe.
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
                .route(&global, &pins, &[], &[], &reqs, &LAYERS, &CUTS, neg, 0)
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

    /// No two nets may end up with overlapping geometry on the same layer. This is
    /// the invariant `pair` was violating: a pin lands on one layer, `add_pin_access`
    /// stamps pads on *both* (a `Pin` has no layer to tell it which), and reserving
    /// only the landed layer left the sibling node free for another net's trunk —
    /// after `overuse` had already been summed, so `dr` reported a clean route over a
    /// hard short and LVS read the two nets as one.
    #[test]
    fn no_two_nets_share_geometry_on_a_layer() {
        let cfg = DetailedCfg::default();
        let router = DetailedRoute { cfg };
        // Pins packed within a pitch of each other on a shared column, which is what
        // makes two nets compete for the same landing site.
        let pins: Vec<(NetId, Rect, LayerId)> = (0..4u16)
            .flat_map(|n| {
                let off = i32::from(n) * 300;
                [
                    (NetId(n), Rect { x: 2_000 + off, y: 2_000, w: 170, h: 170 }, LAYERS[0]),
                    (NetId(n), Rect { x: 2_000 + off, y: 9_000, w: 170, h: 170 }, LAYERS[0]),
                ]
            })
            .collect();
        let global = Routes { wires: vec![Vec::new(); 4] };
        let reqs = Requirements::<Routes>::default();
        let mut neg = gr::Negotiation::new();
        let (routes, _) = router.route(&global, &pins, &[], &[], &reqs, &LAYERS, &CUTS, &mut neg, 0);

        let hits = |a: Rect, b: Rect| {
            a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
        };
        for (i, na) in routes.wires.iter().enumerate() {
            for (j, nb) in routes.wires.iter().enumerate().skip(i + 1) {
                for wa in na {
                    for wb in nb {
                        assert!(
                            wa.layer != wb.layer || !hits(wa.rect, wb.rect),
                            "nets {i} and {j} overlap on layer {:?}: {:?} vs {:?}",
                            wa.layer,
                            wa.rect,
                            wb.rect
                        );
                    }
                }
            }
        }
    }
}
