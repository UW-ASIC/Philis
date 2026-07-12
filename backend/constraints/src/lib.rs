//! Constraint extraction types for analog PNR.
//!
//! One file per constraint. Each struct implements its primary scope trait
//! (`PairConstraint`, `DeviceConstraint`, `NetConstraint`, `GroupConstraint`)
//! plus `Contractable` when it participates in lifecycle tracking.

pub mod contract;
mod record;
pub mod types;

pub mod cell_level;
pub mod metadata;
pub mod placement_level;
pub mod routing_level;

pub use record::ConstraintRecord;
pub use types::*;

// ── Re-export constraint structs at crate root ──
pub use cell_level::aging::AgingConstraint;
pub use cell_level::dummy::DummyConstraint;
pub use cell_level::environment::EnvironmentalConstraint;
pub use cell_level::guard_ring::GuardRingRequirement;
pub use cell_level::injector::{
    detect_injectors, exclude_injectors_from_clusters, InjectorCandidate,
};
pub use cell_level::lde::LdeBound;
pub use cell_level::stress::StressConstraint;
pub use cell_level::unitization::UnitizationConstraint;

pub use routing_level::align::StraightNet;
pub use routing_level::antenna::AntennaConstraint;
pub use routing_level::crosstalk::CrosstalkExclusion;
pub use routing_level::current_flow::CurrentFlowTag;
pub use routing_level::differential::{DifferentialPair, RouteMatchingTolerance};
pub use routing_level::parasitic::{ParasiticBudget, WireParasiticParams};

pub use placement_level::cc::CcGroup;
pub use placement_level::dti::DtiPair;
pub use placement_level::isolation::IsolationConstraint;
pub use placement_level::proximity::ProximityRule;
pub use placement_level::symmetry::{width_ratio, MatchingPair, SymmetryGroup};
pub use placement_level::thermal::ThermalGradientConstraint;

pub use metadata::bias_current::BiasCurrentTag;
pub use metadata::classify::NetClassification;
pub use metadata::esd::EsdConstraint;

pub use contract::{validate_constraint_record, ContractValidation};

// ── Well-known net-name patterns ──

pub const SUPPLY_NAMES: &[&str] = &["vdd", "vcc", "avdd", "dvdd", "vdd!", "vcc!"];
pub const GROUND_NAMES: &[&str] = &["gnd", "vss", "avss", "dvss", "gnd!", "vss!", "0"];
pub const SUBSTRATE_NAMES: &[&str] = &["sub", "psub", "bulk", "vsub"];
pub const CLK_PATTERNS: &[&str] = &["clk", "clock", "phi1", "phi2", "ck"];
