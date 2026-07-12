use crate::types::{
    ConstraintContract, ConstraintStage, ConstraintStatus, ConstraintStrength, Contractable,
    NetConstraint,
};

/// Net that must route as one straight segment.
///
/// Placement aligns pins along the perpendicular axis so the
/// router can emit a single segment.
#[derive(Debug, Clone)]
pub struct StraightNet {
    pub net: String,
    /// `true`: pins share x (vertical route); `false`: pins share y.
    pub vertical: bool,
}

impl NetConstraint for StraightNet {
    fn net_name(&self) -> &str {
        &self.net
    }
}

impl Contractable for StraightNet {
    fn strength(&self) -> ConstraintStrength {
        ConstraintStrength::Soft
    }
    fn priority(&self) -> i32 {
        50
    }

    fn stages(&self) -> &[ConstraintStage] {
        &[ConstraintStage::Routing]
    }

    fn to_contract(&self, _device_names: &[String]) -> ConstraintContract {
        ConstraintContract {
            constraint_id: format!("straight_{}", self.net),
            kind: "straight_net".into(),
            scope: vec![self.net.clone()],
            strength: self.strength(),
            priority: self.priority(),
            source: "align_extractor".into(),
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
