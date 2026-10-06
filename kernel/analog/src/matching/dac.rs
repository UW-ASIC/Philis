//! Capacitor-array figures under an oxide-thickness gradient (MAT-11).
//!
//! Units are `(slot, x_um, y_um)` about the array centroid. A unit's
//! capacitance is `C ∝ 1/t` with `t = t0·(1 + g·(x cosθ + y sinθ))` (DACP eq.
//! 11, the t0/t model); `g` per µm is a sweep reading of DACP's γ = 10/100
//! ppm, not a deck value. INL/DNL are DACP eqs 17–18, M_sys eq. 12; the second
//! moments are REV eqs 8–10.
//! Split DAC (MAT-18): C_A is DACP eq. 1, non-unit plate sizing §III-D, slot
//! order §IV-C.

/// Per-slot capacitance, unit-normalised: `Σ 1/(1 + g·(x cosθ + y sinθ))`
/// over each slot's units, `slots` entries. Panics when a unit's slot is
/// `≥ slots`.
#[must_use]
pub fn gradient_caps(units: &[(u8, f64, f64)], slots: usize, g_per_um: f64, theta: f64) -> Vec<f64> {
    let (c, s) = (theta.cos(), theta.sin());
    let mut cap = vec![0.0; slots];
    for &(k, x, y) in units {
        cap[usize::from(k)] += 1.0 / (1.0 + g_per_um * (x * c + y * s));
    }
    cap
}

/// Worst `(|INL|, |DNL|)`, LSB, over θ = k·π/`steps` (k < steps): slot 0 is
/// the termination unit, bits 1..=`n_bits`; every code's transfer
/// `Σ_{b set} C_b / ΣC · 2ⁿ` against the ideal code. `(0, 0)` when `steps`
/// is 0 or there are no units. Cost O(steps · (units + 2ⁿ)); requires
/// `n_bits < usize::BITS` and every unit slot `≤ n_bits`.
#[must_use]
pub fn inl_dnl(units: &[(u8, f64, f64)], n_bits: u8, g_per_um: f64, steps: usize) -> (f64, f64) {
    let n = usize::from(n_bits);
    let codes = 1usize << n;
    let (mut inl, mut dnl) = (0.0f64, 0.0f64);
    for k in 0..steps {
        let cap = gradient_caps(units, n + 1, g_per_um, k as f64 * std::f64::consts::PI / steps as f64);
        let total: f64 = cap.iter().sum();
        let t = |c: usize| (1..=n).filter(|b| c >> (b - 1) & 1 == 1).map(|b| cap[b]).sum::<f64>() / total * codes as f64;
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
/// capacitance of each slot against slot 0's. `counts[i]` is slot `i`'s unit
/// count and must be positive; `0` with fewer than two slots or no steps.
#[must_use]
pub fn msys(units: &[(u8, f64, f64)], counts: &[u16], g_per_um: f64, steps: usize) -> f64 {
    let mut worst = 0.0f64;
    for k in 0..steps {
        let cap = gradient_caps(units, counts.len(), g_per_um, k as f64 * std::f64::consts::PI / steps as f64);
        let unit0 = cap[0] / f64::from(counts[0]);
        for i in 1..counts.len() {
            worst = worst.max((cap[i] / f64::from(counts[i]) / unit0 - 1.0).abs());
        }
    }
    worst
}

/// max over slots of ‖M_slot − M_array‖_F per unit, µm² (M = [xx, xy, yy] about the array centroid).
/// Unit coordinates must already be centred on the array centroid. Slots
/// without units, and slots `≥ slots`, are skipped; `0` without units.
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
    let Some(all) = mean(&|_| true) else { return 0.0 };
    (0..slots)
        .filter_map(|s| mean(&|u| usize::from(u.0) == s))
        .map(|m| ((m[0] - all[0]).powi(2) + 2.0 * (m[1] - all[1]).powi(2) + (m[2] - all[2]).powi(2)).sqrt())
        .fold(0.0, f64::max)
}

/// C_A in units of C_u: `C_T^LSB / C_T^MSB` (DACP eq. 1); the LSB total
/// counts the termination unit. Infinite when `ct_msb_units` is 0.
#[must_use]
pub fn attenuation_cap(ct_lsb_units: u32, ct_msb_units: u32) -> f64 {
    f64::from(ct_lsb_units) / f64::from(ct_msb_units)
}

/// Non-unit side lengths `(H, l)`, `H ≥ l`, of area `A` with the unit's
/// perimeter-to-area ratio `k = (H_u + l_u)/(H_u·l_u)`: roots of
/// `z² − kA·z + A = 0` (DACP §III-D eqs 37–45). `None` when `k²A² < 4A` (no
/// rectangle of that area keeps the unit's edge sensitivity). Unit sides
/// must be positive.
#[must_use]
pub fn nonunit_dims(area_um2: f64, unit_h_um: f64, unit_l_um: f64) -> Option<(f64, f64)> {
    let ka = (unit_h_um + unit_l_um) / (unit_h_um * unit_l_um) * area_um2;
    let disc = ka * ka - 4.0 * area_um2;
    (disc >= 0.0).then(|| ((ka + disc.sqrt()) / 2.0, (ka - disc.sqrt()) / 2.0))
}

/// Owner per cell (row-major, `None` = dummy) of a split DAC: ids `0..=L` the
/// LSB bank `[1, 1, 2, …, 2^(L-1)]`, `L+1..=L+M` the MSB bank
/// `[1, 2, …, 2^(M-1)]`, `L+M+1` the two C_A halves. DACP §IV-C order along
/// [`pattern::spiral`]: C_A on the most central reflected pair; the odd caps
/// (C0, C1, C_{L+1}) next, the centre cell first on an odd grid, else split
/// across one pair as `centro_assign` does; then the remaining pairs, LSB and
/// MSB tokens alternating (each bank largest cap first), each at the next free
/// pair.
///
/// Requires `l_bits ≥ 1`, `m_bits ≥ 1` (debug-asserted) and a grid of at
/// least `2^L + 2^M + 1` cells; on a smaller grid the tail tokens are dropped.
///
/// [`pattern::spiral`]: crate::matching::pattern::spiral
#[must_use]
pub fn split_dac_assign(l_bits: u8, m_bits: u8, rows: usize, cols: usize) -> Vec<Option<u8>> {
    use crate::matching::pattern::{spiral, Grid};
    let (l, m) = (usize::from(l_bits), usize::from(m_bits));
    debug_assert!(l >= 1 && m >= 1, "split DAC needs both banks: L={l}, M={m}");
    let mut left: Vec<usize> = std::iter::once(1).chain((0..l).map(|k| 1 << k)).chain((0..m).map(|k| 1 << k)).collect();
    let n = rows * cols;
    debug_assert!(n >= (1 << l) + (1 << m) - 1 + 2, "{rows}×{cols} cannot hold L={l}, M={m}");
    let order = spiral(rows, cols);
    let mut g = Grid { cols, slot: vec![None; n] };
    let next = |g: &Grid| order.iter().copied().find(|&i| g.free_pair(i));
    if let Some(i) = next(&g) {
        g.put_pair(i, (l + m + 1) as u8);
    }
    let mut odd = vec![0, 1, l + 1];
    if n % 2 == 1 {
        g.slot[n / 2] = Some(0);
        odd.remove(0);
    }
    for pair in odd.chunks(2) {
        let Some(i) = next(&g) else { break };
        g.slot[i] = Some(pair[0] as u8);
        if let Some(&d) = pair.get(1) {
            let j = g.refl(i);
            g.slot[j] = Some(d as u8);
        }
    }
    for d in [0, 1, l + 1] {
        left[d] = 0;
    }
    let tokens = |ids: std::ops::RangeInclusive<usize>| ids.rev().flat_map(|d| std::iter::repeat_n(d, left[d] / 2)).collect::<Vec<_>>();
    let (lsb, msb) = (tokens(0..=l), tokens(l + 1..=l + m));
    let mut seq: Vec<usize> = Vec::with_capacity(lsb.len() + msb.len());
    for k in 0..lsb.len().max(msb.len()) {
        seq.extend(lsb.get(k));
        seq.extend(msb.get(k));
    }
    for d in seq {
        let Some(i) = next(&g) else { break };
        g.put_pair(i, d as u8);
    }
    g.slot
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
            .filter_map(|(i, s)| s.map(|s| (s, ((i % cols) as f64 - cx) * p, ((i / cols) as f64 - cy) * p)))
            .collect()
    }

    #[test]
    fn chessboard_inl_not_worse_on_uniform_pitch() {
        let counts = [1, 1, 2, 4, 8, 16];
        for (rows, cols) in [(4, 8), (8, 4)] {
            let inl = |f| inl_dnl(&grid(&counts, rows, cols, f, 2.0), 5, 1e-4, 4 * rows.max(cols)).0;
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
        let row = |o: [u8; 4]| o.iter().zip([-3.0, -1.0, 1.0, 3.0]).map(|(&s, x)| (s, x, 0.0)).collect::<Vec<_>>();
        let abba = msys(&row([0, 1, 1, 0]), &[2, 2], 1e-4, 16);
        let aabb = msys(&row([0, 0, 1, 1]), &[2, 2], 1e-4, 16);
        assert!(abba < 1e-6, "{abba}");
        assert!(aabb > 1e-5, "{aabb}");
    }

    #[test]
    fn attenuation_cap_formula() {
        assert!((attenuation_cap(8, 7) - 8.0 / 7.0).abs() < 1e-12);
        assert!((attenuation_cap(4, 3) - 4.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn nonunit_dims_keep_the_unit_edge_sensitivity() {
        let (h, l) = nonunit_dims(12.0, 3.0, 3.0).unwrap();
        assert!((h - 6.0).abs() < 1e-9 && (l - 2.0).abs() < 1e-9, "{h} {l}");
        for (a, hu, lu) in [(12.0, 3.0, 3.0), (10.0, 2.0, 3.0), (37.5, 4.0, 2.5)] {
            let (h, l) = nonunit_dims(a, hu, lu).unwrap();
            assert!(h >= l);
            assert!((h * l - a).abs() < 1e-9);
            assert!(((h + l) / (h * l) - (hu + lu) / (hu * lu)).abs() < 1e-9);
        }
        assert_eq!(nonunit_dims(4.0, 3.0, 3.0), None);
    }

    #[test]
    fn split_assign_is_exact_for_even_count_caps() {
        for (lb, mb, rows, cols) in [(2u8, 2u8, 3usize, 3usize), (3, 3, 3, 6), (3, 3, 4, 5), (4, 3, 5, 5)] {
            let slot = split_dac_assign(lb, mb, rows, cols);
            let n = slot.len();
            let want: Vec<usize> = std::iter::once(1)
                .chain((0..lb).map(|k| 1 << k))
                .chain((0..mb).map(|k| 1 << k))
                .chain([2])
                .collect();
            for (d, &w) in want.iter().enumerate() {
                let cells: Vec<usize> = (0..n).filter(|&i| slot[i] == Some(d as u8)).collect();
                assert_eq!(cells.len(), w, "L={lb} M={mb} {rows}×{cols} id {d}");
                if w % 2 == 0 {
                    assert!(cells.iter().all(|&i| slot[n - 1 - i] == slot[i]), "L={lb} M={mb} id {d} not centro");
                }
            }
            let r2 = |i: usize| {
                let (dr, dc) = (2 * (i / cols) as i64 - rows as i64 + 1, 2 * (i % cols) as i64 - cols as i64 + 1);
                dr * dr + dc * dc
            };
            let ca = lb + mb + 1;
            let min = (0..n).filter(|&i| r2(i) > 0).map(r2).min().unwrap();
            assert!((0..n).filter(|&i| slot[i] == Some(ca)).all(|i| r2(i) == min), "L={lb} M={mb} C_A not central");
            if (lb, mb, rows, cols) == (3, 3, 3, 6) {
                // §IV-C order: the first pairs after C_A and the odd caps hold C_L, then C_{L+M}.
                let fixed = [Some(0), Some(1), Some(lb + 1), Some(ca), None];
                let mut seen = vec![false; n];
                let mut ids = Vec::new();
                for i in crate::matching::pattern::spiral(rows, cols) {
                    if !seen[n - 1 - i] && !fixed.contains(&slot[i]) {
                        ids.push(slot[i].unwrap());
                    }
                    seen[i] = true;
                }
                assert_eq!(ids[..2], [lb, lb + mb], "pair order {ids:?}");
            }
        }
    }
}
