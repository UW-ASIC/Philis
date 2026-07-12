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

Every DRC entry accepts common fields `id` (stable nonempty identity), `kind`
(implementation selector) and optional `enabled` (boolean, default true). These are the
complete currently accepted legacy `kind` mappings; all dimensions are positive DBU,
areas are positive DBU², fractions are finite `[0,1]`, and ratios are positive unitless
values unless the row says otherwise.

| `kind` | Required JSON fields | Current meaning / validation |
|---|---|---|
| `min_width` | `layer`, `min` | minimum width |
| `min_spacing` | `layer`, `min` | same-layer spacing |
| `min_spacing_diff` | `layer` (or `layer_a`), `layer_b`, `min` | cross-layer spacing |
| `min_enclosure`, `well_enclosure` | `outer`, `inner`, `min` | `well_enclosure` is a legacy alias |
| `min_extension` | `layer`, `ref` (or `reference`), `min` | extension past reference layer |
| `min_area` | `layer`, `min` | minimum polygon area; `min` is i64 DBU² |
| `max_width` | `layer`, `max` | maximum narrow dimension |
| `notch` | `layer`, `min` | exterior same-shape gap |
| `min_edge_length` | `layer`, `min` | per-edge minimum length |
| `off_grid` | `grid` | all vertices on positive DBU grid |
| `angle` | `allowed` | nonempty integer degree array, each value in `[0,180)` |
| `min_density` | `layer`, `window`, `min_frac` | positive DBU window; minimum fraction |
| `max_density` | `layer`, `window`, `max_frac` | positive DBU window; maximum fraction |
| `overlap` | `layer` (or `layer_a`), `layer_b`, `min` | simplified positive-overlap width |
| `corner_to_corner` | `layer`, `min` | diagonal corner distance |
| `antenna` | `layer`, `ratio` | simplified single-layer area ratio |
| `antenna_car` | `layers`, `ratio`; optional `diode_layer` | nonempty unique ordered layer stack and optional diode marker |
| `eol_spacing` | `layer`, `eol_width`, `eol_spacing` | single-threshold EOL rule |
| `wide_dependent_spacing` | `layer`, `width_threshold`, `wide_spacing` | single wide-wire threshold |
| `prl_spacing` | `layer`, `prl_threshold`, `prl_spacing` | single parallel-run threshold |
| `asymmetric_enclosure` | `outer`, `inner`, `min` | `min` resolves to minimum one-side enclosure |
| `min_enclosed_area` | `layer`, `min` | `min` resolves to minimum hole area in DBU² |
| `cheesing` | `layer`, `max` | `max` resolves to maximum unslotted area in DBU² |
| `redundant_via` | `layer`, `min_count`, `within` | `min_count >= 2`; `within` is DBU distance |
| `via_array_spacing` | `layer`, `array_threshold`, `array_spacing` | `array_threshold >= 2`; spacing in DBU |
| `max_distance_to_tap` | `layer`, `layer_b`, `max_dist` | first layer is diffusion, second is tap |
| `multi_patterning` | `layer`, `num_colors`, `min` | colors in `[2,64]`; `min` resolves to color spacing |

Always-on polygon-validity checking is policy, not a JSON `kind`. This legacy enum is
not a production foundry rule language; see [DRC limitations](engines/drc.md).

The `pex` object maps a declared layer name to this complete scalar model:

| JSON field | Unit | Current use |
|---|---|---|
| `sheet_res_ohm_sq` | Ω/□ | required; `R = Rs * Leq/Weq` |
| `area_cap_af_um2` | aF/µm² | required area-to-ground coefficient |
| `fringe_cap_af_um` | aF/µm | required perimeter/fringe coefficient |
| `coupling_cap_af_um` | aF/µm | required same-layer coupling coefficient |
| `coupling_ref_spacing_nm` | nm | required reference spacing in coupling scale |
| `via_res_ohm` | Ω/cut polygon | optional, nonnegative; default `0` means undeclared |
| `interlayer_cap_af_um2` | aF/µm² | optional, nonnegative; default `0` means undeclared |

All PEX coefficients are finite and nonnegative; the coupling reference spacing must be
strictly positive when `coupling_cap_af_um > 0`. A zero optional coefficient means that
model is not declared, not that the physical mechanism is universally zero.

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
