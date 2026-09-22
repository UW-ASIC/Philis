//! Cell-tier structural directives, read cold by `cells` (not scored per move).

use pnr_core::ids::{DeviceId, NetId};
use pnr_core::netlist::DeviceKind;

/// Dummies at a matched device's edges so edge fingers see the same etch/LOD
/// environment as interior ones.
#[derive(Clone, Copy)]
pub struct DummyRequirement {
    pub device: DeviceId,
    pub dummy_type: DummyType,
    /// Moat extension beyond active, nm.
    pub moat_ext_nm: i32,
    /// Min poly-to-dummy clearance, nm.
    pub min_poly_clearance_nm: i32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DummyType {
    /// Poly gate only.
    GateDummy,
    /// Diffusion only (LOD `SA`/`SB`).
    MoatDummy,
    /// Gate + moat.
    Full,
}

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

/// Layout-dependent-effect (WPE + LOD/STI) bounds for a matched pair.
/// Units: distances nm, ΔVth µV, current mismatch hundredths of a percent.
#[derive(Clone, Copy)]
pub struct LdeBound {
    pub pair: (DeviceId, DeviceId),
    pub min_well_edge_distance_nm: i32,
    pub max_wpe_dvth_uv: i32,
    pub max_lod_did_cpct: i32,
    pub max_sa_sb_mismatch_nm: i32,
    pub outer_sa_nm: i32,
    pub outer_sb_nm: i32,
    pub inner_sa_nm: i32,
    pub inner_sb_nm: i32,
    pub sti_dvth_uv: i32,
    pub same_width: bool,
    pub same_orientation: bool,
}

/// Keep `device` within this distance of the die stress centroid.
#[derive(Clone, Copy)]
pub struct StressBound {
    pub device: DeviceId,
    pub max_centroid_distance_nm: i32,
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
