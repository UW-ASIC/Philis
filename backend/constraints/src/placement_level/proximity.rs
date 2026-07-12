use crate::types::{
    ConstraintContract, ConstraintStage, ConstraintStatus, ConstraintStrength, Contractable,
    DeviceId, PairConstraint,
};

/// Proximity rule between two devices (minimum or maximum spacing).
#[derive(Debug, Clone)]
pub struct ProximityRule {
    pub device_a: DeviceId,
    pub device_b: DeviceId,
    /// Minimum distance in um. 0.0 = "as close as possible".
    pub min_distance_um: f64,
}

impl PairConstraint for ProximityRule {
    fn device_a(&self) -> DeviceId {
        self.device_a
    }
    fn device_b(&self) -> DeviceId {
        self.device_b
    }
    fn distance_budget_um(&self) -> f64 {
        self.min_distance_um
    }
}

impl Contractable for ProximityRule {
    fn strength(&self) -> ConstraintStrength {
        ConstraintStrength::Soft
    }
    fn priority(&self) -> i32 {
        40
    }

    fn stages(&self) -> &[ConstraintStage] {
        &[ConstraintStage::Placement]
    }

    fn to_contract(&self, device_names: &[String]) -> ConstraintContract {
        let na = dn(device_names, self.device_a);
        let nb = dn(device_names, self.device_b);
        ConstraintContract {
            constraint_id: format!("prox_{na}_{nb}"),
            kind: "proximity_rule".into(),
            scope: vec![na.into(), nb.into()],
            strength: self.strength(),
            priority: self.priority(),
            source: "proximity_extractor".into(),
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
