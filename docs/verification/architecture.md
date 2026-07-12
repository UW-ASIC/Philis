# Verification architecture and source map

## System flow

```text
PDK JSON ──> VerifySchema/Deck ──────────────────────────────────────────┐
                                                                         │
GDS ──> lossless GdsLibrary ──> checked flatten/hierarchy adapter ─┐     │
OASIS ──> declared supported subset ───────────────────────────────┤     │
direct caller GeometryStore ───────────────────────────────────────┤     │
                                                                  v     v
                    exact geometry + hierarchy index/tiles + typed policy
                         │                  │                 │
                         v                  v                 v
                       DRC          layout extraction       PEX graph/model
                                          │                 (currently scalar)
SPICE/CDL/Spectre subset ─> AST/binding ──>│
                                          v
                              production LVS compare/witness
                                          │
layout + LVS + PEX + activity/mission ────> reliability evidence adapters
                                          │
                                          v
                               signoff four-state reports
                                          │
                                          v
                              neutral correlation/freeze artifacts
```

The checked production path is intentionally layered. A lossless parser preserves
what was present. A verification adapter decides whether the input is representable
and valid for the declared check. Checker code consumes typed, unit-bearing data and
must not reinterpret raw records or silently guess missing identity.

## Module boundaries

| Module | Responsibility | Must not do |
|---|---|---|
| [`schema.rs`](../../verify/src/schema.rs), [`params.rs`](../../verify/src/params.rs) | strict recognized-object parsing, layer/model/rule resolution, typed deck | parse and ignore unknown operations; hide missing units/refs |
| [`gds_lossless.rs`](../../verify/src/gds_lossless.rs) | lossless GDS records, checked read/write, validation, transforms, flattening, PATH stroking | invent net/port semantics |
| [`gds.rs`](../../verify/src/gds.rs), [`lib.rs`](../../verify/src/lib.rs) | compatibility and strict deck-aware GDS loading, mapped stores, units/unmapped diagnostics | silently rescale or discard unmapped geometry |
| [`oasis.rs`](../../verify/src/oasis.rs) | explicit OASIS supported subset and conversion | accept undeclared records/modal behavior |
| [`geometry/exact.rs`](../../verify/src/geometry/exact.rs) | validated rings/polygons, exact predicates and rectilinear booleans | checker-specific policy or bbox substitutes |
| [`geometry.rs`](../../verify/src/geometry.rs) | flat SoA `GeometryStore`, edges and legacy/shared primitives | represent lossless hierarchy/properties |
| [`hierarchy_index.rs`](../../verify/src/hierarchy_index.rs) | transformed hierarchical candidates, deterministic tiles/halos | decide rule semantics or electrical identity |
| [`drc/mod.rs`](../../verify/src/drc/mod.rs) | resolved-rule execution and markers | private final geometry approximations after shared API exists |
| [`lvs/netlist.rs`](../../verify/src/lvs/netlist.rs), [`binding.rs`](../../verify/src/lvs/binding.rs) | reference syntax, includes, parameters/models and bound hierarchy | inspect layout geometry |
| [`lvs/extract.rs`](../../verify/src/lvs/extract.rs), [`detailed_extract.rs`](../../verify/src/lvs/detailed_extract.rs) | layout connectivity/device/property evidence | silently default ambiguous devices/ports |
| [`lvs/production.rs`](../../verify/src/lvs/production.rs), [`hier_production.rs`](../../verify/src/lvs/hier_production.rs) | deterministic comparison, mappings, witnesses and abstract hierarchy | manufacture missing layout evidence |
| [`pex/mod.rs`](../../verify/src/pex/mod.rs) | current analytical extraction and checked completeness | claim field-solver/distributed topology semantics |
| [`signoff/`](../../verify/src/signoff/) | typed antenna/CMP/power/reliability/ESD-latch-up analysis cores | infer absent extraction/stimulus/model inputs as clean |
| [`src/bin/correlation.rs`](../../verify/src/bin/correlation.rs), [`correlation/`](../../verify/correlation/) | neutral validation, compare, dispositions, freeze | run proprietary tools or declare foundry acceptance |

## Public API layers

The crate root in [`verify/src/lib.rs`](../../verify/src/lib.rs) exposes three tiers:

1. Compatibility entry points such as `load_gds` and legacy `run_lvs`. They preserve
   constrained historical behavior and are not automatically suitable for signoff.
2. Checked input/extraction entry points such as `load_gds_strict`,
   `read_gds_checked`, `run_pex_by_net_checked`, `extract_detailed_netlist`, and
   production LVS functions. New signoff integrations use these.
3. Low-level objects (`GdsLibrary`, `GeometryStore`, exact polygons, detailed
   netlists, hierarchy indexes) for explicit orchestration. Callers own validation
   choices not performed by their selected entry point.

API additions that carry identity, diagnostics, units, or status are semantically
visible even if Rust compilation remains source-compatible. Downstream gates for
`pnr-core` and `pnr-backend` are required for every public change.

## Identity and provenance joins

The hardest integration boundaries are where one representation joins another:

- GDS/OASIS records to geometry and hierarchy paths;
- text/properties to LVS ports/nets/devices;
- LVS device terminals to PEX nodes;
- PEX/layout/netlist/activity to signoff evidence;
- native reports to neutral correlation artifacts.

Treat each as a checked adapter with a dedicated owner and ambiguity tests. A correct
producer and correct consumer can still form an unsafe system if the adapter collapses
hierarchy, units, properties, source paths, or error status.

## Test placement

Most module tests are inline under `#[cfg(test)]`. Cross-checker cases live in
[`verify/conformance/`](../../verify/conformance/); neutral exchange fixtures and
schemas live in [`verify/correlation/`](../../verify/correlation/). Add the smallest
reproducer beside the owning algorithm, then a cross-module case when the defect
crosses an adapter or public facade. The full required commands are in
[test gates](contributing/test-gates.md).
