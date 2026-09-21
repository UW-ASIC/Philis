//! Drawn geometry → gdsverify's `GeometryStore` + `Provenance`. The one place
//! Philis geometry becomes what the verification engine reads.
//!
//! Every check takes `&[pnr_core::Shape]` (a `Macro`'s shapes, or the flattened
//! shapes of a `Routes`) plus the pin labels, and runs against a store built
//! here. A Philis [`pnr_core::LayerId`] is the deck's own layer id (see
//! `Pdk::gv_layer`), so it feeds the store directly.
//!
//! Build order mirrors gdsverify's own `read_layout`/`load_into`:
//! push (arrival order) → `finish` (sort by layer, yielding the permutation) →
//! `Provenance::permute` → `derive_layers_into` (materialise the deck's derived
//! marker layers) → `resolve_labels` (bind each pin's point to the conductor
//! polygon under it).

use gdsverify::geom::ops::Point;
use gdsverify::geom::{GeometryStore, GeometryStoreBuilder, LayerId as GvLayerId};
use gdsverify::ingest::deck::Deck;
use gdsverify::ingest::layout::derive_layers_into;
use gdsverify::ingest::provenance::PathTable;
use gdsverify::ingest::{Provenance, StrTable};
use gdsverify::geom::Dbu;
use pnr_core::Shape;

/// A named pin: the label a schematic net leaves on the drawn geometry. The
/// point `(x, y)` (nm) must lie on a shape of `layer`, and `layer` must be one
/// the deck's `connectivity.labels` pairs with a conductor — that binding is
/// how the extracted net gets its name (and becomes an LVS port).
#[derive(Clone, Debug)]
pub struct LabeledPin {
    pub name: String,
    /// Deck layer id (`pnr_core::LayerId.0`).
    pub layer: u16,
    pub x: i32,
    pub y: i32,
}

/// Build the store + provenance the engine reads from drawn shapes and pins.
///
/// # Errors
/// A derived-layer boolean failure, or a pin label whose point lies on no shape
/// of the conductor its layer names (fail closed: a net silently losing its
/// name would read as an LVS port mismatch nobody can trace).
pub fn build_store(
    shapes: &[Shape],
    pins: &[LabeledPin],
    deck: &Deck,
    strings: &mut StrTable,
) -> Result<(GeometryStore, Provenance), String> {
    let mut builder = GeometryStoreBuilder::with_capacity(shapes.len(), shapes.len() * 4);
    let mut provenance = Provenance::default();
    for s in shapes {
        builder.push_rect(
            GvLayerId(s.layer.0),
            Dbu::new_unchecked(i64::from(s.rect.x)),
            Dbu::new_unchecked(i64::from(s.rect.y)),
            Dbu::new_unchecked(i64::from(s.rect.w)),
            Dbu::new_unchecked(i64::from(s.rect.h)),
        );
        provenance.push(PathTable::ROOT, &[]);
    }
    // Placed labels are not keyed by PolyId, so they may go down before the
    // permute; binding happens in `resolve_labels` against the sorted store.
    for p in pins {
        let name = strings.intern(&p.name);
        provenance.place_label(
            Point {
                x: Dbu::new_unchecked(i64::from(p.x)),
                y: Dbu::new_unchecked(i64::from(p.y)),
            },
            GvLayerId(p.layer),
            name,
        );
    }

    let (mut store, permutation) = builder.finish(deck.layers.len());
    provenance.permute(&permutation);
    derive_layers_into(&mut store, &mut provenance, deck.layers.derived())
        .map_err(|e| format!("derived layers: {e}"))?;
    provenance
        .resolve_labels(&store, &deck.connectivity)
        .map_err(|e| format!("pin label: {e}"))?;
    Ok((store, provenance))
}
