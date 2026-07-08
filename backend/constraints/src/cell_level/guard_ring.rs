use crate::types::{
    Contractable, ConstraintContract, ConstraintStage, ConstraintStrength, ConstraintStatus,
    DeviceConstraint, DeviceId, GuardRingType,
};

/// Guard ring requirement for latchup / noise isolation.
#[derive(Debug, Clone)]
pub struct GuardRingRequirement {
    pub device_id: DeviceId,
    pub ring_type: GuardRingType,
    /// Tap contact pitch (um).
    pub tap_pitch_um: f64,
    /// Minimum ring metal width (um).
    pub min_width_um: f64,
    /// Maximum ring resistance (ohms).
    pub max_ring_resistance_ohm: f64,
    pub enclosure_complete: bool,
    /// Net the ring connects to (e.g. "VSS").
    pub connection_net: String,
}

impl DeviceConstraint for GuardRingRequirement {
    fn device_id(&self) -> DeviceId { self.device_id }
}

impl Contractable for GuardRingRequirement {
    fn strength(&self) -> ConstraintStrength { ConstraintStrength::Hard }
    fn priority(&self) -> i32 { 80 }

    fn stages(&self) -> &[ConstraintStage] {
        &[ConstraintStage::CellGen, ConstraintStage::Placement]
    }

    fn to_contract(&self, device_names: &[String]) -> ConstraintContract {
        let n = dn(device_names, self.device_id);
        ConstraintContract {
            constraint_id: format!("guard_ring_{n}"),
            kind: "guard_ring".into(),
            scope: vec![n.into()],
            strength: self.strength(),
            priority: self.priority(),
            source: "isolation_extractor".into(),
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
