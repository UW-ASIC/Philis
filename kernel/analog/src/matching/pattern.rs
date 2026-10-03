//! Common-centroid cell assignment on a row-major unit grid: which grids can
//! hold a matched set point-symmetrically ([`grids`]) and which member owns
//! each cell ([`centro_assign`]). Doubled offsets from the grid centre
//! (`2r − rows + 1`, `2c − cols + 1`) keep every moment an integer.

use std::cmp::Reverse;

use super::moments;

/// How [`centro_assign`] deals each member's reflected pairs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fill {
    /// DACP spiral: members contiguous, ring by ring (cap_array Spiral).
    Compact,
    /// Farthest-point reflected pairs (cap_array Chessboard).
    Dispersed,
    /// Reflected pairs dealt to balance every member's second moment: the
    /// outermost pair first, to the member furthest below its share
    /// `count_d · R / T` of the total r² (LPT).
    Balanced,
}

/// Grids `(rows, cols)` that can hold `counts` point-symmetrically, nearest
/// square first (aspect compared exactly), then fewest empty cells, then
/// fewest rows; the `1 × T` row always last. With exactly one odd member both
/// sides are odd, so it takes the centre and the empties pair up. Aspect
/// above `max_aspect` (≥ 1) is dropped. Empty for `T = 0`.
#[must_use]
pub fn grids(counts: &[u16], max_aspect: f64) -> Vec<(usize, usize)> {
    let t: usize = counts.iter().map(|&c| usize::from(c)).sum();
    if t == 0 {
        return vec![];
    }
    let one_odd = counts.iter().filter(|&&c| c % 2 == 1).count() == 1;
    let mut out: Vec<(usize, usize)> = (1..=t.isqrt() + 2)
        .filter(|&r| !(one_odd && r % 2 == 0))
        .map(|r| {
            let c = t.div_ceil(r);
            (r, if one_odd && c % 2 == 0 { c + 1 } else { c })
        })
        .filter(|&(r, c)| r.max(c) as f64 / r.min(c) as f64 <= max_aspect)
        .collect();
    out.sort_by(|&(ra, ca), &(rb, cb)| {
        (ra.max(ca) * rb.min(cb))
            .cmp(&(rb.max(cb) * ra.min(ca)))
            .then((ra * ca).cmp(&(rb * cb)))
            .then(ra.cmp(&rb))
    });
    out.dedup();
    if !out.contains(&(1, t)) {
        out.push((1, t));
    }
    out
}

/// Whether any grid admits an exact assignment: at most one member odd.
#[must_use]
pub fn cc_feasible(counts: &[u16]) -> bool {
    counts.iter().filter(|&&c| c % 2 == 1).count() <= 1
}

/// Owner per cell of a `rows × cols` grid (row-major, `None` = empty or
/// dummy), and `exact`: every member's cells are closed under the 180°
/// rotation about the grid centre (first moments coincide exactly).
///
/// Odd counts first: one takes the centre cell of an odd grid, the rest pair
/// up across the centre (inexact). Then reflected pairs by `fill`; `Compact`
/// and `Dispersed` serve the largest member first.
#[must_use]
pub fn centro_assign(counts: &[u16], rows: usize, cols: usize, fill: Fill) -> (Vec<Option<u8>>, bool) {
    let total: usize = counts.iter().map(|&u| usize::from(u)).sum();
    debug_assert!(rows * cols >= total, "{rows}×{cols} cannot hold {total}");
    let n = rows * cols;
    let mut g = Grid { cols, slot: vec![None; n] };
    let key = |i: usize| {
        let (dr, dc) = (2 * (i / cols) as i64 - rows as i64 + 1, 2 * (i % cols) as i64 - cols as i64 + 1);
        ((dr * dr + dc * dc), (dr as f64).atan2(dc as f64))
    };
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| key(a).0.cmp(&key(b).0).then(key(a).1.total_cmp(&key(b).1)));
    // Odd counts first: the centre cell (odd grid) for one, reflected pairs
    // split between two for the rest.
    let mut left: Vec<usize> = counts.iter().map(|&u| usize::from(u)).collect();
    let mut odd: Vec<usize> = (0..left.len()).filter(|&d| left[d] % 2 == 1).collect();
    if n % 2 == 1 {
        if let Some(d) = odd.pop() {
            g.slot[n / 2] = Some(d as u8);
            left[d] -= 1;
        }
    }
    for pair in odd.chunks(2) {
        let Some(i) = order.iter().copied().find(|&i| g.free_pair(i)) else { break };
        let j = g.refl(i);
        g.slot[i] = Some(pair[0] as u8);
        left[pair[0]] -= 1;
        if let Some(&d) = pair.get(1) {
            g.slot[j] = Some(d as u8);
            left[d] -= 1;
        }
    }
    if fill == Fill::Balanced {
        // One representative per free reflected pair, nearest the centre
        // first; the first met of each pair has the smaller angle.
        let want = left.iter().sum::<usize>() / 2;
        let mut taken = vec![false; n];
        let mut reps: Vec<usize> = Vec::with_capacity(want);
        for &i in &order {
            if reps.len() == want {
                break;
            }
            if g.free_pair(i) && !taken[g.refl(i)] {
                taken[i] = true;
                reps.push(i);
            }
        }
        // Member d's share of the r² total R is count_d·R/T; deal the
        // outermost pair to the member furthest below it (integer, ×T).
        let r2 = |i: usize| key(i).0;
        let r_all: i64 = (0..n).filter(|&i| g.slot[i].is_some()).map(r2).sum::<i64>() + 2 * reps.iter().map(|&i| r2(i)).sum::<i64>();
        let mut s = vec![0i64; counts.len()];
        for i in 0..n {
            if let Some(d) = g.slot[i] {
                s[usize::from(d)] += r2(i);
            }
        }
        reps.sort_by(|&a, &b| key(b).0.cmp(&key(a).0).then(key(a).1.total_cmp(&key(b).1)));
        for i in reps {
            let deficit = |d: usize| i64::from(counts[d]) * r_all - total as i64 * s[d];
            // Ties → lower d: `max_by_key` keeps the last maximum, so walk down.
            let Some(d) = (0..counts.len()).rev().filter(|&d| left[d] >= 2).max_by_key(|&d| deficit(d)) else { break };
            g.put_pair(i, d as u8);
            left[d] -= 2;
            s[d] += 2 * r2(i);
        }
    } else {
        let mut by_size: Vec<usize> = (0..left.len()).collect();
        by_size.sort_by_key(|&d| (Reverse(left[d]), d));
        for d in by_size {
            if fill == Fill::Dispersed {
                let cells: Vec<usize> = (0..n).collect();
                g.spread(&cells, d as u8, left[d]);
            } else {
                for &i in &order {
                    if left[d] >= 2 && g.free_pair(i) {
                        g.put_pair(i, d as u8);
                        left[d] -= 2;
                    }
                }
            }
        }
    }
    let exact = (0..n).all(|i| g.slot[i] == g.slot[n - 1 - i]);
    (g.slot, exact)
}

/// A row-major unit grid under construction. Point reflection through the
/// centre is index reversal: `(R-1-r)·C + (C-1-c) = RC-1-i`.
pub struct Grid {
    pub cols: usize,
    pub slot: Vec<Option<u8>>,
}

impl Grid {
    pub fn refl(&self, i: usize) -> usize {
        self.slot.len() - 1 - i
    }

    pub fn free_pair(&self, i: usize) -> bool {
        let j = self.refl(i);
        i != j && self.slot[i].is_none() && self.slot[j].is_none()
    }

    pub fn put_pair(&mut self, i: usize, s: u8) {
        let j = self.refl(i);
        self.slot[i] = Some(s);
        self.slot[j] = Some(s);
    }

    /// Cells of the `rows × cols` rectangle at `(r0, c0)`.
    pub fn rect(&self, (r0, c0, rows, cols): (usize, usize, usize, usize)) -> Vec<usize> {
        (r0..r0 + rows).flat_map(|r| (c0..c0 + cols).map(move |c| (r, c))).map(|(r, c)| r * self.cols + c).collect()
    }

    pub fn rc(&self, i: usize) -> (i64, i64) {
        ((i / self.cols) as i64, (i % self.cols) as i64)
    }

    /// Complete chessboard of C0..=Ctop over `rect` (`2^top` cells): Ctop on
    /// the black squares, C0/C1 on the most central pair, each other bit by
    /// [`Grid::spread`].
    pub fn chessboard(&mut self, rect: (usize, usize, usize, usize), top: u8) {
        let cells = self.rect(rect);
        for &i in &cells {
            let (r, c) = self.rc(i);
            if (r - rect.0 as i64 + c - rect.1 as i64) % 2 == 0 {
                self.slot[i] = Some(top);
            }
        }
        // The one-unit C0/C1 cannot be common-centroid: pin them to the free
        // pair nearest the centre before the spread claims it.
        let rows = self.slot.len() / self.cols;
        let off = |i: usize| {
            let (r, c) = self.rc(i);
            (2 * r - rows as i64 + 1).pow(2) + (2 * c - self.cols as i64 + 1).pow(2)
        };
        let mid = cells.iter().copied().filter(|&i| self.free_pair(i)).min_by_key(|&i| (off(i), i)).expect("C0/C1 pair");
        self.slot[mid] = Some(0);
        let j = self.refl(mid);
        self.slot[j] = Some(1);
        for b in (2..top).rev() {
            self.spread(&cells, b, 1 << (b - 1));
        }
    }

    /// `count` units of `s` from `cells`, in reflected pairs, each pair the
    /// free cell farthest from the units already placed (max dispersion).
    ///
    /// ponytail: greedy farthest-point, O(cells²) per bit; fine to 256 units.
    pub fn spread(&mut self, cells: &[usize], s: u8, count: usize) {
        let mut mine: Vec<(i64, i64)> = Vec::new();
        while mine.len() < count {
            let best = cells
                .iter()
                .copied()
                .filter(|&i| self.free_pair(i))
                .max_by_key(|&i| {
                    let (r, c) = self.rc(i);
                    let near = mine.iter().map(|&(qr, qc)| (r - qr).pow(2) + (c - qc).pow(2)).min();
                    (near.unwrap_or(i64::MAX), Reverse(i))
                })
                .expect("a free reflected pair");
            self.put_pair(best, s);
            mine.push(self.rc(best));
            mine.push(self.rc(self.refl(best)));
        }
    }

    /// Algorithm 1 step 2 for one corridor (`outer` minus `inner`): Ci pairs
    /// block-chessboard first, Ci+1 on what is left.
    ///
    /// ponytail: the paper walks the upper-half blocks explicitly; this orders
    /// the corridor by block parity and lets the reflection mirror it, so a
    /// block whose mirror has the other parity mixes colours.
    pub fn corridor(&mut self, outer: (usize, usize, usize, usize), inner: (usize, usize, usize, usize), i: u8, bs: usize) {
        let inside = self.rect(inner);
        let mut cells: Vec<usize> = self.rect(outer).into_iter().filter(|c| !inside.contains(c)).collect();
        cells.sort_by_key(|&c| {
            let (r, col) = self.rc(c);
            let (br, bc) = ((r as usize - outer.0) / bs, (col as usize - outer.1) / bs);
            ((br + bc) % 2, c)
        });
        let mut left = 1usize << (i - 1);
        for &c in &cells {
            if left > 0 && self.free_pair(c) {
                self.put_pair(c, i);
                left -= 2;
            }
        }
        for &c in &cells {
            self.slot[c].get_or_insert(i + 1);
        }
    }
}

/// Which end of a diffusion-legal CC row carries the shared, device-crossing
/// region: a `Drain` row never joins two devices across a source (so every
/// mismatched boundary is a drain, the quiet node for a ratioed mirror), a
/// `Source` row the reverse.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outer {
    Drain,
    Source,
}

/// Whether `s` never joins two different devices across the wrong region:
/// the boundary between fingers `i, i+1` is region `i+1`, even (`D`) when
/// `outer` is `Source`, odd (`S`) when `outer` is `Drain` (mosfet.rs
/// `is_s`).
#[must_use]
pub fn diffusion_legal(s: &[usize], outer: Outer) -> bool {
    s.windows(2).enumerate().all(|(i, w)| w[0] == w[1] || ((i + 1) % 2 == 1) == (outer == Outer::Drain))
}

/// Deals `p` reflected pairs of fingers at `seq[off..off + 2p]` in quads
/// (token `t` with its mirror `p − 1 − t`), each quad to the member whose
/// share of the centred weight `Σ(2i − n + 1)²` is furthest ahead of what it
/// already holds in `seq`. `h[d]` is device `d`'s remaining quad count.
fn deal(counts: &[u16], n: usize, seq: &mut [Option<usize>], p: usize, h: &mut [usize], off: usize) {
    let k = |i: usize| -> i64 {
        let v = 2 * i as i64 - n as i64 + 1;
        v * v
    };
    // Q is the weight not yet committed to any device: it starts at the
    // row's total and shrinks by every quad dealt (and by what the caller
    // pre-placed), so each pick compares against what is still in play.
    let mut q: i64 = (0..n).map(k).sum();
    let mut s = vec![0i64; counts.len()];
    for (i, slot) in seq.iter().enumerate() {
        if let Some(d) = *slot {
            let ki = k(i);
            s[d] += ki;
            q -= ki;
        }
    }
    for t in 0..p / 2 {
        let mut best_d = None;
        let mut best_val = i64::MIN;
        for (d, &hd) in h.iter().enumerate() {
            if hd == 0 {
                continue;
            }
            let val = i64::from(counts[d]) * q - n as i64 * s[d];
            if val > best_val {
                best_val = val;
                best_d = Some(d);
            }
        }
        let d = best_d.expect("a device with quads left");
        let fingers = [off + 2 * t, off + 2 * t + 1, off + 2 * (p - 1 - t), off + 2 * (p - 1 - t) + 1];
        let sum_k: i64 = fingers.iter().map(|&f| k(f)).sum();
        for &f in &fingers {
            seq[f] = Some(d);
        }
        s[d] += sum_k;
        q -= sum_k;
        h[d] -= 1;
    }
}

/// The second-moment residual (Hastings §13.3) of `seq`'s per-member finger
/// indices, treated as unit weights on a line: lower is a tighter centroid.
fn row_r2(seq: &[usize], ndev: usize) -> f64 {
    let mut by_member: Vec<Vec<moments::Pt>> = vec![Vec::new(); ndev];
    for (i, &d) in seq.iter().enumerate() {
        by_member[d].push(moments::Pt { x: i as f64, y: 0.0, w: 1.0, phi: (0, 0) });
    }
    let refs: Vec<&[moments::Pt]> = by_member.iter().map(Vec::as_slice).collect();
    moments::cancelled_order(&refs, 2, 0.0).1[2]
}

/// A diffusion-legal common-centroid finger row for a ratioed MOS mirror
/// (plan-02 §3 step 2–3): `counts[d]` fingers per device, mirror-symmetric,
/// with every inter-device boundary landing on `outer`'s region so no two
/// devices share the wrong diffusion. `None` when no legal row exists.
#[must_use]
pub fn diffusion_cc_row(counts: &[u16], outer: Outer) -> Option<Vec<usize>> {
    match outer {
        Outer::Drain => {
            let n: usize = counts.iter().map(|&c| usize::from(c)).sum();
            if n == 0 || n % 2 != 0 || counts.iter().any(|&c| c % 2 != 0) {
                return None;
            }
            let p = (n - 2) / 2;
            let mut best: Option<(Vec<usize>, f64)> = None;
            for e in 0..counts.len() {
                if counts[e] < 2 {
                    continue;
                }
                let mut c = counts.to_vec();
                c[e] -= 2;
                let pairs: Vec<usize> = c.iter().map(|&x| usize::from(x) / 2).collect();
                let odd: Vec<usize> = (0..pairs.len()).filter(|&d| pairs[d] % 2 == 1).collect();
                if odd.len() != p % 2 {
                    continue;
                }
                let mut seq: Vec<Option<usize>> = vec![None; n];
                seq[0] = Some(e);
                seq[n - 1] = Some(e);
                let mut h: Vec<usize> = pairs.iter().map(|&x| x / 2).collect();
                if p % 2 == 1 {
                    let odd_member = odd[0];
                    seq[1 + 2 * (p / 2)] = Some(odd_member);
                    seq[2 + 2 * (p / 2)] = Some(odd_member);
                }
                deal(&c, n, &mut seq, p, &mut h, 1);
                let seq: Vec<usize> = seq.into_iter().map(|s| s.expect("deal fills every finger")).collect();
                let r2 = row_r2(&seq, counts.len());
                match &best {
                    None => best = Some((seq, r2)),
                    Some((_, best_r2)) if r2 < best_r2 - 1e-9 => best = Some((seq, r2)),
                    Some(_) => {}
                }
            }
            best.map(|(seq, _)| seq)
        }
        Outer::Source => {
            if counts.iter().any(|&c| c % 2 != 0) {
                return None;
            }
            let n: usize = counts.iter().map(|&c| usize::from(c)).sum();
            let p = n / 2;
            let pairs: Vec<usize> = counts.iter().map(|&x| usize::from(x) / 2).collect();
            let odd: Vec<usize> = (0..pairs.len()).filter(|&d| pairs[d] % 2 == 1).collect();
            if odd.len() != p % 2 {
                return None;
            }
            let mut seq: Vec<Option<usize>> = vec![None; n];
            let mut h: Vec<usize> = pairs.iter().map(|&x| x / 2).collect();
            if p % 2 == 1 {
                let odd_member = odd[0];
                seq[2 * (p / 2)] = Some(odd_member);
                seq[2 * (p / 2) + 1] = Some(odd_member);
            }
            deal(counts, n, &mut seq, p, &mut h, 0);
            Some(seq.into_iter().map(|s| s.expect("deal fills every finger")).collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owners(slot: &[Option<u8>]) -> String {
        slot.iter().map(|s| s.map_or('_', |d| char::from(b'0' + d))).collect()
    }

    /// Every a:b with at most one odd count, on every grid offered, is exact:
    /// counts kept, each member's doubled offsets sum to zero, empties paired.
    #[test]
    fn every_single_odd_ratio_is_exact() {
        for a in 1..=4u16 {
            for b in 1..=16u16 {
                let counts = [a, b];
                if !cc_feasible(&counts) {
                    continue;
                }
                for (rows, cols) in grids(&counts, 3.0) {
                    let (slot, exact) = centro_assign(&counts, rows, cols, Fill::Balanced);
                    let at = format!("{counts:?} on {rows}×{cols}: {}", owners(&slot));
                    assert!(exact, "{at}");
                    let n = slot.len();
                    assert!((0..n).all(|i| slot[i].is_none() == slot[n - 1 - i].is_none()), "{at}");
                    for (d, &k) in counts.iter().enumerate() {
                        let mine: Vec<usize> = (0..n).filter(|&i| slot[i] == Some(d as u8)).collect();
                        assert_eq!(mine.len(), usize::from(k), "{at}: member {d}");
                        let off = mine.iter().fold((0i64, 0i64), |(r, c), &i| (r + 2 * (i / cols) as i64 - rows as i64 + 1, c + 2 * (i % cols) as i64 - cols as i64 + 1));
                        assert_eq!(off, (0, 0), "{at}: member {d} off centre");
                    }
                }
            }
        }
    }

    #[test]
    fn equal_pair_is_a_diagonal_quad() {
        assert_eq!(grids(&[2, 2], 3.0), [(2, 2), (3, 2), (1, 4)]);
        assert_eq!(centro_assign(&[2, 2], 2, 2, Fill::Balanced), (vec![Some(0), Some(1), Some(1), Some(0)], true));
    }

    /// 1:4 is the "+" cross on 3×3 (not Hastings' Fig. 10.24A, which needs a
    /// half-row offset no grid cell has); 1:6 and 1:8 fill outward.
    #[test]
    fn four_to_one_is_a_cross() {
        assert_eq!(grids(&[1, 4], 3.0), [(3, 3), (1, 5)]);
        let s = |c: &[u16], r, k| owners(&centro_assign(c, r, k, Fill::Balanced).0);
        assert_eq!(s(&[1, 4], 3, 3), "_1_101_1_");
        assert_eq!(s(&[1, 6], 3, 3), "11_101_11");
        assert_eq!(s(&[1, 8], 3, 3), "111101111");
        assert_eq!(s(&[4, 4, 4], 3, 4), "020121121020");
    }

    #[test]
    fn two_odd_members_are_reported_inexact() {
        assert!(!cc_feasible(&[1, 1, 2]));
        let (slot, exact) = centro_assign(&[1, 1, 2], 2, 2, Fill::Balanced);
        assert!(!exact, "{}", owners(&slot));
        assert_eq!(slot.iter().flatten().count(), 4);
    }

    #[test]
    fn grid_order_is_deterministic() {
        assert_eq!(grids(&[4, 4, 4], 3.0), [(3, 4), (4, 3), (5, 3), (2, 6), (1, 12)]);
        assert_eq!(grids(&[1, 8], 3.0), [(3, 3), (5, 3), (1, 9)]);
        assert_eq!(grids(&[1, 2, 2], 3.0), [(3, 3), (1, 5)]);
        assert_eq!(grids(&[3, 5], 3.0), [(3, 3), (2, 4), (4, 2), (1, 8)]);
        assert!(grids(&[0, 0], 3.0).is_empty());
    }

    fn letters(seq: &[usize]) -> String {
        seq.iter().map(|&d| (b'A' + d as u8) as char).collect()
    }

    fn devs(s: &str) -> Vec<usize> {
        s.bytes().map(|b| usize::from(b - b'A')).collect()
    }

    /// Every admitted row keeps counts, is a palindrome (mirror symmetry),
    /// and never joins two devices across the wrong region; an odd count
    /// never admits one.
    #[test]
    fn diffusion_rows_never_join_two_devices_on_a_drain() {
        let check = |counts: &[u16], outer: Outer| {
            if counts.iter().any(|&c| c % 2 == 1) {
                assert!(diffusion_cc_row(counts, outer).is_none(), "{counts:?} {outer:?}: odd count admitted a row");
                return;
            }
            if let Some(s) = diffusion_cc_row(counts, outer) {
                for (d, &cnt) in counts.iter().enumerate() {
                    assert_eq!(s.iter().filter(|&&x| x == d).count(), usize::from(cnt), "{counts:?} {outer:?}: {s:?}");
                }
                let n = s.len();
                assert!((0..n).all(|i| s[i] == s[n - 1 - i]), "{counts:?} {outer:?}: {s:?} not a palindrome");
                assert!(diffusion_legal(&s, outer), "{counts:?} {outer:?}: {s:?} not diffusion-legal");
            }
        };
        for outer in [Outer::Drain, Outer::Source] {
            for a in 1..=16u16 {
                for b in 1..=16u16 {
                    check(&[a, b], outer);
                }
            }
            for a in 1..=8u16 {
                for b in 1..=8u16 {
                    for c in 1..=8u16 {
                        check(&[a, b, c], outer);
                    }
                }
            }
        }
    }

    #[test]
    fn equal_counts_reproduce_the_old_orders() {
        assert_eq!(letters(&diffusion_cc_row(&[2, 2], Outer::Drain).unwrap()), "ABBA");
        assert_eq!(letters(&diffusion_cc_row(&[4, 4], Outer::Drain).unwrap()), "ABBAABBA");
        assert_eq!(letters(&diffusion_cc_row(&[6, 6], Outer::Drain).unwrap()), "ABBAABBAABBA");
        assert_eq!(letters(&diffusion_cc_row(&[8, 8], Outer::Drain).unwrap()), "ABBAABBAABBAABBA");
        assert_eq!(letters(&diffusion_cc_row(&[4, 4, 4], Outer::Drain).unwrap()), "ABBCCAACCBBA");
        assert_eq!(letters(&diffusion_cc_row(&[4, 4, 4, 4], Outer::Drain).unwrap()), "ABBCCDDAADDCCBBA");
        let new = diffusion_cc_row(&[8, 8, 8], Outer::Drain).unwrap();
        assert_eq!(letters(&new), "ABBCCCCAABBAABBAACCCCBBA");
        let old = devs("ABBCCAACCBBAABBCCAACCBBA");
        let (r2_new, r2_old) = (row_r2(&new, 3), row_r2(&old, 3));
        assert!(r2_new <= r2_old + 1e-9, "r2(new)={r2_new} should not exceed r2(old)={r2_old}");
    }

    #[test]
    fn two_to_four_is_a_bbbb_a() {
        assert_eq!(diffusion_cc_row(&[2, 4], Outer::Drain).unwrap(), vec![0, 1, 1, 1, 1, 0]);
    }

    #[test]
    fn two_to_four_source_is_bb_aa_bb() {
        assert_eq!(diffusion_cc_row(&[2, 4], Outer::Source).unwrap(), vec![1, 1, 0, 0, 1, 1]);
    }

    #[test]
    fn source_needs_even_pair_parity() {
        assert!(diffusion_cc_row(&[2, 2], Outer::Source).is_none());
        assert_eq!(letters(&diffusion_cc_row(&[4, 4], Outer::Source).unwrap()), "AABBBBAA");
    }

    #[test]
    fn one_to_two_has_no_row() {
        assert!(diffusion_cc_row(&[1, 2], Outer::Drain).is_none());
        assert!(diffusion_cc_row(&[1, 2], Outer::Source).is_none());
    }

    #[test]
    fn align_example_is_exact() {
        let s = diffusion_cc_row(&[2, 2, 4, 8, 8], Outer::Drain).unwrap();
        assert_eq!(letters(&s), "ADDEECCEEDDBBDDEECCEEDDA");
        let n = s.len() as i64;
        for d in 0..5usize {
            let sum: i64 = s.iter().enumerate().filter(|&(_, &x)| x == d).map(|(i, _)| 2 * i as i64 - (n - 1)).sum();
            assert_eq!(sum, 0, "member {d} off centre");
        }
        assert_eq!(letters(&diffusion_cc_row(&[4, 8], Outer::Drain).unwrap()), "ABBBBAABBBBA");
    }
}
