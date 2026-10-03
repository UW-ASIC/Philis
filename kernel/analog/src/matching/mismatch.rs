//! One matched pair's mismatch ledger: the random σ the sizing bought, the
//! systematic terms placement spends (gradient, thermal, LOD) and the single
//! allowance they share (plan-02 MAT-04). Pure math, mV throughout.

/// Pelgrom pair σ of two devices of unequal gate area, mV:
/// `A·√((1/a1 + 1/a2)/2)`, `A` = pair constant mV·µm, `a` µm² (LDM eq. 4,
/// PDF p.3; Hastings eq. 13.49, PDF p.688). `0` when an area is not positive.
#[must_use]
pub fn sigma_pair(a_pair: f32, a1_um2: f32, a2_um2: f32) -> f32 {
    if a1_um2 <= 0.0 || a2_um2 <= 0.0 {
        return 0.0;
    }
    a_pair * ((1.0 / a1_um2 + 1.0 / a2_um2) / 2.0).sqrt()
}

/// Voltage matching (a diff pair: V_GS at equal I) or current matching (a
/// mirror/load: I at equal V_GS), Hastings §13.2 eqs 13.40–13.41; `Ratio` for
/// passives (MAT-10).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MatchKind {
    Voltage,
    Current,
    Ratio,
}

/// How much systematic mismatch a pair may spend.
#[derive(Clone, Copy, Debug)]
pub enum Budget {
    /// `η·σ_rand` (the default, η = 0.3).
    Eta(f32),
    /// Total 1σ budget `b`, mV: the layout gets `√max(0, b² − σ_rand²)`.
    Sigma1Mv(f32),
    /// The 1σ systematic allowance itself, mV.
    Allowance(f32),
}

impl Budget {
    /// The systematic allowance, mV, given the pair's random σ.
    #[must_use]
    pub fn allowance(self, sigma_rand: f32) -> f32 {
        match self {
            Budget::Eta(eta) => eta * sigma_rand,
            Budget::Sigma1Mv(b) => (b * b - sigma_rand * sigma_rand).max(0.0).sqrt(),
            Budget::Allowance(a) => a,
        }
    }
}

/// Process numbers of one set (deck, per polarity); `None` = not given, that
/// term is unknown.
#[derive(Clone, Copy, Debug, Default)]
pub struct Coeffs {
    /// `A_VT`, mV·µm.
    pub avt_mv_um: Option<f32>,
    /// `S_VT`, µV/µm.
    pub svt_uv_per_um: Option<f32>,
    /// BSIM4 `KVTH0`, mV·µm.
    pub kvth0_mv_um: Option<f32>,
    /// `|dV_T/dT|`, µV/K.
    pub tc_uv_per_k: Option<f32>,
}

/// One pair's ledger, mV unless named otherwise.
///
/// - `sigma_rand` = `A_VT·√((1/a₁+1/a₂)/2)` ([`sigma_pair`]).
/// - `sigma_grad` = `S_VT·|Δm|` (Pelgrom eq. (1) distance term, PDF p.1).
/// - `mu_thermal` = `TC·|ΔT|` at the two centroids (Hastings eq. 8.23, PDF p.388).
/// - `mu_lod` = `KVTH0·|⟨lod⟩_a − ⟨lod⟩_b|`, unit-weighted means (REV eq. 11).
/// - `coincidence` = `|Δm|/tol` when both members are drawn interleaved in one
///   cell (Hastings Table 8.4 rule 1, PDF p.392): process-free, binds at 1.
/// - `second_order_nm` = `‖M_a − M_b‖_F / L`: the second-moment residue as an
///   equivalent centroid offset (cost and report only; no deck has a coefficient).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ledger {
    pub sigma_rand: f32,
    pub sigma_grad: f32,
    pub mu_thermal: f32,
    pub mu_lod: f32,
    pub allowance: f32,
    pub coincidence: Option<f32>,
    pub second_order_nm: f32,
    pub delta_m_nm: f32,
    pub known: bool,
}

impl Ledger {
    /// Systematic terms spent: deterministic ones add by magnitude (worst
    /// case, Lampaert eq. 4.25), one budget for all (DVP eq. 1).
    #[must_use]
    pub fn spent(&self) -> f32 {
        self.mu_thermal + self.mu_lod + self.sigma_grad
    }

    /// Fraction of the tighter check used; finite at a zero allowance.
    #[must_use]
    pub fn usage(&self) -> f32 {
        (self.spent() / self.allowance.max(f32::EPSILON)).max(self.coincidence.unwrap_or(0.0))
    }

    /// Overshoot past the allowance (as a fraction of it) or past coincidence;
    /// a zero allowance reads 1 as soon as anything is spent ([`crate::rule::over`]).
    #[must_use]
    pub fn residual(&self) -> f32 {
        crate::rule::over(self.spent() - self.allowance, self.allowance)
            .max(self.coincidence.map_or(0.0, |c| c - 1.0))
            .max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unequal_areas_use_the_per_device_variance() {
        assert!((sigma_pair(10.0, 4.0, 16.0) - 3.953).abs() < 1e-3);
    }

    #[test]
    fn one_allowance_is_spent_once() {
        let l = Ledger { mu_thermal: 0.4, mu_lod: 0.4, allowance: 0.637, ..Ledger::default() };
        assert!((l.usage() - 1.256).abs() < 1e-3, "{}", l.usage());
        assert!((l.residual() - 0.256).abs() < 1e-3, "{}", l.residual());
    }

    #[test]
    fn sigma1_budget_leaves_the_quadrature_share() {
        assert!((Budget::Sigma1Mv(3.0 * 1.09f32.sqrt()).allowance(3.0) - 0.9).abs() < 1e-3);
        assert_eq!(Budget::Sigma1Mv(2.0).allowance(3.0), 0.0);
    }

    #[test]
    fn zero_allowance_is_a_violation_not_nan() {
        let l = Ledger { mu_lod: 0.01, ..Ledger::default() };
        assert_eq!(l.residual(), 1.0);
        assert!(l.usage().is_finite());
        assert_eq!(Ledger::default().residual(), 0.0);
        assert_eq!(Budget::Sigma1Mv(1.0).allowance(2.124), 0.0);
    }
}
