//! Constraint extraction types for analog PNR.
//!
//! One file per constraint. Each struct implements its primary scope trait
//! (`PairConstraint`, `DeviceConstraint`, `NetConstraint`, `GroupConstraint`)
//! plus `Contractable` when it participates in lifecycle tracking.

pub mod types;

pub mod aging;
pub mod align;
pub mod antenna;
pub mod bias_current;
pub mod blocks;
pub mod cc;
pub mod classify;
pub mod contract;
pub mod crosstalk;
pub mod current_flow;
pub mod dti;
pub mod dummy;
pub mod environment;
pub mod esd;
pub mod guard_ring;
pub mod isolation;
pub mod lde;
pub mod matching_pair;
pub mod matching_spec;
pub mod parasitic;
pub mod proximity;
pub mod smp;
pub mod stress;
pub mod symmetry;
pub mod thermal;
pub mod unitization;

pub use types::*;

// ── Re-export constraint structs at crate root ──
pub use aging::AgingConstraint;
pub use align::StraightNet;
pub use antenna::AntennaConstraint;
pub use bias_current::BiasCurrentTag;
pub use blocks::AnalogBlock;
pub use cc::CcGroup;
pub use classify::NetClassification;
pub use contract::{validate_constraint_record, ContractValidation};
pub use crosstalk::CrosstalkExclusion;
pub use current_flow::CurrentFlowTag;
pub use dti::DtiPair;
pub use dummy::DummyConstraint;
pub use environment::EnvironmentalConstraint;
pub use esd::{EsdConstraint, EsdRequirement};
pub use guard_ring::GuardRingRequirement;
pub use isolation::IsolationConstraint;
pub use lde::LdeBound;
pub use matching_spec::{MatchingSpec, RouteMatchingTolerance};
pub use parasitic::ParasiticBudget;
pub use proximity::ProximityRule;
pub use smp::{HsmpgNode, SmpEdge};
pub use stress::StressConstraint;
pub use symmetry::{MatchingPair, SymmetryGroup};
pub use thermal::{ThermalGradientConstraint, ThermalTag};
pub use unitization::UnitizationConstraint;

// ── Well-known net-name patterns ──

pub const SUPPLY_NAMES: &[&str] = &["vdd", "vcc", "avdd", "dvdd", "vdd!", "vcc!"];
pub const GROUND_NAMES: &[&str] = &["gnd", "vss", "avss", "dvss", "gnd!", "vss!", "0"];
pub const SUBSTRATE_NAMES: &[&str] = &["sub", "psub", "bulk", "vsub"];
pub const CLK_PATTERNS: &[&str] = &["clk", "clock", "phi1", "phi2", "ck"];
