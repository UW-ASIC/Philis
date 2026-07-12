//! Placement domain types, cost math, and slot implementations.
//!
//! No external deps — the downstream crate (pnr-placement) builds [`PlaceCold`]
//! from its frontend types and hands off to these slots.

use std::marker::PhantomData;

use crate::{
    slots::{
        AllLegal, AlwaysAccept, Geometric, Metropolis, OverflowStop, RampWeights, StepDecay,
        UnitWeights,
    },
    Control, Core, CostFn, Density, Domain, Ledger, Legality, SplitMix64, Stage, Stop, Telemetry,
};

pub const NONE: u32 = u32::MAX;

/// Common-centroid interdigitation pattern (mirrors `pnr_constraints::PatternType`
/// so the engine stays self-contained in the hot loop).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CcPattern {
    /// No pattern specified — centroid-only matching.
    #[default]
    None,
    /// ABBA interdigitation.
    Abba,
    /// ABAB interdigitation.
    Abab,
    /// 2-D common centroid.
    CommonCentroid2d,
}

// ---------------------------------------------------------------------------
// Domain
// ---------------------------------------------------------------------------

pub struct PlaceDomain;

impl Domain for PlaceDomain {
    type Hot = PlaceHot;
    type Cold = PlaceCold;
    type Mv = PlaceMv;
}

/// Cell orientation (matches substrate3::Orientation semantics).
/// Engine-local copy so the hot loop stays self-contained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum Orient {
    #[default]
    N,
    /// 180° rotation.
    S,
    /// Mirror across vertical axis (flip left-right, a.k.a. FN / MX).
    FN,
    /// Mirror across horizontal axis (flip top-bottom, a.k.a. FS / MY).
    FS,
}

impl Orient {
    /// Transform a pin offset (dx, dy) relative to cell center, given cell
    /// half-extents (hw, hh). Returns transformed (dx, dy).
    #[inline]
    pub fn transform_pin(self, dx: f32, dy: f32) -> (f32, f32) {
        match self {
            Self::N => (dx, dy),
            Self::S => (-dx, -dy),
            Self::FN => (-dx, dy),
            Self::FS => (dx, -dy),
        }
    }
}

/// Hot SoA state: cell centers (nm), per-symmetry-group mirror axis, and the
/// spatial bucket index that makes overlap queries O(local) instead of O(n).
#[derive(Debug, Clone)]
pub struct PlaceHot {
    pub x: Vec<f32>,
    pub y: Vec<f32>,
    /// Vertical mirror-axis x per symmetry group (nm).
    pub axis: Vec<f32>,
    pub buckets: Buckets,
    /// Per-cell orientation (N for most; FN for mirror partners).
    pub orient: Vec<Orient>,
    /// Per-cell current variant index (indexes into `PlaceCold::variants`).
    pub variant_idx: Vec<u16>,
    /// 2.1: current half-extents (mutated on reshape; initialized from cold.hw/hh).
    /// When non-empty, takes precedence over cold.hw/cold.hh for gap/overlap.
    pub cur_hw: Vec<f32>,
    pub cur_hh: Vec<f32>,
}

/// Uniform-grid spatial index. Bucket side = the largest cell dimension, so
/// every potential overlap partner of a cell lives in the 3x3 neighborhood of
/// its bucket. Cores keep it current on commit.
#[derive(Debug, Clone, Default)]
pub struct Buckets {
    size: f32,
    nx: u32,
    ny: u32,
    bins: Vec<Vec<u32>>,
}

impl Buckets {
    pub fn build(cold: &PlaceCold, xs: &[f32], ys: &[f32]) -> Self {
        let base_max = cold
            .hw
            .iter()
            .zip(&cold.hh)
            .map(|(w, h)| (2.0 * w).max(2.0 * h))
            .fold(1.0f32, f32::max);
        // Reshape moves can make a cell much larger than its initial variant.
        // The 3x3-neighborhood guarantee only holds when the bucket side uses
        // the largest possible footprint, not merely the starting footprint.
        let variant_max = cold
            .variants
            .iter()
            .flatten()
            .map(|&(hw, hh)| (2.0 * hw).max(2.0 * hh))
            .fold(1.0f32, f32::max);
        let max_dim = base_max.max(variant_max);
        let size = max_dim.max(cold.die.0.max(cold.die.1) / 64.0);
        let nx = ((cold.die.0 / size).ceil() as u32).max(1);
        let ny = ((cold.die.1 / size).ceil() as u32).max(1);
        let mut b = Self { size, nx, ny, bins: vec![Vec::new(); (nx * ny) as usize] };
        for (c, (&x, &y)) in xs.iter().zip(ys).enumerate() {
            let i = b.idx(x, y);
            b.bins[i].push(c as u32);
        }
        b
    }

    fn idx(&self, x: f32, y: f32) -> usize {
        let bx = ((x / self.size) as u32).min(self.nx - 1);
        let by = ((y / self.size) as u32).min(self.ny - 1);
        (by * self.nx + bx) as usize
    }

    pub fn rebuild(&mut self, xs: &[f32], ys: &[f32]) {
        self.bins.iter_mut().for_each(Vec::clear);
        for (c, (&x, &y)) in xs.iter().zip(ys).enumerate() {
            let i = self.idx(x, y);
            self.bins[i].push(c as u32);
        }
    }

    pub fn move_cell(&mut self, c: u32, old: (f32, f32), new: (f32, f32)) {
        let (oi, ni) = (self.idx(old.0, old.1), self.idx(new.0, new.1));
        if oi == ni {
            return;
        }
        if let Some(k) = self.bins[oi].iter().position(|&m| m == c) {
            self.bins[oi].swap_remove(k);
        }
        self.bins[ni].push(c);
    }

    /// Cells in the 3x3 bucket neighborhood of (x, y).
    pub fn near(&self, x: f32, y: f32, out: &mut Vec<u32>) {
        out.clear();
        let bx = ((x / self.size) as i64).clamp(0, i64::from(self.nx) - 1);
        let by = ((y / self.size) as i64).clamp(0, i64::from(self.ny) - 1);
        for dy in -1..=1i64 {
            for dx in -1..=1i64 {
                let (tx, ty) = (bx + dx, by + dy);
                if tx >= 0 && ty >= 0 && tx < i64::from(self.nx) && ty < i64::from(self.ny) {
                    out.extend_from_slice(&self.bins[(ty * i64::from(self.nx) + tx) as usize]);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Move
// ---------------------------------------------------------------------------

/// A proposed move: a handful of cells (SA) or all of them (analytical step),
/// with an optional axis update and the delta the proposer already computed.
#[derive(Debug, Clone, Default)]
pub struct PlaceMv {
    pub idx: Vec<u32>,
    pub nx: Vec<f32>,
    pub ny: Vec<f32>,
    /// (group, new axis x) when the move shifts a whole symmetry island.
    pub axis: Option<(u32, f32)>,
    /// Full-cost delta cached by an analytical core; SA leaves `None` and the
    /// CostFn computes it incrementally.
    pub cached_delta: Option<f64>,
    /// 2.1: Reshape: (cell, new_hw, new_hh, new_variant_idx, old_hw, old_hh, old_variant_idx).
    /// Applied atomically on commit; reverted on reject.
    pub reshape: Vec<(u32, f32, f32, u16, f32, f32, u16)>,
    /// 2.4: Orientation changes: (cell, new_orient, old_orient).
    pub orient_changes: Vec<(u32, Orient, Orient)>,
}

impl PlaceMv {
    /// Reset for refill — keeps buffer capacity (the driver reuses one move
    /// for the whole stage).
    pub fn clear(&mut self) {
        self.idx.clear();
        self.nx.clear();
        self.ny.clear();
        self.axis = None;
        self.cached_delta = None;
        self.reshape.clear();
        self.orient_changes.clear();
    }

    pub fn new_pos(&self, cell: u32, hot: &PlaceHot) -> (f32, f32) {
        match self.idx.iter().position(|&i| i == cell) {
            Some(k) => (self.nx[k], self.ny[k]),
            None => (hot.x[cell as usize], hot.y[cell as usize]),
        }
    }

    /// Proposed half-width for cell during this move (accounts for reshape).
    #[inline]
    pub fn prop_hw(&self, cell: usize, hot: &PlaceHot, cold: &PlaceCold) -> f32 {
        for &(c, nhw, _, _, _, _, _) in &self.reshape {
            if c as usize == cell { return nhw; }
        }
        eff_hw(hot, cold, cell)
    }

    /// Proposed half-height for cell during this move (accounts for reshape).
    #[inline]
    pub fn prop_hh(&self, cell: usize, hot: &PlaceHot, cold: &PlaceCold) -> f32 {
        for &(c, _, nhh, _, _, _, _) in &self.reshape {
            if c as usize == cell { return nhh; }
        }
        eff_hh(hot, cold, cell)
    }
}

// ---------------------------------------------------------------------------
// Cold context
// ---------------------------------------------------------------------------

/// CSR: item -> list of u32.
#[derive(Debug, Clone, Default)]
pub struct Csr {
    pub start: Vec<u32>,
    pub items: Vec<u32>,
}

impl Csr {
    pub fn row(&self, i: u32) -> &[u32] {
        &self.items[self.start[i as usize] as usize..self.start[i as usize + 1] as usize]
    }
}

/// Build a cell → item-index CSR from (cell, item) incidence pairs.
/// Pairs must arrive in ascending item order (natural when emitted from an
/// indexed loop) so each row stays sorted — merged rows across moved cells
/// then dedup back to exactly the flat-list evaluation order, keeping f64
/// accumulation bitwise identical to a full scan.
fn incidence_csr(n_cells: usize, inc: &[(u32, u32)]) -> Csr {
    let mut start = vec![0u32; n_cells + 1];
    for &(c, _) in inc {
        start[c as usize + 1] += 1;
    }
    for i in 0..n_cells {
        start[i + 1] += start[i];
    }
    let mut items = vec![0u32; inc.len()];
    let mut cursor = start.clone();
    for &(c, k) in inc {
        items[cursor[c as usize] as usize] = k;
        cursor[c as usize] += 1;
    }
    Csr { start, items }
}

/// Incidence pairs for a list of (a, b) pair-rules: rule k touches cells a and b.
fn pair_incidence(pairs: impl Iterator<Item = (u32, u32)>) -> Vec<(u32, u32)> {
    let mut inc = Vec::new();
    for (k, (a, b)) in pairs.enumerate() {
        inc.push((a, k as u32));
        if b != a {
            inc.push((b, k as u32));
        }
    }
    inc
}

/// Merge the CSR rows of all moved cells into `out`: sorted, deduped item ids —
/// the exact subset of the flat list that `touches()` would have matched.
fn merge_rows(csr: &Csr, idx: &[u32], out: &mut Vec<u32>) {
    out.clear();
    // Single-cell moves are the common case: one row is already sorted-unique
    // (incidence built in ascending item order, deduped) — skip the sort.
    if let [c] = idx {
        out.extend_from_slice(csr.row(*c));
        return;
    }
    for &c in idx {
        out.extend_from_slice(csr.row(c));
    }
    out.sort_unstable();
    out.dedup();
}

#[derive(Debug, Clone, Default)]
pub struct SymTable {
    /// Per cell: symmetry group or NONE.
    pub group_of: Vec<u32>,
    /// Per cell: mirror partner; self for self-symmetric; NONE if ungrouped.
    pub partner: Vec<u32>,
    /// Group -> member cells.
    pub groups: Vec<Vec<u32>>,
    /// Group axis fixed by the constraint (angstrom-sourced) vs free.
    pub fixed: Vec<bool>,
    /// Initial axis x per group, nm (constraint value or die center).
    pub axis0: Vec<f32>,
}

/// Soft pull-together / hard keep-apart pair, nm.
#[derive(Debug, Clone, Copy)]
pub struct PairRule {
    pub a: u32,
    pub b: u32,
    pub gap_nm: f32,
    pub weight: f32,
}

/// A deck-qualified direct-connect transform. `dx`/`dy` are the required
/// center displacement `b - a`. Only the listed variant pair in normal
/// orientation may waive rectangular overlap; all arbitrary overlap remains
/// penalized normally.
#[derive(Debug, Clone, Copy)]
pub struct Abutment {
    pub a: u32,
    pub b: u32,
    pub variant_a: u16,
    pub variant_b: u16,
    pub dx: f32,
    pub dy: f32,
}

#[derive(Debug, Clone, Default)]
pub struct PlaceCold {
    /// Half-extents per cell, nm.
    pub hw: Vec<f32>,
    pub hh: Vec<f32>,
    /// Net -> unique member cells (nets with <2 cells dropped).
    pub nets: Csr,
    pub net_w: Vec<f32>,
    /// Cell -> nets (indices into `nets` rows).
    pub cell_nets: Csr,
    pub sym: SymTable,
    /// Soft attraction: proximity(gap=0), thermal, cc handled separately.
    pub pulls: Vec<PairRule>,
    /// Hard min edge-to-edge gap (isolation).
    pub pushes: Vec<PairRule>,
    /// Common-centroid groups, flat CSR: group i side A = row 2i, side B =
    /// row 2i+1. Contiguous — no per-group Vec scatter loads in the SA loop.
    pub cc: Csr,
    /// Per-CC-group pattern type (parallel to `cc`).
    pub cc_pattern: Vec<CcPattern>,
    /// StraightNet alignment: (cells, vertical) — align x if vertical.
    pub aligns: Vec<(Vec<u32>, bool)>,
    pub die: (f32, f32),
    pub grid: f32,
    /// Total raw edge clearance represented by the inflated half-extents.
    /// Public constraint checks add this back so their distances are measured
    /// against drawn cell bboxes rather than the virtual placement footprint.
    pub cell_margin: f32,
    /// Per-cell layer bitmask (bit = PDK LayerId present in cell geometry).
    /// Cells overlap only when masks intersect. Empty → all cells conflict.
    pub layer_mask: Vec<u64>,
    /// Per-device stress constraint: (device_idx, max_centroid_distance_nm).
    /// Cost term pulls device toward die center.
    pub stress: Vec<(u32, f32)>,
    /// DTI forbidden zones: (device_a, device_b, min_gap_nm, max_gap_nm).
    /// Devices must be abutting (gap < min) or far apart (gap > max).
    pub dti_zones: Vec<(u32, u32, f32, f32)>,

    // -- 3.1 + 3.2: pin offsets for orientation-aware / variant-aware HPWL --

    /// Per-cell pin offsets from cell center, per net.
    /// `pin_cell[k]` = cell index, `pin_net[k]` = net index,
    /// `pin_dx[k]`/`pin_dy[k]` = offset from cell center (variant 0 / orient N).
    /// Empty when no pin geometry is available (falls back to cell-center HPWL).
    pub pin_cell: Vec<u32>,
    pub pin_net: Vec<u32>,
    pub pin_dx: Vec<f32>,
    pub pin_dy: Vec<f32>,

    /// Per-cell variants: `variants[cell]` = list of (hw, hh) alternatives.
    /// Empty slice = no variants (use hw/hh as-is).
    pub variants: Vec<Vec<(f32, f32)>>,
    /// Per-variant pin offsets: `variant_pins[cell][var]` = Vec of (net, dx, dy).
    /// Parallel to `variants`. Empty = same pins for all variants.
    pub variant_pins: Vec<Vec<Vec<(u32, f32, f32)>>>,
    /// Initial variant selected by the cross-iteration cell optimizer.
    pub initial_variant: Vec<u16>,
    /// Feedback loss per cell/variant, in placement-cost units.
    pub variant_penalty: Vec<Vec<f64>>,
    /// DRC-qualified same-net abutments that can legally overlap footprints.
    pub abutments: Vec<Abutment>,

    // -- 3.3: shared spacing model --

    /// Device class index per cell (0-based, derived from DeviceType).
    /// Used for class-pair spacing lookups.
    pub class_idx: Vec<u8>,
    /// Dense class-pair spacing table: `class_spacing[a * n_classes + b]` = min gap nm.
    /// n_classes = `class_count`. Zero = no special spacing required.
    pub class_spacing: Vec<f32>,
    pub class_count: u8,
    /// Sparse per-pair min-distance overrides (from isolation constraints).
    /// Takes precedence over class_spacing when present.
    /// Invariant: entries normalized (a ≤ b) and sorted by (a, b) —
    /// call [`Self::normalize_pair_min_dist`] after populating.
    pub pair_min_dist: Vec<(u32, u32, f32)>,
}

impl PlaceCold {
    /// Number of common-centroid groups.
    #[inline]
    pub fn cc_count(&self) -> usize {
        self.cc.start.len().saturating_sub(1) / 2
    }

    /// Side A and side B cell slices of common-centroid group `i`.
    #[inline]
    pub fn cc_group(&self, i: usize) -> (&[u32], &[u32]) {
        (self.cc.row(2 * i as u32), self.cc.row(2 * i as u32 + 1))
    }

    /// Normalize + sort `pair_min_dist` so [`required_gap`] can binary-search.
    /// Duplicate pairs keep the largest gap (most conservative).
    pub fn normalize_pair_min_dist(&mut self) {
        for e in &mut self.pair_min_dist {
            if e.0 > e.1 {
                std::mem::swap(&mut e.0, &mut e.1);
            }
        }
        self.pair_min_dist
            .sort_by(|x, y| (x.0, x.1).cmp(&(y.0, y.1)).then(y.2.total_cmp(&x.2)));
        self.pair_min_dist.dedup_by_key(|e| (e.0, e.1));
    }
}

#[inline]
fn layers_conflict(cold: &PlaceCold, a: usize, b: usize) -> bool {
    cold.layer_mask.is_empty()
        || (cold.layer_mask[a] & cold.layer_mask[b]) != 0
}

/// Get effective half-width for cell i, preferring hot (reshape) over cold.
#[inline]
pub fn eff_hw(hot: &PlaceHot, cold: &PlaceCold, i: usize) -> f32 {
    if !hot.cur_hw.is_empty() { hot.cur_hw[i] } else { cold.hw[i] }
}
/// Get effective half-height for cell i, preferring hot (reshape) over cold.
#[inline]
pub fn eff_hh(hot: &PlaceHot, cold: &PlaceCold, i: usize) -> f32 {
    if !hot.cur_hh.is_empty() { hot.cur_hh[i] } else { cold.hh[i] }
}

/// Required min edge-to-edge gap between cells a and b (nm).
/// Checks sparse pair overrides first, then class-pair table.
/// `pair_min_dist` must be normalized (a ≤ b) and sorted — see
/// [`PlaceCold::normalize_pair_min_dist`]; lookup is a binary search.
#[inline]
pub fn required_gap(cold: &PlaceCold, a: u32, b: u32) -> f32 {
    // Sparse pair overrides (isolation constraints)
    let key = (a.min(b), a.max(b));
    if let Ok(i) = cold
        .pair_min_dist
        .binary_search_by_key(&key, |&(pa, pb, _)| (pa, pb))
    {
        return cold.pair_min_dist[i].2;
    }
    // Dense class-pair table
    if cold.class_count > 0 && !cold.class_idx.is_empty() {
        let ca = cold.class_idx[a as usize] as usize;
        let cb = cold.class_idx[b as usize] as usize;
        let nc = cold.class_count as usize;
        if ca < nc && cb < nc {
            return cold.class_spacing[ca * nc + cb];
        }
    }
    0.0
}

/// Resolved pin position for cell `c` on net `ni`, accounting for orientation
/// and variant. Falls back to cell center when no pin data exists.
#[inline]
fn pin_offset(cold: &PlaceCold, c: u32, ni: u32, vi: usize) -> Option<(f32, f32)> {
    let ci = c as usize;
    if ci < cold.variant_pins.len() && vi < cold.variant_pins[ci].len() {
        for &(net, dx, dy) in &cold.variant_pins[ci][vi] {
            if net == ni {
                return Some((dx, dy));
            }
        }
    }
    for k in 0..cold.pin_cell.len() {
        if cold.pin_cell[k] == c && cold.pin_net[k] == ni {
            return Some((cold.pin_dx[k], cold.pin_dy[k]));
        }
    }
    None
}

#[inline]
fn pin_pos(cold: &PlaceCold, hot: &PlaceHot, c: u32, ni: u32) -> (f32, f32) {
    let ci = c as usize;
    let vi = hot.variant_idx.get(ci).copied().unwrap_or(0) as usize;
    let orient = hot.orient.get(ci).copied().unwrap_or(Orient::N);
    if let Some((dx, dy)) = pin_offset(cold, c, ni, vi) {
        let (dx, dy) = orient.transform_pin(dx, dy);
        (hot.x[ci] + dx, hot.y[ci] + dy)
    } else {
        (hot.x[ci], hot.y[ci])
    }
}

fn moved_variant(hot: &PlaceHot, mv: &PlaceMv, c: u32) -> u16 {
    mv.reshape
        .iter()
        .find_map(|&(d, _, _, vi, _, _, _)| (d == c).then_some(vi))
        .unwrap_or_else(|| hot.variant_idx.get(c as usize).copied().unwrap_or(0))
}

fn moved_orient(hot: &PlaceHot, mv: &PlaceMv, c: u32) -> Orient {
    mv.orient_changes
        .iter()
        .find_map(|&(d, new, _)| (d == c).then_some(new))
        .unwrap_or_else(|| hot.orient.get(c as usize).copied().unwrap_or(Orient::N))
}

fn moved_pin_pos(cold: &PlaceCold, hot: &PlaceHot, mv: &PlaceMv, c: u32, ni: u32) -> (f32, f32) {
    let (x, y) = mv.new_pos(c, hot);
    if let Some((dx, dy)) = pin_offset(cold, c, ni, moved_variant(hot, mv, c) as usize) {
        let (dx, dy) = moved_orient(hot, mv, c).transform_pin(dx, dy);
        (x + dx, y + dy)
    } else {
        (x, y)
    }
}

fn has_explicit_separation(cold: &PlaceCold, a: u32, b: u32) -> bool {
    let key = (a.min(b), a.max(b));
    cold.pair_min_dist.binary_search_by_key(&key, |&(x, y, _)| (x, y)).is_ok()
        || cold.pushes.iter().any(|p| (p.a == a && p.b == b) || (p.a == b && p.b == a))
}

fn legal_abutment_current(cold: &PlaceCold, hot: &PlaceHot, a: usize, b: usize) -> bool {
    let (a, b) = if a <= b { (a as u32, b as u32) } else { (b as u32, a as u32) };
    if has_explicit_separation(cold, a, b) {
        return false;
    }
    cold.abutments.iter().any(|r| {
        r.a == a
            && r.b == b
            && hot.variant_idx.get(a as usize).copied().unwrap_or(0) == r.variant_a
            && hot.variant_idx.get(b as usize).copied().unwrap_or(0) == r.variant_b
            && hot.orient.get(a as usize).copied().unwrap_or(Orient::N) == Orient::N
            && hot.orient.get(b as usize).copied().unwrap_or(Orient::N) == Orient::N
            && (hot.x[b as usize] - hot.x[a as usize] - r.dx).abs() <= cold.grid * 2.0
            && (hot.y[b as usize] - hot.y[a as usize] - r.dy).abs() <= cold.grid * 2.0
    })
}

fn legal_abutment_proposed(cold: &PlaceCold, hot: &PlaceHot, mv: &PlaceMv, a: usize, b: usize) -> bool {
    let (a, b) = if a <= b { (a as u32, b as u32) } else { (b as u32, a as u32) };
    if has_explicit_separation(cold, a, b) {
        return false;
    }
    let (ax, ay) = mv.new_pos(a, hot);
    let (bx, by) = mv.new_pos(b, hot);
    cold.abutments.iter().any(|r| {
        r.a == a
            && r.b == b
            && moved_variant(hot, mv, a) == r.variant_a
            && moved_variant(hot, mv, b) == r.variant_b
            && moved_orient(hot, mv, a) == Orient::N
            && moved_orient(hot, mv, b) == Orient::N
            && (bx - ax - r.dx).abs() <= cold.grid * 2.0
            && (by - ay - r.dy).abs() <= cold.grid * 2.0
    })
}

// ---------------------------------------------------------------------------
// Cost math
// ---------------------------------------------------------------------------

pub fn net_bbox(cells: &[u32], xs: &[f32], ys: &[f32]) -> (f32, f32, f32, f32) {
    let (mut x0, mut x1, mut y0, mut y1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for &c in cells {
        let (x, y) = (xs[c as usize], ys[c as usize]);
        x0 = x0.min(x);
        x1 = x1.max(x);
        y0 = y0.min(y);
        y1 = y1.max(y);
    }
    (x0, x1, y0, y1)
}

pub fn hpwl(cold: &PlaceCold, xs: &[f32], ys: &[f32]) -> f64 {
    let mut total = 0.0f64;
    for n in 0..cold.net_w.len() {
        let cells = cold.nets.row(n as u32);
        let (x0, x1, y0, y1) = net_bbox(cells, xs, ys);
        total += f64::from(cold.net_w[n]) * f64::from((x1 - x0) + (y1 - y0));
    }
    total
}

/// Pin-aware HPWL: uses pin offsets + orientation when available.
pub fn hpwl_pins(cold: &PlaceCold, hot: &PlaceHot) -> f64 {
    if cold.pin_cell.is_empty() {
        return hpwl(cold, &hot.x, &hot.y);
    }
    let mut total = 0.0f64;
    for n in 0..cold.net_w.len() {
        let ni = n as u32;
        let cells = cold.nets.row(ni);
        let (mut x0, mut x1, mut y0, mut y1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
        for &c in cells {
            let (px, py) = pin_pos(cold, hot, c, ni);
            x0 = x0.min(px);
            x1 = x1.max(px);
            y0 = y0.min(py);
            y1 = y1.max(py);
        }
        total += f64::from(cold.net_w[n]) * f64::from((x1 - x0) + (y1 - y0));
    }
    total
}

/// Soft analog terms: pulls (quadratic beyond gap), CC centroid error,
/// straight-net alignment. All in nm-comparable units.
pub fn soft_terms(cold: &PlaceCold, xs: &[f32], ys: &[f32]) -> f64 {
    let mut t = 0.0f64;
    for p in &cold.pulls {
        let dx = xs[p.a as usize] - xs[p.b as usize];
        let dy = ys[p.a as usize] - ys[p.b as usize];
        let d = (dx * dx + dy * dy).sqrt();
        let ex = (d - p.gap_nm).max(0.0);
        t += f64::from(p.weight) * f64::from(ex * ex) * 1e-3;
    }
    for ci in 0..cold.cc_count() {
        let (ga, gb) = cold.cc_group(ci);
        let (cax, cay) = centroid(ga, xs, ys);
        let (cbx, cby) = centroid(gb, xs, ys);
        let (dx, dy) = (cax - cbx, cay - cby);
        t += f64::from(dx * dx + dy * dy) * 1e-3;
        // Pattern-aware per-device alignment pulls
        let pat = cold.cc_pattern.get(ci).copied().unwrap_or(CcPattern::None);
        t += cc_pattern_cost(pat, ga, gb, |c| (xs[c as usize], ys[c as usize]));
    }
    for (cells, vertical) in &cold.aligns {
        let coords: &[f32] = if *vertical { xs } else { ys };
        let mean = cells.iter().map(|&c| coords[c as usize]).sum::<f32>() / cells.len() as f32;
        for &c in cells {
            t += f64::from((coords[c as usize] - mean).abs());
        }
    }
    // stress: penalize distance from die center beyond max_centroid_distance
    let (cx, cy) = (cold.die.0 / 2.0, cold.die.1 / 2.0);
    for &(dev, max_d) in &cold.stress {
        let dx = xs[dev as usize] - cx;
        let dy = ys[dev as usize] - cy;
        let d = (dx * dx + dy * dy).sqrt();
        let ex = (d - max_d).max(0.0);
        t += f64::from(ex * ex) * 1e-3;
    }
    // 3.3: spacing-model soft term — penalize violation of required gap (class-pair + per-pair)
    // ponytail: only evaluates pair_min_dist pairs + push pairs; class-pair handled in HardGaps
    for &(a, b, min_d) in &cold.pair_min_dist {
        let (ai, bi) = (a as usize, b as usize);
        if ai < xs.len() && bi < xs.len() {
            let gx = (xs[ai] - xs[bi]).abs() - (cold.hw[ai] + cold.hw[bi]);
            let gy = (ys[ai] - ys[bi]).abs() - (cold.hh[ai] + cold.hh[bi]);
            let g = gx.max(gy);
            let violation = (min_d - g).max(0.0);
            t += f64::from(violation * violation) * 4e-3;
        }
    }
    t
}

pub fn cost_at(cold: &PlaceCold, xs: &[f32], ys: &[f32]) -> f64 {
    hpwl(cold, xs, ys) + soft_terms(cold, xs, ys)
}

/// Cost with pin-aware HPWL (uses hot.orient for pin transforms).
pub fn cost_at_pins(cold: &PlaceCold, hot: &PlaceHot) -> f64 {
    hpwl_pins(cold, hot) + soft_terms(cold, &hot.x, &hot.y)
}

pub fn centroid(cells: &[u32], xs: &[f32], ys: &[f32]) -> (f32, f32) {
    let n = cells.len().max(1) as f32;
    let sx: f32 = cells.iter().map(|&c| xs[c as usize]).sum();
    let sy: f32 = cells.iter().map(|&c| ys[c as usize]).sum();
    (sx / n, sy / n)
}

/// Raw drawn-bbox edge-to-edge gap between two cells (negative = overlapping).
pub fn gap(cold: &PlaceCold, a: u32, b: u32, xs: &[f32], ys: &[f32]) -> f32 {
    let (ai, bi) = (a as usize, b as usize);
    if !layers_conflict(cold, ai, bi) { return f32::MAX; }
    let gx = (xs[ai] - xs[bi]).abs() - (cold.hw[ai] + cold.hw[bi]);
    let gy = (ys[ai] - ys[bi]).abs() - (cold.hh[ai] + cold.hh[bi]);
    gx.max(gy) + cold.cell_margin
}

/// Rectangle overlap area of cells a and b at the given coords.
pub fn overlap_area(cold: &PlaceCold, a: usize, b: usize, xs: &[f32], ys: &[f32]) -> f64 {
    if !layers_conflict(cold, a, b) { return 0.0; }
    let ox = (cold.hw[a] + cold.hw[b]) - (xs[a] - xs[b]).abs();
    let oy = (cold.hh[a] + cold.hh[b]) - (ys[a] - ys[b]).abs();
    if ox > 0.0 && oy > 0.0 {
        f64::from(ox) * f64::from(oy)
    } else {
        0.0
    }
}

/// Overlap area using effective (reshape-aware) sizes.
fn overlap_area_eff(hot: &PlaceHot, cold: &PlaceCold, a: usize, b: usize, xs: &[f32], ys: &[f32]) -> f64 {
    if !layers_conflict(cold, a, b) { return 0.0; }
    if legal_abutment_current(cold, hot, a, b) { return 0.0; }
    let ox = (eff_hw(hot, cold, a) + eff_hw(hot, cold, b)) - (xs[a] - xs[b]).abs();
    let oy = (eff_hh(hot, cold, a) + eff_hh(hot, cold, b)) - (ys[a] - ys[b]).abs();
    if ox > 0.0 && oy > 0.0 { f64::from(ox) * f64::from(oy) } else { 0.0 }
}

/// Total pairwise overlap area with effective (reshape-aware) sizes — the
/// truthful post-placement check; `total_overlap` with cold sizes reports
/// phantom overlaps for reshaped cells.
pub fn total_overlap_eff(hot: &PlaceHot, cold: &PlaceCold) -> f64 {
    let n = cold.hw.len();
    let mut t = 0.0;
    for a in 0..n {
        for b in a + 1..n {
            t += overlap_area_eff(hot, cold, a, b, &hot.x, &hot.y);
        }
    }
    t
}

/// Bounding box over the current margin-inflated cell footprints.
pub fn placement_bbox(hot: &PlaceHot, cold: &PlaceCold) -> (f32, f32, f32, f32) {
    let mut bbox = (
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    );
    for i in 0..cold.hw.len() {
        bbox.0 = bbox.0.min(hot.x[i] - eff_hw(hot, cold, i));
        bbox.1 = bbox.1.min(hot.y[i] - eff_hh(hot, cold, i));
        bbox.2 = bbox.2.max(hot.x[i] + eff_hw(hot, cold, i));
        bbox.3 = bbox.3.max(hot.y[i] + eff_hh(hot, cold, i));
    }
    bbox
}

/// Total pairwise overlap area — exact O(n²) reference, used once per flow for
/// the final report.
pub fn total_overlap(cold: &PlaceCold, xs: &[f32], ys: &[f32]) -> f64 {
    let n = cold.hw.len();
    let mut t = 0.0;
    for a in 0..n {
        for b in a + 1..n {
            t += overlap_area(cold, a, b, xs, ys);
        }
    }
    t
}

// ---------------------------------------------------------------------------
// Constraint checks (pure evaluation; the ledger in pnr-placement maps these
// to ConstraintContract updates)
// ---------------------------------------------------------------------------

pub enum PlaceCheck {
    /// |xa + xb − 2·axis| ≤ tol ∧ |ya − yb| ≤ tol
    SymPair { a: u32, b: u32, g: u32 },
    /// centroid(A) ≈ centroid(B)
    Cc { i: usize },
    /// edge gap ≥ min (hard isolation)
    MinGap { a: u32, b: u32, min_nm: f32 },
    /// soft keep-within (proximity budget; 0 ⇒ "adjacent", tol = 4·max span)
    MaxDist { a: u32, b: u32, max_nm: f32 },
    /// device within max distance of die center (stress)
    CentroidDist { d: u32, max_nm: f32 },
    /// DTI: gap < min (abutting, shared trench) or > max (fully separate)
    OutsideBand { a: u32, b: u32, min_nm: f32, max_nm: f32 },
}

/// Evaluate a single constraint check. Returns (satisfied, violation_metric).
pub fn eval_check(ch: &PlaceCheck, hot: &PlaceHot, cold: &PlaceCold) -> (bool, f32) {
    let tol = cold.grid * 2.0;
    match *ch {
        PlaceCheck::SymPair { a, b, g } => {
            let ex =
                (hot.x[a as usize] + hot.x[b as usize] - 2.0 * hot.axis[g as usize]).abs();
            let ey = (hot.y[a as usize] - hot.y[b as usize]).abs();
            (ex <= tol && ey <= tol, ex.max(ey))
        }
        PlaceCheck::Cc { i } => {
            let (ga, gb) = cold.cc_group(i);
            let (ax, ay) = centroid(ga, &hot.x, &hot.y);
            let (bx, by) = centroid(gb, &hot.x, &hot.y);
            let e = ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt();
            (e <= tol * 4.0, e)
        }
        PlaceCheck::MinGap { a, b, min_nm } => {
            let gp = gap(cold, a, b, &hot.x, &hot.y);
            (gp >= min_nm, min_nm - gp)
        }
        PlaceCheck::MaxDist { a, b, max_nm } => {
            let gp = gap(cold, a, b, &hot.x, &hot.y);
            let span = 2.0
                * (cold.hw[a as usize] + cold.hw[b as usize])
                    .max(cold.hh[a as usize] + cold.hh[b as usize]);
            let lim = if max_nm > 0.0 { max_nm } else { span };
            (gp <= lim, gp - lim)
        }
        PlaceCheck::CentroidDist { d, max_nm } => {
            let (cx, cy) = (cold.die.0 / 2.0, cold.die.1 / 2.0);
            let dx = hot.x[d as usize] - cx;
            let dy = hot.y[d as usize] - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            (dist <= max_nm + tol, dist - max_nm)
        }
        PlaceCheck::OutsideBand { a, b, min_nm, max_nm } => {
            let gp = gap(cold, a, b, &hot.x, &hot.y);
            let ok = gp < min_nm || gp > max_nm;
            // metric: penetration depth into the forbidden band
            (ok, (gp - min_nm).min(max_nm - gp).max(0.0))
        }
    }
}

// ---------------------------------------------------------------------------
// Global stage — analytical descent
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct GlobalCfg {
    pub max_iters: u32,
    pub overflow_target: f32,
    pub step0: f32,
    pub momentum: f32,
    pub lambda0: f32,
    pub sym_weight: f32,
    pub target_util: f32,
}

impl Default for GlobalCfg {
    fn default() -> Self {
        Self {
            max_iters: 500,
            overflow_target: 0.15,
            step0: 0.04,
            momentum: 0.85,
            lambda0: 0.5,
            sym_weight: 4.0,
            target_util: 0.7,
        }
    }
}

pub struct Bins {
    pub nb: usize,
    pub bw: f32,
    pub bh: f32,
    pub util: Vec<f32>,
}

impl Bins {
    pub fn new(cold: &PlaceCold) -> Self {
        let n = cold.hw.len();
        let nb = ((n as f32).sqrt().ceil() as usize).clamp(4, 24);
        Bins {
            nb,
            bw: cold.die.0 / nb as f32,
            bh: cold.die.1 / nb as f32,
            util: vec![0.0; nb * nb],
        }
    }

    pub fn fill(&mut self, cold: &PlaceCold, xs: &[f32], ys: &[f32]) {
        self.util.iter_mut().for_each(|u| *u = 0.0);
        // ponytail: whole cell area lands in its center bin
        for i in 0..cold.hw.len() {
            let b = self.bin_of(xs[i], ys[i]);
            self.util[b] += 4.0 * cold.hw[i] * cold.hh[i] / (self.bw * self.bh);
        }
    }

    pub fn bin_of(&self, x: f32, y: f32) -> usize {
        let bx = ((x / self.bw) as usize).min(self.nb - 1);
        let by = ((y / self.bh) as usize).min(self.nb - 1);
        by * self.nb + bx
    }

    pub fn overflow(&self, cold: &PlaceCold, target_util: f32) -> f32 {
        let total: f32 = cold.hw.iter().zip(&cold.hh).map(|(w, h)| 4.0 * w * h).sum();
        if total <= 0.0 {
            return 0.0;
        }
        let over: f32 = self
            .util
            .iter()
            .map(|&u| (u - target_util).max(0.0) * self.bw * self.bh)
            .sum();
        over / total
    }
}

pub struct GlobalCost;
impl CostFn<PlaceDomain> for GlobalCost {
    fn eval(&self, hot: &PlaceHot, cold: &PlaceCold) -> f64 {
        cost_at(cold, &hot.x, &hot.y)
    }
    fn delta(&self, _: &PlaceHot, _: &PlaceCold, mv: &PlaceMv) -> f64 {
        mv.cached_delta.unwrap_or(0.0)
    }
}

pub struct BinDensity {
    pub target_util: f32,
}
impl Density<PlaceDomain> for BinDensity {
    fn overflow(&self, hot: &PlaceHot, cold: &PlaceCold) -> f32 {
        let mut bins = Bins::new(cold);
        bins.fill(cold, &hot.x, &hot.y);
        bins.overflow(cold, self.target_util)
    }
}

pub struct DescentCore {
    pub cfg: GlobalCfg,
}

pub struct DescentScratch {
    vx: Vec<f32>,
    vy: Vec<f32>,
    gx: Vec<f32>,
    gy: Vec<f32>,
    bins: Bins,
    lambda: f32,
    overflow: f32,
}

impl Core<PlaceDomain> for DescentCore {
    type Scratch = DescentScratch;

    fn scratch(&self, hot: &PlaceHot, cold: &PlaceCold) -> DescentScratch {
        let n = hot.x.len();
        DescentScratch {
            vx: vec![0.0; n],
            vy: vec![0.0; n],
            gx: vec![0.0; n],
            gy: vec![0.0; n],
            bins: Bins::new(cold),
            lambda: self.cfg.lambda0,
            overflow: 1.0,
        }
    }

    fn propose(
        &self,
        hot: &PlaceHot,
        cold: &PlaceCold,
        sc: &mut DescentScratch,
        ctl: &Control,
        _rng: &mut SplitMix64,
        mv: &mut PlaceMv,
    ) -> bool {
        let n = hot.x.len();
        sc.gx.iter_mut().for_each(|g| *g = 0.0);
        sc.gy.iter_mut().for_each(|g| *g = 0.0);

        // HPWL subgradient
        for ni in 0..cold.net_w.len() {
            let cells = cold.nets.row(ni as u32);
            let w = cold.net_w[ni];
            let (x0, x1, y0, y1) = net_bbox(cells, &hot.x, &hot.y);
            for &c in cells {
                let ci = c as usize;
                if hot.x[ci] >= x1 - 0.5 {
                    sc.gx[ci] += w;
                }
                if hot.x[ci] <= x0 + 0.5 {
                    sc.gx[ci] -= w;
                }
                if hot.y[ci] >= y1 - 0.5 {
                    sc.gy[ci] += w;
                }
                if hot.y[ci] <= y0 + 0.5 {
                    sc.gy[ci] -= w;
                }
            }
        }

        // soft pulls (proximity / thermal)
        for p in &cold.pulls {
            let (ai, bi) = (p.a as usize, p.b as usize);
            let (dx, dy) = (hot.x[ai] - hot.x[bi], hot.y[ai] - hot.y[bi]);
            let d = (dx * dx + dy * dy).sqrt().max(1.0);
            let ex = (d - p.gap_nm).max(0.0);
            let k = 2.0 * p.weight * ex * 1e-3 / d;
            sc.gx[ai] += k * dx;
            sc.gx[bi] -= k * dx;
            sc.gy[ai] += k * dy;
            sc.gy[bi] -= k * dy;
        }

        // common centroid
        for ci in 0..cold.cc_count() {
            let (ga, gb) = cold.cc_group(ci);
            let (ax, ay) = centroid(ga, &hot.x, &hot.y);
            let (bx, by) = centroid(gb, &hot.x, &hot.y);
            let (dx, dy) = (ax - bx, ay - by);
            for &c in ga {
                sc.gx[c as usize] += 2e-3 * dx / ga.len() as f32;
                sc.gy[c as usize] += 2e-3 * dy / ga.len() as f32;
            }
            for &c in gb {
                sc.gx[c as usize] -= 2e-3 * dx / gb.len() as f32;
                sc.gy[c as usize] -= 2e-3 * dy / gb.len() as f32;
            }
            // pattern-aware gradient: pull devices toward their pattern slots
            let pat = cold.cc_pattern.get(ci).copied().unwrap_or(CcPattern::None);
            if pat != CcPattern::None && ga.len() == 2 && gb.len() == 2 {
                let all: [u32; 4] = [ga[0], ga[1], gb[0], gb[1]];
                let nn = all.len() as f32;
                let cx = all.iter().map(|&c| hot.x[c as usize]).sum::<f32>() / nn;
                let d_spacing = all
                    .iter()
                    .map(|&c| (hot.x[c as usize] - cx).abs())
                    .sum::<f32>()
                    / nn;
                let d_spacing = d_spacing.max(100.0);
                let offsets: [f32; 4] = [-1.5, -0.5, 0.5, 1.5];
                let assignment: [(usize, usize); 4] = match pat {
                    CcPattern::Abba => [(0, 0), (1, 0), (1, 1), (0, 1)],
                    CcPattern::Abab => [(0, 0), (1, 0), (0, 1), (1, 1)],
                    CcPattern::CommonCentroid2d => [(0, 0), (1, 0), (1, 1), (0, 1)],
                    CcPattern::None => unreachable!(),
                };
                let sides: [&[u32]; 2] = [ga, gb];
                for (slot, &(side, idx)) in assignment.iter().enumerate() {
                    if idx < sides[side].len() {
                        let c = sides[side][idx] as usize;
                        let target_x = cx + offsets[slot] * d_spacing;
                        sc.gx[c] += 1e-3 * (hot.x[c] - target_x);
                    }
                }
            }
        }

        // straight-net alignment
        for (cells, vertical) in &cold.aligns {
            let coords: &[f32] = if *vertical { &hot.x } else { &hot.y };
            let mean =
                cells.iter().map(|&c| coords[c as usize]).sum::<f32>() / cells.len() as f32;
            for &c in cells {
                let g = (coords[c as usize] - mean).signum();
                if *vertical {
                    sc.gx[c as usize] += g;
                } else {
                    sc.gy[c as usize] += g;
                }
            }
        }

        // differentiable symmetry error
        let mut new_axis = hot.axis.clone();
        for (gi, members) in cold.sym.groups.iter().enumerate() {
            if !cold.sym.fixed[gi] && !members.is_empty() {
                let mut s = 0.0f32;
                let mut cnt = 0.0f32;
                for &m in members {
                    let p = cold.sym.partner[m as usize];
                    s += if p == m {
                        hot.x[m as usize]
                    } else {
                        (hot.x[m as usize] + hot.x[p as usize]) / 2.0
                    };
                    cnt += 1.0;
                }
                new_axis[gi] = s / cnt;
            }
            let ax = new_axis[gi];
            let k = self.cfg.sym_weight;
            for &m in members {
                let mi = m as usize;
                let p = cold.sym.partner[mi];
                if p == m {
                    sc.gx[mi] += k * (hot.x[mi] - ax) * 1e-3;
                } else if p > m {
                    let pi = p as usize;
                    let ex = (hot.x[mi] + hot.x[pi]) / 2.0 - ax;
                    let ey = hot.y[mi] - hot.y[pi];
                    sc.gx[mi] += k * ex * 1e-3;
                    sc.gx[pi] += k * ex * 1e-3;
                    sc.gy[mi] += k * ey * 1e-3;
                    sc.gy[pi] -= k * ey * 1e-3;
                }
            }
        }

        // density push
        sc.bins.fill(cold, &hot.x, &hot.y);
        let nb = sc.bins.nb;
        for i in 0..n {
            let b = sc.bins.bin_of(hot.x[i], hot.y[i]);
            let over = sc.bins.util[b] - self.cfg.target_util;
            if over <= 0.0 {
                continue;
            }
            let (bx, by) = (b % nb, b / nb);
            let mut best = (sc.bins.util[b], 0i32, 0i32);
            for (dx, dy) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                let (tx, ty) = (bx as i32 + dx, by as i32 + dy);
                if tx >= 0 && ty >= 0 && (tx as usize) < nb && (ty as usize) < nb {
                    let u = sc.bins.util[ty as usize * nb + tx as usize];
                    if u < best.0 {
                        best = (u, dx, dy);
                    }
                }
            }
            sc.gx[i] -= sc.lambda * over * best.1 as f32;
            sc.gy[i] -= sc.lambda * over * best.2 as f32;
        }

        // momentum step
        let span = cold.die.0.max(cold.die.1);
        let gmax = sc
            .gx
            .iter()
            .chain(sc.gy.iter())
            .fold(0.0f32, |m, g| m.max(g.abs()))
            .max(1e-12);
        let scale = ctl.step * span / gmax;

        mv.clear();
        mv.idx.extend(0..n as u32);
        for i in 0..n {
            sc.vx[i] = self.cfg.momentum * sc.vx[i] - scale * sc.gx[i];
            sc.vy[i] = self.cfg.momentum * sc.vy[i] - scale * sc.gy[i];
            mv.nx
                .push((hot.x[i] + sc.vx[i]).clamp(cold.hw[i], (cold.die.0 - cold.hw[i]).max(cold.hw[i])));
            mv.ny
                .push((hot.y[i] + sc.vy[i]).clamp(cold.hh[i], (cold.die.1 - cold.hh[i]).max(cold.hh[i])));
        }
        mv.cached_delta =
            Some(cost_at(cold, &mv.nx, &mv.ny) - cost_at(cold, &hot.x, &hot.y));
        true
    }

    fn commit(
        &self,
        hot: &mut PlaceHot,
        cold: &PlaceCold,
        sc: &mut DescentScratch,
        mv: &mut PlaceMv,
    ) {
        hot.x.copy_from_slice(&mv.nx);
        hot.y.copy_from_slice(&mv.ny);
        hot.buckets.rebuild(&hot.x, &hot.y);
        for (gi, members) in cold.sym.groups.iter().enumerate() {
            if cold.sym.fixed[gi] || members.is_empty() {
                continue;
            }
            let mut s = 0.0f32;
            for &m in members {
                let p = cold.sym.partner[m as usize];
                s += if p == m {
                    hot.x[m as usize]
                } else {
                    (hot.x[m as usize] + hot.x[p as usize]) / 2.0
                };
            }
            hot.axis[gi] = s / members.len() as f32;
        }
        sc.bins.fill(cold, &hot.x, &hot.y);
        sc.overflow = sc.bins.overflow(cold, self.cfg.target_util);
    }

    fn epoch(
        &self,
        _hot: &mut PlaceHot,
        _cold: &PlaceCold,
        sc: &mut DescentScratch,
        _t: &Telemetry,
    ) {
        if sc.overflow > 0.05 {
            sc.lambda = (sc.lambda * 1.05).min(1e3);
        }
    }
}

/// Deterministic initial state: everything near die center with seeded jitter;
/// symmetric partners start mirrored so the sym penalty begins near zero.
/// 2.4: mirror partners get FN orientation for true mirror-image symmetry.
pub fn initial_state(cold: &PlaceCold, rng: &mut SplitMix64) -> PlaceHot {
    let n = cold.hw.len();
    let (cx, cy) = (cold.die.0 / 2.0, cold.die.1 / 2.0);
    let jitter = cold.die.0.min(cold.die.1) * 0.15;
    let has_variants = !cold.variants.is_empty() && cold.variants.iter().any(|v| v.len() > 1);
    let mut hot = PlaceHot {
        x: vec![0.0; n],
        y: vec![0.0; n],
        axis: cold.sym.axis0.clone(),
        buckets: Buckets::default(),
        orient: vec![Orient::N; n],
        variant_idx: if cold.initial_variant.len() == n {
            cold.initial_variant.clone()
        } else {
            vec![0; n]
        },
        cur_hw: if has_variants { cold.hw.clone() } else { Vec::new() },
        cur_hh: if has_variants { cold.hh.clone() } else { Vec::new() },
    };
    for i in 0..n {
        hot.x[i] = cx + rng.centered(jitter);
        hot.y[i] = cy + rng.centered(jitter);
    }
    for (gi, members) in cold.sym.groups.iter().enumerate() {
        let ax = hot.axis[gi];
        for &m in members {
            let mi = m as usize;
            let p = cold.sym.partner[mi];
            if p == m {
                hot.x[mi] = ax;
            } else if p > m {
                let pi = p as usize;
                hot.x[pi] = 2.0 * ax - hot.x[mi];
                hot.y[pi] = hot.y[mi];
                // 2.4: mirror partner gets FN orientation
                hot.orient[pi] = Orient::FN;
            }
        }
    }
    hot.buckets = Buckets::build(cold, &hot.x, &hot.y);
    hot
}

pub fn run_global<Lg: Ledger<PlaceDomain>>(
    hot: &mut PlaceHot,
    cold: &PlaceCold,
    ledger: &mut Lg,
    cfg: &GlobalCfg,
    rng: &mut SplitMix64,
) -> Telemetry {
    let stage = Stage {
        cost: GlobalCost,
        density: BinDensity { target_util: cfg.target_util },
        legality: AllLegal,
        weights: UnitWeights,
        core: DescentCore { cfg: cfg.clone() },
        accept: AlwaysAccept,
        schedule: StepDecay {
            step0: cfg.step0,
            decay: 0.995,
            step_min: 0.002,
            moves_per_step: 1,
        },
        stop: OverflowStop {
            max_iters: cfg.max_iters,
            min_iters: 60,
            target: cfg.overflow_target,
        },
        _d: PhantomData,
    };
    stage.run(hot, cold, ledger, rng)
}

// ---------------------------------------------------------------------------
// Detailed stage — symmetry-preserving SA
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DetailedCfg {
    pub max_iters: u32,
    pub min_iters: u32,
    pub moves_per_cell: u32,
    pub alpha: f64,
    pub range0: f32,
    pub min_accept_rate: f32,
    /// Weight of normalized placement-outline area in the SA objective.
    /// Zero restores wirelength/constraint-only behavior.
    pub outline_weight: f64,
}

impl Default for DetailedCfg {
    fn default() -> Self {
        Self {
            max_iters: 220,
            min_iters: 20,
            moves_per_cell: 60,
            alpha: 0.93,
            range0: 0.4,
            min_accept_rate: 0.02,
            outline_weight: 1.5,
        }
    }
}

/// Reused scratch (RefCell: `CostFn::delta` takes `&self`; stages are
/// single-threaded, so this is uncontended buffer reuse, not shared state).
pub struct SaCost {
    nets_scratch: std::cell::RefCell<Vec<u32>>,
    /// Lazy cell→(pulls, cc-groups, aligns) index — same rationale as
    /// [`HardGaps::pair_idx`]: kills the per-move full-list `touches` scans.
    term_idx: std::cell::OnceCell<(Csr, Csr, Csr)>,
    row_scratch: std::cell::RefCell<Vec<u32>>,
    outline_weight: f64,
}

impl SaCost {
    fn new(outline_weight: f64) -> Self {
        Self {
            nets_scratch: std::cell::RefCell::new(Vec::new()),
            term_idx: std::cell::OnceCell::new(),
            row_scratch: std::cell::RefCell::new(Vec::new()),
            outline_weight: outline_weight.max(0.0),
        }
    }
}

impl CostFn<PlaceDomain> for SaCost {
    fn eval(&self, hot: &PlaceHot, cold: &PlaceCold) -> f64 {
        let variant_cost: f64 = hot.variant_idx.iter().enumerate().map(|(c, &vi)| {
            cold.variant_penalty.get(c).and_then(|v| v.get(vi as usize)).copied().unwrap_or(0.0)
        }).sum();
        cost_at_pins(cold, hot)
            + variant_cost
            + self.outline_weight * outline_metric(cold, hot, None)
    }

    fn delta(&self, hot: &PlaceHot, cold: &PlaceCold, mv: &PlaceMv) -> f64 {
        let mut nets = self.nets_scratch.borrow_mut();
        nets.clear();
        nets.extend(
            mv.idx.iter().flat_map(|&c| cold.cell_nets.row(c).iter().copied()),
        );
        nets.sort_unstable();
        nets.dedup();
        let nets = &*nets;

        let mut d = 0.0f64;
        for &ni in nets {
            let cells = cold.nets.row(ni);
            let w = f64::from(cold.net_w[ni as usize]);
            let (mut ox0, mut ox1, mut oy0, mut oy1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
            let (mut x0, mut x1, mut y0, mut y1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
            for &c in cells {
                let (ox, oy) = pin_pos(cold, hot, c, ni);
                ox0 = ox0.min(ox);
                ox1 = ox1.max(ox);
                oy0 = oy0.min(oy);
                oy1 = oy1.max(oy);
                let (x, y) = moved_pin_pos(cold, hot, mv, c, ni);
                x0 = x0.min(x);
                x1 = x1.max(x);
                y0 = y0.min(y);
                y1 = y1.max(y);
            }
            d += w * f64::from((x1 - x0) + (y1 - y0) - (ox1 - ox0) - (oy1 - oy0));
        }

        for &(c, _, _, nvi, _, _, ovi) in &mv.reshape {
            let row = cold.variant_penalty.get(c as usize);
            let old = row.and_then(|v| v.get(ovi as usize)).copied().unwrap_or(0.0);
            let new = row.and_then(|v| v.get(nvi as usize)).copied().unwrap_or(0.0);
            d += new - old;
        }

        if self.outline_weight > 0.0 {
            d += self.outline_weight
                * (outline_metric(cold, hot, Some(mv)) - outline_metric(cold, hot, None));
        }

        let (pull_csr, cc_csr, align_csr) = self.term_idx.get_or_init(|| {
            let n = cold.hw.len();
            let mut cc_inc = Vec::new();
            for ci in 0..cold.cc_count() {
                let (ga, gb) = cold.cc_group(ci);
                cc_inc.extend(ga.iter().chain(gb).map(|&c| (c, ci as u32)));
            }
            let mut align_inc = Vec::new();
            for (ai, (cells, _)) in cold.aligns.iter().enumerate() {
                align_inc.extend(cells.iter().map(|&c| (c, ai as u32)));
            }
            // A cell listed twice in one group/align would duplicate its row
            // entry and break merge_rows' sorted-unique row invariant.
            cc_inc.sort_unstable();
            cc_inc.dedup();
            align_inc.sort_unstable();
            align_inc.dedup();
            (
                incidence_csr(n, &pair_incidence(cold.pulls.iter().map(|p| (p.a, p.b)))),
                incidence_csr(n, &cc_inc),
                incidence_csr(n, &align_inc),
            )
        });
        let mut rows = self.row_scratch.borrow_mut();
        merge_rows(pull_csr, &mv.idx, &mut rows);
        for &k in rows.iter() {
            let p = &cold.pulls[k as usize];
            d += pull_cost(p, mv.new_pos(p.a, hot), mv.new_pos(p.b, hot))
                - pull_cost(
                    p,
                    (hot.x[p.a as usize], hot.y[p.a as usize]),
                    (hot.x[p.b as usize], hot.y[p.b as usize]),
                );
        }
        merge_rows(cc_csr, &mv.idx, &mut rows);
        for &k in rows.iter() {
            let ci = k as usize;
            let (ga, gb) = cold.cc_group(ci);
            d += cc_cost(ga, gb, |c| mv.new_pos(c, hot))
                - cc_cost(ga, gb, |c| (hot.x[c as usize], hot.y[c as usize]));
            let pat = cold.cc_pattern.get(ci).copied().unwrap_or(CcPattern::None);
            d += cc_pattern_cost(pat, ga, gb, |c| mv.new_pos(c, hot))
                - cc_pattern_cost(pat, ga, gb, |c| {
                    (hot.x[c as usize], hot.y[c as usize])
                });
        }
        merge_rows(align_csr, &mv.idx, &mut rows);
        for &k in rows.iter() {
            let (cells, vertical) = &cold.aligns[k as usize];
            d += align_cost(cells, *vertical, |c| mv.new_pos(c, hot))
                - align_cost(cells, *vertical, |c| {
                    (hot.x[c as usize], hot.y[c as usize])
                });
        }
        d
    }
}

/// Normalized outline objective in wirelength-like units. Dividing area by
/// the square root of total cell area keeps the term comparable across block
/// sizes; the small aspect penalty avoids long, routing-hostile slivers.
fn outline_metric(cold: &PlaceCold, hot: &PlaceHot, mv: Option<&PlaceMv>) -> f64 {
    if hot.x.is_empty() {
        return 0.0;
    }
    let mut xmin = f32::INFINITY;
    let mut ymin = f32::INFINITY;
    let mut xmax = f32::NEG_INFINITY;
    let mut ymax = f32::NEG_INFINITY;
    let mut occupied_area = 0.0f64;
    for i in 0..hot.x.len() {
        let (x, y, hw, hh) = if let Some(mv) = mv {
            let (x, y) = mv.new_pos(i as u32, hot);
            (x, y, mv.prop_hw(i, hot, cold), mv.prop_hh(i, hot, cold))
        } else {
            (hot.x[i], hot.y[i], eff_hw(hot, cold, i), eff_hh(hot, cold, i))
        };
        xmin = xmin.min(x - hw);
        xmax = xmax.max(x + hw);
        ymin = ymin.min(y - hh);
        ymax = ymax.max(y + hh);
        occupied_area += 4.0 * f64::from(hw * hh);
    }
    // Price the legal envelope of a mirror pair even while the current state
    // still overlaps. Without this look-ahead, a very wide folded variant can
    // appear compact until post-SA legalization suddenly pushes its partner
    // outward and creates a long row.
    for (gi, members) in cold.sym.groups.iter().enumerate() {
        let axis = mv
            .and_then(|proposal| proposal.axis.filter(|&(group, _)| group as usize == gi))
            .map_or(hot.axis[gi], |(_, axis)| axis);
        for &m in members {
            let p = cold.sym.partner[m as usize];
            if p == NONE || p <= m {
                continue;
            }
            let (mi, pi) = (m as usize, p as usize);
            let proposal = mv;
            let (mx, _) = proposal.map_or((hot.x[mi], hot.y[mi]), |change| {
                change.new_pos(m, hot)
            });
            let (px, _) = proposal.map_or((hot.x[pi], hot.y[pi]), |change| {
                change.new_pos(p, hot)
            });
            let mhw = proposal.map_or(eff_hw(hot, cold, mi), |change| {
                change.prop_hw(mi, hot, cold)
            });
            let phw = proposal.map_or(eff_hw(hot, cold, pi), |change| {
                change.prop_hw(pi, hot, cold)
            });
            let center_gap = mhw + phw + required_gap(cold, m, p);
            if (mx - px).abs() >= center_gap {
                continue;
            }
            let radius = center_gap * 0.5;
            if mx <= px {
                xmin = xmin.min(axis - radius - mhw);
                xmax = xmax.max(axis + radius + phw);
            } else {
                xmin = xmin.min(axis - radius - phw);
                xmax = xmax.max(axis + radius + mhw);
            }
        }
    }
    let width = (xmax - xmin).max(cold.grid);
    let height = (ymax - ymin).max(cold.grid);
    let cell_scale = cold
        .hw
        .iter()
        .zip(&cold.hh)
        .map(|(&hw, &hh)| 4.0 * f64::from(hw * hh))
        .sum::<f64>()
        .sqrt()
        .max(1.0);
    // The occupied-area component makes variant growth visible even while
    // cells overlap and the enclosing bbox has not expanded yet.
    (f64::from(width * height) + 0.5 * occupied_area) / cell_scale
        + 0.05 * f64::from((width - height).abs())
}

fn pull_cost(p: &PairRule, a: (f32, f32), b: (f32, f32)) -> f64 {
    let (dx, dy) = (a.0 - b.0, a.1 - b.1);
    let ex = ((dx * dx + dy * dy).sqrt() - p.gap_nm).max(0.0);
    f64::from(p.weight) * f64::from(ex * ex) * 1e-3
}

fn cc_cost(ga: &[u32], gb: &[u32], pos: impl Fn(u32) -> (f32, f32)) -> f64 {
    let cen = |cells: &[u32]| {
        let n = cells.len().max(1) as f32;
        let (mut sx, mut sy) = (0.0f32, 0.0f32);
        for &c in cells {
            let (x, y) = pos(c);
            sx += x;
            sy += y;
        }
        (sx / n, sy / n)
    };
    let (ax, ay) = cen(ga);
    let (bx, by) = cen(gb);
    f64::from((ax - bx).powi(2) + (ay - by).powi(2)) * 1e-3
}

/// Pattern-aware CC cost: for ABBA/ABAB with 2+2 devices, generate target
/// positions at [-1.5d, -0.5d, +0.5d, +1.5d] from the group centroid and pull
/// individual devices toward their assigned slot.
fn cc_pattern_cost(
    pat: CcPattern,
    ga: &[u32],
    gb: &[u32],
    pos: impl Fn(u32) -> (f32, f32),
) -> f64 {
    if pat == CcPattern::None || ga.len() != 2 || gb.len() != 2 {
        return 0.0;
    }
    // Combined centroid (guard above ensures exactly 2+2 cells — no alloc)
    let all: [u32; 4] = [ga[0], ga[1], gb[0], gb[1]];
    let n = all.len() as f32;
    let cx: f32 = all.iter().map(|&c| pos(c).0).sum::<f32>() / n;
    // Average device half-width as spacing unit
    let d = all
        .iter()
        .map(|&c| {
            let (x, _) = pos(c);
            (x - cx).abs()
        })
        .sum::<f32>()
        / n;
    let d = d.max(100.0); // avoid degenerate near-zero spacing

    // Slot x-offsets from centroid: [-1.5d, -0.5d, +0.5d, +1.5d]
    let offsets = [-1.5, -0.5, 0.5, 1.5];
    // ABBA: A at +-1.5d, B at +-0.5d → slot assignment: [A0, B0, B1, A1]
    // ABAB: A at -1.5d, +0.5d; B at -0.5d, +1.5d → [A0, B0, A1, B1]
    let assignment: [(usize, usize); 4] = match pat {
        // (side: 0=ga, 1=gb, index within side)
        CcPattern::Abba => [(0, 0), (1, 0), (1, 1), (0, 1)],
        CcPattern::Abab => [(0, 0), (1, 0), (0, 1), (1, 1)],
        // 2D CC: same as ABBA for the 1D pull; real 2D would need y slots too
        CcPattern::CommonCentroid2d => [(0, 0), (1, 0), (1, 1), (0, 1)],
        CcPattern::None => unreachable!(),
    };
    let sides: [&[u32]; 2] = [ga, gb];
    let weight = 0.5e-3; // moderate pull
    let mut cost = 0.0f64;
    for (slot, &(side, idx)) in assignment.iter().enumerate() {
        if idx >= sides[side].len() {
            continue;
        }
        let c = sides[side][idx];
        let (x, _) = pos(c);
        let target_x = cx + offsets[slot] as f32 * d;
        let ex = x - target_x;
        cost += f64::from(ex * ex) * weight;
    }
    cost
}

fn align_cost(cells: &[u32], vertical: bool, pos: impl Fn(u32) -> (f32, f32)) -> f64 {
    let coord = |c: u32| if vertical { pos(c).0 } else { pos(c).1 };
    let mean = cells.iter().map(|&c| coord(c)).sum::<f32>() / cells.len() as f32;
    cells.iter().map(|&c| f64::from((coord(c) - mean).abs())).sum()
}

/// Reused neighbor-scan buffers (see [`SaCost`] for the RefCell rationale).
#[derive(Default)]
pub struct OverlapDensity {
    cand_scratch: std::cell::RefCell<(Vec<u32>, Vec<u32>)>,
}

impl Density<PlaceDomain> for OverlapDensity {
    fn delta(&self, hot: &PlaceHot, cold: &PlaceCold, mv: &PlaceMv) -> f64 {
        let mut scratch = self.cand_scratch.borrow_mut();
        let (cand, cand2) = &mut *scratch;
        let mut d = 0.0f64;
        for (k, &a) in mv.idx.iter().enumerate() {
            let ai = a as usize;
            let (nax, nay) = (mv.nx[k], mv.ny[k]);
            hot.buckets.near(hot.x[ai], hot.y[ai], cand);
            // Old and new positions usually share a bucket (low-temp moves are
            // small) — near() is then identical and the second scan is skipped.
            // Sort+dedup of the union keeps the same candidate order either way.
            if hot.buckets.idx(nax, nay) != hot.buckets.idx(hot.x[ai], hot.y[ai]) {
                hot.buckets.near(nax, nay, cand2);
                cand.extend_from_slice(cand2);
            }
            cand.sort_unstable();
            cand.dedup();
            // Hoist a's proposed size — constant across the candidate scan.
            let (pa_hw, pa_hh) = (mv.prop_hw(ai, hot, cold), mv.prop_hh(ai, hot, cold));
            for &b in cand.iter() {
                if b == a {
                    continue;
                }
                let bi = b as usize;
                let kb_pos = mv.idx.iter().position(|&m| m == b);
                if matches!(kb_pos, Some(kb) if kb < k) {
                    continue; // pair handled at b's (lower) index — skip before costing
                }
                // Old overlap uses current sizes; new overlap uses proposed sizes
                let old = overlap_area_eff(hot, cold, ai, bi, &hot.x, &hot.y);
                if let Some(kb) = kb_pos {
                    let new = if legal_abutment_proposed(cold, hot, mv, ai, bi) { 0.0 } else {
                        rect_overlap_sized(cold, ai, bi,
                        pa_hw, pa_hh,
                        mv.prop_hw(bi, hot, cold), mv.prop_hh(bi, hot, cold),
                        nax, nay, mv.nx[kb], mv.ny[kb])
                    };
                    d += new - old;
                } else {
                    let new = if legal_abutment_proposed(cold, hot, mv, ai, bi) { 0.0 } else {
                        rect_overlap_sized(cold, ai, bi,
                        pa_hw, pa_hh,
                        eff_hw(hot, cold, bi), eff_hh(hot, cold, bi),
                        nax, nay, hot.x[bi], hot.y[bi])
                    };
                    d += new - old;
                }
            }
        }
        d
    }

    fn overflow(&self, hot: &PlaceHot, cold: &PlaceCold) -> f32 {
        let n = cold.hw.len();
        let total: f32 = (0..n).map(|i| 4.0 * eff_hw(hot, cold, i) * eff_hh(hot, cold, i)).sum();
        if total <= 0.0 {
            return 0.0;
        }
        let mut cand = Vec::with_capacity(16);
        let mut over = 0.0f64;
        for a in 0..n {
            hot.buckets.near(hot.x[a], hot.y[a], &mut cand);
            for &b in &cand {
                if (b as usize) > a {
                    over += overlap_area_eff(hot, cold, a, b as usize, &hot.x, &hot.y);
                }
            }
        }
        (over as f32) / total
    }
}

fn rect_overlap_sized(cold: &PlaceCold, a: usize, b: usize,
    hw_a: f32, hh_a: f32, hw_b: f32, hh_b: f32,
    ax: f32, ay: f32, bx: f32, by: f32,
) -> f64 {
    if !layers_conflict(cold, a, b) { return 0.0; }
    let ox = (hw_a + hw_b) - (ax - bx).abs();
    let oy = (hh_a + hh_b) - (ay - by).abs();
    if ox > 0.0 && oy > 0.0 {
        f64::from(ox) * f64::from(oy)
    } else {
        0.0
    }
}

#[allow(dead_code)]
fn rect_overlap(cold: &PlaceCold, a: usize, b: usize, ax: f32, ay: f32, bx: f32, by: f32) -> f64 {
    rect_overlap_sized(cold, a, b, cold.hw[a], cold.hh[a], cold.hw[b], cold.hh[b], ax, ay, bx, by)
}

/// Reused neighbor-scan buffer (see [`SaCost`] for the RefCell rationale).
#[derive(Default)]
pub struct HardGaps {
    cand_scratch: std::cell::RefCell<Vec<u32>>,
    /// Lazy cell→(pushes, dti_zones) index. One HardGaps instance serves
    /// exactly one stage run over one `cold`, so building once is sound.
    /// Turns the per-move full-list scans into per-moved-cell row merges.
    pair_idx: std::cell::OnceCell<(Csr, Csr)>,
    row_scratch: std::cell::RefCell<Vec<u32>>,
}

/// Edge-to-edge gap between `a` and `b` at their CURRENT positions/sizes.
#[inline]
fn cur_gap(hot: &PlaceHot, cold: &PlaceCold, a: usize, b: usize) -> f32 {
    let gx = (hot.x[a] - hot.x[b]).abs() - (eff_hw(hot, cold, a) + eff_hw(hot, cold, b));
    let gy = (hot.y[a] - hot.y[b]).abs() - (eff_hh(hot, cold, a) + eff_hh(hot, cold, b));
    gx.max(gy)
}

/// Edge-to-edge gap between `a` and `b` at the PROPOSED positions/sizes.
#[inline]
fn new_gap(hot: &PlaceHot, cold: &PlaceCold, mv: &PlaceMv, a: usize, b: usize) -> f32 {
    let (ax, ay) = mv.new_pos(a as u32, hot);
    let (bx, by) = mv.new_pos(b as u32, hot);
    let gx = (ax - bx).abs() - (mv.prop_hw(a, hot, cold) + mv.prop_hw(b, hot, cold));
    let gy = (ay - by).abs() - (mv.prop_hh(a, hot, cold) + mv.prop_hh(b, hot, cold));
    gx.max(gy)
}

impl Legality<PlaceDomain> for HardGaps {
    /// NON-WORSENING semantics: a move is illegal only when it makes some
    /// hard gap WORSE than it currently is. The global stage routinely hands
    /// SA a state that already violates hard constraints — absolute rejection
    /// would freeze SA solid (zero accepted moves), locking the violations in.
    /// Non-worsening lets SA walk out of violation while never deepening it.
    fn is_legal(&self, hot: &PlaceHot, cold: &PlaceCold, mv: &PlaceMv) -> bool {
        let (push_csr, dti_csr) = self.pair_idx.get_or_init(|| {
            let n = cold.hw.len();
            (
                incidence_csr(n, &pair_incidence(cold.pushes.iter().map(|p| (p.a, p.b)))),
                incidence_csr(n, &pair_incidence(cold.dti_zones.iter().map(|z| (z.0, z.1)))),
            )
        });
        let mut rows = self.row_scratch.borrow_mut();
        merge_rows(push_csr, &mv.idx, &mut rows);
        for &k in rows.iter() {
            let p = &cold.pushes[k as usize];
            let (ai, bi) = (p.a as usize, p.b as usize);
            let ng = new_gap(hot, cold, mv, ai, bi);
            if ng < p.gap_nm && ng < cur_gap(hot, cold, ai, bi) {
                return false;
            }
        }
        // DTI forbidden zones: gap must be < min (abutting) or > max (far
        // apart). Inside the band, penetration depth must not increase.
        merge_rows(dti_csr, &mv.idx, &mut rows);
        for &k in rows.iter() {
            let (a, b, min_gap, max_gap) = cold.dti_zones[k as usize];
            let (ai, bi) = (a as usize, b as usize);
            let depth = |g: f32| (g - min_gap).min(max_gap - g); // >0 = inside band
            let nd = depth(new_gap(hot, cold, mv, ai, bi));
            if nd > 0.0 && nd > depth(cur_gap(hot, cold, ai, bi)).max(0.0) {
                return false;
            }
        }
        drop(rows);
        // 2.3 + 3.3: class-pair spacing — enforce required_gap between moved cells
        // and their neighbors via bucket scan
        if cold.class_count > 0 || !cold.pair_min_dist.is_empty() {
            let mut cand = self.cand_scratch.borrow_mut();
            for (k, &a) in mv.idx.iter().enumerate() {
                let (nax, nay) = (mv.nx[k], mv.ny[k]);
                hot.buckets.near(nax, nay, &mut cand);
                let ai = a as usize;
                // Hoist a's proposed pos/size — new_gap would re-derive them
                // (mv.idx / mv.reshape scans) for every candidate.
                let (pa_hw, pa_hh) = (mv.prop_hw(ai, hot, cold), mv.prop_hh(ai, hot, cold));
                for &b in cand.iter() {
                    if b == a { continue; }
                    let bi = b as usize;
                    if legal_abutment_proposed(cold, hot, mv, ai, bi) {
                        continue;
                    }
                    let req = required_gap(cold, a, b);
                    if req <= 0.0 { continue; }
                    let (bx, by) = mv.new_pos(b, hot);
                    let gx = (nax - bx).abs() - (pa_hw + mv.prop_hw(bi, hot, cold));
                    let gy = (nay - by).abs() - (pa_hh + mv.prop_hh(bi, hot, cold));
                    let ng = gx.max(gy);
                    if ng < req && ng < cur_gap(hot, cold, ai, bi) {
                        return false;
                    }
                }
            }
        }
        true
    }
}

pub struct SymSaCore;

impl SymSaCore {
    #[allow(dead_code)]
    fn t_bounds(cells: &[u32], pos: &[f32], half: &[f32], span: f32) -> (f32, f32) {
        let (mut lo, mut hi) = (f32::MIN, f32::MAX);
        for &c in cells {
            let ci = c as usize;
            lo = lo.max(half[ci] - pos[ci]);
            hi = hi.min(span - half[ci] - pos[ci]);
        }
        let lo = lo.min(0.0);
        let hi = hi.max(0.0);
        // oversized cell: can't fit, allow zero displacement
        if lo > hi { (0.0, 0.0) } else { (lo, hi) }
    }

    fn t_bounds_eff(cells: &[u32], pos: &[f32], hot: &PlaceHot, cold: &PlaceCold,
        use_hw: bool, span: f32,
    ) -> (f32, f32) {
        let (mut lo, mut hi) = (f32::MIN, f32::MAX);
        for &c in cells {
            let ci = c as usize;
            let h = if use_hw { eff_hw(hot, cold, ci) } else { eff_hh(hot, cold, ci) };
            lo = lo.max(h - pos[ci]);
            hi = hi.min(span - h - pos[ci]);
        }
        let lo = lo.min(0.0);
        let hi = hi.max(0.0);
        if lo > hi { (0.0, 0.0) } else { (lo, hi) }
    }

    /// Reshape a self-symmetric cell or both members of a mirror pair as one
    /// atomic move. Paired cells only use shape-compatible variant indices, so
    /// the symmetry invariant remains true throughout annealing.
    fn sym_reshape(
        m: u32,
        p: u32,
        axis: f32,
        hot: &PlaceHot,
        cold: &PlaceCold,
        rng: &mut SplitMix64,
        mv: &mut PlaceMv,
    ) -> bool {
        let (mi, pi) = (m as usize, p as usize);
        let Some(mvars) = cold.variants.get(mi) else { return false };
        if mvars.len() <= 1 {
            return false;
        }

        let current_m = hot.variant_idx.get(mi).copied().unwrap_or(0) as usize;
        let pick = if m == p {
            let mut vi = rng.below(mvars.len());
            if vi == current_m {
                vi = (vi + 1) % mvars.len();
            }
            vi
        } else {
            let Some(pvars) = cold.variants.get(pi) else { return false };
            let current_p = hot.variant_idx.get(pi).copied().unwrap_or(0) as usize;
            let compatible = |&vi: &usize| {
                mvars[vi] == pvars[vi] && (vi != current_m || vi != current_p)
            };
            let range = 0..mvars.len().min(pvars.len());
            let count = range.clone().filter(compatible).count();
            let Some(vi) = range.filter(compatible).nth(rng.below(count.max(1))) else {
                return false;
            };
            vi
        };

        let (mhw, mhh) = mvars[pick];
        let (x, y) = if m == p {
            if axis < mhw || axis > cold.die.0 - mhw {
                return false;
            }
            (axis, hot.y[mi].clamp(mhh, (cold.die.1 - mhh).max(mhh)))
        } else {
            let (phw, phh) = cold.variants[pi][pick];
            let lo = mhw.max(2.0 * axis - (cold.die.0 - phw));
            let hi = (cold.die.0 - mhw).min(2.0 * axis - phw);
            if lo > hi {
                return false;
            }
            let hh = mhh.max(phh);
            (
                hot.x[mi].clamp(lo, hi),
                hot.y[mi].clamp(hh, (cold.die.1 - hh).max(hh)),
            )
        };

        let mut add = |cell: u32, nx: f32, ny: f32, nhw: f32, nhh: f32| {
            let ci = cell as usize;
            let old_vi = hot.variant_idx.get(ci).copied().unwrap_or(0);
            mv.reshape.push((
                cell,
                nhw,
                nhh,
                pick as u16,
                eff_hw(hot, cold, ci),
                eff_hh(hot, cold, ci),
                old_vi,
            ));
            mv.idx.push(cell);
            mv.nx.push(nx);
            mv.ny.push(ny);
        };
        add(m, x, y, mhw, mhh);
        if p != m {
            let (phw, phh) = cold.variants[pi][pick];
            add(p, 2.0 * axis - x, y, phw, phh);
        }
        true
    }

    /// Change orientations without breaking a vertical mirror relation.
    fn sym_orient(
        m: u32,
        p: u32,
        hot: &PlaceHot,
        rng: &mut SplitMix64,
        mv: &mut PlaceMv,
    ) -> bool {
        let (mi, pi) = (m as usize, p as usize);
        if m == p {
            let old = hot.orient.get(mi).copied().unwrap_or(Orient::N);
            let new = if old == Orient::N { Orient::S } else { Orient::N };
            mv.idx.push(m);
            mv.nx.push(hot.x[mi]);
            mv.ny.push(hot.y[mi]);
            mv.orient_changes.push((m, new, old));
            return true;
        }

        let choices = [
            (Orient::N, Orient::FN),
            (Orient::S, Orient::FS),
            (Orient::FN, Orient::N),
            (Orient::FS, Orient::S),
        ];
        let old_m = hot.orient.get(mi).copied().unwrap_or(Orient::N);
        let old_p = hot.orient.get(pi).copied().unwrap_or(Orient::FN);
        let mut choice_idx = rng.below(choices.len());
        if choices[choice_idx] == (old_m, old_p) {
            choice_idx = (choice_idx + 1) % choices.len();
        }
        let choice = choices[choice_idx];
        for (cell, ci, new, old) in [(m, mi, choice.0, old_m), (p, pi, choice.1, old_p)] {
            mv.idx.push(cell);
            mv.nx.push(hot.x[ci]);
            mv.ny.push(hot.y[ci]);
            mv.orient_changes.push((cell, new, old));
        }
        true
    }

    fn abut_target(r: &Abutment, c: u32, hot: &PlaceHot, cold: &PlaceCold) -> Option<(f32, f32, u16)> {
        let (other, x, y, vi, other_vi) = if c == r.a {
            let o = r.b as usize;
            (r.b, hot.x[o] - r.dx, hot.y[o] - r.dy, r.variant_a, r.variant_b)
        } else if c == r.b {
            let o = r.a as usize;
            (r.a, hot.x[o] + r.dx, hot.y[o] + r.dy, r.variant_b, r.variant_a)
        } else {
            return None;
        };
        let oi = other as usize;
        if cold.sym.group_of.get(oi).copied().unwrap_or(NONE) != NONE
            || hot.variant_idx.get(oi).copied().unwrap_or(0) != other_vi
            || hot.orient.get(oi).copied().unwrap_or(Orient::N) != Orient::N
        {
            return None;
        }
        let ci = c as usize;
        let (hw, hh) = cold.variants.get(ci)
            .and_then(|v| v.get(vi as usize)).copied()
            .unwrap_or((eff_hw(hot, cold, ci), eff_hh(hot, cold, ci)));
        (x >= hw && x <= cold.die.0 - hw && y >= hh && y <= cold.die.1 - hh)
            .then_some((x, y, vi))
    }
}

impl Core<PlaceDomain> for SymSaCore {
    type Scratch = ();

    fn scratch(&self, _: &PlaceHot, _: &PlaceCold) {}

    fn propose(
        &self,
        hot: &PlaceHot,
        cold: &PlaceCold,
        _sc: &mut (),
        ctl: &Control,
        rng: &mut SplitMix64,
        mv: &mut PlaceMv,
    ) -> bool {
        let n = hot.x.len();
        if n == 0 {
            return false;
        }
        let span = cold.die.0.max(cold.die.1);
        let r = ctl.range * span;
        let c = rng.below(n) as u32;
        let g = cold.sym.group_of[c as usize];

        mv.clear();
        if g != NONE {
            let members = &cold.sym.groups[g as usize];
            let roll = rng.f32();
            if roll < 0.30 {
                let (lox, hix) = Self::t_bounds_eff(members, &hot.x, hot, cold, true, cold.die.0);
                let (loy, hiy) = Self::t_bounds_eff(members, &hot.y, hot, cold, false, cold.die.1);
                let dx = rng.centered(r).clamp(lox, hix);
                let dy = rng.centered(r).clamp(loy, hiy);
                for &m in members {
                    if cold.sym.partner[m as usize] != NONE {
                        mv.idx.push(m);
                    }
                }
                mv.idx.sort_unstable();
                mv.idx.dedup();
                mv.nx.extend(mv.idx.iter().map(|&m| hot.x[m as usize] + dx));
                mv.ny.extend(mv.idx.iter().map(|&m| hot.y[m as usize] + dy));
                mv.axis = Some((g, hot.axis[g as usize] + dx));
            } else {
                let m = members[rng.below(members.len())];
                let mi = m as usize;
                let p = cold.sym.partner[mi];
                let ax = hot.axis[g as usize];
                if roll < 0.50 && Self::sym_reshape(m, p, ax, hot, cold, rng, mv) {
                    return true;
                }
                if roll < 0.60
                    && !cold.pin_cell.is_empty()
                    && Self::sym_orient(m, p, hot, rng, mv)
                {
                    return true;
                }
                if p == m {
                    let (loy, hiy) = Self::t_bounds_eff(&[m], &hot.y, hot, cold, false, cold.die.1);
                    let dy = rng.centered(r).clamp(loy, hiy);
                    mv.idx.push(m);
                    mv.nx.push(ax);
                    mv.ny.push(hot.y[mi] + dy);
                } else {
                    let pi = p as usize;
                    let hw_m = eff_hw(hot, cold, mi);
                    let hw_p = eff_hw(hot, cold, pi);
                    let dx_lo = (hw_m - hot.x[mi])
                        .max(hot.x[pi] - (cold.die.0 - hw_p));
                    let dx_hi = (cold.die.0 - hw_m - hot.x[mi])
                        .min(hot.x[pi] - hw_p);
                    let (loy, hiy) =
                        Self::t_bounds_eff(&[m, p], &hot.y, hot, cold, false, cold.die.1);
                    let dx = rng.centered(r).clamp(dx_lo.min(0.0), dx_hi.max(0.0));
                    let dy = rng.centered(r).clamp(loy, hiy);
                    let nx = hot.x[mi] + dx;
                    let ny = hot.y[mi] + dy;
                    mv.idx.push(m);
                    mv.nx.push(nx);
                    mv.ny.push(ny);
                    mv.idx.push(p);
                    mv.nx.push(2.0 * ax - nx);
                    mv.ny.push(ny);
                }
            }
        } else {
            // 2.1: reshape op (~10% probability for cells with variants)
            let ci = c as usize;
            let has_variants = ci < cold.variants.len() && cold.variants[ci].len() > 1;
            let roll = rng.f32();
            let abut_count = cold.abutments.iter()
                .filter(|r| Self::abut_target(r, c, hot, cold).is_some())
                .count();
            if has_variants && roll < 0.10 {
                // Pick a random variant different from current
                let cur = if ci < hot.variant_idx.len() { hot.variant_idx[ci] as usize } else { 0 };
                let nv = cold.variants[ci].len();
                let mut vi = rng.below(nv);
                if vi == cur && nv > 1 { vi = (vi + 1) % nv; }
                let (nhw, nhh) = cold.variants[ci][vi];
                mv.reshape.push((c, nhw, nhh, vi as u16, eff_hw(hot, cold, ci), eff_hh(hot, cold, ci),
                    if ci < hot.variant_idx.len() { hot.variant_idx[ci] } else { 0 }));
                // Keep position, just reshape
                mv.idx.push(c);
                let (xlo, xhi) = (nhw, (cold.die.0 - nhw).max(nhw));
                let (ylo, yhi) = (nhh, (cold.die.1 - nhh).max(nhh));
                mv.nx.push(hot.x[ci].clamp(xlo, xhi));
                mv.ny.push(hot.y[ci].clamp(ylo, yhi));
            } else if !cold.pin_cell.is_empty() && roll < 0.20 {
                let old = hot.orient.get(ci).copied().unwrap_or(Orient::N);
                let choices = [Orient::N, Orient::S, Orient::FN, Orient::FS];
                let mut new = choices[rng.below(choices.len())];
                if new == old { new = choices[(new as usize + 1) % choices.len()]; }
                mv.idx.push(c);
                mv.nx.push(hot.x[ci]);
                mv.ny.push(hot.y[ci]);
                mv.orient_changes.push((c, new, old));
            } else if abut_count > 0 && roll < 0.35 {
                let pick = rng.below(abut_count);
                let rule = cold.abutments.iter()
                    .filter(|r| Self::abut_target(r, c, hot, cold).is_some())
                    .nth(pick).expect("counted abutment");
                let (x, y, vi) = Self::abut_target(rule, c, hot, cold).expect("filtered abutment");
                let old_vi = hot.variant_idx.get(ci).copied().unwrap_or(0);
                if vi != old_vi {
                    let (nhw, nhh) = cold.variants[ci][vi as usize];
                    mv.reshape.push((c, nhw, nhh, vi, eff_hw(hot, cold, ci), eff_hh(hot, cold, ci), old_vi));
                }
                let old_o = hot.orient.get(ci).copied().unwrap_or(Orient::N);
                if old_o != Orient::N {
                    mv.orient_changes.push((c, Orient::N, old_o));
                }
                mv.idx.push(c);
                mv.nx.push(x);
                mv.ny.push(y);
            } else if roll < 0.70 {
                let (lox, hix) = Self::t_bounds_eff(&[c], &hot.x, hot, cold, true, cold.die.0);
                let (loy, hiy) = Self::t_bounds_eff(&[c], &hot.y, hot, cold, false, cold.die.1);
                mv.idx.push(c);
                mv.nx.push(hot.x[ci] + rng.centered(r).clamp(lox, hix));
                mv.ny.push(hot.y[ci] + rng.centered(r).clamp(loy, hiy));
            } else {
                let o = rng.below(n) as u32;
                if o == c || cold.sym.group_of[o as usize] != NONE {
                    mv.idx.push(c);
                    mv.nx.push(hot.x[ci]);
                    mv.ny.push(hot.y[ci]);
                } else {
                    let oi = o as usize;
                    let (hw_c, hh_c) = (eff_hw(hot, cold, ci), eff_hh(hot, cold, ci));
                    let (hw_o, hh_o) = (eff_hw(hot, cold, oi), eff_hh(hot, cold, oi));
                    mv.idx.push(c);
                    mv.nx.push(hot.x[oi].clamp(hw_c, (cold.die.0 - hw_c).max(hw_c)));
                    mv.ny.push(hot.y[oi].clamp(hh_c, (cold.die.1 - hh_c).max(hh_c)));
                    mv.idx.push(o);
                    mv.nx.push(hot.x[ci].clamp(hw_o, (cold.die.0 - hw_o).max(hw_o)));
                    mv.ny.push(hot.y[ci].clamp(hh_o, (cold.die.1 - hh_o).max(hh_o)));
                }
            }
        }
        true
    }

    fn commit(&self, hot: &mut PlaceHot, _cold: &PlaceCold, _sc: &mut (), mv: &mut PlaceMv) {
        for (k, &c) in mv.idx.iter().enumerate() {
            let ci = c as usize;
            let old = (hot.x[ci], hot.y[ci]);
            hot.x[ci] = mv.nx[k];
            hot.y[ci] = mv.ny[k];
            hot.buckets.move_cell(c, old, (mv.nx[k], mv.ny[k]));
        }
        if let Some((g, ax)) = mv.axis {
            hot.axis[g as usize] = ax;
        }
        // 2.1: apply reshape — update cur_hw/cur_hh/variant_idx in hot state
        for &(c, nhw, nhh, nvi, _ohw, _ohh, _ovi) in &mv.reshape {
            let ci = c as usize;
            if !hot.cur_hw.is_empty() {
                hot.cur_hw[ci] = nhw;
                hot.cur_hh[ci] = nhh;
            }
            if ci < hot.variant_idx.len() {
                hot.variant_idx[ci] = nvi;
            }
        }
        // 2.4: apply orientation changes
        for &(c, new_o, _old_o) in &mv.orient_changes {
            let ci = c as usize;
            if ci < hot.orient.len() {
                hot.orient[ci] = new_o;
            }
        }
    }
}

pub struct SaStop {
    pub max_iters: u32,
    pub min_iters: u32,
    pub min_accept_rate: f32,
}

impl Stop for SaStop {
    fn done(&self, t: &Telemetry, open: usize) -> bool {
        if t.iters >= self.max_iters {
            return true;
        }
        t.iters >= self.min_iters
            && t.epoch_accept_rate < self.min_accept_rate
            && t.overflow <= 1e-4
            && open == 0
    }
}

pub fn run_detailed<Lg: Ledger<PlaceDomain>>(
    hot: &mut PlaceHot,
    cold: &PlaceCold,
    ledger: &mut Lg,
    cfg: &DetailedCfg,
    rng: &mut SplitMix64,
) -> Telemetry {
    let n = hot.x.len().max(1);

    let core = SymSaCore;
    let cost = SaCost::new(cfg.outline_weight);
    let probe = Control { temp: 0.0, step: 0.0, range: cfg.range0, moves_per_step: 0 };
    let mut sum = 0.0f64;
    let mut cnt = 0u32;
    let mut sc = ();
    let mut mv = PlaceMv::default();
    for _ in 0..128 {
        if core.propose(hot, cold, &mut sc, &probe, rng, &mut mv) {
            sum += cost.delta(hot, cold, &mv).abs();
            cnt += 1;
        }
    }
    // t0 = 0.02 x avg probe |delta|. SA starts from the GLOBAL stage's
    // solution, not from random: the old 10x factor random-walked the input
    // away (cost 0.6M -> 20M in epoch 0 on the 5T OTA) and 220 epochs never
    // re-annealed it. Probe deltas are dominated by range0-sized (0.4 x die)
    // jumps, so even 1x is hot enough to erase the input. Swept on the local
    // fixtures: 1x -> OTA routed WL 258k nm, 0.1x -> 216k, 0.02x -> 106k,
    // 0.01x -> 248k (too greedy, freezes in the first local minimum).
    let t0 = (sum / f64::from(cnt.max(1))).max(1.0) * 0.02;

    let cost0 = cost_at_pins(cold, hot).max(1.0);
    let area: f64 = cold.hw.iter().zip(&cold.hh).map(|(w, h)| 4.0 * f64::from(w * h)).sum();
    let w0 = cost0 / area.max(1.0);

    let stage = Stage {
        cost: SaCost::new(cfg.outline_weight),
        density: OverlapDensity::default(),
        legality: HardGaps::default(),
        weights: RampWeights { w0, gain: 1.08, w_max: w0 * 1e4 },
        core: SymSaCore,
        accept: Metropolis,
        schedule: Geometric {
            t0,
            alpha: cfg.alpha,
            range0: cfg.range0,
            range_decay: 0.96,
            range_min: cold.grid / cold.die.0.max(cold.die.1),
            moves_per_step: cfg.moves_per_cell * n as u32,
        },
        stop: SaStop {
            max_iters: cfg.max_iters,
            min_iters: cfg.min_iters,
            min_accept_rate: cfg.min_accept_rate,
        },
        _d: PhantomData,
    };
    stage.run(hot, cold, ledger, rng)
}

// ---------------------------------------------------------------------------
// Refinement stage — low-temp SA with boosted constraint weights
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RefinementCfg {
    pub max_iters: u32,
    pub min_iters: u32,
    pub moves_per_cell: u32,
    pub alpha: f64,
    pub range0: f32,
    pub min_accept_rate: f32,
    pub constraint_boost: f64,
    pub outline_weight: f64,
}

impl Default for RefinementCfg {
    fn default() -> Self {
        Self {
            max_iters: 80,
            min_iters: 10,
            moves_per_cell: 40,
            alpha: 0.96,
            range0: 0.1,
            min_accept_rate: 0.01,
            constraint_boost: 4.0,
            outline_weight: 2.0,
        }
    }
}

/// Refinement: same core + cost, lower temperature, boosted constraint weights.
pub fn run_refinement<Lg: Ledger<PlaceDomain>>(
    hot: &mut PlaceHot,
    cold: &PlaceCold,
    ledger: &mut Lg,
    cfg: &RefinementCfg,
    rng: &mut SplitMix64,
) -> Telemetry {
    let n = hot.x.len().max(1);

    let core = SymSaCore;
    let cost = SaCost::new(cfg.outline_weight);
    let probe = Control { temp: 0.0, step: 0.0, range: cfg.range0, moves_per_step: 0 };
    let mut sum = 0.0f64;
    let mut cnt = 0u32;
    let mut sc = ();
    let mut mv = PlaceMv::default();
    for _ in 0..64 {
        if core.propose(hot, cold, &mut sc, &probe, rng, &mut mv) {
            sum += cost.delta(hot, cold, &mv).abs();
            cnt += 1;
        }
    }
    // ponytail: start at 2x average delta — low temp, mostly downhill
    let t0 = (sum / f64::from(cnt.max(1))).max(1.0) * 2.0;

    let cost0 = cost_at_pins(cold, hot).max(1.0);
    let area: f64 = cold.hw.iter().zip(&cold.hh).map(|(w, h)| 4.0 * f64::from(w * h)).sum();
    let w0 = (cost0 / area.max(1.0)) * cfg.constraint_boost;

    let stage = Stage {
        cost: SaCost::new(cfg.outline_weight),
        density: OverlapDensity::default(),
        legality: HardGaps::default(),
        weights: RampWeights { w0, gain: 1.04, w_max: w0 * 1e5 },
        core: SymSaCore,
        accept: Metropolis,
        schedule: Geometric {
            t0,
            alpha: cfg.alpha,
            range0: cfg.range0,
            range_decay: 0.98,
            range_min: cold.grid / cold.die.0.max(cold.die.1),
            moves_per_step: cfg.moves_per_cell * n as u32,
        },
        stop: SaStop {
            max_iters: cfg.max_iters,
            min_iters: cfg.min_iters,
            min_accept_rate: cfg.min_accept_rate,
        },
        _d: PhantomData,
    };
    stage.run(hot, cold, ledger, rng)
}

// ---------------------------------------------------------------------------
// 2.5: Hierarchical ASF-B*-tree — per-group symmetric-feasible sub-trees
// ---------------------------------------------------------------------------

/// A node in the ASF (Analog Symmetry-Feasible) B*-tree.
/// Each symmetry group gets its own small tree over pair representatives;
/// the top-level tree references groups as hierarchy nodes.
#[derive(Debug, Clone)]
pub struct AsfNode {
    /// Cell index (for leaf) or group index (for hierarchy).
    pub id: u32,
    pub kind: AsfNodeKind,
    /// Left child (first child in B*-tree).
    pub left: Option<u32>,
    /// Right sibling.
    pub right: Option<u32>,
    /// Parent.
    pub parent: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsfNodeKind {
    /// A regular cell (ungrouped or representative of a pair).
    Leaf,
    /// A symmetry group: contains a sub-tree for intra-group packing.
    /// The sub-tree root is stored in `AsfTree::sub_roots[group_idx]`.
    Hierarchy { group: u32 },
}

/// Hierarchical ASF-B*-tree: top-level tree + per-group sub-trees.
#[derive(Debug, Clone, Default)]
pub struct AsfTree {
    /// All nodes (top-level + sub-tree nodes share this arena).
    pub nodes: Vec<AsfNode>,
    /// Top-level tree root index into `nodes`.
    pub root: Option<u32>,
    /// Per symmetry group: sub-tree root index into `nodes`.
    /// `sub_roots[gi]` = Some(node_idx) when group gi has a sub-tree.
    pub sub_roots: Vec<Option<u32>>,
}

impl AsfTree {
    /// Build an initial ASF-B*-tree from the cold context.
    /// Groups with >1 member get hierarchy nodes at the top level;
    /// ungrouped cells are leaf nodes. Intra-group pairs are arranged
    /// as representatives in sub-trees.
    pub fn build(cold: &PlaceCold) -> Self {
        let n = cold.hw.len();
        let mut tree = AsfTree {
            nodes: Vec::with_capacity(n + cold.sym.groups.len()),
            root: None,
            sub_roots: vec![None; cold.sym.groups.len()],
        };

        let mut top_ids = Vec::new();

        // Create hierarchy nodes for each symmetry group
        for (gi, members) in cold.sym.groups.iter().enumerate() {
            if members.is_empty() { continue; }
            let hier_idx = tree.nodes.len() as u32;
            tree.nodes.push(AsfNode {
                id: gi as u32,
                kind: AsfNodeKind::Hierarchy { group: gi as u32 },
                left: None,
                right: None,
                parent: None,
            });
            top_ids.push(hier_idx);

            // Sub-tree: one node per pair representative (lower index of each pair)
            // + self-symmetric devices
            let mut reps = Vec::new();
            for &m in members {
                let mi = m as usize;
                let p = cold.sym.partner[mi];
                if p == m || m < p {
                    let sub_idx = tree.nodes.len() as u32;
                    tree.nodes.push(AsfNode {
                        id: m,
                        kind: AsfNodeKind::Leaf,
                        left: None,
                        right: None,
                        parent: Some(hier_idx),
                    });
                    reps.push(sub_idx);
                }
            }
            // Chain sub-tree nodes: first is root, rest are right siblings
            if let Some(&first) = reps.first() {
                tree.sub_roots[gi] = Some(first);
                tree.nodes[hier_idx as usize].left = Some(first);
                for w in reps.windows(2) {
                    tree.nodes[w[0] as usize].right = Some(w[1]);
                }
            }
        }

        // Add ungrouped cells as leaf nodes
        for i in 0..n {
            if cold.sym.group_of[i] == NONE {
                let leaf_idx = tree.nodes.len() as u32;
                tree.nodes.push(AsfNode {
                    id: i as u32,
                    kind: AsfNodeKind::Leaf,
                    left: None,
                    right: None,
                    parent: None,
                });
                top_ids.push(leaf_idx);
            }
        }

        // Chain top-level nodes: first is root, rest are right siblings
        if let Some(&first) = top_ids.first() {
            tree.root = Some(first);
            for w in top_ids.windows(2) {
                tree.nodes[w[0] as usize].right = Some(w[1]);
            }
        }

        tree
    }

    /// Pack the tree into (x, y) coordinates using a contour-based scheme.
    /// Returns updated positions for all cells. Groups are packed as blocks
    /// with their sub-tree determining intra-group layout; mirror partners
    /// are placed symmetrically around the group's axis.
    pub fn pack(&self, cold: &PlaceCold, hot: &PlaceHot) -> Vec<(f32, f32)> {
        let n = cold.hw.len();
        let mut pos = vec![(0.0f32, 0.0f32); n];

        // Simple left-to-right contour packing for the top-level tree
        let mut cursor_x = 0.0f32;
        let mut node = self.root;
        while let Some(ni) = node {
            let nd = &self.nodes[ni as usize];
            match nd.kind {
                AsfNodeKind::Leaf => {
                    let ci = nd.id as usize;
                    let hw = eff_hw(hot, cold, ci);
                    let hh = eff_hh(hot, cold, ci);
                    pos[ci] = (cursor_x + hw, hh);
                    cursor_x += 2.0 * hw;
                }
                AsfNodeKind::Hierarchy { group } => {
                    let gi = group as usize;
                    if gi < cold.sym.groups.len() {
                        // Pack representatives left-to-right within group
                        let mut group_x = cursor_x;
                        let mut sub = self.sub_roots.get(gi).copied().flatten();
                        while let Some(si) = sub {
                            let sn = &self.nodes[si as usize];
                            let ci = sn.id as usize;
                            let hw = eff_hw(hot, cold, ci);
                            pos[ci] = (group_x + hw, eff_hh(hot, cold, ci));
                            group_x += 2.0 * hw;
                            // Place mirror partner symmetrically
                            let p = cold.sym.partner[ci];
                            if p != ci as u32 && p != NONE {
                                let axis = (cursor_x + group_x) / 2.0;
                                pos[p as usize] = (2.0 * axis - pos[ci].0, pos[ci].1);
                            }
                            sub = sn.right;
                        }
                        // Account for mirrored half
                        let group_w = (group_x - cursor_x) * 2.0;
                        cursor_x += group_w.max(group_x - cursor_x);
                    }
                }
            }
            node = nd.right;
        }
        pos
    }

    /// Swap two top-level nodes (SA move for group reordering).
    pub fn swap_top(&mut self, a: u32, b: u32) {
        if a == b || a as usize >= self.nodes.len() || b as usize >= self.nodes.len() {
            return;
        }
        let (ai, bi) = (a as usize, b as usize);
        let a_id = self.nodes[ai].id;
        let a_kind = self.nodes[ai].kind;
        self.nodes[ai].id = self.nodes[bi].id;
        self.nodes[ai].kind = self.nodes[bi].kind;
        self.nodes[bi].id = a_id;
        self.nodes[bi].kind = a_kind;
    }

    /// Number of top-level nodes.
    pub fn top_count(&self) -> usize {
        let mut count = 0;
        let mut node = self.root;
        while let Some(ni) = node {
            count += 1;
            node = self.nodes[ni as usize].right;
        }
        count
    }
}

// ponytail: AsfTree.pack() is the contour-based initial layout; full whitespace
// recovery needs contour-node splicing — add when area is the binding constraint.
// Group-local SA ops (intra-group reshape/reorder) add when >8-pair groups appear.

// ═══════════════════════════════════════════════════════════════════════
//  Constraint-graph compaction (post-SA whitespace recovery)
// ═══════════════════════════════════════════════════════════════════════

/// 1D constraint-graph compaction, x then y. Symmetry-safe: cells linked by
/// symmetry groups, common-centroid groups, or active direct-connects move as
/// rigid clusters, so every intra-cluster relation (mirror, centroid, abut)
/// is preserved exactly. Inter-cluster min gaps come from `required_gap` +
/// `pushes` (same sources as HardGaps). Gap semantics match `cur_gap`:
/// max(gx, gy) >= req, so an axis constraint only binds when the other axis
/// projection does not already clear the requirement.
///
/// Shifts are snapped to `cold.grid`. Symmetry axes move with their cluster.
/// Clusters containing a fixed-axis group are frozen in x.
///
/// Returns the compacted bounding box (minx, miny, maxx, maxy) over the
/// margin-inflated cell extents.
/// ponytail: O(clusters^2 * members) sweep — interval tree if >>800 cells.
pub fn compact_placement(
    hot: &mut PlaceHot,
    cold: &PlaceCold,
    slack: f32,
) -> (f32, f32, f32, f32) {
    let n = cold.hw.len();
    if n == 0 {
        return (0.0, 0.0, 0.0, 0.0);
    }

    // --- union-find over rigid links ---
    let mut uf: Vec<u32> = (0..n as u32).collect();
    fn find(uf: &mut [u32], mut i: u32) -> u32 {
        while uf[i as usize] != i {
            uf[i as usize] = uf[uf[i as usize] as usize];
            i = uf[i as usize];
        }
        i
    }
    let union = |uf: &mut [u32], a: u32, b: u32| {
        let (ra, rb) = (find(uf, a), find(uf, b));
        if ra != rb {
            uf[ra as usize] = rb;
        }
    };
    for g in &cold.sym.groups {
        for w in g.windows(2) {
            union(&mut uf, w[0], w[1]);
        }
    }
    for i in 0..cold.cc_count() {
        let (a, b) = cold.cc_group(i);
        for w in a.windows(2) {
            union(&mut uf, w[0], w[1]);
        }
        for w in b.windows(2) {
            union(&mut uf, w[0], w[1]);
        }
        if let (Some(&fa), Some(&fb)) = (a.first(), b.first()) {
            union(&mut uf, fa, fb);
        }
    }
    // Alignment is one-dimensional (same x for a vertical straight net, same
    // y for a horizontal one). Welding aligned cells here made that relation
    // rigid in both dimensions and frequently connected an entire circuit by
    // transitivity. SA still optimizes the alignment term; compaction is free
    // to remove whitespace in the unconstrained dimension.
    // Direct-connect pairs are rigid clusters once a qualified transform is
    // active; compaction must preserve the zero-length connection.
    for r in &cold.abutments {
        if legal_abutment_current(cold, hot, r.a as usize, r.b as usize) {
            union(&mut uf, r.a, r.b);
        }
    }
    // DTI pairs: pairs at-or-near abutment stay welded (SA legality already
    // walked them out of the forbidden band; squeezing must not re-enter it);
    // far pairs get a d_dti lower bound in `req` below instead of rigidity —
    // rigid-union froze whole circuits solid (every N/P pair is DTI'd),
    // making compaction a no-op. Split at the band midpoint: welding a far
    // pair wastes area, flooring a near pair explodes the die to d_dti.
    // A pair whose isolation requirement exceeds s_max can't legally abut —
    // welding it would freeze an isolation violation; it must take the far
    // branch (d_dti floor) instead. Welds are transitive, so also refuse a
    // weld when any cross-cluster pair it would merge sits below its
    // required gap (e.g. mp1–mn1–mp2–mn2 chains freezing a far iso pair).
    let n_u32 = n as u32;
    for &(a, b, min_gap, max_gap) in &cold.dti_zones {
        if cur_gap(hot, cold, a as usize, b as usize) > (min_gap + max_gap) * 0.5
            || required_gap(cold, a, b) > min_gap
        {
            continue;
        }
        let root_of: Vec<u32> = (0..n_u32).map(|i| find(&mut uf, i)).collect();
        let (ra, rb) = (root_of[a as usize], root_of[b as usize]);
        if ra == rb {
            continue;
        }
        let bad = (0..n as usize).filter(|&i| root_of[i] == ra).any(|i| {
            (0..n as usize).filter(|&j| root_of[j] == rb).any(|j| {
                let r = required_gap(cold, i as u32, j as u32);
                r > 0.0 && cur_gap(hot, cold, i, j) < r
            })
        });
        if !bad {
            union(&mut uf, a, b);
        }
    }

    // --- clusters ---
    let mut cluster_of = vec![0u32; n];
    let mut clusters: Vec<Vec<u32>> = Vec::new();
    {
        let mut root_to_cluster: Vec<u32> = vec![u32::MAX; n];
        for i in 0..n as u32 {
            let r = find(&mut uf, i) as usize;
            if root_to_cluster[r] == u32::MAX {
                root_to_cluster[r] = clusters.len() as u32;
                clusters.push(Vec::new());
            }
            cluster_of[i as usize] = root_to_cluster[r];
            clusters[root_to_cluster[r] as usize].push(i);
        }
    }

    // Fixed-axis groups freeze their cluster in x.
    let mut x_frozen = vec![false; clusters.len()];
    for (gi, &fixed) in cold.sym.fixed.iter().enumerate() {
        if fixed {
            if let Some(&m) = cold.sym.groups[gi].first() {
                x_frozen[cluster_of[m as usize] as usize] = true;
            }
        }
    }

    // Required min gap incl. sparse push rules (same inputs as HardGaps),
    // plus routing slack between clusters.
    let req = |a: u32, b: u32| -> f32 {
        let mut g = required_gap(cold, a, b);
        for p in &cold.pushes {
            if (p.a == a && p.b == b) || (p.a == b && p.b == a) {
                g = g.max(p.gap_nm);
            }
        }
        // Far-state DTI pairs must not be squeezed into the forbidden band.
        // (Abutting pairs share a cluster and never reach this query.)
        // +50nm: post-compaction mirror-snap/reconcile nudge positions by a
        // few nm — sitting exactly at d_dti flips to a violation.
        for &(da, db, _, max_gap) in &cold.dti_zones {
            if (da == a && db == b) || (da == b && db == a) {
                g = g.max(max_gap + 50.0);
            }
        }
        g + slack
    };

    let grid = cold.grid.max(1.0);

    // Mirror partners share y by definition, so their x-distance from the
    // axis must carry the complete spacing requirement. SA can hand
    // compaction a symmetric-but-overlapping pair; moving both partners
    // outward by the same amount preserves the mirror equation exactly.
    for (gi, members) in cold.sym.groups.iter().enumerate() {
        let axis = hot.axis[gi];
        for &m in members {
            let p = cold.sym.partner[m as usize];
            if p == NONE || p <= m {
                continue;
            }
            let (mi, pi) = (m as usize, p as usize);
            let min_distance = (eff_hw(hot, cold, mi)
                + eff_hw(hot, cold, pi)
                + req(m, p))
                * 0.5;
            let distance = (hot.x[mi] - axis).abs();
            if distance >= min_distance {
                continue;
            }
            let distance = (min_distance / grid).ceil() * grid;
            let m_is_left = hot.x[mi] < axis || (hot.x[mi] == axis && m < p);
            hot.x[mi] = if m_is_left {
                axis - distance
            } else {
                axis + distance
            };
            hot.x[pi] = 2.0 * axis - hot.x[mi];
        }
    }

    // --- intra-cluster y-pack for pure symmetry-group clusters ---
    // The rounds below move clusters rigidly, so vertical whitespace SA left
    // INSIDE a welded group (often the whole circuit) is otherwise permanent.
    // Mirror pairs share dy (x untouched) — symmetry survives exactly.
    for members in cold.sym.groups.iter() {
        let Some(&m0) = members.first() else { continue };
        let ci = cluster_of[m0 as usize] as usize;
        // merged with cc/align/DTI-weld cells outside the group — geometry
        // beyond the mirror relation may be load-bearing; leave rigid
        if clusters[ci].len() != members.len() {
            continue;
        }
        // an abutting DTI pair inside the group must stay welded
        let abutting = cold.dti_zones.iter().any(|&(a, b, min_g, max_g)| {
            cluster_of[a as usize] as usize == ci
                && cluster_of[b as usize] as usize == ci
                && cur_gap(hot, cold, a as usize, b as usize) <= (min_g + max_g) * 0.5
        });
        if abutting {
            continue;
        }
        // Row units: (m, partner) share y; self-symmetric cells solo.
        let mut units: Vec<(u32, u32)> = members
            .iter()
            .filter_map(|&m| {
                let p = cold.sym.partner[m as usize];
                (p == NONE || p >= m).then_some((m, if p == NONE { m } else { p }))
            })
            .collect();
        units.sort_by(|a, b| {
            let lead = |u: &(u32, u32)| {
                let (m, p) = (u.0 as usize, u.1 as usize);
                (hot.y[m] - eff_hh(hot, cold, m)).min(hot.y[p] - eff_hh(hot, cold, p))
            };
            lead(a).total_cmp(&lead(b))
        });
        let mut packed: Vec<u32> = Vec::new();
        for &(m, p) in &units {
            let cells: &[u32] = if m == p { &[m][..] } else { &[m, p][..] };
            let mut lb = f32::NEG_INFINITY;
            for &i in cells {
                let ii = i as usize;
                lb = lb.max(eff_hh(hot, cold, ii) - hot.y[ii]);
                for &j in &packed {
                    let jj = j as usize;
                    if !layers_conflict(cold, ii, jj) {
                        continue;
                    }
                    let r = req(i, j);
                    let gx = (hot.x[ii] - hot.x[jj]).abs()
                        - (eff_hw(hot, cold, ii) + eff_hw(hot, cold, jj));
                    if gx >= r {
                        continue;
                    }
                    // y is the only free axis here — x is fixed by the mirror
                    lb = lb.max(
                        hot.y[jj] + eff_hh(hot, cold, jj) + r + eff_hh(hot, cold, ii)
                            - hot.y[ii],
                    );
                }
            }
            if lb > f32::NEG_INFINITY {
                let shift = (lb / grid).ceil() * grid;
                if shift != 0.0 {
                    for &i in cells {
                        hot.y[i as usize] += shift;
                    }
                }
            }
            packed.extend_from_slice(cells);
        }
    }

    // x then y pass, iterated to a fixpoint: a pass can change which axis
    // separates a pair, leaving it unenforced in a single x+y round.
    // ponytail: 4 rounds bounds the loop; unresolved overlap after that is
    // reported by total_overlap_eff downstream.
    for round in 0..4 {
    let mut moved = false;
    for horizontal in [true, false] {
        // Sort clusters by their leading edge on this axis.
        let lead = |c: &[u32], hot: &PlaceHot| -> f32 {
            c.iter()
                .map(|&i| {
                    let i = i as usize;
                    if horizontal {
                        hot.x[i] - eff_hw(hot, cold, i)
                    } else {
                        hot.y[i] - eff_hh(hot, cold, i)
                    }
                })
                .fold(f32::INFINITY, f32::min)
        };
        let mut order: Vec<u32> = (0..clusters.len() as u32).collect();
        order.sort_by(|&a, &b| {
            lead(&clusters[a as usize], hot).total_cmp(&lead(&clusters[b as usize], hot))
        });

        let mut placed: Vec<u32> = Vec::with_capacity(clusters.len());
        for &ci in &order {
            if horizontal && x_frozen[ci as usize] {
                placed.push(ci);
                continue;
            }
            // Lower bound on this cluster's shift: every member must clear
            // every member of every already-placed cluster on this axis
            // whenever the OTHER axis projection does not clear the gap.
            let mut lb = f32::NEG_INFINITY;
            for &i in &clusters[ci as usize] {
                let ii = i as usize;
                let (pi, si, po, so) = if horizontal {
                    (hot.x[ii], eff_hw(hot, cold, ii), hot.y[ii], eff_hh(hot, cold, ii))
                } else {
                    (hot.y[ii], eff_hh(hot, cold, ii), hot.x[ii], eff_hw(hot, cold, ii))
                };
                // die/origin bound: leading edge stays >= 0
                lb = lb.max(si - pi);
                for &cj in &placed {
                    for &j in &clusters[cj as usize] {
                        let jj = j as usize;
                        if !layers_conflict(cold, ii, jj) {
                            continue;
                        }
                        let r = req(i, j);
                        let (qj, tj, qo, to) = if horizontal {
                            (hot.x[jj], eff_hw(hot, cold, jj), hot.y[jj], eff_hh(hot, cold, jj))
                        } else {
                            (hot.y[jj], eff_hh(hot, cold, jj), hot.x[jj], eff_hw(hot, cold, jj))
                        };
                        // other-axis projection already clears the gap -> no
                        // constraint on this axis (max(gx,gy) semantics)
                        let other_gap = (po - qo).abs() - (so + to);
                        if other_gap >= r {
                            continue;
                        }
                        // Direction assignment: enforce the gap on the pair's
                        // separating axis (the one with the larger current
                        // gap), so a small y deficit is fixed by the y pass
                        // instead of a full-cell-width x shove. Ties go to the
                        // y pass. Enforced against EVERY placed cluster — a
                        // leading-side-only check let clusters land on top of
                        // previously shifted ones.
                        let pass_gap = (pi - qj).abs() - (si + tj);
                        let other_separates =
                            if horizontal { other_gap >= pass_gap } else { other_gap > pass_gap };
                        if other_separates {
                            continue;
                        }
                        lb = lb.max((qj + tj + r + si) - pi);
                    }
                }
            }
            if lb > f32::NEG_INFINITY {
                // shift left (or up to lb if current position violates);
                // snap up to grid so the bound stays satisfied
                let shift = (lb / grid).ceil() * grid;
                if shift < 0.0 || shift > 0.0 {
                    moved = true;
                    for &i in &clusters[ci as usize] {
                        let ii = i as usize;
                        if horizontal {
                            hot.x[ii] += shift;
                        } else {
                            hot.y[ii] += shift;
                        }
                    }
                    if horizontal {
                        // move symmetry axes rigidly with their cluster
                        for (gi, members) in cold.sym.groups.iter().enumerate() {
                            if let Some(&m) = members.first() {
                                if cluster_of[m as usize] == ci {
                                    hot.axis[gi] += shift;
                                }
                            }
                        }
                    }
                }
            }
            placed.push(ci);
        }
    }
    if round > 0 && !moved {
        break;
    }
    }

    placement_bbox(hot, cold)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlap_buckets_cover_the_largest_reshape_variant() {
        let cold = PlaceCold {
            hw: vec![100.0, 100.0],
            hh: vec![100.0, 100.0],
            variants: vec![vec![(900.0, 100.0)], vec![(900.0, 100.0)]],
            die: (5_000.0, 5_000.0),
            ..Default::default()
        };
        let buckets = Buckets::build(&cold, &[100.0, 1_600.0], &[100.0, 100.0]);
        let mut nearby = Vec::new();
        buckets.near(100.0, 100.0, &mut nearby);

        assert!(buckets.size >= 1_800.0);
        assert!(nearby.contains(&1));
    }
}
