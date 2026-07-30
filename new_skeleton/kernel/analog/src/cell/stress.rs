//! Die-stress placement bound (cell tier).
//!
//! Ported from `backend/constraints/src/cell_level/stress.rs`.

use pnr_core::ids::DeviceId;

/// **Package/die stress.** CTE mismatch and package deformation impose a
/// mechanical stress field that varies with distance from die centre; strain
/// modulates carrier mobility (~10% at the corners). Matched devices must sit
/// away from high-gradient edges. Tier thresholds: ≥200 µm (minimal), ≥500 µm
/// (moderate), central half of die (exceptional).
///
/// - **Role:** structural placement bound (soft in practice). Cold.
/// - **Books:** AOAL ch02/2.8, ch08/8.2.8, ch13/13.2.5 (#61); FOLD 6.6/6.6.3–6.6.4
///   (#7, #9, #33); PNR_ANALOG 00/2.3 (#11–12), 02/4.A.3 (#49).
#[derive(Clone, Copy)]
pub struct StressBound {
    pub device: DeviceId,
    /// Max distance from the die stress-centroid, `nm`.
    // was: max_centroid_distance_um: f64.
    pub max_centroid_distance_nm: i32,
}
