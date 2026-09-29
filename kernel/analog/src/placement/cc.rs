//! Common-centroid (placement tier).

use pnr_core::ids::DeviceId;
use pnr_core::layout::Layout;

/// Common centroid of a whole matched array: side A's electrically weighted
/// first moment against side B's, over the physical units each schematic
/// device owns ([`pnr_core::UnitLib`]). Pairwise coincidence is a weaker
/// condition (each pair centred while the array stays lopsided).
///
/// Without unit data it falls back to area-weighted cell boxes and reports
/// itself **unknown**: an outline is a proxy, not the device's moment.
///
/// Violated when either check fails:
/// - **coincidence** (Hastings 3e Table 8.4 rule 1, PDF p.392): over the
///   cells that draw both sides (an interleaved array) the weighted first
///   moments must coincide to within [`COINCIDENCE_TOL`] of the array's extent.
///   Dimensionless, needs no process data. Separate cells cannot coincide, so
///   it does not bind there.
/// - **gradient** (Pelgrom & Duinmaijer 1988 eq.(1)): the centroid offset, read
///   as a gradient distance, spends more than `gradient_share` of the random
///   mismatch of `gate_um2` (see [`crate::placement::MatchingPair`]). Needs the
///   deck's `S/A`; without it this half is unknown.
///
/// The exact equality belongs in the pattern representation (ABBA /
/// checkerboard), not a projection.
#[derive(Clone)]
pub struct CentroidGroup {
    pub a_side: Vec<DeviceId>,
    pub b_side: Vec<DeviceId>,
    /// Gate area `W·L·fingers` of the largest member, µm² (tightest budget).
    pub gate_um2: f32,
    /// Allowed `σ_gradient / σ_random`.
    pub gradient_share: f32,
    /// `S_VT / A_VT`, 1/µm², from the deck; `0` = unknown.
    pub gradient_per_avt_um2: f32,
    /// LOD: `KVTH0 / σ_rand` (BSIM4 stress ΔVT per unit `Δ(1/SA+1/SB)` over
    /// the random mismatch), µm; `0` = unknown. The sides' mean stress terms
    /// must agree to within the gradient share of `σ_rand`: an ABBA row puts
    /// one side's fingers nearer the diffusion ends (Hastings §13.3 r9).
    pub lod_per_sigma_um: f32,
    /// `device → cell`, set by `retarget`; empty = the side ids already name
    /// cells. Only the bbox fallback reads it: sides stay schematic devices.
    pub cell_of: Vec<u16>,
}

impl CentroidGroup {
    /// Weighted first moment of a device set, nm: over its units when the
    /// layout carries them, else over its cells' boxes by area.
    fn centroid(&self, l: &Layout, side: &[DeviceId]) -> Option<(f64, f64)> {
        Self::unit_centroid(l, side).or_else(|| self.box_centroid(l, side))
    }

    fn unit_centroid(l: &Layout, side: &[DeviceId]) -> Option<(f64, f64)> {
        let (mut sx, mut sy, mut sw) = (0.0f64, 0.0f64, 0.0f64);
        for u in side.iter().flat_map(|&d| l.units.of_device(l, d)) {
            let w = u.weight as f64;
            sx += f64::from(u.x) * w;
            sy += f64::from(u.y) * w;
            sw += w;
        }
        (sw > 0.0).then(|| (sx / sw, sy / sw))
    }

    fn box_centroid(&self, l: &Layout, side: &[DeviceId]) -> Option<(f64, f64)> {
        let (mut sx, mut sy, mut sw) = (0.0f64, 0.0f64, 0.0f64);
        for d in side {
            let i = self.cell_of.get(d.0 as usize).map_or(d.0, |&c| c) as usize;
            if i >= l.x.len() {
                continue;
            }
            let w = (4.0 * f64::from(l.hw[i]) * f64::from(l.hh[i])).max(1.0);
            sx += f64::from(l.x[i]) * w;
            sy += f64::from(l.y[i]) * w;
            sw += w;
        }
        (sw > 0.0).then(|| (sx / sw, sy / sw))
    }
}

/// Centroids "coincide" within this fraction of the array's extent: unit
/// coordinates are integer nm, so an exact pattern lands at ~0.
///
/// ponytail: a fixed 1%; a curved field would want the second moments too
/// (Hastings eq.8.26), add them when a deck characterises one.
pub const COINCIDENCE_TOL: f32 = 0.01;

impl CentroidGroup {
    /// Centroid separation, nm; `0` when a side is empty.
    fn offset_nm(&self, l: &Layout) -> f32 {
        let (Some((ax, ay)), Some((bx, by))) = (self.centroid(l, &self.a_side), self.centroid(l, &self.b_side))
        else {
            return 0.0;
        };
        (ax - bx).hypot(ay - by) as f32
    }

    /// Centroid offset over array extent, both measured on the units of the
    /// cells that draw **both** sides (the interleaved part: a merged pair can
    /// share a stage with separately drawn members). `None` when no cell draws
    /// both sides, or without unit data.
    fn interleaved_offset(&self, l: &Layout) -> Option<f32> {
        let cell = |d: &DeviceId| l.units.cell_of.get(d.0 as usize).copied();
        let shared: Vec<u16> =
            self.a_side.iter().filter_map(cell).filter(|c| self.b_side.iter().any(|d| cell(d) == Some(*c))).collect();
        if shared.is_empty() {
            return None;
        }
        let inside = |side: &[DeviceId]| -> Vec<DeviceId> {
            side.iter().copied().filter(|d| cell(d).is_some_and(|c| shared.contains(&c))).collect()
        };
        let (a, b) = (inside(&self.a_side), inside(&self.b_side));
        let ((ax, ay), (bx, by)) = (Self::unit_centroid(l, &a)?, Self::unit_centroid(l, &b)?);
        let (mut x0, mut x1, mut y0, mut y1) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
        for u in a.iter().chain(&b).flat_map(|&d| l.units.of_device(l, d)) {
            (x0, x1, y0, y1) = (x0.min(u.x), x1.max(u.x), y0.min(u.y), y1.max(u.y));
        }
        Some((ax - bx).hypot(ay - by) as f32 / (x1 - x0).max(y1 - y0).max(1) as f32)
    }

    /// Mean LOD stress term of a side, 1/µm, over its fingers that carry one
    /// (BSIM4 averages `1/(SA+L/2)` over a device's fingers).
    fn lod(l: &Layout, side: &[DeviceId]) -> Option<f32> {
        let (mut s, mut n) = (0.0f32, 0u32);
        for u in side.iter().flat_map(|&d| l.units.of_device(l, d)).filter(|u| u.lod.is_finite()) {
            s += u.lod;
            n += 1;
        }
        (n > 0).then(|| s / n as f32)
    }

    /// LOD ΔVT between the sides over the allowed share of `σ_rand`; `0`
    /// without the deck's `KVTH0` or unit data.
    fn lod_used(&self, l: &Layout) -> f32 {
        match (Self::lod(l, &self.a_side), Self::lod(l, &self.b_side)) {
            (Some(a), Some(b)) if self.lod_per_sigma_um > 0.0 => {
                (a - b).abs() * self.lod_per_sigma_um / self.gradient_share.max(f32::EPSILON)
            }
            _ => 0.0,
        }
    }

    /// Spent fraction of the tighter check (see the type docs).
    fn used(&self, l: &Layout) -> f32 {
        let d = self.offset_nm(l);
        let g = crate::placement::matching_pair::gradient_over_random(d, self.gate_um2, self.gradient_per_avt_um2);
        let coincidence = self.interleaved_offset(l).map_or(0.0, |u| u / COINCIDENCE_TOL);
        (g / self.gradient_share.max(f32::EPSILON)).max(coincidence).max(self.lod_used(l))
    }
}

impl crate::rule::RuleBatch<Layout> for CentroidGroup {
    /// Squared centroid separation `· 1e-3`.
    fn cost(&self, l: &Layout) -> f32 {
        let d = self.offset_nm(l);
        d * d * 1e-3
    }
    fn violations(&self, l: &Layout) -> u32 {
        u32::from(self.count() > 0 && self.used(l) > 1.0)
    }
    fn residual(&self, l: &Layout) -> f64 {
        f64::from((self.used(l) - 1.0).max(0.0))
    }
    /// Both sides need units (else the offset is a bbox proxy), and there must
    /// be a check to run: the process's gradient coefficient, or an
    /// interleaved array for coincidence.
    fn unknown(&self, l: &Layout) -> u32 {
        let units = Self::unit_centroid(l, &self.a_side).is_some() && Self::unit_centroid(l, &self.b_side).is_some();
        let measured = units && (self.gradient_per_avt_um2 > 0.0 || self.interleaved_offset(l).is_some());
        u32::from(self.count() > 0 && !measured)
    }
    fn worst_usage(&self, l: &Layout) -> Option<f32> {
        (self.count() > 0).then(|| self.used(l))
    }
    fn kind(&self) -> &'static str {
        "CommonCentroid"
    }
    /// One condition per array.
    fn count(&self) -> usize {
        usize::from(!self.a_side.is_empty() && !self.b_side.is_empty())
    }
    /// Sides keep their schematic devices (units are owned by those); the
    /// map is kept for the bbox fallback.
    fn retarget(&mut self, cell_of: &[u16]) {
        self.cell_of = cell_of.to_vec();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule::RuleBatch;
    use pnr_core::ids::DeviceId;

    /// Four devices: sides {0,2} and {1,3}.
    fn layout(xs: &[i32], ys: &[i32], half: i32) -> Layout {
        let n = xs.len();
        Layout {
            x: xs.to_vec(),
            y: ys.to_vec(),
            hw: vec![half; n],
            hh: vec![half; n],
            axis: vec![0; n],
            groups: vec![],
            orient: vec![pnr_core::Orient::default(); n],
            variant: vec![0; n],
            branch: Vec::new(),
            power_uw: vec![0; n],
            temp_mc: vec![0; n],
            units: Default::default(),
        }
    }

    fn grp() -> CentroidGroup {
        CentroidGroup {
            a_side: vec![DeviceId(0), DeviceId(2)],
            b_side: vec![DeviceId(1), DeviceId(3)],
            gate_um2: 20.0,
            gradient_share: 0.3,
            gradient_per_avt_um2: 2.5e-4,
            lod_per_sigma_um: 0.0,
            cell_of: Vec::new(),
        }
    }

    #[test]
    fn interleaved_array_has_coincident_centroids() {
        // A B B A — the canonical common-centroid order. Both sides centre on 15.
        let l = layout(&[0, 10, 20, 30], &[0, 0, 0, 0], 100);
        let g = CentroidGroup {
            a_side: vec![DeviceId(0), DeviceId(3)],
            b_side: vec![DeviceId(1), DeviceId(2)],
            ..grp()
        };
        assert_eq!(g.cost(&l), 0.0, "ABBA must be centroid-balanced");
    }

    #[test]
    fn segregated_array_is_penalised_even_when_every_pair_looks_centred() {
        // A A B B: the array is lopsided — exactly the gradient the pattern
        // exists to cancel — and the group cost sees it.
        let l = layout(&[0, 10, 20, 30], &[0, 0, 0, 0], 100);
        let seg = CentroidGroup {
            a_side: vec![DeviceId(0), DeviceId(1)],
            b_side: vec![DeviceId(2), DeviceId(3)],
            ..grp()
        };
        assert!(seg.cost(&l) > 0.0, "segregated sides must cost");
    }

    #[test]
    fn centroid_is_area_weighted() {
        // Side A: a big device at 0 and a small one at 100. Side B: one device
        // placed at the *unweighted* mean (50). If weighting were ignored the
        // cost would be zero; it must not be.
        let mut l = layout(&[0, 100, 50], &[0, 0, 0], 100);
        l.hw[0] = 1_000; // the device at x=0 dominates by area
        let g = CentroidGroup { a_side: vec![DeviceId(0), DeviceId(1)], b_side: vec![DeviceId(2)], ..grp() };
        assert!(g.cost(&l) > 0.0, "area weighting must move the centroid off the plain mean");

        // Put B at the area-weighted centroid instead and it vanishes.
        let (cx, _) = g.centroid(&l, &g.a_side).unwrap();
        l.x[2] = cx.round() as i32;
        assert!(g.cost(&l) < 1e-3, "coincident weighted centroids cost nothing");
    }

    #[test]
    fn one_condition_per_array_not_per_device() {
        let l = layout(&[0, 10, 20, 30], &[0, 0, 0, 0], 100);
        assert_eq!(grp().count(), 1, "the array carries one centroid condition");
        assert_eq!(grp().violations(&l), 0, "10 nm of offset is nothing");
    }

    #[test]
    fn a_far_off_centroid_is_a_violation() {
        // A A … B B a millimetre apart: the gradient term dwarfs the random one.
        let l = layout(&[0, 10, 1_000_000, 1_000_010], &[0, 0, 0, 0], 100);
        let seg = CentroidGroup { a_side: vec![DeviceId(0), DeviceId(1)], b_side: vec![DeviceId(2), DeviceId(3)], ..grp() };
        assert_eq!(seg.violations(&l), 1);
        assert!(seg.residual(&l) > 0.0 && seg.worst_usage(&l).unwrap() > 1.0);
    }

    /// The acceptance test from the notes: changing only a cell's outline
    /// (halo, aspect) must not move the electrical centroid. With units the
    /// offset is exact and known; without them it is a proxy and unknown.
    #[test]
    fn units_not_outlines_set_the_centroid() {
        use pnr_core::{Rect, Unit, UnitLib};
        // One merged cell drawing A B B A; devices 0 (A) and 1 (B).
        let units: Vec<Unit> = [(0u8, 100), (1, 300), (1, 500), (0, 700)]
            .iter()
            .map(|&(owner, x)| Unit { owner, x, y: 50, weight: 10, phi: (1, 0), sa: 0, sb: 0 })
            .collect();
        let bbox = Rect { x: 0, y: 0, w: 800, h: 100 };
        let alts = [(bbox, &units[..])];
        let lib = UnitLib::build(vec![0, 0], &[vec![DeviceId(0), DeviceId(1)]], std::iter::once(&alts[..]));
        let g = CentroidGroup { a_side: vec![DeviceId(0)], b_side: vec![DeviceId(1)], ..grp() };

        let mut l = layout(&[5_000], &[5_000], 400);
        l.hh[0] = 50;
        assert_eq!(g.unknown(&l), 1, "no units: a bbox proxy is not a measurement");
        l.units = std::sync::Arc::new(lib);
        assert_eq!(g.unknown(&l), 0);
        assert_eq!(g.cost(&l), 0.0, "ABBA: coincident electrical moments");
        // Grow the outline (a guard-ring halo): the moments do not move.
        l.hw[0] = 4_000;
        assert_eq!(g.cost(&l), 0.0);
    }

    /// LOD: in an ABBA row A owns both end fingers, nearest the diffusion
    /// ends. With the ends 0.4 µm past the outer gates the stress terms of the
    /// sides differ enough to spend the budget; a 3 µm moat evens them out.
    #[test]
    fn lod_imbalance_of_an_abba_row_needs_a_moat() {
        use pnr_core::{Rect, Unit, UnitLib};
        let at = |ext: i32| {
            // Gate centres at 0.5 µm pitch; the diffusion runs `ext` past
            // the outer gates.
            let xs = [0, 500, 1000, 1500];
            let (lo, hi) = (-ext, 1500 + ext);
            let units: Vec<Unit> = [0u8, 1, 1, 0]
                .iter()
                .zip(xs)
                .map(|(&owner, x)| Unit { owner, x, y: 50, weight: 10, phi: (1, 0), sa: x - lo, sb: hi - x })
                .collect();
            let alts = [(Rect { x: lo, y: 0, w: hi - lo, h: 100 }, &units[..])];
            let mut l = layout(&[5_000], &[5_000], 1_000);
            l.units = std::sync::Arc::new(UnitLib::build(vec![0, 0], &[vec![DeviceId(0), DeviceId(1)]], std::iter::once(&alts[..])));
            l
        };
        // sky130 pfet: KVTH0 32.9 mV·µm over σ_rand = 11.5/√4 mV.
        let g = CentroidGroup { a_side: vec![DeviceId(0)], b_side: vec![DeviceId(1)], gradient_per_avt_um2: 0.0, lod_per_sigma_um: 32.9 / 5.75, ..grp() };
        assert!(g.lod_used(&at(400)) > 1.0, "short ends: {}", g.lod_used(&at(400)));
        assert!(g.lod_used(&at(3_000)) < g.lod_used(&at(400)) / 2.0);
    }

    /// Hastings rule 1 needs no process data: in one merged cell, ABBA passes
    /// and AABB fails, with `S/A` unknown.
    #[test]
    fn coincidence_binds_an_interleaved_array_without_gradient_data() {
        use pnr_core::{Rect, Unit, UnitLib};
        let lib = |order: [u8; 4]| {
            let units: Vec<Unit> = order
                .iter()
                .zip([100, 300, 500, 700])
                .map(|(&owner, x)| Unit { owner, x, y: 50, weight: 10, phi: (1, 0), sa: 0, sb: 0 })
                .collect();
            let alts = [(Rect { x: 0, y: 0, w: 800, h: 100 }, &units[..])];
            UnitLib::build(vec![0, 0], &[vec![DeviceId(0), DeviceId(1)]], std::iter::once(&alts[..]))
        };
        let g = CentroidGroup { a_side: vec![DeviceId(0)], b_side: vec![DeviceId(1)], gradient_per_avt_um2: 0.0, ..grp() };
        let mut l = layout(&[5_000], &[5_000], 400);
        l.units = std::sync::Arc::new(lib([0, 1, 1, 0]));
        assert_eq!((g.unknown(&l), g.violations(&l)), (0, 0), "ABBA: measured and coincident");
        l.units = std::sync::Arc::new(lib([0, 0, 1, 1]));
        assert_eq!((g.unknown(&l), g.violations(&l)), (0, 1), "AABB: centroids a half-array apart");
        assert!(g.residual(&l) > 0.0);

        // Two separate cells cannot coincide: without S the check is unknown, not failed.
        let units = [Unit { owner: 0, x: 50, y: 50, weight: 10, phi: (1, 0), sa: 0, sb: 0 }];
        let alts = [(Rect { x: 0, y: 0, w: 100, h: 100 }, &units[..])];
        let two = UnitLib::build(
            vec![0, 1],
            &[vec![DeviceId(0)], vec![DeviceId(1)]],
            [&alts[..], &alts[..]].into_iter(),
        );
        let mut l = layout(&[0, 5_000], &[0, 0], 50);
        l.units = std::sync::Arc::new(two);
        assert_eq!((g.unknown(&l), g.violations(&l)), (1, 0));
    }

    /// A stage mixing a merged pair with separately drawn members: the merged
    /// cell's coincidence still binds (the separate cells cannot coincide).
    #[test]
    fn a_merged_pair_is_checked_inside_a_mixed_stage() {
        use pnr_core::{Rect, Unit, UnitLib};
        let pair = |order: [u8; 4]| -> Vec<Unit> {
            order.iter().zip([100, 300, 500, 700]).map(|(&owner, x)| Unit { owner, x, y: 50, weight: 10, phi: (1, 0), sa: 0, sb: 0 }).collect()
        };
        let one = [Unit { owner: 0, x: 50, y: 50, weight: 10, phi: (1, 0), sa: 0, sb: 0 }];
        let lib = |order| {
            let merged = pair(order);
            let a0 = [(Rect { x: 0, y: 0, w: 800, h: 100 }, &merged[..])];
            let a1 = [(Rect { x: 0, y: 0, w: 100, h: 100 }, &one[..])];
            let members = [vec![DeviceId(0), DeviceId(1)], vec![DeviceId(2)], vec![DeviceId(3)]];
            UnitLib::build(vec![0, 0, 1, 2], &members, [&a0[..], &a1[..], &a1[..]].into_iter())
        };
        // Sides {0 (merged), 2 (own cell)} vs {1 (merged), 3 (own cell)}.
        let g = CentroidGroup { a_side: vec![DeviceId(0), DeviceId(2)], b_side: vec![DeviceId(1), DeviceId(3)], gradient_per_avt_um2: 0.0, ..grp() };
        let mut l = layout(&[5_000, 0, 10_000], &[0, 0, 0], 50);
        l.hw[0] = 400;
        l.units = std::sync::Arc::new(lib([0, 1, 1, 0]));
        assert_eq!((g.unknown(&l), g.violations(&l)), (0, 0), "ABBA in the merged cell");
        l.units = std::sync::Arc::new(lib([0, 0, 1, 1]));
        assert_eq!(g.violations(&l), 1, "AABB in the merged cell, whatever the other cells do");
    }
}
