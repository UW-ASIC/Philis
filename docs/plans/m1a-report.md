# M1a report: verify pass 1

This pass checks the M1 exit criteria owned only by M1a items (00-MASTER-PLAN.md §5 "### M1": the EXT and PERF
bullets, the carried "`cargo test` 0 failed", and field report 01's `post_layout_rc_filter_simulates`) on branch `m1a`
at `2a09c30` (Merge fix-export; GPurify 6341f18; re-vendor decks), which is `main` with no M1a item merged.

Every number was measured on 2026-10-03 in the integration worktree, release profile:

- `cargo build --release --workspace` (exit 0)
- `cargo test --release --workspace --no-fail-fast`, twice: once without `PDK_ROOT`, once with it (below)
- `cargo run --release -p benchmark --bin bench local`, twice, same split
- `cargo test --release -p annotator --test corpus -- --ignored`, to measure the EXT placeholders

**Environment.** `~/.volare` does not exist on this machine, so with `PDK_ROOT` unset the bench finds no sky130 ngspice
library (`benchmarks/src/bench.rs:146`, `op_config`): every row printed `bias none`, Electromigration read 46 unknown,
CommonNode and ThermalGradient 6 unknown each, and IrDrop had no rows (total 217). The second run set
`PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk` (read only), and the numbers below come from it. The test
counts are identical in both runs, and neither log has a `skipping`/`unavailable` line.

**Verdict: M1a is not done.** No item was merged, so none of the criteria below can be met by M1a work. The one
criterion that is fully measurable without M1a code, `cargo test` 0 failed, is not met (`hier_elaborate`).

## 1. Items

Merged: none.

Not merged, all with reason `deferred-cap` from the workflow (no branch was offered for merge): FLOW-07, FLOW-13,
PERF-08, PERF-03 (step 3), PERF-30, PERF-09, EXT-02, EXT-03, EXT-04, EXT-06, EXT-10, GAP-10, EXT-05, EXT-07, EXT-09,
EXT-11, GAP-04.

## 2. Exit criteria (00-MASTER-PLAN.md §5, M1, M1a-owned)

| # | Criterion | Owner | Status | Evidence |
|---|---|---|---|---|
| 1 | `negative_corpus` passes for `sc_switches` and `equal_fets` (T3) | EXT-04 | not applicable (not merged) | `negative_corpus_sc_switches_and_equal_fets` is still `#[ignore = "passes after EXT-04"]`; with `--ignored` it FAILS at backend/annotator/tests/corpus.rs:132 |
| 2 | `latch` has 2 DiffPair leaves | EXT-04/06 | not applicable (not merged) | `corpus_expectations` still pins `latch` to two `Group` leaves (corpus.rs:98) and passes |
| 3 | `bjt_mirror` collector nets `Signal` | EXT-02 | not applicable (not merged) | not measured; no test on the branch checks it |
| 4 | 0 `Differential` in `hard` | EXT-09 | not applicable (not merged) | bench constraint table: `Differential hard 3` |
| 5 | `permutation_invariance` on `canon_leaves` (T4) | EXT-06 | not applicable (not merged) | still `#[ignore]`; with `--ignored` it FAILS at corpus.rs:172, `ota5t seed 1`: `net_pairs {}` vs `{("vout1", "vout2")}` |
| 6 | T6: 0 device pairs with both Proximity and Isolation on every corpus circuit, strongarm included | GAP-04 | not applicable (not merged) | `no_emitted_conflicts` ok (strongarm excluded); `no_emitted_conflicts_strongarm` still `#[ignore]`, with `--ignored` FAILS at corpus.rs:203 |
| 7 | T7 (coverage) | EXT-10 | not applicable (not merged) | `coverage_is_total` still `#[ignore]` with no body: `not implemented: EXT-10 adds Problem::coverage` (corpus.rs:243) |
| 8 | T8 (≤ 2.0 s on 12,500 devices, release) | EXT-06 | not applicable (not merged) | `scale.rs::twelve_thousand_devices` still `#[ignore]`; not run (M0 measured > 13 min) |
| 9 | `BiasSummary.resolved == devices` on rc_filter and bjt_mirror | PERF-09 | not applicable (not merged) | bench: bjt_mirror `bias none` with `[op] operating point unavailable (no device operating points: )`; rc_filter `bias 28 uW, probe` with EM unknown 1 |
| 10 | No `bsim4v5.out` in the cwd | PERF-09 | not applicable (not merged) | none found in the worktree outside `target/` after both test and bench runs; the isolated-deck change it checks is not on the branch |
| 11 | OTA with the new `.subckt` header: 0 `floating_gate` rows; old header 2 (vbias, vbn) | FLOW-07, PERF-03 step 3 | not applicable (not merged) | `floating_gate` is in the skipped list of all 10 bench rows (exempt: no port list) |
| 12 | `cargo test --release --workspace` 0 failed (`hier_elaborate` green) | FLOW-13 step 0 | **not met** (FLOW-13 not merged) | 444 passed, 1 failed, 7 ignored; the failure is `hier_elaborate` (§3) |
| 13 | `post_layout_rc_filter_simulates` green with ngspice | PERF-30 | not applicable (not merged) | the test does not exist on the branch (`grep -rn` empty) |

## 3. Test summary

`cargo test --release --workspace --no-fail-fast`: **444 passed, 1 failed, 7 ignored** (exit 101), the same with and
without `PDK_ROOT`.

- FAILED: `frontend/library/tests/hier_elaborate.rs::hierarchical_composition_elaborates_with_a_correct_schematic`,
  panicked at hier_elaborate.rs:216: `hierarchical layout must be LVS-clean against its schematic`, 8 ×
  `lvs/lvs.unpaired_device` and 17 × `lvs/lvs.unpaired_net`. Unchanged from M0 (same counts); owner FLOW-13 step 0.
- Ignored (7, all with their owner in the attribute): `align_gold.rs::strongarm_matches_align_gold` (EXT-14);
  `corpus.rs::coverage_is_total` (EXT-10), `negative_corpus_sc_switches_and_equal_fets` (EXT-04),
  `no_emitted_conflicts_strongarm` (EXT-14/REL-09), `permutation_invariance` (EXT-06); `scale.rs::twelve_thousand_devices`
  (EXT-06); `signoff_fixtures.rs::large_fixtures_sign_off_within_baseline` (full flow, run with `--ignored`).
- `--ignored` on `corpus.rs`: 0 passed, 4 failed (rows 1, 5, 6, 7 above).

M0's other red test is now green: `fixtures_sign_off_within_baseline` (dac4 at `feedback_iters = 1`, M0 criterion 2)
is ok, with ERC 0 for dac4. That comes from the fix-export/GPurify merge on `main`, not from M1a, and is outside this
pass's criteria (FLOW-17, RTE-06).

## 4. Bench (`bench local`, seed 1, sky130, `PDK_ROOT` set)

Exit 0; 10/10 circuits placed and routed. Per-circuit times sum to 99.7 s. The ms column is for orientation only. The
only stderr was `[op] operating point unavailable (no device operating points: ); continuing with zero power` for the
two BJT fixtures, as in M0.

| circuit | ms | cells / nets | WL nm | unrouted | route hard | overuse | DRC | LVS | ERC | warn | C Σ / sig fF | area µm² | util % | active % | best | outer, esc | bias | EM known (viol), unk, max use | usage | lattice off | clr residue nm² |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| bgr_core | 5 383 | 2 / 3 | 260 250 | 0 | 0 | 0 | 0 | PARTIAL(9) | 0 | 0 | 77.1 / 19.8 | 135.0 | 100.0 | 1.2 | 2/5 conv. | 1, 0 | none | 0 (0), 1, none | 1.000 | 0 | 0 |
| bjt_mirror | 4 764 | 2 / 5 | 73 480 | 0 | 0 | 0 | 0 | PARTIAL(2) | 0 | 0 | 35.8 / 34.0 | 64.8 | 79.4 | 1.8 | 1/5 conv. | 1, 0 | none | 0 (0), 1, none | 1.260 | 0 | 0 |
| chain4 | 5 523 | 4 / 7 | 44 675 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 22.6 / 8.4 | 62.6 | 100.0 | 19.4 | 1/5 conv. | 1, 0 | 0 µW probe | 5 (0), 0, 0.000 | 1.000 | 1 | 0 |
| dac4 | 14 582 | 14 / 12 | 511 855 | 0 | 1 | 0 | 0 | PARTIAL(16) | 0 | 0 | 293.3 / 130.5 | 2477.3 | 63.8 | 0.6 | 4/15 budget | 3, 2 | 112 µW probe | 11 (0), 0, 0.221 | 1.567 | 6 | 0 |
| ota | 19 256 | 5 / 9 | 497 540 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 620.7 / 280.0 | 1729.8 | 87.7 | 42.1 | 1/20 budget | 4, 3 | 2 µW probe | 6 (0), 0, 0.003 | 1.140 | 2 | 0 |
| ota_constrained | 19 135 | 5 / 9 | 497 540 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 620.7 / 280.0 | 1729.8 | 87.7 | 42.1 | 1/20 budget | 4, 3 | 2 µW probe | 6 (0), 0, 0.003 | 1.140 | 2 | 0 |
| pair | 4 471 | 2 / 3 | 19 905 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 4.8 / 3.1 | 8.6 | 100.0 | 48.4 | 1/5 conv. | 1, 0 | 0 µW probe | 3 (0), 0, 0.000 | 1.000 | 1 | 0 |
| quad | 4 347 | 4 / 3 | 37 945 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 9.3 / 6.1 | 15.9 | 100.0 | 51.2 | 5/5 conv. | 1, 0 | 0 µW probe | 3 (0), 0, 0.000 | 1.000 | 1 | 0 |
| rc_filter | 4 502 | 3 / 5 | 87 230 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 24.5 / 7.8 | 163.3 | 62.0 | 2.8 | 2/5 conv. | 1, 0 | 28 µW probe | 3 (0), 1, 0.055 | 1.613 | 2 | 0 |
| tt_ota | 17 745 | 5 / 9 | 497 540 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 620.7 / 280.0 | 1729.8 | 87.7 | 42.1 | 1/20 budget | 4, 3 | 2 µW probe | 6 (0), 0, 0.003 | 1.140 | 2 | 0 |

Same on every row: overlap 0 nm², matched mismatch 0, dp temps 220, `islands extra`/`decode fail`/`matched incompat`
`n/a`. The skipped-rule lists are M0's.

**Against M0 §4.** DRC, LVS, ERC, unrouted, route hard, overuse, bias and EM are identical on all 10 rows, as are WL,
area and placement on the 9 non-dac4 rows. Every difference comes from the fix-export/GPurify merge (`2a09c30`), since
no M1a code is on the branch:

- Extracted capacitance (C Σ / sig fF): bgr_core 76.7 / 19.6 → 77.1 / 19.8; bjt_mirror 33.0 / 31.3 → 35.8 / 34.0; ota,
  ota_constrained, tt_ota 588.4 / 263.8 → 620.7 / 280.0; rc_filter 24.0 / 7.6 → 24.5 / 7.8; chain4, pair, quad
  unchanged.
- dac4: WL 640 880 → 511 855, area 2400.6 → 2477.3 µm², util 61.1 → 63.8 %, active 0.8 → 0.6 %, best 2/15 → 4/15,
  usage 1.636 → 1.567, lattice off 8 → 6, clr residue 291 300 → 0 nm², C 297.2 / 147.8 → 293.3 / 130.5. `route hard 1`
  is still the in-loop antenna row (max use 2.111 → 2.025).

Constraint satisfaction, verbatim totals:

| type | arm | total | sat | viol | unk | n/a | rate | max use (at) | Θ |
|---|---|---|---|---|---|---|---|---|---|
| Antenna | hard | 21 | 20 | 1 | 0 | 0 | 95 % | 2.025 (dac4) | 1.025 |
| CommonCentroid | budget | 3 | 0 | 3 | 0 | 0 | 0 % | 2.195 (tt_ota) | 3.584 |
| CommonNode | budget | 6 | 6 | 0 | 0 | 0 | 100 % | 0.001 (tt_ota) | 0 |
| CouplingBudget | budget | 39 | 39 | 0 | 0 | 0 | 100 % | 0.201 (dac4) | 0 |
| CrosstalkExclusion | budget | 12 | 12 | 0 | 0 | 0 | 100 % | 0.475 (tt_ota) | 0 |
| Differential | hard | 3 | 3 | 0 | 0 | 0 | 100 % | – | 0 |
| Electromigration | hard | 46 | 43 | 0 | 3 | 0 | 100 % | 0.221 (dac4) | 0 |
| Environment | budget | 6 | 6 | 0 | 0 | 0 | 100 % | 0.320 (tt_ota) | 0 |
| IrDrop | budget | 23 | 23 | 0 | 0 | 0 | 100 % | 0.111 (dac4) | 0 |
| MatchingPair | budget | 6 | 6 | 0 | 0 | 0 | 100 % | 0.000 (tt_ota) | 0 |
| ParasiticBudget | budget | 39 | 39 | 0 | 0 | 0 | 100 % | 0.938 (dac4) | 0 |
| Proximity | budget | 11 | 11 | 0 | 0 | 0 | 100 % | 0.255 (tt_ota) | 0 |
| Symmetry | hard | 9 | 9 | 0 | 0 | 0 | 100 % | – | 0 |
| ThermalGradient | budget | 6 | 6 | 0 | 0 | 0 | 100 % | 0.000 (tt_ota) | 0 |
| Utilization | budget | 10 | 10 | 0 | 0 | 0 | 100 % | 0.968 (rc_filter) | 0 |
| **OVERALL** | | 240 | 233 | 4 | 3 | 0 | 98 % | | |

The counts are identical to M0. The max-use values changed: CouplingBudget 0.523 → 0.201, ParasiticBudget 0.824 →
0.938, IrDrop 0.125 → 0.111, Utilization 0.982 (dac4) → 0.968 (rc_filter), Antenna 2.111 → 2.025.

## 5. Open, for the orchestrator/user

- All 17 M1a items still need to be implemented and merged. The order in §5 M1 step 1–2 stands: FLOW-07 first.
- The environment note in the task ("sky130 models at `~/.volare/sky130A`") does not hold on this machine. Benches and
  ngspice tests need `PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk`, or else the bench silently loses
  bias/EM/IrDrop data (above).
