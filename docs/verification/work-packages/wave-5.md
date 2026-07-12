# Wave 5 work packages — distributed, multi-corner PEX

Goal: replace scalar per-net estimates with a topology-preserving electrical network,
qualified process/corner models and validated outputs. Current PEX remains a useful
analytical estimator. Field-solver structures, process data and golden extractor
results are external prerequisites.

## P5.1 — conductor/via node graph and terminal mapping

| Field | Requirement |
|---|---|
| Read first | [`pex/mod.rs`](../../../verify/src/pex/mod.rs), L4.2 detailed connectivity, G2.1/G2.4 geometry/indexes |
| Prerequisites | G2.1, G2.4, L4.2 and L4.3 accepted |
| Owned files | planned `verify/src/pex/{graph,topology}.rs`, the compatibility adapter in `verify/src/pex/mod.rs`, and planned `verify/correlation/corpus/pex/topology/**` |
| Forbidden files | `verify/src/lvs/**`, `verify/src/geometry/**`, `verify/src/signoff/**`, `verify/conformance/manifest.json`, and any dangling/synthetic unmapped terminal export |
| API outcome | deterministic conductor/via node-edge graph whose terminals map to extracted devices/ports and preserve source geometry/hierarchy |
| Tasks | conductor fracture/junction nodes; via arrays/contact edges; terminal attachment; component provenance; connected-island diagnostics; selected-net graph slicing |
| Unsupported/non-goals | unsupported geometry/terminal ambiguity blocks checked extraction; no zero-resistance invented joins |
| Tests | series/parallel/T/cross/via-array graphs; missing terminal negative; zero/one-unit dimensions; adversarial concave/all-angle/seam/floating/duplicate attachment |
| Focused/full gates | graph extraction/topology tests, LVS-PEX identity integration, global gate |
| Acceptance/evidence | canonical graph equals independent hand structures; every endpoint maps to net/geometry/terminal |
| Score promotion | only after golden element-topology correlation |
| Parallel/fan-in hazards | graph schema lands before model/extractor streams; one owner controls node canonicalization |

## P5.2 — process stack, corners and resistance models

| Field | Requirement |
|---|---|
| Read first | current PEX params/schema, P5.1 graph, selected process-stack/model docs |
| Prerequisites | P5.1 accepted and `EXT-PROCESS` satisfied |
| Owned files | planned `verify/src/pex/{process,resistance,reduction}.rs`, PEX-owned process records in `verify/src/schema.rs`, and planned `verify/correlation/corpus/pex/resistance/**` |
| Forbidden files | `verify/src/lvs/**`, `verify/src/signoff/**`, DRC/LVS-owned schema records, `verify/conformance/manifest.json`, and hard-coded process constants |
| API outcome | typed stack/dielectric/conductor/via models and multi-corner R extraction with width/thickness/temp/size effects, arrays and topology-preserving reduction |
| Tasks | unit-safe interpolation/tables; corner composition; sheet/3D/via/contact resistance; temperature/material laws; reduction error control and protected terminals |
| Unsupported/non-goals | undeclared extrapolation is error; process physics not supplied by foundry remain unsupported |
| Tests | analytic bars/vias/branch networks; missing model/unit negative; table boundary; adversarial ultra-wide/narrow, via arrays, hot gradients and reduction topology |
| Focused/full gates | schema/model/R tests per corner, global gate |
| Acceptance/evidence | topology and R correlate to field solver/golden extractor within reviewed typed bounds |
| Score promotion | resistance families only after independent multi-corner correlation |
| Parallel/fan-in hazards | schema and solver can split after units/corner identity freeze |

## P5.3 — ground and coupling capacitance with context

| Field | Requirement |
|---|---|
| Read first | current area/fringe/lateral/interlayer code, P5.1 graph, process stack from P5.2 |
| Prerequisites | G2.1, P5.1 and P5.2 accepted; `EXT-PROCESS` satisfied |
| Owned files | planned `verify/src/pex/capacitance.rs`, capacitance-owned records in planned `verify/src/pex/process.rs`, and planned `verify/correlation/corpus/pex/capacitance/**` |
| Forbidden files | `verify/src/{lvs,signoff}/**`, `verify/src/geometry/**`, `verify/conformance/manifest.json`, and inferred fill/shield coefficients outside the versioned process model |
| API outcome | ground, lateral, vertical and diagonal coupling elements with same-net, shield and explicit fill awareness |
| Tasks | neighbor search/exact geometry measures; dielectric adjacency; fringe/corner models; shield interception; fill model; coupling partition and conservation; protected terminal reduction |
| Unsupported/non-goals | model-free 3D effects remain unsupported; no generic average coefficient across arbitrary layers |
| Tests | plate/wire/comb/shield/fill structures; same-net exclusion; spacing/width/overlap boundaries; adversarial diagonal corners, intervening shields, hierarchy seams and dense fill |
| Focused/full gates | C model/property tests, field structure suite, global gate |
| Acceptance/evidence | component topology and values correlate across declared corners; physical breakdown retained, including 386 aF fixture mechanisms |
| Score promotion | capacitance families after independent field/golden correlation |
| Parallel/fan-in hazards | lateral/vertical models may split using frozen element/provenance schema; reconcile without double counting |

## P5.4 — device/substrate, RLC/frequency and electrothermal options

| Field | Requirement |
|---|---|
| Read first | L4.3 device records, P5.1 graph, P5.2/P5.3 models, selected process requirements |
| Prerequisites | L4.3, P5.1, P5.2 and P5.3 accepted; `EXT-PROCESS` and `EXT-DEVICE` satisfied |
| Owned files | planned `verify/src/pex/{device,substrate,inductance,frequency,thermal}.rs` and planned `verify/correlation/corpus/pex/{device,substrate,rlc,thermal}/**` |
| Forbidden files | `verify/src/lvs/**`, `verify/src/signoff/**`, `verify/conformance/manifest.json`, and enabled optional-physics code without a target-process model |
| API outcome | typed optional junction/device/substrate networks and, where required, L/frequency/electrothermal elements with provenance |
| Tasks | AD/AS/PD/PS-based junctions; contact/gate parasitics; well/substrate network; inductance/mutual terms; skin/proximity; thermal coupling/iteration and convergence status |
| Unsupported/non-goals | each optional family declares scope; absent model returns unsupported/not-run, never guessed clean |
| Tests | analytic device/substrate/RL/thermal structures; missing/singular negatives; DC/frequency/temperature boundaries; adversarial convergence and mutual-sign/order cases |
| Focused/full gates | family-specific solver/model tests, global gate |
| Acceptance/evidence | topology/values correlate with appropriate field/device solver across declared range |
| Score promotion | one family at a time with independent evidence |
| Parallel/fan-in hazards | split by physics after common graph/units/provenance; electrothermal consumes stable R/C |

## P5.5 — standard outputs, reduction and distributed execution

| Field | Requirement |
|---|---|
| Read first | [`lvs/spice.rs`](../../../verify/src/lvs/spice.rs), P5 graph, correlation artifact schema, G2.4 tiles |
| Prerequisites | G2.4, P5.1, P5.2, P5.3 and P5.4 accepted |
| Owned files | planned `verify/src/pex/{export,distributed}.rs`, PEX-owned reduction integration in planned `verify/src/pex/reduction.rs`, and planned `verify/correlation/corpus/pex/exports/**` |
| Forbidden files | `verify/src/lvs/spice.rs` except through a separately reviewed shared-export commit, `verify/src/bin/correlation.rs`, `verify/correlation/schemas/**`, and every export that drops diagnostics or uses nondeterministic node names |
| API outcome | validated DSPF/SPEF plus selected-net and multi-corner outputs; deterministic topology-preserving reduction and distributed/restart artifacts |
| Tasks | canonical naming; units/header/name maps; external parser validation; protected terminals/coupling; error-bounded reduction; deterministic tile merge; partial-failure/error propagation |
| Unsupported/non-goals | output dialects not validated by independent reader remain experimental and named accordingly |
| Tests | round-trip external parser; multi-corner/select-net; reduction error boundaries; adversarial escaping, huge nets, worker reorder/crash/restart/seam elements |
| Focused/full gates | export/reduction/distributed simulation, global gate, deterministic hashes |
| Acceptance/evidence | independent readers accept files; canonical elements and values match neutral/golden artifacts |
| Score promotion | output/capacity families require independent extractor/field and Wave 7 capacity evidence |
| Parallel/fan-in hazards | formats may split after canonical graph naming freezes; distributed merge follows element ownership contract |

## Wave 5 exit

Element topology and values correlate to field-solver structures and the target golden
extractor across every declared corner and reviewed error bound. PEX ≥27/32 is a
post-correlation target; foundry qualification remains external.
