# gdsverify — GDS verification core (DRC / LVS / PEX)

A data-oriented, optionally GPU-accelerated GDS verification library in Rust, plus a Python
conformance-suite generator that emits GDS + a machine-readable expected-results manifest.

The library reads a single `params.json` (layer table + the unified DRC rule superset + PEX
process constants) and exposes three entry points — `run_drc`, `run_lvs`, `run_pex` — over a
struct-of-arrays geometry core that you can also build directly (no GDS required).

Docs: [writing params.json](docs/params-json.md) · [DRC/LVS/PEX rules & algorithms](docs/verification-rules.md) · [library usage without GDS](docs/library-usage.md)

## Layout

```
Cargo.toml
CONFORMANCE_PLAN.md         the suite design / contract
conformance/
  generator.py              emits conformance.gds + manifest.json + params.json (needs gdstk)
  klayout_deck.drc          equivalent KLayout DRC deck (head-to-head)
  klayout_compare.py        runs KLayout per case, diffs against the manifest
src/geometry.rs             DOD core: SoA GeometryStore, edges, distance/point-in-poly primitives
src/params.rs               params.json loader; unified DRC rule superset (tagged union)
src/drc.rs                  DRC engine: x-sweep candidate pruning + one geometric kernel per rule
src/lvs.rs                  connectivity (union-find) + device extraction + graph compare
src/pex.rs                  analytical R / area+fringe C / lateral coupling C
src/gds.rs                  minimal GDSII binary reader (std-only)
src/rule.rs                 Rule / Kernel traits
src/gpu.rs                  CubeCL GPU backend (feature "gpu") + total CPU fallback
src/bin/conformance.rs      harness: all three checkers vs the manifest, exit 0 iff green
```

## Build & run

```bash
# 1. (re)generate the suite — gdstk comes from the repo flake devShell
python3 conformance/generator.py

# 2. run the conformance harness (from verify/)
cargo run --release --bin conformance -- conformance          # CPU
cargo run --release --features gpu --bin conformance -- conformance   # CUDA prefilter
```

Expected tail: `== summary: 38 passed, 0 failed ==` — identical with and without a GPU.

The GPU path needs `libcuda` (driver), `libnvrtc`, and CUDA headers (`CUDA_PATH`) at runtime;
the top-level flake devShell exports all three. Missing driver/toolkit degrades silently to
the CPU path — even mid-run.

## Versus KLayout

```bash
python3 conformance/klayout_compare.py conformance   # needs klayout (in the flake devShell)
```

Current result: **24/24 detection agreement on every rule stock KLayout DRC can express, and
4 rule categories it cannot** (poly-past-diff extension, off-grid vertices, windowed min/max
density) that gdsverify checks from the same `params.json`. The conformance manifest also pins
exact measured values (gap widths in DBU), which the harness asserts — not just counts.

The suite includes adversarial geometry that bbox-approximation checkers get wrong (and this
engine gets right): interlocking-L spacing with overlapping bboxes, U-shape notch vs width
disambiguation (interior/exterior facing-gap classification), L-shaped-outer enclosure, and
multi-window density.

Performance vs KLayout on large layouts: see `../benchmark/` (`generator.py` + `run.py`);
~12x faster end-to-end at 200k polygons with exact violation-count parity.

## Design notes

**Data-oriented core.** `GeometryStore` holds flat `verts_x`/`verts_y` arrays; a polygon is an
index range (`poly_vert_start`/`poly_vert_len`), not an object, and shapes reference each other
by `u32` index rather than pointer. Rules are a tagged union matched in a tight loop.

**Unified DRC superset.** Every PDK obeys the same rule set; a rule is disabled by setting its
parameter to 0 (or `enabled:false`) in `params.json`. 15 rule types are implemented (width,
spacing same/diff, enclosure, extension, area, max-width, notch, edge-length, off-grid, angle,
min/max density, overlap, corner-to-corner). Pairwise rules share an O(P log P) x-sweep
candidate generator; density accumulates per-window in one pass over the polygons.

**Analytical PEX.** No field solver: resistance is `Rs·L/W`; area+fringe capacitance is
`Ca·A + Cf·P`; lateral coupling is `Ck·Lp·(Sref/S)`.

**LVS.** Union-find connectivity over conductor+via layers (poly-over-diff is a gate, not a
short), device recognition from poly∩diff with implant selecting N/P, then graph comparison
against the reference schematic netlist.

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
wire fields run at CPU speed, and on dense candidate sets — the interlocked-comb dataset in
`../benchmark/` (`generator.py combs:N`), where bbox pruning is structurally useless — it's
CPU 3.7s vs GPU 0.28s steady-state (13x) on an RTX 4060; KLayout takes 5.3s on the same
layout with identical findings. Diagnose GPU fallbacks with `GDSVERIFY_GPU_LOG=1` and
`--example gpu_probe`.
