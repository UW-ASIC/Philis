use crate::types::{
    ConstraintContract, ConstraintStage, ConstraintStatus, ConstraintStrength, Contractable,
    DeviceId, EnvironmentalKind, ThresholdUnit,
};

/// Environmental (layout-dependent) constraint.
///
/// Covers WPE four-edge, LOD SA/SB, dummy moat extension,
/// hydrogenation keepout, metal-over-gate, thermal exclusion.
#[derive(Debug, Clone)]
pub struct EnvironmentalConstraint {
    pub constraint_id: String,
    pub kind: EnvironmentalKind,
    /// Devices in scope.
    pub scope: Vec<DeviceId>,
    pub strength: ConstraintStrength,
    pub threshold: f64,
    pub units: ThresholdUnit,
}

impl Contractable for EnvironmentalConstraint {
    fn strength(&self) -> ConstraintStrength {
        self.strength
    }
    fn priority(&self) -> i32 {
        65
    }

    fn stages(&self) -> &[ConstraintStage] {
        &[ConstraintStage::Placement]
    }

    fn to_contract(&self, device_names: &[String]) -> ConstraintContract {
        let scope: Vec<String> = self
            .scope
            .iter()
            .map(|id| dn(device_names, *id).into())
            .collect();
        ConstraintContract {
            constraint_id: self.constraint_id.clone(),
            kind: format!("environmental_{:?}", self.kind),
            scope,
            strength: self.strength,
            priority: self.priority(),
            source: "environment_extractor".into(),
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
