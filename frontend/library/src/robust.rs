//! Statistical layer over the schematic sensitivities ([`crate::perf`]): per
//! spec bound the random spread σ_f from device V_T mismatch (GRAEB-06,
//! σ_f = √(∇fᵀC∇f) with a diagonal C), the worst-case distance β = margin/σ_f
//! (GRAEB-07), its yield share Φ(β) (GRAEB-08), the statistical headroom
//! `margin − β_target·σ_f` (GRAEB-16), and a linearised joint yield
//! (GRAEB-33). V_T only: current-factor mismatch is not modelled, so σ_f
//! underestimates current-matched specs.

use pnr_core::Netlist;

use crate::perf::{Param, PerfResult, SensTable, Spec};

/// β a bound is designed to ("three-sigma design", GRAEB-16).
pub const BETA_TARGET: f64 = 3.0;

/// Device random V_T σ, V: `A_VT/√(2·A_gate)` with `A_gate` =
/// [`pnr_core::Device::gate_area_um2`] (µm²) and `avt_mv_um` the deck's pair
/// coefficients `[nfet, pfet]`, mV·µm (σ(ΔV_T) of a pair, so one device
/// carries 1/√2 of it). `None` for a non-FET, a FET without W/L, or without
/// the deck's A_VT for its polarity.
#[must_use]
pub fn device_sigma_v(netlist: &Netlist, avt_mv_um: [Option<f32>; 2]) -> Vec<Option<f64>> {
    use pnr_core::DeviceKind as K;
    netlist
        .devices
        .iter()
        .map(|d| {
            let avt = match d.kind {
                K::Nmos => avt_mv_um[0]?,
                K::Pmos => avt_mv_um[1]?,
                _ => return None,
            };
            let area = d.mos_size()?.gate_area_um2();
            (area > 0.0).then(|| f64::from(avt) / (2.0 * area).sqrt() / 1000.0)
        })
        .collect()
}

/// One bound's statistics; every field `None` when σ_f is unknown.
#[derive(Clone, Debug, PartialEq)]
pub struct BoundStat {
    /// Index into [`PerfResult::bounds`].
    pub bound: usize,
    /// σ_f, metric units; `≥ 0`.
    pub sigma_f: Option<f64>,
    /// Worst-case distance `margin/σ_f`, in σ: negative when the bound is
    /// already missed; `±∞` when σ_f = 0 (no spread), `+∞` for a zero margin.
    pub beta: Option<f64>,
    /// Φ(β).
    pub yield_part: Option<f64>,
    /// `margin − BETA_TARGET·σ_f`, metric units.
    pub headroom_stat: Option<f64>,
    /// `(device, s²σ²/σ_f²)`, largest first, at most 3; every share `0` when
    /// σ_f = 0.
    pub shares: Vec<(u16, f64)>,
}

/// Per FET (a `GateOffset` row of `t`): `(device, s·σ)` with `s` the row's
/// derivative of spec `j`. `None` when `t` has no such row, or any FET's σ or
/// derivative is unknown (unknown, never an underestimate). The row is used
/// whatever its `linear`: with a σ step it is the one-σ secant.
fn terms(t: &SensTable, sigma_v: &[Option<f64>], j: usize) -> Option<Vec<(u16, f64)>> {
    let v: Vec<(u16, f64)> = t
        .rows
        .iter()
        .filter_map(|r| match r.param {
            Param::GateOffset { device } => Some((device, r.d[j], sigma_v.get(device as usize).copied().flatten())),
            _ => None,
        })
        .map(|(dev, s, sigma)| Some((dev, s? * sigma?)))
        .collect::<Option<_>>()?;
    (!v.is_empty()).then_some(v)
}

/// Why bound `b`'s σ_f is `None` ([`bound_stats`]): `"no A_VT"` when its
/// table has a `GateOffset` row whose FET has no σ, `"unmeasured"` when the
/// bound has no value, else `"no sensitivities"` (no table for its scenario,
/// no `GateOffset` row, or a derivative missing).
///
/// # Panics
/// If `b >= post.bounds.len()`.
#[must_use]
pub fn unknown_reason(tables: &[SensTable], sigma_v: &[Option<f64>], post: &PerfResult, b: usize) -> &'static str {
    let bound = &post.bounds[b];
    let Some(t) = tables.iter().find(|t| t.scenario == bound.scenario) else { return "no sensitivities" };
    if t.rows.iter().any(|r| matches!(r.param, Param::GateOffset { device } if sigma_v.get(device as usize).copied().flatten().is_none())) {
        "no A_VT"
    } else if bound.value.is_none() {
        "unmeasured"
    } else {
        "no sensitivities"
    }
}

/// `(f_post + Δf_sys − lo)` for a floor, `(hi − f_post − Δf_sys)` for a
/// ceiling; `None` without a value or bound.
fn margin(post: &PerfResult, specs: &[Spec], b: usize, sys: &[f64]) -> Option<f64> {
    let bound = &post.bounds[b];
    let f = bound.value? + sys.get(b).copied().unwrap_or(0.0);
    let spec = &specs[bound.spec];
    if bound.upper {
        Some(spec.max? - f)
    } else {
        Some(f - spec.min?)
    }
}

/// Per bound of `post` (spec `specs[bound.spec]`): σ_f² = Σ_i s_i²σ_i² +
/// Σ_(p,q,σ_g)∈grad ((s_p − s_q)/2)²σ_g², s_i the `GateOffset` derivative in
/// the table whose `scenario` is the bound's; β = margin/σ_f with `sys[b]` =
/// Δf_sys added to the post-layout value (empty = none). A `grad` entry
/// `(p, q, σ_g)` is a gradient term shared by devices `p` and `q` (a device
/// without a row reads `s = 0`).
///
/// A bound is all-unknown when its scenario has no table, any FET's σ or
/// derivative is unknown ([`unknown_reason`] says which), or it has no value.
/// σ_f = 0 (every derivative zero) is known: β is `+∞` for a margin `≥ 0`
/// and `−∞` below, Φ(β) is 1 or 0, and every share is 0.
///
/// # Panics
/// If a bound's `spec` is out of `specs`, or a `GateOffset` row's `d` is
/// shorter than the spec count.
#[must_use]
pub fn bound_stats(tables: &[SensTable], sigma_v: &[Option<f64>], post: &PerfResult, specs: &[Spec], sys: &[f64], grad: &[(u16, u16, f64)]) -> Vec<BoundStat> {
    (0..post.bounds.len())
        .map(|b| {
            let bound = &post.bounds[b];
            let unknown = BoundStat { bound: b, sigma_f: None, beta: None, yield_part: None, headroom_stat: None, shares: Vec::new() };
            let Some(t) = tables.iter().find(|t| t.scenario == bound.scenario) else { return unknown };
            let (Some(ts), Some(m)) = (terms(t, sigma_v, bound.spec), margin(post, specs, b, sys)) else { return unknown };
            let s = |dev: u16| t.rows.iter().find(|r| r.param == Param::GateOffset { device: dev }).and_then(|r| r.d[bound.spec]).unwrap_or(0.0);
            let var: f64 = ts.iter().map(|&(_, x)| x * x).sum::<f64>() + grad.iter().map(|&(p, q, g)| ((s(p) - s(q)) / 2.0 * g).powi(2)).sum::<f64>();
            let sigma = var.sqrt();
            // σ_f = 0: no spread, so the bound holds for certain when its margin does.
            let beta = if sigma > 0.0 { m / sigma } else if m >= 0.0 { f64::INFINITY } else { f64::NEG_INFINITY };
            let mut shares: Vec<(u16, f64)> = ts.iter().map(|&(d, x)| (d, if var > 0.0 { x * x / var } else { 0.0 })).collect();
            shares.sort_by(|a, b| b.1.total_cmp(&a.1));
            shares.truncate(3);
            BoundStat { bound: b, sigma_f: Some(sigma), beta: Some(beta), yield_part: Some(phi(beta)), headroom_stat: Some(m - BETA_TARGET * sigma), shares }
        })
        .collect()
}

/// PERF-14's spec tiers `(failed, shortfall)` of `post`. `beta_key`: failed = #bounds with β < 0 or β unknown
/// (NaN reads as unknown), shortfall = max_b max(0, [`BETA_TARGET`] − β_b), +∞ when any β is unknown. Otherwise (σ unknown for the run):
/// failed = #bounds unmeasured or missing their side ([`crate::perf::miss`] of the one-sided spec > 0), shortfall =
/// `post.residual` (the pre-PERF-14 tier).
#[must_use]
pub fn key_tiers(stats: &[BoundStat], post: &PerfResult, specs: &[Spec], beta_key: bool) -> (u32, f64) {
    if beta_key {
        let beta = |s: &BoundStat| s.beta.filter(|b| !b.is_nan());
        let failed = stats.iter().filter(|s| beta(s).is_none_or(|b| b < 0.0)).count() as u32;
        let shortfall = stats.iter().map(|s| beta(s).map_or(f64::INFINITY, |b| (BETA_TARGET - b).max(0.0))).fold(0.0, f64::max);
        return (failed, shortfall);
    }
    let failed = post.bounds.iter().filter(|b| crate::perf::side_miss(&specs[b.spec], b.upper, b.value) > 0.0).count() as u32;
    (failed, post.residual)
}

/// Smallest β over `stats`; `None` if empty or any is unknown (NaN reads as
/// unknown).
#[must_use]
pub fn min_beta(stats: &[BoundStat]) -> Option<f64> {
    stats.iter().try_fold(None, |acc: Option<f64>, s| {
        let b = s.beta.filter(|b| !b.is_nan())?;
        Some(Some(acc.map_or(b, |a| a.min(b))))
    })?
}

/// Fraction of `samples` normal draws `t_i ~ N(0, 1)` (one per device, shared
/// by every bound) for which every bound holds under the linear model `f_b =
/// f_post,b + Δf_sys,b + Σ_i s_bi σ_i t_i`. `None` if `samples` is 0 or any
/// bound's σ_f is unknown; `Some(1.0)` with no bounds. Deterministic for
/// `seed`. Cost O(samples · (devices + Σ_b terms_b)).
#[must_use]
pub fn linear_joint_yield(tables: &[SensTable], sigma_v: &[Option<f64>], post: &PerfResult, specs: &[Spec], sys: &[f64], samples: usize, seed: u64) -> Option<f64> {
    if samples == 0 {
        return None;
    }
    // Per bound: margin and (device, s·σ) signed so a positive sum eats margin.
    let bounds: Vec<(f64, Vec<(usize, f64)>)> = (0..post.bounds.len())
        .map(|b| {
            let bound = &post.bounds[b];
            let t = tables.iter().find(|t| t.scenario == bound.scenario)?;
            let sign = if bound.upper { 1.0 } else { -1.0 };
            let ts = terms(t, sigma_v, bound.spec)?.into_iter().map(|(d, x)| (usize::from(d), sign * x)).collect();
            Some((margin(post, specs, b, sys)?, ts))
        })
        .collect::<Option<_>>()?;
    let mut rng = gp::mechanics::SplitMix64::new(seed);
    let mut t = vec![0.0; sigma_v.len()];
    let mut pass = 0usize;
    for _ in 0..samples {
        // Box–Muller: two independent N(0, 1) draws per pair of uniforms.
        for pair in t.chunks_mut(2) {
            let (u1, u2) = (rng.f64(), rng.f64());
            let r = (-2.0 * (1.0 - u1).ln()).sqrt();
            let a = std::f64::consts::TAU * u2;
            pair[0] = r * a.cos();
            if let Some(x) = pair.get_mut(1) {
                *x = r * a.sin();
            }
        }
        if bounds.iter().all(|(m, ts)| ts.iter().map(|&(d, x)| x * t.get(d).copied().unwrap_or(0.0)).sum::<f64>() <= *m) {
            pass += 1;
        }
    }
    Some(pass as f64 / samples as f64)
}

/// Standard normal cdf, `0.5·erfc(−x/√2)`; erfc by Abramowitz–Stegun 7.1.26
/// (|error| ≤ 1.5e-7) for `z ≥ 0`, `2 − erfc(−z)` below, so `Φ(x) + Φ(−x) =
/// 1` exactly. An implementation choice, not from docs/ref/.
#[must_use]
pub fn phi(x: f64) -> f64 {
    fn erfc(z: f64) -> f64 {
        if z < 0.0 {
            return 2.0 - erfc(-z);
        }
        let t = 1.0 / (1.0 + 0.327_591_1 * z);
        let poly = t * (0.254_829_592 + t * (-0.284_496_736 + t * (1.421_413_741 + t * (-1.453_152_027 + t * 1.061_405_429))));
        poly * (-z * z).exp()
    }
    0.5 * erfc(-x / std::f64::consts::SQRT_2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::perf::{score, Parasitics, SensRow};

    fn table(d: &[f64]) -> SensTable {
        let rows = d.iter().enumerate().map(|(i, &d)| SensRow { param: Param::GateOffset { device: i as u16 }, step: 1e-3, d: vec![Some(d)], linear: true }).collect();
        SensTable { scenario: 0, at: Parasitics::default(), base: PerfResult::default(), rows, sims: 0 }
    }

    fn floor(value: f64, min: f64) -> (Vec<Spec>, PerfResult) {
        let specs = vec![Spec { metric: "gain".into(), min: Some(min), max: None }];
        let post = score(&specs, &[vec![Some(value)]], &[0]);
        (specs, post)
    }

    #[test]
    fn sigma_f_of_a_symmetric_pair() {
        let (specs, post) = floor(10.0, 0.0);
        let s = &bound_stats(&[table(&[1000.0, -1000.0])], &[Some(1e-3), Some(1e-3)], &post, &specs, &[], &[])[0];
        assert!((s.sigma_f.unwrap() - 2f64.sqrt()).abs() < 1e-12, "{s:?}");
        assert_eq!(s.shares.len(), 2);
        for (k, &(d, w)) in s.shares.iter().enumerate() {
            assert!(d as usize == k && (w - 0.5).abs() < 1e-12, "{s:?}");
        }
    }

    /// Graeb Table 14: gain 76 dB against a 65 dB floor at σ_f = 4.4 dB is β 2.5.
    #[test]
    fn beta_reproduces_graeb_table_14_gain() {
        let (specs, post) = floor(76.0, 65.0);
        let s = &bound_stats(&[table(&[4400.0])], &[Some(1e-3)], &post, &specs, &[], &[])[0];
        assert!((s.beta.unwrap() - 2.5).abs() < 1e-12, "{s:?}");
        assert!((s.headroom_stat.unwrap() - (11.0 - 3.0 * 4.4)).abs() < 1e-12, "{s:?}");
    }

    #[test]
    fn systematic_shift_moves_beta() {
        let (specs, post) = floor(76.0, 65.0);
        let s = &bound_stats(&[table(&[4400.0])], &[Some(1e-3)], &post, &specs, &[1.1], &[])[0];
        assert!((s.beta.unwrap() - 2.75).abs() < 1e-12, "{s:?}");
    }

    #[test]
    fn a_missing_sigma_is_unknown() {
        let (specs, post) = floor(10.0, 0.0);
        let s = &bound_stats(&[table(&[1000.0, -1000.0])], &[Some(1e-3), None], &post, &specs, &[], &[])[0];
        assert_eq!((s.sigma_f, s.beta, s.yield_part), (None, None, None), "{s:?}");
    }

    #[test]
    fn unknown_reason_names_the_missing_input() {
        let (_, post) = floor(10.0, 0.0);
        assert_eq!(unknown_reason(&[], &[], &post, 0), "no sensitivities");
        assert_eq!(unknown_reason(&[table(&[1.0, 1.0])], &[Some(1e-3), None], &post, 0), "no A_VT");
        let mut t = table(&[1.0]);
        t.rows[0].d[0] = None;
        assert_eq!(unknown_reason(&[t], &[Some(1e-3)], &post, 0), "no sensitivities");
    }

    /// Graeb Table 12.
    #[test]
    fn phi_matches_table_12() {
        for (x, want) in [(-1.0, 0.159), (0.0, 0.500), (1.0, 0.841), (2.0, 0.977), (3.0, 0.999)] {
            assert!((phi(x) - want).abs() < 1e-3, "Φ({x}) = {}", phi(x));
        }
        assert!((phi(-0.3) + phi(0.3) - 1.0).abs() < 1e-7);
    }

    #[test]
    fn device_sigma_of_a_sky130_nfet() {
        use pnr_core::{Device, DeviceKind, Net, NetId};
        let dev = |kind, params: Vec<(String, i64)>| Device { name: "M1".into(), kind, model: String::new(), terminals: vec![("G".into(), NetId(0))], params };
        let nl = Netlist {
            devices: vec![
                dev(DeviceKind::Nmos, vec![("w".into(), 1000), ("l".into(), 1000), ("m".into(), 1)]),
                dev(DeviceKind::Resistor, vec![("w".into(), 1000), ("l".into(), 1000)]),
            ],
            nets: vec![Net { name: "g".into() }],
            ..Default::default()
        };
        let s = device_sigma_v(&nl, [Some(9.5), Some(11.5)]);
        assert!((s[0].unwrap() - 6.7175e-3).abs() < 1e-5, "{s:?}");
        assert_eq!(s[1], None);
        assert_eq!(device_sigma_v(&nl, [None, Some(11.5)])[0], None);
    }

    #[test]
    fn linear_yield_of_one_bound_is_phi_beta() {
        // σ_f = 1000 · 1e-3 = 1, margin 1.5: β = 1.5.
        let (specs, post) = floor(1.5, 0.0);
        let y = linear_joint_yield(&[table(&[1000.0])], &[Some(1e-3)], &post, &specs, &[], 100_000, 1).unwrap();
        assert!((y - phi(1.5)).abs() <= 0.005, "{y} vs {}", phi(1.5));
    }

    fn stat(b: usize, beta: Option<f64>) -> BoundStat {
        BoundStat { bound: b, sigma_f: beta.map(|_| 1.0), beta, yield_part: None, headroom_stat: None, shares: Vec::new() }
    }

    #[test]
    fn robust_beats_barely_passing() {
        let (specs, post) = floor(10.0, 0.0);
        assert_eq!(key_tiers(&[stat(0, Some(2.5))], &post, &specs, true), (0, 0.5));
        assert_eq!(key_tiers(&[stat(0, Some(1.0))], &post, &specs, true), (0, 2.0));
        assert!(crate::key_lt(&(0, 0, 0.5, 9.0, 9.0, 9.0), &(0, 0, 2.0, 0.0, 0.0, 0.0)), "robustness outranks Θ, C, area");
    }

    #[test]
    fn an_unknown_bound_counts_as_failed() {
        let specs = vec![Spec { metric: "gain".into(), min: Some(0.0), max: None }, Spec { metric: "pm".into(), min: Some(60.0), max: None }];
        let post = score(&specs, &[vec![Some(10.0), None]], &[0]);
        assert_eq!(key_tiers(&[stat(0, Some(4.0)), stat(1, None)], &post, &specs, true), (1, f64::INFINITY));
        assert_eq!(key_tiers(&[], &post, &specs, false), (1, post.residual));
        assert_eq!(min_beta(&[stat(0, Some(4.0)), stat(1, None)]), None);
        let (specs, post) = floor(10.0, 0.0);
        assert_eq!(key_tiers(&[], &post, &specs, false), (0, 0.0), "a met bound in miss mode");
        let (specs, post) = floor(5.0, 10.0);
        assert!(post.residual > 0.0);
        assert_eq!(key_tiers(&[], &post, &specs, false), (1, post.residual), "a measured miss in miss mode");
        assert_eq!(key_tiers(&[stat(0, Some(-0.5))], &post, &specs, true), (1, 3.5), "β < 0 fails");
    }
}

/// Step-2 coverage: every public function's branches and corners. Oracles:
/// the GRAEB formulas in the docs, Φ's symmetry, and hand-worked values.
#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use crate::perf::{score, Parasitics, SensRow};
    use pnr_core::{Device, DeviceKind};

    fn gate_rows(d: &[f64]) -> Vec<SensRow> {
        d.iter().enumerate().map(|(i, &d)| SensRow { param: Param::GateOffset { device: i as u16 }, step: 1e-3, d: vec![Some(d)], linear: true }).collect()
    }

    fn table(scenario: usize, rows: Vec<SensRow>) -> SensTable {
        SensTable { scenario, at: Parasitics::default(), base: PerfResult::default(), rows, sims: 0 }
    }

    fn one(min: Option<f64>, max: Option<f64>, value: Option<f64>) -> (Vec<Spec>, PerfResult) {
        let specs = vec![Spec { metric: "m".into(), min, max }];
        let post = score(&specs, &[vec![value]], &[0]);
        (specs, post)
    }

    fn fet(kind: DeviceKind, params: &[(&str, i64)]) -> Device {
        Device { name: "M".into(), kind, model: String::new(), terminals: Vec::new(), params: params.iter().map(|&(k, v)| (k.into(), v)).collect() }
    }

    #[test]
    fn device_sigma_reads_the_polarity_and_the_area() {
        let nl = Netlist {
            devices: vec![
                fet(DeviceKind::Pmos, &[("w", 1000), ("l", 1000)]),
                fet(DeviceKind::Nmos, &[("w", 1000), ("l", 1000), ("m", 2)]),
                fet(DeviceKind::Nmos, &[("w", 1000)]),
            ],
            ..Default::default()
        };
        let s = device_sigma_v(&nl, [Some(10.0), Some(20.0)]);
        assert_eq!(s.len(), 3);
        // PMOS: 20 mV·µm / √(2·1 µm²) = 14.142 mV.
        assert!((s[0].unwrap() - 0.02 / 2f64.sqrt()).abs() < 1e-12, "{s:?}");
        // Twice the area: σ / √2.
        assert!((s[1].unwrap() - 0.01 / 2.0).abs() < 1e-12, "{s:?}");
        assert_eq!(s[2], None, "no L");
        assert_eq!(device_sigma_v(&nl, [Some(10.0), None])[0], None, "no PMOS A_VT");
        assert!(device_sigma_v(&Netlist::default(), [Some(1.0), Some(1.0)]).is_empty());
    }

    #[test]
    fn a_bound_without_a_table_or_rows_or_value_is_unknown() {
        let (specs, post) = one(Some(0.0), None, Some(1.0));
        let unknown = |tables: &[SensTable], post: &PerfResult| {
            let s = &bound_stats(tables, &[Some(1e-3)], post, &specs, &[], &[])[0];
            (s.sigma_f, s.beta, s.yield_part, s.headroom_stat, s.shares.is_empty())
        };
        let none = (None, None, None, None, true);
        assert_eq!(unknown(&[], &post), none, "no table");
        assert_eq!(unknown(&[table(1, gate_rows(&[1.0]))], &post), none, "table at another scenario");
        assert_eq!(unknown(&[table(0, Vec::new())], &post), none, "no GateOffset row");
        let (_, unmeasured) = one(Some(0.0), None, None);
        assert_eq!(unknown(&[table(0, gate_rows(&[1.0]))], &unmeasured), none, "no value");
    }

    #[test]
    fn a_spec_without_bounds_has_no_stats() {
        let (specs, post) = one(None, None, Some(1.0));
        assert!(bound_stats(&[table(0, gate_rows(&[1.0]))], &[Some(1e-3)], &post, &specs, &[], &[]).is_empty());
    }

    #[test]
    fn a_ceiling_margin_is_hi_minus_f() {
        let (specs, post) = one(None, Some(10.0), Some(7.0));
        let s = &bound_stats(&[table(0, gate_rows(&[1000.0]))], &[Some(1e-3)], &post, &specs, &[], &[])[0];
        assert!((s.sigma_f.unwrap() - 1.0).abs() < 1e-12);
        assert!((s.beta.unwrap() - 3.0).abs() < 1e-12, "{s:?}");
        assert!(s.headroom_stat.unwrap().abs() < 1e-12, "β exactly at target: no headroom");
        assert!((s.yield_part.unwrap() - phi(3.0)).abs() < 1e-15);
        // Δf_sys raises the post-layout value: a ceiling loses margin.
        let s = &bound_stats(&[table(0, gate_rows(&[1000.0]))], &[Some(1e-3)], &post, &specs, &[1.0], &[])[0];
        assert!((s.beta.unwrap() - 2.0).abs() < 1e-12, "{s:?}");
    }

    /// σ_f² = Σ s²σ² + ((s_p − s_q)/2)²σ_g²: (1)² + (1)² + ((1000 + 1000)/2 · 1e-3)² = 3.
    #[test]
    fn a_shared_gradient_adds_half_the_difference() {
        let (specs, post) = one(Some(0.0), None, Some(3.0));
        let s = &bound_stats(&[table(0, gate_rows(&[1000.0, -1000.0]))], &[Some(1e-3), Some(1e-3)], &post, &specs, &[], &[(0, 1, 1e-3)])[0];
        assert!((s.sigma_f.unwrap() - 3f64.sqrt()).abs() < 1e-12, "{s:?}");
        // Gradient variance is not attributed to a device: the shares sum to 2/3.
        assert!((s.shares.iter().map(|x| x.1).sum::<f64>() - 2.0 / 3.0).abs() < 1e-12, "{s:?}");
    }

    #[test]
    fn shares_are_the_three_largest_in_order() {
        let (specs, post) = one(Some(0.0), None, Some(3.0));
        let sig = [Some(1e-3); 4];
        let s = &bound_stats(&[table(0, gate_rows(&[100.0, 400.0, 300.0, 200.0]))], &sig, &post, &specs, &[], &[])[0];
        let devs: Vec<u16> = s.shares.iter().map(|x| x.0).collect();
        assert_eq!(devs, [1, 2, 3]);
        assert!(s.shares.windows(2).all(|w| w[0].1 >= w[1].1));
    }

    /// No sensitivity at all: σ_f = 0 is known, β is ±∞ by the margin's
    /// sign, Φ(β) is 1 or 0, and no share is NaN.
    #[test]
    fn zero_spread_is_known_and_finite_shares() {
        let (specs, post) = one(Some(0.0), None, Some(2.0));
        let s = &bound_stats(&[table(0, gate_rows(&[0.0, 0.0]))], &[Some(1e-3), Some(1e-3)], &post, &specs, &[], &[])[0];
        assert_eq!(s.sigma_f, Some(0.0));
        assert_eq!(s.beta, Some(f64::INFINITY));
        assert_eq!(s.yield_part, Some(1.0));
        assert_eq!(s.headroom_stat, Some(2.0));
        assert!(s.shares.iter().all(|x| x.1 == 0.0), "{s:?}");
        let (specs, post) = one(Some(5.0), None, Some(2.0));
        let s = &bound_stats(&[table(0, gate_rows(&[0.0]))], &[Some(1e-3)], &post, &specs, &[], &[])[0];
        assert_eq!((s.beta, s.yield_part), (Some(f64::NEG_INFINITY), Some(0.0)));
        let (specs, post) = one(Some(2.0), None, Some(2.0));
        let s = &bound_stats(&[table(0, gate_rows(&[0.0]))], &[Some(1e-3)], &post, &specs, &[], &[])[0];
        assert_eq!(s.beta, Some(f64::INFINITY), "a zero margin with no spread is met");
    }

    #[test]
    fn unknown_reason_says_unmeasured_when_sigma_is_known() {
        let (_, post) = one(Some(0.0), None, None);
        assert_eq!(unknown_reason(&[table(0, gate_rows(&[1.0]))], &[Some(1e-3)], &post, 0), "unmeasured");
        assert_eq!(unknown_reason(&[table(0, gate_rows(&[1.0]))], &[], &post, 0), "no A_VT", "σ past the slice is missing");
    }

    #[test]
    #[should_panic]
    fn unknown_reason_panics_past_the_bounds() {
        let (_, post) = one(Some(0.0), None, Some(1.0));
        let _ = unknown_reason(&[], &[], &post, 1);
    }

    fn stat(beta: Option<f64>) -> BoundStat {
        BoundStat { bound: 0, sigma_f: beta.map(|_| 1.0), beta, yield_part: None, headroom_stat: None, shares: Vec::new() }
    }

    #[test]
    fn beta_key_tiers_of_no_bounds_are_zero() {
        let (specs, post) = one(None, None, Some(1.0));
        assert_eq!(key_tiers(&[], &post, &specs, true), (0, 0.0));
    }

    #[test]
    fn beta_above_target_has_no_shortfall() {
        let (specs, post) = one(Some(0.0), None, Some(1.0));
        assert_eq!(key_tiers(&[stat(Some(7.0)), stat(Some(0.0))], &post, &specs, true), (0, 3.0), "β = 0 is not failed");
    }

    /// NaN is not a β: it fails and has unbounded shortfall, like unknown.
    #[test]
    fn a_nan_beta_counts_as_unknown() {
        let (specs, post) = one(Some(0.0), None, Some(1.0));
        assert_eq!(key_tiers(&[stat(Some(f64::NAN))], &post, &specs, true), (1, f64::INFINITY));
        assert_eq!(min_beta(&[stat(Some(f64::NAN)), stat(Some(1.0))]), None);
        assert_eq!(min_beta(&[stat(Some(1.0)), stat(Some(f64::NAN))]), None);
    }

    #[test]
    fn miss_tiers_count_each_side_separately() {
        // Floor 10 and ceiling 20; one scenario at 25 misses the ceiling only.
        let specs = vec![Spec { metric: "m".into(), min: Some(10.0), max: Some(20.0) }];
        let post = score(&specs, &[vec![Some(25.0)]], &[0]);
        assert_eq!(key_tiers(&[], &post, &specs, false), (1, post.residual));
        assert!((post.residual - 0.25).abs() < 1e-12);
    }

    #[test]
    fn min_beta_is_the_smallest() {
        assert_eq!(min_beta(&[]), None);
        assert_eq!(min_beta(&[stat(Some(2.0)), stat(Some(-1.0)), stat(Some(5.0))]), Some(-1.0));
        assert_eq!(min_beta(&[stat(Some(f64::INFINITY))]), Some(f64::INFINITY));
    }

    #[test]
    fn joint_yield_corners() {
        let (specs, post) = one(Some(0.0), None, Some(1.0));
        let t = [table(0, gate_rows(&[1000.0]))];
        assert_eq!(linear_joint_yield(&t, &[Some(1e-3)], &post, &specs, &[], 0, 1), None, "no draws: no estimate");
        assert_eq!(linear_joint_yield(&t, &[None], &post, &specs, &[], 100, 1), None, "unknown σ");
        assert_eq!(linear_joint_yield(&[], &[Some(1e-3)], &post, &specs, &[], 100, 1), None, "no table");
        let (none_specs, none_post) = one(None, None, Some(1.0));
        assert_eq!(linear_joint_yield(&[], &[], &none_post, &none_specs, &[], 10, 1), Some(1.0), "no bound: always holds");
        let a = linear_joint_yield(&t, &[Some(1e-3)], &post, &specs, &[], 1000, 7);
        assert_eq!(a, linear_joint_yield(&t, &[Some(1e-3)], &post, &specs, &[], 1000, 7), "deterministic for a seed");
    }

    /// A ceiling at β = 1 with three devices (an odd count: the last
    /// Box–Muller pair is half used) still converges to Φ(1).
    #[test]
    fn joint_yield_of_a_ceiling_with_an_odd_device_count() {
        let (specs, post) = one(None, Some(3f64.sqrt()), Some(0.0));
        let t = [table(0, gate_rows(&[1000.0, 1000.0, 1000.0]))];
        let y = linear_joint_yield(&t, &[Some(1e-3); 3], &post, &specs, &[], 100_000, 3).unwrap();
        assert!((y - phi(1.0)).abs() < 0.005, "{y} vs {}", phi(1.0));
    }

    /// Two bounds on one perfectly correlated metric (floor and ceiling at
    /// ±1σ) hold together with probability Φ(1) − Φ(−1).
    #[test]
    fn joint_yield_of_a_window() {
        let (specs, post) = one(Some(-1.0), Some(1.0), Some(0.0));
        let t = [table(0, gate_rows(&[1000.0]))];
        let y = linear_joint_yield(&t, &[Some(1e-3)], &post, &specs, &[], 100_000, 5).unwrap();
        let want = phi(1.0) - phi(-1.0);
        assert!((y - want).abs() < 0.006, "{y} vs {want}");
    }

    #[test]
    fn phi_is_a_cdf() {
        // A–S 7.1.26 is good to 1.5e-7 on erfc, so Φ to 7.5e-8.
        assert!((phi(0.0) - 0.5).abs() < 1e-7);
        assert!(phi(-40.0) >= 0.0 && phi(-40.0) < 1e-12);
        assert!((phi(40.0) - 1.0).abs() < 1e-12);
        assert_eq!(phi(f64::INFINITY), 1.0);
        assert_eq!(phi(f64::NEG_INFINITY), 0.0);
        let xs: Vec<f64> = (-60..=60).map(|i| f64::from(i) / 10.0).collect();
        assert!(xs.windows(2).all(|w| phi(w[0]) <= phi(w[1])), "monotone");
        for &x in &xs {
            assert!((phi(x) + phi(-x) - 1.0).abs() < 1e-8, "symmetric at {x}");
        }
        // Tabulated Φ(1.96) = 0.975002.
        assert!((phi(1.96) - 0.975_002).abs() < 2e-7);
    }
}
