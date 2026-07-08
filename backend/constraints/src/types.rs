//! Constraint trait taxonomy and core types.
//!
//! Five traits partition constraints by *how downstream stages consume them*,
//! not by what physical effect they describe:
//!
//! - [`PairConstraint`]   — two devices + a spatial/electrical budget
//! - [`DeviceConstraint`] — tags a single device
//! - [`NetConstraint`]    — scoped to a net name
//! - [`GroupConstraint`]  — clusters of devices (symmetry, CC, unitization)
//! - [`Contractable`]     — lifecycle-tracked through the pipeline
//!
//! A constraint type implements one primary scope trait (Pair/Device/Net/Group)
//! plus [`Contractable`] when it participates in contract coverage.

// ───────────────────────────────────────────────────────────────────
//  Arena-index newtypes — u32, not usize. Option<NonZero> is free.
// ───────────────────────────────────────────────────────────────────

/// Index into the device arena. `u32` — 4 B, niche-optimizable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct DeviceId(pub u32);

/// Index into the net arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct NetId(pub u32);

/// Sentinel for unresolvable device names.
pub const DEVICE_ID_UNKNOWN: DeviceId = DeviceId(u32::MAX);

// ───────────────────────────────────────────────────────────────────
//  Enums — closed sets, encode don't polymorphize
// ───────────────────────────────────────────────────────────────────

/// Matching quality tier. Ordered: None < Minimal < Moderate < Exceptional.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(u8)]
pub enum MatchingTier {
    #[default]
    None = 0,
    Minimal = 1,
    Moderate = 2,
    Exceptional = 3,
}

/// How a matched pair relates electrically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MatchingType {
    DiffPair,
    Mirror,
    Ratio,
    Passive,
}

/// Pipeline stage that consumes a constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ConstraintStage {
    CellGen,
    Placement,
    Routing,
    Signoff,
}

/// Hard vs soft enforcement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ConstraintStrength {
    Soft,
    Hard,
}

/// Constraint lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ConstraintStatus {
    Emitted,
    Consumed,
    Satisfied,
    Violated,
    Waived,
}

/// Current flow direction for orientation constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CurrentFlowDir {
    LeftToRight,
    RightToLeft,
    Bidirectional,
}

/// Net classification for routing constraint derivation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum NetClass {
    Signal,
    Clock,
    Supply,
    Ground,
    Sensitive,
    Substrate,
}

/// Common-centroid interdigitation pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PatternType {
    Abba,
    Abab,
    CommonCentroid2d,
}

/// Dummy device type at cell edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DummyType {
    GateDummy,
    MoatDummy,
    Full,
}

/// Aging mechanism categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum AgingMechanism {
    /// Hot Carrier Injection — short-channel NMOS degradation.
    Hci,
    /// Negative Bias Temperature Instability — PMOS Vth shift.
    Nbti,
    /// Time-Dependent Dielectric Breakdown — oxide field limit.
    Tddb,
    /// Gate Oxide Integrity — gettering distance from N+/NBL.
    Goi,
}

/// Device type for unitization decomposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DeviceType {
    Nmos,
    Pmos,
    Resistor,
    Capacitor,
    Bjt,
}

/// Guard ring type for latchup/noise isolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum GuardRingType {
    PsubRing,
    NwellRing,
    DoubleRing,
}

/// ESD protection type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum EsdProtectionType {
    Primary,
    Secondary,
    Cdm,
    RailClamp,
}

/// Shielding type for sensitive nets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ShieldType {
    None,
    Grounded,
    DiffShield,
}

/// Allowed cell orientation transforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Orientation {
    R0,
    R90,
    R180,
    R270,
    Mx,
    My,
    Mxy,
    Myx,
}

/// SMP edge type in the symmetry-matching-proximity multigraph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SmpEdgeType {
    Symmetry,
    Matching,
    Proximity,
}

/// HSMPG node type in the hierarchical clustering tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum HsmpgNodeType {
    Leaf,
    SymmetryCluster,
    ProximityCluster,
    Root,
}

/// Voltage domain tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum VoltDomain {
    Core,
    Io,
    Analog,
}

/// Environmental constraint sub-kind (tagged union discriminant).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum EnvironmentalKind {
    WpeFourEdge,
    LodSaSb,
    DummyMoat,
    HydrogenationKeepout,
    MetalOverGate,
    ThermalExclusion,
}

/// Recognized analog building block type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BlockType {
    DiffPair,
    CurrentMirror,
    CascodeMirror,
    CascodePair,
    TailSource,
}

// ───────────────────────────────────────────────────────────────────
//  Traits — the five behavioral interfaces
// ───────────────────────────────────────────────────────────────────

/// A constraint between two devices — placement iterates these for
/// cross-cell cost terms, verify checks measured values against budgets.
///
/// Access pattern: hot in placement inner loop (device_a, device_b,
/// distance_budget read together per element → AoS is correct here).
pub trait PairConstraint {
    fn device_a(&self) -> DeviceId;
    fn device_b(&self) -> DeviceId;

    /// Budget distance in um. 0.0 = "as close as possible" (attraction).
    /// Placement converts to angstroms; verify compares against measured.
    fn distance_budget_um(&self) -> f64;
}

/// A constraint that tags a single device — placement looks these up
/// via device→cell mapping, cell-gen consumes during generation.
pub trait DeviceConstraint {
    fn device_id(&self) -> DeviceId;
}

/// A constraint scoped to a net — routing uses for topology selection,
/// EM derate, budget-aware rip-up; verify checks post-PEX.
pub trait NetConstraint {
    fn net_name(&self) -> &str;
}

/// A constraint that defines a cluster of devices — cell-gen decomposes
/// into interdigitated unit cells, placement treats intra-group as one cell.
pub trait GroupConstraint {
    fn devices(&self) -> &[DeviceId];
}

/// Lifecycle-tracked constraint. Every constraint that has a `contract_for_*`
/// builder implements this. Placement reads priority×strength to scale cost;
/// verify aggregates coverage.
pub trait Contractable {
    fn strength(&self) -> ConstraintStrength;
    fn priority(&self) -> i32;
    fn stages(&self) -> &[ConstraintStage];
    fn to_contract(&self, device_names: &[String]) -> ConstraintContract;
}

// ───────────────────────────────────────────────────────────────────
//  Contract lifecycle record
// ───────────────────────────────────────────────────────────────────

/// Tracked constraint with lifecycle status.
pub struct ConstraintContract {
    pub constraint_id: String,
    pub kind: String,
    pub scope: Vec<String>,
    pub strength: ConstraintStrength,
    pub priority: i32,
    pub source: String,
    pub source_confidence: f64,
    pub derived_from: Vec<String>,
    pub relaxation_policy: Option<String>,
    pub stage_consumption: Vec<ConstraintStage>,
    pub status: ConstraintStatus,
    pub violation_metric: Option<f64>,
    pub violation_units: Option<String>,
    pub waiver_reason: Option<String>,
    pub status_history: Vec<StatusEntry>,
}

impl ConstraintContract {
    pub fn consume(&mut self, stage: &str) {
        self.status = ConstraintStatus::Consumed;
        self.status_history.push(StatusEntry {
            stage: stage.into(),
            status: ConstraintStatus::Consumed,
            evidence: None,
            metric: None,
        });
    }

    pub fn satisfy(&mut self, stage: &str, evidence: &str) {
        self.status = ConstraintStatus::Satisfied;
        self.status_history.push(StatusEntry {
            stage: stage.into(),
            status: ConstraintStatus::Satisfied,
            evidence: Some(evidence.into()),
            metric: None,
        });
    }

    pub fn violate(&mut self, stage: &str, metric: f64, units: &str) {
        self.status = ConstraintStatus::Violated;
        self.violation_metric = Some(metric);
        self.violation_units = Some(units.into());
        self.status_history.push(StatusEntry {
            stage: stage.into(),
            status: ConstraintStatus::Violated,
            evidence: None,
            metric: Some(metric),
        });
    }

    pub fn waive(&mut self, stage: &str, reason: &str) {
        self.status = ConstraintStatus::Waived;
        self.waiver_reason = Some(reason.into());
        self.status_history.push(StatusEntry {
            stage: stage.into(),
            status: ConstraintStatus::Waived,
            evidence: Some(reason.into()),
            metric: None,
        });
    }
}

/// One entry in the constraint lifecycle history.
pub struct StatusEntry {
    pub stage: String,
    pub status: ConstraintStatus,
    pub evidence: Option<String>,
    pub metric: Option<f64>,
}
