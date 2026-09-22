//! Unit-device decomposition (cell tier).
//!
//! Ported from `backend/constraints/src/cell_level/unitization.rs`.

use pnr_core::ids::DeviceId;
use pnr_core::netlist::DeviceKind;

/// **Unitization.** Decomposes devices into identical unit fingers/segments so
/// their ratios are exact integers — the precondition for good matching. Even
/// finger counts enable orientation symmetry; interdigitated / common-centroid
/// patterns cancel first-order doping and implant gradients (~60% lower Pelgrom
/// mismatch). Folding cuts gate resistance ∝ `1/N_f²`; LOD equalisation needs
/// `SA≈SB` across fingers. The unit geometry must be shared verbatim by every
/// instance in the group, and (for FETs) the same device variant.
///
/// - **Role:** structural — read by `cells` to pick and draw the variant. Cold.
/// - **Books:** AOAL ch01/1.3.1, 1.4, ch08/8.2.7, 8.3.2 (#64); FOLD 6.3 (#37–40);
///   ALS 5.7/5.7.2 (#23); PNR_ANALOG 01/3.A, 4.A (#29, #39).
#[derive(Clone)]
pub struct Unitization {
    pub devices: Vec<DeviceId>,
    /// Device class of the group (all instances share it).
    // was: DeviceType {Nmos,Pmos,Resistor,Capacitor,Bjt}; reuse netlist DeviceKind.
    pub device_type: DeviceKind,
    /// Per-device unit-finger/segment counts; ratios preserve target sizing.
    // was: instance_unit_counts: HashMap<String,u32> keyed by device name.
    pub dev_nf: Vec<u16>,
    /// Integer target ratio between instances (e.g. `[1, 4]` for a 1:4 mirror).
    // was: target_ratio: HashMap<String,u32>.
    pub target_ratio: Vec<u16>,
    /// Unit finger/segment width, `nm`.
    // was: unit_geometry["w"] in angstroms.
    pub unit_w: i32,
    /// Unit finger/segment length, `nm`.
    // was: unit_geometry["l"] in angstroms.
    pub unit_l: i32,
    /// Allowed composition of the units into each instance.
    pub series_parallel: SeriesParallel,
    // Layout pattern (Interdig/Cc*) is NOT a field: the cell generator chooses the
    // best pattern internally from this unitization + the matching constraint.
    /// FET instances must use the identical device variant (same flavour/oxide).
    pub same_variant_required: bool,
    pub dummy_required: bool,
    pub route_matching_required: bool,
}

/// How unit elements compose into a device instance.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SeriesParallel {
    /// Units in parallel (fingers of one transistor, parallel resistor segments).
    Parallel,
    /// Units in series (stacked resistor/cap segments).
    Series,
    /// Repeated identical stages (e.g. ladder rungs).
    RepeatedStage,
}
