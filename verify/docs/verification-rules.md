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

### The 15 rules

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

1. **Connectivity extraction** — union-find (index-based, path-halving) over all polygons
   on connective layers. Conductors: `diff, poly, li, met1, met2`; vias: `licon, mcon,
   via1`. Any two overlapping connective polygons are unioned — a via overlapping two
   conductors joins them transitively. **Exception: poly over diff is NOT unioned** — that
   overlap is a transistor gate, not a short. Union-find roots become compact net IDs.

2. **Device extraction** — each poly polygon crossing a diff polygon is a MOS:
   - *gate* = the poly's net;
   - *source/drain* = nets of the li/met1 conductors landing on the diff to the left and
     right of the gate (by centroid x vs gate centroid x);
   - *N/P type* = whichever implant (`nsdm`/`psdm`) overlaps the **channel region**
     (gate ∩ diff bbox — not the whole gate poly, which may span both diffusions);
     default NMOS;
   - body/well net is simplified to a constant (not structurally compared).

3. **Comparison** (netgen-style partition refinement, simplified Gemini):
   - device-count check first: NMOS and PMOS counts must match the reference netlist
     (mismatch ⇒ `device count mismatch`);
   - then a topology invariant: devices are labeled by invariants and the induced
     partition structure must match. For the CMOS-inverter conformance cases this reduces
     to *shared gate net + shared drain net between the N and P device*, which
     distinguishes match / swapped-gate / open-output.

The reference netlist (`RefNetlist`) comes from the schematic side — the conformance
manifest supplies it as named-net device lists.

Result: `LvsResult { matched, reason, extracted_devices, nmos, pmos }`.

## PEX

Entry point: `run_pex(store, deck)`. Analytical, pattern-based 2.5D — **no field solver**.
Runs only on layers with a `pex` entry in params.json. Constants per layer:
`Rs` (Ω/□), `Ca` (aF/µm²), `Cf` (aF/µm), `Ck` (aF/µm), `Sref` (nm).

| parasitic | formula | which shapes |
|---|---|---|
| Resistance | `R = Rs · L/W` (sheet res × number of squares; L/W from bbox long/short side) | wire-like polygons only: aspect ratio ≥ 2 |
| Area + fringe cap | `C = Ca·A + Cf·P` (A in µm², P = perimeter in µm, to substrate) | plate-like polygons only: aspect ratio < 2 |
| Lateral coupling cap | `C_c = Ck · Lp · (Sref/S)` — parallel run length `Lp` (µm) scaled inversely with spacing `S` relative to reference spacing | same-layer bbox pairs that face each other: overlap on one axis, positive gap on the other |

The wire/plate split is deliberate: a shape is either a resistor (wire) or a
substrate capacitor (plate) in this model, never both. Coupling considers horizontal
pairs (x-overlap, y-gap) then vertical pairs (y-overlap, x-gap).

Output: `PexReport` with helpers `total_resistance(layer)`, `total_cap()`, and typed
accessors (`resistances()`, `area_caps()`, `coupling_caps()`).
