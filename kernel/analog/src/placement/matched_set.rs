//! Matched sets (placement tier): one mismatch ledger and one allowance per
//! pair, replacing the separate Pelgrom distance, thermal and centroid rules
//! that each spent the same budget on their own (plan-02 MAT-04).

use pnr_core::ids::DeviceId;
use pnr_core::layout::Layout;

use crate::matching::class::{Family, MatchClass};
use crate::matching::mismatch::{
    bjt_sigma_vbe_mv, mobility_pct, ratio_thermal_pct, sigma_current_pct, sigma_pair, sigma_voltage_mv, Budget, Coeffs, Ledger, LedgerRow, LedgerUnit,
    MatchKind, GRADIENT_SHARE,
};
use crate::matching::moments::{cancelled_order, phi_equal, sums, Pt};
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
    pub family: Family,
    /// What the set's environment and limits scale with (Moderate until
    /// EXT-20 reads it from the intent).
    pub class: MatchClass,
    pub coeffs: Coeffs,
    pub budget: Budget,
    /// Netlist gate area `W·L·m` per member, µm²; read only without units.
    pub gate_um2: Vec<f32>,
    /// Coincidence tolerance, nm (half the cut lattice).
    pub tol_nm: f32,
    /// `device → cell`, set by `retarget`; empty = the ids name cells.
    pub cell_of: Vec<u16>,
    /// `g_m/I_D` of `members[0]`, 1/V: EXT-17's `gm_us/id_ua`; `None` keeps the ledger in mV.
    pub gm_over_id: Option<f32>,
}

/// Unit moments of one member plus its weighted LOD sum (`Σw·lod`, `Σw` over
/// units with a finite `lod`) and, when `hot`, its weighted rise `Σw·rise(u)`
/// (mK·weight; 0 otherwise).
///
/// ponytail: O(units·cells) per pair; the plan's 7-point stencil bounds it at
/// 7·cells but is 50 % low within ~L of a heater (card m2-analog-matching-3).
fn member(l: &Layout, d: DeviceId, hot: bool) -> (crate::matching::moments::Sums, f64, f64, f64) {
    let (mut lw, mut w, mut t) = (0.0f64, 0.0f64, 0.0f64);
    let s = sums(l.units.of_device(l, d).inspect(|u| {
        if u.lod.is_finite() {
            lw += u.weight as f64 * f64::from(u.lod);
            w += u.weight as f64;
        }
        if hot {
            t += u.weight as f64 * f64::from(l.rise_at_point_mc(u.x, u.y));
        }
    }).map(Pt::from));
    (s, lw, w, t)
}

impl MatchedSet {
    /// A non-MOS set (MAT-10): `areas_um2` are the members' netlist areas
    /// (EXT-20's), `cell_of` empty, no `g_m/I`. The ledger is in % for R/C,
    /// mV of ΔV_BE for bipolar/diode.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn for_family(
        members: Vec<DeviceId>,
        family: Family,
        kind: MatchKind,
        class: MatchClass,
        coeffs: Coeffs,
        budget: Budget,
        areas_um2: Vec<f32>,
        tol_nm: f32,
    ) -> MatchedSet {
        MatchedSet { members, kind, family, class, coeffs, budget, gate_um2: areas_um2, tol_nm, cell_of: Vec::new(), gm_over_id: None }
    }

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
        let hot = l.power_uw.iter().any(|&p| p != 0);
        let ((sa, lwa, wa, ta), (sb, lwb, wb, tb)) = (member(l, a, hot), member(l, b, hot));
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
            if self.family == Family::Mos {
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

        let (a0, ai) = self.areas(&sa, &sb, i);
        let c = &self.coeffs;
        let ka = c.ka_pct_um.map(|k| sigma_pair(k, a0, ai));
        let (unit, sigma_rand) = match self.family {
            Family::Mos => {
                let sigma_vt = c.avt_mv_um.map_or(0.0, |av| sigma_pair(av, a0, ai));
                // MAT-09: with A_β and G a mirror's ledger is in % (eq. 13.43) and a
                // pair's σ gains the β share (eq. 13.42); without them, mV as before.
                match (c.abeta_pct_um.zip(self.gm_over_id), self.kind) {
                    (Some((ab, g)), MatchKind::Current) => (LedgerUnit::Pct, sigma_current_pct(sigma_vt, g, sigma_pair(ab, a0, ai))),
                    (Some((ab, g)), _) => (LedgerUnit::Mv, sigma_voltage_mv(sigma_vt, g, sigma_pair(ab, a0, ai))),
                    (None, _) => (LedgerUnit::Mv, sigma_vt),
                }
            }
            Family::Resistor | Family::Capacitor => (LedgerUnit::Pct, ka.unwrap_or(0.0)),
            Family::Bipolar | Family::Diode => (LedgerUnit::Mv, ka.map_or(0.0, bjt_sigma_vbe_mv)),
        };
        let budgeted = sigma_rand > 0.0 || matches!(self.budget, Budget::Allowance(_));

        let (mut sigma_grad, mut mu_lod, mut mu_thermal) = (0.0, 0.0, 0.0);
        if budgeted {
            // |ΔT̄|, mK (0 without power): unit-weighted mean rise per member,
            // the centroid samples without units.
            let dt_mk = if !hot {
                0.0
            } else if units {
                (ta / sa.w - tb / sb.w).abs() as f32
            } else {
                let rise = |(x, y): (f64, f64)| l.rise_at_point_mc(x.round() as i32, y.round() as i32);
                (rise(ca) - rise(cb)).abs()
            };
            match self.family {
                Family::Mos => {
                    // µV/µm · nm → mV.
                    sigma_grad = c.svt_uv_per_um.unwrap_or(0.0) * delta_m_nm * 1e-6;
                    if units && wa > 0.0 && wb > 0.0 {
                        mu_lod = c.kvth0_mv_um.unwrap_or(0.0) * ((lwa / wa - lwb / wb).abs() as f32);
                    }
                    // µV/K · mK → mV.
                    mu_thermal = c.tc_uv_per_k.unwrap_or(0.0) * dt_mk * 1e-6;
                    // mV systematic terms → % of current: ΔI/I = G·ΔV (×0.1 for mV → %).
                    if unit == LedgerUnit::Pct {
                        let k = 0.1 * self.gm_over_id.unwrap_or(0.0);
                        (sigma_grad, mu_thermal, mu_lod) = (k * sigma_grad, k * mu_thermal, k * mu_lod);
                        // MAT-14: a mirror's β ∝ T^−exp adds by magnitude; an mV ledger has no G for it.
                        if let (MatchKind::Current, Some(e), Some(t)) = (self.kind, c.mobility_exp, c.die_temp_k) {
                            mu_thermal += mobility_pct(e, dt_mk / 1e3, t).abs();
                        }
                    }
                }
                Family::Resistor | Family::Capacitor => {
                    // %/mm · nm → %.
                    sigma_grad = c.sd_pct_per_mm.unwrap_or(0.0) * delta_m_nm * 1e-6;
                    mu_thermal = ratio_thermal_pct(c.tc_ppm_per_k.unwrap_or(0.0), dt_mk);
                }
                // µV/K · mK → mV.
                Family::Bipolar | Family::Diode => mu_thermal = c.vbe_tc_uv_per_k.unwrap_or(0.0) * dt_mk * 1e-6,
            }
        }
        Ledger {
            sigma_rand,
            sigma_grad,
            mu_thermal,
            mu_lod,
            allowance: self.budget_in(unit).allowance(sigma_rand),
            unit,
            coincidence,
            second_order_nm,
            delta_m_nm,
            known: units && (budgeted || coincidence.is_some()),
        }
    }

    /// Areas of pair `(0, i)`, µm²: MOS unit weights (gate nm²) when both
    /// members have units, else `gate_um2` (always for R/C/BJT: a resistor
    /// unit's weight is not its area).
    fn areas(&self, sa: &crate::matching::moments::Sums, sb: &crate::matching::moments::Sums, i: usize) -> (f32, f32) {
        if self.family == Family::Mos && sa.w > 0.0 && sb.w > 0.0 {
            ((sa.w / 1e6) as f32, (sb.w / 1e6) as f32)
        } else {
            (self.gate_um2.first().copied().unwrap_or(0.0), self.gate_um2.get(i).copied().unwrap_or(0.0))
        }
    }

    /// The budget in a ledger's unit: [`Budget::to_pct`] on a mirror's %
    /// ledger. A budget of the other unit with no G to convert it (a `Sigma1Pct`
    /// on an mV ledger, a `Sigma1Mv` on an R/C % ledger) is
    /// `Eta(GRADIENT_SHARE)`. An `Allowance` on an R/C ledger is read as %.
    #[must_use]
    pub fn budget_in(&self, unit: LedgerUnit) -> Budget {
        match (unit, self.gm_over_id, self.budget) {
            (LedgerUnit::Pct, Some(g), b) => b.to_pct(g),
            (LedgerUnit::Pct, None, Budget::Sigma1Mv(_)) | (LedgerUnit::Mv, _, Budget::Sigma1Pct(_)) => Budget::Eta(GRADIENT_SHARE),
            (_, _, b) => b,
        }
    }

    /// The areas [`Self::ledger`] reads for pair `(0, i)` on `l`, µm².
    #[must_use]
    pub fn pair_areas(&self, l: &Layout, i: usize) -> (f32, f32) {
        self.areas(&member(l, self.members[0], false).0, &member(l, self.members[i], false).0, i)
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
    fn matched_pairs(&self, out: &mut Vec<(u32, u32)>) {
        let Some((&m0, rest)) = self.members.split_first() else { return };
        let c0 = self.cell(m0) as u32;
        out.extend(rest.iter().map(|&m| (c0, self.cell(m) as u32)));
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
    /// Report only: allocates the members' units per pair.
    fn ledger_rows(&self, l: &Layout, out: &mut Vec<LedgerRow>) {
        let (a, sa) = (self.members[0], member(l, self.members[0], false).0);
        let pa: Vec<Pt> = l.units.of_device(l, a).map(Pt::from).collect();
        for i in 1..self.members.len() {
            let b = self.members[i];
            let g = self.ledger(l, i);
            let sb = member(l, b, false).0;
            let units = sa.w > 0.0 && sb.w > 0.0;
            let pb: Vec<Pt> = l.units.of_device(l, b).map(Pt::from).collect();
            out.push(LedgerRow {
                members: (u32::from(a.0), u32::from(b.0)),
                unit: g.unit.as_str(),
                sigma_rand: g.sigma_rand,
                sigma_layout: g.sigma_grad,
                mu_thermal: g.mu_thermal,
                mu_lod: g.mu_lod,
                allowance: g.allowance,
                usage: g.usage(),
                order: if units { cancelled_order(&[&pa, &pb], 4, 1e-3).0 } else { 0 },
                second_order_nm: g.second_order_nm,
                phi_equal: units.then(|| phi_equal(&sa, &sb)),
                known: g.known,
                sizing_limited: matches!(self.budget_in(g.unit), Budget::Sigma1Mv(b) | Budget::Sigma1Pct(b) if g.sigma_rand > 0.0 && g.sigma_rand >= b),
            });
        }
    }
    fn sizing_notes(&self, l: &Layout, out: &mut Vec<crate::matching::sizing::SizingNote>) {
        // ponytail: EXT-20 passes the intent class's limit.
        out.extend(crate::matching::sizing::notes(self, l, None));
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
        family: Family::Mos,
        class: MatchClass::Moderate,
        coeffs: Coeffs {
            avt_mv_um: Some(9.5),
            svt_uv_per_um: Some(1.63),
            kvth0_mv_um: Some(9.8),
            tc_uv_per_k: Some(765.0),
            ..Coeffs::default()
        },
        budget: Budget::Eta(0.3),
        gate_um2: vec![20.0, 20.0],
        tol_nm: 5.0,
        cell_of: Vec::new(),
        gm_over_id: None,
    }
}

/// Cells at `(xs, ys)`, each `2·half` square, no units, no power.
#[cfg(test)]
pub(crate) fn layout(xs: &[i32], ys: &[i32], half: i32) -> Layout {
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

#[cfg(test)]
pub(crate) fn unit(owner: u8, x: i32, y: i32, weight: i64) -> pnr_core::Unit {
    pnr_core::Unit { owner, x, y, weight, phi: (1, 0), sa: 0, sb: 0 }
}

/// Devices 0 and 1, one 20 µm² unit each, alone in cells 0 and 1 `dx` apart.
#[cfg(test)]
pub(crate) fn singles(dx: i32) -> Layout {
    let one = [unit(0, 50, 50, 20_000_000)];
    let alts = [(pnr_core::Rect { x: 0, y: 0, w: 100, h: 100 }, &one[..])];
    let lib = pnr_core::UnitLib::build(vec![0, 1], &[vec![DeviceId(0)], vec![DeviceId(1)]], [&alts[..], &alts[..]].into_iter());
    let mut l = layout(&[0, dx], &[0, 0], 50);
    l.units = std::sync::Arc::new(lib);
    l
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule::RuleBatch;
    use pnr_core::{Rect, Unit, UnitLib};
    use std::sync::Arc;

    /// Devices 0 and 1 drawn in one cell (cell 0) over `bbox`.
    fn merged(units: &[Unit], bbox: Rect) -> UnitLib {
        let alts = [(bbox, units)];
        UnitLib::build(vec![0, 0], &[vec![DeviceId(0), DeviceId(1)]], std::iter::once(&alts[..]))
    }

    /// One 800 × 100 cell drawing `order` (owner per unit) at x 100…700.
    fn row(order: [u8; 4]) -> UnitLib {
        let units: Vec<Unit> = order.iter().zip([100, 300, 500, 700]).map(|(&o, x)| unit(o, x, 50, 10)).collect();
        merged(&units, Rect { x: 0, y: 0, w: 800, h: 100 })
    }

    #[test]
    fn ledger_rows_one_per_pair() {
        let l = singles(1_000);
        let mut s = pair(0, 1);
        let mut rows = Vec::new();
        s.ledger_rows(&l, &mut rows);
        let g = s.ledger(&l, 1);
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        assert_eq!((r.members, r.unit, r.known), ((0, 1), "mV", g.known));
        assert_eq!((r.sigma_rand, r.sigma_layout, r.mu_thermal, r.mu_lod), (g.sigma_rand, g.sigma_grad, g.mu_thermal, g.mu_lod));
        assert_eq!((r.allowance, r.usage, r.second_order_nm), (g.allowance, g.usage(), g.second_order_nm));
        assert_eq!(r.phi_equal, Some(true));
        assert!(!r.sizing_limited);
        // 20 µm² each: σ_rand 2.124 mV ≥ a 1 mV total, nothing left for layout.
        s.budget = Budget::Sigma1Mv(1.0);
        rows.clear();
        s.ledger_rows(&l, &mut rows);
        assert!((rows[0].sigma_rand - 2.124).abs() < 1e-3, "{}", rows[0].sigma_rand);
        assert_eq!(rows[0].allowance, 0.0);
        assert!(rows[0].sizing_limited);
        // ABBA cancels the first moment order (common centroid); AABB none.
        let mut l = layout(&[5_000], &[5_000], 400);
        l.units = Arc::new(row([0, 1, 1, 0]));
        rows.clear();
        pair(0, 1).ledger_rows(&l, &mut rows);
        assert_eq!(rows[0].order, 1);
        l.units = Arc::new(row([0, 0, 1, 1]));
        rows.clear();
        pair(0, 1).ledger_rows(&l, &mut rows);
        assert_eq!(rows[0].order, 0);
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
    fn default_pair_is_moderate_mos() {
        let s = pair(0, 1);
        assert_eq!((s.family, s.class), (Family::Mos, MatchClass::Moderate));
    }

    #[test]
    fn ratioed_mos_pair_has_no_coincidence_check() {
        // [1, 2] passes `cc_feasible` but has no diffusion-legal row (MAT-03).
        let units = [unit(0, 300, 50, 10), unit(1, 100, 50, 10), unit(1, 500, 50, 10)];
        let mut l = layout(&[5_000], &[5_000], 400);
        l.units = Arc::new(merged(&units, Rect { x: 0, y: 0, w: 800, h: 100 }));
        let mut s = pair(0, 1);
        assert_eq!(s.ledger(&l, 1).coincidence, None);
        s.family = Family::Resistor;
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

    /// Unit-weighted mean rise of `owner`'s units on `l`, mK (equal weights).
    fn brute_t(l: &Layout, xs: &[i32], y: i32) -> f32 {
        xs.iter().map(|&x| l.rise_at_point_mc(x, y)).sum::<f32>() / xs.len() as f32
    }

    /// Units of devices 0/1 at `a`/`b` (y `uy`, weight 10) merged in cell 1
    /// (centre `cx`, half-extents 10 µm × 500 nm); cell 0 a 10 mW heater at
    /// `hx` with half-extent `hh`.
    fn heated(a: &[i32], b: &[i32], uy: i32, cx: i32, hx: i32, hh: i32) -> Layout {
        let units: Vec<Unit> = a.iter().map(|&x| unit(0, x, uy, 10)).chain(b.iter().map(|&x| unit(1, x, uy, 10))).collect();
        let heater = [(Rect { x: 0, y: 0, w: 2 * hh, h: 2 * hh }, &[][..])];
        let alts = [(Rect { x: 0, y: 0, w: 20_000, h: 1_000 }, &units[..])];
        let lib = UnitLib::build(vec![1, 1], &[vec![], vec![DeviceId(0), DeviceId(1)]], [&heater[..], &alts[..]].into_iter());
        let mut l = layout(&[hx, cx], &[0, 0], hh);
        (l.hw[1], l.hh[1]) = (10_000, 500);
        l.power_uw[0] = 10_000;
        l.units = Arc::new(lib);
        l
    }

    #[test]
    fn thermal_averages_the_field_over_units() {
        // Placed unit = cell origin (centre − half) + local offset: y 0, in
        // line with the heater (P/(2πkr): AABB 2130 mK, ABBA 1311 mK).
        let lay = |a: [i32; 2], b: [i32; 2]| heated(&a, &b, 500, 10_000, 0, 1_000);
        let check = |a: [i32; 2], b: [i32; 2]| {
            let l = lay(a, b);
            let pa: Vec<i32> = l.units.of_device(&l, DeviceId(0)).map(|u| u.x).collect();
            let pb: Vec<i32> = l.units.of_device(&l, DeviceId(1)).map(|u| u.x).collect();
            let y = l.units.of_device(&l, DeviceId(0)).next().unwrap().y;
            let brute = 765e-6 * (brute_t(&l, &pa, y) - brute_t(&l, &pb, y)).abs();
            let mu = pair(0, 1).ledger(&l, 1).mu_thermal;
            assert!((mu - brute).abs() < 1e-3 * brute, "{mu} vs {brute}");
            mu
        };
        let mu = check([2_500, 7_500], [12_500, 17_500]);
        assert!((mu - 1.630).abs() < 2e-3, "{mu}");
        let abba = check([2_500, 17_500], [7_500, 12_500]);
        assert!(abba > 0.5 && (abba - 1.003).abs() < 2e-3, "{abba}");
        // `cost` adds 3e5·(μ/allowance)² over the geometric pull.
        let mut aabb = lay([2_500, 7_500], [12_500, 17_500]);
        let s = MatchedSet { budget: Budget::Allowance(1.0), ..pair(0, 1) };
        let hot = s.cost(&aabb);
        aabb.power_uw[0] = 0;
        let heat = hot - s.cost(&aabb);
        assert!((heat / (3e5 * mu * mu) - 1.0).abs() < 1e-3, "{heat}");
    }

    /// Placed x of `d`'s units on `l`, and their common y.
    fn xs(l: &Layout, d: u16) -> (Vec<i32>, i32) {
        let u: Vec<_> = l.units.of_device(l, DeviceId(d)).collect();
        (u.iter().map(|u| u.x).collect(), u[0].y)
    }

    /// 1-row units at x = 0…3 µm, a 10 mW heater (half-extent 1 µm) 8 µm away.
    fn near_heater(order: [u8; 4]) -> Layout {
        let (a, b): (Vec<i32>, Vec<i32>) = {
            let a = order.iter().zip([0, 1_000, 2_000, 3_000]).filter(|(&o, _)| o == 0).map(|(_, x)| x).collect();
            let b = order.iter().zip([0, 1_000, 2_000, 3_000]).filter(|(&o, _)| o == 1).map(|(_, x)| x).collect();
            (a, b)
        };
        // Cell 1 centred at 10 µm: local x + 0 → placed x 0…3 µm; heater at 8 µm.
        heated(&a, &b, 500, 10_000, 8_000, 1_000)
    }

    #[test]
    fn thermal_matches_unit_sampling() {
        let brute = |l: &Layout| {
            let ((pa, y), (pb, _)) = (xs(l, 0), xs(l, 1));
            765e-6 * (brute_t(l, &pa, y) - brute_t(l, &pb, y)).abs()
        };
        let l = near_heater([0, 0, 1, 1]);
        let (mu, b) = (pair(0, 1).ledger(&l, 1).mu_thermal, brute(&l));
        assert!((mu - b).abs() < 1e-3 * b && (b - 0.406).abs() < 0.01, "{mu} vs {b}");
        let l = near_heater([0, 1, 1, 0]);
        let g = pair(0, 1).ledger(&l, 1);
        let b = brute(&l);
        assert_eq!(g.delta_m_nm, 0.0);
        assert!((g.mu_thermal - b).abs() < 1e-3 * b && g.mu_thermal > 0.05, "{} vs {b}", g.mu_thermal);
        assert!((b - 0.0637).abs() < 0.005, "{b}");
    }

    #[test]
    fn mobility_adds_on_a_pct_mirror() {
        let l = near_heater([0, 0, 1, 1]);
        let ((pa, y), (pb, _)) = (xs(&l, 0), xs(&l, 1));
        let dt = (brute_t(&l, &pa, y) - brute_t(&l, &pb, y)).abs();
        let base = pair(0, 1);
        let pct = |exp| MatchedSet {
            coeffs: Coeffs { abeta_pct_um: Some(2.2), mobility_exp: exp, die_temp_k: Some(300.0), ..base.coeffs },
            gm_over_id: Some(10.0),
            ..pair(0, 1)
        };
        let (with, without) = (pct(Some(1.7)).ledger(&l, 1).mu_thermal, pct(None).ledger(&l, 1).mu_thermal);
        let want = 1.7 * (dt / 1000.0) / 300.0 * 100.0;
        assert!((with - without - want).abs() < 1e-4, "{with} − {without} vs {want}");
        let mv = |exp| MatchedSet { gm_over_id: None, ..pct(exp) }.ledger(&l, 1).mu_thermal;
        assert_eq!(mv(Some(1.7)), mv(None));
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
        // Two one-unit cells 400 µm apart: S_D 1 %/mm → σ_grad 0.4 %.
        let l = singles(400_000);
        let s = MatchedSet {
            members: vec![DeviceId(0), DeviceId(1)],
            kind: MatchKind::Current,
            family: Family::Resistor,
            class: MatchClass::Moderate,
            coeffs: Coeffs { sd_pct_per_mm: Some(1.0), ..Coeffs::default() },
            budget: Budget::Allowance(0.637),
            gate_um2: vec![0.0, 0.0],
            tol_nm: 5.0,
            cell_of: Vec::new(),
            gm_over_id: None,
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
    fn mirror_row_in_pct_with_abeta() {
        let l = singles(1_000);
        let rows = |s: &MatchedSet| {
            let mut out = Vec::new();
            s.ledger_rows(&l, &mut out);
            out.remove(0)
        };
        let base = pair(0, 1);
        let mv = rows(&base);
        let sigma_vt = sigma_pair(9.5, 20.0, 20.0);
        let sb = sigma_pair(1.0, 20.0, 20.0);
        let s = MatchedSet { coeffs: Coeffs { abeta_pct_um: Some(1.0), ..base.coeffs }, gm_over_id: Some(10.0), ..pair(0, 1) };
        let r = rows(&s);
        assert_eq!(r.unit, "%");
        assert_eq!(r.sigma_rand, sigma_current_pct(sigma_vt, 10.0, sb));
        assert!(r.sigma_rand > 0.1 * 10.0 * sigma_vt + 1e-3, "{r:?}");
        let r = rows(&MatchedSet { gm_over_id: None, ..s.clone() });
        assert_eq!((r.unit, r.sigma_rand), ("mV", mv.sigma_rand));
        let r = rows(&MatchedSet { kind: MatchKind::Voltage, ..s });
        assert_eq!(r.unit, "mV");
        assert!(r.sigma_rand > sigma_vt, "{r:?}");
    }

    #[test]
    fn for_family_dispatches() {
        let l = singles(1_000);
        let row = |s: &MatchedSet| {
            let mut out = Vec::new();
            s.ledger_rows(&l, &mut out);
            out.remove(0)
        };
        let set = |family, ka| {
            let coeffs = Coeffs { ka_pct_um: ka, sd_pct_per_mm: Some(1.0), tc_ppm_per_k: Some(100.0), ..Coeffs::default() };
            let ids = vec![DeviceId(0), DeviceId(1)];
            MatchedSet::for_family(ids, family, MatchKind::Ratio, MatchClass::Moderate, coeffs, Budget::Eta(0.3), vec![36.0, 36.0], 5.0)
        };
        let r = row(&set(Family::Resistor, Some(2.0)));
        assert_eq!((r.unit, r.known, r.mu_lod), ("%", true, 0.0), "{r:?}");
        // B2: the netlist 36 µm², not the 20 µm² unit weight.
        assert!((r.sigma_rand - sigma_pair(2.0, 36.0, 36.0)).abs() < 1e-6, "{r:?}");
        assert!((r.sigma_layout - 1e-3).abs() < 1e-6, "1 %/mm over 1 µm: {r:?}");
        assert!(!row(&set(Family::Resistor, None)).known);
        let r = row(&set(Family::Bipolar, Some(2.0)));
        assert_eq!((r.unit, r.sigma_layout), ("mV", 0.0));
        assert!((r.sigma_rand - bjt_sigma_vbe_mv(sigma_pair(2.0, 36.0, 36.0))).abs() < 1e-7, "{r:?}");
    }

    /// A budget of the other unit with no G is `Eta(GRADIENT_SHARE)`: a % class
    /// on an mV pair, an mV offset on an R/C ratio.
    #[test]
    fn budget_in_never_crosses_units_without_g() {
        let mv = MatchedSet { budget: Budget::Sigma1Pct(0.5), ..pair(0, 1) };
        assert_eq!(mv.budget_in(LedgerUnit::Mv), Budget::Eta(GRADIENT_SHARE));
        let mirror = MatchedSet { gm_over_id: Some(10.0), ..mv };
        assert_eq!(mirror.budget_in(LedgerUnit::Pct), Budget::Sigma1Pct(0.5));
        let rc = MatchedSet { family: Family::Resistor, budget: Budget::Sigma1Mv(1.0), ..pair(0, 1) };
        assert_eq!(rc.budget_in(LedgerUnit::Pct), Budget::Eta(GRADIENT_SHARE));
        assert_eq!(MatchedSet { budget: Budget::Sigma1Pct(0.5), ..rc }.budget_in(LedgerUnit::Pct), Budget::Sigma1Pct(0.5));
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
