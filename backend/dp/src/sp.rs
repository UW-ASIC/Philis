//! Hierarchical symmetric-feasible sequence pair (PLC-08; Balasa §1.3, Plantage
//! Alg. 3.3 for shadows). Every symmetry group and proximity block is a node
//! whose kids are contiguous in its parent's code, so no outsider enters its
//! bbox. [`decode`] turns a code into lattice coordinates with exact pairwise
//! gaps (PLC-07 [`Gap`]) and exact mirror equations, or says why it cannot.
//!
//! Invariant: a node's node kids have smaller indices than it (root last), so
//! index order is a post-order and reverse index order a pre-order.

use gp::spacing::{Face, Gap, Profile, SpacingTable};
use pnr_core::ids::AxisId;
use pnr_core::{DeviceId, Layout};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kid {
    Cell(u16),
    Node(u16),
}

/// A symmetry node's axis and involution: `mate[k]` = partner kid slot,
/// `mate[k] == k` = self-symmetric (centred on the axis).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sym {
    pub axis: AxisId,
    pub mate: Vec<u16>,
}

/// `alpha`, `beta`: permutations of kid slots (the node's sequence pair).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub kids: Vec<Kid>,
    pub alpha: Vec<u16>,
    pub beta: Vec<u16>,
    pub sym: Option<Sym>,
}

/// `home[cell] = (node, kid slot)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tree {
    pub nodes: Vec<Node>,
    pub root: u16,
    pub home: Vec<(u16, u16)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeError {
    /// `cell` is mirrored on axis `a` and again on `b`; `b`'s group was dropped.
    CellInTwoAxes { cell: u16, a: AxisId, b: AxisId },
}

/// Inverse permutation: `inv[seq[p]] = p`.
fn inverse(seq: &[u16], inv: &mut Vec<u32>) {
    inv.clear();
    inv.resize(seq.len(), 0);
    for (p, &k) in seq.iter().enumerate() {
        inv[usize::from(k)] = p as u32;
    }
}

impl Tree {
    /// Symmetry nodes from `pairs` (`(a, b, axis)`, `a == b` = self) grouped by
    /// axis in first-appearance order; then one proximity node per block of
    /// `blocks` but the last (the annotator's glue) holding ≥ 2 kids: its whole
    /// symmetry nodes and its remaining cells; the root holds the rest. A cell
    /// named on a second axis drops that axis's whole group and is reported. A
    /// cell paired twice on one axis keeps its first partner. Every node starts
    /// with α = β = slot order, symmetry nodes then [`Tree::make_sf`].
    #[must_use]
    pub fn build(n_cells: usize, pairs: &[(u32, u32, u16)], blocks: &[Vec<DeviceId>]) -> (Tree, Vec<TreeError>) {
        let n = n_cells;
        let ok = |p: &&(u32, u32, u16)| (p.0 as usize) < n && (p.1 as usize) < n;
        let mut axes: Vec<u16> = Vec::new();
        for p in pairs.iter().filter(ok) {
            if !axes.contains(&p.2) {
                axes.push(p.2);
            }
        }
        let mut errs = Vec::new();
        let mut axis_of: Vec<Option<u16>> = vec![None; n];
        let mut sym_of: Vec<Option<u16>> = vec![None; n];
        let mut nodes: Vec<Node> = Vec::new();
        for &ax in &axes {
            let group: Vec<(u16, u16)> = pairs.iter().filter(ok).filter(|p| p.2 == ax).map(|p| (p.0 as u16, p.1 as u16)).collect();
            let mut clash = false;
            for c in group.iter().flat_map(|&(a, b)| [a, b]) {
                if let Some(o) = axis_of[usize::from(c)].filter(|&o| o != ax) {
                    clash = true;
                    let e = TreeError::CellInTwoAxes { cell: c, a: AxisId(o), b: AxisId(ax) };
                    if !errs.contains(&e) {
                        errs.push(e);
                    }
                }
            }
            if clash {
                continue;
            }
            let (mut cells, mut mate): (Vec<u16>, Vec<Option<u16>>) = (Vec::new(), Vec::new());
            let mut slot = |c: u16, mate: &mut Vec<Option<u16>>| {
                cells.iter().position(|&x| x == c).unwrap_or_else(|| {
                    cells.push(c);
                    mate.push(None);
                    cells.len() - 1
                })
            };
            for &(a, b) in &group {
                let (sa, sb) = (slot(a, &mut mate), slot(b, &mut mate));
                if mate[sa].is_none() && mate[sb].is_none() {
                    (mate[sa], mate[sb]) = (Some(sb as u16), Some(sa as u16));
                }
            }
            let id = nodes.len() as u16;
            for &c in &cells {
                axis_of[usize::from(c)] = Some(ax);
                sym_of[usize::from(c)] = Some(id);
            }
            let k = cells.len() as u16;
            nodes.push(Node {
                kids: cells.iter().map(|&c| Kid::Cell(c)).collect(),
                alpha: (0..k).collect(),
                beta: (0..k).collect(),
                sym: Some(Sym { axis: AxisId(ax), mate: mate.iter().enumerate().map(|(s, m)| m.unwrap_or(s as u16)).collect() }),
            });
        }
        let n_sym = nodes.len();
        let mut used = vec![false; n_sym];
        let mut homed = vec![false; n];
        let mut in_blk = vec![false; n];
        for blk in &blocks[..blocks.len().saturating_sub(1)] {
            in_blk.fill(false);
            for d in blk.iter().filter(|d| usize::from(d.0) < n) {
                in_blk[usize::from(d.0)] = true;
            }
            let mut kids = Vec::new();
            for (s, node) in nodes.iter().enumerate().take(n_sym) {
                let all = node.kids.iter().all(|k| matches!(k, Kid::Cell(c) if in_blk[usize::from(*c)]));
                if !used[s] && all {
                    kids.push(Kid::Node(s as u16));
                }
            }
            for c in (0..n).filter(|&c| in_blk[c] && sym_of[c].is_none() && !homed[c]) {
                kids.push(Kid::Cell(c as u16));
            }
            if kids.len() < 2 {
                continue;
            }
            for k in &kids {
                match *k {
                    Kid::Node(s) => used[usize::from(s)] = true,
                    Kid::Cell(c) => homed[usize::from(c)] = true,
                }
            }
            let k = kids.len() as u16;
            nodes.push(Node { kids, alpha: (0..k).collect(), beta: (0..k).collect(), sym: None });
        }
        let mut kids: Vec<Kid> = (n_sym..nodes.len()).map(|i| Kid::Node(i as u16)).collect();
        kids.extend((0..n_sym).filter(|&s| !used[s]).map(|s| Kid::Node(s as u16)));
        kids.extend((0..n).filter(|&c| sym_of[c].is_none() && !homed[c]).map(|c| Kid::Cell(c as u16)));
        let k = kids.len() as u16;
        nodes.push(Node { kids, alpha: (0..k).collect(), beta: (0..k).collect(), sym: None });
        let mut home = vec![(0, 0); n];
        for (ni, node) in nodes.iter().enumerate() {
            for (s, k) in node.kids.iter().enumerate() {
                if let Kid::Cell(c) = *k {
                    home[usize::from(c)] = (ni as u16, s as u16);
                }
            }
        }
        let root = (nodes.len() - 1) as u16;
        let mut t = Tree { nodes, root, home };
        for s in 0..n_sym {
            t.make_sf(s as u16);
        }
        (t, errs)
    }

    /// Same nodes, kids, axes and mates (codes may differ): a warm start's
    /// tree still fits the inputs (PLC-10).
    #[must_use]
    pub fn same_structure(&self, o: &Tree) -> bool {
        self.root == o.root
            && self.nodes.len() == o.nodes.len()
            && self.nodes.iter().zip(&o.nodes).all(|(a, b)| a.kids == b.kids && a.sym == b.sym)
    }

    /// Balasa eq 1.1 for distinct kids `x`, `y` of a symmetry node:
    /// `(pa[x] < pa[y]) == (pb[mate[y]] < pb[mate[x]])`. `true` off symmetry nodes.
    #[must_use]
    pub fn is_sf(&self, node: u16) -> bool {
        let nd = &self.nodes[usize::from(node)];
        let Some(s) = &nd.sym else { return true };
        let (mut pa, mut pb) = (Vec::new(), Vec::new());
        inverse(&nd.alpha, &mut pa);
        inverse(&nd.beta, &mut pb);
        let m = |k: usize| usize::from(s.mate[k]);
        let k = nd.kids.len();
        (0..k).all(|x| (0..k).all(|y| x == y || (pa[x] < pa[y]) == (pb[m(y)] < pb[m(x)])))
    }

    /// Rewrite a symmetry node's β as its α reversed through `mate`
    /// (Balasa L2389–2390); then [`Tree::is_sf`] holds. No-op elsewhere.
    pub fn make_sf(&mut self, node: u16) {
        let nd = &mut self.nodes[usize::from(node)];
        if let Some(s) = &nd.sym {
            nd.beta = nd.alpha.iter().rev().map(|&g| s.mate[usize::from(g)]).collect();
        }
    }

    /// Embed a coordinate placement: per node, α by `(x − y, slot)` and β by
    /// `(x + y, slot)` of each kid's key point (cell centre, mean centre of a
    /// node's cells), then [`Tree::make_sf`].
    pub fn seed_from(&mut self, l: &Layout) {
        let mut key: Vec<(i64, i64, i64)> = vec![(0, 0, 0); self.nodes.len()];
        for ni in 0..self.nodes.len() {
            let pts: Vec<(i64, i64, i64)> = self.nodes[ni]
                .kids
                .iter()
                .map(|k| match *k {
                    Kid::Cell(c) => (i64::from(l.x[usize::from(c)]), i64::from(l.y[usize::from(c)]), 1),
                    Kid::Node(m) => key[usize::from(m)],
                })
                .collect();
            key[ni] = pts.iter().fold((0, 0, 0), |a, p| (a.0 + p.0, a.1 + p.1, a.2 + p.2));
            let at = |s: u16| {
                let (x, y, c) = pts[usize::from(s)];
                (x / c.max(1), y / c.max(1))
            };
            let nd = &mut self.nodes[ni];
            nd.alpha.sort_by_key(|&s| (at(s).0 - at(s).1, s));
            nd.beta.sort_by_key(|&s| (at(s).0 + at(s).1, s));
            self.make_sf(ni as u16);
        }
    }

    /// Deterministic start: symmetry nodes take Balasa's initial code
    /// `α = a_1…a_p c_1…c_s b_p…b_1` (L2375–2405; `a` the lower slot of each
    /// pair), every other node one row α = β by descending kid area (`w·h`
    /// summed over a node's cells), ties by slot.
    pub fn seed_constructive(&mut self, w: &[i32], h: &[i32]) {
        let mut area: Vec<i64> = vec![0; self.nodes.len()];
        for ni in 0..self.nodes.len() {
            let ka: Vec<i64> = self.nodes[ni]
                .kids
                .iter()
                .map(|k| match *k {
                    Kid::Cell(c) => i64::from(w[usize::from(c)]) * i64::from(h[usize::from(c)]),
                    Kid::Node(m) => area[usize::from(m)],
                })
                .collect();
            area[ni] = ka.iter().sum();
            let nd = &mut self.nodes[ni];
            let k = nd.kids.len() as u16;
            if let Some(s) = &nd.sym {
                let a: Vec<u16> = (0..k).filter(|&x| s.mate[usize::from(x)] > x).collect();
                let c = (0..k).filter(|&x| s.mate[usize::from(x)] == x);
                nd.alpha = a.iter().copied().chain(c).chain(a.iter().rev().map(|&x| s.mate[usize::from(x)])).collect();
                self.make_sf(ni as u16);
            } else {
                nd.alpha = (0..k).collect();
                nd.alpha.sort_by_key(|&x| (-ka[usize::from(x)], x));
                nd.beta.clone_from(&nd.alpha);
            }
        }
    }
}

/// Per cell, for its current (variant, orient): extents (even, nm), oriented
/// profile (`None` = spaced at `table.fallback`), routing halo per [`Face`]
/// (empty or short = zero; PLC-15). `axis_grid = Some((p0, P))` puts every axis
/// on a routing track centreline (PLC-28).
pub struct Geo<'a> {
    pub w: &'a [i32],
    pub h: &'a [i32],
    pub prof: &'a [Option<&'a Profile>],
    pub halo: &'a [[i32; 4]],
    pub table: &'a SpacingTable,
    pub lattice: i32,
    pub axis_grid: Option<(i32, i32)>,
}

/// Decoded cell lower-left corners, one `(axis, x)` per symmetry node in node
/// order, and how many `fix_monotone` passes the decode needed.
#[derive(Default, Debug)]
pub struct Out {
    pub x0: Vec<i32>,
    pub y0: Vec<i32>,
    pub axis: Vec<(AxisId, i32)>,
    pub fixes: u32,
}

/// Why a code has no realisation; payload: node index.
#[derive(Debug, PartialEq, Eq)]
pub enum Fail {
    SymY(u16),
    SymX(u16),
    Band(u16),
}

/// Reused buffers so a decode allocates only on growth, and each node's last
/// result: [`decode`] re-decodes only nodes [`Scratch::touch`] marked (PLC-10).
#[derive(Default)]
pub struct Scratch {
    /// Per node: needs a re-decode; parent node (empty = nothing cached yet).
    dirty: Vec<bool>,
    parent: Vec<u16>,
    pa: Vec<u32>,
    pb: Vec<u32>,
    /// Per node, per kid slot: position relative to the node's origin.
    rel_x: Vec<Vec<i32>>,
    rel_y: Vec<Vec<i32>>,
    lb: Vec<i32>,
    lby: Vec<i32>,
    /// Per node: merged profile (`None` when a member has none), halo, `(W, H)`, `2·axis` relative, origin.
    node_prof: Vec<Option<Profile>>,
    node_halo: Vec<[i32; 4]>,
    dim: Vec<(i32, i32)>,
    ax2: Vec<i32>,
    org: Vec<(i32, i32)>,
    /// Current node's kids: extents, profile, halo, is-a-node; gap cache `[R, T]` by `i·k + j`.
    kw: Vec<i32>,
    kh: Vec<i32>,
    kp: Vec<Option<Profile>>,
    khalo: Vec<[i32; 4]>,
    knode: Vec<bool>,
    gc: Vec<[Option<Gap>; 2]>,
}

impl Scratch {
    /// Mark `node` and its ancestors for re-decode: its code, or a kid's
    /// extents or profile, changed (or changed back).
    pub fn touch(&mut self, t: &Tree, node: u16) {
        if self.parent.len() != t.nodes.len() {
            return;
        }
        let mut n = usize::from(node);
        loop {
            self.dirty[n] = true;
            if n == usize::from(t.root) {
                break;
            }
            n = usize::from(self.parent[n]);
        }
    }

    /// Drop every cached node result (a new tree or a wholesale code change).
    pub fn invalidate(&mut self) {
        self.parent.clear();
    }
}

/// `table.gap` when both profiles exist, else `fallback` (as `PlaceRules::gaps`).
fn gap(g: &Geo, pi: Option<&Profile>, f: Face, pj: Option<&Profile>) -> Gap {
    match (pi, pj) {
        (Some(a), Some(b)) => g.table.gap(a, f, b),
        _ => Gap { abut: false, min: g.table.fallback },
    }
}

/// A separation `s` satisfies `gp` plus `halo`: abutment (`s == 0`, no halo) or `s ≥ min + halo`.
fn sep_ok(s: i32, gp: Gap, halo: i32) -> bool {
    (gp.abut && halo == 0 && s == 0) || s >= gp.min + halo
}

/// The longest-path edge weight: `0` when abutment is allowed, else `min + halo`.
fn need(gp: Gap, halo: i32) -> i32 {
    if gp.abut && halo == 0 { 0 } else { gp.min + halo }
}

fn round_up(v: i32, q: i32) -> i32 {
    let q = q.max(1);
    v.div_euclid(q) * q + if v.rem_euclid(q) == 0 { 0 } else { q }
}

/// Kid boxes of one node (relative or absolute), with the geometry the gaps read.
struct Kids<'s> {
    x: &'s [i32],
    y: &'s [i32],
    w: &'s [i32],
    h: &'s [i32],
    p: &'s [Option<Profile>],
    halo: &'s [[i32; 4]],
    node: &'s [bool],
}

impl Kids<'_> {
    /// Gap kid `i` owes `j` across `i`'s face `R` (`ax = 0`) or `T` (`ax = 1`),
    /// cached. A node kid never abuts: its merged profile hides deeper members.
    fn gap(&self, g: &Geo, gc: &mut [[Option<Gap>; 2]], i: usize, j: usize, ax: usize) -> (Gap, i32) {
        let k = self.x.len();
        let gp = *gc[i * k + j][ax].get_or_insert_with(|| {
            let mut gp = gap(g, self.p[i].as_ref(), [Face::R, Face::T][ax], self.p[j].as_ref());
            gp.abut &= !self.node[i] && !self.node[j];
            gp
        });
        let halo = if ax == 0 { self.halo[i][2] + self.halo[j][0] } else { self.halo[i][3] + self.halo[j][1] };
        (gp, halo)
    }

    /// `i` and `j` are apart on axis `ax` by a separation their gap accepts.
    fn ok(&self, g: &Geo, gc: &mut [[Option<Gap>; 2]], i: usize, j: usize, ax: usize) -> bool {
        let (p, s) = if ax == 0 { (self.x, self.w) } else { (self.y, self.h) };
        let (lo, hi) = if p[i] + s[i] <= p[j] { (i, j) } else if p[j] + s[j] <= p[i] { (j, i) } else { return false };
        let (gp, halo) = self.gap(g, gc, lo, hi, ax);
        sep_ok(p[hi] - p[lo] - s[lo], gp, halo)
    }
}

/// Merged node profile and halo: per face and role, the least member inset
/// plus the member's distance to that face; `present` ORed, `matched` the max
/// class, `set`/`well_net` when common. `None` when any member has no profile.
fn merge(k: &Kids, wd: i32, ht: i32) -> (Option<Profile>, [i32; 4]) {
    let mut halo = [0; 4];
    let mut out = Profile::default();
    let (mut net, mut set) = (None, None);
    let mut all = true;
    for i in 0..k.x.len() {
        let d = [k.x[i], k.y[i], wd - k.x[i] - k.w[i], ht - k.y[i] - k.h[i]];
        for f in 0..4 {
            halo[f] = halo[f].max(k.halo[i][f] - d[f]);
        }
        let Some(p) = &k.p[i] else {
            all = false;
            continue;
        };
        for f in 0..4 {
            let e = &p.edge[f];
            let mut bits = e.present;
            while bits != 0 {
                let r = bits.trailing_zeros() as usize;
                bits &= bits - 1;
                out.edge[f].put(r, e.inset[r].saturating_add(d[f]));
            }
        }
        net = Some(if net.is_none_or(|n| n == p.well_net) { p.well_net } else { None });
        set = Some(if set.is_none_or(|s| s == p.set) { p.set } else { None });
        out.matched = out.matched.max(p.matched);
    }
    (out.well_net, out.set) = (net.flatten(), set.flatten());
    (all.then_some(out), halo)
}

/// Decode one node into `s.rel_x/rel_y[ni]`, `s.dim[ni]`, `s.node_prof[ni]`,
/// `s.ax2[ni]` (steps 1–6 of plan-04 PLC-08). Kid nodes must be decoded.
fn decode_node(t: &Tree, g: &Geo, s: &mut Scratch, ni: usize, fixes: &mut u32) -> Result<(), Fail> {
    let nd = &t.nodes[ni];
    let k = nd.kids.len();
    let fail_at = ni as u16;
    s.kw.clear();
    s.kh.clear();
    s.kp.clear();
    s.khalo.clear();
    s.knode.clear();
    for kid in &nd.kids {
        let (w, h, p, halo, is_node) = match *kid {
            Kid::Cell(c) => {
                let c = usize::from(c);
                (g.w[c], g.h[c], g.prof.get(c).copied().flatten().copied(), g.halo.get(c).copied().unwrap_or([0; 4]), false)
            }
            Kid::Node(m) => {
                let m = usize::from(m);
                (s.dim[m].0, s.dim[m].1, s.node_prof[m], s.node_halo[m], true)
            }
        };
        s.kw.push(w);
        s.kh.push(h);
        s.kp.push(p);
        s.khalo.push(halo);
        s.knode.push(is_node);
    }
    inverse(&nd.alpha, &mut s.pa);
    inverse(&nd.beta, &mut s.pb);
    s.gc.clear();
    s.gc.resize(k * k, [None; 2]);
    s.lb.clear();
    s.lb.resize(k, 0);
    s.lby.clear();
    s.lby.resize(k, 0);
    let mut x = std::mem::take(&mut s.rel_x[ni]);
    let mut y = std::mem::take(&mut s.rel_y[ni]);
    x.clear();
    x.resize(k, 0);
    y.clear();
    y.resize(k, 0);

    let Scratch { pa, pb, lb, lby, kw, kh, kp, khalo, knode, gc, .. } = s;
    let (pa, beta) = (&*pa, &nd.beta);
    let _ = pb;
    let (kw, kh, kp, khalo, knode): (&[i32], &[i32], &[Option<Profile>], &[[i32; 4]], &[bool]) = (kw, kh, kp, khalo, knode);
    let mate = nd.sym.as_ref().map(|s| &s.mate);
    let pairs: Vec<(usize, usize)> = mate.map_or(Vec::new(), |m| {
        (0..k).filter(|&a| usize::from(m[a]) > a).map(|a| (a, usize::from(m[a]))).collect()
    });
    let has_self = mate.is_some_and(|m| (0..k).any(|a| usize::from(m[a]) == a));
    let p = pairs.len();
    let mut ax2 = 0;
    let band_cap = 2 * k + 2;
    let mut pass = 0;
    loop {
        // Step 2: Y longest path in β order; symmetry pairs equalise centres.
        for ypass in 1..=p + 2 {
            for bj in 0..k {
                let j = usize::from(beta[bj]);
                let mut v = lby[j];
                for &bi in &beta[..bj] {
                    let i = usize::from(bi);
                    if pa[i] > pa[j] {
                        let kd = Kids { x: &x, y: &y, w: kw, h: kh, p: kp, halo: khalo, node: knode };
                        let (gp, halo) = kd.gap(g, gc, i, j, 1);
                        v = v.max(y[i] + kh[i] + need(gp, halo));
                    }
                }
                y[j] = v;
            }
            let mut raised = false;
            for &(a, b) in &pairs {
                let (ca, cb) = (2 * y[a] + kh[a], 2 * y[b] + kh[b]);
                let (lo, want) = match ca.cmp(&cb) {
                    std::cmp::Ordering::Less => (a, cb),
                    std::cmp::Ordering::Greater => (b, ca),
                    std::cmp::Ordering::Equal => continue,
                };
                lby[lo] = lby[lo].max(round_up((want - kh[lo]) / 2, g.lattice));
                raised = true;
            }
            if !raised {
                break;
            }
            if ypass == p + 2 {
                return Err(Fail::SymY(fail_at));
            }
        }
        // Steps 3–4: X longest path with shadows; symmetry nodes raise right
        // members and self kids onto `ax2` and repeat until exact (fix_monotone).
        let step_x = |x: &mut [i32], ax2: Option<i32>, gc: &mut [[Option<Gap>; 2]]| {
            for bj in 0..k {
                let j = usize::from(beta[bj]);
                let mut v = lb[j];
                for &bi in &beta[..bj] {
                    let i = usize::from(bi);
                    if pa[i] < pa[j] {
                        let kd = Kids { x: &*x, y: &y, w: kw, h: kh, p: kp, halo: khalo, node: knode };
                        let gx = if kd.ok(g, gc, i, j, 1) {
                            0
                        } else {
                            let (gp, halo) = kd.gap(g, gc, i, j, 0);
                            need(gp, halo)
                        };
                        v = v.max(x[i] + kw[i] + gx);
                    }
                }
                if let (Some(ax2), Some(m)) = (ax2, mate) {
                    let l = usize::from(m[j]);
                    if l == j {
                        v = v.max((ax2 - kw[j]) / 2);
                    } else if pa[l] < pa[j] {
                        v = v.max((2 * ax2 - (2 * x[l] + kw[l]) - kw[j]) / 2);
                    }
                }
                x[j] = v;
            }
        };
        step_x(&mut x, None, gc);
        if mate.is_some() {
            let bound = |x: &[i32]| {
                let pv = pairs.iter().map(|&(a, b)| (2 * x[a] + kw[a] + 2 * x[b] + kw[b] + 1).div_euclid(2));
                let sv = (0..k).filter(|&a| mate.is_some_and(|m| usize::from(m[a]) == a)).map(|a| 2 * x[a] + kw[a]);
                pv.chain(sv).max().unwrap_or(0)
            };
            let snap = |v: i32| axis_snap(v, g.lattice, has_self, g.axis_grid);
            ax2 = snap(bound(&x));
            let mut it = 0;
            loop {
                step_x(&mut x, Some(ax2), gc);
                let b = bound(&x);
                if b == ax2 {
                    break;
                }
                it += 1;
                if it > 2 * p + 2 {
                    return Err(Fail::SymX(fail_at));
                }
                *fixes += 1;
                ax2 = snap(b.max(ax2));
            }
        }
        // Step 6: close merge bands (0 < gap < min with `abut`) on every related
        // pair not separated acceptably on the other axis.
        let mut raised = false;
        for bj in 0..k {
            let j = usize::from(beta[bj]);
            for &bi in &beta[..bj] {
                let i = usize::from(bi);
                let kd = Kids { x: &x, y: &y, w: kw, h: kh, p: kp, halo: khalo, node: knode };
                if kd.ok(g, gc, i, j, 0) || kd.ok(g, gc, i, j, 1) {
                    continue;
                }
                let ax = usize::from(pa[i] > pa[j]);
                let (gp, halo) = kd.gap(g, gc, i, j, ax);
                if ax == 0 {
                    lb[j] = lb[j].max(x[i] + kw[i] + gp.min + halo);
                } else {
                    lby[j] = lby[j].max(y[i] + kh[i] + gp.min + halo);
                }
                raised = true;
            }
        }
        if !raised {
            break;
        }
        pass += 1;
        if pass > band_cap {
            return Err(Fail::Band(fail_at));
        }
    }
    let wd = (0..k).map(|i| x[i] + kw[i]).max().unwrap_or(0);
    let ht = (0..k).map(|i| y[i] + kh[i]).max().unwrap_or(0);
    let kd = Kids { x: &x, y: &y, w: kw, h: kh, p: kp, halo: khalo, node: knode };
    let (prof, halo) = merge(&kd, wd, ht);
    s.node_prof[ni] = prof;
    s.node_halo[ni] = halo;
    s.dim[ni] = (wd, ht);
    s.ax2[ni] = ax2;
    s.rel_x[ni] = x;
    s.rel_y[ni] = y;
    Ok(())
}

/// Step 2x′: the least admissible `2·axis` ≥ `v`: a multiple of `2·lattice`
/// with a self-symmetric kid (its centre must land on the lattice), else of
/// `lattice`.
fn axis_snap(v: i32, lattice: i32, has_self: bool, _grid: Option<(i32, i32)>) -> i32 {
    round_up(v, if has_self { 2 * lattice } else { lattice })
}

/// Decode `t` (post-order; only nodes marked by [`Scratch::touch`] once
/// `s` holds a decode of `t`), then assemble absolute corners (pre-order) into
/// `out`. `Err` names the first node with no realisation; `out` is then stale.
pub fn decode(t: &Tree, g: &Geo, s: &mut Scratch, out: &mut Out) -> Result<(), Fail> {
    let nn = t.nodes.len();
    s.rel_x.resize_with(nn, Vec::new);
    s.rel_y.resize_with(nn, Vec::new);
    s.node_prof.resize(nn, None);
    s.node_halo.resize(nn, [0; 4]);
    s.dim.resize(nn, (0, 0));
    s.ax2.resize(nn, 0);
    s.org.resize(nn, (0, 0));
    if s.parent.len() != nn {
        s.parent.clear();
        s.parent.resize(nn, t.root);
        for (ni, nd) in t.nodes.iter().enumerate() {
            for k in &nd.kids {
                if let Kid::Node(m) = *k {
                    s.parent[usize::from(m)] = ni as u16;
                }
            }
        }
        s.dirty.clear();
        s.dirty.resize(nn, true);
    }
    let mut fixes = 0;
    for ni in 0..nn {
        if s.dirty[ni] {
            decode_node(t, g, s, ni, &mut fixes)?;
            s.dirty[ni] = false;
        }
    }
    out.fixes = fixes;
    out.x0.clear();
    out.x0.resize(t.home.len(), 0);
    out.y0.clear();
    out.y0.resize(t.home.len(), 0);
    s.org[usize::from(t.root)] = (0, 0);
    for ni in (0..nn).rev() {
        let (ox, oy) = s.org[ni];
        for (slot, kid) in t.nodes[ni].kids.iter().enumerate() {
            let at = (ox + s.rel_x[ni][slot], oy + s.rel_y[ni][slot]);
            match *kid {
                Kid::Cell(c) => (out.x0[usize::from(c)], out.y0[usize::from(c)]) = at,
                Kid::Node(m) => s.org[usize::from(m)] = at,
            }
        }
    }
    out.axis.clear();
    for (ni, nd) in t.nodes.iter().enumerate() {
        if let Some(sym) = &nd.sym {
            out.axis.push((sym.axis, (2 * s.org[ni].0 + s.ax2[ni]) / 2));
        }
    }
    Ok(())
}

/// Check a decode from scratch on absolute coordinates: in every node, every
/// two kids (node kids by their cells' bbox and merged profile) are apart on
/// some axis by a gap they accept; every mirror pair has `C_l + C_r = 2·ax2`
/// and equal centre y, every self kid `C_c = ax2`; every corner and `2·axis`
/// on the lattice.
#[must_use]
pub fn verify(t: &Tree, g: &Geo, out: &Out) -> bool {
    let lat = g.lattice.max(1);
    if out.x0.iter().chain(&out.y0).any(|v| v % lat != 0) || out.axis.iter().any(|a| (2 * a.1) % lat != 0) {
        return false;
    }
    let nn = t.nodes.len();
    let mut bbox: Vec<(i32, i32, i32, i32)> = vec![(0, 0, 0, 0); nn];
    let mut prof: Vec<Option<Profile>> = vec![None; nn];
    let mut halo: Vec<[i32; 4]> = vec![[0; 4]; nn];
    let mut sym_i = 0;
    for (ni, nd) in t.nodes.iter().enumerate() {
        let k = nd.kids.len();
        let (mut x, mut y, mut w, mut h) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let (mut p, mut hl, mut kn) = (Vec::new(), Vec::new(), Vec::new());
        for kid in &nd.kids {
            match *kid {
                Kid::Cell(c) => {
                    let c = usize::from(c);
                    (x.push(out.x0[c]), y.push(out.y0[c]), w.push(g.w[c]), h.push(g.h[c]));
                    (p.push(g.prof.get(c).copied().flatten().copied()), hl.push(g.halo.get(c).copied().unwrap_or([0; 4])), kn.push(false));
                }
                Kid::Node(m) => {
                    let (bx, by, bw, bh) = bbox[usize::from(m)];
                    (x.push(bx), y.push(by), w.push(bw), h.push(bh));
                    (p.push(prof[usize::from(m)]), hl.push(halo[usize::from(m)]), kn.push(true));
                }
            }
        }
        let mut gc = vec![[None; 2]; k * k];
        let kd = Kids { x: &x, y: &y, w: &w, h: &h, p: &p, halo: &hl, node: &kn };
        for i in 0..k {
            for j in i + 1..k {
                if !kd.ok(g, &mut gc, i, j, 0) && !kd.ok(g, &mut gc, i, j, 1) {
                    return false;
                }
            }
        }
        if let Some(s) = &nd.sym {
            let Some(&(_, axis)) = out.axis.get(sym_i) else { return false };
            sym_i += 1;
            for a in 0..k {
                let b = usize::from(s.mate[a]);
                let ok = if a == b {
                    2 * x[a] + w[a] == 2 * axis
                } else {
                    2 * x[a] + w[a] + 2 * x[b] + w[b] == 4 * axis && 2 * y[a] + h[a] == 2 * y[b] + h[b]
                };
                if !ok {
                    return false;
                }
            }
        }
        let x0 = x.iter().copied().min().unwrap_or(0);
        let y0 = y.iter().copied().min().unwrap_or(0);
        let x1 = (0..k).map(|i| x[i] + w[i]).max().unwrap_or(0);
        let y1 = (0..k).map(|i| y[i] + h[i]).max().unwrap_or(0);
        let rel_x: Vec<i32> = x.iter().map(|v| v - x0).collect();
        let rel_y: Vec<i32> = y.iter().map(|v| v - y0).collect();
        let kd = Kids { x: &rel_x, y: &rel_y, w: &w, h: &h, p: &p, halo: &hl, node: &kn };
        (prof[ni], halo[ni]) = merge(&kd, x1 - x0, y1 - y0);
        bbox[ni] = (x0, y0, x1 - x0, y1 - y0);
    }
    true
}

/// Assembly step 8: `x = x0 + w/2`, `hw = w/2` (same for y), and each
/// symmetry node's axis into `l.axis` (grown when short).
pub fn to_layout(out: &Out, w: &[i32], h: &[i32], l: &mut Layout) {
    for c in 0..out.x0.len() {
        (l.hw[c], l.hh[c]) = (w[c] / 2, h[c] / 2);
        (l.x[c], l.y[c]) = (out.x0[c] + w[c] / 2, out.y0[c] + h[c] / 2);
    }
    for &(a, v) in &out.axis {
        let a = usize::from(a.0);
        if l.axis.len() <= a {
            l.axis.resize(a + 1, v);
        }
        l.axis[a] = v;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gp::mechanics::SplitMix64;

    fn geo<'a>(w: &'a [i32], h: &'a [i32], prof: &'a [Option<&'a Profile>], t: &'a SpacingTable) -> Geo<'a> {
        Geo { w, h, prof, halo: &[], table: t, lattice: 10, axis_grid: None }
    }

    /// Balasa Example 1 (L2021–2028; dims from the Fig. 1.18 caption) as a
    /// hierarchy: the pair cells form the symmetry node, the other six are
    /// root siblings, each code restricted from the flat (EDCKAFGIHJBL,
    /// KACDEFGLHBJI) with the node at its first member's position.
    #[test]
    fn balasa_example_1_decodes_symmetric() {
        // A B C D E F G H I J K L
        let dims = [(2, 4), (3, 1), (4, 2), (5, 3), (4, 2), (2, 10), (2, 10), (2, 3), (5, 5), (4, 2), (3, 2), (3, 2)];
        let w: Vec<i32> = dims.iter().map(|d| d.0 * 1000).collect();
        let h: Vec<i32> = dims.iter().map(|d| d.1 * 1000).collect();
        let (f, g_, k, l_, c, j) = (5u32, 6, 10, 11, 2, 9);
        let (mut t, errs) = Tree::build(12, &[(f, g_, 0), (k, l_, 0), (c, j, 0)], &[]);
        assert!(errs.is_empty());
        // Sym slots: F0 G1 K2 L3 C4 J5. α = C K F G J L, β = K C F G L J.
        t.nodes[0].alpha = vec![4, 2, 0, 1, 5, 3];
        t.nodes[0].beta = vec![2, 4, 0, 1, 3, 5];
        assert!(t.is_sf(0));
        // Root slots: S0 A1 B2 D3 E4 H5 I6. α = E D S A I H B, β = S A D E H B I.
        let r = usize::from(t.root);
        t.nodes[r].alpha = vec![4, 3, 0, 1, 6, 5, 2];
        t.nodes[r].beta = vec![0, 1, 3, 4, 5, 2, 6];
        let table = SpacingTable::uniform(0, 10);
        let prof = vec![None; 12];
        let g = geo(&w, &h, &prof, &table);
        let mut out = Out::default();
        assert_eq!(decode(&t, &g, &mut Scratch::default(), &mut out), Ok(()));
        assert!(verify(&t, &g, &out));
        let cc = |i: u32| 2 * out.x0[i as usize] + w[i as usize];
        let ax2 = 2 * out.axis[0].1;
        assert_eq!((cc(f) + cc(g_), cc(k) + cc(l_), cc(c) + cc(j)), (2 * ax2, 2 * ax2, 2 * ax2));
        for a in 0..12 {
            for b in a + 1..12 {
                let apart = out.x0[a] + w[a] <= out.x0[b] || out.x0[b] + w[b] <= out.x0[a] || out.y0[a] + h[a] <= out.y0[b] || out.y0[b] + h[b] <= out.y0[a];
                assert!(apart, "cells {a} and {b} overlap");
            }
        }
        // Derived by hand: in the node K is below C and L below J (2000 each);
        // the node sits at the root's origin. Balasa's flat 4000 needs A below
        // C, which a contiguous node cannot express.
        assert_eq!((out.y0[c as usize], out.y0[j as usize]), (2000, 2000));
        assert_eq!(out.axis[0].1, 6000);
    }

    #[test]
    fn make_sf_satisfies_condition_1_1() {
        let (mut t, _) = Tree::build(5, &[(0, 1, 0), (2, 3, 0), (4, 4, 0)], &[]);
        let mut rng = SplitMix64::new(7);
        for _ in 0..1000 {
            let a = &mut t.nodes[0].alpha;
            for i in (1..a.len()).rev() {
                a.swap(i, rng.below(i + 1));
            }
            t.make_sf(0);
            assert!(t.is_sf(0), "{:?}", t.nodes[0]);
        }
    }

    /// Two same-net n-well kids (`Gap { abut: true, min: 1270 }`) facing across
    /// x; cell 2 (below A, left of B) sets B's longest path. `wide` = 500 ⇒
    /// gap 500, banded ⇒ closed to 1270; `wide` = 0 ⇒ abutted, stays 0.
    fn band(wide: i32) -> i32 {
        let mut table = SpacingTable::uniform(0, 10);
        table.rule[0][0] = 1270; // ROLES[0] = "nwell", mergeable
        let mut p = Profile { well_net: Some(pnr_core::NetId(1)), ..Profile::default() };
        for f in 0..4 {
            p.edge[f].put(0, 0);
        }
        assert_eq!(table.gap(&p, Face::R, &p), Gap { abut: true, min: 1270 });
        let (w, h) = ([1000, 1000, 1000 + wide], [1000, 2000, 500]);
        let prof = [Some(&p), Some(&p), None];
        let (mut t, _) = Tree::build(3, &[], &[]);
        // Root slots = cells A0 B1 C2: A left of B, C below A, C left of B.
        let r = usize::from(t.root);
        t.nodes[r].alpha = vec![0, 2, 1];
        t.nodes[r].beta = vec![2, 0, 1];
        let g = geo(&w, &h, &prof, &table);
        let mut out = Out::default();
        assert_eq!(decode(&t, &g, &mut Scratch::default(), &mut out), Ok(()));
        assert!(verify(&t, &g, &out));
        out.x0[1] - (out.x0[0] + w[0])
    }

    #[test]
    fn merge_band_is_closed() {
        assert_eq!(band(500), 1270);
        assert_eq!(band(0), 0);
    }

    #[test]
    fn conflicting_axes_are_reported() {
        let (t, errs) = Tree::build(4, &[(0, 1, 0), (0, 2, 1), (3, 2, 1)], &[]);
        assert_eq!(errs, vec![TreeError::CellInTwoAxes { cell: 0, a: AxisId(0), b: AxisId(1) }]);
        assert_eq!(t.nodes.iter().filter(|n| n.sym.is_some()).count(), 1);
    }
}
