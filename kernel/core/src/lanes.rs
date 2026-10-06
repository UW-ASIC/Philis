//! The one SIMD layer. Every vector loop in the workspace goes through here.
//!
//! [`fold_i32`] owns chunking and the tail: a kernel writes its lane body once,
//! and the last partial chunk is loaded zero-padded with a `valid` mask, so no
//! scalar tail duplicates the body. Kernels are generic over `S: Simd` and
//! `#[inline(always)]`; the public wrappers `dispatch!` once on the cached
//! [`level`], so every kernel is compiled per instruction set and picked at
//! runtime.
//!
//! Lane bodies passed to [`fold_i32`] must be `#[inline(always)]` closures, or
//! they compile outside the `target_feature` region and every op becomes a call.

use std::sync::OnceLock;

pub use fearless_simd::{dispatch, prelude::*, Level};

/// Widest native `i32` vector across supported levels (AVX-512: 16 lanes).
const MAX_LANES: usize = 16;

/// The CPU's SIMD level, detected once.
#[inline]
#[must_use]
pub fn level() -> Level {
    static LEVEL: OnceLock<Level> = OnceLock::new();
    *LEVEL.get_or_init(Level::new)
}

/// Fold over equal-length `i32` columns, one native vector per column per step.
///
/// `body(acc, base, lanes, valid)`: `lanes[k]` holds `cols[k][base..base + LEN]`;
/// on the last partial chunk the missing lanes read `0` and are clear in
/// `valid`. Bodies must ignore invalid lanes (usually by `select`).
///
/// # Panics
/// If the columns differ in length.
#[inline(always)]
pub fn fold_i32<S: Simd, const K: usize, A>(
    simd: S,
    cols: [&[i32]; K],
    mut acc: A,
    mut body: impl FnMut(A, usize, [S::i32s; K], S::mask32s) -> A,
) -> A {
    let n = cols.first().map_or(0, |c| c.len());
    assert!(cols.iter().all(|c| c.len() == n), "fold_i32: ragged columns");
    let w = S::i32s::LEN;
    let full = n - n % w;
    let all = S::mask32s::splat(simd, true);
    let mut i = 0;
    while i < full {
        acc = body(acc, i, cols.map(|c| S::i32s::from_slice(simd, &c[i..i + w])), all);
        i += w;
    }
    if i < n {
        let rem = n - i;
        let lanes = cols.map(|c| {
            let mut pad = [0i32; MAX_LANES];
            pad[..rem].copy_from_slice(&c[i..]);
            S::i32s::from_slice(simd, &pad[..w])
        });
        acc = body(acc, i, lanes, S::mask32s::from_bitmask(simd, (1u64 << rem) - 1));
    }
    acc
}

/// Borrowed box columns: centre `(x, y)`, half-extents `(hw, hh)`, nm. The
/// shape [`crate::Layout`] already stores, so borrowing it is free.
/// All four columns must have the same length.
#[derive(Clone, Copy)]
pub struct Boxes<'a> {
    /// Centre x per box, nm.
    pub x: &'a [i32],
    /// Centre y per box, nm.
    pub y: &'a [i32],
    /// Half-width per box, nm.
    pub hw: &'a [i32],
    /// Half-height per box, nm.
    pub hh: &'a [i32],
}

/// One box, same convention as [`Boxes`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Box4 {
    /// Centre x, nm.
    pub x: i32,
    /// Centre y, nm.
    pub y: i32,
    /// Half-width, nm.
    pub hw: i32,
    /// Half-height, nm.
    pub hh: i32,
}

/// `Σ_b ox·oy` over every box `b` whose clearance-inflated overlap with `q` is
/// positive on both axes, nm². Exact (i64 products). Includes `q` itself if it
/// is one of `boxes`; the caller subtracts that term.
///
/// Precondition: per-axis sums `|q.x − x| ` and `q.hw + hw + clearance` fit
/// `i32` (coordinates within ±1e9 nm with sub-mm boxes always do).
///
/// # Panics
/// If the columns differ in length.
#[must_use]
pub fn overlap_area(boxes: Boxes, q: Box4, clearance: i32) -> i64 {
    dispatch!(level(), simd => overlap_area_simd(simd, boxes, q, clearance))
}

#[inline(always)]
fn overlap_area_simd<S: Simd>(simd: S, b: Boxes, q: Box4, clearance: i32) -> i64 {
    let qx = S::i32s::splat(simd, q.x);
    let qy = S::i32s::splat(simd, q.y);
    let rx = S::i32s::splat(simd, q.hw + clearance);
    let ry = S::i32s::splat(simd, q.hh + clearance);
    let zero = S::i32s::splat(simd, 0);
    let acc = fold_i32(
        simd,
        [b.x, b.y, b.hw, b.hh],
        S::i64s::splat(simd, 0),
        #[inline(always)]
        |acc, _, [x, y, hw, hh], valid| {
            let ox = rx + hw - (qx - x).abs();
            let oy = ry + hh - (qy - y).abs();
            let hit = valid & ox.simd_gt(zero) & oy.simd_gt(zero);
            let (ox_lo, ox_hi) = hit.select(ox, zero).widen();
            let (oy_lo, oy_hi) = hit.select(oy, zero).widen();
            acc + ox_lo * oy_lo + ox_hi * oy_hi
        },
    );
    acc.reduce_sum()
}

/// `(min, max)` of `xs`; `None` when empty.
#[must_use]
pub fn min_max(xs: &[i32]) -> Option<(i32, i32)> {
    if xs.is_empty() {
        return None;
    }
    Some(dispatch!(level(), simd => min_max_simd(simd, xs)))
}

#[inline(always)]
fn min_max_simd<S: Simd>(simd: S, xs: &[i32]) -> (i32, i32) {
    let (lo, hi) = fold_i32(
        simd,
        [xs],
        (S::i32s::splat(simd, i32::MAX), S::i32s::splat(simd, i32::MIN)),
        #[inline(always)]
        |(lo, hi), _, [v], valid| (lo.min(valid.select(v, lo)), hi.max(valid.select(v, hi))),
    );
    (lo.reduce_min(), hi.reduce_max())
}

/// Append to `out` every row where `now` and `was` differ in any column,
/// ascending.
///
/// # Panics
/// If the columns differ in length.
pub fn changed(now: Boxes, was: Boxes, out: &mut Vec<u32>) {
    dispatch!(level(), simd => changed_simd(simd, now, was, out));
}

#[inline(always)]
fn changed_simd<S: Simd>(simd: S, a: Boxes, b: Boxes, out: &mut Vec<u32>) {
    fold_i32(
        simd,
        [a.x, a.y, a.hw, a.hh, b.x, b.y, b.hw, b.hh],
        out,
        #[inline(always)]
        |out, base, [ax, ay, aw, ah, bx, by, bw, bh], valid| {
            let same = ax.simd_eq(bx) & ay.simd_eq(by) & aw.simd_eq(bw) & ah.simd_eq(bh);
            let mut bits = (valid & !same).to_bitmask();
            while bits != 0 {
                out.push((base + bits.trailing_zeros() as usize) as u32);
                bits &= bits - 1;
            }
            out
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic LCG; enough spread to hit both overlap signs.
    fn noise(n: usize, seed: u64, span: i32) -> Vec<i32> {
        let mut s = seed;
        (0..n)
            .map(|_| {
                s = s.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
                ((s >> 33) as i32).rem_euclid(span)
            })
            .collect()
    }

    fn overlap_oracle(b: Boxes, q: Box4, c: i32) -> i64 {
        (0..b.x.len())
            .map(|i| {
                let ox = i64::from(q.hw + b.hw[i] + c - (q.x - b.x[i]).abs());
                let oy = i64::from(q.hh + b.hh[i] + c - (q.y - b.y[i]).abs());
                if ox > 0 && oy > 0 { ox * oy } else { 0 }
            })
            .sum()
    }

    /// Lengths 0..=3·16 cover the empty case and every tail width on every level.
    #[test]
    fn kernels_match_their_scalar_oracles_at_every_tail_length() {
        for n in 0..=48 {
            let (x, y) = (noise(n, 1, 200_000), noise(n, 2, 200_000));
            let (hw, hh) = (noise(n, 3, 40_000), noise(n, 4, 40_000));
            let b = Boxes { x: &x, y: &y, hw: &hw, hh: &hh };
            let q = Box4 { x: 100_000, y: 90_000, hw: 30_000, hh: 20_000 };
            assert_eq!(overlap_area(b, q, 250), overlap_oracle(b, q, 250), "overlap n={n}");

            let want = x.iter().copied().min().zip(x.iter().copied().max());
            assert_eq!(min_max(&x), want, "min_max n={n}");

            let mut moved = x.clone();
            let flips: Vec<u32> = (0..n as u32).filter(|i| i % 3 == 1).collect();
            for &i in &flips {
                moved[i as usize] += 5;
            }
            let mut got = Vec::new();
            let now = Boxes { x: &moved, ..b };
            changed(now, b, &mut got);
            assert_eq!(got, flips, "changed n={n}");
        }
    }

    /// A 1e6-nm square overlapping itself is 4e12 nm²: past i32, and past f32's
    /// exact range. The kernel must stay exact.
    #[test]
    fn overlap_products_do_not_overflow() {
        let v = [1_000_000];
        let b = Boxes { x: &[0], y: &[0], hw: &v, hh: &v };
        let q = Box4 { x: 0, y: 0, hw: 1_000_000, hh: 1_000_000 };
        assert_eq!(overlap_area(b, q, 0), 4_000_000_000_000);
    }
}
