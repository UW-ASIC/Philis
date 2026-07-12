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
