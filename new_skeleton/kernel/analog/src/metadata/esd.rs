//! ESD protection (metadata tier).
//!
//! Ported from `backend/constraints/src/metadata/esd.rs`.

/// **ESD constraint.** An ESD event injects `>100 A` in `<1 ns`; each pad needs
/// layered clamps (primary transient suppressor, secondary CDM catch diode, series
/// limiting) so the voltage never exceeds device snapback. Clamp distance, metal
/// width, and silicide use set the parasitic resistance that bounds the transient
/// peak — hence a hard bus-resistance budget.
///
/// - **Role:** pad metadata — drives `cells` (which clamps) and routing (wire the
///   clamps within the resistance budget). Hard (priority 90).
/// - **Books:** AOAL ch01/1.4.2, ch05/5.1–5.4, ch14/14.4 (#48); FOLD 7.4 (#24–28);
///   PNR_ANALOG 02/5.A.1 (#54).
#[derive(Clone, Copy, Debug)]
pub struct EsdConstraint {
    pub pad: pnr_core::ids::NetId, // was: pad_name: String
    pub protection: EsdProtectionType,
    pub primary_clamp_required: bool,
    pub secondary_cdm_required: bool,
    /// Max bus resistance from pad to clamp, milli-ohm. was: ohm: f64
    pub max_bus_r_mohm: i64,
    /// Effective ground-return required (antiparallel diodes between ground lines).
    pub ecgr_required: bool,
    pub silicide_block_required: bool,
}

/// ESD protection topology required at a pad.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EsdProtectionType {
    Primary,
    Secondary,
    Cdm,
    RailClamp,
}
