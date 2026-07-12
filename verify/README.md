# gdsverify — GDS verification core (DRC / LVS / PEX)

A data-oriented, optionally GPU-accelerated GDS verification library in Rust, plus a Python
conformance-suite generator that emits GDS + a machine-readable expected-results manifest.

The library reads a single `params.json` (layer table + the unified DRC rule superset + PEX
process constants) and exposes `run_drc`, `run_lvs`, `run_pex`, and the fail-closed
`run_signoff_suite` — over a
struct-of-arrays geometry core that you can also build directly (no GDS required).

Docs: [writing params.json](docs/params-json.md) · [DRC/LVS/PEX rules & algorithms](docs/verification-rules.md) · [library usage without GDS](docs/library-usage.md)

## Layout

```
Cargo.toml
conformance/
  generator/                Rust generator: emits conformance.gds + manifest.json + params.json
  conformance.rs            harness: all four checkers vs the manifest, exit 0 iff green
  perf.rs                   CPU vs GPU DRC baseline pair (identical-report assert)
  klayout_compare.py        runs KLayout per case, diffs against the manifest
src/geometry.rs             DOD core: SoA GeometryStore, edges, distance/point-in-poly primitives
src/params.rs               params.json loader; unified DRC rule superset (tagged union) + ERC thresholds
src/schema.rs               VerifySchema: what a PDK must provide
src/drc/                    DRC engine: x-sweep candidate pruning + one geometric kernel per rule
src/lvs/                    connectivity (union-find) + device extraction + graph compare + hierarchy
src/erc/                    electrical rule checks (14 checks, thresholds from params.json "erc")
src/pex/                    analytical R / area+fringe C / lateral + interlayer coupling C
src/signoff/                antenna, density/CMP, IR, EM, reliability, ESD/latch-up analyses
src/gds.rs                  GDSII reader: BOUNDARY/PATH/BOX/TEXT + SREF/AREF flattening w/ transforms
src/traits.rs               VerifyCheck trait + CubeCL GPU kernels (feature "gpu") + CPU fallback
```

## Build & run

```bash
# 1. (re)generate the suite
cargo run --release --bin generate_conformance -- conformance

# 2. run the conformance harness (from verify/)
cargo run --release --bin conformance -- conformance          # CPU
cargo run --release --features gpu --bin conformance -- conformance   # CUDA prefilter

# 3. CPU vs GPU wall-clock pair on a dense layout (GPU needs the flake devShell)
cargo run --release --features gpu --bin perf -- 2000
```

Audited current tail (2026-07-12): `== summary: 162 passed, 0 failed ==`.
Coverage reports 51/51 rule labels with a negative-direction test. That is a useful
unit-regression gate, not marker/value/topology correlation or foundry signoff
qualification. See the [proprietary signoff audit](docs/proprietary-signoff-audit.md) and
the gated [compliance execution roadmap](docs/compliance-roadmap.md).

The GPU path needs `libcuda` (driver), `libnvrtc`, and CUDA headers (`CUDA_PATH`) at runtime;
the top-level flake devShell exports all three. Missing driver/toolkit degrades silently to
the CPU path — even mid-run.

## Versus KLayout

```bash
python3 conformance/klayout_compare.py conformance   # needs klayout (in the flake devShell)
```

Current result: **75/75 detection agreement (100%) on every rule KLayout can express or script
(hierarchy, paths, 45° geometry, PRL/wide/EOL spacing, densities, cheesing, redundant vias),
plus the rule categories it cannot** (poly-past-diff extension, off-grid vertices, windowed min/max
density) that gdsverify checks from the same `params.json`. The conformance manifest also pins
exact measured values (gap widths in DBU), which the harness asserts — not just counts.

The suite includes adversarial geometry that bbox-approximation checkers get wrong (and this
engine gets right): interlocking-L spacing with overlapping bboxes, U-shape notch vs width
disambiguation (interior/exterior facing-gap classification), L-shaped-outer enclosure, and
multi-window density.

Performance vs KLayout (RTX 4060 machine, `conformance/perf.rs` vs
`conformance/klayout_perf.py`, identical geometry, violation counts equal):

| workload | gdsverify CPU | gdsverify GPU | KLayout |
|---|---|---|---|
| combs:2000 (64M edge pairs, clean) | 21 ms | 4 ms | 67 ms |
| combs:4000 (256M edge pairs, clean) | 44 ms | 12 ms | 139 ms |
| bars:100k (43k violations) | 55 ms | 50 ms | 684 ms |

The CPU wins come from algorithm, not hardware: axis-adaptive sweep for candidate
pairs and cutoff-bounded grid buckets for edge-pair distance (exact below the rule
limit, sentinel above). The GPU then multiplies the remaining dense clean-geometry
work 3-5x; violation-heavy layouts run at CPU speed because exact rechecks are
irreducible. `klayout_perf.py` re-times the same geometry in KLayout.

## Design notes

**Data-oriented core.** `GeometryStore` holds flat `verts_x`/`verts_y` arrays; a polygon is an
index range (`poly_vert_start`/`poly_vert_len`), not an object, and shapes reference each other
by `u32` index rather than pointer. Rules are a tagged union matched in a tight loop.

**Unified DRC superset.** Every PDK obeys the same rule set; a rule is disabled explicitly with
`enabled:false`. Missing, zero, negative, non-finite, or out-of-range required parameters are deck
construction errors. Rule IDs are independent of rule kinds, so a deck can configure multiple
instances of one kind on different layers. 28 configured rule types are implemented (width,
spacing same/diff, enclosure, extension, area, max-width, notch, edge-length, off-grid, angle,
min/max density, overlap, corner-to-corner). Pairwise rules share an O(P log P) x-sweep
candidate generator; density accumulates per-window in one pass over the polygons.

**Analytical PEX.** No field solver: every supported rectilinear conductor receives
`Rs·Leq/Weq` resistance and `Ca·A + Cf·P` ground capacitance; lateral coupling is
`Ck·Lp·(Sref/S)`. Unsupported conductor geometry creates an extraction diagnostic and
makes `PexReport::is_complete()` false. Fill and shielding are not inferred from shape
size or hard-coded layer names; they require future process-stack parameters.

**LVS.** Union-find connectivity uses exact supported rectilinear overlap/contact after a
bbox candidate filter, with cross-layer joins bounded by declared via relations
(poly-over-channel is a gate, not a short). Device recognition requires one unambiguous
type implant; unsupported all-angle geometry and missing/conflicting implants stop
extraction. Production hierarchy, body-pin comparison and foundry-complete device
properties remain roadmap work.

**GPU (feature `gpu`).** Kernels are written once in Rust with CubeCL's `#[cube]` — e.g.
`pt_seg_d2` mirrors `geometry::point_seg_dist2` line for line — and compiled at runtime
for the selected backend (CUDA here; flip cubecl's cargo feature for ROCm/WGPU). Every
scan runs as a *conservative prefilter*: the GPU clears everything comfortably clean, the
CPU exact-rechecks (integer math, same code as the pure-CPU path) only near-threshold
work, so reports are bit-identical with or without a device.

GPU rule coverage — every rule with a geometric inner loop:

| kernel                | rules                                        |
|-----------------------|----------------------------------------------|
| `edge_pair_dist2`     | min_spacing, min_spacing_diff, corner_to_corner, min_enclosure |
| `facing_gap`          | min_width, notch (per-polygon clean mask)    |
| `edge_len2`           | min_edge_length                              |
| `angle_dev`           | angle                                        |
| `offgrid` (exact i32) | off_grid                                     |

The remaining rules (min_area, max_width, min_extension, overlap, min/max density) and the
PEX formulas are O(1)-per-item bbox/window arithmetic with no inner loop — a GPU round
trip costs more than the compute, so they stay on the CPU by construction, not omission.

`Backend::Gpu` is workload-aware per rule and per layer: below a measured break-even
(`GPU_MIN_PAIR_WORK` / `GPU_MIN_LINEAR_WORK` in drc.rs) the scan stays on the CPU, which
finishes before a GPU round trip would start. So requesting the GPU is always safe: sparse
or violation-heavy layouts run at CPU speed, dense clean candidate sets (interlocked combs,
`perf.rs combs:N`, where bbox pruning is structurally useless) run 3-5x faster than the
already-sweep-pruned CPU path — see the table above. Diagnose GPU fallbacks with
`GDSVERIFY_GPU_LOG=1`.
