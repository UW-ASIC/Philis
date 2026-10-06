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
    pub wpe_nm: [f32; 2],
    pub ose_nm: [f32; 2],
    pub wpe_min_nm: f32,
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
/// State-free (the numbers are precomputed), so it reports on any stage.
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
/// and diff rects.
#[derive(Debug, Default)]
pub struct EnvGeo {
    pub bbox: Vec<Vec<Rect>>,
    pub wells: Vec<Vec<Vec<Rect>>>,
    pub diffs: Vec<Vec<Vec<Rect>>>,
}

/// Every matched pair's [`Surroundings`], re-measured on each layout it is
/// asked about: the placement-tier batch (PLC-29), so gp/dp see WPE/OSE in Θ.
#[derive(Clone, Debug)]
pub struct LiveEnvironment {
    /// `(member a, member b, cell of a, cell of b)`.
    pub pairs: Vec<(DeviceId, DeviceId, u16, u16)>,
    pub geo: std::sync::Arc<EnvGeo>,
    pub wpe_min_nm: f32,
    pub ose_range_nm: f32,
}

impl LiveEnvironment {
    /// Each pair's [`Surroundings`] on `l`, with `extra_wells` (placed, world
    /// frame) joining the cells' n-wells: WPE is a member's channels' mean
    /// distance to the nwell union's edge, OSE its cell's diffusion's gap to
    /// every other cell's.
    #[must_use]
    pub fn surroundings_with(&self, l: &Layout, extra_wells: &[Rect]) -> Vec<Surroundings> {
        let n = self.geo.bbox.len().min(l.x.len());
        let geo = &*self.geo;
        let placed = |well: bool, c: usize| {
            let v = usize::from(l.variant.get(c).copied().unwrap_or(0));
            (if well { &geo.wells } else { &geo.diffs })[c][v].iter().map(move |&r| place_rect(geo.bbox[c][v], r, l, c))
        };
        let wells: Vec<Rect> = extra_wells.iter().copied().chain((0..n).flat_map(|c| placed(true, c))).collect();
        let diffs: Vec<Vec<Rect>> = (0..n).map(|c| placed(false, c).collect()).collect();
        let inside = |r: &Rect, (x, y): (i32, i32)| r.x <= x && x <= r.x + r.w && r.y <= y && y <= r.y + r.h;
        let covered = |p: (i32, i32)| wells.iter().any(|r| inside(r, p));
        let gap = |r: &Rect, (x, y): (i32, i32)| {
            let dx = (r.x - x).max(x - (r.x + r.w)).max(0);
            let dy = (r.y - y).max(y - (r.y + r.h)).max(0);
            f64::from(dx).hypot(f64::from(dy)) as f32
        };
        // Distance from `p` to the nwell union's boundary: to the nearest
        // well outside it, else to the nearest uncovered point just past a
        // well edge (projections onto every edge, and the corners).
        let wpe = |p: (i32, i32)| -> f32 {
            if !covered(p) {
                return wells.iter().map(|r| gap(r, p)).fold(f32::INFINITY, f32::min);
            }
            let mut best = f32::INFINITY;
            for r in &wells {
                let (x0, x1, y0, y1) = (r.x, r.x + r.w, r.y, r.y + r.h);
                let cx = p.0.clamp(x0, x1);
                let cy = p.1.clamp(y0, y1);
                for q in [(x0 - 1, cy), (x1 + 1, cy), (cx, y0 - 1), (cx, y1 + 1), (x0 - 1, y0 - 1), (x1 + 1, y0 - 1), (x0 - 1, y1 + 1), (x1 + 1, y1 + 1)] {
                    if !covered(q) {
                        best = best.min(f64::from(q.0 - p.0).hypot(f64::from(q.1 - p.1)) as f32);
                    }
                }
            }
            best
        };
        let rect_gap = |a: &Rect, b: &Rect| {
            let dx = (a.x - (b.x + b.w)).max(b.x - (a.x + a.w)).max(0);
            let dy = (a.y - (b.y + b.h)).max(b.y - (a.y + a.h)).max(0);
            f64::from(dx).hypot(f64::from(dy)) as f32
        };
        let mean_wpe = |d: DeviceId| {
            let (s, n) = l.units.of_device(l, d).fold((0.0f32, 0u32), |(s, n), u| (s + wpe((u.x, u.y)).min(1e7), n + 1));
            if n == 0 { f32::INFINITY } else { s / n as f32 }
        };
        let ose = |c: u16| {
            let c = usize::from(c);
            let Some(own) = diffs.get(c) else { return f32::INFINITY };
            (0..diffs.len())
                .filter(|&o| o != c)
                .flat_map(|o| &diffs[o])
                .flat_map(|f| own.iter().map(move |m| (f, m)))
                .map(|(f, m)| rect_gap(f, m))
                .fold(f32::INFINITY, f32::min)
        };
        self.pairs
            .iter()
            .map(|&(a, b, ca, cb)| Surroundings {
                wpe_nm: [mean_wpe(a), mean_wpe(b)],
                ose_nm: [ose(ca), ose(cb)],
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

    fn now(&self, l: &Layout) -> Environment {
        Environment(self.surroundings(l))
    }
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
        let unit = [Unit { owner: 0, x: 500, y: 500, weight: 1, phi: (1, 0), sa: 0, sb: 0 }];
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
