//! The drawing surface shared by every generator (and by `macro_master`), plus
//! the sizing/pin helpers the generators have in common.

use analog::cell::Unitization;
use analog::Constraints;
use pnr_core::{DeviceGroup, LayerId, Macro, NetId, Pin, Process, Rect, Shape};

/// Accumulates grid-snapped rectangles and pins into a [`Macro`].
pub struct Builder {
    grid: i32,
    shapes: Vec<Shape>,
    pins: Vec<Pin>,
}

impl Builder {
    #[must_use]
    pub fn new(grid: i32) -> Self {
        Self { grid, shapes: Vec::new(), pins: Vec::new() }
    }

    /// Draw `r` on `layer`, snapped to grid. Width/height clamp up to one grid
    /// step so a snapped-to-zero rect never trips a min-width rule.
    pub fn rect(&mut self, layer: LayerId, r: Rect) {
        let rect = self.snap(r);
        self.shapes.push(Shape { layer, rect });
    }

    /// Register a pin, snapped like [`Builder::rect`].
    pub fn pin(&mut self, mut pin: Pin) {
        pin.at = self.snap(pin.at);
        self.pins.push(pin);
    }

    fn snap(&self, r: Rect) -> Rect {
        let g = self.grid.max(1);
        Rect {
            x: snap_to_grid(r.x, self.grid),
            y: snap_to_grid(r.y, self.grid),
            w: snap_to_grid(r.w, self.grid).max(g),
            h: snap_to_grid(r.h, self.grid).max(g),
        }
    }

    /// The finished macro. The bbox corner and extents are whole multiples of
    /// two grid steps: the placer stamps a cell at `centre - bbox.w / 2`
    /// (on-grid only if the half-extent is), and a placement that keeps the
    /// corner on the cut lattice keeps every cut on it.
    #[must_use]
    pub fn finish(self) -> Macro {
        let tight = bbox_of(&self.shapes);
        let step = 2 * self.grid.max(1);
        let x = tight.x.div_euclid(step) * step;
        let y = tight.y.div_euclid(step) * step;
        let bbox = Rect {
            x,
            y,
            w: (tight.x + tight.w - x + step - 1) / step * step,
            h: (tight.y + tight.h - y + step - 1) / step * step,
        };
        Macro { shapes: self.shapes, pins: self.pins, bbox }
    }
}

/// The lattice cut positions snap to: two grid steps. magic grows a cut that
/// is off this lattice by a grid step, which then reads as missing enclosure.
#[must_use]
pub fn cut_lattice(process: &dyn Process) -> i32 {
    2 * process.grid().max(1)
}

/// Round `v` down onto the cut lattice `lat`.
#[must_use]
pub fn snap_cut(v: i32, lat: i32) -> i32 {
    v.div_euclid(lat) * lat
}

fn bbox_of(shapes: &[Shape]) -> Rect {
    let Some(first) = shapes.first() else {
        return Rect { x: 0, y: 0, w: 0, h: 0 };
    };
    let (mut x0, mut y0) = (first.rect.x, first.rect.y);
    let (mut x1, mut y1) = (x0 + first.rect.w, y0 + first.rect.h);
    for s in shapes {
        x0 = x0.min(s.rect.x);
        y0 = y0.min(s.rect.y);
        x1 = x1.max(s.rect.x + s.rect.w);
        y1 = y1.max(s.rect.y + s.rect.h);
    }
    Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
}

/// Round half away from zero to a multiple of `grid` (`grid <= 0`: identity).
fn snap_to_grid(value: i32, grid: i32) -> i32 {
    if grid <= 0 {
        return value;
    }
    let rem = value % grid;
    if rem.abs() >= (grid + 1) / 2 {
        value - rem + grid * value.signum()
    } else {
        value - rem
    }
}

/// Resolve a layer role the cell cannot be correct without.
///
/// # Panics
/// If the deck does not declare `role` (`Pdk::validate` rejects such decks, so
/// reaching here is a generator/deck bug, never a process variation).
#[must_use]
pub fn req(process: &dyn Process, role: &str) -> LayerId {
    process.layer(role).unwrap_or_else(|| {
        panic!("PDK declares no layer for mandatory role {role:?}; add it to the deck's cell.layers section")
    })
}

/// A pin `d{di}:{term}` over `at`. The net is a placeholder, distinct per
/// `(di, term)`, that the caller rebinds by pin name.
#[must_use]
pub fn pin(di: usize, term: &str, at: Rect, layer: LayerId) -> Pin {
    const TERMS: [&str; 7] = ["G", "S", "D", "B", "P", "N", "C"];
    let t = TERMS.iter().position(|&x| x == term).unwrap_or(TERMS.len());
    Pin { name: format!("d{di}:{term}"), net: NetId((di * 8 + t) as u16), at, layer }
}

/// Resolved sizing for the devices in a group.
pub struct Sizing {
    /// Unit finger/segment width, nm.
    pub unit_w: i32,
    /// Unit finger/segment length, nm.
    pub unit_l: i32,
    /// Finger/segment count per group member.
    pub dev_nf: Vec<u16>,
}

/// The unitization covering every device of `group` (subset match), if any.
#[must_use]
pub fn unitization<'a>(group: &DeviceGroup, c: &'a Constraints) -> Option<&'a Unitization> {
    if group.devices.is_empty() {
        return None;
    }
    c.unitization.iter().find(|u| group.devices.iter().all(|d| u.devices.contains(d)))
}

/// Group sizing from the covering unitization, else one finger at
/// `def_w`×`def_l` per device.
#[must_use]
pub fn sizing(group: &DeviceGroup, c: &Constraints, def_w: i32, def_l: i32) -> Sizing {
    let n = group.devices.len().max(1);
    let Some(u) = unitization(group, c) else {
        return Sizing { unit_w: def_w, unit_l: def_l, dev_nf: vec![1; n] };
    };
    // Each member reads its own slot of the (possibly larger) unitization.
    let mut dev_nf: Vec<u16> = group
        .devices
        .iter()
        .map(|d| {
            let slot = u.devices.iter().position(|x| x == d);
            slot.and_then(|i| u.dev_nf.get(i)).copied().unwrap_or(1).max(1)
        })
        .collect();
    if dev_nf.is_empty() {
        dev_nf = vec![1; n];
    }
    Sizing {
        unit_w: if u.unit_w > 0 { u.unit_w } else { def_w },
        unit_l: if u.unit_l > 0 { u.unit_l } else { def_l },
        dev_nf,
    }
}

/// Centroid-leaning interleave for per-device counts: fill outside-in, each
/// mirror pair from the device with the most fingers left.
pub(crate) fn greedy_centroid(counts: &[usize]) -> Vec<usize> {
    let total: usize = counts.iter().sum();
    let mut remaining = counts.to_vec();
    let mut seq = vec![0usize; total];
    let most = |r: &[usize]| {
        r.iter().enumerate().filter(|(_, &n)| n > 0).max_by_key(|(_, &n)| n).map_or(0, |(i, _)| i)
    };
    let (mut lo, mut hi) = (0, total);
    while lo < hi {
        let p = most(&remaining);
        seq[lo] = p;
        remaining[p] -= 1;
        lo += 1;
        if lo < hi {
            let q = if remaining[p] > 0 { p } else { most(&remaining) };
            hi -= 1;
            seq[hi] = q;
            remaining[q] = remaining[q].saturating_sub(1);
        }
    }
    seq
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_rounds_half_away_from_zero() {
        assert_eq!(snap_to_grid(7, 5), 5);
        assert_eq!(snap_to_grid(8, 5), 10);
        assert_eq!(snap_to_grid(-8, 5), -10);
        assert_eq!(snap_to_grid(-7, 5), -5);
        assert_eq!(snap_to_grid(10, 5), 10);
    }

    #[test]
    fn greedy_centroid_uses_every_finger() {
        let seq = greedy_centroid(&[4, 2, 2]);
        assert_eq!(seq.len(), 8);
        for (d, &n) in [4, 2, 2].iter().enumerate() {
            assert_eq!(seq.iter().filter(|&&x| x == d).count(), n);
        }
    }
}
