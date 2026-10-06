//! Physical units of drawn devices: what matching rules measure instead of a
//! cell's bounding box.
//!
//! A merged cell draws several schematic devices; its bbox centre and area say
//! nothing about where each device's current flows. Every generator therefore
//! records one [`Unit`] per active unit (a MOS finger): its owner, active-area
//! centre, electrical weight and signed S→D direction. [`UnitLib`] flattens
//! them for every (cell, variant) so a rule can read the chosen variant's units
//! in world coordinates from the [`Layout`] alone.

use crate::geom::{Orient, Rect};
use crate::ids::DeviceId;
use crate::layout::Layout;

/// One active unit in its macro's local frame.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Unit {
    /// Member index within the cell's group (the `d{owner}:` pin prefix).
    pub owner: u8,
    /// Active-area centre x, nm.
    pub x: i32,
    /// Active-area centre y, nm.
    pub y: i32,
    /// Electrical weight (MOS: gate area W·L, nm²). Moments weight by this,
    /// never by drawn outline.
    pub weight: i64,
    /// Unit S→D current direction, each component in `{-1, 0, 1}`.
    pub phi: (i8, i8),
    /// Channel centre to the diffusion's two ends along the current, nm
    /// (BSIM4's `SA + L/2`, `SB + L/2`: the LOD stress distances). `0` = not a
    /// MOS finger.
    pub sa: i32,
    /// Far-side counterpart of [`Unit::sa`], nm; `0` = not a MOS finger.
    pub sb: i32,
}

/// Every (cell, variant)'s units, SoA. Immutable once built; shared by `Arc`.
///
/// Layout: cells own consecutive variant *slots*; each slot owns a consecutive
/// unit range. The unit columns (`owner` … `lod`) are parallel.
#[derive(Default, Debug)]
pub struct UnitLib {
    /// `cell_of[device]` = its cell. Empty = no units known (rules fall back).
    pub cell_of: Vec<u16>,
    /// First slot of each cell, plus one sentinel; slot = `slot0[cell] +
    /// variant`, valid while `< slot0[cell + 1]`.
    slot0: Vec<u32>,
    /// Per slot plus one sentinel: unit range start (`start[slot]..start[slot + 1]`).
    start: Vec<u32>,
    /// Per slot: the macro bbox the local frame is relative to.
    bbox: Vec<Rect>,
    /// Per unit: the schematic device that owns it.
    owner: Vec<DeviceId>,
    /// Per unit: local-frame centre x, nm.
    x: Vec<i32>,
    /// Per unit: local-frame centre y, nm.
    y: Vec<i32>,
    /// Per unit: electrical weight ([`Unit::weight`]).
    weight: Vec<i64>,
    /// Per unit: local-frame S→D direction.
    phi: Vec<(i8, i8)>,
    /// Per unit: LOD stress term, 1/µm; `NaN` = unknown ([`PlacedUnit::lod`]).
    lod: Vec<f32>,
}

/// A unit placed in the world: owner is the schematic device.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PlacedUnit {
    /// Schematic device carrying this unit's current.
    pub owner: DeviceId,
    /// World centre x, nm.
    pub x: i32,
    /// World centre y, nm.
    pub y: i32,
    /// Electrical weight ([`Unit::weight`]).
    pub weight: i64,
    /// World S→D direction, each component in `{-1, 0, 1}`.
    pub phi: (i8, i8),
    /// LOD stress term `1/(SA+L/2) + 1/(SB+L/2)`, 1/µm; `NaN` = unknown.
    pub lod: f32,
}

impl UnitLib {
    /// Builds from each cell's variants (`(bbox, units)` per alternative, cell
    /// `i` being the `i`-th item of `variants`) and each cell's member devices
    /// (`members[cell][owner]`). A unit whose `owner` is not a member is
    /// dropped. A unit's LOD term is known only when both `sa` and `sb` are
    /// positive.
    ///
    /// # Panics
    /// If `variants` yields more cells than `members` has rows.
    #[must_use]
    pub fn build<'a>(
        cell_of: Vec<u16>,
        members: &[Vec<DeviceId>],
        variants: impl Iterator<Item = &'a [(Rect, &'a [Unit])]>,
    ) -> Self {
        let mut lib = UnitLib { cell_of, ..UnitLib::default() };
        for (cell, alts) in variants.enumerate() {
            lib.slot0.push(lib.bbox.len() as u32);
            for &(bbox, units) in alts {
                lib.start.push(lib.owner.len() as u32);
                lib.bbox.push(bbox);
                for u in units {
                    let Some(&d) = members[cell].get(usize::from(u.owner)) else { continue };
                    lib.owner.push(d);
                    lib.x.push(u.x);
                    lib.y.push(u.y);
                    lib.weight.push(u.weight);
                    lib.phi.push(u.phi);
                    lib.lod.push(if u.sa > 0 && u.sb > 0 { 1e3 / u.sa as f32 + 1e3 / u.sb as f32 } else { f32::NAN });
                }
            }
        }
        lib.slot0.push(lib.bbox.len() as u32);
        lib.start.push(lib.owner.len() as u32);
        lib
    }

    /// Returns `true` with no unit data: rules must fall back or report unknown.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.owner.is_empty()
    }

    /// Units of `cell`'s current variant (`l.variant[cell]`, `0` when absent),
    /// turned by `l.orient[cell]` and placed with the same stamp as
    /// [`crate::place_macro`], in world coordinates. Empty when the cell or
    /// variant is unknown to the library.
    ///
    /// # Panics
    /// If the library knows `cell` but `l.x`/`l.y`/`l.hw`/`l.hh` are shorter.
    pub fn placed(&self, l: &Layout, cell: usize) -> impl Iterator<Item = PlacedUnit> + '_ {
        let v = l.variant.get(cell).copied().unwrap_or(0) as usize;
        let slot = (cell < self.slot0.len().saturating_sub(1))
            .then(|| self.slot0[cell] as usize + v)
            .filter(|&s| s < self.slot0[cell + 1] as usize);
        let range = slot.map_or(0..0, |s| self.start[s] as usize..self.start[s + 1] as usize);
        // Same transform as `place_macro`: turn about the origin, then shift the
        // turned bbox's lower-left onto the device's placed lower-left.
        let o = l.orient.get(cell).copied().unwrap_or_default();
        let (ax, ay) = slot.map_or((0, 0), |s| {
            let anchor = o.apply_rect(self.bbox[s]);
            (l.x[cell] - l.hw[cell] - anchor.x, l.y[cell] - l.hh[cell] - anchor.y)
        });
        range.map(move |i| {
            let (x, y) = o.apply(self.x[i], self.y[i]);
            PlacedUnit { owner: self.owner[i], x: x + ax, y: y + ay, weight: self.weight[i], phi: turn_dir(o, self.phi[i]), lod: self.lod[i] }
        })
    }

    /// Every placed unit owned by `device`, in its cell's unit order. Empty
    /// when the device has no cell.
    pub fn of_device(&self, l: &Layout, device: DeviceId) -> impl Iterator<Item = PlacedUnit> + '_ {
        let cell = self.cell_of.get(device.0 as usize).map_or(usize::MAX, |&c| c as usize);
        self.placed(l, cell).filter(move |u| u.owner == device)
    }
}

/// A direction turns with the cell (a point about the origin, no shift).
fn turn_dir(o: Orient, (x, y): (i8, i8)) -> (i8, i8) {
    let (x, y) = o.apply(i32::from(x), i32::from(y));
    (x as i8, y as i8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(x: i32, orient: Orient) -> Layout {
        Layout {
            x: vec![x],
            y: vec![0],
            hw: vec![500],
            hh: vec![100],
            orient: vec![orient],
            variant: vec![0],
            axis: vec![],
            branch: vec![],
            groups: vec![],
            power_uw: vec![0],
            temp_mc: vec![0],
            units: std::sync::Arc::default(),
        }
    }

    /// One cell, one variant: a 1000×200 bbox with two fingers of devices 7, 9.
    fn lib() -> UnitLib {
        let units = [
            Unit { owner: 0, x: 100, y: 100, weight: 5, phi: (1, 0), sa: 0, sb: 0 },
            Unit { owner: 1, x: 900, y: 100, weight: 5, phi: (-1, 0), sa: 0, sb: 0 },
        ];
        let bbox = Rect { x: 0, y: 0, w: 1000, h: 200 };
        let alts = [(bbox, &units[..])];
        UnitLib::build(vec![0; 10], &[vec![DeviceId(7), DeviceId(9)]], std::iter::once(&alts[..]))
    }

    #[test]
    fn units_follow_the_cell_and_its_mirror() {
        let lib = lib();
        let l = layout(10_000, Orient::R0);
        let a: Vec<_> = lib.of_device(&l, DeviceId(7)).collect();
        assert_eq!((a[0].x, a[0].y, a[0].phi), (9_600, 0, (1, 0)));
        // Mirrored in x: the unit swaps ends and its current reverses.
        let l = layout(10_000, Orient::Mx180);
        let a: Vec<_> = lib.of_device(&l, DeviceId(7)).collect();
        assert_eq!((a[0].x, a[0].phi), (10_400, (-1, 0)));
    }

    #[test]
    fn a_missing_variant_yields_nothing_not_a_panic() {
        let lib = lib();
        let mut l = layout(0, Orient::R0);
        l.variant[0] = 3;
        assert_eq!(lib.placed(&l, 0).count(), 0);
        assert_eq!(lib.placed(&l, 5).count(), 0);
    }
}
