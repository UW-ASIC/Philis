use crate::types::{
    ConstraintContract, ConstraintStage, ConstraintStatus, ConstraintStrength, Contractable,
    DeviceId, PairConstraint,
};

/// DTI (Deep Trench Isolation) pair constraint.
///
/// The pair must either abut (gap < `s_max`, sharing one trench) or sit
/// fully apart (gap > `d_dti`); the band between is forbidden.
#[derive(Debug, Clone)]
pub struct DtiPair {
    pub device_a: DeviceId,
    pub device_b: DeviceId,
    pub s_max: f64,
    pub d_dti: f64,
}

impl PairConstraint for DtiPair {
    fn device_a(&self) -> DeviceId {
        self.device_a
    }
    fn device_b(&self) -> DeviceId {
        self.device_b
    }
    fn distance_budget_um(&self) -> f64 {
        self.d_dti
    }
}

impl Contractable for DtiPair {
    fn strength(&self) -> ConstraintStrength {
        ConstraintStrength::Hard
    }
    fn priority(&self) -> i32 {
        75
    }

    fn stages(&self) -> &[ConstraintStage] {
        &[ConstraintStage::Placement]
    }

    fn to_contract(&self, device_names: &[String]) -> ConstraintContract {
        let dn = |id: DeviceId| -> &str {
            device_names
                .get(id.0 as usize)
                .map_or("<unknown>", String::as_str)
        };
        let (na, nb) = (dn(self.device_a), dn(self.device_b));
        ConstraintContract {
            constraint_id: format!("dti_{na}_{nb}"),
            kind: "dti".into(),
            scope: vec![na.into(), nb.into()],
            strength: self.strength(),
            priority: self.priority(),
            source: "dti_extractor".into(),
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
