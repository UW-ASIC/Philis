# Current status and remaining work

Status date: 2026-07-12. Code baseline for this document set:
`8b5895a431727d16721466e65b0735ca82bf0c61`, the reviewed Wave 2 and Wave 4
Batch A integration point including the checked GDS-to-production-LVS hierarchy
adapter. The adapter is an accepted `partial` supported subset on this baseline;
the Wave 3 fixes remain **integration pending**.

## Capability state vocabulary

Use these labels consistently in code review and status changes:

| State | Meaning | May increase score? |
|---|---|---:|
| `absent` | No qualifying implementation foundation exists for the capability family. | No |
| `foundation` | Shared types or algorithms exist and have focused local tests, but general consumers or end-to-end use are incomplete. | No |
| `partial` | A documented subset works on general supported input and fails closed outside it; important production semantics remain. | No, unless already present in the audit baseline |
| `integration_pending` | Work exists on an unmerged or unaccepted stream, or a required cross-module adapter/gate is missing. | No |
| `correlated` | General supported input has independent marker/topology/value correlation within reviewed bounds. | Yes, after evidence review |
| `qualified` | The exact engine/deck/model/corpus combination is accepted by the foundry or tapeout owner. | Records qualification; never inferred from tests |

The audited score remains **24.5/112 (22%)**:

| Engine | Score | Denominator | Assessment |
|---|---:|---:|---|
| DRC | 12.0 | 44 | useful in-design checker; not signoff-equivalent |
| LVS | 8.0 | 36 | flat/small-layout core plus production foundations; not production LVS |
| PEX | 4.5 | 32 | analytical estimator; not signoff extraction |

## Settled foundations

The settled baseline before Batch A passed 78 library tests, all 162 conformance
cases with 51/51 negative-direction coverage entries, downstream `pnr-core` and
`pnr-backend` compile gates, 18 backend tests, and 16 correlation-harness tests.
Batch A's Wave 2 plus Wave 4 integration point passes 137 library tests, the same
162 conformance cases, and downstream compile gates. These are local regression
results, not independent correlation.

Implemented foundations include:

- fail-closed antenna, density/CMP, IR-drop, electromigration, reliability, and
  ESD/latch-up analysis cores that consume typed evidence;
- stable DRC rule IDs separated from rule kinds, strict recognized-object deck
  validation, GDS unit validation, and visible unmapped-layer diagnostics;
- exact `i128` predicates and validated rings/polygons, including exact
  rectilinear union/intersection/subtraction with holes;
- checked, lossless GDS records and hierarchy; orthogonal hierarchy indexing and
  deterministic tiling; an explicitly declared but intentionally partial OASIS subset;
- a fail-closed SPICE/CDL parser foundation, body-aware production LVS records,
  deterministic comparison/witness structures, parameter binding, and a checked
  GDS hierarchy adapter with explicit text/property evidence, black boxes,
  equated cells, physical flatten correlation and source/hierarchy provenance;
- analytical PEX in which every supported rectilinear conductor receives R and
  ground C, incomplete extraction is diagnostic, and exported lumped networks do
  not contain dangling parasitic nodes;
- a neutral deterministic correlation/freeze exchange format.

## Immediate Batch A completion gate

Do these before starting shared Wave 5 or Wave 6 integration:

1. Land and independently re-review the Wave 3 fixes. Review has identified
   seven blocking classes: one-DBU derived-layer offset loss; bounded-coloring
   stitch/search-limit errors; dropped invalid-geometry markers; stale waiver
   dispositions; missing ancestor invalidation; bbox/nested-polarity false-cleans;
   and invalid density ranges.
2. Preserve the accepted checked GDS hierarchy to `HierLayout` adapter gate: it
   preserves hierarchy paths and explicit text/property evidence, supports the
   declared orthogonal SREF/AREF subset, rejects ambiguity, and never invents ports.
3. Cherry-pick only reviewed feature/fix commits into the fan-in branch. Exclude
   formatting-only detours and unrelated changes.
4. Run every [global integration gate](contributing/test-gates.md#global-integration-gate)
   on the combined tree.
5. Update only the dated test counts and the two pending labels in this page if
   the combined gate passes. Do not promote a capability score.

## Remaining roadmap at a glance

| Wave | Remaining outcome | Primary dependency | External dependency |
|---|---|---|---|
| 2 | complete all-angle geometry, exact PATH semantics, full declared OASIS subset, lossless checked adapters, hierarchical seam equivalence | accepted Batch A geometry/layout APIs | second geometry engine and format corpora |
| 3 | execute production conditional/table-driven DRC semantics exactly; calibrated fill/CMP; complete decomposition; deterministic incremental/distributed result flow | Wave 2 kernel and hierarchy | selected foundry deck and golden markers |
| 4 | complete production netlist dialect, layout label/port binding, foundry devices/properties, true hierarchy, symmetric reductions, capacity | Wave 2 layout adapter | selected foundry device deck and golden LVS |
| 5 | distributed terminal-aware multi-corner PEX graph, calibrated RC/RLC/substrate/thermal models, DSPF/SPEF | Waves 2 and 4 terminal identity | process stack, field solver, golden extractor |
| 6 | derive all typed signoff evidence from layout/netlist/activity/mission profiles with end-to-end provenance | Waves 4 and 5 | foundry reliability models and qualification inputs |
| 7 | large corpus, vendor adapters, full-chip capacity, restart/incremental/distributed qualification, frozen accepted release | Waves 2–6 | licenses, golden runs, tapeout-owner/foundry approval |

The detailed packages, file ownership, tests, and gates are in the
[roadmap index](compliance/roadmap.md).
