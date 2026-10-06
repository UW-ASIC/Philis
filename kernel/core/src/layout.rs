//! Placement state: what placement rules score against.

use crate::geom::Orient;
use crate::ids::{AxisId, DeviceId, Target};

/// Device-indexed SoA placement state. Rules read it and never mutate it
/// (except `Rule::project`), so scoring is pure.
///
/// Invariant: whoever writes `orient[i]` or `variant[i]` rewrites `hw[i]`/`hh[i]`
/// in the same breath, so every bbox/overlap read sees the drawn footprint.
pub struct Layout {
    /// Device centre x, nm.
    pub x: Vec<i32>,
    /// Device centre y, nm.
    pub y: Vec<i32>,
    /// Half-width, nm, after `orient`.
    pub hw: Vec<i32>,
    /// Half-height, nm, after `orient`.
    pub hh: Vec<i32>,
    /// Stamp transform per device; applied at the single stamping site.
    pub orient: Vec<Orient>,
    /// Chosen drawn variant per cell (index into `gp::VariantSpace`); a discrete
    /// search variable. `0` = first alternative.
    pub variant: Vec<u16>,
    /// Symmetry-axis x by [`AxisId`], nm.
    pub axis: Vec<i32>,
    /// Disjunctive commitments by [`crate::ids::BranchId`]; meaning is the owning
    /// rule's (for `DtiBand`: `false` = share, `true` = isolate). All-`false` is
    /// a valid start.
    pub branch: Vec<bool>,
    /// Member devices by [`crate::GroupId`].
    pub groups: Vec<Vec<DeviceId>>,
    /// Dissipation per device, µW (`0` = unknown: a sensor, not a source).
    pub power_uw: Vec<i32>,
    /// Temperature rise per device, milli-°C. Cache of
    /// [`crate::thermal::rises_mc`]; stale until [`Layout::refresh_temps`].
    pub temp_mc: Vec<i32>,
    /// Every (cell, variant)'s physical units; read through `variant`/`orient`.
    /// Empty before cells are drawn.
    pub units: std::sync::Arc<crate::units::UnitLib>,
}

impl Layout {
    /// Area of the box enclosing every cell's footprint, nm² (`0` with no
    /// cells). O(n).
    #[must_use]
    pub fn footprint_nm2(&self) -> f64 {
        let n = self.x.len();
        if n == 0 {
            return 0.0;
        }
        let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
        for i in 0..n {
            (x0, y0) = (x0.min(self.x[i] - self.hw[i]), y0.min(self.y[i] - self.hh[i]));
            (x1, y1) = (x1.max(self.x[i] + self.hw[i]), y1.max(self.y[i] + self.hh[i]));
        }
        f64::from(x1 - x0) * f64::from(y1 - y0)
    }

    /// Normalising length, nm: `sqrt(Σ cell area)` over every cell's current
    /// `hw`/`hh` (PLC-18); `1` with no cells, so callers never divide by 0.
    ///
    /// ponytail: O(n) per call, read once per rule eval; cache on `Layout` if
    /// PLC-09's T7 profile shows it. Live, not cached, so dp's reshape moves stay exact.
    #[must_use]
    pub fn l_ref(&self) -> f32 {
        let a: f64 = self.hw.iter().zip(&self.hh).map(|(&w, &h)| 4.0 * f64::from(w) * f64::from(h)).sum();
        a.sqrt().max(1.0) as f32
    }

    /// `(cx, cy, hw, hh)` of a target, nm; a group is the smallest
    /// centre/half-extent box enclosing every member's footprint (an odd-nm
    /// span rounds the box outward by up to 1 nm). O(members).
    ///
    /// # Panics
    /// On an out-of-range target or an empty group.
    #[inline]
    #[must_use]
    pub fn bbox(&self, t: Target) -> (i32, i32, i32, i32) {
        match t {
            Target::Device(d) => {
                let i = d.0 as usize;
                (self.x[i], self.y[i], self.hw[i], self.hh[i])
            }
            Target::Group(g) => {
                let members = &self.groups[g.0 as usize];
                assert!(!members.is_empty(), "group has no members");
                let (mut lo_x, mut lo_y, mut hi_x, mut hi_y) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
                for d in members {
                    let i = d.0 as usize;
                    lo_x = lo_x.min(self.x[i] - self.hw[i]);
                    lo_y = lo_y.min(self.y[i] - self.hh[i]);
                    hi_x = hi_x.max(self.x[i] + self.hw[i]);
                    hi_y = hi_y.max(self.y[i] + self.hh[i]);
                }
                ((lo_x + hi_x) / 2, (lo_y + hi_y) / 2, (hi_x - lo_x) / 2, (hi_y - lo_y) / 2)
            }
        }
    }

    /// Centre `(cx, cy)` of [`Layout::bbox`], nm.
    ///
    /// # Panics
    /// As [`Layout::bbox`].
    #[inline]
    #[must_use]
    pub fn centre(&self, t: Target) -> (i32, i32) {
        let (cx, cy, _, _) = self.bbox(t);
        (cx, cy)
    }

    /// Half-extents `(hw, hh)` of [`Layout::bbox`], nm.
    ///
    /// # Panics
    /// As [`Layout::bbox`].
    #[inline]
    #[must_use]
    pub fn extent(&self, t: Target) -> (i32, i32) {
        let (_, _, hw, hh) = self.bbox(t);
        (hw, hh)
    }

    /// Axis x; an id past the table falls back to [`Self::centre_x_estimate`]
    /// (axes are per block, and there can be more blocks than devices).
    #[inline]
    #[must_use]
    pub fn axis_x(&self, a: AxisId) -> i32 {
        self.axis.get(a.0 as usize).copied().unwrap_or_else(|| self.centre_x_estimate())
    }

    /// Mean device centre x, nm, truncated toward zero (`0` when empty).
    #[inline]
    #[must_use]
    pub fn centre_x_estimate(&self) -> i32 {
        if self.x.is_empty() {
            return 0;
        }
        (self.x.iter().map(|&v| i64::from(v)).sum::<i64>() / self.x.len() as i64) as i32
    }

    /// Debug-only structural check: every column device-length, extents
    /// non-negative, group members in range. `ctx` names the producing stage.
    /// A no-op in release builds.
    ///
    /// # Panics
    /// In debug builds, on the first violated invariant, naming `ctx`.
    #[inline]
    pub fn debug_check(&self, ctx: &str) {
        if !cfg!(debug_assertions) {
            return;
        }
        let n = self.x.len();
        for (name, len) in [
            ("y", self.y.len()),
            ("hw", self.hw.len()),
            ("hh", self.hh.len()),
            ("orient", self.orient.len()),
            ("variant", self.variant.len()),
            ("power_uw", self.power_uw.len()),
            ("temp_mc", self.temp_mc.len()),
        ] {
            assert_eq!(len, n, "{ctx}: Layout.{name} has {len} entries, x has {n}");
        }
        for i in 0..n {
            assert!(self.hw[i] >= 0 && self.hh[i] >= 0, "{ctx}: device {i} has negative extent");
        }
        for (gi, members) in self.groups.iter().enumerate() {
            for d in members {
                assert!((d.0 as usize) < n, "{ctx}: group {gi} names device {} of {n}", d.0);
            }
        }
    }

    /// Debug-only legality check: [`Layout::debug_check`], then no two device
    /// footprints overlap with positive area (abutment is legal; stacked macros
    /// become merged nets / doubled W in LVS). O(n²); a no-op in release builds.
    ///
    /// # Panics
    /// In debug builds, on the first violation, naming `ctx` and the pair.
    #[inline]
    pub fn debug_check_placed(&self, ctx: &str) {
        if !cfg!(debug_assertions) {
            return;
        }
        self.debug_check(ctx);
        for a in 0..self.x.len() {
            for b in (a + 1)..self.x.len() {
                let ox = (self.hw[a] + self.hw[b]) - (self.x[a] - self.x[b]).abs();
                let oy = (self.hh[a] + self.hh[b]) - (self.y[a] - self.y[b]).abs();
                assert!(ox <= 0 || oy <= 0, "{ctx}: devices {a} and {b} overlap {ox}x{oy} nm");
            }
        }
    }

    /// Euclidean edge-to-edge gap between two targets' boxes, nm; `0` when they
    /// touch or overlap. Symmetric.
    ///
    /// # Panics
    /// As [`Layout::bbox`].
    #[inline]
    #[must_use]
    pub fn edge_gap(&self, a: Target, b: Target) -> f32 {
        let (ax, ay, ahw, ahh) = self.bbox(a);
        let (bx, by, bhw, bhh) = self.bbox(b);
        let gx = i64::from(((ax - bx).abs() - (ahw + bhw)).max(0));
        let gy = i64::from(((ay - by).abs() - (ahh + bhh)).max(0));
        ((gx * gx + gy * gy) as f32).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cells(hw: Vec<i32>, hh: Vec<i32>) -> Layout {
        let n = hw.len();
        Layout {
            x: vec![0; n],
            y: vec![0; n],
            hw,
            hh,
            orient: vec![Orient::default(); n],
            variant: vec![0; n],
            axis: Vec::new(),
            branch: Vec::new(),
            groups: Vec::new(),
            power_uw: vec![0; n],
            temp_mc: vec![0; n],
            units: Default::default(),
        }
    }

    #[test]
    fn l_ref_is_root_of_cell_area() {
        // Areas 4·3·4 + 4·6·2 = 96 nm².
        assert_eq!(cells(vec![3, 6], vec![4, 2]).l_ref(), 96f32.sqrt());
        assert_eq!(cells(Vec::new(), Vec::new()).l_ref(), 1.0);
    }
}
