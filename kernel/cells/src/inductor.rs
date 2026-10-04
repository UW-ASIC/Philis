//! Inductor generator: draws nothing. No deck recognises an inductor (LVS
//! skips them, `cellgen::reference`), and the rectangular met1 spiral this
//! used to draw anchored every turn at (0,0), so the turns merged into one
//! conductor with an unvia'd li centre tap: a short, not a coil (AC-03). An
//! empty enumeration makes `library::signoff` report `cell/undrawable`.
//!
//! ponytail: a real spiral (offset turns, crossover, keep-out) is deferred
//! (plan-03 appendix, former CELL-28).

use analog::Constraints;
use pnr_core::{DeviceGroup, Macro, Process};

use crate::builder::Builder;
use crate::Cell;

/// The inductor cell; it has no variants.
#[derive(Clone)]
pub struct Inductor;

impl Cell for Inductor {
    fn enumerate(
        _group: &DeviceGroup,
        _constraints: &Constraints,
        _process: &dyn Process,
    ) -> Vec<Self> {
        vec![]
    }

    fn draw(
        &self,
        _group: &DeviceGroup,
        _constraints: &Constraints,
        process: &dyn Process,
    ) -> Macro {
        Builder::new(process.grid()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// With no recogniser, an inductor has no construction: it enumerates
    /// nothing, so signoff reports it rather than checking a drawing.
    #[test]
    fn an_inductor_is_undrawable_without_a_recogniser() {
        use crate::testkit;
        let pdk = testkit::pdk().expect("pdks/sky130.json is checked in");
        let (group, c) = testkit::group_of(pnr_core::DeviceKind::Inductor, 1, 1, 2000, 20_000);
        assert!(Inductor::enumerate(&group, &c, &pdk).is_empty());
    }
}
