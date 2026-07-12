# Compliance implementation roadmap

The target is feature-complete enough to seek qualification for a selected process,
not an abstract “all foundries” claim. Work proceeds by dependency, with independent
streams only where ownership does not overlap.

## Dependency order

```text
Batch A finalization
  ├─ Wave 2 exact geometry + checked hierarchy/formats
  │    ├─ Wave 3 production DRC
  │    └─ Wave 4 production LVS
  │          └─ Wave 5 distributed multi-corner PEX
  │                  └─ Wave 6 derived reliability evidence
  └───────────────────────────────┬───────────────
                                  └─ Wave 7 correlation/capacity/qualification
```

Wave 7 corpus and adapter scaffolding may proceed alongside engine work, but no
capability is promoted from synthetic/self-comparison evidence. Wave 6 analyzers can
be unit-tested independently; their extraction integration waits for stable Wave 4
device/terminal identity and Wave 5 electrical graphs.

## Work-package index

| Wave | Detailed packages | Exit target |
|---|---|---|
| 2 | [exact geometry and preserved hierarchy](../work-packages/wave-2.md) | DRC ≥24/44 and LVS ≥15/36 only after correlation |
| 3 | [production DRC](../work-packages/wave-3.md) | DRC ≥38/44 only after selected-deck correlation |
| 4 | [production LVS](../work-packages/wave-4.md) | LVS ≥31/36 only after selected-deck correlation |
| 5 | [distributed multi-corner PEX](../work-packages/wave-5.md) | PEX ≥27/32 only after field-solver/golden correlation |
| 6 | [reliability evidence integration](../work-packages/wave-6.md) | every result traceable to geometry, identity, stimulus/corner, model and limit |
| 7 | [correlation, capacity and qualification](../work-packages/wave-7.md) | exact frozen combination accepted by tapeout owner/foundry |

Each package defines read-first files, prerequisites, owned/forbidden files, API
outcome, tasks, explicit unsupported scope, four-direction tests, focused/full gates,
acceptance evidence, score policy, and parallel/fan-in hazards. An implementation
branch that omits any field is incomplete even if its unit tests pass.

## Dependency gate IDs

Work-package prerequisite rows use these stable non-package gates. A gate is satisfied
only by the named evidence; a verbal assumption is not sufficient.

| Gate ID | Required evidence |
|---|---|
| `BA-W3-REVIEW` | satisfied on the documented Batch A baseline: all seven Wave 3 review blockers fixed, independently re-reviewed, and accepted on fan-in |
| `BA-GDS-LVS-ADAPTER` | checked GDS-to-`HierLayout` adapter accepted with ambiguity/identity regressions |
| `EXT-DECK` | selected foundry DRC deck revision and complete operation inventory available under approved access |
| `EXT-NETLIST` | selected SPICE/CDL/Spectre dialect corpus and include/security policy fixed |
| `EXT-DEVICE` | selected foundry LVS device-recognition deck, models, properties and tolerances available |
| `EXT-PROCESS` | versioned conductor/dielectric/corner/fill/thermal process models available |
| `EXT-ACTIVITY` | approved vector, vectorless, waveform and mission-profile inputs with provenance available |
| `EXT-RELIABILITY` | foundry reliability equations, limits, validity domains and qualification structures available |
| `EXT-LICENSE` | licensed independent tool/field-solver execution environment and legal data-handling plan approved |
| `EXT-GOLDEN` | pinned independent golden artifacts and review-approved error-bound policy available |
| `EXT-SIGNING` | organizational release owner and signing/identity policy approved |
| `EXT-QUALIFICATION` | foundry/tapeout-owner submission program, acceptance criteria and authorized approver identified |

## Wave gates

At each wave boundary:

1. audit diffs for file ownership, public API compatibility, unit/identity preservation,
   deterministic ordering, and fail-closed behavior;
2. require focused positive/negative/boundary/adversarial evidence;
3. run the [global integration gate](../contributing/test-gates.md#global-integration-gate);
4. compare full marker geometry, topology and values where a golden is available;
5. record unsupported operations explicitly and stop deck/input loading when selected
   scope uses one;
6. update status without promoting the frozen score unless independent evidence is attached.

## Qualification target

All six gates are required for one exact release:

| Gate | Evidence |
|---|---|
| semantic coverage | every operation/context/table/property/device/reduction used by the selected deck has schema, implementation, negative test and marker/topology/value case; unknowns error |
| numerical correctness | positive, negative and boundary structures correlate to an independent implementation with reviewed tolerances |
| capacity | declared hierarchical designs complete within deterministic runtime/memory envelopes; restart/incremental/distributed modes exercised |
| debug workflow | stable IDs, result database, hierarchy paths, cross-probe objects and waiver provenance |
| reliability | derived evidence and qualified limits for antenna, CMP/density, EM/IR, voltage/aging, ESD and latch-up |
| external acceptance | foundry/tapeout owner accepts exact engine/deck/model/corpus versions and dispositions |
