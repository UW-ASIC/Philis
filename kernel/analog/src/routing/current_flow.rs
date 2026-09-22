//! Current-flow direction tag (routing tier).
//!
//! Ported from `backend/constraints/src/routing_level/current_flow.rs`.

use pnr_core::ids::DeviceId;

/// **Current-flow tag.** *Not a scored rule — a directional annotation* that makes
/// routing electromigration-aware. Unidirectional DC current is far more damaging
/// than AC; tagging a device's current direction lets the router avoid 90° bends
/// (EM hot spots), prefer obtuse bends, and split current. EM lifetime follows
/// Black's law `MTTF ∝ 1/Jⁿ` (`n≈2`), so direction awareness is a reliability lever.
///
/// - **Role:** metadata that *shapes* how routing rules apply; consumed by `gr`/`dr`.
/// - **Arity:** Device→Direction (directional).
/// - **Books:** FOLD 7.5/7.5.1 (#29–31); PNR_ANALOG 00/4.1 (#20).
#[derive(Clone, Copy)]
pub struct CurrentFlowTag {
    pub device: DeviceId,
    pub dir: CurrentDir,
}

/// Direction of dominant DC current through a device terminal pair.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CurrentDir {
    LtoR,
    RtoL,
    Bidirectional,
}
