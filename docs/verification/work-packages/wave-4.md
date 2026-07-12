# Wave 4 work packages — production LVS

Goal: deterministic, hierarchy-preserving comparison from real checked layout evidence
to the selected foundry schematic dialect and device deck. Production record, binding,
matching, detailed extraction and abstract hierarchy foundations exist at the documented
baseline. The real GDS hierarchy/text/property adapter is integration pending.

## L4.1 — complete the required reference-netlist dialect

| Field | Requirement |
|---|---|
| Read first | [`lvs/netlist.rs`](../../../verify/src/lvs/netlist.rs), [`lvs/binding.rs`](../../../verify/src/lvs/binding.rs), selected corpus syntax inventory |
| Prerequisites | target SPICE/CDL/Spectre subset and include-security policy fixed |
| Owned files | parser/AST/binder/conversion and syntax fixtures |
| Forbidden files | permissive skip of statements/options, unrestricted host includes, layout extraction |
| API outcome | source-spanned AST and deterministic bound hierarchy for required SPICE/CDL/Spectre parameters, expressions, buses, globals, includes, models/aliases and conditions |
| Tasks | `.lib`/conditions/functions as required; unit/suffix semantics; bus expansion; scoped parameters; include cycle/root policy; model aliases and primitive mapping |
| Unsupported/non-goals | dialect outside declared subset returns typed syntax/unsupported error; no best-effort line skipping |
| Tests | positive constructs/round conversions; unknown/malformed/include negatives; numeric boundaries; adversarial recursion, shadowing, bus direction, locale and path traversal |
| Focused/full gates | parser/binder/conversion tests, grammar corpus, global gate |
| Acceptance/evidence | every corpus statement classified supported/unsupported; vendor-produced netlists parse deterministically |
| Score promotion | only with independent schematic topology/property correlation |
| Parallel/fan-in hazards | grammar and binding may split after AST freeze; one owner resolves dialect precedence |

## L4.2 — exact connectivity and evidence binding

| Field | Requirement |
|---|---|
| Read first | [`lvs/extract.rs`](../../../verify/src/lvs/extract.rs), [`lvs/detailed_extract.rs`](../../../verify/src/lvs/detailed_extract.rs), G2.2 adapter, production identity types |
| Prerequisites | G2.1/G2.2 accepted and real checked GDS-to-`HierLayout` adapter merged |
| Owned files | connectivity/extraction/evidence adapter tests |
| Forbidden files | bbox contact decisions, invented port labels, implicit cross-layer joins, matcher changes |
| API outcome | exact conductor/via connectivity, ports/text/properties/body/well, soft-connect/open candidates with source geometry and hierarchy provenance |
| Tasks | derived conductor regions; via stacks/cut arrays; boundary-contact policies; ambiguity rules; port direction/global binding; four-terminal body/substrate identity; actionable open/short physical witness objects |
| Unsupported/non-goals | ambiguous text/property association is error; no legacy layer-name inference in strict path |
| Tests | legal/illegal touch and via stacks; duplicate/missing labels; body/well swaps; one-unit boundaries; adversarial concave/all-angle/seams/arrays/conflicting properties |
| Focused/full gates | extraction/adapter tests, flat-vs-hier connectivity, global gate |
| Acceptance/evidence | every extracted net/terminal links to layout objects/path; flattened and hierarchy evidence agree |
| Score promotion | only after independent connectivity/open/short marker correlation |
| Parallel/fan-in hazards | GDS adapter has single owner; extraction consumes it without reinterpretation |

## L4.3 — foundry device recognition and properties

| Field | Requirement |
|---|---|
| Read first | [`params.rs`](../../../verify/src/params.rs) device config, detailed extraction, [`lvs/production.rs`](../../../verify/src/lvs/production.rs) typed records |
| Prerequisites | L4.2 exact regions/terminals and selected foundry recognition deck/model inventory |
| Owned files | device schema/recognizers/property extraction and golden cells |
| Forbidden files | hard-coded generic device guesses, silently defaulted body/model/property, matcher tolerance changes |
| API outcome | complete required MOS/R/C/diode/BJT/custom device records with model/class, terminals, typed properties, equations and source evidence |
| Tasks | MOS S/D segmentation, W/L/M/NF, AD/AS/PD/PS; resistor/cap/diode/BJT geometry/properties; selected varactor/MOSCAP/inductor/custom devices; ambiguity and overlap precedence |
| Unsupported/non-goals | devices outside selected deck are explicit extraction errors; no zero placeholder property presented as measured |
| Tests | one golden cell per variant/property; wrong/missing marker negative; exact tolerance boundary; adversarial overlapping markers, multifinger shapes, swapped implants and degenerate terminals |
| Focused/full gates | recognizer/property tests, selected device corpus, global gate |
| Acceptance/evidence | layout and reference device/property artifacts correlate with selected golden extractor |
| Score promotion | per device family after independent general-input correlation |
| Parallel/fan-in hazards | split by disjoint device families after shared typed property/region API freezes |

## L4.4 — deterministic graph matching and symmetric reductions

| Field | Requirement |
|---|---|
| Read first | [`lvs/production.rs`](../../../verify/src/lvs/production.rs), [`lvs/compare.rs`](../../../verify/src/lvs/compare.rs), current reduction code |
| Prerequisites | L4.1 bound reference and L4.2/L4.3 detailed layout identities |
| Owned files | matcher/reduction/witness logic and graph corpora |
| Forbidden files | probabilistic acceptance, layout-only reduction, silent seed relaxation, witness-free mismatch |
| API outcome | deterministic complete matching within declared resource bounds, named-net seeds, legal pin swaps, symmetric layout/reference reductions and reproducible open/short/property witnesses |
| Tasks | canonical partitions; equivalence rules; S/D and declared symmetry; series/parallel fixpoint both sides; resource-limit status; minimal conflicting mappings and cross-probe objects |
| Unsupported/non-goals | exhausted bounds return indeterminate/error, not match; undeclared swaps/equivalences forbidden |
| Tests | automorphisms, seeded names, legal/illegal swaps, reduction symmetry; tolerance edges; adversarial symmetric graphs/order/resource limits and near-isomorphic shorts/opens |
| Focused/full gates | matcher property/differential tests, global gate, deterministic replay |
| Acceptance/evidence | every mismatch references schematic/layout objects and witness; golden topology mappings agree |
| Score promotion | only after independent topology/property/witness correlation |
| Parallel/fan-in hazards | reductions and matcher can split only with frozen canonical record/equivalence schema |

## L4.5 — real hierarchy, black boxes, equated cells and capacity

| Field | Requirement |
|---|---|
| Read first | [`lvs/hier_production.rs`](../../../verify/src/lvs/hier_production.rs), binding hierarchy, G2.2 adapter/cache |
| Prerequisites | L4.1–L4.4 APIs accepted |
| Owned files | production hierarchy orchestration/cache, hierarchy corpus |
| Forbidden files | synthetic-root collapse, unbound child ports, unconditional flatten, cache keys without ancestor/deck/model identity |
| API outcome | real subcircuit binding with black boxes/equated cells/selective flattening and hierarchy/fully-flat equivalence |
| Tasks | child-port maps; parameterized instances/arrays; matched-cell abstraction; cross-boundary reductions where legal; negative black-box semantics; cache invalidation; million-instance bounded execution |
| Unsupported/non-goals | ambiguous/missing child reference is error; black box is not wildcard equivalence unless explicitly declared |
| Tests | nested/array/parameter hierarchy; black-box positive and negative; selective flatten boundaries; adversarial recursion, missing cells/ports, ancestor edit and cache poisoning |
| Focused/full gates | hierarchy tests, flat-vs-hier canonical comparison, global gate, capacity smoke |
| Acceptance/evidence | hierarchy-preserving and fully flattened runs equivalent on corpus; runtime/memory recorded; mismatches preserve both paths |
| Score promotion | only after independent hierarchical/full-chip correlation |
| Parallel/fan-in hazards | capacity/cache follows semantic hierarchy; do not optimize before equivalence gate passes |

## Wave 4 exit

The selected netlist/device deck is fully classified; real layout evidence binds ports,
properties and hierarchy; flat and hierarchical runs agree; every mismatch cross-probes
both designs. LVS ≥31/36 is a post-correlation target, not an implementation claim.
