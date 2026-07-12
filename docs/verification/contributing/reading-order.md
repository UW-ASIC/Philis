# Contributor reading order and source ownership

Use this route instead of reading the crate front to back. It keeps format,
geometry, deck, extraction, and qualification responsibilities separate.

## Before changing code

1. Read [current status](../status.md), the [contracts](../contracts.md), and the
   package for your assigned wave under [`work-packages/`](../work-packages/).
2. Read [`verify/src/lib.rs`](../../../verify/src/lib.rs) for the public surface,
   then only the source/test files named by the package.
3. Read [`verify/src/params.rs`](../../../verify/src/params.rs) and
   [`verify/src/schema.rs`](../../../verify/src/schema.rs) before adding syntax.
   Parsed-but-ignored syntax is forbidden.
4. Read [`verify/conformance/conformance.rs`](../../../verify/conformance/conformance.rs)
   before changing result semantics, and
   [`verify/src/bin/correlation.rs`](../../../verify/src/bin/correlation.rs)
   before changing correlated artifacts or dispositions.
5. Record the supported subset, new `Unsupported` behavior, stable IDs and units,
   focused tests, full gates, and fan-in dependencies before implementation begins.

## Source map

| Concern | Read first | Adjacent tests/evidence | Owner boundary |
|---|---|---|---|
| exact geometry | `verify/src/geometry/exact.rs`, then `geometry.rs` | inline `#[cfg(test)]`; conformance geometry cases | owns predicates/booleans, not checker policy |
| GDS/OASIS/hierarchy | `gds_lossless.rs`, `gds.rs`, `oasis.rs`, `hierarchy_index.rs` | inline round-trip, malformed and seam tests | owns lossless representation/adapters, not LVS naming guesses |
| deck/schema | `schema.rs`, `params.rs` | parser/negative tests in both files | owns rejection and typed resolution, not checker shortcuts |
| DRC | `drc/mod.rs` | inline tests; `conformance/generator/drc_gen.rs` | consumes shared geometry; no private bbox substitute |
| reference netlists | `lvs/netlist.rs`, `lvs/binding.rs` | parser, include, expression, binding tests | owns syntax/binding, not layout extraction |
| LVS extraction | `lvs/extract.rs`, `lvs/detailed_extract.rs` | inline extraction tests | owns layout connectivity/device evidence |
| LVS comparison/hierarchy | `lvs/production.rs`, `lvs/hier_production.rs`, `lvs/compare.rs` | witness, reduction, flat-vs-hier tests | owns deterministic matching; adapters must not invent identity |
| PEX | `pex/mod.rs`, `lvs/spice.rs` | inline PEX/export tests; PEX conformance | owns electrical graph/models/output, not process constants |
| signoff | `signoff/mod.rs` and family files | inline analytic/boundary/error tests | consumes typed evidence until Wave 6 derives it |
| neutral correlation | `src/bin/correlation.rs`, `correlation/schemas/` | binary-local tests and fixtures | owns exchange/compare/freeze, not vendor execution |
| backend integration | `backend/src/**` references to `gdsverify` | `pnr-backend` gates | downstream consumer; coordinate API changes here |

## Ownership and forbidden overlap

One worktree owns one bounded file set. Geometry work must not silently rewrite
DRC expectations; DRC work must not introduce a private polygon kernel; LVS work
must not reinterpret GDS records; PEX work must not hard-code process/foundry
constants; correlation work must not bless deltas merely to turn a gate green.

Do not edit manifests, expected results, broad formatting, or unrelated docs unless
the task packet names them. When two packages need the same API, land and gate the
provider first, then rebase/cherry-pick the consumer. Cross-stream adapters are
their own owned package, not an implicit responsibility split across branches.

## Task-packet template

Every delegated task must state:

```text
ID and outcome:
Base commit / worktree / branch / isolated CARGO_TARGET_DIR:
Read first (exact source paths):
Prerequisites (package IDs and dependency gate IDs):
Owned files (exact repository paths/globs):
Forbidden files (exact repository paths/globs):
Public API or artifact outcome:
Supported subset and explicit Unsupported cases:
Positive / negative / boundary / adversarial tests:
Focused gates:
Full gates:
Acceptance evidence:
Score-promotion evidence (normally none):
Parallel work and fan-in hazards:
Required commit/report:
```

If the task cannot fill a field, it is not ready for parallel implementation.
