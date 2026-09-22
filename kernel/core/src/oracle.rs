//! Signoff-grade measurement callable mid-search. `verify::LiveOracle` is the
//! real impl; [`NullOracle`] reproduces oracle-free behaviour exactly. Lives
//! here so `dp` can take an oracle without depending on the verifier.

use crate::geom::Shape;

/// What a DRC pass found.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DrcSummary {
    /// Findings in the checked region. `0` = clean.
    pub violations: u32,
    /// Σ (limit − measured) over the findings, nm.
    pub shortfall_nm: i64,
}

/// Every method gates an accept/reject decision (or forks the RNG stream), so
/// every impl must be a **deterministic pure function of `shapes`**.
pub trait Oracle {
    /// DRC over `shapes` (a region or a whole layout).
    fn drc(&self, shapes: &[Shape]) -> DrcSummary;

    /// Total extracted capacitance over `shapes`, fF.
    fn pex_cap_ff(&self, shapes: &[Shape]) -> f32;

    /// `true` when extraction of `shapes` would not yield exactly
    /// `expected_devices` devices (implant-merge hazard). Callers veto on `true`;
    /// impls should err toward `true`.
    fn merge_ambiguous(&self, shapes: &[Shape], expected_devices: usize) -> bool;
}

/// Never vetoes, measures everything as zero, consumes no RNG.
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
