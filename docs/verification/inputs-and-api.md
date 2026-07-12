# Inputs, decks and API guide

## PDK/deck JSON

`Deck::from_json` adapts one shared PDK JSON document through `VerifySchema` and
resolves it into typed layer, DRC, LVS, ERC and PEX configuration. Verification-owned
objects reject unknown fields; unrelated top-level PDK sections are tolerated because
other repository components own them.

```json
{
  "dbu_nm": 1.0,
  "layers": { "met1": { "layer": 7, "datatype": 0 } },
  "drc": [
    { "id": "M1.W.1", "kind": "min_width", "layer": "met1", "min": 140 }
  ],
  "connectivity": { "conductors": [], "vias": [] },
  "device_recognition": { "mos": [], "resistors": [], "diodes": [], "capacitors": [], "bjts": [] },
  "lvs": {},
  "pex": {},
  "erc": {}
}
```

Rule `id` is stable report identity; `kind` selects implementation. Object-map syntax
is retained for compatibility, including repeated kinds when each key is a distinct ID.
Disable only with `"enabled": false`; zero/missing/negative/non-finite required limits
are validation errors, including on disabled entries. Every referenced layer/device/
connectivity/model must resolve. Duplicate IDs or GDS `(layer, datatype)` aliases error.

Current legacy DRC kinds include width/spacing/cross-spacing, enclosure/extension,
area/max-width/notch/edge length/off-grid/angle, min/max density, overlap,
corner-to-corner, antenna/CAR, EOL/PRL/wide spacing, asymmetric enclosure,
enclosed-area/cheesing, redundant/via-array rules, tap distance, multi-patterning and
always-on polygon validity. This enum is not a foundry rule language; see
[DRC limitations](engines/drc.md).

PEX per-layer scalar fields use explicit units: Ω/□, aF/µm², aF/µm, nm and optional
Ω/cut or aF/µm² interlayer terms. A zero optional coefficient means that model is not
declared, not that the physical mechanism is universally zero.

## GDS input policies

```rust
let compatibility = gdsverify::load_gds(path, &deck)?;
let signoff = gdsverify::load_gds_strict(path, &deck)?;
```

Both retain integer GDS DBU coordinates and require deck-aware `UNITS` agreement.
`load_gds` preserves closed invalid polygons for legacy DRC validity markers.
`load_gds_strict` uses strict lossless parsing and validation. `read_gds_library` /
`write_gds_library` expose raw/lossless structure; `flatten_gds_library` is an explicit
checked conversion with `GdsFlattenOptions` and `GdsGeometryPolicy`.

Always inspect unmapped geometry and typed errors. A raw/lossless round trip does not
imply that every record is supported for verification. The current GDS-supported and
unsupported semantics are in [geometry/layout](engines/geometry-layout.md).

## OASIS

`read_oasis`, `write_oasis` and `OASIS_CAPABILITIES` define the current explicit
subset. A caller must inspect the declaration and reject a file needing undeclared
features. OASIS must converge on the same checked layout/identity contract as GDS;
format-specific checker behavior is forbidden.

## Direct geometry API

Callers may construct `GeometryStore` without a stream file:

```rust
use gdsverify::{Deck, GeometryStore, run_drc};

let deck = Deck::from_json(include_str!("params.json"))?;
let met1 = deck.layers.id("met1").ok_or("missing met1")?;
let mut store = GeometryStore::new();
store.add_rect(met1, 0, 0, 500, 90);
let report = run_drc(&store, &deck);
```

`GeometryStore` is a flat struct-of-arrays and cannot preserve lossless cell/property
semantics by itself. Bulk callers must keep vertex ranges, layer IDs and bboxes
consistent. Prefer validated exact polygon types for new topology algorithms.

## Reference netlists and LVS

`parse_netlist` parses one source; `parse_netlist_with_includes` makes include
resolution an explicit caller policy. `bind_reference_hierarchy` evaluates parameters,
models, black boxes and equated-cell configuration. `leaf_subcircuit_to_ref_netlist`
bridges a supported leaf to legacy comparison. Production code uses detailed records:

```text
extract_detailed_netlist -> DetailedExtractedNetlist
bind_reference_hierarchy -> BoundReferenceHierarchy
compare_production / compare_hierarchical_production -> mappings + witnesses
```

The real checked GDS evidence adapter into production hierarchy is still integration
pending. Do not synthesize a root path or bind ports from proximity alone.

## DRC, PEX and signoff

`run_drc` and `run_drc_backend` return `DrcReport`. `Backend::Gpu` is a conservative
prefilter; CPU exact recheck preserves report identity. GPU availability/fallback is
not proof that GPU tests ran.

`run_pex` returns the legacy analytical report. `run_pex_by_net_checked` blocks on
unsupported extraction diagnostics. Neither is a distributed signoff extractor; see
[PEX](engines/pex.md).

`run_signoff_suite` aggregates typed optional configurations into four-state family
reports. Complete configuration and evidence are mandatory for `CLEAN`; absence is
`NOT_RUN`, invalid/ambiguous evidence is `ERROR`.

## Neutral correlation CLI

```text
cargo run -p gdsverify --bin correlation -- compare GOLDEN ACTUAL [options]
cargo run -p gdsverify --bin correlation -- freeze --root ROOT --output OUT INPUT...
cargo run -p gdsverify --bin correlation -- verify-freeze MANIFEST --root ROOT
cargo run -p gdsverify --bin correlation -- summarize ARTIFACT
```

Exit codes are 0 for accepted match/freeze, 2 for input/schema/semantic errors, 3 for
unaccepted deltas, 4 for freeze changes, and 64 for usage. Detailed artifact, tolerance,
disposition and freeze contracts are in [qualification](compliance/qualification.md).
