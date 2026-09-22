//! Dummy devices (cell tier).
//!
//! Ported from `backend/constraints/src/cell_level/dummy.rs`.

use pnr_core::ids::DeviceId;

/// **Dummy devices.** Dummies at a matched device's edges equalise the etch and
/// diffusion environment so *active* edge fingers see the same surroundings as
/// interior ones, cancelling LOD (length-of-diffusion) mismatch from asymmetric
/// `SA`/`SB`. Moat extension and poly-to-dummy clearance control fringing fields;
/// the dummy flavour (gate-only, moat, or full) trades area against how much of
/// the edge environment it reproduces.
///
/// - **Role:** structural — `cells` emits the dummy geometry. Cold; Hard for
///   Moderate+ matching tiers.
/// - **Books:** AOAL ch02/2.3.3–2.7.5, ch08/8.2.3, ch13/13.2.2 (#46);
///   FOLD 6.6/6.6.3 (#5); PNR_ANALOG 01/5.A (#33).
#[derive(Clone, Copy)]
pub struct DummyRequirement {
    pub device: DeviceId,
    pub dummy_type: DummyType,
    /// Moat extension beyond active, `nm`.
    // was: moat_ext_um: f64.
    pub moat_ext_nm: i32,
    /// Min poly-to-dummy clearance, `nm`.
    // was: min_poly_clearance_um: f64.
    pub min_poly_clearance_nm: i32,
}

/// Dummy flavour drawn at the cell edge.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DummyType {
    /// Poly gate dummy only (matches poly-edge lithography).
    GateDummy,
    /// Moat/diffusion dummy (matches active-edge etch, LOD `SA`/`SB`).
    MoatDummy,
    /// Full dummy device (gate + moat); required for `L < 1 µm`, Moderate+ tiers.
    Full,
}
