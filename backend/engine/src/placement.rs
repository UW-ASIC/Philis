//! Placement domain types, cost math, and slot implementations.
//!
//! No external deps — the downstream crate (pnr-placement) builds [`PlaceCold`]
//! from its frontend types and hands off to these slots.

use std::marker::PhantomData;

use crate::{
    slots::{
        AllLegal, AlwaysAccept, Geometric, Metropolis, OverflowStop, RampWeights, StepDecay,
        UnitWeights,
    },
    Control, Core, CostFn, Density, Domain, Ledger, Legality, SplitMix64, Stage, Stop, Telemetry,
};

pub const NONE: u32 = u32::MAX;

/// Common-centroid interdigitation pattern (mirrors `pnr_constraints::PatternType`
/// so the engine stays self-contained in the hot loop).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CcPattern {
    /// No pattern specified — centroid-only matching.
    #[default]
    None,
    /// ABBA interdigitation.
    Abba,
    /// ABAB interdigitation.
    Abab,
    /// 2-D common centroid.
    CommonCentroid2d,
}

// ---------------------------------------------------------------------------
// Domain
// ---------------------------------------------------------------------------

pub struct PlaceDomain;

impl Domain for PlaceDomain {
    type Hot = PlaceHot;
    type Cold = PlaceCold;
    type Mv = PlaceMv;
}

/// Hot SoA state: cell centers (nm), per-symmetry-group mirror axis, and the
/// spatial bucket index that makes overlap queries O(local) instead of O(n).
#[derive(Debug, Clone)]
pub struct PlaceHot {
    pub x: Vec<f32>,
    pub y: Vec<f32>,
    /// Vertical mirror-axis x per symmetry group (nm).
    pub axis: Vec<f32>,
    pub buckets: Buckets,
}

/// Uniform-grid spatial index. Bucket side = the largest cell dimension, so
/// every potential overlap partner of a cell lives in the 3x3 neighborhood of
/// its bucket. Cores keep it current on commit.
#[derive(Debug, Clone, Default)]
pub struct Buckets {
    size: f32,
    nx: u32,
    ny: u32,
    bins: Vec<Vec<u32>>,
}

impl Buckets {
    pub fn build(cold: &PlaceCold, xs: &[f32], ys: &[f32]) -> Self {
        let max_dim = cold
            .hw
            .iter()
            .zip(&cold.hh)
            .map(|(w, h)| (2.0 * w).max(2.0 * h))
            .fold(1.0f32, f32::max);
        let size = max_dim.max(cold.die.0.max(cold.die.1) / 64.0);
        let nx = ((cold.die.0 / size).ceil() as u32).max(1);
        let ny = ((cold.die.1 / size).ceil() as u32).max(1);
        let mut b = Self { size, nx, ny, bins: vec![Vec::new(); (nx * ny) as usize] };
        for (c, (&x, &y)) in xs.iter().zip(ys).enumerate() {
            let i = b.idx(x, y);
            b.bins[i].push(c as u32);
        }
        b
    }

    fn idx(&self, x: f32, y: f32) -> usize {
        let bx = ((x / self.size) as u32).min(self.nx - 1);
        let by = ((y / self.size) as u32).min(self.ny - 1);
        (by * self.nx + bx) as usize
    }

    pub fn rebuild(&mut self, xs: &[f32], ys: &[f32]) {
        self.bins.iter_mut().for_each(Vec::clear);
        for (c, (&x, &y)) in xs.iter().zip(ys).enumerate() {
            let i = self.idx(x, y);
            self.bins[i].push(c as u32);
        }
    }

    pub fn move_cell(&mut self, c: u32, old: (f32, f32), new: (f32, f32)) {
        let (oi, ni) = (self.idx(old.0, old.1), self.idx(new.0, new.1));
        if oi == ni {
            return;
        }
        if let Some(k) = self.bins[oi].iter().position(|&m| m == c) {
            self.bins[oi].swap_remove(k);
        }
        self.bins[ni].push(c);
    }

    /// Cells in the 3x3 bucket neighborhood of (x, y).
    pub fn near(&self, x: f32, y: f32, out: &mut Vec<u32>) {
        out.clear();
        let bx = ((x / self.size) as i64).clamp(0, i64::from(self.nx) - 1);
        let by = ((y / self.size) as i64).clamp(0, i64::from(self.ny) - 1);
        for dy in -1..=1i64 {
            for dx in -1..=1i64 {
                let (tx, ty) = (bx + dx, by + dy);
                if tx >= 0 && ty >= 0 && tx < i64::from(self.nx) && ty < i64::from(self.ny) {
                    out.extend_from_slice(&self.bins[(ty * i64::from(self.nx) + tx) as usize]);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Move
// ---------------------------------------------------------------------------

/// A proposed move: a handful of cells (SA) or all of them (analytical step),
/// with an optional axis update and the delta the proposer already computed.
#[derive(Debug, Clone, Default)]
pub struct PlaceMv {
    pub idx: Vec<u32>,
    pub nx: Vec<f32>,
    pub ny: Vec<f32>,
    /// (group, new axis x) when the move shifts a whole symmetry island.
    pub axis: Option<(u32, f32)>,
    /// Full-cost delta cached by an analytical core; SA leaves `None` and the
    /// CostFn computes it incrementally.
    pub cached_delta: Option<f64>,
}

impl PlaceMv {
    pub fn new_pos(&self, cell: u32, hot: &PlaceHot) -> (f32, f32) {
        match self.idx.iter().position(|&i| i == cell) {
            Some(k) => (self.nx[k], self.ny[k]),
            None => (hot.x[cell as usize], hot.y[cell as usize]),
        }
    }
}

// ---------------------------------------------------------------------------
// Cold context
// ---------------------------------------------------------------------------

/// CSR: item -> list of u32.
#[derive(Debug, Clone, Default)]
pub struct Csr {
    pub start: Vec<u32>,
    pub items: Vec<u32>,
}

impl Csr {
    pub fn row(&self, i: u32) -> &[u32] {
        &self.items[self.start[i as usize] as usize..self.start[i as usize + 1] as usize]
    }
}

#[derive(Debug, Clone, Default)]
pub struct SymTable {
    /// Per cell: symmetry group or NONE.
    pub group_of: Vec<u32>,
    /// Per cell: mirror partner; self for self-symmetric; NONE if ungrouped.
    pub partner: Vec<u32>,
    /// Group -> member cells.
    pub groups: Vec<Vec<u32>>,
    /// Group axis fixed by the constraint (angstrom-sourced) vs free.
    pub fixed: Vec<bool>,
    /// Initial axis x per group, nm (constraint value or die center).
    pub axis0: Vec<f32>,
}

/// Soft pull-together / hard keep-apart pair, nm.
#[derive(Debug, Clone, Copy)]
pub struct PairRule {
    pub a: u32,
    pub b: u32,
    pub gap_nm: f32,
    pub weight: f32,
}

#[derive(Debug, Clone, Default)]
pub struct PlaceCold {
    /// Half-extents per cell, nm.
    pub hw: Vec<f32>,
    pub hh: Vec<f32>,
    /// Net -> unique member cells (nets with <2 cells dropped).
    pub nets: Csr,
    pub net_w: Vec<f32>,
    /// Cell -> nets (indices into `nets` rows).
    pub cell_nets: Csr,
    pub sym: SymTable,
    /// Soft attraction: proximity(gap=0), thermal, cc handled separately.
    pub pulls: Vec<PairRule>,
    /// Hard min edge-to-edge gap (isolation).
    pub pushes: Vec<PairRule>,
    /// Common-centroid: (side A cells, side B cells).
    pub cc: Vec<(Vec<u32>, Vec<u32>)>,
    /// Per-CC-group pattern type (parallel to `cc`).
    pub cc_pattern: Vec<CcPattern>,
    /// StraightNet alignment: (cells, vertical) — align x if vertical.
    pub aligns: Vec<(Vec<u32>, bool)>,
    pub die: (f32, f32),
    pub grid: f32,
    /// Per-cell layer bitmask (bit = PDK LayerId present in cell geometry).
    /// Cells overlap only when masks intersect. Empty → all cells conflict.
    pub layer_mask: Vec<u64>,
}

#[inline]
fn layers_conflict(cold: &PlaceCold, a: usize, b: usize) -> bool {
    cold.layer_mask.is_empty()
        || (cold.layer_mask[a] & cold.layer_mask[b]) != 0
}

// ---------------------------------------------------------------------------
// Cost math
// ---------------------------------------------------------------------------

pub fn net_bbox(cells: &[u32], xs: &[f32], ys: &[f32]) -> (f32, f32, f32, f32) {
    let (mut x0, mut x1, mut y0, mut y1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for &c in cells {
        let (x, y) = (xs[c as usize], ys[c as usize]);
        x0 = x0.min(x);
        x1 = x1.max(x);
        y0 = y0.min(y);
        y1 = y1.max(y);
    }
    (x0, x1, y0, y1)
}

pub fn hpwl(cold: &PlaceCold, xs: &[f32], ys: &[f32]) -> f64 {
    let mut total = 0.0f64;
    for n in 0..cold.net_w.len() {
        let cells = cold.nets.row(n as u32);
        let (x0, x1, y0, y1) = net_bbox(cells, xs, ys);
        total += f64::from(cold.net_w[n]) * f64::from((x1 - x0) + (y1 - y0));
    }
    total
}

/// Soft analog terms: pulls (quadratic beyond gap), CC centroid error,
/// straight-net alignment. All in nm-comparable units.
pub fn soft_terms(cold: &PlaceCold, xs: &[f32], ys: &[f32]) -> f64 {
    let mut t = 0.0f64;
    for p in &cold.pulls {
        let dx = xs[p.a as usize] - xs[p.b as usize];
        let dy = ys[p.a as usize] - ys[p.b as usize];
        let d = (dx * dx + dy * dy).sqrt();
        let ex = (d - p.gap_nm).max(0.0);
        t += f64::from(p.weight) * f64::from(ex * ex) * 1e-3;
    }
    for (ci, (ga, gb)) in cold.cc.iter().enumerate() {
        let (cax, cay) = centroid(ga, xs, ys);
        let (cbx, cby) = centroid(gb, xs, ys);
        let (dx, dy) = (cax - cbx, cay - cby);
        t += f64::from(dx * dx + dy * dy) * 1e-3;
        // Pattern-aware per-device alignment pulls
        let pat = cold.cc_pattern.get(ci).copied().unwrap_or(CcPattern::None);
        t += cc_pattern_cost(pat, ga, gb, |c| (xs[c as usize], ys[c as usize]));
    }
    for (cells, vertical) in &cold.aligns {
        let coords: &[f32] = if *vertical { xs } else { ys };
        let mean = cells.iter().map(|&c| coords[c as usize]).sum::<f32>() / cells.len() as f32;
        for &c in cells {
            t += f64::from((coords[c as usize] - mean).abs());
        }
    }
    t
}

pub fn cost_at(cold: &PlaceCold, xs: &[f32], ys: &[f32]) -> f64 {
    hpwl(cold, xs, ys) + soft_terms(cold, xs, ys)
}

pub fn centroid(cells: &[u32], xs: &[f32], ys: &[f32]) -> (f32, f32) {
    let n = cells.len().max(1) as f32;
    let sx: f32 = cells.iter().map(|&c| xs[c as usize]).sum();
    let sy: f32 = cells.iter().map(|&c| ys[c as usize]).sum();
    (sx / n, sy / n)
}

/// Edge-to-edge gap between two cells (negative = overlapping).
pub fn gap(cold: &PlaceCold, a: u32, b: u32, xs: &[f32], ys: &[f32]) -> f32 {
    let (ai, bi) = (a as usize, b as usize);
    if !layers_conflict(cold, ai, bi) { return f32::MAX; }
    let gx = (xs[ai] - xs[bi]).abs() - (cold.hw[ai] + cold.hw[bi]);
    let gy = (ys[ai] - ys[bi]).abs() - (cold.hh[ai] + cold.hh[bi]);
    gx.max(gy)
}

/// Rectangle overlap area of cells a and b at the given coords.
pub fn overlap_area(cold: &PlaceCold, a: usize, b: usize, xs: &[f32], ys: &[f32]) -> f64 {
    if !layers_conflict(cold, a, b) { return 0.0; }
    let ox = (cold.hw[a] + cold.hw[b]) - (xs[a] - xs[b]).abs();
    let oy = (cold.hh[a] + cold.hh[b]) - (ys[a] - ys[b]).abs();
    if ox > 0.0 && oy > 0.0 {
        f64::from(ox) * f64::from(oy)
    } else {
        0.0
    }
}

/// Total pairwise overlap area — exact O(n²) reference, used once per flow for
/// the final report.
pub fn total_overlap(cold: &PlaceCold, xs: &[f32], ys: &[f32]) -> f64 {
    let n = cold.hw.len();
    let mut t = 0.0;
    for a in 0..n {
        for b in a + 1..n {
            t += overlap_area(cold, a, b, xs, ys);
        }
    }
    t
}

// ---------------------------------------------------------------------------
// Constraint checks (pure evaluation; the ledger in pnr-placement maps these
// to ConstraintContract updates)
// ---------------------------------------------------------------------------

pub enum PlaceCheck {
    /// |xa + xb − 2·axis| ≤ tol ∧ |ya − yb| ≤ tol
    SymPair { a: u32, b: u32, g: u32 },
    /// centroid(A) ≈ centroid(B)
    Cc { i: usize },
    /// edge gap ≥ min (hard isolation)
    MinGap { a: u32, b: u32, min_nm: f32 },
    /// soft keep-within (proximity budget; 0 ⇒ "adjacent", tol = 4·max span)
    MaxDist { a: u32, b: u32, max_nm: f32 },
}

/// Evaluate a single constraint check. Returns (satisfied, violation_metric).
pub fn eval_check(ch: &PlaceCheck, hot: &PlaceHot, cold: &PlaceCold) -> (bool, f32) {
    let tol = cold.grid * 2.0;
    match *ch {
        PlaceCheck::SymPair { a, b, g } => {
            let ex =
                (hot.x[a as usize] + hot.x[b as usize] - 2.0 * hot.axis[g as usize]).abs();
            let ey = (hot.y[a as usize] - hot.y[b as usize]).abs();
            (ex <= tol && ey <= tol, ex.max(ey))
        }
        PlaceCheck::Cc { i } => {
            let (ga, gb) = &cold.cc[i];
            let (ax, ay) = centroid(ga, &hot.x, &hot.y);
            let (bx, by) = centroid(gb, &hot.x, &hot.y);
            let e = ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt();
            (e <= tol * 4.0, e)
        }
        PlaceCheck::MinGap { a, b, min_nm } => {
            let gp = gap(cold, a, b, &hot.x, &hot.y);
            (gp >= min_nm, min_nm - gp)
        }
        PlaceCheck::MaxDist { a, b, max_nm } => {
            let gp = gap(cold, a, b, &hot.x, &hot.y);
            let span = 2.0
                * (cold.hw[a as usize] + cold.hw[b as usize])
                    .max(cold.hh[a as usize] + cold.hh[b as usize]);
            let lim = if max_nm > 0.0 { max_nm } else { span };
            (gp <= lim, gp - lim)
        }
    }
}

// ---------------------------------------------------------------------------
// Global stage — analytical descent
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct GlobalCfg {
    pub max_iters: u32,
    pub overflow_target: f32,
    pub step0: f32,
    pub momentum: f32,
    pub lambda0: f32,
    pub sym_weight: f32,
    pub target_util: f32,
}

impl Default for GlobalCfg {
    fn default() -> Self {
        Self {
            max_iters: 500,
            overflow_target: 0.15,
            step0: 0.04,
            momentum: 0.85,
            lambda0: 0.5,
            sym_weight: 4.0,
            target_util: 0.7,
        }
    }
}

pub struct Bins {
    pub nb: usize,
    pub bw: f32,
    pub bh: f32,
    pub util: Vec<f32>,
}

impl Bins {
    pub fn new(cold: &PlaceCold) -> Self {
        let n = cold.hw.len();
        let nb = ((n as f32).sqrt().ceil() as usize).clamp(4, 24);
        Bins {
            nb,
            bw: cold.die.0 / nb as f32,
            bh: cold.die.1 / nb as f32,
            util: vec![0.0; nb * nb],
        }
    }

    pub fn fill(&mut self, cold: &PlaceCold, xs: &[f32], ys: &[f32]) {
        self.util.iter_mut().for_each(|u| *u = 0.0);
        // ponytail: whole cell area lands in its center bin
        for i in 0..cold.hw.len() {
            let b = self.bin_of(xs[i], ys[i]);
            self.util[b] += 4.0 * cold.hw[i] * cold.hh[i] / (self.bw * self.bh);
        }
    }

    pub fn bin_of(&self, x: f32, y: f32) -> usize {
        let bx = ((x / self.bw) as usize).min(self.nb - 1);
        let by = ((y / self.bh) as usize).min(self.nb - 1);
        by * self.nb + bx
    }

    pub fn overflow(&self, cold: &PlaceCold, target_util: f32) -> f32 {
        let total: f32 = cold.hw.iter().zip(&cold.hh).map(|(w, h)| 4.0 * w * h).sum();
        if total <= 0.0 {
            return 0.0;
        }
        let over: f32 = self
            .util
            .iter()
            .map(|&u| (u - target_util).max(0.0) * self.bw * self.bh)
            .sum();
        over / total
    }
}

pub struct GlobalCost;
impl CostFn<PlaceDomain> for GlobalCost {
    fn eval(&self, hot: &PlaceHot, cold: &PlaceCold) -> f64 {
        cost_at(cold, &hot.x, &hot.y)
    }
    fn delta(&self, _: &PlaceHot, _: &PlaceCold, mv: &PlaceMv) -> f64 {
        mv.cached_delta.unwrap_or(0.0)
    }
}

pub struct BinDensity {
    pub target_util: f32,
}
impl Density<PlaceDomain> for BinDensity {
    fn overflow(&self, hot: &PlaceHot, cold: &PlaceCold) -> f32 {
        let mut bins = Bins::new(cold);
        bins.fill(cold, &hot.x, &hot.y);
        bins.overflow(cold, self.target_util)
    }
}

pub struct DescentCore {
    pub cfg: GlobalCfg,
}

pub struct DescentScratch {
    vx: Vec<f32>,
    vy: Vec<f32>,
    gx: Vec<f32>,
    gy: Vec<f32>,
    bins: Bins,
    lambda: f32,
    overflow: f32,
}

impl Core<PlaceDomain> for DescentCore {
    type Scratch = DescentScratch;

    fn scratch(&self, hot: &PlaceHot, cold: &PlaceCold) -> DescentScratch {
        let n = hot.x.len();
        DescentScratch {
            vx: vec![0.0; n],
            vy: vec![0.0; n],
            gx: vec![0.0; n],
            gy: vec![0.0; n],
            bins: Bins::new(cold),
            lambda: self.cfg.lambda0,
            overflow: 1.0,
        }
    }

    fn propose(
        &self,
        hot: &PlaceHot,
        cold: &PlaceCold,
        sc: &mut DescentScratch,
        ctl: &Control,
        _rng: &mut SplitMix64,
    ) -> Option<PlaceMv> {
        let n = hot.x.len();
        sc.gx.iter_mut().for_each(|g| *g = 0.0);
        sc.gy.iter_mut().for_each(|g| *g = 0.0);

        // HPWL subgradient
        for ni in 0..cold.net_w.len() {
            let cells = cold.nets.row(ni as u32);
            let w = cold.net_w[ni];
            let (x0, x1, y0, y1) = net_bbox(cells, &hot.x, &hot.y);
            for &c in cells {
                let ci = c as usize;
                if hot.x[ci] >= x1 - 0.5 {
                    sc.gx[ci] += w;
                }
                if hot.x[ci] <= x0 + 0.5 {
                    sc.gx[ci] -= w;
                }
                if hot.y[ci] >= y1 - 0.5 {
                    sc.gy[ci] += w;
                }
                if hot.y[ci] <= y0 + 0.5 {
                    sc.gy[ci] -= w;
                }
            }
        }

        // soft pulls (proximity / thermal)
        for p in &cold.pulls {
            let (ai, bi) = (p.a as usize, p.b as usize);
            let (dx, dy) = (hot.x[ai] - hot.x[bi], hot.y[ai] - hot.y[bi]);
            let d = (dx * dx + dy * dy).sqrt().max(1.0);
            let ex = (d - p.gap_nm).max(0.0);
            let k = 2.0 * p.weight * ex * 1e-3 / d;
            sc.gx[ai] += k * dx;
            sc.gx[bi] -= k * dx;
            sc.gy[ai] += k * dy;
            sc.gy[bi] -= k * dy;
        }

        // common centroid
        for (ci, (ga, gb)) in cold.cc.iter().enumerate() {
            let (ax, ay) = centroid(ga, &hot.x, &hot.y);
            let (bx, by) = centroid(gb, &hot.x, &hot.y);
            let (dx, dy) = (ax - bx, ay - by);
            for &c in ga {
                sc.gx[c as usize] += 2e-3 * dx / ga.len() as f32;
                sc.gy[c as usize] += 2e-3 * dy / ga.len() as f32;
            }
            for &c in gb {
                sc.gx[c as usize] -= 2e-3 * dx / gb.len() as f32;
                sc.gy[c as usize] -= 2e-3 * dy / gb.len() as f32;
            }
            // pattern-aware gradient: pull devices toward their pattern slots
            let pat = cold.cc_pattern.get(ci).copied().unwrap_or(CcPattern::None);
            if pat != CcPattern::None && ga.len() == 2 && gb.len() == 2 {
                let all: Vec<u32> = ga.iter().chain(gb).copied().collect();
                let nn = all.len() as f32;
                let cx = all.iter().map(|&c| hot.x[c as usize]).sum::<f32>() / nn;
                let d_spacing = all
                    .iter()
                    .map(|&c| (hot.x[c as usize] - cx).abs())
                    .sum::<f32>()
                    / nn;
                let d_spacing = d_spacing.max(100.0);
                let offsets: [f32; 4] = [-1.5, -0.5, 0.5, 1.5];
                let assignment: [(usize, usize); 4] = match pat {
                    CcPattern::Abba => [(0, 0), (1, 0), (1, 1), (0, 1)],
                    CcPattern::Abab => [(0, 0), (1, 0), (0, 1), (1, 1)],
                    CcPattern::CommonCentroid2d => [(0, 0), (1, 0), (1, 1), (0, 1)],
                    CcPattern::None => unreachable!(),
                };
                let sides: [&[u32]; 2] = [ga, gb];
                for (slot, &(side, idx)) in assignment.iter().enumerate() {
                    if idx < sides[side].len() {
                        let c = sides[side][idx] as usize;
                        let target_x = cx + offsets[slot] * d_spacing;
                        sc.gx[c] += 1e-3 * (hot.x[c] - target_x);
                    }
                }
            }
        }

        // straight-net alignment
        for (cells, vertical) in &cold.aligns {
            let coords: &[f32] = if *vertical { &hot.x } else { &hot.y };
            let mean =
                cells.iter().map(|&c| coords[c as usize]).sum::<f32>() / cells.len() as f32;
            for &c in cells {
                let g = (coords[c as usize] - mean).signum();
                if *vertical {
                    sc.gx[c as usize] += g;
                } else {
                    sc.gy[c as usize] += g;
                }
            }
        }

        // differentiable symmetry error
        let mut new_axis = hot.axis.clone();
        for (gi, members) in cold.sym.groups.iter().enumerate() {
            if !cold.sym.fixed[gi] && !members.is_empty() {
                let mut s = 0.0f32;
                let mut cnt = 0.0f32;
                for &m in members {
                    let p = cold.sym.partner[m as usize];
                    s += if p == m {
                        hot.x[m as usize]
                    } else {
                        (hot.x[m as usize] + hot.x[p as usize]) / 2.0
                    };
                    cnt += 1.0;
                }
                new_axis[gi] = s / cnt;
            }
            let ax = new_axis[gi];
            let k = self.cfg.sym_weight;
            for &m in members {
                let mi = m as usize;
                let p = cold.sym.partner[mi];
                if p == m {
                    sc.gx[mi] += k * (hot.x[mi] - ax) * 1e-3;
                } else if p > m {
                    let pi = p as usize;
                    let ex = (hot.x[mi] + hot.x[pi]) / 2.0 - ax;
                    let ey = hot.y[mi] - hot.y[pi];
                    sc.gx[mi] += k * ex * 1e-3;
                    sc.gx[pi] += k * ex * 1e-3;
                    sc.gy[mi] += k * ey * 1e-3;
                    sc.gy[pi] -= k * ey * 1e-3;
                }
            }
        }

        // density push
        sc.bins.fill(cold, &hot.x, &hot.y);
        let nb = sc.bins.nb;
        for i in 0..n {
            let b = sc.bins.bin_of(hot.x[i], hot.y[i]);
            let over = sc.bins.util[b] - self.cfg.target_util;
            if over <= 0.0 {
                continue;
            }
            let (bx, by) = (b % nb, b / nb);
            let mut best = (sc.bins.util[b], 0i32, 0i32);
            for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let (tx, ty) = (bx as i32 + dx, by as i32 + dy);
                if tx >= 0 && ty >= 0 && (tx as usize) < nb && (ty as usize) < nb {
                    let u = sc.bins.util[ty as usize * nb + tx as usize];
                    if u < best.0 {
                        best = (u, dx, dy);
                    }
                }
            }
            sc.gx[i] -= sc.lambda * over * best.1 as f32;
            sc.gy[i] -= sc.lambda * over * best.2 as f32;
        }

        // momentum step
        let span = cold.die.0.max(cold.die.1);
        let gmax = sc
            .gx
            .iter()
            .chain(sc.gy.iter())
            .fold(0.0f32, |m, g| m.max(g.abs()))
            .max(1e-12);
        let scale = ctl.step * span / gmax;

        let mut mv = PlaceMv {
            idx: (0..n as u32).collect(),
            nx: Vec::with_capacity(n),
            ny: Vec::with_capacity(n),
            axis: None,
            cached_delta: None,
        };
        for i in 0..n {
            sc.vx[i] = self.cfg.momentum * sc.vx[i] - scale * sc.gx[i];
            sc.vy[i] = self.cfg.momentum * sc.vy[i] - scale * sc.gy[i];
            mv.nx
                .push((hot.x[i] + sc.vx[i]).clamp(cold.hw[i], cold.die.0 - cold.hw[i]));
            mv.ny
                .push((hot.y[i] + sc.vy[i]).clamp(cold.hh[i], cold.die.1 - cold.hh[i]));
        }
        mv.cached_delta =
            Some(cost_at(cold, &mv.nx, &mv.ny) - cost_at(cold, &hot.x, &hot.y));
        Some(mv)
    }

    fn commit(
        &self,
        hot: &mut PlaceHot,
        cold: &PlaceCold,
        sc: &mut DescentScratch,
        mv: &PlaceMv,
    ) {
        hot.x.copy_from_slice(&mv.nx);
        hot.y.copy_from_slice(&mv.ny);
        hot.buckets.rebuild(&hot.x, &hot.y);
        for (gi, members) in cold.sym.groups.iter().enumerate() {
            if cold.sym.fixed[gi] || members.is_empty() {
                continue;
            }
            let mut s = 0.0f32;
            for &m in members {
                let p = cold.sym.partner[m as usize];
                s += if p == m {
                    hot.x[m as usize]
                } else {
                    (hot.x[m as usize] + hot.x[p as usize]) / 2.0
                };
            }
            hot.axis[gi] = s / members.len() as f32;
        }
        sc.bins.fill(cold, &hot.x, &hot.y);
        sc.overflow = sc.bins.overflow(cold, self.cfg.target_util);
    }

    fn epoch(
        &self,
        _hot: &mut PlaceHot,
        _cold: &PlaceCold,
        sc: &mut DescentScratch,
        _t: &Telemetry,
    ) {
        if sc.overflow > 0.05 {
            sc.lambda = (sc.lambda * 1.05).min(1e3);
        }
    }
}

/// Deterministic initial state: everything near die center with seeded jitter;
/// symmetric partners start mirrored so the sym penalty begins near zero.
pub fn initial_state(cold: &PlaceCold, rng: &mut SplitMix64) -> PlaceHot {
    let n = cold.hw.len();
    let (cx, cy) = (cold.die.0 / 2.0, cold.die.1 / 2.0);
    let jitter = cold.die.0.min(cold.die.1) * 0.15;
    let mut hot = PlaceHot {
        x: vec![0.0; n],
        y: vec![0.0; n],
        axis: cold.sym.axis0.clone(),
        buckets: Buckets::default(),
    };
    for i in 0..n {
        hot.x[i] = cx + rng.centered(jitter);
        hot.y[i] = cy + rng.centered(jitter);
    }
    for (gi, members) in cold.sym.groups.iter().enumerate() {
        let ax = hot.axis[gi];
        for &m in members {
            let mi = m as usize;
            let p = cold.sym.partner[mi];
            if p == m {
                hot.x[mi] = ax;
            } else if p > m {
                let pi = p as usize;
                hot.x[pi] = 2.0 * ax - hot.x[mi];
                hot.y[pi] = hot.y[mi];
            }
        }
    }
    hot.buckets = Buckets::build(cold, &hot.x, &hot.y);
    hot
}

pub fn run_global<Lg: Ledger<PlaceDomain>>(
    hot: &mut PlaceHot,
    cold: &PlaceCold,
    ledger: &mut Lg,
    cfg: &GlobalCfg,
    rng: &mut SplitMix64,
) -> Telemetry {
    let stage = Stage {
        cost: GlobalCost,
        density: BinDensity { target_util: cfg.target_util },
        legality: AllLegal,
        weights: UnitWeights,
        core: DescentCore { cfg: cfg.clone() },
        accept: AlwaysAccept,
        schedule: StepDecay {
            step0: cfg.step0,
            decay: 0.995,
            step_min: 0.002,
            moves_per_step: 1,
        },
        stop: OverflowStop {
            max_iters: cfg.max_iters,
            min_iters: 60,
            target: cfg.overflow_target,
        },
        _d: PhantomData,
    };
    stage.run(hot, cold, ledger, rng)
}

// ---------------------------------------------------------------------------
// Detailed stage — symmetry-preserving SA
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DetailedCfg {
    pub max_iters: u32,
    pub min_iters: u32,
    pub moves_per_cell: u32,
    pub alpha: f64,
    pub range0: f32,
    pub min_accept_rate: f32,
}

impl Default for DetailedCfg {
    fn default() -> Self {
        Self {
            max_iters: 220,
            min_iters: 20,
            moves_per_cell: 60,
            alpha: 0.93,
            range0: 0.4,
            min_accept_rate: 0.02,
        }
    }
}

pub struct SaCost;
impl CostFn<PlaceDomain> for SaCost {
    fn eval(&self, hot: &PlaceHot, cold: &PlaceCold) -> f64 {
        cost_at(cold, &hot.x, &hot.y)
    }

    fn delta(&self, hot: &PlaceHot, cold: &PlaceCold, mv: &PlaceMv) -> f64 {
        let mut nets: Vec<u32> = mv
            .idx
            .iter()
            .flat_map(|&c| cold.cell_nets.row(c).iter().copied())
            .collect();
        nets.sort_unstable();
        nets.dedup();

        let mut d = 0.0f64;
        for &ni in &nets {
            let cells = cold.nets.row(ni);
            let w = f64::from(cold.net_w[ni as usize]);
            let (ox0, ox1, oy0, oy1) = net_bbox(cells, &hot.x, &hot.y);
            let (mut x0, mut x1, mut y0, mut y1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
            for &c in cells {
                let (x, y) = mv.new_pos(c, hot);
                x0 = x0.min(x);
                x1 = x1.max(x);
                y0 = y0.min(y);
                y1 = y1.max(y);
            }
            d += w * f64::from((x1 - x0) + (y1 - y0) - (ox1 - ox0) - (oy1 - oy0));
        }

        for p in &cold.pulls {
            if touches(mv, p.a) || touches(mv, p.b) {
                d += pull_cost(p, mv.new_pos(p.a, hot), mv.new_pos(p.b, hot))
                    - pull_cost(
                        p,
                        (hot.x[p.a as usize], hot.y[p.a as usize]),
                        (hot.x[p.b as usize], hot.y[p.b as usize]),
                    );
            }
        }
        for (ci, (ga, gb)) in cold.cc.iter().enumerate() {
            if ga.iter().chain(gb).any(|&c| touches(mv, c)) {
                d += cc_cost(ga, gb, |c| mv.new_pos(c, hot))
                    - cc_cost(ga, gb, |c| (hot.x[c as usize], hot.y[c as usize]));
                let pat = cold.cc_pattern.get(ci).copied().unwrap_or(CcPattern::None);
                d += cc_pattern_cost(pat, ga, gb, |c| mv.new_pos(c, hot))
                    - cc_pattern_cost(pat, ga, gb, |c| {
                        (hot.x[c as usize], hot.y[c as usize])
                    });
            }
        }
        for (cells, vertical) in &cold.aligns {
            if cells.iter().any(|&c| touches(mv, c)) {
                d += align_cost(cells, *vertical, |c| mv.new_pos(c, hot))
                    - align_cost(cells, *vertical, |c| {
                        (hot.x[c as usize], hot.y[c as usize])
                    });
            }
        }
        d
    }
}

fn touches(mv: &PlaceMv, c: u32) -> bool {
    mv.idx.contains(&c)
}

fn pull_cost(p: &PairRule, a: (f32, f32), b: (f32, f32)) -> f64 {
    let (dx, dy) = (a.0 - b.0, a.1 - b.1);
    let ex = ((dx * dx + dy * dy).sqrt() - p.gap_nm).max(0.0);
    f64::from(p.weight) * f64::from(ex * ex) * 1e-3
}

fn cc_cost(ga: &[u32], gb: &[u32], pos: impl Fn(u32) -> (f32, f32)) -> f64 {
    let cen = |cells: &[u32]| {
        let n = cells.len().max(1) as f32;
        let (mut sx, mut sy) = (0.0f32, 0.0f32);
        for &c in cells {
            let (x, y) = pos(c);
            sx += x;
            sy += y;
        }
        (sx / n, sy / n)
    };
    let (ax, ay) = cen(ga);
    let (bx, by) = cen(gb);
    f64::from((ax - bx).powi(2) + (ay - by).powi(2)) * 1e-3
}

/// Pattern-aware CC cost: for ABBA/ABAB with 2+2 devices, generate target
/// positions at [-1.5d, -0.5d, +0.5d, +1.5d] from the group centroid and pull
/// individual devices toward their assigned slot.
fn cc_pattern_cost(
    pat: CcPattern,
    ga: &[u32],
    gb: &[u32],
    pos: impl Fn(u32) -> (f32, f32),
) -> f64 {
    if pat == CcPattern::None || ga.len() != 2 || gb.len() != 2 {
        return 0.0;
    }
    // Combined centroid
    let all: Vec<u32> = ga.iter().chain(gb).copied().collect();
    let n = all.len() as f32;
    let cx: f32 = all.iter().map(|&c| pos(c).0).sum::<f32>() / n;
    // Average device half-width as spacing unit
    let d = all
        .iter()
        .map(|&c| {
            let (x, _) = pos(c);
            (x - cx).abs()
        })
        .sum::<f32>()
        / n;
    let d = d.max(100.0); // avoid degenerate near-zero spacing

    // Slot x-offsets from centroid: [-1.5d, -0.5d, +0.5d, +1.5d]
    let offsets = [-1.5, -0.5, 0.5, 1.5];
    // ABBA: A at +-1.5d, B at +-0.5d → slot assignment: [A0, B0, B1, A1]
    // ABAB: A at -1.5d, +0.5d; B at -0.5d, +1.5d → [A0, B0, A1, B1]
    let assignment: [(usize, usize); 4] = match pat {
        // (side: 0=ga, 1=gb, index within side)
        CcPattern::Abba => [(0, 0), (1, 0), (1, 1), (0, 1)],
        CcPattern::Abab => [(0, 0), (1, 0), (0, 1), (1, 1)],
        // 2D CC: same as ABBA for the 1D pull; real 2D would need y slots too
        CcPattern::CommonCentroid2d => [(0, 0), (1, 0), (1, 1), (0, 1)],
        CcPattern::None => unreachable!(),
    };
    let sides: [&[u32]; 2] = [ga, gb];
    let weight = 0.5e-3; // moderate pull
    let mut cost = 0.0f64;
    for (slot, &(side, idx)) in assignment.iter().enumerate() {
        if idx >= sides[side].len() {
            continue;
        }
        let c = sides[side][idx];
        let (x, _) = pos(c);
        let target_x = cx + offsets[slot] as f32 * d;
        let ex = x - target_x;
        cost += f64::from(ex * ex) * weight;
    }
    cost
}

fn align_cost(cells: &[u32], vertical: bool, pos: impl Fn(u32) -> (f32, f32)) -> f64 {
    let coord = |c: u32| if vertical { pos(c).0 } else { pos(c).1 };
    let mean = cells.iter().map(|&c| coord(c)).sum::<f32>() / cells.len() as f32;
    cells.iter().map(|&c| f64::from((coord(c) - mean).abs())).sum()
}

pub struct OverlapDensity;
impl Density<PlaceDomain> for OverlapDensity {
    fn delta(&self, hot: &PlaceHot, cold: &PlaceCold, mv: &PlaceMv) -> f64 {
        let mut cand = Vec::with_capacity(16);
        let mut d = 0.0f64;
        for (k, &a) in mv.idx.iter().enumerate() {
            let ai = a as usize;
            let (nax, nay) = (mv.nx[k], mv.ny[k]);
            hot.buckets.near(hot.x[ai], hot.y[ai], &mut cand);
            let mut cand2 = Vec::with_capacity(16);
            hot.buckets.near(nax, nay, &mut cand2);
            cand.extend_from_slice(&cand2);
            cand.sort_unstable();
            cand.dedup();
            for &b in &cand {
                if b == a {
                    continue;
                }
                if let Some(kb) = mv.idx.iter().position(|&m| m == b) {
                    if kb < k {
                        continue;
                    }
                    let bi = b as usize;
                    d += rect_overlap(cold, ai, bi, nax, nay, mv.nx[kb], mv.ny[kb])
                        - overlap_area(cold, ai, bi, &hot.x, &hot.y);
                } else {
                    let bi = b as usize;
                    d += rect_overlap(cold, ai, bi, nax, nay, hot.x[bi], hot.y[bi])
                        - overlap_area(cold, ai, bi, &hot.x, &hot.y);
                }
            }
        }
        d
    }

    fn overflow(&self, hot: &PlaceHot, cold: &PlaceCold) -> f32 {
        let total: f32 = cold.hw.iter().zip(&cold.hh).map(|(w, h)| 4.0 * w * h).sum();
        if total <= 0.0 {
            return 0.0;
        }
        let mut cand = Vec::with_capacity(16);
        let mut over = 0.0f64;
        for a in 0..cold.hw.len() {
            hot.buckets.near(hot.x[a], hot.y[a], &mut cand);
            for &b in &cand {
                if (b as usize) > a {
                    over += overlap_area(cold, a, b as usize, &hot.x, &hot.y);
                }
            }
        }
        (over as f32) / total
    }
}

fn rect_overlap(cold: &PlaceCold, a: usize, b: usize, ax: f32, ay: f32, bx: f32, by: f32) -> f64 {
    if !layers_conflict(cold, a, b) { return 0.0; }
    let ox = (cold.hw[a] + cold.hw[b]) - (ax - bx).abs();
    let oy = (cold.hh[a] + cold.hh[b]) - (ay - by).abs();
    if ox > 0.0 && oy > 0.0 {
        f64::from(ox) * f64::from(oy)
    } else {
        0.0
    }
}

pub struct HardGaps;
impl Legality<PlaceDomain> for HardGaps {
    fn is_legal(&self, hot: &PlaceHot, cold: &PlaceCold, mv: &PlaceMv) -> bool {
        for p in &cold.pushes {
            if touches(mv, p.a) || touches(mv, p.b) {
                let (ax, ay) = mv.new_pos(p.a, hot);
                let (bx, by) = mv.new_pos(p.b, hot);
                let (ai, bi) = (p.a as usize, p.b as usize);
                let gx = (ax - bx).abs() - (cold.hw[ai] + cold.hw[bi]);
                let gy = (ay - by).abs() - (cold.hh[ai] + cold.hh[bi]);
                if gx.max(gy) < p.gap_nm {
                    return false;
                }
            }
        }
        true
    }
}

pub struct SymSaCore;

impl SymSaCore {
    fn t_bounds(cells: &[u32], pos: &[f32], half: &[f32], span: f32) -> (f32, f32) {
        let (mut lo, mut hi) = (f32::MIN, f32::MAX);
        for &c in cells {
            let ci = c as usize;
            lo = lo.max(half[ci] - pos[ci]);
            hi = hi.min(span - half[ci] - pos[ci]);
        }
        (lo.min(0.0), hi.max(0.0))
    }
}

impl Core<PlaceDomain> for SymSaCore {
    type Scratch = ();

    fn scratch(&self, _: &PlaceHot, _: &PlaceCold) {}

    fn propose(
        &self,
        hot: &PlaceHot,
        cold: &PlaceCold,
        _sc: &mut (),
        ctl: &Control,
        rng: &mut SplitMix64,
    ) -> Option<PlaceMv> {
        let n = hot.x.len();
        if n == 0 {
            return None;
        }
        let span = cold.die.0.max(cold.die.1);
        let r = ctl.range * span;
        let c = rng.below(n) as u32;
        let g = cold.sym.group_of[c as usize];

        let mut mv = PlaceMv::default();
        if g != NONE {
            let members = &cold.sym.groups[g as usize];
            if rng.f32() < 0.5 {
                let (lox, hix) = Self::t_bounds(members, &hot.x, &cold.hw, cold.die.0);
                let (loy, hiy) = Self::t_bounds(members, &hot.y, &cold.hh, cold.die.1);
                let dx = rng.centered(r).clamp(lox, hix);
                let dy = rng.centered(r).clamp(loy, hiy);
                for &m in members {
                    if cold.sym.partner[m as usize] != NONE {
                        mv.idx.push(m);
                        mv.nx.push(hot.x[m as usize] + dx);
                        mv.ny.push(hot.y[m as usize] + dy);
                    }
                }
                mv.idx.sort_unstable();
                mv.idx.dedup();
                mv.nx = mv.idx.iter().map(|&m| hot.x[m as usize] + dx).collect();
                mv.ny = mv.idx.iter().map(|&m| hot.y[m as usize] + dy).collect();
                mv.axis = Some((g, hot.axis[g as usize] + dx));
            } else {
                let m = members[rng.below(members.len())];
                let mi = m as usize;
                let p = cold.sym.partner[mi];
                let ax = hot.axis[g as usize];
                if p == m {
                    let (loy, hiy) = Self::t_bounds(&[m], &hot.y, &cold.hh, cold.die.1);
                    let dy = rng.centered(r).clamp(loy, hiy);
                    mv.idx.push(m);
                    mv.nx.push(ax);
                    mv.ny.push(hot.y[mi] + dy);
                } else {
                    let pi = p as usize;
                    let dx_lo = (cold.hw[mi] - hot.x[mi])
                        .max(hot.x[pi] - (cold.die.0 - cold.hw[pi]));
                    let dx_hi = (cold.die.0 - cold.hw[mi] - hot.x[mi])
                        .min(hot.x[pi] - cold.hw[pi]);
                    let (loy, hiy) =
                        Self::t_bounds(&[m, p], &hot.y, &cold.hh, cold.die.1);
                    let dx = rng.centered(r).clamp(dx_lo.min(0.0), dx_hi.max(0.0));
                    let dy = rng.centered(r).clamp(loy, hiy);
                    let nx = hot.x[mi] + dx;
                    let ny = hot.y[mi] + dy;
                    mv.idx.push(m);
                    mv.nx.push(nx);
                    mv.ny.push(ny);
                    mv.idx.push(p);
                    mv.nx.push(2.0 * ax - nx);
                    mv.ny.push(ny);
                }
            }
        } else if rng.f32() < 0.75 {
            let ci = c as usize;
            let (lox, hix) = Self::t_bounds(&[c], &hot.x, &cold.hw, cold.die.0);
            let (loy, hiy) = Self::t_bounds(&[c], &hot.y, &cold.hh, cold.die.1);
            mv.idx.push(c);
            mv.nx.push(hot.x[ci] + rng.centered(r).clamp(lox, hix));
            mv.ny.push(hot.y[ci] + rng.centered(r).clamp(loy, hiy));
        } else {
            let o = rng.below(n) as u32;
            if o == c || cold.sym.group_of[o as usize] != NONE {
                let ci = c as usize;
                mv.idx.push(c);
                mv.nx.push(hot.x[ci]);
                mv.ny.push(hot.y[ci]);
            } else {
                let (ci, oi) = (c as usize, o as usize);
                mv.idx.push(c);
                mv.nx
                    .push(hot.x[oi].clamp(cold.hw[ci], cold.die.0 - cold.hw[ci]));
                mv.ny
                    .push(hot.y[oi].clamp(cold.hh[ci], cold.die.1 - cold.hh[ci]));
                mv.idx.push(o);
                mv.nx
                    .push(hot.x[ci].clamp(cold.hw[oi], cold.die.0 - cold.hw[oi]));
                mv.ny
                    .push(hot.y[ci].clamp(cold.hh[oi], cold.die.1 - cold.hh[oi]));
            }
        }
        Some(mv)
    }

    fn commit(&self, hot: &mut PlaceHot, _cold: &PlaceCold, _sc: &mut (), mv: &PlaceMv) {
        for (k, &c) in mv.idx.iter().enumerate() {
            let ci = c as usize;
            let old = (hot.x[ci], hot.y[ci]);
            hot.x[ci] = mv.nx[k];
            hot.y[ci] = mv.ny[k];
            hot.buckets.move_cell(c, old, (mv.nx[k], mv.ny[k]));
        }
        if let Some((g, ax)) = mv.axis {
            hot.axis[g as usize] = ax;
        }
    }
}

pub struct SaStop {
    pub max_iters: u32,
    pub min_iters: u32,
    pub min_accept_rate: f32,
}

impl Stop for SaStop {
    fn done(&self, t: &Telemetry, open: usize) -> bool {
        if t.iters >= self.max_iters {
            return true;
        }
        t.iters >= self.min_iters
            && t.epoch_accept_rate < self.min_accept_rate
            && t.overflow <= 1e-4
            && open == 0
    }
}

pub fn run_detailed<Lg: Ledger<PlaceDomain>>(
    hot: &mut PlaceHot,
    cold: &PlaceCold,
    ledger: &mut Lg,
    cfg: &DetailedCfg,
    rng: &mut SplitMix64,
) -> Telemetry {
    let n = hot.x.len().max(1);

    let core = SymSaCore;
    let cost = SaCost;
    let probe = Control { temp: 0.0, step: 0.0, range: cfg.range0, moves_per_step: 0 };
    let mut sum = 0.0f64;
    let mut cnt = 0u32;
    let mut sc = ();
    for _ in 0..128 {
        if let Some(mv) = core.propose(hot, cold, &mut sc, &probe, rng) {
            sum += cost.delta(hot, cold, &mv).abs();
            cnt += 1;
        }
    }
    let t0 = (sum / f64::from(cnt.max(1))).max(1.0) * 10.0;

    let cost0 = cost_at(cold, &hot.x, &hot.y).max(1.0);
    let area: f64 = cold.hw.iter().zip(&cold.hh).map(|(w, h)| 4.0 * f64::from(w * h)).sum();
    let w0 = cost0 / area.max(1.0);

    let stage = Stage {
        cost: SaCost,
        density: OverlapDensity,
        legality: HardGaps,
        weights: RampWeights { w0, gain: 1.08, w_max: w0 * 1e4 },
        core: SymSaCore,
        accept: Metropolis,
        schedule: Geometric {
            t0,
            alpha: cfg.alpha,
            range0: cfg.range0,
            range_decay: 0.96,
            range_min: cold.grid / cold.die.0.max(cold.die.1),
            moves_per_step: cfg.moves_per_cell * n as u32,
        },
        stop: SaStop {
            max_iters: cfg.max_iters,
            min_iters: cfg.min_iters,
            min_accept_rate: cfg.min_accept_rate,
        },
        _d: PhantomData,
    };
    stage.run(hot, cold, ledger, rng)
}
