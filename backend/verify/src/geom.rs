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
    /// The net name the label binds (an LVS reference net / port name).
    pub name: String,
    /// Deck layer id of the pin's drawing layer (`pnr_core::LayerId.0`).
    pub layer: u16,
    /// Label x, nm.
    pub x: i32,
    /// Label y, nm.
    pub y: i32,
}

/// Builds the store the engine reads, in gdsverify's own load order: push →
/// `finish` (sort by layer) → derive marker layers → bind labels.
///
/// Shapes on a layer in `merge` are pushed as their union (KLayout merged
/// semantics): PEX sums area and fringe per stored polygon, so two drawn rects
/// that overlap would count the overlap twice and their shared edge as fringe.
/// A merged polygon with holes goes in as its disjoint rect decomposition
/// (a holed ring is not one store polygon); its slab edges inside the ring
/// then count as fringe. Other layers go in one polygon per shape. Shapes
/// with a non-positive width or height draw nothing and are not loaded.
/// Coordinates are taken as nm (1 dbu = 1 nm).
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

/// Returns the label layer that names the conductor drawn on `drawn`: a label row
/// whose conductor is `drawn` or is derived from it (sky130 `li` names
/// `li_c = li not li_rs`; ihp labels only through `metal1_label`). `drawn`
/// itself when it is such a label layer, else the first matching row in
/// deck order. `None` when no label reaches it.
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

#[cfg(test)]
mod cleanup_tests {
    use super::*;
    use crate::{Checker, Pdk};
    use pnr_core::{Process, Rect};

    struct Fixture {
        pdk: Pdk,
        checker: Checker,
    }

    fn fixture(name: &str) -> Fixture {
        let pdk = Pdk::builtin(name).unwrap();
        let checker = Checker::new(&pdk, true).unwrap();
        Fixture { pdk, checker }
    }

    impl Fixture {
        fn shape(&self, role: &str, x: i32, y: i32, w: i32, h: i32) -> Shape {
            Shape { layer: self.pdk.layer(role).unwrap(), rect: Rect { x, y, w, h } }
        }

        fn id(&self, role: &str) -> u16 {
            self.pdk.layer(role).unwrap().0
        }

        fn build(&mut self, shapes: &[Shape], pins: &[LabeledPin], merge: &[u16]) -> Result<(GeometryStore, Provenance), String> {
            let loaded = &mut self.checker.loaded;
            build_store(shapes, pins, &loaded.deck, &mut loaded.strings, merge)
        }
    }

    fn polys(store: &GeometryStore, layer: u16) -> usize {
        store.polys_on_layer(GvLayerId(layer)).len()
    }

    #[test]
    fn nothing_builds_an_empty_store() {
        let mut f = fixture("sky130");
        let (store, prov) = f.build(&[], &[], &[]).unwrap();
        assert_eq!(polys(&store, f.id("met1")), 0);
        assert!(prov.labels().is_empty());
    }

    #[test]
    fn overlapping_shapes_merge_only_on_merge_layers() {
        let mut f = fixture("sky130");
        let m1 = f.id("met1");
        let shapes = [f.shape("met1", 0, 0, 1000, 500), f.shape("met1", 800, 0, 1000, 500), f.shape("met1", 5000, 0, 500, 500)];
        let (plain, _) = f.build(&shapes, &[], &[]).unwrap();
        assert_eq!(polys(&plain, m1), 3, "one polygon per shape");
        let (merged, _) = f.build(&shapes, &[], &[m1]).unwrap();
        assert_eq!(polys(&merged, m1), 2, "the overlapping pair is one polygon, the far one its own");
    }

    // A merged ring has a hole: it goes in as disjoint rects, never one
    // polygon, and every piece is a 4-vertex rect.
    #[test]
    fn a_merged_ring_goes_in_as_rects() {
        let mut f = fixture("sky130");
        let m1 = f.id("met1");
        let shapes = [
            f.shape("met1", 0, 0, 1000, 10_000),
            f.shape("met1", 9000, 0, 1000, 10_000),
            f.shape("met1", 0, 0, 10_000, 1000),
            f.shape("met1", 0, 9000, 10_000, 1000),
        ];
        let (store, _) = f.build(&shapes, &[], &[m1]).unwrap();
        let range = store.polys_on_layer(GvLayerId(m1));
        assert!(range.len() >= 4, "{range:?}");
        let mut area: i64 = 0;
        for p in range {
            let (xs, ys) = store.poly_verts(gdsverify::geom::PolyId(p));
            assert_eq!(xs.len(), 4);
            let (x0, x1) = (xs.iter().min().unwrap().raw(), xs.iter().max().unwrap().raw());
            let (y0, y1) = (ys.iter().min().unwrap().raw(), ys.iter().max().unwrap().raw());
            area += (x1 - x0) * (y1 - y0);
        }
        assert_eq!(area, 100_000_000 - 64_000_000, "the true ring area, nothing counted twice");
    }

    // A zero or negative extent draws nothing: not loaded, merged or not.
    #[test]
    fn degenerate_shapes_are_not_loaded() {
        let mut f = fixture("sky130");
        let m1 = f.id("met1");
        let shapes = [
            f.shape("met1", 0, 0, 1000, 1000),
            f.shape("met1", 2000, 0, 0, 1000),
            f.shape("met1", 3000, 0, 1000, 0),
            f.shape("met1", 4000, 0, -10, 1000),
        ];
        for merge in [&[][..], &[m1][..]] {
            let (store, _) = f.build(&shapes, &[], merge).unwrap();
            assert_eq!(polys(&store, m1), 1, "merge {merge:?}");
        }
    }

    // Coordinates are widened before the far corner is formed.
    #[test]
    fn a_shape_at_the_i32_edge_does_not_overflow() {
        let mut f = fixture("sky130");
        let m1 = f.id("met1");
        let shapes = [f.shape("met1", i32::MAX - 5, 0, 10, 10)];
        let (store, _) = f.build(&shapes, &[], &[]).unwrap();
        let p = store.polys_on_layer(GvLayerId(m1)).start;
        let (xs, _) = store.poly_verts(gdsverify::geom::PolyId(p));
        assert_eq!(xs.iter().max().unwrap().raw(), i64::from(i32::MAX) + 5);
    }

    #[test]
    fn a_pin_on_a_layer_no_label_reaches_is_refused() {
        let mut f = fixture("sky130");
        let nsdm = f.id("nsdm");
        let shapes = [f.shape("nsdm", 0, 0, 1000, 1000)];
        let pin = LabeledPin { name: "A".into(), layer: nsdm, x: 500, y: 500 };
        let err = f.build(&shapes, &[pin], &[]).unwrap_err();
        assert!(err.starts_with("pin A: no label layer"), "{err}");
    }

    #[test]
    fn a_pin_on_no_shape_is_refused() {
        let mut f = fixture("sky130");
        let m1 = f.id("met1");
        let shapes = [f.shape("met1", 0, 0, 1000, 1000)];
        let pin = LabeledPin { name: "A".into(), layer: m1, x: 5000, y: 5000 };
        let err = f.build(&shapes, &[pin], &[m1]).unwrap_err();
        assert!(err.starts_with("pin label: "), "{err}");
    }

    #[test]
    fn a_pin_on_its_shape_binds() {
        let mut f = fixture("sky130");
        let m1 = f.id("met1");
        let shapes = [f.shape("met1", 0, 0, 1000, 1000)];
        let pin = LabeledPin { name: "A".into(), layer: m1, x: 500, y: 500 };
        let (_, prov) = f.build(&shapes, &[pin], &[m1]).unwrap();
        assert_eq!(prov.labels().len(), 1);
    }

    #[test]
    fn label_layer_finds_the_conductors_label() {
        for pdk in ["sky130", "gf180mcu", "ihp_sg13g2"] {
            let f = fixture(pdk);
            let deck = &f.checker.loaded.deck;
            assert!(label_layer(deck, f.id("met1")).is_some(), "{pdk}: met1 has a label");
            // A self-labelled conductor names itself.
            let c = &deck.connectivity;
            for r in (0..c.label_layer.len()).filter(|&r| c.label_layer[r] == c.label_names[r]) {
                assert_eq!(label_layer(deck, c.label_layer[r].0), Some(c.label_layer[r].0), "{pdk}");
            }
        }
        let f = fixture("sky130");
        assert_eq!(label_layer(&f.checker.loaded.deck, f.id("nsdm")), None, "an implant is no conductor");
        assert_eq!(label_layer(&f.checker.loaded.deck, u16::MAX), None);
    }
}
