//! Matched sets (placement tier): one mismatch ledger and one allowance per
//! pair, replacing the separate Pelgrom distance, thermal and centroid rules
//! that each spent the same budget on their own (plan-02 MAT-04).

use pnr_core::ids::DeviceId;
use pnr_core::layout::Layout;

use crate::matching::mismatch::{sigma_pair, Budget, Coeffs, Ledger, MatchKind};
use crate::matching::moments::{sums, Pt};
use crate::matching::pattern::{cc_feasible, diffusion_cc_row, Outer};

/// A set of devices that must match its reference, pair by pair `(0, i)`:
/// each pair's systematic terms (gradient, thermal, LOD) share one allowance
/// ([`Ledger`]), and a pair whose unit counts admit a common-centroid row
/// must also coincide, merged into one cell or not: drawing it as two cells
/// (whose centroids never coincide) is no escape from the check.
///
/// Positions are the members' unit moments when the layout carries units,
/// else their cells' centres; a pair without units is **unknown** (an outline
/// is a proxy, not the device's moment) and is pulled by `cost` only.
#[derive(Clone)]
pub struct MatchedSet {
    /// Slot 0 = reference (a mirror's diode device); pairs are `(0, i)`.
    pub members: Vec<DeviceId>,
    pub kind: MatchKind,
    /// MOS members: coincidence feasibility is a diffusion-legal row.
    pub mos: bool,
    pub coeffs: Coeffs,
    pub budget: Budget,
    /// Netlist gate area `W·L·m` per member, µm²; read only without units.
    pub gate_um2: Vec<f32>,
    /// Coincidence tolerance, nm (half the cut lattice).
    pub tol_nm: f32,
    /// `device → cell`, set by `retarget`; empty = the ids name cells.
    pub cell_of: Vec<u16>,
}

/// Unit moments of one member plus its weighted LOD sum (`Σw·lod`, `Σw` over
/// units with a finite `lod`).
fn member(l: &Layout, d: DeviceId) -> (crate::matching::moments::Sums, f64, f64) {
    let (mut lw, mut w) = (0.0f64, 0.0f64);
    let s = sums(l.units.of_device(l, d).inspect(|u| {
        if u.lod.is_finite() {
            lw += u.weight as f64 * f64::from(u.lod);
            w += u.weight as f64;
        }
    }).map(Pt::from));
    (s, lw, w)
}

impl MatchedSet {
    fn cell(&self, d: DeviceId) -> usize {
        self.cell_of.get(d.0 as usize).copied().unwrap_or(d.0) as usize
    }

    /// Ledger of pair `(members[0], members[i])` on `l` (see [`Ledger`]).
    #[must_use]
    pub fn ledger(&self, l: &Layout, i: usize) -> Ledger {
        self.ledger_with(l, i, true)
    }

    /// `coincide == false` leaves `coincidence` `None` (and `known` without
    /// it): `cost` never reads it, and the MOS row search allocates per call.
    fn ledger_with(&self, l: &Layout, i: usize, coincide: bool) -> Ledger {
        let (a, b) = (self.members[0], self.members[i]);
        let ((sa, lwa, wa), (sb, lwb, wb)) = (member(l, a), member(l, b));
        let units = sa.w > 0.0 && sb.w > 0.0;
        let at = |s: &crate::matching::moments::Sums, d: DeviceId| {
            s.centroid().unwrap_or_else(|| {
                let c = self.cell(d);
                if c < l.x.len() { (f64::from(l.x[c]), f64::from(l.y[c])) } else { (0.0, 0.0) }
            })
        };
        let (ca, cb) = (at(&sa, a), at(&sb, b));
        let delta_m_nm = (ca.0 - cb.0).hypot(ca.1 - cb.1) as f32;

        // Feasibility reads the counts, not the drawing: gating on one cell let
        // the start ranking (Θ) prefer a pair split 34 µm apart over a merged
        // AABB 3 µm apart (ota, bench seed 1).
        let counts = [sa.n as u16, sb.n as u16];
        let feasible = || {
            if self.mos {
                diffusion_cc_row(&counts, Outer::Drain).is_some() || diffusion_cc_row(&counts, Outer::Source).is_some()
            } else {
                cc_feasible(&counts)
            }
        };
        let coincidence = (coincide && units && feasible()).then(|| delta_m_nm / self.tol_nm);

        let second_order_nm = if units {
            let c = ((sa.x + sb.x) / (sa.w + sb.w), (sa.y + sb.y) / (sa.w + sb.w));
            let (ma, mb) = (sa.second(c), sb.second(c));
            let d = [ma[0] - mb[0], ma[1] - mb[1], ma[2] - mb[2]];
            let f = (d[0] * d[0] + 2.0 * d[1] * d[1] + d[2] * d[2]).sqrt();
            let reach = [a, b]
                .iter()
                .flat_map(|&m| l.units.of_device(l, m))
                .map(|u| (f64::from(u.x) - c.0).hypot(f64::from(u.y) - c.1))
                .fold(0.0, f64::max);
            if reach > 0.0 { (f / reach) as f32 } else { 0.0 }
        } else {
            0.0
        };

        let area = |s: &crate::matching::moments::Sums, k: usize| {
            if units { (s.w / 1e6) as f32 } else { self.gate_um2.get(k).copied().unwrap_or(0.0) }
        };
        let sigma_rand = self.coeffs.avt_mv_um.map_or(0.0, |av| sigma_pair(av, area(&sa, 0), area(&sb, i)));
        let budgeted = sigma_rand > 0.0 || matches!(self.budget, Budget::Allowance(_));

        let (mut sigma_grad, mut mu_lod, mut mu_thermal) = (0.0, 0.0, 0.0);
        if budgeted {
            // µV/µm · nm → mV.
            sigma_grad = self.coeffs.svt_uv_per_um.unwrap_or(0.0) * delta_m_nm * 1e-6;
            if units && wa > 0.0 && wb > 0.0 {
                mu_lod = self.coeffs.kvth0_mv_um.unwrap_or(0.0) * ((lwa / wa - lwb / wb).abs() as f32);
            }
            if l.power_uw.iter().any(|&p| p != 0) {
                let rise = |(x, y): (f64, f64)| l.rise_at_point_mc(x.round() as i32, y.round() as i32);
                // µV/K · mK → mV.
                mu_thermal = self.coeffs.tc_uv_per_k.unwrap_or(0.0) * (rise(ca) - rise(cb)).abs() * 1e-6;
            }
        }
        Ledger {
            sigma_rand,
            sigma_grad,
            mu_thermal,
            mu_lod,
            allowance: self.budget.allowance(sigma_rand),
            coincidence,
            second_order_nm,
            delta_m_nm,
            known: units && (budgeted || coincidence.is_some()),
        }
    }

    fn ledgers<'a>(&'a self, l: &'a Layout) -> impl Iterator<Item = Ledger> + 'a {
        (1..self.members.len()).map(move |i| self.ledger(l, i))
    }
}

impl crate::rule::RuleBatch<Layout> for MatchedSet {
    /// `1e-3·(Δm² + so²)` nm², plus `3e5·(μ_thermal/allowance)²` per pair.
    ///
    /// ponytail: the scales of the rules this replaced (distance pull, thermal
    /// at-spec cost), kept until PLC-18 normalises costs.
    fn cost(&self, l: &Layout) -> f32 {
        (1..self.members.len())
            .map(|i| self.ledger_with(l, i, false))
            .map(|g| {
                let thermal = if g.allowance > 0.0 { 3e5 * (g.mu_thermal / g.allowance).powi(2) } else { 0.0 };
                1e-3 * (g.delta_m_nm * g.delta_m_nm + g.second_order_nm * g.second_order_nm) + thermal
            })
            .sum()
    }
    fn violations(&self, l: &Layout) -> u32 {
        self.ledgers(l).filter(|g| g.known && g.usage() > 1.0).count() as u32
    }
    fn residual(&self, l: &Layout) -> f64 {
        self.ledgers(l).filter(|g| g.known).map(|g| f64::from(g.residual())).sum()
    }
    fn unknown(&self, l: &Layout) -> u32 {
        self.ledgers(l).filter(|g| !g.known).count() as u32
    }
    fn worst_usage(&self, l: &Layout) -> Option<f32> {
        self.ledgers(l).filter(|g| g.known).map(|g| g.usage()).reduce(f32::max)
    }
    fn kind(&self) -> &'static str {
        "MatchedSet"
    }
    /// One condition per pair `(0, i)`.
    fn count(&self) -> usize {
        self.members.len().saturating_sub(1)
    }
    /// Members stay schematic devices (units are owned by those); the map
    /// serves the cell-centre fallback and repair ids.
    fn retarget(&mut self, cell_of: &[u16]) {
        self.cell_of = cell_of.to_vec();
    }
    fn touched(&self, out: &mut Vec<u32>) {
        out.extend(self.members.iter().map(|&m| self.cell(m) as u32));
    }
    fn violating_ids(&self, l: &Layout, out: &mut Vec<u32>) {
        let mut v = Vec::new();
        self.violating_residuals(l, &mut v);
        out.extend(v.into_iter().map(|(id, _)| id));
    }
    fn violating_residuals(&self, l: &Layout, out: &mut Vec<(u32, f32)>) {
        for (i, g) in self.ledgers(l).enumerate() {
            if g.known && g.usage() > 1.0 {
                let r = g.residual();
                out.push((self.cell(self.members[0]) as u32, r));
                out.push((self.cell(self.members[i + 1]) as u32, r));
            }
        }
    }
    fn offset_allowances(&self, l: &Layout, out: &mut Vec<(u32, u32, f32)>) {
        for i in 1..self.members.len() {
            let g = self.ledger(l, i);
            if g.known {
                out.push((u32::from(self.members[0].0), u32::from(self.members[i].0), (g.allowance - g.spent()).max(0.0)));
            }
        }
    }
}

/// A sky130 nfet current pair `(a, b)`: 2 × 20 µm², `Eta(0.3)`, tol 5 nm.
#[cfg(test)]
pub(crate) fn pair(a: u16, b: u16) -> MatchedSet {
    MatchedSet {
        members: vec![DeviceId(a), DeviceId(b)],
        kind: MatchKind::Current,
        mos: true,
        coeffs: Coeffs {
            avt_mv_um: Some(9.5),
            svt_uv_per_um: Some(1.63),
            kvth0_mv_um: Some(9.8),
            tc_uv_per_k: Some(765.0),
        },
        budget: Budget::Eta(0.3),
        gate_um2: vec![20.0, 20.0],
        tol_nm: 5.0,
        cell_of: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule::RuleBatch;
    use pnr_core::{Rect, Unit, UnitLib};
    use std::sync::Arc;

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

    fn unit(owner: u8, x: i32, y: i32, weight: i64) -> Unit {
        Unit { owner, x, y, weight, phi: (1, 0), sa: 0, sb: 0 }
    }

    /// Devices 0 and 1 drawn in one cell (cell 0) over `bbox`.
    fn merged(units: &[Unit], bbox: Rect) -> UnitLib {
        let alts = [(bbox, units)];
        UnitLib::build(vec![0, 0], &[vec![DeviceId(0), DeviceId(1)]], std::iter::once(&alts[..]))
    }

    /// Devices 0 and 1, one 20 µm² unit each, alone in cells 0 and 1 `dx` apart.
    fn singles(dx: i32) -> Layout {
        let one = [unit(0, 50, 50, 20_000_000)];
        let alts = [(Rect { x: 0, y: 0, w: 100, h: 100 }, &one[..])];
        let lib = UnitLib::build(vec![0, 1], &[vec![DeviceId(0)], vec![DeviceId(1)]], [&alts[..], &alts[..]].into_iter());
        let mut l = layout(&[0, dx], &[0, 0], 50);
        l.units = Arc::new(lib);
        l
    }

    /// One 800 × 100 cell drawing `order` (owner per unit) at x 100…700.
    fn row(order: [u8; 4]) -> UnitLib {
        let units: Vec<Unit> = order.iter().zip([100, 300, 500, 700]).map(|(&o, x)| unit(o, x, 50, 10)).collect();
        merged(&units, Rect { x: 0, y: 0, w: 800, h: 100 })
    }

    #[test]
    fn merged_pair_reads_its_units_not_its_cell() {
        let s = pair(0, 1);
        let mut l = layout(&[5_000], &[5_000], 400);
        l.hh[0] = 50;
        l.units = Arc::new(row([0, 1, 1, 0]));
        let g = s.ledger(&l, 1);
        assert_eq!(g.coincidence, Some(0.0));
        assert!(g.known);
        assert_eq!(s.violations(&l), 0);
        let cost = s.cost(&l);
        l.hw[0] = 4_000; // a guard-ring halo: the moments do not move
        assert_eq!(s.cost(&l), cost);
        l.units = Arc::new(row([0, 0, 1, 1]));
        assert!(s.ledger(&l, 1).usage() > 1.0);
        assert_eq!(s.violations(&l), 1);
    }

    #[test]
    fn separate_cells_a_millimetre_apart_spend_the_distance_term() {
        let g = pair(0, 1).ledger(&singles(1_000_000), 1);
        assert!((g.sigma_grad - 1.63).abs() < 0.01, "{}", g.sigma_grad);
        assert_eq!(g.coincidence, None, "one unit each admits no centroid row");
    }

    #[test]
    fn splitting_a_feasible_pair_into_two_cells_is_no_escape() {
        // ABBA-feasible counts [2, 2], drawn as two 2-unit cells 10 µm apart.
        let two = [unit(0, 25, 50, 10), unit(0, 75, 50, 10)];
        let alts = [(Rect { x: 0, y: 0, w: 100, h: 100 }, &two[..])];
        let lib = UnitLib::build(vec![0, 1], &[vec![DeviceId(0)], vec![DeviceId(1)]], [&alts[..], &alts[..]].into_iter());
        let mut l = layout(&[0, 10_000], &[0, 0], 50);
        l.units = Arc::new(lib);
        let s = pair(0, 1);
        assert_eq!(s.ledger(&l, 1).coincidence, Some(2_000.0));
        assert_eq!(s.violations(&l), 1);
    }

    #[test]
    fn ratioed_mos_pair_has_no_coincidence_check() {
        // [1, 2] passes `cc_feasible` but has no diffusion-legal row (MAT-03).
        let units = [unit(0, 300, 50, 10), unit(1, 100, 50, 10), unit(1, 500, 50, 10)];
        let mut l = layout(&[5_000], &[5_000], 400);
        l.units = Arc::new(merged(&units, Rect { x: 0, y: 0, w: 800, h: 100 }));
        let mut s = pair(0, 1);
        assert_eq!(s.ledger(&l, 1).coincidence, None);
        s.mos = false;
        assert!(s.ledger(&l, 1).coincidence.is_some());
    }

    #[test]
    fn stage_pooling_cannot_cancel() {
        // {0,1} and {2,3} offset in opposite directions: pooled sides would
        // cancel, per-pair ledgers each see the full offset.
        let d = 10_000;
        let l = layout(&[0, d, d, 0], &[0; 4], 100);
        assert_eq!(pair(0, 1).ledger(&l, 1).delta_m_nm, d as f32);
        assert_eq!(pair(2, 3).ledger(&l, 1).delta_m_nm, d as f32);
    }

    #[test]
    fn unknown_is_not_charged() {
        let mut s = pair(0, 1);
        let l = layout(&[0, 5_000], &[0, 0], 100);
        assert!(s.cost(&l) > 0.0, "the pull still applies");
        assert_eq!((s.residual(&l), s.unknown(&l)), (0.0, 1));
        s.retarget(&[0, 0]);
        assert_eq!(s.cost(&l), 0.0);
        assert_eq!((s.residual(&l), s.unknown(&l)), (0.0, 1));
    }

    #[test]
    fn units_without_avt_or_a_row_are_unknown() {
        // 1:1 single units admit no row; `Eta` without A_VT has no allowance.
        let s = MatchedSet { coeffs: Coeffs { avt_mv_um: None, ..pair(0, 1).coeffs }, ..pair(0, 1) };
        let l = singles(1_000_000);
        assert!(!s.ledger(&l, 1).known);
        assert_eq!((s.violations(&l), s.unknown(&l)), (0, 1));
    }

    #[test]
    fn thermal_reads_unit_centroids_of_a_merged_pair() {
        let lay = |a: [i32; 2], b: [i32; 2]| {
            let units: Vec<Unit> = a.iter().map(|&x| unit(0, x, 500, 10)).chain(b.iter().map(|&x| unit(1, x, 500, 10))).collect();
            let heater = [(Rect { x: 0, y: 0, w: 2_000, h: 2_000 }, &[][..])];
            let alts = [(Rect { x: 0, y: 0, w: 20_000, h: 1_000 }, &units[..])];
            let lib = UnitLib::build(vec![1, 1], &[vec![], vec![DeviceId(0), DeviceId(1)]], [&heater[..], &alts[..]].into_iter());
            let mut l = layout(&[0, 10_000], &[0, 0], 1_000);
            (l.hw[1], l.hh[1]) = (10_000, 500);
            l.power_uw[0] = 10_000;
            l.units = Arc::new(lib);
            l
        };
        let mut aabb = lay([2_500, 7_500], [12_500, 17_500]);
        let mu = pair(0, 1).ledger(&aabb, 1).mu_thermal;
        assert!((mu - 1.097).abs() < 0.01, "{mu}");
        assert!(pair(0, 1).ledger(&lay([2_500, 17_500], [7_500, 12_500]), 1).mu_thermal < 0.01);
        // `cost` adds 3e5·(μ/allowance)² over the geometric pull.
        let s = MatchedSet { budget: Budget::Allowance(1.0), ..pair(0, 1) };
        let hot = s.cost(&aabb);
        aabb.power_uw[0] = 0;
        let heat = hot - s.cost(&aabb);
        assert!((heat / (3e5 * mu * mu) - 1.0).abs() < 1e-3, "{heat}");
    }

    #[test]
    fn lod_is_weighted_by_unit_area() {
        let u = |owner, sa, sb, weight| Unit { owner, x: 100, y: 50, weight, phi: (1, 0), sa, sb };
        let units = [u(0, 500, 1_500, 10), u(0, 1_000, 1_000, 30), u(1, 1_000, 1_000, 40)];
        let mut l = layout(&[5_000], &[5_000], 400);
        l.units = Arc::new(merged(&units, Rect { x: 0, y: 0, w: 800, h: 100 }));
        let g = pair(0, 1).ledger(&l, 1);
        assert!((g.mu_lod - 1.633).abs() < 1e-3, "{}", g.mu_lod);
    }

    #[test]
    fn second_moment_separates_abba_from_abba_baab() {
        let mk = |rows: &[[u8; 4]]| {
            let units: Vec<Unit> = rows
                .iter()
                .enumerate()
                .flat_map(|(r, o)| o.iter().zip([0, 1_000, 2_000, 3_000]).map(move |(&w, x)| unit(w, x, r as i32 * 1_000, 10)))
                .collect();
            let mut l = layout(&[5_000], &[5_000], 2_000);
            l.units = Arc::new(merged(&units, Rect { x: 0, y: 0, w: 4_000, h: 4_000 }));
            pair(0, 1).ledger(&l, 1).second_order_nm
        };
        let abba = mk(&[[0, 1, 1, 0]]);
        assert!(abba > 1_000.0, "{abba}");
        assert!(mk(&[[0, 1, 1, 0], [1, 0, 0, 1]]) < 1.0);
        // Diagonal A, anti-diagonal B: equal xx and yy, only xy differs, so
        // F = √2·|Δxy| = 7.07e5 nm² over reach 707 nm.
        let diag = [unit(0, 0, 0, 10), unit(0, 1_000, 1_000, 10), unit(1, 0, 1_000, 10), unit(1, 1_000, 0, 10)];
        let mut l = layout(&[5_000], &[5_000], 2_000);
        l.units = Arc::new(merged(&diag, Rect { x: 0, y: 0, w: 4_000, h: 4_000 }));
        let so = pair(0, 1).ledger(&l, 1).second_order_nm;
        assert!((so - 1_000.0).abs() < 1.0, "{so}");
    }

    #[test]
    fn retarget_keeps_schematic_members() {
        let mut s = pair(0, 1);
        s.retarget(&[3, 3]);
        assert_eq!(s.members, vec![DeviceId(0), DeviceId(1)]);
        assert_eq!(s.cell_of, vec![3, 3]);
    }

    #[test]
    fn remaining_allowance_subtracts_placement_spend() {
        // Two one-unit cells 400 um apart: S_VT 1.0 µV/µm·nm → σ_grad 0.4 mV.
        let l = singles(400_000);
        let s = MatchedSet {
            members: vec![DeviceId(0), DeviceId(1)],
            kind: MatchKind::Current,
            mos: false,
            coeffs: Coeffs { avt_mv_um: None, svt_uv_per_um: Some(1.0), kvth0_mv_um: None, tc_uv_per_k: None },
            budget: Budget::Allowance(0.637),
            gate_um2: vec![0.0, 0.0],
            tol_nm: 5.0,
            cell_of: Vec::new(),
        };
        let mut out = Vec::new();
        s.offset_allowances(&l, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!((out[0].0, out[0].1), (0, 1));
        assert!((out[0].2 - 0.237).abs() < 1e-3, "{:?}", out[0]);

        let s = MatchedSet { budget: Budget::Allowance(0.3), ..s };
        out.clear();
        s.offset_allowances(&l, &mut out);
        assert_eq!(out[0].2, 0.0);
    }

    #[test]
    fn violating_ids_are_cells() {
        // Cell 0 is an unrelated device; devices 0 and 1 sit alone in cells 1
        // and 2, 1 mm apart (σ_grad over the allowance, as above).
        let one = [unit(0, 50, 50, 20_000_000)];
        let other = [(Rect { x: 0, y: 0, w: 100, h: 100 }, &[][..])];
        let alts = [(Rect { x: 0, y: 0, w: 100, h: 100 }, &one[..])];
        let members = [vec![], vec![DeviceId(0)], vec![DeviceId(1)]];
        let lib = UnitLib::build(vec![1, 2], &members, [&other[..], &alts[..], &alts[..]].into_iter());
        let mut l = layout(&[0, 0, 1_000_000], &[0; 3], 50);
        l.units = Arc::new(lib);
        let mut s = pair(0, 1);
        s.retarget(&[1, 2]);
        let mut ids = Vec::new();
        s.violating_ids(&l, &mut ids);
        ids.sort_unstable();
        assert_eq!(ids, vec![1, 2]);
    }
}
