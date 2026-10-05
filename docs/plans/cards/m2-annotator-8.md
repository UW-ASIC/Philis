# M2+ annotator — segment 8 (M6): EXT-27, EXT-28, EXT-29

Base: m2-annotator at f516518 (`git merge m2`: already up to date). `docs/plans/dag-m2-m6.{json,md}` are not in this
tree (dropped from the branch; read from main `HEAD`): EXT-27 hard EXT-13, EXT-14 (FLOW-07 done); EXT-28 hard
EXT-13, EXT-17; EXT-29 hard EXT-16, EXT-17. No step-level notes; PLC-06 not needed.

| Item | Class | Why |
|---|---|---|
| EXT-27 | do (verify only) | Landed in segment 6 (b8dec72, review fixes 9438b1d). Acceptance test exists and passes. No edit. |
| EXT-28 | do (verify only) | Landed in segment 6 (9d69527). Gold `Order` reproduced by a passing test. No edit. |
| EXT-29 | do (verify only) | Landed in segment 6 (e2764d0). Unit and corpus acceptance tests pass. No edit. |

The plan text (plan-01 §EXT-27..29) is stale on all three: "Current: none / flat parser" no longer holds. The
departures from the plan are recorded in `m2-annotator-report.md` "EXT-27/28/29 (segment 6)" and card 6; facts below.

## EXT-27 Hierarchy
- Code: `backend/annotator/src/hier.rs`: `devices` :17, `corresponding` :38, `same_template(nl, drawn) -> Vec<(u32,u32)>`
  :64, `arrays(nl, drawn, bias: &[bool]) -> Vec<Vec<u32>>` :73 (plan said `canon, classes`; bias is the structural
  `classify::bias_lines` because Bias classes are assigned after the requirement graph), `ports_pair` :93 (pub).
  Parser: `frontend/library/src/parse.rs:327` fills `Netlist.insts` (the plan's `parse.rs:35-39` "flat" is stale).
- Tests: `hier::tests::{two_identical_otas_pair_up :180, identical_inverters_mirror, bias_shared_array :229,
  flat_netlist_unchanged}`; acceptance `frontend/library/tests/hier_annotate.rs::two_instance_fd_ota` (parsed
  `.subckt`, two X instances: per-instance DiffPair leaf, cross-instance match).
- Command: `cargo test -p annotator hier::`; `cargo test -p library --test hier_annotate`.

## EXT-28 Signal-flow and current-flow ordering
- Code: `backend/annotator/src/flow.rs`: `stage_order(hg, classes, ports, canon) -> Vec<Vec<DeviceId>>` :27,
  `current_paths(hg, op: Option<&OpFacts>, classes, canon) -> Vec<(Vec<Vec<DeviceId>>, f64)>` :77.
- Contract (differs from plan-01/plan-04 PLC-25 L624): `Intent.order: Vec<analog::intent::Order { steps:
  Vec<Vec<DeviceId>>, dir: AxisDir, reversible, weight }>`, `steps[0]` bottom/left; `AxisDir` per gap-critic C14.
  PLC-25 must read this shape.
- Tests: `flow::tests::three_stage_stage_order :201`; `tests/align_gold.rs::{strongarm_order_matches_gold :90,
  gold_order_entry :107}`; `symmetric_blocks_become_seeds` expects `(4, 0, 4)` (sidecar `Order` consumed).
- Command: `cargo test -p annotator flow:: --test align_gold` (or the full `-p annotator` run below).

## EXT-29 Structure and bias audit
- Code: `backend/annotator/src/audit.rs`: `KINDS` :12 (vgst_low, clm_mismatch, cascode_ratio, cascode_bulk,
  bjt_ratio, vce_unequal, audit_not_checked), `audit(intent, nl, cascodes: &[[DeviceId;4]], op: Option<&OpFacts>)
  -> Vec<Diagnostic>` :34 (plan signature `drawn, ev` stale). Missing op data → `audit_not_checked`, never a
  guessed diagnostic. Library writes `audit: Vec<String>` + `AUDIT:` line in the metadata report.
- Sizing checks (H13-43/44, D*) belong to GAP-02 in MAT (gap-critic C3), not here.
- Tests: `audit::tests::{vgst_floor :160, bjt_ratio_odd :169, cascode_ratio_mismatch :177}` plus a CLM case;
  acceptance `tests/corpus.rs::audit_on_corpus` (every corpus circuit, no panic; three_stage no `vgst_low`).

## Verification run (this segment, load avg ~60)
- `PDK_ROOT=… cargo test -p annotator --no-fail-fast`: lib 138, align_gold 5, corpus 37 pass; `scale` fails in
  debug only (it is a release test, T8 2.0 s).
- `cargo test --release -p annotator --test scale`: pass (0.96 s).
- `cargo test -p library --test hier_annotate`: pass.

No product edits; nothing deferred.
