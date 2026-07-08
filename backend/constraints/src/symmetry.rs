use crate::types::{
    Contractable, ConstraintContract, ConstraintStage, ConstraintStrength, ConstraintStatus,
    DeviceId, GroupConstraint, MatchingTier, MatchingType, PairConstraint,
};

/// A pair of matched devices within a symmetry group.
#[derive(Debug, Clone)]
pub struct MatchingPair {
    pub device_a: DeviceId,
    pub device_b: DeviceId,
    pub matching_type: MatchingType,
    pub tier: MatchingTier,
    /// Maximum threshold voltage mismatch budget (mV).
    pub max_dvth_mv: f64,
    /// Maximum drain current mismatch budget (%).
    pub max_did_pct: f64,
    /// Reduced width ratio `(a, b)` for ratioed mirrors/BJT area.
    pub w_ratio: Option<(u16, u16)>,
}

impl PairConstraint for MatchingPair {
    fn device_a(&self) -> DeviceId { self.device_a }
    fn device_b(&self) -> DeviceId { self.device_b }
    fn distance_budget_um(&self) -> f64 { 0.0 }
}

impl Contractable for MatchingPair {
    fn strength(&self) -> ConstraintStrength {
        if self.tier >= MatchingTier::Moderate {
            ConstraintStrength::Hard
        } else {
            ConstraintStrength::Soft
        }
    }

    fn priority(&self) -> i32 {
        match self.tier {
            MatchingTier::Exceptional => 95,
            MatchingTier::Moderate => 80,
            MatchingTier::Minimal => 50,
            MatchingTier::None => 30,
        }
    }

    fn stages(&self) -> &[ConstraintStage] {
        &[ConstraintStage::Placement, ConstraintStage::Routing]
    }

    fn to_contract(&self, device_names: &[String]) -> ConstraintContract {
        let na = dn(device_names, self.device_a);
        let nb = dn(device_names, self.device_b);
        ConstraintContract {
            constraint_id: format!("match_{na}_{nb}"),
            kind: "matching_pair".into(),
            scope: vec![na.into(), nb.into()],
            strength: self.strength(),
            priority: self.priority(),
            source: "symmetry_extractor".into(),
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

/// A group of devices placed symmetrically about an axis.
#[derive(Debug, Clone, Default)]
pub struct SymmetryGroup {
    pub group_id: String,
    /// Symmetry axis position (angstroms), if known.
    pub axis: Option<i32>,
    pub pairs: Vec<MatchingPair>,
    /// Devices self-symmetric about the group axis (e.g. tail sources).
    pub self_symmetric: Vec<DeviceId>,
}

impl SymmetryGroup {
    /// Highest matching tier across all pairs.
    #[must_use]
    pub fn max_tier(&self) -> MatchingTier {
        self.pairs
            .iter()
            .map(|mp| mp.tier)
            .max()
            .unwrap_or(MatchingTier::None)
    }

    /// All unique devices referenced by this group.
    #[must_use]
    pub fn all_devices(&self) -> Vec<DeviceId> {
        let mut devs: Vec<DeviceId> = self
            .pairs
            .iter()
            .flat_map(|mp| [mp.device_a, mp.device_b])
            .chain(self.self_symmetric.iter().copied())
            .collect();
        devs.sort_unstable();
        devs.dedup();
        devs
    }
}

impl GroupConstraint for SymmetryGroup {
    fn devices(&self) -> &[DeviceId] {
        &self.self_symmetric
    }
}

fn dn<'a>(device_names: &'a [String], id: DeviceId) -> &'a str {
    device_names
        .get(id.0 as usize)
        .map_or("<unknown>", String::as_str)
}
