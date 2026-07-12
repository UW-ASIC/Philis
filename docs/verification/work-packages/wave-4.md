# Wave 4 work packages — production LVS

Goal: deterministic, hierarchy-preserving comparison from real checked layout evidence
to the selected foundry schematic dialect and device deck. Production record, binding,
matching, detailed extraction and abstract hierarchy foundations exist at the documented
baseline. The checked GDS hierarchy/text/property adapter is accepted for its declared
strict orthogonal subset; selected-deck breadth and independent correlation remain.

## L4.1 — complete the required reference-netlist dialect

| Field | Requirement |
|---|---|
| Read first | [`lvs/netlist.rs`](../../../verify/src/lvs/netlist.rs), [`lvs/binding.rs`](../../../verify/src/lvs/binding.rs), selected corpus syntax inventory |
| Prerequisites | `EXT-NETLIST` satisfied |
| Owned files | `verify/src/lvs/netlist.rs`, `verify/src/lvs/binding.rs`, and their inline syntax/include/expression/binding tests |
| Forbidden files | `verify/src/lvs/{extract,detailed_extract,production,hier_production}.rs`, `verify/src/{drc,pex,signoff}/**`, unrestricted host-path fixtures, and permissive skipped-statement lists |
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
| Prerequisites | G2.1, G2.2 and G2.4 accepted; `BA-GDS-LVS-ADAPTER` satisfied |
| Owned files | `verify/src/lvs/extract.rs`, `verify/src/lvs/detailed_extract.rs`, `verify/src/lvs/gds_adapter.rs`, and their inline connectivity/evidence tests |
| Forbidden files | `verify/src/lvs/{compare,production,hier_production}.rs`, `verify/src/geometry/**`, `verify/src/{drc,pex,signoff}/**`, and any adapter code that invents port labels |
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
| Prerequisites | L4.2 accepted and `EXT-DEVICE` satisfied |
| Owned files | device-owned records in `verify/src/schema.rs` and `verify/src/params.rs`; planned `verify/src/lvs/devices/**`; planned `verify/correlation/corpus/lvs/devices/**` |
| Forbidden files | `verify/src/lvs/{compare,production,hier_production}.rs`, DRC-owned schema records, `verify/conformance/manifest.json`, and hard-coded/defaulted device properties |
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
| Prerequisites | L4.1, L4.2 and L4.3 accepted |
| Owned files | `verify/src/lvs/compare.rs`, `verify/src/lvs/production.rs`, reduction code in `verify/src/lvs/extract.rs` only through a separately owned reduction commit, and planned `verify/correlation/corpus/lvs/matching/**` |
| Forbidden files | `verify/src/lvs/{netlist,binding,detailed_extract,hier_production}.rs`, `verify/src/{geometry,drc,pex,signoff}/**`, and any probabilistic/witness-free acceptance path |
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
| Prerequisites | L4.1, L4.2, L4.3 and L4.4 accepted; `BA-GDS-LVS-ADAPTER` satisfied |
| Owned files | `verify/src/lvs/hierarchical.rs`, `verify/src/lvs/hier_production.rs`, hierarchy-owned cache code, and planned `verify/correlation/corpus/lvs/hierarchy/**` |
| Forbidden files | `verify/src/lvs/{netlist,binding,detailed_extract}.rs`, `verify/src/{geometry,drc,pex,signoff}/**`, and cache records without ancestor/deck/model identity |
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
