//! [`Macro`] and the one placement stamp that turns local drawings into placed
//! shapes.

use crate::geom::{Pin, Rect, Shape};
use crate::layout::Layout;

/// A placeable unit of drawn geometry — what `cells` draws and `macroMaster`
/// injects; downstream cannot tell them apart.
#[derive(Clone, PartialEq, Debug, Default)]
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
    /// Devices this macro draws that extract differently from their schematic
    /// card (a resistor's series segments, a MIM's unit plates, a diode's or
    /// BJT's units). The LVS reference lists these for `device` instead of the
    /// schematic card.
    pub drawn: Vec<Drawn>,
    /// Regions where foreign metal changes the device; RTE decides the policy
    /// per `why`. Local frame, stamped by [`place_macro`] like `shapes`.
    pub keepouts: Vec<Keepout>,
    /// Per-variant figures (CELL-19).
    pub figures: Figures,
}

/// One extracted device member `owner` drew, as LVS will see it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Drawn {
    pub owner: u8,
    /// Filled by the caller when it binds pins (`cellgen::bind_pins`).
    pub device: Option<crate::ids::DeviceId>,
    pub kind: DrawnKind,
    /// Terminal nodes in the LVS reference's pin order: R/C/D `[P, N]`; BJT
    /// `cellgen::BJT_PINS` (the order `cellgen::reference` uses, one const so
    /// the two cannot diverge).
    pub nodes: [Node; 3],
    /// Drawn body, nm: resistor segment W×L, MIM plate W×L, diode junction
    /// W×L, emitter W×L.
    pub w: i32,
    pub l: i32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DrawnKind {
    Resistor,
    Capacitor,
    Diode,
    Npn,
    Pnp,
}

/// A member pin by terminal name, or a node internal to the macro.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Node {
    Unused,
    Pin(&'static str),
    Internal(u16),
}

/// A region foreign metal must respect, and why.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Keepout {
    pub rect: Rect,
    pub why: KeepWhy,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeepWhy {
    Gate { owner: u8 },
    ResistorBody { owner: u8 },
    CapPlate { owner: u8 },
}

/// Per-variant figures; filled by CELL-19.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Figures {}

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
        drawn: m.drawn.clone(),
        keepouts: m.keepouts.iter().map(|k| Keepout { rect: shift(k.rect), ..*k }).collect(),
        figures: m.figures,
    }
}

/// [`place_macro`] over the whole table.
#[must_use]
pub fn place_macros(macros: &[Macro], l: &Layout) -> Vec<Macro> {
    macros.iter().enumerate().map(|(i, m)| place_macro(m, l, i)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Orient;

    #[test]
    fn place_macro_moves_keepouts_and_keeps_drawn() {
        let drawn = vec![Drawn { owner: 0, device: None, kind: DrawnKind::Resistor, nodes: [Node::Pin("P"), Node::Pin("N"), Node::Unused], w: 690, l: 20_000 }];
        let m = Macro {
            bbox: Rect { x: 0, y: 0, w: 400, h: 200 },
            keepouts: vec![Keepout { rect: Rect { x: 0, y: 0, w: 100, h: 50 }, why: KeepWhy::ResistorBody { owner: 0 } }],
            drawn: drawn.clone(),
            ..Default::default()
        };
        let at = |o: Orient| Layout {
            x: vec![1000],
            y: vec![2000],
            hw: vec![200],
            hh: vec![100],
            orient: vec![o],
            variant: vec![0],
            axis: vec![],
            branch: vec![],
            groups: vec![],
            power_uw: vec![0],
            temp_mc: vec![0],
            units: Default::default(),
        };
        let p = place_macro(&m, &at(Orient::R0), 0);
        assert_eq!(p.keepouts[0].rect, Rect { x: 800, y: 1900, w: 100, h: 50 });
        assert_eq!(p.keepouts[0].why, KeepWhy::ResistorBody { owner: 0 });
        assert_eq!(p.drawn, drawn);
        let r = place_macro(&m, &at(Orient::R90), 0).keepouts[0].rect;
        assert_eq!((r.w, r.h), (50, 100));
    }
}
