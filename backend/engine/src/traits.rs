//! Core traits, generic Stage driver, reusable slot implementations, and RNG.

use std::marker::PhantomData;

// ---------------------------------------------------------------------------
// RNG — deterministic, seedable, dependency-free (SplitMix64)
// ---------------------------------------------------------------------------

pub struct SplitMix64(pub u64);

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    #[inline(always)]
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    #[inline(always)]
    pub fn f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform in [0, 1).
    #[inline(always)]
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u32 << 24) as f32
    }

    /// Uniform integer in [0, n). n == 0 returns 0.
    #[inline(always)]
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next_u64() % n as u64) as usize }
    }

    /// Uniform in [-r, r].
    #[inline(always)]
    pub fn centered(&mut self, r: f32) -> f32 {
        (self.f32() * 2.0 - 1.0) * r
    }
}

// ---------------------------------------------------------------------------
// Domain + telemetry
// ---------------------------------------------------------------------------

/// The problem shape a stage runs over. `Hot` is caller-owned mutable state
/// touched every move; `Cold` is read-only context (constraints, connectivity);
/// `Mv` is a proposed change with any cached evaluation baked in.
pub trait Domain {
    type Hot;
    type Cold;
    type Mv;
}

/// Per-outer-iteration knobs produced by [`Schedule`].
#[derive(Debug, Clone, Copy)]
pub struct Control {
    /// Metropolis temperature (0 for analytical stages).
    pub temp: f64,
    /// Solver step size / learning rate.
    pub step: f32,
    /// Range-limiter window (fraction of die or absolute — slot convention).
    pub range: f32,
    /// Inner-loop moves per outer iteration.
    pub moves_per_step: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct TracePoint {
    pub iter: u32,
    pub cost: f64,
    pub temp: f64,
    pub overflow: f32,
    pub accept_rate: f32,
}

/// Running counters + per-epoch trace. Returned by [`Stage::run`]; the caller
/// dumps it for debugging.
#[derive(Debug, Clone, Default)]
pub struct Telemetry {
    pub iters: u32,
    pub proposed: u64,
    pub accepted: u64,
    pub illegal: u64,
    /// Incrementally-tracked objective (exact-evaluated once more at the end).
    pub cost: f64,
    pub initial_cost: f64,
    pub best_cost: f64,
    pub overflow: f32,
    pub epoch_accept_rate: f32,
    pub trace: Vec<TracePoint>,
    ep_prop: u64,
    ep_acc: u64,
}

impl Telemetry {
    fn start(cost: f64) -> Self {
        Self { cost, initial_cost: cost, best_cost: cost, ..Self::default() }
    }

    #[inline(always)]
    fn record_accept(&mut self, d_terms: f64) {
        self.accepted += 1;
        self.ep_acc += 1;
        self.cost += d_terms;
        if self.cost < self.best_cost {
            self.best_cost = self.cost;
        }
    }

    fn end_epoch(&mut self, temp: f64) {
        self.epoch_accept_rate =
            if self.ep_prop == 0 { 0.0 } else { self.ep_acc as f32 / self.ep_prop as f32 };
        self.trace.push(TracePoint {
            iter: self.iters,
            cost: self.cost,
            temp,
            overflow: self.overflow,
            accept_rate: self.epoch_accept_rate,
        });
        self.ep_prop = 0;
        self.ep_acc = 0;
        self.iters += 1;
    }

    /// CSV dump of the per-epoch trace (debug artifact).
    pub fn trace_csv(&self) -> String {
        let mut s = String::from("iter,cost,temp,overflow,accept_rate\n");
        for p in &self.trace {
            s.push_str(&format!(
                "{},{},{},{},{}\n",
                p.iter, p.cost, p.temp, p.overflow, p.accept_rate
            ));
        }
        s
    }
}

// ---------------------------------------------------------------------------
// The 8 slots
// ---------------------------------------------------------------------------

/// Slot 1: the objective. `delta` is the hot path — incremental, may read a
/// cached value off the move.
pub trait CostFn<D: Domain> {
    /// Full evaluation (cold path: stage start/end).
    fn eval(&self, hot: &D::Hot, cold: &D::Cold) -> f64;
    /// Incremental cost change if `mv` were committed.
    fn delta(&self, hot: &D::Hot, cold: &D::Cold, mv: &D::Mv) -> f64;
}

/// Slot 2: spreading / capacity model. Blended with the objective by
/// [`Weights`]; `overflow` feeds [`Stop`] and telemetry.
pub trait Density<D: Domain> {
    #[inline(always)]
    fn delta(&self, _hot: &D::Hot, _cold: &D::Cold, _mv: &D::Mv) -> f64 {
        0.0
    }
    #[inline(always)]
    fn overflow(&self, _hot: &D::Hot, _cold: &D::Cold) -> f32 {
        0.0
    }
}

/// Slot 3: hard constraints. A move failing this is discarded before costing.
pub trait Legality<D: Domain> {
    fn is_legal(&self, hot: &D::Hot, cold: &D::Cold, mv: &D::Mv) -> bool;
}

/// Slot 4: penalty balancing across iterations (λ-scaling, overlap ramps).
pub trait Weights<D: Domain> {
    type State: Copy;
    fn init(&self, hot: &D::Hot, cold: &D::Cold) -> Self::State;
    fn update(&self, s: Self::State, t: &Telemetry) -> Self::State;
    /// Blend objective delta and density delta into the acceptance delta.
    fn blend(&self, s: Self::State, terms: f64, dens: f64) -> f64;
}

/// Slot 5: the search/solve primitive. `propose` returns `None` when it has
/// nothing to do (converged / all work items clean) — an epoch with zero
/// proposals ends the stage regardless of [`Stop`].
pub trait Core<D: Domain> {
    type Scratch;
    fn scratch(&self, hot: &D::Hot, cold: &D::Cold) -> Self::Scratch;
    fn propose(
        &self,
        hot: &D::Hot,
        cold: &D::Cold,
        sc: &mut Self::Scratch,
        ctl: &Control,
        rng: &mut SplitMix64,
    ) -> Option<D::Mv>;
    fn commit(&self, hot: &mut D::Hot, cold: &D::Cold, sc: &mut Self::Scratch, mv: &D::Mv);
    /// Once per outer iteration, after weights/schedule advance (e.g.
    /// PathFinder history-cost bump, λ adaptation inside an analytical core).
    fn epoch(&self, _hot: &mut D::Hot, _cold: &D::Cold, _sc: &mut Self::Scratch, _t: &Telemetry) {}
}

/// Slot 6: acceptance rule.
pub trait Accept {
    fn decide(&self, delta: f64, temp: f64, rng: &mut SplitMix64) -> bool;
}

/// Slot 7: cooling / step-size schedule.
pub trait Schedule {
    type State: Copy;
    fn start(&self) -> Self::State;
    fn advance(&self, s: Self::State, t: &Telemetry) -> Self::State;
    fn control(&self, s: Self::State) -> Control;
}

/// Slot 8: convergence + contract closure. `open` = contracts not yet
/// Satisfied/Waived (from the [`Ledger`]).
pub trait Stop {
    fn done(&self, t: &Telemetry, open: usize) -> bool;
}

/// Constraint-contract reconciliation, called once per outer iteration.
/// `()` is the no-ledger ledger.
pub trait Ledger<D: Domain> {
    fn reconcile(&mut self, hot: &D::Hot, cold: &D::Cold);
    fn open(&self) -> usize;
}

impl<D: Domain> Ledger<D> for () {
    fn reconcile(&mut self, _: &D::Hot, _: &D::Cold) {}
    fn open(&self) -> usize {
        0
    }
}

// ---------------------------------------------------------------------------
// The generic driver
// ---------------------------------------------------------------------------

/// One stage = eight concrete slot types. Monomorphizes per combination; the
/// inner loop below contains no policy branches, only data branches.
pub struct Stage<D, C, Dn, L, W, K, A, S, T>
where
    D: Domain,
    C: CostFn<D>,
    Dn: Density<D>,
    L: Legality<D>,
    W: Weights<D>,
    K: Core<D>,
    A: Accept,
    S: Schedule,
    T: Stop,
{
    pub cost: C,
    pub density: Dn,
    pub legality: L,
    pub weights: W,
    pub core: K,
    pub accept: A,
    pub schedule: S,
    pub stop: T,
    pub _d: PhantomData<D>,
}

impl<D, C, Dn, L, W, K, A, S, T> Stage<D, C, Dn, L, W, K, A, S, T>
where
    D: Domain,
    C: CostFn<D>,
    Dn: Density<D>,
    L: Legality<D>,
    W: Weights<D>,
    K: Core<D>,
    A: Accept,
    S: Schedule,
    T: Stop,
{
    pub fn run<Lg: Ledger<D>>(
        &self,
        hot: &mut D::Hot,
        cold: &D::Cold,
        ledger: &mut Lg,
        rng: &mut SplitMix64,
    ) -> Telemetry {
        let mut sc = self.core.scratch(hot, cold);
        let mut ws = self.weights.init(hot, cold);
        let mut ss = self.schedule.start();
        let mut t = Telemetry::start(self.cost.eval(hot, cold));

        loop {
            let ctl = self.schedule.control(ss);
            for _ in 0..ctl.moves_per_step {
                let Some(mv) = self.core.propose(hot, cold, &mut sc, &ctl, rng) else {
                    break;
                };
                t.proposed += 1;
                t.ep_prop += 1;
                if !self.legality.is_legal(hot, cold, &mv) {
                    t.illegal += 1;
                    continue;
                }
                let d_terms = self.cost.delta(hot, cold, &mv);
                let d_dens = self.density.delta(hot, cold, &mv);
                let delta = self.weights.blend(ws, d_terms, d_dens);
                if self.accept.decide(delta, ctl.temp, rng) {
                    self.core.commit(hot, cold, &mut sc, &mv);
                    t.record_accept(d_terms);
                }
            }
            let idle = t.ep_prop == 0;
            ws = self.weights.update(ws, &t);
            ss = self.schedule.advance(ss, &t);
            self.core.epoch(hot, cold, &mut sc, &t);
            t.overflow = self.density.overflow(hot, cold);
            ledger.reconcile(hot, cold);
            t.end_epoch(ctl.temp);
            if idle || self.stop.done(&t, ledger.open()) {
                break;
            }
        }
        t.cost = self.cost.eval(hot, cold);
        t
    }
}

// ---------------------------------------------------------------------------
// Reusable pure slots
// ---------------------------------------------------------------------------

pub mod slots {
    use super::*;

    /// Legality that accepts everything (global/analytical stages).
    pub struct AllLegal;
    impl<D: Domain> Legality<D> for AllLegal {
        #[inline(always)]
        fn is_legal(&self, _: &D::Hot, _: &D::Cold, _: &D::Mv) -> bool {
            true
        }
    }

    /// No density model.
    pub struct NoDensity;
    impl<D: Domain> Density<D> for NoDensity {}

    /// terms + dens, no state, no adaptation.
    pub struct UnitWeights;
    impl<D: Domain> Weights<D> for UnitWeights {
        type State = ();
        fn init(&self, _: &D::Hot, _: &D::Cold) {}
        fn update(&self, _: (), _: &Telemetry) {}
        #[inline(always)]
        fn blend(&self, _: (), terms: f64, dens: f64) -> f64 {
            terms + dens
        }
    }

    /// Geometric penalty ramp: blend = terms + w·dens, w ← w·gain each epoch.
    pub struct RampWeights {
        pub w0: f64,
        pub gain: f64,
        pub w_max: f64,
    }
    impl<D: Domain> Weights<D> for RampWeights {
        type State = f64;
        fn init(&self, _: &D::Hot, _: &D::Cold) -> f64 {
            self.w0
        }
        fn update(&self, w: f64, _: &Telemetry) -> f64 {
            (w * self.gain).min(self.w_max)
        }
        #[inline(always)]
        fn blend(&self, w: f64, terms: f64, dens: f64) -> f64 {
            terms + w * dens
        }
    }

    /// Always accept (numerical solvers / negotiated congestion).
    pub struct AlwaysAccept;
    impl Accept for AlwaysAccept {
        #[inline(always)]
        fn decide(&self, _: f64, _: f64, _: &mut SplitMix64) -> bool {
            true
        }
    }

    /// Metropolis criterion.
    pub struct Metropolis;
    impl Accept for Metropolis {
        #[inline(always)]
        fn decide(&self, delta: f64, temp: f64, rng: &mut SplitMix64) -> bool {
            delta <= 0.0 || (temp > 0.0 && rng.f64() < (-delta / temp).exp())
        }
    }

    /// Geometric cooling with a shrinking range limiter (TimberWolf-style).
    #[derive(Clone, Copy)]
    pub struct Geometric {
        pub t0: f64,
        pub alpha: f64,
        pub range0: f32,
        pub range_decay: f32,
        pub range_min: f32,
        pub moves_per_step: u32,
    }
    impl Schedule for Geometric {
        type State = (f64, f32);
        fn start(&self) -> (f64, f32) {
            (self.t0, self.range0)
        }
        fn advance(&self, (temp, range): (f64, f32), _: &Telemetry) -> (f64, f32) {
            (temp * self.alpha, (range * self.range_decay).max(self.range_min))
        }
        fn control(&self, (temp, range): (f64, f32)) -> Control {
            Control { temp, step: 0.0, range, moves_per_step: self.moves_per_step }
        }
    }

    /// Decaying step size (analytical solvers, negotiated-congestion epochs).
    #[derive(Clone, Copy)]
    pub struct StepDecay {
        pub step0: f32,
        pub decay: f32,
        pub step_min: f32,
        pub moves_per_step: u32,
    }
    impl Schedule for StepDecay {
        type State = f32;
        fn start(&self) -> f32 {
            self.step0
        }
        fn advance(&self, s: f32, _: &Telemetry) -> f32 {
            (s * self.decay).max(self.step_min)
        }
        fn control(&self, s: f32) -> Control {
            Control { temp: 0.0, step: s, range: 0.0, moves_per_step: self.moves_per_step }
        }
    }

    /// Stop on iteration budget, freeze-out, or clean convergence.
    #[derive(Clone, Copy)]
    pub struct FreezeStop {
        pub max_iters: u32,
        pub min_iters: u32,
        pub min_accept_rate: f32,
    }
    impl Stop for FreezeStop {
        fn done(&self, t: &Telemetry, open: usize) -> bool {
            if t.iters >= self.max_iters {
                return true;
            }
            t.iters >= self.min_iters
                && t.epoch_accept_rate < self.min_accept_rate
                && open == 0
        }
    }

    /// Stop when overflow reaches target (routing / global density), or budget.
    #[derive(Clone, Copy)]
    pub struct OverflowStop {
        pub max_iters: u32,
        pub min_iters: u32,
        pub target: f32,
    }
    impl Stop for OverflowStop {
        fn done(&self, t: &Telemetry, open: usize) -> bool {
            if t.iters >= self.max_iters {
                return true;
            }
            t.iters >= self.min_iters && t.overflow <= self.target && open == 0
        }
    }
}

// ---------------------------------------------------------------------------
// Tests — the pure algorithm on toy problems
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::slots::*;
    use super::*;

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
        fn delta(&self, _: &Vec<f64>, _: &Vec<f64>, mv: &(usize, f64, f64)) -> f64 {
            mv.2
        }
    }

    struct Perturb;
    impl Core<Toy> for Perturb {
        type Scratch = ();
        fn scratch(&self, _: &Vec<f64>, _: &Vec<f64>) {}
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
            let d = (nx - c[i]).powi(2) - (h[i] - c[i]).powi(2);
            Some((i, nx, d))
        }
        fn commit(&self, h: &mut Vec<f64>, _: &Vec<f64>, _: &mut (), mv: &(usize, f64, f64)) {
            h[mv.0] = mv.1;
        }
    }

    fn stage() -> Stage<Toy, SqCost, NoDensity, AllLegal, UnitWeights, Perturb, Metropolis, Geometric, FreezeStop>
    {
        Stage {
            cost: SqCost,
            density: NoDensity,
            legality: AllLegal,
            weights: UnitWeights,
            core: Perturb,
            accept: Metropolis,
            schedule: Geometric {
                t0: 10.0,
                alpha: 0.9,
                range0: 4.0,
                range_decay: 0.95,
                range_min: 0.01,
                moves_per_step: 200,
            },
            stop: FreezeStop { max_iters: 300, min_iters: 5, min_accept_rate: 0.01 },
            _d: PhantomData,
        }
    }

    #[test]
    fn sa_converges_on_toy() {
        let targets: Vec<f64> = (0..32).map(|i| f64::from(i) * 0.7 - 10.0).collect();
        let mut xs = vec![0.0; 32];
        let mut rng = SplitMix64::new(42);
        let t = stage().run(&mut xs, &targets, &mut (), &mut rng);

        assert!(t.cost < t.initial_cost * 0.01, "cost {} vs initial {}", t.cost, t.initial_cost);
        assert!(t.accepted > 0 && t.proposed >= t.accepted);
        assert_eq!(t.trace.len() as u32, t.iters);
    }

    #[test]
    fn deterministic_given_seed() {
        let targets: Vec<f64> = (0..8).map(f64::from).collect();
        let run = || {
            let mut xs = vec![0.0; 8];
            let mut rng = SplitMix64::new(7);
            stage().run(&mut xs, &targets, &mut (), &mut rng);
            xs
        };
        assert_eq!(run(), run());
    }

    struct CloseEnough(usize);
    impl Ledger<Toy> for CloseEnough {
        fn reconcile(&mut self, hot: &Vec<f64>, cold: &Vec<f64>) {
            self.0 = hot.iter().zip(cold).filter(|(x, t)| (*x - *t).abs() > 0.5).count();
        }
        fn open(&self) -> usize {
            self.0
        }
    }

    #[test]
    fn stop_waits_for_ledger() {
        let targets: Vec<f64> = (0..8).map(f64::from).collect();
        let mut xs = vec![100.0; 8];
        let mut rng = SplitMix64::new(3);
        let mut ledger = CloseEnough(usize::MAX);
        stage().run(&mut xs, &targets, &mut ledger, &mut rng);
        assert_eq!(ledger.open(), 0, "stage must not stop with open contracts (short of max_iters)");
    }

    #[test]
    fn core_none_ends_stage() {
        struct Never;
        impl Core<Toy> for Never {
            type Scratch = ();
            fn scratch(&self, _: &Vec<f64>, _: &Vec<f64>) {}
            fn propose(
                &self,
                _: &Vec<f64>,
                _: &Vec<f64>,
                _: &mut (),
                _: &Control,
                _: &mut SplitMix64,
            ) -> Option<(usize, f64, f64)> {
                None
            }
            fn commit(&self, _: &mut Vec<f64>, _: &Vec<f64>, _: &mut (), _: &(usize, f64, f64)) {}
        }
        let s = Stage {
            cost: SqCost,
            density: NoDensity,
            legality: AllLegal,
            weights: UnitWeights,
            core: Never,
            accept: AlwaysAccept,
            schedule: StepDecay { step0: 1.0, decay: 0.9, step_min: 0.1, moves_per_step: 1 },
            stop: FreezeStop { max_iters: u32::MAX, min_iters: 0, min_accept_rate: 0.0 },
            _d: PhantomData,
        };
        let mut xs = vec![0.0; 4];
        let t = s.run(&mut xs, &vec![1.0; 4], &mut (), &mut SplitMix64::new(1));
        assert_eq!(t.iters, 1, "idle epoch terminates");
    }
}
