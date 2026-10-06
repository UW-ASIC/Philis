//! Annealing over hierarchical sequence-pair codes (PLC-09): every state is a
//! [`sp::Tree`] code plus per-cell variant/orient/branch, decoded exactly
//! ([`sp::decode`]), so a placement is always overlap-free, gap-exact and
//! mirror-exact; no projection, no legalizer. A code the decoder rejects is a
//! rejected move ([`PlaceStats::decode_fail`]).

use std::sync::Arc;

use analog::Requirements;
use gp::mechanics::{choose_variants, report, Nets, SplitMix64};
use gp::spacing::Profile;
use pnr_core::{DeviceId, Layout, Macro, Orient, Report, UnitLib, Violation};

use crate::eval::Eval;
use crate::sp::{self, decode, Geo, Out, Scratch, Tree};
use crate::{locks, quarter_turn, seed_branches, PlaceStats, Schedule};

/// Everything [`place_sp`] reads.
pub struct PlaceInput<'a> {
    pub macros: &'a [Macro],
    pub variants: &'a [gp::VariantSpace],
    /// Per cell variant for [`Start::Constructive`] (missing = 0); the other
    /// starts carry their own.
    pub assignment: &'a [u16],
    pub reqs: &'a Requirements<Layout>,
    pub fixed: &'a [bool],
    /// Recognition blocks, glue last ([`sp::Tree::build`]).
    pub blocks: &'a [Vec<DeviceId>],
    pub rules: gp::PlaceRules,
    pub locks: &'a locks::Locks,
    /// Per cell routing halo by face (PLC-15); empty = zero.
    pub halo: &'a [[i32; 4]],
    pub net_weight: &'a [f32],
    pub n_axes: usize,
    /// Per cell, µW; empty = unpowered.
    pub power_uw: &'a [i32],
    pub units: Arc<UnitLib>,
}

/// Where the anneal starts: a coordinate placement's embedding
/// ([`Tree::seed_from`]), the deterministic [`Tree::seed_constructive`], or an
/// incumbent's code.
pub enum Start<'a> {
    Cold(&'a Layout),
    Constructive,
    Warm { tree: &'a Tree, variant: &'a [u16], orient: &'a [Orient] },
}

/// Which detailed placer the flow runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DpMode {
    /// [`crate::place`]: flat SA, projection, legalizer.
    #[default]
    Flat,
    /// [`place_sp`].
    Sp,
}

/// `E = HPWL/L_ref + (A_bb/A_cells − 1) + augmented_cost`, dimensionless:
/// `A_cells = Σ 4·hw·hh = L_ref²`; the area term gives netless fixtures a pull
/// toward a small footprint (BAL1-33). The reference [`Eval::energy`] must
/// reproduce bit for bit (PLC-10).
#[cfg(test)]
fn energy(nets: &Nets, reqs: &Requirements<Layout>, l: &Layout, prices: &gp::Prices) -> f64 {
    let cells: f64 = l.hw.iter().zip(&l.hh).map(|(&w, &h)| 4.0 * f64::from(w) * f64::from(h)).sum();
    let area = if cells > 0.0 { l.footprint_nm2() / cells - 1.0 } else { 0.0 };
    gp::mechanics::hpwl(nets, l) / f64::from(l.l_ref()) + area + gp::mechanics::augmented_cost(reqs, l, prices)
}

/// Two distinct uniform indices below `k` (`k ≥ 2`).
fn two(rng: &mut SplitMix64, k: usize) -> (usize, usize) {
    let i = rng.below(k);
    let j = (i + 1 + rng.below(k - 1)) % k;
    (i, j)
}

/// M1: swap two kids in α; a symmetry node then rewrites β ([`Tree::make_sf`]),
/// which swaps their mates there (Balasa §1.3.3).
fn m1(t: &mut Tree, node: usize, rng: &mut SplitMix64) {
    let (i, j) = two(rng, t.nodes[node].alpha.len());
    t.nodes[node].alpha.swap(i, j);
    t.make_sf(node as u16);
}

/// M2: swap two kids in β; a symmetry node then rewrites α as β reversed
/// through `mate` (the same S-F relation read the other way).
fn m2(t: &mut Tree, node: usize, rng: &mut SplitMix64) {
    let nd = &mut t.nodes[node];
    let (i, j) = two(rng, nd.beta.len());
    nd.beta.swap(i, j);
    if let Some(s) = &nd.sym {
        nd.alpha = nd.beta.iter().rev().map(|&g| s.mate[usize::from(g)]).collect();
    }
}

/// M3: swap two kids in both sequences; in a symmetry node, a pair flip
/// `x ↔ mate[x]` (relabelling by the involution keeps eq 1.1). `false` when
/// the node has no pair.
fn m3(t: &mut Tree, node: usize, rng: &mut SplitMix64) -> bool {
    let nd = &mut t.nodes[node];
    let (a, b) = match &nd.sym {
        Some(s) => {
            let paired: Vec<u16> = (0..s.mate.len() as u16).filter(|&x| s.mate[usize::from(x)] != x).collect();
            if paired.is_empty() {
                return false;
            }
            let x = paired[rng.below(paired.len())];
            (x, s.mate[usize::from(x)])
        }
        None => {
            let (i, j) = two(rng, nd.kids.len());
            (i as u16, j as u16)
        }
    };
    for seq in [&mut nd.alpha, &mut nd.beta] {
        let (i, j) = (seq.iter().position(|&g| g == a).unwrap(), seq.iter().position(|&g| g == b).unwrap());
        seq.swap(i, j);
    }
    true
}

/// Gate key: `(hard batches, Σ hard residual, Θ)`; Θ is `0` outside the quench.
type Key = (usize, f64, f64);

/// Anneal state; `l.variant`/`l.orient`/`l.branch` are the discrete variables.
struct St<'a> {
    inp: &'a PlaceInput<'a>,
    prices: &'a gp::Prices,
    tree: Tree,
    w: Vec<i32>,
    h: Vec<i32>,
    prof: Vec<Option<&'a Profile>>,
    l: Layout,
    nets: Nets,
    cell_nets: Vec<Vec<u32>>,
    scratch: Scratch,
    out: Out,
    /// Undo: the touched node's code, changed cells' `(cell, variant, orient)`, a flipped branch.
    u_node: Option<usize>,
    u_alpha: Vec<u16>,
    u_beta: Vec<u16>,
    u_cells: Vec<(usize, u16, Orient)>,
    u_branch: Option<usize>,
    /// Undo log of the last decode (PLC-10): `(cell, x, y, hw, hh)` it moved, and the axes.
    u_l: Vec<(usize, i32, i32, i32, i32)>,
    s_axis: Vec<i32>,
    /// Cells whose `(x0, y0, w, h, orient, variant)` the last decode changed.
    moved: Vec<usize>,
    eval: Eval,
    /// Nodes with ≥ 2 kids and the running sum of their kid counts.
    pick: Vec<(usize, usize)>,
    branches: Vec<usize>,
    stats: PlaceStats,
}

impl<'a> St<'a> {
    fn macro_of(&self, c: usize, v: u16) -> &'a Macro {
        let inp = self.inp;
        inp.variants.get(c).and_then(|s| s.alternatives.get(usize::from(v))).unwrap_or(&inp.macros[c])
    }

    /// Extents and profile of cell `c` at its current variant and orient.
    fn set_geom(&mut self, c: usize) {
        let inp = self.inp;
        let (v, o) = (self.l.variant[c], self.l.orient[c]);
        let b = self.macro_of(c, v).bbox;
        (self.w[c], self.h[c]) = if o.swaps_axes() { (b.h, b.w) } else { (b.w, b.h) };
        self.prof[c] = inp.rules.profile_of(c, v, o);
    }

    /// Mark the nodes the pending move touched for re-decode (also on undo:
    /// their cached results are then the rejected state's).
    fn mark(&mut self) {
        if let Some(nd) = self.u_node {
            self.scratch.touch(&self.tree, nd as u16);
        }
        for i in 0..self.u_cells.len() {
            self.scratch.touch_cell(&self.tree, self.u_cells[i].0);
        }
    }

    /// Decode the dirty nodes and write the cells that moved into `l`,
    /// logging them ([`St::undo`]) and listing them in `moved`.
    fn realize(&mut self) -> Result<(), sp::Fail> {
        self.mark();
        let g = Geo {
            w: &self.w,
            h: &self.h,
            prof: &self.prof,
            halo: self.inp.halo,
            table: &self.inp.rules.spacing,
            lattice: self.inp.rules.grid,
            axis_grid: None,
        };
        decode(&self.tree, &g, &mut self.scratch, &mut self.out)?;
        self.moved.clear();
        self.u_l.clear();
        let l = &mut self.l;
        for c in 0..self.out.x0.len() {
            let (w2, h2) = (self.w[c] / 2, self.h[c] / 2);
            let v = (self.out.x0[c] + w2, self.out.y0[c] + h2, w2, h2);
            let was = (l.x[c], l.y[c], l.hw[c], l.hh[c]);
            if v != was {
                self.u_l.push((c, was.0, was.1, was.2, was.3));
                (l.x[c], l.y[c], l.hw[c], l.hh[c]) = v;
                self.moved.push(c);
            }
        }
        for &(c, ..) in &self.u_cells {
            if !self.moved.contains(&c) {
                self.moved.push(c);
            }
        }
        self.s_axis.clone_from(&l.axis);
        for &(a, v) in &self.out.axis {
            let a = usize::from(a.0);
            if l.axis.len() <= a {
                l.axis.resize(a + 1, v);
            }
            l.axis[a] = v;
        }
        Ok(())
    }

    /// Re-score the cells the last decode moved ([`Eval::update`]).
    fn rescore(&mut self) {
        let inp = self.inp;
        self.eval.update(&self.moved, self.u_branch.is_some(), inp.reqs, &self.nets, &self.l, self.prices);
    }

    fn rescore_all(&mut self) {
        let inp = self.inp;
        self.eval.full(inp.reqs, &self.nets, &self.l, self.prices);
    }

    fn energy(&self) -> f64 {
        self.eval.energy(&self.l)
    }

    fn key(&self, quench: bool) -> Key {
        let phi = self.eval.phi();
        (phi.0, phi.1, if quench { self.eval.theta() } else { 0.0 })
    }

    fn save_code(&mut self, node: usize) {
        self.u_node = Some(node);
        self.u_alpha.clone_from(&self.tree.nodes[node].alpha);
        self.u_beta.clone_from(&self.tree.nodes[node].beta);
    }

    /// Apply one random move (plan-04 PLC-09 M1–M6) and record its undo.
    /// `false`: nothing applicable was drawn (state untouched).
    fn propose(&mut self, rng: &mut SplitMix64) -> bool {
        self.u_node = None;
        self.u_cells.clear();
        self.u_branch = None;
        let n = self.l.x.len();
        let total = self.pick.last().map_or(0, |p| p.1);
        let node = if total == 0 { None } else {
            let r = rng.below(total);
            self.pick.iter().find(|p| r < p.1).map(|p| p.0)
        };
        let root = usize::from(self.tree.root);
        let root_m1 = |s: &mut Self, rng: &mut SplitMix64| {
            if s.tree.nodes[root].kids.len() < 2 {
                return false;
            }
            s.save_code(root);
            m1(&mut s.tree, root, rng);
            true
        };
        let r = rng.f64();
        match (r, node) {
            (r, Some(nd)) if r < 0.30 => {
                self.save_code(nd);
                m1(&mut self.tree, nd, rng);
                true
            }
            (r, Some(nd)) if r < 0.60 => {
                self.save_code(nd);
                m2(&mut self.tree, nd, rng);
                true
            }
            (r, Some(nd)) if r < 0.75 => {
                self.save_code(nd);
                m3(&mut self.tree, nd, rng) || {
                    self.u_node = None;
                    false
                }
            }
            (r, _) if (0.75..0.83).contains(&r) => {
                let c = rng.below(n);
                let set = self.inp.locks.members(c, false);
                if set.iter().any(|&m| self.inp.fixed.get(m).copied().unwrap_or(false)) {
                    return false;
                }
                let o0 = quarter_turn(self.l.orient[set[0]]);
                for &m in &set {
                    self.u_cells.push((m, self.l.variant[m], self.l.orient[m]));
                    self.l.orient[m] = o0.then(self.inp.locks.rel.get(m).copied().unwrap_or(Orient::R0));
                    self.set_geom(m);
                }
                true
            }
            (r, _) if (0.83..0.90).contains(&r) && self.inp.variants.len() == n => self.reshape(rng.below(n), rng),
            (r, _) if (0.90..0.92).contains(&r) && !self.branches.is_empty() => {
                let b = self.branches[rng.below(self.branches.len())];
                self.l.branch[b] = !self.l.branch[b];
                self.u_branch = Some(b);
                true
            }
            _ => root_m1(self, rng),
        }
    }

    /// M5: the shape set of `c` to one uniformly drawn other variant; pins follow.
    fn reshape(&mut self, c: usize, rng: &mut SplitMix64) -> bool {
        let inp = self.inp;
        let set = inp.locks.members(c, true);
        let depth = inp.variants[c].alternatives.len();
        let fixed = |m: usize| inp.fixed.get(m).copied().unwrap_or(false);
        if depth < 2 || set.iter().any(|&m| fixed(m) || inp.variants[m].alternatives.len() != depth) {
            return false;
        }
        let cur = usize::from(self.l.variant[c]);
        let draw = rng.below(depth - 1);
        let next = if cur < depth && draw >= cur { draw + 1 } else { draw };
        for &m in &set {
            self.u_cells.push((m, self.l.variant[m], self.l.orient[m]));
            self.l.variant[m] = next as u16;
            self.set_geom(m);
            self.nets.reshape_cell(m, &self.cell_nets[m], &inp.variants[m].alternatives[next]);
        }
        true
    }

    /// Revert the last [`St::propose`], its decode and its scores.
    fn undo(&mut self) {
        self.mark();
        if let Some(nd) = self.u_node.take() {
            self.tree.nodes[nd].alpha.clone_from(&self.u_alpha);
            self.tree.nodes[nd].beta.clone_from(&self.u_beta);
        }
        while let Some((c, v, o)) = self.u_cells.pop() {
            if self.l.variant[c] != v {
                let m = self.macro_of(c, v);
                self.nets.reshape_cell(c, &self.cell_nets[c], m);
            }
            (self.l.variant[c], self.l.orient[c]) = (v, o);
            self.set_geom(c);
        }
        if let Some(b) = self.u_branch.take() {
            self.l.branch[b] = !self.l.branch[b];
        }
        let l = &mut self.l;
        for &(c, x, y, hw, hh) in &self.u_l {
            (l.x[c], l.y[c], l.hw[c], l.hh[c]) = (x, y, hw, hh);
        }
        if !self.u_l.is_empty() || !self.moved.is_empty() {
            l.axis.clone_from(&self.s_axis);
        }
        self.u_l.clear();
        self.moved.clear();
        self.eval.revert();
    }

    /// Propose, decode, gate. Returns `Some(ΔE)` when the move decoded.
    fn trial(&mut self, rng: &mut SplitMix64, cur: &mut (Key, f64), temp: f64, quench: bool) -> Option<f64> {
        if !self.propose(rng) {
            return None;
        }
        self.stats.proposals += 1;
        if self.realize().is_err() {
            *self.stats.decode_fail.get_or_insert(0) += 1;
            self.undo();
            return None;
        }
        self.rescore();
        let (k1, e1) = (self.key(quench), self.energy());
        let de = e1 - cur.1;
        let ok = if k1 != cur.0 { k1 < cur.0 } else { de <= 0.0 || (temp > 0.0 && rng.f64() < (-de / temp).exp()) };
        if ok {
            self.stats.accepted += 1;
            *cur = (k1, e1);
            self.eval.commit();
            (self.u_node, self.u_branch) = (None, None);
            self.u_cells.clear();
            self.u_l.clear();
            self.moved.clear();
        } else {
            self.undo();
        }
        Some(de)
    }
}

/// Snapshot of the best state seen, by `(Φ, Θ, E)`.
struct Best {
    key: (Key, f64),
    tree: Tree,
    variant: Vec<u16>,
    orient: Vec<Orient>,
    branch: Vec<bool>,
}

/// The start state, decoded and scored: the tree built on `inp`, coded per
/// `start` (a [`Start::Warm`] whose tree no longer fits the inputs falls back
/// to [`Tree::seed_constructive`] and sets [`PlaceStats::warm_fallback`]).
/// `false` when no seed decodes. Prices must be bound.
fn init<'a>(inp: &'a PlaceInput<'a>, start: &Start, prices: &'a gp::Prices) -> (St<'a>, Vec<sp::TreeError>, bool) {
    let n = inp.macros.len();
    let reqs = inp.reqs;
    let mut pairs = Vec::new();
    for b in &reqs.hard {
        b.mirror_pairs(&mut pairs);
    }
    let (mut tree, errs) = Tree::build(n, &pairs, inp.blocks);
    let warm_ok = match start {
        Start::Warm { tree: t0, variant, orient } => t0.same_structure(&tree) && variant.len() == n && orient.len() == n,
        _ => true,
    };
    let (variant, orient): (Vec<u16>, Vec<Orient>) = match start {
        Start::Cold(l0) => ((0..n).map(|c| l0.variant.get(c).copied().unwrap_or(0)).collect(), (0..n).map(|c| l0.orient.get(c).copied().unwrap_or_default()).collect()),
        Start::Warm { variant, orient, .. } if warm_ok => (variant.to_vec(), orient.to_vec()),
        _ => ((0..n).map(|c| inp.assignment.get(c).copied().unwrap_or(0)).collect(), vec![Orient::R0; n]),
    };
    let mut l = Layout {
        x: vec![0; n],
        y: vec![0; n],
        hw: vec![0; n],
        hh: vec![0; n],
        variant,
        axis: vec![0; inp.n_axes.max(1)],
        branch: vec![false; n],
        groups: (0..n).map(|i| vec![DeviceId(i as u16)]).collect(),
        orient,
        power_uw: if inp.power_uw.len() == n { inp.power_uw.to_vec() } else { vec![0; n] },
        temp_mc: vec![0; n],
        units: inp.units.clone(),
    };
    inp.locks.align(&mut l.orient);
    let branches: Vec<usize> = seed_branches(reqs, &mut l.branch).iter().map(|b| usize::from(b.0)).collect();
    let nets = Nets::from_macros(&choose_variants(inp.macros, inp.variants, &l.variant)).weigh(inp.net_weight);
    let mut pick = Vec::new();
    let mut acc = 0;
    for (i, nd) in tree.nodes.iter().enumerate() {
        if nd.kids.len() >= 2 {
            acc += nd.kids.len();
            pick.push((i, acc));
        }
    }
    match start {
        Start::Cold(l0) => tree.seed_from(l0),
        Start::Warm { tree: t0, .. } if warm_ok => tree.clone_from(t0),
        _ => {}
    }
    let mut st = St {
        inp,
        prices,
        tree,
        w: vec![0; n],
        h: vec![0; n],
        prof: vec![None; n],
        l,
        cell_nets: nets.cell_nets(n),
        eval: Eval::new(reqs, &nets, n),
        nets,
        scratch: Scratch::default(),
        out: Out::default(),
        u_node: None,
        u_alpha: Vec::new(),
        u_beta: Vec::new(),
        u_cells: Vec::new(),
        u_branch: None,
        u_l: Vec::new(),
        s_axis: Vec::new(),
        moved: Vec::new(),
        pick,
        branches,
        stats: PlaceStats {
            matched_incompatible: Some(inp.locks.incompatible),
            decode_fail: Some(0),
            warm_fallback: !warm_ok,
            ..Default::default()
        },
    };
    for c in 0..n {
        st.set_geom(c);
    }
    if matches!(start, Start::Constructive) || !warm_ok {
        st.tree.seed_constructive(&st.w, &st.h);
    }
    if st.realize().is_err() {
        st.tree.seed_constructive(&st.w, &st.h);
        st.scratch.invalidate();
        if st.realize().is_err() {
            return (st, errs, false);
        }
    }
    st.l.refresh_temps();
    st.rescore_all();
    (st, errs, true)
}

/// Detailed placement over hierarchical S-F sequence pairs (plan-04 PLC-09),
/// seed-deterministic. Moves M1–M6 (node ∝ kid count); gate lexicographic on
/// `(hard batches, Σ hard residual)`, then Metropolis on [`energy`]; a final
/// 3-temperature quench gates on `(…, Θ)` too, and the best `(Φ, Θ, E)` state
/// seen is returned, so Θ is never traded for PEX. `T0 = −mean(ΔE⁺)/ln p0`
/// over `32·m` probe moves (`m = Σ kids`; `1e-12` when none is uphill), `T ←
/// alpha·T` after `moves_per_kid·m` moves; stop at `T < 1e-3·T0`, after 5
/// temperatures that improve the best E by ≤ 0.1 %, or at `max_temps`.
/// `stats.temps` counts the temperatures before the quench. Prices are bound
/// once and read only. A cell on two symmetry axes is a hard report row.
pub fn place_sp(inp: &PlaceInput, start: Start, schedule: Schedule, prices: &mut gp::Prices, seed: u64) -> (Layout, Tree, Report, PlaceStats) {
    let reqs = inp.reqs;
    let n = inp.macros.len();
    prices.bind(reqs);
    let prices: &gp::Prices = prices;
    let mut rng = SplitMix64::new(seed);
    let (mut st, errs, ok) = init(inp, &start, prices);
    if !ok {
        // ponytail: never seen (100 % of random S-F codes decode); reported, not hidden.
        let St { l, nets, tree, stats, .. } = st;
        let mut rep = report(&nets, reqs, &l, prices, &inp.rules);
        rep.hard_violations.push(Violation { rule: "sp decode failed".into(), margin: 1 });
        return (l, tree, rep, stats);
    }
    let m: usize = st.tree.nodes.iter().map(|nd| nd.kids.len()).sum();
    let mut cur = (st.key(false), st.energy());
    // T0 from probe moves, each undone.
    let (mut up, mut n_up) = (0.0, 0u32);
    for _ in 0..32 * m {
        if !st.propose(&mut rng) {
            continue;
        }
        if st.realize().is_ok() {
            st.rescore();
            let de = st.energy() - cur.1;
            if de > 0.0 {
                up += de;
                n_up += 1;
            }
        }
        st.undo();
    }
    let t0 = if n_up > 0 { -(up / f64::from(n_up)) / schedule.p0.ln() } else { 1e-12 };
    let n_t = schedule.moves_per_kid as usize * m;
    let mut temp = t0;
    let snap = |st: &St, key| Best { key, tree: st.tree.clone(), variant: st.l.variant.clone(), orient: st.l.orient.clone(), branch: st.l.branch.clone() };
    let qkey = |st: &St| (st.key(true), st.energy());
    let mut best = snap(&st, qkey(&st));
    let mut best_e = cur.1;
    let mut hist: Vec<f64> = Vec::new();
    let mut quench_left = None::<u32>;
    loop {
        let quench = quench_left.is_some();
        if quench {
            cur = qkey(&st);
        }
        for _ in 0..n_t {
            if st.trial(&mut rng, &mut cur, temp, quench).is_some() {
                best_e = best_e.min(cur.1);
                let k = if quench { cur } else { qkey(&st) };
                if k < best.key {
                    best = snap(&st, k);
                }
            }
        }
        // Thermal field is global: refresh per temperature, never per move.
        st.l.refresh_temps();
        st.rescore_all();
        cur = (st.key(quench), st.energy());
        temp *= schedule.alpha;
        match &mut quench_left {
            Some(0) => break,
            Some(q) => *q -= 1,
            None => {
                st.stats.temps += 1;
                hist.push(best_e);
                let t = hist.len() - 1;
                let flat = t >= 5 && (hist[t] - hist[t - 5]).abs() <= 1e-3 * hist[t - 5].abs().max(1e-12);
                if flat || temp < 1e-3 * t0 || st.stats.temps >= schedule.max_temps {
                    quench_left = Some(2);
                }
            }
        }
    }
    if best.key < qkey(&st) {
        st.tree = best.tree;
        (st.l.variant, st.l.orient, st.l.branch) = (best.variant, best.orient, best.branch);
        for c in 0..n {
            st.set_geom(c);
        }
        st.nets = Nets::from_macros(&choose_variants(inp.macros, inp.variants, &st.l.variant)).weigh(inp.net_weight);
        st.scratch.invalidate();
        st.realize().expect("the best state decoded once");
        st.l.refresh_temps();
    }
    let St { l, nets, tree, stats, .. } = st;
    let mut rep = report(&nets, reqs, &l, prices, &inp.rules);
    for e in errs {
        let sp::TreeError::CellInTwoAxes { cell, .. } = e;
        rep.hard_violations.push(Violation { rule: format!("conflicting symmetry cell {cell}"), margin: 1 });
    }
    (l, tree, rep, stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sp::to_layout;
    use gp::mechanics::{analog_phi, analog_theta};
    use analog::placement::symmetry::{SymMode, Symmetry, SymmetryGroup};
    use analog::Rule;
    use pnr_core::geom::Rect;
    use pnr_core::ids::{AxisId, Target};

    fn cell(w: i32, h: i32, pin: Option<u16>) -> Macro {
        let pins = pin.map_or(Vec::new(), |net| {
            vec![pnr_core::Pin {
                name: "P".into(),
                net: pnr_core::NetId(net),
                at: Rect { x: w / 2 - 50, y: h / 2 - 50, w: 100, h: 100 },
                layer: pnr_core::LayerId(0),
            }]
        });
        Macro { pins, bbox: Rect { x: 0, y: 0, w, h }, ..Default::default() }
    }

    fn input<'a>(macros: &'a [Macro], reqs: &'a Requirements<Layout>, locks: &'a locks::Locks, gap: i32) -> PlaceInput<'a> {
        PlaceInput {
            macros,
            variants: &[],
            assignment: &[],
            reqs,
            fixed: &[],
            blocks: &[],
            rules: gp::PlaceRules::uniform(10, gap),
            locks,
            halo: &[],
            net_weight: &[],
            n_axes: 1,
            power_uw: &[],
            units: Arc::default(),
        }
    }

    /// Cells side by side in index order, 2000 nm apart.
    fn row(macros: &[Macro]) -> Layout {
        let n = macros.len();
        Layout {
            x: (0..n as i32).map(|i| 2_000 * i).collect(),
            y: vec![0; n],
            hw: macros.iter().map(|m| m.bbox.w / 2).collect(),
            hh: macros.iter().map(|m| m.bbox.h / 2).collect(),
            variant: vec![0; n],
            axis: vec![0],
            branch: vec![],
            groups: vec![],
            orient: vec![Orient::R0; n],
            power_uw: vec![0; n],
            temp_mc: vec![0; n],
            units: Arc::default(),
        }
    }

    #[test]
    fn sf_moves_preserve_sf() {
        let mut rng = SplitMix64::new(7);
        for _ in 0..10_000 {
            let (p, s) = (1 + rng.below(4), rng.below(3));
            let mut pairs: Vec<(u32, u32, u16)> = (0..p as u32).map(|i| (2 * i, 2 * i + 1, 0)).collect();
            pairs.extend((2 * p..2 * p + s).map(|c| (c as u32, c as u32, 0)));
            let (mut t, _) = Tree::build(2 * p + s, &pairs, &[]);
            match rng.below(3) {
                0 => m1(&mut t, 0, &mut rng),
                1 => m2(&mut t, 0, &mut rng),
                _ => {
                    m3(&mut t, 0, &mut rng);
                }
            }
            assert!(t.is_sf(0), "{:?}", t.nodes[0]);
        }
    }

    fn sym(a: u16, b: u16) -> Symmetry {
        Symmetry { a: Target::Device(DeviceId(a)), b: Target::Device(DeviceId(b)), axis: AxisId(0), mode: SymMode::Perfect }
    }

    #[test]
    fn anneal_is_deterministic() {
        let macros: Vec<Macro> = (0..6).map(|i| cell(1_000 + 200 * (i % 3), 2_000, Some((i % 2) as u16))).collect();
        let reqs = Requirements { hard: vec![Box::new(SymmetryGroup(vec![sym(0, 3)]))], ..Default::default() };
        let lk = locks::locks(&reqs, 6, &[]);
        let inp = input(&macros, &reqs, &lk, 270);
        let coarse = row(&macros);
        let run = || place_sp(&inp, Start::Cold(&coarse), Schedule::cold(), &mut gp::Prices::new(), 3);
        let ((a, ta, ra, sa), (b, tb, _, _)) = (run(), run());
        assert_eq!(ta, tb);
        assert_eq!((&a.variant, &a.orient, &a.x, &a.y), (&b.variant, &b.orient, &b.x, &b.y));
        assert!(ra.hard_violations.is_empty(), "{:?}", ra.hard_violations.iter().map(|v| &v.rule).collect::<Vec<_>>());
        assert!(sa.proposals > 0 && sa.decode_fail.is_some());
    }

    #[derive(Clone, Copy)]
    struct Separation;
    impl Rule for Separation {
        type On = Layout;
        fn cost(self, _: &Layout) -> f32 {
            0.0
        }
        fn residual(self, l: &Layout) -> f32 {
            ((2_000 - (l.x[0] - l.x[1]).abs()).max(0) as f32) / 2_000.0
        }
    }

    /// Cells 0 and 1 share a net (HPWL pulls them together) and a budget wants
    /// their centres ≥ 2000 apart. Start row 0, 2, 1 at gap 0: centres 500 and
    /// 2500, Θ = 0. Θ is unpriced, so the anneal proper may trade it away.
    #[test]
    fn quench_never_returns_worse_theta_than_start() {
        let macros = [cell(1_000, 1_000, Some(0)), cell(1_000, 1_000, Some(0)), cell(1_000, 1_000, None)];
        let reqs = Requirements::<Layout> { budget: vec![Box::new(vec![Separation])], ..Default::default() };
        let lk = locks::locks(&reqs, 3, &[]);
        let inp = input(&macros, &reqs, &lk, 0);
        let mut coarse = row(&macros);
        coarse.x = vec![0, 4_000, 2_000];
        for seed in 1..=5 {
            let (l, ..) = place_sp(&inp, Start::Cold(&coarse), Schedule::cold(), &mut gp::Prices::new(), seed);
            assert!(analog_theta(&reqs, &l) <= 0.0, "seed {seed}: Θ {} at x {:?}", analog_theta(&reqs, &l), l.x);
        }
    }

    #[test]
    fn schedule_stops_early_on_a_flat_landscape() {
        let macros = [cell(1_000, 1_000, None), cell(1_000, 1_000, None)];
        let reqs = Requirements::<Layout>::default();
        let lk = locks::locks(&reqs, 2, &[]);
        let inp = input(&macros, &reqs, &lk, 0);
        let (_, _, _, stats) = place_sp(&inp, Start::Constructive, Schedule::cold(), &mut gp::Prices::new(), 1);
        assert_eq!(stats.temps, 6);
    }

    /// Four 1000 nm squares, every gap 200 (no abutment): α = ABCD, β = CDAB is
    /// a 2×2 block of 2200² nm; α = β = ABCD one row of 4600 × 1000 nm.
    #[test]
    fn area_term_is_bbox_over_cell_area() {
        let e = |alpha: Vec<u16>, beta: Vec<u16>| {
            let (mut t, _) = Tree::build(4, &[], &[]);
            let r = usize::from(t.root);
            (t.nodes[r].alpha, t.nodes[r].beta) = (alpha, beta);
            let table = gp::spacing::SpacingTable::uniform(200, 10);
            let (w, h, prof) = ([1_000; 4], [1_000; 4], [None; 4]);
            let g = Geo { w: &w, h: &h, prof: &prof, halo: &[], table: &table, lattice: 10, axis_grid: None };
            let mut out = Out::default();
            decode(&t, &g, &mut Scratch::default(), &mut out).unwrap();
            let macros = [cell(1_000, 1_000, None), cell(1_000, 1_000, None), cell(1_000, 1_000, None), cell(1_000, 1_000, None)];
            let mut l = row(&macros);
            to_layout(&out, &w, &h, &mut l);
            energy(&Nets::from_macros(&macros), &Requirements::default(), &l, &gp::Prices::new())
        };
        assert!((e(vec![0, 1, 2, 3], vec![2, 3, 0, 1]) - 0.21).abs() < 1e-6);
        assert!((e(vec![0, 1, 2, 3], vec![0, 1, 2, 3]) - 0.15).abs() < 1e-6);
    }

    use analog::placement::utilization::Utilization;
    use analog::placement::{Isolation, Proximity};

    /// 14 cells of three sizes on four nets; Proximity (cost), Isolation and a
    /// global Utilization floor (budget).
    fn bench14() -> (Vec<Macro>, Requirements<Layout>) {
        let macros: Vec<Macro> = (0..14).map(|i| cell(1_000 + 400 * (i % 3), 1_200 + 200 * (i % 2), Some((i % 4) as u16))).collect();
        let d = |i: u16| Target::Device(DeviceId(i));
        let reqs = Requirements::<Layout> {
            budget: vec![
                Box::new(vec![Isolation { a: d(0), b: d(7), min_distance_nm: 6_000 }, Isolation { a: d(3), b: d(9), min_distance_nm: 4_000 }]),
                Box::new(Utilization { u_min: 0.9 }),
            ],
            cost: vec![Box::new(vec![Proximity { a: d(1), b: d(12), max_distance_nm: 2_000 }, Proximity { a: d(5), b: d(6), max_distance_nm: 1_000 }])],
            ..Default::default()
        };
        (macros, reqs)
    }

    fn priced(reqs: &Requirements<Layout>, l: &Layout) -> gp::Prices {
        let mut p = gp::Prices::new();
        p.settle(reqs, l);
        p.settle(reqs, l);
        p
    }

    /// Cell 0's temperature rise, which every powered cell's position sets
    /// (as `MatchedSet`'s `mu_thermal`); it touches only cell 0.
    struct Field;
    impl analog::RuleBatch<Layout> for Field {
        fn cost(&self, l: &Layout) -> f32 {
            l.rise_at_point_mc(l.x[0], l.y[0]) / 1_000.0
        }
        fn violations(&self, _: &Layout) -> u32 {
            0
        }
        fn touched(&self, out: &mut Vec<u32>) {
            out.push(0);
        }
        fn reads_field(&self) -> bool {
            true
        }
    }

    #[test]
    fn incremental_matches_full_evaluation() {
        let (macros, mut reqs) = bench14();
        reqs.cost.push(Box::new(Field));
        let lk = locks::locks(&reqs, 14, &[]);
        let power: Vec<i32> = (0..14).map(|c| if c % 4 == 1 { 20_000 } else { 0 }).collect();
        let inp = PlaceInput { power_uw: &power, ..input(&macros, &reqs, &lk, 270) };
        let coarse = row(&macros);
        let prices = priced(&reqs, &coarse);
        let (mut st, _, ok) = init(&inp, &Start::Cold(&coarse), &prices);
        assert!(ok);
        let nets = Nets::from_macros(&macros);
        let mut rng = SplitMix64::new(11);
        let mut cur = (st.key(true), st.energy());
        let (mut accepted, mut checked) = (0, 0);
        for _ in 0..2_000 {
            let before = st.stats.accepted;
            if st.trial(&mut rng, &mut cur, 0.5, true).is_none() {
                continue;
            }
            accepted += st.stats.accepted - before;
            let full = energy(&nets, &reqs, &st.l, &prices);
            assert!((st.energy() - full).abs() <= 1e-6 * full.abs(), "E_inc {} vs E_full {full}", st.energy());
            assert_eq!((st.eval.phi(), st.eval.theta()), (analog_phi(&reqs, &st.l), analog_theta(&reqs, &st.l)));
            checked += 1;
        }
        assert!(accepted > 100 && checked > accepted, "accepted {accepted} of {checked}");
    }

    /// Profiles that differ by face and orient (an n-well on the R0 right face,
    /// three bulk nets), so a turn changes gaps and a stale cached gap shows.
    #[test]
    fn cached_decode_equals_fresh_decode() {
        use gp::spacing::{Profile, Profiles, SpacingTable};
        let (macros, reqs) = bench14();
        let lk = locks::locks(&reqs, 14, &[]);
        let mut inp = input(&macros, &reqs, &lk, 270);
        let mut table = SpacingTable::uniform(270, 10);
        table.rule[0][0] = 1_270; // ROLES[0] = "nwell"
        let prof = |net: u16| {
            let mut p = Profile { well_net: Some(pnr_core::NetId(net)), ..Profile::default() };
            p.edge[2].put(0, 0);
            p
        };
        inp.rules = gp::PlaceRules::new(10, table, Profiles { of: (0..14).map(|c| vec![Profiles::orients(&prof(c % 3))]).collect() });
        let coarse = row(&macros);
        let prices = gp::Prices::new();
        let (mut st, _, _) = init(&inp, &Start::Cold(&coarse), &prices);
        let mut rng = SplitMix64::new(5);
        let mut cur = (st.key(false), st.energy());
        for _ in 0..1_000 {
            st.trial(&mut rng, &mut cur, 0.05, false);
            let g = Geo { w: &st.w, h: &st.h, prof: &st.prof, halo: &[], table: &st.inp.rules.spacing, lattice: 10, axis_grid: None };
            let mut fresh = Out::default();
            decode(&st.tree, &g, &mut Scratch::default(), &mut fresh).unwrap();
            let x: Vec<i32> = (0..14).map(|c| fresh.x0[c] + st.w[c] / 2).collect();
            let y: Vec<i32> = (0..14).map(|c| fresh.y0[c] + st.h[c] / 2).collect();
            assert_eq!((&x, &y), (&st.l.x, &st.l.y));
        }
    }

    #[test]
    fn warm_start_at_zero_temperature_never_worsens_the_incumbent_key() {
        let (macros, reqs) = bench14();
        let lk = locks::locks(&reqs, 14, &[]);
        let inp = input(&macros, &reqs, &lk, 270);
        let coarse = row(&macros);
        let nets = Nets::from_macros(&macros);
        let key = |l: &Layout| {
            let phi = analog_phi(&reqs, l);
            (phi.0, phi.1, analog_theta(&reqs, l), energy(&nets, &reqs, l, &gp::Prices::new()))
        };
        let (l0, t0, ..) = place_sp(&inp, Start::Cold(&coarse), Schedule::cold(), &mut gp::Prices::new(), 1);
        // p0 = 0 ⇒ T0 = 0: only downhill moves pass Metropolis.
        let zero = Schedule { max_temps: 1, p0: 0.0, ..Schedule::warm() };
        for seed in 2..=4 {
            let warm = Start::Warm { tree: &t0, variant: &l0.variant, orient: &l0.orient };
            let (l1, _, _, st) = place_sp(&inp, warm, zero, &mut gp::Prices::new(), seed);
            assert!(!st.warm_fallback);
            assert!(key(&l1) <= key(&l0), "seed {seed}: {:?} > {:?}", key(&l1), key(&l0));
        }
    }

    #[test]
    fn warm_start_falls_back_on_changed_groups() {
        let macros: Vec<Macro> = (0..6).map(|_| cell(1_000, 1_000, None)).collect();
        let reqs = Requirements { hard: vec![Box::new(SymmetryGroup(vec![sym(0, 1), sym(2, 3), sym(4, 5)]))], ..Default::default() };
        let lk = locks::locks(&reqs, 6, &[]);
        let inp = input(&macros, &reqs, &lk, 0);
        let (t2, _) = Tree::build(6, &[(0, 1, 0), (2, 3, 0)], &[]);
        let warm = Start::Warm { tree: &t2, variant: &[0; 6], orient: &[Orient::R0; 6] };
        let (_, t, rep, st) = place_sp(&inp, warm, Schedule::warm(), &mut gp::Prices::new(), 1);
        assert!(st.warm_fallback);
        assert_ne!(t.nodes[0].kids.len(), t2.nodes[0].kids.len());
        assert!(rep.hard_violations.is_empty());
    }
}
