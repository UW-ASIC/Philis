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
/// `wide[ci]` nets (any layer over one track: they claim tracks before the
/// nets that can detour), then the rest under a hard rule, then under a
/// budget, then free nets; within a
/// tier higher impact `weight` first (the most critical claim tracks before
/// the nets that can detour), then fewer terminals, then index. Shield
/// references go last: ROAD builds shields after everything else.
///
/// ponytail: impact is the net's own sensitivity weight, not Lampaert's `F_k`
/// from a pre-route of every net (`Σ_j ΔP_j^k / ΔP_j`); `F_k` waits for
/// RTE-21 step 4.
#[must_use]
pub fn order_by_priority(pins: &[usize], net_ids: &[u32], reqs: &Requirements<Routes>, weight: &[f32], wide: &[bool]) -> Vec<u32> {
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
        let t = if sym.contains(&net) {
            0
        } else if wide.get(ci as usize).copied().unwrap_or(false) {
            1
        } else {
            [&hard, &budget].iter().position(|ids| ids.contains(&net)).map_or(4, |t| t + 2)
        };
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
    /// Appends the along-track nodes a foreign net keeps clear of `tree`'s
    /// run and via ends ([`LayerSpec::halo_wire`], [`LayerSpec::halo_via`]),
    /// unsorted, own metal included. Default: none.
    fn halo_nodes(&self, tree: &[Vec<u32>], out: &mut Vec<u32>) {
        let _ = (tree, out);
    }
    /// Appends the halo a via between `a` and `b` casts (both ends). Default: none.
    fn via_halo(&self, a: u32, b: u32, out: &mut Vec<u32>) {
        let _ = (a, b, out);
    }
    /// A lower bound on any path cost from `a` to `b` (the A* heuristic): it
    /// must never exceed the cheapest edge sum, and must not drop by more than
    /// an edge's base cost across it (consistent). Default: 0 (Dijkstra).
    fn lower_bound(&self, a: u32, b: u32) -> f32 {
        let _ = (a, b);
        0.0
    }
    /// Writes the footprint of a `k`-track connection anchored at `n`: its
    /// `k` across-tracks from `n` toward +, plus `guard` metal-free tracks each
    /// side where in bounds. `false` when a drawn track falls off the graph.
    /// Default: `n` alone.
    fn footprint(&self, n: u32, k: u8, guard: u8, out: &mut Vec<u32>) -> bool {
        let _ = (k, guard);
        out.clear();
        out.push(n);
        true
    }
    /// Appends the corner a via `a`→`b` between a `ka`-track and a `kb`-track
    /// connection covers, on both layers. `false` when it falls off the graph.
    /// Default: none.
    fn via_block(&self, a: u32, b: u32, ka: u8, kb: u8, out: &mut Vec<u32>) -> bool {
        let _ = (a, b, ka, kb, out);
        true
    }
}

/// Most lattice layers a [`LayerSpec`] table describes.
pub const MAX_LAYERS: usize = 8;
/// Coarsest track stride a routed layer may have, in base pitches (tuning
/// default): a layer needing more is not worth its vias (RTE-11 caps the stack).
pub const MAX_STRIDE: u32 = 4;

/// One routed metal on the base-pitch lattice (Hastings Eq 15.19,
/// `P = W_v + 2·E_mv + S_m`: a track pitch clears the via pad, not just the wire).
#[derive(Clone, Debug)]
pub struct LayerSpec {
    pub id: LayerId,
    /// Runs along x (even lattice layers).
    pub horizontal: bool,
    /// A track every `stride` base pitches across the layer's direction.
    pub stride: u32,
    /// One-track wire width, nm.
    pub wire: i32,
    /// Spacing a wire keeps (min spacing incl. EOL), nm.
    pub space: i32,
    /// Via pad sides across / along the wire, nm (largest over the cuts that land here).
    pub pad_across: i32,
    pub pad_along: i32,
    /// Along-track nodes a foreign net keeps clear of a wire end / of a via end.
    pub halo_wire: u8,
    pub halo_via: u8,
}

impl Default for LayerSpec {
    /// Layer 0, stride 1, everything else zero.
    fn default() -> Self {
        Self { id: LayerId(0), horizontal: true, stride: 1, wire: 0, space: 0, pad_across: 0, pad_along: 0, halo_wire: 0, halo_via: 0 }
    }
}

/// The fine track lattice: capacity 1, even layers horizontal, odd vertical,
/// adjacent layers joined by `via_cost` at the same track coordinate. Nodes sit
/// on a `pitch` (base pitch `p0`) grid on every layer; a layer of stride `s`
/// has a track every `s` rows (horizontal) or columns (vertical), and nodes off
/// its tracks get no edges.
pub struct TrackGrid {
    pub nx: u32,
    pub ny: u32,
    pub pitch: i32,
    pub via_cost: f32,
    pub n_layers: u32,
    /// Per layer, empty (uniform: stride 1, no halos) or `n_layers` long.
    pub specs: Vec<LayerSpec>,
    /// `new` raised `pitch` over the one asked for (a die past 1200 pitches).
    pub coarsened: bool,
}

impl TrackGrid {
    /// Track grid over `die` with one layer per spec (one layer when empty);
    /// `p0` is raised to at least `max(die)/1200`.
    #[must_use]
    pub fn new(die: (i32, i32), p0: i32, specs: Vec<LayerSpec>, via_cost: f32) -> Self {
        let pitch = p0.max(die.0.max(die.1) / 1200).max(1);
        let nx = (die.0 / pitch).max(2) as u32;
        let ny = (die.1 / pitch).max(2) as u32;
        let n_layers = (specs.len() as u32).max(1);
        Self { nx, ny, pitch, via_cost, n_layers, specs, coarsened: pitch > p0 }
    }

    /// Uniform track grid over `die` (stride 1 on every layer, no halos).
    #[must_use]
    pub fn with_layers(die: (i32, i32), pitch: i32, via_cost: f32, n_layers: u32) -> Self {
        Self { n_layers: n_layers.max(1), ..Self::new(die, pitch, Vec::new(), via_cost) }
    }

    /// Track stride of `layer`, base pitches (1 without specs).
    #[must_use]
    pub fn stride(&self, layer: u32) -> u32 {
        self.specs.get(layer as usize).map_or(1, |s| s.stride.max(1))
    }

    /// `n` lies on one of its layer's tracks.
    #[must_use]
    pub fn on_track(&self, n: u32) -> bool {
        let (ix, iy, layer) = self.ixy(n);
        (if layer % 2 == 0 { iy } else { ix }) % self.stride(layer) == 0
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
                        if !claimed[n as usize] && !stacked && self.on_track(n) && ok(n) {
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
        if !self.on_track(n) {
            return 0;
        }
        let (ix, iy, layer) = self.ixy(n);
        let mut k = 0;
        let mut push = |ok: bool, m: u32, cost: f32| {
            // Planar steps stay on the track; a via needs a track on both layers.
            if ok && self.on_track(m) {
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
        let s = self.stride(layer);
        let (lo, hi) = if layer % 2 == 0 { (iy >= s, iy + s < self.ny) } else { (ix >= s, ix + s < self.nx) };
        let step = s * if layer % 2 == 0 { self.nx } else { 1 };
        let mut k = 0;
        for (ok, m) in [(lo, n.wrapping_sub(step)), (hi, n + step)] {
            if ok {
                out[k] = m;
                k += 1;
            }
        }
        k
    }
    /// `|Δix| + |Δiy| + via_cost·|Δl|`: every edge costs at least its base
    /// (1 per step, `via_cost` per via) and node costs are ≥ 0.
    fn lower_bound(&self, a: u32, b: u32) -> f32 {
        let ((ax, ay, al), (bx, by, bl)) = (self.ixy(a), self.ixy(b));
        (ax.abs_diff(bx) + ay.abs_diff(by)) as f32 + self.via_cost * al.abs_diff(bl) as f32
    }
    fn footprint(&self, n: u32, k: u8, guard: u8, out: &mut Vec<u32>) -> bool {
        out.clear();
        let (ix, iy, l) = self.ixy(n);
        let s = i64::from(self.stride(l));
        let horiz = l % 2 == 0;
        let (at, lim) = if horiz { (i64::from(iy), i64::from(self.ny)) } else { (i64::from(ix), i64::from(self.nx)) };
        let (k, guard) = (i64::from(k.max(1)), i64::from(guard));
        for j in -guard..k + guard {
            let t = at + j * s;
            if !(0..lim).contains(&t) {
                if (0..k).contains(&j) {
                    return false;
                }
                continue;
            }
            out.push(if horiz { self.node(ix, t as u32, l) } else { self.node(t as u32, iy, l) });
        }
        true
    }
    fn via_block(&self, a: u32, b: u32, ka: u8, kb: u8, out: &mut Vec<u32>) -> bool {
        let (ix, iy, la) = self.ixy(a);
        let lb = self.ixy(b).2;
        // The horizontal layer's tracks step y, the vertical one's x.
        let ((kh, sh), (kv, sv)) = {
            let (ta, tb) = ((ka.max(1), self.stride(la)), (kb.max(1), self.stride(lb)));
            if la % 2 == 0 { (ta, tb) } else { (tb, ta) }
        };
        for l in [la, lb] {
            for i in 0..u32::from(kh) {
                for j in 0..u32::from(kv) {
                    let (x, y) = (ix + j * sv, iy + i * sh);
                    if x >= self.nx || y >= self.ny {
                        return false;
                    }
                    out.push(self.node(x, y, l));
                }
            }
        }
        true
    }
    fn via_halo(&self, a: u32, b: u32, out: &mut Vec<u32>) {
        for v in [a, b] {
            let h = self.specs.get(self.ixy(v).2 as usize).map_or(0, |s| s.halo_via);
            self.along(v, h, -1, out);
            self.along(v, h, 1, out);
        }
    }
    fn halo_nodes(&self, tree: &[Vec<u32>], out: &mut Vec<u32>) {
        if self.specs.is_empty() {
            return;
        }
        let spec = |n: u32| &self.specs[self.ixy(n).2 as usize];
        for branch in tree {
            let mut start = 0;
            for i in 1..=branch.len() {
                if i < branch.len() && self.ixy(branch[i]).2 == self.ixy(branch[i - 1]).2 {
                    continue;
                }
                // `branch[start..i]` is one same-layer run; past each end.
                if i - start >= 2 {
                    let (a, b) = (branch[start], branch[i - 1]);
                    let dir = if b > a { 1 } else { -1 };
                    self.along(a, spec(a).halo_wire, -dir, out);
                    self.along(b, spec(b).halo_wire, dir, out);
                }
                if i < branch.len() {
                    self.via_halo(branch[i - 1], branch[i], out);
                }
                start = i;
            }
        }
    }
}

impl TrackGrid {
    /// Appends `h` along-track nodes from `n` on its layer toward `dir` (−1, +1), in bounds.
    fn along(&self, n: u32, h: u8, dir: i64, out: &mut Vec<u32>) {
        let (ix, iy, l) = self.ixy(n);
        for k in 1..=i64::from(h) {
            let (x, y) = if l % 2 == 0 { (i64::from(ix) + dir * k, i64::from(iy)) } else { (i64::from(ix), i64::from(iy) + dir * k) };
            if x >= 0 && y >= 0 && x < i64::from(self.nx) && y < i64::from(self.ny) {
                out.push(self.node(x as u32, y as u32, l));
            }
        }
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
    /// Per node, how many nets' halos ([`RGraph::halo_nodes`]) cover it: a
    /// halo conflicts only with foreign metal ([`RouteHot::over`]), never with
    /// another halo. `halos[net]` is the net's sorted halo set, own metal excluded.
    pub halo: Vec<u16>,
    pub halos: Vec<Vec<u32>>,
    /// Per net, its sorted, deduped metal footprint (every track of a
    /// `k`-track connection, its via corners and wide-metal guard tracks):
    /// what `usage` and `sens` count.
    pub foot: Vec<Vec<u32>>,
}

impl RouteHot {
    #[must_use]
    pub fn new(nodes: usize, nets: usize) -> Self {
        Self {
            usage: vec![0; nodes],
            hist: vec![0.0; nodes],
            trees: vec![Vec::new(); nets],
            weight: Vec::new(),
            sens: Vec::new(),
            halo: vec![0; nodes],
            halos: vec![Vec::new(); nets],
            foot: vec![Vec::new(); nets],
        }
    }

    /// Overuse at node `n`: `usage + halo − cap` where metal sits, floored at
    /// 0; a node with halos and no metal is not over.
    #[must_use]
    pub fn over(&self, n: usize, cap: u16) -> u16 {
        if self.usage[n] > 0 { (self.usage[n] + self.halo[n]).saturating_sub(cap) } else { 0 }
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

    /// Sorted, deduped node ids of `net`'s footprint ([`RouteHot::foot`]).
    #[must_use]
    pub fn tree_nodes(&self, net: usize) -> Vec<u32> {
        self.foot[net].clone()
    }

    /// Replace `net`'s tree, footprint and halo set, moving their claims with
    /// them; an empty `foot` is the branches' own nodes (one track). Use
    /// [`RouteCtx::commit`], which derives `foot` and `halo` from the graph.
    pub fn commit(&mut self, net: usize, branches: Vec<Vec<u32>>, foot: Vec<u32>, halo: Vec<u32>) {
        let foot = if foot.is_empty() {
            let mut v: Vec<u32> = branches.iter().flatten().copied().collect();
            v.sort_unstable();
            v.dedup();
            v
        } else {
            foot
        };
        for &n in &self.halos[net] {
            self.halo[n as usize] -= 1;
        }
        for &n in &halo {
            self.halo[n as usize] += 1;
        }
        self.halos[net] = halo;
        let w = self.weight.get(net).copied().unwrap_or(0.0);
        for &n in &self.foot[net] {
            self.usage[n as usize] -= 1;
            if w > 0.0 {
                self.sens[n as usize] -= w;
            }
        }
        self.trees[net] = branches;
        self.foot[net] = foot;
        for &n in &self.foot[net] {
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
    /// Per net, tracks per layer of its connections, and wide-metal guard
    /// tracks each side ([`RGraph::footprint`]); empty = 1 track, no guard.
    pub k: Vec<[u8; MAX_LAYERS]>,
    pub guard: Vec<[u8; MAX_LAYERS]>,
    /// Per net, [`NetSearch::term_k`] (parallel to `terms`); empty = none.
    pub term_k: Vec<Vec<[u8; MAX_LAYERS]>>,
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
            k: Vec::new(),
            guard: Vec::new(),
            term_k: Vec::new(),
        }
    }

    /// Tracks of `branch` of `net`: those of the terminal it ends at
    /// ([`route_net`] branches end at their target), else the net's `k`
    /// (a conservative bound for a branch from a tree edit).
    #[must_use]
    pub fn branch_k(&self, net: usize, branch: &[u32]) -> [u8; MAX_LAYERS] {
        let tk = self.term_k.get(net).map_or(&[][..], Vec::as_slice);
        branch
            .last()
            .and_then(|n| self.terms[net].iter().position(|t| t == n))
            .and_then(|i| tk.get(i).copied())
            .unwrap_or_else(|| self.k_of(net).0)
    }

    /// `net`'s tracks and guard tracks per layer.
    fn k_of(&self, net: usize) -> ([u8; MAX_LAYERS], [u8; MAX_LAYERS]) {
        (self.k.get(net).copied().unwrap_or([1; MAX_LAYERS]), self.guard.get(net).copied().unwrap_or([0; MAX_LAYERS]))
    }

    /// Commit `branches` as `net`'s tree with the footprint and halo the
    /// graph gives them at the net's `k` (deduped; the halo spans every
    /// track and leaves out the net's own metal).
    pub fn commit(&self, hot: &mut RouteHot, net: usize, branches: Vec<Vec<u32>>) {
        let (k, guard) = self.k_of(net);
        let layer = |n: u32| self.graph.pos(n).2 as usize;
        let mut foot: Vec<u32> = Vec::new();
        let mut buf = Vec::new();
        if k.iter().any(|&t| t > 1) || guard.iter().any(|&t| t > 0) {
            for b in &branches {
                let k = self.branch_k(net, b);
                for (i, &n) in b.iter().enumerate() {
                    let l = layer(n);
                    if self.graph.footprint(n, k[l], guard[l], &mut buf) {
                        foot.extend_from_slice(&buf);
                    } else {
                        foot.push(n);
                    }
                    // Off the graph: no partial corner, like `footprint`'s fallback.
                    let len = foot.len();
                    if i > 0 && layer(b[i - 1]) != l && !self.graph.via_block(b[i - 1], n, k[layer(b[i - 1])], k[l], &mut foot) {
                        foot.truncate(len);
                    }
                }
            }
            foot.sort_unstable();
            foot.dedup();
        }
        let mut halo = Vec::new();
        self.graph.halo_nodes(&branches, &mut halo);
        if !halo.is_empty() {
            if !foot.is_empty() {
                let raw = std::mem::take(&mut halo);
                for h in raw {
                    if self.graph.footprint(h, k[layer(h)], 0, &mut buf) {
                        halo.extend_from_slice(&buf);
                    } else {
                        halo.push(h);
                    }
                }
            }
            let mut own: Vec<u32> = if foot.is_empty() { branches.iter().flatten().copied().collect() } else { foot.clone() };
            own.sort_unstable();
            halo.sort_unstable();
            halo.dedup();
            halo.retain(|n| own.binary_search(n).is_err());
        }
        hot.commit(net, branches, foot, halo);
    }

    /// Route `net` against the current state with an extra per-node `penalty`
    /// (empty = none).
    pub fn reroute(&self, hot: &RouteHot, net: usize, p_fac: f32, penalty: &[f32], dij: &mut Dij) -> Option<Vec<Vec<u32>>> {
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
        let (k, guard) = self.k_of(net);
        let q = NetSearch {
            net: net as u32,
            terms: &self.terms[net],
            k,
            guard,
            term_k: self.term_k.get(net).map_or(&[][..], Vec::as_slice),
            own: &hot.foot[net],
            own_halo: &hot.halos[net],
            penalty,
            keepout: (&self.keepout, self.own_cells.get(net).map_or(&[][..], Vec::as_slice)),
            elec,
        };
        route_net(&self.graph, hot, &self.reserved, &q, p_fac, dij)
    }
}

/// Stamp-based A* scratch, reused across searches without clearing: one per
/// `dr::route` call.
pub struct Dij {
    /// Heap pops over every search this scratch ran (`RouteStats::expanded`).
    pub pops: u64,
    dist: Vec<f32>,
    prev: Vec<u32>,
    seen: Vec<u32>,
    stamp: u32,
    heap: BinaryHeap<Reverse<(u32, u32)>>,
    in_tree: Vec<u32>,
    is_old: Vec<u32>,
    is_old_halo: Vec<u32>,
}

impl Dij {
    #[must_use]
    pub fn new(nodes: usize) -> Self {
        Self {
            pops: 0,
            dist: vec![0.0; nodes],
            prev: vec![NONE; nodes],
            seen: vec![0; nodes],
            stamp: 0,
            heap: BinaryHeap::new(),
            in_tree: vec![0; nodes],
            is_old: vec![0; nodes],
            is_old_halo: vec![0; nodes],
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

/// One net's search query for [`route_net`].
pub struct NetSearch<'a> {
    pub net: u32,
    /// Terminals; `terms[0]` is the root.
    pub terms: &'a [u32],
    /// Tracks per layer, and wide-metal guard tracks each side ([`RGraph::footprint`]).
    pub k: [u8; MAX_LAYERS],
    pub guard: [u8; MAX_LAYERS],
    /// Per terminal, the tracks of the branch reaching it (parallel to
    /// `terms`; `terms[0]`'s is unused). Non-empty routes the targets in the
    /// given order (no distance sort), each at its own `k`; empty = `k` for all.
    pub term_k: &'a [[u8; MAX_LAYERS]],
    /// The net's current footprint and halo: neither counts against it.
    pub own: &'a [u32],
    pub own_halo: &'a [u32],
    /// Extra cost per node (empty = none).
    pub penalty: &'a [f32],
    /// `(matched cell per node, the net's own cells)`: [`KEEPOUT_COST`] over any other cell.
    pub keepout: (&'a [u32], &'a [u32]),
    pub elec: Elec<'a>,
}

/// Connect `q.terms` into a branch tree by repeated multi-source Dijkstra from
/// the growing tree, targets nearest-first. A node costs, summed over its
/// footprint at the layer's `k` ([`RGraph::footprint`]), `hist + p_fac·overuse`,
/// where the net's own footprint does not count as usage and foreign halos
/// do ([`RouteHot::over`]); plus `penalty + keepout + elec` of the node
/// itself. A via edge adds the same over its corner block
/// ([`RGraph::via_block`]) and `p_fac` per foreign metal node in the halo it
/// casts. `None` if a target is unreachable without a footprint entering a
/// node `reserved` for another net or leaving the graph.
pub fn route_net<G: RGraph>(g: &G, hot: &RouteHot, reserved: &[u32], q: &NetSearch, p_fac: f32, dij: &mut Dij) -> Option<Vec<Vec<u32>>> {
    let (usage, hist, terms, net, elec) = (&hot.usage[..], &hot.hist[..], q.terms, q.net, &q.elec);
    let halo = (&hot.halo[..], q.own_halo);
    let (old_nodes, penalty, keepout) = (q.own, q.penalty, q.keepout);
    let Some(&root) = terms.first() else { return Some(Vec::new()) };
    let Dij { pops, dist, prev, seen, stamp, heap, in_tree, is_old, is_old_halo } = dij;
    // `call` marks old/tree membership for this call; each target search bumps
    // `stamp` again for `seen`. Stamps only grow, so stale marks never match.
    *stamp = stamp.wrapping_add(1);
    let call = *stamp;
    for &n in old_nodes {
        is_old[n as usize] = call;
    }
    for &n in halo.1 {
        is_old_halo[n as usize] = call;
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
    // A via's series R, and the halo it casts on foreign metal at the
    // present-congestion price, charged on the layer change.
    let mut cast = Vec::new();
    let mut via = |a: u32, b: u32| -> f32 {
        let (la, lb) = (g.pos(a).2, g.pos(b).2);
        if la == lb {
            return 0.0;
        }
        cast.clear();
        g.via_halo(a, b, &mut cast);
        let crowd = cast.iter().filter(|&&m| usage[m as usize] > u16::from(is_old[m as usize] == call)).count();
        let r = if elec.current <= 0.0 { 0.0 } else { elec.current * elec.via_r.get(la.min(lb) as usize).copied().unwrap_or(0.0) };
        r + p_fac * crowd as f32
    };
    // Congestion of one footprint node: `None` when another net holds it.
    let congestion = |i: usize| -> Option<f32> {
        if reserved.get(i).is_some_and(|&o| o != NONE && o != net) {
            return None;
        }
        // ponytail: a via's own halo is priced (`via`), a run end's is not;
        // the dirty test reroutes whichever net it lands on.
        let foreign_halo = halo.0.get(i).map_or(0, |&h| h.saturating_sub(u16::from(is_old_halo[i] == call)));
        let eff = usage[i].saturating_sub(u16::from(is_old[i] == call)) + foreign_halo;
        Some(hist[i] + p_fac * f32::from((eff + 1).saturating_sub(cap)))
    };
    let wide_at = |k: &[u8; MAX_LAYERS]| k.iter().any(|&t| t > 1) || q.guard.iter().any(|&t| t > 0);
    let mut fp = Vec::new();
    let mut node_cost = |i: usize, k: &[u8; MAX_LAYERS], wide: bool| -> Option<f32> {
        let mut c = if wide {
            let l = g.pos(i as u32).2 as usize;
            if !g.footprint(i as u32, k[l], q.guard[l], &mut fp) {
                return None;
            }
            fp.iter().try_fold(0.0, |s, &m| Some(s + congestion(m as usize)?))?
        } else {
            congestion(i)?
        };
        let foreign = keepout.0.get(i).is_some_and(|&c| c != NONE && keepout.1.binary_search(&c).is_err());
        c += penalty.get(i).copied().unwrap_or(0.0) + parasitic(i) + if foreign { KEEPOUT_COST } else { 0.0 };
        Some(c)
    };
    // A wide via's corner block (both layers), `None` when illegal.
    let mut block = Vec::new();
    let mut corner = |a: u32, b: u32, k: &[u8; MAX_LAYERS], wide: bool| -> Option<f32> {
        let (la, lb) = (g.pos(a).2 as usize, g.pos(b).2 as usize);
        if !wide || la == lb || (k[la] <= 1 && k[lb] <= 1) {
            return Some(0.0);
        }
        block.clear();
        if !g.via_block(a, b, k[la], k[lb], &mut block) {
            return None;
        }
        block.iter().try_fold(0.0, |s, &m| Some(s + congestion(m as usize)?))
    };

    let (x0, y0, _) = g.pos(root);
    let mut targets: Vec<usize> = (1..terms.len()).collect();
    if q.term_k.is_empty() {
        targets.sort_by_key(|&t| {
            let (x, y, _) = g.pos(terms[t]);
            (x - x0).abs() + (y - y0).abs()
        });
    }
    let mut tree = vec![root];
    let mut branches = vec![vec![root]];
    let mut buf = [(0u32, 0.0f32); 6];
    for t in targets {
        let target = terms[t];
        if in_tree[target as usize] == call {
            continue;
        }
        let tk = q.term_k.get(t).unwrap_or(&q.k);
        let wide = wide_at(tk);
        *stamp = stamp.wrapping_add(1);
        let s = *stamp;
        heap.clear();
        // A*: keyed by `g + h`; a stale entry's key no longer equals the
        // same expression over the node's settled `g`, bit for bit.
        let h = |n: u32| g.lower_bound(n, target);
        for &n in &tree {
            seen[n as usize] = s;
            dist[n as usize] = 0.0;
            prev[n as usize] = NONE;
            heap.push(Reverse((h(n).to_bits(), n)));
        }
        let mut found = false;
        while let Some(Reverse((fb, n))) = heap.pop() {
            *pops += 1;
            let d = dist[n as usize];
            if fb != (d + h(n)).to_bits() {
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
                let (Some(c), Some(v)) = (node_cost(i, tk, wide), corner(n, nb, tk, wide)) else { continue };
                debug_assert!(c >= 0.0 && v >= 0.0, "negative node cost breaks A*");
                let nd = d + base + c + v + via(n, nb);
                if seen[i] != s || nd < dist[i] {
                    seen[i] = s;
                    dist[i] = nd;
                    prev[i] = n;
                    heap.push(Reverse(((nd + h(nb)).to_bits(), nb)));
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
pub fn run_pathfinder<G: RGraph>(hot: &mut RouteHot, cold: &RouteCtx<G>, p_fac: f32, hist_inc: f32, max_iters: u32, dij: &mut Dij) -> (f32, u32) {
    let cap = cold.graph.cap();
    let (mut overflow, mut best, mut stall, mut iters, mut p) = (0.0, f32::INFINITY, 0, 0, p_fac.min(P_FAC_MAX));
    for _ in 0..max_iters {
        iters += 1;
        let mut moved = false;
        for &net in &cold.order {
            let net = net as usize;
            let tree = &hot.trees[net];
            let over = |&n: &u32| hot.over(n as usize, cap) > 0;
            let dirty = tree.is_empty() || tree.iter().flatten().any(over) || hot.halos[net].iter().any(over);
            if !dirty || cold.terms[net].is_empty() {
                continue;
            }
            if let Some(branches) = cold.reroute(hot, net, p, &[], dij) {
                cold.commit(hot, net, branches);
                moved = true;
            }
        }
        overflow = bump_history(&hot.usage, &hot.halo, &mut hot.hist, cap, hist_inc);
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

/// `hist += inc · over` per node ([`RouteHot::over`]; `halo` empty = none);
/// returns `Σ over`.
fn bump_history(usage: &[u16], halo: &[u16], hist: &mut [f32], cap: u16, inc: f32) -> f32 {
    let mut total = 0.0;
    for (i, (h, &u)) in hist.iter_mut().zip(usage).enumerate() {
        let on = if u > 0 { halo.get(i).copied().unwrap_or(0) } else { 0 };
        let over = f32::from((u + on).saturating_sub(cap));
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
    /// Extra width past the +across side (+y on a horizontal track layer, +x
    /// on a vertical one), nm: a merged `k`-track run or via corner.
    pub across: i32,
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

/// Straight runs and vias of every tree, each run at its track layer's
/// `widths` entry (the last for a layer past the end). Single-node runs emit
/// no wire; vias are deduped by `(x, y, layer)` and sized to the lower layer's width.
///
/// A run of a branch on a layer of `k(net, branch)[l] > 1` tracks (stride `s`) is drawn
/// merged, `wire + (k−1)·s·p0` wide toward +across, below the layer's first
/// wide-metal threshold `wide[l]` (`i32::MAX` = none); at or over it, as `k`
/// wires joined by a `wire`-wide cap at each end (parallel routes). A via
/// with either side over one track also gets its corner block on both
/// layers, merged.
///
/// ponytail: the corner block is merged even past the threshold; the
/// search's guard tracks keep foreign metal at the wide spacing.
#[must_use]
pub fn extract_geometry(hot: &RouteHot, grid: &TrackGrid, widths: &[i32], k: &dyn Fn(usize, &[u32]) -> [u8; MAX_LAYERS], wide: &[i32]) -> (Vec<Wire>, Vec<Via>) {
    let width_of = |l: u32| widths.get(l as usize).or(widths.last()).copied().unwrap_or(0);
    let wide_of = |l: u32| wide.get(l as usize).copied().unwrap_or(i32::MAX);
    let step = |l: u32| grid.stride(l) as i32 * grid.pitch;
    let (mut wires, mut vias) = (Vec::new(), Vec::new());
    for (net, tree) in hot.trees.iter().enumerate() {
        let ni = net;
        let net = net as u32;
        for branch in tree {
            let kn = k(ni, branch);
            let kl = |l: u32| i32::from(kn.get(l as usize).copied().unwrap_or(1).max(1));
            let run = |wires: &mut Vec<Wire>, nodes: &[u32], l: u32| {
                emit_run(wires, grid, net, nodes, width_of(l), kl(l), step(l), wide_of(l));
            };
            let mut start = 0;
            for i in 1..branch.len() {
                let (px, py, pl) = grid.pos(branch[i - 1]);
                let (cx, cy, cl) = grid.pos(branch[i]);
                if pl != cl {
                    run(&mut wires, &branch[start..i], pl);
                    let (x, y) = (cx.min(px), cy.min(py));
                    vias.push(Via { net, x, y, size: width_of(pl.min(cl)), layer: pl.min(cl) });
                    if kl(pl) > 1 || kl(cl) > 1 {
                        let (h, v) = if pl % 2 == 0 { (pl, cl) } else { (cl, pl) };
                        let (wh, wv) = (width_of(h), width_of(v));
                        let (sh, sv) = ((kl(h) - 1) * step(h), (kl(v) - 1) * step(v));
                        let w = wh.min(wv);
                        let (bx, by) = (x - wv / 2 + w / 2, y - wh / 2 + w / 2);
                        let at = |layer, x1, y1, across| Wire { net, layer, x0: bx, y0: by, x1, y1, width: w, across };
                        wires.push(at(h, x + sv + wv / 2 - w / 2, by, sh + wh - w));
                        wires.push(at(v, bx, y + sh + wh / 2 - w / 2, sv + wv - w));
                    }
                    start = i;
                }
            }
            if let Some(&last) = branch.get(start) {
                run(&mut wires, &branch[start..], grid.ixy(last).2);
            }
        }
    }
    vias.sort_unstable_by_key(|v| (v.x, v.y, v.layer, v.net));
    vias.dedup_by_key(|v| (v.x, v.y, v.layer));
    (wires, vias)
}

/// Split a same-layer node run into straight wires, `k` tracks `step` apart
/// (see [`extract_geometry`]).
#[allow(clippy::too_many_arguments)]
fn emit_run(wires: &mut Vec<Wire>, grid: &TrackGrid, net: u32, nodes: &[u32], width: i32, k: i32, step: i32, wide: i32) {
    let Some(&first) = nodes.first() else { return };
    let (mut sx, mut sy, layer) = grid.pos(first);
    let horiz = layer % 2 == 0;
    let span = (k - 1) * step;
    let mut push = |(x0, y0): (i32, i32), (x1, y1): (i32, i32)| {
        let wire = |dx, dy, x1, y1, across| Wire { net, layer, x0: x0 + dx, y0: y0 + dy, x1: x1 + dx, y1: y1 + dy, width, across };
        if k <= 1 || width + span < wide {
            wires.push(wire(0, 0, x1, y1, span));
            return;
        }
        for j in 0..k {
            let (dx, dy) = if horiz { (0, j * step) } else { (j * step, 0) };
            wires.push(wire(dx, dy, x1, y1, 0));
        }
        let (cx, cy) = if horiz { (0, span) } else { (span, 0) };
        for (ex, ey) in [(x0, y0), (x1, y1)] {
            wires.push(Wire { net, layer, x0: ex, y0: ey, x1: ex + cx, y1: ey + cy, width, across: 0 });
        }
    };
    let (mut lx, mut ly) = (sx, sy);
    for &n in &nodes[1..] {
        let (x, y, _) = grid.pos(n);
        let straight = if !horiz { x == lx } else { y == ly };
        if !straight {
            push((sx, sy), (lx, ly));
            (sx, sy) = (lx, ly);
        }
        (lx, ly) = (x, y);
    }
    if (lx, ly) != (sx, sy) {
        push((sx, sy), (lx, ly));
    }
}

/// Per-net shapes (layer = track index): a wire is its centre line widened by
/// `width/2` on every side, plus `across` on its +across side; a via is a
/// `size²` square centred on `(x, y)`.
#[must_use]
pub fn to_shapes(nets: usize, wires: &[Wire], vias: &[Via]) -> Vec<Vec<Shape>> {
    let mut out = vec![Vec::new(); nets];
    for w in wires {
        let h = w.width / 2;
        let (x0, y0) = (w.x0.min(w.x1) - h, w.y0.min(w.y1) - h);
        let (x1, y1) = (w.x0.max(w.x1) + h, w.y0.max(w.y1) + h);
        let mut rect = Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
        if w.layer % 2 == 0 {
            rect.h += w.across;
        } else {
            rect.w += w.across;
        }
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
        let (over, iters) = run_pathfinder(&mut hot, &cold, 1.0, 0.5, 150, &mut Dij::new(cold.graph.nodes()));
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
        let (over, iters) = run_pathfinder(&mut hot, &cold, 1.0, 0.5, 150, &mut Dij::new(cold.graph.nodes()));
        assert_eq!(over, 1.0);
        assert!((STALL_ITERS..=STALL_ITERS + 1).contains(&iters), "{iters}");
    }

    /// The same absolute node (−250 nm) seen through two frame origins hits
    /// one history bucket, and not the bucket of +250 nm: a truncating key
    /// (`/`) folds both into bucket 0 and reads the larger 7.0.
    #[test]
    fn history_keys_ignore_the_frame() {
        let mut neg = Negotiation::new();
        neg.accumulate(&[3.0, 7.0], |n| ([250, 750][n as usize] - 500, 250, 0));
        let mut h = [0.0];
        neg.seed(&mut h, |_| (750 - 1_000, 250, 0));
        assert_eq!(h[0], 3.0);
    }

    #[test]
    fn bump_history_matches_definition() {
        let usage = [0u16, 1, 2, 5];
        let mut hist = [0.0f32; 4];
        assert_eq!(bump_history(&usage, &[], &mut hist, 1, 0.5), 5.0);
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
        assert_eq!(order_by_priority(&pins, &[0, 1, 2], &reqs, &[], &[]), vec![0, 1, 2]);
        assert_eq!(order_by_priority(&pins, &[2, 1, 0], &reqs, &[], &[]), vec![1, 2, 0], "the budgeted nets route first");
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
            let mut hot = RouteHot::new(g.nodes(), 1);
            (hot.usage, hot.hist) = (usage.clone(), hist.clone());
            let tree = route_net(&g, &hot, &[], &search(&terms, elec), 1.0, &mut Dij::new(g.nodes())).unwrap();
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
            let mut hot = RouteHot::new(g.nodes(), 1);
            (hot.usage, hot.hist) = (usage.clone(), hist.clone());
            let tree = route_net(&g, &hot, &[], &search(&terms, elec), 1.0, &mut Dij::new(g.nodes())).unwrap();
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
        let order = order_by_priority(&[2; 6], &[0, 1, 2, 3, 4, 5], &reqs, &[0.2, 0.9, 0.0, 0.0, 0.0, 0.0], &[]);
        assert_eq!(order, vec![3, 4, 5, 1, 0, 2], "pair, budgeted victim, free by impact, shield reference");
    }

    /// A wide net claims its tracks after the pair and before the other hard nets.
    #[test]
    fn wide_nets_route_before_other_hard_nets() {
        use analog::routing::{CrosstalkExclusion, Differential};
        let mut reqs = Requirements::<Routes>::default();
        reqs.hard.push(Box::new(vec![Differential { pos: NetId(0), neg: NetId(1), max_len_delta_pct10: 50, same_layer_required: true, stack: None, aggressor_weight: None }]));
        let x = |a, b| CrosstalkExclusion { a: NetId(a), b: NetId(b), min_spacing_nm: 2_000, margin_pct: 0 };
        reqs.hard.push(Box::new(vec![x(2, 3), x(3, 4)]));
        let ids = [0, 1, 2, 3, 4];
        assert_eq!(order_by_priority(&[2; 5], &ids, &reqs, &[], &[false, false, false, true, false]), vec![0, 1, 3, 2, 4]);
        assert_eq!(order_by_priority(&[2; 5], &ids, &reqs, &[], &[false; 5]), vec![0, 1, 2, 3, 4]);
    }

    fn search<'a>(terms: &'a [u32], elec: Elec<'a>) -> NetSearch<'a> {
        NetSearch { net: 0, terms, k: [1; MAX_LAYERS], guard: [0; MAX_LAYERS], term_k: &[], own: &[], own_halo: &[], penalty: &[], keepout: (&[], &[]), elec }
    }

    fn spec(stride: u32, halo_via: u8) -> LayerSpec {
        LayerSpec { stride, halo_via, ..LayerSpec::default() }
    }

    /// Off-track nodes of a stride-2 layer get no edges, and no node has an
    /// off-track neighbour; an on-track layer-2 node still vias down.
    #[test]
    fn off_track_nodes_have_no_edges() {
        let g = TrackGrid::new((10 * 100, 10 * 100), 100, vec![spec(1, 0), spec(1, 0), spec(2, 0)], 4.0);
        let mut buf = [(0u32, 0.0f32); 6];
        for n in 0..g.nodes() as u32 {
            let k = g.neighbors(n, &mut buf);
            assert!(buf[..k].iter().all(|&(m, _)| g.on_track(m)), "node {n}");
            if !g.on_track(n) {
                assert_eq!(k, 0, "off-track node {n}");
            }
        }
        assert!(!g.on_track(g.node(3, 3, 2)) && g.on_track(g.node(3, 4, 2)));
        let n = g.node(3, 4, 2);
        let k = g.neighbors(n, &mut buf);
        assert!(buf[..k].iter().any(|&(m, _)| m == g.node(3, 4, 1)));
    }

    /// `beside` on a stride-2 horizontal layer is two rows over.
    #[test]
    fn beside_steps_by_stride() {
        let g = TrackGrid::new((10 * 100, 10 * 100), 100, vec![spec(1, 0), spec(1, 0), spec(2, 0)], 4.0);
        let n = g.node(5, 4, 2);
        let mut out = [0u32; 2];
        let k = g.beside(n, &mut out);
        assert_eq!(&out[..k], &[n - 2 * g.nx, n + 2 * g.nx]);
    }

    /// A via end casts a one-node halo along both layers' tracks: a foreign
    /// wire there is over capacity, two foreign halos on an empty node are not.
    #[test]
    fn a_via_end_claims_its_halo() {
        let g = TrackGrid::new((10 * 100, 10 * 100), 100, vec![spec(1, 1), spec(1, 1)], 4.0);
        let (a, b) = (g.node(5, 5, 0), g.node(5, 5, 1));
        let want = [g.node(4, 5, 0), g.node(6, 5, 0), g.node(5, 4, 1), g.node(5, 6, 1)];
        let cold = RouteCtx::new(g, vec![Vec::new(); 3], vec![0, 1, 2]);
        let mut hot = RouteHot::new(cold.graph.nodes(), 3);
        cold.commit(&mut hot, 0, vec![vec![a, b]]);
        for n in 0..cold.graph.nodes() {
            assert_eq!(hot.halo[n], u16::from(want.contains(&(n as u32))), "node {n}");
        }
        cold.commit(&mut hot, 1, vec![vec![want[1], cold.graph.node(7, 5, 0)]]);
        assert_eq!(hot.over(want[1] as usize, 1), 1, "foreign metal in the halo");
        // A second foreign halo on (4, 5), which holds no metal.
        let empty = cold.graph.node(4, 5, 0);
        hot.halo[empty as usize] += 1;
        assert_eq!(hot.over(empty as usize, 1), 0, "two halos on an empty node");
    }

    /// A 2-track via at the top edge: its corner block falls off the graph, so
    /// no partial corner (6, 9) joins the footprint, and a halo node whose own
    /// footprint falls off stays as itself.
    #[test]
    fn an_off_graph_corner_or_halo_keeps_one_node() {
        let g = TrackGrid::new((10 * 100, 10 * 100), 100, vec![spec(1, 1), spec(1, 1)], 4.0);
        let (a, b) = (g.node(5, 9, 0), g.node(5, 9, 1));
        let (corner, halo) = (g.node(6, 9, 0), g.node(4, 9, 0));
        let mut cold = RouteCtx::new(g, vec![Vec::new(); 1], vec![0]);
        cold.k = vec![[2; MAX_LAYERS]];
        let mut hot = RouteHot::new(cold.graph.nodes(), 1);
        cold.commit(&mut hot, 0, vec![vec![a, b]]);
        assert!(!hot.foot[0].contains(&corner), "{:?}", hot.foot[0]);
        assert_eq!(hot.halo[halo as usize], 1);
    }

    /// `TrackGrid` without its A* bound: plain Dijkstra over the same graph.
    struct NoBound(TrackGrid);
    impl RGraph for NoBound {
        fn nodes(&self) -> usize {
            self.0.nodes()
        }
        fn cap(&self) -> u16 {
            self.0.cap()
        }
        fn neighbors(&self, n: u32, out: &mut [(u32, f32); 6]) -> usize {
            self.0.neighbors(n, out)
        }
        fn pos(&self, n: u32) -> (i32, i32, u32) {
            self.0.pos(n)
        }
    }

    /// Σ (edge base + history of the node entered) over a tree's branches.
    fn tree_cost(g: &TrackGrid, hist: &[f32], tree: &[Vec<u32>]) -> f32 {
        let mut buf = [(0u32, 0.0f32); 6];
        tree.iter()
            .flat_map(|b| b.windows(2))
            .map(|w| {
                let k = g.neighbors(w[0], &mut buf);
                buf[..k].iter().find(|e| e.0 == w[1]).unwrap().1 + hist[w[1] as usize]
            })
            .sum()
    }

    /// A* finds paths of the same cost as Dijkstra on random history fields.
    #[test]
    fn astar_matches_dijkstra_cost() {
        let mut seed = 0x2545_f491_u64;
        let mut rnd = || {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            (seed >> 33) as u32
        };
        for _ in 0..50 {
            let g = TrackGrid::with_layers((30 * 100, 30 * 100), 100, 4.0, 2);
            let mut hot = RouteHot::new(g.nodes(), 1);
            hot.hist = (0..g.nodes()).map(|_| (rnd() % 2001) as f32 / 1000.0).collect();
            let terms = [g.node(rnd() % 30, rnd() % 30, 0), g.node(rnd() % 30, rnd() % 30, 0)];
            let q = search(&terms, Elec::default());
            let a = route_net(&g, &hot, &[], &q, 1.0, &mut Dij::new(g.nodes())).unwrap();
            let plain = NoBound(TrackGrid::with_layers((30 * 100, 30 * 100), 100, 4.0, 2));
            let d = route_net(&plain, &hot, &[], &q, 1.0, &mut Dij::new(g.nodes())).unwrap();
            let (ca, cd) = (tree_cost(&g, &hot.hist, &a), tree_cost(&g, &hot.hist, &d));
            assert!((ca - cd).abs() < 1e-4 * cd.max(1.0), "A* {ca} vs Dijkstra {cd}");
        }
    }

    /// On an open lattice A* pops at most half the nodes Dijkstra does.
    #[test]
    fn astar_expands_fewer_nodes() {
        let g = TrackGrid::with_layers((200 * 100, 200 * 100), 100, 4.0, 2);
        let hot = RouteHot::new(g.nodes(), 1);
        let terms = [g.node(100, 100, 0), g.node(150, 100, 0)];
        let q = search(&terms, Elec::default());
        let (mut a, mut d) = (Dij::new(g.nodes()), Dij::new(g.nodes()));
        route_net(&g, &hot, &[], &q, 1.0, &mut a).unwrap();
        route_net(&NoBound(TrackGrid::with_layers((200 * 100, 200 * 100), 100, 4.0, 2)), &hot, &[], &q, 1.0, &mut d).unwrap();
        assert!(a.pops * 2 <= d.pops, "A* {} vs Dijkstra {}", a.pops, d.pops);
    }
}
