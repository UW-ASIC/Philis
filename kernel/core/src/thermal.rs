//! Steady-state die temperature by superposition of per-device point sources,
//! `ΔT(r) = P / (2π·k·r)` (semi-infinite substrate), with `r` floored at the
//! source's own half-extent. Linear in power, so the sum is exact for the
//! per-source model. Global in the power map: refresh per epoch, never per move.

use crate::ids::Target;
use crate::layout::Layout;

/// Bulk Si thermal conductivity, W/(m·K), ~300 K.
///
/// ponytail: one bulk constant, no BEOL/package θ_JA. Gradients between nearby
/// matched devices (what the rule scores) are far more robust than absolute
/// rises; read `k` from the PDK when it grows a thermal section.
const K_SI_W_PER_M_K: f32 = 148.0;

/// `ΔT[mK] = P[µW]·1e6 / (2π·k·r[nm])`.
const SCALE_UW_NM_TO_MK: f32 = 1.0e6;

/// Temperature rise per device, milli-°C. `power_uw[j]` (missing = 0) is
/// device `j`'s dissipation. O(n²).
#[must_use]
pub fn rises_mc(l: &Layout, power_uw: &[i32]) -> Vec<i32> {
    let n = l.x.len();
    let mut out = vec![0i32; n];
    if power_uw.iter().all(|&p| p == 0) {
        return out;
    }
    let denom = 2.0 * std::f32::consts::PI * K_SI_W_PER_M_K;
    for (i, o) in out.iter_mut().enumerate() {
        let mut rise = 0.0f32;
        for j in 0..n {
            let p = power_uw.get(j).copied().unwrap_or(0);
            if p == 0 {
                continue;
            }
            let r_floor = (l.hw[j].max(l.hh[j])).max(1) as f32;
            let r = if i == j {
                r_floor
            } else {
                let dx = (l.x[i] - l.x[j]) as f32;
                let dy = (l.y[i] - l.y[j]) as f32;
                (dx * dx + dy * dy).sqrt().max(r_floor)
            };
            rise += (p as f32) * SCALE_UW_NM_TO_MK / (denom * r);
        }
        *o = rise.round() as i32;
    }
    out
}

impl Layout {
    /// Recompute [`Layout::temp_mc`] from `power_uw` and current positions.
    pub fn refresh_temps(&mut self) {
        self.temp_mc = rises_mc(self, &self.power_uw);
    }

    /// |ΔT| between two targets, milli-°C. A group reads as its hottest member.
    #[inline]
    #[must_use]
    pub fn delta_temp_mc(&self, a: Target, b: Target) -> i32 {
        let temp = |d: crate::DeviceId| self.temp_mc.get(d.0 as usize).copied().unwrap_or(0);
        let t = |x: Target| match x {
            Target::Device(d) => temp(d),
            Target::Group(g) => {
                self.groups.get(g.0 as usize).and_then(|ms| ms.iter().map(|&d| temp(d)).max()).unwrap_or(0)
            }
        };
        (t(a) - t(b)).abs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{DeviceId, Target};

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
        assert_eq!(
            l.delta_temp_mc(Target::Device(DeviceId(1)), Target::Device(DeviceId(2))),
            0
        );
    }

    #[test]
    fn moving_a_pair_onto_one_isotherm_kills_its_delta() {
        // The whole point of a thermal-aware placer: the pair's ΔT is a function
        // of placement, so a move can fix it.
        let (mut l, _) = bench();
        l.refresh_temps();
        let split = l.delta_temp_mc(Target::Device(DeviceId(1)), Target::Device(DeviceId(2)));
        assert!(split > 0, "asymmetric placement must show a gradient");

        // Put both sensors the same distance from the heater (mirrored) — one
        // isotherm, so the gradient across the pair vanishes.
        l.x = vec![0, 10_000, -10_000];
        l.refresh_temps();
        let iso = l.delta_temp_mc(Target::Device(DeviceId(1)), Target::Device(DeviceId(2)));
        assert_eq!(iso, 0, "mirrored pair sits on one isotherm");
        assert!(iso < split);
    }
}
