# M0 report: verify pass 2

This pass checks the exit criteria of milestone M0 ("Honest measurement", 00-MASTER-PLAN.md §5) on branch `m0` at
`ba49ae2` (M0: close RTE T2), 88 commits after `ddab38e`. The only code change since pass 1 (`2245425`, report only;
`2f37409` code) is `ba49ae2`, which ports RTE-03 onto m0 (§1).

Every number here was measured on 2026-10-02 in the integration worktree. The profile is release unless stated.
The commands were:

- `cargo build --release --workspace`
- `cargo test --release --workspace --no-fail-fast`
- `cargo run --release -p benchmark --bin bench local`
- three targeted release re-runs: `large_fixtures_sign_off_within_baseline --ignored`,
  `antenna_in_loop_never_passes_what_signoff_fails --nocapture`, and the debug
  `every_generator_enumerates_on_every_deck`
- a debug run of `flow_smoke`, `placement_metrics` and `antenna_diode`, which exercise `debug_check_joined`, new in
  `ba49ae2`

ngspice and the sky130 ngspice models (`~/.volare/sky130A`) were present, so the ngspice tests ran rather than
skipped: no `SKIP` line appears in the test log. magic, netgen and klayout were not on PATH, and no M0 test calls
them.

**Verdict: M0 is not done.** 12 of the 13 criteria below are met:

- Criterion #1 (CI) is met with a gap, unchanged from pass 1.
- Criterion #11 (RTE T2) is met as the master plan words it. It is not met under plan-05's stricter wording, and
  RTE-03's own acceptance is not met (§1, §2).
- Criterion #2 is not met. Its item, RTE-06, was not merged.

The suite has two red tests, the same two as in pass 1:

- the dac4 test (#2);
- `hier_elaborate`, which FLOW-14 made honest and which has stayed red since, with no M0 owner.

## 1. Items

Merged (24, through review): FLOW-14, GAP-20, FLOW-04, FLOW-05, FLOW-01, CELL-01, CELL-02, CELL-04, EXT-01, PLC-01,
RTE-32, RTE-02, FLOW-02, FLOW-03, PERF-01, PERF-02, PERF-04, PERF-06, PERF-07, PERF-03 (steps 1–2; its port step
waits for FLOW-07, M1), REL-01, REL-03, RTE-09, REL-02.

Not merged through review (2). Their state on m0 differs:

- **RTE-03** (foreign metal hard; shorts, opens, congestion as V). The review **rejected** the m0-routing branch for
  three reasons:
  1. Acceptance 3 is not met, and the deviation text understated the gap ("one non-winning epoch").
     `rte03-h.err` and `r2_dac4.err` each had 8 of 180 dac4 dr reports with `unresolved congestion`.
  2. The branch did not merge cleanly with m0: `score()` with `cap_total`, `Routes::terms`, and the 3-tuple `route`
     return.
  3. A resolver test case was missing.

  **The code is nevertheless on m0 now.** `ba49ae2` ports `e8ef0e9`, `b36ca9f` and `93fe6fb` onto m0 outside the
  item review:
  - It resolves the conflicts. It keeps `foreign_metal` and the `cap_total`-free `score()`, keeps `routes.terms`, and
    adjusts the 3-tuple tests.
  - It adds the missing resolver case: against the later net's trunk, the earlier net's jog is the one deleted
    (`resolver_sacrifices_the_later_access_shape` ok).
  - It re-measures the gap and states that acceptance 3 is not met.

  In this pass, all seven RTE-03 dr tests are ok: `a_ring_band_is_a_hard_obstacle`,
  `a_route_touching_foreign_cell_metal_is_a_short`, `resolver_sacrifices_the_later_access_shape`,
  `an_xy_overlap_on_non_adjacent_layers_is_open`, `a_trunk_merges_with_its_own_cell_strap`, `ring_pins_are_routed`
  and `negotiation_persists_across_calls`, with the dr unit target at 34/0/0. So is `signoff_fixtures`
  `--include-ignored` on the merged tree (REL-03's EM check runs on the same final geometry), and so are the debug
  flow tests.

  The gap, from the port session's logs: one `eprintln` per dr report, seed 1, FEEDBACK_ITERS 5, removed before
  commit; `scratchpad/base_dac4.err` and `t2_dac4.err`.
  - **At base m0 `2245425`:** 61 of dac4's 90 dr reports carry residual overuse (3 to 65 nodes) only as Θ `routing
    overuse`. The commit message says 62; the log has 61.
  - **At `ba49ae2`:** no report has a Θ overuse entry. 7 of 90 carry `unresolved congestion` V, with margins 4, 4, 3,
    4, 4, 4 and 16, each paired with a `drawn short nets 3/4`, `2/3` or `2/4` V. 16 carry `open net` V. None carries
    `drawn short net n to cell metal`. The other 9 circuits have none of these entries in any of their 450 reports.
  - **Per call index** (dr report lines 15, 31, 48, 53, 69, 70 and 85 of 90), base had Θ `routing overuse` at the same
    index on 6 of the 7. Overuse was 3, 65, 3, 3, 4 and 3. At line 48, base had overuse 0. This is aligned by call
    order only: placement feedback can diverge once routing differs, so it is indicative, not a paired comparison.
  - **Count:** the review's 8 of 180 came from the old branch's runs. This pass did not re-instrument (no code
    changes allowed), so 7 of 90 is the port's measurement, not an independent one.

  **Open decision (orchestrator/user):** accept RTE-03 on m0 with acceptance 3 recorded as not met (residual
  congestion is PathFinder convergence, RTE-08, M1), or revert `ba49ae2`. The review required that decision to be
  made explicitly. A commit on m0 does not make it.
- **RTE-06** (antenna repair on drawn geometry). **Rejected**, not on m0. Its own acceptance is red:
  `fixtures_sign_off_within_baseline` fails on that branch with the same `dac4: ERC rows ["erc/ar.met2.1:gate"]`.
  The stated cause is plausible: `routing_stack` (frontend/library/src/elaborate.rs:211–220) gives dr only
  [met1, met2] on sky130, so a cap top plate on met2 that is over the limit before routing cannot be repaired inside
  dr. No code defect was found. Two undisclosed, untested dr changes were also found:
  - a `frozen` set that keeps shielded nets out of the post-shield antenna pass;
  - a `judge` rollback gate on redraws.

  **The user still has to decide:** merge RTE-06 as "acceptance not met" and move the dac4 fix to FLOW-05/FLOW-08 or
  to the routing stack / cap pin layer, or re-scope the item.

## 2. Exit criteria (00-MASTER-PLAN.md §5, M0)

| # | Criterion | Status | Evidence |
|---|---|---|---|
| 1 | CI runs `cargo test --workspace` on every push; release generator tests; nightly tool tests fail loudly when a tool is missing (FLOW T13) | **met, with a gap** | `.github/workflows/ci.yml` has three jobs. `test` runs on push: `cargo build --workspace --locked`, then `cargo test --workspace --locked --no-fail-fast`. `release` runs `cell_selfcheck` and `ota_cross_pdk` in release. `nightly` runs with `PHILIS_REQUIRE_TOOLS=1` and `--include-ignored`, with ngspice and sky130 at `fa87f8f4…` via volare. `tool_or_skip_tells_present_from_missing` ok. CI was not observed running because nothing is pushed. **Gap (unchanged):** the `library` unit test `fixture_nets_without_a_resistor_keep_em_sizing` (frontend/library/src/lib.rs:1706–1707) skips with `eprintln!("SKIP …")` and returns, and it ignores `PHILIS_REQUIRE_TOOLS`. A nightly run that lacked ngspice or the models would pass it instead of failing loudly. The suite is also not green (§3: two red tests). plan-08's M0 row expects only the dac4 test to be red. |
| 2 | `signoff_fixtures` green including dac4 at `feedback_iters = 1` (REL T1, RTE T11) | **not met** (RTE-06 not merged) | `fixtures_sign_off_within_baseline` FAILED at benchmarks/tests/signoff_fixtures.rs:173: `dac4: ERC rows ["erc/ar.met2.1:gate"], expected []`. dac4's row: DRC 0, ERC 1, PEX 282.0 fF, LVS clean with 16 unverified. The other 6 default fixtures have DRC 0, ERC 0 and clean LVS (§3). `large_fixtures_sign_off_within_baseline` (`--ignored`) ok: ota, ota_constrained and tt_ota each have DRC 0, ERC 0, PEX 585.8 fF, clean LVS and 0 unverified. |
| 3 | `antenna_in_loop_never_passes_what_signoff_fails` green (REL T2) | **met** | ok. With `--nocapture`, dac4's signoff gives `["erc/ar.met2.1:gate"]` and the in-loop count is `(5 total, 4 satisfied, 0 unknown)`, so the in-loop model flags the violation. pair, quad, rc_filter and chain4: in-loop `(1, 1, 0)`, no signoff rows. bjt_mirror and bgr_core: no rows on either side. |
| 4 | Bench LVS reads `PARTIAL(2)` bjt_mirror, `PARTIAL(9)` bgr_core, `PARTIAL(16)` dac4, `MATCH` for the other 7 (PERF M0) | **met** | Bench table §4, LVS column, exactly as required. |
| 5 | Warnings printed and absent from \|V\| | **met** | The bench prints `warnings N` per circuit (0 on all 10). `start_tests::warnings_are_not_violations`, `v_counts_rules_not_batches`, `theta_counts_each_budget_once` and `nan_loses` ok. |
| 6 | FLOW T1 size fidelity: 0 devices | **met** | `size_tests::drawn_width_equals_simulated_width` ok, over all MOS on all 10 fixtures. `mos_size_follows_spice_semantics` and `per_finger_convention_stores_total_width` ok. |
| 7 | FLOW T3 deck honesty 0/0/0 | **met** | Not embedded: `every_builtin_pdk_loads` and `vendored_decks_name_their_origin` ok. Without `_source`: `a_process_number_needs_a_source` ok. Required keys unread: `generators_read_exactly_the_required_keys` ok (kernel/cells/tests/deck_keys.rs). `sidecar_rejects_a_misspelt_key` ok. |
| 8 | FLOW T4: LU rules present, no antenna sidewall off by > 5 % | **met** | `latch_up_rules_find_a_far_tap` and `antenna_sidewall_thickness_matches_pex` ok. pdks/decks/sky130.deck:517–519 has LU.2, LU.2.1 and LU.3. In the bench they run, or are listed as `Skipped(EmptyLayer)` where the fixture lacks the diffusion. |
| 9 | Exactly one dual step per epoch (FLOW T6) | **met** | `start_tests::one_dual_step_per_epoch` ok. gp `a_slack_budget_relaxes_its_price` and `a_saturated_price_is_reported` ok. dp `place_does_not_settle` ok. |
| 10 | EM known/unknown/violated counts printed per fixture (REL T3/T4) | **met** | Bench `EM known K (viol V), unknown U, max use X` on every row (§4). Violations are 0 everywhere. Unknown is 1 on bgr_core, bjt_mirror and rc_filter. Totals: Electromigration 46 rows, 43 satisfied, 3 unknown. |
| 11 | No `drawn short` or `unresolved congestion` hidden in Θ (RTE T2) | **met as worded in the master plan; not met under plan-05's T2 / RTE-03 acceptance** | `grep -rn 'routing overuse'` over backend, frontend, kernel and benchmarks `*.rs` is empty. `dr::score` (backend/dr/src/lib.rs:1910–1945) puts residual overuse in V as `unresolved congestion` (margin = overuse). It puts shorts in V as `drawn short nets a/b` and `drawn short net n to cell metal` (foreign cell/ring metal, `cross_net_shorts`), and opens in V as `open net n` (`open_components`, layer-aware). `BLOCKED` (dr:41) makes foreign metal a hard obstacle, and `charge_rect` is gone. Tests: `a_route_touching_foreign_cell_metal_is_a_short`, `a_ring_band_is_a_hard_obstacle`, `an_xy_overlap_on_non_adjacent_layers_is_open` and `resolver_sacrifices_the_later_access_shape` ok. Bench winners: overuse 0 and route hard 0 on all 10, except dac4's in-loop `route/Antenna: net top 1.111` (target/bench_debug/dac4/violations.txt). **But** plan-05's M0 row (T2) reads "no `drawn short`/`unresolved congestion` V … on any bench circuit". 7 of dac4's 90 dr reports (non-winning) carry exactly those (§1), so that stricter target and RTE-03 Acceptance 3 are not met. RTE-03 itself awaits the decision in §1. |
| 12 | PLC baseline table and EXT corpus expectations recorded | **met** | docs/plans/baseline-plc-01.md: 10 fixtures × seeds 1–5. annotator `corpus_expectations` and `negative_corpus` ok. Corpus tests for later items are `#[ignore]` and name the item (§3). |
| 13 | `every_generator_enumerates_on_every_deck` green in debug on 4 decks; 0 inductor shapes in any output | **met** | `cargo test -p cells --test cell_selfcheck every_generator_enumerates_on_every_deck` (debug, `unoptimized + debuginfo`) ok. `Inductor::draw` returns an empty macro (kernel/cells/src/inductor.rs). `an_inductor_is_undrawable_without_a_recogniser`, `an_inductor_is_one_undrawable_finding` and `verify` `an_inductor_is_unverified` ok. No bench fixture has an `L` card. |

## 3. Test summary

`cargo build --release --workspace`: exit 0.

`cargo test --release --workspace --no-fail-fast`: exit 101. **435 passed, 2 failed, 7 ignored**, summed over every
`test result` line. Pass 1 had 430/2/7; the +5 are RTE-03's dr tests (dr 29 → 34). Doc-tests: 0.

Failing tests (the same two as pass 1, the same messages):

1. **`benchmark` `tests/signoff_fixtures.rs::fixtures_sign_off_within_baseline`** panicked at
   signoff_fixtures.rs:173: `dac4: ERC rows ["erc/ar.met2.1:gate"], expected []` (criterion 2; RTE-06).
2. **`library` `tests/hier_elaborate.rs::hierarchical_composition_elaborates_with_a_correct_schematic`** panicked at
   hier_elaborate.rs:216: `hierarchical layout must be LVS-clean against its schematic`. The rows are
   8 × `lvs/lvs.unpaired_device:-` and 17 × `lvs/lvs.unpaired_net:-`.
   - This is not a regression. FLOW-14 (`0ef8506`) replaced an assertion-free check, and its commit records the test
     red with these counts.
   - No M0 item fixes it, so it needs an owner.

Ignored (7), each with its reason in the attribute:

- `strongarm_matches_align_gold`: EXT-14.
- `coverage_is_total`: EXT-10.
- `negative_corpus_sc_switches_and_equal_fets`: EXT-04.
- `no_emitted_conflicts_strongarm`: EXT-14, REL-09.
- `permutation_invariance`: EXT-06.
- `twelve_thousand_devices`: EXT-06, T8.
- `large_fixtures_sign_off_within_baseline`: run separately in this pass, ok (criterion 2).

Extra runs:

- Release `antenna_in_loop… --nocapture`: ok.
- Debug `every_generator_enumerates_on_every_deck`: ok.
- Debug `library` `flow_smoke` 1/0/0, `placement_metrics` 2/0/0 and `antenna_diode` 1/0/0. In debug,
  `debug_check_joined` runs on every winner claiming 0 hard violations.

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
| library | `perf_postlayout` | 3/0/0 (ngspice ran, 34 s) |
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

The run exited 0 in 99.8 s wall. Load average was 9.4 at the start and 7.8 at the end, on 32 cores, with other
sessions running. 10/10 circuits were placed and routed. The only stderr was `[op] operating point unavailable (no
device operating points: ); continuing with zero power` (the BJT fixtures have no operating point).

| circuit | ms | cells / nets | WL nm | unrouted | route hard | overuse | DRC | LVS | ERC | warn | C Σ / sig fF | area µm² | util % | active % | best | outer, esc | bias | EM known (viol), unk, max use | usage | lattice off | clr residue nm² |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| bgr_core | 5 290 | 2 / 3 | 260 250 | 0 | 0 | 0 | 0 | PARTIAL(9) | 0 | 0 | 76.7 / 19.6 | 135.0 | 100.0 | 1.2 | 2/5 conv. | 1, 0 | none | 0 (0), 1, none | 1.000 | 0 | 0 |
| bjt_mirror | 4 746 | 2 / 5 | 73 480 | 0 | 0 | 0 | 0 | PARTIAL(2) | 0 | 0 | 33.0 / 31.3 | 64.8 | 79.4 | 1.8 | 1/5 conv. | 1, 0 | none | 0 (0), 1, none | 1.260 | 0 | 0 |
| chain4 | 4 498 | 4 / 7 | 44 675 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 22.6 / 8.4 | 62.6 | 100.0 | 19.4 | 1/5 conv. | 1, 0 | 0 µW probe | 5 (0), 0, 0.000 | 1.000 | 1 | 0 |
| dac4 | 13 317 | 14 / 12 | 640 880 | 0 | 1 | 0 | 0 | PARTIAL(16) | 0 | 0 | 297.2 / 147.8 | 2400.6 | 61.1 | 0.8 | 2/15 budget | 3, 2 | 112 µW probe | 11 (0), 0, 0.221 | 1.636 | 8 | 291 300 |
| ota | 19 699 | 5 / 9 | 497 540 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 588.4 / 263.8 | 1729.8 | 87.7 | 42.1 | 1/20 budget | 4, 3 | 2 µW probe | 6 (0), 0, 0.003 | 1.140 | 2 | 0 |
| ota_constrained | 19 789 | 5 / 9 | 497 540 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 588.4 / 263.8 | 1729.8 | 87.7 | 42.1 | 1/20 budget | 4, 3 | 2 µW probe | 6 (0), 0, 0.003 | 1.140 | 2 | 0 |
| pair | 4 206 | 2 / 3 | 19 905 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 4.8 / 3.1 | 8.6 | 100.0 | 48.4 | 1/5 conv. | 1, 0 | 0 µW probe | 3 (0), 0, 0.000 | 1.000 | 1 | 0 |
| quad | 4 271 | 4 / 3 | 37 945 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 9.3 / 6.1 | 15.9 | 100.0 | 51.2 | 5/5 conv. | 1, 0 | 0 µW probe | 3 (0), 0, 0.000 | 1.000 | 1 | 0 |
| rc_filter | 4 484 | 3 / 5 | 87 230 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 24.0 / 7.6 | 163.3 | 62.0 | 2.8 | 2/5 conv. | 1, 0 | 28 µW probe | 3 (0), 1, 0.055 | 1.613 | 2 | 0 |
| tt_ota | 19 340 | 5 / 9 | 497 540 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 588.4 / 263.8 | 1729.8 | 87.7 | 42.1 | 1/20 budget | 4, 3 | 2 µW probe | 6 (0), 0, 0.003 | 1.140 | 2 | 0 |

These columns are the same on every row: overlap 0 nm², matched mismatch 0, islands extra 0, dp temps 220,
decode fail 0, matched incompat 0.

dac4's `route hard 1` is the in-loop antenna rule on net `top` (`route/Antenna: net top 1.1108099`, max use 2.111).
Signoff passes that net at FEEDBACK_ITERS 5 (ERC 0) and fails it at 1 (criterion 2). The winner therefore has
|V| ≥ 1.

**dac4 since pass 1:** this is the only bench row that moved; the other 9 are identical apart from wall time. The
only code change in between is `ba49ae2`, and the epoch count is fixed (`budget` = 15 epochs, not a time budget), so
the move comes from RTE-03's dr. dac4's winner is a different layout:

| | pass 1 | pass 2 |
|---|---|---|
| WL nm | 691 870 | 640 880 |
| C Σ fF | 310.4 | 297.2 |
| C sig fF | 143.2 | 147.8 |
| area µm² | 2635.2 | 2400.6 |
| util % | 60.0 | 61.1 |
| lattice off | 6 | 8 |
| clr residue nm² | 0 | 291 300 |
| antenna max use | 2.025 | 2.111 |

- **clr residue:** clearance encroachment beyond plain overlap, `PlacementMetrics::clearance_residue_nm2`
  (frontend/library/src/geometry.rs:29). It is measured, never steered on, and PLC-01's baseline already shows it
  nonzero on dac4 seed 5.
- **Reproducibility:** the dac4 row reproduces the port session's own bench run (`scratchpad/bench_t2.out`: WL
  640 880, C 297.2 / 147.8).

Skipped rules (from `signoff.coverage.skipped_rules`):

- **Every fixture:** `lvs.device_count_mos`, `lvs.device_count_bjt` and `lvs.parametric` (`NotInDeck`);
  `multiple_drivers` (`Waived`); `m1.density`…`m4.density` (`ChipLevel`); `floating_gate` (exempt: no port list).
- **Per fixture:** the LU rules that have no layer (`EmptyLayer`).
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

Pass 1 to pass 2, all of it dac4:

- Utilization went from 9/10 to 10/10 (max use 1.000 → 0.982).
- Antenna max use went from 2.025 to 2.111.
- CouplingBudget max use went from 0.202 to 0.523.
- ParasiticBudget max use went from 0.936 to 0.824.
- IrDrop max use went from 0.131 to 0.125.
- The overall totals went from 232/5/3 to 233/4/3.

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
| bgr_core | 16 113 → 5 290 | 260 320 → 260 250 | 0 → 0 | MATCH¹ → PARTIAL(9) | 1 → 0 | 77.2 → 76.7 | 135.0 → 135.0 | 100.0 → 100.0 | 4/5 budget → 2/5 conv. |
| bjt_mirror | 12 300 → 4 746 | 74 085 → 73 480 | 0 → 0 | MATCH¹ → PARTIAL(2) | 1 → 0 | 33.7 → 33.0 | 64.7 → 64.8 | 79.4 → 79.4 | 3/5 budget → 1/5 conv. |
| chain4 | 11 647 → 4 498 | 44 675 → 44 675 | 0 → 0 | MATCH → MATCH | 0 → 0 | 22.6 → 22.6 | 62.6 → 62.6 | 100.0 → 100.0 | 1/5 → 1/5 |
| dac4 | 287 520 → 13 317 | 580 410 → 640 880 | **18 → 0** | MATCH⁵ → PARTIAL(16) | 0 → 0 | 275.8 → 297.2 | 2326.9 → 2400.6 | 60.0 → 61.1 | 5/15 → 2/15 budget |
| ota (×3, identical) | 52–59 k → 19–20 k | 444 720 → 497 540 | 0 → 0 | MATCH → MATCH | 0 → 0 | 616.0 → 588.4 | 1865.1 → 1729.8 | 85.9 → 87.7 | 1/20 conv. → 1/20 budget |
| pair | 11 468 → 4 206 | 17 700 → 19 905 | 0 → 0 | MATCH → MATCH | 0 → 0 | 4.6 → 4.8 | 8.6 → 8.6 | 100.0 → 100.0 | 4/5 → 1/5 |
| quad | 16 803 → 4 271 | 37 945 → 37 945 | 0 → 0 | MATCH → MATCH | 0 → 0 | 9.3 → 9.3 | 15.9 → 15.9 | 100.0 → 100.0 | 5/5 → 5/5 |
| rc_filter | 13 445 → 4 484 | 86 835 → 87 230 | 0 → 0 | MATCH → MATCH | 0 → 0 | 24.4 → 24.0 | 131.7 → 163.3 | 76.9 → 62.0 | 5/5 → 2/5 |

- **Wall time:** 539.5 s before, 99.8 s now. Both runs were under unknown or uneven load, and no item claims a
  speed-up. Per baseline-plc-01.md, the timings are for orientation only.
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
5. **The OTA CommonCentroid rows** are 0/3 since FLOW-01, pending `cellgen::folds` honouring CommonCentroid.
