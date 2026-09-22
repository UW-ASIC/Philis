//! Common-centroid (placement tier).

use pnr_core::ids::{DeviceId, Target};
use pnr_core::layout::Layout;
use crate::rule::Rule;

/// Pairwise centroid pull between two representatives. Objective only.
#[derive(Clone, Copy)]
pub struct CommonCentroid {
    pub a: Target,
    pub b: Target,
}

/// Common centroid of a whole matched array: side A's **area-weighted**
/// centroid against side B's. Pairwise coincidence is a weaker condition (each
/// pair centred while the array stays lopsided).
///
/// ponytail: cost-only; `violations` is always `0`. The exact equality belongs
/// in the pattern representation (ABBA / checkerboard), not a projection.
pub struct CentroidGroup {
    pub a_side: Vec<DeviceId>,
    pub b_side: Vec<DeviceId>,
}

impl CentroidGroup {
    /// Area-weighted centroid of a device set, nm; out-of-range ids skipped.
    fn centroid(l: &Layout, side: &[DeviceId]) -> Option<(f64, f64)> {
        let (mut sx, mut sy, mut sw) = (0.0f64, 0.0f64, 0.0f64);
        for d in side {
            let i = d.0 as usize;
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

impl crate::rule::RuleBatch<Layout> for CentroidGroup {
    /// Squared centroid separation `· 1e-3`.
    fn cost(&self, l: &Layout) -> f32 {
        let (Some((ax, ay)), Some((bx, by))) = (Self::centroid(l, &self.a_side), Self::centroid(l, &self.b_side))
        else {
            return 0.0;
        };
        let (dx, dy) = ((ax - bx) as f32, (ay - by) as f32);
        (dx * dx + dy * dy) * 1e-3
    }
    fn violations(&self, _l: &Layout) -> u32 {
        0
    }
    fn kind(&self) -> &'static str {
        "CommonCentroid"
    }
    /// One condition per array.
    fn count(&self) -> usize {
        usize::from(!self.a_side.is_empty() && !self.b_side.is_empty())
    }
    /// Element-wise through `cell_of`; members sharing a cell repeat its id.
    fn retarget(&mut self, cell_of: &[u16]) {
        for d in self.a_side.iter_mut().chain(self.b_side.iter_mut()) {
            if let Some(&c) = cell_of.get(d.0 as usize) {
                *d = DeviceId(c);
            }
        }
    }
}

impl Rule for CommonCentroid {
    type On = Layout;
    /// Squared centre distance `· 1e-3`.
    fn cost(self, l: &Layout) -> f32 {
        let (ax, ay) = l.centre(self.a);
        let (bx, by) = l.centre(self.b);
        let dx = (ax - bx) as f32;
        let dy = (ay - by) as f32;
        (dx * dx + dy * dy) * 1e-3
    }
    fn retarget(self, cell_of: &[u16]) -> Self {
        Self { a: self.a.retarget(cell_of), b: self.b.retarget(cell_of) }
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
        }
    }

    fn grp() -> CentroidGroup {
        CentroidGroup {
            a_side: vec![DeviceId(0), DeviceId(2)],
            b_side: vec![DeviceId(1), DeviceId(3)],
        }
    }

    #[test]
    fn interleaved_array_has_coincident_centroids() {
        // A B B A — the canonical common-centroid order. Both sides centre on 15.
        let l = layout(&[0, 10, 20, 30], &[0, 0, 0, 0], 100);
        let g = CentroidGroup {
            a_side: vec![DeviceId(0), DeviceId(3)],
            b_side: vec![DeviceId(1), DeviceId(2)],
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
        let g = CentroidGroup { a_side: vec![DeviceId(0), DeviceId(1)], b_side: vec![DeviceId(2)] };
        assert!(g.cost(&l) > 0.0, "area weighting must move the centroid off the plain mean");

        // Put B at the area-weighted centroid instead and it vanishes.
        let (cx, _) = CentroidGroup::centroid(&l, &g.a_side).unwrap();
        l.x[2] = cx.round() as i32;
        assert!(g.cost(&l) < 1e-3, "coincident weighted centroids cost nothing");
    }

    #[test]
    fn one_condition_per_array_not_per_device() {
        let l = layout(&[0, 10, 20, 30], &[0, 0, 0, 0], 100);
        assert_eq!(grp().count(), 1, "the array carries one centroid condition");
        assert_eq!(grp().violations(&l), 0, "objective, never a legality failure");
    }
}
