//! Steady-state die temperature by superposition of per-device point sources,
//! `ΔT(r) = P / (2π·k·r)` (semi-infinite substrate), with `r` floored at the
//! source's own half-extent. Linear in power, so the sum is exact for the
//! per-source model. Global in the power map: refresh per epoch, never per move.

use crate::layout::Layout;

/// Bulk Si thermal conductivity, W/(m·K), ~300 K.
///
/// ponytail: one bulk constant, no BEOL/package θ_JA. Gradients between nearby
/// matched devices (what the rule scores) are far more robust than absolute
/// rises; read `k` from the PDK when it grows a thermal section.
pub const K_SI_W_PER_M_K: f32 = 148.0;

/// `ΔT[mK] = P[µW]·1e6 / (2π·k·r[nm])`.
const SCALE_UW_NM_TO_MK: f32 = 1.0e6;

/// Hastings eq. 5.6: the rise of a uniform W×L source over its own area,
/// `ln(4L/W)·P/(π·k·L)` (L = the longer side), mK. Sides floored at 1 nm.
#[must_use]
pub fn self_rise_mc(p_uw: i32, w_nm: i32, l_nm: i32, k_w_per_m_k: f32) -> f32 {
    let (w, l) = (w_nm.min(l_nm).max(1) as f32, w_nm.max(l_nm).max(1) as f32);
    (4.0 * l / w).ln() * p_uw as f32 * SCALE_UW_NM_TO_MK / (std::f32::consts::PI * k_w_per_m_k * l)
}

/// `Σ_j self_rise_mc(p_j, footprint_j)`, mK: no device rises more under
/// [`rises_mc`], wherever the devices are placed. Each mutual term
/// `P/(2πk·r)` with `r ≥ L_j/2` is `≤ P/(πk·L_j)`, below eq. 5.6's
/// `ln(4L/W)·P/(πk·L_j) ≥ ln4·P/(πk·L_j)`.
#[must_use]
pub fn rise_bound_mc(p_uw: &[i32], footprint_nm: &[(i32, i32)], k_w_per_m_k: f32) -> f32 {
    p_uw.iter().zip(footprint_nm).map(|(&p, &(w, l))| self_rise_mc(p, w, l, k_w_per_m_k)).sum()
}

/// Temperature rise per device, milli-°C. `power_uw[j]` (missing = 0) is
/// device `j`'s dissipation. O(n²).
#[must_use]
pub fn rises_mc(l: &Layout, power_uw: &[i32]) -> Vec<i32> {
    if power_uw.iter().all(|&p| p == 0) {
        return vec![0; l.x.len()];
    }
    (0..l.x.len()).map(|i| rise_at(l, power_uw, l.x[i], l.y[i]).round() as i32).collect()
}

/// Rise at `(x, y)` from every source at the current positions, milli-°C,
/// `r` floored at each source's half-extent (so a device's own centre reads
/// its self-heating at `r_floor`). O(n).
fn rise_at(l: &Layout, power_uw: &[i32], x: i32, y: i32) -> f32 {
    let denom = 2.0 * std::f32::consts::PI * K_SI_W_PER_M_K;
    let mut rise = 0.0f32;
    for j in 0..l.x.len() {
        let p = power_uw.get(j).copied().unwrap_or(0);
        if p == 0 {
            continue;
        }
        let r_floor = (l.hw[j].max(l.hh[j])).max(1) as f32;
        let (dx, dy) = ((x - l.x[j]) as f32, (y - l.y[j]) as f32);
        rise += (p as f32) * SCALE_UW_NM_TO_MK / (denom * dx.hypot(dy).max(r_floor));
    }
    rise
}

impl Layout {
    /// Recompute [`Layout::temp_mc`] from `power_uw` and current positions.
    pub fn refresh_temps(&mut self) {
        self.temp_mc = rises_mc(self, &self.power_uw);
    }

    /// Rise at `(x, y)` from every powered source at the **current**
    /// positions (not the epoch-frozen `temp_mc`), milli-°C: what a trial
    /// move can be priced on. O(n).
    #[must_use]
    pub fn rise_at_point_mc(&self, x: i32, y: i32) -> f32 {
        rise_at(self, &self.power_uw, x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Three devices in a row: a 10 mW heater at x=0, sensors at 10 µm and 50 µm.
    fn bench() -> (Layout, Vec<i32>) {
        let l = Layout {
            x: vec![0, 10_000, 50_000],
            y: vec![0, 0, 0],
            hw: vec![500, 500, 500],
            hh: vec![500, 500, 500],
            axis: vec![0],
            groups: vec![],
            orient: vec![crate::Orient::default(); 3],
            variant: vec![0; 3],
            // No disjunctive constraint in a thermal bench; an empty table is the
            // documented all-`false` starting commitment.
            branch: Vec::new(),
            power_uw: vec![10_000, 0, 0],
            temp_mc: vec![0; 3],
            units: Default::default(),
        };
        let p = l.power_uw.clone();
        (l, p)
    }

    #[test]
    fn heat_falls_off_with_distance() {
        let (l, p) = bench();
        let t = rises_mc(&l, &p);
        assert!(t[0] > t[1], "source hotter than near sensor: {t:?}");
        assert!(t[1] > t[2], "near sensor hotter than far one: {t:?}");
        // 1/r spreading: 5× the distance ⇒ ~1/5 the rise.
        let ratio = t[1] as f32 / t[2].max(1) as f32;
        assert!((ratio - 5.0).abs() < 0.5, "expected ~5x falloff, got {ratio}");
    }

    #[test]
    fn no_power_means_no_gradient() {
        let (mut l, _) = bench();
        l.power_uw = vec![0, 0, 0];
        l.refresh_temps();
        assert!(l.temp_mc.iter().all(|&t| t == 0));
    }

    #[test]
    fn moving_a_pair_onto_one_isotherm_kills_its_delta() {
        // The whole point of a thermal-aware placer: the pair's ΔT is a function
        // of placement, so a move can fix it.
        let (mut l, _) = bench();
        l.refresh_temps();
        let split = (l.temp_mc[1] - l.temp_mc[2]).abs();
        assert!(split > 0, "asymmetric placement must show a gradient");

        // Put both sensors the same distance from the heater (mirrored) — one
        // isotherm, so the gradient across the pair vanishes.
        l.x = vec![0, 10_000, -10_000];
        l.refresh_temps();
        let iso = (l.temp_mc[1] - l.temp_mc[2]).abs();
        assert_eq!(iso, 0, "mirrored pair sits on one isotherm");
        assert!(iso < split);
    }

    #[test]
    fn the_live_field_follows_a_move_before_any_refresh() {
        let (mut l, _) = bench();
        l.refresh_temps();
        let live = |l: &Layout| (l.rise_at_point_mc(l.x[1], l.y[1]) - l.rise_at_point_mc(l.x[2], l.y[2])).abs();
        let frozen = |l: &Layout| (l.temp_mc[1] - l.temp_mc[2]).abs();
        assert!((live(&l) - frozen(&l) as f32).abs() <= 1.0, "agrees when fresh");
        l.x[2] = -10_000; // onto partner 1's isotherm; temp_mc is now stale
        assert!(live(&l) < 1.0);
        assert!(frozen(&l) > 0, "the frozen field has not moved");
    }

    /// Hastings §5.1 example: 100 mW over a 25 µm square on k = 130 W/(m·K)
    /// rises ≈ 13.6 K.
    #[test]
    fn hastings_self_heating_example() {
        let r = self_rise_mc(100_000, 25_000, 25_000, 130.0);
        assert!((r - 13_600.0).abs() <= 100.0, "{r}");
    }

    /// 2 × 1 mW on 10 µm squares: bound 2·ln4·1 mW/(π·148·10 µm) ≈ 596 mK,
    /// and no placement (overlap allowed) rises a device above it.
    #[test]
    fn rise_bound_holds_for_any_placement() {
        let bound = rise_bound_mc(&[1_000, 1_000], &[(10_000, 10_000); 2], K_SI_W_PER_M_K);
        assert!((bound - 596.0).abs() <= 2.0, "{bound}");
        let (mut l, _) = bench();
        l.hw = vec![5_000; 2];
        l.hh = vec![5_000; 2];
        l.power_uw = vec![1_000, 1_000];
        let mut seed = 0x2545_f491_4f6c_dd1du64;
        let mut next = || {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            ((seed >> 33) % 100_001) as i32 - 50_000
        };
        for _ in 0..50 {
            l.x = vec![next(), next()];
            l.y = vec![next(), next()];
            for (i, t) in rises_mc(&l, &l.power_uw).into_iter().enumerate() {
                assert!(t as f32 <= bound, "device {i} at {:?}/{:?}: {t} > {bound}", l.x, l.y);
            }
        }
    }

    #[test]
    fn point_rise_equals_device_rise_at_its_centre() {
        let (l, p) = bench();
        let t = rises_mc(&l, &p);
        for i in 0..3 {
            assert!((l.rise_at_point_mc(l.x[i], l.y[i]) - t[i] as f32).abs() <= 0.5, "device {i}");
        }
    }
}
