use crate::types::{
    Contractable, ConstraintContract, ConstraintStage, ConstraintStrength, ConstraintStatus,
    DeviceId, GroupConstraint, PatternType,
};

/// Common-centroid pattern recommendation for a matched pair.
#[derive(Debug, Clone)]
pub struct CcGroup {
    /// Devices in the A-side of the pattern.
    pub group_a: Vec<DeviceId>,
    /// Devices in the B-side of the pattern.
    pub group_b: Vec<DeviceId>,
    pub pattern: PatternType,
}

impl GroupConstraint for CcGroup {
    fn devices(&self) -> &[DeviceId] {
        // ponytail: returns group_a only; callers needing both use group_a + group_b directly
        &self.group_a
    }
}

impl CcGroup {
    /// All devices across both sides.
    #[must_use]
    pub fn all_devices(&self) -> Vec<DeviceId> {
        self.group_a
            .iter()
            .chain(self.group_b.iter())
            .copied()
            .collect()
    }
}

impl Contractable for CcGroup {
    fn strength(&self) -> ConstraintStrength { ConstraintStrength::Soft }
    fn priority(&self) -> i32 { 60 }

    fn stages(&self) -> &[ConstraintStage] {
        &[ConstraintStage::CellGen, ConstraintStage::Placement]
    }

    fn to_contract(&self, device_names: &[String]) -> ConstraintContract {
        let scope: Vec<String> = self
            .all_devices()
            .iter()
            .map(|id| dn(device_names, *id).into())
            .collect();
        ConstraintContract {
            constraint_id: format!("cc_{}", scope.join("_")),
            kind: "common_centroid".into(),
            scope,
            strength: self.strength(),
            priority: self.priority(),
            source: "cc_extractor".into(),
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
