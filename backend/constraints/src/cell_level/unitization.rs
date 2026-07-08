use std::collections::HashMap;

use crate::types::{
    Contractable, ConstraintContract, ConstraintStage, ConstraintStrength, ConstraintStatus,
    DeviceId, DeviceType, GroupConstraint, PatternType,
};

/// Unitization constraint for decomposing devices into unit elements.
///
/// Drives interdigitation pattern and ratio-matching in cell generation.
#[derive(Debug, Clone)]
pub struct UnitizationConstraint {
    pub group_id: String,
    pub device_type: DeviceType,
    /// Unit element geometry (e.g. {"w": 5000, "l": 1500} in angstroms).
    pub unit_geometry: HashMap<String, f64>,
    /// Number of unit elements per device instance.
    pub instance_unit_counts: HashMap<String, u32>,
    /// Target ratio between device instances.
    pub target_ratio: HashMap<String, u32>,
    /// "parallel", "series", or "repeated_stage".
    pub series_parallel_allowed: String,
    pub required_pattern: PatternType,
    pub same_variant_required: bool,
    pub dummy_required: bool,
    pub route_matching_required: bool,
    /// Device IDs in this unitization group (stored for GroupConstraint).
    pub devices: Vec<DeviceId>,
}

impl GroupConstraint for UnitizationConstraint {
    fn devices(&self) -> &[DeviceId] { &self.devices }
}

impl Contractable for UnitizationConstraint {
    fn strength(&self) -> ConstraintStrength { ConstraintStrength::Hard }
    fn priority(&self) -> i32 { 80 }

    fn stages(&self) -> &[ConstraintStage] {
        &[ConstraintStage::CellGen]
    }

    fn to_contract(&self, _device_names: &[String]) -> ConstraintContract {
        ConstraintContract {
            constraint_id: format!("unit_{}", self.group_id),
            kind: "unitization".into(),
            scope: vec![self.group_id.clone()],
            strength: self.strength(),
            priority: self.priority(),
            source: "unitization_extractor".into(),
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
