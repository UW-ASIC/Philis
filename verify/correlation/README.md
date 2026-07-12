# Neutral correlation and freeze artifacts

This directory contains the portable JSON schemas and fixtures consumed by the
`correlation` binary. The canonical semantics, disposition rules, freeze policy,
external adapter requirements and evidence ladder are in
[correlation and qualification](../../docs/verification/compliance/qualification.md).

Run from the repository root:

```text
cargo run -p gdsverify --bin correlation -- compare GOLDEN ACTUAL \
  [--config verify/correlation/fixtures/compare-config.json] \
  [--dispositions DISPOSITIONS] [--report REPORT]

cargo run -p gdsverify --bin correlation -- freeze --root ROOT \
  [--output MANIFEST] INPUT...

cargo run -p gdsverify --bin correlation -- verify-freeze MANIFEST \
  --root ROOT [--report REPORT]

cargo run -p gdsverify --bin correlation -- summarize ARTIFACT \
  [--report SUMMARY]
```

Exit codes: 0 accepted match/freeze; 2 input/schema/semantic error; 3 unaccepted
delta; 4 changed freeze; 64 invalid usage.

Files in this directory:

- `schemas/run-artifact.schema.json`: full DRC/LVS/PEX/signoff/capacity exchange
- `schemas/compare-config.schema.json`: typed numeric/geometry tolerances
- `schemas/dispositions.schema.json`: exact scoped, owned and expiring deltas
- `schemas/freeze-manifest.schema.json`: normalized content-addressed input freeze
- `fixtures/pex_386af.json`: 10 aF area + 176 aF fringe + 200 aF mutual coupling

This harness validates exchange structure and deterministic comparison. Licensed
vendor adapters, golden runs, foundry data and qualification approval are external.
