//! Drawn geometry → [`gdsverify::GeometryStore`]. The one place Philis geometry
//! becomes what the verification engine reads.
//!
//! Every check takes `&[pnr_core::Shape]` (a `Macro`'s shapes, or the flattened
//! shapes of a `Routes`) and a [`Pdk`], and runs against a store built here. A
//! Philis [`pnr_core::LayerId`] is the deck's own layer id (see `Pdk::from_deck`),
//! so it feeds the store directly.

use pnr_core::{Routes, Shape};

use crate::pdk::Pdk;

/// Build a gdsverify [`GeometryStore`](gdsverify::GeometryStore) from drawn
/// shapes. Rectangles only — Philis geometry is rectangle-decomposed.
#[must_use]
pub fn store_from_shapes(shapes: &[Shape], pdk: &Pdk) -> gdsverify::GeometryStore {
    let mut store = gdsverify::GeometryStore::new();
    for s in shapes {
        store.add_rect(
            pdk.gv_layer(s.layer),
            s.rect.x,
            s.rect.y,
            s.rect.w,
            s.rect.h,
        );
    }
    store
}

/// Flatten every net's wire shapes into one store. Routing checks run over the
/// whole drawn routing at once, exactly like DRC over a cell.
#[must_use]
pub fn store_from_routes(routes: &Routes, pdk: &Pdk) -> gdsverify::GeometryStore {
    let mut store = gdsverify::GeometryStore::new();
    for net_wires in &routes.wires {
        for s in net_wires {
            store.add_rect(
                pdk.gv_layer(s.layer),
                s.rect.x,
                s.rect.y,
                s.rect.w,
                s.rect.h,
            );
        }
    }
    store
}
