//! Routing domain types, graphs, and slot implementations.
//!
//! No external deps — the downstream crate (pnr-routing) builds the ledger
//! from its frontend types and hands off to these slots.

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::marker::PhantomData;

use crate::{
    slots::{AllLegal, AlwaysAccept, OverflowStop, StepDecay, UnitWeights},
    Control, Core, CostFn, Density, Domain, Ledger, SplitMix64, Stage, Telemetry,
};

pub const NONE: u32 = u32::MAX;

// ---------------------------------------------------------------------------
// RGraph trait
// ---------------------------------------------------------------------------

/// A routing resource graph with uniform node capacity.
pub trait RGraph {
    fn nodes(&self) -> usize;
    fn cap(&self) -> u16;
    fn neighbors(&self, n: u32, out: &mut [(u32, f32); 6]) -> usize;
    fn pos(&self, n: u32) -> (i32, i32, u32);
    fn region(&self, _n: u32) -> u32 {
        0
    }
}

// ---------------------------------------------------------------------------
// Global: 2-D gcell grid
// ---------------------------------------------------------------------------

pub struct GcellGrid {
    pub nx: u32,
    pub ny: u32,
    pub gw: f32,
    pub gh: f32,
    pub capacity: u16,
}

impl GcellGrid {
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

pub struct TrackGrid {
    pub nx: u32,
    pub ny: u32,
    pub pitch: i32,
    pub via_cost: f32,
    pub allowed: Vec<bool>,
    /// Landing holes punched into blocked cell met1: usable as terminals
    /// (via access) but not as lateral corridors — a met1 wire between two
    /// such nodes crosses foreign in-cell pads the grid can't see.
    pub terminal_only: Vec<bool>,
    pub n_layers: u32,
    /// Per-(iy*nx+ix) gcell region id — `region()` is a table load instead of
    /// div/mod + float math per neighbor expansion in the corridor epochs.
    region_of: Vec<u32>,
}

impl TrackGrid {
    pub fn new(die: (i32, i32), pitch: i32, via_cost: f32) -> Self {
        Self::with_layers(die, pitch, via_cost, 2)
    }

    pub fn with_layers(die: (i32, i32), pitch: i32, via_cost: f32, n_layers: u32) -> Self {
        // Layer order comes from the PDK; even indices route horizontally and
        // odd indices vertically. Only the graph's u32 node-id space limits it.
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

    pub fn set_regions(&mut self, gcells: &GcellGrid) {
        for i in 0..self.layer_size() {
            let (x, y, _) = self.pos(i);
            let rx = ((x as f32 / gcells.gw) as u32).min(gcells.nx - 1);
            let ry = (y as f32 / gcells.gh) as u32;
            self.region_of[i as usize] = ry * gcells.nx + rx;
        }
    }

    pub fn layer_size(&self) -> u32 {
        self.nx * self.ny
    }

    pub fn node(&self, ix: u32, iy: u32, layer: u32) -> u32 {
        layer * self.layer_size() + iy * self.nx + ix
    }

    pub fn nearest(&self, x: i32, y: i32, layer: u32) -> u32 {
        let ix = ((x / self.pitch) as u32).min(self.nx - 1);
        let iy = ((y / self.pitch) as u32).min(self.ny - 1);
        self.node(ix, iy, layer)
    }

    pub fn ixy(&self, n: u32) -> (u32, u32, u32) {
        let layer = n / self.layer_size();
        let r = n % self.layer_size();
        (r % self.nx, r / self.nx, layer)
    }

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

    /// Node ids of one layer whose centers fall inside the box (nm).
    pub fn nodes_in_box(&self, x0: i32, y0: i32, x1: i32, y1: i32, layer: u32) -> Vec<u32> {
        let clamp_x = |v: i32| (v / self.pitch).clamp(0, self.nx as i32 - 1) as u32;
        let clamp_y = |v: i32| (v / self.pitch).clamp(0, self.ny as i32 - 1) as u32;
        let mut out = Vec::new();
        for iy in clamp_y(y0)..=clamp_y(y1) {
            for ix in clamp_x(x0)..=clamp_x(x1) {
                let (px, py, _) = self.pos(self.node(ix, iy, layer));
                if px >= x0 && px <= x1 && py >= y0 && py <= y1 {
                    out.push(self.node(ix, iy, layer));
                }
            }
        }
        out
    }

    pub fn claim(
        &mut self,
        x: i32,
        y: i32,
        claimed: &mut [bool],
        ok: impl Fn(i32, i32) -> bool,
    ) -> Option<u32> {
        self.claim_minarea(x, y, claimed, ok, 0)
    }

    /// Like [`claim`], but also rejects candidates whose landing wire
    /// (from pin to claimed node) would violate `min_area` (nm²).
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
        // Landings only on met1/met2 — geometry emission draws pin stubs on
        // those layers only; met3 (layer 2) is a pure routing layer.
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
                            // min-area check: landing wire from pin to node
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

    /// Return up to `k` candidate nodes for pin access, each with displacement.
    /// Does NOT mark them as claimed — the caller must claim the winner.
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
        // Even layers horizontal (±x), odd layers vertical (±y).
        if layer % 2 == 0 {
            // Terminal-only nodes (met1 landing holes in blocked cell area)
            // get via access only — no lateral wire.
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

// ---------------------------------------------------------------------------
// Routing domain
// ---------------------------------------------------------------------------

pub struct RouteDomain<G>(PhantomData<G>);

impl<G: RGraph> Domain for RouteDomain<G> {
    type Hot = RouteHot;
    type Cold = RouteCtx<G>;
    type Mv = NetRoute;
}

pub struct RouteHot {
    pub usage: Vec<u16>,
    pub hist: Vec<f32>,
    pub trees: Vec<Vec<Vec<u32>>>,
}

impl RouteHot {
    pub fn new(nodes: usize, nets: usize) -> Self {
        Self {
            usage: vec![0; nodes],
            hist: vec![0.0; nodes],
            trees: vec![Vec::new(); nets],
        }
    }

    pub fn tree_nodes(&self, net: usize) -> Vec<u32> {
        let mut v = Vec::new();
        self.tree_nodes_into(net, &mut v);
        v
    }

    /// Buffer-reusing variant for hot paths (propose/commit run per net per epoch).
    pub fn tree_nodes_into(&self, net: usize, out: &mut Vec<u32>) {
        out.clear();
        out.extend(self.trees[net].iter().flatten().copied());
        out.sort_unstable();
        out.dedup();
    }
}

pub struct RouteCtx<G> {
    pub graph: G,
    pub terms: Vec<Vec<u32>>,
    pub net_w: Vec<f32>,
    pub order: Vec<u32>,
    pub corridors: Vec<Vec<u32>>,
    pub reserved: Vec<u32>,
    /// Per-net maximum route length in Dijkstra cost units. `None` = unconstrained.
    pub max_len: Vec<Option<f32>>,
}

#[derive(Default)]
pub struct NetRoute {
    pub net: u32,
    pub branches: Vec<Vec<u32>>,
    pub len: f64,
    pub old_len: f64,
}

// ---------------------------------------------------------------------------
// Congestion-aware multi-source Dijkstra
// ---------------------------------------------------------------------------

pub struct Dij {
    dist: Vec<f32>,
    prev: Vec<u32>,
    seen: Vec<u32>,
    stamp: u32,
    heap: BinaryHeap<Reverse<(u32, u32)>>,
    /// Stamp-based tree membership for the current `route_net` call —
    /// O(1) "is this node already in the tree?" instead of a linear scan.
    in_tree: Vec<u32>,
    tree_stamp: u32,
    /// Stamp-based membership in the net's previous route — O(1) lookup in
    /// `node_cost` instead of a binary search per neighbor expansion.
    is_old: Vec<u32>,
    old_stamp: u32,
}

impl Dij {
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
    // Destructure so `node_cost` (immutable `is_old`) and the search (mutable
    // dist/prev/seen/heap) borrow disjoint fields.
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
    let node_cost = |n: u32| -> f32 {
        let eff = usage[n as usize].saturating_sub(u16::from(is_old[n as usize] == os));
        hist[n as usize] + p_fac * f32::from((eff + 1).saturating_sub(cap))
    };

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
                // Parasitic length constraint: skip nodes beyond budget
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
// Slots
// ---------------------------------------------------------------------------

pub struct TreeLen;
impl<G: RGraph> CostFn<RouteDomain<G>> for TreeLen {
    fn eval(&self, hot: &RouteHot, cold: &RouteCtx<G>) -> f64 {
        hot.trees
            .iter()
            .zip(&cold.net_w)
            .map(|(t, &w)| {
                f64::from(w)
                    * t.iter()
                        .map(|b| b.len().saturating_sub(1) as f64)
                        .sum::<f64>()
            })
            .sum()
    }
    fn delta(&self, _: &RouteHot, cold: &RouteCtx<G>, mv: &NetRoute) -> f64 {
        f64::from(cold.net_w[mv.net as usize]) * (mv.len - mv.old_len)
    }
}

pub struct Overuse;
impl<G: RGraph> Density<RouteDomain<G>> for Overuse {
    fn overflow(&self, hot: &RouteHot, cold: &RouteCtx<G>) -> f32 {
        let cap = cold.graph.cap();
        hot.usage
            .iter()
            .map(|&u| f32::from(u.saturating_sub(cap)))
            .sum()
    }
}

pub struct PathFinder {
    pub p_fac: f32,
    pub hist_inc: f32,
}

pub struct PfScratch {
    dij: Dij,
    cursor: usize,
    epochs: u32,
    /// Reused old-tree node buffer — propose/commit run per net per epoch.
    nodes_buf: Vec<u32>,
}

const CORRIDOR_EPOCHS: u32 = 8;

impl<G: RGraph> Core<RouteDomain<G>> for PathFinder {
    type Scratch = PfScratch;

    fn scratch(&self, _hot: &RouteHot, cold: &RouteCtx<G>) -> PfScratch {
        PfScratch {
            dij: Dij::new(cold.graph.nodes()),
            cursor: 0,
            epochs: 0,
            nodes_buf: Vec::new(),
        }
    }

    fn propose(
        &self,
        hot: &RouteHot,
        cold: &RouteCtx<G>,
        sc: &mut PfScratch,
        _ctl: &Control,
        _rng: &mut SplitMix64,
        mv: &mut NetRoute,
    ) -> bool {
        let cap = cold.graph.cap();
        while sc.cursor < cold.order.len() {
            let net = cold.order[sc.cursor] as usize;
            sc.cursor += 1;
            let dirty = hot.trees[net].is_empty()
                || hot.trees[net]
                    .iter()
                    .flatten()
                    .any(|&n| hot.usage[n as usize] > cap);
            if !dirty || cold.terms[net].is_empty() {
                continue;
            }
            hot.tree_nodes_into(net, &mut sc.nodes_buf);
            let old_len: f64 = hot.trees[net]
                .iter()
                .map(|b| b.len().saturating_sub(1) as f64)
                .sum();
            let corridor: &[u32] = if sc.epochs < CORRIDOR_EPOCHS {
                cold.corridors.get(net).map_or(&[], std::vec::Vec::as_slice)
            } else {
                &[]
            };
            let ml = cold.max_len.get(net).copied().flatten();
            let routed = route_net(
                &cold.graph,
                &hot.usage,
                &hot.hist,
                &sc.nodes_buf,
                &cold.terms[net],
                corridor,
                net as u32,
                &cold.reserved,
                self.p_fac,
                ml,
                &mut sc.dij,
            )
            .or_else(|| {
                (!corridor.is_empty()).then(|| {
                    route_net(
                        &cold.graph,
                        &hot.usage,
                        &hot.hist,
                        &sc.nodes_buf,
                        &cold.terms[net],
                        &[],
                        net as u32,
                        &cold.reserved,
                        self.p_fac,
                        ml,
                        &mut sc.dij,
                    )
                })?
            });
            if let Some((branches, len)) = routed {
                *mv = NetRoute {
                    net: net as u32,
                    branches,
                    len,
                    old_len,
                };
                return true;
            }
        }
        false
    }

    fn commit(
        &self,
        hot: &mut RouteHot,
        _cold: &RouteCtx<G>,
        sc: &mut PfScratch,
        mv: &mut NetRoute,
    ) {
        hot.tree_nodes_into(mv.net as usize, &mut sc.nodes_buf);
        for &n in &sc.nodes_buf {
            hot.usage[n as usize] -= 1;
        }
        // Move the branches in — no clone of the whole route tree.
        hot.trees[mv.net as usize] = std::mem::take(&mut mv.branches);
        hot.tree_nodes_into(mv.net as usize, &mut sc.nodes_buf);
        for &n in &sc.nodes_buf {
            hot.usage[n as usize] += 1;
        }
    }

    fn epoch(&self, hot: &mut RouteHot, cold: &RouteCtx<G>, sc: &mut PfScratch, _t: &Telemetry) {
        let cap = cold.graph.cap();
        for (n, &u) in hot.usage.iter().enumerate() {
            if u > cap {
                hot.hist[n] += self.hist_inc * f32::from(u - cap);
            }
        }
        sc.cursor = 0;
        sc.epochs += 1;
    }
}

// ---------------------------------------------------------------------------
// Stage assembly
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct GlobalRouteCfg {
    pub gcells_per_side: u32,
    pub gcell_capacity: u16,
    pub max_iters: u32,
    pub p_fac: f32,
    pub hist_inc: f32,
}

impl Default for GlobalRouteCfg {
    fn default() -> Self {
        Self {
            gcells_per_side: 16,
            gcell_capacity: 6,
            max_iters: 40,
            p_fac: 3.0,
            hist_inc: 0.5,
        }
    }
}

pub fn run_pathfinder<G: RGraph, Lg: Ledger<RouteDomain<G>>>(
    hot: &mut RouteHot,
    cold: &RouteCtx<G>,
    ledger: &mut Lg,
    p_fac: f32,
    hist_inc: f32,
    max_iters: u32,
    rng: &mut SplitMix64,
) -> Telemetry {
    let stage = Stage {
        cost: TreeLen,
        density: Overuse,
        legality: AllLegal,
        weights: UnitWeights,
        core: PathFinder { p_fac, hist_inc },
        accept: AlwaysAccept,
        schedule: StepDecay {
            step0: 0.0,
            decay: 1.0,
            step_min: 0.0,
            moves_per_step: cold.terms.len().max(1) as u32,
        },
        stop: OverflowStop {
            max_iters,
            min_iters: 1,
            target: 0.0,
        },
        _d: PhantomData,
    };
    stage.run(hot, cold, ledger, rng)
}

pub fn run_global_route(
    hot: &mut RouteHot,
    cold: &RouteCtx<GcellGrid>,
    cfg: &GlobalRouteCfg,
    rng: &mut SplitMix64,
) -> Telemetry {
    run_pathfinder(
        hot,
        cold,
        &mut (),
        cfg.p_fac,
        cfg.hist_inc,
        cfg.max_iters,
        rng,
    )
}

// ---------------------------------------------------------------------------
// Detailed routing config + geometry types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DetailedRouteCfg {
    pub pitch: i32,
    pub wire_width: i32,
    pub via_cost: f32,
    pub max_iters: u32,
    pub p_fac: f32,
    pub hist_inc: f32,
    /// Minimum wire area (nm²); 0 = no enforcement.
    pub min_area: i64,
    /// Landing-claim clearance (nm) from extra obstacles (in-cell met1 pads
    /// the router can't model): obstacle-pad half-width + stub half-width +
    /// met1 spacing. 0 = fall back to `pitch`.
    pub obstacle_clearance: i32,
    /// EOL (end-of-line) spacing (nm); 0 = disabled.
    /// After routing, wire endpoints closer than this to another-net endpoint
    /// trigger a one-shot repair pass.
    pub eol_spacing: i32,
    /// Extra spacing required between wide (power) nets and adjacent signal
    /// nets (nm); 0 = disabled.  Post-route PRL check + selective re-route.
    pub wide_net_extra_spacing: i32,
    /// Number of ordered routing conductors declared by the PDK.
    pub n_layers: u32,
}

impl Default for DetailedRouteCfg {
    fn default() -> Self {
        Self {
            // ponytail: 430/290 keeps met1 DRC-clean by construction
            pitch: 430,
            wire_width: 290,
            via_cost: 4.0,
            max_iters: 150,
            p_fac: 2.0,
            hist_inc: 0.5,
            min_area: 0,
            obstacle_clearance: 0,
            eol_spacing: 0,
            wide_net_extra_spacing: 0,
            n_layers: 2,
        }
    }
}

pub struct Term {
    pub net: u32,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct PinLanding {
    pub net: u32,
    pub pin: (i32, i32),
    pub node: (i32, i32),
    pub layer: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct PinAccessFailure {
    pub net: u32,
    pub pin: (i32, i32),
}

/// Build the track graph: block met1 under every placed cell, then claim one
/// landing node per terminal.
pub fn build_track_grid(
    die: (i32, i32),
    cells: &[(i32, i32, i32, i32)],
    terms: &[Term],
    obstacles: &[(i32, i32)],
    nets: usize,
    cfg: &DetailedRouteCfg,
    protect_pins: bool,
) -> (
    TrackGrid,
    Vec<Vec<u32>>,
    Vec<u32>,
    Vec<PinLanding>,
    Vec<PinAccessFailure>,
    Vec<u32>,
) {
    let mut grid = TrackGrid::with_layers(die, cfg.pitch, cfg.via_cost, cfg.n_layers);
    let halo = if protect_pins { grid.pitch } else { 0 };
    for &(x0, y0, x1, y1) in cells {
        grid.block_met1(x0 - halo, y0 - halo, x1 + halo, y1 + halo);
    }
    let mut claimed = vec![false; grid.nodes()];
    let mut reserved = vec![NONE; grid.nodes()];
    let mut net_terms: Vec<Vec<u32>> = vec![Vec::new(); nets];
    let mut missing = vec![0u32; nets];
    let mut landings = Vec::with_capacity(terms.len());
    let mut failures = Vec::new();

    let min_area = cfg.min_area;

    // --- Pass 1: claim one node per terminal in original order (with min-area). ---
    let oc = if cfg.obstacle_clearance > 0 {
        cfg.obstacle_clearance
    } else {
        cfg.pitch
    };
    for t in terms {
        let pad = cfg.pitch - 140;
        let box_clear = |nx: i32, ny: i32, ox: i32, oy: i32| {
            let p = cfg.pitch;
            ox <= t.x.min(nx) - p
                || ox >= t.x.max(nx) + p
                || oy <= t.y.min(ny) - p
                || oy >= t.y.max(ny) + p
        };
        // Obstacles (in-cell met1 pads) only need pad+stub+spacing clearance
        // from the pin->node corridor — a full-pitch box starves dense cells.
        let obs_clear = |nx: i32, ny: i32, ox: i32, oy: i32| {
            ox <= t.x.min(nx) - oc
                || ox >= t.x.max(nx) + oc
                || oy <= t.y.min(ny) - oc
                || oy >= t.y.max(ny) + oc
        };
        let clear = |nx: i32, ny: i32| {
            !protect_pins
                || (terms.iter().all(|o| {
                    if std::ptr::eq(o, t) {
                        return true;
                    }
                    let p = cfg.pitch;
                    let mate = o.net == t.net && (o.x - t.x).abs() < pad && (o.y - t.y).abs() < pad;
                    if mate {
                        let (dx, dy) = ((nx - o.x).abs(), (ny - o.y).abs());
                        dx.max(dy) >= p || (dx < pad && dy < pad)
                    } else {
                        box_clear(nx, ny, o.x, o.y)
                    }
                }) && obstacles.iter().all(|&(ox, oy)| obs_clear(nx, ny, ox, oy)))
        };
        if let Some(n) = grid.claim_minarea(t.x, t.y, &mut claimed, clear, min_area) {
            reserved[n as usize] = t.net;
            let row = &mut net_terms[t.net as usize];
            if !row.contains(&n) {
                row.push(n);
            }
            let (nx, ny, layer) = grid.pos(n);
            landings.push(PinLanding {
                net: t.net,
                pin: (t.x, t.y),
                node: (nx, ny),
                layer,
            });
        } else {
            missing[t.net as usize] += 1;
            failures.push(PinAccessFailure {
                net: t.net,
                pin: (t.x, t.y),
            });
        }
    }

    // --- Pass 2: multi-candidate refinement for nets with 2+ pins. ---
    // Re-assign pins of multi-pin nets using candidate sets to minimize
    // total displacement and avoid inter-pin conflicts.
    let max_k: usize = 4;
    let mut net_pin_indices: Vec<Vec<usize>> = vec![Vec::new(); nets];
    for (i, t) in terms.iter().enumerate() {
        net_pin_indices[t.net as usize].push(i);
    }

    for net_idx in 0..nets {
        let pin_idxs = &net_pin_indices[net_idx];
        if pin_idxs.len() < 2 {
            continue;
        }

        // Find which landings belong to this net (by matching pin coords).
        let mut landing_idxs: Vec<usize> = Vec::new();
        for &ti in pin_idxs {
            let t = &terms[ti];
            if let Some(li) = landings
                .iter()
                .position(|l| l.net == t.net && l.pin == (t.x, t.y))
            {
                landing_idxs.push(li);
            }
        }
        // Only refine if all pins were claimed in pass 1.
        if landing_idxs.len() != pin_idxs.len() {
            continue;
        }

        // Unclaim current nodes for this net so candidates can see them.
        let mut old_nodes: Vec<u32> = Vec::with_capacity(pin_idxs.len());
        for &li in &landing_idxs {
            let l = &landings[li];
            let n = grid.nearest(l.node.0, l.node.1, l.layer);
            old_nodes.push(n);
            claimed[n as usize] = false;
        }

        // Build per-term clearance closure.
        let make_clear = |ti: usize| {
            let t = &terms[ti];
            let pad = cfg.pitch - 140;
            let oc = if cfg.obstacle_clearance > 0 {
                cfg.obstacle_clearance
            } else {
                cfg.pitch
            };
            move |nx: i32, ny: i32| -> bool {
                let box_clear = |ox: i32, oy: i32| {
                    let p = cfg.pitch;
                    ox <= t.x.min(nx) - p
                        || ox >= t.x.max(nx) + p
                        || oy <= t.y.min(ny) - p
                        || oy >= t.y.max(ny) + p
                };
                let obs_clear = |ox: i32, oy: i32| {
                    ox <= t.x.min(nx) - oc
                        || ox >= t.x.max(nx) + oc
                        || oy <= t.y.min(ny) - oc
                        || oy >= t.y.max(ny) + oc
                };
                !protect_pins
                    || (terms.iter().all(|o| {
                        if std::ptr::eq(o, t) {
                            return true;
                        }
                        let p = cfg.pitch;
                        let mate =
                            o.net == t.net && (o.x - t.x).abs() < pad && (o.y - t.y).abs() < pad;
                        if mate {
                            let (dx, dy) = ((nx - o.x).abs(), (ny - o.y).abs());
                            dx.max(dy) >= p || (dx < pad && dy < pad)
                        } else {
                            box_clear(o.x, o.y)
                        }
                    }) && obstacles.iter().all(|&(ox, oy)| obs_clear(ox, oy)))
            }
        };

        // Collect candidates per pin.
        let mut pin_cands: Vec<Vec<(u32, i32)>> = Vec::with_capacity(pin_idxs.len());
        for &ti in pin_idxs {
            let t = &terms[ti];
            let clear = make_clear(ti);
            pin_cands.push(grid.claim_candidates(t.x, t.y, &claimed, clear, min_area, max_k));
        }

        // Greedy assignment sorted by displacement.
        let mut assignments: Vec<Option<u32>> = vec![None; pin_idxs.len()];
        let mut used: Vec<u32> = Vec::new();
        let mut choices: Vec<(usize, usize, i32)> = Vec::new();
        for (pi, cands) in pin_cands.iter().enumerate() {
            for (ci, &(_node, disp)) in cands.iter().enumerate() {
                choices.push((pi, ci, disp));
            }
        }
        choices.sort_by_key(|&(_, _, d)| d);
        for &(pi, ci, _) in &choices {
            if assignments[pi].is_some() {
                continue;
            }
            let node = pin_cands[pi][ci].0;
            if used.contains(&node) {
                continue;
            }
            assignments[pi] = Some(node);
            used.push(node);
        }

        // Accept only if refinement strictly reduces total displacement.
        let old_disp: i32 = pin_idxs
            .iter()
            .zip(&landing_idxs)
            .map(|(&ti, &li)| {
                let t = &terms[ti];
                let l = &landings[li];
                (t.x - l.node.0).abs() + (t.y - l.node.1).abs()
            })
            .sum();
        let new_disp: i32 = assignments
            .iter()
            .enumerate()
            .map(|(slot, asgn)| match asgn {
                Some(n) => {
                    let t = &terms[pin_idxs[slot]];
                    let (nx, ny, _) = grid.pos(*n);
                    (t.x - nx).abs() + (t.y - ny).abs()
                }
                None => i32::MAX / 4,
            })
            .sum();

        let all_assigned = assignments.iter().all(|a| a.is_some());
        if all_assigned && new_disp < old_disp {
            // Accept: remove old reservations, apply new.
            for &old_n in &old_nodes {
                reserved[old_n as usize] = NONE;
            }
            let row = &mut net_terms[net_idx];
            row.retain(|n| !old_nodes.contains(n));

            for (slot, &ti) in pin_idxs.iter().enumerate() {
                let t = &terms[ti];
                let n = assignments[slot].unwrap();
                claimed[n as usize] = true;
                if !grid.allowed[n as usize] {
                    grid.terminal_only[n as usize] = true;
                }
                grid.allowed[n as usize] = true;
                reserved[n as usize] = t.net;
                if !row.contains(&n) {
                    row.push(n);
                }
                let (nx, ny, layer) = grid.pos(n);
                let li = landing_idxs[slot];
                landings[li] = PinLanding {
                    net: t.net,
                    pin: (t.x, t.y),
                    node: (nx, ny),
                    layer,
                };
            }
        } else {
            // Revert: re-claim old nodes.
            for &old_n in &old_nodes {
                claimed[old_n as usize] = true;
            }
        }
    }

    (grid, net_terms, missing, landings, failures, reserved)
}

pub fn run_detailed_route<Lg: Ledger<RouteDomain<TrackGrid>>>(
    hot: &mut RouteHot,
    cold: &RouteCtx<TrackGrid>,
    ledger: &mut Lg,
    cfg: &DetailedRouteCfg,
    rng: &mut SplitMix64,
) -> Telemetry {
    run_pathfinder(
        hot,
        cold,
        ledger,
        cfg.p_fac,
        cfg.hist_inc,
        cfg.max_iters,
        rng,
    )
}

// ---------------------------------------------------------------------------
// Geometry extraction
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct Wire {
    pub net: u32,
    pub layer: u32,
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
    pub width: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct Via {
    pub net: u32,
    pub x: i32,
    pub y: i32,
    pub size: i32,
    /// Index of the lower conductor in the PDK's ordered routing stack.
    pub layer: u32,
}

pub fn extract_geometry(hot: &RouteHot, grid: &TrackGrid, width: i32) -> (Vec<Wire>, Vec<Via>) {
    extract_geometry_minarea(hot, grid, width, 0)
}

/// Extract wires/vias with optional min-area enforcement.
/// Short wires are extended symmetrically to meet `min_area` (nm²).
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
    // ponytail: extend short wires to meet min-area; pad symmetrically
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
    // Branches sharing a junction each emit the via -> identical stacked pads
    // -> every nearby DRC spacing violation double-counted. One via per site.
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

/// Same-layer center-to-center clearance between two nets' route trees.
/// ponytail: O(|A|·|B|) node scan — analog nets are short.
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
    // min of squared distances == squared min distance — sqrt once at the end.
    if best2 == f32::MAX {
        f32::MAX
    } else {
        best2.sqrt()
    }
}

// ---------------------------------------------------------------------------
// Post-route EOL repair
// ---------------------------------------------------------------------------

/// Scan wire endpoints for EOL spacing violations (two wire tips on different
/// nets facing each other within `eol_spacing`).  Returns the set of grid
/// nodes to block and the nets that need re-routing.
pub fn eol_violations(wires: &[Wire], grid: &TrackGrid, eol_spacing: i32) -> (Vec<u32>, Vec<u32>) {
    // Collect endpoints: (x, y, layer, net)
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
                // Block the grid node nearest to endpoint b (the offender)
                let node = grid.nearest(bx, by, bl);
                blocked_nodes.push(node);
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

// ---------------------------------------------------------------------------
// Post-route PRL (parallel run length) check for wide nets
// ---------------------------------------------------------------------------

/// Identify signal nets running parallel to power-net wires within
/// `extra_spacing`.  Returns grid nodes to block and signal nets to re-route.
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

            // Only check parallel wires
            if pw_horiz != sw_horiz {
                continue;
            }

            if pw_horiz {
                // Both horizontal: check y-distance and x-overlap
                let ydist = (py0 - sy0).abs();
                let overlap = sx1.min(px1) - sx0.max(px0);
                if overlap > 0 && ydist < extra_spacing && ydist > 0 {
                    let node = grid.nearest(sx0, sy0, sw.layer);
                    blocked_nodes.push(node);
                    if !affected_nets.contains(&sw.net) {
                        affected_nets.push(sw.net);
                    }
                }
            } else {
                // Both vertical: check x-distance and y-overlap
                let xdist = (px0 - sx0).abs();
                let overlap = sy1.min(py1) - sy0.max(py0);
                if overlap > 0 && xdist < extra_spacing && xdist > 0 {
                    let node = grid.nearest(sx0, sy0, sw.layer);
                    blocked_nodes.push(node);
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
    use super::{RGraph, TrackGrid};

    #[test]
    fn track_grid_uses_the_entire_declared_stack() {
        let grid = TrackGrid::with_layers((10_000, 10_000), 500, 4.0, 9);
        let top = grid.node(0, 0, 8);

        assert_eq!(grid.n_layers, 9);
        assert_eq!(grid.pos(top).2, 8);
        assert_eq!(grid.nodes(), 9 * grid.layer_size() as usize);
    }
}
