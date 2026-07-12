//! Stage-partitioned analog constraint storage.
//!
//! The record is cold flow data: each stage reads contiguous vectors for the
//! constraint families it consumes. It belongs here—not in an algorithm
//! crate—so cells, engine digestion, placement, routing, and signoff share one
//! dependency-neutral contract.

use crate::{
    AgingConstraint, AntennaConstraint, BiasCurrentTag, CcGroup, CrosstalkExclusion,
    CurrentFlowTag, DifferentialPair, DtiPair, DummyConstraint, EnvironmentalConstraint,
    EsdConstraint, GuardRingRequirement, IsolationConstraint, LdeBound, NetClassification,
    ParasiticBudget, ProximityRule, StraightNet, StressConstraint, SymmetryGroup,
    ThermalGradientConstraint, UnitizationConstraint,
};

/// Constraint vectors grouped by their primary consumer and access pattern.
#[derive(Clone, Default)]
pub struct ConstraintRecord {
    // Placement-level vectors.
    pub symmetry: Vec<SymmetryGroup>,
    pub cc: Vec<CcGroup>,
    pub proximity: Vec<ProximityRule>,
    pub isolation: Vec<IsolationConstraint>,
    pub thermal: Vec<ThermalGradientConstraint>,
    pub stress: Vec<StressConstraint>,
    pub dti: Vec<DtiPair>,
    pub guard_ring: Vec<GuardRingRequirement>,
    pub environment: Vec<EnvironmentalConstraint>,

    // Cell-level vectors.
    pub lde: Vec<LdeBound>,
    pub dummy: Vec<DummyConstraint>,
    pub unitization: Vec<UnitizationConstraint>,
    pub aging: Vec<AgingConstraint>,

    // Routing-level vectors.
    pub net_class: Vec<NetClassification>,
    pub crosstalk: Vec<CrosstalkExclusion>,
    pub straight: Vec<StraightNet>,
    pub parasitic: Vec<ParasiticBudget>,
    pub differential: Vec<DifferentialPair>,
    pub antenna: Vec<AntennaConstraint>,
    pub current_flow: Vec<CurrentFlowTag>,

    // Cross-stage metadata.
    pub esd: Vec<EsdConstraint>,
    pub bias_current: Vec<BiasCurrentTag>,
}
