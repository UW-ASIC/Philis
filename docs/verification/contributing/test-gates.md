# Test gates, worktrees, and golden policy

## Isolated-worktree workflow

Start from the orchestrator-named, clean commit. Never branch from a worktree with
uncommitted user changes.

```bash
git branch codex/<bounded-task> <exact-base-sha>
git worktree add /tmp/pnr-<task> codex/<bounded-task>
export CARGO_TARGET_DIR=/tmp/pnr-target-<task>
```

Commit only owned files. Report the exact commit SHA, file list, focused/full gate
results, remaining `Unsupported` cases, and integration hazards. The fan-in owner
cherry-picks reviewed commits in dependency order and resolves no semantic ambiguity
without returning it to the producing owner.

## Required local evidence

A feature requires all four directions appropriate to its domain:

- positive: a valid structure is accepted or produces the expected object/value;
- negative: a violating/mismatching/malformed structure cannot pass silently;
- boundary: equality, one unit below, and one unit above the limit;
- adversarial: degeneracy, overflow, ambiguity, hierarchy seams, ordering, or
  incomplete evidence most likely to expose a false clean.

A bug fix requires a minimal reproducer and a negative-direction test. Changed
expected output requires a physical/semantic explanation plus a focused regression.
Changing an expected manifest alone is never acceptance.

## Focused gates

Run the smallest owning unit target while iterating, then at least:

```bash
cargo test -p gdsverify --lib <focused_test_filter>
cargo test -p gdsverify --all-targets --no-run
```

Format readers add round-trip and malformed/fuzz corpora. Geometry adds a second-engine
differential corpus. LVS adds both hierarchy-preserving and flattened comparison.
PEX adds analytic structures and topology/value comparison. Signoff adds missing-input,
singular/ambiguous, boundary, and provenance cases.

## Global integration gate

Run from the combined, settled fan-in tree with one isolated target directory:

```bash
cargo test -p gdsverify --lib
cargo run -p gdsverify --bin conformance -- verify/conformance
cargo test -p gdsverify --all-targets --no-run
cargo test -p pnr-core --lib --no-run
cargo test -p pnr-backend --lib --no-run
```

When backend behavior changes, also run `cargo test -p pnr-backend --lib`. When
correlation code or schema changes, run `cargo test -p gdsverify --bin correlation`.
GPU work adds CPU/GPU identical-report tests on a configured runner; absence of CUDA
must not be presented as GPU validation.

`CLEAN` is valid only when all required input is complete, valid, and evaluated for
the declared scope. `NOT_RUN`, extraction diagnostics, unknown syntax, unsupported
geometry, singular electrical systems, or missing model limits are never clean.

## Golden correlation policy

Compare substance, not counts:

| Domain | Required comparison |
|---|---|
| DRC | stable rule ID, layer, measurement/units, complete marker rings, hierarchy path |
| LVS | nets/pins, devices/models/terminals/properties, mappings, actionable witness |
| PEX | conductor nodes, endpoints, element kind/value/unit, mechanism breakdown, corner |
| signoff | status, geometry/device/net identity, stimulus/corner, model and limit revisions |
| capacity | deterministic result hash, runtime, peak memory, restart/incremental/distributed mode |

Tolerances are typed, explicit, reviewed, and pinned. A disposition must name an owner,
engineering reason, exact fingerprint/scope, creation and expiry. Missing/expired/wildcard
dispositions fail closed. Foundry/vendor data, licenses, qualified decks, field-solver
values, and approvals are external prerequisites; synthetic fixtures cannot replace them.

The 386 aF fixture has a required physical disposition: **10 aF area + 176 aF fringe
+ 200 aF mutual coupling = 386 aF**. Preserve the three mechanisms and units, not
only their scalar total.

## Score promotion

Unit or conformance tests may move a package from absent to `foundation` or `partial`.
Only general supported input plus independent correlation evidence can promote a
capability score. Only the foundry/tapeout owner can mark the exact frozen combination
`qualified`. Update [`capabilities.json`](../compliance/capabilities.json) only in a
review that includes that evidence and a matching human-readable matrix change.
