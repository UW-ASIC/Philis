use crate::types::{
    ConstraintContract, ConstraintStage, ConstraintStatus, ConstraintStrength, Contractable,
};

/// Route matching tolerances for matched net pairs (symmetry-derived).
#[derive(Debug, Clone)]
pub struct RouteMatchingTolerance {
    pub max_length_delta_pct: f64,
    pub max_r_delta_pct: f64,
    pub max_c_delta_pct: f64,
    pub max_coupling_delta_pct: f64,
    pub same_layer_required: bool,
    pub same_via_count_required: bool,
}

impl Default for RouteMatchingTolerance {
    fn default() -> Self {
        Self {
            max_length_delta_pct: 5.0,
            max_r_delta_pct: 5.0,
            max_c_delta_pct: 5.0,
            max_coupling_delta_pct: 10.0,
            same_layer_required: true,
            same_via_count_required: true,
        }
    }
}

/// Differential routing constraint: two nets that must be routed with
/// matched length, resistance, and capacitance.
#[derive(Debug, Clone)]
pub struct DifferentialPair {
    pub net_pos: String,
    pub net_neg: String,
    pub max_length_delta_pct: f64,
    pub max_r_delta_pct: f64,
    pub max_c_delta_pct: f64,
    pub same_layer_required: bool,
}

impl Default for DifferentialPair {
    fn default() -> Self {
        Self {
            net_pos: String::new(),
            net_neg: String::new(),
            max_length_delta_pct: 5.0,
            max_r_delta_pct: 5.0,
            max_c_delta_pct: 5.0,
            same_layer_required: true,
        }
    }
}

impl Contractable for DifferentialPair {
    fn strength(&self) -> ConstraintStrength {
        ConstraintStrength::Hard
    }
    fn priority(&self) -> i32 {
        80
    }

    fn stages(&self) -> &[ConstraintStage] {
        &[ConstraintStage::Routing]
    }

    fn to_contract(&self, _device_names: &[String]) -> ConstraintContract {
        ConstraintContract {
            constraint_id: format!("diff_{}_{}", self.net_pos, self.net_neg),
            kind: "differential_pair".into(),
            scope: vec![self.net_pos.clone(), self.net_neg.clone()],
            strength: self.strength(),
            priority: self.priority(),
            source: "differential_extractor".into(),
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
