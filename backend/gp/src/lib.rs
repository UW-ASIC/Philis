//! Global placement: momentum gradient descent on pin HPWL + bin density + the
//! analog cost of `Requirements`, gated so no step raises hard-rule Φ.
//! Also owns [`Prices`] (augmented-Lagrangian λ/ρ per budget batch) and the
//! shared placement primitives in [`mechanics`] that `dp` reuses.

pub mod mechanics;

use std::collections::BTreeMap;

use analog::Requirements;
use pnr_core::geom::LayerId;
use pnr_core::{Layout, Macro, Report};

use mechanics::{
    analog_cost, analog_phi, canvas_side, choose_variants, clamp_to_die, half_extents,
    initial_layout, report, Nets, SplitMix64,
};

/// One placeable cell's pre-drawn alternatives; `layout.variant[i]` indexes
/// `alternatives`. Cells sharing a `lock` id must hold the same variant index
/// (and so need index-compatible spaces).
pub struct VariantSpace {
    pub alternatives: Vec<Macro>,
    pub lock: Option<u16>,
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
}

#[derive(Clone, Copy)]
struct Price {
    /// Non-positive multiplier (`λ ← λ − ρ·c`); `−λ` is the price.
    lambda: f32,
    rho: f32,
    /// Residual at the previous dual step, for the did-it-shrink test.
    residual: f32,
}

const RHO_FLOOR: f32 = 0.25;
const RHO_GAIN: f32 = 2.0;
const RHO_MAX: f32 = 64.0;
// ponytail: flat cap so a physically unsatisfiable budget cannot swamp the
// objective; a saturated λ then reads as settled in `drift`.
const LAMBDA_MAX: f32 = 64.0;

impl Default for Prices {
    fn default() -> Self {
        Self { priced: BTreeMap::new(), weight: Vec::new(), drift: f64::INFINITY }
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

    /// Dual step `λ ← λ − ρ·c(l)`, once per `place`; ρ doubles for a residual
    /// that did not shrink.
    pub fn settle(&mut self, reqs: &Requirements<Layout>, l: &Layout) {
        let mut drift_sq = 0.0f64;
        for (bi, key) in keys(reqs).into_iter().enumerate() {
            let c = reqs.budget[bi].residual(l).max(0.0) as f32;
            let p = self.priced.entry(key).or_insert(Price {
                lambda: 0.0,
                rho: RHO_FLOOR,
                residual: f32::INFINITY,
            });
            if c > 0.0 && c >= p.residual {
                p.rho = (p.rho * RHO_GAIN).min(RHO_MAX);
            }
            p.residual = c;
            let next = (p.lambda - p.rho * c).max(-LAMBDA_MAX);
            drift_sq += f64::from(next - p.lambda).powi(2);
            p.lambda = next;
        }
        self.drift = drift_sq.sqrt();
        self.bind(reqs);
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

pub trait GlobalPlacer {
    /// Coarse placement of `macros` (variant 0 of each `variants[i]`),
    /// seed-deterministic. `layers` is unused: placement is layer-agnostic.
    fn place(
        &self,
        macros: &[Macro],
        variants: &[VariantSpace],
        reqs: &Requirements<Layout>,
        layers: &[LayerId],
        prices: &mut Prices,
        seed: u64,
    ) -> (Layout, Report);
}

/// Edge-to-edge clearance between cells, nm (shared with `dp`). sky130's nwell
/// spacing is the binding inter-device rule; below it wells/implants merge and
/// LVS aborts.
pub const CLEARANCE_NM: i32 = 2000;

const MAX_ITERS: u32 = 500;
const MIN_ITERS: u32 = 60;
const OVERFLOW_TARGET: f32 = 0.15;
const UTILIZATION: f32 = 0.4;
const GRID: i32 = 5;
const STEP0: f32 = 0.04;
const STEP_MIN: f32 = 0.002;
const STEP_DECAY: f32 = 0.995;
const MOMENTUM: f32 = 0.85;
const LAMBDA0: f32 = 0.5;
const TARGET_UTIL: f32 = 0.7;
/// Finite-difference probe (nm) for the analog-cost gradient.
const ANALOG_PROBE: i32 = 64;

#[derive(Default)]
pub struct Analytical;

impl GlobalPlacer for Analytical {
    fn place(
        &self,
        macros: &[Macro],
        variants: &[VariantSpace],
        reqs: &Requirements<Layout>,
        _layers: &[LayerId],
        prices: &mut Prices,
        seed: u64,
    ) -> (Layout, Report) {
        let n = macros.len();
        let mut rng = SplitMix64::new(seed);
        // gp does not search variants: index 0 everywhere, dp reshapes.
        let variant = vec![0u16; n];
        let drawn = choose_variants(macros, variants, &variant);
        prices.bind(reqs);

        let (hw, hh) = half_extents(&drawn);
        let side = canvas_side(&hw, &hh, UTILIZATION, GRID);
        let mut l = initial_layout(&drawn, variant, side, &mut rng);
        let nets = Nets::from_macros(&drawn);
        if n == 0 {
            let rep = report(&nets, reqs, &l, prices);
            return (l, rep);
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

        for iter in 0..MAX_ITERS {
            gx.fill(0.0);
            gy.fill(0.0);

            // (a) HPWL subgradient: a net's bbox-extreme cells are pulled inward.
            for ni in 0..nets.count() {
                let cells = nets.row(ni);
                let (mut x0, mut x1, mut y0, mut y1) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
                for &c in cells {
                    let i = c as usize;
                    x0 = x0.min(l.x[i]);
                    x1 = x1.max(l.x[i]);
                    y0 = y0.min(l.y[i]);
                    y1 = y1.max(l.y[i]);
                }
                for &c in cells {
                    let i = c as usize;
                    gx[i] += f32::from(l.x[i] >= x1) - f32::from(l.x[i] <= x0);
                    gy[i] += f32::from(l.y[i] >= y1) - f32::from(l.y[i] <= y0);
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
            save_x.copy_from_slice(&l.x);
            save_y.copy_from_slice(&l.y);
            for i in 0..n {
                vx[i] = MOMENTUM * vx[i] - scale * gx[i];
                vy[i] = MOMENTUM * vy[i] - scale * gy[i];
                l.x[i] = clamp_to_die(l.x[i] + vx[i] as i32, l.hw[i], side);
                l.y[i] = clamp_to_die(l.y[i] + vy[i] as i32, l.hh[i], side);
            }
            if analog_phi(reqs, &l) > before_phi {
                std::mem::swap(&mut l.x, &mut save_x);
                std::mem::swap(&mut l.y, &mut save_y);
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

        prices.settle(reqs, &l);
        let rep = report(&nets, reqs, &l, prices);
        (l, rep)
    }
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
