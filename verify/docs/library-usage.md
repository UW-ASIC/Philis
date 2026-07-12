# Using gdsverify as a library (no GDS)

Nothing in the checkers requires GDS. The GDS reader is just one producer of the core
data structure, `GeometryStore` — you can build one directly (immediate mode; you own all
the data) and run the exact same `run_drc` / `run_lvs` / `run_pex`.

```toml
[dependencies]
gdsverify = { path = "../verify" }          # feature "gpu" for the CUDA prefilter
```

## Minimal example

```rust
use gdsverify::{Deck, GeometryStore, run_drc};

let deck = Deck::from_json(include_str!("params.json"))?;   // or gdsverify::load_deck(path)
let met1 = deck.layers.id("met1").unwrap();

let mut store = GeometryStore::new();
store.add_rect(met1, 0, 0, 500, 90);        // x, y, w, h — a too-narrow wire
let report = run_drc(&store, &deck);
assert_eq!(report.by_kind("min_width").len(), 1);
```

You still need a `Deck` (from `params.json`) even without GDS — it supplies the layer
name → `LayerId` mapping and the rule/PEX parameters. See
[params-json.md](params-json.md).

## GDS imports and units

`read_gds` and `load_gds` keep every coordinate as the exact `i32` database-unit
integer stored in the file; neither silently rescales or rounds geometry.
`GdsLayout::units` exposes the two values from the GDS `UNITS` record, including
`database_unit_nm()`. The low-level `read_gds` parser can represent a legacy stream
without `UNITS` as `None`. The deck-aware `load_gds` entry point is fail-closed: it
requires `UNITS` and rejects a database-unit size that differs from `deck.dbu_nm`
(apart from documented REAL8 representation tolerance).

After import, inspect `GdsLayout::unmapped_geometry`. It is a deterministic list
of `(layer, datatype, element_count)` diagnostics for BOUNDARY/BOX/PATH records
not present in the supplied `LayerTable`. Callers can reject or explicitly waive
them; geometry cannot disappear without a caller-visible count. TEXT records keep
their raw GDS layer/datatype as metadata and are not included in this geometry count.

## The GeometryStore

Struct-of-arrays; a polygon is an index range, not an object (`src/geometry.rs`):

```rust
pub struct GeometryStore {
    pub verts_x: Vec<i32>,          // every vertex of every polygon (hot)
    pub verts_y: Vec<i32>,
    pub poly_layer: Vec<LayerId>,   // per-polygon
    pub poly_vert_start: Vec<u32>,  // index range into the vertex arrays
    pub poly_vert_len: Vec<u32>,
    pub poly_bbox: Vec<Bbox>,       // kept alongside for fast reject
}
```

Coordinates are `i32` DBU (1 DBU = 1 nm in the conformance setup). All fields are `pub`,
so you can also bulk-fill the arrays yourself for large imports — just keep the six
vectors consistent (bbox must cover its polygon's vertices).

### Building

```rust
let mut store = GeometryStore::new();

// axis-aligned rectangle
let p: PolyId = store.add_rect(layer, x, y, w, h);

// general polygon: closed ring, first point NOT repeated
let l = store.add_polygon(layer, &[
    (0, 0), (300, 0), (300, 100), (100, 100), (100, 300), (0, 300),
]);
```

`LayerId` is a plain `u16`. With a `Deck`, get it from the layer table
(`deck.layers.id("met1")`); without one — e.g. for pure geometry queries — any consistent
small integers work.

### Querying

```rust
store.poly_count();                 // number of polygons
store.polys_on_layer(met1);         // Vec<PolyId> on that layer
let (s, e) = store.poly_range(p);   // vertex index bounds into verts_x/verts_y
store.poly_vertex(s, i);            // (x, y) of vertex i
store.poly_bbox[p.0 as usize];      // Bbox { xmin, ymin, xmax, ymax }
store.area(p);                      // shoelace, i64 DBU²
store.signed_area2(p);              // 2× signed area; positive = CCW
```

Standalone geometry primitives (usable without any checker):

```rust
use gdsverify::geometry::{build_edges, seg_seg_dist2, point_in_poly, clipped_area, isqrt};

let edges = build_edges(&store, met1);        // dense Vec<Edge> for one layer
let d2 = seg_seg_dist2(&edges[0], &edges[5]); // squared distance, 0 = touch/cross
point_in_poly(&store, p, x, y);               // strictly inside (boundary = false)
clipped_area(&store, p, x0, y0, x1, y1);      // exact area within a window
```

## Running the checkers

```rust
use gdsverify::{run_drc, run_drc_backend, run_pex, run_lvs, Backend};
use gdsverify::{RefNetlist, RefDevice, DeviceKind};

// DRC — CPU, or GPU prefilter (identical report either way; falls back silently)
let drc = run_drc(&store, &deck);
let drc = run_drc_backend(&store, &deck, Backend::Gpu);
for v in &drc.violations {
    println!("{} on {} at ({},{}): {} < {}", v.kind, v.layer, v.x, v.y, v.measured, v.limit);
}

// PEX — needs "pex" entries in params.json for the layers you care about
let pex = run_pex(&store, &deck);
println!("R(met1) = {} ohm, C = {} aF", pex.total_resistance("met1"), pex.total_cap());

// LVS — reference netlist built in code (this is the schematic side)
let reference = RefNetlist { devices: vec![
    // w/l in nm enable the parametric W/L check; w: 0, l: 0 = topology-only (legacy)
    RefDevice { kind: DeviceKind::Nmos, gate: "in".into(), source: "vss".into(), drain: "out".into(), w: 0, l: 0 },
    RefDevice { kind: DeviceKind::Pmos, gate: "in".into(), source: "vdd".into(), drain: "out".into(), w: 0, l: 0 },
]};
let lvs = run_lvs(&store, &deck, &reference);
assert!(lvs.matched, "{}", lvs.reason);
```

LVS caveat: connectivity is defined over the well-known layer names
(`diff/poly/li/met1/met2` + `licon/mcon/via1` + `nsdm`/`psdm`) — your params.json layer
table must use those names for extraction to see your geometry
(see [verification-rules.md](verification-rules.md)).

## Scope / cells

A store is flat — there is no cell hierarchy inside it. Checking one cell means building
a store containing only that cell's polygons (the GDS path does exactly this:
`load_gds(...)` returns per-cell stores). Same trick scopes a check to any subset you
like: copy the polygons you care about into a fresh store.
