//! Cell-tier structural directives, read cold by `cells` (not scored per move).

use pnr_core::ids::{DeviceId, NetId};
use pnr_core::netlist::DeviceKind;

/// A latch-up guard ring around `device`, tied to `connection_net`.
#[derive(Clone, Copy)]
pub struct GuardRingRequirement {
    pub device: DeviceId,
    pub ring_type: GuardRingType,
    /// May merge with a neighbouring ring of the same class.
    pub shareable: bool,
    /// Tap-contact pitch, nm.
    pub tap_pitch_nm: i32,
    /// Min ring metal width, nm.
    pub min_width_nm: i32,
    /// Max ring resistance, mΩ.
    pub max_ring_resistance_mohm: i64,
    /// Ring must fully enclose the device.
    pub enclosure_complete: bool,
    /// Net the ring taps (substrate/well rail).
    pub connection_net: NetId,
}

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

/// Decompose `devices` into identical unit fingers/segments so ratios are
/// exact integers. Every instance shares the unit geometry (and, for FETs, the
/// variant). The layout pattern is chosen by the cell generator.
#[derive(Clone)]
pub struct Unitization {
    pub devices: Vec<DeviceId>,
    pub device_type: DeviceKind,
    /// Unit count per device.
    pub dev_nf: Vec<u16>,
    /// Integer ratio between instances, e.g. `[1, 4]`.
    pub target_ratio: Vec<u16>,
    /// Unit width, nm.
    pub unit_w: i32,
    /// Unit length, nm.
    pub unit_l: i32,
    pub series_parallel: SeriesParallel,
    pub same_variant_required: bool,
    pub dummy_required: bool,
    pub route_matching_required: bool,
}

/// How units compose into one instance.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SeriesParallel {
    Parallel,
    Series,
    RepeatedStage,
}
