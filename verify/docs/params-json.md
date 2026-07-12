# Writing `params.json`

One JSON = one PDK. The library reads a single `params.json` describing the layer
table, DRC rules, PEX constants, LVS connectivity/device recognition and ERC limits
(`src/params.rs`, `Deck::from_json`). The repository's complete PDK documents may also
contain top-level cell-generation/model sections owned by other consumers.

```json
{
  "layers": { ... },   // required
  "drc":    { ... },   // required (may be empty {})
  "pex":    { ... },   // optional analytical models
  "lvs":    { ... },
  "connectivity": { ... },
  "device_recognition": { ... },
  "erc":    { ... }
}
```

All dimensions are integers in **DBU** (database units; 1 DBU = 1 nm in the conformance
setup, `dbu_nm = 1.0`). Areas are DBU². Density fractions are floats in [0, 1].

## 1. `layers`

Maps a symbolic name to a GDS `(layer, datatype)` pair. Every layer referenced anywhere
else in the file must be declared here; an unknown name is a load error
(`unknown layer 'x'`). Names must be non-empty, pairs must be in the supported
non-negative signed-16-bit GDS record range, and two names may not alias the same pair.
Aliasing is rejected because the input mapper could otherwise make one symbolic layer
silently unreachable.

```json
"layers": {
  "met1": { "layer": 7, "datatype": 0 },
  "mcon": { "layer": 6, "datatype": 0 }
}
```

Internal `LayerId`s are assigned in `(layer, datatype)` order, so IDs (and report order)
are stable across runs regardless of JSON key order.

LVS does not infer well-known names. The `connectivity` and `device_recognition`
sections explicitly declare conductor, via, gate/channel, implant and marker roles;
unknown references, duplicate memberships and unsupported device/flavor names are errors.

## 2. `drc`

Every rule has an independent stable `id` (reported in violations) and `kind` (the
implementation selector). The preferred object form maps **rule ID → parameters**;
put `kind` in the body when the ID differs from the kind. This permits any number of
instances of one kind:

```json
"drc": {
  "M1.W.1": { "kind": "min_width", "layer": "met1", "min": 140 },
  "M2.W.1": { "kind": "min_width", "layer": "met2", "min": 200 }
}
```

An explicit array is also accepted:

```json
"drc": [
  { "id": "M1.W.1", "kind": "min_width", "layer": "met1", "min": 140 }
]
```

Legacy maps remain valid: for `"min_width": { ... }`, the key is used as both
`id` and `kind`. IDs must be unique, and an unknown kind is a load error.

### Disabling rules

Every PDK obeys the same unified rule superset. A configured rule is disabled only
with `"enabled": false` (default is `true` when omitted). Required limits that are
missing, zero, negative, non-finite, or out of range are construction errors; they
never silently remove a check. Disabled entries are still fully parsed and validated:
`enabled` controls execution, not whether the declared deck operation is understood.

Compatibility note: legacy **map syntax** is preserved. Decks that previously used
a zero principal limit as an implicit disable must be migrated to `"enabled": false`;
rejecting that ambiguous fail-open convention is an intentional validation change.

### The 15 rule types

| key | parameters | meaning |
|---|---|---|
| `min_width` | `layer`, `min` | narrowest interior dimension ≥ min |
| `min_spacing` | `layer`, `min` | same-layer shape-to-shape gap ≥ min |
| `min_spacing_diff` | `layer_a`, `layer_b`, `min` | cross-layer gap ≥ min |
| `min_enclosure` | `outer`, `inner`, `min` | outer must enclose inner by ≥ min on all sides |
| `min_extension` | `layer`, `ref`, `min` | layer must extend past ref by ≥ min (e.g. poly endcap over diff) |
| `min_area` | `layer`, `min` | polygon area ≥ min (DBU²) |
| `max_width` | `layer`, `max` | narrow dimension ≤ max (slotting trigger) |
| `notch` | `layer`, `min` | same-polygon exterior facing gap ≥ min |
| `min_edge_length` | `layer`, `min` | every edge ≥ min |
| `off_grid` | `grid` | all vertices (all layers) on a `grid`-DBU grid |
| `angle` | `allowed` (int array, degrees) | edge orientations restricted to the list, e.g. `[0, 45, 90, 135]` |
| `min_density` | `layer`, `window`, `min_frac` | windowed coverage fraction ≥ min_frac |
| `max_density` | `layer`, `window`, `max_frac` | windowed coverage fraction ≤ max_frac |
| `overlap` | `layer_a`, `layer_b`, `min` | where the two layers overlap, overlap width ≥ min |
| `corner_to_corner` | `layer`, `min` | diagonal corner distance between shapes ≥ min |

Example:

```json
"drc": {
  "min_width":       { "layer": "met1", "min": 140 },
  "min_spacing":     { "layer": "met1", "min": 140 },
  "min_enclosure":   { "outer": "met1", "inner": "mcon", "min": 60 },
  "min_extension":   { "layer": "poly", "ref": "diff", "min": 130 },
  "min_area":        { "layer": "met1", "min": 100000 },
  "max_width":       { "enabled": false, "layer": "met1", "max": 5000 },
  "off_grid":        { "grid": 5 },
  "angle":           { "allowed": [0, 45, 90, 135] },
  "min_density":     { "layer": "met1", "window": 2000, "min_frac": 0.2 },
  "max_density":     { "layer": "met1", "window": 2000, "max_frac": 0.7 }
}
```

Gotchas:

- Missing or non-positive required dimensions are errors. Density fractions must be
  finite and in `[0, 1]`; density windows must be positive.
- `angle.allowed` must be a non-empty integer array with values in `[0, 180)`.
- Multi-patterning color counts must be in `[2, 64]`.
- Unknown properties inside DRC, layer, PEX, ERC, LVS, connectivity and device-recognition
  objects are errors. Unrelated top-level sections are retained for compatibility with the
  repository's complete PDK document, whose other consumers own those sections.
- Rules are sorted by ID at load time, so report order is deterministic.

## 3. `pex`

Optional. Maps a layer name to its process constants. A layer named here but not in
`layers` is a descriptive deck-construction error.

```json
"pex": {
  "met1": {
    "sheet_res_ohm_sq":       0.125,
    "area_cap_af_um2":        30.0,
    "fringe_cap_af_um":       40.0,
    "coupling_cap_af_um":     50.0,
    "coupling_ref_spacing_nm": 140
  }
}
```

The five base fields are required per layer. `via_res_ohm` and
`interlayer_cap_af_um2` are optional non-negative coefficients (default `0`, meaning
that model is not declared on the layer):

| field | unit | used in |
|---|---|---|
| `sheet_res_ohm_sq` | Ω/□ | `R = Rs · L/W` |
| `area_cap_af_um2` | aF/µm² | `C_area = Ca · A` |
| `fringe_cap_af_um` | aF/µm | `C_fringe = Cf · P` |
| `coupling_cap_af_um` | aF/µm | `C_c = Ck · Lp · (Sref/S)` |
| `coupling_ref_spacing_nm` | nm | the `Sref` in the coupling formula |
| `via_res_ohm` | Ω/cut polygon | fixed analytical via/contact resistance |
| `interlayer_cap_af_um2` | aF/µm² | simplified cross-layer overlap coefficient |

See [verification-rules.md](verification-rules.md) for what the checkers do with all of
this. A complete working deck is at `conformance/params.json`.
