# M0 report: verify pass 3

This pass checks the exit criteria of milestone M0 ("Honest measurement", 00-MASTER-PLAN.md §5) on branch `m0` at
`4f9082f` (M0: report (verify pass 2)), 89 commits after `ddab38e`. There is no code change since pass 2:
`git diff --stat ba49ae2 4f9082f` touches only this file. Pass 3 re-measures everything from scratch on the same
code, checks pass 2's numbers against fresh runs, and re-counts the RTE-03 gap from the raw dr-report logs.

Every number here was measured on 2026-10-02 in the integration worktree. The profile is release unless stated.
The commands were:

- `cargo build --release --workspace`
- `cargo test --release --workspace --no-fail-fast`
- `cargo run --release -p benchmark --bin bench local`
- three targeted re-runs: release `large_fixtures_sign_off_within_baseline -- --ignored --nocapture`, release
  `antenna_in_loop_never_passes_what_signoff_fails -- --nocapture`, and debug
  `cargo test -p cells --test cell_selfcheck every_generator_enumerates_on_every_deck`

ngspice and the sky130 ngspice models (`~/.volare/sky130A`) were present, so the ngspice tests ran rather than
skipped: the test log has no `SKIP` line (`grep -c SKIP` = 0). magic, netgen and klayout were not on PATH, and no
M0 test calls them.

**Verdict: M0 is not done.** The result is unchanged from pass 2. 12 of the 13 criteria below are met:

- Criterion #1 (CI) is met with a gap. The review panel (§7) found the nightly job could not tell the truth as built;
  that and the gap are fixed on m0 since.
- Criterion #11 (RTE T2) is met as the master plan words it. It is not met under plan-05's stricter wording, and
  RTE-03's own acceptance is not met (§1, §2).
- Criterion #2 is not met. Its item, RTE-06, was not merged.

The suite has two red tests, the same two as in passes 1 and 2:

- the dac4 test (#2);
- `hier_elaborate`, which FLOW-14 made honest and which has stayed red since, with no M0 owner.

Pass 3 corrects one figure from pass 2. Base m0's Θ `routing overuse` reports carry residual overuse of **3 to 73**
nodes, not 3 to 65 (`base_dac4.err` line 29, `overuse=73`). The commit message of `ba49ae2` already says 73. §1 has
the details.

## 1. Items

Merged (24, through review): FLOW-14, GAP-20, FLOW-04, FLOW-05, FLOW-01, CELL-01, CELL-02, CELL-04, EXT-01, PLC-01,
RTE-32, RTE-02, FLOW-02, FLOW-03, PERF-01, PERF-02, PERF-04, PERF-06, PERF-07, PERF-03 (steps 1–2; its port step
waits for FLOW-07, M1), REL-01, REL-03, RTE-09, REL-02.

Not merged through review (2). Their state on m0 differs:

- **RTE-03** (foreign metal hard; shorts, opens, congestion as V). The review **rejected** the m0-routing branch for
  three reasons:
  1. Acceptance 3 is not met, and the deviation text understated the gap ("one non-winning epoch").
  2. The branch did not merge cleanly with m0: `score()` with `cap_total`, `Routes::terms`, and the 3-tuple `route`
     return.
  3. A resolver test case was missing.

  **The code is nevertheless on m0.** `ba49ae2` ports `e8ef0e9`, `b36ca9f` and `93fe6fb` onto m0 outside the item
  review:
  - It resolves the conflicts. It keeps `foreign_metal` and the `cap_total`-free `score()`, keeps `routes.terms`, and
    adjusts the 3-tuple tests.
  - It adds the missing resolver case: against the later net's trunk, the earlier net's jog is the one deleted.
  - It re-measures the gap and states in its commit message that acceptance 3 is not met.

  **Branch state:** `m0-routing` now points at `bb556b4` (FLOW-05: merge), so the three original commits are on no
  branch. `m0-antenna` points at `dac139f` (REL-02: merge), so RTE-06's commits `b635606`, `63bc2a2` and `32efea4`
  are on no branch either. Both are reachable only through the branch reflogs (§6).

  In this pass, all seven RTE-03 dr tests are ok, and the dr unit target is 34/0/0:
  `a_ring_band_is_a_hard_obstacle`, `a_route_touching_foreign_cell_metal_is_a_short`,
  `resolver_sacrifices_the_later_access_shape`, `an_xy_overlap_on_non_adjacent_layers_is_open`,
  `a_trunk_merges_with_its_own_cell_strap`, `ring_pins_are_routed` and `negotiation_persists_across_calls`.
  `signoff_fixtures` passes on the merged tree, including the `--ignored` OTA test, and REL-03's EM check runs on the
  same final geometry. The one exception is dac4 (#2).

  **The gap, re-counted by this pass** from the logs the port and review sessions left. Each log has one line per dr
  report (seed 1, FEEDBACK_ITERS 5) and lives in the session scratchpad:

  | log | code | dr reports | `routing overuse` Θ | `unresolved congestion` V | `drawn short nets` V | `open net` V | `to cell metal` V |
  |---|---|---|---|---|---|---|---|
  | `base_dac4.err` | base m0 `2245425` | 90 | **61** (overuse 3–73) | 0 | 0 | 4 | 0 |
  | `t2_dac4.err` | `ba49ae2` (on m0) | 90 | 0 | **7** | 7 | 16 | 0 |
  | `rte03-h.err` | old branch HEAD | 180 | 0 | 8 | 8 | 23 | 0 |
  | `r2_dac4.err` | old branch, round 2 | 180 | 0 | 8 | 8 | 23 | 0 |

  - **On `ba49ae2`:** the 7 reports are at lines 15, 31, 48, 53, 69, 70 and 85 of 90. Their margins are 4, 4, 3, 4,
    4, 4 and 16, and each is paired with `drawn short nets 3/4`, `2/3`, or `2/3` + `2/4`. The review's lines 17, 55,
    98, 104, 119, 129, 138 and 161 in `rte03-h.err` are confirmed: 8 of 180.
  - **The base column:** the commit message of `ba49ae2` says 62 base reports; the log has 61. Pass 2 said "3 to 65"
    nodes; the log's maximum is 73.
  - **Same call index at base:** 6 of the 7 lines carry Θ `routing overuse`, with overuse 3, 65, 3, 3, 4 and 3. Line
    48 has overuse 0 at base. This aligns reports by call order only: placement feedback can diverge once routing
    differs, so it is indicative, not a paired comparison.
  - **Base also had `open net` V** on 4 reports (lines 12, 27, 29, 52). So `open net` V predates `ba49ae2`.
  - **The other 9 circuits:** the port session reports none of these entries in their 450 reports. This pass did not
    re-instrument, because code changes are not allowed.

  **Open decision (orchestrator/user):** either accept RTE-03 on m0 with acceptance 3 recorded as not met, or revert
  `ba49ae2`. Acceptance 3 fails on 7 of 90 non-winning dac4 dr reports. Residual congestion is a PathFinder
  convergence problem (RTE-08, M1). The review required this decision to be made explicitly. A commit on m0 does not
  make it.
- **RTE-06** (antenna repair on drawn geometry). **Rejected**, not on m0. Its own acceptance is red:
  `fixtures_sign_off_within_baseline` fails on that branch with the same `dac4: ERC rows ["erc/ar.met2.1:gate"]`.
  The stated cause is plausible: `routing_stack` (frontend/library/src/elaborate.rs:211–220) gives dr only
  [met1, met2] on sky130, so a cap top plate on met2 that is over the limit before routing cannot be repaired inside
  dr. No code defect was found. Two undisclosed, untested dr changes were also found:
  - a `frozen` set that keeps shielded nets out of the post-shield antenna pass;
  - a `judge` rollback gate on redraws.

  **The user still has to decide:** merge RTE-06 as "acceptance not met" and move the dac4 fix to FLOW-05/FLOW-08 or
  to the routing stack / cap pin layer, or re-scope the item. The branch's work is no longer on `m0-antenna`
  (see above).

## 2. Exit criteria (00-MASTER-PLAN.md §5, M0)

Every test named below was re-checked `ok` in this pass's release log unless stated otherwise.

| # | Criterion | Status | Evidence |
|---|---|---|---|
| 1 | CI runs `cargo test --workspace` on every push; release generator tests; nightly tool tests fail loudly when a tool is missing (FLOW T13) | **met, with a gap** | `.github/workflows/ci.yml`, three gating jobs:<br>• `test` (on push): `cargo build --workspace --locked`, then `cargo test --workspace --locked --no-fail-fast`.<br>• `release`: `cell_selfcheck` and `ota_cross_pdk` in release.<br>• `nightly`: `PHILIS_REQUIRE_TOOLS=1`, `--include-ignored`, ngspice, and sky130 at `fa87f8f4…` via volare. **Review panel (§7, findings 3/15):** workspace-wide `--include-ignored` also ran the EXT placeholders (`coverage_is_total` is `unimplemented!`, `twelve_thousand_devices` > 13 min), so this job was always red or hung and a missing-tool panic was indistinguishable; criterion 1 was therefore **not met** as measured here. Fixed: the job now runs the non-ignored workspace plus the ignored signoff target by name, with a 90-min timeout.<br>`tool_or_skip_tells_present_from_missing` ok. CI was not observed running, because nothing is pushed.<br>**Gap (fixed by the review panel, §7 findings 4/16):** the `library` unit test `fixture_nets_without_a_resistor_keep_em_sizing` (frontend/library/src/lib.rs:1706) skips with `eprintln!("SKIP …")` and returns, and it ignores `PHILIS_REQUIRE_TOOLS`. Only perf_postlayout.rs reads that variable. A nightly run without ngspice or the models would pass this test instead of failing loudly.<br>The suite is also not green (§3: two red tests), while plan-08's M0 row expects only the dac4 test to be red. |
| 2 | `signoff_fixtures` green including dac4 at `feedback_iters = 1` (REL T1, RTE T11) | **not met** (RTE-06 not merged) | `fixtures_sign_off_within_baseline` FAILED at benchmarks/tests/signoff_fixtures.rs:173: `dac4: ERC rows ["erc/ar.met2.1:gate"], expected []`. dac4's row: DRC 0, ERC 1, PEX 282.0 fF, LVS clean with 16 unverified. The other 6 default fixtures have DRC 0, ERC 0 and clean LVS (§3). `large_fixtures_sign_off_within_baseline` (`--ignored`) ok: ota, ota_constrained and tt_ota each have DRC 0, ERC 0, PEX 585.8 fF, clean LVS and 0 unverified. |
| 3 | `antenna_in_loop_never_passes_what_signoff_fails` green (REL T2) | **met** | ok. With `--nocapture`:<br>• dac4: signoff `["erc/ar.met2.1:gate"]`, in-loop `(5 total, 4 satisfied, 0 unknown)`, so the in-loop model flags the violation.<br>• pair, quad, rc_filter, chain4: in-loop `(1, 1, 0)`, no signoff rows.<br>• bjt_mirror, bgr_core: no rows on either side. |
| 4 | Bench LVS reads `PARTIAL(2)` bjt_mirror, `PARTIAL(9)` bgr_core, `PARTIAL(16)` dac4, `MATCH` for the other 7 (PERF M0) | **met** | Bench table §4, LVS column, exactly as required. |
| 5 | Warnings printed and absent from \|V\| | **met** | The bench prints `warnings N` per circuit (0 on all 10). `start_tests::warnings_are_not_violations`, `v_counts_rules_not_batches`, `theta_counts_each_budget_once` and `nan_loses` ok. |
| 6 | FLOW T1 size fidelity: 0 devices | **met** | `size_tests::drawn_width_equals_simulated_width` ok, over all MOS on all 10 fixtures. `netlist::tests::mos_size_follows_spice_semantics` and `parse::size_tests::per_finger_convention_stores_total_width` ok. |
| 7 | FLOW T3 deck honesty 0/0/0 | **met** | Not embedded: `pdk::tests::every_builtin_pdk_loads` and `vendored_decks_name_their_origin` ok. Without `_source`: `a_process_number_needs_a_source` ok. Required keys unread: `generators_read_exactly_the_required_keys` ok (kernel/cells/tests/deck_keys.rs). `sidecar_rejects_a_misspelt_key` ok. |
| 8 | FLOW T4: LU rules present, no antenna sidewall off by > 5 % | **met** | `pdk::tests::latch_up_rules_find_a_far_tap` and `antenna_sidewall_thickness_matches_pex` ok. pdks/decks/sky130.deck:517–519 has LU.2, LU.2.1 and LU.3. In the bench they run, or are listed as skipped (`EmptyLayer`) where the fixture lacks the diffusion. |
| 9 | Exactly one dual step per epoch (FLOW T6) | **met** | `start_tests::one_dual_step_per_epoch` ok. gp `price_tests::a_slack_budget_relaxes_its_price` and `a_saturated_price_is_reported` ok. dp `tests::place_does_not_settle` ok. |
| 10 | EM known/unknown/violated counts printed per fixture (REL T3/T4) | **met** | Bench `EM known K (viol V), unknown U, max use X` on every row (§4). Violations are 0 everywhere. Unknown is 1 on bgr_core, bjt_mirror and rc_filter. Totals: Electromigration 46 rows, 43 satisfied, 3 unknown. |
| 11 | No `drawn short` or `unresolved congestion` hidden in Θ (RTE T2) | **met as worded in the master plan; not met under plan-05's T2 / RTE-03 acceptance** | **Θ:** `grep -rn 'routing overuse' --include='*.rs'` over backend, frontend, kernel and benchmarks is empty (exit 1).<br>**V:** `dr::score` (backend/dr/src/lib.rs:1906–1942) puts residual overuse in V as `unresolved congestion` (margin = overuse). It puts shorts in V as `drawn short nets a/b` and `drawn short net n to cell metal`, and opens in V as `open net n`.<br>**Tests:** `a_route_touching_foreign_cell_metal_is_a_short`, `a_ring_band_is_a_hard_obstacle`, `an_xy_overlap_on_non_adjacent_layers_is_open` and `resolver_sacrifices_the_later_access_shape` ok.<br>**Bench winners:** overuse 0 and route hard 0 on all 10, except dac4's in-loop `route/Antenna: net top 1.1108099` (target/bench_debug/dac4/violations.txt:2).<br>**But** plan-05's M0 row (T2) reads "no `drawn short`/`unresolved congestion` V … on any bench circuit". 7 of dac4's 90 dr reports (none of them the winner) carry exactly those (§1). So that stricter target and RTE-03 Acceptance 3 are not met. RTE-03 itself awaits the decision in §1. |
| 12 | PLC baseline table and EXT corpus expectations recorded | **met** | docs/plans/baseline-plc-01.md: 10 fixtures × seeds 1–5. annotator `corpus_expectations` and `negative_corpus` ok. The corpus tests for later items are `#[ignore]` and name their item (§3). |
| 13 | `every_generator_enumerates_on_every_deck` green in debug on 4 decks; 0 inductor shapes in any output | **met** | The debug run (`unoptimized + debuginfo`) is ok. The test is not `cfg`-gated and loops sky130, gf180mcu, ihp_sg13g2 and generic_finfet (kernel/cells/tests/cell_selfcheck.rs:421). `Inductor::draw` returns an empty macro (kernel/cells/src/inductor.rs). `inductor::tests::an_inductor_is_undrawable_without_a_recogniser`, `an_inductor_is_one_undrawable_finding` and `verify` `tests::an_inductor_is_unverified` ok. No bench fixture has an `L` card. |

## 3. Test summary

`cargo build --release --workspace`: exit 0.

`cargo test --release --workspace --no-fail-fast`: exit 101. **435 passed, 2 failed, 7 ignored**, summed over every
`test result` line. This equals pass 2. Doc-tests: 0.

Failing tests (the same two as passes 1 and 2, with the same messages):

1. **`benchmark` `tests/signoff_fixtures.rs::fixtures_sign_off_within_baseline`** panicked at
   signoff_fixtures.rs:173:9: `dac4: ERC rows ["erc/ar.met2.1:gate"], expected []` (criterion 2; RTE-06).
2. **`library` `tests/hier_elaborate.rs::hierarchical_composition_elaborates_with_a_correct_schematic`** panicked at
   hier_elaborate.rs:216:5: `hierarchical layout must be LVS-clean against its schematic`, with 8 ×
   `lvs/lvs.unpaired_device:-` and 17 × `lvs/lvs.unpaired_net:-`. This is not a regression: FLOW-14 (`0ef8506`)
   replaced an assertion-free check, and recorded the test as red with these counts. No M0 item fixes it, so it needs
   an owner.

Ignored (7), each with its reason in the attribute:

- `strongarm_matches_align_gold`: EXT-14.
- `coverage_is_total`: EXT-10.
- `negative_corpus_sc_switches_and_equal_fets`: EXT-04.
- `no_emitted_conflicts_strongarm`: EXT-14, REL-09.
- `permutation_invariance`: EXT-06.
- `twelve_thousand_devices`: EXT-06, T8.
- `large_fixtures_sign_off_within_baseline`: run separately in this pass, ok (criterion 2).

Per target (passed/failed/ignored):

| Crate | Target | Passed / failed / ignored |
|---|---|---|
| analog | unit tests | 85/0/0 |
| annotator | unit tests | 33/0/0 |
| annotator | `align_gold` | 0/0/1 |
| annotator | `corpus` | 3/0/4 |
| annotator | `scale` | 0/0/1 |
| benchmark | `bench` unit tests | 13/0/0 |
| benchmark | `signoff_fixtures` | **16/1/1** |
| cells | unit tests | 39/0/0 |
| cells | `cell_selfcheck` | 10/0/0 |
| cells | `deck_keys` | 1/0/0 |
| dp | unit tests | 28/0/0 |
| dr | unit tests | 34/0/0 |
| gp | unit tests | 6/0/0 |
| gr | unit tests | 11/0/0 |
| library | unit tests | 73/0/0 |
| library | `antenna_diode` | 1/0/0 |
| library | `drawn_cards` | 2/0/0 |
| library | `elaborate` | 1/0/0 |
| library | `emit_roundtrip` | 2/0/0 |
| library | `extra_devices` | 1/0/0 |
| library | `flow_smoke` | 1/0/0 |
| library | `hier_elaborate` | **0/1/0** |
| library | `injected_macro` | 1/0/0 |
| library | `ota_cross_pdk` | 3/0/0 |
| library | `perf_postlayout` | 3/0/0 (ngspice ran, 48 s) |
| library | `placement_metrics` | 2/0/0 |
| macro_master | unit tests | 9/0/0 |
| pnr_core | unit tests | 16/0/0 |
| verify | unit tests | 39/0/0 |
| visualizer | unit tests | 2/0/0 |

`signoff_fixtures` per fixture:

| fixture | DRC (routing / cells) | ERC | PEX Σ C | LVS |
|---|---|---|---|---|
| pair | 0 (0/0) | 0 | 4.8 fF | clean, 0 unverified |
| quad | 0 (0/0) | 0 | 10.6 fF | clean, 0 unverified |
| rc_filter | 0 (0/0) | 0 | 27.8 fF | clean, 0 unverified |
| bjt_mirror | 0 (0/0) | 0 | 33.7 fF | clean, 2 unverified |
| bgr_core | 0 (0/0) | 0 | 81.6 fF | clean, 9 unverified |
| chain4 | 0 (0/0) | 0 | 24.1 fF | clean, 0 unverified |
| dac4 | 0 (0/0) | **1** (`ar.met2.1`) | 282.0 fF | clean, 16 unverified |
| ota / ota_constrained / tt_ota (`--ignored`) | 0 (0/0) | 0 | 585.8 fF | clean, 0 unverified |

## 4. Bench (`bench local`, seed 1, sky130, FEEDBACK_ITERS 5)

The run exited 0. 10/10 circuits were placed and routed, and the per-circuit times sum to 143.0 s. The circuits run
in sequence, so that sum is about the wall time; it was not timed separately. Load average was 14.4 just before the
bench and 9.9 at its end, on 32 cores, with other sessions running. That explains why every ms figure is higher than
in pass 2.

The only stderr was `[op] operating point unavailable (no device operating points: ); continuing with zero power`,
printed for the two BJT fixtures, which have no operating point.

| circuit | ms | cells / nets | WL nm | unrouted | route hard | overuse | DRC | LVS | ERC | warn | C Σ / sig fF | area µm² | util % | active % | best | outer, esc | bias | EM known (viol), unk, max use | usage | lattice off | clr residue nm² |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| bgr_core | 9 034 | 2 / 3 | 260 250 | 0 | 0 | 0 | 0 | PARTIAL(9) | 0 | 0 | 76.7 / 19.6 | 135.0 | 100.0 | 1.2 | 2/5 conv. | 1, 0 | none | 0 (0), 1, none | 1.000 | 0 | 0 |
| bjt_mirror | 7 487 | 2 / 5 | 73 480 | 0 | 0 | 0 | 0 | PARTIAL(2) | 0 | 0 | 33.0 / 31.3 | 64.8 | 79.4 | 1.8 | 1/5 conv. | 1, 0 | none | 0 (0), 1, none | 1.260 | 0 | 0 |
| chain4 | 6 639 | 4 / 7 | 44 675 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 22.6 / 8.4 | 62.6 | 100.0 | 19.4 | 1/5 conv. | 1, 0 | 0 µW probe | 5 (0), 0, 0.000 | 1.000 | 1 | 0 |
| dac4 | 20 630 | 14 / 12 | 640 880 | 0 | 1 | 0 | 0 | PARTIAL(16) | 0 | 0 | 297.2 / 147.8 | 2400.6 | 61.1 | 0.8 | 2/15 budget | 3, 2 | 112 µW probe | 11 (0), 0, 0.221 | 1.636 | 8 | 291 300 |
| ota | 27 962 | 5 / 9 | 497 540 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 588.4 / 263.8 | 1729.8 | 87.7 | 42.1 | 1/20 budget | 4, 3 | 2 µW probe | 6 (0), 0, 0.003 | 1.140 | 2 | 0 |
| ota_constrained | 27 406 | 5 / 9 | 497 540 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 588.4 / 263.8 | 1729.8 | 87.7 | 42.1 | 1/20 budget | 4, 3 | 2 µW probe | 6 (0), 0, 0.003 | 1.140 | 2 | 0 |
| pair | 5 858 | 2 / 3 | 19 905 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 4.8 / 3.1 | 8.6 | 100.0 | 48.4 | 1/5 conv. | 1, 0 | 0 µW probe | 3 (0), 0, 0.000 | 1.000 | 1 | 0 |
| quad | 5 754 | 4 / 3 | 37 945 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 9.3 / 6.1 | 15.9 | 100.0 | 51.2 | 5/5 conv. | 1, 0 | 0 µW probe | 3 (0), 0, 0.000 | 1.000 | 1 | 0 |
| rc_filter | 6 714 | 3 / 5 | 87 230 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 24.0 / 7.6 | 163.3 | 62.0 | 2.8 | 2/5 conv. | 1, 0 | 28 µW probe | 3 (0), 1, 0.055 | 1.613 | 2 | 0 |
| tt_ota | 25 545 | 5 / 9 | 497 540 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 588.4 / 263.8 | 1729.8 | 87.7 | 42.1 | 1/20 budget | 4, 3 | 2 µW probe | 6 (0), 0, 0.003 | 1.140 | 2 | 0 |

These columns are the same on every row: overlap 0 nm², matched mismatch 0, dp temps 220. The bench also printed
islands extra, decode fail and matched incompat as 0, but nothing measures them before PLC-12, PLC-08 and PLC-03:
they were placeholders, not measured zeros. Since the review panel they print `n/a` (§7, finding 17). The `key tier` column equals `C sig` on every row.

**Reproducibility:** every column except ms is identical to pass 2, on all 10 rows and in the constraint table. The
code is unchanged, the epoch count is fixed (`budget` = 15 or 20 epochs, not a time budget), and the seed is 1, so
the run is deterministic. dac4 still matches the port session's run (`bench_t2.out`: WL 640 880, C 297.2 / 147.8).

**dac4's `route hard 1`** is the in-loop antenna rule on net `top` (`route/Antenna: net top 1.1108099`, max use
2.111). Signoff passes that net at FEEDBACK_ITERS 5 (ERC 0), but fails it at 1 (criterion 2). The winner therefore
has |V| ≥ 1.

**dac4 since pass 1** (from pass 2, unchanged here): the winner is a different layout since `ba49ae2`. WL went from
691 870 to 640 880 nm, C Σ from 310.4 to 297.2 fF, area from 2635.2 to 2400.6 µm², and lattice off from 6 to 8.
Clearance residue went from 0 to 291 300 nm². That metric (`PlacementMetrics::clearance_residue_nm2`,
frontend/library/src/geometry.rs:29) is measured but never steered on, and PLC-01's baseline already shows it nonzero
on dac4 seed 5.

**Skipped rules** (from `signoff.coverage.skipped_rules`, as printed):

- **Every fixture:** `lvs.device_count_mos`, `lvs.device_count_bjt` and `lvs.parametric` (`NotInDeck`);
  `multiple_drivers` (`Waived`); `m1.density`…`m4.density` (`ChipLevel`); `floating_gate` (exempt: no port list).
- **Per fixture:** the LU rules whose layer is empty (`EmptyLayer`).
- **bgr_core and bjt_mirror only:** `ir_drop`, `esd.pad`, the `vgs/vds` ratings and `EM.met*_via*`
  (`NoDesignIntent`, unbiased).

Constraint satisfaction, verbatim totals:

| type | arm | total | sat | viol | unk | max use (at) | Θ |
|---|---|---|---|---|---|---|---|
| Antenna | hard | 21 | 20 | 1 | 0 | 2.111 (dac4) | 1.111 |
| CommonCentroid | budget | 3 | 0 | 3 | 0 | 2.195 (tt_ota) | 3.584 |
| CommonNode | budget | 6 | 6 | 0 | 0 | 0.001 (tt_ota) | 0 |
| CouplingBudget | budget | 39 | 39 | 0 | 0 | 0.523 (dac4) | 0 |
| CrosstalkExclusion | budget | 12 | 12 | 0 | 0 | 0.475 (tt_ota) | 0 |
| Differential | hard | 3 | 3 | 0 | 0 | – | 0 |
| Electromigration | hard | 46 | 43 | 0 | 3 | 0.221 (dac4) | 0 |
| Environment | budget | 6 | 6 | 0 | 0 | 0.320 (tt_ota) | 0 |
| IrDrop | budget | 23 | 23 | 0 | 0 | 0.125 (dac4) | 0 |
| MatchingPair | budget | 6 | 6 | 0 | 0 | 0.000 (tt_ota) | 0 |
| ParasiticBudget | budget | 39 | 39 | 0 | 0 | 0.824 (dac4) | 0 |
| Proximity | budget | 11 | 11 | 0 | 0 | 0.255 (tt_ota) | 0 |
| Symmetry | hard | 9 | 9 | 0 | 0 | – | 0 |
| ThermalGradient | budget | 6 | 6 | 0 | 0 | 0.000 (tt_ota) | 0 |
| Utilization | budget | 10 | 10 | 0 | 0 | 0.982 (dac4) | 0 |
| **OVERALL** | | 240 | 233 | 4 | 3 | | rate 98 % |

## 5. Against the pre-M0 ground truth (audit-06 §G)

**Tests:**

| | pre-M0 (G.2) | now |
|---|---|---|
| Result | 331 passed, 1 failed, 2 ignored, exit 101 | 435 passed, 2 failed, 7 ignored, exit 101 |
| Failing | dac4 `erc/ar.met2.1:gate`, the same rule as before | dac4 (same rule) and `hier_elaborate` |
| Ignored | 2: `large_fixtures…`, `tmp_deck_drc::dump` | 7 (EXT corpus placeholders and `large_fixtures…`; `tmp_deck_drc.rs` was deleted by FLOW-14) |
| dr unit tests | 25 | 34 |

`hier_elaborate` was "passing" before only because it asserted nothing (FLOW-14). The OTA-class fixtures were never
signed off by a default run before. They are still `#[ignore]` in the default run, but were run here and are clean.

**`signoff_fixtures` per fixture:**

- DRC is 0 everywhere, before and now.
- ERC before: bjt_mirror 1, bgr_core 1, dac4 1. Now only dac4 has 1; the `supply_short` false positive is gone.
- PEX is unchanged on six fixtures. dac4 went from 326.0 to 282.0 fF.
- LVS: bjt_mirror and bgr_core no longer read "clean" with nothing compared; they report 2 and 9 unverified. dac4's
  15 silently dropped caps are now 16 unverified (Σm).

**Bench, G.3 → now:**

| circuit | ms | WL nm | overuse | LVS | ERC | C Σ fF | area µm² | util % | best |
|---|---|---|---|---|---|---|---|---|---|
| bgr_core | 16 113 → 9 034 | 260 320 → 260 250 | 0 → 0 | MATCH¹ → PARTIAL(9) | 1 → 0 | 77.2 → 76.7 | 135.0 → 135.0 | 100.0 → 100.0 | 4/5 budget → 2/5 conv. |
| bjt_mirror | 12 300 → 7 487 | 74 085 → 73 480 | 0 → 0 | MATCH¹ → PARTIAL(2) | 1 → 0 | 33.7 → 33.0 | 64.7 → 64.8 | 79.4 → 79.4 | 3/5 budget → 1/5 conv. |
| chain4 | 11 647 → 6 639 | 44 675 → 44 675 | 0 → 0 | MATCH → MATCH | 0 → 0 | 22.6 → 22.6 | 62.6 → 62.6 | 100.0 → 100.0 | 1/5 → 1/5 |
| dac4 | 287 520 → 20 630 | 580 410 → 640 880 | **18 → 0** | MATCH⁵ → PARTIAL(16) | 0 → 0 | 275.8 → 297.2 | 2326.9 → 2400.6 | 60.0 → 61.1 | 5/15 → 2/15 budget |
| ota (×3, identical) | 52–59 k → 26–28 k | 444 720 → 497 540 | 0 → 0 | MATCH → MATCH | 0 → 0 | 616.0 → 588.4 | 1865.1 → 1729.8 | 85.9 → 87.7 | 1/20 conv. → 1/20 budget |
| pair | 11 468 → 5 858 | 17 700 → 19 905 | 0 → 0 | MATCH → MATCH | 0 → 0 | 4.6 → 4.8 | 8.6 → 8.6 | 100.0 → 100.0 | 4/5 → 1/5 |
| quad | 16 803 → 5 754 | 37 945 → 37 945 | 0 → 0 | MATCH → MATCH | 0 → 0 | 9.3 → 9.3 | 15.9 → 15.9 | 100.0 → 100.0 | 5/5 → 5/5 |
| rc_filter | 13 445 → 6 714 | 86 835 → 87 230 | 0 → 0 | MATCH → MATCH | 0 → 0 | 24.4 → 24.0 | 131.7 → 163.3 | 76.9 → 62.0 | 5/5 → 2/5 |

- **Wall time:** 539.5 s before, about 143 s now (99.8 s in pass 2). All three runs were under unknown or uneven
  load, and no item claims a speed-up. Per baseline-plc-01.md, the timings are for orientation only.
- **dac4 cell count:** 15 before, 14 now. The 15th was a flow-inserted antenna diode.
- **The changes the table was meant to show:**
  1. Every "MATCH" that compared nothing now reads PARTIAL(n) (PERF-02).
  2. The ERC `supply_short` false positives are gone.
  3. dac4's winner overuse is 0, and since `ba49ae2` residual overuse can no longer sit in Θ at all: it is V.
- **Moves explained in the merge commits:**
  - **Epoch selection** changed the winner and C on several fixtures. FLOW-02 (`lex_key`), FLOW-03 and PERF-07
    (signal-C tier) change which epoch wins; for dac4, so does RTE-03's dr (§4).
  - **Device sizes:** FLOW-01. It moved the OTA WL, area and C, and the OTA CommonCentroid rows.

**Constraint table:**

- Totals: 241/237/2 viol/2 unk before; 240/233/4 viol/3 unk now. IrDrop rows went from 24 to 23.
- **CommonCentroid** went from 3/3 satisfied (max use 0.000) to 0/3 (max use 2.195, Θ 3.584).
  - The FLOW-01 merge records this as the honest result of drawing the simulated size: XM1/XM2 (W=10u nf=2) fold to
    two 5 µm fingers each, placed AABB.
  - `cellgen::folds` ignores CommonCentroid. That is follow-up work, not done in M0.
- **Antenna** went from 21/21 to 20/21, with dac4 `top` at 2.111.
- **ParasiticBudget** went from 38/39 (dac4 1.016) to 39/39 (0.824). **Utilization** went from 9/10 to 10/10.
- **Electromigration** unknown went from 2 to 3 (the new one is rc_filter). It was not traced to an item; REL-01
  ("unknown is not zero") and REL-03 both touch these rows.
- **Skip log:**
  - The `m*.density` rules were "run" over zero windows before (AV-18). They are now listed `ChipLevel` (PERF-04).
  - Whether EM ran is now printed per fixture (AV-26 closed).
  - The LU rules now exist and run (FLOW-05).

## 6. Open, for the orchestrator/user

1. **RTE-03:** accept `ba49ae2` on m0 with Acceptance 3 recorded as not met (7 of 90 non-winning dac4 dr reports
   carry `unresolved congestion` + `drawn short nets` V; convergence is RTE-08, M1), or revert it. The review asked for
   this decision explicitly. Criterion 11 holds under the master plan's wording only while `ba49ae2` stays.
2. **RTE-06:** merge as "acceptance not met" or re-scope (§1). Criterion 2 stays red until dac4's met2 antenna is
   fixed.
3. **`hier_elaborate`** (8 unpaired devices, 17 unpaired nets) is red with no owner in M0.
4. **`fixture_nets_without_a_resistor_keep_em_sizing`** should use the `PHILIS_REQUIRE_TOOLS` skip that
   perf_postlayout uses, so the nightly job cannot pass it when a tool is missing. No code was changed in this pass.
6. **Lost branch work:** `m0-routing` and `m0-antenna` were reset to merge commits. RTE-03's original commits
   (`e8ef0e9`, `b36ca9f`, `93fe6fb`, ported in `ba49ae2`) and RTE-06's (`b635606`, `63bc2a2`, `32efea4`) are on no
   branch. They are reachable only through the reflogs (`m0-routing@{1}`, `m0-antenna@{1}`) until those expire. If
   RTE-06 is to be merged as "acceptance not met", `32efea4` must be re-pointed by a branch first.
5. **The OTA CommonCentroid rows** are 0/3 since FLOW-01, pending `cellgen::folds` honouring CommonCentroid.

## 7. Review panel

The M0 review panel raised 26 findings. Each was checked against the code on `m0` at `0c40ce5`, and the real ones were
fixed in "M0: review-panel fixes". After the fixes, `cargo test --release --workspace --no-fail-fast` gives **439
passed, 2 failed, 7 ignored**. The four new passes are new tests. The two failures are the same two as before: dac4
(`erc/ar.met2.1:gate`) and `hier_elaborate`. With ngspice and the models present, the log has no `SKIP` line. A
mutation check confirms the new undrawn-device assertion fails when the epoch rows are removed. With
`PHILIS_REQUIRE_TOOLS=1` and `PDK_ROOT=/nonexistent`, `fixture_nets_without_a_resistor_keep_em_sizing` panics as
intended. Debug `antenna_diode`, `extra_devices` and `flow_smoke` pass. The bench was not re-run; the printed bench
columns that changed are listed under 6 and 17.

| # | Finding | Outcome |
|---|---|---|
| 1 | RTE-06 not on m0; dac4 exit criterion unmet | **Deferred (owner decision).** The finding is real. The commits are preserved on the new branch `m0-rte06-rejected` (`32efea4`). Status recorded in plan-05 RTE-06. Owner choice: merge with "acceptance not met" plus a named owner for dac4's met2 cap-plate antenna, or re-scope §5 criterion 2. Baseline not relaxed. |
| 2 | RTE-03 on m0 without review; Acceptance 3 unmet | **Deferred (owner decision).** The finding is real. The originals are preserved on `m0-rte03-original` (`93fe6fb`). plan-05 RTE-03 now records "not merged through review" and "acceptance 3 not met (7/90 dac4 dr reports)". Owner choice: revert `ba49ae2`, or keep it and hand the residual congestion to RTE-08 (M1). |
| 3 | Nightly `--include-ignored` runs EXT placeholders | **Fixed.** The nightly job runs the non-ignored workspace plus `-p benchmark --test signoff_fixtures -- --ignored`. The choice is recorded in plan-08 FLOW-14. |
| 4 | EM-sizing test ignores `PHILIS_REQUIRE_TOOLS` | **Fixed.** `library::tools::{present_or_skip, tool_or_skip, sky130_models}` (`#[doc(hidden)]`) are shared by perf_postlayout.rs and the unit test. |
| 5 | `lex_key` drops `lvs-coverage/` rows silently | **Fixed (option b, amendment).** Recorded in plan-07 PERF-02 step 3 and master §6.1. The `RunStats::converged` doc and `lex_key` comment now say that converged does not imply LVS-complete; `certified()` does. |
| 6 | `satisfied` counted unknowns | **Fixed.** `BudgetStatus` gains `violations`. `satisfied = total − violations − unknown` (saturating, because `CentroidGroup` can count a rule as both violated and unknown). `met()` means no violations. `hard_violated()` sums `violations`, so V is unchanged. The verdict shows `VIOLATED + UNKNOWN`. The report table prints viol/unk columns. The bench's EM `viol` reads `violations`. New test: `an_unknown_is_not_satisfied`. |
| 7 | `touch` closure duplicates `Rect::touches` | **Fixed.** |
| 8 | Via grouping deviates from REL-03 step 4.3 | **Fixed (amendment).** The transitive grouping and its known false-pass case are recorded in plan-06 REL-03 step 4.3. Code unchanged: pair-wise groups would split dr's arrays into single cuts that falsely fail. |
| 9 | CELL-01 keep-outs never filled | **Fixed (record).** plan-03 CELL-01 now states that keep-outs are plumbing only until CELL-06/RTE-15, and that `Drawn` is resistor-only. AC-17 stays open. The commit message cannot be changed (no history rewrite). |
| 10 | Dead `cell.antenna_sidewall` fallback; stale gf180 text | **Fixed.** The fallback and the `antenna_sidewall` registry row are deleted, so a reintroduced table now fails validation. `antenna_source`'s reader text is corrected. gf180's sentence now says the checker runs its sidewall rules. |
| 11 | `em_rules` drops entries past 16 silently | **Fixed.** `em.len() > MAX_LAYERS` pushes its own missing row. The unchecked-layer check reads the full `em` list. New test: `em_rules_name_what_goes_unchecked`. |
| 12 | Winner debug check filters `diode_marker`, not the credit layer | **Fixed.** It filters on `antenna_diode_credit()`'s layer, the one the epoch draws on. |
| 13 | `unverified()` lists unread keys as assumed | **Fixed.** Keys whose registry `reader` is `unread` are excluded. The test now expects `gate_cap_af_um2` in the list and `n_well_depth` out of it. |
| 14 | Garbled EM missing-input text that names no layer | **Fixed.** The text now reads "deck EM limit on every routed and pin-access layer (li unchecked)" with the layer names, leaked once per run. |
| 15 | Nightly always red or hanging; criterion #1 overstated | **Fixed.** Same fix as 3, plus `timeout-minutes: 90`. §2 criterion 1 records that the job, as measured before this fix, did not meet T13. |
| 16 | Same as 4 | **Fixed** (see 4). |
| 17 | Unmeasured counters print as 0 | **Fixed.** `PlacementMetrics::islands_extra`, `PlaceStats::decode_fail` and `matched_incompatible` are now `Option` (`None` until PLC-12/08/03), and the bench prints `n/a`. The §4 sentence is corrected. |
| 18 | LVS says params are compared, but only MOS are | **Fixed.** The coverage note says MOS W/L only. `signoff_checked` adds a skipped row `lvs.parameter_mismatch(non-MOS values)` with the count of compared R/D/C/BJT cards, so the bench's `skipped [...]` names it. The CRATES.md note is corrected. New test: `a_compared_resistor_value_is_listed_as_not_compared`. Bench MATCH/PARTIAL semantics are unchanged (criterion 4 stays as worded). |
| 19 | Sidecar load failures skip cellgen/extra_devices tests | **Fixed.** These tests use `Pdk::builtin("sky130").expect(..)`, and so does `multi_start_is_deterministic`, which had the same skip. |
| 20 | REL T2 test is per fixture, not per net; counted unknowns twice | **Partly fixed, partly deferred to M1.** The assertion is now `b.violations > 0`, with no double count. The per-net check is deferred: a signoff ERC row carries no net (`verify::Finding` is a point on the `gate` layer), so per-net attribution needs net-tagged ERC findings. The limitation is stated in the test's doc. |
| 21 | An undrawable device never enters the epoch key | **Fixed.** `undrawable()` rows are shared by `signoff` and the epoch, so they count in the epoch's \|V\| (a constant row). A run with an undrawn device never reads feasible or converged. The test asserts `!converged`; it fails without the fix (mutation-checked). |
| 22 | `drawn_cards` drops a card with a missing pin | **Fixed.** A missing pin becomes the non-port net `~{cell}.{owner}.no-{t}`, and the card stays. The unit test covers it. |
| 23 | DRC warnings counted inconsistently | **Fixed.** `verify::Finding` gains `warning`. `signoff_drc` and the fixture DRC count leave warnings out; the bench's `drc_located.txt` tags them. |
| 24 | `shortfall` reports 0 for a violated maximum | **Fixed.** A length margin is `|limit − measured|`; every row is a violation, so this is the shortfall or the overshoot. The test is updated (100 vs 170 now reads 70). |
| 25 | strongarm skipped by name in the conflict test | **Fixed.** `strongarm_conflicts_are_todays` pins today's set exactly: `(mn1, mp8)` and `(mn2, mp8)`. |
| 26 | Vacuous overlap assertion | **Fixed.** It now asserts `overlap_nm2 == gp::mechanics::encroachment(&sol.layout, 0)`. |

No finding was rejected outright. Two (1, 2) wait on the owner decisions recorded in §6, and part of 20 is deferred to
M1.
