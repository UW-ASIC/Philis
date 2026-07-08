# Writing `params.json`

One JSON = one PDK. The library reads a single `params.json` describing three things —
the layer table, the DRC rule deck, and PEX process constants — and that is the *only*
configuration input (`src/params.rs`, `Deck::from_json`).

```json
{
  "layers": { ... },   // required
  "drc":    { ... },   // required (may be empty {})
  "pex":    { ... }    // optional
}
```

All dimensions are integers in **DBU** (database units; 1 DBU = 1 nm in the conformance
setup, `dbu_nm = 1.0`). Areas are DBU². Density fractions are floats in [0, 1].

## 1. `layers`

Maps a symbolic name to a GDS `(layer, datatype)` pair. Every layer referenced anywhere
else in the file must be declared here; an unknown name is a load error
(`unknown layer 'x'`).

```json
"layers": {
  "met1": { "layer": 7, "datatype": 0 },
  "mcon": { "layer": 6, "datatype": 0 }
}
```

Internal `LayerId`s are assigned in `(layer, datatype)` order, so IDs (and report order)
are stable across runs regardless of JSON key order.

Note for LVS: connectivity is defined over well-known names — conductors
`diff, poly, li, met1, met2` and vias `licon, mcon, via1`, plus implants `nsdm`/`psdm`
for N/P selection (`src/lvs.rs`, `connective_layers`). Use these exact names if you want
LVS to work; layers with other names are simply ignored by LVS.

## 2. `drc`

A map of **rule-type → parameters**. The key is both the rule's ID in reports and its
type discriminator, so each rule type appears **at most once per deck** (you cannot have
two `min_width` entries for different layers). An unknown key is a load error.

### Disabling rules

Every PDK obeys the same unified rule superset; you hide a rule rather than delete it:

- `"enabled": false` — explicit off (default is `true` when omitted), or
- principal limit `0` — `min: 0`, `max: 0`, or `grid: 0` disables the rule.

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
  "max_width":       { "enabled": false, "layer": "met1", "max": 0 },
  "off_grid":        { "grid": 5 },
  "angle":           { "allowed": [0, 45, 90, 135] },
  "min_density":     { "layer": "met1", "window": 2000, "min_frac": 0.2 },
  "max_density":     { "layer": "met1", "window": 2000, "max_frac": 0.7 }
}
```

Gotchas:

- Missing numeric fields default to 0 — which *disables* the rule silently. Spell out
  every limit.
- `min_density`/`max_density` don't have the limit-0 escape; use `enabled: false`
  (a `window` ≤ 0 also makes the check a no-op).
- `angle` with a missing/empty `allowed` list flags **every** edge.
- Rules are sorted by ID at load time, so report order is deterministic.

## 3. `pex`

Optional. Maps a layer name to its process constants. Layers named here but not in
`layers` are silently skipped.

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

All five fields are required per layer:

| field | unit | used in |
|---|---|---|
| `sheet_res_ohm_sq` | Ω/□ | `R = Rs · L/W` |
| `area_cap_af_um2` | aF/µm² | `C_area = Ca · A` |
| `fringe_cap_af_um` | aF/µm | `C_fringe = Cf · P` |
| `coupling_cap_af_um` | aF/µm | `C_c = Ck · Lp · (Sref/S)` |
| `coupling_ref_spacing_nm` | nm | the `Sref` in the coupling formula |

See [verification-rules.md](verification-rules.md) for what the checkers do with all of
this. A complete working deck is at `conformance/params.json`.
