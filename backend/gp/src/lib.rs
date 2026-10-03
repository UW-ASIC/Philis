//! Global placement: momentum gradient descent on pin HPWL + bin density + the
//! analog cost of `Requirements`, gated so no step raises hard-rule Φ.
//! Also owns [`Prices`] (augmented-Lagrangian λ/ρ per budget batch) and the
//! shared placement primitives in [`mechanics`] that `dp` reuses.

pub mod mechanics;

use std::collections::BTreeMap;

use analog::Requirements;
use pnr_core::{Layout, Macro, Report};

use mechanics::{
    analog_cost, analog_phi, canvas_side, choose_variants, clamp_to_die, half_extents,
    initial_layout, report, snap, Nets, SplitMix64,
};

/// One placeable cell's pre-drawn alternatives; `layout.variant[i]` indexes
/// `alternatives`.
pub struct VariantSpace {
    pub alternatives: Vec<Macro>,
}

/// Augmented-Lagrangian state per budget batch, carried across epochs by the
/// caller. Batches are keyed by `(kind, ordinal among same-kind batches)` so a
/// price survives batches appended after it.
pub struct Prices {
    priced: BTreeMap<(&'static str, u32), Price>,
    /// `−λ` per positional batch index for this epoch (hot-path read).
    weight: Vec<f32>,
    /// `‖λ_{k+1} − λ_k‖` of the last [`Prices::settle`]; `INFINITY` before one.
    drift: f64,
    /// Dual steps taken: one per epoch (the flow's), none inside `place`.
    steps: u32,
    /// Kinds, each once, whose λ sits at `−LAMBDA_MAX` with a batch still violated
    /// after the last step: the cap, not the layout, is what stopped them.
    saturated: Vec<&'static str>,
}

#[derive(Clone, Copy)]
struct Price {
    /// Multiplier in `[−LAMBDA_MAX, 0]` (`λ ← λ − ρ·g`); `−λ` is the price.
    lambda: f32,
    rho: f32,
    /// Constraint value `g` at the previous dual step, for the did-it-shrink test.
    residual: f32,
}

const RHO_FLOOR: f32 = 0.25;
const RHO_GAIN: f32 = 2.0;
const RHO_MAX: f32 = 64.0;
// ponytail: flat cap so a physically unsatisfiable budget cannot swamp the
// objective; a saturated λ reads as settled in `drift`, so it is reported in
// [`Prices::saturated`] and blocks convergence.
const LAMBDA_MAX: f32 = 64.0;

impl Default for Prices {
    fn default() -> Self {
        Self { priced: BTreeMap::new(), weight: Vec::new(), drift: f64::INFINITY, steps: 0, saturated: Vec::new() }
    }
}

impl Prices {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Multiplier movement of the last dual step; `INFINITY` before the first.
    #[must_use]
    pub fn drift(&self) -> f64 {
        self.drift
    }

    /// Bind carried prices to this epoch's batch order (no dual step).
    pub fn bind(&mut self, reqs: &Requirements<Layout>) {
        debug_assert!(
            reqs.budget.iter().all(|b| reqs.hard.iter().all(|h| h.kind() != b.kind())),
            "gp::Prices: a batch kind is registered in both `hard` and `budget`"
        );
        self.weight = keys(reqs)
            .iter()
            .map(|k| self.priced.get(k).map_or(0.0, |p| -p.lambda))
            .collect();
    }

    /// Projected dual step `λ ← clamp(λ − ρ·g, −LAMBDA_MAX, 0)`, once per
    /// epoch on the layout the epoch is scored on (PLAN §5; `place` only binds).
    /// `g` is dimensionless, a fraction of the batch's own budget: the residual
    /// when violated, else minus the slack, so the price relaxes on slack
    /// (PLAN.md:124). ρ doubles for a residual that did not shrink and halves
    /// on slack, within `[RHO_FLOOR, RHO_MAX]`.
    pub fn settle(&mut self, reqs: &Requirements<Layout>, l: &Layout) {
        let mut drift_sq = 0.0f64;
        self.saturated.clear();
        for (bi, key) in keys(reqs).into_iter().enumerate() {
            let b = &reqs.budget[bi];
            let r = b.residual(l).max(0.0) as f32;
            // Slack only where the batch can measure it: spent fraction
            // (`worst_usage`) else headroom-derived criticality. A batch that
            // reports neither has criticality 1 → g = 0 → price held.
            let g = if r > 0.0 {
                r
            } else {
                b.worst_usage(l).map_or(-(1.0 - b.criticality(l)), |u| (u - 1.0).min(0.0))
            };
            let p = self.priced.entry(key).or_insert(Price {
                lambda: 0.0,
                rho: RHO_FLOOR,
                residual: f32::INFINITY,
            });
            if g > 0.0 && g >= p.residual {
                p.rho = (p.rho * RHO_GAIN).min(RHO_MAX); // stuck → harder
            } else if g <= 0.0 {
                p.rho = (p.rho / RHO_GAIN).max(RHO_FLOOR); // slack → softer
            }
            p.residual = g;
            let next = (p.lambda - p.rho * g).clamp(-LAMBDA_MAX, 0.0);
            drift_sq += f64::from(next - p.lambda).powi(2);
            p.lambda = next;
            if next <= -LAMBDA_MAX && r > 0.0 && !self.saturated.contains(&key.0) {
                self.saturated.push(key.0);
            }
        }
        self.drift = drift_sq.sqrt();
        self.steps += 1;
        self.bind(reqs);
    }

    /// Kinds with a batch still violated and λ at the cap after the last
    /// [`Prices::settle`], each listed once however many batches share it.
    #[must_use]
    pub fn saturated(&self) -> &[&'static str] {
        &self.saturated
    }

    /// Dual steps taken so far.
    #[must_use]
    pub fn steps(&self) -> u32 {
        self.steps
    }

    /// `−λ` for budget batch `bi`; `0.0` when unbound.
    #[inline]
    #[must_use]
    pub(crate) fn weight_of(&self, bi: usize) -> f32 {
        self.weight.get(bi).copied().unwrap_or(0.0)
    }
}

fn keys(reqs: &Requirements<Layout>) -> Vec<(&'static str, u32)> {
    (0..reqs.budget.len())
        .map(|bi| {
            let kind = reqs.budget[bi].kind();
            (kind, reqs.budget[..bi].iter().filter(|o| o.kind() == kind).count() as u32)
        })
        .collect()
}

/// Per-net HPWL weight, indexed by `NetId`, mean `1` over the weighted nets
/// (unweighted = `1`). A net's weight is the fraction of a budget one aF of it
/// spends (Lampaert 1999 eq.2.12–2.13, `ΔP = Σ S·Δx`; CRATES #3): the positive
/// performance sensitivities `sens` (`(net, 1/aF)`, summed over specs) where a
/// spec sees the net, else `1/c_budget` of its class. A tighter budget pulls
/// harder; rails (unbudgeted) keep `1`.
///
/// Placement's HPWL weights, and the routers' (library masks those to budgeted
/// nets).
///
/// ponytail: `1/c_budget` is a class heuristic, not a sensitivity. Measured on
/// ota over seeds 1–5, dropping it from placement first *helped* (mean C
/// 315.0 → 286.9 fF) and, after routing changes, *hurt* (256.1 → 280.3 fF;
/// chain4 +6%, rc_filter +7% at seed 1). Kept on that later data; re-measure
/// over seeds whenever routing changes.
#[must_use]
pub fn net_weights(classes: &[analog::metadata::NetClassification], sens: &[(pnr_core::NetId, f32)]) -> Vec<f32> {
    let mut raw: Vec<Option<f32>> = classes.iter().map(|c| c.c_budget_af.filter(|&b| b > 0).map(|b| 1.0 / b as f32)).collect();
    let mut from_perf = vec![0.0f32; raw.len()];
    for &(n, w) in sens {
        if let Some(s) = from_perf.get_mut(usize::from(n.0)) {
            *s += w.max(0.0);
        }
    }
    for (r, &s) in raw.iter_mut().zip(&from_perf) {
        if s > 0.0 {
            *r = Some(s);
        }
    }
    let known: Vec<f32> = raw.iter().flatten().copied().collect();
    let mean = known.iter().sum::<f32>() / known.len().max(1) as f32;
    raw.iter().map(|r| r.map_or(1.0, |w| w / mean)).collect()
}

/// Process numbers placement needs, from the deck (shared with `dp`).
#[derive(Clone, Copy, Debug)]
pub struct Rules {
    /// Every cell origin snaps to it, nm.
    pub grid: i32,
    /// Edge-to-edge gap between cells, nm: below it wells/implants of
    /// different cells merge.
    pub clearance: i32,
}

const MAX_ITERS: u32 = 500;
const MIN_ITERS: u32 = 60;
const OVERFLOW_TARGET: f32 = 0.15;
const UTILIZATION: f32 = 0.4;
const STEP0: f32 = 0.04;
const STEP_MIN: f32 = 0.002;
const STEP_DECAY: f32 = 0.995;
const MOMENTUM: f32 = 0.85;
const LAMBDA0: f32 = 0.5;
const TARGET_UTIL: f32 = 0.7;
/// Finite-difference probe (nm) for the analog-cost gradient.
const ANALOG_PROBE: i32 = 64;

/// Everything [`place`] reads.
pub struct GpInput<'a> {
    pub macros: &'a [Macro],
    pub variants: &'a [VariantSpace],
    pub assignment: &'a [u16],
    pub reqs: &'a Requirements<Layout>,
    pub rules: Rules,
    pub net_weight: &'a [f32],
    /// Symmetry axes, one per block (`Problem::blocks`).
    pub n_axes: usize,
    /// Per cell, µW; empty = unpowered.
    pub power_uw: &'a [i32],
    pub units: std::sync::Arc<pnr_core::UnitLib>,
    /// `false`: return the initial pile (`GpMode::Pile`).
    pub iterate: bool,
}

/// Coarse placement of `macros`, drawn as `assignment` picks from each
/// `variants[i]` (missing = 0), seed-deterministic. `net_weight[NetId]` weights each net's HPWL (empty =
/// unweighted; see [`net_weights`]). Each block's axis follows the mean
/// midpoint of its hard mirror pairs. The layout carries `power_uw` and `units`
/// throughout, so matched-set thermal terms see the real field; temperatures
/// are refreshed once, on return. `iterate = false` returns the seeded pile
/// from `initial_layout` unrefined, still reported: the baseline that measures
/// what the analytic loop adds (neither mode moves prices; the epoch's one dual
/// step is the caller's, after dp).
pub fn place(inp: &GpInput, prices: &mut Prices, seed: u64) -> (Layout, Report) {
    let &GpInput { macros, variants, assignment, reqs, rules, net_weight, n_axes, power_uw, iterate, .. } = inp;
    let n = macros.len();
    let mut rng = SplitMix64::new(seed);
    // gp does not search variants: it keeps the caller's, dp reshapes.
    let variant: Vec<u16> = (0..n).map(|i| assignment.get(i).copied().unwrap_or(0)).collect();
    let drawn = choose_variants(macros, variants, &variant);
    prices.bind(reqs);

    let (hw, hh) = half_extents(&drawn);
    let side = canvas_side(&hw, &hh, UTILIZATION, rules.grid);
    let mut l = initial_layout(&drawn, variant, side, n_axes, &mut rng);
    if power_uw.len() == n {
        l.power_uw.copy_from_slice(power_uw);
    }
    l.units = inp.units.clone();
    let nets = Nets::from_macros(&drawn).weigh(net_weight);
    if n == 0 || !iterate {
        l.refresh_temps();
        let rep = report(&nets, reqs, &l, prices, rules.clearance);
        return (l, rep);
    }
    let mut pairs = Vec::new();
    for b in &reqs.hard {
        b.mirror_pairs(&mut pairs);
    }

    let mut gx = vec![0.0f32; n];
    let mut gy = vec![0.0f32; n];
    let mut vx = vec![0.0f32; n];
    let mut vy = vec![0.0f32; n];
    let nb = ((n as f32).sqrt().ceil() as usize).clamp(4, 24);
    let bw = (side as f32 / nb as f32).max(1.0);
    let mut util = vec![0.0f32; nb * nb];
    let mut step = STEP0;
    let mut lambda = LAMBDA0;
    let span = side as f32;
    let mut save_x = vec![0i32; n];
    let mut save_y = vec![0i32; n];
    let mut save_axis = Vec::new();

    for iter in 0..MAX_ITERS {
        gx.fill(0.0);
        gy.fill(0.0);

        // (a) weighted pin-HPWL subgradient: the cells owning a net's
        // bbox-extreme pins are pulled inward.
        for ni in 0..nets.count() {
            let (mut x0, mut x1, mut y0, mut y1) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
            for k in nets.span(ni) {
                let (px, py) = nets.pin(k, &l);
                (x0, x1, y0, y1) = (x0.min(px), x1.max(px), y0.min(py), y1.max(py));
            }
            let w = nets.weight(ni);
            for (k, &c) in nets.span(ni).zip(nets.row(ni)) {
                let (px, py) = nets.pin(k, &l);
                let i = c as usize;
                gx[i] += w * (f32::from(px >= x1) - f32::from(px <= x0));
                gy[i] += w * (f32::from(py >= y1) - f32::from(py <= y0));
            }
        }

        // (b) analog-cost gradient (priced budgets included) by central difference.
        let inv = 1.0f32 / (2.0 * ANALOG_PROBE as f32);
        for i in 0..n {
            let ox = l.x[i];
            l.x[i] = ox + ANALOG_PROBE;
            let cp = analog_cost(reqs, &l, prices);
            l.x[i] = ox - ANALOG_PROBE;
            let cm = analog_cost(reqs, &l, prices);
            l.x[i] = ox;
            gx[i] += (cp - cm) * inv;

            let oy = l.y[i];
            l.y[i] = oy + ANALOG_PROBE;
            let cp = analog_cost(reqs, &l, prices);
            l.y[i] = oy - ANALOG_PROBE;
            let cm = analog_cost(reqs, &l, prices);
            l.y[i] = oy;
            gy[i] += (cp - cm) * inv;
        }

        // (c) density push: overfull bins push toward the emptiest neighbour.
        util.fill(0.0);
        let bin_of = |x: i32, y: i32| -> usize {
            let bx = ((x as f32 / bw) as usize).min(nb - 1);
            let by = ((y as f32 / bw) as usize).min(nb - 1);
            by * nb + bx
        };
        for i in 0..n {
            util[bin_of(l.x[i], l.y[i])] += 4.0 * l.hw[i] as f32 * l.hh[i] as f32 / (bw * bw);
        }
        for i in 0..n {
            let b = bin_of(l.x[i], l.y[i]);
            let over = util[b] - TARGET_UTIL;
            if over <= 0.0 {
                continue;
            }
            let (bx, by) = (b % nb, b / nb);
            let mut best = (util[b], 0i32, 0i32);
            for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let (tx, ty) = (bx as i32 + dx, by as i32 + dy);
                if tx >= 0 && ty >= 0 && (tx as usize) < nb && (ty as usize) < nb {
                    let u = util[ty as usize * nb + tx as usize];
                    if u < best.0 {
                        best = (u, dx, dy);
                    }
                }
            }
            gx[i] -= lambda * over * best.1 as f32;
            gy[i] -= lambda * over * best.2 as f32;
        }

        // (d) momentum step, largest gradient moves ~step·span. A step that
        // raises hard-rule Φ is undone, momentum killed, step halved. Overlap
        // is deliberately not in this gate: the start is a pile at the centre.
        let gmax = gx.iter().chain(&gy).fold(0.0f32, |m, g| m.max(g.abs())).max(1e-6);
        let scale = step * span / gmax;
        let before_phi = analog_phi(reqs, &l);
        save_axis.clone_from(&l.axis);
        save_x.copy_from_slice(&l.x);
        save_y.copy_from_slice(&l.y);
        for i in 0..n {
            vx[i] = MOMENTUM * vx[i] - scale * gx[i];
            vy[i] = MOMENTUM * vy[i] - scale * gy[i];
            l.x[i] = clamp_to_die(l.x[i] + vx[i] as i32, l.hw[i], side);
            l.y[i] = clamp_to_die(l.y[i] + vy[i] as i32, l.hh[i], side);
        }
        // Each block's axis follows its pairs (mean midpoint), so two stages are not pinned to one line.
        for id in 0..l.axis.len() {
            let (s, k) = pairs
                .iter()
                .filter(|p| usize::from(p.2) == id && (p.0 as usize) < n && (p.1 as usize) < n)
                .fold((0i64, 0i64), |(s, k), p| (s + i64::from(l.x[p.0 as usize]) + i64::from(l.x[p.1 as usize]), k + 2));
            if k > 0 {
                l.axis[id] = snap((s / k) as i32, rules.grid);
            }
        }
        if analog_phi(reqs, &l) > before_phi {
            std::mem::swap(&mut l.x, &mut save_x);
            std::mem::swap(&mut l.y, &mut save_y);
            std::mem::swap(&mut l.axis, &mut save_axis);
            vx.fill(0.0);
            vy.fill(0.0);
            step = (step * 0.5).max(STEP_MIN);
        }

        // (e) cooling + density-weight ramp.
        step = (step * STEP_DECAY).max(STEP_MIN);
        let overflow = bin_overflow(&util, &l, bw);
        if overflow > 0.05 {
            lambda = (lambda * 1.05).min(1e3);
        }
        if iter + 1 >= MIN_ITERS && overflow <= OVERFLOW_TARGET {
            break;
        }
    }

    l.refresh_temps();
    let rep = report(&nets, reqs, &l, prices, rules.clearance);
    (l, rep)
}

/// Σ bin overflow area over total device area.
fn bin_overflow(util: &[f32], l: &Layout, bw: f32) -> f32 {
    let total: f32 = (0..l.x.len()).map(|i| 4.0 * l.hw[i] as f32 * l.hh[i] as f32).sum();
    if total <= 0.0 {
        return 0.0;
    }
    util.iter().map(|&u| (u - TARGET_UTIL).max(0.0) * bw * bw).sum::<f32>() / total
}

#[cfg(test)]
mod price_tests {
    use super::*;
    use analog::Rule;

    /// A budget rule whose residual is **read out of the layout**, so a test can drive
    /// it epoch by epoch: `x[0] = 1000` ⇒ one full budget past spec, `x[0] = 0` ⇒
    /// satisfied.
    #[derive(Clone, Copy)]
    struct Budget;
    impl Rule for Budget {
        type On = Layout;
        fn cost(self, _: &Layout) -> f32 {
            1.0
        }
        fn residual(self, l: &Layout) -> f32 {
            (l.x[0] as f32 / 1000.0).clamp(0.0, 1.0)
        }
    }

    /// [`Budget`] that also reports its spent fraction, unclamped, so a test can
    /// read slack: `x[0] = -500` ⇒ usage 0.5 with residual 0.
    #[derive(Clone, Copy)]
    struct Spent;
    impl Rule for Spent {
        type On = Layout;
        fn cost(self, l: &Layout) -> f32 {
            Budget.cost(l)
        }
        fn residual(self, l: &Layout) -> f32 {
            Budget.residual(l)
        }
        fn usage(self, l: &Layout) -> Option<f32> {
            Some(1.0 + l.x[0] as f32 / 1000.0)
        }
    }

    fn bench() -> (Requirements<Layout>, Layout) {
        let reqs = Requirements {
            hard: Vec::new(),
            budget: vec![Box::new(vec![Budget])],
            cost: Vec::new(),
        };
        let l = Layout {
            x: vec![1_000],
            y: vec![0],
            hw: vec![100],
            hh: vec![100],
            variant: vec![0],
            axis: vec![0],
            branch: vec![false],
            groups: vec![vec![pnr_core::DeviceId(0)]],
            orient: vec![pnr_core::Orient::default(); 1],
            power_uw: vec![0],
            temp_mc: vec![0],
            units: Default::default(),
        };
        (reqs, l)
    }

    /// The whole reason λ/ρ are carried instead of recomputed (D9): a budget that has
    /// been binding for several epochs must cost strictly more each epoch. With the
    /// old headroom-only weighting all three epochs below priced identically.
    #[test]
    fn rho_escalates_for_a_residual_that_does_not_shrink() {
        let (reqs, l) = bench();
        let mut prices = Prices::new();

        // No dual step yet ⇒ no price, and explicitly *not* stationary.
        prices.bind(&reqs);
        assert_eq!(prices.weight_of(0), 0.0);
        assert!(prices.drift().is_infinite(), "a fresh Prices must not read as settled");

        // `x[0] = 1000` pins the residual at one full budget, so it never shrinks.
        let mut weights = Vec::new();
        for _ in 0..4 {
            prices.settle(&reqs, &l);
            weights.push(prices.weight_of(0));
        }

        // Price rising is the ratchet; the *increments* rising is ρ escalating, which
        // is the part a recompute-from-headroom weight can never do.
        let steps: Vec<f32> = weights.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(
            weights.windows(2).all(|w| w[1] > w[0]),
            "price must rise every epoch: {weights:?}"
        );
        assert!(
            steps.windows(2).all(|s| s[1] > s[0]),
            "rho must escalate, so each epoch's step must exceed the last: {steps:?}"
        );
    }

    /// PLAN §5 terminates on `‖λ_{k+1} − λ_k‖ < δ`. That probe is only usable if it
    /// actually falls as the budget stops binding — a batch that has gone slack must
    /// stop moving its multiplier, and `drift` must say so.
    #[test]
    fn drift_decreases_as_prices_settle() {
        let (reqs, mut l) = bench();
        let mut prices = Prices::new();

        // Residual 1.0 → 0.5 → 0.1 → 0: it shrinks every epoch, so ρ holds at its floor
        // and the dual step shrinks with it.
        let mut drifts = Vec::new();
        for residual in [1_000, 500, 100, 0] {
            l.x[0] = residual;
            prices.settle(&reqs, &l);
            drifts.push(prices.drift());
        }
        assert!(
            drifts.windows(2).all(|d| d[1] < d[0]),
            "drift must fall as the residual shrinks: {drifts:?}"
        );
        assert_eq!(
            *drifts.last().unwrap(),
            0.0,
            "a satisfied budget trades nothing, so its price is stationary"
        );
    }

    /// PLAN.md:124: a budget gone slack loses its price instead of keeping the
    /// one it ratcheted to while violated; λ is projected onto `≤ 0`, so it
    /// stops at exactly zero rather than turning into a reward.
    #[test]
    fn a_slack_budget_relaxes_its_price() {
        let (mut reqs, mut l) = bench();
        reqs.budget = vec![Box::new(vec![Spent])];
        let mut prices = Prices::new();
        l.x[0] = 500;
        for _ in 0..3 {
            prices.settle(&reqs, &l);
        }
        let mut last = prices.weight_of(0);
        assert!(last > 0.0, "three violated epochs priced the budget");

        l.x[0] = -500; // half the budget spent, residual 0
        let mut relaxing = 0;
        while last > 0.0 {
            prices.settle(&reqs, &l);
            let w = prices.weight_of(0);
            assert!(w < last, "the price must fall every slack epoch: {w} after {last}");
            last = w;
            relaxing += 1;
            assert!(relaxing < 100, "price never reached zero: {last}");
        }
        assert_eq!(last, 0.0, "clamped at zero");
        for _ in 0..3 {
            prices.settle(&reqs, &l);
            assert_eq!(prices.weight_of(0), 0.0, "a relaxed price stays at zero");
        }
    }

    /// A λ held at the cap reads as stationary in `drift` while the budget is
    /// still violated, so it must be reported as binding (ρ ramps 0.25 … 64,
    /// Σ > 64 well inside 30 steps). Two capped batches of one kind (ordinals
    /// 0 and 1) list that kind once.
    #[test]
    fn a_saturated_price_is_reported() {
        let (mut reqs, l) = bench(); // residual 1.0 every epoch
        reqs.budget.push(Box::new(vec![Budget]));
        let mut prices = Prices::new();
        prices.settle(&reqs, &l);
        assert!(prices.saturated().is_empty(), "one step does not reach the cap");
        for _ in 1..30 {
            prices.settle(&reqs, &l);
        }
        assert_eq!((prices.weight_of(0), prices.weight_of(1)), (LAMBDA_MAX, LAMBDA_MAX));
        assert_eq!(prices.saturated(), [reqs.budget[0].kind()], "the capped kind is reported binding, once");
        assert_eq!(prices.steps(), 30);
    }

    /// Prices are keyed by rule kind, not by position, so a batch keeps its price when
    /// the annotator appends an in-loop feedback batch ahead of the next epoch.
    #[test]
    fn price_survives_an_appended_batch() {
        let (reqs, l) = bench();
        let mut prices = Prices::new();
        prices.settle(&reqs, &l);
        let paid = prices.weight_of(0);
        assert!(paid > 0.0);

        // Next epoch: `Requirements` re-derived with a feedback batch appended.
        let grown = Requirements {
            hard: Vec::new(),
            budget: vec![Box::new(vec![Budget]), Box::new(vec![Budget, Budget])],
            cost: Vec::new(),
        };
        prices.bind(&grown);
        assert_eq!(prices.weight_of(0), paid, "the surviving batch kept its price");
        assert_eq!(prices.weight_of(1), 0.0, "the new batch starts unpriced");
    }
}

#[cfg(test)]
mod weight_tests {
    use analog::metadata::{NetClass, NetClassification};
    use pnr_core::NetId;

    fn class(net: u16, c_budget_af: Option<i64>) -> NetClassification {
        NetClassification { net: NetId(net), class: NetClass::Signal, c_budget_af, max_coupling_af: None }
    }

    #[test]
    fn a_tighter_budget_pulls_harder_and_sensitivities_win() {
        let classes = [class(0, Some(1_000)), class(1, Some(4_000)), class(2, None)];
        let w = super::net_weights(&classes, &[]);
        assert!(w[0] > w[1], "1 fF budget outweighs 4 fF: {w:?}");
        assert!((w[0] + w[1] - 2.0).abs() < 1e-5, "mean 1 over weighted nets");
        assert_eq!(w[2], 1.0, "a rail keeps unit weight");

        // A spec that feels net 1 hardest overrides its class weight.
        let w = super::net_weights(&classes, &[(NetId(1), 1e-2), (NetId(1), -5.0)]);
        assert!(w[1] > w[0], "{w:?}");
    }

}

#[cfg(test)]
mod place_tests {
    use super::*;
    use analog::matching::mismatch::{Budget, Coeffs, MatchKind};
    use analog::placement::symmetry::{Symmetry, SymmetryGroup};
    use analog::placement::MatchedSet;
    use pnr_core::geom::Rect;
    use pnr_core::ids::{AxisId, DeviceId, Target};

    fn input<'a>(macros: &'a [Macro], reqs: &'a Requirements<Layout>, n_axes: usize, power: &'a [i32]) -> GpInput<'a> {
        GpInput {
            macros,
            variants: &[],
            assignment: &[],
            reqs,
            rules: Rules { grid: 10, clearance: 270 },
            net_weight: &[],
            n_axes,
            power_uw: power,
            units: Default::default(),
            iterate: true,
        }
    }

    fn cells(n: usize) -> Vec<Macro> {
        vec![Macro { bbox: Rect { x: 0, y: 0, w: 1_000, h: 1_000 }, ..Default::default() }; n]
    }

    fn sym(a: u16, b: u16, axis: u16) -> SymmetryGroup {
        SymmetryGroup(vec![Symmetry { a: Target::Device(DeviceId(a)), b: Target::Device(DeviceId(b)), axis: AxisId(axis) }])
    }

    #[test]
    fn gp_keeps_one_axis_per_block() {
        let macros = cells(6);
        let reqs = Requirements::<Layout> {
            hard: vec![Box::new(sym(0, 1, 0)), Box::new(sym(2, 3, 1))],
            budget: vec![],
            cost: vec![Box::new(sym(0, 1, 0)), Box::new(sym(2, 3, 1))],
        };
        let mut apart = false;
        for seed in 1..=5u64 {
            let (l, _) = place(&input(&macros, &reqs, 2, &[]), &mut Prices::new(), seed);
            assert_eq!(l.axis.len(), 2, "seed {seed}");
            apart |= l.axis[0] != l.axis[1];
        }
        assert!(apart, "the two blocks never left one shared axis");
    }

    /// C28 rewrite of the `ThermalGradient` test: a powered cell pushes the
    /// matched pair onto one isotherm.
    #[test]
    fn gp_sees_power() {
        let macros = cells(3);
        let reqs = Requirements::<Layout> {
            hard: vec![],
            budget: vec![],
            cost: vec![Box::new(MatchedSet {
                members: vec![DeviceId(0), DeviceId(1)],
                kind: MatchKind::Voltage,
                mos: true,
                coeffs: Coeffs { tc_uv_per_k: Some(1_000.0), ..Default::default() },
                budget: Budget::Allowance(1.0),
                gate_um2: vec![],
                tol_nm: 5.0,
                cell_of: vec![],
            })],
        };
        let hot = [0, 0, 10_000];
        let spread = |power: &[i32], seed: u64| {
            let (mut l, _) = place(&input(&macros, &reqs, 1, power), &mut Prices::new(), seed);
            l.power_uw = hot.to_vec();
            (l.rise_at_point_mc(l.x[0], l.y[0]) - l.rise_at_point_mc(l.x[1], l.y[1])).abs()
        };
        let (mut on, mut off) = (Vec::new(), Vec::new());
        for seed in 1..=10u64 {
            on.push(spread(&hot, seed));
            off.push(spread(&[0, 0, 0], seed));
        }
        let mean = |v: &[f32]| v.iter().sum::<f32>() / v.len() as f32;
        assert!(mean(&on) < mean(&off), "powered {on:?} vs unpowered {off:?}");
    }
}
