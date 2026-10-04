//! EXT-21: one systematic-offset budget per spec split over the matched sets by
//! sensitivity (LAMP-09, lampaert.txt eqs. 4.25–4.27, L4718–4765). Per margin
//! row j (one per finite spec bound) and set k: `S_jk` = the set's ΔV_T
//! sensitivity, `K_j` = #sets with `S_jk > 0`, `δ_k = min_j M_j/(K_j·S_jk)`, so
//! `Σ_k S_jk·δ_k ≤ M_j` for every row (each δ_k is at most its share of every
//! row touching it). Allowances are 1σ ΔV_T, mV: a Current FET set's ledger
//! converts them to % itself (`MatchedSet::budget_in`).

use analog::intent::{Diagnostic, Half, MatchSpec};
use analog::matching::class::Family;
use analog::matching::mismatch::sigma_pair;
use pnr_core::ids::DeviceId;
use pnr_core::Netlist;

use crate::evidence::{Sensitivities, SpecSens};

/// One allocatable set: compared sides (device lists) and its random 1σ, mV.
pub struct SetIn {
    pub sides: Vec<(Vec<DeviceId>, Vec<DeviceId>)>,
    pub sigma_mv: Option<f32>,
}

/// `(sign, margin, bound)` per finite bound of `s`: sign −1 floor, +1 ceiling.
/// Headroom is `(proc.0 | f0) − lo` / `hi − (proc.1 | f0)` (LAMP-01, the process
/// spread eats it first) less the `β·σ_f` yield reserve (GRAEB-16; no reserve
/// without `σ_f`). A non-finite bound or `f0` gives no row. Shared with EXT-25.
pub(crate) fn margins(s: &SpecSens, beta: f64) -> Vec<(f64, f64, f64)> {
    if !s.f0.is_finite() {
        return Vec::new();
    }
    let (plo, phi) = s.proc.unwrap_or((s.f0, s.f0));
    let reserve = s.sigma_f.map_or(0.0, |sf| beta * sf);
    let floor = s.lo.filter(|v| v.is_finite()).map(|lo| (-1.0, plo - lo - reserve, lo));
    let ceil = s.hi.filter(|v| v.is_finite()).map(|hi| (1.0, hi - phi - reserve, hi));
    floor.into_iter().chain(ceil).collect()
}

/// LAMP-09 split. Returns per set `(allowance mV, weight)`: the allowance is
/// `min_j M_j/(K_j·S_jk)` capped at `max_eta·σ_k` (uncapped without σ_k), or
/// `max_eta·σ_k` when no spec touches the set (`None` without σ_k); a row with
/// `M_j ≤ 0` gives its sets 0 and one `spec_infeasible_at_schematic` (LAMP-49).
/// The weight is the set's largest share of a spec's variance,
/// `max_j (S_jk·σ_k)²/σ_f,j²` clamped to 1 (GRAEB-06), `None` unless σ_k and
/// some σ_f are known.
#[must_use]
pub fn allocate(sets: &[SetIn], sens: &Sensitivities, beta: f64, max_eta: f32) -> (Vec<(Option<f32>, Option<f32>)>, Vec<Diagnostic>) {
    let n = sets.len();
    let mut delta: Vec<Option<f64>> = vec![None; n];
    let mut weight: Vec<Option<f64>> = vec![None; n];
    let mut diags = Vec::new();
    for s in &sens.specs {
        let side = |ds: &[DeviceId]| ds.iter().map(|&d| s.d_vt.iter().filter(|e| e.0 == d).map(|e| e.1).sum::<f64>()).sum::<f64>().abs();
        let sk: Vec<f64> = sets.iter().map(|k| k.sides.iter().map(|(a, b)| (side(a) + side(b)) / 2.0).fold(0.0, f64::max)).collect();
        let touched: Vec<usize> = (0..n).filter(|&k| sk[k] > 0.0).collect();
        if touched.is_empty() {
            continue;
        }
        for (sign, m, _) in margins(s, beta) {
            if m <= 0.0 {
                let mut devices: Vec<DeviceId> = touched.iter().flat_map(|&k| sets[k].sides.iter().flat_map(|(a, b)| a.iter().chain(b))).copied().collect();
                devices.sort_unstable_by_key(|d| d.0);
                devices.dedup();
                let bound = if sign < 0.0 { "min" } else { "max" };
                diags.push(Diagnostic { kind: "spec_infeasible_at_schematic", devices, message: format!("{}:{bound} margin {m} ≤ 0", s.metric) });
            }
            for &k in &touched {
                let term = m.max(0.0) / (touched.len() as f64 * sk[k]);
                delta[k] = Some(delta[k].map_or(term, |d| d.min(term)));
            }
        }
        if let Some(sf) = s.sigma_f.filter(|&v| v > 0.0) {
            for (k, set) in sets.iter().enumerate() {
                if let Some(sig) = set.sigma_mv {
                    let w = (sk[k] * f64::from(sig) / sf).powi(2);
                    weight[k] = Some(weight[k].map_or(w, |v| v.max(w)));
                }
            }
        }
    }
    let out = sets
        .iter()
        .zip(delta.iter().zip(&weight))
        .map(|(set, (d, w))| {
            let cap = set.sigma_mv.map(|s| f64::from(max_eta * s));
            let a = match (*d, cap) {
                (Some(d), Some(c)) => Some(d.min(c)),
                (d, c) => d.or(c),
            };
            (a.map(|v| v as f32), w.map(|v| v.min(1.0) as f32))
        })
        .collect();
    (out, diags)
}

/// [`SetIn`] of a MOS set (`None` otherwise): half A against half B when the set
/// has halves, else the reference (slot `reference`, default 0) against each
/// other member. σ_k is the largest Pelgrom pair σ over those comparisons at the
/// summed gate areas (`None` without the deck's A_VT or a gate area).
pub(crate) fn set_in(s: &MatchSpec, nl: &Netlist, avt: [Option<f32>; 2]) -> Option<SetIn> {
    if s.family != Family::Mos || s.members.is_empty() {
        return None;
    }
    let half = |h: Half| s.members.iter().filter(|m| m.half == Some(h)).map(|m| m.device).collect::<Vec<_>>();
    let sides = if s.members.iter().any(|m| m.half.is_some()) {
        vec![(half(Half::A), half(Half::B))]
    } else {
        let r = s.reference.unwrap_or(0);
        s.members.iter().enumerate().filter(|&(i, _)| i != r).map(|(_, m)| (vec![s.members[r].device], vec![m.device])).collect()
    };
    let area = |ds: &[DeviceId]| ds.iter().map(|d| nl.devices[d.0 as usize].gate_area_um2()).sum::<f64>() as f32;
    let sigma_mv = crate::emit::by_polarity(nl, s.members[0].device, avt)
        .map(|a| sides.iter().map(|(x, y)| sigma_pair(a, area(x), area(y))).fold(0.0, f32::max))
        .filter(|&v| v > 0.0);
    Some(SetIn { sides, sigma_mv })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(f0: f64, lo: Option<f64>, hi: Option<f64>, sigma_f: Option<f64>, d_vt: Vec<(DeviceId, f64)>) -> SpecSens {
        SpecSens { metric: "m".into(), f0, lo, hi, proc: None, sigma_f, d_c: vec![], d_r: vec![], d_vt, d_t: vec![], d_cc: vec![] }
    }

    /// `n` one-pair sets, set k = devices (2k, 2k+1), σ 1 mV.
    fn pairs(n: u16) -> Vec<SetIn> {
        (0..n).map(|k| SetIn { sides: vec![(vec![DeviceId(2 * k)], vec![DeviceId(2 * k + 1)])], sigma_mv: Some(1.0) }).collect()
    }

    /// `(+s, −s)` on set k for each `s`.
    fn dvt(s: &[f64]) -> Vec<(DeviceId, f64)> {
        s.iter().enumerate().flat_map(|(k, &v)| [(DeviceId(2 * k as u16), v), (DeviceId(2 * k as u16 + 1), -v)]).collect()
    }

    #[test]
    fn allocation_respects_every_spec() {
        let (sa, sb) = ([2.0, 0.5, 0.0], [1.0, 3.0, 0.2]);
        let sens = Sensitivities { specs: vec![spec(0.0, None, Some(10.0), None, dvt(&sa)), spec(20.0, Some(12.0), None, Some(1.0), dvt(&sb))] };
        let (out, diags) = allocate(&pairs(3), &sens, 3.0, 1e6);
        assert!(diags.is_empty());
        let d: Vec<f64> = out.iter().map(|o| f64::from(o.0.unwrap())).collect();
        for (s, m) in [(sa, 10.0), (sb, 5.0)] {
            let used: f64 = s.iter().zip(&d).map(|(s, d)| s * d).sum();
            assert!(used <= m + 1e-5, "{used} > {m}");
        }
        assert!((d[2] - 5.0 / (3.0 * 0.2)).abs() < 1e-4, "set 3 takes B's term: {}", d[2]);
    }

    /// lampaert Table 4.2's first four sensitivities as relative weights.
    #[test]
    fn lampaert_table_4_2_ordering() {
        let s = [12.0, 2.9, 2.9, 0.2];
        let sens = Sensitivities { specs: vec![spec(20.0, Some(10.0), None, None, dvt(&s))] };
        let (out, _) = allocate(&pairs(4), &sens, 3.0, 1e9);
        let d: Vec<f64> = out.iter().map(|o| f64::from(o.0.unwrap())).collect();
        assert!(d[3] > d[1] && d[1] == d[2] && d[2] > d[0], "{d:?}");
        let used: f64 = s.iter().zip(&d).map(|(s, d)| s * d).sum();
        assert!((used - 10.0).abs() < 1e-5, "{used}");
    }

    #[test]
    fn negative_margin_is_diagnosed() {
        let sens = Sensitivities { specs: vec![spec(9.0, Some(10.0), None, None, dvt(&[1.0, 2.0, 0.0]))] };
        let mut sets = pairs(3);
        sets[2].sigma_mv = Some(2.0);
        let (out, diags) = allocate(&sets, &sens, 3.0, 3.0);
        assert_eq!((out[0].0, out[1].0, out[2].0), (Some(0.0), Some(0.0), Some(6.0)));
        assert_eq!(diags.iter().filter(|d| d.kind == "spec_infeasible_at_schematic").count(), 1);
    }

    #[test]
    fn two_sided_spec_takes_the_tighter_bound() {
        let mut s = spec(8.0, Some(0.0), Some(10.0), None, dvt(&[1.0]));
        s.proc = Some((7.0, 9.0));
        let m: Vec<f64> = margins(&s, 3.0).iter().map(|r| r.1).collect();
        assert_eq!(m, [7.0, 1.0]);
        let mut sets = pairs(1);
        sets[0].sigma_mv = None;
        let (out, _) = allocate(&sets, &Sensitivities { specs: vec![s] }, 3.0, 3.0);
        assert_eq!(out[0].0, Some(1.0));
    }

    #[test]
    fn weight_is_variance_share() {
        let sens = Sensitivities { specs: vec![spec(20.0, Some(0.0), None, Some(2.0), dvt(&[1.0]))] };
        let (out, _) = allocate(&pairs(1), &sens, 3.0, 3.0);
        assert_eq!(out[0].1, Some(0.25));
    }
}
