use crate::types::{
    ConstraintContract, ConstraintStage, ConstraintStatus, ConstraintStrength, Contractable,
    DeviceConstraint, DeviceId, GuardRingType,
};

/// Guard ring requirement for latchup / noise isolation.
#[derive(Debug, Clone)]
pub struct GuardRingRequirement {
    pub device_id: DeviceId,
    pub ring_type: GuardRingType,
    /// Whether this ring may be shared with nearby devices that request the
    /// same ring type and connection net. Pad injectors and explicitly
    /// isolated devices must set this to false.
    pub shareable: bool,
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
    fn device_id(&self) -> DeviceId {
        self.device_id
    }
}

impl Contractable for GuardRingRequirement {
    fn strength(&self) -> ConstraintStrength {
        ConstraintStrength::Hard
    }
    fn priority(&self) -> i32 {
        80
    }

    fn stages(&self) -> &[ConstraintStage] {
        // The ring outline depends on the placed device cluster. Its terminal
        // is then consumed by routing and the completed enclosure is judged at
        // signoff; it is deliberately not part of the device-cell footprint.
        &[ConstraintStage::Routing, ConstraintStage::Signoff]
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
