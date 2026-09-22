//! # `gr` — global routing, plus the routing mechanics `dr` shares.
//!
//! [`GlobalRoute`] builds nets from the placed macro pins and runs negotiated-
//! congestion PathFinder on an `N×N` gcell grid. Its output is a coarse [`Routes`]
//! (gcell centre-line segments) that `dr` uses as die extent and corridor seeds.
//!
//! The grids ([`GcellGrid`], [`TrackGrid`]), the search ([`route_net`],
//! [`run_pathfinder`]) and geometry extraction ([`extract_geometry`]) live here so
//! both routers run one PathFinder core.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

use analog::Requirements;
use pnr_core::geom::{LayerId, Rect, Shape};
use pnr_core::report::Violation;
use pnr_core::{Layout, Macro, Report, Routes};

pub use pnr_core::place_macros;

/// Coarse-grid tuning.
#[derive(Debug, Clone, Copy)]
pub struct GlobalCfg {
    pub gcells_per_side: u32,
    pub gcell_capacity: u16,
    pub max_iters: u32,
    pub p_fac: f32,
    pub hist_inc: f32,
}

impl Default for GlobalCfg {
    fn default() -> Self {
        Self { gcells_per_side: 16, gcell_capacity: 6, max_iters: 40, p_fac: 3.0, hist_inc: 0.5 }
    }
}

/// PathFinder history `h_n`, owned by the orchestrator and carried across outer
/// epochs so the negotiation keeps its prices (the anti-oscillation guarantee).
///
/// Keyed by absolute 500 nm buckets, not grid index: grids are rebuilt from a
/// bounding box the placer moves every epoch. `BTreeMap` + `max` folding keep
/// [`Negotiation::pressure`] deterministic.
#[derive(Default)]
pub struct Negotiation {
    hist: BTreeMap<(Tier, u32, i32, i32), f32>,
}

/// Which router's resources a history entry belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Global,
    Detailed,
}

const HIST_QUANTUM_NM: i32 = 500;

fn hist_key(tier: Tier, (x, y, layer): (i32, i32, u32)) -> (Tier, u32, i32, i32) {
    (tier, layer, x.div_euclid(HIST_QUANTUM_NM), y.div_euclid(HIST_QUANTUM_NM))
}

impl Negotiation {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Σ `h_n` — still climbing means resources are still contested.
    #[must_use]
    pub fn pressure(&self) -> f64 {
        self.hist.values().map(|&h| f64::from(h)).sum()
    }

    /// Seed per-node history; `pos` must return **absolute** `(x, y, layer)`.
    pub fn seed(&self, tier: Tier, hist: &mut [f32], pos: impl Fn(u32) -> (i32, i32, u32)) {
        if self.hist.is_empty() {
            return;
        }
        for (n, h) in hist.iter_mut().enumerate() {
            *h = self.hist.get(&hist_key(tier, pos(n as u32))).copied().unwrap_or(0.0);
        }
    }

    /// Fold per-node history back in (`max` per bucket: resolution- and order-free).
    pub fn accumulate(&mut self, tier: Tier, hist: &[f32], pos: impl Fn(u32) -> (i32, i32, u32)) {
        for (n, &h) in hist.iter().enumerate() {
            if h > 0.0 {
                let e = self.hist.entry(hist_key(tier, pos(n as u32))).or_insert(0.0);
                *e = e.max(h);
            }
        }
    }
}

/// Routability of one group's variant, measured by routing its own nets.
pub struct GroupPrice {
    /// Pin-bbox half-perimeter wirelength, nm.
    pub hpwl: i64,
    /// Residual gcell overflow after negotiation.
    pub overflow: i64,
    /// Every terminal reached by its net's tree.
    pub reachable: bool,
}

/// Price a group's variant by routing its nets in isolation on a grid over the
/// group's own bbox. `macros` must already be placed relative to one another.
/// `layers` is unused (the coarse plan is 2-D).
#[must_use]
pub fn price_group(macros: &[Macro], _layers: &[LayerId], cfg: &GlobalCfg) -> GroupPrice {
    let (net_ids, pins) = build_nets(macros);
    if pins.is_empty() {
        return GroupPrice { hpwl: 0, overflow: 0, reachable: true };
    }
    let (lo_x, lo_y, hi_x, hi_y) = group_bbox(macros);
    let hpwl = pins
        .iter()
        .map(|pts| {
            let (x0, x1) = pts.iter().fold((i32::MAX, i32::MIN), |a, p| (a.0.min(p.0), a.1.max(p.0)));
            let (y0, y1) = pts.iter().fold((i32::MAX, i32::MIN), |a, p| (a.0.min(p.1), a.1.max(p.1)));
            i64::from(x1 - x0) + i64::from(y1 - y0)
        })
        .sum();
    let die = ((hi_x - lo_x).max(2), (hi_y - lo_y).max(2));
    let grid = GcellGrid::new(die, cfg.gcells_per_side, cfg.gcell_capacity);
    let terms = gcell_terms(&grid, &pins, (lo_x, lo_y));
    let cold = RouteCtx::new(grid, terms, order_by_pin_count(&pins));
    let mut hot = RouteHot::new(cold.graph.nodes(), net_ids.len());
    let overflow = run_pathfinder(&mut hot, &cold, cfg.p_fac, cfg.hist_inc, cfg.max_iters);
    let reachable = (0..net_ids.len()).all(|ci| {
        let tree = hot.tree_nodes(ci);
        cold.terms[ci].iter().all(|t| tree.binary_search(t).is_ok())
    });
    GroupPrice { hpwl, overflow: overflow.ceil() as i64, reachable }
}

/// `(lo_x, lo_y, hi_x, hi_y)` over macro bboxes, shapes and pins.
fn group_bbox(macros: &[Macro]) -> (i32, i32, i32, i32) {
    let rects = macros.iter().flat_map(|m| {
        std::iter::once(m.bbox).chain(m.shapes.iter().map(|s| s.rect)).chain(m.pins.iter().map(|p| p.at))
    });
    let (mut lo, mut hi) = ((i32::MAX, i32::MAX), (i32::MIN, i32::MIN));
    for r in rects {
        lo = (lo.0.min(r.x), lo.1.min(r.y));
        hi = (hi.0.max(r.x + r.w), hi.1.max(r.y + r.h));
    }
    if lo.0 == i32::MAX {
        (0, 0, 2, 2)
    } else {
        (lo.0, lo.1, hi.0, hi.1)
    }
}

/// A global-routing algorithm. `rings` are this epoch's guard rings (absolute;
/// both obstacle and net target). Deterministic over `(inputs, neg)`; `seed` is
/// unused by PathFinder.
pub trait GlobalRouter {
    #[allow(clippy::too_many_arguments)]
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

/// The coarse router.
#[derive(Default)]
pub struct GlobalRoute {
    pub cfg: GlobalCfg,
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
        let cfg = self.cfg;
        let mut placed = place_macros(macros, placement);
        placed.extend(rings.iter().cloned()); // rings are already absolute
        let base_layer = layers.first().copied().unwrap_or(LayerId(0));
        let n_nets = placed.iter().flat_map(|m| &m.pins).map(|p| p.net.0 as usize + 1).max().unwrap_or(0);
        let (net_ids, pins) = build_nets(&placed);
        if pins.is_empty() {
            let routes = Routes { wires: vec![Vec::new(); n_nets] };
            let report = score(&routes, reqs, 0.0, 0.0);
            return (routes, report);
        }

        let grid = GcellGrid::new(die_extent(&placed), cfg.gcells_per_side, cfg.gcell_capacity);
        let terms = gcell_terms(&grid, &pins, (0, 0));
        let cold = RouteCtx::new(grid, terms, order_by_pin_count(&pins));
        let mut hot = RouteHot::new(cold.graph.nodes(), net_ids.len());
        neg.seed(Tier::Global, &mut hot.hist, |n| cold.graph.pos(n));
        // Ring metal consumes wiring resource, capped below capacity so a ring can
        // never manufacture overflow on its own (its net must still reach it).
        for s in rings.iter().flat_map(|m| &m.shapes).filter(|s| layers.contains(&s.layer)) {
            charge_rect(&mut hot.usage, &cold.graph, s.rect);
        }
        let overflow = run_pathfinder(&mut hot, &cold, cfg.p_fac, cfg.hist_inc, cfg.max_iters);
        neg.accumulate(Tier::Global, &hot.hist, |n| cold.graph.pos(n));

        let mut wires = vec![Vec::new(); n_nets];
        for (ci, tree) in hot.trees.iter().enumerate() {
            let dst = &mut wires[net_ids[ci] as usize];
            for branch in tree {
                for w in branch.windows(2) {
                    dst.push(seg_shape(cold.graph.pos(w[0]), cold.graph.pos(w[1]), base_layer));
                }
                // A lone terminal still marks its gcell so `dr` seeds a corridor there.
                if let [n] = branch[..] {
                    dst.push(seg_shape(cold.graph.pos(n), cold.graph.pos(n), base_layer));
                }
            }
        }
        let routes = Routes { wires };
        let cap_total = f32::from(cold.graph.cap()) * cold.graph.nodes() as f32;
        let report = score(&routes, reqs, overflow, cap_total);
        (routes, report)
    }
}

/// Deduped gcell terminals per net, pins shifted by `-origin`.
fn gcell_terms(grid: &GcellGrid, pins: &[Vec<(i32, i32)>], origin: (i32, i32)) -> Vec<Vec<u32>> {
    pins.iter()
        .map(|pts| {
            let mut t: Vec<u32> =
                pts.iter().map(|&(x, y)| grid.at((x - origin.0) as f32, (y - origin.1) as f32)).collect();
            t.sort_unstable();
            t.dedup();
            t
        })
        .collect()
}

/// Fewer terminals first; ties by index.
fn order_by_pin_count(pins: &[Vec<(i32, i32)>]) -> Vec<u32> {
    let mut order: Vec<u32> = (0..pins.len() as u32).collect();
    order.sort_by_key(|&a| (pins[a as usize].len(), a));
    order
}

/// Add one unit of usage to every gcell `r` overlaps, never reaching capacity.
fn charge_rect(usage: &mut [u16], g: &GcellGrid, r: Rect) {
    let ceiling = g.capacity.saturating_sub(1);
    let lo = g.at(r.x as f32, r.y as f32);
    let hi = g.at((r.x + r.w) as f32, (r.y + r.h) as f32);
    for gy in (lo / g.nx)..=(hi / g.nx) {
        for gx in (lo % g.nx)..=(hi % g.nx) {
            let n = (gy * g.nx + gx) as usize;
            usage[n] = (usage[n] + 1).min(ceiling);
        }
    }
}

/// Die `(w, h)` from the origin to the far corner of every bbox and pin.
fn die_extent(macros: &[Macro]) -> (i32, i32) {
    let (mut w, mut h) = (2, 2);
    for r in macros.iter().flat_map(|m| std::iter::once(m.bbox).chain(m.pins.iter().map(|p| p.at))) {
        w = w.max(r.x + r.w);
        h = h.max(r.y + r.h);
    }
    (w, h)
}

/// Degenerate-thin centre-line segment between two points (coarse geometry).
fn seg_shape((ax, ay, _): (i32, i32, u32), (bx, by, _): (i32, i32, u32), layer: LayerId) -> Shape {
    let (x, y) = (ax.min(bx), ay.min(by));
    Shape { layer, rect: Rect { x, y, w: (ax.max(bx) - x).max(1), h: (ay.max(by) - y).max(1) } }
}

/// `(V, Θ)` entries from `reqs.hard` / `reqs.budget`, one per violating batch, as
/// measured residuals. Shared with `dr` so both stages report on one scale.
#[must_use]
pub fn analog_tiers(routes: &Routes, reqs: &Requirements<Routes>) -> (Vec<Violation>, Vec<Violation>) {
    let hard = reqs
        .hard
        .iter()
        .enumerate()
        .filter(|(_, b)| b.violations(routes) > 0)
        .map(|(i, b)| Violation::from_residual(format!("routing hard batch {i}"), b.residual(routes)))
        .collect();
    let budget = reqs
        .budget
        .iter()
        .enumerate()
        .filter_map(|(i, b)| {
            let r = b.residual(routes);
            (r > 0.0).then(|| Violation::from_residual(format!("routing budget batch {i}"), r))
        })
        .collect();
    (hard, budget)
}

/// Report: analog tiers, plus residual `overflow / cap_total` as a Θ entry; cost
/// keeps the raw overflow plus the criticality-weighted analog cost.
pub(crate) fn score(routes: &Routes, reqs: &Requirements<Routes>, overflow: f32, cap_total: f32) -> Report {
    let (hard_violations, mut budget_violations) = analog_tiers(routes, reqs);
    if overflow > 0.0 && cap_total > 0.0 {
        budget_violations.push(Violation::from_residual("routing overflow", f64::from(overflow / cap_total)));
    }
    let analog_cost: f32 = reqs.cost.iter().map(|b| b.criticality(routes) * b.cost(routes)).sum();
    Report { hard_violations, budget_violations, cost: overflow + analog_cost }
}

/// Nets with ≥2 distinct pin centres: `(net id, sorted deduped points)`. A net
/// whose pins collapse to one point is not routed.
fn build_nets(macros: &[Macro]) -> (Vec<u32>, Vec<Vec<(i32, i32)>>) {
    let mut pts: BTreeMap<u16, Vec<(i32, i32)>> = BTreeMap::new();
    for p in macros.iter().flat_map(|m| &m.pins) {
        pts.entry(p.net.0).or_default().push((p.at.x + p.at.w / 2, p.at.y + p.at.h / 2));
    }
    let (mut ids, mut out) = (Vec::new(), Vec::new());
    for (net, mut v) in pts {
        v.sort_unstable();
        v.dedup();
        if v.len() >= 2 {
            ids.push(u32::from(net));
            out.push(v);
        }
    }
    (ids, out)
}

// ---------------------------------------------------------------------------
// Routing resource graphs
// ---------------------------------------------------------------------------

/// "No node" / "unreserved".
pub const NONE: u32 = u32::MAX;

/// A routing graph with uniform per-node capacity.
pub trait RGraph {
    fn nodes(&self) -> usize;
    fn cap(&self) -> u16;
    /// Writes up to six `(neighbour, base_cost)` edges; returns the count.
    fn neighbors(&self, n: u32, out: &mut [(u32, f32); 6]) -> usize;
    /// `(x, y, layer)` centre of `n`, nm.
    fn pos(&self, n: u32) -> (i32, i32, u32);
    /// Corridor-region (gcell) id of `n`.
    fn region(&self, n: u32) -> u32;
}

/// The coarse `N × N` gcell grid over the die, four planar unit-cost neighbours.
pub struct GcellGrid {
    pub nx: u32,
    pub ny: u32,
    pub gw: f32,
    pub gh: f32,
    pub capacity: u16,
}

impl GcellGrid {
    #[must_use]
    pub fn new(die: (i32, i32), n_per_side: u32, capacity: u16) -> Self {
        let n = n_per_side.max(2);
        Self { nx: n, ny: n, gw: die.0 as f32 / n as f32, gh: die.1 as f32 / n as f32, capacity }
    }

    /// Gcell containing `(x, y)`, clamped to the grid.
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
        let edges = [
            (x > 0, n.wrapping_sub(1)),
            (x + 1 < self.nx, n + 1),
            (y > 0, n.wrapping_sub(self.nx)),
            (y + 1 < self.ny, n + self.nx),
        ];
        let mut k = 0;
        for (ok, m) in edges {
            if ok {
                out[k] = (m, 1.0);
                k += 1;
            }
        }
        k
    }
    fn pos(&self, n: u32) -> (i32, i32, u32) {
        let (x, y) = (n % self.nx, n / self.nx);
        (((x as f32 + 0.5) * self.gw) as i32, ((y as f32 + 0.5) * self.gh) as i32, 0)
    }
    fn region(&self, n: u32) -> u32 {
        n
    }
}

/// The fine track lattice: capacity 1, even layers horizontal, odd vertical,
/// adjacent layers joined by `via_cost` at the same track coordinate.
pub struct TrackGrid {
    pub nx: u32,
    pub ny: u32,
    pub pitch: i32,
    pub via_cost: f32,
    pub n_layers: u32,
    region_of: Vec<u32>,
}

impl TrackGrid {
    /// Track grid over `die`; `pitch` is raised to at least `max(die)/1200`.
    #[must_use]
    pub fn with_layers(die: (i32, i32), pitch: i32, via_cost: f32, n_layers: u32) -> Self {
        let pitch = pitch.max(die.0.max(die.1) / 1200).max(1);
        let nx = (die.0 / pitch).max(2) as u32;
        let ny = (die.1 / pitch).max(2) as u32;
        Self { nx, ny, pitch, via_cost, n_layers: n_layers.max(1), region_of: vec![0; (nx * ny) as usize] }
    }

    /// Precompute each track's gcell region (corridor restriction).
    pub fn set_regions(&mut self, gcells: &GcellGrid) {
        for i in 0..self.layer_size() {
            let (x, y, _) = self.pos(i);
            self.region_of[i as usize] = gcells.at(x as f32, y as f32);
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

    /// Column of the pitch bin containing `x`, clamped.
    #[must_use]
    pub fn bin_x(&self, x: i32) -> u32 {
        (x / self.pitch).clamp(0, self.nx as i32 - 1) as u32
    }

    /// Row of the pitch bin containing `y`, clamped.
    #[must_use]
    pub fn bin_y(&self, y: i32) -> u32 {
        (y / self.pitch).clamp(0, self.ny as i32 - 1) as u32
    }

    #[must_use]
    pub fn ixy(&self, n: u32) -> (u32, u32, u32) {
        let r = n % self.layer_size();
        (r % self.nx, r / self.nx, n / self.layer_size())
    }

    /// Up to `k` unclaimed landing nodes passing `ok` for a pin at `(x, y)`: Chebyshev rings
    /// `0..=rmax`, layer 0 before layer 1, nearest ring first. A layer-1 node within
    /// two rows of a claimed layer-1 node in its column is skipped (stacked vias).
    #[must_use]
    pub fn candidates(
        &self,
        x: i32,
        y: i32,
        claimed: &[bool],
        ok: impl Fn(u32) -> bool,
        rmax: i32,
        k: usize,
    ) -> Vec<u32> {
        let (cx, cy) = (self.bin_x(x) as i32, self.bin_y(y) as i32);
        let (nx, ny) = (self.nx as i32, self.ny as i32);
        let mut out = Vec::new();
        for layer in 0..self.n_layers.min(2) {
            for r in 0..=rmax {
                for dy in -r..=r {
                    for dx in -r..=r {
                        let (ix, iy) = (cx + dx, cy + dy);
                        if dx.abs().max(dy.abs()) != r || ix < 0 || iy < 0 || ix >= nx || iy >= ny {
                            continue;
                        }
                        let n = self.node(ix as u32, iy as u32, layer);
                        let stacked = layer == 1
                            && (-2..=2).any(|d: i32| {
                                let jy = iy + d;
                                d != 0 && (0..ny).contains(&jy) && claimed[self.node(ix as u32, jy as u32, 1) as usize]
                            });
                        if !claimed[n as usize] && !stacked && ok(n) {
                            out.push(n);
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
        let mut push = |ok: bool, m: u32, cost: f32| {
            if ok {
                out[k] = (m, cost);
                k += 1;
            }
        };
        if layer % 2 == 0 {
            push(ix > 0, n.wrapping_sub(1), 1.0);
            push(ix + 1 < self.nx, n + 1, 1.0);
        } else {
            push(iy > 0, n.wrapping_sub(self.nx), 1.0);
            push(iy + 1 < self.ny, n + self.nx, 1.0);
        }
        push(layer > 0, n.wrapping_sub(self.layer_size()), self.via_cost);
        push(layer + 1 < self.n_layers, n + self.layer_size(), self.via_cost);
        k
    }
    fn pos(&self, n: u32) -> (i32, i32, u32) {
        let (ix, iy, layer) = self.ixy(n);
        (ix as i32 * self.pitch + self.pitch / 2, iy as i32 * self.pitch + self.pitch / 2, layer)
    }
    fn region(&self, n: u32) -> u32 {
        self.region_of[(n % self.layer_size()) as usize]
    }
}

// ---------------------------------------------------------------------------
// Negotiated-congestion PathFinder
// ---------------------------------------------------------------------------

/// Mutable routing state: per-node usage and history, per-net branch trees.
pub struct RouteHot {
    pub usage: Vec<u16>,
    pub hist: Vec<f32>,
    /// `trees[net]` = branches, each a node path.
    pub trees: Vec<Vec<Vec<u32>>>,
}

impl RouteHot {
    #[must_use]
    pub fn new(nodes: usize, nets: usize) -> Self {
        Self { usage: vec![0; nodes], hist: vec![0.0; nodes], trees: vec![Vec::new(); nets] }
    }

    /// Sorted, deduped node ids of `net`'s tree.
    #[must_use]
    pub fn tree_nodes(&self, net: usize) -> Vec<u32> {
        let mut v: Vec<u32> = self.trees[net].iter().flatten().copied().collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    /// Replace `net`'s tree, moving its usage claim with it.
    pub fn commit(&mut self, net: usize, branches: Vec<Vec<u32>>) {
        for n in self.tree_nodes(net) {
            self.usage[n as usize] -= 1;
        }
        self.trees[net] = branches;
        for n in self.tree_nodes(net) {
            self.usage[n as usize] += 1;
        }
    }
}

/// Immutable per-run context. `corridors[net]` (sorted region ids) restricts the
/// first [`CORRIDOR_EPOCHS`]; `reserved[node]` is a hard owner (`NONE` = free);
/// `penalty[net][node]` is an extra cost for that net (missing = none).
pub struct RouteCtx<G> {
    pub graph: G,
    pub terms: Vec<Vec<u32>>,
    pub order: Vec<u32>,
    pub corridors: Vec<Vec<u32>>,
    pub reserved: Vec<u32>,
    pub penalty: Vec<Vec<f32>>,
}

impl<G: RGraph> RouteCtx<G> {
    /// No corridors, reservations or penalties.
    pub fn new(graph: G, terms: Vec<Vec<u32>>, order: Vec<u32>) -> Self {
        Self { graph, terms, order, corridors: Vec::new(), reserved: Vec::new(), penalty: Vec::new() }
    }

    /// Route `net` against the current state: inside its corridor when `corridor`,
    /// falling back to unrestricted.
    pub fn reroute(&self, hot: &RouteHot, net: usize, corridor: bool, p_fac: f32, dij: &mut Dij) -> Option<Vec<Vec<u32>>> {
        let old = hot.tree_nodes(net);
        let corr = if corridor { self.corridors.get(net).map_or(&[][..], Vec::as_slice) } else { &[] };
        let pen = self.penalty.get(net).map_or(&[][..], Vec::as_slice);
        let search = |c: &[u32], dij: &mut Dij| {
            route_net(&self.graph, &hot.usage, &hot.hist, &old, &self.terms[net], c, net as u32, &self.reserved, pen, p_fac, dij)
        };
        search(corr, dij).or_else(|| if corr.is_empty() { None } else { search(&[], dij) })
    }
}

/// Stamp-based Dijkstra scratch, reused across searches without clearing.
pub struct Dij {
    dist: Vec<f32>,
    prev: Vec<u32>,
    seen: Vec<u32>,
    stamp: u32,
    heap: BinaryHeap<Reverse<(u32, u32)>>,
    in_tree: Vec<u32>,
    is_old: Vec<u32>,
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
            is_old: vec![0; nodes],
        }
    }
}

/// Connect `terms` into a branch tree by repeated multi-source Dijkstra from the
/// growing tree, targets nearest-first. Node cost is `hist + p_fac·overuse +
/// penalty`, where the net's own `old_nodes` do not count as usage. `None` if a
/// target is unreachable inside `corridor` (empty = unrestricted) without
/// entering a node `reserved` for another net.
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
    penalty: &[f32],
    p_fac: f32,
    dij: &mut Dij,
) -> Option<Vec<Vec<u32>>> {
    let Some(&root) = terms.first() else { return Some(Vec::new()) };
    let Dij { dist, prev, seen, stamp, heap, in_tree, is_old } = dij;
    // `call` marks old/tree membership for this call; each target search bumps
    // `stamp` again for `seen`. Stamps only grow, so stale marks never match.
    *stamp = stamp.wrapping_add(1);
    let call = *stamp;
    for &n in old_nodes {
        is_old[n as usize] = call;
    }
    in_tree[root as usize] = call;
    let cap = g.cap();
    let node_cost = |i: usize| -> f32 {
        let eff = usage[i].saturating_sub(u16::from(is_old[i] == call));
        hist[i] + p_fac * f32::from((eff + 1).saturating_sub(cap)) + penalty.get(i).copied().unwrap_or(0.0)
    };

    let (x0, y0, _) = g.pos(root);
    let mut targets = terms[1..].to_vec();
    targets.sort_by_key(|&t| {
        let (x, y, _) = g.pos(t);
        (x - x0).abs() + (y - y0).abs()
    });
    let mut tree = vec![root];
    let mut branches = vec![vec![root]];
    let mut buf = [(0u32, 0.0f32); 6];
    for target in targets {
        if in_tree[target as usize] == call {
            continue;
        }
        *stamp = stamp.wrapping_add(1);
        let s = *stamp;
        heap.clear();
        for &n in &tree {
            seen[n as usize] = s;
            dist[n as usize] = 0.0;
            prev[n as usize] = NONE;
            heap.push(Reverse((0, n)));
        }
        let mut found = false;
        while let Some(Reverse((db, n))) = heap.pop() {
            let d = f32::from_bits(db);
            if d > dist[n as usize] {
                continue;
            }
            if n == target {
                found = true;
                break;
            }
            let k = g.neighbors(n, &mut buf);
            for &(nb, base) in &buf[..k] {
                let i = nb as usize;
                if !corridor.is_empty() && corridor.binary_search(&g.region(nb)).is_err() {
                    continue;
                }
                if reserved.get(i).is_some_and(|&o| o != NONE && o != net) {
                    continue;
                }
                let nd = d + base + node_cost(i);
                if seen[i] != s || nd < dist[i] {
                    seen[i] = s;
                    dist[i] = nd;
                    prev[i] = n;
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
        for &n in &path {
            tree.push(n);
            in_tree[n as usize] = call;
        }
        branches.push(path);
    }
    Some(branches)
}

/// Epochs that restrict each search to its corridor.
const CORRIDOR_EPOCHS: u32 = 8;

/// PathFinder: each epoch reroutes dirty nets (no tree, or a node over capacity)
/// in `order`, then adds `hist_inc · overuse` to history on overused nodes.
/// Stops at zero overflow, on a no-move epoch, or after `max_iters`. Returns the
/// residual overflow `Σ max(0, usage − cap)`.
pub fn run_pathfinder<G: RGraph>(hot: &mut RouteHot, cold: &RouteCtx<G>, p_fac: f32, hist_inc: f32, max_iters: u32) -> f32 {
    let cap = cold.graph.cap();
    let mut dij = Dij::new(cold.graph.nodes());
    let mut overflow = 0.0;
    for epoch in 0..max_iters {
        let mut moved = false;
        for &net in &cold.order {
            let net = net as usize;
            let tree = &hot.trees[net];
            let dirty = tree.is_empty() || tree.iter().flatten().any(|&n| hot.usage[n as usize] > cap);
            if !dirty || cold.terms[net].is_empty() {
                continue;
            }
            if let Some(branches) = cold.reroute(hot, net, epoch < CORRIDOR_EPOCHS, p_fac, &mut dij) {
                hot.commit(net, branches);
                moved = true;
            }
        }
        overflow = bump_history(&hot.usage, &mut hot.hist, cap, hist_inc);
        if !moved || overflow == 0.0 {
            break;
        }
    }
    overflow
}

/// `hist += inc · max(0, usage − cap)` per node; returns `Σ max(0, usage − cap)`.
fn bump_history(usage: &[u16], hist: &mut [f32], cap: u16, inc: f32) -> f32 {
    let mut total = 0.0;
    for (h, &u) in hist.iter_mut().zip(usage) {
        let over = f32::from(u.saturating_sub(cap));
        *h += inc * over;
        total += over;
    }
    total
}

// ---------------------------------------------------------------------------
// Route tree → geometry
// ---------------------------------------------------------------------------

/// One extracted straight run (track-layer index, centre-line endpoints).
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

/// A layer transition at `(x, y)` from track layer `layer` to `layer + 1`.
#[derive(Clone, Copy)]
pub struct Via {
    pub net: u32,
    pub x: i32,
    pub y: i32,
    pub size: i32,
    pub layer: u32,
}

/// Straight runs and vias of every tree. Single-node runs emit no wire; vias are
/// deduped by `(x, y, layer)`.
#[must_use]
pub fn extract_geometry(hot: &RouteHot, grid: &TrackGrid, width: i32) -> (Vec<Wire>, Vec<Via>) {
    let (mut wires, mut vias) = (Vec::new(), Vec::new());
    for (net, tree) in hot.trees.iter().enumerate() {
        let net = net as u32;
        for branch in tree {
            let mut start = 0;
            for i in 1..branch.len() {
                let (px, py, pl) = grid.pos(branch[i - 1]);
                let (cx, cy, cl) = grid.pos(branch[i]);
                if pl != cl {
                    emit_run(&mut wires, grid, net, &branch[start..i], width);
                    vias.push(Via { net, x: cx.min(px), y: cy.min(py), size: width, layer: pl.min(cl) });
                    start = i;
                }
            }
            emit_run(&mut wires, grid, net, &branch[start..], width);
        }
    }
    vias.sort_unstable_by_key(|v| (v.x, v.y, v.layer, v.net));
    vias.dedup_by_key(|v| (v.x, v.y, v.layer));
    (wires, vias)
}

/// Split a same-layer node run into straight wires.
fn emit_run(wires: &mut Vec<Wire>, grid: &TrackGrid, net: u32, nodes: &[u32], width: i32) {
    let Some(&first) = nodes.first() else { return };
    let (mut sx, mut sy, layer) = grid.pos(first);
    let (mut lx, mut ly) = (sx, sy);
    for &n in &nodes[1..] {
        let (x, y, _) = grid.pos(n);
        let straight = if layer % 2 == 1 { x == lx } else { y == ly };
        if !straight {
            wires.push(Wire { net, layer, x0: sx, y0: sy, x1: lx, y1: ly, width });
            (sx, sy) = (lx, ly);
        }
        (lx, ly) = (x, y);
    }
    if (lx, ly) != (sx, sy) {
        wires.push(Wire { net, layer, x0: sx, y0: sy, x1: lx, y1: ly, width });
    }
}

/// Per-net shapes (layer = track index): a wire is its centre line widened by
/// `width/2` on every side; a via is a `size²` square centred on `(x, y)`.
#[must_use]
pub fn to_shapes(nets: usize, wires: &[Wire], vias: &[Via]) -> Vec<Vec<Shape>> {
    let mut out = vec![Vec::new(); nets];
    for w in wires {
        let h = w.width / 2;
        let (x0, y0) = (w.x0.min(w.x1) - h, w.y0.min(w.y1) - h);
        let (x1, y1) = (w.x0.max(w.x1) + h, w.y0.max(w.y1) + h);
        let rect = Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
        out[w.net as usize].push(Shape { layer: LayerId(w.layer as u16), rect });
    }
    for v in vias {
        let h = v.size / 2;
        let rect = Rect { x: v.x - h, y: v.y - h, w: v.size, h: v.size };
        out[v.net as usize].push(Shape { layer: LayerId(v.layer as u16), rect });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use pnr_core::geom::Pin;
    use pnr_core::ids::NetId;

    fn m(pins: Vec<(u16, i32, i32)>) -> Macro {
        let r = Rect { x: 0, y: 0, w: 20_000, h: 20_000 };
        let pin = |(net, x, y)| Pin {
            name: format!("p{net}"),
            net: NetId(net),
            at: Rect { x, y, w: 200, h: 200 },
            layer: LayerId(0),
        };
        Macro { shapes: vec![Shape { layer: LayerId(0), rect: r }], pins: pins.into_iter().map(pin).collect(), bbox: r }
    }

    /// Empty layout: every macro keeps its own (absolute) coordinates.
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

    const LAYERS: [LayerId; 2] = [LayerId(0), LayerId(1)];

    #[test]
    fn two_pin_net_routes_coarsely() {
        let macros = vec![m(vec![(0, 1_000, 1_000)]), m(vec![(0, 18_000, 18_000)])];
        let reqs = Requirements::<Routes>::default();
        let (routes, report) =
            GlobalRoute::default().route(&lay(), &macros, &[], &reqs, &LAYERS, &mut Negotiation::new(), 7);
        assert_eq!(routes.wires.len(), 1);
        assert!(!routes.wires[0].is_empty());
        assert!(report.hard_violations.is_empty());
    }

    /// History must survive `route` and change the next call's result.
    #[test]
    fn negotiation_persists_across_calls() {
        let cfg = GlobalCfg { gcells_per_side: 4, gcell_capacity: 1, ..GlobalCfg::default() };
        let router = GlobalRoute { cfg };
        let macros: Vec<Macro> = (0..6u16)
            .flat_map(|n| {
                let off = i32::from(n) * 1_000;
                [m(vec![(n, 1_000 + off, 1_000)]), m(vec![(n, 18_000 - off, 18_000)])]
            })
            .collect();
        let reqs = Requirements::<Routes>::default();
        let run = |neg: &mut Negotiation| router.route(&lay(), &macros, &[], &reqs, &LAYERS, neg, 3).0;
        let flat = |r: &Routes| -> Vec<(u16, i32, i32, i32, i32)> {
            r.wires.iter().flatten().map(|s| (s.layer.0, s.rect.x, s.rect.y, s.rect.w, s.rect.h)).collect()
        };
        let mut neg = Negotiation::new();
        let first = run(&mut neg);
        let p1 = neg.pressure();
        assert!(p1 > 0.0);
        let second = run(&mut neg);
        assert!(neg.pressure() > p1);
        assert_ne!(flat(&first), flat(&second), "second call was not seeded with history");
    }

    fn wire(len: i32) -> Routes {
        Routes { wires: vec![vec![Shape { layer: LayerId(0), rect: Rect { x: 0, y: 0, w: len, h: 1 } }]] }
    }

    fn budget(cap: i64) -> Box<dyn analog::RuleBatch<Routes>> {
        Box::new(vec![analog::routing::ParasiticBudget {
            net: NetId(0),
            max_r_mohm: 1_000_000,
            max_c_af: 100_000_000,
            max_len_nm: cap,
            margin_pct: 10,
        }])
    }

    /// A budget residual reaches Θ with a measured, nonzero margin.
    #[test]
    fn budget_residual_reaches_theta() {
        let mut reqs = Requirements::<Routes>::default();
        reqs.budget.push(budget(1_000));
        assert!(score(&wire(900), &reqs, 0.0, 0.0).budget_violations.is_empty());
        let over = score(&wire(1_500), &reqs, 0.0, 0.0);
        assert_eq!(over.budget_violations[0].margin, 500);
        assert!(!over.feasible());
    }

    /// Hard margins are measured overshoot, not counts.
    #[test]
    fn hard_batch_margin_is_measured_not_counted() {
        let margin_of = |cap: i64| {
            let mut reqs = Requirements::<Routes>::default();
            reqs.hard.push(budget(cap));
            score(&wire(3_000), &reqs, 0.0, 0.0).hard_violations[0].margin
        };
        assert_eq!((margin_of(2_000), margin_of(1_000)), (500, 2_000));
    }

    /// Overflow's Θ entry is a capacity residual; `cost` keeps the raw count.
    #[test]
    fn overflow_theta_is_a_capacity_residual_not_a_count() {
        let reqs = Requirements::<Routes>::default();
        let r = score(&wire(100), &reqs, 3.0, 600.0);
        assert_eq!(r.budget_violations[0].margin, 5);
        assert_eq!(r.cost, 3.0);
        assert_eq!(score(&wire(100), &reqs, 1.0, 1_000_000.0).budget_violations[0].margin, 1);
    }

    #[test]
    fn price_group_measures_the_group_not_the_die() {
        let far = |x: i32, y: i32| {
            let mut mac = m(vec![(0, x, y)]);
            mac.bbox = Rect { x, y, w: 2_000, h: 2_000 };
            mac.shapes[0].rect = mac.bbox;
            mac
        };
        let price = price_group(&[far(100_000, 100_000), far(108_000, 106_000)], &LAYERS, &GlobalCfg::default());
        assert!(price.reachable);
        assert_eq!(price.overflow, 0);
        assert_eq!(price.hpwl, 14_000);
    }

    #[test]
    fn bump_history_matches_definition() {
        let usage = [0u16, 1, 2, 5];
        let mut hist = [0.0f32; 4];
        assert_eq!(bump_history(&usage, &mut hist, 1, 0.5), 5.0);
        assert_eq!(hist, [0.0, 0.0, 0.5, 2.0]);
    }
}
