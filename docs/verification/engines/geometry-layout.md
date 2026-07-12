# Geometry, layout formats and hierarchy

## Implemented foundation

[`geometry/exact.rs`](../../../verify/src/geometry/exact.rs) provides `i128` orientation,
segment intersection, winding/point/contact classification, validated rings/polygons,
and deterministic exact rectilinear union/intersection/subtraction with holes. It
reports invalid topology, unsupported non-rectilinear booleans, overflow/complexity
limits and winding errors as typed failures.

[`gds_lossless.rs`](../../../verify/src/gds_lossless.rs) preserves library/structure
records, element metadata/properties, BOUNDARY, PATH, BOX, TEXT, NODE, SREF/AREF,
transforms and unsupported records. Strict/compatibility read modes, write/round-trip,
checked flattening and PATH stroking are separate operations. [`gds.rs`](../../../verify/src/gds.rs)
maps supported geometry into per-cell `GeometryStore`s while retaining units, text,
hierarchy/property side data and unmapped-layer diagnostics.

[`hierarchy_index.rs`](../../../verify/src/hierarchy_index.rs) provides hierarchy-aware
candidates with instance paths and deterministic tiling/halos. [`oasis.rs`](../../../verify/src/oasis.rs)
has an explicit capability constant and a small fail-closed supported subset.

## Required invariants

- raw records may be lossless even when they are unsupported for verification;
- checked conversion preserves cell/instance/element/property/text identity;
- units are explicit; deck-aware loading never silently rescales;
- bbox indexes generate candidates only; exact geometry decides semantics;
- unsupported transform/path/geometry produces a typed error, not no geometry;
- flat, hierarchy-preserving and tiled paths have canonical equivalent results.

## Current unsupported boundary

All-angle boolean/offset support is incomplete. Checked GDS verification still rejects
NODE geometry semantics, absolute STRANS behavior, nonorthogonal/nonintegral transforms,
and round/diagonal/odd/negative PATH cases not exactly representable. OASIS still needs
the production-required PATH/TEXT/repetition/property/modal/reference/XYRELATIVE/
placement/CBLOCK/compression subset.

Use [Wave 2](../work-packages/wave-2.md) for exact tasks/tests/acceptance. No checker
may build a private replacement for a missing shared operation.
