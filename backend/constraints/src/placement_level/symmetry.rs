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

// ── Ratioed current-mirror detection (item 1.8) ──

/// Relaxed device signature: (DeviceType, L, model) — same as `device_signature`
/// but drops W requirement. Two devices with same (type, L, model) but different W
/// form a ratioed mirror if `W_a / W_b` reduces to a small integer ratio.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RelaxedSignature {
    pub device_type: String,
    pub l: i32,
    pub model: String,
}

impl RelaxedSignature {
    pub fn from_fields(device_type: &str, l: i32, model: &str) -> Self {
        Self {
            device_type: device_type.into(),
            l,
            model: model.into(),
        }
    }
}

/// Compute the reduced integer ratio `(a, b)` from two widths.
/// Returns `None` if the ratio exceeds `max_ratio` or is not a clean integer
/// ratio within `tolerance` (fractional).
pub fn width_ratio(w_a: i32, w_b: i32, max_ratio: u16, tolerance: f64) -> Option<(u16, u16)> {
    if w_a <= 0 || w_b <= 0 {
        return None;
    }
    let g = gcd(w_a, w_b);
    let ra = (w_a / g) as u16;
    let rb = (w_b / g) as u16;
    if ra > max_ratio || rb > max_ratio {
        return None;
    }
    // Verify the ratio reconstructs cleanly
    let reconstructed_a = i32::from(ra) * g;
    let reconstructed_b = i32::from(rb) * g;
    let err_a = (reconstructed_a - w_a).abs() as f64 / w_a as f64;
    let err_b = (reconstructed_b - w_b).abs() as f64 / w_b as f64;
    if err_a > tolerance || err_b > tolerance {
        return None;
    }
    Some((ra, rb))
}

fn gcd(mut a: i32, mut b: i32) -> i32 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}
