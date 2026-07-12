# Correlation and qualification

## Neutral artifact contract

The correlation binary and schemas under [`verify/correlation/`](../../../verify/correlation/)
define an exchange format, comparison policy and content freeze. They do not run a
vendor tool or establish foundry acceptance.

A run artifact records producing engine/tool/version, deck/model/corpus IDs and hashes,
process/corner, exact command and creation provenance. Missing result categories are
errors; an evaluated empty category is `[]`. DRC includes complete marker rings and
measurements; LVS includes nets/devices/terminals/properties/mappings/witnesses; PEX
includes nodes/endpoints/elements/units/physical breakdown; signoff uses only the four
contract statuses.

Numeric tolerance for a typed quantity is:

```text
absolute + relative * max(abs(golden), abs(actual))
```

The boundary is inclusive. IDs, units, status, topology and metadata are exact.
Geometry tolerance applies per coordinate but does not permit changed vertex count,
ring role or topology.

## Dispositions

A delta disposition applies only to an exact canonical fingerprint and complete
JSON-pointer scope. It requires `approved` status, nonempty owner and engineering
reason, ordered creation/expiry timestamps, and an unexpired date. Wildcards,
duplicates, expired entries, wrong scope/fingerprint and malformed files fail closed.
Accepted and unwaived deltas remain separate in the report.

## Freeze

Freeze manifests contain normalized root-relative regular-file paths, byte lengths and
SHA-256. Verification detects additions, changes and removals. Symlinks, special files,
path traversal, overlapping/duplicate inputs and noncanonical ordering are rejected.
Place the output manifest outside any directory it freezes.

## External adapter requirements

A vendor/golden adapter must:

1. run pinned tool, deck, model, corpus, process and corner revisions;
2. hash exact inputs and retain the full invocation;
3. preserve source precision/units and assign stable IDs;
4. convert full marker geometry, LVS topology/witnesses and PEX element topology/
   breakdown, not counts;
5. emit `NOT_RUN`/`ERROR` for absent or failed checks;
6. validate and freeze its artifact before comparison;
7. keep proprietary data/licenses in their approved external environment.

## Evidence ladder

| Level | Required evidence | Allowed claim |
|---|---|---|
| local regression | unit/conformance positive+negative+boundary+adversarial tests | foundation/partial implementation |
| independent correlation | pinned independent producer; full object/value compare; reviewed tolerances/deltas | correlated supported family |
| capacity qualification | representative full-chip deterministic runtime/memory, restart/incremental/distributed evidence | published operating envelope |
| foundry/owner acceptance | exact frozen combination and formal accepted scope | qualified for that scope only |

Required external prerequisites remain: licensed vendor adapters/runs, selected foundry
decks/models, thousands of boundary cells and representative full chips, independent
field-solver/golden values and approved bounds, organizational signing if required, and
formal tapeout-owner/foundry approval. See [Wave 7](../work-packages/wave-7.md).
