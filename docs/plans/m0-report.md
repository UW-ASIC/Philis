# M0 report: verify pass 4

This pass checks the exit criteria of milestone M0 ("Honest measurement", 00-MASTER-PLAN.md §5) on branch `m0` at
`44b37f1` (M0: review-panel fixes), 91 commits after `ddab38e`. Unlike pass 3, the code changed since the last pass:
`44b37f1` touches 26 files (verify, library, dp, em.rs, bench, signoff_fixtures, CI, gf180 sidecar), with the
per-finding outcome in §7. The review panel ran the suite but not the bench. Pass 4 re-measures everything on the new
code, and also re-counts the RTE-03 gap on this code with a throw-away instrumented copy (below).

Every number here was measured on 2026-10-02 in the integration worktree. The profile is release unless stated.
The commands were:

- `cargo build --release --workspace`
- `cargo test --release --workspace --no-fail-fast`
- `cargo run --release -p benchmark --bin bench local`
- targeted re-runs: release `large_fixtures_sign_off_within_baseline -- --ignored --nocapture`, release
  `antenna_in_loop_never_passes_what_signoff_fails -- --nocapture`, debug
  `cargo test -p cells --test cell_selfcheck every_generator_enumerates_on_every_deck`, and the loud-skip checks of
  criterion 1 (`PHILIS_REQUIRE_TOOLS=1 PDK_ROOT=/nonexistent`).
- RTE T2 count: `git archive 44b37f1` into the session scratchpad, one `eprintln!` of every hard row at the end of
  `dr::score` (backend/dr/src/lib.rs:1943), then `bench local` (all 10) and `bench local dac4`. Nothing of this is in
  the repository.

ngspice and the sky130 ngspice models (`~/.volare/sky130A`) were present, so the ngspice tests ran rather than
skipped: the test log has no `skipping`/`unavailable` line (`grep -ciE` = 0). magic, netgen and klayout were not on
PATH, and no M0 test calls them.

**Verdict: M0 is not done.** 12 of the 13 criteria below are met; the result is unchanged from pass 3 except that
criterion #1's gap is closed:

- Criterion #2 is not met. Its item, RTE-06, was not merged (owner decision pending, §6).
- Criterion #11 (RTE T2) is met as the master plan words it. It is not met under plan-05's stricter wording, and
  RTE-03's own acceptance 3 is not met (§1, §2). RTE-03's code is on m0 (`ba49ae2`) without an item review.

The suite has two red tests, the same two as in passes 1–3: the dac4 test (#2) and `hier_elaborate`, which FLOW-14
made honest and which has no M0 owner.

## 1. Items

Merged (24, through review): FLOW-14, GAP-20, FLOW-04, FLOW-05, FLOW-01, CELL-01, CELL-02, CELL-04, EXT-01, PLC-01,
RTE-32, RTE-02, FLOW-02, FLOW-03, PERF-01, PERF-02, PERF-04, PERF-06, PERF-07, PERF-03 (steps 1–2; its port step
waits for FLOW-07, M1), REL-01, REL-03, RTE-09, REL-02.

Not merged through review (2):

- **RTE-03** (foreign metal hard; shorts, opens, congestion as V). The review **rejected** the m0-routing branch:
  1. Acceptance 3 is not met, and the deviation text understated the gap ("one non-winning epoch"; the branch's own
     logs `rte03-h.err` and `r2_dac4.err` show 8 of 180 dac4 dr reports).
  2. The branch did not merge cleanly with m0 (`score()` with `cap_total`, `Routes::terms`, the 3-tuple `route`).
  3. A resolver test case was missing (later net trunk-only, earlier net's access shape deleted).

  **The code is nevertheless on m0.** `ba49ae2` ("M0: close RTE T2") ports `e8ef0e9`, `b36ca9f` and `93fe6fb` outside
  the item review. It resolves the three conflicts (keeps `foreign_metal`, the `cap_total`-free `score()`,
  `routes.terms`; adjusts the 3-tuple tests), adds the missing resolver case, and states in its commit message that
  acceptance 3 is not met. The originals are kept on branch `m0-rte03-original` (`93fe6fb`). plan-05 RTE-03 records
  the status. All seven RTE-03 dr tests are ok in this pass (`a_ring_band_is_a_hard_obstacle`,
  `a_route_touching_foreign_cell_metal_is_a_short`, `resolver_sacrifices_the_later_access_shape`,
  `an_xy_overlap_on_non_adjacent_layers_is_open`, `a_trunk_merges_with_its_own_cell_strap`, `ring_pins_are_routed`,
  `negotiation_persists_across_calls`); the dr unit target is 34/0/0. `signoff_fixtures` on the merged tree is green
  except dac4 (#2), including the `--ignored` OTA test, with REL-03's EM check on the same final geometry.

  **The gap, re-measured on `44b37f1`** (instrumented copy, seed 1, FEEDBACK_ITERS 5, one line per `dr::score` call):

  | run | dr reports | `unresolved congestion` V | `drawn short nets` V | `open net` V | `to cell metal` V | `pin access sacrificed` V |
  |---|---|---|---|---|---|---|
  | all 10 circuits | 540 | 7 | 7 | 16 | 0 | 1 |
  | dac4 alone | 90 | **7** | 7 | 16 | 0 | 1 |
  | the other 9 | 450 | 0 | 0 | 0 | 0 | 0 |

  - dac4's 7 reports carry margins 4, 4, 3, 4, 4, 4 and 16, each with `drawn short nets 3/4`, `2/3`, or `2/3` + `2/4`
    (the 16 one also has `open net 3`). This is the same multiset as pass 3's `t2_dac4.err` on `ba49ae2`. The report
    indices differ slightly (15, 30, 48, 52, 69, 70, 85 here; 15, 31, 48, 53, 69, 70, 85 there), consistent with the
    stderr interleaving of concurrently scored starts; the counts are the measure.
  - None of the 7 is a winner: the bench winner has overuse 0 and no `drawn short`/`open` row (§4).
  - Every one of dac4's 90 reports also carries `batch:routing hard 0` (margin 1025 or 1111): the in-loop antenna
    batch (the winner's `route/Antenna: net top 1.1108099`), not a short or congestion. The 9 non-empty reports of the
    other circuits are `batch:routing hard 1` on the three identical OTAs (non-winners; their winners read route
    hard 0).
  - For the base comparison (pre-`ba49ae2` m0 had 61 of 90 dac4 reports with Θ `routing overuse`, overuse 3–73, and
    4 with `open net` V) see pass 3's `base_dac4.err` figures, unchanged here; the base code was not re-run.

  **Open decision (orchestrator/user):** accept RTE-03 on m0 with acceptance 3 recorded as not met (residual
  congestion → RTE-08, M1), or revert `ba49ae2`. A commit on m0 does not make this decision.
- **RTE-06** (antenna repair on drawn geometry). **Rejected**, not on m0; commits `b635606`/`63bc2a2`/`32efea4` kept
  on branch `m0-rte06-rejected`. Its own acceptance was red on its branch with the same
  `dac4: ERC rows ["erc/ar.met2.1:gate"]`. The stated cause is plausible: `routing_stack`
  (frontend/library/src/elaborate.rs:211–220) gives dr only [met1, met2] on sky130, so a cap top plate on met2 that
  is over the limit before routing cannot be repaired inside dr. Two undisclosed, untested dr changes were also found
  (a `frozen` set keeping shielded nets out of the post-shield antenna pass; a `judge` rollback gate on redraws).
  **The user still has to decide:** merge as "acceptance not met" and name the item that owns dac4's met2 cap-plate
  antenna, or re-scope criterion 2.

## 2. Exit criteria (00-MASTER-PLAN.md §5, M0)

Every test named below is `ok` in this pass's release log unless stated otherwise.

| # | Criterion | Status | Evidence |
|---|---|---|---|
| 1 | CI runs `cargo test --workspace` on every push; release generator tests; nightly tool tests fail loudly when a tool is missing (FLOW T13) | **met** (not observed on GitHub: nothing is pushed) | `.github/workflows/ci.yml`:<br>• `test` (push/PR): `cargo build --workspace --locked`, `cargo test --workspace --locked --no-fail-fast` (Cargo.lock is tracked).<br>• `release`: `cell_selfcheck` and `ota_cross_pdk` in release.<br>• `nightly` (schedule/dispatch, `timeout-minutes: 90`): `PHILIS_REQUIRE_TOOLS=1`, ngspice, sky130 `fa87f8f4…` via volare; `cargo test --release --workspace` plus `-p benchmark --test signoff_fixtures -- --ignored` by name (pass 3's finding that workspace `--include-ignored` hung on EXT placeholders is fixed).<br>Loud skip, checked this pass: with `PHILIS_REQUIRE_TOOLS=1 PDK_ROOT=/nonexistent`, `fixture_nets_without_a_resistor_keep_em_sizing` and both model-dependent `perf_postlayout` tests panic (`PHILIS_REQUIRE_TOOLS=1 and sky130 models (…) is missing`, frontend/library/src/lib.rs:38); without the variable they print `… unavailable — skipping`. All go through `library::tools::present_or_skip` (pass 3's gap closed). `tool_or_skip_tells_present_from_missing` ok.<br>The suite itself is not green (§3), so `test` would be red on push; that is the honest state, nothing is waived. |
| 2 | `signoff_fixtures` green including dac4 at `feedback_iters = 1` (REL T1, RTE T11) | **not met** (RTE-06 not merged) | `fixtures_sign_off_within_baseline` FAILED at benchmarks/tests/signoff_fixtures.rs:175: `dac4: ERC rows ["erc/ar.met2.1:gate"], expected []`. dac4: DRC 0, ERC 1, PEX 282.0 fF, LVS clean with 16 unverified. The other 6 default fixtures: DRC 0, ERC 0, clean LVS (§3). `large_fixtures_sign_off_within_baseline` (`--ignored`) ok: ota, ota_constrained, tt_ota each DRC 0, ERC 0, PEX 585.8 fF, clean LVS, 0 unverified. |
| 3 | `antenna_in_loop_never_passes_what_signoff_fails` green (REL T2) | **met** | ok. `--nocapture` (now printing total, sat, viol, unk):<br>• dac4: signoff `["erc/ar.met2.1:gate"]`, in-loop `(5, 4, 1, 0)`: the in-loop model flags it.<br>• pair, quad, rc_filter, chain4: `(1, 1, 0, 0)`, no signoff rows.<br>• bjt_mirror, bgr_core: no rows on either side.<br>Per fixture, not per net (review finding 20, deferred to M1). |
| 4 | Bench LVS reads `PARTIAL(2)` bjt_mirror, `PARTIAL(9)` bgr_core, `PARTIAL(16)` dac4, `MATCH` for the other 7 (PERF M0) | **met** | Bench table §4, LVS column, exactly as required. |
| 5 | Warnings printed and absent from \|V\| | **met** | Bench prints `warnings N` per circuit (0 on all 10). `warnings_are_not_violations`, `v_counts_rules_not_batches`, `theta_counts_each_budget_once`, `nan_loses` ok. DRC warnings are now kept out of the DRC count too (`verify::Finding::warning`, review finding 23). |
| 6 | FLOW T1 size fidelity: 0 devices | **met** | `size_tests::drawn_width_equals_simulated_width` ok (all MOS, all 10 fixtures); `mos_size_follows_spice_semantics`, `per_finger_convention_stores_total_width` ok. |
| 7 | FLOW T3 deck honesty 0/0/0 | **met** | `every_builtin_pdk_loads`, `vendored_decks_name_their_origin`, `a_process_number_needs_a_source`, `generators_read_exactly_the_required_keys` (kernel/cells/tests/deck_keys.rs), `sidecar_rejects_a_misspelt_key` ok. |
| 8 | FLOW T4: LU rules present, no antenna sidewall off by > 5 % | **met** | `latch_up_rules_find_a_far_tap`, `antenna_sidewall_thickness_matches_pex` ok. pdks/decks/sky130.deck:517–519 has LU.2, LU.2.1, LU.3. The bench runs them or lists them skipped (`EmptyLayer`) where the fixture lacks the diffusion. |
| 9 | Exactly one dual step per epoch (FLOW T6) | **met** | `start_tests::one_dual_step_per_epoch`, gp `a_slack_budget_relaxes_its_price`, `a_saturated_price_is_reported`, dp `place_does_not_settle` ok. |
| 10 | EM known/unknown/violated counts printed per fixture (REL T3/T4) | **met** | Bench `EM known K (viol V), unknown U, max use X` on every row (§4); violations 0 everywhere, unknown 1 on bgr_core, bjt_mirror, rc_filter. Totals: Electromigration 46 rows, 43 satisfied, 0 violated, 3 unknown. `an_unknown_is_not_satisfied`, `em_rules_name_what_goes_unchecked` ok. |
| 11 | No `drawn short` or `unresolved congestion` hidden in Θ (RTE T2) | **met as worded in the master plan; not met under plan-05's T2 / RTE-03 acceptance 3** | **Θ:** `grep -rn 'routing overuse' --include='*.rs' backend frontend kernel benchmarks` is empty (exit 1).<br>**V:** `dr::score` (backend/dr/src/lib.rs:1910–1945) puts residual overuse in V as `unresolved congestion` (margin = overuse), shorts as `drawn short nets a/b` / `drawn short net n to cell metal`, opens as `open net n`.<br>**Tests:** `a_route_touching_foreign_cell_metal_is_a_short`, `a_ring_band_is_a_hard_obstacle`, `an_xy_overlap_on_non_adjacent_layers_is_open`, `resolver_sacrifices_the_later_access_shape` ok.<br>**Measured on this code (§1):** 7 of dac4's 90 dr reports (none a winner) carry `unresolved congestion` + `drawn short nets` V; 0 of the other 9 circuits' 450. They are visible in V, not hidden in Θ, so the master-plan criterion holds. plan-05's M0 row T2 ("no `drawn short`/`unresolved congestion` V … on any bench circuit") and RTE-03 acceptance 3 do not. Holds only while `ba49ae2` stays on m0 (§6). |
| 12 | PLC baseline table and EXT corpus expectations recorded | **met** | docs/plans/baseline-plc-01.md (10 fixtures × seeds 1–5). annotator `corpus_expectations`, `negative_corpus`, `strongarm_conflicts_are_todays` ok; the corpus tests for later items are `#[ignore]` with their item named (§3). |
| 13 | `every_generator_enumerates_on_every_deck` green in debug on 4 decks; 0 inductor shapes in any output | **met** | Debug run (`unoptimized + debuginfo`) ok; the test loops sky130, gf180mcu, ihp_sg13g2, generic_finfet (kernel/cells/tests/cell_selfcheck.rs:421) and is not `cfg`-gated. `Inductor::draw` returns an empty macro; `an_inductor_is_undrawable_without_a_recogniser`, `an_inductor_is_one_undrawable_finding`, verify `an_inductor_is_unverified` ok. No bench fixture has an `L` card. |

## 3. Test summary

`cargo build --release --workspace`: exit 0.

`cargo test --release --workspace --no-fail-fast`: exit 101. **439 passed, 2 failed, 7 ignored**, summed over every
`test result` line (pass 3: 435/2/7; +4 = the review panel's new tests). Doc-tests: 0.

Failing tests (the same two as passes 1–3):

1. **`benchmark` `tests/signoff_fixtures.rs::fixtures_sign_off_within_baseline`** panicked at
   signoff_fixtures.rs:175:9: `dac4: ERC rows ["erc/ar.met2.1:gate"], expected []` (criterion 2; RTE-06). The line
   moved from 173 because `44b37f1` edited the file; the message is the same.
2. **`library` `tests/hier_elaborate.rs::hierarchical_composition_elaborates_with_a_correct_schematic`** panicked at
   hier_elaborate.rs:216:5: `hierarchical layout must be LVS-clean against its schematic`, with 8 ×
   `lvs/lvs.unpaired_device:-` and 17 × `lvs/lvs.unpaired_net:-`. Not a regression: FLOW-14 (`0ef8506`) replaced an
   assertion-free check and recorded it red with these counts. No M0 item fixes it.

Ignored (7), each with its reason in the attribute: `strongarm_matches_align_gold` (EXT-14), `coverage_is_total`
(EXT-10), `negative_corpus_sc_switches_and_equal_fets` (EXT-04), `no_emitted_conflicts_strongarm` (EXT-14, REL-09),
`permutation_invariance` (EXT-06), `twelve_thousand_devices` (EXT-06, T8), `large_fixtures_sign_off_within_baseline`
(run separately in this pass: ok).

Per target (passed/failed/ignored; changes against pass 3 in bold):

| Crate | Target | P/F/I |
|---|---|---|
| analog | unit | 85/0/0 |
| annotator | unit | 33/0/0 |
| annotator | `align_gold` | 0/0/1 |
| annotator | `corpus` | **4**/0/4 |
| annotator | `scale` | 0/0/1 |
| benchmark | `bench` unit | 13/0/0 |
| benchmark | `signoff_fixtures` | 16/**1**/1 |
| cells | unit | 39/0/0 |
| cells | `cell_selfcheck` | 10/0/0 |
| cells | `deck_keys` | 1/0/0 |
| dp | unit | 28/0/0 |
| dr | unit | 34/0/0 |
| gp | unit | 6/0/0 |
| gr | unit | 11/0/0 |
| library | unit | **75**/0/0 |
| library | `antenna_diode`, `elaborate`, `extra_devices`, `flow_smoke`, `injected_macro` | 1/0/0 each |
| library | `drawn_cards`, `emit_roundtrip`, `placement_metrics` | 2/0/0 each |
| library | `hier_elaborate` | 0/**1**/0 |
| library | `ota_cross_pdk` | 3/0/0 |
| library | `perf_postlayout` | 3/0/0 (ngspice ran, 45.6 s) |
| macro_master | unit | 9/0/0 |
| pnr_core | unit | 16/0/0 |
| verify | unit | **40**/0/0 |
| visualizer | unit | 2/0/0 |

`signoff_fixtures` per fixture (identical to pass 3):

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

Exit 0; 10/10 circuits placed and routed. Per-circuit times sum to 141.7 s (sequential, so about the wall time; not
timed separately). Load average was 10–12 on 32 cores with other sessions running; the ms column is for orientation
only. The only stderr was `[op] operating point unavailable (no device operating points: ); continuing with zero
power` for the two BJT fixtures.

| circuit | ms | cells / nets | WL nm | unrouted | route hard | overuse | DRC | LVS | ERC | warn | C Σ / sig fF | area µm² | util % | active % | best | outer, esc | bias | EM known (viol), unk, max use | usage | lattice off | clr residue nm² |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| bgr_core | 6 104 | 2 / 3 | 260 250 | 0 | 0 | 0 | 0 | PARTIAL(9) | 0 | 0 | 76.7 / 19.6 | 135.0 | 100.0 | 1.2 | 2/5 conv. | 1, 0 | none | 0 (0), 1, none | 1.000 | 0 | 0 |
| bjt_mirror | 5 681 | 2 / 5 | 73 480 | 0 | 0 | 0 | 0 | PARTIAL(2) | 0 | 0 | 33.0 / 31.3 | 64.8 | 79.4 | 1.8 | 1/5 conv. | 1, 0 | none | 0 (0), 1, none | 1.260 | 0 | 0 |
| chain4 | 5 722 | 4 / 7 | 44 675 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 22.6 / 8.4 | 62.6 | 100.0 | 19.4 | 1/5 conv. | 1, 0 | 0 µW probe | 5 (0), 0, 0.000 | 1.000 | 1 | 0 |
| dac4 | 19 146 | 14 / 12 | 640 880 | 0 | 1 | 0 | 0 | PARTIAL(16) | 0 | 0 | 297.2 / 147.8 | 2400.6 | 61.1 | 0.8 | 2/15 budget | 3, 2 | 112 µW probe | 11 (0), 0, 0.221 | 1.636 | 8 | 291 300 |
| ota | 28 022 | 5 / 9 | 497 540 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 588.4 / 263.8 | 1729.8 | 87.7 | 42.1 | 1/20 budget | 4, 3 | 2 µW probe | 6 (0), 0, 0.003 | 1.140 | 2 | 0 |
| ota_constrained | 32 396 | 5 / 9 | 497 540 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 588.4 / 263.8 | 1729.8 | 87.7 | 42.1 | 1/20 budget | 4, 3 | 2 µW probe | 6 (0), 0, 0.003 | 1.140 | 2 | 0 |
| pair | 5 526 | 2 / 3 | 19 905 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 4.8 / 3.1 | 8.6 | 100.0 | 48.4 | 1/5 conv. | 1, 0 | 0 µW probe | 3 (0), 0, 0.000 | 1.000 | 1 | 0 |
| quad | 6 222 | 4 / 3 | 37 945 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 9.3 / 6.1 | 15.9 | 100.0 | 51.2 | 5/5 conv. | 1, 0 | 0 µW probe | 3 (0), 0, 0.000 | 1.000 | 1 | 0 |
| rc_filter | 6 159 | 3 / 5 | 87 230 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 24.0 / 7.6 | 163.3 | 62.0 | 2.8 | 2/5 conv. | 1, 0 | 28 µW probe | 3 (0), 1, 0.055 | 1.613 | 2 | 0 |
| tt_ota | 26 735 | 5 / 9 | 497 540 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 588.4 / 263.8 | 1729.8 | 87.7 | 42.1 | 1/20 budget | 4, 3 | 2 µW probe | 6 (0), 0, 0.003 | 1.140 | 2 | 0 |

Same on every row: overlap 0 nm², matched mismatch 0, dp temps 220, `key tier` = `C sig`. `islands extra`,
`decode fail` and `matched incompat` print `n/a` (review finding 17: unmeasured before PLC-12/08/03; pass 3 printed 0).

**Against pass 3:** every measured column except ms is identical on all 10 rows, and the constraint table totals are
identical. The review-panel fixes changed what is printed, not the layouts: the three `n/a` columns, and rc_filter's
skip list now names `lvs.parameter_mismatch(non-MOS values)` (finding 18: its resistor value is compared by nobody).
The constraint table gained an `n/a` column (0 everywhere) and `rate` is now sat / checked, so Electromigration reads
100 % with 3 unknown, not a lower rate.

**dac4's `route hard 1`** is the in-loop antenna rule on net `top` (`route/Antenna: net top 1.1108099`,
target/bench_debug/dac4/violations.txt; max use 2.111). Signoff passes that net at FEEDBACK_ITERS 5 (ERC 0) but fails
it at 1 (criterion 2). `violations.txt` also lists `lvs-coverage/unverified:Capacitor:cap_generic_m1m2 16`.

**Skipped rules** (`signoff.coverage.skipped_rules`, as printed):

- Every fixture: `lvs.device_count_mos`, `lvs.device_count_bjt`, `lvs.parametric` (`NotInDeck`); `multiple_drivers`
  (`Waived`); `m1.density`…`m4.density` (`ChipLevel`); `floating_gate` (exempt: no port list).
- Per fixture: the LU rules whose layer is empty (`EmptyLayer`).
- bgr_core and bjt_mirror: `ir_drop`, `esd.pad`, the `vgs/vds` ratings and `EM.met*_via*` (`NoDesignIntent`).
- rc_filter: `lvs.parameter_mismatch(non-MOS values)`.

Constraint satisfaction, verbatim totals:

| type | arm | total | sat | viol | unk | n/a | rate | max use (at) | Θ |
|---|---|---|---|---|---|---|---|---|---|
| Antenna | hard | 21 | 20 | 1 | 0 | 0 | 95 % | 2.111 (dac4) | 1.111 |
| CommonCentroid | budget | 3 | 0 | 3 | 0 | 0 | 0 % | 2.195 (tt_ota) | 3.584 |
| CommonNode | budget | 6 | 6 | 0 | 0 | 0 | 100 % | 0.001 (tt_ota) | 0 |
| CouplingBudget | budget | 39 | 39 | 0 | 0 | 0 | 100 % | 0.523 (dac4) | 0 |
| CrosstalkExclusion | budget | 12 | 12 | 0 | 0 | 0 | 100 % | 0.475 (tt_ota) | 0 |
| Differential | hard | 3 | 3 | 0 | 0 | 0 | 100 % | – | 0 |
| Electromigration | hard | 46 | 43 | 0 | 3 | 0 | 100 % | 0.221 (dac4) | 0 |
| Environment | budget | 6 | 6 | 0 | 0 | 0 | 100 % | 0.320 (tt_ota) | 0 |
| IrDrop | budget | 23 | 23 | 0 | 0 | 0 | 100 % | 0.125 (dac4) | 0 |
| MatchingPair | budget | 6 | 6 | 0 | 0 | 0 | 100 % | 0.000 (tt_ota) | 0 |
| ParasiticBudget | budget | 39 | 39 | 0 | 0 | 0 | 100 % | 0.824 (dac4) | 0 |
| Proximity | budget | 11 | 11 | 0 | 0 | 0 | 100 % | 0.255 (tt_ota) | 0 |
| Symmetry | hard | 9 | 9 | 0 | 0 | 0 | 100 % | – | 0 |
| ThermalGradient | budget | 6 | 6 | 0 | 0 | 0 | 100 % | 0.000 (tt_ota) | 0 |
| Utilization | budget | 10 | 10 | 0 | 0 | 0 | 100 % | 0.982 (dac4) | 0 |
| **OVERALL** | | 240 | 233 | 4 | 3 | 0 | 98 % | | |

## 5. Against the pre-M0 ground truth (audit-06 §G)

**Tests:**

| | pre-M0 (G.2) | now |
|---|---|---|
| Result | 331 passed, 1 failed, 2 ignored, exit 101 | 439 passed, 2 failed, 7 ignored, exit 101 |
| Failing | dac4 `erc/ar.met2.1:gate` | dac4 (same rule) and `hier_elaborate` |
| Ignored | 2: `large_fixtures…`, `tmp_deck_drc::dump` | 7 (EXT corpus placeholders and `large_fixtures…`; `tmp_deck_drc.rs` deleted by FLOW-14) |
| dr unit tests | 25 | 34 |
| verify unit tests | 19 | 40 |
| library unit tests | 48 | 75 |

`hier_elaborate` "passed" before only because it asserted nothing (FLOW-14). The OTA-class fixtures were never
signed off by a default run before; they are still `#[ignore]` in the default run (nightly runs them by name), and
were run here: clean.

**`signoff_fixtures` per fixture:**

- DRC is 0 everywhere, before and now.
- ERC before: bjt_mirror 1, bgr_core 1 (`supply_short` false positives), dac4 1. Now only dac4 has 1.
- PEX is unchanged on six fixtures; dac4 326.0 → 282.0 fF.
- LVS: bjt_mirror and bgr_core no longer read "clean" with nothing compared; they report 2 and 9 unverified. dac4's
  15 silently dropped caps are now 16 unverified (Σm).

**Bench, G.3 → now:**

| circuit | ms | WL nm | overuse | LVS | ERC | C Σ fF | area µm² | util % | best |
|---|---|---|---|---|---|---|---|---|---|
| bgr_core | 16 113 → 6 104 | 260 320 → 260 250 | 0 → 0 | MATCH¹ → PARTIAL(9) | 1 → 0 | 77.2 → 76.7 | 135.0 → 135.0 | 100.0 → 100.0 | 4/5 budget → 2/5 conv. |
| bjt_mirror | 12 300 → 5 681 | 74 085 → 73 480 | 0 → 0 | MATCH¹ → PARTIAL(2) | 1 → 0 | 33.7 → 33.0 | 64.7 → 64.8 | 79.4 → 79.4 | 3/5 budget → 1/5 conv. |
| chain4 | 11 647 → 5 722 | 44 675 → 44 675 | 0 → 0 | MATCH → MATCH | 0 → 0 | 22.6 → 22.6 | 62.6 → 62.6 | 100.0 → 100.0 | 1/5 → 1/5 |
| dac4 | 287 520 → 19 146 | 580 410 → 640 880 | **18 → 0** | MATCH⁵ → PARTIAL(16) | 0 → 0 | 275.8 → 297.2 | 2326.9 → 2400.6 | 60.0 → 61.1 | 5/15 → 2/15 budget |
| ota (×3, identical) | 52–59 k → 27–32 k | 444 720 → 497 540 | 0 → 0 | MATCH → MATCH | 0 → 0 | 616.0 → 588.4 | 1865.1 → 1729.8 | 85.9 → 87.7 | 1/20 conv. → 1/20 budget |
| pair | 11 468 → 5 526 | 17 700 → 19 905 | 0 → 0 | MATCH → MATCH | 0 → 0 | 4.6 → 4.8 | 8.6 → 8.6 | 100.0 → 100.0 | 4/5 → 1/5 |
| quad | 16 803 → 6 222 | 37 945 → 37 945 | 0 → 0 | MATCH → MATCH | 0 → 0 | 9.3 → 9.3 | 15.9 → 15.9 | 100.0 → 100.0 | 5/5 → 5/5 |
| rc_filter | 13 445 → 6 159 | 86 835 → 87 230 | 0 → 0 | MATCH → MATCH | 0 → 0 | 24.4 → 24.0 | 131.7 → 163.3 | 76.9 → 62.0 | 5/5 → 2/5 |

¹ G.3: nothing compared. ⁵ G.3: the 15 unit caps were not in the reference.

- **Wall time:** 539.5 s before, about 142 s now. Both under uneven load; no item claims a speed-up (baseline-plc-01.md:
  timings for orientation only).
- **dac4 cell count:** 15 → 14. The 15th was a flow-inserted antenna diode.
- **What the table was meant to show:** every "MATCH" that compared nothing now reads PARTIAL(n) (PERF-02); the ERC
  `supply_short` false positives are gone; dac4's winner overuse is 0, and residual overuse can no longer sit in Θ at
  all since `ba49ae2` (it is V, §1).
- **Moves explained in the merge commits:** epoch selection (FLOW-02 `lex_key`, FLOW-03, PERF-07 signal-C tier; for
  dac4 also RTE-03's dr) changed the winner and C on several fixtures; FLOW-01 (drawn size = simulated size) moved
  the OTA WL, area, C and CommonCentroid rows.

**Constraint table:**

- Totals: 241/237/2 viol/2 unk before; 240/233/4 viol/3 unk now. IrDrop rows 24 → 23.
- **CommonCentroid** 3/3 satisfied (max use 0.000) → 0/3 (2.195, Θ 3.584). The FLOW-01 merge records this as the
  honest result of drawing the simulated size (XM1/XM2 W=10u nf=2 fold to two 5 µm fingers each, placed AABB);
  `cellgen::folds` ignores CommonCentroid, follow-up work outside M0.
- **Antenna** 21/21 → 20/21 (dac4 `top` at 2.111).
- **ParasiticBudget** 38/39 (dac4 1.016) → 39/39 (0.824). **Utilization** 9/10 → 10/10.
- **Electromigration** unknown 2 → 3 (the new one is rc_filter); not traced to a single item (REL-01 and REL-03 both
  touch these rows).
- **Skip log:** the `m*.density` rules "ran" over zero windows before (AV-18), now `ChipLevel` (PERF-04); whether EM
  ran is printed per fixture (AV-26 closed); the LU rules exist and run (FLOW-05).

## 6. Open, for the orchestrator/user

1. **RTE-03:** accept `ba49ae2` on m0 with acceptance 3 recorded as not met (7 of 90 non-winning dac4 dr reports carry
   `unresolved congestion` + `drawn short nets` V, re-measured on `44b37f1`; 0 of 450 elsewhere; convergence is
   RTE-08, M1), or revert it. Criterion 11 holds under the master plan's wording only while `ba49ae2` stays.
2. **RTE-06:** merge from `m0-rte06-rejected` as "acceptance not met" with a named owner for dac4's met2 cap-plate
   antenna, or re-scope criterion 2. Criterion 2 stays red until then. The dac4 ERC baseline is not relaxed.
3. **`hier_elaborate`** (8 unpaired devices, 17 unpaired nets) is red with no owner in M0.
4. **The OTA CommonCentroid rows** are 0/3 since FLOW-01, pending `cellgen::folds` honouring CommonCentroid.
5. **REL T2 per-net check** (review finding 20) is deferred to M1: signoff ERC rows carry no net.

Closed since pass 3: the EM-sizing test's loud skip (§2 #1), the nightly job's hang, and the lost branch work (the
RTE-03 and RTE-06 originals are on `m0-rte03-original` and `m0-rte06-rejected`).

## 7. Review panel

Recorded at `44b37f1`, kept as written. Pass 4 confirms its suite figure (439/2/7) and runs the bench it did not run (§4: no layout column moved).

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
