use crate::types::{
    Contractable, ConstraintContract, ConstraintStage, ConstraintStrength, ConstraintStatus,
    DeviceConstraint, DeviceId,
};

/// Die-position stress constraint limiting centroid distance from die center.
#[derive(Debug, Clone)]
pub struct StressConstraint {
    pub device_id: DeviceId,
    /// Maximum allowed centroid distance from die center (um).
    pub max_centroid_distance_um: f64,
}

impl DeviceConstraint for StressConstraint {
    fn device_id(&self) -> DeviceId { self.device_id }
}

impl Contractable for StressConstraint {
    fn strength(&self) -> ConstraintStrength { ConstraintStrength::Soft }
    fn priority(&self) -> i32 { 50 }

    fn stages(&self) -> &[ConstraintStage] {
        &[ConstraintStage::Placement]
    }

    fn to_contract(&self, device_names: &[String]) -> ConstraintContract {
        let n = dn(device_names, self.device_id);
        ConstraintContract {
            constraint_id: format!("stress_{n}"),
            kind: "stress".into(),
            scope: vec![n.into()],
            strength: self.strength(),
            priority: self.priority(),
            source: "stress_extractor".into(),
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

fn dn<'a>(device_names: &'a [String], id: DeviceId) -> &'a str {
    device_names
        .get(id.0 as usize)
        .map_or("<unknown>", String::as_str)
}
