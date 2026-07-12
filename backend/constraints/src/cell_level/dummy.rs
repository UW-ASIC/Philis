use crate::types::{
    ConstraintContract, ConstraintStage, ConstraintStatus, ConstraintStrength, Contractable,
    DeviceConstraint, DeviceId, DummyType, MatchingTier,
};

/// Dummy device specification for LOD matching at cell edges.
#[derive(Debug, Clone)]
pub struct DummyConstraint {
    pub device_id: DeviceId,
    pub dummy_type: DummyType,
    /// Moat extension beyond active (um).
    pub moat_ext_um: f64,
    /// Minimum poly-to-dummy clearance (um).
    pub min_poly_clearance_um: f64,
}

impl DeviceConstraint for DummyConstraint {
    fn device_id(&self) -> DeviceId {
        self.device_id
    }
}

impl DummyConstraint {
    /// Contract strength depends on the device's matching tier (caller provides).
    pub fn to_contract_with_tier(
        &self,
        device_names: &[String],
        tier: MatchingTier,
    ) -> ConstraintContract {
        let strength = if tier >= MatchingTier::Moderate {
            ConstraintStrength::Hard
        } else {
            ConstraintStrength::Soft
        };
        let priority = match tier {
            MatchingTier::Exceptional => 85,
            MatchingTier::Moderate => 70,
            _ => 40,
        };
        let n = dn(device_names, self.device_id);
        ConstraintContract {
            constraint_id: format!("dummy_{n}"),
            kind: "dummy".into(),
            scope: vec![n.into()],
            strength,
            priority,
            source: "dummy_extractor".into(),
            source_confidence: 1.0,
            derived_from: Vec::new(),
            relaxation_policy: None,
            stage_consumption: vec![ConstraintStage::CellGen],
            status: ConstraintStatus::Emitted,
            violation_metric: None,
            violation_units: None,
            waiver_reason: None,
            status_history: Vec::new(),
        }
    }
}

impl Contractable for DummyConstraint {
    fn strength(&self) -> ConstraintStrength {
        ConstraintStrength::Soft
    }
    fn priority(&self) -> i32 {
        40
    }

    fn stages(&self) -> &[ConstraintStage] {
        &[ConstraintStage::CellGen]
    }

    fn to_contract(&self, device_names: &[String]) -> ConstraintContract {
        self.to_contract_with_tier(device_names, MatchingTier::None)
    }
}

fn dn<'a>(device_names: &'a [String], id: DeviceId) -> &'a str {
    device_names
        .get(id.0 as usize)
        .map_or("<unknown>", String::as_str)
}
