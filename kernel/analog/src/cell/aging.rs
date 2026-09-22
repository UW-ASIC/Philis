//! Aging / wear-out (cell tier).
//!
//! Ported from `backend/constraints/src/cell_level/aging.rs`.

use pnr_core::ids::DeviceId;

/// **Aging.** Wear-out mechanisms — HCI, NBTI, TDDB, GOI — drift `Vth`, gain, and
/// leakage over the device lifetime and must stay within spec at end-of-life.
/// Stress-accelerated models predict the drift; severity gates reliability signoff.
///
/// - **Role:** reliability annotation; consulted at signoff / by wire sizing. Cold.
/// - **Books:** AOAL ch01/1.4.1, ch04/4.2, ch05/5.1–5.3, ch13/13.2.3 (#39);
///   FOLD 7.2/7.2.2 (#18); PNR_ANALOG 02/2.A (#43).
pub struct AgingConstraint {
    pub device: DeviceId,
    pub mechanism: AgingMechanism,
    /// Advisory vs must-fix; gates whether signoff blocks on this drift.
    // was: severity: Severity {Warning,Violation}; kept the enum over a bool.
    pub severity: Severity,
    /// Human-readable note on the flagged condition (e.g. bias / duty-cycle).
    pub description: String,
}

/// Reliability severity: advisory warning vs blocking violation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Severity {
    Warning,
    Violation,
}

/// Dominant wear-out mechanism tagged on a device.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AgingMechanism {
    /// Hot-carrier injection.
    Hci,
    /// Negative-bias temperature instability.
    Nbti,
    /// Time-dependent dielectric breakdown.
    Tddb,
    /// Gate-oxide integrity.
    Goi,
}
