use crate::types::{
    Contractable, ConstraintContract, ConstraintStage, ConstraintStrength, ConstraintStatus,
    EsdProtectionType, NetConstraint,
};

/// Lightweight ESD requirement for a pad net (pre-typed).
#[derive(Debug, Clone)]
pub struct EsdRequirement {
    pub pad_name: String,
    pub protection_type: String,
    pub max_trigger_voltage_v: f64,
    pub ecgr_required: bool,
}

impl NetConstraint for EsdRequirement {
    fn net_name(&self) -> &str { &self.pad_name }
}

/// Typed ESD constraint for a pad net.
#[derive(Debug, Clone)]
pub struct EsdConstraint {
    pub pad_name: String,
    pub protection_type: EsdProtectionType,
    pub primary_clamp_required: bool,
    pub secondary_cdm_required: bool,
    pub max_bus_resistance_ohm: f64,
    pub ecgr_required: bool,
    pub silicide_block_required: bool,
}

impl NetConstraint for EsdConstraint {
    fn net_name(&self) -> &str { &self.pad_name }
}

impl Contractable for EsdConstraint {
    fn strength(&self) -> ConstraintStrength { ConstraintStrength::Hard }
    fn priority(&self) -> i32 { 90 }

    fn stages(&self) -> &[ConstraintStage] {
        &[ConstraintStage::CellGen, ConstraintStage::Routing]
    }

    fn to_contract(&self, _device_names: &[String]) -> ConstraintContract {
        ConstraintContract {
            constraint_id: format!("esd_{}", self.pad_name),
            kind: "esd".into(),
            scope: vec![self.pad_name.clone()],
            strength: self.strength(),
            priority: self.priority(),
            source: "esd_extractor".into(),
            source_confidence: 1.0,
            derived_from: Vec::new(),
            relaxation_policy: None,
            stage_consumption: self.stages().to_vec(),
            status: ConstraintStatus::Emitted,
            violation_metric: None,
            violation_units: None,
            waiver_reason: None,
            status_history: Vec::new(),
        }
    }
}
