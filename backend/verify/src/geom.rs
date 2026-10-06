//! Drawn geometry → gdsverify's `GeometryStore` + `Provenance`. A Philis
//! `LayerId` is the deck's own layer id, so shapes feed the store directly.

use gdsverify::geom::ops::Point;
use gdsverify::geom::derive::merge_into;
use gdsverify::geom::rects::decompose_into;
use gdsverify::geom::view::validate_layer_into;
use gdsverify::geom::{Dbu, GeometryStore, GeometryStoreBuilder, LayerId as GvLayerId, ValidatedLayer};
use gdsverify::ingest::deck::Deck;
use gdsverify::ingest::layout::derive_layers;
use gdsverify::ingest::{Provenance, StrTable};
use pnr_core::Shape;

/// A named pin label. The point `(x, y)` (nm) must lie on a shape of `layer`,
/// the pin's drawing layer; the label goes on whichever layer the deck's
/// `connectivity.labels` binds to the conductor built from it ([`label_layer`]),
/// and that binding names the extracted net (and makes it a port).
#[derive(Clone, Debug)]
pub struct LabeledPin {
    pub name: String,
    /// Deck layer id (`pnr_core::LayerId.0`).
    pub layer: u16,
    pub x: i32,
    pub y: i32,
}

/// Build the store the engine reads, in gdsverify's own load order: push →
/// `finish` (sort by layer) → derive marker layers → bind labels.
///
/// Shapes on a layer in `merge` are pushed as their union (KLayout merged
/// semantics): PEX sums area and fringe per stored polygon, so two drawn rects
/// that overlap would count the overlap twice and their shared edge as fringe.
/// A merged polygon with holes goes in as its disjoint rect decomposition
/// (a holed ring is not one store polygon); its slab edges inside the ring
/// then count as fringe. Other layers go in one polygon per shape.
///
/// # Errors
/// A merge refused by gdsverify's boolean, a derived-layer failure, or a pin
/// label on no shape of its conductor (fail closed: a net silently losing its
/// name would be an untraceable LVS miss).
pub fn build_store(
    shapes: &[Shape],
    pins: &[LabeledPin],
    deck: &Deck,
    strings: &mut StrTable,
    merge: &[u16],
) -> Result<(GeometryStore, Provenance), String> {
    let nm = |v: i32| Dbu::new_unchecked(i64::from(v));
    let mut builder = GeometryStoreBuilder::with_capacity(shapes.len(), shapes.len() * 4);
    let mut provenance = Provenance::default();
    let mut merged: Vec<(u16, GeometryStoreBuilder)> = Vec::new();
    for s in shapes {
        let (x0, y0, x1, y1) = (nm(s.rect.x), nm(s.rect.y), nm(s.rect.x + s.rect.w), nm(s.rect.y + s.rect.h));
        let (b, l) = if merge.contains(&s.layer.0) {
            let i = merged.iter().position(|(l, _)| *l == s.layer.0).unwrap_or_else(|| {
                merged.push((s.layer.0, GeometryStoreBuilder::default()));
                merged.len() - 1
            });
            (&mut merged[i].1, GvLayerId(0))
        } else {
            (&mut builder, GvLayerId(s.layer.0))
        };
        b.push(l, &[x0, x1, x1, x0], &[y0, y0, y1, y1]);
    }
    let (mut raw, mut union, mut rects, mut start) = (ValidatedLayer::default(), ValidatedLayer::default(), Vec::new(), Vec::new());
    for (l, b) in merged {
        let (tmp, _) = b.finish(1);
        validate_layer_into(&tmp, GvLayerId(0), &mut raw).map_err(|e| format!("merge layer {l}: {e}"))?;
        merge_into(&raw, &mut union).map_err(|e| format!("merge layer {l}: {e}"))?;
        decompose_into(&union, &mut rects, &mut start);
        for i in 0..union.len() {
            let poly = union.get(i as u32);
            if poly.holes().next().is_none() {
                let (xs, ys) = poly.outer().coords();
                builder.push(GvLayerId(l), xs, ys);
            } else {
                for r in &rects[start[i] as usize..start[i + 1] as usize] {
                    builder.push(GvLayerId(l), &[r.xlo, r.xhi, r.xhi, r.xlo], &[r.ylo, r.ylo, r.yhi, r.yhi]);
                }
            }
        }
    }
    for p in pins {
        let name = strings.intern(&p.name);
        let layer = label_layer(deck, p.layer).ok_or_else(|| format!("pin {}: no label layer names a conductor of layer {}", p.name, p.layer))?;
        provenance.place_label(Point { x: nm(p.x), y: nm(p.y) }, GvLayerId(layer), name);
    }
    let (mut store, _) = builder.finish(deck.layers.len());
    derive_layers(&mut store, deck, &provenance, strings).map_err(|e| format!("derived layers: {e}"))?;
    provenance
        .resolve_labels(&store, &deck.connectivity)
        .map_err(|e| format!("pin label: {e}"))?;
    Ok((store, provenance))
}

/// The label layer that names the conductor drawn on `drawn`: a label row
/// whose conductor is `drawn` or is derived from it (sky130 `li` names
/// `li_c = li not li_rs`; ihp labels only through `metal1_label`). `drawn`
/// itself when it is such a label layer. `None` when no label reaches it.
#[must_use]
pub fn label_layer(deck: &Deck, drawn: u16) -> Option<u16> {
    let c = &deck.connectivity;
    let d = GvLayerId(drawn);
    let rows: Vec<usize> = (0..c.label_layer.len())
        .filter(|&r| c.label_names[r] == d || deck.layers.operands(c.label_names[r]).contains(&d))
        .collect();
    rows.iter()
        .find(|&&r| c.label_layer[r] == d)
        .or_else(|| rows.first())
        .map(|&r| c.label_layer[r].0)
}
