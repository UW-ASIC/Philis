//! Guard rings (cell tier).
//!
//! Ported from `backend/constraints/src/cell_level/guard_ring.rs`.

use pnr_core::ids::{DeviceId, NetId};

/// **Guard ring.** A parasitic SCR forms wherever adjacent NMOS/PMOS let
/// `β_npn·β_pnp ≥ 1`; forward injection then latches. Guard rings collect or
/// block injected carriers before they reach the opposite well, preventing
/// latch-up. Ring width follows a resistance target (`< 10 Ω`); tap pitch bounds
/// debiasing; a *complete* enclosure prevents current bypass.
///
/// - **Role:** structural — `cells` draws the ring; completeness is checked at
///   signoff. Hard.
/// - **Books:** AOAL ch05/5.4, ch09/9.1.3, ch14/14.2.3–14.2.5 (#49–50);
///   FOLD 7.1/7.1.3 (#15–16); PNR_ANALOG 00/4.6, 01/6.A (#25–26, #34).
#[derive(Clone, Copy)]
pub struct GuardRingRequirement {
    pub device: DeviceId,
    pub ring_type: GuardRingType,
    pub shareable: bool,
    /// Tap-contact pitch, `nm`; bounds ring debiasing.
    // was: tap_pitch_um: f64.
    pub tap_pitch_nm: i32,
    /// Min ring metal width, `nm`.
    // was: min_width_um: f64.
    pub min_width_nm: i32,
    /// Max ring resistance target, milli-ohm (`< 10 Ω` ⇒ `< 10_000`).
    // was: max_ring_resistance_ohm: f64.
    pub max_ring_resistance_mohm: i64,
    /// Ring must fully enclose (no gap for current bypass).
    pub enclosure_complete: bool,
    /// Net the ring ties to (typically the substrate/well tap rail).
    // was: connection_net: String.
    pub connection_net: NetId,
}

/// Guard-ring flavour (electron-collecting / hole-collecting / blocking variants).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GuardRingType {
    /// Electron-collecting (n+ in p-sub).
    Ecgr,
    /// Hole-collecting (p+ in n-well).
    Hcgr,
    /// Hole-blocking.
    Hbgr,
    /// Electron-blocking.
    Ebgr,
}
