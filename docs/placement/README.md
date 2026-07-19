# Placement

This directory is the implementation reference for the `pnr-placement` crate in
[`backend/placement`](../../backend/placement/). It documents the behavior that
the current code actually implements, including the `pnr-engine` placement
types that the crate re-exports. It is not a proposal for a future placer.

The crate converts a cell/net hypergraph, cell footprints, physical
constraints, and optional routing feedback into a grid-snapped placement. The
flow is deterministic for a fixed input and seed. It performs:

1. square planning-canvas estimation;
2. frontend-to-engine model construction;
3. analytical global placement;
4. symmetry-preserving detailed simulated annealing;
5. a second, narrower refinement anneal;
6. exact symmetry re-snap;
7. constraint-graph compaction and die shrink;
8. final contract reconciliation and optional artifact emission.

The returned object contains drawn cell dimensions, but optimization and
overlap reporting use virtual planning footprints that can include cell margin
and routing-feedback inflation. Keep that distinction in mind when comparing
`Placement::sizes` with `PlacementReport::overlap_final`.

## Reading guide

- [Public API and configuration](api.md) is the complete callable API,
  defaults, array contracts, outputs, and artifact formats.
- [Model and algorithms](model-and-algorithms.md) explains die sizing, the cold
  model, objectives, moves, stages, compaction, and determinism.
- [Constraint handling](constraints.md) is the exhaustive
  `ConstraintRecord`-to-placement mapping and the exact satisfaction tests.
- [Operational behavior and limitations](behavior-and-limitations.md) lists
  preconditions, non-fatal failure modes, reporting boundaries, and known
  implementation limitations.

## Minimal use

```rust
use pnr_constraints::ConstraintRecord;
use pnr_placement::{estimate_sizes, run_placement, PlacementConfig};

let sizes = estimate_sizes(&graph);
let result = run_placement(
    &graph,
    &sizes,
    &ConstraintRecord::default(),
    &PlacementConfig::default(),
    &[], // empty layer masks mean every pair can geometrically conflict
);

if !result.report.validation.hard_violations.is_empty() {
    eprintln!("{}", result.report);
}
```

`run_placement` returns a `PlacementResult`, not a `Result`. Residual overlap,
unmet hard constraints, and an exhausted optimizer budget are reported in the
result rather than converted into a Rust error. Invalid structural inputs can
still assert or panic; see [Input invariants](api.md#input-invariants).

## Units and indexing

- Geometry, coordinates, gaps, pitch, margin, halo, and wirelength-like metrics
  are in nanometres unless a field explicitly says otherwise.
- Areas are in nm².
- `SymmetryGroup::axis` enters the API in angstroms and is divided by 10 when
  the engine model is built.
- Constraint distance fields ending in `_um` enter in micrometres and are
  multiplied by 1,000.
- Cell-indexed vectors use `BipartiteHypergraph::cells` order.
- Hypergraph net IDs use `BipartiteHypergraph::nets` order. The placer removes
  nets with fewer than two distinct cells and remaps retained nets internally.
- Variant indices address the corresponding row of
  `PlacementConfig::variant_sizes`.
- Layer masks use bit `k` to indicate that a cell has geometry on PDK layer ID
  `k`; an empty mask vector selects conservative all-pairs conflict behavior.

## Source map

| Concern | Source of truth |
|---|---|
| Public façade, flow, outputs, debug files | [`backend/placement/src/lib.rs`](../../backend/placement/src/lib.rs) |
| Frontend model conversion and contract ledger | [`backend/placement/src/model.rs`](../../backend/placement/src/model.rs) |
| Placement state, objectives, moves, stages, and compaction | [`backend/engine/src/placement.rs`](../../backend/engine/src/placement.rs) |
| Generic stage driver and telemetry | [`backend/engine/src/traits.rs`](../../backend/engine/src/traits.rs) |
| Constraint record and contract lifecycle | [`backend/constraints/src`](../../backend/constraints/src/) |
| Production integration and feedback population | [`backend/src/flow.rs`](../../backend/src/flow.rs) |
