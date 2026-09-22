//! Pelgrom area matching — the **sizing** half of device matching.
//!
//! Random mismatch between two nominally identical devices splits into two terms
//! with two different owners:
//!
//! ```text
//! σ²(ΔP) = A_P²/(W·L)   +   S_P²·r²
//!          ╰─ area ─╯        ╰─ distance ─╯
//!          this file          placement (symmetry / common-centroid)
//! ```
//!
//! The area term depends only on drawn dimensions, so **no placement move can
//! change it** — a planar placer cannot make a transistor bigger. It is settled
//! here, when the generator picks a variant's finger and segment count, which is
//! why `MatchingPair` is a *Cost* rule in the placement tier rather than a Hard
//! one (see `backend/TODO.md` §2 and `annotator::emit`).
//!
//! Books: AOAL ch08/8.1, 8.2.7; ALS 2.2.2. See also DATE'21 (Sapatnekar) for the
//! two-term decomposition.

use pnr_core::Macro;

/// Threshold-voltage mismatch of a device of area `w_nm × l_nm`, in µV.
///
/// Pelgrom's law `σ(ΔVth) = A_Vth / √(W·L)`, with `A_Vth` in µV·µm and the
/// dimensions converted nm → µm. A degenerate (zero-area) device reports a huge
/// mismatch rather than dividing by zero — it is never acceptable.
#[must_use]
pub fn sigma_vth_uv(w_nm: i32, l_nm: i32, avt_uv_um: i32) -> f32 {
    let w_um = w_nm as f32 / 1000.0;
    let l_um = l_nm as f32 / 1000.0;
    let area_um2 = (w_um * l_um).max(1e-6);
    avt_uv_um as f32 / area_um2.sqrt()
}

/// Does a device of this area meet a `max_dvth_mv10` (mV×10) mismatch budget?
#[must_use]
pub fn area_meets_budget(w_nm: i32, l_nm: i32, avt_uv_um: i32, max_dvth_mv10: i32) -> bool {
    let budget_uv = max_dvth_mv10 as f32 * 100.0; // mV·10 → µV
    sigma_vth_uv(w_nm, l_nm, avt_uv_um) <= budget_uv
}

/// Smallest device area, µm², that would meet the budget — what the generator
/// must grow to. `σ = A/√area ≤ budget ⇒ area ≥ (A/budget)²`.
#[must_use]
pub fn required_area_um2(avt_uv_um: i32, max_dvth_mv10: i32) -> f32 {
    let budget_uv = (max_dvth_mv10 as f32 * 100.0).max(1e-6);
    let r = avt_uv_um as f32 / budget_uv;
    r * r
}

/// A drawn device too small for its matching budget.
#[derive(Debug, Clone, PartialEq)]
pub struct AreaShortfall {
    /// Index into the macro list (device index).
    pub device: usize,
    /// Achieved mismatch, µV.
    pub sigma_uv: f32,
    /// Budgeted mismatch, µV.
    pub budget_uv: f32,
    /// Area actually drawn, µm².
    pub area_um2: f32,
    /// Area needed to meet the budget, µm².
    pub required_um2: f32,
}

/// Audit drawn macros against a per-device matching budget.
///
/// `budget_mv10[i]` is device `i`'s `max_dvth_mv10` (`None` = unmatched, so no
/// budget). Returns one entry per device whose **drawn area** cannot meet its
/// budget no matter where it is placed.
///
/// This is a *report*, not a repair: choosing a larger variant is the generator's
/// job and the variant-reshape search is not wired yet (`cells::Cell::enumerate`
/// enumerates but nothing ranks). Surfacing the shortfall keeps the constraint
/// honest instead of silently unmet — the failure mode the thermal rule used to
/// have.
#[must_use]
pub fn audit(macros: &[Macro], budget_mv10: &[Option<i32>], avt_uv_um: &[i32]) -> Vec<AreaShortfall> {
    let mut out = Vec::new();
    for (i, m) in macros.iter().enumerate() {
        let Some(Some(budget)) = budget_mv10.get(i).copied() else {
            continue;
        };
        let avt = avt_uv_um.get(i).copied().unwrap_or(4000);
        let (w, h) = (m.bbox.w, m.bbox.h);
        if area_meets_budget(w, h, avt, budget) {
            continue;
        }
        let w_um = w as f32 / 1000.0;
        let h_um = h as f32 / 1000.0;
        out.push(AreaShortfall {
            device: i,
            sigma_uv: sigma_vth_uv(w, h, avt),
            budget_uv: budget as f32 * 100.0,
            area_um2: w_um * h_um,
            required_um2: required_area_um2(avt, budget),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // NMOS A_Vth ≈ 4000 µV·µm; a 1 mV (mv10 = 10) budget needs σ ≤ 1000 µV,
    // i.e. area ≥ (4000/1000)² = 16 µm².
    const AVT: i32 = 4000;
    const BUDGET_MV10: i32 = 10;

    #[test]
    fn pelgrom_area_threshold_is_the_square_of_the_ratio() {
        assert!((required_area_um2(AVT, BUDGET_MV10) - 16.0).abs() < 1e-3);
        // 4µm × 4µm = 16 µm² sits exactly on the budget.
        assert!(area_meets_budget(4_000, 4_000, AVT, BUDGET_MV10));
        // Half the area misses it.
        assert!(!area_meets_budget(2_000, 4_000, AVT, BUDGET_MV10));
    }

    #[test]
    fn bigger_devices_match_better() {
        let small = sigma_vth_uv(1_000, 1_000, AVT);
        let big = sigma_vth_uv(4_000, 4_000, AVT);
        assert!(big < small, "σ must fall as √area grows");
        // 4× the linear size ⇒ 4× the √area ⇒ ¼ the σ.
        assert!((small / big - 4.0).abs() < 1e-3);
    }

    #[test]
    fn audit_flags_only_budgeted_undersized_devices() {
        use pnr_core::geom::Rect;
        let mk = |w, h| Macro {
            bbox: Rect { x: 0, y: 0, w, h },
            shapes: vec![],
            pins: vec![],
        };
        let macros = vec![mk(1_000, 1_000), mk(4_000, 4_000), mk(1_000, 1_000)];
        // Device 2 is undersized but unmatched → no budget → not flagged.
        let budgets = vec![Some(BUDGET_MV10), Some(BUDGET_MV10), None];
        let avt = vec![AVT; 3];
        let found = audit(&macros, &budgets, &avt);
        assert_eq!(found.len(), 1, "only the budgeted undersized device: {found:?}");
        assert_eq!(found[0].device, 0);
        assert!(found[0].required_um2 > found[0].area_um2);
    }
}
