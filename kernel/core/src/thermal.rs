//! Steady-state die temperature by **superposition of per-device point sources**.
//!
//! This is the model that makes [`crate::Layout::temp_mc`] real, and with it the
//! analog `ThermalGradient` rule: without it the rule's estimated gradient stays
//! at zero and the check is a tautology.
//!
//! ## Why superposition and not a solve
//!
//! The thermal-driven analog placers (NTU DAC'09; Lampaert; Liu) pre-simulate a
//! thermal profile per power device and combine them by superposition through a
//! lookup table, precisely because a full solve is far too slow to sit inside a
//! placement loop. Steady-state heat conduction is linear in the source powers,
//! so the sum of per-source profiles *is* the exact profile for that power set —
//! the approximation is in the per-source profile, not in the superposition.
//!
//! The per-source profile used here is the classic semi-infinite-substrate
//! spreading solution, `ΔT(r) = P / (2π·k·r)`, floored at the device's own
//! half-extent so a device's self-heating stays finite.
//!
//! ## Cadence
//!
//! Temperature is a **global** property of the whole power distribution: moving
//! one device changes every device's temperature. Per `backend/TODO.md` §1 it is
//! therefore refreshed at epoch/iteration boundaries, never per trial move.

use crate::layout::Layout;

/// Thermal conductivity of bulk silicon, W/(m·K), at ~300 K.
///
/// ponytail: one bulk constant for the whole die — no per-layer stack, no BEOL,
/// no package θ_JA. Real dies are anisotropic and package-dominated; when a PDK
/// grows a thermal section, read `k` (and an ambient offset) from it instead.
/// Calibrate against silicon before trusting absolute temperatures — the
/// *gradient between nearby matched devices*, which is what the rule scores, is
/// far more robust than the absolute rise.
const K_SI_W_PER_M_K: f32 = 148.0;

/// Convert `P/(2π·k·r)` from (µW, nm) into milli-Kelvin.
///
/// `ΔT[K] = P[W] / (2π·k·r[m])`. With `P` in µW (`1e-6 W`) and `r` in nm
/// (`1e-9 m`) the unit factor is `1e-6/1e-9 = 1e3`, and milli-K adds `1e3`:
/// `ΔT[mK] = P[µW]·1e6 / (2π·k·r[nm])`.
const SCALE_UW_NM_TO_MK: f32 = 1.0e6;

/// Steady-state temperature rise above ambient for every device, milli-°C.
///
/// `power_uw[i]` is device `i`'s dissipation in µW; devices with zero (or
/// missing) power are pure sensors — they still *receive* heat, they just do not
/// emit any. Returns one rise per device, so `temp[a] − temp[b]` is the ΔT a
/// matched pair sees.
///
/// `O(n²)` over devices, called once per epoch rather than per move.
#[must_use]
pub fn rises_mc(l: &Layout, power_uw: &[i32]) -> Vec<i32> {
    let n = l.x.len();
    let mut out = vec![0i32; n];
    if power_uw.iter().all(|&p| p == 0) {
        return out; // no sources → uniform die, honest zero
    }
    let denom = 2.0 * std::f32::consts::PI * K_SI_W_PER_M_K;
    for (i, o) in out.iter_mut().enumerate() {
        let mut rise = 0.0f32;
        for j in 0..n {
            let p = power_uw.get(j).copied().unwrap_or(0);
            if p == 0 {
                continue;
            }
            // Distance from source j to victim i, floored at the source's own
            // half-extent so self-heating (r → 0) stays finite.
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
    /// Recompute [`Layout::temp_mc`] from [`Layout::power_uw`] and the current
    /// device positions. Call at epoch/iteration boundaries — see the module
    /// docs on cadence.
    pub fn refresh_temps(&mut self) {
        self.temp_mc = rises_mc(self, &self.power_uw.clone());
    }

    /// Temperature difference between two devices, milli-°C — what a matched
    /// pair's thermal budget is scored against. `0` when temperatures have never
    /// been refreshed (all-zero `temp_mc`).
    #[inline]
    #[must_use]
    pub fn delta_temp_mc(&self, a: crate::ids::Target, b: crate::ids::Target) -> i32 {
        let t = |x: crate::ids::Target| -> i32 {
            match x {
                crate::ids::Target::Device(d) => {
                    self.temp_mc.get(d.0 as usize).copied().unwrap_or(0)
                }
                // A group's temperature is its hottest member: a matched *group*
                // is limited by its worst-placed device, not its average.
                crate::ids::Target::Group(g) => self
                    .groups
                    .get(g.0 as usize)
                    .map(|ms| {
                        ms.iter()
                            .map(|d| self.temp_mc.get(d.0 as usize).copied().unwrap_or(0))
                            .max()
                            .unwrap_or(0)
                    })
                    .unwrap_or(0),
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
