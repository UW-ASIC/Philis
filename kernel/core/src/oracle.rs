//! The free-oracle seam (PLAN §1's oracle service, D4-revised).
//!
//! A stage that wants signoff-grade feedback *during* the search consumes this
//! trait; `backend/verify` provides the live implementation over gdsverify, and
//! [`NullOracle`] is the always-clean stand-in that reproduces pre-oracle
//! behaviour exactly. The trait lives here, not in `verify`, so `dp` can take an
//! oracle handle without a dependency on the verification engine — the same
//! reason [`crate::Macro`] lives here rather than in `cells`.
//!
//! Granularity is fixed by arithmetic, not taste: ~5.7M candidate moves × 1 ms
//! per call is 95 minutes of serial oracle time, so a stage calls **per accepted
//! move, risk-gated, bbox-scoped, and hard-capped** — never per candidate at
//! serial cost. The per-candidate lane exists only as [`Oracle::drc_batch`], the
//! reserved seam for a GPU batched evaluation that supplies the parallelism a
//! single trajectory lacks (PLAN §8).

use crate::geom::Shape;

/// What a DRC pass found, reduced to the two numbers a gate needs: how many
/// findings, and the summed nm shortfall (limit − measured) across them — the
/// same margin unit [`crate::Violation`] carries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DrcSummary {
    /// Rule findings in the checked region. `0` = clean.
    pub violations: u32,
    /// Σ (limit − measured) over the findings, nm — a severity, not a count, so
    /// two callers comparing regions can rank a 1 nm graze under a 1 µm merge.
    pub shortfall_nm: i64,
}

/// Signoff-grade measurement over drawn geometry, callable mid-search.
///
/// **Determinism contract (PLAN §8a, strengthened).** Every method here may gate
/// an accept/reject decision — a DRC verdict vetoes a committed move, an LVS
/// ambiguity vetoes a merge — and even the PEX *value* forks the Metropolis RNG
/// stream, because a different finite-difference gradient changes which uphill
/// draws are taken. So every implementation must be a **deterministic pure
/// function of `shapes`**: same shapes, same answer, on every backend, every
/// run. An oracle that cannot promise that (a nondeterministic GPU reduction, a
/// sampled random-walk PEX) must not be passed to a stage; PLAN §8a's partition
/// — "any output that gates a hard-constraint decision must be computed
/// deterministically" — collapses here to *all of it gates*, because the
/// objective and the gate share one trajectory. Stage determinism is therefore
/// over `(inputs, seed, prices, oracle)`.
pub trait Oracle {
    /// DRC over `shapes` (a bbox-scoped region or a whole layout). Any
    /// violation is grounds for the caller to veto the move that drew it.
    fn drc(&self, shapes: &[Shape]) -> DrcSummary;

    /// DRC over many candidate regions at once.
    ///
    /// **The reserved seam for the GPU batched-candidate lane** (PLAN §8's
    /// primary GPU win: thousands of hypothetical regions scored in parallel,
    /// the batch dimension supplying the parallelism one trajectory lacks). The
    /// default is the honest serial loop; do **not** change a caller's call
    /// granularity — per-accepted, risk-gated — before a batched implementation
    /// actually lands, because per-candidate at serial cost is the 95-minute
    /// arithmetic this module's docs exist to forbid.
    fn drc_batch(&self, regions: &[Vec<Shape>]) -> Vec<DrcSummary> {
        regions.iter().map(|r| self.drc(r)).collect()
    }

    /// Total extracted capacitance over `shapes`, fF — the finite-difference
    /// probe value for the FD-PEX gradient field. Deterministic like everything
    /// else here (see the trait doc: this value forks the RNG stream).
    fn pex_cap_ff(&self, shapes: &[Shape]) -> f32;

    /// Would extraction of `shapes` see something other than `expected_devices`
    /// distinct devices? `true` = ambiguous — the placement-induced
    /// implant-merge hazard of PLAN §3c: geometry that is DRC-legal but
    /// extracts as one wrong/ambiguous device. The caller treats `true` as a
    /// veto; a conservative implementation errs toward `true`.
    fn merge_ambiguous(&self, shapes: &[Shape], expected_devices: usize) -> bool;
}

/// The oracle that never vetoes and measures everything as zero.
///
/// Passing it reproduces pre-oracle behaviour **exactly**: no veto ever fires,
/// the FD-PEX gradient field is all zero (so the added linear cost term is
/// identically `0.0` and every Metropolis delta is unchanged), the epoch DRC
/// count is `0` (so the stop criterion is what it always was), and no method
/// consumes RNG. Byte-identical trajectories are the regression anchor for
/// every pre-oracle test.
pub struct NullOracle;

impl Oracle for NullOracle {
    fn drc(&self, _shapes: &[Shape]) -> DrcSummary {
        DrcSummary::default()
    }
    fn pex_cap_ff(&self, _shapes: &[Shape]) -> f32 {
        0.0
    }
    fn merge_ambiguous(&self, _shapes: &[Shape], _expected_devices: usize) -> bool {
        false
    }
}
