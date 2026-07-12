# Proprietary-compliance execution roadmap

Status date: 2026-07-12. The detailed capability score and bug evidence live in
[`proprietary-signoff-audit.md`](proprietary-signoff-audit.md). This document turns
that audit into bounded implementation waves with objective exit gates.

## Qualification target

“Proprietary-level” is reached only when all six gates below are satisfied for a
specific process, deck revision, engine revision, and supported input/output set.

| Gate | Required evidence |
|---|---|
| Semantic coverage | A machine-readable inventory shows that every operation, context, table, property, device and reduction used by the target foundry decks is implemented or explicitly rejected. Unknown syntax/data is a hard error. |
| Numerical correctness | Positive, negative and boundary-value structures agree with an independent golden implementation, including marker geometry, measurements, extracted topology and properties. |
| Capacity | Hierarchical production designs complete within declared memory/runtime limits; restart, deterministic parallel execution and incremental reruns are tested. |
| Debug/product workflow | Stable rule IDs, waiver provenance, result databases, hierarchy paths, layout/schematic cross-probe and machine-readable reports are present. |
| Reliability coverage | Antenna, density/CMP, EM/IR, voltage/aging, ESD and latch-up consume extracted design evidence and foundry-qualified limits. Missing evidence is never clean. |
| Foundry qualification | The foundry or tapeout owner qualifies the exact engine/deck/version combination and accepts correlation deltas. Feature completion alone is not certification. |

The audit baseline is **24.5 / 112 capability-family equivalents (22%)**:
DRC 12.0/44, LVS 8.0/36 and PEX 4.5/32. These are product-capability scores,
not line or branch coverage.

## Active execution ledger

This ledger prevents later waves from racing shared foundations. “Queued” means
the work package is specified but must wait for the named API dependency.

| Package | State | Ownership / dependency |
|---|---|---|
| Wave 1 integration gate | Passed | Independent settled-tree gate passed; exact command evidence is recorded below |
| G2.1a exact predicates + rectilinear booleans | Completed | Accepted shared-kernel foundation; consumer migration remains in G2.1b/Waves 3–5 |
| W4.1 SPICE/CDL parser foundation | Completed | Strict parser/AST foundation accepted; production dialect, evaluation and hierarchy gaps remain Wave 4 work |
| G2.2 lossless GDS/hierarchy semantics | Queued | Starts after Wave 1 GDS API freezes and G2.1 types are reviewed |
| G2.3 OASIS reader | Queued | Starts after G2.2 defines the lossless layout database contract |
| G2.4 hierarchical indexes/tiling | Queued | Starts after G2.1 and G2.2 |
| Wave 3 DRC semantics | Queued | Starts after G2.1 boolean API freezes |
| Wave 4 extraction/compare/hierarchy | Queued | Consumes the completed W4.1 parser foundation plus G2.1/G2.2 |
| Wave 5 distributed PEX | Queued | Consumes G2.1, LVS terminal identity and process-stack schema |
| Wave 6 evidence extraction | Queued | Consumes Waves 4–5 and signoff model provenance schema |
| W7.1 correlation harness foundation | Completed | Local marker/value/topology harness accepted; qualification still requires external golden/foundry access |

## Wave 0 — audit and fail-closed signoff foundation

State: implemented in the current working tree; the settled-tree Wave 1 integration
regression passed. Wave 6 still must derive the typed evidence from real design inputs.

| ID | Code surface | Concrete deliverable | Exit test |
|---|---|---|---|
| S0.1 | `src/signoff/antenna.rs` | Per-connected-net gate/collector metrics, area and sidewall modes, diode correction, traceable rule IDs | Disconnected collectors do not count; extraction/unsupported geometry returns `ERROR`; violating EGAR is localized |
| S0.2 | `src/signoff/density_cmp.rs` | Explicit die, covering windows, exact rectangular-union density, gradient and calibrated thickness limits | Empty windows are evaluated; overlap is counted once; gaps/invalid models return `ERROR` |
| S0.3 | `src/signoff/power.rs` | Resistive DC solve, absolute/percent IR limits, metal/via EM, temperature derating and Blech handling | Singular islands and missing limits fail closed; analytic networks match hand calculations |
| S0.4 | `src/signoff/reliability.rs` | Voltage/thermal checks and named inverse-power/Arrhenius lifetime models | Mission-profile inputs are validated; boundary and accelerated-stress cases are pinned |
| S0.5 | `src/signoff/esd_latchup.rs` | Directed, capacity-qualified ESD paths and guard-ring/bias/distance/tap evidence | Missing paths/rings violate; malformed or incomplete evidence returns `ERROR` |
| S0.6 | `src/signoff/mod.rs`, backend facade | Four-state `CLEAN / VIOLATIONS / NOT_RUN / ERROR` suite aggregation | `all_clean()` is false unless all six families actually ran and passed |

## Wave 1 — close known false-clean defects

State: the implementation streams are merged and the independent settled-tree
integration gate passed. Capability scores remain frozen until independent correlation
evidence exists.

### W1-A: DRC deck and GDS integrity

Code owner surface: `src/schema.rs`, `src/params.rs`, `src/gds.rs`, DRC/deck/GDS tests.

1. Separate stable rule `id` from rule `kind` and accept multiple instances of
   every kind while preserving legacy map decks.
2. Reject duplicate IDs, unknown layer names, non-finite thresholds, invalid
   ranges and malformed device/connectivity/PEX references.
3. Reject truncated/invalid GDS records, consume and expose `UNITS`, and retain
   explicit diagnostics for unmapped input layers.
4. Pin both modern-list and legacy-map parsing with positive and negative tests.

Exit gate: malformed decks/GDS cannot silently remove a check; two `min_width`
rules survive parse/resolve/run with distinct IDs; all prior valid decks still load.

### W1-B: LVS topology and normalization

Code owner surface: `src/lvs/**` and LVS-only tests.

1. Replace bbox-only electrical contact decisions with exact rectilinear
   intersection/touch predicates.
2. Join different conductor layers only through declared via connectivity.
3. Restore safe series normalization to a fixpoint. Compatibility, shared-net
   degree, named/global/gate use and device parameters must all be proven before
   a merge.
4. Add adversarial concave false-short, legal boundary-contact, illegal
   cross-layer overlap and positive/negative series tests.

Exit gate: the existing `LVS_SERIES_MERGE` regression passes without introducing
a false merge; the complete conformance suite and LVS unit suite are green.

### W1-C: PEX electrical semantics

Code owner surface: `src/pex/**`, `src/lvs/spice.rs` and PEX/SPICE-only tests.

1. Give every supported conductor both resistance and ground capacitance.
2. Base capacitance on actual rectilinear area/perimeter and make unsupported
   resistance geometry diagnostic instead of zero-valued.
3. Remove unconditional, hard-coded fill/shield multipliers.
4. Make report component ratios dimensionally valid.
5. Never export a parasitic resistor to a dangling node; emit a valid mapped
   lumped network or explicitly omit/diagnose it.

Exit gate: wire R and C coexist, nonrectangular area is correct, and a parsed
SPICE connectivity test proves every emitted parasitic element affects a circuit node.

### W1 integration gate

The orchestrator reviews every diff for ownership, API compatibility and
fail-closed behavior, then runs:

```text
cargo test -p gdsverify --lib
cargo run -p gdsverify --bin conformance -- verify/conformance
cargo test -p pnr-core --lib --no-run
cargo test -p pnr-backend --lib --no-run
```

Settled-tree evidence on 2026-07-12:

| Gate command | Result |
|---|---|
| `cargo test -p gdsverify --lib` | PASS — 78 passed, 0 failed |
| `cargo run -p gdsverify --bin conformance -- verify/conformance` | PASS — 162 passed, 0 failed; all 51/51 coverage entries have a negative test |
| `cargo test -p pnr-core --lib --no-run` | PASS — downstream API compile gate |
| `cargo test -p pnr-backend --lib --no-run` | PASS — downstream API compile gate |
| `cargo test -p pnr-backend --lib` | PASS — 18 passed, 0 failed |
| `cargo test -p gdsverify --bin correlation` | PASS — 16 passed, 0 failed |

The changed PEX per-net expectation is physically decomposed and independently pinned:
10 aF area + 176 aF fringe + 200 aF mutual coupling = 386 aF. The `PEX_FILL`
expectation is the unmodified 14.6025 aF analytical result with a 1e-6 aF tolerance;
no synthetic fill/shield correction remains.

Any changed expected result needs a physical explanation and a focused regression;
updating a manifest alone is not acceptance.

Wave 1 compatibility dispositions:

- Legacy DRC object maps remain accepted, but zero-valued implicit disable is rejected;
  use `enabled:false`. Disabled entries are still schema-validated.
- `load_gds` now requires a valid `UNITS` record matching `deck.dbu_nm`; callers that
  intentionally inspect legacy unitless streams must use low-level `read_gds` and make
  their own explicit unit decision.
- Unknown fields inside recognized verification objects are errors. Unrelated top-level
  sections remain accepted because the repository shares one full-PDK document among
  multiple consumers.
- Additive fields on public report/config enums and structs (`GdsLayout`, `Parasitic`,
  `FlowConfig`, `SignoffReport`) are source-visible API changes; in-workspace consumers
  use `Default`/wildcard patterns and compile gates cover them.

## Wave 2 — exact geometry and preserved hierarchy

Target score after correlation: DRC at least 24/44 and LVS at least 15/36.

| ID | Deliverable | Required acceptance corpus |
|---|---|---|
| G2.1 | One robust integer/rational polygon kernel for union, intersection, subtraction, holes/keyholes, edge classification, offsets and arbitrary angles | Random differential tests against a second geometry engine; degeneracy, overflow and winding corpus |
| G2.2 | Exact GDS BOUNDARY/PATH semantics and lossless hierarchy, SREF/AREF transforms, properties and text association | Round-trip fixtures for every supported record/transform plus malformed-input fuzzing |
| G2.3 | OASIS input with explicit supported-feature declaration | OASIS/GDS equivalent-layout hash and result equivalence corpus |
| G2.4 | Hierarchical spatial indexes and deterministic tiling | Flat-vs-hierarchical result equivalence and seam-marker tests |

No DRC/LVS checker may retain a private bbox substitute after G2.1; bbox remains
only a conservative candidate filter.

## Wave 3 — production DRC deck semantics

Target score: at least 38/44 before foundry qualification.

1. Add derived-layer boolean expressions and typed measurement variables.
2. Add conditional/table-driven width, spacing, PRL, EOL, enclosure, cut class,
   density and antenna rules with same/different-net, voltage, region, cell and
   hierarchy contexts.
3. Replace greedy multi-patterning with a complete bounded coloring/decomposition
   solver including precolors, stitches and conflict witnesses.
4. Add fill generation, exclusion regions and calibrated multi-level CMP models.
5. Add stable result databases, hierarchy paths, waiver fingerprints/provenance,
   incremental invalidation and deterministic distributed execution.

Exit gate: every operation used by the selected foundry deck has a schema entry,
implementation, negative test and golden-correlated marker case. Unsupported
operations stop deck loading.

## Wave 4 — production LVS

Target score: at least 31/36 before foundry qualification.

1. Parse the required SPICE/CDL/Spectre subset with parameters, expressions,
   buses, globals, includes and model aliases.
2. Implement exact connectivity, soft-connect/open handling, port/text/property
   binding and four-terminal body/well comparison.
3. Add complete foundry device recognition/property extraction for MOS, R, C,
   diode, BJT and selected custom/parameterized devices.
4. Use deterministic graph matching with named-net seeds, legal pin swaps,
   symmetric reference/layout reductions and actionable open/short witnesses.
5. Implement real hierarchical subcircuit binding, black boxes, equated cells
   and selective flattening.

Exit gate: hierarchy-preserving and fully flattened runs are equivalent on the
corpus; every mismatch includes schematic/layout objects and a reproducible witness.

## Wave 5 — distributed, multi-corner PEX

Target score: at least 27/32 before field-solver/foundry qualification.

1. Replace scalar per-net sums with a conductor/via node graph whose terminals
   map to extracted devices and ports.
2. Add process-stack/dielectric/corner models, width/thickness/temperature/size
   effects, via arrays and topology-preserving reduction.
3. Extract ground, lateral, vertical and diagonal coupling with same-net and
   shield awareness; model fill through explicit process parameters.
4. Add junction/device/substrate parasitics, then RLC/frequency/electrothermal
   options for processes that require them.
5. Emit validated DSPF/SPEF and selected-net/multi-corner outputs.

Exit gate: element topology and values correlate against field-solver structures
and the target golden extractor across declared corners and error bounds.

## Wave 6 — reliability extraction integration

The Wave 0 analyzers currently consume typed evidence. This wave derives that
evidence from layout, netlist, activity and mission-profile inputs:

1. fabrication-stage antenna networks and foundry gate/diode equations;
2. extracted power-grid topology, vector/vectorless currents and thermal map;
3. device terminal voltage histories and named aging mechanisms;
4. context-aware ESD clamp/trigger/rail paths with device sizing and point-to-point R;
5. substrate/well topology, injector/victim identification and geometric guard-ring evidence.

Exit gate: every reported electrical result links back to extracted geometry,
device/net identity, stimulus/corner, model revision and limit revision.

## Wave 7 — correlation, capacity and qualification

1. Build thousands of parameterized positive/negative/boundary cells plus
   representative full-chip regressions.
2. Compare marker geometry, measurement values, net/device topology and parasitic
   values—not only pass/fail counts.
3. Publish deterministic runtime/memory/capacity envelopes and exercise restart,
   incremental and distributed modes.
4. Freeze engine, deck, model and corpus versions; triage every delta with an
   owner and accepted disposition.
5. Submit the exact combination for foundry/tapeout-owner qualification.

## Permanent merge policy

- A checker returns `CLEAN` only with complete valid inputs and evaluated scope.
- Every rule/model has a stable unique ID and units.
- Unknown syntax, layers, devices, properties, units or unsupported geometry are
  errors, never skipped checks.
- Every bug fix adds a minimal reproducer and a negative-direction test.
- Golden deltas require an engineering disposition; expected files are not
  changed merely to make a test green.
- Capability scores are updated only when the family works on general supported
  input and has independent correlation evidence.
