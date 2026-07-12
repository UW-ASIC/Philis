use std::collections::HashSet;

use super::{CheckReport, SignoffCheck, SignoffViolation};

#[derive(Debug, Clone, PartialEq)]
pub struct VoltageStress {
    pub id: String,
    pub measured_abs_v: f64,
    pub max_abs_v: f64,
    pub location: Option<(i32, i32)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThermalStress {
    pub id: String,
    pub measured_c: f64,
    pub max_c: f64,
    pub location: Option<(i32, i32)>,
}

/// Foundry-calibrated inverse-power/Arrhenius lifetime model.  It can represent
/// mechanisms such as BTI, HCI, TDDB, stress migration, or EM when populated
/// with the corresponding foundry coefficients.
#[derive(Debug, Clone, PartialEq)]
pub struct AgingStress {
    pub id: String,
    pub mechanism: String,
    pub reference_lifetime_hours: f64,
    pub reference_stress: f64,
    pub applied_stress: f64,
    pub stress_exponent: f64,
    pub reference_temperature_c: f64,
    pub applied_temperature_c: f64,
    pub activation_energy_ev: f64,
    /// Fraction of lifetime spent under this stress, in [0, 1].
    pub duty_cycle: f64,
    pub location: Option<(i32, i32)>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReliabilityConfig {
    pub required_lifetime_hours: f64,
    pub voltage_stresses: Vec<VoltageStress>,
    pub thermal_stresses: Vec<ThermalStress>,
    pub aging_stresses: Vec<AgingStress>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgingStressResult {
    pub id: String,
    pub mechanism: String,
    pub predicted_lifetime_hours: f64,
    pub required_lifetime_hours: f64,
}

#[derive(Debug, Clone)]
pub struct ReliabilityReport {
    pub check: CheckReport,
    pub aging: Vec<AgingStressResult>,
}

impl ReliabilityReport {
    pub(crate) fn not_run(reason: impl Into<String>) -> Self {
        Self {
            check: CheckReport::not_run(SignoffCheck::Reliability, reason),
            aging: Vec::new(),
        }
    }

    fn error(reason: impl Into<String>) -> Self {
        Self {
            check: CheckReport::error(SignoffCheck::Reliability, reason),
            aging: Vec::new(),
        }
    }
}

const BOLTZMANN_EV_PER_K: f64 = 8.617_333_262_145e-5;

pub fn check_reliability(config: &ReliabilityConfig) -> ReliabilityReport {
    if config.voltage_stresses.is_empty()
        && config.thermal_stresses.is_empty()
        && config.aging_stresses.is_empty()
    {
        return ReliabilityReport::error(
            "reliability configuration contains no stress observations",
        );
    }
    if !config.required_lifetime_hours.is_finite() || config.required_lifetime_hours <= 0.0 {
        return ReliabilityReport::error(
            "required reliability lifetime must be finite and positive",
        );
    }

    let mut violations = Vec::new();
    let mut aging_results = Vec::new();
    let mut voltage_ids = HashSet::new();
    for s in &config.voltage_stresses {
        if s.id.is_empty()
            || !voltage_ids.insert(s.id.as_str())
            || !s.measured_abs_v.is_finite()
            || !s.max_abs_v.is_finite()
            || s.measured_abs_v < 0.0
            || s.max_abs_v <= 0.0
        {
            return ReliabilityReport::error(format!("voltage stress '{}' is invalid", s.id));
        }
        if s.measured_abs_v > s.max_abs_v {
            violations.push(SignoffViolation {
                check: SignoffCheck::Reliability,
                rule_id: format!("reliability.voltage.{}", s.id),
                message: format!(
                    "'{}' voltage stress {:.6}V exceeds {:.6}V",
                    s.id, s.measured_abs_v, s.max_abs_v,
                ),
                location: s.location,
                measured: Some(s.measured_abs_v),
                limit: Some(s.max_abs_v),
                units: "V".into(),
            });
        }
    }
    let mut thermal_ids = HashSet::new();
    for s in &config.thermal_stresses {
        if s.id.is_empty()
            || !thermal_ids.insert(s.id.as_str())
            || !s.measured_c.is_finite()
            || !s.max_c.is_finite()
            || s.measured_c <= -273.15
            || s.max_c <= -273.15
        {
            return ReliabilityReport::error(format!("thermal stress '{}' is invalid", s.id));
        }
        if s.measured_c > s.max_c {
            violations.push(SignoffViolation {
                check: SignoffCheck::Reliability,
                rule_id: format!("reliability.thermal.{}", s.id),
                message: format!(
                    "'{}' temperature {:.3}C exceeds {:.3}C",
                    s.id, s.measured_c, s.max_c,
                ),
                location: s.location,
                measured: Some(s.measured_c),
                limit: Some(s.max_c),
                units: "degC".into(),
            });
        }
    }

    let mut aging_ids = HashSet::new();
    for s in &config.aging_stresses {
        if s.id.is_empty()
            || !aging_ids.insert(s.id.as_str())
            || s.mechanism.is_empty()
            || !s.reference_lifetime_hours.is_finite()
            || s.reference_lifetime_hours <= 0.0
            || !s.reference_stress.is_finite()
            || s.reference_stress <= 0.0
            || !s.applied_stress.is_finite()
            || s.applied_stress < 0.0
            || !s.stress_exponent.is_finite()
            || s.stress_exponent <= 0.0
            || !s.reference_temperature_c.is_finite()
            || s.reference_temperature_c <= -273.15
            || !s.applied_temperature_c.is_finite()
            || s.applied_temperature_c <= -273.15
            || !s.activation_energy_ev.is_finite()
            || s.activation_energy_ev < 0.0
            || !s.duty_cycle.is_finite()
            || !(0.0..=1.0).contains(&s.duty_cycle)
        {
            return ReliabilityReport::error(format!("aging stress '{}' is invalid", s.id));
        }

        let predicted = if s.applied_stress == 0.0 || s.duty_cycle == 0.0 {
            f64::INFINITY
        } else {
            let stress_factor = (s.reference_stress / s.applied_stress).powf(s.stress_exponent);
            let tref = s.reference_temperature_c + 273.15;
            let tuse = s.applied_temperature_c + 273.15;
            let thermal_factor =
                (s.activation_energy_ev / BOLTZMANN_EV_PER_K * (1.0 / tuse - 1.0 / tref)).exp();
            s.reference_lifetime_hours * stress_factor * thermal_factor / s.duty_cycle
        };
        if predicted.is_nan() || predicted <= 0.0 {
            return ReliabilityReport::error(format!(
                "aging model '{}' produced an invalid lifetime",
                s.id
            ));
        }
        aging_results.push(AgingStressResult {
            id: s.id.clone(),
            mechanism: s.mechanism.clone(),
            predicted_lifetime_hours: predicted,
            required_lifetime_hours: config.required_lifetime_hours,
        });
        if predicted < config.required_lifetime_hours {
            violations.push(SignoffViolation {
                check: SignoffCheck::Reliability,
                rule_id: format!("reliability.aging.{}", s.id),
                message: format!(
                    "{} '{}' predicted lifetime {:.3}h is below {:.3}h",
                    s.mechanism, s.id, predicted, config.required_lifetime_hours,
                ),
                location: s.location,
                measured: Some(predicted),
                limit: Some(config.required_lifetime_hours),
                units: "hours".into(),
            });
        }
    }

    aging_results.sort_by(|a, b| a.id.cmp(&b.id));
    ReliabilityReport {
        check: CheckReport::from_violations(SignoffCheck::Reliability, violations, Vec::new()),
        aging: aging_results,
    }
}
