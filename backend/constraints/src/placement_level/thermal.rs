use crate::types::{
    ConstraintContract, ConstraintStage, ConstraintStatus, ConstraintStrength, Contractable,
    DeviceId, PairConstraint,
};

/// Pairwise thermal gradient constraint between matched-pair devices.
///
/// Bounds `|delta_T_A - delta_T_B|` from all power sources.
#[derive(Debug, Clone)]
pub struct ThermalGradientConstraint {
    pub device_a: DeviceId,
    pub device_b: DeviceId,
    /// Maximum allowable temperature delta (Celsius).
    pub max_delta_c: f64,
    /// Estimated gradient at constraint-extraction time (Celsius).
    pub estimated_gradient_c: f64,
}

impl PairConstraint for ThermalGradientConstraint {
    fn device_a(&self) -> DeviceId {
        self.device_a
    }
    fn device_b(&self) -> DeviceId {
        self.device_b
    }
    fn distance_budget_um(&self) -> f64 {
        0.0
    }
}

impl Contractable for ThermalGradientConstraint {
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
        let na = dn(device_names, self.device_a);
        let nb = dn(device_names, self.device_b);
        ConstraintContract {
            constraint_id: format!("tgrad_{na}_{nb}"),
            kind: "thermal_gradient".into(),
            scope: vec![na.into(), nb.into()],
            strength: self.strength(),
            priority: self.priority(),
            source: "thermal_extractor".into(),
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
