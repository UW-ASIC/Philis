# Placement operational behavior and limitations

## What counts as success

`run_placement` always returns after the configured stage budgets or an earlier
convergence condition. It does not define a Boolean success flag. A caller
normally evaluates at least:

```rust
let legal = result.report.overlap_final < tolerance_nm2
    && result.report.validation.hard_violations.is_empty()
    && result.report.validation.emitted == 0
    && result.report.validation.consumed == 0;
```

Choose the overlap tolerance deliberately: `overlap_final` measures virtual
planning footprints, not just drawn cells. Also inspect output bounds when
fixed axes or oversized cells are possible.

The integrated flow additionally requires clean routing and signoff; placement
alone is not a tapeout guarantee.

## Non-fatal outcomes

- Global, detailed, and refinement stages may stop at their iteration budgets.
- Residual virtual overlap is returned and reported.
- Hard placement contracts may remain violated.
- Invalid constraint references are usually dropped and warned about rather
  than returned as errors.
- Debug writes requested through `PlacementConfig::debug_dir` may fail; the
  failure is logged and the placement is still returned.
- Compaction can be rejected; the pre-compaction state is retained and still
  halo-padded into a final die.

## Panics and malformed inputs

- A cell-count/size-count mismatch triggers an unconditional assertion.
- A zero or negative `grid` makes die rounding invalid and can divide by zero.
- A non-empty short `layer_masks` vector can be indexed out of bounds.
- Invalid/empty common-centroid groups can make ledger indices disagree with
  the filtered engine groups.
- Contradictory symmetry membership can corrupt the intended partner/group
  relation without a validation error.
- Extremely large geometry, cell counts, or variant counts can overflow the
  `i32`, `u32`, or `u16` storage used at API boundaries.
- NaN/negative weights and malformed dimensions are not comprehensively
  rejected and can poison ordering, objective, or geometry math.

Validate these conditions at the caller boundary when input is not produced by
the canonical frontend.

## Geometry and objective limitations

- Placement geometry is axis-aligned rectangular footprint geometry. Polygonal
  shape, holes, per-layer outlines, and blockages are reduced to a size plus a
  coarse layer-presence mask.
- Disjoint layer masks allow cells to overlap completely. This is intentional
  for non-conflicting layers but is only as correct as the masks supplied.
- A disjoint mask makes the ledger gap effectively infinite: minimum-gap and
  DTI far-branch checks pass, while proximity/thermal maximum-gap checks fail.
  Detailed hard push checks do not apply this same layer short-circuit, so
  search-time and final-check behavior can differ.
- Layer masks are per cell, not per variant. A reshape that changes a variant’s
  layer composition continues to use the originally supplied mask.
- A one-entry `variant_sizes` row does not enter the engine’s reshape table,
  but its entry can still replace `Placement::sizes` at output. Keep that entry
  identical to the input size if only one candidate exists.
- Routing feedback inflation changes optimization footprints but not returned
  drawn sizes. Consequently reported overlap can exist even when drawn
  rectangles do not overlap.
- Global placement uses cell-centre HPWL even when physical pins are supplied;
  physical offsets begin affecting the detailed stage.
- Placement pin data represent one offset per cell/net lookup. Multiple access
  shapes are a routing concern.
- Pin-aware HPWL is disabled unless at least one first-variant pin survives net
  remapping into the flat base table.
- The initial canvas includes the selected input sizes and margin but does not
  account for every alternative variant or routing-feedback inflation.
- Ordinary overlap is penalized, not forbidden, and therefore has no absolute
  legality guarantee.
- The common-centroid `CommonCentroid2d` pattern currently reuses a 1-D ABBA
  x-slot pull.
- Thermal constraints are handled by proximity heuristics, not a thermal field
  solver.
- Stress appears in full objective evaluation but is absent from both the
  analytical gradient and incremental SA delta, so current search does not
  actively optimize it.
- Explicit pair-gap soft cost is likewise absent from incremental SA delta;
  its effective detailed-stage enforcement comes from non-worsening legality.
- Proximity’s `min_distance_um` is implemented as a pull radius and final
  maximum gap, despite its name.
- The dense device-class spacing table is currently zero everywhere; explicit
  isolation provides the real nonzero spacing model.
- Direct-connect rules are trusted. The placer itself does not qualify them
  against a deck.
- Final distance contracts use base input dimensions rather than reshape-aware
  selected-variant dimensions, although overlap and compaction use the latter.
- `Telemetry::best_cost` does not identify a retained best-state snapshot; the
  engine returns the last committed state of each stage.

## Contract and constraint limitations

- Only placement-created contracts appear in `PlacementReport`; ignored and
  routing/signoff-only record families are absent.
- Self-symmetric group members do not get a standalone contract.
- Guard-ring placement consumes a contract whose declared lifecycle normally
  belongs to routing/signoff, because the placer reserves ring space early.
- Multiple guard-ring neighbour checks share one contract and are reconciled
  sequentially. A later passing check can leave the visible contract satisfied
  even if an earlier neighbour failed, while the ledger open count still saw
  the failure during that reconciliation. `ConstraintContract::satisfy` does
  not clear an older violation metric, so the resulting satisfied contract can
  also retain and print the earlier metric.
- Hard `Emitted` or `Consumed` contracts do not appear in
  `ContractValidation::hard_violations`; callers seeking complete coverage must
  inspect lifecycle counts as well.
- Satisfied detail text always refers to the generic `2 * grid` tolerance even
  when the underlying check uses another threshold.
- More generally, lifecycle transitions to `Satisfied` do not clear an older
  violation metric/units. Treat metrics as current only when status is
  `Violated`, and use history for prior failures.

## Reporting limitations

- Refinement telemetry is not stored in `PlacementReport` and has no trace
  artifact.
- `hpwl_initial` and `hpwl_final` include net weights and feedback multipliers;
  the display’s `nm` label should be read as weighted-nm cost, not raw total
  wirelength.
- `overlap_final` includes margin/inflation and excludes qualified layer-mask
  and abutment pairs.
- `placement.txt` omits orientations, selected variants, symmetry axes,
  planning margin, and feedback inflation.
- The display line labelled “unconsumed” prints only contracts still
  `Emitted`; it does not print the separate unresolved `Consumed` count.

## Fixed-axis caveats

A fixed axis is an absolute x coordinate. Final recentering therefore leaves x
unchanged for the entire placement whenever any fixed group exists. This can
leave unused space to the left, and a malformed/out-of-range fixed axis can
produce out-of-bounds geometry. The y dimension is still recentered. The final
die width is based on the absolute maximum x plus halo.

The analytical stage does not update a fixed axis, and compaction freezes its
cluster in x. However, the detailed/refinement whole-symmetry-island move
currently translates the axis without checking the fixed flag. No ledger check
compares it with the requested absolute coordinate. A fixed axis can therefore
drift during SA even though later stages preserve its resulting coordinate.

## Debugging checklist

1. Check stderr for dropped references, per-stage costs, compaction rejection,
   and hard violations.
2. Inspect `contracts.txt` for lifecycle coverage and metrics, not only the
   display summary.
3. Compare `global_trace.csv` and `detailed_trace.csv` for overflow and
   freeze-out. Remember that refinement is absent from the traces.
4. Compare drawn rectangles from `placement.txt` with
   `report.overlap_final`; a discrepancy often comes from margin, inflation,
   layer masks, selected variants, or legal abutments.
5. Verify that `sizes[i]`, `initial_variants[i]`, variant pin rows, and layer
   masks describe the same generated candidate.
6. If routing feedback appears ineffective, confirm exact net-name keys and
   inspect the multipliers/inflation passed into the next iteration.

## Tests in the crate

The placement crate’s unit tests cover:

- OTA symmetry, bounds, non-overlap, HPWL improvement, and contract closure;
- legality and compactness across multiple seeds;
- guard-ring spacing across seeds;
- a hard isolation gap;
- stress and DTI contract reconciliation;
- debug artifact creation.

The engine also tests bucket sizing for the largest reshape variant. These are
regression checks, not formal proof for arbitrary malformed constraints or PDK
geometry.
