//! Zero-overhead check: the generic monomorphized `Stage` driver vs a
//! hand-written specialized loop vs `Box<dyn>` slots, same work, same seed.
//!
//!   cargo run --release -p pnr-engine --example overhead
//!
//! Expected: Stage ≈ hand-written (within noise); dyn measurably slower.
//! Kept as an example, not a test — wall-clock asserts flake in CI.

use std::hint::black_box;
use std::marker::PhantomData;
use std::time::Instant;

use pnr_engine::slots::{AllLegal, Geometric, Metropolis, NoDensity, UnitWeights};
use pnr_engine::{Control, Core, CostFn, Domain, SplitMix64, Stage};

const N: usize = 256;
const MOVES: u64 = 2_000_000;

struct Toy;
impl Domain for Toy {
    type Hot = Vec<f64>;
    type Cold = Vec<f64>;
    type Mv = (usize, f64, f64);
}

struct SqCost;
impl CostFn<Toy> for SqCost {
    fn eval(&self, h: &Vec<f64>, c: &Vec<f64>) -> f64 {
        h.iter().zip(c).map(|(x, t)| (x - t) * (x - t)).sum()
    }
    #[inline(always)]
    fn delta(&self, _: &Vec<f64>, _: &Vec<f64>, mv: &(usize, f64, f64)) -> f64 {
        mv.2
    }
}

struct Perturb;
impl Core<Toy> for Perturb {
    type Scratch = ();
    fn scratch(&self, _: &Vec<f64>, _: &Vec<f64>) {}
    #[inline(always)]
    fn propose(
        &self,
        h: &Vec<f64>,
        c: &Vec<f64>,
        _: &mut (),
        ctl: &Control,
        rng: &mut SplitMix64,
    ) -> Option<(usize, f64, f64)> {
        let i = rng.below(h.len());
        let nx = h[i] + f64::from(rng.centered(ctl.range));
        Some((i, nx, (nx - c[i]).powi(2) - (h[i] - c[i]).powi(2)))
    }
    #[inline(always)]
    fn commit(&self, h: &mut Vec<f64>, _: &Vec<f64>, _: &mut (), mv: &(usize, f64, f64)) {
        h[mv.0] = mv.1;
    }
}

fn schedule() -> Geometric {
    Geometric {
        t0: 5.0,
        alpha: 0.999_995,
        range0: 2.0,
        range_decay: 1.0,
        range_min: 0.01,
        moves_per_step: MOVES as u32,
    }
}

struct OneEpoch;
impl pnr_engine::Stop for OneEpoch {
    fn done(&self, t: &pnr_engine::Telemetry, _: usize) -> bool {
        t.iters >= 1
    }
}

fn run_generic(targets: &[f64]) -> (f64, u128) {
    let stage = Stage {
        cost: SqCost,
        density: NoDensity,
        legality: AllLegal,
        weights: UnitWeights,
        core: Perturb,
        accept: Metropolis,
        schedule: schedule(),
        stop: OneEpoch,
        _d: PhantomData,
    };
    let mut xs = vec![0.0f64; N];
    let mut rng = SplitMix64::new(42);
    let t0 = Instant::now();
    let t = stage.run(black_box(&mut xs), black_box(&targets.to_vec()), &mut (), &mut rng);
    (t.cost, t0.elapsed().as_nanos())
}

/// The same algorithm written by hand — the bar the generic driver must meet.
fn run_hand(targets: &[f64]) -> (f64, u128) {
    let mut xs = vec![0.0f64; N];
    let mut rng = SplitMix64::new(42);
    let temp = 5.0;
    let range = 2.0f32;
    let t0 = Instant::now();
    for _ in 0..MOVES {
        let i = rng.below(N);
        let nx = xs[i] + f64::from(rng.centered(range));
        let delta = (nx - targets[i]).powi(2) - (xs[i] - targets[i]).powi(2);
        if delta <= 0.0 || rng.f64() < (-delta / temp).exp() {
            xs[i] = nx;
        }
    }
    let cost: f64 = xs.iter().zip(targets).map(|(x, t)| (x - t) * (x - t)).sum();
    (black_box(cost), t0.elapsed().as_nanos())
}

// --- dyn baseline: what the slot design deliberately avoids -----------------

trait DynCost {
    fn delta(&self, mv: &(usize, f64, f64)) -> f64;
}
trait DynCore {
    fn propose(&self, h: &[f64], c: &[f64], range: f32, rng: &mut SplitMix64)
        -> (usize, f64, f64);
    fn commit(&self, h: &mut [f64], mv: &(usize, f64, f64));
}
trait DynAccept {
    fn decide(&self, delta: f64, temp: f64, rng: &mut SplitMix64) -> bool;
}

struct DCost;
impl DynCost for DCost {
    fn delta(&self, mv: &(usize, f64, f64)) -> f64 {
        mv.2
    }
}
struct DCore;
impl DynCore for DCore {
    fn propose(
        &self,
        h: &[f64],
        c: &[f64],
        range: f32,
        rng: &mut SplitMix64,
    ) -> (usize, f64, f64) {
        let i = rng.below(h.len());
        let nx = h[i] + f64::from(rng.centered(range));
        (i, nx, (nx - c[i]).powi(2) - (h[i] - c[i]).powi(2))
    }
    fn commit(&self, h: &mut [f64], mv: &(usize, f64, f64)) {
        h[mv.0] = mv.1;
    }
}
struct DAccept;
impl DynAccept for DAccept {
    fn decide(&self, delta: f64, temp: f64, rng: &mut SplitMix64) -> bool {
        delta <= 0.0 || rng.f64() < (-delta / temp).exp()
    }
}

fn run_dyn(targets: &[f64]) -> (f64, u128) {
    let cost: Box<dyn DynCost> = Box::new(DCost);
    let core: Box<dyn DynCore> = Box::new(DCore);
    let accept: Box<dyn DynAccept> = Box::new(DAccept);
    let mut xs = vec![0.0f64; N];
    let mut rng = SplitMix64::new(42);
    let t0 = Instant::now();
    for _ in 0..MOVES {
        let mv = core.propose(black_box(&xs), targets, 2.0, &mut rng);
        let d = cost.delta(&mv);
        if accept.decide(d, 5.0, &mut rng) {
            core.commit(&mut xs, &mv);
        }
    }
    let c: f64 = xs.iter().zip(targets).map(|(x, t)| (x - t) * (x - t)).sum();
    (black_box(c), t0.elapsed().as_nanos())
}

fn main() {
    let targets: Vec<f64> = (0..N).map(|i| i as f64 * 0.31 - 40.0).collect();
    // warm up + measure best-of-3 (cheap noise control)
    let best = |f: &dyn Fn(&[f64]) -> (f64, u128)| {
        (0..3).map(|_| f(&targets)).min_by_key(|r| r.1).unwrap()
    };
    let (cg, tg) = best(&run_generic);
    let (ch, th) = best(&run_hand);
    let (cd, td) = best(&run_dyn);

    let per = |ns: u128| ns as f64 / MOVES as f64;
    println!("{MOVES} moves, N={N}, best of 3:");
    println!("  generic Stage : {:7.2} ns/move (final cost {cg:.1})", per(tg));
    println!("  hand-written  : {:7.2} ns/move (final cost {ch:.1})", per(th));
    println!("  Box<dyn> slots: {:7.2} ns/move (final cost {cd:.1})", per(td));
    println!(
        "  Stage/hand = {:.2}x, dyn/hand = {:.2}x",
        per(tg) / per(th),
        per(td) / per(th)
    );
}
