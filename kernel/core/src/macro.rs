//! [`Macro`] and the one placement stamp that turns local drawings into placed
//! shapes.

use crate::geom::{Pin, Rect, Shape};
use crate::layout::Layout;

/// A placeable unit of drawn geometry — what `cells` draws and `macroMaster`
/// injects; downstream cannot tell them apart.
#[derive(Clone, PartialEq, Debug)]
pub struct Macro {
    pub shapes: Vec<Shape>,
    /// Routing entry points.
    pub pins: Vec<Pin>,
    /// Bounding box of `shapes`, nm.
    pub bbox: Rect,
    /// Active units (local frame), for matching. Empty for non-generated cells.
    pub units: Vec<crate::units::Unit>,
    /// Dummy gates drawn on a member's diffusion; each extracts as a device.
    pub dummies: Vec<Dummy>,
}

/// A dummy gate on member `owner`'s diffusion: gate and far side tied to the
/// member's bulk, near side its `edge` terminal (`"S"` or `"D"`), so it is an
/// off transistor that extraction still sees. The LVS reference lists one card
/// per dummy with those nets.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Dummy {
    pub owner: u8,
    pub pmos: bool,
    pub edge: &'static str,
    /// Channel width and length, nm.
    pub w: i32,
    pub l: i32,
}

/// Stamp macro `m` at device `i`'s placed position: turn by `l.orient[i]`, then
/// centre the turned bbox on `(l.x[i], l.y[i])` using `l.hw/hh` (the layout is a
/// centre + half-extent model). Turning about the bbox corner keeps the result
/// on-grid. `i` past the layout (guard rings) means already absolute: returned
/// unchanged.
#[must_use]
pub fn place_macro(m: &Macro, l: &Layout, i: usize) -> Macro {
    if i >= l.x.len() {
        return m.clone();
    }
    let o = l.orient.get(i).copied().unwrap_or_default();
    let anchor = o.apply_rect(m.bbox);
    let (ax, ay) = (l.x[i] - l.hw[i] - anchor.x, l.y[i] - l.hh[i] - anchor.y);
    let shift = |r: Rect| {
        let r = o.apply_rect(r);
        Rect { x: r.x + ax, y: r.y + ay, w: r.w, h: r.h }
    };
    Macro {
        bbox: shift(m.bbox),
        shapes: m.shapes.iter().map(|s| Shape { layer: s.layer, rect: shift(s.rect) }).collect(),
        pins: m.pins.iter().map(|p| Pin { at: shift(p.at), ..p.clone() }).collect(),
        units: m.units.clone(), // ponytail: stays local; world units come from `UnitLib::placed`,
        dummies: m.dummies.clone(),
    }
}

/// [`place_macro`] over the whole table.
#[must_use]
pub fn place_macros(macros: &[Macro], l: &Layout) -> Vec<Macro> {
    macros.iter().enumerate().map(|(i, m)| place_macro(m, l, i)).collect()
}

/// Fraction of its member terminal's DC current each pin carries, parallel
/// to `m.pins`. MOS S/D pins: fingers adjacent to the pin's region over the
/// member's fingers, from `m.units` (owner, centre, S→D `phi`). Any other
/// pin: `min(1, 2/n)` over the `n` pins sharing its name — an upper bound,
/// since each finger touches one region of each terminal (n ≤ F) and a
/// region touches at most two fingers (share ≤ 2/F ≤ 2/n).
///
/// `units` stay in the local frame through [`place_macro`], so call this on
/// the unplaced macro; pin order is the same in both.
#[must_use]
pub fn pin_shares(m: &Macro) -> Vec<f32> {
    // Fingers per pin: each finger feeds the nearest pin of its own D region
    // ahead along `phi` and of its S region behind it.
    let mut w = vec![0u32; m.pins.len()];
    for u in m.units.iter().filter(|u| u.phi != (0, 0)) {
        let d = (i32::from(u.phi.0), i32::from(u.phi.1));
        for (t, dir) in [("D", d), ("S", (-d.0, -d.1))] {
            let name = format!("d{}:{t}", u.owner);
            let nearest = m
                .pins
                .iter()
                .enumerate()
                .filter(|(_, p)| p.name == name)
                .filter_map(|(j, p)| {
                    let (dx, dy) = (p.at.x + p.at.w / 2 - u.x, p.at.y + p.at.h / 2 - u.y);
                    (dx * dir.0 + dy * dir.1 > 0).then_some((dx.abs() + dy.abs(), j))
                })
                .min();
            if let Some((_, j)) = nearest {
                w[j] += 1;
            }
        }
    }
    m.pins
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let (sum, n) = m.pins.iter().zip(&w).filter(|(q, _)| q.name == p.name).fold((0, 0), |(s, n), (_, &wj)| (s + wj, n + 1));
            if sum > 0 { w[i] as f32 / sum as f32 } else { (2.0 / n as f32).min(1.0) }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LayerId, NetId, Unit};

    fn cell(pins: &[(&str, i32)], units: &[(i32, i8)]) -> Macro {
        Macro {
            shapes: Vec::new(),
            pins: pins.iter().map(|&(n, x)| Pin { name: n.into(), net: NetId(0), at: Rect { x: x - 85, y: -85, w: 170, h: 170 }, layer: LayerId(0) }).collect(),
            bbox: Rect { x: 0, y: 0, w: 8_000, h: 1_000 },
            units: units.iter().map(|&(x, phi)| Unit { owner: 0, x, y: 0, weight: 1, phi: (phi, 0), sa: 0, sb: 0 }).collect(),
            dummies: Vec::new(),
        }
    }

    /// Four fingers S D S D S, alternating S→D: each D region feeds two
    /// fingers, the end S regions one each and the middle S two.
    #[test]
    fn pin_shares_follow_finger_adjacency() {
        let m = cell(
            &[("d0:S", 0), ("d0:D", 2_000), ("d0:S", 4_000), ("d0:D", 6_000), ("d0:S", 8_000)],
            &[(1_000, 1), (3_000, -1), (5_000, 1), (7_000, -1)],
        );
        let s = pin_shares(&m);
        assert_eq!([s[1], s[3]], [0.5, 0.5], "D");
        assert_eq!([s[0], s[2], s[4]], [0.25, 0.5, 0.25], "S");
    }

    #[test]
    fn pin_shares_without_units_bound_by_two_over_n() {
        let m = cell(&[("d0:S", 0), ("d0:S", 4_000), ("d0:S", 8_000)], &[]);
        assert_eq!(pin_shares(&m), vec![2.0 / 3.0; 3]);
    }
}
