//! Layout-dependent environment of matched devices (placement tier, budget).

use crate::rule::RuleBatch;

/// Allowed relative difference between members' distances inside range: a
/// design tolerance (no deck models WPE/OSE magnitudes), not a process number.
pub const ENV_TOL: f32 = 0.2;

/// One matched pair's measured surroundings, from placed geometry: the mean
/// distance of each member's channels to the nearest nwell edge (inside or
/// out), and of each member's diffusion to the nearest foreign diffusion.
/// `f32::INFINITY` = none on the die.
///
/// The ranges are the deck's: `wpe_min_nm` its moderate WPE clearance (the
/// distance a well edge must keep, Hastings §13.3 r8), `ose_range_nm` its
/// moderate LOD extension (past which STI stress has faded). `0` = the deck
/// gives none, and that half reads unknown.
#[derive(Clone, Copy, Debug)]
pub struct Surroundings {
    pub wpe_nm: [f32; 2],
    pub ose_nm: [f32; 2],
    pub wpe_min_nm: f32,
    pub ose_range_nm: f32,
}

impl Surroundings {
    /// Spent fraction, the worst of: a member's well edge nearer than
    /// `wpe_min_nm`; the members' WPE distances (within ten times it)
    /// differing by more than [`ENV_TOL`]; their OSE distances (within
    /// `ose_range_nm`) likewise. Distances past range count as the range: the
    /// effect has faded. `None` when the deck gives neither range.
    #[must_use]
    pub fn usage(&self) -> Option<f32> {
        let skew = |d: [f32; 2], range: f32| {
            let (a, b) = (d[0].min(range), d[1].min(range));
            (a - b).abs() / a.max(b).max(1.0) / ENV_TOL
        };
        let wpe = (self.wpe_min_nm > 0.0).then(|| {
            let near = self.wpe_nm.iter().map(|&d| self.wpe_min_nm / d.max(1.0)).fold(0.0f32, f32::max);
            near.max(skew(self.wpe_nm, 10.0 * self.wpe_min_nm))
        });
        let ose = (self.ose_range_nm > 0.0).then(|| skew(self.ose_nm, self.ose_range_nm));
        match (wpe, ose) {
            (None, None) => None,
            (a, b) => Some(a.unwrap_or(0.0).max(b.unwrap_or(0.0))),
        }
    }
}

/// Every matched pair's [`Surroundings`], measured once per placed layout.
/// State-free (the numbers are precomputed), so it reports on any stage.
#[derive(Clone, Debug, Default)]
pub struct Environment(pub Vec<Surroundings>);

impl<S> RuleBatch<S> for Environment {
    fn cost(&self, _: &S) -> f32 {
        self.0.iter().filter_map(Surroundings::usage).sum()
    }
    fn violations(&self, _: &S) -> u32 {
        self.0.iter().filter(|s| s.usage().is_some_and(|u| u > 1.0)).count() as u32
    }
    fn residual(&self, _: &S) -> f64 {
        self.0.iter().filter_map(Surroundings::usage).map(|u| f64::from((u - 1.0).max(0.0))).sum()
    }
    fn unknown(&self, _: &S) -> u32 {
        self.0.iter().filter(|s| s.usage().is_none()).count() as u32
    }
    fn kind(&self) -> &'static str {
        "Environment"
    }
    fn count(&self) -> usize {
        self.0.len()
    }
    fn worst_usage(&self, _: &S) -> Option<f32> {
        self.0.iter().filter_map(Surroundings::usage).reduce(f32::max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn near_or_unequal_surroundings_spend_the_budget() {
        let far = Surroundings { wpe_nm: [f32::INFINITY; 2], ose_nm: [f32::INFINITY; 2], wpe_min_nm: 3_000.0, ose_range_nm: 3_000.0 };
        assert_eq!(far.usage(), Some(0.0), "nothing nearby");
        let wpe = Surroundings { wpe_nm: [1_500.0, 1_500.0], ..far };
        assert!((wpe.usage().unwrap() - 2.0).abs() < 1e-6, "a well edge at half the floor");
        let ose = Surroundings { ose_nm: [500.0, 2_000.0], ..far };
        assert!(ose.usage().unwrap() > 1.0, "one member crowded by foreign diffusion, the other not");
        let even = Surroundings { ose_nm: [500.0, 520.0], ..far };
        assert!(even.usage().unwrap() < 1.0, "crowded alike: matched");
        let blind = Surroundings { wpe_min_nm: 0.0, ose_range_nm: 0.0, ..ose };
        assert_eq!(blind.usage(), None, "a deck without the ranges: unknown, not a pass");
    }
}
