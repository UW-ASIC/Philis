# Wave 6 work packages — reliability evidence extraction

Goal: connect the Wave 0 analysis cores to real layout, extracted devices/nets,
activity and mission profiles. Every electrical result must link to geometry,
device/net identity, stimulus/corner, model revision and limit revision. Foundry
reliability equations, calibrated limits and qualification data are external.

## R6.1 — fabrication-stage antenna evidence

| Field | Requirement |
|---|---|
| Read first | [`signoff/antenna.rs`](../../../verify/src/signoff/antenna.rs), L4 connectivity/device terminals, selected process antenna rules |
| Prerequisites | G2.1, L4.2 and L4.3 accepted; `EXT-RELIABILITY` satisfied for antenna equations/stages |
| Owned files | planned `verify/src/signoff/antenna_evidence.rs`, antenna-evidence records in `verify/src/signoff/antenna.rs`, and planned `verify/correlation/corpus/signoff/antenna/**` |
| Forbidden files | `verify/src/{geometry,lvs,pex}/**`, `verify/conformance/manifest.json`, generic ratio defaults, and final-net-only or ambiguous diode attribution |
| API outcome | stage-specific gate/collector/contact/diode evidence and foundry equation evaluation with physical provenance |
| Tasks | layer construction sequence; temporary networks; area/sidewall/contact metrics; oxide/gate class; diode attribution/bonus; cumulative rules and stage markers |
| Unsupported/non-goals | missing stage/equation/device class is `ERROR`/`NOT_RUN`; no universal antenna formula |
| Tests | disconnect/reconnect by stage; diode positive/ambiguous negative; exact ratio boundary; adversarial multi-gate, hierarchy/via seams and unsupported geometry |
| Focused/full gates | extractor+analyzer analytic tests, global gate |
| Acceptance/evidence | each violation cross-probes stage geometry/net/gate/rule/model; correlates to selected golden antenna deck |
| Score promotion | only after independent foundry-deck marker/value correlation |
| Parallel/fan-in hazards | model schema and extraction may split after common evidence identity freezes |

## R6.2 — power-grid, current and thermal evidence

| Field | Requirement |
|---|---|
| Read first | [`signoff/power.rs`](../../../verify/src/signoff/power.rs), P5 graph/models, activity/current source interfaces |
| Prerequisites | L4.2, P5.1, P5.2 and P5.5 accepted; `EXT-ACTIVITY`, `EXT-PROCESS` and `EXT-RELIABILITY` satisfied |
| Owned files | planned `verify/src/signoff/{power_evidence,activity,thermal_map}.rs`, adapter-owned integration in `verify/src/signoff/power.rs`, and planned `verify/correlation/corpus/signoff/power/**` |
| Forbidden files | `verify/src/{geometry,lvs,pex}/**`, `verify/conformance/manifest.json`, guessed loads/temperatures, and code that discards singular islands |
| API outcome | extracted supply graph with source/load/current/temperature evidence per corner feeding IR/EM analyzers |
| Tasks | supply/ground identification; source boundary conditions; vector currents and vectorless envelope; via current split; thermal interpolation; convergence/error and provenance |
| Unsupported/non-goals | absent activity/current/thermal scope remains `NOT_RUN`; vectorless assumptions explicitly versioned |
| Tests | hand-solvable grids; missing/singular negatives; limit equality/1-step; adversarial tiny currents, disconnected islands, current reversal, thermal hotspots and via arrays |
| Focused/full gates | grid adapter/solver/analyzer tests, global gate, multi-corner replay |
| Acceptance/evidence | node/branch results link to geometry/load/stimulus/model and correlate to independent EM/IR solver |
| Score promotion | only with independent multi-corner result correlation |
| Parallel/fan-in hazards | extraction and stimulus can split after node/load identity contract; thermal consumes stable geometry |

## R6.3 — voltage histories and aging mechanisms

| Field | Requirement |
|---|---|
| Read first | [`signoff/reliability.rs`](../../../verify/src/signoff/reliability.rs), L4 device terminals, mission-profile specification |
| Prerequisites | L4.2 and L4.3 accepted; `EXT-ACTIVITY` and `EXT-RELIABILITY` satisfied |
| Owned files | planned `verify/src/signoff/{waveform,mission_profile,aging_models}.rs`, adapter-owned integration in `verify/src/signoff/reliability.rs`, and planned `verify/correlation/corpus/signoff/aging/**` |
| Forbidden files | `verify/src/{geometry,lvs,pex}/**`, `verify/conformance/manifest.json`, unnamed generic aging, and out-of-domain extrapolation/default-zero histories |
| API outcome | terminal voltage/temperature duty histories and named aging mechanisms with coefficients, validity domain, result and provenance |
| Tasks | waveform normalization; stress-state classification; time/temperature integration; BTI/HCI/TDDB or selected mechanisms; early/normal/end-of-life corners; uncertainty handling |
| Unsupported/non-goals | only foundry-modeled named mechanisms run; out-of-domain input errors |
| Tests | constant/pulsed/accelerated analytic cases; missing mapping/model negatives; exact lifetime boundary; adversarial discontinuities, zero duty, long duration/overflow and out-of-domain stress |
| Focused/full gates | model integration tests, global gate |
| Acceptance/evidence | result maps to device terminal/history/corner/model/limit and correlates to accepted reliability simulator/reference |
| Score promotion | reliability evidence does not change DRC/LVS/PEX score; qualification requires external acceptance |
| Parallel/fan-in hazards | mechanism families may split after waveform/provenance API freezes |

## R6.4 — context-aware ESD path evidence

| Field | Requirement |
|---|---|
| Read first | [`signoff/esd_latchup.rs`](../../../verify/src/signoff/esd_latchup.rs), L4 device recognition, P5 point-to-point R graph |
| Prerequisites | L4.3, P5.1 and P5.2 accepted; `EXT-RELIABILITY` satisfied for ESD topology/sizing rules |
| Owned files | planned `verify/src/signoff/esd_evidence.rs`, ESD-owned integration in `verify/src/signoff/esd_latchup.rs`, and planned `verify/correlation/corpus/signoff/esd/**` |
| Forbidden files | `verify/src/{geometry,lvs,pex}/**`, `verify/conformance/manifest.json`, topology-only clean paths, undirected substitutes, and any HBM/CDM qualification claim |
| API outcome | context-aware pad-to-rail discharge paths with clamp/trigger identity, direction, sizing, capacity and point-to-point R provenance |
| Tasks | path contexts/modes; clamp/trigger structures; rail domains; device sizing; resistance/current capacity; competing/failure paths; localized witness |
| Unsupported/non-goals | analysis is layout/circuit verification, not device-level HBM/CDM certification; missing model/evidence is non-clean |
| Tests | valid/blocked/wrong-direction/undersized paths; exact R/capacity bounds; adversarial multiple rails, loops, shared clamps, open triggers and hierarchy crossing |
| Focused/full gates | path/recognizer/analyzer tests, global gate |
| Acceptance/evidence | each requirement reports path devices/nets/geometry/R/model/limit and correlates to qualified reliability flow |
| Score promotion | none without external reliability qualification |
| Parallel/fan-in hazards | recognizer and path solver split only after device/path identity contract |

## R6.5 — substrate/well and latch-up evidence

| Field | Requirement |
|---|---|
| Read first | latch-up structures in [`signoff/esd_latchup.rs`](../../../verify/src/signoff/esd_latchup.rs), L4 body/well extraction, G2.1 geometry |
| Prerequisites | G2.1, L4.2 and L4.3 accepted; `EXT-RELIABILITY` satisfied for latch-up/guard-ring rules |
| Owned files | planned `verify/src/signoff/latchup_evidence.rs`, latch-up-owned integration in `verify/src/signoff/esd_latchup.rs`, and planned `verify/correlation/corpus/signoff/latchup/**` |
| Forbidden files | `verify/src/{geometry,lvs,pex}/**`, `verify/conformance/manifest.json`, bbox-only ring continuity, unproven bias, and arbitrary injector/victim labels |
| API outcome | context-aware injector/victim sites and geometric/electrical guard-ring evidence with continuity, width, spacing, bias, tap and provenance |
| Tasks | well/substrate regions; parasitic path context; injector/victim recognition; danger zones; ring topology/coverage; bias connectivity and tap density/distance |
| Unsupported/non-goals | missing substrate model or ambiguous bodies is error; geometry check alone cannot imply latch-up clean |
| Tests | enclosing/broken/unbiased/too-thin rings; exact distance/width boundaries; adversarial keyholes, multiple wells, hierarchy seams, floating taps and nested rings |
| Focused/full gates | topology/geometry/analyzer tests, global gate |
| Acceptance/evidence | result cross-probes injector/victim/ring/taps/nets/rule/model and correlates to selected golden flow |
| Score promotion | none without independent reliability correlation/acceptance |
| Parallel/fan-in hazards | substrate topology follows L4 body identity; ring geometry can proceed against frozen region API |

## Wave 6 exit

No caller-created anonymous evidence is sufficient for a production `CLEAN`: every
reported electrical result traces end to end through extracted design evidence and
versioned stimulus/model/limit inputs. External reliability qualification remains required.
