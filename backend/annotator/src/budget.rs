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
            let (nets, weights): (Vec<NetId>, Vec<f32>) = s.d_c.iter().filter(|e| keep(w(e.1))).map(|e| (e.0, w(e.1) as f32)).unzip();
            let (r_nets, r_weights): (Vec<NetId>, Vec<f32>) = s.d_r.iter().filter(|e| keep(w(e.1))).map(|e| (e.0, w(e.1) as f32)).unzip();
            let coupling = s.d_cc.iter().filter(|e| keep(w(e.2))).map(|e| (e.0, e.1, w(e.2) as f32)).collect();
            for e in s.d_c.iter().filter(|e| keep(w(e.1))) {
                class.entry(e.0 .0).or_default().1 |= w(e.1).abs() * c_ref >= policy.rc_share;
            }
            if let Some(r_ref) = r_ref {
                for e in s.d_r.iter().filter(|e| keep(w(e.1))) {
                    class.entry(e.0 .0).or_default().0 |= w(e.1).abs() * r_ref >= policy.rc_share;
                }
            }
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
