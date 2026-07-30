//! Common-centroid coincidence (placement tier).
//!
//! Ported from `backend/constraints/src/placement_level/cc.rs` (`CcGroup`).

use pnr_core::ids::Target;
use pnr_core::layout::Layout;
use crate::rule::Rule;

/// **Common-centroid.** Matched groups placed symmetrically about a shared
/// centroid cancel first-order thermal and process gradients — Hastings' five
/// rules: coincident centroids, mirror symmetry, dispersion, compactness, uniform
/// orientation. ~60% lower systematic mismatch than single-axis ABBA in 2D. The
/// *pattern* (`Cc1d`/`Cc2d`) is chosen structurally in [`crate::Constraints`];
/// this rule only enforces centroid coincidence of the two sides.
///
/// - **Enforcement:** [`crate::Mode::Cost`] (was soft, priority 60).
/// - **Arity:** Device↔Device (group side A ↔ side B).
/// - **Books:** AOAL ch08/8.2.7, ch13/13.2.6 (#42); FOLD 6.6/6.6.2 (#4);
///   ALS 3.1/3.1.2 (#3); PNR_ANALOG 00/1.5 (#6).
#[derive(Clone, Copy)]
pub struct CommonCentroid {
    /// Representative device of side A (its side's centroid is derived in the loop).
    pub a: Target,
    /// Representative device of side B.
    pub b: Target,
}

/// Common-centroid over the **whole matched array** — set A against set B.
///
/// A common centroid is a property of two *sets*, not two devices: it is the A
/// devices' centroid coinciding with the B devices' centroid that cancels the
/// first-order gradient across the array. Enforcing it pair-by-pair is a weaker
/// and different condition — every pair can sit centred on its own point while
/// the array as a whole stays lopsided, leaving exactly the gradient term the
/// pattern exists to cancel.
///
/// This restores the arity the original `CcGroup { group_a, group_b }` carried
/// and the port flattened into a device pair. The centroid is **area-weighted**,
/// because a gradient integrates over area — the plain mean of centres is only
/// correct when every member is the same size.
///
/// Books: AOAL ch08/8.2.7, ch13/13.2.6 (#42); ALS 3.1/3.1.2 (Hastings' rules).
pub struct CentroidGroup {
    /// Devices forming side A of the matched array.
    pub a_side: Vec<pnr_core::ids::DeviceId>,
    /// Devices forming side B.
    pub b_side: Vec<pnr_core::ids::DeviceId>,
}

impl CentroidGroup {
    /// Area-weighted centroid of a device set, `nm`.
    fn centroid(l: &Layout, side: &[pnr_core::ids::DeviceId]) -> Option<(f64, f64)> {
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

    /// Squared separation of the two side centroids, in the engine's CC units.
    fn separation(&self, l: &Layout) -> f32 {
        let (Some((ax, ay)), Some((bx, by))) =
            (Self::centroid(l, &self.a_side), Self::centroid(l, &self.b_side))
        else {
            return 0.0;
        };
        let (dx, dy) = ((ax - bx) as f32, (ay - by) as f32);
        (dx * dx + dy * dy) * 1e-3
    }
}

impl crate::rule::RuleBatch<Layout> for CentroidGroup {
    fn cost(&self, l: &Layout) -> f32 {
        self.separation(l)
    }
    fn violations(&self, _l: &Layout) -> u32 {
        0 // objective, not legality: the concrete pattern is chosen structurally
    }
    // AUDIT (`Rule::project`, `docs/API-WISH.md`): this is the one constraint in the crate
    // that is an **exact equality run as a penalty only**, and adding a `project` here is
    // not the fix.
    //
    // PLAN §4c states it as a group-level linear equality —
    // `Σ_A a_i·(x_i − c) = Σ_B a_j·(x_j − c) = 0`, area-weighted — and says pairwise
    // enforcement is "provably weaker (it does not deliver first-order gradient
    // cancellation)". `separation` above is the right *measure*; squaring it and handing it
    // to the optimiser is exactly PLAN §4a's parking failure, since the gradient vanishes
    // as the centroids converge and on an integer grid the array stops one unit lopsided.
    // `violations` returning a constant `0` then makes that permanent — the batch is
    // registered in `cost` only (`annotator::emit::placement`), so nothing above the
    // feasibility frontier ever asks.
    //
    // A `project` would have to *choose* which devices absorb the correction, and the
    // answer is not local: the legal choices are the interleavings (ABBA / checkerboard),
    // a permutation of the array rather than a nudge of two coordinates. PLAN gives it the
    // same answer it gives symmetry — put it in the representation, where the interleaving
    // satisfies the equality by the decoding rule. Left as debt rather than
    // half-implemented, because a projection that shifted coordinates would satisfy the
    // equation while destroying the pattern that makes it mean anything.
    fn kind(&self) -> &'static str {
        "CommonCentroid"
    }
    fn count(&self) -> usize {
        // One condition per array, however many devices it holds.
        usize::from(!self.a_side.is_empty() && !self.b_side.is_empty())
    }

    /// Element-wise through `cell_of` — the sides are bare [`pnr_core::ids::DeviceId`]
    /// lists, not [`Target`]s, so the batch maps them itself. Members that collapse
    /// into one cell simply repeat the id; the area-weighted centroid then counts
    /// that macro once per member, which is the honest weight for a side that put
    /// several units inside it.
    fn retarget(&mut self, cell_of: &[u16]) {
        for d in self.a_side.iter_mut().chain(self.b_side.iter_mut()) {
            if let Some(&c) = cell_of.get(d.0 as usize) {
                *d = pnr_core::ids::DeviceId(c);
            }
        }
    }
}

impl Rule for CommonCentroid {
    type On = Layout;
    /// Squared distance between the two side centroids; `0.0` when coincident.
    ///
    /// The engine's CC term (`engine::soft_terms`): `(dx² + dy²) · 1e-3` between
    /// side centroids. With one representative device per side, each side's
    /// centroid is that device, so the two centres stand in for the two centroids.
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
