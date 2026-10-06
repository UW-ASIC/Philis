//! Sizing-reach diagnostics (GAP-02): where a matched pair's sizing, not its
//! layout, decides the mismatch. Reported, never charged to Θ.

use pnr_core::layout::Layout;

use crate::matching::mismatch::{Budget, MatchKind};
use crate::placement::matched_set::MatchedSet;

/// One sizing-reach diagnostic of pair `members` (schematic ids, reference first). Reported, never charged to Θ.
///
/// `kind` / units of `have`, `need`: `"area"` and `"budget_area"` µm² (the
/// smaller member's area vs the area that meets the limit), `"dstar"` µm (the
/// placed centroid distance vs Pelgrom's D*). `lever` names what to change.
#[derive(Clone, Debug, PartialEq)]
pub struct SizingNote {
    /// Schematic ids `(reference, member)`.
    pub members: (u32, u32),
    /// `"area"`, `"budget_area"` or `"dstar"`.
    pub kind: &'static str,
    /// What the layout has, in `kind`'s unit.
    pub have: f32,
    /// What meets the limit, in `kind`'s unit.
    pub need: f32,
    /// What to change.
    pub lever: &'static str,
}

/// Hastings Table 13.4 area for a 6σ offset limit `dv_mv`: `(6·A_VT/ΔV)²`, µm² (H13-43, L42369–42407).
/// Infinite at `dv_mv = 0`.
#[must_use]
pub fn area_need_um2(avt_mv_um: f32, dv_mv: f32) -> f32 {
    (6.0 * avt_mv_um / dv_mv).powi(2)
}

/// Pelgrom D* = A_VT/(S_VT·√(WL)), µm (MM-06, pelgrom.txt L88–90); A mV·µm, S µV/µm → ×1000.
/// The distance beyond which the gradient term outgrows the random one; infinite at a zero `S_VT` or area.
#[must_use]
pub fn dstar_um(avt_mv_um: f32, svt_uv_per_um: f32, wl_um2: f32) -> f32 {
    1000.0 * avt_mv_um / (svt_uv_per_um * wl_um2.sqrt())
}

/// Notes of every pair `(0, i)` of `set` on `l`, pairs in member order; `class_limit_mv` = a user/spec class's 6σ mV
/// limit (Voltage kind only). Empty for a set of fewer than two members.
///
/// - `"area"`: the smaller area is under [`area_need_um2`] of the class limit.
/// - `"budget_area"`: a `Sigma1Mv(b)` budget (`Sigma1Pct` on a % ledger) below σ_rand; `need = have·(σ_rand/b)²`
///   (MM-14); lever `"area"` for a voltage pair, `"length"` for a mirror (MM-21).
/// - `"dstar"`: units drawn and A_VT, S_VT known; `"distance term non-binding"`
///   when the placed distance is under a tenth of D*.
#[must_use]
pub fn notes(set: &MatchedSet, l: &Layout, class_limit_mv: Option<f32>) -> Vec<SizingNote> {
    let mut out = Vec::new();
    let avt = set.coeffs.avt_mv_um;
    for i in 1..set.members.len() {
        let g = set.ledger(l, i);
        let (a0, ai) = set.pair_areas(l, i);
        let have_a = a0.min(ai);
        let members = (u32::from(set.members[0].0), u32::from(set.members[i].0));
        let mut note = |kind, have, need, lever| out.push(SizingNote { members, kind, have, need, lever });
        if let (MatchKind::Voltage, Some(dv), Some(av)) = (set.kind, class_limit_mv, avt) {
            let need = area_need_um2(av, dv);
            if have_a < need {
                note("area", have_a, need, "area");
            }
        }
        if let Budget::Sigma1Mv(b) | Budget::Sigma1Pct(b) = set.budget_in(g.unit) {
            if b > 0.0 && g.sigma_rand > b {
                let lever = if set.kind == MatchKind::Current { "length" } else { "area" };
                note("budget_area", have_a, have_a * (g.sigma_rand / b).powi(2), lever);
            }
        }
        if let (Some(av), Some(sv), true) = (avt, set.coeffs.svt_uv_per_um, g.known) {
            let (have, need) = (g.delta_m_nm / 1000.0, dstar_um(av, sv, have_a));
            // ponytail: 0.1 is policy; the source gives only "ignorable below 100 µm²".
            let lever = if have < 0.1 * need { "distance term non-binding" } else { "distance" };
            note("dstar", have, need, lever);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::placement::matched_set::{layout, pair, singles};

    /// Voltage pair 0/1, A_VT 9.5, 20 µm² each from the netlist (no units):
    /// σ_rand 2.124 mV over a 1 mV 1σ budget.
    fn voltage() -> MatchedSet {
        MatchedSet { kind: MatchKind::Voltage, budget: Budget::Sigma1Mv(1.0), ..pair(0, 1) }
    }

    #[test]
    fn table_13_4_5v_exc() {
        assert!((area_need_um2(12.5, 1.0) - 5625.0).abs() < 1e-2);
    }

    #[test]
    fn dstar_sky130() {
        assert!((dstar_um(9.5, 1.63, 100.0) - 583.0).abs() < 1.0);
    }

    #[test]
    fn budget_below_random_gives_multiplier() {
        let l = layout(&[0, 5_000], &[0, 0], 100);
        let n = notes(&voltage(), &l, None);
        assert_eq!(n.len(), 1, "{n:?}");
        assert_eq!((n[0].members, n[0].kind, n[0].lever), ((0, 1), "budget_area", "area"));
        assert!((n[0].need / n[0].have - 4.51).abs() < 0.01, "{n:?}");
    }

    #[test]
    fn mirror_names_length() {
        let l = layout(&[0, 5_000], &[0, 0], 100);
        let n = notes(&MatchedSet { kind: MatchKind::Current, ..voltage() }, &l, None);
        assert_eq!((n.len(), n[0].kind, n[0].lever), (1, "budget_area", "length"));
    }

    /// Units drawn, 20 µm² each: D* = 1303 µm; 1 µm apart is non-binding,
    /// 200 µm apart is not.
    #[test]
    fn dstar_names_the_distance_lever() {
        for (dx, lever) in [(1_000, "distance term non-binding"), (200_000, "distance")] {
            let l = singles(dx);
            let s = pair(0, 1);
            let g = s.ledger(&l, 1);
            let n = notes(&s, &l, None);
            assert_eq!(n.len(), 1, "{n:?}");
            assert_eq!((n[0].kind, n[0].lever), ("dstar", lever), "{n:?}");
            assert_eq!(n[0].have, g.delta_m_nm / 1000.0);
            assert!((g.delta_m_nm - dx as f32).abs() < 1.0, "{}", g.delta_m_nm);
            assert_eq!(n[0].need, dstar_um(9.5, 1.63, 20.0));
        }
    }

    #[test]
    fn single_member_set_has_no_pair_to_note() {
        let l = layout(&[0, 5_000], &[0, 0], 100);
        let one = MatchedSet { members: vec![pnr_core::ids::DeviceId(0)], ..voltage() };
        assert!(notes(&one, &l, Some(3.0)).is_empty());
    }

    #[test]
    fn no_area_note_without_avt() {
        let l = layout(&[0, 5_000], &[0, 0], 100);
        let mut s = voltage();
        s.coeffs.avt_mv_um = None;
        assert!(notes(&s, &l, Some(3.0)).iter().all(|n| n.kind != "area" && n.kind != "dstar"));
    }

    #[test]
    fn area_note_only_with_class_limit() {
        let l = layout(&[0, 5_000], &[0, 0], 100);
        assert!(notes(&voltage(), &l, None).iter().all(|n| n.kind != "area"));
        let n: Vec<_> = notes(&voltage(), &l, Some(3.0)).into_iter().filter(|n| n.kind == "area").collect();
        assert_eq!(n.len(), 1);
        assert!((n[0].need - 361.0).abs() < 0.1 && n[0].have == 20.0, "{n:?}");
    }
}
