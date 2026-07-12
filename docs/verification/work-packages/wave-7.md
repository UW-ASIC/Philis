# Wave 7 work packages — correlation, capacity and qualification

Goal: prove the exact engine/deck/model/corpus combination, publish its operating
envelope and submit it for external acceptance. The repository contains a neutral
compare/freeze foundation, not vendor adapters, golden data, licenses or approval.

## Q7.1 — scalable parameterized corpus

| Field | Requirement |
|---|---|
| Read first | [`verify/conformance/`](../../../verify/conformance/), correlation schemas, all selected deck operation inventories |
| Prerequisites | stable rule/device/model IDs and supported-subset declarations from Waves 2–6 |
| Owned files | generated corpus definitions, immutable source seeds/metadata, corpus validators |
| Forbidden files | hand-edited expected pass counts, proprietary inputs without approved storage/license, self-generated “golden” as independent evidence |
| API outcome | thousands of reproducible positive/negative/boundary/adversarial cells plus representative hierarchical full-chip cases with stable IDs/hashes |
| Tasks | parameter sweeps around every threshold; geometry degeneracies; device/topology/property variants; process corners; signoff stimulus cases; minimization and deduplication |
| Unsupported/non-goals | synthetic corpus supplements but does not replace tapeout-representative designs |
| Tests | generator determinism/schema; every operation has four directions; adversarial seed replay and mutation coverage |
| Focused/full gates | generator tests, corpus validation, global gate on bounded tier |
| Acceptance/evidence | coverage matrix maps each selected operation/model to cases and expected evidence source |
| Score promotion | corpus alone does not promote score |
| Parallel/fan-in hazards | split by domain using frozen case/artifact schema; central owner prevents ID/coverage duplication |

## Q7.2 — licensed golden adapters and exact comparison

| Field | Requirement |
|---|---|
| Read first | [`verify/correlation/`](../../../verify/correlation/), [`src/bin/correlation.rs`](../../../verify/src/bin/correlation.rs), vendor result specifications |
| Prerequisites | vendor licenses/access, pinned tool/deck/model versions, Q7.1 corpus and legal data-handling plan |
| Owned files | vendor-specific adapters in approved boundary, neutral artifacts, adapter tests with redistributable samples |
| Forbidden files | vendor secrets in repo, count-only conversion, missing category -> clean, coordinate/value rounding without source preservation |
| API outcome | deterministic adapters emitting complete neutral DRC markers, LVS topology/witnesses, PEX topology/values and signoff status/provenance |
| Tasks | run orchestration; parser/converter; unit/coordinate mapping; stable IDs; metadata/hashes; artifact validation/freeze; typed tolerance and exact disposition workflow |
| Unsupported/non-goals | neutral-schema validity does not prove vendor run correctness; unsupported vendor records stop adapter |
| Tests | complete sample conversions; missing/malformed/unknown negatives; tolerance boundary; adversarial equal-count/different-geometry/topology and unit mismatch |
| Focused/full gates | adapter fixture tests, correlation binary tests, global gate |
| Acceptance/evidence | reproducible golden/actual artifact pairs and reviewed deltas by case/domain |
| Score promotion | eligible only for families with general-input independent correlation and accepted bounds |
| Parallel/fan-in hazards | adapters may split by vendor/domain; neutral schema changes require central review and backward/version policy |

## Q7.3 — capacity, determinism, restart and incremental qualification

| Field | Requirement |
|---|---|
| Read first | G2.4 tiling, D3.5 result DB/scheduler, L4.5 cache, P5.5 distributed artifacts |
| Prerequisites | semantically correlated engines and representative full-chip corpus |
| Owned files | benchmark/capacity harness, run manifests, failure-injection tests, published envelopes |
| Forbidden files | unrepeatable stopwatch anecdotes, changed hardware/toolchain without metadata, result equality by count/hash without canonical artifact inspection |
| API outcome | versioned runtime/memory/capacity envelope and deterministic full/restart/incremental/distributed modes |
| Tasks | design tiers; hardware/software metadata; thread/worker scaling; peak memory; cancellation/restart; edit invalidation; worker failure/retry; artifact equality and performance regression thresholds |
| Unsupported/non-goals | claims outside measured envelope; one machine does not establish all-platform performance |
| Tests | repeatability across schedules; ancestor/local edits; crash/corruption/lost worker; boundary at declared capacity |
| Focused/full gates | capacity smoke in CI, scheduled full runs, global gate, artifact comparison |
| Acceptance/evidence | published percentile envelopes and byte/canonical equivalence across modes with frozen metadata |
| Score promotion | supports infrastructure/capacity families only alongside independent semantic correlation |
| Parallel/fan-in hazards | domain benchmarks split after common run schema; resource-heavy runs centrally scheduled |

## Q7.4 — release freeze, delta triage and provenance

| Field | Requirement |
|---|---|
| Read first | correlation freeze/disposition schemas, [test-gates golden policy](../contributing/test-gates.md#golden-correlation-policy) |
| Prerequisites | accepted Q7.2/Q7.3 artifacts and organizational owner/signing policy |
| Owned files | release manifests, reviewed dispositions, release evidence index; secrets/keys external |
| Forbidden files | wildcard/permanent anonymous waivers, manifests containing themselves, mutable external references, repo-stored private keys |
| API outcome | immutable content-addressed engine/deck/model/corpus/results bundle with owner, expiry, exact deltas and optional organizational signatures |
| Tasks | freeze all inputs/outputs/commands; verify additions/removals; triage each delta; owner/expiry; release notes and supported scope; PKI integration if required |
| Unsupported/non-goals | v1 identity fields are not PKI; do not claim cryptographic organizational approval without external signing system |
| Tests | tamper/add/remove/path/symlink negatives; disposition expiry/scope/fingerprint; signature verification boundaries if added |
| Focused/full gates | correlation/freeze tests, clean-room verify, global gate |
| Acceptance/evidence | third party can reproduce/verify bundle and trace every delta to accepted disposition |
| Score promotion | matrix update occurs only in same review as evidence bundle and independent acceptance |
| Parallel/fan-in hazards | one release owner freezes final set; late engine/deck/model change invalidates downstream evidence |

## Q7.5 — foundry/tapeout-owner qualification

| Field | Requirement |
|---|---|
| Read first | exact foundry/owner qualification program, Q7.1–Q7.4 evidence index, [capability matrix](../compliance/capability-matrix.md) |
| Prerequisites | external agreement on supported process/deck/models, error bounds, corpus, capacity envelope and submission format |
| Owned files | repository-side submission index and non-confidential acceptance metadata; external portal/materials remain controlled externally |
| Forbidden files | self-certification, claim expansion beyond accepted versions/scope, publishing confidential qualification data |
| API outcome | precise qualified-scope record: engine/deck/model/corpus hashes, process/corners, supported inputs/outputs, limits and approval identity/date |
| Tasks | submit exact bundle; answer deltas; rerun requested cases; freeze corrections; record accepted scope and renewal/change-control triggers |
| Unsupported/non-goals | qualification for one combination does not cover another foundry, deck revision, engine commit, model or unsupported operation |
| Tests | clean-room bundle verification and qualification-requested regressions; no purely local test can substitute for approval |
| Focused/full gates | exact submitted global/correlation/capacity gates; external program gates |
| Acceptance/evidence | formal acceptance from foundry or designated tapeout owner for exact combination |
| Score promotion | mark `qualified` only from that acceptance; preserve correlation state separately |
| Parallel/fan-in hazards | external coordination is critical path; any frozen input change triggers scoped requalification |

## Wave 7 exit

The exact release is reproducible, independently correlated, capacity-qualified within
its published envelope, frozen with reviewed deltas, and formally accepted for its
declared scope. Until Q7.5 completes, describe the engine as unqualified regardless of
feature score.
