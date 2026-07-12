# Wave 2 work packages — exact geometry and preserved hierarchy

Goal: one checked layout representation and one exact geometry kernel shared by DRC,
LVS and PEX. Bboxes remain conservative candidate filters only. Current state is a
strong `foundation`: rectilinear booleans, checked/lossless GDS, hierarchy index/tiles,
and a partial declared OASIS subset exist. The checked GDS-to-production-LVS adapter
is integration pending.

## G2.1 — complete the exact polygon kernel and migrate consumers

| Field | Requirement |
|---|---|
| Read first | [`geometry/exact.rs`](../../../verify/src/geometry/exact.rs), [`geometry.rs`](../../../verify/src/geometry.rs), then `rg "poly_bbox|bbox" verify/src/drc verify/src/lvs verify/src/pex` |
| Prerequisites | None beyond the documented `b4a4965` Batch A geometry baseline; freeze the overflow/complexity policy before exposing G2.1 operations |
| Owned files | `verify/src/geometry.rs`, `verify/src/geometry/**`, and their inline `#[cfg(test)]` modules. Consumer migrations require separate commits owned by the affected package |
| Forbidden files | `verify/src/gds*.rs`, `verify/src/oasis.rs`, `verify/src/{drc,lvs,pex,signoff}/**`, `verify/conformance/**`, and `verify/correlation/**` except in separately assigned consumer/correlation commits |
| API outcome | validated integer/rational polygons with union/intersection/subtraction, holes/keyholes, edge classification, arbitrary-angle offsets and explicit complexity/overflow errors |
| Tasks | add rational intersection/offset representation; canonical winding/ring ordering; exact all-angle topology; deterministic fracture; migrate every electrical/geometric decision to shared API; retain bbox only for candidate generation |
| Unsupported/non-goals | curvilinear geometry and unbounded exact arithmetic unless selected process requires them; never fall back to bbox or floating clean on complexity limit |
| Tests | positive boolean/offset identities; negative invalid rings/self-intersection; boundary touching/coincident/1-DBU slivers; adversarial overflow, huge coordinates, winding, keyholes and randomized differential tests against a second engine |
| Focused gates | geometry unit/property tests; deterministic corpus replay; consumer-specific tests after each migration |
| Full gates | all global gates plus flat/hier and CPU/GPU identical-report checks where affected |
| Acceptance evidence | zero private decision-making bbox substitutes; differential corpus with seed/version; canonical output hashes and typed failure corpus |
| Score promotion | none from implementation; only after independent DRC/LVS marker/connectivity correlation on general all-angle input |
| Parallel/fan-in hazards | core API churn blocks G2.2/G2.4 and Waves 3–5; land types first, operations second, consumers one subsystem at a time |

## G2.2 — exact GDS semantics and checked hierarchy adapters

| Field | Requirement |
|---|---|
| Read first | [`gds_lossless.rs`](../../../verify/src/gds_lossless.rs), [`gds.rs`](../../../verify/src/gds.rs), [`lib.rs`](../../../verify/src/lib.rs), [`lvs/hier_production.rs`](../../../verify/src/lvs/hier_production.rs) |
| Prerequisites | G2.1 accepted; `BA-GDS-LVS-ADAPTER` is the exit dependency for the LVS adapter portion; identity contract in [contracts](../contracts.md) |
| Owned files | `verify/src/gds.rs`, `verify/src/gds_lossless.rs`, planned `verify/src/lvs/gds_adapter.rs`, and planned format fixtures under `verify/conformance/gds/**` |
| Forbidden files | `verify/src/lvs/{production,compare,hier_production}.rs`, `verify/src/drc/**`, `verify/src/pex/**`, `verify/conformance/manifest.json`, and `verify/correlation/**`; no proximity-based text-to-net policy |
| API outcome | lossless round trip plus strict verification adapter for exact BOUNDARY/PATH, SREF/AREF, properties/text association and hierarchy paths; typed error contains record/cell/instance evidence |
| Tasks | finish legal PATH types/width/end extensions and all-angle stroking; absolute/reflected/magnified transforms with exact representability checks; preserve PLEX/properties/text; cycle/missing-cell/array validation; build checked `GdsLibrary -> HierLayout` adapter using only declared property/text rules |
| Unsupported/non-goals | NODE electrical meaning, vendor extensions, non-integral transform output unless explicitly modeled; ambiguity is error, never guessed ports/nets |
| Tests | record-by-record round trips; malformed ordering/type/length; transform equality at representable boundaries; adversarial nested SREF/AREF, reflections, cycles, duplicate labels/properties; flattened-vs-hierarchy geometry/identity equivalence |
| Focused gates | GDS lossless/read/write/stroke/adapter tests and malformed-input fuzz replay |
| Full gates | global gates plus LVS hierarchy corpus through real adapter |
| Acceptance evidence | supported-record declaration; fixture matrix for every record/transform; adapter preserves source/hierarchy/property provenance; unsupported fixture fails closed |
| Score promotion | no promotion until independent GDS database and LVS marker/topology correlation |
| Parallel/fan-in hazards | adapter consumes W4 types; freeze those types first. Do not merge a lossless parser without a checked-verification boundary |

## G2.3 — production-scoped OASIS input

| Field | Requirement |
|---|---|
| Read first | [`oasis.rs`](../../../verify/src/oasis.rs), [`gds_lossless.rs`](../../../verify/src/gds_lossless.rs), OASIS capability constant/tests |
| Prerequisites | G2.2 accepted; no external gate beyond the selected OASIS production corpus supplied with `EXT-DECK` |
| Owned files | `verify/src/oasis.rs` and OASIS-only fixtures/fuzz seeds under planned `verify/conformance/oasis/**` |
| Forbidden files | `verify/src/{drc,lvs,pex,signoff}/**`, `verify/src/geometry/**`, `verify/conformance/manifest.json`, and every capability declaration not backed by an OASIS fixture |
| API outcome | explicit versioned supported-feature declaration and checked OASIS-to-layout path equivalent to supported GDS semantics |
| Tasks | implement PATH/TEXT, repetitions, properties, modal/reference tables, XYRELATIVE, placements/transforms, CBLOCK/compression as required by selected corpus; validate checksums/signatures if in target scope |
| Unsupported/non-goals | features outside selected production subset remain typed `Unsupported` with record/offset, not silently skipped |
| Tests | OASIS/GDS equivalent-layout hashes and result equivalence; truncated/invalid/modal-state negative cases; integer boundaries; decompression bombs, cyclic refs and malformed repetition adversarial cases |
| Focused gates | OASIS unit/round-trip/fuzz tests with declared-capability coverage report |
| Full gates | global gate plus DRC/LVS/PEX equivalence on paired OASIS/GDS corpus |
| Acceptance evidence | every declared feature has positive and malformed fixture; every encountered undeclared feature stops import |
| Score promotion | only after independent equivalent-database/result correlation |
| Parallel/fan-in hazards | compression parser may proceed independently; semantic records wait for G2.2 canonical objects |

## G2.4 — hierarchical indexes and deterministic tiling completion

| Field | Requirement |
|---|---|
| Read first | [`hierarchy_index.rs`](../../../verify/src/hierarchy_index.rs), checked GDS adapter, DRC candidate generation |
| Prerequisites | G2.1 and G2.2 accepted |
| Owned files | `verify/src/hierarchy_index.rs` and its inline hierarchy/tile/equivalence tests |
| Forbidden files | `verify/src/drc/**`, `verify/src/lvs/**`, `verify/src/pex/**`, planned D3.5 result/waiver/scheduler files, and `verify/correlation/**` |
| API outcome | deterministic hierarchy-aware candidate enumeration and tile ownership/halo contract that cannot lose or duplicate seam results |
| Tasks | complete transformed bounds/exact recheck; stable instance paths; halo sizing by rule reach; canonical marker ownership; ancestor-aware invalidation keys; bounded memory enumeration |
| Unsupported/non-goals | semantic flattening of unsupported transforms; cross-tile result reconciliation by count only |
| Tests | flat-vs-hier equivalence; one-unit seam markers; nested/array/reflection boundaries; adversarial duplicate instances, deep hierarchy, very large arrays, thread-count/order variation |
| Focused gates | hierarchy index/tile tests and deterministic hash replay across thread counts |
| Full gates | global gate plus representative hierarchical DRC/LVS corpus |
| Acceptance evidence | identical canonical result objects flat/hier and tiled/untiled; runtime/memory measurements recorded, not yet qualification |
| Score promotion | only with independent hierarchy marker/topology correlation |
| Parallel/fan-in hazards | Wave 3 result DB/invalidation consumes keys; coordinate schema before parallel implementation |

## Wave 2 exit

Every selected format operation has a supported declaration or typed error; arbitrary
angles and holes use the shared kernel; checked adapters preserve identity; flat,
hierarchical and tiled runs are equivalent. The desired DRC 24/44 and LVS 15/36 are
targets **after** independent correlation, not automatic completion scores.
