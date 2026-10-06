//! Steady-state die temperature by superposition of per-device sources on a
//! semi-infinite substrate: outside a source's floor disc (radius = its
//! larger half-extent) a point source, `ΔT(r) = P / (2π·k·r)`; inside it the
//! source's own-area rise, Hastings eq. 5.6 ([`self_rise_mc`]). Linear in
//! power, so the sum is exact for the per-source model. Global in the power map: refresh per epoch, never per move.

use crate::layout::Layout;

/// Bulk Si thermal conductivity, W/(m·K), ~300 K.
///
/// ponytail: one bulk constant, no BEOL/package θ_JA. Gradients between nearby
/// matched devices (what the rule scores) are far more robust than absolute
/// rises; read `k` from the PDK when it grows a thermal section.
pub const K_SI_W_PER_M_K: f32 = 148.0;

/// Unit factor: `ΔT[mK] = P[µW]·1e6 / (2π·k[W/(m·K)]·r[nm])`.
const SCALE_UW_NM_TO_MK: f32 = 1.0e6;

/// Hastings eq. 5.6: the rise of a uniform W×L source over its own area,
/// `ln(4L/W)·P/(π·k·L)` (L = the longer side), mK, for `p_uw` µW on a
/// `w_nm`×`l_nm` footprint (either order) and conductivity `k_w_per_m_k`.
/// Sides floored at 1 nm, so it is always finite for `k > 0`. Linear in power.
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

/// Temperature rise per device at its centre, milli-°C, rounded. `power_uw[j]`
/// (missing = 0) is device `j`'s dissipation; entries past the layout are
/// ignored. O(n²); allocates the result.
#[must_use]
pub fn rises_mc(l: &Layout, power_uw: &[i32]) -> Vec<i32> {
    if power_uw.iter().all(|&p| p == 0) {
        return vec![0; l.x.len()];
    }
    (0..l.x.len()).map(|i| rise_at(l, power_uw, l.x[i], l.y[i]).round() as i32).collect()
}

/// Rise at `(x, y)` from every source at the current positions, milli-°C.
/// Inside a source's floor disc (its larger half-extent) the term is eq. 5.6
/// for that source, so a device's own centre, or an overlapping neighbour
/// anywhere in the disc, reads the same self term. O(n).
///
/// ponytail: the field steps at `r = r_floor`: inside reads
/// `ln(4L/W)·P/(πk·L)`, just outside `P/(πk·L)`, a factor `ln(4L/W)` (ln4 ≈
/// 1.39 for a square, more for a long source). A neighbour crossing the disc
/// edge in gp/dp sees that step in the gradient score; blend the two terms
/// over the edge if a move loop is seen to chatter on it.
fn rise_at(l: &Layout, power_uw: &[i32], x: i32, y: i32) -> f32 {
    let denom = 2.0 * std::f32::consts::PI * K_SI_W_PER_M_K;
    let mut rise = 0.0f32;
    for j in 0..l.x.len() {
        let p = power_uw.get(j).copied().unwrap_or(0);
        if p == 0 {
            continue;
        }
        let r_floor = (l.hw[j].max(l.hh[j])).max(1) as f32;
        // Differences in i64: two on-die i32 coordinates can be > i32::MAX apart.
        let (dx, dy) = ((i64::from(x) - i64::from(l.x[j])) as f32, (i64::from(y) - i64::from(l.y[j])) as f32);
        let r = dx.hypot(dy);
        rise += if r < r_floor {
            self_rise_mc(p, 2 * l.hw[j].min(l.hh[j]), 2 * l.hw[j].max(l.hh[j]), K_SI_W_PER_M_K)
        } else {
            (p as f32) * SCALE_UW_NM_TO_MK / (denom * r)
        };
    }
    rise
}

impl Layout {
    /// Recomputes [`Layout::temp_mc`] from `power_uw` and current positions
    /// ([`rises_mc`]). O(n²).
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

    /// REL-14: a lone 100 mW device on a 25 µm square reads eq. 5.6,
    /// ln4·0.1 W/(π·148·25 µm) = 11.93 K, not the floored point source
    /// (0.1 W/(2π·148·12.5 µm) = 8.60 K).
    #[test]
    fn the_self_term_is_eq_5_6() {
        let (mut l, _) = bench();
        l.x.truncate(1);
        l.y.truncate(1);
        l.hw = vec![12_500];
        l.hh = vec![12_500];
        l.power_uw = vec![100_000];
        let t = rises_mc(&l, &l.power_uw)[0];
        assert!((t - 11_930).abs() <= 20, "{t}");
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
