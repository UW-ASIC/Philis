# Current status and remaining work

Status date: 2026-07-12. Code baseline for this document set:
`f646d9c5b8b17fbab775a25f0fc499a873678add`, the reviewed Batch A integration
point for the Wave 2/4 checked GDS-to-production-LVS hierarchy adapter and the
accepted Wave 3 DRC foundations. The adapter and supported Wave 3 execution paths
are accepted `partial` subsets; infrastructure without an end-to-end consumer is
accepted as `foundation`. Neither state implies correlation or qualification.

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
The combined Batch A baseline passes 175 library tests, the same 162 conformance
cases with 51/51 negative-direction coverage, and the downstream compile gates.
These are local regression results, not independent correlation.

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
- exact rectilinear derived-layer operations, a strict typed DRC schema/context
  facade, a bounded complete coloring solver with precolors, stitches and explicit
  search-limit outcomes, deterministic legal fill/recheck, stable result fingerprints,
  waiver lifecycle, ancestor-aware invalidation and deterministic tile merge;
- analytical PEX in which every supported rectilinear conductor receives R and
  ground C, incomplete extraction is diagnostic, and exported lumped networks do
  not contain dangling parasitic nodes;
- a neutral deterministic correlation/freeze exchange format.

## Accepted Batch A integration gate

The accepted baseline records these completed integration controls:

1. The seven Wave 3 blocker classes have focused regressions and independent
   re-review: derived offsets, coloring stitches/search limits, invalid-geometry
   propagation, waiver lifecycle, ancestor invalidation, exact overlap/nested
   polarity, and density ranges.
2. The accepted checked GDS hierarchy to `HierLayout` adapter gate remains intact: it
   preserves hierarchy paths and explicit text/property evidence, supports the
   declared orthogonal SREF/AREF subset, rejects ambiguity, and never invents ports.
3. Fan-in contains the reviewed semantic sequence and excludes its formatting-only
   detour/revert.
4. The [global integration gate](contributing/test-gates.md#global-integration-gate)
   passes on a freshly generated corpus.
5. The capability score remains frozen because no independent correlation evidence
   accompanied this integration.

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
