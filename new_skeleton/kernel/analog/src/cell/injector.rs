//! Substrate injectors (cell tier).
//!
//! Ported from `backend/constraints/src/cell_level/injector.rs`.

use pnr_core::ids::DeviceId;

/// **Substrate injector.** Charge pumps, ESD diodes, and low-series-R pads pump
/// minority carriers into the substrate. A series-R walk from pad nets flags
/// low-resistance injection paths (`< 1 kΩ`); an injector may **not** share a
/// guard ring with clean devices — it is forced into a private (singleton) ring
/// cluster. "Guard the injector" (AOAL ch14) contains the injection current.
///
/// - **Role:** structural — drives singleton guard-ring clustering in `cells`. Cold.
/// - **Books:** AOAL ch14/14.1–14.2 (#20, #51h); PNR_ANALOG 02/2.A (#42).
// ponytail: this is the DATA result only. The series-R pad-net graph walk and the
// cluster-exclusion pass (source `detect_injectors` / `exclude_injectors_from_clusters`)
// are algorithm code and live in `cells`, not in this zero-dep theory crate.
pub struct InjectorCandidate {
    pub device: DeviceId,
    /// Series resistance from the pad net, milli-ohm.
    // was: series_r_ohm: f64.
    pub series_r_mohm: i64,
    pub is_injector: bool,
}
