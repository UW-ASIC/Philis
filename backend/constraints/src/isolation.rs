use crate::types::{
    Contractable, ConstraintContract, ConstraintStage, ConstraintStrength, ConstraintStatus,
    DeviceId, PairConstraint,
};

/// Isolation constraint between a noisy and sensitive device.
#[derive(Debug, Clone)]
pub struct IsolationConstraint {
    /// Noisy device.
    pub device_a: DeviceId,
    /// Sensitive device.
    pub device_b: DeviceId,
    /// Minimum separation (um).
    pub min_distance_um: f64,
    pub requires_guard_ring: bool,
    pub reason: String,
}

impl PairConstraint for IsolationConstraint {
    fn device_a(&self) -> DeviceId { self.device_a }
    fn device_b(&self) -> DeviceId { self.device_b }
    fn distance_budget_um(&self) -> f64 { self.min_distance_um }
}

impl Contractable for IsolationConstraint {
    fn strength(&self) -> ConstraintStrength { ConstraintStrength::Hard }
    fn priority(&self) -> i32 { 85 }

    fn stages(&self) -> &[ConstraintStage] {
        &[ConstraintStage::Placement]
    }

    fn to_contract(&self, device_names: &[String]) -> ConstraintContract {
        let na = dn(device_names, self.device_a);
        let nb = dn(device_names, self.device_b);
        ConstraintContract {
            constraint_id: format!("iso_{na}_{nb}"),
            kind: "isolation".into(),
            scope: vec![na.into(), nb.into()],
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
