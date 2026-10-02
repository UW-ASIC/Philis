//! The PDK seam generators are written against. Layers are looked up by role
//! (`"poly"`, `"metal1"`) and rules by name, so no generator hard-codes a PDK
//! number. The real impl is `verify::Pdk`.

use crate::LayerId;

pub trait Process {
    /// Layer for a role; `None` when this process omits that (optional) layer.
    fn layer(&self, role: &str) -> Option<LayerId>;

    /// Rule value in nm, or `default` when the process does not specify it.
    fn rule(&self, name: &str, default: i32) -> i32;

    /// Fabrication grid, nm. Every emitted coordinate snaps to it.
    fn grid(&self) -> i32;

    /// Sheet resistance of a role's layer, Ω/□ (for a cut, Ω per cut);
    /// `None` when the process does not characterise it.
    fn sheet_ohm(&self, role: &str) -> Option<f32> {
        let _ = role;
        None
    }

    /// Resistance of one cut of role `cut` landing on role `onto`, Ω per cut
    /// (a deck states contact resistance per landing: sky130 licon on poly
    /// 152, on a p-tap 585); `None` when the process does not characterise it.
    fn cut_ohm(&self, cut: &str, onto: &str) -> Option<f32> {
        let _ = (cut, onto);
        None
    }

    /// Spacing a role's layer needs between its shapes, nm: the widest of its
    /// `min_spacing`, its wide-metal spacing (a strap easily passes the width
    /// threshold) and, for a cut, its array spacing (a cell's cuts easily
    /// reach the array threshold). `None` = not given.
    fn space(&self, role: &str) -> Option<i32> {
        let _ = role;
        None
    }

    /// Spacing a role's line end (a narrow edge) keeps to anything, nm;
    /// `None` = not given.
    fn eol_space(&self, role: &str) -> Option<i32> {
        let _ = role;
        None
    }

    /// A role's layer's minimum width, nm; `None` = not given.
    fn width(&self, role: &str) -> Option<i32> {
        let _ = role;
        None
    }

    /// How far `outer` must enclose `inner` on every side, nm (the widest
    /// such rule); `None` = not given.
    fn enclosure(&self, outer: &str, inner: &str) -> Option<i32> {
        let _ = (outer, inner);
        None
    }

    /// The end-cap: how far `outer` must pass `inner` on at least one side
    /// of each axis (an asymmetric enclosure), nm; `None` = not given.
    fn endcap(&self, outer: &str, inner: &str) -> Option<i32> {
        let _ = (outer, inner);
        None
    }

    /// How far `outer` must extend past `inner`'s edges (a salicide block
    /// past its poly), nm; `None` = not given.
    fn extension(&self, outer: &str, inner: &str) -> Option<i32> {
        let _ = (outer, inner);
        None
    }

    /// A role's layer's minimum polygon area, nm²; `None` = not given.
    fn area(&self, role: &str) -> Option<i64> {
        let _ = role;
        None
    }

    /// Spacing the deck requires between two roles' layers, nm (its
    /// two-layer spacing rules, widest); `None` = not given.
    fn space_between(&self, a: &str, b: &str) -> Option<i32> {
        let _ = (a, b);
        None
    }

}
