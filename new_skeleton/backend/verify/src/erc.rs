//! ERC — electrical-rule check over drawn geometry via gdsverify.
//!
//! Geometry-derived heuristic findings (floating well, missing tie, supply
//! short, floating gate, …) plus the six-family signoff suite (antenna, density
//! compare, IR drop, EM, reliability, ESD/latchup).
//!
//! ERC routes through the backend selector for symmetry with DRC, but note:
//! gdsverify's ERC has no GPU seam of its own, so `Backend::Gpu` still executes
//! on the CPU (per gdsverify) — the backend arg only carries telemetry intent.

use pnr_core::Shape;

use crate::drc::inloop_backend;
use crate::geom::store_from_shapes;
use crate::pdk::Pdk;

pub use gdsverify::{ErcReport, ErcViolation, SignoffCheck, SignoffConfig};

/// Run ERC over drawn geometry with the given signoff config (evidence for the
/// six-family suite). A default config runs the connectivity/heuristic checks
/// and reports the advanced families as NotRun (blocking by construction).
#[must_use]
pub fn run_erc(shapes: &[Shape], pdk: &Pdk, config: &SignoffConfig) -> ErcReport {
    let store = store_from_shapes(shapes, pdk);
    // Prefer the explicit-backend entry (records telemetry); it falls back to
    // the plain path if connectivity extraction fails.
    match gdsverify::run_erc_backend(&store, &pdk.deck, config, inloop_backend()) {
        Ok((report, _telemetry)) => report,
        Err(_) => gdsverify::run_erc(&store, &pdk.deck, config),
    }
}

/// ERC with the default (evidence-free) config — the in-loop path. Surfaces the
/// heuristic connectivity findings without demanding tapeout-only evidence.
#[must_use]
pub fn run_erc_inloop(shapes: &[Shape], pdk: &Pdk) -> ErcReport {
    run_erc(shapes, pdk, &SignoffConfig::default())
}
