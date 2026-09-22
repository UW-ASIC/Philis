//! Environmental / proximity effects (cell tier).
//!
//! Ported from `backend/constraints/src/cell_level/environment.rs`.

use pnr_core::ids::DeviceId;

/// **Environmental effects.** Layout-dependent phenomena that shift `Vth`: the
/// Well-Proximity Effect (scattered implant ions near a well edge), hydrogenation
/// diffusing under metal (up to ~20% current mismatch on matched gates), and
/// metal-over-gate / LOD stress. Tier-set well distances (2 µm minimal … 5 µm
/// exceptional) keep matched-gate `Vth` inside budget.
///
/// - **Role:** structural placement/geometry directive. Cold.
/// - **Books:** AOAL ch02/2.3.4, ch06/6.2 (#47); PNR_ANALOG 00/2.1, 2.2, 2.4
///   (#9, #10, #13–14), 02/4.A.1 (#48).
pub struct EnvironmentalConstraint {
    pub scope: Vec<DeviceId>,
    pub kind: EnvironmentalKind,
    /// Threshold magnitude; interpret with `units`.
    pub threshold: i32,
    /// Unit the `threshold` is expressed in (distance, ΔVth, current %, …).
    pub units: ThresholdUnit,
}

/// Which environmental phenomenon this constraint bounds.
// was: 3 variants; ported the real 6-variant sub-kind set.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EnvironmentalKind {
    /// Well-proximity effect, four-edge implant-scatter model (min well-edge dist).
    WpeFourEdge,
    /// LOD / STI stress via `SA`/`SB` diffusion asymmetry.
    LodSaSb,
    /// Dummy moat-extension requirement.
    DummyMoat,
    /// Hydrogenation keepout under metal.
    HydrogenationKeepout,
    /// Metal-over-gate stress.
    MetalOverGate,
    /// Thermal exclusion zone around a heat source.
    ThermalExclusion,
}

/// Unit a threshold is measured in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThresholdUnit {
    /// Distance, `nm`.
    Nm,
    /// Voltage shift, µV.
    Uv,
    /// Fractional mismatch, hundredths of a percent.
    Cpct,
    /// Temperature, milli-°C.
    Mcelsius,
}
