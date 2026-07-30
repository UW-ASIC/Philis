//! PEX — parasitic extraction over drawn geometry via gdsverify.
//!
//! PEX is the one check that isn't pass/fail: it yields R/C parasitics that feed
//! a **Cost** rule in the loop (route for less parasitic delay) and populate the
//! `cost` field of the signoff report. `is_complete()` is still a hard gate —
//! an extractor diagnostic (geometry it refused to model) blocks tapeout.

use pnr_core::Shape;

use crate::geom::store_from_shapes;
use crate::pdk::Pdk;

pub use gdsverify::{Parasitic, PexReport};

/// Analytical PEX over drawn geometry: sheet-R, area/fringe/coupling C.
#[must_use]
pub fn run_pex(shapes: &[Shape], pdk: &Pdk) -> PexReport {
    let store = store_from_shapes(shapes, pdk);
    gdsverify::run_pex(&store, &pdk.deck)
}

/// Total extracted capacitance in femtofarads — the scalar the Cost rule pays.
/// gdsverify reports attofarads; /1000 → fF.
#[must_use]
pub fn total_cap_ff(report: &PexReport) -> f32 {
    (report.total_cap() / 1000.0) as f32
}
