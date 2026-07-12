# Neutral correlation and freeze harness

This directory defines the Wave 7.1 exchange contract for comparing `gdsverify`
with an independently produced golden run. It is a correlation harness, not a
vendor adapter, a qualification result, or evidence of foundry certification.

The executable is built automatically from
`verify/src/bin/correlation.rs`:

```text
cargo run -p gdsverify --bin correlation -- compare GOLDEN ACTUAL \
  [--config compare-config.json] [--dispositions dispositions.json] \
  [--report report.json]
cargo run -p gdsverify --bin correlation -- freeze --root ROOT \
  [--output manifest.json] INPUT...
cargo run -p gdsverify --bin correlation -- verify-freeze MANIFEST \
  --root ROOT [--report report.json]
cargo run -p gdsverify --bin correlation -- summarize ARTIFACT \
  [--report summary.json]
```

When `--report` or `--output` is omitted, deterministic pretty JSON is written
to stdout. A one-line human summary is written to stderr. Reports contain no
current timestamp, so identical inputs produce identical JSON bytes.

## Exit codes

| Code | Meaning |
|---:|---|
| 0 | Exact match, all deltas explicitly accepted, or freeze verified |
| 2 | I/O, JSON, schema, semantic-validation, disposition, or unsafe-path error |
| 3 | At least one comparison delta has no exact valid disposition |
| 4 | Freeze manifest has an added, missing, or changed file |
| 64 | Invalid command-line usage |

An expired disposition is a schema error (2), not an ignored waiver. A valid
but nonmatching disposition is listed as unused and cannot change exit code 3.

## Run artifact v1

[`schemas/run-artifact.schema.json`](schemas/run-artifact.schema.json) is the
portable syntax. The Rust validator applies additional fail-closed invariants
that JSON Schema cannot conveniently express:

- every case/result ID is nonempty and unique in its scope;
- every content hash is lowercase SHA-256;
- timestamps use exactly `YYYY-MM-DDTHH:MM:SSZ` and are calendar-valid;
- marker geometry has one outer ring, at least three distinct points per ring,
  and an exact bounding box;
- PEX endpoints reference declared nodes, component quantity/unit matches its
  element total, and component mechanism IDs are unique;
- runtime is expressed as `runtime` in `s`, and peak memory as `memory` in
  `byte`;
- missing result categories are errors. An evaluated empty category is `[]`.

Metadata records engine and producing-tool name/version, deck/model/corpus IDs
and content hashes, process, corner, and the complete command. All metadata is
compared exactly by default. `created_at` is retained as provenance but is the
only field excluded from comparison identity. This means changing a tool,
version, deck, model, corpus, process, corner, or command produces a visible
delta.

DRC comparison uses stable marker/rule IDs, layers, measurement quantities,
bounding boxes, and complete polygon rings. Equal marker counts or equal
bounding boxes are insufficient. Ring start, direction, and result order are
canonicalized. `geometry_tolerance_nm` is an explicit per-coordinate tolerance;
it does not permit a different vertex count, ring role, or topology.

LVS comparison includes net-to-pin membership, device model/kind/terminal
mapping and parameters, and mismatch objects with layout/schematic references
and a witness. PEX comparison includes node-to-net mapping, element endpoints,
kind, total value/unit, and named physical component breakdown. The
[`fixtures/pex_386af.json`](fixtures/pex_386af.json) example represents 386 aF
using the fixture deck's actual terms: 10 aF area (`25 * 0.4`), 176 aF fringe
(`40 * 4.4`), and 200 aF mutual coupling (`100 * 2 * (200 / 200)`). It does not
record only a scalar total or invent an arbitrary component split.

Signoff results use only `clean`, `violations`, `not_run`, or `error`. Adapters
must never translate missing or failed analysis into `clean`.

## Numeric tolerances

[`schemas/compare-config.schema.json`](schemas/compare-config.schema.json)
maps a typed quantity name to nonnegative absolute and relative tolerances:

```json
{
  "schema_version": "gdsverify.correlation.compare-config/v1",
  "geometry_tolerance_nm": 0,
  "quantities": {
    "capacitance": { "absolute": 0.5, "relative": 0.001 },
    "resistance": { "absolute": 0.01, "relative": 0.001 }
  }
}
```

The accepted numeric error is
`absolute + relative * max(abs(golden), abs(actual))`. The boundary is
inclusive. Quantities absent from the map are exact. IDs, units, status,
topology, and metadata are never relaxed by numeric tolerance.

## Delta dispositions

[`schemas/dispositions.schema.json`](schemas/dispositions.schema.json) defines
reviewed exceptions. A fingerprint is SHA-256 of canonical JSON containing the
complete delta path, kind, expected value, actual value, and fingerprint schema
version. A waiver applies only when all of these are true:

1. the fingerprint matches exactly;
2. `scope` equals that delta's complete JSON-pointer path (no root scope,
   wildcards, or traversal syntax);
3. status is `approved`;
4. owner and engineering reason are nonempty;
5. creation and expiry are valid, ordered UTC timestamps and expiry is still in
   the future.

Accepted and unwaived deltas are separate arrays in the report. Rejected,
superseded, wrong-scope, and wrong-fingerprint entries remain unused. Duplicate,
expired, malformed, or wildcard dispositions fail the whole file.

## Freeze manifests

[`schemas/freeze-manifest.schema.json`](schemas/freeze-manifest.schema.json)
records canonical root-relative inputs and every regular file's byte length and
SHA-256. SHA-256 is implemented in-tree and checked against standard vectors;
platform-dependent hashers are not used. Directory inputs are re-enumerated at
verification, so additions are detected as well as changes and removals.

Paths must be UTF-8, normalized, root-relative, and remain inside `--root`.
Symlinks and special files are rejected. Inputs and entries must be unique and
strictly sorted; overlapping inputs are rejected rather than silently
deduplicated. Put an output manifest outside any directory it freezes, or it
will correctly appear as a subsequently added file.

## External adapter contract

An adapter for Calibre, Pegasus, Quantus, or another golden tool should:

1. run a pinned tool/version with a pinned deck, model set, corpus, process and
   corner;
2. hash the exact input bytes and preserve the complete invocation in metadata;
3. assign stable IDs before emitting results;
4. convert coordinates and values into explicitly declared units without
   rounding away the source measurement;
5. emit complete marker geometry, LVS topology/witnesses, PEX topology and
   physical component values—not report counts;
6. emit `not_run` or `error` for absent/failed checks;
7. validate the artifact with `summarize`, freeze all inputs/artifacts, then use
   `compare` under review-controlled tolerances and dispositions.

Vendor report formats and licenses are external to this repository. Merely
emitting a valid neutral artifact does not establish that a vendor run was
correct, that two algorithms are equivalent, or that a foundry accepts the
result.

## Regression matrix and remaining W7.2 work

Binary-local tests cover exact and stable-order matches, inclusive numeric
boundaries, unit mismatch, equal-count topology mismatch, equal-bbox/count
marker-shape mismatch, missing cases, exact/expired/mismatched dispositions,
malformed schemas, deterministic report snapshots, standard SHA-256 vectors,
and freeze tamper/add/remove/duplicate cases. The 386 aF fixture is also parsed
and checked for its three-component sum.

Wave 7.1 intentionally leaves these W7.2 dependencies unresolved:

- actual licensed-tool adapters and vendor-specific report parsers;
- thousands of generated boundary cells and representative full-chip corpora;
- independent field-solver/foundry golden values and approved error bounds;
- runtime/memory capacity envelopes, restart and incremental qualification, and
  deterministic distributed execution runs;
- cryptographic signatures backed by an organizational identity system (v1
  records review identity and immutable fingerprints, but is not a PKI format);
- formal tapeout-owner and foundry approval of the frozen engine/deck/model/
  corpus combination.

Until those are supplied and reviewed, this harness is infrastructure for
correlation—not a claim of proprietary-tool parity or signoff qualification.
