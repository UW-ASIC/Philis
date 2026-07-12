# DRC engine

## Current path

[`drc/mod.rs`](../../../verify/src/drc/mod.rs) resolves typed `DrcRuleParam`s and runs
CPU kernels or conservative GPU prefilters. Violations carry stable rule ID and kind,
layer, measured value, limit and location. Pairwise candidate pruning uses sweeps;
near-threshold GPU candidates are exact-rechecked on CPU.

The legacy/current set covers 28 configured rule kinds: core width/spacing/area/
enclosure/extension, grid/angle/density/overlap/corner, antenna/CAR, EOL/PRL/wide
spacing, asymmetric enclosure, enclosed-area/cheesing, redundant/via-array/tap,
multi-patterning, plus polygon validity policy. Presence is not production semantics:
several forms remain single-threshold, rectangle/fragment/group heuristics.

The checked signoff density and antenna families live under [`signoff/`](../../../verify/src/signoff/)
and should not be conflated with their simpler legacy DRC counterparts.

## Batch A integration-pending work

The Wave 3 stream adds exact derived layers, typed production deck/context foundations,
bounded coloring, fill/result DB/invalidation. It is not accepted at this baseline.
Seven blocking defect classes are listed in [current status](../status.md); all require
focused regressions and independent re-review.

## Production gap

Required remaining semantics include general conditional/table-driven measurement;
same/different-net, voltage, region, cell and hierarchy contexts; exact general polygon
width/spacing/PRL/EOL/enclosure/cut/holes; complete bounded decomposition; calibrated
fill/CMP; stable cross-probe result DB; ancestor-aware incremental invalidation; and
deterministic restart/distributed execution.

Every selected foundry deck operation must have a schema entry, implementation,
unknown/negative test and golden marker. A parsed but ignored field is a deck-load bug.
See [Wave 3](../work-packages/wave-3.md).

## Open GPU performance backlog

These items are migrated from the retired conformance TODO. They are capacity work,
not semantic-compliance evidence, and require fresh measurements before changing a
threshold:

1. Replace the GPU far-mask's full `|Ea| * |Eb|` product with grid-localized edge-pair
   input so its work scales like the CPU grid path. On the historical RTX 4060 run,
   the CPU overtook the brute-product GPU near `combs:16000` fingers.
2. Evaluate vectorized `Line<f32>` loads and shared-memory tiling for the remaining
   throughput headroom. Historical eight-evaluations-per-thread tiling reduced occupancy
   and was reverted; do not reintroduce it without a focused benchmark.
3. Replace fixed `GPU_MIN_PAIR_WORK` / `GPU_MIN_LINEAR_WORK` values with a deterministic
   calibration probe and publish the resulting hardware/runtime metadata.

Historical context only: repeat uploads were about 1 µs, initial upload about 79 µs,
and the pair kernel about 25 billion evaluations/s on that RTX 4060. These numbers are
not a current capacity envelope; Q7.3 owns reproducible qualification measurements.
