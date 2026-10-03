//! Detailed placement: bounded-move Metropolis SA over the coarse layout with
//! displacement, swap, rotate, variant-reshape and DTI-branch-flip moves.
//!
//! Every move goes through [`Sa::trial`]: a move that breaks a hard equality is
//! repaired by projecting the broken batch (a symmetry partner follows its
//! mirror), then gated lexicographically on `(violating hard batches, Σ hard
//! residual, clearance encroachment nm², Θ)`. Metropolis only votes on the PEX
//! tier (pin HPWL + priced analog cost). [`legalize::separate_overlaps`] closes
//! any residue.

pub mod legalize;
pub mod locks;

use analog::Requirements;
use pnr_core::ids::BranchId;
use pnr_core::{Layout, Macro, Orient, Report};

use gp::mechanics::{
    analog_cost, analog_phi, analog_theta, choose_variants, encroach,
    hpwl, report, snap, variant_extents, Nets, SplitMix64,
};

const MAX_ITERS: u32 = 220;
/// Inner moves per epoch = `MOVES_PER_CELL · n`.
const MOVES_PER_CELL: usize = 60;
const ALPHA: f64 = 0.93;
/// Initial displacement window, fraction of the die span.
const RANGE0: f32 = 0.4;
const RANGE_DECAY: f32 = 0.96;
/// Clearance-inflated area / move-region area floor: the region the SA may use
/// is grown until everything fits at this fill.
const REGION_FILL: f64 = 0.5;
/// Runaway guard for the terminal legalizer (it exits early when clean/stalled).
const LEGALIZE_SWEEPS: u32 = 64;

/// What the anneal did, for measurement only: no counter feeds a decision, so
/// the placement is the same with or without anyone reading them.
#[derive(Clone, Copy, Debug, Default)]
pub struct PlaceStats {
    /// Temperature steps (epochs) run.
    pub temps: u32,
    /// `Sa::trial` calls: every move that reached the gate.
    pub proposals: u64,
    /// Proposals kept.
    pub accepted: u64,
    /// Codes the decoder could not realise; `None` (not measured) until
    /// PLC-08 adds a decoder.
    pub decode_fail: Option<u64>,
    /// Distinct matched pairs whose variant spaces differ, so they cannot be
    /// shape-locked ([`locks::Locks::incompatible`]).
    pub matched_incompatible: Option<u32>,
}

/// The mutable columns a move can touch, for rollback.
#[derive(Default)]
struct Snap {
    x: Vec<i32>,
    y: Vec<i32>,
    hw: Vec<i32>,
    hh: Vec<i32>,
    variant: Vec<u16>,
    orient: Vec<Orient>,
    axis: Vec<i32>,
    branch: Vec<bool>,
}

impl Snap {
    fn save(&mut self, l: &Layout) {
        self.x.clone_from(&l.x);
        self.y.clone_from(&l.y);
        self.hw.clone_from(&l.hw);
        self.hh.clone_from(&l.hh);
        self.variant.clone_from(&l.variant);
        self.orient.clone_from(&l.orient);
        self.axis.clone_from(&l.axis);
        self.branch.clone_from(&l.branch);
    }

    fn restore(&self, l: &mut Layout) {
        l.x.clone_from(&self.x);
        l.y.clone_from(&self.y);
        l.hw.clone_from(&self.hw);
        l.hh.clone_from(&self.hh);
        l.variant.clone_from(&self.variant);
        l.orient.clone_from(&self.orient);
        l.axis.clone_from(&self.axis);
        l.branch.clone_from(&self.branch);
    }

    /// Exchange the geometry columns with `l` (O(1)), to measure the pre-move state.
    fn swap_geometry(&mut self, l: &mut Layout) {
        std::mem::swap(&mut self.x, &mut l.x);
        std::mem::swap(&mut self.y, &mut l.y);
        std::mem::swap(&mut self.hw, &mut l.hw);
        std::mem::swap(&mut self.hh, &mut l.hh);
    }
}

/// SA state. `nets` is owned because a reshape patches pin offsets in place.
struct Sa<'a> {
    nets: Nets,
    cell_nets: Vec<Vec<u32>>,
    reqs: &'a Requirements<Layout>,
    /// Read-only during the anneal: Metropolis needs a fixed energy.
    prices: &'a gp::Prices,
    fixed: &'a [bool],
    clearance: i32,
    grid: i32,
    snap: Snap,
    moved: Vec<usize>,
    stats: PlaceStats,
}

impl<'a> Sa<'a> {
    fn new(
        nets: Nets,
        n: usize,
        reqs: &'a Requirements<Layout>,
        prices: &'a gp::Prices,
        fixed: &'a [bool],
        rules: gp::Rules,
    ) -> Self {
        Sa {
            cell_nets: nets.cell_nets(n),
            nets,
            reqs,
            prices,
            fixed,
            clearance: rules.clearance,
            grid: rules.grid,
            snap: Snap::default(),
            moved: Vec::new(),
            stats: PlaceStats::default(),
        }
    }

    fn is_fixed(&self, i: usize) -> bool {
        self.fixed.get(i).copied().unwrap_or(false)
    }

    /// PEX tier: pin HPWL + priced analog cost.
    fn pex(&self, l: &Layout) -> f64 {
        hpwl(&self.nets, l) + f64::from(analog_cost(self.reqs, l, self.prices))
    }

    /// Clearance encroachment of every pair with a member in `self.moved`.
    fn encroach_moved(&self, l: &Layout) -> f64 {
        let mut t = 0.0;
        for (i, &c) in self.moved.iter().enumerate() {
            for b in 0..l.x.len() {
                if b != c && !self.moved[..i].contains(&b) {
                    t += encroach(l, c, b, self.clearance);
                }
            }
        }
        t
    }

    /// Apply `mutate`, repair any newly broken hard batch by projection, and
    /// keep the result iff [`accept`] says so; otherwise roll back. Pairs with
    /// no moved member cancel in the encroachment comparison, so it is exact.
    fn trial(
        &mut self,
        l: &mut Layout,
        rng: &mut SplitMix64,
        temp: f64,
        mutate: impl FnOnce(&mut Layout, &mut Nets, &[Vec<u32>]),
    ) -> bool {
        self.stats.proposals += 1;
        self.snap.save(l);
        let phi0 = analog_phi(self.reqs, l);
        let theta0 = analog_theta(self.reqs, l);
        let pex0 = self.pex(l);
        mutate(l, &mut self.nets, &self.cell_nets);

        let mut phi1 = analog_phi(self.reqs, l);
        if phi1.0 > phi0.0 {
            for b in &self.reqs.hard {
                if b.violations(l) > 0 {
                    b.project(l, self.grid);
                }
            }
            phi1 = analog_phi(self.reqs, l);
        }

        let s = &self.snap;
        self.moved.clear();
        self.moved.extend((0..l.x.len()).filter(|&i| {
            l.x[i] != s.x[i] || l.y[i] != s.y[i] || l.hw[i] != s.hw[i] || l.hh[i] != s.hh[i]
        }));
        let ov1 = self.encroach_moved(l);
        self.snap.swap_geometry(l);
        let ov0 = self.encroach_moved(l);
        self.snap.swap_geometry(l);

        let before = (phi0.0, phi0.1, ov0, theta0);
        let after = (phi1.0, phi1.1, ov1, analog_theta(self.reqs, l));
        if accept(before, after, self.pex(l) - pex0, temp, rng) {
            self.stats.accepted += 1;
            return true;
        }
        self.snap.restore(l);
        false
    }
}

/// Refine `coarse` into a legal placement, seed-deterministic, with the
/// anneal's [`PlaceStats`].
///
/// `net_weight[NetId]` weights each net's HPWL (see [`gp::net_weights`]).
/// `macros[i]` supplies cell `i`'s pins when `variants[i]` has no alternative
/// for `coarse.variant[i]`. `fixed[i]` draws cell `i` as given (no reshape, no
/// rotation); its position is placed like any cell's.
/// `locks` turns and reshapes matched cells as one set (PLC-03).
#[allow(clippy::too_many_arguments)]
pub fn place(
    coarse: &Layout,
    macros: &[Macro],
    variants: &[gp::VariantSpace],
    reqs: &Requirements<Layout>,
    fixed: &[bool],
    locks: &locks::Locks,
    prices: &mut gp::Prices,
    rules: gp::Rules,
    net_weight: &[f32],
    seed: u64,
) -> (Layout, Report, PlaceStats) {
    let gp::Rules { grid, clearance } = rules;
    let n = coarse.x.len();
    let mut rng = SplitMix64::new(seed);
    let mut l = Layout {
        x: coarse.x.clone(),
        y: coarse.y.clone(),
        hw: coarse.hw.clone(),
        hh: coarse.hh.clone(),
        variant: coarse.variant.clone(),
        axis: coarse.axis.clone(),
        branch: coarse.branch.clone(),
        groups: coarse.groups.clone(),
        orient: coarse.orient.clone(),
        power_uw: coarse.power_uw.clone(),
        temp_mc: coarse.temp_mc.clone(),
        units: coarse.units.clone(),
    };
    l.refresh_temps();

    let branch_ids = seed_branches(reqs, &mut l.branch);
    let sym = sym_groups(reqs, n);

    prices.bind(reqs);
    // Nets from the geometry `l.variant` names, so HPWL scores real pins.
    let nets = Nets::from_macros(&choose_variants(macros, variants, &l.variant)).weigh(net_weight);
    if n == 0 {
        let rep = report(&nets, reqs, &l, prices, clearance);
        return (l, rep, PlaceStats { matched_incompatible: Some(locks.incompatible), ..Default::default() });
    }

    // Move region: the coarse footprint bbox, grown about its centre until the
    // clearance-inflated cells fit at `REGION_FILL`.
    let (mut xmin, mut ymin, mut xmax, mut ymax) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    let mut need = 0.0f64;
    for i in 0..n {
        xmin = xmin.min(l.x[i] - l.hw[i]);
        ymin = ymin.min(l.y[i] - l.hh[i]);
        xmax = xmax.max(l.x[i] + l.hw[i]);
        ymax = ymax.max(l.y[i] + l.hh[i]);
        need += f64::from(2 * l.hw[i] + clearance) * f64::from(2 * l.hh[i] + clearance);
    }
    let side = (need / REGION_FILL).sqrt() as i32;
    let grow = |lo: &mut i32, hi: &mut i32| {
        let short = side - (*hi - *lo);
        if short > 0 {
            *lo -= short / 2;
            *hi += short - short / 2;
        }
    };
    grow(&mut xmin, &mut xmax);
    grow(&mut ymin, &mut ymax);
    let span = (xmax - xmin).max(ymax - ymin).max(1) as f32;
    let clamp_x = |c: i32, half: i32| c.clamp(xmin + half, (xmax - half).max(xmin + half));
    let clamp_y = |c: i32, half: i32| c.clamp(ymin + half, (ymax - half).max(ymin + half));

    let mut sa = Sa::new(nets, n, reqs, prices, fixed, rules);
    sa.stats.matched_incompatible = Some(locks.incompatible);

    // t0 = 0.02 · mean |ΔPEX| over probe moves: refine gp, don't randomise it.
    let mut range = RANGE0;
    let probe_r = (range * span) as i32 as f32;
    let pex0 = sa.pex(&l);
    let mut sum = 0.0f64;
    for _ in 0..128 {
        let c = rng.below(n);
        let (ox, oy) = (l.x[c], l.y[c]);
        l.x[c] = clamp_x(ox + rng.centered(probe_r) as i32, l.hw[c]);
        l.y[c] = clamp_y(oy + rng.centered(probe_r) as i32, l.hh[c]);
        sum += (sa.pex(&l) - pex0).abs();
        (l.x[c], l.y[c]) = (ox, oy);
    }
    let mut temp = (sum / 128.0).max(1.0) * 0.02;

    let can_rotate = l.orient.len() == n;
    let can_reshape = variants.len() == n && l.variant.len() == n;
    let moves_per_epoch = MOVES_PER_CELL * n;
    let range_min = grid as f32 / span;

    // A fixed schedule: stopping at a low accept rate (the old early exit)
    // cost ota 5% C and 24% area. Lampaert 1999 pp.117–119 (T0 = −ΔC⁺/ln 0.6,
    // stop after 10 flat chains) was measured 2026-09: ota C −12% but
    // rc_filter C +7%, bjt_mirror area +22%; not adopted.
    // ponytail: ~4x dp time; revisit if runtime binds.
    for _ in 0..MAX_ITERS {
        let r = (range * span) as i32 as f32;
        for _ in 0..moves_per_epoch {
            // 70% displace, 20% swap, 2.5% branch flip, 5% reshape, else rotate.
            let roll = rng.f32();
            let c = rng.below(n);
            let _ = if !sym.is_empty() && roll < 0.08 {
                // Compound moves: every mirror equation holds before and after,
                // so no projection drags the rest of the stage.
                let g = &sym[rng.below(sym.len())];
                let (a, b) = g.pairs[rng.below(g.pairs.len())];
                match rng.below(3) {
                    0 => {
                        let (dx, dy) = (snap(rng.centered(r) as i32, grid), snap(rng.centered(r) as i32, grid));
                        try_group_shift(&mut sa, &mut l, &mut rng, temp, g, dx, dy, &clamp_x, &clamp_y)
                    }
                    1 => {
                        let d = snap(rng.centered(r) as i32 / 2, grid);
                        try_pair_expand(&mut sa, &mut l, &mut rng, temp, a, b, d, &clamp_x)
                    }
                    _ => a != b && try_pair_swap(&mut sa, &mut l, &mut rng, temp, a, b),
                }
            } else if roll < 0.70 {
                let nx = clamp_x(l.x[c] + rng.centered(r) as i32, l.hw[c]);
                let ny = clamp_y(l.y[c] + rng.centered(r) as i32, l.hh[c]);
                try_move(&mut sa, &mut l, &mut rng, temp, c, nx, ny)
            } else if roll < 0.90 {
                let o = rng.below(n);
                o != c && try_swap(&mut sa, &mut l, &mut rng, temp, c, o, &clamp_x, &clamp_y)
            } else if !branch_ids.is_empty() && roll < 0.925 {
                let bid = usize::from(branch_ids[rng.below(branch_ids.len())].0);
                try_branch(&mut sa, &mut l, &mut rng, temp, bid)
            } else if can_reshape && roll >= 0.95 {
                try_reshape(&mut sa, &mut l, &mut rng, temp, &locks.members(c, true), variants, &clamp_x, &clamp_y)
            } else {
                let set = locks.members(c, false);
                can_rotate
                    && !set.iter().any(|&m| sa.is_fixed(m))
                    && try_rotate(&mut sa, &mut l, &mut rng, temp, &set, &clamp_x, &clamp_y)
            };
        }

        // One exact projection per epoch for batches violated before any move.
        project_hard(reqs, &mut l, grid);
        // Thermal field is global: refresh per epoch, never per move.
        l.refresh_temps();

        temp *= ALPHA;
        range = (range * RANGE_DECAY).max(range_min);
        sa.stats.temps += 1;
    }

    for a in &mut l.axis {
        *a = snap(*a, grid);
    }
    for i in 0..n {
        l.x[i] = snap(l.x[i], grid);
        l.y[i] = snap(l.y[i], grid);
    }
    // Grid snap can shave a clearance by a few nm; the legalizer restores it.
    legalize::separate_overlaps(&mut l, reqs, grid, clearance, LEGALIZE_SWEEPS);
    l.refresh_temps();

    let Sa { nets, stats, .. } = sa;
    let rep = report(&nets, reqs, &l, prices, clearance);
    (l, rep, stats)
}

/// Disjunctive branches (DtiBand share/isolate): grow `branch` to the highest
/// id and seed each from its recognised structure. Returns the ids, sorted.
fn seed_branches(reqs: &Requirements<Layout>, branch: &mut Vec<bool>) -> Vec<BranchId> {
    let mut branch_seeds: Vec<(BranchId, bool)> = Vec::new();
    for b in &reqs.hard {
        b.branches(&mut branch_seeds);
    }
    branch_seeds.sort_unstable_by_key(|&(id, _)| id.0);
    branch_seeds.dedup();
    if let Some(&(hi, _)) = branch_seeds.last() {
        if branch.len() <= usize::from(hi.0) {
            branch.resize(usize::from(hi.0) + 1, false);
        }
    }
    for &(id, s) in &branch_seeds {
        branch[usize::from(id.0)] = s;
    }
    branch_seeds.iter().map(|&(id, _)| id).collect()
}

/// Project every violated hard batch onto its feasible set and roll the whole
/// sweep back if Φ rose. Returns whether it kept.
fn project_hard(reqs: &Requirements<Layout>, l: &mut Layout, grid: i32) -> bool {
    let before = analog_phi(reqs, l);
    if before.0 == 0 {
        return false;
    }
    let (px, py, paxis) = (l.x.clone(), l.y.clone(), l.axis.clone());
    for batch in &reqs.hard {
        // Re-checked per batch: a satisfied SymmetryGroup would still re-average its axis.
        if batch.violations(l) > 0 {
            batch.project(l, grid);
        }
    }
    if analog_phi(reqs, l) > before {
        l.x = px;
        l.y = py;
        l.axis = paxis;
        return false;
    }
    l.refresh_temps();
    true
}

/// Lexicographic acceptance: a change in the gate key decides outright (lower
/// wins, even at a PEX cost); only a tie lets Metropolis judge `d_pex`.
#[inline]
fn accept(before: (usize, f64, f64, f64), after: (usize, f64, f64, f64), d_pex: f64, temp: f64, rng: &mut SplitMix64) -> bool {
    if after != before {
        return after < before;
    }
    d_pex <= 0.0 || (temp > 0.0 && rng.f64() < (-d_pex / temp).exp())
}

fn try_move(sa: &mut Sa, l: &mut Layout, rng: &mut SplitMix64, temp: f64, c: usize, nx: i32, ny: i32) -> bool {
    sa.trial(l, rng, temp, |l, _, _| {
        l.x[c] = nx;
        l.y[c] = ny;
    })
}

#[allow(clippy::too_many_arguments)]
fn try_swap(
    sa: &mut Sa,
    l: &mut Layout,
    rng: &mut SplitMix64,
    temp: f64,
    c: usize,
    o: usize,
    clamp_x: &impl Fn(i32, i32) -> i32,
    clamp_y: &impl Fn(i32, i32) -> i32,
) -> bool {
    sa.trial(l, rng, temp, |l, _, _| {
        let (cx, cy) = (clamp_x(l.x[o], l.hw[c]), clamp_y(l.y[o], l.hh[c]));
        let (ox, oy) = (clamp_x(l.x[c], l.hw[o]), clamp_y(l.y[c], l.hh[o]));
        (l.x[c], l.y[c], l.x[o], l.y[o]) = (cx, cy, ox, oy);
    })
}

/// One symmetry axis and the mirror pairs sharing it (self-pairs `a == b` sit
/// on the axis).
struct SymGroup {
    axis: usize,
    pairs: Vec<(usize, usize)>,
    members: Vec<usize>,
}

fn sym_groups(reqs: &Requirements<Layout>, n: usize) -> Vec<SymGroup> {
    let mut raw = Vec::new();
    for b in &reqs.hard {
        b.mirror_pairs(&mut raw);
    }
    let mut out: Vec<SymGroup> = Vec::new();
    for (a, b, axis) in raw {
        let (a, b, axis) = (a as usize, b as usize, usize::from(axis));
        if a >= n || b >= n {
            continue;
        }
        let i = out.iter().position(|g| g.axis == axis).unwrap_or_else(|| {
            out.push(SymGroup { axis, pairs: Vec::new(), members: Vec::new() });
            out.len() - 1
        });
        let g = &mut out[i];
        g.pairs.push((a, b));
        for m in [a, b] {
            if !g.members.contains(&m) {
                g.members.push(m);
            }
        }
    }
    out
}

/// Translate a whole symmetric group and its axis by `(dx, dy)`. Rejected
/// outright if the die boundary would clamp any member (that would break the
/// mirror equations the move exists to keep).
#[allow(clippy::too_many_arguments)]
fn try_group_shift(
    sa: &mut Sa,
    l: &mut Layout,
    rng: &mut SplitMix64,
    temp: f64,
    g: &SymGroup,
    dx: i32,
    dy: i32,
    clamp_x: &impl Fn(i32, i32) -> i32,
    clamp_y: &impl Fn(i32, i32) -> i32,
) -> bool {
    let fits = g.members.iter().all(|&m| {
        clamp_x(l.x[m] + dx, l.hw[m]) == l.x[m] + dx && clamp_y(l.y[m] + dy, l.hh[m]) == l.y[m] + dy
    });
    if !fits || (dx, dy) == (0, 0) {
        return false;
    }
    sa.trial(l, rng, temp, |l, _, _| {
        for &m in &g.members {
            l.x[m] += dx;
            l.y[m] += dy;
        }
        if let Some(a) = l.axis.get_mut(g.axis) {
            *a += dx;
        }
    })
}

/// Move a mirror pair `d` further apart (or closer), axis fixed: `x_a + x_b`
/// is unchanged, so the pair stays mirrored.
#[allow(clippy::too_many_arguments)]
fn try_pair_expand(
    sa: &mut Sa,
    l: &mut Layout,
    rng: &mut SplitMix64,
    temp: f64,
    a: usize,
    b: usize,
    d: i32,
    clamp_x: &impl Fn(i32, i32) -> i32,
) -> bool {
    let (lo, hi) = if l.x[a] <= l.x[b] { (a, b) } else { (b, a) };
    let (nlo, nhi) = (l.x[lo] - d, l.x[hi] + d);
    if a == b || d == 0 || nlo >= nhi || clamp_x(nlo, l.hw[lo]) != nlo || clamp_x(nhi, l.hw[hi]) != nhi {
        return false;
    }
    sa.trial(l, rng, temp, |l, _, _| {
        l.x[lo] = nlo;
        l.x[hi] = nhi;
    })
}

/// Exchange a mirror pair's sides: still mirrored, the other half of the
/// circuit now faces each neighbour (routing and gradient exposure change).
fn try_pair_swap(sa: &mut Sa, l: &mut Layout, rng: &mut SplitMix64, temp: f64, a: usize, b: usize) -> bool {
    sa.trial(l, rng, temp, |l, _, _| {
        l.x.swap(a, b);
        l.y.swap(a, b);
    })
}

/// Flip one DTI share/isolate commitment. Geometry is untouched, so on a legal
/// pair the decision lands on `DtiBand::cost` in the PEX tier; mid-band, a flip
/// toward the nearer exit lowers Φ's margin and is taken outright.
fn try_branch(sa: &mut Sa, l: &mut Layout, rng: &mut SplitMix64, temp: f64, bid: usize) -> bool {
    sa.trial(l, rng, temp, |l, _, _| l.branch[bid] = !l.branch[bid])
}

/// One 90° step within the current reflection class; never introduces a mirror.
fn quarter_turn(o: Orient) -> Orient {
    match o {
        Orient::R0 => Orient::R90,
        Orient::R90 => Orient::R180,
        Orient::R180 => Orient::R270,
        Orient::R270 => Orient::R0,
        Orient::Mx => Orient::Mx90,
        Orient::Mx90 => Orient::Mx180,
        Orient::Mx180 => Orient::Mx270,
        Orient::Mx270 => Orient::Mx,
    }
}

/// Turn a whole orient set a quarter, in one trial; `hw`/`hh` swap with the
/// orientation (`Layout::orient`). No move introduces a mirror, so a matched
/// set keeps one orientation: channels stay parallel and S→D current runs the
/// same way across it.
fn try_rotate(
    sa: &mut Sa,
    l: &mut Layout,
    rng: &mut SplitMix64,
    temp: f64,
    set: &[usize],
    clamp_x: &impl Fn(i32, i32) -> i32,
    clamp_y: &impl Fn(i32, i32) -> i32,
) -> bool {
    sa.trial(l, rng, temp, |l, _, _| {
        for &c in set {
            l.orient[c] = quarter_turn(l.orient[c]);
            (l.hw[c], l.hh[c]) = (l.hh[c], l.hw[c]);
            l.x[c] = clamp_x(l.x[c], l.hw[c]);
            l.y[c] = clamp_y(l.y[c], l.hh[c]);
        }
    })
}

/// Swap a whole shape set (first member `set[0]`) to one uniformly drawn other
/// variant, in one trial. Extents follow the new bbox (transposed under a
/// turned orient) and the pin offsets are patched before pricing, so the move
/// is scored on where pins land. Refused when any member is fixed or has a
/// different variant count.
#[allow(clippy::too_many_arguments)]
fn try_reshape(
    sa: &mut Sa,
    l: &mut Layout,
    rng: &mut SplitMix64,
    temp: f64,
    set: &[usize],
    variants: &[gp::VariantSpace],
    clamp_x: &impl Fn(i32, i32) -> i32,
    clamp_y: &impl Fn(i32, i32) -> i32,
) -> bool {
    let c = set[0];
    let depth = variants[c].alternatives.len();
    if depth < 2 || set.iter().any(|&m| sa.is_fixed(m) || variants[m].alternatives.len() != depth) {
        return false;
    }
    let cur = l.variant[c] as usize;
    let draw = rng.below(depth - 1);
    let next = if cur < depth && draw >= cur { draw + 1 } else { draw };

    let ok = sa.trial(l, rng, temp, |l, nets, cell_nets| {
        for &m in set {
            let alt = &variants[m].alternatives[next];
            let (w, h) = variant_extents(alt);
            l.variant[m] = next as u16;
            (l.hw[m], l.hh[m]) = match l.orient.get(m) {
                Some(o) if o.swaps_axes() => (h, w),
                _ => (w, h),
            };
            l.x[m] = clamp_x(l.x[m], l.hw[m]);
            l.y[m] = clamp_y(l.y[m], l.hh[m]);
            nets.reshape_cell(m, &cell_nets[m], alt);
        }
    });
    if !ok {
        for &m in set {
            if let Some(alt) = variants[m].alternatives.get(l.variant[m] as usize) {
                sa.nets.reshape_cell(m, &sa.cell_nets[m], alt);
            }
        }
    }
    ok
}

#[cfg(test)]
mod tests;
