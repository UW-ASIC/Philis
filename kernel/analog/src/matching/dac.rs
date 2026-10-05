//! Capacitor-array figures under an oxide-thickness gradient (MAT-11).
//!
//! Units are `(slot, x_um, y_um)` about the array centroid. A unit's
//! capacitance is `C ∝ 1/t` with `t = t0·(1 + g·(x cosθ + y sinθ))` (DACP eq.
//! 11, the t0/t model); `g` per µm is a sweep reading of DACP's γ = 10/100
//! ppm, not a deck value. INL/DNL are DACP eqs 17–18, M_sys eq. 12; the second
//! moments are REV eqs 8–10.

/// Per-slot capacitance, unit-normalised: `Σ 1/(1 + g·(x cosθ + y sinθ))`.
#[must_use]
pub fn gradient_caps(
    units: &[(u8, f64, f64)],
    slots: usize,
    g_per_um: f64,
    theta: f64,
) -> Vec<f64> {
    let (c, s) = (theta.cos(), theta.sin());
    let mut cap = vec![0.0; slots];
    for &(k, x, y) in units {
        cap[usize::from(k)] += 1.0 / (1.0 + g_per_um * (x * c + y * s));
    }
    cap
}

/// Worst `(|INL|, |DNL|)`, LSB, over θ = k·π/`steps` (k < steps): slot 0 is
/// the termination unit, bits 1..=`n_bits`; every code's transfer
/// `Σ_{b set} C_b / ΣC · 2ⁿ` against the ideal code.
#[must_use]
pub fn inl_dnl(units: &[(u8, f64, f64)], n_bits: u8, g_per_um: f64, steps: usize) -> (f64, f64) {
    let n = usize::from(n_bits);
    let codes = 1usize << n;
    let (mut inl, mut dnl) = (0.0f64, 0.0f64);
    for k in 0..steps {
        let cap = gradient_caps(
            units,
            n + 1,
            g_per_um,
            k as f64 * std::f64::consts::PI / steps as f64,
        );
        let total: f64 = cap.iter().sum();
        let t = |c: usize| {
            (1..=n)
                .filter(|b| c >> (b - 1) & 1 == 1)
                .map(|b| cap[b])
                .sum::<f64>()
                / total
                * codes as f64
        };
        for c in 0..codes {
            inl = inl.max((t(c) - c as f64).abs());
            if c + 1 < codes {
                dnl = dnl.max((t(c + 1) - t(c) - 1.0).abs());
            }
        }
    }
    (inl, dnl)
}

/// Systematic ratio mismatch M_sys: worst over θ (as [`inl_dnl`]) and slots
/// `i ≥ 1` of `|(C_i/counts_i) / (C_0/counts_0) − 1|`, the per-unit
/// capacitance of each slot against slot 0's.
#[must_use]
pub fn msys(units: &[(u8, f64, f64)], counts: &[u16], g_per_um: f64, steps: usize) -> f64 {
    let mut worst = 0.0f64;
    for k in 0..steps {
        let cap = gradient_caps(
            units,
            counts.len(),
            g_per_um,
            k as f64 * std::f64::consts::PI / steps as f64,
        );
        let unit0 = cap[0] / f64::from(counts[0]);
        for i in 1..counts.len() {
            worst = worst.max((cap[i] / f64::from(counts[i]) / unit0 - 1.0).abs());
        }
    }
    worst
}

/// max over slots of ‖M_slot − M_array‖_F per unit, µm² (M = [xx, xy, yy] about the array centroid).
#[must_use]
pub fn second_um2(units: &[(u8, f64, f64)], slots: usize) -> f64 {
    let mean = |f: &dyn Fn(&(u8, f64, f64)) -> bool| {
        let (mut m, mut n) = ([0.0f64; 3], 0.0f64);
        for u in units.iter().filter(|u| f(u)) {
            m = [m[0] + u.1 * u.1, m[1] + u.1 * u.2, m[2] + u.2 * u.2];
            n += 1.0;
        }
        (n > 0.0).then(|| m.map(|v| v / n))
    };
    let Some(all) = mean(&|_| true) else {
        return 0.0;
    };
    (0..slots)
        .filter_map(|s| mean(&|u| usize::from(u.0) == s))
        .map(|m| {
            ((m[0] - all[0]).powi(2) + 2.0 * (m[1] - all[1]).powi(2) + (m[2] - all[2]).powi(2))
                .sqrt()
        })
        .fold(0.0, f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matching::pattern::{centro_assign, Fill};

    /// `centro_assign` slots at pitch `p` µm, centred.
    fn grid(counts: &[u16], rows: usize, cols: usize, fill: Fill, p: f64) -> Vec<(u8, f64, f64)> {
        let (slots, _) = centro_assign(counts, rows, cols, fill);
        let (cx, cy) = ((cols as f64 - 1.0) / 2.0, (rows as f64 - 1.0) / 2.0);
        slots
            .iter()
            .enumerate()
            .filter_map(|(i, s)| {
                s.map(|s| {
                    (
                        s,
                        ((i % cols) as f64 - cx) * p,
                        ((i / cols) as f64 - cy) * p,
                    )
                })
            })
            .collect()
    }

    #[test]
    fn chessboard_inl_not_worse_on_uniform_pitch() {
        let counts = [1, 1, 2, 4, 8, 16];
        for (rows, cols) in [(4, 8), (8, 4)] {
            let inl = |f| {
                inl_dnl(
                    &grid(&counts, rows, cols, f, 2.0),
                    5,
                    1e-4,
                    4 * rows.max(cols),
                )
                .0
            };
            let (d, c) = (inl(Fill::Dispersed), inl(Fill::Compact));
            assert!(d < c, "{rows}×{cols}: dispersed {d} vs compact {c}");
        }
    }

    #[test]
    fn radial_metric_no_longer_hides_anisotropy() {
        let u = [(0, 2.0, 0.0), (0, -2.0, 0.0), (1, 0.0, 2.0), (1, 0.0, -2.0)];
        assert!((second_um2(&u, 2) - 8f64.sqrt()).abs() < 1e-9);
    }

    #[test]
    fn msys_second_order_for_exact_cc() {
        let row = |o: [u8; 4]| {
            o.iter()
                .zip([-3.0, -1.0, 1.0, 3.0])
                .map(|(&s, x)| (s, x, 0.0))
                .collect::<Vec<_>>()
        };
        let abba = msys(&row([0, 1, 1, 0]), &[2, 2], 1e-4, 16);
        let aabb = msys(&row([0, 0, 1, 1]), &[2, 2], 1e-4, 16);
        assert!(abba < 1e-6, "{abba}");
        assert!(aabb > 1e-5, "{aabb}");
    }
}
