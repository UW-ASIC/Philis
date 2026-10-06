//! Layout-dependent environment of matched devices (placement tier, budget).

use crate::rule::RuleBatch;
use pnr_core::{place_rect, DeviceId, Layout, Rect};

/// Allowed relative difference between members' distances inside range: a
/// design tolerance (no deck models WPE/OSE magnitudes), not a process number.
pub const ENV_TOL: f32 = 0.2;

/// One matched pair's measured surroundings, from placed geometry: the mean
/// distance of each member's channels to the nearest nwell edge (inside or
/// out), and of each member's diffusion to the nearest foreign diffusion.
/// `f32::INFINITY` = none on the die.
///
/// The ranges are the deck's: `wpe_min_nm` its moderate WPE clearance (the
/// distance a well edge must keep, Hastings §13.3 rule 19), `ose_range_nm` its
/// moderate LOD extension (past which STI stress has faded). `0` = the deck
/// gives none, and that half reads unknown.
#[derive(Clone, Copy, Debug)]
pub struct Surroundings {
    /// Per member, mean distance of its channels to the nearest nwell edge, nm.
    pub wpe_nm: [f32; 2],
    /// Per member, gap from its cell's diffusion to the nearest foreign one, nm.
    pub ose_nm: [f32; 2],
    /// Deck's moderate WPE clearance, nm; `≤ 0` = none given.
    pub wpe_min_nm: f32,
    /// Deck's moderate LOD extension, nm; `≤ 0` = none given.
    pub ose_range_nm: f32,
}

impl Surroundings {
    /// Spent fraction, the worst of: a member's well edge nearer than
    /// `wpe_min_nm`; the members' WPE distances (within ten times it)
    /// differing by more than [`ENV_TOL`]; their OSE distances (within
    /// `ose_range_nm`) likewise. Distances past range count as the range: the
    /// effect has faded. `None` when the deck gives neither range.
    #[must_use]
    pub fn usage(&self) -> Option<f32> {
        let skew = |d: [f32; 2], range: f32| {
            let (a, b) = (d[0].min(range), d[1].min(range));
            (a - b).abs() / a.max(b).max(1.0) / ENV_TOL
        };
        let wpe = (self.wpe_min_nm > 0.0).then(|| {
            let near = self.wpe_nm.iter().map(|&d| self.wpe_min_nm / d.max(1.0)).fold(0.0f32, f32::max);
            near.max(skew(self.wpe_nm, 10.0 * self.wpe_min_nm))
        });
        let ose = (self.ose_range_nm > 0.0).then(|| skew(self.ose_nm, self.ose_range_nm));
        match (wpe, ose) {
            (None, None) => None,
            (a, b) => Some(a.unwrap_or(0.0).max(b.unwrap_or(0.0))),
        }
    }
}

/// Every matched pair's [`Surroundings`], measured once per placed layout.
/// State-free (the numbers are precomputed), so it reports on any stage. A
/// pair whose deck gives neither range counts as unknown, never as a pass.
#[derive(Clone, Debug, Default)]
pub struct Environment(pub Vec<Surroundings>);

impl<S> RuleBatch<S> for Environment {
    fn cost(&self, _: &S) -> f32 {
        self.0.iter().filter_map(Surroundings::usage).sum()
    }
    fn violations(&self, _: &S) -> u32 {
        self.0.iter().filter(|s| s.usage().is_some_and(|u| u > 1.0)).count() as u32
    }
    fn residual(&self, _: &S) -> f64 {
        self.0.iter().filter_map(Surroundings::usage).map(|u| f64::from((u - 1.0).max(0.0))).sum()
    }
    fn unknown(&self, _: &S) -> u32 {
        self.0.iter().filter(|s| s.usage().is_none()).count() as u32
    }
    fn kind(&self) -> &'static str {
        "Environment"
    }
    fn count(&self) -> usize {
        self.0.len()
    }
    fn worst_usage(&self, _: &S) -> Option<f32> {
        self.0.iter().filter_map(Surroundings::usage).reduce(f32::max)
    }
}

/// Per cell, per variant, in the macro's own frame: its bbox, n-well rects
/// and diff rects. Indexed `[cell][variant]`; the three tables are parallel,
/// and every variant a [`Layout::variant`] names must exist in each.
#[derive(Debug, Default)]
pub struct EnvGeo {
    /// Macro bbox per `[cell][variant]` (the frame the rects below live in).
    pub bbox: Vec<Vec<Rect>>,
    /// N-well rects per `[cell][variant]`.
    pub wells: Vec<Vec<Vec<Rect>>>,
    /// Diffusion rects per `[cell][variant]`.
    pub diffs: Vec<Vec<Vec<Rect>>>,
}

/// Every matched pair's [`Surroundings`], re-measured on each layout it is
/// asked about: the placement-tier batch (PLC-29), so gp/dp see WPE/OSE in Θ.
#[derive(Clone, Debug)]
pub struct LiveEnvironment {
    /// `(member a, member b, cell of a, cell of b)`.
    pub pairs: Vec<(DeviceId, DeviceId, u16, u16)>,
    /// Cell geometry shared across starts.
    pub geo: std::sync::Arc<EnvGeo>,
    /// See [`Surroundings::wpe_min_nm`].
    pub wpe_min_nm: f32,
    /// See [`Surroundings::ose_range_nm`].
    pub ose_range_nm: f32,
}

impl LiveEnvironment {
    /// Each pair's [`Surroundings`] on `l`, with `extra_wells` (placed, world
    /// frame) joining the cells' n-wells: WPE is a member's channels' mean
    /// distance to the nwell union's edge, OSE its cell's diffusion's gap to
    /// every other cell's. A member without units, or a cell past the
    /// geometry, reads `f32::INFINITY`.
    ///
    /// Cost: O(units · wells²) for WPE plus O(diffs²) for OSE; allocates the
    /// placed rects once per call.
    ///
    /// # Panics
    /// When a cell's [`Layout::variant`] is past its [`EnvGeo`] variants.
    #[must_use]
    pub fn surroundings_with(&self, l: &Layout, extra_wells: &[Rect]) -> Vec<Surroundings> {
        let geo = &*self.geo;
        let n = geo.bbox.len().min(l.x.len());
        let placed = |table: &[Vec<Vec<Rect>>], c: usize| {
            let v = usize::from(l.variant.get(c).copied().unwrap_or(0));
            let frame = geo.bbox[c][v];
            table[c][v].iter().map(move |&r| place_rect(frame, r, l, c)).collect::<Vec<_>>()
        };
        let mut wells = extra_wells.to_vec();
        for c in 0..n {
            wells.extend(placed(&geo.wells, c));
        }
        let diffs: Vec<Vec<Rect>> = (0..n).map(|c| placed(&geo.diffs, c)).collect();
        let mean_wpe = |d: DeviceId| {
            let (s, k) = l.units.of_device(l, d).fold((0.0f32, 0u32), |(s, k), u| (s + well_edge_distance(&wells, (u.x, u.y)).min(WPE_CAP_NM), k + 1));
            if k == 0 { f32::INFINITY } else { s / k as f32 }
        };
        self.pairs
            .iter()
            .map(|&(a, b, ca, cb)| Surroundings {
                wpe_nm: [mean_wpe(a), mean_wpe(b)],
                ose_nm: [foreign_diff_gap(&diffs, ca), foreign_diff_gap(&diffs, cb)],
                wpe_min_nm: self.wpe_min_nm,
                ose_range_nm: self.ose_range_nm,
            })
            .collect()
    }

    /// [`Self::surroundings_with`] on the cells' own wells only.
    #[must_use]
    pub fn surroundings(&self, l: &Layout) -> Vec<Surroundings> {
        self.surroundings_with(l, &[])
    }

    /// The precomputed batch for `l`.
    fn now(&self, l: &Layout) -> Environment {
        Environment(self.surroundings(l))
    }
}

/// Per-unit WPE distance cap, nm: a unit with no well on the die must not make
/// its member's mean infinite while its partner's is finite.
const WPE_CAP_NM: f32 = 1e7;

/// Whether `p` lies in `r`, edges included.
#[inline]
fn contains(r: &Rect, (x, y): (i32, i32)) -> bool {
    r.x <= x && x <= r.x + r.w && r.y <= y && y <= r.y + r.h
}

/// Euclidean gap from `p` to `r`, nm; `0` inside or on the edge.
#[inline]
fn point_gap(r: &Rect, (x, y): (i32, i32)) -> f32 {
    let dx = (r.x - x).max(x - (r.x + r.w)).max(0);
    let dy = (r.y - y).max(y - (r.y + r.h)).max(0);
    f64::from(dx).hypot(f64::from(dy)) as f32
}

/// Euclidean edge-to-edge gap between two rects, nm; `0` when they touch.
#[inline]
fn rect_gap(a: &Rect, b: &Rect) -> f32 {
    let dx = (a.x - (b.x + b.w)).max(b.x - (a.x + a.w)).max(0);
    let dy = (a.y - (b.y + b.h)).max(b.y - (a.y + a.h)).max(0);
    f64::from(dx).hypot(f64::from(dy)) as f32
}

/// Distance from `p` to the boundary of the union of `wells`, nm:
/// outside, to the nearest well; inside, to the nearest uncovered lattice
/// point just past a well edge (projections onto every edge, and the
/// corners). `f32::INFINITY` with no wells, or when every candidate is
/// covered.
fn well_edge_distance(wells: &[Rect], p: (i32, i32)) -> f32 {
    let covered = |q: (i32, i32)| wells.iter().any(|r| contains(r, q));
    if !covered(p) {
        return wells.iter().map(|r| point_gap(r, p)).fold(f32::INFINITY, f32::min);
    }
    let mut best = f32::INFINITY;
    for r in wells {
        let (x0, x1, y0, y1) = (r.x, r.x + r.w, r.y, r.y + r.h);
        let (cx, cy) = (p.0.clamp(x0, x1), p.1.clamp(y0, y1));
        for q in [(x0 - 1, cy), (x1 + 1, cy), (cx, y0 - 1), (cx, y1 + 1), (x0 - 1, y0 - 1), (x1 + 1, y0 - 1), (x0 - 1, y1 + 1), (x1 + 1, y1 + 1)] {
            if !covered(q) {
                best = best.min(f64::from(q.0 - p.0).hypot(f64::from(q.1 - p.1)) as f32);
            }
        }
    }
    best
}

/// Smallest gap from cell `c`'s diffusion to any other cell's, nm;
/// `f32::INFINITY` when `c` is past `diffs` or nothing else has diffusion.
fn foreign_diff_gap(diffs: &[Vec<Rect>], c: u16) -> f32 {
    let c = usize::from(c);
    let Some(own) = diffs.get(c) else { return f32::INFINITY };
    let mut best = f32::INFINITY;
    for (o, theirs) in diffs.iter().enumerate() {
        if o == c {
            continue;
        }
        for f in theirs {
            for m in own {
                best = best.min(rect_gap(f, m));
            }
        }
    }
    best
}

// ponytail: global batch, re-measured on every dp trial (`O(P·Σ_c rects)`); if
// FLOW-09's dp timer shows > 30 % of dp time here, cache unmoved cells' placed
// rects per trial (PLC-10's `Eval` knows the moved set).
impl RuleBatch<Layout> for LiveEnvironment {
    fn cost(&self, l: &Layout) -> f32 {
        self.now(l).cost(l)
    }
    fn violations(&self, l: &Layout) -> u32 {
        self.now(l).violations(l)
    }
    fn residual(&self, l: &Layout) -> f64 {
        self.now(l).residual(l)
    }
    fn unknown(&self, l: &Layout) -> u32 {
        self.now(l).unknown(l)
    }
    fn kind(&self) -> &'static str {
        "Environment"
    }
    fn count(&self) -> usize {
        self.pairs.len()
    }
    fn worst_usage(&self, l: &Layout) -> Option<f32> {
        self.now(l).worst_usage(l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn near_or_unequal_surroundings_spend_the_budget() {
        let far = Surroundings { wpe_nm: [f32::INFINITY; 2], ose_nm: [f32::INFINITY; 2], wpe_min_nm: 3_000.0, ose_range_nm: 3_000.0 };
        assert_eq!(far.usage(), Some(0.0), "nothing nearby");
        let wpe = Surroundings { wpe_nm: [1_500.0, 1_500.0], ..far };
        assert!((wpe.usage().unwrap() - 2.0).abs() < 1e-6, "a well edge at half the floor");
        let ose = Surroundings { ose_nm: [500.0, 2_000.0], ..far };
        assert!(ose.usage().unwrap() > 1.0, "one member crowded by foreign diffusion, the other not");
        let even = Surroundings { ose_nm: [500.0, 520.0], ..far };
        assert!(even.usage().unwrap() < 1.0, "crowded alike: matched");
        let blind = Surroundings { wpe_min_nm: 0.0, ose_range_nm: 0.0, ..ose };
        assert_eq!(blind.usage(), None, "a deck without the ranges: unknown, not a pass");
    }

    use pnr_core::{Unit, UnitLib};

    /// Cells a, b (1000², diff = bbox, one unit at the centre, devices 0/1)
    /// and c (2000×1000, nwell = bbox, no diff); a, b at x = 500, 3500.
    fn env() -> LiveEnvironment {
        let sq = Rect { x: 0, y: 0, w: 1000, h: 1000 };
        let wide = Rect { x: 0, y: 0, w: 2000, h: 1000 };
        LiveEnvironment {
            pairs: vec![(DeviceId(0), DeviceId(1), 0, 1)],
            geo: std::sync::Arc::new(EnvGeo {
                bbox: vec![vec![sq], vec![sq], vec![wide]],
                wells: vec![vec![vec![]], vec![vec![]], vec![vec![wide]]],
                diffs: vec![vec![vec![sq]], vec![vec![sq]], vec![vec![]]],
            }),
            wpe_min_nm: 3000.0,
            ose_range_nm: 0.0,
        }
    }

    fn layout(cx: i32) -> Layout {
        let unit = [Unit { owner: 0, x: 500, y: 500, weight: 1, phi: (1, 0), sa_sb: None }];
        let sq = Rect { x: 0, y: 0, w: 1000, h: 1000 };
        let one = [(sq, &unit[..])];
        let none = [(Rect { x: 0, y: 0, w: 2000, h: 1000 }, &[][..])];
        let units = UnitLib::build(vec![0, 1], &[vec![DeviceId(0)], vec![DeviceId(1)], vec![]], [&one[..], &one[..], &none[..]].into_iter());
        Layout {
            x: vec![500, 3500, cx],
            y: vec![500; 3],
            hw: vec![500, 500, 1000],
            hh: vec![500; 3],
            orient: vec![pnr_core::Orient::R0; 3],
            variant: vec![0; 3],
            axis: vec![],
            branch: vec![],
            groups: vec![],
            power_uw: vec![0; 3],
            temp_mc: vec![0; 3],
            units: std::sync::Arc::new(units),
        }
    }

    #[test]
    fn a_foreign_well_near_one_member_violates() {
        let env = env();
        let near = layout(5500);
        assert_eq!(env.surroundings(&near)[0].wpe_nm, [4000.0, 1000.0]);
        assert_eq!(env.violations(&near), 1);
        let far = layout(41_000);
        assert_eq!(env.violations(&far), 0);
        let u = env.worst_usage(&far).expect("WPE range known");
        assert!((u - 3000.0 / 36_500.0).abs() < 1e-6, "{u}");
    }

    #[test]
    fn ring_wells_join_the_report_only() {
        let (env, l) = (env(), layout(41_000));
        assert_eq!(env.surroundings_with(&l, &[Rect { x: 4000, y: 0, w: 1000, h: 1000 }])[0].wpe_nm[1], 500.0);
        assert!(env.surroundings(&l)[0].wpe_nm[1] > 30_000.0);
    }
}

#[cfg(test)]
mod cleanup_tests {
    use super::*;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect { x, y, w, h }
    }

    fn s(wpe_nm: [f32; 2], ose_nm: [f32; 2], wpe_min_nm: f32, ose_range_nm: f32) -> Surroundings {
        Surroundings { wpe_nm, ose_nm, wpe_min_nm, ose_range_nm }
    }

    const INF: f32 = f32::INFINITY;

    #[test]
    fn usage_reads_only_the_ranges_the_deck_gives() {
        // OSE alone: 500 vs 1000 nm differ by half the larger, over a 0.2 tolerance.
        let ose = s([1.0, 1.0], [500.0, 1_000.0], 0.0, 3_000.0);
        assert!((ose.usage().unwrap() - 2.5).abs() < 1e-6, "WPE must be ignored without its range");
        // WPE alone, nothing near: free.
        assert_eq!(s([INF; 2], [1.0, 9_000.0], 3_000.0, 0.0).usage(), Some(0.0));
    }

    #[test]
    fn distances_past_range_count_as_the_range() {
        assert_eq!(s([INF; 2], [5_000.0, 9_000.0], 0.0, 3_000.0).usage(), Some(0.0), "both faded");
        // WPE skew range is ten times the floor: 40 µm vs 50 µm are both past 30 µm.
        assert_eq!(s([40_000.0, 50_000.0], [INF; 2], 3_000.0, 0.0).usage().map(|u| u <= 0.1), Some(true));
    }

    #[test]
    fn one_member_near_a_well_dominates() {
        // Near term 3000/1000 = 3; skew (30 000 − 1000)/30 000/0.2 = 29/6 wins.
        let u = s([INF, 1_000.0], [INF; 2], 3_000.0, 0.0).usage().unwrap();
        assert!((u - 29.0 / 6.0).abs() < 1e-5, "{u}");
    }

    #[test]
    fn an_empty_batch_reports_nothing() {
        let e = Environment::default();
        assert_eq!((RuleBatch::<()>::cost(&e, &()), e.violations(&()), e.residual(&()), e.unknown(&())), (0.0, 0, 0.0, 0));
        assert_eq!((RuleBatch::<()>::worst_usage(&e, &()), RuleBatch::<()>::count(&e)), (None, 0));
    }

    #[test]
    fn a_batch_sums_known_pairs_and_counts_unknown_ones() {
        let hot = s([1_500.0, 1_500.0], [INF; 2], 3_000.0, 0.0); // usage 2
        let blind = s([1.0; 2], [1.0, 2.0], 0.0, 0.0); // unknown
        let calm = s([INF; 2], [INF; 2], 3_000.0, 3_000.0); // usage 0
        let e = Environment(vec![hot, blind, calm]);
        let cost = RuleBatch::<()>::cost(&e, &());
        assert!((cost - 2.0).abs() < 1e-6);
        assert_eq!((e.violations(&()), e.unknown(&()), RuleBatch::<()>::count(&e)), (1, 1, 3));
        assert!((e.residual(&()) - 1.0).abs() < 1e-6);
        assert_eq!(RuleBatch::<()>::worst_usage(&e, &()).map(|u| (u - 2.0).abs() < 1e-6), Some(true));
        assert_eq!(RuleBatch::<()>::kind(&e), "Environment");
    }

    #[test]
    fn rect_and_point_gaps_are_euclidean() {
        let r = rect(0, 0, 1_000, 1_000);
        assert!(contains(&r, (0, 0)) && contains(&r, (1_000, 1_000)) && !contains(&r, (1_001, 0)));
        assert_eq!(point_gap(&r, (500, 500)), 0.0);
        assert_eq!(point_gap(&r, (1_300, 1_400)), 500.0);
        assert_eq!(point_gap(&r, (-300, 500)), 300.0);
        assert_eq!(rect_gap(&r, &rect(1_300, 1_400, 10, 10)), 500.0);
        assert_eq!(rect_gap(&r, &rect(1_000, 0, 10, 10)), 0.0, "touching");
        assert_eq!(rect_gap(&r, &rect(200, 200, 10, 10)), 0.0, "nested");
    }

    #[test]
    fn well_edge_distance_sees_the_union() {
        assert_eq!(well_edge_distance(&[], (0, 0)), INF);
        let one = [rect(0, 0, 1_000, 1_000)];
        assert_eq!(well_edge_distance(&one, (1_300, 1_400)), 500.0, "outside: to the well");
        assert_eq!(well_edge_distance(&one, (500, 500)), 501.0, "inside: to the first uncovered point");
        assert_eq!(well_edge_distance(&one, (900, 500)), 101.0);
        // A second well abutting on the right hides that edge.
        let two = [one[0], rect(1_000, 0, 1_000, 1_000)];
        assert_eq!(well_edge_distance(&two, (900, 500)), 501.0);
    }

    #[test]
    fn foreign_diff_gap_skips_the_own_cell() {
        let diffs = vec![vec![rect(0, 0, 100, 100)], vec![rect(400, 0, 100, 100), rect(5_000, 0, 1, 1)], vec![]];
        assert_eq!(foreign_diff_gap(&diffs, 0), 300.0);
        assert_eq!(foreign_diff_gap(&diffs, 1), 300.0);
        assert_eq!(foreign_diff_gap(&diffs, 2), INF, "no diffusion of its own");
        assert_eq!(foreign_diff_gap(&diffs, 9), INF, "past the geometry");
        assert_eq!(foreign_diff_gap(&diffs[..1], 0), INF, "alone on the die");
    }

    fn empty_layout(n: usize) -> Layout {
        Layout {
            x: vec![0; n],
            y: vec![0; n],
            hw: vec![10; n],
            hh: vec![10; n],
            orient: vec![pnr_core::Orient::R0; n],
            variant: vec![0; n],
            axis: vec![],
            branch: vec![],
            groups: vec![],
            power_uw: vec![0; n],
            temp_mc: vec![0; n],
            units: Default::default(),
        }
    }

    #[test]
    fn live_batch_without_geometry_or_units() {
        let live = |pairs| LiveEnvironment { pairs, geo: Default::default(), wpe_min_nm: 3_000.0, ose_range_nm: 0.0 };
        let none = live(vec![]);
        let l = empty_layout(0);
        assert!(none.surroundings(&l).is_empty());
        assert_eq!((none.cost(&l), none.violations(&l), none.count(), none.worst_usage(&l)), (0.0, 0, 0, None));
        // Members without units and cells past the geometry read "none on the die".
        let one = live(vec![(DeviceId(0), DeviceId(1), 0, 1)]);
        let l = empty_layout(2);
        let got = one.surroundings(&l);
        assert_eq!((got[0].wpe_nm, got[0].ose_nm), ([INF; 2], [INF; 2]));
        assert_eq!((one.count(), one.kind(), one.unknown(&l)), (1, "Environment", 0));
    }
}
