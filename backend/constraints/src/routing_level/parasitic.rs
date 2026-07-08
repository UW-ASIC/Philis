use crate::types::{
    Contractable, ConstraintContract, ConstraintStage, ConstraintStrength, ConstraintStatus,
    NetConstraint,
};

/// Parasitic budget for a net (max resistance and capacitance).
#[derive(Debug, Clone)]
pub struct ParasiticBudget {
    pub net_name: String,
    /// Maximum parasitic resistance (ohms).
    pub max_r: f64,
    /// Maximum parasitic capacitance (fF).
    pub max_c: f64,
}

impl NetConstraint for ParasiticBudget {
    fn net_name(&self) -> &str { &self.net_name }
}

impl Contractable for ParasiticBudget {
    fn strength(&self) -> ConstraintStrength { ConstraintStrength::Soft }
    fn priority(&self) -> i32 { 60 }

    fn stages(&self) -> &[ConstraintStage] {
        &[ConstraintStage::Routing]
    }

    fn to_contract(&self, _device_names: &[String]) -> ConstraintContract {
        ConstraintContract {
            constraint_id: format!("parasitic_{}", self.net_name),
            kind: "parasitic_budget".into(),
            scope: vec![self.net_name.clone()],
            strength: self.strength(),
            priority: self.priority(),
            source: "parasitic_extractor".into(),
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
