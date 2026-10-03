//! # `gr` — the routing mechanics `dr` runs: the track lattice ([`TrackGrid`]), the
//! search ([`route_net`], [`run_pathfinder`]), negotiation history ([`Negotiation`])
//! and geometry extraction ([`extract_geometry`]). The coarse gcell router is
//! retired (RTE-07): its plan was discarded and only seeded corridors.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

use analog::{RepairKind, Requirements};
use pnr_core::geom::{LayerId, Rect, Shape};
use pnr_core::report::Violation;
use pnr_core::{Macro, Routes};

pub use pnr_core::place_macros;

/// PathFinder history `h_n`, owned by the orchestrator and carried across outer
/// epochs so the negotiation keeps its prices (the anti-oscillation guarantee).
///
/// Keyed by absolute 500 nm buckets, not grid index: grids are rebuilt from a
/// bounding box the placer moves every epoch. `BTreeMap` + `max` folding keep
/// [`Negotiation::pressure`] deterministic. Quantum `HIST_QUANTUM_NM` until
/// RTE-10 keys on the lattice pitch.
#[derive(Default, Clone)]
pub struct Negotiation {
    hist: BTreeMap<(u32, i32, i32), f32>,
}

const HIST_QUANTUM_NM: i32 = 500;

fn hist_key((x, y, layer): (i32, i32, u32)) -> (u32, i32, i32) {
    (layer, x.div_euclid(HIST_QUANTUM_NM), y.div_euclid(HIST_QUANTUM_NM))
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
    pub fn seed(&self, hist: &mut [f32], pos: impl Fn(u32) -> (i32, i32, u32)) {
        if self.hist.is_empty() {
            return;
        }
        for (n, h) in hist.iter_mut().enumerate() {
            *h = self.hist.get(&hist_key(pos(n as u32))).copied().unwrap_or(0.0);
        }
    }

    /// Fold per-node history back in (`max` per bucket: resolution- and order-free).
    pub fn accumulate(&mut self, hist: &[f32], pos: impl Fn(u32) -> (i32, i32, u32)) {
        for (n, &h) in hist.iter().enumerate() {
            if h > 0.0 {
                let e = self.hist.entry(hist_key(pos(n as u32))).or_insert(0.0);
                *e = e.max(h);
            }
        }
    }
}

/// Σ over nets of the half-perimeter of their pin centres, nm (a net
/// whose pins share one centre adds 0): the wirelength half of pricing one
/// cell's variant in isolation.
#[must_use]
pub fn group_hpwl(macros: &[Macro]) -> i64 {
    let mut span: BTreeMap<u16, (i32, i32, i32, i32)> = BTreeMap::new();
    for p in macros.iter().flat_map(|m| &m.pins) {
        let (x, y) = (p.at.x + p.at.w / 2, p.at.y + p.at.h / 2);
        let e = span.entry(p.net.0).or_insert((x, x, y, y));
        *e = (e.0.min(x), e.1.max(x), e.2.min(y), e.3.max(y));
    }
    span.values().map(|&(x0, x1, y0, y1)| i64::from(x1 - x0) + i64::from(y1 - y0)).sum()
}

/// Route order, compact net ids (ROAD's policy, TOPO ch. 4; Lampaert 1999
/// eqs. 5.12–5.14): nets under a symmetry rule (`Differential`) first, then
/// the rest under a hard rule, then under a budget, then free nets; within a
/// tier higher impact `weight` first (the most critical claim tracks before
/// the nets that can detour), then fewer terminals, then index. Shield
/// references go last: ROAD builds shields after everything else.
///
/// ponytail: impact is the net's own sensitivity weight, not Lampaert's `F_k`
/// from a pre-route of every net (`Σ_j ΔP_j^k / ΔP_j`).
#[must_use]
pub fn order_by_priority(pins: &[usize], net_ids: &[u32], reqs: &Requirements<Routes>, weight: &[f32]) -> Vec<u32> {
    let sym = symmetric_nets(reqs);
    let (mut hard, mut budget, mut shields) = (Vec::new(), Vec::new(), Vec::new());
    for b in reqs.hard.iter().filter(|b| b.repair_kind() != RepairKind::Mirror) {
        b.touched(&mut hard);
    }
    for b in &reqs.budget {
        b.touched(&mut budget);
    }
    for b in reqs.hard.iter().chain(&reqs.budget) {
        b.shield_pairs(&mut shields);
    }
    let tier = |ci: u32| {
        let net = net_ids[ci as usize];
        let t = [&sym, &hard, &budget].iter().position(|ids| ids.contains(&net)).unwrap_or(3);
        (shields.iter().any(|&(_, r)| r == net), t)
    };
    let impact = |ci: u32| Reverse(weight.get(ci as usize).copied().unwrap_or(0.0).to_bits());
    let mut order: Vec<u32> = (0..pins.len() as u32).collect();
    order.sort_by_key(|&a| (tier(a), impact(a), pins[a as usize], a));
    order
}

/// Nets under a mirror rule (`Differential`, [`RepairKind::Mirror`]), hard or budget.
#[must_use]
pub fn symmetric_nets(reqs: &Requirements<Routes>) -> Vec<u32> {
    let mut out = Vec::new();
    for b in reqs.hard.iter().chain(&reqs.budget).filter(|b| b.repair_kind() == RepairKind::Mirror) {
        b.touched(&mut out);
    }
    out
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
        .map(|(i, b)| Violation::from_residual(format!("{}routing hard {i}", Violation::BATCH), b.residual(routes)))
        .collect();
    let budget = reqs
        .budget
        .iter()
        .enumerate()
        .filter_map(|(i, b)| {
            let r = b.residual(routes);
            (r > 0.0).then(|| Violation::from_residual(format!("{}routing budget {i}", Violation::BATCH), r))
        })
        .collect();
    (hard, budget)
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
    /// Writes the nodes on the tracks either side of `n` (same layer, same
    /// run position) — where a wire through `n` couples laterally; returns the
    /// count. Default: none (a gcell has no tracks).
    fn beside(&self, n: u32, out: &mut [u32; 2]) -> usize {
        let _ = (n, out);
        0
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
}

impl TrackGrid {
    /// Track grid over `die`; `pitch` is raised to at least `max(die)/1200`.
    #[must_use]
    pub fn with_layers(die: (i32, i32), pitch: i32, via_cost: f32, n_layers: u32) -> Self {
        let pitch = pitch.max(die.0.max(die.1) / 1200).max(1);
        let nx = (die.0 / pitch).max(2) as u32;
        let ny = (die.1 / pitch).max(2) as u32;
        Self { nx, ny, pitch, via_cost, n_layers: n_layers.max(1) }
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
    fn beside(&self, n: u32, out: &mut [u32; 2]) -> usize {
        let (ix, iy, layer) = self.ixy(n);
        let (lo, hi) = if layer % 2 == 0 { (iy > 0, iy + 1 < self.ny) } else { (ix > 0, ix + 1 < self.nx) };
        let step = if layer % 2 == 0 { self.nx } else { 1 };
        let mut k = 0;
        for (ok, m) in [(lo, n.wrapping_sub(step)), (hi, n + step)] {
            if ok {
                out[k] = m;
                k += 1;
            }
        }
        k
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
    /// Per net, its parasitic weight ([`Elec::weight`]); empty = none.
    pub weight: Vec<f32>,
    /// Per node, the summed weight of the nets on it: what a foreign wire
    /// alongside costs them ([`Elec::sens`]). Kept by [`RouteHot::commit`].
    pub sens: Vec<f32>,
}

impl RouteHot {
    #[must_use]
    pub fn new(nodes: usize, nets: usize) -> Self {
        Self { usage: vec![0; nodes], hist: vec![0.0; nodes], trees: vec![Vec::new(); nets], weight: Vec::new(), sens: Vec::new() }
    }

    /// Price parasitics with per-net `weight`: tracks each node's [`RouteHot::sens`].
    pub fn set_weights(&mut self, weight: Vec<f32>) {
        self.sens = vec![0.0; self.usage.len()];
        for (net, &w) in weight.iter().enumerate() {
            for n in self.tree_nodes(net) {
                self.sens[n as usize] += w;
            }
        }
        self.weight = weight;
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
        let w = self.weight.get(net).copied().unwrap_or(0.0);
        for n in self.tree_nodes(net) {
            self.usage[n as usize] -= 1;
            if w > 0.0 {
                self.sens[n as usize] -= w;
            }
        }
        self.trees[net] = branches;
        for n in self.tree_nodes(net) {
            self.usage[n as usize] += 1;
            if w > 0.0 {
                self.sens[n as usize] += w;
            }
        }
    }
}

/// Immutable per-run context. `reserved[node]` is a hard owner (`NONE` = free).
pub struct RouteCtx<G> {
    pub graph: G,
    pub terms: Vec<Vec<u32>>,
    pub order: Vec<u32>,
    pub reserved: Vec<u32>,
    /// Per net, how much its parasitics cost ([`Elec::weight`]); empty = none.
    pub weight: Vec<f32>,
    /// Per graph layer, [`Elec::layer_c`] and [`Elec::beside_c`]; empty = none.
    pub layer_c: Vec<f32>,
    pub beside_c: Vec<f32>,
    /// Per net, route on length and congestion alone, blind to [`Elec`]: the
    /// two halves of a differential pair must see mirrored costs, and the
    /// parasitic field around them is not mirror-symmetric (a hard symmetry
    /// outranks a C budget). Empty = none.
    pub plain: Vec<bool>,
    /// Per net [`Elec::current`], and per layer [`Elec::layer_r`] / [`Elec::via_r`];
    /// empty = none.
    pub current: Vec<f32>,
    pub layer_r: Vec<f32>,
    pub via_r: Vec<f32>,
    /// Per node, the matched cell it lies over (`NONE` = none), and per net
    /// the sorted cells it has a pin in: a net pays [`KEEPOUT_COST`] per node
    /// over a matched cell it does not belong to (a line over one member of a
    /// pair skews it, Razavi Fig. 19.17). Empty = none.
    pub keepout: Vec<u32>,
    pub own_cells: Vec<Vec<u32>>,
}

/// Extra cost of one node over a foreign matched cell, in steps.
pub const KEEPOUT_COST: f32 = 2.0;

impl<G: RGraph> RouteCtx<G> {
    /// No reservations.
    pub fn new(graph: G, terms: Vec<Vec<u32>>, order: Vec<u32>) -> Self {
        Self {
            graph,
            terms,
            order,
            reserved: Vec::new(),
            weight: Vec::new(),
            layer_c: Vec::new(),
            beside_c: Vec::new(),
            plain: Vec::new(),
            current: Vec::new(),
            layer_r: Vec::new(),
            via_r: Vec::new(),
            keepout: Vec::new(),
            own_cells: Vec::new(),
        }
    }

    /// Route `net` against the current state with an extra per-node `penalty`
    /// (empty = none).
    pub fn reroute(&self, hot: &RouteHot, net: usize, p_fac: f32, penalty: &[f32], dij: &mut Dij) -> Option<Vec<Vec<u32>>> {
        let old = hot.tree_nodes(net);
        let elec = if self.plain.get(net).copied().unwrap_or(false) {
            Elec::default()
        } else {
            Elec {
                weight: self.weight.get(net).copied().unwrap_or(0.0),
                layer_c: &self.layer_c,
                beside_c: &self.beside_c,
                sens: &hot.sens,
                current: self.current.get(net).copied().unwrap_or(0.0),
                layer_r: &self.layer_r,
                via_r: &self.via_r,
            }
        };
        let (g, t, r) = (&self.graph, &self.terms[net], &self.reserved);
        let own = self.own_cells.get(net).map_or(&[][..], Vec::as_slice);
        route_net(g, &hot.usage, &hot.hist, &old, t, net as u32, r, penalty, (&self.keepout, own), p_fac, &elec, dij)
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

/// The electrical part of a node's cost for one net (Lampaert 1999 eqs.
/// 5.10–5.11, `ΔP = S_C·C + ΣS_Cc·C_c`; ANAGRAM's ParasiticCost, TOPO eq. 4.4):
/// `weight · layer_c[l] + beside_c[l] · Σ_alongside (weight + their sens)` over
/// foreign tracks alongside: a coupling costs both nets' sensitivities
/// (`ΣS_Cc·C_c` counts the victim whichever net is routed second), so an
/// unweighted aggressor still pays to run beside a sensitive net. Every term
/// is ≥ 0, so Dijkstra stays exact.
///
/// Series R is priced for current-carrying nets: `current · layer_r[l]` per
/// step and `current · via_r[l]` per via.
///
/// ponytail: via C is not priced; R is at the drawn wire width (fattening
/// happens after the search).
#[derive(Clone, Copy, Default)]
pub struct Elec<'a> {
    /// The net's sensitivity to its own C, `[0, 1]` (1 = the most sensitive).
    pub weight: f32,
    /// Ground C of one step on each layer, over the cheapest layer's.
    pub layer_c: &'a [f32],
    /// Lateral C to one occupied adjacent track per step, same scale.
    pub beside_c: &'a [f32],
    /// Per node, the summed weight of the nets on it ([`RouteHot::sens`]).
    pub sens: &'a [f32],
    /// The net's DC current over the heaviest net's, `[0, 1]`: what its series
    /// R costs (IR drop, PWR-02; Lampaert eq. 5.10's `S_R·R`).
    pub current: f32,
    /// Series R of one step on each layer, and of one via up from each layer,
    /// over the least resistive layer's step.
    pub layer_r: &'a [f32],
    pub via_r: &'a [f32],
}

/// Connect `terms` into a branch tree by repeated multi-source Dijkstra from the
/// growing tree, targets nearest-first. Node cost is `hist + p_fac·overuse +
/// penalty + keepout + elec`, where the net's own `old_nodes` do not count as
/// usage and `keepout = (cell per node, the net's own cells)` charges
/// [`KEEPOUT_COST`] over any other cell.
/// `None` if a target is unreachable without entering a node `reserved` for
/// another net.
#[allow(clippy::too_many_arguments)]
pub fn route_net<G: RGraph>(
    g: &G,
    usage: &[u16],
    hist: &[f32],
    old_nodes: &[u32],
    terms: &[u32],
    net: u32,
    reserved: &[u32],
    penalty: &[f32],
    keepout: (&[u32], &[u32]),
    p_fac: f32,
    elec: &Elec,
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
    let parasitic = |i: usize| -> f32 {
        if elec.weight <= 0.0 && elec.sens.is_empty() && elec.current <= 0.0 {
            return 0.0;
        }
        let l = g.pos(i as u32).2 as usize;
        let mut beside = [0u32; 2];
        let k = g.beside(i as u32, &mut beside);
        // Foreign tracks alongside, each at this net's weight plus theirs.
        let coupled: f32 = beside[..k]
            .iter()
            .map(|&j| {
                let (j, mine) = (j as usize, is_old[j as usize] == call);
                let foreign = usage[j] > u16::from(mine);
                let theirs = elec.sens.get(j).map_or(0.0, |&s| s - if mine { elec.weight } else { 0.0 });
                if foreign { elec.weight + theirs.max(0.0) } else { 0.0 }
            })
            .sum();
        let per = |v: &[f32]| v.get(l).copied().unwrap_or(0.0);
        elec.weight * per(elec.layer_c) + per(elec.beside_c) * coupled + elec.current * per(elec.layer_r)
    };
    // A via's series R, charged on the layer change.
    let via = |a: u32, b: u32| -> f32 {
        if elec.current <= 0.0 {
            return 0.0;
        }
        let (la, lb) = (g.pos(a).2, g.pos(b).2);
        if la == lb { 0.0 } else { elec.current * elec.via_r.get(la.min(lb) as usize).copied().unwrap_or(0.0) }
    };
    let node_cost = |i: usize| -> f32 {
        let eff = usage[i].saturating_sub(u16::from(is_old[i] == call));
        let foreign = keepout.0.get(i).is_some_and(|&c| c != NONE && keepout.1.binary_search(&c).is_err());
        hist[i] + p_fac * f32::from((eff + 1).saturating_sub(cap)) + penalty.get(i).copied().unwrap_or(0.0) + parasitic(i)
            + if foreign { KEEPOUT_COST } else { 0.0 }
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
                if reserved.get(i).is_some_and(|&o| o != NONE && o != net) {
                    continue;
                }
                let nd = d + base + node_cost(i) + via(n, nb);
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

/// Present-congestion factor growth per PathFinder iteration, and its cap
/// (tuning defaults; the cap keeps f32 costs finite).
const P_GROWTH: f32 = 1.3;
const P_FAC_MAX: f32 = 1000.0;
/// Iterations without an overflow decrease before negotiation gives up
/// (tuning default): an unsolvable instance stops here, not at `max_iters`.
const STALL_ITERS: u32 = 10;

/// PathFinder: iteration `i` reroutes the dirty nets (no tree, or a node over
/// capacity) in `order` at present-congestion factor
/// `min(p_fac·P_GROWTH^i, P_FAC_MAX)`, then adds `hist_inc · overuse` to
/// history. Stops at zero overflow, on a no-move iteration, after
/// `STALL_ITERS` iterations without an overflow decrease, or after
/// `max_iters`. Returns `(Σ max(0, usage − cap), iterations run)`.
pub fn run_pathfinder<G: RGraph>(hot: &mut RouteHot, cold: &RouteCtx<G>, p_fac: f32, hist_inc: f32, max_iters: u32) -> (f32, u32) {
    let cap = cold.graph.cap();
    let mut dij = Dij::new(cold.graph.nodes());
    let (mut overflow, mut best, mut stall, mut iters, mut p) = (0.0, f32::INFINITY, 0, 0, p_fac.min(P_FAC_MAX));
    for _ in 0..max_iters {
        iters += 1;
        let mut moved = false;
        for &net in &cold.order {
            let net = net as usize;
            let tree = &hot.trees[net];
            let dirty = tree.is_empty() || tree.iter().flatten().any(|&n| hot.usage[n as usize] > cap);
            if !dirty || cold.terms[net].is_empty() {
                continue;
            }
            if let Some(branches) = cold.reroute(hot, net, p, &[], &mut dij) {
                hot.commit(net, branches);
                moved = true;
            }
        }
        overflow = bump_history(&hot.usage, &mut hot.hist, cap, hist_inc);
        if overflow < best {
            (best, stall) = (overflow, 0);
        } else {
            stall += 1;
        }
        if !moved || overflow == 0.0 || stall >= STALL_ITERS {
            break;
        }
        p = (p * P_GROWTH).min(P_FAC_MAX);
    }
    (overflow, iters)
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
        Macro { shapes: vec![Shape { layer: LayerId(0), rect: r }], pins: pins.into_iter().map(pin).collect(), bbox: r, units: Vec::new(), dummies: Vec::new(), ..Default::default() }
    }

    fn wire(len: i32) -> Routes {
        Routes { wires: vec![vec![Shape { layer: LayerId(0), rect: Rect { x: 0, y: 0, w: len, h: 1 } }]], ..Default::default()  }
    }

    fn budget(cap: i64) -> Box<dyn analog::RuleBatch<Routes>> {
        Box::new(vec![analog::routing::ParasiticBudget {
            net: NetId(0),
            max_len_nm: cap,
            max_c_af: 0,
            margin_pct: 10,
            stack: None,
        }])
    }

    /// A budget residual reaches Θ with a measured, nonzero margin.
    #[test]
    fn budget_residual_reaches_theta() {
        let mut reqs = Requirements::<Routes>::default();
        reqs.budget.push(budget(1_000));
        assert!(analog_tiers(&wire(900), &reqs).1.is_empty());
        assert_eq!(analog_tiers(&wire(1_500), &reqs).1[0].margin, 500);
    }

    /// Hard margins are measured overshoot, not counts.
    #[test]
    fn hard_batch_margin_is_measured_not_counted() {
        let margin_of = |cap: i64| {
            let mut reqs = Requirements::<Routes>::default();
            reqs.hard.push(budget(cap));
            analog_tiers(&wire(3_000), &reqs).0[0].margin
        };
        assert_eq!((margin_of(2_000), margin_of(1_000)), (500, 2_000));
    }

    #[test]
    fn group_hpwl_measures_the_group_not_the_die() {
        let far = |x: i32, y: i32| {
            let mut mac = m(vec![(0, x, y)]);
            mac.bbox = Rect { x, y, w: 2_000, h: 2_000 };
            mac.shapes[0].rect = mac.bbox;
            mac
        };
        assert_eq!(group_hpwl(&[far(100_000, 100_000), far(108_000, 106_000)]), 14_000);
    }

    /// Three nets crossing one 12×12 two-layer lattice resolve: the growing
    /// present-congestion factor ends negotiation at zero overflow, early.
    #[test]
    fn pathfinder_converges_on_a_solvable_crossing() {
        let g = TrackGrid::with_layers((12 * 200, 12 * 200), 200, 4.0, 2);
        let n = |x, y| g.node(x, y, 0);
        let terms = vec![vec![n(0, 4), n(11, 6)], vec![n(0, 5), n(11, 5)], vec![n(0, 6), n(11, 4)]];
        let cold = RouteCtx::new(g, terms, vec![0, 1, 2]);
        let mut hot = RouteHot::new(cold.graph.nodes(), 3);
        let (over, iters) = run_pathfinder(&mut hot, &cold, 1.0, 0.5, 150);
        assert_eq!(over, 0.0);
        assert!(iters < 150, "{iters}");
    }

    /// Two nets sharing a terminal node overflow it whatever they route: the
    /// stall rule ends negotiation (the lower bound rules out the no-move rule).
    #[test]
    fn an_unsolvable_instance_stops_on_stall() {
        let g = TrackGrid::with_layers((5 * 200, 2 * 200), 200, 4.0, 1);
        let n = |x, y| g.node(x, y, 0);
        let terms = vec![vec![n(0, 0), n(2, 0)], vec![n(2, 0), n(4, 0)]];
        let cold = RouteCtx::new(g, terms, vec![0, 1]);
        let mut hot = RouteHot::new(cold.graph.nodes(), 2);
        let (over, iters) = run_pathfinder(&mut hot, &cold, 1.0, 0.5, 150);
        assert_eq!(over, 1.0);
        assert!((STALL_ITERS..=STALL_ITERS + 1).contains(&iters), "{iters}");
    }

    /// The same absolute node (−750 nm) seen through two frame origins hits
    /// one history bucket.
    #[test]
    fn history_keys_ignore_the_frame() {
        let mut neg = Negotiation::new();
        neg.accumulate(&[3.0], |_| (250 - 1_000, 250, 0));
        let mut h = [0.0];
        neg.seed(&mut h, |_| (750 - 1_500, 250, 0));
        assert_eq!(h[0], 3.0);
    }

    #[test]
    fn bump_history_matches_definition() {
        let usage = [0u16, 1, 2, 5];
        let mut hist = [0.0f32; 4];
        assert_eq!(bump_history(&usage, &mut hist, 1, 0.5), 5.0);
        assert_eq!(hist, [0.0, 0.0, 0.5, 2.0]);
    }

    /// A budgeted net (under a crosstalk-exclusion rule) routes ahead of an
    /// unconstrained one.
    #[test]
    fn budgeted_nets_route_first() {
        use analog::routing::CrosstalkExclusion;
        let mut reqs = Requirements::<Routes>::default();
        let rule = CrosstalkExclusion { a: NetId(0), b: NetId(1), min_spacing_nm: 2_000, margin_pct: 0 };
        reqs.budget.push(Box::new(vec![rule]));

        let pins = [2usize; 3];
        assert_eq!(order_by_priority(&pins, &[0, 1, 2], &reqs, &[]), vec![0, 1, 2]);
        assert_eq!(order_by_priority(&pins, &[2, 1, 0], &reqs, &[]), vec![1, 2, 0], "the budgeted nets route first");
    }

    /// The electrical term steers the search: with no weight a net runs
    /// straight beside a foreign wire; weighted — or beside a weighted wire —
    /// it jogs one track away to stop coupling.
    #[test]
    fn a_sensitive_net_steers_off_a_coupled_track() {
        let g = TrackGrid::with_layers((40 * 200, 10 * 200), 200, 4.0, 2);
        let mut usage = vec![0u16; g.nodes()];
        for x in 0..40 {
            usage[g.node(x, 5, 0) as usize] = 1; // a foreign horizontal wire on row 5
        }
        let hist = vec![0.0; g.nodes()];
        let terms = [g.node(0, 4, 0), g.node(39, 4, 0)];
        // The foreign wire's own sensitivity, for the aggressor case below.
        let mut sens = vec![0.0f32; g.nodes()];
        for x in 0..40 {
            sens[g.node(x, 5, 0) as usize] = 1.0;
        }
        let path = |w: f32, sens: &[f32]| {
            let elec = Elec { weight: w, layer_c: &[1.0, 1.0], beside_c: &[1.0, 1.0], sens, ..Elec::default() };
            let tree = route_net(&g, &usage, &hist, &[], &terms, 0, &[], &[], (&[], &[]), 1.0, &elec, &mut Dij::new(g.nodes())).unwrap();
            let mut nodes = tree.concat();
            nodes.sort_unstable();
            nodes.dedup();
            nodes
        };
        let beside = |p: &[u32]| p.iter().filter(|&&n| g.ixy(n).1 == 4 && g.ixy(n).2 == 0).count();
        assert_eq!(beside(&path(0.0, &[])), 40, "unweighted: the straight run");
        assert!(beside(&path(1.0, &[])) < 10, "weighted: off the coupled row");
        // An unweighted net keeps off a sensitive net's track too: the coupling
        // costs the victim whichever of the two is routed second.
        assert!(beside(&path(0.0, &sens)) < 10, "aggressor: off the victim's row");
        // A differential half routes blind to all of it (`RouteCtx::plain`).
        let mut cold = RouteCtx::new(g, vec![terms.to_vec()], vec![0]);
        (cold.weight, cold.layer_c, cold.beside_c, cold.plain) = (vec![1.0], vec![1.0; 2], vec![1.0; 2], vec![true]);
        let mut hot = RouteHot::new(cold.graph.nodes(), 1);
        (hot.usage, hot.sens) = (usage.clone(), sens.clone());
        let tree = cold.reroute(&hot, 0, 1.0, &[], &mut Dij::new(cold.graph.nodes())).unwrap();
        let mut nodes = tree.concat();
        nodes.sort_unstable();
        nodes.dedup();
        assert_eq!(nodes.iter().filter(|&&n| cold.graph.ixy(n).1 == 4).count(), 40, "plain: the straight run");
    }

    /// A current-carrying net pays its vias' series R: where an idle net hops
    /// layers to dodge a costly node, a heavy one goes straight through.
    #[test]
    fn a_current_carrying_net_avoids_via_resistance() {
        let g = TrackGrid::with_layers((40 * 200, 10 * 200), 200, 4.0, 2);
        let usage = vec![0u16; g.nodes()];
        let mut hist = vec![0.0; g.nodes()];
        hist[g.node(20, 4, 0) as usize] = 20.0;
        let terms = [g.node(0, 4, 0), g.node(39, 4, 0)];
        let vias = |current: f32| {
            let elec = Elec { current, layer_r: &[1.0, 1.0], via_r: &[24.0], ..Elec::default() };
            let tree = route_net(&g, &usage, &hist, &[], &terms, 0, &[], &[], (&[], &[]), 1.0, &elec, &mut Dij::new(g.nodes())).unwrap();
            tree.iter().flat_map(|b| b.windows(2)).filter(|w| g.ixy(w[0]).2 != g.ixy(w[1]).2).count()
        };
        assert!(vias(0.0) > 0, "idle: hops around the costly node");
        assert_eq!(vias(1.0), 0, "heavy: straight through, no via R");
    }

    /// A differential pair routes first, higher impact before lower within a
    /// tier, and a shield's reference last.
    #[test]
    fn symmetric_nets_first_then_impact_shields_last() {
        use analog::routing::{Differential, Shield};
        let mut reqs = Requirements::<Routes>::default();
        reqs.budget.push(Box::new(vec![Differential { pos: NetId(3), neg: NetId(4), max_len_delta_pct10: 50, same_layer_required: true, stack: None, aggressor_weight: None }]));
        reqs.budget.push(Box::new(vec![Shield { victim: NetId(5), reference: NetId(2), min_coverage_pct: 80, max_gap_nm: 400 }]));
        // Compact i is net i; nets 0 and 1 are free, 1 the more sensitive.
        let order = order_by_priority(&[2; 6], &[0, 1, 2, 3, 4, 5], &reqs, &[0.2, 0.9, 0.0, 0.0, 0.0, 0.0]);
        assert_eq!(order, vec![3, 4, 5, 1, 0, 2], "pair, budgeted victim, free by impact, shield reference");
    }
}
