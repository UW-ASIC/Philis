# Capability matrix

This is an audit of product capability families, not line coverage, rule-enum count,
or conformance-case count. The score is frozen at **24.5/112 (22%)** until reviewed
independent correlation exists. The authoritative machine-readable companion is
[`capabilities.json`](capabilities.json).

Validate duplicate keys, exact engine/group identities, states, sums and percentages
from the repository root:

```bash
python3 docs/verification/scripts/validate_capabilities.py
```

JSON Schema fixes the shape and identity sets; the script enforces cross-record
arithmetic that JSON Schema cannot express.

## Summary

| Engine | Score | Total | Coverage | Current claim |
|---|---:|---:|---:|---|
| DRC | 12.0 | 44 | 27% | useful prototype/in-design checker |
| LVS | 8.0 | 36 | 22% | small flat core plus production foundations |
| PEX | 4.5 | 32 | 14% | analytical estimator |
| Total | 24.5 | 112 | 22% | not proprietary/foundry qualified |

## DRC — 12.0/44

| Four-family group | Score | State and remaining gap |
|---|---:|---|
| input formats, units/properties, transforms/arrays, hierarchy | 1.5 | `foundation`: checked/lossless GDS and hierarchy plus partial OASIS exist; checked adapters and full declared format semantics remain |
| booleans, holes/keyholes, all-angle offsets, exact PATH | 0.5 | `foundation`: exact predicates and rectilinear booleans; all-angle offset/stroking and consumer migration remain |
| width, spacing, area, enclosure/extension | 2.5 | `partial`: broad rule presence; some general polygons/context still approximated |
| PRL/EOL/dependent tables/corner-notch context | 2.0 | `partial`: simplified thresholds; general typed table semantics and exact execution remain |
| cut classes/asymmetric enclosure/via arrays/min-cut | 1.5 | `partial`: simplified grouping/center heuristics |
| net/voltage/region/cell/hierarchy contexts | 0.5 | `partial`: narrow strict same-net support; production contexts pending |
| density/union/exclusions/fill/CMP | 1.0 | `partial`: checked rectangular signoff core; calibrated multilevel model/fill and legacy fixes pending |
| fabrication antenna/sidewall/contact/diode/gate class | 1.5 | `partial`: typed analyzer core; extraction-stage evidence and foundry equations pending |
| decomposition/stitches/precolor/litho/yield | 0.5 | `integration_pending`: bounded solver work requires defect fixes/re-review; litho/yield absent |
| advanced-device/EUV/curvilinear/3D/package | 0.0 | absent; target-process scope must decide required subset |
| foundry language, result DB, incremental/distributed, correlation | 0.5 | `foundation`: schema/result/correlation pieces; production execution and independent correlation absent |

## LVS — 8.0/36

The retired audit table's row values summed to 8.5 while its declared/canonical LVS
total was 8.0. This matrix preserves the frozen 8.0 total and conservatively assigns
1.0, rather than 1.5, to matching/witnesses until the real layout-evidence adapter and
independent topology correlation exist. This is an arithmetic disposition, not a score
promotion or demotion.

| Four-family group | Score | State and remaining gap |
|---|---:|---|
| SPICE/CDL/Spectre, parameters/buses/globals/includes/models, layout properties | 0.5 | `foundation`: strict SPICE/CDL subset and binding; Spectre breadth and GDS evidence adapter pending |
| exact connectivity, cuts/derived layers, opens/soft-connect | 1.0 | `partial`: exact supported rectilinear contact and cut policy; all-angle/derived integration/soft-connect remain |
| MOS recognition, S/D/body/well, W/L/NF/M | 1.5 | `partial`: body-aware production records; diffusion metrics/fingers and checked GDS binding incomplete |
| R/C/diode/BJT and special/custom devices | 1.5 | `partial`: basic configured forms; foundry-complete recognition absent |
| model/class and extraction properties/equations/reductions | 1.0 | `partial`: typed property/tolerance foundations; actual foundry properties incomplete |
| deterministic matching, swaps, seeds, witnesses | 1.0 | `foundation/partial`: production matcher/witnesses exist; end-to-end layout label seeds require adapter/corpus |
| safe symmetric series/parallel/equivalence | 0.5 | `partial`: constrained reductions; cross-boundary and full symmetric semantics remain |
| hierarchy, ports, black boxes/equated cells/selective flatten | 0.5 | `foundation`: abstract hierarchy/binding exists; real GDS adapter is integration pending |
| capacity/incremental/distributed/results/correlation | 0.5 | `foundation`: caching structures and neutral harness; production evidence absent |

## PEX — 4.5/32

| Four-family group | Score | State and remaining gap |
|---|---:|---|
| qualified stack and process/temperature/variation corners | 0.5 | `partial`: per-layer scalars only |
| conductor union, distributed nodes/vias, terminal mapping | 0.5 | `partial`: scalar per-net attribution; topology graph absent |
| arbitrary-shape/distributed/size/temp R and via arrays | 0.75 | `partial`: Manhattan equivalent R and fixed via R |
| ground/fringe/3D shield/fill C | 0.75 | `partial`: Manhattan area+fringe; calibrated process context absent |
| lateral/vertical/diagonal coupling and shield/same-net | 1.0 | `partial`: analytical bbox models; calibrated field solution absent |
| device/junction/substrate/LDE parasitics | 0.25 | minimal device capacitance only |
| inductance/frequency/substrate noise/electrothermal | 0.0 | absent |
| DSPF/SPEF/multi-corner/reduction/correlation | 0.75 | rudimentary SPICE and neutral artifacts; standard validated outputs/correlation absent |

## Promotion rule

`foundation`, `partial`, and `integration_pending` work does not raise these numbers.
For each promoted family, attach the supported-subset declaration, general-input tests,
independent artifact pair, tolerance/disposition review, reproducible commands, and
frozen engine/deck/model/corpus hashes. Qualification is a separate external decision.
