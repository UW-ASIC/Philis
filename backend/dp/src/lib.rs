//! Detailed placement: bounded-move Metropolis SA over the coarse layout with
//! displacement, swap, rotate, variant-reshape and DTI-branch-flip moves.
//!
//! Every move goes through [`Sa::trial`]: a move that breaks a hard equality is
//! repaired by projecting the broken batch (a symmetry partner follows its
//! mirror), then gated lexicographically on `(violating hard batches, Φ margin +
//! clearance encroachment, Θ)`. Metropolis only votes on the PEX tier (pin HPWL
//! + priced analog cost). [`legalize::separate_overlaps`] closes any residue.

pub mod legalize;

use analog::Requirements;
use pnr_core::ids::BranchId;
use pnr_core::{Layout, Macro, Orient, Report};

use gp::mechanics::{
    analog_cost, analog_phi, analog_theta, choose_variants, encroach,
    hpwl, report, snap, variant_extents, Nets, SplitMix64,
};
use gp::CLEARANCE_NM;

const MAX_ITERS: u32 = 220;
/// Inner moves per epoch = `MOVES_PER_CELL · n`.
const MOVES_PER_CELL: usize = 60;
const ALPHA: f64 = 0.93;
/// Initial displacement window, fraction of the die span.
const RANGE0: f32 = 0.4;
const RANGE_DECAY: f32 = 0.96;
const GRID: i32 = 5;
/// Clearance-inflated area / move-region area floor: the region the SA may use
/// is grown until everything fits at this fill.
const REGION_FILL: f64 = 0.5;
/// Runaway guard for the terminal legalizer (it exits early when clean/stalled).
const LEGALIZE_SWEEPS: u32 = 64;

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
    snap: Snap,
    moved: Vec<usize>,
}

impl<'a> Sa<'a> {
    fn new(
        nets: Nets,
        n: usize,
        reqs: &'a Requirements<Layout>,
        prices: &'a gp::Prices,
        fixed: &'a [bool],
        clearance: i32,
    ) -> Self {
        Sa {
            cell_nets: nets.cell_nets(n),
            nets,
            reqs,
            prices,
            fixed,
            clearance,
            snap: Snap::default(),
            moved: Vec::new(),
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
        self.snap.save(l);
        let phi0 = analog_phi(self.reqs, l);
        let theta0 = analog_theta(self.reqs, l);
        let pex0 = self.pex(l);
        mutate(l, &mut self.nets, &self.cell_nets);

        let mut phi1 = analog_phi(self.reqs, l);
        if phi1.0 > phi0.0 {
            for b in &self.reqs.hard {
                if b.violations(l) > 0 {
                    b.project(l, GRID);
                }
            }
            for i in (0..l.x.len()).filter(|&i| self.is_fixed(i)) {
                l.x[i] = self.snap.x[i];
                l.y[i] = self.snap.y[i];
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

        let before = (phi0.0, phi0.1 + ov0, theta0);
        let after = (phi1.0, phi1.1 + ov1, analog_theta(self.reqs, l));
        if accept(before, after, self.pex(l) - pex0, temp, rng) {
            return true;
        }
        self.snap.restore(l);
        false
    }
}

/// Refine `coarse` into a legal placement, seed-deterministic.
///
/// `macros[i]` supplies cell `i`'s pins when `variants[i]` has no alternative
/// for `coarse.variant[i]`. `fixed[i]` pins cell `i` (position and variant).
pub fn place(
    coarse: &Layout,
    macros: &[Macro],
    variants: &[gp::VariantSpace],
    reqs: &Requirements<Layout>,
    fixed: &[bool],
    prices: &mut gp::Prices,
    seed: u64,
) -> (Layout, Report) {
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
    };
    l.refresh_temps();

    // Disjunctive branches (DtiBand share/isolate): size the table to the
    // highest id and seed each from its recognised structure.
    let mut branch_seeds: Vec<(BranchId, bool)> = Vec::new();
    for b in &reqs.hard {
        b.branches(&mut branch_seeds);
    }
    branch_seeds.sort_unstable_by_key(|&(id, _)| id.0);
    branch_seeds.dedup();
    if let Some(&(hi, _)) = branch_seeds.last() {
        if l.branch.len() <= usize::from(hi.0) {
            l.branch.resize(usize::from(hi.0) + 1, false);
        }
    }
    for &(id, s) in &branch_seeds {
        l.branch[usize::from(id.0)] = s;
    }
    let branch_ids: Vec<BranchId> = branch_seeds.iter().map(|&(id, _)| id).collect();

    prices.bind(reqs);
    // Nets from the geometry `l.variant` names, so HPWL scores real pins.
    let nets = Nets::from_macros(&choose_variants(macros, variants, &l.variant));
    if n == 0 {
        let rep = report(&nets, reqs, &l, prices);
        return (l, rep);
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
        need += f64::from(2 * l.hw[i] + CLEARANCE_NM) * f64::from(2 * l.hh[i] + CLEARANCE_NM);
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

    let mut sa = Sa::new(nets, n, reqs, prices, fixed, CLEARANCE_NM);

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
    let range_min = GRID as f32 / span;

    // A fixed schedule: stopping at a low accept rate (the old early exit)
    // cost ota 5% C and 24% area. ponytail: ~4x dp time; revisit if runtime binds.
    for _ in 0..MAX_ITERS {
        let r = (range * span) as i32 as f32;
        for _ in 0..moves_per_epoch {
            // 70% displace, 20% swap, 2.5% branch flip, 5% reshape, else rotate.
            let roll = rng.f32();
            let c = rng.below(n);
            if sa.is_fixed(c) {
                continue;
            }
            let _ = if roll < 0.70 {
                let nx = clamp_x(l.x[c] + rng.centered(r) as i32, l.hw[c]);
                let ny = clamp_y(l.y[c] + rng.centered(r) as i32, l.hh[c]);
                try_move(&mut sa, &mut l, &mut rng, temp, c, nx, ny)
            } else if roll < 0.90 {
                let o = rng.below(n);
                o != c && !sa.is_fixed(o) && try_swap(&mut sa, &mut l, &mut rng, temp, c, o, &clamp_x, &clamp_y)
            } else if !branch_ids.is_empty() && roll < 0.925 {
                let bid = usize::from(branch_ids[rng.below(branch_ids.len())].0);
                try_branch(&mut sa, &mut l, &mut rng, temp, bid)
            } else if can_reshape && roll >= 0.95 {
                try_reshape(&mut sa, &mut l, &mut rng, temp, c, variants, &clamp_x, &clamp_y)
            } else {
                can_rotate && rotatable(&l, c) && try_rotate(&mut sa, &mut l, &mut rng, temp, c, &clamp_x, &clamp_y)
            };
        }

        // One exact projection per epoch for batches violated before any move.
        project_hard(reqs, &mut l, fixed, GRID);
        // Thermal field is global: refresh per epoch, never per move.
        l.refresh_temps();

        temp *= ALPHA;
        range = (range * RANGE_DECAY).max(range_min);
    }

    for a in &mut l.axis {
        *a = snap(*a, GRID);
    }
    for i in 0..n {
        l.x[i] = snap(l.x[i], GRID);
        l.y[i] = snap(l.y[i], GRID);
    }
    // Grid snap can shave a clearance by a few nm; the legalizer restores it.
    legalize::separate_overlaps(&mut l, reqs, fixed, GRID, CLEARANCE_NM, LEGALIZE_SWEEPS);
    l.refresh_temps();

    let Sa { nets, .. } = sa;
    prices.settle(reqs, &l);
    let rep = report(&nets, reqs, &l, prices);
    (l, rep)
}

/// Project every violated hard batch onto its feasible set, restore pinned
/// cells, and roll the whole sweep back if Φ rose. Returns whether it kept.
fn project_hard(reqs: &Requirements<Layout>, l: &mut Layout, fixed: &[bool], grid: i32) -> bool {
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
    for i in 0..l.x.len() {
        if fixed.get(i).copied().unwrap_or(false) {
            l.x[i] = px[i];
            l.y[i] = py[i];
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
fn accept(before: (usize, f64, f64), after: (usize, f64, f64), d_pex: f64, temp: f64, rng: &mut SplitMix64) -> bool {
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

/// Grouped (matched) devices never turn, and no move introduces a mirror, so
/// every matched device keeps its seeded orientation: channels stay parallel
/// and S→D current runs the same way across the set, by construction.
fn rotatable(l: &Layout, c: usize) -> bool {
    !l.groups.iter().any(|g| g.len() > 1 && g.iter().any(|d| d.0 as usize == c))
}

/// Turn `c` a quarter; `hw`/`hh` swap with the orientation (`Layout::orient`).
fn try_rotate(
    sa: &mut Sa,
    l: &mut Layout,
    rng: &mut SplitMix64,
    temp: f64,
    c: usize,
    clamp_x: &impl Fn(i32, i32) -> i32,
    clamp_y: &impl Fn(i32, i32) -> i32,
) -> bool {
    sa.trial(l, rng, temp, |l, _, _| {
        l.orient[c] = quarter_turn(l.orient[c]);
        (l.hw[c], l.hh[c]) = (l.hh[c], l.hw[c]);
        l.x[c] = clamp_x(l.x[c], l.hw[c]);
        l.y[c] = clamp_y(l.y[c], l.hh[c]);
    })
}

/// Swap `c` to one uniformly drawn other variant. Extents follow the new bbox
/// (transposed under a turned orient) and the pin offsets are patched before
/// pricing, so the move is scored on where pins land.
#[allow(clippy::too_many_arguments)]
fn try_reshape(
    sa: &mut Sa,
    l: &mut Layout,
    rng: &mut SplitMix64,
    temp: f64,
    c: usize,
    variants: &[gp::VariantSpace],
    clamp_x: &impl Fn(i32, i32) -> i32,
    clamp_y: &impl Fn(i32, i32) -> i32,
) -> bool {
    let depth = variants[c].alternatives.len();
    if depth < 2 || sa.is_fixed(c) {
        return false;
    }
    let cur = l.variant[c] as usize;
    let draw = rng.below(depth - 1);
    let next = if cur < depth && draw >= cur { draw + 1 } else { draw };

    let ok = sa.trial(l, rng, temp, |l, nets, cell_nets| {
        let alt = &variants[c].alternatives[next];
        let (w, h) = variant_extents(alt);
        l.variant[c] = next as u16;
        (l.hw[c], l.hh[c]) = match l.orient.get(c) {
            Some(o) if o.swaps_axes() => (h, w),
            _ => (w, h),
        };
        l.x[c] = clamp_x(l.x[c], l.hw[c]);
        l.y[c] = clamp_y(l.y[c], l.hh[c]);
        nets.reshape_cell(c, &cell_nets[c], alt);
    });
    if !ok {
        if let Some(alt) = variants[c].alternatives.get(l.variant[c] as usize) {
            sa.nets.reshape_cell(c, &sa.cell_nets[c], alt);
        }
    }
    ok
}

#[cfg(test)]
mod tests;
