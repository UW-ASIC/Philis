# Wave 7 work packages — correlation, capacity and qualification

Goal: prove the exact engine/deck/model/corpus combination, publish its operating
envelope and submit it for external acceptance. The repository contains a neutral
compare/freeze foundation, not vendor adapters, golden data, licenses or approval.

## Q7.1 — scalable parameterized corpus

| Field | Requirement |
|---|---|
| Read first | [`verify/conformance/`](../../../verify/conformance/), correlation schemas, all selected deck operation inventories |
| Prerequisites | accepted subsets of G2.1–G2.4, D3.1–D3.5, L4.1–L4.5, P5.1–P5.5 and R6.1–R6.5 with stable IDs/capability declarations; incomplete packages may contribute only explicitly labeled foundation cases |
| Owned files | `verify/conformance/generator/**`, planned `verify/correlation/corpus/{drc,lvs,pex,signoff}/**`, and planned corpus validators under `verify/correlation/corpus/tools/**` |
| Forbidden files | `verify/conformance/manifest.json`, engine files under `verify/src/**`, proprietary inputs outside approved external storage, and self-produced artifacts labeled independent golden |
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
| Prerequisites | Q7.1 accepted; `EXT-DECK`, `EXT-PROCESS`, `EXT-LICENSE` and `EXT-GOLDEN` satisfied |
| Owned files | planned `verify/correlation/adapters/**`, redistributable adapter fixtures under planned `verify/correlation/fixtures/adapters/**`, and generated neutral artifacts in approved external storage only |
| Forbidden files | engine files under `verify/src/**`, `verify/correlation/schemas/**` except a separately owned schema-version commit, vendor secrets anywhere in the repository, and count-only/missing-category-clean conversions |
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
| Prerequisites | D3.5, L4.5, P5.5, Q7.1 and Q7.2 accepted; `EXT-GOLDEN` satisfied |
| Owned files | planned `verify/correlation/capacity/**`, planned `benchmark/verification/**`, and published envelopes under planned `docs/verification/qualification-records/capacity/**` |
| Forbidden files | engine files under `verify/src/**`, hand-edited golden artifacts, benchmark outputs without hardware/toolchain metadata, and count-only equality reports |
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
| Prerequisites | Q7.2 and Q7.3 accepted; `EXT-SIGNING` satisfied |
| Owned files | planned `verify/correlation/releases/**`, reviewed dispositions under planned `verify/correlation/releases/*/dispositions.json`, and planned `docs/verification/qualification-records/releases/**`; keys remain external |
| Forbidden files | engine files under `verify/src/**`, wildcard/permanent anonymous waiver files, manifests containing their output directory, mutable external references, and private keys anywhere in the repository |
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
| Prerequisites | Q7.4 accepted and `EXT-QUALIFICATION` satisfied |
| Owned files | planned `docs/verification/qualification-records/accepted-scope/**` and non-confidential submission indexes under planned `docs/verification/qualification-records/submissions/**`; external portal materials remain external |
| Forbidden files | engine files under `verify/src/**`, capability-score files outside the acceptance review, self-issued approval records, claim expansion beyond accepted hashes/scope, and confidential qualification data |
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
