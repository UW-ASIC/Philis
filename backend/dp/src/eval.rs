//! Incremental evaluation for the SP anneal (PLC-10): per-net weighted HPWL and
//! per-batch values cached; a move recomputes only the nets of the cells it
//! moved and the batches whose [`analog::RuleBatch::touched`] ids it moved,
//! plus every global batch (no touched ids: `Utilization`,
//! `LiveEnvironment`, …; or [`analog::RuleBatch::reads_field`]: `MatchedSet`). Totals are re-summed in the order
//! [`gp::mechanics`] sums them, so a cached value is bit-identical to a full
//! evaluation, not merely close.

use analog::Requirements;
use gp::mechanics::Nets;
use pnr_core::Layout;

/// One batch's cached numbers: hard `(residual, 0, violations)`, budget
/// `(residual, priced term, 0)`, cost `(criticality·cost, 0, 0)`.
type Row = (f64, f64, u32);

/// Cached per-net and per-batch terms of the SP anneal's energy, with an
/// undo log: [`Eval::revert`] restores every value replaced since the last
/// [`Eval::commit`] (or [`Eval::full`]).
///
/// Invariant: after `full`/`update`, every cached value equals what a fresh
/// evaluation of the current layout would give for it, provided the moved
/// cells passed to `update` are every cell whose geometry changed.
pub(crate) struct Eval {
    /// Weighted HPWL per net row, nm.
    net_hpwl: Vec<f64>,
    /// Batches flattened: hard, then budget, then cost.
    row: Vec<Row>,
    /// `reqs.hard.len()`: rows `[0, n_hard)` are hard.
    n_hard: usize,
    /// `reqs.budget.len()`: rows `[n_hard, n_hard + n_budget)` are budget.
    n_budget: usize,
    /// Cell → net rows it touches.
    cell_nets: Vec<Vec<u32>>,
    /// Cell → non-global rows whose `touched` ids include it.
    cell_rows: Vec<Vec<u32>>,
    /// Rows recomputed on every update (no touched ids, or field readers).
    global: Vec<u32>,
    /// Net row → stamp of the update that last recomputed it (dedup).
    seen_n: Vec<u32>,
    /// Batch row → stamp of the update that last recomputed it (dedup).
    seen_r: Vec<u32>,
    /// Current update's stamp; `0` is never live (wraparound clears the marks).
    stamp: u32,
    /// `(net row, old value)` replaced since the last commit.
    undo_n: Vec<(u32, f64)>,
    /// `(batch row, old value)` replaced since the last commit.
    undo_r: Vec<(u32, Row)>,
}

impl Eval {
    /// Index of every batch of `reqs` and every net of `nets` over `n` cells;
    /// every cached value starts at zero until [`Eval::full`]. Touched ids
    /// `>= n` are ignored.
    ///
    /// # Panics
    /// When a net names a device `>= n`.
    pub(crate) fn new(reqs: &Requirements<Layout>, nets: &Nets, n: usize) -> Self {
        let (n_hard, n_budget) = (reqs.hard.len(), reqs.budget.len());
        let mut cell_rows = vec![Vec::new(); n];
        let mut global = Vec::new();
        let mut ids = Vec::new();
        for (i, b) in reqs.hard.iter().chain(&reqs.budget).chain(&reqs.cost).enumerate() {
            ids.clear();
            b.touched(&mut ids);
            ids.sort_unstable();
            ids.dedup();
            // ponytail: a field reader is global, not "members ∪ powered cells":
            // that refinement (3.3× on dac4) lost exactness on opamp3, whose power
            // is all 0, so `MatchedSet::touched` misses a cell its units read
            // (reported to the matching owner). Narrow it once `touched` is exact.
            if ids.is_empty() || b.reads_field() {
                global.push(i as u32);
                continue;
            }
            for &c in ids.iter().filter(|&&c| (c as usize) < n) {
                cell_rows[c as usize].push(i as u32);
            }
        }
        let rows = n_hard + n_budget + reqs.cost.len();
        Eval {
            net_hpwl: vec![0.0; nets.count()],
            row: vec![(0.0, 0.0, 0); rows],
            n_hard,
            n_budget,
            cell_nets: nets.cell_nets(n),
            cell_rows,
            global,
            seen_n: vec![0; nets.count()],
            seen_r: vec![0; rows],
            stamp: 0,
            undo_n: Vec::new(),
            undo_r: Vec::new(),
        }
    }

    /// Weighted HPWL of net row `ni` under `l`, nm.
    fn net(nets: &Nets, l: &Layout, ni: usize) -> f64 {
        let (mut x0, mut x1, mut y0, mut y1) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
        for k in nets.span(ni) {
            let (px, py) = nets.pin(k, l);
            (x0, x1, y0, y1) = (x0.min(px), x1.max(px), y0.min(py), y1.max(py));
        }
        f64::from(nets.weight(ni)) * f64::from((x1 - x0) + (y1 - y0))
    }

    /// Fresh value of flattened batch row `i` (see [`Row`]); a budget's priced
    /// term is `λ·r⁺ + ρ/2·r⁺²`.
    fn row_of(&self, i: usize, reqs: &Requirements<Layout>, l: &Layout, prices: &gp::Prices) -> Row {
        if i < self.n_hard {
            let b = &reqs.hard[i];
            (b.residual(l), 0.0, b.violations(l))
        } else if i < self.n_hard + self.n_budget {
            let bi = i - self.n_hard;
            let raw = reqs.budget[bi].residual(l);
            let r = raw.max(0.0);
            (raw, f64::from(prices.weight_of(bi)) * r + 0.5 * f64::from(prices.rho_of(bi)) * r * r, 0)
        } else {
            let b = &reqs.cost[i - self.n_hard - self.n_budget];
            (f64::from(b.criticality(l) * b.cost(l)), 0.0, 0)
        }
    }

    /// Recompute everything (start, thermal refresh, branch flip); clears the undo log.
    pub(crate) fn full(&mut self, reqs: &Requirements<Layout>, nets: &Nets, l: &Layout, prices: &gp::Prices) {
        for ni in 0..self.net_hpwl.len() {
            self.net_hpwl[ni] = Self::net(nets, l, ni);
        }
        for i in 0..self.row.len() {
            self.row[i] = self.row_of(i, reqs, l, prices);
        }
        self.commit();
    }

    /// Recompute the nets and batches `moved` cells reach, plus the global
    /// batches (`all` = every batch: a branch flip moves no cell), logging old
    /// values for [`Eval::revert`]. Each net and row is recomputed at most once
    /// per call.
    ///
    /// # Panics
    /// When a moved cell is `>= n`.
    pub(crate) fn update(&mut self, moved: &[usize], all: bool, reqs: &Requirements<Layout>, nets: &Nets, l: &Layout, prices: &gp::Prices) {
        self.stamp = self.stamp.wrapping_add(1);
        if self.stamp == 0 {
            self.seen_n.fill(0);
            self.seen_r.fill(0);
            self.stamp = 1;
        }
        let s = self.stamp;
        for &c in moved {
            for k in 0..self.cell_nets[c].len() {
                let ni = self.cell_nets[c][k] as usize;
                if std::mem::replace(&mut self.seen_n[ni], s) != s {
                    self.undo_n.push((ni as u32, self.net_hpwl[ni]));
                    self.net_hpwl[ni] = Self::net(nets, l, ni);
                }
            }
        }
        let redo = |e: &mut Self, i: usize| {
            if std::mem::replace(&mut e.seen_r[i], s) != s {
                e.undo_r.push((i as u32, e.row[i]));
                e.row[i] = e.row_of(i, reqs, l, prices);
            }
        };
        if all {
            for i in 0..self.row.len() {
                redo(self, i);
            }
            return;
        }
        for &c in moved {
            for k in 0..self.cell_rows[c].len() {
                let i = self.cell_rows[c][k] as usize;
                redo(self, i);
            }
        }
        for k in 0..self.global.len() {
            let i = self.global[k] as usize;
            redo(self, i);
        }
    }

    /// Restore every value the [`Eval::update`]s since the last commit replaced.
    pub(crate) fn revert(&mut self) {
        while let Some((i, v)) = self.undo_n.pop() {
            self.net_hpwl[i as usize] = v;
        }
        while let Some((i, v)) = self.undo_r.pop() {
            self.row[i as usize] = v;
        }
    }

    /// Keep the values the last [`Eval::update`] wrote (drop the undo log).
    pub(crate) fn commit(&mut self) {
        self.undo_n.clear();
        self.undo_r.clear();
    }

    /// Φ as [`gp::mechanics::analog_phi`]: `(violating hard batches, Σ their residuals)`.
    pub(crate) fn phi(&self) -> (usize, f64) {
        let mut out = (0, 0.0);
        for r in &self.row[..self.n_hard] {
            if r.2 > 0 {
                out.0 += 1;
                out.1 += r.0;
            }
        }
        out
    }

    /// Θ as [`gp::mechanics::analog_theta`].
    pub(crate) fn theta(&self) -> f64 {
        self.row[self.n_hard..self.n_hard + self.n_budget].iter().map(|r| r.0).sum()
    }

    /// `E` exactly as [`crate::anneal`]'s full `energy`: HPWL/L_ref + area term + augmented cost.
    pub(crate) fn energy(&self, l: &Layout) -> f64 {
        let mut hpwl = 0.0f64;
        for &v in &self.net_hpwl {
            hpwl += v;
        }
        let cells: f64 = l.hw.iter().zip(&l.hh).map(|(&w, &h)| 4.0 * f64::from(w) * f64::from(h)).sum();
        let area = if cells > 0.0 { l.footprint_nm2() / cells - 1.0 } else { 0.0 };
        let hb = self.n_hard + self.n_budget;
        let objective: f64 = self.row[hb..].iter().map(|r| r.0).sum();
        let priced: f64 = self.row[self.n_hard..hb].iter().map(|r| r.1).sum();
        hpwl / f64::from(l.l_ref()) + area + (objective + priced)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use analog::RuleBatch;
    use gp::mechanics::{analog_phi, analog_theta, augmented_cost, hpwl};
    use pnr_core::geom::Rect;
    use pnr_core::ids::DeviceId;
    use pnr_core::{LayerId, Macro, NetId, Orient, Pin};

    /// Reads `l.x[cell]` (and `l.branch[0]` when `branchy`); claims to touch
    /// only `touched`. Violated past `limit`, residual `x / 1000`.
    struct XBatch {
        cell: usize,
        touched: Vec<u32>,
        field: bool,
        branchy: bool,
        limit: i32,
    }

    impl XBatch {
        fn local(cell: usize, limit: i32) -> Self {
            XBatch { cell, touched: vec![cell as u32], field: false, branchy: false, limit }
        }
        fn value(&self, l: &Layout) -> i32 {
            l.x[self.cell] + if self.branchy && l.branch.first() == Some(&true) { 7_000 } else { 0 }
        }
    }

    impl RuleBatch<Layout> for XBatch {
        fn cost(&self, l: &Layout) -> f32 {
            self.value(l).abs() as f32
        }
        fn violations(&self, l: &Layout) -> u32 {
            u32::from(self.value(l) > self.limit)
        }
        fn residual(&self, l: &Layout) -> f64 {
            f64::from(self.value(l)) / 1_000.0
        }
        fn reads_field(&self) -> bool {
            self.field
        }
        fn touched(&self, out: &mut Vec<u32>) {
            out.extend_from_slice(&self.touched);
        }
    }

    fn layout(xs: &[i32]) -> Layout {
        let n = xs.len();
        Layout {
            x: xs.to_vec(),
            y: vec![0; n],
            hw: vec![500; n],
            hh: vec![500; n],
            variant: vec![0; n],
            axis: vec![0; n],
            branch: vec![false; n],
            groups: (0..n).map(|i| vec![DeviceId(i as u16)]).collect(),
            orient: vec![Orient::R0; n],
            power_uw: vec![0; n],
            temp_mc: vec![0; n],
            units: Default::default(),
        }
    }

    /// 1 µm cell with one pin on each of `nets`, at the centre.
    fn cell(nets: &[u16]) -> Macro {
        let pins = nets
            .iter()
            .map(|&n| Pin { name: format!("P{n}"), net: NetId(n), at: Rect { x: 450, y: 450, w: 100, h: 100 }, layer: LayerId(0) })
            .collect();
        Macro { pins, bbox: Rect { x: 0, y: 0, w: 1_000, h: 1_000 }, ..Default::default() }
    }

    /// Reference energy: the full evaluation `anneal::energy` performs.
    fn reference(nets: &Nets, reqs: &Requirements<Layout>, l: &Layout, prices: &gp::Prices) -> f64 {
        let cells: f64 = l.hw.iter().zip(&l.hh).map(|(&w, &h)| 4.0 * f64::from(w) * f64::from(h)).sum();
        let area = if cells > 0.0 { l.footprint_nm2() / cells - 1.0 } else { 0.0 };
        hpwl(nets, l) / f64::from(l.l_ref()) + area + augmented_cost(reqs, l, prices)
    }

    fn assert_exact(e: &Eval, nets: &Nets, reqs: &Requirements<Layout>, l: &Layout, prices: &gp::Prices) {
        assert_eq!(e.phi(), analog_phi(reqs, l));
        assert_eq!(e.theta().to_bits(), analog_theta(reqs, l).to_bits());
        assert_eq!(e.energy(l).to_bits(), reference(nets, reqs, l, prices).to_bits(), "{} vs {}", e.energy(l), reference(nets, reqs, l, prices));
    }

    /// Three cells, two nets, one batch per tier plus a global cost batch.
    fn bench() -> (Nets, Requirements<Layout>, Layout) {
        let nets = Nets::from_macros(&[cell(&[0]), cell(&[0, 1]), cell(&[1])]);
        let reqs = Requirements {
            hard: vec![Box::new(XBatch::local(0, 1_000)) as Box<dyn RuleBatch<Layout>>],
            budget: vec![Box::new(XBatch::local(1, 0))],
            cost: vec![Box::new(XBatch::local(2, 0)), Box::new(XBatch { touched: vec![], ..XBatch::local(0, 0) })],
        };
        (nets, reqs, layout(&[0, 3_000, 9_000]))
    }

    #[test]
    fn empty_eval_is_zero() {
        let (nets, reqs) = (Nets::from_macros(&[]), Requirements::<Layout>::default());
        let l = layout(&[]);
        let mut e = Eval::new(&reqs, &nets, 0);
        e.full(&reqs, &nets, &l, &gp::Prices::new());
        e.update(&[], false, &reqs, &nets, &l, &gp::Prices::new());
        assert_eq!((e.phi(), e.theta(), e.energy(&l)), ((0, 0.0), 0.0, 0.0));
    }

    #[test]
    fn full_matches_a_fresh_evaluation_bit_for_bit() {
        let (nets, reqs, l) = bench();
        let prices = gp::Prices::new();
        let mut e = Eval::new(&reqs, &nets, 3);
        e.full(&reqs, &nets, &l, &prices);
        assert_exact(&e, &nets, &reqs, &l, &prices);
    }

    #[test]
    fn update_tracks_every_move_and_revert_restores() {
        let (nets, reqs, mut l) = bench();
        let prices = gp::Prices::new();
        let mut e = Eval::new(&reqs, &nets, 3);
        e.full(&reqs, &nets, &l, &prices);
        let before = e.energy(&l);
        for (c, x) in [(0, 5_000), (1, -4_000), (2, 100)] {
            let old = l.x[c];
            l.x[c] = x;
            e.update(&[c], false, &reqs, &nets, &l, &prices);
            assert_exact(&e, &nets, &reqs, &l, &prices);
            e.revert();
            l.x[c] = old;
            assert_exact(&e, &nets, &reqs, &l, &prices);
            assert_eq!(e.energy(&l).to_bits(), before.to_bits());
        }
    }

    /// Two updates without a commit: one revert undoes both.
    #[test]
    fn revert_undoes_every_update_since_the_last_commit() {
        let (nets, reqs, mut l) = bench();
        let prices = gp::Prices::new();
        let mut e = Eval::new(&reqs, &nets, 3);
        e.full(&reqs, &nets, &l, &prices);
        let x0 = l.x.clone();
        l.x[0] = 2_000;
        e.update(&[0], false, &reqs, &nets, &l, &prices);
        l.x[0] = 4_000;
        l.x[1] = 8_000;
        e.update(&[0, 1, 0], false, &reqs, &nets, &l, &prices);
        assert_exact(&e, &nets, &reqs, &l, &prices);
        e.revert();
        l.x = x0;
        assert_exact(&e, &nets, &reqs, &l, &prices);
    }

    #[test]
    fn commit_keeps_the_new_values() {
        let (nets, reqs, mut l) = bench();
        let prices = gp::Prices::new();
        let mut e = Eval::new(&reqs, &nets, 3);
        e.full(&reqs, &nets, &l, &prices);
        l.x[2] = -6_000;
        e.update(&[2], false, &reqs, &nets, &l, &prices);
        e.commit();
        e.revert();
        assert_exact(&e, &nets, &reqs, &l, &prices);
    }

    /// A field reader touching only cell 1 still reads cell 0: it is global.
    #[test]
    fn a_field_reader_is_rescored_on_any_move() {
        let nets = Nets::from_macros(&[]);
        let reqs = Requirements::<Layout> {
            budget: vec![Box::new(XBatch { touched: vec![1], field: true, ..XBatch::local(0, 0) })],
            ..Default::default()
        };
        let prices = gp::Prices::new();
        let mut l = layout(&[0, 3_000]);
        let mut e = Eval::new(&reqs, &nets, 2);
        assert_eq!(e.global, vec![0]);
        e.full(&reqs, &nets, &l, &prices);
        l.x[0] = 2_500;
        e.update(&[0], false, &reqs, &nets, &l, &prices);
        assert_exact(&e, &nets, &reqs, &l, &prices);
    }

    /// A branch flip moves no cell: `all` rescores every batch.
    #[test]
    fn all_rescores_batches_no_moved_cell_reaches() {
        let nets = Nets::from_macros(&[]);
        let reqs = Requirements::<Layout> {
            hard: vec![Box::new(XBatch { branchy: true, ..XBatch::local(1, 5_000) })],
            ..Default::default()
        };
        let prices = gp::Prices::new();
        let mut l = layout(&[0, 0]);
        let mut e = Eval::new(&reqs, &nets, 2);
        e.full(&reqs, &nets, &l, &prices);
        assert_eq!(e.phi().0, 0);
        l.branch[0] = true;
        e.update(&[], true, &reqs, &nets, &l, &prices);
        assert_eq!(e.phi().0, 1);
        assert_exact(&e, &nets, &reqs, &l, &prices);
        e.revert();
        assert_eq!(e.phi().0, 0);
    }

    /// Touched ids are deduplicated and ids past `n` dropped.
    #[test]
    fn index_dedups_touched_and_drops_out_of_range_ids() {
        let reqs = Requirements::<Layout> {
            cost: vec![Box::new(XBatch { touched: vec![1, 1, 0, 9], ..XBatch::local(0, 0) })],
            ..Default::default()
        };
        let e = Eval::new(&reqs, &Nets::from_macros(&[]), 2);
        assert_eq!(e.cell_rows, vec![vec![0], vec![0]]);
        assert!(e.global.is_empty());
    }

    /// The dedup stamp wraps without leaving a stale mark that would skip a row.
    #[test]
    fn stamp_wraparound_still_rescores() {
        let (nets, reqs, mut l) = bench();
        let prices = gp::Prices::new();
        let mut e = Eval::new(&reqs, &nets, 3);
        e.full(&reqs, &nets, &l, &prices);
        e.stamp = u32::MAX - 1;
        for x in [1_000, 2_000, 3_000] {
            l.x[0] = x;
            e.update(&[0], false, &reqs, &nets, &l, &prices);
            e.commit();
            assert_exact(&e, &nets, &reqs, &l, &prices);
        }
    }

    /// Pins ±1.5e9 nm apart: the span overflows `i32`, the cached HPWL must not.
    #[test]
    fn net_hpwl_is_exact_past_the_i32_span() {
        let nets = Nets::from_macros(&[cell(&[0]), cell(&[0])]);
        let reqs = Requirements::<Layout>::default();
        let prices = gp::Prices::new();
        let l = layout(&[-1_500_000_000, 1_500_000_000]);
        let mut e = Eval::new(&reqs, &nets, 2);
        e.full(&reqs, &nets, &l, &prices);
        assert_eq!(e.net_hpwl, vec![3e9]);
        assert_exact(&e, &nets, &reqs, &l, &prices);
    }
}
