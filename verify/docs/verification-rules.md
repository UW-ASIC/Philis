# DRC rules, LVS algorithm, PEX rules

What each checker actually computes. Sources: `src/drc.rs`, `src/lvs.rs`, `src/pex.rs`.
How to *configure* them is in [params-json.md](params-json.md).

All coordinates are `i32` DBU; all distance math is integer (squared distances in `i64`,
reported via integer sqrt), so results are exact and bit-identical across CPU and GPU runs.

## DRC

Entry point: `run_drc(store, deck)` / `run_drc_backend(store, deck, Backend::Gpu)`.
The engine loops the resolved rule deck and `match`es each rule (tagged union, no vtables)
to one geometric kernel. Output is a flat `Vec<Violation>`
(`rule_id, kind, layer, measured, limit, x, y`) — measured values in DBU, density in ppm,
`-1` where N/A (angle).

### Shared machinery

- **Candidate pruning** (`candidate_pairs`): every pairwise rule (spacing, enclosure,
  corner-to-corner) shares an O(P log P + K) x-sweep — sort polygons by `xmin`, look ahead
  only while x-ranges can still be within `min`, filter on y. No quadratic all-pairs scan.
- **Distance primitive** (`seg_seg_dist2` / `poly_poly_dist2`): min squared distance
  between edge sets; 0 means touching/crossing.
- **Facing-gap classification** (`facing_gaps`): for same-polygon checks, parallel edge
  pairs with positive overlap span are measured, and the gap's midpoint is tested with
  point-in-polygon: midpoint **inside** ⇒ a width, **outside** ⇒ a notch. This is what
  stops a U-shape's notch from being reported as a narrow width (the classic naive-scan
  false positive).
- **GPU prefilter** (feature `gpu`): spacing/width/notch/enclosure/corner/edge-length/
  angle/off-grid scans can run a conservative f32 prefilter on the device; anything not
  *comfortably* clean is exact-rechecked on the CPU with the same integer code, so reports
  are identical with or without a GPU. Below a measured break-even
  (`GPU_MIN_PAIR_WORK`/`GPU_MIN_LINEAR_WORK`) the scan stays on the CPU.

### Core rules (15 of 28 configured kinds)

The table below describes the original core set. The resolved deck also contains
antenna/CAR, EOL/PRL/wide-dependent spacing, asymmetric enclosure, enclosed-area,
cheesing, redundant-via, via-array, tap-distance, multi-patterning and polygon-validity
checks. These are simplified engine-specific predicates, not a foundry rule language.

| rule | algorithm |
|---|---|
| `min_width` | Rectangles (4 verts): `min(bbox.w, bbox.h)` — exact. General rectilinear: every *interior* facing gap < min is one violation site (an L with two thin arms = two violations). |
| `min_spacing` | Per candidate pair, exact `poly_poly_dist2`. Distance 0 (abutting/crossing) ⇒ merged shape, exempt. One polygon strictly inside the other ⇒ hole/island merge semantics, exempt. Overlapping *bboxes* alone do **not** exempt (interlocking Ls are still checked). |
| `min_spacing_diff` | Same as `min_spacing` but across two layers; touching layers are not a spacing pair. |
| `min_enclosure` | Phase 1: find the outer polygon that *truly contains* each inner (all vertices strictly inside — not bbox, so L-shaped outers work). Unhosted inner ⇒ violation with measured 0. Phase 2: margin = min boundary-to-boundary distance; < min ⇒ violation. |
| `min_extension` | For each layer/reference bbox overlap: orientation from the layer shape's long axis, then protrusion past the reference measured on both ends; each end < min is a violation (e.g. poly endcap over diff). |
| `min_area` | Shoelace area per polygon. |
| `max_width` | `min(bbox.w, bbox.h) > max` (slotting trigger). |
| `notch` | Same facing-gap scan as width, but *exterior* gaps only. |
| `min_edge_length` | Per-edge `len² < min²` (zero-length edges skipped). |
| `off_grid` | Every vertex of every layer: `x % grid != 0 || y % grid != 0`. GPU path is exact integer remainder. |
| `angle` | Edge orientation normalized to [0,180); must match an allowed angle within 0.5°. |
| `min_density` / `max_density` | Layer's global bbox tiled into `window`-sized windows anchored at the field origin. Coverage accumulated in one pass over polygons (O(P + windows)), each polygon **exactly clipped** to the window (Sutherland–Hodgman + shoelace) — bbox coverage would badly overstate combs. Fraction vs limit per window; measured/limit reported in ppm. Assumes same-layer shapes don't overlap (true post-merge). |
| `overlap` | Where two layers' bboxes overlap with positive area, overlap width = `min(ix, iy)`; disjoint pairs are not this rule's job. |
| `corner_to_corner` | Only fires when two shapes are diagonally offset (no x *and* no y span overlap — otherwise it's `min_spacing`'s job); min vertex-to-vertex distance < min. |

## LVS

Entry point: `run_lvs(store, deck, reference)` = `extract_netlist` + `compare`.

Three stages:

1. **Connectivity extraction** — the deck explicitly names conductor layers and via-to-
   conductor relations. A sweep produces bbox candidates, then supported rectilinear
   polygon regions are tested for real positive-area overlap or policy-controlled
   same-layer boundary contact. Cross-layer conductor overlap is not connectivity unless
   it is mediated by an allowed cut, or the caller explicitly selects the documented
   cut-less compatibility mode. Non-rectilinear, degenerate and self-intersecting geometry
   on extraction layers is an error, not an empty result.

2. **Device extraction** — configured gate/channel crossings split source/drain regions.
   A MOS must match exactly one rule/type implant over the actual channel region;
   missing, conflicting N/P, or conflicting HVT/LVT markers stop extraction. Simple
   configured resistor, capacitor, diode and BJT recognition also exists. Body/well
   extraction remains incomplete and body pins are not yet compared structurally.

3. **Comparison** — device/property screening and graph refinement support legal MOS
   source/drain permutation plus constrained series/parallel normalization. Series merge
   requires compatible type/flavor/class/body/width, a degree-two internal S/D net, and no
   gate/passive/named/global observation. Production named-port seeding, complete witnesses,
   symmetric reference reductions and hierarchy-preserving comparison remain roadmap work.

The reference netlist (`RefNetlist`) comes from the schematic side — the conformance
manifest supplies it as named-net device lists.

Result: `LvsResult { matched, reason, extracted_devices, nmos, pmos }`.

## PEX

Entry point: `run_pex(store, deck)`. Analytical, pattern-based 2.5D — **no field solver**.
Runs only on layers with a `pex` entry in params.json. Constants per layer:
`Rs` (Ω/□), `Ca` (aF/µm²), `Cf` (aF/µm), `Ck` (aF/µm), `Sref` (nm).

| parasitic | formula | which shapes |
|---|---|---|
| Resistance | `R = Rs · Leq/Weq`; `Leq/Weq` is derived from actual Manhattan polygon area/perimeter | every supported rectilinear conductor |
| Area + fringe cap | `C = Ca·A + Cf·P` using actual polygon area/perimeter | every supported rectilinear conductor, including wires |
| Lateral coupling cap | `C_c = Ck · Lp · (Sref/S)` — parallel run length `Lp` (µm) scaled inversely with spacing `S` relative to reference spacing | same-layer bbox pairs that face each other: overlap on one axis, positive gap on the other |
| Interlayer overlap cap | configured overlap coefficient times bbox overlap area | every configured layer pair; stack adjacency/shield semantics are not modeled |
| Via resistance | fixed configured resistance per cut polygon | configured via/contact layers |

Unsupported R/ground-C geometry emits an `ExtractionDiagnostic`; it is not converted
to zero. `PexReport::is_complete()` and `run_pex_by_net_checked()` make that diagnostic
blocking for signoff and parasitic budgets. Fill and shielding are not guessed from
polygon size or layer names. Resistance remains a scalar per-net estimate, not a
terminal-aware distributed network.

Output: `PexReport` with helpers `total_resistance(layer)`, `total_cap()`, and typed
accessors (`resistances()`, `area_caps()`, `coupling_caps()`, `diagnostics()`).
