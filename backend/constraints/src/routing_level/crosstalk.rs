use crate::types::{
    Contractable, ConstraintContract, ConstraintStage, ConstraintStrength, ConstraintStatus,
};

/// Crosstalk exclusion between an aggressor and victim net pair.
#[derive(Debug, Clone)]
pub struct CrosstalkExclusion {
    pub net_a: String,
    pub net_b: String,
    /// Minimum routing spacing (um).
    pub min_spacing_um: f64,
}

impl Contractable for CrosstalkExclusion {
    fn strength(&self) -> ConstraintStrength { ConstraintStrength::Hard }
    fn priority(&self) -> i32 { 70 }

    fn stages(&self) -> &[ConstraintStage] {
        &[ConstraintStage::Routing]
    }

    fn to_contract(&self, _device_names: &[String]) -> ConstraintContract {
        ConstraintContract {
            constraint_id: format!("xtalk_{}_{}", self.net_a, self.net_b),
            kind: "crosstalk_exclusion".into(),
            scope: vec![self.net_a.clone(), self.net_b.clone()],
            strength: self.strength(),
            priority: self.priority(),
            source: "crosstalk_extractor".into(),
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
