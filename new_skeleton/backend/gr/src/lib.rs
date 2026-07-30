//! # `gr` — global routing (an Algorithm-contract stage)
//!
//! [`GlobalRouter`] plans coarse routes over the placed macros, consuming the
//! routing [`Requirements`] (Net-arity rules scored against [`Routes`], including
//! the `verify`-fed antenna/DRC **Hard** rules and parasitic **Cost**). Drop-in;
//! the active algorithm is selected in `frontend/library`'s `algorithms` module.
//!
//! ## What this stage does (docs/routing/model-and-algorithms.md: Global routing)
//!
//! Build compact nets from the placed macro pins, then run the negotiated-
//! congestion PathFinder on a coarse `N×N` gcell grid over the die. The output is
//! a **coarse** [`Routes`]: each net's gcell route-tree segments emitted as
//! centre-line [`Shape`]s. That coarse `Routes` is the hand-off to `dr`, which
//! reconstructs terminals + corridors from it and realises DRC-clean track
//! geometry. Global geometry is guidance, not signoff (docs: "final wire geometry
//! comes from detailed routing").
//!
//! ## Objective / legality (ANALOG-BLIND)
//!
//! The [`Report`] cost = built-in mechanics (gcell overflow) **plus** the analog
//! objective `reqs.cost.iter().map(|b| b.cost(&routes)).sum()`; legality (V) counts
//! `reqs.hard` violations, and Θ is `reqs.budget`'s normalised residuals plus
//! residual gcell overflow — congestion the negotiation is still paying down is not
//! an illegal layout. No analog term is hardcoded — the analog rules judge the
//! produced [`Routes`] directly.

#![allow(dead_code)]

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use analog::Requirements;
use pnr_core::geom::{LayerId, Pin, Rect, Shape};
use pnr_core::{Layout, Macro, Report, Routes};

/// Coarse-grid tuning (docs: Global routing). Defaults mirror the ported
/// `GlobalRouteCfg`.
#[derive(Debug, Clone, Copy)]
pub struct GlobalCfg {
    pub gcells_per_side: u32,
    pub gcell_capacity: u16,
    pub max_iters: u32,
    pub p_fac: f32,
    pub hist_inc: f32,
    /// Fallback die margin (nm) added around the macro bounding box.
    pub die_margin: i32,
}

impl Default for GlobalCfg {
    fn default() -> Self {
        Self {
            gcells_per_side: 16,
            gcell_capacity: 6,
            max_iters: 40,
            p_fac: 3.0,
            hist_inc: 0.5,
            die_margin: 0,
        }
    }
}

/// Cross-epoch negotiated-congestion state: the PathFinder history cost `h_n` per
/// routing resource.
///
/// **Owned by the orchestrator, not by the router.** The router borrows it
/// mutably, accumulates into it, and hands it back — so it survives across outer
/// feedback iterations. That permanence *is* the anti-oscillation guarantee:
/// McMurchie & Ebeling's cost `c_n = (b_n + h_n)·p_n` converges because "the
/// effect of h_n is to permanently increase the cost of using congested nodes"
/// (PLAN §4e). A router constructed fresh each iteration discards `h_n` at exactly
/// the moment it starts to pay off, and the negotiation can loop forever.
///
/// It lives outside the router for a second reason: PLAN §5's termination
/// criterion has to *inspect* it. "Constraint prices are stationary" is a
/// statement about `h_n`, and a caller cannot read it through `&dyn GlobalRouter`.
///
/// Shared by `gr` and `dr` (both run a negotiation), which is why it lives here
/// rather than in either one — `dr` already depends on this crate.
#[derive(Default)]
pub struct Negotiation {
    // ponytail: opaque on purpose. Resource keying is the implementation's
    // business (gcell index for `gr`, track node for `dr`); exposing it would
    // freeze both routers' internal grids into this crate's public API.
    //
    // `BTreeMap`, not `HashMap`, and `max` rather than `+=` in
    // [`Negotiation::accumulate`]: `pressure()` is a float reduction over every
    // entry, so the container must iterate in a fixed order and the per-entry
    // update must not depend on the order nodes are visited in. A `HashMap` gives
    // neither, and a nondeterministic price feeding a hard-constraint gate is
    // PLAN §8f's named failure (iii).
    hist: std::collections::BTreeMap<Key, f32>,
}

/// `(tier, layer, bucket_x, bucket_y)` — one negotiated routing resource.
type Key = (Tier, u32, i32, i32);

/// Which router's resources a history entry belongs to.
///
/// Both routers quantise to the same physical buckets, so without a discriminant
/// a coarse gcell's accumulated price would be read back as a track node's price
/// and the two negotiations would silently drive each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    /// `gr` — gcell resources.
    Global,
    /// `dr` — track-node resources.
    Detailed,
}

/// Physical quantum (nm) that history is bucketed at.
///
/// The key has to be **placement-invariant**, and a raw grid index is not: both
/// grids are derived from the geometry's bounding box, the placer moves that box
/// every epoch, so "gcell 37" is a different piece of silicon each time and the
/// price accumulated on it means nothing. An absolute-nm bucket *is* invariant —
/// the same region of the die keys to the same bucket whatever the extent does,
/// which is the property `c_n = (b_n + h_n)·p_n` needs for `h_n` to be a price of
/// anything (PLAN §4e).
///
/// 500 nm sits below the coarsest gcell (die/16 ≈ 2 µm on `pair`) and at about one
/// detailed track pitch (430 nm), so neither tier collapses its whole grid into a
/// single bucket nor spreads one resource over many.
const HIST_QUANTUM_NM: i32 = 500;

#[inline]
fn hist_key(tier: Tier, (x, y, layer): (i32, i32, u32)) -> Key {
    // Euclidean-style floor division: `-1 / 500` truncates to 0 in Rust, which
    // would fold the whole strip left of the origin into bucket 0. Placed macro
    // geometry does reach negative coordinates.
    (
        tier,
        layer,
        x.div_euclid(HIST_QUANTUM_NM),
        y.div_euclid(HIST_QUANTUM_NM),
    )
}

impl Negotiation {
    /// A fresh negotiation with no history — first epoch of a run.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Σ `h_n` over all resources — the stationarity probe PLAN §5 terminates on.
    ///
    /// History that is still climbing means resources are still contested and the
    /// negotiation has not settled; history that has gone flat means the router has
    /// stopped trading routes and the solution is as negotiated as it will get.
    /// Feasibility alone does not certify that (a run can be legal while still
    /// actively re-routing), which is why the caller needs this number and not just
    /// a violation count.
    #[must_use]
    pub fn pressure(&self) -> f64 {
        self.hist.values().map(|&h| f64::from(h)).sum()
    }

    /// Seed a router's per-node history from the accumulated field, on entry.
    ///
    /// `pos` maps a node id to its **absolute** `(x, y, layer)` — absolute because
    /// the bucket key is (see [`HIST_QUANTUM_NM`]), so a router working in a shifted
    /// frame (which `dr` does) must add its frame origin back here.
    pub fn seed(&self, tier: Tier, hist: &mut [f32], pos: impl Fn(u32) -> (i32, i32, u32)) {
        if self.hist.is_empty() {
            return;
        }
        for (n, h) in hist.iter_mut().enumerate() {
            *h = self
                .hist
                .get(&hist_key(tier, pos(n as u32)))
                .copied()
                .unwrap_or(0.0);
        }
    }

    /// Fold a router's per-node history back into the accumulated field, on exit.
    ///
    /// `max`, not `+=`: several nodes can share a bucket, and summing them would
    /// make the price depend on grid resolution and on visit order. The maximum is
    /// the price of the most contested resource in that region — scale-free, and
    /// still monotone, which is the permanence the convergence argument rests on.
    pub fn accumulate(&mut self, tier: Tier, hist: &[f32], pos: impl Fn(u32) -> (i32, i32, u32)) {
        for (n, &h) in hist.iter().enumerate() {
            if h <= 0.0 {
                continue;
            }
            let e = self.hist.entry(hist_key(tier, pos(n as u32))).or_insert(0.0);
            *e = e.max(h);
        }
    }
}

/// What routing a single group's nets in isolation costs — the routability half of
/// a variant hypothesis's price (PLAN §2 step 2).
///
/// The parasitic half comes from `verify` separately; this struct is deliberately
/// geometry-only so pricing needs no PDK and no extraction.
pub struct GroupPrice {
    /// Half-perimeter wirelength of the group's own nets, nm.
    pub hpwl: i64,
    /// Residual gcell overflow after negotiation — how congested this variant's
    /// pin arrangement makes its own neighbourhood.
    pub overflow: i64,
    /// Every pin reached by its net. A variant whose pin faces cannot be reached at
    /// all is not merely expensive, it is **infeasible**, and no placement of it
    /// will help — that distinction is what lets the outer loop tell a
    /// position-space local minimum from a variant-space binding (PLAN §5).
    pub reachable: bool,
}

/// Price ONE group's variant hypothesis by actually routing its nets, cheaply, in
/// isolation.
///
/// This is PLAN §2's answer to why up-front variant pricing is myopic *and* why
/// pure annealing over variants wastes moves: a variant change relocates pins, so
/// routability is **discontinuous** in the variant index and there is no gradient
/// to interpolate. The only way to know the consequence is to draw it and route
/// it. Generators are byte-deterministic, so this measurement is reproducible.
///
/// A free function rather than a [`GlobalRouter`] method on purpose: it is not an
/// algorithm swap point (nobody wants to substitute a different *pricer*), and
/// putting it on the trait would force every impl to supply boilerplate. The
/// caller is `frontend/library`'s cell generation, which already depends on `gr`.
///
/// `macros` are the group's members **already placed relative to one another** in
/// the arrangement the variant implies — pricing an unplaced group would measure
/// nothing, since every pin would sit at the origin.
#[must_use]
pub fn price_group(macros: &[Macro], layers: &[LayerId], cfg: &GlobalCfg) -> GroupPrice {
    // `layers` is accepted for signature symmetry with `route` and because a future
    // pricer will want the stack depth; the coarse gcell plan is 2-D, so pricing
    // reads nothing off it.
    let _ = layers;

    let n_nets = macros
        .iter()
        .flat_map(|m| m.pins.iter())
        .map(|p| p.net.0 as usize + 1)
        .max()
        .unwrap_or(0);
    let mut nets = build_nets(macros, n_nets);
    if nets.pins.is_empty() {
        // No net with two distinct terminals inside the group: nothing to route, so
        // nothing is unreachable either. A one-pin net leaves the group and is the
        // block router's problem, not this hypothesis's.
        return GroupPrice {
            hpwl: 0,
            overflow: 0,
            reachable: true,
        };
    }

    // Grid over the GROUP's own bbox, not the die. A group is a few devices a few
    // µm across; sizing the grid from `die_extent` (which measures from the origin)
    // would spend all 16×16 gcells on the empty space between the origin and the
    // group and leave the group itself inside one gcell, where every net is
    // trivially "routed" and the measurement says nothing.
    let (lo_x, lo_y, hi_x, hi_y) = group_bbox(macros);
    let die = (
        (hi_x - lo_x + cfg.die_margin).max(2),
        (hi_y - lo_y + cfg.die_margin).max(2),
    );

    // HPWL from the pins, before any routing: it is the lower bound the route is
    // priced against, and it is defined even when the route fails.
    let hpwl: i64 = nets
        .pins
        .iter()
        .map(|pts| {
            let (mut x0, mut y0) = (i32::MAX, i32::MAX);
            let (mut x1, mut y1) = (i32::MIN, i32::MIN);
            for &(x, y) in pts {
                (x0, y0) = (x0.min(x), y0.min(y));
                (x1, y1) = (x1.max(x), y1.max(y));
            }
            i64::from(x1 - x0) + i64::from(y1 - y0)
        })
        .sum();

    let ggrid = GcellGrid::new(die, cfg.gcells_per_side, cfg.gcell_capacity);
    let terms: Vec<Vec<u32>> = nets
        .pins
        .iter()
        .map(|pts| {
            let mut t: Vec<u32> = pts
                .iter()
                .map(|&(x, y)| ggrid.at((x - lo_x) as f32, (y - lo_y) as f32))
                .collect();
            t.sort_unstable();
            t.dedup();
            t
        })
        .collect();

    let n_compact = nets.pins.len();
    let cold = RouteCtx {
        graph: ggrid,
        terms,
        net_w: std::mem::take(&mut nets.weights),
        order: std::mem::take(&mut nets.order),
        corridors: Vec::new(),
        reserved: Vec::new(),
        max_len: vec![None; n_compact],
    };
    let mut hot = RouteHot::new(cold.graph.nodes(), n_compact);
    // No `Negotiation`: a price is a measurement of this hypothesis in isolation.
    // Seeding it with the block's history would price the variant by where the
    // *previous* variant's routes were contested, which is exactly the myopia PLAN
    // §2 warns about, inverted.
    let tel = run_pathfinder(
        &mut hot,
        &cold,
        &mut (),
        cfg.p_fac,
        cfg.hist_inc,
        cfg.max_iters,
    );

    // Reachable = every terminal gcell ended up in its net's tree. `route_net`
    // returns `None` for a target it cannot reach and `run_pathfinder` then leaves
    // the tree as it was, so an unreachable pin shows up as a terminal missing from
    // the tree (and a wholly unroutable net as an empty tree).
    let reachable = (0..n_compact).all(|ci| {
        let tree = hot.tree_nodes(ci);
        cold.terms[ci].iter().all(|t| tree.binary_search(t).is_ok())
    });

    GroupPrice {
        hpwl,
        overflow: tel.overflow.ceil() as i64,
        reachable,
    }
}

/// `(lo_x, lo_y, hi_x, hi_y)` over macro bboxes **and** pins — the group's own
/// extent, wherever in the plane the arrangement put it.
fn group_bbox(macros: &[Macro]) -> (i32, i32, i32, i32) {
    let mut lo = (i32::MAX, i32::MAX);
    let mut hi = (i32::MIN, i32::MIN);
    for m in macros {
        for r in std::iter::once(&m.bbox).chain(m.shapes.iter().map(|s| &s.rect)) {
            lo = (lo.0.min(r.x), lo.1.min(r.y));
            hi = (hi.0.max(r.x + r.w), hi.1.max(r.y + r.h));
        }
        for p in &m.pins {
            lo = (lo.0.min(p.at.x), lo.1.min(p.at.y));
            hi = (hi.0.max(p.at.x + p.at.w), hi.1.max(p.at.y + p.at.h));
        }
    }
    if lo.0 == i32::MAX {
        return (0, 0, 2, 2);
    }
    (lo.0, lo.1, hi.0, hi.1)
}

/// A global-routing algorithm.
///
/// The trait takes `macros: &[Macro]` in addition to `placement` (matching the
/// `gp` contract): net connectivity to route lives on the placed macros' pins
/// (`Pin::net`), which `Layout` alone does not carry. `placement` gives group/axis
/// context; the die extent is derived from the macro geometry.
///
/// `layers` is the PDK-permitted routing metal stack (as [`LayerId`]s). The coarse
/// gcell plan is 2-D, so `gr` routes on the *first* routing layer of the stack
/// (`layers[0]`) rather than assuming layer 0; `dr` consumes the full stack.
pub trait GlobalRouter {
    /// Plan coarse routes over `placement`/`macros` on the `layers` stack,
    /// deterministically for `seed` **and** for the incoming `neg` history.
    ///
    /// `rings` are the guard rings drawn around this epoch's *placed* group boxes.
    /// They arrive as a separate slice rather than appended to `macros` because
    /// macro indices are device indices everywhere else in the flow, and because a
    /// ring is **both** an obstacle (its shapes block tracks) and a target (its
    /// pins carry a real `NetId` that has to be connected). A ring cannot be built
    /// before placement — it encloses a placed bbox — so it is placement-derived
    /// state handed in fresh every epoch, not part of the loop-invariant inputs.
    ///
    /// `neg` carries the negotiation history forward across epochs; see
    /// [`Negotiation`]. Determinism is over `(inputs, seed, neg)`, not `(inputs,
    /// seed)` alone — the same placement routed with more history routes
    /// differently, which is the whole point.
    fn route(
        &self,
        placement: &Layout,
        macros: &[Macro],
        rings: &[Macro],
        reqs: &Requirements<Routes>,
        layers: &[LayerId],
        neg: &mut Negotiation,
        seed: u64,
    ) -> (Routes, Report);
}

/// The default drop-in; replaced at the single selection point in `frontend/library`.
#[derive(Default)]
pub struct Placeholder;

impl GlobalRouter for Placeholder {
    fn route(
        &self,
        placement: &Layout,
        macros: &[Macro],
        rings: &[Macro],
        reqs: &Requirements<Routes>,
        layers: &[LayerId],
        neg: &mut Negotiation,
        seed: u64,
    ) -> (Routes, Report) {
        GlobalRoute::default().route(placement, macros, rings, reqs, layers, neg, seed)
    }
}

/// The real coarse router.
pub struct GlobalRoute {
    pub cfg: GlobalCfg,
}

impl Default for GlobalRoute {
    fn default() -> Self {
        Self {
            cfg: GlobalCfg::default(),
        }
    }
}

impl GlobalRouter for GlobalRoute {
    fn route(
        &self,
        placement: &Layout,
        macros: &[Macro],
        rings: &[Macro],
        reqs: &Requirements<Routes>,
        layers: &[LayerId],
        neg: &mut Negotiation,
        _seed: u64,
    ) -> (Routes, Report) {
        // PathFinder is deterministic for identical inputs (docs: Complexity and
        // determinism — the search does not sample the RNG), so `seed` selects
        // nothing here. `neg` DOES select: identical inputs with different incoming
        // history route differently, by design.
        let cfg = self.cfg;

        // Route against **placed** geometry. A `Macro`'s pins and shapes are in
        // macro-local coordinates; routing them directly would connect the pins
        // of a device sitting at the origin regardless of where the placer put
        // it. Worse, two devices on one net then land at near-identical local
        // points, dedup collapses them below two distinct terminals, and the net
        // is silently dropped as obstacle-only — which is why most nets came back
        // unrouted with zero wirelength.
        //
        // ponytail: clones the macro list once per call. Analog blocks are tens
        // of devices; if that ever matters, translate lazily inside `build_nets`
        // instead.
        let mut macros_owned = place_macros(macros, placement);
        // Rings are a net TARGET (D5): each band carries a `ring` pin on the
        // well/substrate net, and until it enters `build_nets` the ring is an
        // island — it reached signoff as geometry and the router never saw it.
        // They are ALREADY absolute (placement-derived, drawn around a placed
        // bbox), so they must not go through `place_macros`; appending them after
        // the devices also keeps `macros` indices device indices, which every other
        // stage relies on.
        macros_owned.extend(rings.iter().cloned());
        let macros: &[Macro] = &macros_owned;

        // Coarse global routing is a single 2-D gcell plan, so it lands on the
        // first routing layer the PDK permits (met1 in a bottom-up stack) instead
        // of assuming layer 0. `dr` fans this out across the whole `layers` stack.
        let base_layer = layers.first().copied().unwrap_or(LayerId(0));

        // Total NetId space so per-net Routes indexing stays stable.
        let n_nets = macros
            .iter()
            .flat_map(|m| m.pins.iter())
            .map(|p| p.net.0 as usize + 1)
            .max()
            .unwrap_or(0);
        let nets = build_nets(macros, n_nets);
        let n_compact = nets.pins.len();

        let die = die_extent(macros, cfg.die_margin);

        // Empty problem → empty Routes, but still scored by the analog rules.
        if n_compact == 0 {
            let routes = Routes {
                wires: vec![Vec::new(); n_nets],
            };
            let report = score(&routes, reqs, 0.0, 0.0);
            return (routes, report);
        }

        // ---- Global (gcell) PathFinder ----
        let ggrid = GcellGrid::new(die, cfg.gcells_per_side, cfg.gcell_capacity);
        let terms: Vec<Vec<u32>> = nets
            .pins
            .iter()
            .map(|pts| {
                let mut t: Vec<u32> = pts
                    .iter()
                    .map(|&(x, y)| ggrid.at(x as f32, y as f32))
                    .collect();
                t.sort_unstable();
                t.dedup();
                t
            })
            .collect();

        let cold = RouteCtx {
            graph: ggrid,
            terms,
            net_w: nets.weights.clone(),
            order: nets.order.clone(),
            corridors: Vec::new(),
            reserved: Vec::new(),
            max_len: vec![None; n_compact],
        };
        let mut hot = RouteHot::new(cold.graph.nodes(), n_compact);
        // Carry the negotiation in: `hist_inc` bumps inside `run_pathfinder` are
        // per-epoch, and without this they died with the call — the router paid the
        // whole price of discovering a contested gcell and then forgot it, every
        // outer iteration (PLAN §4e).
        neg.seed(Tier::Global, &mut hot.hist, |n| cold.graph.pos(n));
        // Rings as OBSTACLE: a band is drawn metal and really does consume wiring
        // resource in the gcells it crosses. Charged as usage rather than forbidden
        // outright, because the ring's own net has to reach the pins that sit *on*
        // the band — a hard blockage makes the ring unroutable by construction.
        // Capped below `cap` so a ring can never manufacture overflow on its own:
        // overflow is a Θ residual now, and a fabricated residual is a budget the
        // search can never pay down.
        for m in rings {
            // Routing metal only. A ring also draws tap/implant/cut bands over the
            // same rects, and charging those too would put the whole band at
            // capacity for the sake of layers nothing routes on.
            for s in m.shapes.iter().filter(|s| layers.contains(&s.layer)) {
                charge_rect(&mut hot.usage, &cold.graph, s.rect);
            }
        }
        let tel = run_pathfinder(
            &mut hot,
            &cold,
            &mut (),
            cfg.p_fac,
            cfg.hist_inc,
            cfg.max_iters,
        );
        neg.accumulate(Tier::Global, &hot.hist, |n| cold.graph.pos(n));

        // ---- Emit coarse Routes: gcell route-tree segments as centre-line
        // Shapes on layer 0 (the hand-off currency for `dr`). ----
        let mut wires = vec![Vec::new(); n_nets];
        for (ci, tree) in hot.trees.iter().enumerate() {
            let net_id = nets.net_id[ci] as usize;
            let dst = &mut wires[net_id];
            for branch in tree {
                for w in branch.windows(2) {
                    let (ax, ay, _) = cold.graph.pos(w[0]);
                    let (bx, by, _) = cold.graph.pos(w[1]);
                    dst.push(seg_shape(ax, ay, bx, by, base_layer));
                }
                // A single-node branch (a lone terminal) still marks its gcell so
                // `dr` recovers a terminal there.
                if branch.len() == 1 {
                    let (ax, ay, _) = cold.graph.pos(branch[0]);
                    dst.push(seg_shape(ax, ay, ax, ay, base_layer));
                }
            }
        }
        let routes = Routes { wires };

        // Built-in mechanic: residual gcell overflow, normalised by the grid's total
        // wiring capacity so its Θ entry is a residual, not a raw node count. Analog
        // objective added on top.
        let cap_total = f32::from(cold.graph.cap()) * cold.graph.nodes() as f32;
        let report = score(&routes, reqs, tel.overflow, cap_total);
        (routes, report)
    }
}

/// Add one unit of usage to every gcell `r` overlaps, without ever reaching
/// capacity (see the ring-obstacle comment in [`GlobalRoute::route`]).
fn charge_rect(usage: &mut [u16], g: &GcellGrid, r: Rect) {
    let ceiling = g.cap().saturating_sub(1);
    if ceiling == 0 {
        return; // capacity 1: any charge is a hard block, which is not what this is
    }
    let lo = g.at(r.x as f32, r.y as f32);
    let hi = g.at((r.x + r.w) as f32, (r.y + r.h) as f32);
    for gy in (lo / g.nx)..=(hi / g.nx) {
        for gx in (lo % g.nx)..=(hi % g.nx) {
            let n = (gy * g.nx + gx) as usize;
            usage[n] = (usage[n] + 1).min(ceiling);
        }
    }
}

/// Die bounds `(w, h)` from the macro geometry bounding box plus `margin` (nm).
fn die_extent(macros: &[Macro], margin: i32) -> (i32, i32) {
    let mut hi_x = 0i32;
    let mut hi_y = 0i32;
    for m in macros {
        let r = &m.bbox;
        hi_x = hi_x.max(r.x + r.w);
        hi_y = hi_y.max(r.y + r.h);
        for p in &m.pins {
            hi_x = hi_x.max(p.at.x + p.at.w);
            hi_y = hi_y.max(p.at.y + p.at.h);
        }
    }
    ((hi_x + margin).max(2), (hi_y + margin).max(2))
}

/// A degenerate-thin centre-line segment `Shape` on `layer` between two points.
/// (Coarse geometry — `dr` re-realises it as real track wires.)
fn seg_shape(ax: i32, ay: i32, bx: i32, by: i32, layer: LayerId) -> Shape {
    let (x0, y0) = (ax.min(bx), ay.min(by));
    Shape {
        layer,
        rect: Rect {
            x: x0,
            y: y0,
            w: (ax.max(bx) - x0).max(1),
            h: (ay.max(by) - y0).max(1),
        },
    }
}

/// The analog half of a routing [`Report`]'s two violation tiers: `(V, Θ)` from
/// `reqs.hard` and `reqs.budget`, one entry per *violating* batch.
///
/// Shared by [`score`] and `dr::score` (hence `pub`) because the two stages must
/// report the same batch on the same scale — `frontend/library::lex_key` sums their
/// Θ contributions together, so a divergence here is invisible per-crate and
/// silently corrupts the middle tier.
///
/// Both margins are **measured** residuals, never violation counts: D10 requires it,
/// since `Report::phi` and `Report::lex` both *sum* margins and a count makes "one
/// batch, three violations" read identically to a 3-unit shortfall. The batch is the
/// only `dyn` seam, so the rule identity inside a violating batch is unrecoverable
/// and the entry is named `"routing {tier} batch {i}"` — a reporting-granularity
/// limit, not a measurement one.
///
/// The milli-budget scale lives in `Violation::from_residual` (pnr_core), the one
/// home for the factor — `library::lex_key` sums Θ across placement and routing, so
/// every stage must scale a residual identically.
#[must_use]
pub fn analog_tiers(
    routes: &Routes,
    reqs: &Requirements<Routes>,
) -> (Vec<pnr_core::report::Violation>, Vec<pnr_core::report::Violation>) {
    use pnr_core::report::Violation;
    let mut hard = Vec::new();
    for (i, batch) in reqs.hard.iter().enumerate() {
        if batch.violations(routes) > 0 {
            hard.push(Violation::from_residual(
                format!("routing hard batch {i}"),
                batch.residual(routes),
            ));
        }
    }
    // Θ from the declared budgets. `ParasiticBudget`, `CouplingBudget` and
    // `CrosstalkExclusion` are the three the annotator registers on the routing side
    // (D15); until this loop existed their residuals reached nobody and the whole
    // middle tier of routing was the built-in mechanic alone.
    //
    // Gated on `residual > 0` rather than `violations > 0`: a budget's residual *is*
    // its violation, and a batch that is over spec by an amount too small to matter
    // still has to appear or `Report::feasible` would call it clean.
    let budget = reqs
        .budget
        .iter()
        .enumerate()
        .filter_map(|(i, batch)| {
            let residual = batch.residual(routes);
            (residual > 0.0)
                .then(|| Violation::from_residual(format!("routing budget batch {i}"), residual))
        })
        .collect();
    (hard, budget)
}

/// Assemble the [`Report`]: legality = analog `hard` violations + residual
/// `overflow`; cost = built-in mechanic (`overflow`) + analog `cost` sum.
/// **Analog-blind**: the analog terms come straight from the rule batches.
///
/// Which tier a batch belongs to is **not** this function's decision — it is
/// `Requirements`' three-arm partition, and all three arms are now read: `hard` → V,
/// `budget` → Θ, `cost` → PEX. The built-in routing budget joins Θ beside them:
/// congestion above capacity is a budget the negotiation is still paying down, not an
/// illegal layout, and it is exactly the number the orchestrator used to recover by
/// string-matching the rule name.
///
/// `cap_total` is the grid's total wiring capacity (gcell capacity × node count) —
/// overflow's denominator. Overflow has no budget of its own to normalise by, so
/// `Σ(usage − cap) / Σcap` is the residual that makes it commensurate with the
/// milli-budget entries beside it in Θ (D17). `Report::cost` deliberately keeps the
/// **raw** overflow: the PEX tier's terms are relative weights within one stage, and
/// rescaling one of them would silently re-weight that tier.
pub(crate) fn score(
    routes: &Routes,
    reqs: &Requirements<Routes>,
    overflow: f32,
    cap_total: f32,
) -> Report {
    use pnr_core::report::Violation;
    let (hard_violations, mut budget_violations) = analog_tiers(routes, reqs);
    debug_assert!(
        cap_total > 0.0 || overflow == 0.0,
        "gr::score: positive overflow with no capacity denominator"
    );
    if overflow > 0.0 && cap_total > 0.0 {
        // Normalised by total capacity and milli-scaled by the constructor, so a
        // congested gcell and a blown coupling budget finally weigh on one scale.
        // `from_residual` ceils, so a real overflow can never round to a margin of
        // 0, which `Report::lex` reads as satisfied.
        budget_violations
            .push(Violation::from_residual("routing overflow", f64::from(overflow / cap_total)));
    }
    // Criticality blend, matching `dr::score`: the PEX tier sums across stages
    // (`library::lex_key` adds `pc + rc`), so the two routing stages must weigh a
    // cost batch on one convention. Numerically a no-op today — `StraightNet` has
    // no `headroom` override, so criticality is pinned at 1.0 — which is exactly
    // when to align conventions: before a rule that declares headroom makes the
    // divergence load-bearing.
    let analog_cost: f32 = reqs.cost.iter().map(|b| b.criticality(routes) * b.cost(routes)).sum();
    Report {
        hard_violations,
        budget_violations,
        cost: overflow + analog_cost,
    }
}

// ===========================================================================
// Net construction
// ===========================================================================
//
// Net construction from placed macro pins (docs: Net construction).
//
// The old router read a hypergraph + `ConstraintRecord` (net names, classes,
// straight nets, priority overrides). The new contract is **analog-blind**: the
// only routing input beyond `Layout` is the placed `Macro`s, whose `Pin`s carry
// `net: NetId` and geometry. Net classification / straight-net / priority data
// lived in the old `ConstraintRecord` and is not reachable through the
// type-erased `Requirements` batches, so weight is uniform and order is driven by
// the point count (docs Route-order key 4). The analog cost/hard rules still
// judge the produced `Routes` unchanged — the objective is not hardcoded here.

/// Compact-net setup: physical points per routed net, obstacle-only points, and
/// the deterministic route order.
pub struct NetSetup {
    /// `net_of[net_id] = Some(compact index)` for routed nets, else `None`.
    pub net_of: Vec<Option<u32>>,
    /// Physical terminal points per compact net (sorted, deduped, ≥2).
    pub pins: Vec<Vec<(i32, i32)>>,
    /// Original `NetId` of each compact net (for indexing the produced `Routes`).
    pub net_id: Vec<u32>,
    /// Uniform per-net weight (telemetry/ordering only; docs: it does not scale
    /// Dijkstra cost).
    pub weights: Vec<f32>,
    /// Route order over compact indices (docs: Route order).
    pub order: Vec<u32>,
    /// Single-point "obstacle-only" landing points (docs: Net construction #4).
    pub obstacles: Vec<(i32, i32)>,
}

/// Pin centre in `nm` from its `Rect` bounding box.
#[inline]
fn pin_centre(at: &Rect) -> (i32, i32) {
    (at.x + at.w / 2, at.y + at.h / 2)
}

/// Build compact nets from every pin on every placed macro (docs: Net
/// construction). `n_nets` is the total `NetId` space so `net_of` can be indexed
/// directly by net id.
#[must_use]
pub fn build_nets(macros: &[Macro], n_nets: usize) -> NetSetup {
    // Gather physical points per net id.
    let mut pts_of: Vec<Vec<(i32, i32)>> = vec![Vec::new(); n_nets];
    for m in macros {
        for p in &m.pins {
            let ni = p.net.0 as usize;
            if ni < n_nets {
                pts_of[ni].push(pin_centre(&p.at));
            }
        }
    }

    let mut s = NetSetup {
        net_of: vec![None; n_nets],
        pins: Vec::new(),
        net_id: Vec::new(),
        weights: Vec::new(),
        order: Vec::new(),
        obstacles: Vec::new(),
    };
    for (ni, pts) in pts_of.iter_mut().enumerate() {
        if pts.is_empty() {
            continue;
        }
        let raw = pts.len();
        pts.sort_unstable();
        pts.dedup();
        // Fewer than two distinct points → obstacle-only, omitted from routing.
        // Two *pins* collapsing to one *point* is the silent-drop mode: the net
        // has real terminals to connect and is dropped anyway, and nothing
        // downstream says so — it surfaces as an `unconnected_pin` at signoff.
        debug_assert!(
            pts.len() >= 2 || raw < 2,
            "gr::build_nets: net {ni} has {raw} pins that collapsed to {} distinct \
             point(s) — it will be dropped as obstacle-only. Pin centres coincide, \
             which usually means two macros are stacked or pins were never rebound \
             to their real nets.",
            pts.len()
        );
        if pts.len() < 2 {
            s.obstacles.extend_from_slice(pts);
            continue;
        }
        s.net_of[ni] = Some(s.pins.len() as u32);
        s.net_id.push(ni as u32);
        s.pins.push(std::mem::take(pts));
        s.weights.push(1.0);
    }

    // Route order: fewer physical points first (docs key 4; the higher-priority
    // keys need net-class/straight/override data this crate cannot reach). Ties
    // broken by compact index for determinism.
    let mut order: Vec<u32> = (0..s.pins.len() as u32).collect();
    order.sort_by(|&a, &b| {
        s.pins[a as usize]
            .len()
            .cmp(&s.pins[b as usize].len())
            .then(a.cmp(&b))
    });
    s.order = order;
    s
}

// ===========================================================================
// Routing resource graphs (was `grid.rs`)
// ===========================================================================
//
// Routing resource graphs: the coarse gcell grid (global) and the fine track
// lattice (detailed). Ported from `backend/engine/src/routing.rs`
// (`GcellGrid`/`TrackGrid`) — the uniform-capacity lattice the docs specify in
// `docs/routing/model-and-algorithms.md` (Global routing, Track graph).

/// Sentinel node/owner id (`u32::MAX`) — "no node" / "unreserved".
pub const NONE: u32 = u32::MAX;

/// A routing resource graph with uniform per-node capacity. `dr` shares this
/// trait so one PathFinder core drives both the gcell and track grids.
pub trait RGraph {
    /// Total node count.
    fn nodes(&self) -> usize;
    /// Uniform per-node capacity.
    fn cap(&self) -> u16;
    /// Up to six `(neighbour, base_cost)` edges out of `n`; returns the count
    /// written into `out`.
    fn neighbors(&self, n: u32, out: &mut [(u32, f32); 6]) -> usize;
    /// `(x, y, layer)` centre of node `n`, in `nm`.
    fn pos(&self, n: u32) -> (i32, i32, u32);
    /// Corridor-region id of `n` (gcell id for the track grid; the node itself
    /// for the gcell grid). Default 0 = single region.
    fn region(&self, _n: u32) -> u32 {
        0
    }
}

// ---------------------------------------------------------------------------
// Global: 2-D gcell grid
// ---------------------------------------------------------------------------

/// The coarse `N × N` gcell grid over the die (docs: Global routing). Every node
/// has uniform `capacity` and four planar unit-cost neighbours.
pub struct GcellGrid {
    pub nx: u32,
    pub ny: u32,
    pub gw: f32,
    pub gh: f32,
    pub capacity: u16,
}

impl GcellGrid {
    /// `N = max(n_per_side, 2)` gcells per side over `die` (nm).
    #[must_use]
    pub fn new(die: (i32, i32), n_per_side: u32, capacity: u16) -> Self {
        let nx = n_per_side.max(2);
        let ny = n_per_side.max(2);
        Self {
            nx,
            ny,
            gw: die.0 as f32 / nx as f32,
            gh: die.1 as f32 / ny as f32,
            capacity,
        }
    }

    /// Gcell id containing physical point `(x, y)`.
    #[must_use]
    pub fn at(&self, x: f32, y: f32) -> u32 {
        let ix = ((x / self.gw) as u32).min(self.nx - 1);
        let iy = ((y / self.gh) as u32).min(self.ny - 1);
        iy * self.nx + ix
    }
}

impl RGraph for GcellGrid {
    fn nodes(&self) -> usize {
        (self.nx * self.ny) as usize
    }
    fn cap(&self) -> u16 {
        self.capacity
    }
    fn neighbors(&self, n: u32, out: &mut [(u32, f32); 6]) -> usize {
        let (x, y) = (n % self.nx, n / self.nx);
        let mut k = 0;
        if x > 0 {
            out[k] = (n - 1, 1.0);
            k += 1;
        }
        if x + 1 < self.nx {
            out[k] = (n + 1, 1.0);
            k += 1;
        }
        if y > 0 {
            out[k] = (n - self.nx, 1.0);
            k += 1;
        }
        if y + 1 < self.ny {
            out[k] = (n + self.nx, 1.0);
            k += 1;
        }
        k
    }
    fn pos(&self, n: u32) -> (i32, i32, u32) {
        let (x, y) = (n % self.nx, n / self.nx);
        (
            ((x as f32 + 0.5) * self.gw) as i32,
            ((y as f32 + 0.5) * self.gh) as i32,
            0,
        )
    }
    fn region(&self, n: u32) -> u32 {
        n
    }
}

// ---------------------------------------------------------------------------
// Detailed: 3-D track grid
// ---------------------------------------------------------------------------

/// The fine track lattice (docs: Track graph). Capacity is always 1; even layers
/// run horizontally, odd layers vertically; adjacent layers connect by
/// `via_cost` at the same track coordinate.
pub struct TrackGrid {
    pub nx: u32,
    pub ny: u32,
    pub pitch: i32,
    pub via_cost: f32,
    pub allowed: Vec<bool>,
    /// met1 landing holes inside blocked cells: via access only, no lateral wire.
    pub terminal_only: Vec<bool>,
    pub n_layers: u32,
    region_of: Vec<u32>,
}

impl TrackGrid {
    /// Track grid over `die` with `n_layers >= 1`. `pitch` is raised to at least
    /// `max(die dim)/1200` and 1 (docs: Track graph).
    #[must_use]
    pub fn with_layers(die: (i32, i32), pitch: i32, via_cost: f32, n_layers: u32) -> Self {
        let n_layers = n_layers.max(1);
        let pitch = pitch.max(((die.0.max(die.1)) / 1200).max(1));
        let nx = (die.0 / pitch).max(2) as u32;
        let ny = (die.1 / pitch).max(2) as u32;
        Self {
            nx,
            ny,
            pitch,
            via_cost,
            allowed: vec![true; (n_layers * nx * ny) as usize],
            terminal_only: vec![false; (n_layers * nx * ny) as usize],
            n_layers,
            region_of: vec![0; (nx * ny) as usize],
        }
    }

    /// Precompute each track node's gcell region (for corridor restriction).
    pub fn set_regions(&mut self, gcells: &GcellGrid) {
        for i in 0..self.layer_size() {
            let (x, y, _) = self.pos(i);
            let rx = ((x as f32 / gcells.gw) as u32).min(gcells.nx - 1);
            let ry = ((y as f32 / gcells.gh) as u32).min(gcells.ny - 1);
            self.region_of[i as usize] = ry * gcells.nx + rx;
        }
    }

    #[must_use]
    pub fn layer_size(&self) -> u32 {
        self.nx * self.ny
    }

    #[must_use]
    pub fn node(&self, ix: u32, iy: u32, layer: u32) -> u32 {
        layer * self.layer_size() + iy * self.nx + ix
    }

    /// Containing-pitch-bin node for `(x, y)` on `layer` (docs: `nearest`).
    #[must_use]
    pub fn nearest(&self, x: i32, y: i32, layer: u32) -> u32 {
        let ix = ((x / self.pitch) as u32).min(self.nx - 1);
        let iy = ((y / self.pitch) as u32).min(self.ny - 1);
        self.node(ix, iy, layer)
    }

    #[must_use]
    pub fn ixy(&self, n: u32) -> (u32, u32, u32) {
        let layer = n / self.layer_size();
        let r = n % self.layer_size();
        (r % self.nx, r / self.nx, layer)
    }

    /// Block met1 nodes whose bins fall in the cell rectangle (docs: every drawn
    /// cell rect blocks met1).
    pub fn block_met1(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        let clamp_x = |v: i32| (v / self.pitch).clamp(0, self.nx as i32 - 1) as u32;
        let clamp_y = |v: i32| (v / self.pitch).clamp(0, self.ny as i32 - 1) as u32;
        for iy in clamp_y(y0)..=clamp_y(y1) {
            for ix in clamp_x(x0)..=clamp_x(x1) {
                let n = self.node(ix, iy, 0) as usize;
                self.allowed[n] = false;
            }
        }
    }

    /// Claim one unique landing node for a terminal (docs: Pin landing).
    /// Chebyshev rings 0..=8, layers 0 and 1 only, with the layer-1 stacked-node
    /// and min-area rejections. Returns the claimed node id.
    pub fn claim_minarea(
        &mut self,
        x: i32,
        y: i32,
        claimed: &mut [bool],
        ok: impl Fn(i32, i32) -> bool,
        min_area: i64,
    ) -> Option<u32> {
        let cx = (x / self.pitch).clamp(0, self.nx as i32 - 1);
        let cy = (y / self.pitch).clamp(0, self.ny as i32 - 1);
        let pad = self.pitch;
        for layer in 0..self.n_layers.min(2) {
            for r in 0..=8i32 {
                for dy in -r..=r {
                    for dx in -r..=r {
                        if dx.abs().max(dy.abs()) != r {
                            continue;
                        }
                        let (ix, iy) = (cx + dx, cy + dy);
                        if ix < 0 || iy < 0 || ix >= self.nx as i32 || iy >= self.ny as i32 {
                            continue;
                        }
                        let n = self.node(ix as u32, iy as u32, layer) as usize;
                        let (px, py, _) = self.pos(n as u32);
                        let stacked = layer == 1
                            && (-2..=2i32).any(|ddy| {
                                let jy = iy + ddy;
                                ddy != 0
                                    && jy >= 0
                                    && jy < self.ny as i32
                                    && claimed[self.node(ix as u32, jy as u32, 1) as usize]
                            });
                        if !claimed[n] && !stacked && (layer == 1 || ok(px, py)) {
                            if min_area > 0 {
                                let span = (px - x).abs().max((py - y).abs()).max(pad);
                                let area = i64::from(span) * i64::from(pad);
                                if area < min_area {
                                    continue;
                                }
                            }
                            claimed[n] = true;
                            if !self.allowed[n] {
                                self.terminal_only[n] = true;
                            }
                            self.allowed[n] = true;
                            return Some(n as u32);
                        }
                    }
                }
            }
        }
        None
    }

    /// Up to `k` un-marked landing candidates (node, displacement) for a pin,
    /// for the multi-candidate refinement (docs: Pin landing refinement).
    #[must_use]
    pub fn claim_candidates(
        &self,
        x: i32,
        y: i32,
        claimed: &[bool],
        ok: impl Fn(i32, i32) -> bool,
        min_area: i64,
        k: usize,
    ) -> Vec<(u32, i32)> {
        let cx = (x / self.pitch).clamp(0, self.nx as i32 - 1);
        let cy = (y / self.pitch).clamp(0, self.ny as i32 - 1);
        let pad = self.pitch;
        let mut out = Vec::with_capacity(k);
        for layer in 0..self.n_layers.min(2) {
            for r in 0..=8i32 {
                for dy in -r..=r {
                    for dx in -r..=r {
                        if dx.abs().max(dy.abs()) != r {
                            continue;
                        }
                        let (ix, iy) = (cx + dx, cy + dy);
                        if ix < 0 || iy < 0 || ix >= self.nx as i32 || iy >= self.ny as i32 {
                            continue;
                        }
                        let n = self.node(ix as u32, iy as u32, layer) as usize;
                        let (px, py, _) = self.pos(n as u32);
                        let stacked = layer == 1
                            && (-2..=2i32).any(|ddy| {
                                let jy = iy + ddy;
                                ddy != 0
                                    && jy >= 0
                                    && jy < self.ny as i32
                                    && claimed[self.node(ix as u32, jy as u32, 1) as usize]
                            });
                        if !claimed[n] && !stacked && (layer == 1 || ok(px, py)) {
                            if min_area > 0 {
                                let span = (px - x).abs().max((py - y).abs()).max(pad);
                                let area = i64::from(span) * i64::from(pad);
                                if area < min_area {
                                    continue;
                                }
                            }
                            let disp = (px - x).abs() + (py - y).abs();
                            out.push((n as u32, disp));
                            if out.len() >= k {
                                return out;
                            }
                        }
                    }
                }
            }
        }
        out
    }
}

impl RGraph for TrackGrid {
    fn nodes(&self) -> usize {
        (self.n_layers * self.layer_size()) as usize
    }
    fn cap(&self) -> u16 {
        1
    }
    fn neighbors(&self, n: u32, out: &mut [(u32, f32); 6]) -> usize {
        let (ix, iy, layer) = self.ixy(n);
        let mut k = 0;
        let push = |node: u32, cost: f32, out: &mut [(u32, f32); 6], k: &mut usize| {
            if self.allowed[node as usize] {
                out[*k] = (node, cost);
                *k += 1;
            }
        };
        if layer % 2 == 0 {
            let lat = |a: u32, b: u32| {
                layer != 0 || (!self.terminal_only[a as usize] && !self.terminal_only[b as usize])
            };
            if ix > 0 {
                let m = self.node(ix - 1, iy, layer);
                if lat(n, m) {
                    push(m, 1.0, out, &mut k);
                }
            }
            if ix + 1 < self.nx {
                let m = self.node(ix + 1, iy, layer);
                if lat(n, m) {
                    push(m, 1.0, out, &mut k);
                }
            }
        } else {
            if iy > 0 {
                push(self.node(ix, iy - 1, layer), 1.0, out, &mut k);
            }
            if iy + 1 < self.ny {
                push(self.node(ix, iy + 1, layer), 1.0, out, &mut k);
            }
        }
        if layer > 0 {
            push(self.node(ix, iy, layer - 1), self.via_cost, out, &mut k);
        }
        if layer + 1 < self.n_layers {
            push(self.node(ix, iy, layer + 1), self.via_cost, out, &mut k);
        }
        k
    }
    fn pos(&self, n: u32) -> (i32, i32, u32) {
        let (ix, iy, layer) = self.ixy(n);
        (
            ix as i32 * self.pitch + self.pitch / 2,
            iy as i32 * self.pitch + self.pitch / 2,
            layer,
        )
    }
    fn region(&self, n: u32) -> u32 {
        self.region_of[(n % self.layer_size()) as usize]
    }
}

// ===========================================================================
// Negotiated-congestion PathFinder (was `pathfinder.rs`)
// ===========================================================================
//
// Negotiated-congestion PathFinder (docs: Global routing / Detailed PathFinder).
//
// Ported from `backend/engine/src/routing.rs`: the multi-source congestion-aware
// Dijkstra (`route_net`) and the dirty-net negotiated-congestion loop. The engine
// expressed the loop through its generic `Stage`/`Core`/`Ledger` framework; here
// it is one concrete `run_pathfinder` since the router is the only consumer.

/// Per-net negotiated-congestion state: usage, history, and route trees.
pub struct RouteHot {
    pub usage: Vec<u16>,
    pub hist: Vec<f32>,
    /// `trees[net]` is a list of branches; each branch is a node path.
    pub trees: Vec<Vec<Vec<u32>>>,
}

impl RouteHot {
    #[must_use]
    pub fn new(nodes: usize, nets: usize) -> Self {
        Self {
            usage: vec![0; nodes],
            hist: vec![0.0; nodes],
            trees: vec![Vec::new(); nets],
        }
    }

    /// Sorted-deduped distinct node ids of `net`'s tree.
    #[must_use]
    pub fn tree_nodes(&self, net: usize) -> Vec<u32> {
        let mut v = Vec::new();
        self.tree_nodes_into(net, &mut v);
        v
    }

    /// Buffer-reusing variant for the hot propose/commit path.
    pub fn tree_nodes_into(&self, net: usize, out: &mut Vec<u32>) {
        out.clear();
        out.extend(self.trees[net].iter().flatten().copied());
        out.sort_unstable();
        out.dedup();
    }
}

/// Immutable per-run routing context (docs: terminals, order, corridors,
/// reservations, per-net length bound).
pub struct RouteCtx<G> {
    pub graph: G,
    pub terms: Vec<Vec<u32>>,
    pub net_w: Vec<f32>,
    pub order: Vec<u32>,
    pub corridors: Vec<Vec<u32>>,
    pub reserved: Vec<u32>,
    /// Per-net max search cost in Dijkstra units; `None` = unbounded.
    pub max_len: Vec<Option<f32>>,
}

/// A reconciler run after each epoch — mirrors the engine `Ledger`. Crosstalk
/// reconciliation keeps the zero-overflow stop open while `open() > 0`; a
/// no-move epoch still ends the stage (docs: Detailed PathFinder). `()` is the
/// no-op ledger used by global routing.
pub trait Ledger<G: RGraph> {
    fn reconcile(&mut self, _hot: &RouteHot, _cold: &RouteCtx<G>) {}
    fn open(&self) -> usize {
        0
    }
}

impl<G: RGraph> Ledger<G> for () {}

/// Stage telemetry (docs: `Report`/telemetry inputs).
#[derive(Default, Clone, Copy)]
pub struct Telemetry {
    pub iters: u32,
    pub overflow: f32,
    pub proposed: u64,
    pub accepted: u64,
}

// ---------------------------------------------------------------------------
// Congestion-aware multi-source Dijkstra
// ---------------------------------------------------------------------------

/// Stamp-based scratch for `route_net` — distance/prev/membership arrays reused
/// across searches without per-call clearing.
pub struct Dij {
    dist: Vec<f32>,
    prev: Vec<u32>,
    seen: Vec<u32>,
    stamp: u32,
    heap: BinaryHeap<Reverse<(u32, u32)>>,
    in_tree: Vec<u32>,
    tree_stamp: u32,
    is_old: Vec<u32>,
    old_stamp: u32,
}

impl Dij {
    #[must_use]
    pub fn new(nodes: usize) -> Self {
        Self {
            dist: vec![0.0; nodes],
            prev: vec![NONE; nodes],
            seen: vec![0; nodes],
            stamp: 0,
            heap: BinaryHeap::new(),
            in_tree: vec![0; nodes],
            tree_stamp: 0,
            is_old: vec![0; nodes],
            old_stamp: 0,
        }
    }
}

#[inline]
fn visit(
    seen: &mut [u32],
    dist: &mut [f32],
    prev: &mut [u32],
    stamp: u32,
    n: u32,
    d: f32,
    from: u32,
) -> bool {
    let i = n as usize;
    if seen[i] != stamp || d < dist[i] {
        seen[i] = stamp;
        dist[i] = d;
        prev[i] = from;
        true
    } else {
        false
    }
}

/// Connect a net's terminals into a branch tree by multi-source Dijkstra from
/// the growing tree (docs: Global routing cost model). `None` if a target is
/// unreachable within the corridor/reservation/length bound.
#[allow(clippy::too_many_arguments)]
pub fn route_net<G: RGraph>(
    g: &G,
    usage: &[u16],
    hist: &[f32],
    old_nodes: &[u32],
    terms: &[u32],
    corridor: &[u32],
    net: u32,
    reserved: &[u32],
    p_fac: f32,
    max_len: Option<f32>,
    dij: &mut Dij,
) -> Option<(Vec<Vec<u32>>, f64)> {
    if terms.is_empty() {
        return Some((Vec::new(), 0.0));
    }
    let Dij {
        dist,
        prev,
        seen,
        stamp,
        heap,
        in_tree,
        tree_stamp,
        is_old,
        old_stamp,
    } = dij;
    let mut branches = vec![vec![terms[0]]];
    let mut tree: Vec<u32> = vec![terms[0]];
    *tree_stamp = tree_stamp.wrapping_add(1);
    in_tree[terms[0] as usize] = *tree_stamp;
    let mut len = 0.0f64;
    let cap = g.cap();

    *old_stamp = old_stamp.wrapping_add(1);
    for &n in old_nodes {
        is_old[n as usize] = *old_stamp;
    }
    let os = *old_stamp;
    let is_old: &[u32] = is_old;
    // effective_usage subtracts this net's own old-tree claim (docs: cost model).
    let node_cost = |n: u32| -> f32 {
        let eff = usage[n as usize].saturating_sub(u16::from(is_old[n as usize] == os));
        hist[n as usize] + p_fac * f32::from((eff + 1).saturating_sub(cap))
    };

    // Targets ordered by Manhattan distance from the first terminal.
    let mut targets: Vec<u32> = terms[1..].to_vec();
    let (x0, y0, _) = g.pos(terms[0]);
    targets.sort_by_key(|&t| {
        let (x, y, _) = g.pos(t);
        (x - x0).abs() + (y - y0).abs()
    });

    let mut buf = [(0u32, 0.0f32); 6];
    for &target in &targets {
        if in_tree[target as usize] == *tree_stamp {
            continue;
        }
        *stamp = stamp.wrapping_add(1);
        heap.clear();
        for &s in &tree {
            visit(seen, dist, prev, *stamp, s, 0.0, NONE);
            heap.push(Reverse((0.0f32.to_bits(), s)));
        }
        let mut found = false;
        while let Some(Reverse((db, n))) = heap.pop() {
            let d = f32::from_bits(db);
            if seen[n as usize] == *stamp && d > dist[n as usize] {
                continue;
            }
            if n == target {
                found = true;
                break;
            }
            let k = g.neighbors(n, &mut buf);
            for &(nb, base) in &buf[..k] {
                if !corridor.is_empty() && corridor.binary_search(&g.region(nb)).is_err() {
                    continue;
                }
                if let Some(&owner) = reserved.get(nb as usize) {
                    if owner != NONE && owner != net {
                        continue;
                    }
                }
                let nd = d + base + node_cost(nb);
                if let Some(ml) = max_len {
                    if nd > ml {
                        continue;
                    }
                }
                if visit(seen, dist, prev, *stamp, nb, nd, n) {
                    heap.push(Reverse((nd.to_bits(), nb)));
                }
            }
        }
        if !found {
            return None;
        }
        let mut path = vec![target];
        let mut cur = target;
        while prev[cur as usize] != NONE {
            cur = prev[cur as usize];
            path.push(cur);
        }
        path.reverse();
        len += path.len() as f64 - 1.0;
        for &n in &path {
            tree.push(n);
            in_tree[n as usize] = *tree_stamp;
        }
        branches.push(path);
    }
    Some((branches, len))
}

// ---------------------------------------------------------------------------
// Negotiated-congestion loop
// ---------------------------------------------------------------------------

/// The first `CORRIDOR_EPOCHS` epochs restrict each search to its global corridor
/// (docs: Detailed corridors).
const CORRIDOR_EPOCHS: u32 = 8;

/// Run PathFinder to convergence (docs: Global routing / Detailed PathFinder).
///
/// Every epoch reroutes dirty nets (empty tree or any over-capacity node) in
/// route order, accepting every proposed reroute; overused nodes then add
/// `hist_inc * (usage - cap)` to history. Stops after ≥1 epoch when overflow is
/// zero **and** the ledger has no open reconciliation, at `max_iters`, or when an
/// epoch proposes no reroutes.
pub fn run_pathfinder<G: RGraph, L: Ledger<G>>(
    hot: &mut RouteHot,
    cold: &RouteCtx<G>,
    ledger: &mut L,
    p_fac: f32,
    hist_inc: f32,
    max_iters: u32,
) -> Telemetry {
    let cap = cold.graph.cap();
    let mut dij = Dij::new(cold.graph.nodes());
    let mut nodes_buf: Vec<u32> = Vec::new();
    let mut tel = Telemetry::default();

    for epoch in 0..max_iters {
        tel.iters = epoch + 1;
        let mut moved = false;
        for &net_u in &cold.order {
            let net = net_u as usize;
            let dirty = hot.trees[net].is_empty()
                || hot.trees[net]
                    .iter()
                    .flatten()
                    .any(|&n| hot.usage[n as usize] > cap);
            if !dirty || cold.terms[net].is_empty() {
                continue;
            }
            hot.tree_nodes_into(net, &mut nodes_buf);
            let corridor: &[u32] = if epoch < CORRIDOR_EPOCHS {
                cold.corridors.get(net).map_or(&[], Vec::as_slice)
            } else {
                &[]
            };
            let ml = cold.max_len.get(net).copied().flatten();
            tel.proposed += 1;
            let routed = route_net(
                &cold.graph,
                &hot.usage,
                &hot.hist,
                &nodes_buf,
                &cold.terms[net],
                corridor,
                net_u,
                &cold.reserved,
                p_fac,
                ml,
                &mut dij,
            )
            // A failed corridor search immediately retries unrestricted.
            .or_else(|| {
                (!corridor.is_empty()).then(|| {
                    route_net(
                        &cold.graph,
                        &hot.usage,
                        &hot.hist,
                        &nodes_buf,
                        &cold.terms[net],
                        &[],
                        net_u,
                        &cold.reserved,
                        p_fac,
                        ml,
                        &mut dij,
                    )
                })?
            });
            let Some((branches, _len)) = routed else {
                continue;
            };
            // Commit: drop the old claim, install branches, re-claim.
            hot.tree_nodes_into(net, &mut nodes_buf);
            for &n in &nodes_buf {
                hot.usage[n as usize] -= 1;
            }
            hot.trees[net] = branches;
            hot.tree_nodes_into(net, &mut nodes_buf);
            for &n in &nodes_buf {
                hot.usage[n as usize] += 1;
            }
            moved = true;
            tel.accepted += 1;
        }

        // Epoch: bump history on overused nodes.
        for (n, &u) in hot.usage.iter().enumerate() {
            if u > cap {
                hot.hist[n] += hist_inc * f32::from(u - cap);
            }
        }
        ledger.reconcile(hot, cold);

        let overflow: f32 = hot
            .usage
            .iter()
            .map(|&u| f32::from(u.saturating_sub(cap)))
            .sum();
        tel.overflow = overflow;

        // A no-move epoch ends the stage even if a contract stays open.
        if !moved {
            break;
        }
        if overflow == 0.0 && ledger.open() == 0 {
            break;
        }
    }
    tel
}

// ===========================================================================
// Route-tree → Shape geometry + EOL / PRL scans (was `geom.rs`)
// ===========================================================================
//
// Route-tree → `Shape` geometry (docs: Geometry extraction) plus the post-route
// EOL / wide-net PRL scans. Ported from `backend/engine/src/routing.rs`.
//
// Emits `pnr_core::Shape` directly (the new `Routes` currency) rather than the
// engine's separate `Wire`/`Via` structs — a `Shape` is a `(layer, rect)`, and a
// via is a square `Shape` on its lower layer.

/// One extracted centreline segment before it becomes a `Shape` — carries the
/// endpoints the EOL/PRL scans need (a `Shape` rect loses run direction).
#[derive(Clone, Copy)]
pub struct Wire {
    pub net: u32,
    pub layer: u32,
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
    pub width: i32,
}

/// A layer transition — drawn as a square pad `Shape` on the lower layer.
#[derive(Clone, Copy)]
pub struct Via {
    pub net: u32,
    pub x: i32,
    pub y: i32,
    pub size: i32,
    pub layer: u32,
}

/// Axis-aligned bounding rect of a wire centreline (half-width on each side).
#[must_use]
pub fn wire_rect(w: &Wire) -> (i32, i32, i32, i32) {
    let h = w.width / 2;
    (
        w.x0.min(w.x1) - h,
        w.y0.min(w.y1) - h,
        w.x0.max(w.x1) + h,
        w.y0.max(w.y1) + h,
    )
}

/// Extract wires/vias from the detailed track trees, with optional `min_area`
/// symmetric extension of short runs (docs: Geometry extraction). Vias are
/// deduped by `(x, y, layer)`.
#[must_use]
pub fn extract_geometry_minarea(
    hot: &RouteHot,
    grid: &TrackGrid,
    width: i32,
    min_area: i64,
) -> (Vec<Wire>, Vec<Via>) {
    let mut wires = Vec::new();
    let mut vias = Vec::new();
    for (net, tree) in hot.trees.iter().enumerate() {
        for branch in tree {
            let mut run_start = 0usize;
            for i in 1..branch.len() {
                let (px, py, pl) = grid.pos(branch[i - 1]);
                let (cx, cy, cl) = grid.pos(branch[i]);
                if pl != cl {
                    emit_run(&mut wires, grid, net as u32, &branch[run_start..i], width);
                    vias.push(Via {
                        net: net as u32,
                        x: cx.min(px),
                        y: cy.min(py),
                        size: width,
                        layer: pl.min(cl),
                    });
                    run_start = i;
                }
            }
            emit_run(&mut wires, grid, net as u32, &branch[run_start..], width);
        }
    }
    if min_area > 0 {
        for w in &mut wires {
            let len = (w.x1 - w.x0).abs() + (w.y1 - w.y0).abs();
            let area = i64::from(len.max(w.width)) * i64::from(w.width);
            if area < min_area {
                let need = ((min_area + i64::from(w.width) - 1) / i64::from(w.width)) as i32;
                let ext = (need - len) / 2 + 1;
                if w.x0 == w.x1 {
                    w.y0 -= ext;
                    w.y1 += ext;
                } else {
                    w.x0 -= ext;
                    w.x1 += ext;
                }
            }
        }
    }
    vias.sort_unstable_by_key(|v| (v.x, v.y, v.layer, v.net));
    vias.dedup_by_key(|v| (v.x, v.y, v.layer));
    (wires, vias)
}

fn emit_run(wires: &mut Vec<Wire>, grid: &TrackGrid, net: u32, nodes: &[u32], width: i32) {
    if nodes.len() < 2 {
        return;
    }
    let (mut sx, mut sy, layer) = grid.pos(nodes[0]);
    let (mut lx, mut ly, _) = grid.pos(nodes[0]);
    for &n in &nodes[1..] {
        let (x, y, _) = grid.pos(n);
        let straight = if layer % 2 == 1 { x == lx } else { y == ly };
        if !straight {
            wires.push(Wire {
                net,
                layer,
                x0: sx,
                y0: sy,
                x1: lx,
                y1: ly,
                width,
            });
            (sx, sy) = (lx, ly);
        }
        (lx, ly) = (x, y);
    }
    if (lx, ly) != (sx, sy) {
        wires.push(Wire {
            net,
            layer,
            x0: sx,
            y0: sy,
            x1: lx,
            y1: ly,
            width,
        });
    }
}

/// Turn extracted wires+vias into per-net `Shape` lists — the `Routes::wires`
/// layout (indexed by net). A via is a `size × size` pad on its lower layer.
#[must_use]
pub fn to_shapes(nets: usize, wires: &[Wire], vias: &[Via]) -> Vec<Vec<Shape>> {
    let mut out = vec![Vec::new(); nets];
    for w in wires {
        let (x0, y0, x1, y1) = wire_rect(w);
        out[w.net as usize].push(Shape {
            layer: LayerId(w.layer as u16),
            rect: Rect {
                x: x0,
                y: y0,
                w: x1 - x0,
                h: y1 - y0,
            },
        });
    }
    for v in vias {
        let h = v.size / 2;
        out[v.net as usize].push(Shape {
            layer: LayerId(v.layer as u16),
            rect: Rect {
                x: v.x - h,
                y: v.y - h,
                w: v.size,
                h: v.size,
            },
        });
    }
    out
}

/// Same-layer centre-to-centre clearance between two nets' track trees (docs:
/// `net_pair_clearance`). `f32::MAX` when they share no layer.
#[must_use]
pub fn net_pair_clearance(hot: &RouteHot, grid: &TrackGrid, a: u32, b: u32) -> f32 {
    let na = hot.tree_nodes(a as usize);
    let nb = hot.tree_nodes(b as usize);
    let mut best2 = f32::MAX;
    for &x in &na {
        let (ax, ay, al) = grid.pos(x);
        for &y in &nb {
            let (bx, by, bl) = grid.pos(y);
            if al != bl {
                continue;
            }
            let d2 = ((ax - bx) as f32).powi(2) + ((ay - by) as f32).powi(2);
            best2 = best2.min(d2);
        }
    }
    if best2 == f32::MAX {
        f32::MAX
    } else {
        best2.sqrt()
    }
}

/// EOL scan: different-net endpoints on the same layer within `eol_spacing`
/// (docs: EOL repair). Returns `(nodes_to_block, nets_to_reroute)`.
#[must_use]
pub fn eol_violations(wires: &[Wire], grid: &TrackGrid, eol_spacing: i32) -> (Vec<u32>, Vec<u32>) {
    let mut endpoints: Vec<(i32, i32, u32, u32)> = Vec::new();
    for w in wires {
        endpoints.push((w.x0, w.y0, w.layer, w.net));
        endpoints.push((w.x1, w.y1, w.layer, w.net));
    }
    let mut blocked_nodes = Vec::new();
    let mut affected_nets = Vec::new();
    let eol2 = i64::from(eol_spacing) * i64::from(eol_spacing);
    for i in 0..endpoints.len() {
        let (ax, ay, al, an) = endpoints[i];
        for j in (i + 1)..endpoints.len() {
            let (bx, by, bl, bn) = endpoints[j];
            if al != bl || an == bn {
                continue;
            }
            let dx = i64::from(ax - bx);
            let dy = i64::from(ay - by);
            if dx * dx + dy * dy < eol2 {
                blocked_nodes.push(grid.nearest(bx, by, bl));
                if !affected_nets.contains(&bn) {
                    affected_nets.push(bn);
                }
            }
        }
    }
    blocked_nodes.sort_unstable();
    blocked_nodes.dedup();
    affected_nets.sort_unstable();
    affected_nets.dedup();
    (blocked_nodes, affected_nets)
}

/// Wide-net PRL scan: signal segments parallel to a power wire, overlapping and
/// within `extra_spacing` centreline (docs: Wide-net PRL repair). Returns
/// `(nodes_to_block, signal_nets_to_reroute)`.
#[must_use]
pub fn prl_violations(
    wires: &[Wire],
    grid: &TrackGrid,
    power_nets: &[u32],
    extra_spacing: i32,
) -> (Vec<u32>, Vec<u32>) {
    let mut blocked_nodes = Vec::new();
    let mut affected_nets = Vec::new();
    for pw in wires.iter().filter(|w| power_nets.contains(&w.net)) {
        let (px0, py0, px1, py1) = (
            pw.x0.min(pw.x1),
            pw.y0.min(pw.y1),
            pw.x0.max(pw.x1),
            pw.y0.max(pw.y1),
        );
        let pw_horiz = py0 == py1;
        for sw in wires.iter().filter(|w| !power_nets.contains(&w.net)) {
            if sw.layer != pw.layer {
                continue;
            }
            let (sx0, sy0, sx1, sy1) = (
                sw.x0.min(sw.x1),
                sw.y0.min(sw.y1),
                sw.x0.max(sw.x1),
                sw.y0.max(sw.y1),
            );
            let sw_horiz = sy0 == sy1;
            if pw_horiz != sw_horiz {
                continue;
            }
            if pw_horiz {
                let ydist = (py0 - sy0).abs();
                let overlap = sx1.min(px1) - sx0.max(px0);
                if overlap > 0 && ydist < extra_spacing && ydist > 0 {
                    blocked_nodes.push(grid.nearest(sx0, sy0, sw.layer));
                    if !affected_nets.contains(&sw.net) {
                        affected_nets.push(sw.net);
                    }
                }
            } else {
                let xdist = (px0 - sx0).abs();
                let overlap = sy1.min(py1) - sy0.max(py0);
                if overlap > 0 && xdist < extra_spacing && xdist > 0 {
                    blocked_nodes.push(grid.nearest(sx0, sy0, sw.layer));
                    if !affected_nets.contains(&sw.net) {
                        affected_nets.push(sw.net);
                    }
                }
            }
        }
    }
    blocked_nodes.sort_unstable();
    blocked_nodes.dedup();
    affected_nets.sort_unstable();
    affected_nets.dedup();
    (blocked_nodes, affected_nets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_core::geom::Pin;
    use pnr_core::ids::NetId;

    fn m(pins: Vec<(u16, i32, i32)>) -> Macro {
        Macro {
            shapes: vec![Shape {
                layer: LayerId(0),
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 20_000,
                    h: 20_000,
                },
            }],
            pins: pins
                .into_iter()
                .map(|(net, x, y)| Pin {
                    name: format!("p{net}"),
                    net: NetId(net),
                    at: Rect {
                        x,
                        y,
                        w: 200,
                        h: 200,
                    },
                })
                .collect(),
            bbox: Rect {
                x: 0,
                y: 0,
                w: 20_000,
                h: 20_000,
            },
        }
    }

    /// An empty `Layout` — every macro is then past `l.x.len()` and keeps its own
    /// (already absolute) coordinates, which is what these tests want.
    fn lay() -> Layout {
        Layout {
            x: vec![],
            y: vec![],
            hw: vec![],
            hh: vec![],
            variant: vec![],
            branch: vec![],
            axis: vec![],
            groups: vec![],
            orient: vec![],
            power_uw: vec![],
            temp_mc: vec![],
        }
    }

    #[test]
    fn two_pin_net_routes_coarsely() {
        // One net with two far-apart pins → a nonempty coarse route, no hard viol.
        let macros = vec![m(vec![(0, 1_000, 1_000)]), m(vec![(0, 18_000, 18_000)])];
        let reqs = Requirements::<Routes>::default();
        let layers = [LayerId(0), LayerId(1)];
        let (routes, report) = GlobalRoute::default().route(
            &lay(),
            &macros,
            &[],
            &reqs,
            &layers,
            &mut Negotiation::new(),
            7,
        );
        assert_eq!(routes.wires.len(), 1);
        assert!(
            !routes.wires[0].is_empty(),
            "coarse route should have segments"
        );
        assert!(
            report.hard_violations.is_empty(),
            "no overflow expected: {:?}",
            report
                .hard_violations
                .iter()
                .map(|v| &v.rule)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn deterministic_for_seed() {
        let macros = vec![m(vec![(0, 1_000, 1_000)]), m(vec![(0, 18_000, 18_000)])];
        let reqs = Requirements::<Routes>::default();
        let layers = [LayerId(0), LayerId(1)];
        let (r1, _) = GlobalRoute::default().route(
            &lay(),
            &macros,
            &[],
            &reqs,
            &layers,
            &mut Negotiation::new(),
            1,
        );
        let (r2, _) = GlobalRoute::default().route(
            &lay(),
            &macros,
            &[],
            &reqs,
            &layers,
            &mut Negotiation::new(),
            999,
        );
        assert_eq!(
            r1.wires[0].len(),
            r2.wires[0].len(),
            "seed must not change routing"
        );
    }

    /// `Negotiation` must survive `route` returning, and must change what the next
    /// call does. If either half breaks, the negotiation is back to throwing `h_n`
    /// away every epoch (D3) and nothing else in the flow can tell.
    #[test]
    fn negotiation_persists_across_calls() {
        // Congestion has to be *unavoidable* or there is nothing to negotiate:
        // 4×4 gcells at capacity 1, with six nets that each have to cross the grid.
        let cfg = GlobalCfg {
            gcells_per_side: 4,
            gcell_capacity: 1,
            ..GlobalCfg::default()
        };
        let router = GlobalRoute { cfg };
        let macros: Vec<Macro> = (0..6u16)
            .flat_map(|n| {
                let off = i32::from(n) * 1_000;
                [
                    m(vec![(n, 1_000 + off, 1_000)]),
                    m(vec![(n, 18_000 - off, 18_000)]),
                ]
            })
            .collect();
        let reqs = Requirements::<Routes>::default();
        let layers = [LayerId(0), LayerId(1)];
        let run = |neg: &mut Negotiation| {
            router
                .route(&lay(), &macros, &[], &reqs, &layers, neg, 3)
                .0
        };

        let mut neg = Negotiation::new();
        let first = run(&mut neg);
        let p1 = neg.pressure();
        assert!(p1 > 0.0, "contested grid must accumulate some history");
        let second = run(&mut neg);
        let p2 = neg.pressure();
        assert!(
            p2 > p1,
            "history must keep climbing while resources stay contested: {p1} -> {p2}"
        );

        // Determinism is over `(inputs, seed, neg)`: the same inputs with more
        // incoming history route differently, which is the point of carrying it.
        let flat = |r: &Routes| -> Vec<(u16, i32, i32, i32, i32)> {
            r.wires
                .iter()
                .flatten()
                .map(|s| (s.layer.0, s.rect.x, s.rect.y, s.rect.w, s.rect.h))
                .collect()
        };
        assert_ne!(
            flat(&first),
            flat(&second),
            "second call saw history and still produced the identical route — it was \
             not seeded"
        );
    }

    /// One net of drawn length `len` — `Routes::length` sums `max(w, h)` per shape.
    fn wire(len: i32) -> Routes {
        Routes {
            wires: vec![vec![Shape {
                layer: LayerId(0),
                rect: Rect { x: 0, y: 0, w: len, h: 1 },
            }]],
        }
    }

    /// A parasitic budget on net 0 with a lowered length cap of `cap` nm. Real rule,
    /// not a stub: the whole point of J1 is that a `kernel/analog` residual override
    /// travels across the crate boundary into `Report`.
    fn budget(cap: i64) -> Box<dyn analog::RuleBatch<Routes>> {
        Box::new(vec![analog::routing::ParasiticBudget {
            net: NetId(0),
            max_r_mohm: 1_000_000,
            max_c_af: 100_000_000,
            max_len_nm: cap,
            margin_pct: 10,
        }])
    }

    /// **J1**: a `reqs.budget` residual must reach `Report::budget_violations` with a
    /// nonzero margin, and must move `Report::lex`'s Θ element. Without this the wiring
    /// is unobservable — `gr` computed correct residuals that reached nobody, and every
    /// per-crate test still passed.
    #[test]
    fn budget_residual_reaches_theta() {
        let mut reqs = Requirements::<Routes>::default();
        reqs.budget.push(budget(1_000));

        // Inside the cap: no Θ entry at all, and the tier reads feasible.
        let clean = score(&wire(900), &reqs, 0.0, 0.0);
        assert!(clean.budget_violations.is_empty(), "900 nm is inside a 1000 nm cap");
        assert_eq!(clean.lex().1, 0.0);

        // 1500 / 1000 - 1 = 0.5 over ⇒ 500 milli-budgets (the `gp::mechanics::report`
        // scale). Nonzero is the load-bearing part: a margin of 0 reads as satisfied.
        let over = score(&wire(1_500), &reqs, 0.0, 0.0);
        assert_eq!(over.budget_violations.len(), 1);
        assert_eq!(over.budget_violations[0].margin, 500);
        assert!(
            over.lex().1 > clean.lex().1,
            "Θ must move when a budget is exceeded: {} vs {}",
            over.lex().1,
            clean.lex().1
        );
        assert!(!over.feasible(), "a positive budget residual is not feasible");
    }

    /// **J2**: a hard batch reports a *measured* margin, not its violation count. Two
    /// batches with the same violation count (1) and different overshoot must produce
    /// different margins — D10, since both Φ and Θ sum margins.
    #[test]
    fn hard_batch_margin_is_measured_not_counted() {
        let routes = wire(3_000);
        let margin_of = |cap: i64| {
            let mut reqs = Requirements::<Routes>::default();
            reqs.hard.push(budget(cap));
            let r = score(&routes, &reqs, 0.0, 0.0);
            assert_eq!(r.hard_violations.len(), 1, "exactly one violating batch");
            r.hard_violations[0].margin
        };
        // Same count, 50% over vs 200% over. Dyadic ratios on purpose: `residual` is
        // f32 and the scale `ceil`s, so 0.2 arrives as 0.20000000298 and lands on 201.
        let slight = margin_of(2_000); // 3000/2000 - 1 = 0.5 ⇒ 500
        let gross = margin_of(1_000); // 3000/1000 - 1 = 2.0 ⇒ 2000
        assert_eq!((slight, gross), (500, 2_000));
        assert!(gross > slight, "a count would make these identical");
    }

    /// Overflow's Θ entry is a residual over total grid capacity, in milli-budgets —
    /// not the raw node-use count it used to be (D17: 1 node of overflow used to
    /// weigh the same as a budget missed by 0.1%). `Report::cost` keeps the raw
    /// count, because rescaling it would silently re-weight the PEX tier.
    #[test]
    fn overflow_theta_is_a_capacity_residual_not_a_count() {
        let reqs = Requirements::<Routes>::default();
        // 3 nodes over on a grid with 600 total capacity ⇒ 0.005 ⇒ 5 milli-budgets.
        let r = score(&wire(100), &reqs, 3.0, 600.0);
        assert_eq!(r.budget_violations.len(), 1);
        assert_eq!(r.budget_violations[0].margin, 5);
        assert_eq!(r.cost, 3.0, "PEX keeps the raw overflow");
        // Tiny but real overflow must not round to a satisfied 0 (ceil).
        let tiny = score(&wire(100), &reqs, 1.0, 1_000_000.0);
        assert_eq!(tiny.budget_violations[0].margin, 1);
    }

    #[test]
    fn price_group_measures_the_group_not_the_die() {
        // A group parked 100 µm from the origin: the grid must follow it, or every
        // pin lands in one gcell and the price is meaningless.
        let far = |x: i32, y: i32, net: u16| {
            let mut mac = m(vec![(net, x, y)]);
            mac.bbox = Rect { x, y, w: 2_000, h: 2_000 };
            mac.shapes[0].rect = mac.bbox;
            mac
        };
        let group = vec![far(100_000, 100_000, 0), far(108_000, 106_000, 0)];
        let price = price_group(&group, &[LayerId(0)], &GlobalCfg::default());
        assert!(price.reachable, "two pins in open space must reach");
        assert_eq!(price.overflow, 0, "one net cannot overflow capacity 6");
        // HPWL is the pin-centre bbox half-perimeter: 8000 + 6000.
        assert_eq!(price.hpwl, 14_000);
    }
}

/// Translate every macro into its **placed** position, matching the convention
/// `frontend/library::geometry::collect` uses for emitted geometry.
///
/// Turning about the bbox lower-left corner (rather than the centre) keeps the
/// whole macro on the fabrication grid: every D4 orientation maps grid multiples
/// to grid multiples, whereas rotating about a centre lands half-pitch off
/// whenever an extent is an odd number of grid steps.
///
/// Devices past the end of the layout keep their local coordinates, which is the
/// same fallback `collect` applies (guard rings arrive as absolute-coordinate
/// macros appended after the device list).
#[must_use]
pub fn place_macros(macros: &[Macro], l: &Layout) -> Vec<Macro> {
    macros
        .iter()
        .enumerate()
        .map(|(i, m)| {
            if i >= l.x.len() {
                return m.clone(); // already absolute
            }
            let (cx, cy) = (l.x[i], l.y[i]);
            let o = l.orient.get(i).copied().unwrap_or_default();
            let anchor = o.apply_rect(m.bbox);
            // Centre the turned bbox on the placed centre. `Layout` is a
            // centre + half-extents model — `gp::mechanics::half_extents` builds
            // `hw`/`hh` from `bbox.w/2` and discards `bbox.x`/`bbox.y` — so
            // anchoring the macro's *local origin* here instead left every drawn
            // device offset from where the placer legalised it by
            // `(hw + bbox.x, hh + bbox.y)`, measured up to 20 µm. Guard rings are
            // appended past `l.x.len()` and are already absolute, so they stayed
            // put while the devices moved, and their tap bands cut through the
            // diffusion they were meant to surround.
            let (hw, hh) = (l.hw[i], l.hh[i]);
            let (ax, ay) = (cx - hw - anchor.x, cy - hh - anchor.y);
            let shift = |r: Rect| {
                let r = if o == pnr_core::Orient::R0 { r } else { o.apply_rect(r) };
                Rect { x: r.x + ax, y: r.y + ay, w: r.w, h: r.h }
            };
            Macro {
                bbox: shift(m.bbox),
                shapes: m
                    .shapes
                    .iter()
                    .map(|s| Shape { layer: s.layer, rect: shift(s.rect) })
                    .collect(),
                pins: m
                    .pins
                    .iter()
                    .map(|p| Pin { at: shift(p.at), ..p.clone() })
                    .collect(),
            }
        })
        .collect()
}
