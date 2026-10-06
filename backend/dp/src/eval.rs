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

pub(crate) struct Eval {
    net_hpwl: Vec<f64>,
    /// Batches flattened: hard, then budget, then cost.
    row: Vec<Row>,
    n_hard: usize,
    n_budget: usize,
    cell_nets: Vec<Vec<u32>>,
    cell_rows: Vec<Vec<u32>>,
    global: Vec<u32>,
    seen_n: Vec<u32>,
    seen_r: Vec<u32>,
    stamp: u32,
    undo_n: Vec<(u32, f64)>,
    undo_r: Vec<(u32, Row)>,
}

impl Eval {
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

    fn net(nets: &Nets, l: &Layout, ni: usize) -> f64 {
        let (mut x0, mut x1, mut y0, mut y1) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
        for k in nets.span(ni) {
            let (px, py) = nets.pin(k, l);
            (x0, x1, y0, y1) = (x0.min(px), x1.max(px), y0.min(py), y1.max(py));
        }
        f64::from(nets.weight(ni)) * f64::from((x1 - x0) + (y1 - y0))
    }

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
    /// values for [`Eval::revert`].
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

    /// Restore the values the last [`Eval::update`] replaced.
    pub(crate) fn revert(&mut self) {
        while let Some((i, v)) = self.undo_n.pop() {
            self.net_hpwl[i as usize] = v;
        }
        while let Some((i, v)) = self.undo_r.pop() {
            self.row[i as usize] = v;
        }
    }

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
