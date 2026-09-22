//! Drawn geometry → gdsverify's `GeometryStore` + `Provenance`. A Philis
//! `LayerId` is the deck's own layer id, so shapes feed the store directly.

use gdsverify::geom::ops::Point;
use gdsverify::geom::{Dbu, GeometryStore, GeometryStoreBuilder, LayerId as GvLayerId};
use gdsverify::ingest::deck::Deck;
use gdsverify::ingest::layout::derive_layers_into;
use gdsverify::ingest::provenance::PathTable;
use gdsverify::ingest::{Provenance, StrTable};
use pnr_core::Shape;

/// A named pin label. The point `(x, y)` (nm) must lie on a shape of `layer`,
/// and `layer` must be one the deck's `connectivity.labels` pairs with a
/// conductor — that binding names the extracted net (and makes it a port).
#[derive(Clone, Debug)]
pub struct LabeledPin {
    pub name: String,
    /// Deck layer id (`pnr_core::LayerId.0`).
    pub layer: u16,
    pub x: i32,
    pub y: i32,
}

/// Build the store the engine reads, in gdsverify's own load order: push →
/// `finish` (sort by layer) → permute provenance → derive marker layers →
/// bind labels.
///
/// # Errors
/// A derived-layer failure, or a pin label on no shape of its conductor (fail
/// closed: a net silently losing its name would be an untraceable LVS miss).
pub fn build_store(
    shapes: &[Shape],
    pins: &[LabeledPin],
    deck: &Deck,
    strings: &mut StrTable,
) -> Result<(GeometryStore, Provenance), String> {
    let nm = |v: i32| Dbu::new_unchecked(i64::from(v));
    let mut builder = GeometryStoreBuilder::with_capacity(shapes.len(), shapes.len() * 4);
    let mut provenance = Provenance::default();
    for s in shapes {
        builder.push_rect(GvLayerId(s.layer.0), nm(s.rect.x), nm(s.rect.y), nm(s.rect.w), nm(s.rect.h));
        provenance.push(PathTable::ROOT, &[]);
    }
    for p in pins {
        let name = strings.intern(&p.name);
        provenance.place_label(Point { x: nm(p.x), y: nm(p.y) }, GvLayerId(p.layer), name);
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
