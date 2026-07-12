# Proprietary-grade DRC/LVS/PEX audit

Audit date: 2026-07-12. Scope: the current working tree in `verify/`, including
the signoff additions made during this audit.

## What “full compliance” means

There is no vendor-neutral checkbox called “full DRC/LVS/PEX compliance.” A
tapeout signoff claim requires all three of the following:

1. the engine implements every operation used by the target foundry deck;
2. the foundry has qualified that exact engine/deck/version combination; and
3. the design team has correlated it against the foundry golden flow on a
   representative regression corpus and accepted the residual deltas.

Commercial tools explicitly emphasize foundry-qualified decks, silicon/tapeout
correlation, advanced-node support, hierarchy and production capacity—not just a
list of geometric predicates. See the official
[Calibre physical-verification overview](https://www.siemens.com/en-us/products/ic/calibre-design/physical-verification/),
[Calibre nmLVS overview](https://www.siemens.com/en-us/products/ic/calibre-design/circuit-verification/nmlvs/),
[Cadence Pegasus overview](https://www.cadence.com/en_US/home/tools/digital-design-and-signoff/silicon-signoff/pegasus-verification-system.html/),
and [Cadence Quantus overview](https://www.cadence.com/en_US/home/tools/digital-design-and-signoff/silicon-signoff/quantus-extraction-solution.html).

Consequently, even a 100% score below would mean “feature-complete enough to
seek qualification,” not “foundry certified.”

## Quantified result

Scoring is deliberately conservative and auditable. Each row contains four
proprietary capability families. A score of 1 means one family is substantially
implemented, 0.5 means a constrained/heuristic implementation, and 0 means it
is absent. Passing one hand-authored conformance case does not make a partial
algorithm full.

| Engine | Capability families | Equivalent implemented | Coverage | Assessment |
|---|---:|---:|---:|---|
| DRC | 44 | 12.0 | **27%** | useful prototype/in-design checker; not signoff-equivalent |
| LVS | 36 | 8.0 | **22%** | small flat CMOS comparator; not production LVS |
| PEX | 32 | 4.5 | **14%** | analytical estimator; not signoff extraction |
| **Total** | **112** | **24.5** | **22%** | substantial research core, far from proprietary qualification |

The score is intentionally unchanged after the Wave 1 bug fixes. The fixes remove
known false-clean behavior, but the permanent score policy requires general-input
support plus independent correlation evidence before a capability family is promoted.

The settled-tree Wave 1 gate passed on 2026-07-12: 78/78 `gdsverify` library tests,
162/162 conformance cases with negative evidence for all 51 coverage entries,
downstream `pnr-core` and `pnr-backend` compile gates, 18/18 backend tests,
and 16/16 correlation-harness tests. These are local regression results, not an
independent foundry or golden-tool correlation claim.

This is stricter than counting the 28 enabled DRC enum variants. A foundry deck
contains many instances, conditional tables, boolean derivations and device
contexts per rule family. “28 Rust enum variants” is therefore not “28 signoff
rules.”

### DRC: 12.0 / 44

| Four-family group | Score | Current state |
|---|---:|---|
| GDS/OASIS/LEF-DEF/OA input; database units/properties; transforms/arrays; hierarchy preservation | 1.5 | GDS subset parses/exposes UNITS and deck-aware loading rejects mismatch; no OASIS/LEF/DEF/OA, hierarchy is flattened, and important properties are lost |
| Exact union/intersection/subtraction; holes/keyholes; arbitrary-angle offsets; exact PATH stroking | 0.5 | a shared exact-predicate/rectilinear-boolean foundation now exists, but DRC consumers still retain private heuristics and all-angle offsets/PATH stroking are absent |
| Width; spacing; area; enclosure/extension | 2.5 | all exist but several are rectangle/bbox or fragment-sum approximations |
| PRL tables; EOL tables; width/length/area-dependent tables; corner/notch/convexity context | 2.0 | one-threshold simplified forms exist; no general lookup-table semantics |
| Cut classes; asymmetric enclosure; via arrays; redundant/min-cut rules | 1.5 | simplified center/group heuristics, not full cut-class/context semantics |
| Same/different-net; voltage-aware; region/cell-name context; hierarchy-aware rules | 0.5 | strict same-net mode only; no voltage/region/cell context |
| Sliding-window density; exact union coverage/exclusions; fill generation; calibrated multi-level CMP | 1.0 | new explicit-die rectangular union/step/gradient/CMP model; legacy DRC density remains unsafe |
| Fabrication-stage antenna; sidewall/contact metrics; diode correction; oxide/gate-class variants | 1.5 | new configured metrics and diode credit; rectangular geometry and extracted-net limitations remain |
| Exact multi-pattern decomposition/stitches; precoloring; pattern/litho matching; critical-area/yield analysis | 0.5 | greedy coloring only; it can false-positive colorable graphs |
| FinFET/GAA cut/block rules; EUV/stochastic checks; curvilinear/mask checks; 2.5D/3D/package checks | 0.0 | absent |
| Foundry rule language/decks; incremental/distributed scale; waiver/error database and cross-probe; golden correlation/restart | 0.5 | rayon and a small JSON schema exist; the production infrastructure does not |

Calibre advertises equation-based DRC, multi-patterning, ML and voltage-dependent
spacing; Pegasus advertises foundry-certified decks, double/triple/quadruple
patterning, FinFET and 3D-IC support. Those are appropriate proprietary-level
reference points, not KLayout detection agreement alone.

### LVS: 8.0 / 36

| Four-family group | Score | Current state |
|---|---:|---|
| SPICE/CDL/Spectre inputs; bus/parameter syntax; labels/ports/globals; robust layout properties | 0.5 | reference structs and a parser foundation exist, but the production language/hierarchy subset is incomplete and GDS text is not attached to nets |
| Exact polygon connectivity; via-stack/cut semantics; derived layers; opens/soft-connect handling | 1.0 | supported rectilinear connectivity now exact-rechecks bbox candidates and follows declared via pairs; all-angle geometry, derived booleans and soft-connect remain incomplete |
| MOS recognition; S/D segmentation; body/well pins; W/L/fingers/multiplicity | 1.5 | basic rectangular MOS and W/L exist; body is extracted inconsistently and omitted from comparison |
| R/C/diode/BJT; varactor/MOSCAP; inductor/transformer; custom/parameterized devices | 1.5 | simple R/C/diode/BJT structs exist; the rest and robust recognition do not |
| Model/flavor/class; area/perimeter properties; M/NF/series/parallel properties; tolerances/equations | 1.0 | flavor and W/L tolerance exist; most properties and combination rules do not |
| Deterministic graph isomorphism; legal pin swaps; named-port seeding; actionable open/short mapping | 1.5 | probabilistic refinement and S/D swap exist; net seeds are not applied to layout names and debug mapping is weak |
| Safe parallel reduction; safe series reduction; user equivalence rules; symmetric property reduction on both sides | 0.5 | constrained layout-side parallel/series reduction exists and the series regression passes; reference-side/equivalence semantics remain incomplete |
| True hierarchical subcircuits; port binding; black boxes; selective flatten/equated cells | 0.5 | API structs exist, but parent subcircuit instances/ports are not compared |
| Million-instance capacity; incremental/distributed runs; cross-probe/results DB; foundry qualification/correlation | 0.5 | sweep pruning exists; production workflow and qualification do not |

The official nmLVS description calls out production-proven connectivity/device
extraction, intricate parameter extraction, hierarchy/logic injection and
interactive short isolation. Those are materially broader than the current flat
rectangle-based extractor.

### PEX: 4.5 / 32

| Four-family group | Score | Current state |
|---|---:|---|
| Qualified process stack; conductor/dielectric geometry; process corners; temperature/variation corners | 0.5 | a few per-layer scalars exist; no qualified stack or corner engine |
| Exact conductor union/fracture; distributed nodes; contact/via networks; terminal mapping | 0.5 | per-polygon attribution and scalar per-net sums only; no topology-preserving node graph |
| Arbitrary-shape R; width/thickness/size effects; via/contact arrays; distributed/topological/temperature-aware R | 0.75 | supported Manhattan area/perimeter equivalent `Rs*L/W` and fixed via R only; unsupported shapes are diagnostic |
| Ground C on every conductor; sidewall/fringe; conformal/3D shielding; fill-aware capacitance | 0.75 | every supported Manhattan conductor receives area+fringe ground C; calibrated shielding/fill/3D effects are absent rather than guessed |
| Lateral coupling; vertical coupling; diagonal/corner/3D coupling; shield/same-net handling | 1.0 | bbox PRL and overlap models exist, without a calibrated field solution |
| Junction/device parasitics; gate/contact parasitics; substrate/well network; device/LDE context | 0.25 | simple recognized capacitor value only |
| Inductance; frequency/skin/proximity effects; substrate noise; electrothermal/self-heating extraction | 0.0 | absent |
| DSPF/SPEF/extracted-view output; multi-corner; reduction/select-net; field-solver/silicon correlation | 0.75 | rudimentary SPICE text and aggregation exist; no standard distributed signoff output |

Commercial references are much more demanding. Calibre xACT advertises a 3D
field solver, simultaneous multi-corner extraction, RLC output, selective-net
processing, reduction and DSPF/SPEF/HSPICE formats; Quantus advertises a built-in
3D field solver and foundry certification down to 2 nm. See the
[Calibre xACT product page](https://www.siemens.com/en-us/products/ic/calibre-design/circuit-verification/xact/)
and [Quantus datasheet](https://www.cadence.com/en_US/home/resources/datasheets/quantus-extraction-solution-ds.html).

## Confirmed bug register

Severity is based on tapeout risk: a false clean is more severe than an extra
marker. “Fixed” means fixed in this audit; it does not imply the surrounding
algorithm is now proprietary-grade.

| # | Severity | Status | Finding |
|---:|---|---|---|
| 1 | Critical | Fixed | Safe series normalization now reaches a fixpoint with compatibility/observability guards; `LVS_SERIES_MERGE` passes and the suite is **162/162**. |
| 2 | Critical | Partial | The explicit-die signoff density/CMP path checks empty space, exact rectangular unions and complete window coverage. Legacy DRC density still anchors scope to target geometry and assumes non-overlap. |
| 3 | Critical | Fixed | Supported rectilinear LVS connectivity/device contact uses bbox only as a candidate filter and exact-rechecks real regions; unsupported all-angle/degenerate geometry errors. A shared all-angle kernel is still Wave 2. |
| 4 | Critical | Partial | Declared via decks now restrict cut-less compatibility joins to layer pairs co-listed by a via. The explicitly legacy, via-less cut-less fallback still permits broad cross-layer overlap and remains a compatibility risk. |
| 5 | Critical | Partial | The signoff antenna path uses extracted legal connectivity. The legacy DRC CAR implementation remains simplified and is not the tapeout gate. |
| 6 | Critical | Partial | The configured signoff antenna path is PDK-driven and fail-closed. The legacy basic DRC antenna rule remains simplified. |
| 7 | Critical | Fixed | Rule `id` and `kind` are independent; object/list encodings support repeated kinds, reject duplicate IDs and retain the legacy map encoding. |
| 8 | High | Fixed | Deck construction rejects unknown/duplicate layer, connectivity, device and PEX references; recognized nested properties are strict in both library and backend PDK adapters. |
| 9 | High | Open | `min_area` sums fragment areas and double-counts overlaps, which can turn an under-area boolean union into a pass. |
| 10 | High | Open | `overlap` checks only bbox-positive pairs and never reports an A-shape with no B counterpart; it is not a “must overlap” rule. |
| 11 | High | Open | `min_extension`, `max_width`, PRL, wide spacing and asymmetric enclosure use bbox/orientation heuristics that are wrong for general polygons. |
| 12 | High | Open | `min_enclosed_area` treats a nested same-polarity polygon as a hole and measures `outer_area-inner_area`; `cheesing` uses the same invalid slot representation. |
| 13 | High | Open | Multi-patterning uses greedy vertex order. Greedy failure does not prove a graph is uncolorable, so legal layouts can be rejected. |
| 14 | High | Open | MOS body/well is not a graph pin in LVS comparison (`ROLE_BODY` is unused); four-terminal connectivity errors can pass. |
| 15 | High | Open | Reference `net_seeds` do not seed layout/reference correspondence. Named VDD/VSS/ports can be permuted; the later “isomorphic seed conflict” check is not name matching. |
| 16 | High | Open | Hierarchical LVS does not bind child ports or compare parent subcircuit instances. `ports`, `subcircuit_instances`, `top_cell`, and matched-cell abstraction are effectively unused; some all-missing-reference cases can report matched. |
| 17 | High | Open | Derived layers are not integrated into deck extraction and operate on bboxes/first overlap rather than exact booleans. |
| 18 | High | Fixed | Global-net merging remapped `net_of_poly` after MOS extraction but did not remap MOS gate/source/drain terminal IDs. |
| 19 | High | Fixed | `extract_netlist()` ignored `deck.lvs_cut_required`; it now inherits the deck setting. |
| 20 | High | Fixed | `run_lvs()` ignored `deck.strict` and `fail_on_floating`; the facade now enforces both. Direct `compare()` still intentionally has no deck policy. |
| 21 | High | Fixed | ERC returned an empty, apparently clean report when LVS extraction failed. It now emits `erc_extraction_error`. |
| 22 | High | Fixed | MIM capacitor extraction multiplied an nm² value directly by an aF/µm² coefficient (a 10^6 unit error at 1 nm DBU). |
| 23 | High | Fixed | Every supported rectilinear conductor receives both R and area+fringe ground C; the per-net fixture pins 10 + 176 + 200 = 386 aF. |
| 24 | High | Open | Per-net resistance is a scalar sum of polygon/via resistance, not a distributed network solve. Series/parallel branches and terminal-to-terminal resistance are therefore wrong. |
| 25 | High | Open | Interlayer C couples every configured layer pair using an average coefficient, regardless of stack adjacency/dielectric/shielding; geometry is bbox overlap. |
| 26 | Medium | Fixed | Hard-coded size/layer fill and shielding factors were removed. Fill/shield effects are now explicitly unmodeled until process parameters exist. |
| 27 | High | Fixed | `floating_metal_cap()` could never return a value because `run_pex_by_net()` discarded `u32::MAX`; sentinel attribution is now retained. |
| 28 | High | Fixed | Port-net lumped R now splits the external port from an internal loaded node and remaps device terminals/capacitance. Internal scalar R without endpoints is omitted with an explicit warning, not emitted dangling. |
| 29 | High | Partial | GDS now validates record framing/types, exposes UNITS/unmapped geometry, and `load_gds` rejects absent/mismatched units. Unsupported records/properties and TEXT-to-net association remain open. |
| 30 | High | Open | PATH type 1 round caps are replaced by square caps; negative absolute-width paths are dropped; arbitrary-angle joints are overlapping quads, not exact stroked geometry. |
| 31 | High | Fixed | The backend historically waived all legacy density violations at final signoff. Final DRC now keeps them blocking; the richer density/CMP report remains `NOT_RUN` until an explicit die/rule config is supplied. |
| 32 | High | Fixed | Duplicate GDS layer/datatype aliases could make one symbolic rule layer unreachable; deck construction now rejects duplicate/out-of-range pairs. |
| 33 | High | Fixed | MOS extraction previously selected the first matching type/flavor marker. Conflicting N/P rules or HVT/LVT markers now stop extraction with focused regressions. |
| 34 | High | Fixed | The power-grid solver normalized residuals to at least 1 A, allowing tiny IC loads to return the nominal initial guess. It now uses the physical RHS scale and pins a 1 pA × 1 MΩ drop. |
| 35 | High | Fixed | Full-window density stepping could leave a trailing die strip unchecked. Non-covering full-window configs now return `ERROR`; callers may enable partial windows or select a covering step. |
| 36 | High | Fixed | Antenna diode relief previously required the marker layer itself to conduct. Non-conductive markers now attribute by exact overlap to one extracted net; zero/multiple-net attribution is `ERROR`. |
| 37 | High | Fixed | The combined ESD/latch-up family could report clean with only one half configured. Both ESD path requirements and latch-up sites are now mandatory evidence scopes. |

## Added signoff analyses

The new `verify/src/signoff/` module and backend report now expose:

| Check | Implemented analysis | Required input / fail-closed behavior |
|---|---|---|
| Antenna | per-connected-net EGAR, cumulative collector layers, area or sidewall metrics, diode area credit/bonus or explicit waiver | explicit rules or convertible deck antenna rules; extraction/unsupported or ambiguous marker geometry is `ERROR` |
| Density/CMP | explicit die boundary, covering full or partial windows, exact union for axis-aligned rectangles, min/max density, neighbor gradient, calibrated first-order thickness model | die + rules; absent input is `NOT_RUN`, unknown/non-rectangular/non-covering scope is `ERROR` |
| IR drop | sparse resistive DC network solve with fixed supplies, load currents, residual/convergence reporting, absolute/percent/overvoltage limits | extracted power network and currents; missing/singular network or empty checked scope is `NOT_RUN`/`ERROR` |
| Electromigration | solved branch current, metal cross-section J, via-cut current, temperature derating, optional Blech exemption, temperature limit | foundry limits, dimensions, cuts and temperatures; missing limits are `ERROR` |
| Reliability | voltage and thermal limits plus foundry-calibrated inverse-power/Arrhenius lifetime models for named mechanisms | stress observations and model coefficients; absent data is `NOT_RUN` |
| ESD and latch-up | directed pad/rail discharge-path search with capacity, clamp and resistance constraints; guard-ring existence/continuity/width/bias/distance/tap evidence | extracted ESD network and latch-up evidence; absent topology is `NOT_RUN` |

These are analysis cores, not foundry certification. In particular, ESD design-rule
verification is not the same as passing device-level HBM/CDM qualification. The current
HBM test method is [ANSI/ESDA/JEDEC JS-001-2024](https://www.esda.org/store/standards/product/392/ansiesdajedec-js-001-2024/).
Calibre PERC’s official scope illustrates the proprietary target: context-aware
schematic/layout structure recognition, ESD paths and sizing, SPICE-accurate transient
analysis, latch-up, point-to-point resistance, current density and voltage-aware DRC
with foundry decks. See the
[Calibre PERC product page](https://www.siemens.com/en-us/products/ic/calibre-design/reliability-verification/perc/).
For latch-up specifically, production checks include guard-ring existence, width,
spacing to aggressors, danger-zone victims and designated-bias connectivity, as
summarized by Siemens’
[guard-ring verification note](https://blogs.sw.siemens.com/calibre/2025/09/23/safeguarding-ic-reliability-calibre-percs-latch-up-guard-ring-check/).

Reliability signoff also requires actual mission-profile analysis. Synopsys describes
foundry-certified EM/IR, aging, high-sigma variation, analog fault simulation and
circuit checks across early/normal/end-of-life in
[PrimeSim Reliability Analysis](https://www.synopsys.com/implementation-and-signoff/ams-simulation/primesim-reliability-analysis.html).

Finally, foundry antenna equations are more nuanced than a metal-area ratio. For
example, SKY130 distinguishes contact bottom area from metal/poly sidewall area and
applies diode area factors and fixed bonuses; see the official
[SKY130 antenna rules](https://skywater-pdk.readthedocs.io/en/main/rules/antenna.html).

## Recommended execution order

The implementation-ready work packages, ownership boundaries and acceptance
commands are tracked in the [compliance execution roadmap](compliance-roadmap.md).

1. Replace the geometry core with exact polygon booleans/offsets and a preserved
   hierarchical database, plus strict parsing/units/error handling.
2. Redesign the deck schema around independent rule IDs and rule kinds, lists/tables,
   derived layers, contexts and fail-on-unknown validation.
3. Make LVS exact and deterministic: legal layer connectivity, real hierarchy/ports,
   body pins, named-net seeding, safe symmetric reduction and broad device/property support.
4. Rebuild PEX around a distributed conductor/via node graph, calibrated multi-corner
   RC models and field-solver correlation; then add L/frequency/substrate/thermal models.
5. Correlate thousands of positive and negative structures against a foundry golden
   deck, then pursue foundry qualification. The existing 162-case suite is a useful
   unit regression, not a signoff qualification corpus.
