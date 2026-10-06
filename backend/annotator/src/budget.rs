//! EXT-25: parasitic budgets from spec sensitivities. One
//! [`PerformanceBudget`] row per finite spec bound ([`crate::allocate::margins`],
//! so the process spread and the `β·σ_f` reserve come off the headroom first):
//! `w = sign·(∂f/∂x)/scale` for ground C (`d_c`), series R (`d_r`) and coupling
//! (`d_cc`), `sign` −1 for a floor and +1 for a ceiling. A bound with margin has
//! `limit 1` and `scale` = margin; one without keeps a do-not-worsen row
//! (`limit 0`, `scale = |bound|`, 1 for a zero bound; GRAEB-11). Helpful terms
//! (`w ≤ 0`) are dropped unless `Policy.credit_helpful` (BAL2-16: the linear
//! model's credit is not trusted by default).

use std::collections::BTreeMap;

use analog::intent::{Diagnostic, RcClass};
use analog::routing::PerformanceBudget;
use pnr_core::ids::NetId;

use crate::allocate::margins;
use crate::evidence::Sensitivities;
use crate::policy::Policy;

/// Rows, the R/C class of every net a kept term assessed, and a
/// `no_layout_margin` diagnostic per bound the schematic already misses. A net
/// is R (C) when `|w_r|·R_ref` (`|w_c|·C_ref`) reaches `rc_share` in some row,
/// `R_ref`/`C_ref` the resistance/ground C of `rc_ref_len_um` of minimum
/// lowest-layer wire; without `r_ohm_per_um` the R test is skipped and a net
/// seen only in `d_r` is not classed (stays Unknown).
///
/// Rows follow `sens.specs` order (floor before ceiling); classes are sorted by
/// net id. `af_per_nm` is the minimum lowest-layer wire's ground C, aF/nm.
#[must_use]
pub fn rows(sens: &Sensitivities, af_per_nm: f32, r_ohm_per_um: Option<f32>, policy: &Policy) -> (Vec<PerformanceBudget>, Vec<(NetId, RcClass)>, Vec<Diagnostic>) {
    let c_ref = f64::from(af_per_nm) * 1000.0 * policy.rc_ref_len_um;
    let r_ref = r_ohm_per_um.map(|r| f64::from(r) * policy.rc_ref_len_um);
    let keep = |w: f64| w > 0.0 || (policy.credit_helpful && w != 0.0);
    let (mut out, mut diags) = (Vec::new(), Vec::new());
    // net → (R, C) reached rc_share.
    let mut class: BTreeMap<u16, (bool, bool)> = BTreeMap::new();
    for s in &sens.specs {
        for (sign, m, bound) in margins(s, policy.beta_target) {
            let side = if sign < 0.0 { "min" } else { "max" };
            let metric = format!("{}:{side}", s.metric);
            let (scale, limit) = if m > 0.0 {
                (m, 1.0)
            } else {
                diags.push(Diagnostic { kind: "no_layout_margin", devices: vec![], message: format!("{metric}: margin {m} ≤ 0, do-not-worsen row") });
                (if bound == 0.0 { 1.0 } else { bound.abs() }, 0.0)
            };
            let w = |d: f64| sign * d / scale;
            // One pass per term list: weigh, keep, class.
            let (mut nets, mut weights) = (Vec::new(), Vec::new());
            for &(n, d) in &s.d_c {
                let w = w(d);
                if keep(w) {
                    nets.push(n);
                    weights.push(w as f32);
                    class.entry(n.0).or_default().1 |= w.abs() * c_ref >= policy.rc_share;
                }
            }
            let (mut r_nets, mut r_weights) = (Vec::new(), Vec::new());
            for &(n, d) in &s.d_r {
                let w = w(d);
                if keep(w) {
                    r_nets.push(n);
                    r_weights.push(w as f32);
                    if let Some(r_ref) = r_ref {
                        class.entry(n.0).or_default().0 |= w.abs() * r_ref >= policy.rc_share;
                    }
                }
            }
            let coupling = s.d_cc.iter().map(|&(x, y, d)| (x, y, w(d))).filter(|e| keep(e.2)).map(|(x, y, w)| (x, y, w as f32)).collect();
            out.push(PerformanceBudget { limit, r_nets, r_weights, coupling, ..PerformanceBudget::ground_c(metric, nets, weights, af_per_nm) });
        }
    }
    let rc = class
        .into_iter()
        .map(|(n, rc)| {
            let c = match rc {
                (true, true) => RcClass::Rc,
                (true, false) => RcClass::R,
                (false, true) => RcClass::C,
                (false, false) => RcClass::None,
            };
            (NetId(n), c)
        })
        .collect();
    (out, rc, diags)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::SpecSens;

    fn spec(lo: f64, hi: f64, f0: f64) -> SpecSens {
        SpecSens { metric: "ugf".into(), f0, lo: Some(lo), hi: Some(hi), proc: None, sigma_f: None, d_c: vec![], d_r: vec![], d_vt: vec![], d_t: vec![], d_cc: vec![] }
    }

    fn run(s: SpecSens, policy: &Policy) -> (Vec<PerformanceBudget>, Vec<(NetId, RcClass)>, Vec<Diagnostic>) {
        rows(&Sensitivities { specs: vec![s] }, 1.0, None, policy)
    }

    #[test]
    fn two_sided_spec_gives_two_rows() {
        let s = SpecSens { d_c: vec![(NetId(0), -1e6)], ..spec(100e6, 300e6, 200e6) };
        let (r, _, _) = run(s, &Policy::default());
        assert_eq!(r.iter().map(|r| r.metric.as_str()).collect::<Vec<_>>(), ["ugf:min", "ugf:max"]);
        assert_eq!((r[0].nets.as_slice(), r[0].weights.as_slice()), (&[NetId(0)][..], &[0.01][..]));
        assert!(r[1].nets.is_empty(), "the ceiling's term helps");
    }

    #[test]
    fn process_spread_shrinks_headroom() {
        let s = SpecSens { proc: Some((180.0, 230.0)), d_c: vec![(NetId(0), -1.0), (NetId(1), 1.0)], ..spec(100.0, 300.0, 200.0) };
        let (r, _, _) = run(s, &Policy::default());
        assert_eq!((r[0].nets[0], r[1].nets[0]), (NetId(0), NetId(1)));
        assert!((r[0].weights[0] - 1.0 / 80.0).abs() < 1e-7 && (r[1].weights[0] - 1.0 / 70.0).abs() < 1e-7, "{:?} {:?}", r[0].weights, r[1].weights);
    }

    #[test]
    fn beta_reserve() {
        let s = SpecSens { sigma_f: Some(10.0), ..spec(100.0, 300.0, 200.0) };
        let m: Vec<f64> = margins(&s, Policy::default().beta_target).iter().map(|r| r.1).collect();
        assert_eq!(m, [70.0, 70.0]);
    }

    #[test]
    fn conservative_split_drops_helpful_terms() {
        let s = SpecSens { d_c: vec![(NetId(0), 0.5), (NetId(1), -0.5)], hi: None, ..spec(10.0, 0.0, 20.0) };
        let (r, _, _) = run(s, &Policy::default());
        assert_eq!((r.len(), r[0].nets.as_slice()), (1, &[NetId(1)][..]));
        assert!((r[0].weights[0] - 0.05).abs() < 1e-7);
    }

    /// Ported from `library::perf` (PERF-06).
    #[test]
    fn rows_cover_both_bounds_of_a_window_spec() {
        let s = SpecSens { d_c: vec![(NetId(0), -1.0)], ..spec(10.0, 20.0, 15.0) };
        let credit = Policy { credit_helpful: true, ..Policy::default() };
        let (r, _, _) = run(s.clone(), &credit);
        assert_eq!(r.iter().map(|r| (r.metric.as_str(), r.weights.clone(), r.limit)).collect::<Vec<_>>(), [("ugf:min", vec![0.2], 1.0), ("ugf:max", vec![-0.2], 1.0)]);
        let (r, _, _) = run(s, &Policy::default());
        assert_eq!((r[0].nets.as_slice(), r[1].nets.len()), (&[NetId(0)][..], 0));
    }

    /// Ported from `library::perf` (PERF-06).
    #[test]
    fn a_missed_bound_keeps_a_do_not_worsen_row() {
        let s = SpecSens { d_c: vec![(NetId(0), -1.0)], hi: None, ..spec(10.0, 0.0, 5.0) };
        let (r, _, d) = run(s, &Policy::default());
        assert_eq!((r.len(), r[0].limit, r[0].weights.as_slice()), (1, 0.0, &[0.1][..]));
        assert_eq!(d.iter().filter(|d| d.kind == "no_layout_margin").count(), 1);
    }

    /// Ported from `library::perf` (PERF-06).
    #[test]
    fn a_non_finite_bound_or_value_has_no_row() {
        let d_c = vec![(NetId(0), -1.0)];
        for s in [spec(f64::NAN, f64::INFINITY, 15.0), spec(10.0, 20.0, f64::NAN)] {
            assert!(run(SpecSens { d_c: d_c.clone(), ..s }, &Policy::default()).0.is_empty());
        }
    }

    #[test]
    fn r_and_coupling_terms() {
        let s = SpecSens { d_r: vec![(NetId(0), -0.2)], d_cc: vec![(NetId(0), NetId(1), -0.1)], hi: None, ..spec(10.0, 0.0, 20.0) };
        let (r, _, _) = run(s, &Policy::default());
        assert_eq!((r[0].r_nets.as_slice(), r[0].r_weights.as_slice()), (&[NetId(0)][..], &[0.02][..]));
        assert_eq!(r[0].coupling, [(NetId(0), NetId(1), 0.01)]);
        assert!(r[0].diff_pairs.is_empty());
    }

    #[test]
    fn rc_class_by_share() {
        let s = SpecSens {
            d_c: vec![(NetId(0), -1e-3), (NetId(2), -1e-3), (NetId(3), -1e-4)],
            d_r: vec![(NetId(1), -0.2), (NetId(2), -0.2)],
            hi: None,
            ..spec(0.0, 0.0, 100.0)
        };
        let (_, rc, _) = rows(&Sensitivities { specs: vec![s] }, 0.05, Some(0.5), &Policy::default());
        assert_eq!(rc, [(NetId(0), RcClass::C), (NetId(1), RcClass::R), (NetId(2), RcClass::Rc), (NetId(3), RcClass::None)]);
    }
}

/// Step-2 coverage: empty input, zero bound, zero weight, R/C class gating.
#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use crate::evidence::SpecSens;

    fn floor(lo: f64, f0: f64) -> SpecSens {
        SpecSens { metric: "gain".into(), f0, lo: Some(lo), hi: None, proc: None, sigma_f: None, d_c: vec![], d_r: vec![], d_vt: vec![], d_t: vec![], d_cc: vec![] }
    }

    #[test]
    fn no_specs_no_rows() {
        let (r, c, d) = rows(&Sensitivities { specs: vec![] }, 1.0, Some(1.0), &Policy::default());
        assert!(r.is_empty() && c.is_empty() && d.is_empty());
    }

    #[test]
    fn a_spec_without_bounds_has_no_row() {
        let s = SpecSens { lo: None, d_c: vec![(NetId(0), -1.0)], ..floor(0.0, 1.0) };
        assert!(rows(&Sensitivities { specs: vec![s] }, 1.0, None, &Policy::default()).0.is_empty());
    }

    /// A missed zero bound scales by 1, not by `|0|`.
    #[test]
    fn a_missed_zero_bound_scales_by_one() {
        let s = SpecSens { d_c: vec![(NetId(3), -2.0)], ..floor(0.0, -5.0) };
        let (r, _, d) = rows(&Sensitivities { specs: vec![s] }, 0.5, None, &Policy::default());
        assert_eq!((r.len(), r[0].limit, r[0].weights.as_slice(), r[0].af_per_nm), (1, 0.0, &[2.0][..], 0.5));
        assert_eq!(d.len(), 1);
        assert!(d[0].message.starts_with("gain:min"), "{}", d[0].message);
        assert!(d[0].devices.is_empty());
    }

    /// Margin exactly 0 is a miss: no headroom to spend.
    #[test]
    fn a_zero_margin_is_a_do_not_worsen_row() {
        let s = SpecSens { d_c: vec![(NetId(0), -1.0)], ..floor(4.0, 4.0) };
        let (r, _, d) = rows(&Sensitivities { specs: vec![s] }, 1.0, None, &Policy::default());
        assert_eq!((r[0].limit, r[0].weights.as_slice(), d.len()), (0.0, &[0.25][..], 1));
    }

    /// A zero sensitivity is neither harmful nor helpful: dropped even with credit.
    #[test]
    fn a_zero_sensitivity_is_never_kept() {
        let s = SpecSens { d_c: vec![(NetId(0), 0.0)], d_r: vec![(NetId(1), 0.0)], d_cc: vec![(NetId(0), NetId(1), 0.0)], ..floor(10.0, 20.0) };
        let credit = Policy { credit_helpful: true, ..Policy::default() };
        let (r, c, _) = rows(&Sensitivities { specs: vec![s] }, 1.0, Some(1.0), &credit);
        assert!(r[0].nets.is_empty() && r[0].r_nets.is_empty() && r[0].coupling.is_empty());
        assert!(c.is_empty(), "{c:?}");
    }

    #[test]
    fn helpful_coupling_is_dropped_by_default() {
        let s = SpecSens { d_cc: vec![(NetId(0), NetId(1), 0.5)], ..floor(10.0, 20.0) };
        let (r, _, _) = rows(&Sensitivities { specs: vec![s] }, 1.0, None, &Policy::default());
        assert!(r[0].coupling.is_empty());
    }

    /// Without `r_ohm_per_um` a net seen only in `d_r` gets no class; one in
    /// `d_c` is still classed by C alone.
    #[test]
    fn r_class_needs_a_sheet_resistance() {
        let s = SpecSens { d_c: vec![(NetId(2), -10.0)], d_r: vec![(NetId(1), -10.0), (NetId(2), -10.0)], ..floor(0.0, 1.0) };
        let (r, c, _) = rows(&Sensitivities { specs: vec![s] }, 1.0, None, &Policy::default());
        assert_eq!(r[0].r_nets, [NetId(1), NetId(2)], "the row still prices R");
        assert_eq!(c, [(NetId(2), RcClass::C)]);
    }

    /// A net's class ORs over every row it appears in, and helpful terms that
    /// are dropped do not class it.
    #[test]
    fn class_accumulates_over_rows_of_kept_terms() {
        // Two specs: net 0 reaches the C share in the first, the R share in the second.
        let a = SpecSens { d_c: vec![(NetId(0), -1.0)], ..floor(0.0, 1.0) };
        let b = SpecSens { metric: "pm".into(), d_r: vec![(NetId(0), -1.0)], d_c: vec![(NetId(5), 1.0)], ..floor(0.0, 1.0) };
        let (r, c, _) = rows(&Sensitivities { specs: vec![a, b] }, 1.0, Some(1.0), &Policy::default());
        assert_eq!(r.iter().map(|r| r.metric.as_str()).collect::<Vec<_>>(), ["gain:min", "pm:min"]);
        assert_eq!(c, [(NetId(0), RcClass::Rc)], "net 5's term helps and is dropped");
    }

    /// The share test is `≥`: a term exactly at `rc_share` classes the net.
    #[test]
    fn class_share_boundary_is_inclusive() {
        let p = Policy { rc_ref_len_um: 2.0, rc_share: 250.0, ..Policy::default() };
        // C_ref = 0.5 aF/nm · 2000 nm = 1000 aF; margin 1, so |w|·C_ref is 250 and 125 (exact).
        let s = SpecSens { d_c: vec![(NetId(0), -0.25), (NetId(1), -0.125)], ..floor(0.0, 1.0) };
        let (_, c, _) = rows(&Sensitivities { specs: vec![s] }, 0.5, None, &p);
        assert_eq!(c, [(NetId(0), RcClass::C), (NetId(1), RcClass::None)]);
    }
}
