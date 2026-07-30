//! Bias-current tag (metadata tier).
//!
//! Ported from `backend/constraints/src/metadata/bias_current.rs`.

use pnr_core::ids::DeviceId;

/// **Bias-current tag.** Records a device's operating point. Current magnitude and
/// overdrive set electromigration lifetime (Black's law `MTTF ∝ J⁻²`) and HCI risk
/// (worst near `Vgs ≈ 0.4·Vds`). Wire sizing and aging analysis read this; untagged
/// devices fall back to nominal assumptions.
///
/// - **Role:** device metadata; consumed by routing (wire sizing) and reliability.
///   Not scored.
/// - **Books:** AOAL ch05/5.1 (#2); PNR_ANALOG 00/4.1 (#20), 02/2.A (#43).
#[derive(Clone, Copy)]
pub struct BiasCurrentTag {
    pub device: DeviceId,
    /// Drain current, µA.
    pub id_ua: i32,
    /// Overdrive `Vgs − Vth`, mV.
    pub vov_mv: i32,
}
