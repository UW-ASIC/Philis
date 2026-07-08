use crate::types::{
    DummyType, MatchingTier, MatchingType, Orientation, ShieldType,
};

/// Route matching tolerances for matched net pairs.
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

/// Complete matching specification for an AOAL-grade symmetry group.
///
/// Aggregates all tier-dependent budgets into a single document.
#[derive(Debug, Clone)]
pub struct MatchingSpec {
    pub group_id: String,
    pub tier: MatchingTier,
    pub match_kind: MatchingType,
    pub random_budget_dvth_mv: f64,
    pub systematic_budget_dvth_mv: f64,
    pub max_wpe_delta_mv: f64,
    pub max_lod_delta_pct: f64,
    pub max_thermal_delta_c: f64,
    pub max_stress_grade: String,
    pub orientation_rule: String,
    pub dummy_style: DummyType,
    pub allowed_transforms: Vec<Orientation>,
    pub shield_policy: ShieldType,
    pub metal_over_gate_policy: String,
    pub route_tolerance: RouteMatchingTolerance,
}
