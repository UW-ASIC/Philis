use crate::types::{
    ConstraintContract, ConstraintStage, ConstraintStatus, ConstraintStrength, Contractable,
    DeviceId, PairConstraint,
};

/// Layout-Dependent Effect bounds for a matched device pair.
///
/// WPE (Well Proximity Effect) and LOD (Length of Diffusion) mismatch
/// budgets derived from PDK parameters and matching tier.
#[derive(Debug, Clone)]
pub struct LdeBound {
    pub pair: (DeviceId, DeviceId),
    /// Minimum distance to well edge for WPE control (um).
    pub min_well_edge_distance_um: f64,
    /// Maximum WPE-induced Vth mismatch (mV).
    pub max_wpe_dvth_mv: f64,
    /// Maximum LOD-induced current mismatch (%).
    pub max_lod_did_pct: f64,
    /// Maximum SA/SB difference for LOD matching (um).
    pub max_sa_sb_mismatch_um: f64,
    /// SA distance (source-side OD extension) for outer fingers (um).
    pub outer_sa_um: f64,
    /// SB distance (drain-side OD extension) for outer fingers (um).
    pub outer_sb_um: f64,
    /// SA for inner fingers (half poly-to-poly spacing) (um).
    pub inner_sa_um: f64,
    /// SB for inner fingers (half poly-to-poly spacing) (um).
    pub inner_sb_um: f64,
    /// Full STI dVth from `k1 * (1/SA_eff - 1/SB_eff)` model (mV).
    pub sti_dvth_mv: f64,
    /// Enforce identical OD width (passives).
    pub same_width: bool,
    /// Enforce identical orientation (passives).
    pub same_orientation: bool,
}

impl PairConstraint for LdeBound {
    fn device_a(&self) -> DeviceId {
        self.pair.0
    }
    fn device_b(&self) -> DeviceId {
        self.pair.1
    }
    fn distance_budget_um(&self) -> f64 {
        self.min_well_edge_distance_um
    }
}

impl Contractable for LdeBound {
    fn strength(&self) -> ConstraintStrength {
        ConstraintStrength::Hard
    }
    fn priority(&self) -> i32 {
        85
    }

    fn stages(&self) -> &[ConstraintStage] {
        &[ConstraintStage::Placement]
    }

    fn to_contract(&self, device_names: &[String]) -> ConstraintContract {
        let na = dn(device_names, self.pair.0);
        let nb = dn(device_names, self.pair.1);
        ConstraintContract {
            constraint_id: format!("lde_{na}_{nb}"),
            kind: "lde_bound".into(),
            scope: vec![na.into(), nb.into()],
            strength: self.strength(),
            priority: self.priority(),
            source: "lde_extractor".into(),
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
