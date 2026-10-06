//! Placement primitives shared by `gp` and `dp`: RNG, pin-based nets/HPWL,
//! overlap, the tiered objective over `Requirements`, canvas and initial layout.
//!
//! All coordinates are nm, cell positions are centres (`Layout::x`/`y`), and
//! every function here is deterministic: the only randomness is the caller's
//! [`SplitMix64`].

use analog::Requirements;
use pnr_core::ids::DeviceId;
use pnr_core::{Layout, Macro, Report, Violation};

use crate::{Prices, VariantSpace};

/// Deterministic SplitMix64 PRNG (Steele, Lea & Flood 2014): same inputs +
/// seed ⇒ same placement. The field is the raw state, advanced by the golden
/// gamma on every draw; any `u64`, `0` included, is a valid seed.
pub struct SplitMix64(pub u64);

impl SplitMix64 {
    /// Generator whose first draw is the SplitMix64 output for state `seed`.
    #[inline]
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// Next 64 uniformly distributed bits; full period 2⁶⁴.
    #[inline(always)]
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, 1)` on the 2⁻⁵³ grid (top 53 bits of one draw).
    #[inline(always)]
    pub fn f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform in `[0, 1)` on the 2⁻²⁴ grid (top 24 bits of one draw).
    #[inline(always)]
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u32 << 24) as f32
    }

    /// Index in `[0, n)` by modulo (bias ≤ n/2⁶⁴, negligible for cell counts);
    /// `n == 0` ⇒ `0`. Consumes one draw either way.
    #[inline(always)]
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }

    /// Uniform in `[-r, r)` for `r ≥ 0` (one [`Self::f32`] draw, scaled).
    #[inline(always)]
    pub fn centered(&mut self, r: f32) -> f32 {
        (self.f32() * 2.0 - 1.0) * r
    }
}

/// CSR net → (device, pin offset from the device's bbox centre, pre-orient).
/// Built from macro pins; nets touching < 2 devices are dropped, and a device
/// with several pins on one net contributes their centroid. Membership is
/// variant-invariant, so a reshape only patches offsets ([`Nets::reshape_cell`]).
///
/// Rows (retained nets) are dense `0..count()`, in ascending `NetId` order;
/// "item" `k` is one (device, offset) entry of the flat arrays.
pub struct Nets {
    /// Row `i`'s items are `start[i]..start[i + 1]`; `len = count() + 1`, `start[0] = 0`.
    start: Vec<u32>,
    /// Device index of each item, ascending within a row.
    items: Vec<u32>,
    /// Pin-centroid offset of each item from its device's bbox centre, R0 frame, nm.
    off: Vec<(i32, i32)>,
    /// `NetId` of each retained row.
    net: Vec<u16>,
    /// HPWL weight of each row (see [`Nets::weigh`]); `1` unweighted.
    weight: Vec<f32>,
}

impl Nets {
    /// Nets of `macros`, device `i` = `macros[i]`, all weights `1`. Offsets
    /// are integer means (truncated toward zero) of pin-rect centres. O(pins +
    /// max `NetId`).
    #[must_use]
    pub fn from_macros(macros: &[Macro]) -> Self {
        let Some(max_net) = macros.iter().flat_map(|m| &m.pins).map(|p| p.net.0).max() else {
            return Self { start: vec![0], items: Vec::new(), off: Vec::new(), net: Vec::new(), weight: Vec::new() };
        };
        let mut per_net: Vec<Vec<(u32, i32, i32)>> = vec![Vec::new(); max_net as usize + 1];
        for (di, m) in macros.iter().enumerate() {
            let (cx, cy) = bbox_centre(m);
            for p in &m.pins {
                per_net[p.net.0 as usize].push((
                    di as u32,
                    p.at.x + p.at.w / 2 - cx,
                    p.at.y + p.at.h / 2 - cy,
                ));
            }
        }
        let mut start = vec![0u32];
        let mut items = Vec::new();
        let mut off = Vec::new();
        let mut net = Vec::new();
        for (ni, pins) in per_net.iter_mut().enumerate() {
            pins.sort_unstable_by_key(|&(d, ..)| d);
            let row0 = items.len();
            for dev in pins.chunk_by(|a, b| a.0 == b.0) {
                let n = dev.len() as i64;
                let sx: i64 = dev.iter().map(|p| i64::from(p.1)).sum();
                let sy: i64 = dev.iter().map(|p| i64::from(p.2)).sum();
                items.push(dev[0].0);
                off.push(((sx / n) as i32, (sy / n) as i32));
            }
            if items.len() - row0 >= 2 {
                start.push(items.len() as u32);
                net.push(ni as u16);
            } else {
                items.truncate(row0);
                off.truncate(row0);
            }
        }
        let weight = vec![1.0; net.len()];
        Self { start, items, off, net, weight }
    }

    /// Weight each net's HPWL by `by_net[NetId]` (missing = 1): a net whose
    /// parasitic hurts the circuit more is pulled shorter (Lampaert 1999
    /// eq.2.12–2.13, `ΔP = Σ S·Δx`). See [`crate::net_weights`].
    #[must_use]
    pub fn weigh(mut self, by_net: &[f32]) -> Self {
        for (w, &n) in self.weight.iter_mut().zip(&self.net) {
            *w = by_net.get(usize::from(n)).copied().unwrap_or(1.0);
        }
        self
    }

    /// HPWL weight of net row `i`.
    ///
    /// # Panics
    /// When `i >= count()`.
    #[inline]
    #[must_use]
    pub fn weight(&self, i: usize) -> f32 {
        self.weight[i]
    }

    /// Retained net rows (nets on ≥ 2 devices).
    #[inline]
    #[must_use]
    pub fn count(&self) -> usize {
        self.start.len() - 1
    }

    /// Devices on net `i`, ascending, each once.
    #[inline]
    #[must_use]
    pub fn row(&self, i: usize) -> &[u32] {
        &self.items[self.span(i)]
    }

    /// Flat item indices of net `i`, for [`Nets::pin`]; parallel to [`Nets::row`].
    ///
    /// # Panics
    /// When `i >= count()`.
    #[inline]
    #[must_use]
    pub fn span(&self, i: usize) -> std::ops::Range<usize> {
        self.start[i] as usize..self.start[i + 1] as usize
    }

    /// Absolute pin position of item `k`: centre + offset turned by `orient`
    /// (a short `orient` table reads as unturned).
    #[inline]
    #[must_use]
    pub fn pin(&self, k: usize, l: &Layout) -> (i32, i32) {
        let d = self.items[k] as usize;
        let (dx, dy) = self.off[k];
        let (dx, dy) = l.orient.get(d).map_or((dx, dy), |o| o.apply(dx, dy));
        (l.x[d] + dx, l.y[d] + dy)
    }

    /// Bounding box `(x0, x1, y0, y1)` of net `i`'s pins under `l`, nm.
    #[inline]
    #[must_use]
    pub fn pin_bbox(&self, i: usize, l: &Layout) -> (i32, i32, i32, i32) {
        let (mut x0, mut x1, mut y0, mut y1) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
        for k in self.span(i) {
            let (px, py) = self.pin(k, l);
            (x0, x1, y0, y1) = (x0.min(px), x1.max(px), y0.min(py), y1.max(py));
        }
        (x0, x1, y0, y1)
    }

    /// Repoint cell `c`'s offsets at `m`'s pins on the nets `rows` (= `cell_nets()[c]`).
    /// A net `m` has no pin for keeps its old offset (a debug assertion: a
    /// variant must keep every terminal); a row `c` is not on is skipped.
    /// O(Σ row length + |rows|·pins).
    pub fn reshape_cell(&mut self, c: usize, rows: &[u32], m: &Macro) {
        let (cx, cy) = bbox_centre(m);
        for &ni in rows {
            let ni = ni as usize;
            let want = self.net[ni];
            let Some(k) = self.span(ni).find(|&k| self.items[k] as usize == c) else {
                continue;
            };
            let (mut n, mut sx, mut sy) = (0i64, 0i64, 0i64);
            for p in m.pins.iter().filter(|p| p.net.0 == want) {
                sx += i64::from(p.at.x + p.at.w / 2 - cx);
                sy += i64::from(p.at.y + p.at.h / 2 - cy);
                n += 1;
            }
            debug_assert!(n > 0, "reshape_cell: cell {c}'s new variant has no pin on net {want}");
            if n > 0 {
                self.off[k] = ((sx / n) as i32, (sy / n) as i32);
            }
        }
    }

    /// Device → net rows it touches, ascending; devices past `n_devices` are
    /// a caller bug (index panic).
    #[must_use]
    pub fn cell_nets(&self, n_devices: usize) -> Vec<Vec<u32>> {
        let mut out = vec![Vec::new(); n_devices];
        for ni in 0..self.count() {
            for &c in self.row(ni) {
                out[c as usize].push(ni as u32);
            }
        }
        out
    }
}

#[inline]
fn bbox_centre(m: &Macro) -> (i32, i32) {
    (m.bbox.x + m.bbox.w / 2, m.bbox.y + m.bbox.h / 2)
}

/// Bounding-box HPWL over pin positions, weighted per net ([`Nets::weigh`]),
/// nm; exact for any `i32` coordinates (no intermediate overflow). O(items).
#[must_use]
pub fn hpwl(nets: &Nets, l: &Layout) -> f64 {
    let mut total = 0.0f64;
    for ni in 0..nets.count() {
        let (x0, x1, y0, y1) = nets.pin_bbox(ni, l);
        total += f64::from(nets.weight[ni]) * f64::from((x1 - x0) + (y1 - y0));
    }
    total
}

/// Clearance-inflated overlap of `a` and `b`, nm²: nonzero iff their edge gap
/// is under `clearance` on both axes (`clearance = 0` is plain overlap).
#[inline]
#[must_use]
pub fn encroach(l: &Layout, a: usize, b: usize, clearance: i32) -> f64 {
    let ox = (l.hw[a] + l.hw[b] + clearance) - (l.x[a] - l.x[b]).abs();
    let oy = (l.hh[a] + l.hh[b] + clearance) - (l.y[a] - l.y[b]).abs();
    if ox > 0 && oy > 0 {
        f64::from(ox) * f64::from(oy)
    } else {
        0.0
    }
}

/// [`encroach`] summed over all pairs; `+0.0` with fewer than two cells (an
/// empty f64 `sum` is `-0.0`, which reports print as "-0"). O(n²).
#[must_use]
pub fn encroachment(l: &Layout, clearance: i32) -> f64 {
    pair_sum(l.x.len(), |a, b| encroach(l, a, b, clearance))
}

/// `Σ f(a, b)` over unordered pairs `a < b < n`, starting from `+0.0`.
pub(crate) fn pair_sum(n: usize, f: impl Fn(usize, usize) -> f64) -> f64 {
    (0..n).flat_map(|a| (a + 1..n).map(move |b| (a, b))).fold(0.0, |t, (a, b)| t + f(a, b))
}

/// PEX-tier objective, dimensionless: `Σ criticality·cost` over `reqs.cost`
/// plus the priced budget residuals `Σ −λ_b·residual_b` (residual unclamped,
/// so a slack budget earns a small reward at a nonzero price). Prices must be
/// bound to `reqs` ([`Prices::bind`]); an unbound batch prices at 0.
#[inline]
#[must_use]
pub fn analog_cost(reqs: &Requirements<Layout>, l: &Layout, prices: &Prices) -> f32 {
    let objective: f32 = reqs.cost.iter().map(|b| b.criticality(l) * b.cost(l)).sum();
    let priced: f64 = reqs
        .budget
        .iter()
        .enumerate()
        .map(|(bi, b)| f64::from(prices.weight_of(bi)) * b.residual(l))
        .sum();
    objective + priced as f32
}

/// Augmented-Lagrangian cost (PLC-09): `Σ crit·cost` over `reqs.cost` plus
/// `Σ_b (−λ_b·r_b + ½·ρ_b·r_b²)`, `r_b` = budget `b`'s residual clamped at 0.
/// The quadratic term pulls before λ has grown.
#[must_use]
pub fn augmented_cost(reqs: &Requirements<Layout>, l: &Layout, prices: &Prices) -> f64 {
    let objective: f64 = reqs.cost.iter().map(|b| f64::from(b.criticality(l) * b.cost(l))).sum();
    let priced: f64 = reqs
        .budget
        .iter()
        .enumerate()
        .map(|(bi, b)| {
            let r = b.residual(l).max(0.0);
            f64::from(prices.weight_of(bi)) * r + 0.5 * f64::from(prices.rho_of(bi)) * r * r
        })
        .sum();
    objective + priced
}

/// PEX-tier energy, dimensionless (PLC-18): `HPWL / L_ref + analog_cost`.
#[must_use]
pub fn pex(nets: &Nets, reqs: &Requirements<Layout>, l: &Layout, prices: &Prices) -> f64 {
    hpwl(nets, l) / f64::from(l.l_ref()) + f64::from(analog_cost(reqs, l, prices))
}

/// Φ over `reqs.hard`: `(violating batches, Σ their residuals)`; the same
/// numbers [`report`] writes.
#[must_use]
pub fn analog_phi(reqs: &Requirements<Layout>, l: &Layout) -> (usize, f64) {
    let mut batches = 0usize;
    let mut margin = 0.0f64;
    for b in &reqs.hard {
        if b.violations(l) > 0 {
            batches += 1;
            margin += b.residual(l);
        }
    }
    (batches, margin)
}

/// Θ: summed budget residual, as each batch reports it (unclamped).
#[inline]
#[must_use]
pub fn analog_theta(reqs: &Requirements<Layout>, l: &Layout) -> f64 {
    reqs.budget.iter().map(|b| b.residual(l)).sum()
}

/// Σ hard-rule violations; zero ⇒ analog-legal.
#[inline]
#[must_use]
pub fn analog_violations(reqs: &Requirements<Layout>, l: &Layout) -> u32 {
    reqs.hard.iter().map(|b| b.violations(l)).sum()
}

/// Stage report: one hard entry per violating hard batch plus residual device
/// overlap and clearance-only encroachment (each only above 0.5 nm²), one
/// budget entry per positive residual, cost = HPWL/L_ref + analog cost
/// ([`pex`]). O(n²) in cells for the overlap terms.
#[must_use]
pub fn report(nets: &Nets, reqs: &Requirements<Layout>, l: &Layout, prices: &Prices, rules: &crate::PlaceRules) -> Report {
    let mut hard_violations: Vec<Violation> = reqs
        .hard
        .iter()
        .enumerate()
        .filter(|(_, b)| b.violations(l) > 0)
        .map(|(bi, b)| {
            Violation::from_residual(format!("{}analog hard {bi} ({:?})", Violation::BATCH, b.kind()), b.residual(l))
        })
        .collect();
    let budget_violations = reqs
        .budget
        .iter()
        .enumerate()
        .filter_map(|(bi, b)| {
            let residual = b.residual(l);
            (residual > 0.0)
                .then(|| Violation::from_residual(format!("{}analog budget {bi}", Violation::BATCH), residual))
        })
        .collect();
    let ov = encroachment(l, 0);
    if ov > 0.5 {
        hard_violations.push(Violation { rule: "device overlap".into(), margin: ov as i64 });
    }
    let residue = rules.encroachment(l) - ov;
    if residue > 0.5 {
        hard_violations.push(Violation { rule: "clearance encroachment".into(), margin: residue.ceil() as i64 });
    }
    let cost = pex(nets, reqs, l, prices) as f32;
    Report { hard_violations, budget_violations, cost }
}

/// Half-extents `(hw, hh)` of a drawn macro, nm, truncated — the
/// `Layout::variant` ⇒ `hw`/`hh` rule.
#[inline]
#[must_use]
pub fn variant_extents(m: &Macro) -> (i32, i32) {
    (m.bbox.w / 2, m.bbox.h / 2)
}

/// [`variant_extents`] of every macro, as the `(hw, hh)` columns of a [`Layout`].
#[must_use]
pub fn half_extents(macros: &[Macro]) -> (Vec<i32>, Vec<i32>) {
    macros.iter().map(variant_extents).unzip()
}

/// Per cell, `variants[i].alternatives[variant[i]]` (cloned); `macros[i]`
/// when `variants`, `variant` or the alternatives are too short. Output
/// length is `macros.len()`.
#[must_use]
pub fn choose_variants(macros: &[Macro], variants: &[VariantSpace], variant: &[u16]) -> Vec<Macro> {
    macros
        .iter()
        .enumerate()
        .map(|(i, m)| {
            variants
                .get(i)
                .and_then(|v| v.alternatives.get(variant[i] as usize))
                .unwrap_or(m)
                .clone()
        })
        .collect()
}

/// Square die side (nm): `√(total footprint area / utilization)`, at least
/// the largest footprint side (1000 nm with no cells), at least 1, rounded up
/// to `grid` (`grid ≤ 0` reads as 1). `utilization` is clamped to
/// `[0.05, 0.95]`. Computed in wide arithmetic: no overflow for any extents
/// whose result fits `i32`.
#[must_use]
pub fn canvas_side(hw: &[i32], hh: &[i32], utilization: f32, grid: i32) -> i32 {
    let total: f64 = hw.iter().zip(hh).map(|(&w, &h)| f64::from(2 * w) * f64::from(2 * h)).sum();
    let area_side = (total / f64::from(utilization.clamp(0.05, 0.95))).sqrt().ceil() as i32;
    let largest = hw.iter().zip(hh).map(|(&w, &h)| (2 * w).max(2 * h)).max().unwrap_or(1000);
    let side = area_side.max(largest).max(1);
    let g = grid.max(1);
    (side + g - 1) / g * g
}

/// Initial layout: devices jittered within `0.15·side` of the die centre
/// (two [`SplitMix64::centered`] draws per device, x then y), single-device
/// groups, `n_axes` (at least one) axes at the centre, every branch `false`,
/// orient `R0`, no power, no units.
/// `variant` becomes `Layout::variant` as given (callers pass one per cell).
/// The cell count must fit `u16`: group ids are `DeviceId(u16)`.
#[must_use]
pub fn initial_layout(macros: &[Macro], variant: Vec<u16>, side: i32, n_axes: usize, rng: &mut SplitMix64) -> Layout {
    let n = macros.len();
    let (hw, hh) = half_extents(macros);
    let c = side / 2;
    let jitter = side as f32 * 0.15;
    let mut x = Vec::with_capacity(n);
    let mut y = Vec::with_capacity(n);
    for _ in 0..n {
        x.push(c + rng.centered(jitter) as i32);
        y.push(c + rng.centered(jitter) as i32);
    }
    Layout {
        x,
        y,
        hw,
        hh,
        variant,
        axis: vec![c; n_axes.max(1)],
        branch: vec![false; n],
        groups: (0..n).map(|i| vec![DeviceId(i as u16)]).collect(),
        orient: vec![pnr_core::geom::Orient::default(); n],
        power_uw: vec![0; n],
        temp_mc: vec![0; n],
        units: Default::default(),
    }
}

/// Clamp a centre so its footprint stays inside `[0, side]`; a footprint
/// wider than the die sits at `half` (its low edge on 0).
#[inline]
#[must_use]
pub fn clamp_to_die(c: i32, half: i32, side: i32) -> i32 {
    c.clamp(half, (side - half).max(half))
}

/// Snap `v` to the nearest multiple of `grid` (`grid ≤ 0` reads as 1), ties
/// away from zero; exact integer arithmetic for every `i32`, saturating at
/// the `i32` range.
#[inline]
#[must_use]
pub fn snap(v: i32, grid: i32) -> i32 {
    let g = grid.max(1);
    ((v as f32 / g as f32).round() as i32) * g
}
