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
    units: Vec<pnr_core::Unit>,
    dummies: Vec<pnr_core::Dummy>,
    drawn: Vec<pnr_core::Drawn>,
    keepouts: Vec<pnr_core::Keepout>,
}

impl Builder {
    #[must_use]
    pub fn new(grid: i32) -> Self {
        Self { grid, shapes: Vec::new(), pins: Vec::new(), units: Vec::new(), dummies: Vec::new(), drawn: Vec::new(), keepouts: Vec::new() }
    }

    /// Draw `r` on `layer`, snapped to grid. Width/height clamp up to one grid
    /// step so a snapped-to-zero rect never trips a min-width rule.
    pub fn rect(&mut self, layer: LayerId, r: Rect) {
        let rect = self.snap(r);
        self.shapes.push(Shape { layer, rect });
    }

    /// Register a pin, snapped like [`Builder::rect`].
    /// Record one active unit (local frame, unsnapped: a centre, not a shape).
    pub fn unit(&mut self, u: pnr_core::Unit) {
        self.units.push(u);
    }

    /// Record one dummy gate drawn on a member's diffusion.
    pub fn dummy(&mut self, d: pnr_core::Dummy) {
        self.dummies.push(d);
    }

    /// Record one device drawn for LVS (`Macro::drawn`).
    pub fn drawn(&mut self, d: pnr_core::Drawn) {
        self.drawn.push(d);
    }

    /// Record a keep-out region, snapped like [`Builder::rect`].
    pub fn keepout(&mut self, r: Rect, why: pnr_core::KeepWhy) {
        let rect = self.snap(r);
        self.keepouts.push(pnr_core::Keepout { rect, why });
    }

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

    /// Cover every contact on poly with the deck's poly-contact mask (`npc`
    /// role; sky130: a poly cut must sit inside nitride poly cut, 100 nm
    /// clear): one strip per cut row, rows closer than the mask's spacing
    /// joined. Nothing on a deck without the role.
    ///
    /// ponytail: a row's strip spans all its cuts; a row that must skip a
    /// gap (a foreign contact between) would need splitting.
    pub fn cover_poly_cuts(&mut self, process: &dyn Process) {
        let (Some(npc), Some(poly), Some(licon)) = (process.layer("npc"), process.layer("poly"), process.layer("licon")) else {
            return;
        };
        if npc == poly {
            return;
        }
        let enc = process.enclosure("npc", "licon").unwrap_or(0);
        let space = process.space("npc").unwrap_or(0);
        let wmin = process.width("npc").unwrap_or(0);
        let inside = |c: &Rect, p: &Rect| c.x >= p.x && c.y >= p.y && c.x + c.w <= p.x + p.w && c.y + c.h <= p.y + p.h;
        let polys: Vec<Rect> = self.shapes.iter().filter(|s| s.layer == poly).map(|s| s.rect).collect();
        let mut rows: Vec<Rect> = Vec::new();
        for c in self.shapes.iter().filter(|s| s.layer == licon && polys.iter().any(|p| inside(&s.rect, p))) {
            let r = Rect { x: c.rect.x - enc, y: c.rect.y - enc, w: c.rect.w + 2 * enc, h: c.rect.h + 2 * enc };
            match rows.iter_mut().find(|q| q.y == r.y && q.h == r.h) {
                Some(q) => *q = hull(*q, r),
                None => rows.push(r),
            }
        }
        // Join strips closer than the mask spacing (one merged figure).
        let near = |a: &Rect, b: &Rect| {
            let dx = (b.x - (a.x + a.w)).max(a.x - (b.x + b.w));
            let dy = (b.y - (a.y + a.h)).max(a.y - (b.y + b.h));
            dx.max(dy) < space
        };
        'again: loop {
            for i in 0..rows.len() {
                for j in i + 1..rows.len() {
                    if near(&rows[i], &rows[j]) {
                        rows[i] = hull(rows[i], rows[j]);
                        rows.swap_remove(j);
                        continue 'again;
                    }
                }
            }
            break;
        }
        for mut r in rows {
            if r.h < wmin {
                r.y -= (wmin - r.h) / 2;
                r.h = wmin;
            }
            if r.w < wmin {
                r.x -= (wmin - r.w) / 2;
                r.w = wmin;
            }
            self.rect(npc, r);
        }
    }

    /// The bbox corner is a multiple of two grid steps (the cut lattice) and
    /// its extents of four, so the half-extent the placer stamps at
    /// (`centre - bbox.w / 2`) is on the cut lattice too (H01-26).
    #[must_use]
    pub fn finish(mut self) -> Macro {
        // Exact duplicates (rings sharing a band draw its cuts twice) are one
        // shape; a checker would read two coincident cuts as zero spacing.
        let mut seen = std::collections::HashSet::new();
        self.shapes.retain(|s| seen.insert((s.layer.0, s.rect.x, s.rect.y, s.rect.w, s.rect.h)));
        let tight = bbox_of(&self.shapes);
        let step = 2 * self.grid.max(1);
        let ext = 2 * step;
        let x = tight.x.div_euclid(step) * step;
        let y = tight.y.div_euclid(step) * step;
        let bbox = Rect {
            x,
            y,
            w: (tight.x + tight.w - x + ext - 1) / ext * ext,
            h: (tight.y + tight.h - y + ext - 1) / ext * ext,
        };
        Macro { shapes: self.shapes, pins: self.pins, bbox, units: self.units, dummies: self.dummies, drawn: self.drawn, keepouts: self.keepouts, ..Default::default() }
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
pub(crate) fn snap_to_grid(value: i32, grid: i32) -> i32 {
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

/// A cell dimension the deck states: its value under the legacy sidecar
/// `key`, never under what the deck's rules require (a sidecar key can only
/// raise it). 0 when neither states it.
#[must_use]
pub fn dim(process: &dyn Process, key: &str) -> i32 {
    let q = |v: Option<i32>| v.unwrap_or(0);
    let deck = match key {
        "contact" => q(process.width("licon")),
        "mcon_size" => q(process.width("mcon")),
        "m1_enc" => q(process.enclosure("met1", "mcon")).max(q(process.endcap("met1", "mcon"))),
        "met1_space" => q(process.space("met1")),
        "poly_ext" => q(process.extension("poly", "diff")),
        "poly_min_width" | "min_gate_l" => q(process.width("poly")),
        "min_finger_width" => q(process.width("diff")),
        "nwell_diff_enc" => q(process.enclosure("nwell", "diff")),
        "nwell_min_width" => q(process.width("nwell")),
        "via_spacing" => q(process.space("via1")),
        "via_enclosure" => q(process.enclosure("met1", "via1")).max(q(process.enclosure("met2", "via1"))),
        _ => 0,
    };
    process.rule(key, 0).max(deck)
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
    fn extents_are_twice_the_cut_lattice() {
        let mut b = Builder::new(5);
        b.rect(LayerId(0), Rect { x: 3, y: 7, w: 1235, h: 41 });
        let m = b.finish();
        assert_eq!(m.bbox.x % 10, 0);
        assert_eq!(m.bbox.y % 10, 0);
        assert_eq!(m.bbox.w % 20, 0);
        assert_eq!(m.bbox.h % 20, 0);
        let s = m.shapes[0].rect;
        assert!(m.bbox.x <= s.x && m.bbox.y <= s.y);
        assert!(m.bbox.x + m.bbox.w >= s.x + s.w && m.bbox.y + m.bbox.h >= s.y + s.h);
    }
}

fn hull(a: Rect, b: Rect) -> Rect {
    let (x0, y0) = (a.x.min(b.x), a.y.min(b.y));
    Rect { x: x0, y: y0, w: (a.x + a.w).max(b.x + b.w) - x0, h: (a.y + a.h).max(b.y + b.h) - y0 }
}
