use crate::types::{
    ConstraintContract, ConstraintStage, ConstraintStatus, ConstraintStrength, Contractable,
    NetConstraint,
};

/// Antenna ratio constraint for a net.
#[derive(Debug, Clone)]
pub struct AntennaConstraint {
    pub net_name: String,
    /// Maximum allowed metal-area-to-gate-area ratio.
    pub max_ratio: f64,
    /// Whether antenna repair must maintain matched symmetry.
    pub matched_symmetric_repair: bool,
}

impl NetConstraint for AntennaConstraint {
    fn net_name(&self) -> &str {
        &self.net_name
    }
}

impl Contractable for AntennaConstraint {
    fn strength(&self) -> ConstraintStrength {
        ConstraintStrength::Hard
    }
    fn priority(&self) -> i32 {
        75
    }

    fn stages(&self) -> &[ConstraintStage] {
        &[ConstraintStage::Routing, ConstraintStage::Signoff]
    }

    fn to_contract(&self, _device_names: &[String]) -> ConstraintContract {
        ConstraintContract {
            constraint_id: format!("antenna_{}", self.net_name),
            kind: "antenna".into(),
            scope: vec![self.net_name.clone()],
            strength: self.strength(),
            priority: self.priority(),
            source: "antenna_extractor".into(),
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
