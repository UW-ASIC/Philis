use crate::types::{
    Contractable, ConstraintContract, ConstraintStage, ConstraintStrength, ConstraintStatus,
    AgingMechanism, DeviceConstraint, DeviceId,
};

/// Aging/reliability constraint for a device.
///
/// Covers HCI, NBTI, TDDB, and GOI.
#[derive(Debug, Clone)]
pub struct AgingConstraint {
    pub device_id: DeviceId,
    pub mechanism: AgingMechanism,
    /// "warning" or "violation".
    pub severity: String,
    pub description: String,
}

impl DeviceConstraint for AgingConstraint {
    fn device_id(&self) -> DeviceId { self.device_id }
}

impl Contractable for AgingConstraint {
    fn strength(&self) -> ConstraintStrength {
        if self.severity == "violation" {
            ConstraintStrength::Hard
        } else {
            ConstraintStrength::Soft
        }
    }

    fn priority(&self) -> i32 {
        if self.severity == "violation" { 70 } else { 30 }
    }

    fn stages(&self) -> &[ConstraintStage] {
        &[ConstraintStage::Signoff]
    }

    fn to_contract(&self, device_names: &[String]) -> ConstraintContract {
        let n = dn(device_names, self.device_id);
        ConstraintContract {
            constraint_id: format!("aging_{n}_{:?}", self.mechanism),
            kind: "aging".into(),
            scope: vec![n.into()],
            strength: self.strength(),
            priority: self.priority(),
            source: "aging_extractor".into(),
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
