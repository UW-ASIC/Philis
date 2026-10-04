//! One matched pair's mismatch ledger: the random σ the sizing bought, the
//! systematic terms placement spends (gradient, thermal, LOD) and the single
//! allowance they share (plan-02 MAT-04). Pure math, mV (or % of a
//! mirror current, MAT-09).

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

use crate::matching::class::ClassLimit;

/// What a ledger's σ and allowance are in: input-referred mV, or a current
/// ratio in % (a mirror whose G = g_m/I is known, MAT-09).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LedgerUnit {
    #[default]
    Mv,
    Pct,
}

impl LedgerUnit {
    /// `"mV"` | `"%"`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            LedgerUnit::Mv => "mV",
            LedgerUnit::Pct => "%",
        }
    }
}

/// Current-domain random σ, %: √((0.1·G·σ_VT)² + σ_β²) (Hastings eq. 13.43; 1/V·mV ×100 = 0.1).
#[must_use]
pub fn sigma_current_pct(sigma_vt_mv: f32, g_per_v: f32, sigma_beta_pct: f32) -> f32 {
    (0.1 * g_per_v * sigma_vt_mv).hypot(sigma_beta_pct)
}

/// Voltage-domain random σ, mV: √(σ_VT² + (10·σ_β/G)²) (eq. 13.42).
#[must_use]
pub fn sigma_voltage_mv(sigma_vt_mv: f32, g_per_v: f32, sigma_beta_pct: f32) -> f32 {
    sigma_vt_mv.hypot(10.0 * sigma_beta_pct / g_per_v)
}

/// Placement's share η of a matched pair's mismatch when no offset budget is
/// given: the gradient term may reach this fraction of the random term the
/// sizing bought (σ grows ≤ 4.4%).
///
/// ponytail: Pelgrom prescribes no η; it is the circuit's to allocate. Set
/// `AnnotationConfig::offset_sigma_mv` to derive it.
pub const GRADIENT_SHARE: f32 = 0.3;

/// How much systematic mismatch a pair may spend.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Budget {
    /// `η·σ_rand` (the default, η = 0.3).
    Eta(f32),
    /// Total 1σ budget `b`, mV: the layout gets `√max(0, b² − σ_rand²)`.
    Sigma1Mv(f32),
    /// The 1σ systematic allowance itself, mV.
    Allowance(f32),
    /// Total 1σ budget `b`, % of a current ratio: [`Budget::to_pct`] of a
    /// `Sigma1Mv` on a % ledger.
    Sigma1Pct(f32),
}

impl Budget {
    /// A class limit as a budget: a voltage limit in mV is 6σ (Hastings
    /// §13.3, hastings.txt L42333–42350), so the 1σ total is a sixth of it.
    /// A % limit (mirror, R/C ratio) is 6σ too: `Sigma1Pct(v/6)`, which only a
    /// % ledger spends (an mV ledger reads it as `Eta(GRADIENT_SHARE)`,
    /// `MatchedSet::budget_in`). An mV limit on a non-voltage pair:
    /// `Eta(GRADIENT_SHARE)`.
    #[must_use]
    pub fn from_class(limit: ClassLimit, kind: MatchKind) -> Budget {
        match (limit, kind) {
            (ClassLimit::Mv(v), MatchKind::Voltage) => Budget::Sigma1Mv(v / 6.0),
            (ClassLimit::Pct(v), _) => Budget::Sigma1Pct(v / 6.0),
            _ => Budget::Eta(GRADIENT_SHARE),
        }
    }

    /// The systematic allowance, mV, given the pair's random σ.
    #[must_use]
    pub fn allowance(self, sigma_rand: f32) -> f32 {
        match self {
            Budget::Eta(eta) => eta * sigma_rand,
            Budget::Sigma1Mv(b) | Budget::Sigma1Pct(b) => (b * b - sigma_rand * sigma_rand).max(0.0).sqrt(),
            Budget::Allowance(a) => a,
        }
    }

    /// An mV budget on a % ledger of transconductance efficiency `g_per_v`
    /// (1/V): ΔI/I = G·ΔV, ×0.1 for mV → %. `Eta` and `Sigma1Pct` unchanged.
    #[must_use]
    pub fn to_pct(self, g_per_v: f32) -> Budget {
        match self {
            Budget::Sigma1Mv(b) => Budget::Sigma1Pct(0.1 * g_per_v * b),
            Budget::Allowance(a) => Budget::Allowance(0.1 * g_per_v * a),
            other => other,
        }
    }
}

/// The pair budget, first source given wins: the circuit's 1σ offset
/// (`Sigma1Mv`), an explicit allowance, the class limit, else
/// `Eta(GRADIENT_SHARE)`. The caller passes `class_limit` only when the class
/// came from the user or a spec: a role-default Moderate (3 mV → 0.5 mV 1σ)
/// is under sky130's 2.124 mV σ_rand of a 20 µm² pair and would fail every
/// Moderate pair.
#[must_use]
pub fn choose(offset_sigma_mv: Option<f32>, allowance: Option<f32>, class_limit: Option<ClassLimit>, kind: MatchKind) -> Budget {
    match (offset_sigma_mv, allowance, class_limit) {
        (Some(b), _, _) => Budget::Sigma1Mv(b),
        (None, Some(a), _) => Budget::Allowance(a),
        (None, None, Some(c)) => Budget::from_class(c, kind),
        (None, None, None) => Budget::Eta(GRADIENT_SHARE),
    }
}

/// Process numbers of one set (deck, per polarity); `None` = not given, that
/// term is unknown.
#[derive(Clone, Copy, Debug, Default)]
pub struct Coeffs {
    /// `A_VT`, mV·µm.
    pub avt_mv_um: Option<f32>,
    /// `S_VT`, µV/µm, at this set's gate length (S(L) when the deck has the fit, MAT-16).
    pub svt_uv_per_um: Option<f32>,
    /// Anisotropic `(S_x, S_y)`, µV/µm: no deck key sets it; isotropic `S_VT` when `None`.
    pub svt_xy: Option<(f32, f32)>,
    /// BSIM4 `KVTH0`, mV·µm.
    pub kvth0_mv_um: Option<f32>,
    /// `|dV_T/dT|`, µV/K.
    pub tc_uv_per_k: Option<f32>,
    /// Current-factor mismatch `A_β`, %·µm (Hastings eq. 13.43).
    pub abeta_pct_um: Option<f32>,
    /// Passive / bipolar area constant `k_A` of a pair, %·µm (R, C: ΔR/R,
    /// ΔC/C, Hastings eqs 8.8/8.12; BJT/diode: ΔI_S/I_S, eq. 10.9).
    pub ka_pct_um: Option<f32>,
    /// Resistor/capacitor temperature coefficient, ppm/K (eq. 8.23).
    pub tc_ppm_per_k: Option<f32>,
    /// `|dV_BE/dT|`, µV/K (bipolar, diode).
    pub vbe_tc_uv_per_k: Option<f32>,
    /// Passive gradient `S_D`, %/mm (eq. 8.15).
    pub sd_pct_per_mm: Option<f32>,
    /// Mobility exponent, β ∝ T^−exp (1.7 NMOS, 1.5 PMOS): read on a mirror's % ledger only.
    pub mobility_exp: Option<f32>,
    /// Die temperature, K (`Config.op`); `None` skips the mobility term.
    pub die_temp_k: Option<f32>,
}

/// Distance coefficient at gate length `l_um`, µV/µm: `√(a + b/L²)` (S_D²
/// grows with 1/L², Schaper & Linnenbank Fig. 8; MM-25).
#[must_use]
pub fn svt_of_l(a_uv2_per_um2: f32, b_uv2: f32, l_um: f32) -> f32 {
    (a_uv2_per_um2 + b_uv2 / (l_um * l_um)).sqrt()
}

/// Gradient σ of a centroid offset `(dx, dy)` nm under `(S_x, S_y)` µV/µm, mV:
/// `‖(S_x·dx, S_y·dy)‖·1e-6`.
#[must_use]
pub fn sigma_grad_mv(s_xy_uv_per_um: (f32, f32), dx_nm: f32, dy_nm: f32) -> f32 {
    (s_xy_uv_per_um.0 * dx_nm).hypot(s_xy_uv_per_um.1 * dy_nm) * 1e-6
}

/// Current-factor change of a member `dt_k` warmer than its partner, %:
/// `−100·exp·ΔT/T` for `k ∝ T^−exp` (Hastings §12.1.1, PDF p.578). Signed,
/// ΔT of member a minus b.
#[must_use]
pub fn mobility_pct(exp: f32, dt_k: f32, t_abs_k: f32) -> f32 {
    -100.0 * exp * dt_k / t_abs_k
}

/// ΔV_BE σ of a bipolar/diode pair from its ΔI_S/I_S σ, mV: `V_T·ln(1 + σ/100)`
/// (Hastings eq. 10.9).
///
/// ponytail: V_T fixed at 25.7 mV (25 °C); REL-05 brings the junction temperature.
#[must_use]
pub fn bjt_sigma_vbe_mv(sigma_i_pct: f32) -> f32 {
    25.7 * (sigma_i_pct / 100.0).ln_1p()
}

/// Ratio error of a passive pair whose members sit `dt_mk` apart, %:
/// `TC·ΔT` (eq. 8.23; ppm/K · mK → ×1e-7).
#[must_use]
pub fn ratio_thermal_pct(tc_ppm_per_k: f32, dt_mk: f32) -> f32 {
    tc_ppm_per_k * dt_mk * 1e-7
}

/// One pair's ledger, in `unit` (mV unless a mirror's G and `A_β` are known).
///
/// - `sigma_rand` = `A_VT·√((1/a₁+1/a₂)/2)` ([`sigma_pair`]).
/// - `sigma_grad` = `‖(S_x·Δx, S_y·Δy)‖` (Pelgrom eq. (1) distance term, PDF p.1;
///   `S_x = S_y = S_VT` unless `svt_xy`).
/// - `mu_thermal` = `TC·|ΔT̄|` (Hastings eq. 8.23, PDF p.388), `ΔT̄` the
///   unit-weighted mean rise over each member's units (centroids without
///   units); a mirror's % ledger adds `|`[`mobility_pct`]`|` (MAT-14), an mV
///   ledger skips it (no G).
/// - `mu_lod` = `KVTH0·|⟨lod⟩_a − ⟨lod⟩_b|`, unit-weighted means (REV eq. 11).
/// - `coincidence` = `|Δm|/tol` when the members' unit counts admit a
///   common-centroid row (Hastings Table 8.4 rule 1, PDF p.392): process-free,
///   binds at 1.
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
    pub unit: LedgerUnit,
}

/// One matched pair's ledger as the report prints it (MAT-13), in `unit`.
///
/// `members` are schematic device ids `(reference, member)`;
/// `sigma_layout` is the gradient term; `order` the moment orders the
/// members' units cancel ([`crate::matching::moments::cancelled_order`], 0
/// without units); `phi_equal` whether their orientation counts agree (`None`
/// without units); `sizing_limited` a `Sigma1Mv`/`Sigma1Pct` budget the random σ alone
/// already meets or exceeds (allowance 0: the sizing, not the layout, must change).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LedgerRow {
    pub members: (u32, u32),
    pub unit: &'static str,
    pub sigma_rand: f32,
    pub sigma_layout: f32,
    pub mu_thermal: f32,
    pub mu_lod: f32,
    pub allowance: f32,
    pub usage: f32,
    pub order: u8,
    pub second_order_nm: f32,
    pub phi_equal: Option<bool>,
    pub known: bool,
    pub sizing_limited: bool,
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
    fn class_budget_is_one_sixth_of_the_limit() {
        assert_eq!(Budget::from_class(ClassLimit::Mv(3.0), MatchKind::Voltage), Budget::Sigma1Mv(0.5));
        assert_eq!(Budget::from_class(ClassLimit::Pct(3.0), MatchKind::Current), Budget::Sigma1Pct(0.5));
        assert_eq!(Budget::from_class(ClassLimit::Mv(3.0), MatchKind::Current), Budget::Eta(0.3));
    }

    #[test]
    fn role_default_keeps_eta() {
        let v = MatchKind::Voltage;
        assert_eq!(choose(None, None, None, v), Budget::Eta(0.3));
        assert_eq!(choose(Some(1.0), Some(0.2), Some(ClassLimit::Mv(3.0)), v), Budget::Sigma1Mv(1.0));
        assert_eq!(choose(None, Some(0.2), Some(ClassLimit::Mv(3.0)), v), Budget::Allowance(0.2));
        assert_eq!(choose(None, None, Some(ClassLimit::Mv(3.0)), v), Budget::Sigma1Mv(0.5));
    }

    #[test]
    fn hastings_eq_8_12_capacitor_pair() {
        assert!((sigma_pair(1.0, 1.0, 4.0) - 0.791).abs() < 1e-3);
    }

    #[test]
    fn bjt_eq_10_9() {
        let s = sigma_pair(2.0, 36.0, 36.0);
        assert!((s - 0.333).abs() < 1e-3, "{s}");
        assert!((bjt_sigma_vbe_mv(s) - 0.0855).abs() < 5e-4, "{}", bjt_sigma_vbe_mv(s));
    }

    #[test]
    fn svt_fit_reproduces_dvp_table_1() {
        for (l, s) in [(0.12, 1.624), (0.24, 0.893), (0.48, 0.580)] {
            assert!((svt_of_l(0.1835, 0.03533, l) - s).abs() < 0.01, "{l}: {}", svt_of_l(0.1835, 0.03533, l));
        }
    }

    #[test]
    fn anisotropic_grad() {
        assert!((sigma_grad_mv((1.0, 2.0), 1000.0, 1000.0) - 5f32.sqrt() * 1e-3).abs() < 1e-6);
    }

    #[test]
    fn mobility_term_sign() {
        assert!((mobility_pct(1.7, 1.0, 300.0) + 0.5667).abs() < 1e-3);
        assert!((mobility_pct(1.5, 1.0, 300.0) + 0.5).abs() < 1e-6);
    }

    #[test]
    fn resistor_thermal_ppm() {
        assert!((ratio_thermal_pct(100.0, 500.0) - 0.005).abs() < 1e-7);
    }

    #[test]
    fn current_domain_matches_hastings_13_43() {
        assert!((sigma_current_pct(1.0, 10.0, 0.5) - 1.118).abs() < 1e-3);
        assert!((sigma_voltage_mv(1.0, 10.0, 0.5) - 1.118).abs() < 1e-3);
    }

    #[test]
    fn without_abeta_usage_is_domain_invariant() {
        let mv = Ledger { sigma_rand: 2.0, sigma_grad: 0.3, mu_thermal: 0.2, mu_lod: 0.1, allowance: Budget::Sigma1Mv(3.0).allowance(2.0), ..Ledger::default() };
        for g in [10.0f32, 7.0] {
            let k = 0.1 * g;
            let pct = Ledger {
                sigma_rand: 2.0 * k,
                sigma_grad: 0.3 * k,
                mu_thermal: 0.2 * k,
                mu_lod: 0.1 * k,
                allowance: Budget::Sigma1Mv(3.0).to_pct(g).allowance(2.0 * k),
                unit: LedgerUnit::Pct,
                ..Ledger::default()
            };
            assert!((pct.usage() - mv.usage()).abs() < 1e-6, "G {g}: {} vs {}", pct.usage(), mv.usage());
        }
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
