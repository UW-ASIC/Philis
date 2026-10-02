# M0 report: verify pass 1

Exit-criteria check of milestone M0 ("Honest measurement", 00-MASTER-PLAN.md §5) on branch `m0` at `2f37409`
(M0 PERF-03: merge), 86 commits after `ddab38e`. Every number here was measured on 2026-10-02 in the integration
worktree, release profile unless stated: `cargo build --release --workspace`, `cargo test --release --workspace
--no-fail-fast`, `cargo run --release -p benchmark --bin bench local`, plus four targeted re-runs (named where used).
ngspice and the sky130 ngspice models (`~/.volare/sky130A`) were present, so the ngspice tests ran rather than skipped.
magic, netgen and klayout were not on PATH; no M0 test calls them.

**Verdict: M0 is not done.** 11 of the 13 criteria below are met, one of them (CI, #1) with a gap noted. The two
not met (#2 and #11) are the ones whose items were not merged (RTE-06, RTE-03). The suite has two red tests: the
dac4 test (#2) and `hier_elaborate`, which FLOW-14 made honest and which has been red ever since without an M0 owner.

## 1. Items

Merged (24): FLOW-14, GAP-20, FLOW-04, FLOW-05, FLOW-01, CELL-01, CELL-02, CELL-04, EXT-01, PLC-01, RTE-32, RTE-02,
FLOW-02, FLOW-03, PERF-01, PERF-02, PERF-04, PERF-06, PERF-07, PERF-03 (steps 1–2; its port step waits for FLOW-07,
M1), REL-01, REL-03, RTE-09, REL-02.

Not merged (2), both rejected at review:

- **RTE-03** (foreign metal hard; shorts, opens, congestion as V). Rejected for three reasons:
  1. Acceptance 3 is not met, and the deviation text understates the gap. It says dac4 produces `unresolved
     congestion` in one non-winning epoch. The implementer's own bench log (`rte03-h.err`) has 8 of 180 dac4 dr
     reports with it, spread across several epochs (margins 3, 4, 16, each paired with a `drawn short nets` V), and
     the round-2 run (`r2_dac4.err`) has the same 8. Merging needs an explicit orchestrator/user decision on the
     correct count and a per-epoch base/HEAD comparison.
  2. The branch does not merge cleanly with m0. There are conflicts in `backend/dr/src/lib.rs` (the
     `compact.is_empty()` return and the final `score()` call) and in `kernel/core/src/routes.rs` (`Terminal`,
     `Routes::terms`). m0's `DetailedRoute::route` now returns a 3-tuple, which breaks five of the item's dr tests.
  3. Minor: `resolver_sacrifices_the_later_access_shape` has no case where the earlier net's access shape is the
     one deleted.
- **RTE-06** (antenna repair on drawn geometry). Rejected because its own acceptance is red:
  `fixtures_sign_off_within_baseline` fails on that branch with the same `dac4: ERC rows ["erc/ar.met2.1:gate"]`.
  The cause given is plausible. `routing_stack` (frontend/library/src/elaborate.rs:211–220) gives dr only
  [met1, met2] on sky130, so a cap top plate on met2 that is over the limit before routing cannot be repaired inside
  dr. No code defect was found. Two dr changes were left undisclosed and untested:
  - a `frozen` set that excludes shielded nets from the post-shield antenna pass;
  - a rollback gate (`judge`) on redraws.

  The user still has to decide: merge RTE-06 as "acceptance not met" and move the dac4 fix to FLOW-05/FLOW-08 or to
  the routing stack / cap pin layer, or re-scope the item.

## 2. Exit criteria (00-MASTER-PLAN.md §5, M0)

| # | Criterion | Status | Evidence |
|---|---|---|---|
| 1 | CI runs `cargo test --workspace` on every push; release generator tests; nightly tool tests fail loudly when a tool is missing (FLOW T13) | **met, with a gap** | `.github/workflows/ci.yml`: the `test` job (`on: push`, `cargo test --workspace --locked --no-fail-fast`); `release` (`cell_selfcheck`, `ota_cross_pdk`); `nightly` (`PHILIS_REQUIRE_TOOLS=1`, `--include-ignored`, ngspice + sky130 via volare). `tool_or_skip_tells_present_from_missing` ok (printed `philis-no-such-binary-7f3a unavailable — skipping`). CI was not observed running because nothing is pushed. **Gap:** `library` unit test `fixture_nets_without_a_resistor_keep_em_sizing` (REL-01, frontend/library/src/lib.rs:1699–1701) skips with an `eprintln!` and returns, and it ignores `PHILIS_REQUIRE_TOOLS`. If the nightly job lacked ngspice or the models, that test would pass instead of failing loudly. Under T13 the suite is also not green (§3: two red tests). plan-08's M0 row expects only the dac4 test to be red. |
| 2 | `signoff_fixtures` green including dac4 at `feedback_iters = 1` (REL T1, RTE T11) | **not met** (RTE-06 not merged) | `fixtures_sign_off_within_baseline` FAILED at benchmarks/tests/signoff_fixtures.rs:173: `dac4: ERC rows ["erc/ar.met2.1:gate"], expected []`. The other 6 default fixtures: DRC 0, ERC 0, LVS clean. `large_fixtures_sign_off_within_baseline` (ota ×3) is `#[ignore]` and was not run in this pass. |
| 3 | `antenna_in_loop_never_passes_what_signoff_fails` green (REL T2) | **met** | ok. `--nocapture`: dac4 signoff `["erc/ar.met2.1:gate"]`, in-loop `(5 total, 4 satisfied, 0 unknown)`, so the in-loop model flags it. pair/quad/rc_filter/chain4 `(1, 1, 0)` and no signoff rows; bjt_mirror/bgr_core no rows either side. |
| 4 | Bench LVS reads `PARTIAL(2)` bjt_mirror, `PARTIAL(9)` bgr_core, `PARTIAL(16)` dac4, `MATCH` for the other 7 (PERF M0) | **met** | Bench table §4, LVS column, exactly as required. |
| 5 | Warnings printed and absent from \|V\| | **met** | Bench prints `warnings N` per circuit (0 on all 10). `start_tests::warnings_are_not_violations` ok; `v_counts_rules_not_batches`, `theta_counts_each_budget_once` and `nan_loses` ok. |
| 6 | FLOW T1 size fidelity: 0 devices | **met** | `size_tests::drawn_width_equals_simulated_width` ok. It checks all 36 MOS on all 10 fixtures, including `XM5 … W=40 L=2 nf=1 m=4`. `mos_size_follows_spice_semantics` and `per_finger_convention_stores_total_width` ok. |
| 7 | FLOW T3 deck honesty 0/0/0 | **met** | Not embedded: `every_builtin_pdk_loads` ok (no filesystem read) and `vendored_decks_name_their_origin` ok. Without `_source`: `a_process_number_needs_a_source` ok, and all four decks load through that check. Required keys unread: `generators_read_exactly_the_required_keys` ok (kernel/cells/tests/deck_keys.rs, sky130). `sidecar_rejects_a_misspelt_key` ok. |
| 8 | FLOW T4: LU rules present, no antenna sidewall off by > 5 % | **met** | `latch_up_rules_find_a_far_tap` ok. `antenna_sidewall_thickness_matches_pex` ok. pdks/decks/sky130.deck:517–519 has LU.2, LU.2.1 and LU.3. In the bench they run, or are listed as `Skipped(EmptyLayer)` where the fixture has no such diffusion (e.g. pair: LU.2.1, LU.3). |
| 9 | Exactly one dual step per epoch (FLOW T6) | **met** | `start_tests::one_dual_step_per_epoch` ok. gp `a_slack_budget_relaxes_its_price` and `a_saturated_price_is_reported` ok; dp `place_does_not_settle` ok. `rho_escalates_for_a_residual_that_does_not_shrink` and `drift_decreases_as_prices_settle` ok, unchanged. |
| 10 | EM known/unknown/violated counts printed per fixture (REL T3/T4) | **met** | Bench `EM known K (viol V), unknown U, max use X` on every row (§4). Violations are 0 everywhere. Unknown is 1 on bgr_core, bjt_mirror and rc_filter. Totals: Electromigration 46 rows, 43 satisfied, 3 unknown. |
| 11 | No `drawn short` or `unresolved congestion` hidden in Θ (RTE T2) | **not met** (RTE-03 not merged) | On m0, `dr::score` (backend/dr/src/lib.rs:1935–1937) still puts residual overuse into Θ as `routing overuse`, and dr has no foreign-metal (cell-metal) short check (AT-05). The bench winners happen to show overuse 0 and no `drawn short` on all 10. The only route V is dac4's `route/Antenna: net top 1.025` (target/bench_debug/dac4/violations.txt). The RTE-03 review found such findings in non-winning dac4 epochs, so this cannot be called met from the winners alone. |
| 12 | PLC baseline table and EXT corpus expectations recorded | **met** | docs/plans/baseline-plc-01.md (10 fixtures × seeds 1–5, `7019ee7`). annotator `corpus_expectations` and `negative_corpus` ok. Corpus tests for later items are `#[ignore]` with the item named (§3). |
| 13 | `every_generator_enumerates_on_every_deck` green in debug on 4 decks; 0 inductor shapes in any output | **met** | `cargo test -p cells --test cell_selfcheck every_generator_enumerates_on_every_deck` (debug, `unoptimized + debuginfo`) ok, decks sky130, gf180mcu, ihp_sg13g2, generic_finfet. `Inductor::draw` returns an empty macro and `enumerate` returns none (kernel/cells/src/inductor.rs). `an_inductor_is_undrawable_without_a_recogniser`, `an_inductor_is_one_undrawable_finding` and `verify` `an_inductor_is_unverified` ok. No bench fixture has an `L` card. |

## 3. Test summary

`cargo build --release --workspace`: exit 0, 0 warnings (incremental).

`cargo test --release --workspace --no-fail-fast`: exit 101. **430 passed, 2 failed, 7 ignored**, summed over every
`test result` line; doc-tests are 0.

Failing tests:

1. `benchmark` `tests/signoff_fixtures.rs::fixtures_sign_off_within_baseline` panicked at signoff_fixtures.rs:173:
   `dac4: ERC rows ["erc/ar.met2.1:gate"], expected []` (criterion 2; RTE-06).
2. `library` `tests/hier_elaborate.rs::hierarchical_composition_elaborates_with_a_correct_schematic` panicked at
   hier_elaborate.rs:216: `hierarchical layout must be LVS-clean against its schematic`, with 8 ×
   `lvs/lvs.unpaired_device:-` and 17 × `lvs/lvs.unpaired_net:-`. This is not a regression. FLOW-14 (`0ef8506`)
   replaced an assertion-free check with "no `lvs` row", and the commit message records it red with these same
   counts. It has stayed red on every chain's runs since. It needs an owner; no M0 item fixes it.

Ignored (7), each with its reason in the attribute:

- `strongarm_matches_align_gold`: EXT-14.
- `coverage_is_total`: EXT-10.
- `negative_corpus_sc_switches_and_equal_fets`: EXT-04.
- `no_emitted_conflicts_strongarm`: EXT-14, REL-09.
- `permutation_invariance`: EXT-06.
- `twelve_thousand_devices`: EXT-06, T8.
- `large_fixtures_sign_off_within_baseline`: OTA-class full flow, `--ignored`.

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
| dr | unit tests | 29/0/0 |
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
| library | `perf_postlayout` | 3/0/0 (ngspice ran, 32 s) |
| library | `placement_metrics` | 2/0/0 |
| macro_master | unit tests | 9/0/0 |
| pnr_core | unit tests | 16/0/0 |
| verify | unit tests | 39/0/0 |
| visualizer | unit tests | 2/0/0 |

## 4. Bench (`bench local`, seed 1, sky130, FEEDBACK_ITERS 5)

The run exited 0 in 120.9 s wall, with load average 5.5–6.1 on 32 cores (other sessions were running). 10/10
circuits were placed and routed. The only stderr was `[op] operating point unavailable (no device operating points:
); continuing with zero power`, twice (bgr_core and bjt_mirror; there is no BJT operating point).

| circuit | ms | cells / nets | WL nm | unrouted | route hard | overuse | DRC | LVS | ERC | warn | C Σ / sig fF | area µm² | util % | active % | best | outer, esc | bias | EM known (viol), unk, max use | lattice off |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| bgr_core | 4 523 | 2 / 3 | 260 250 | 0 | 0 | 0 | 0 | PARTIAL(9) | 0 | 0 | 76.7 / 19.6 | 135.0 | 100.0 | 1.2 | 2/5 conv. | 1, 0 | none | 0 (0), 1, none | 0 |
| bjt_mirror | 4 191 | 2 / 5 | 73 480 | 0 | 0 | 0 | 0 | PARTIAL(2) | 0 | 0 | 33.0 / 31.3 | 64.8 | 79.4 | 1.8 | 1/5 conv. | 1, 0 | none | 0 (0), 1, none | 0 |
| chain4 | 4 214 | 4 / 7 | 44 675 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 22.6 / 8.4 | 62.6 | 100.0 | 19.4 | 1/5 conv. | 1, 0 | 0 µW probe | 5 (0), 0, 0.000 | 1 |
| dac4 | 40 391 | 14 / 12 | 691 870 | 0 | 1 | 0 | 0 | PARTIAL(16) | 0 | 0 | 310.4 / 143.2 | 2635.2 | 60.0 | 0.6 | 2/15 budget | 3, 2 | 112 µW probe | 11 (0), 0, 0.221 | 6 |
| ota | 17 119 | 5 / 9 | 497 540 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 588.4 / 263.8 | 1729.8 | 87.7 | 42.1 | 1/20 budget | 4, 3 | 2 µW probe | 6 (0), 0, 0.003 | 2 |
| ota_constrained | 17 462 | 5 / 9 | 497 540 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 588.4 / 263.8 | 1729.8 | 87.7 | 42.1 | 1/20 budget | 4, 3 | 2 µW probe | 6 (0), 0, 0.003 | 2 |
| pair | 4 151 | 2 / 3 | 19 905 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 4.8 / 3.1 | 8.6 | 100.0 | 48.4 | 1/5 conv. | 1, 0 | 0 µW probe | 3 (0), 0, 0.000 | 1 |
| quad | 4 149 | 4 / 3 | 37 945 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 9.3 / 6.1 | 15.9 | 100.0 | 51.2 | 5/5 conv. | 1, 0 | 0 µW probe | 3 (0), 0, 0.000 | 1 |
| rc_filter | 4 270 | 3 / 5 | 87 230 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 24.0 / 7.6 | 163.3 | 62.0 | 2.8 | 2/5 conv. | 1, 0 | 28 µW probe | 3 (0), 1, 0.055 | 2 |
| tt_ota | 20 314 | 5 / 9 | 497 540 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 588.4 / 263.8 | 1729.8 | 87.7 | 42.1 | 1/20 budget | 4, 3 | 2 µW probe | 6 (0), 0, 0.003 | 2 |

Further columns, the same on every row: overlap 0 nm², clr residue 0 nm², matched mismatch 0, islands extra 0, dp
temps 220. Usage: bgr 1.000, bjt 1.260, chain4 1.000, dac4 1.667, OTAs 1.140, pair 1.000, quad 1.000, rc 1.613.

dac4's `route hard 1` is the in-loop antenna rule on net `top` at use 2.025 (`route/Antenna: net top 1.0249016`).
Signoff passes that net at FEEDBACK_ITERS 5 (ERC 0) and fails it at 1 (criterion 2). The winner therefore has
|V| ≥ 1.

Skipped rules (from `signoff.coverage.skipped_rules`):

- **Every fixture:** `lvs.device_count_mos`, `lvs.device_count_bjt` and `lvs.parametric` (`NotInDeck`);
  `multiple_drivers` (`Waived`); `m1.density`…`m4.density` (`ChipLevel(window 700000 nm > block …)`);
  `floating_gate` (exempt: no port list).
- **Per fixture**, the LU rules that have no layer (`EmptyLayer`).
- **bgr_core and bjt_mirror only:** `ir_drop`, `esd.pad`, the `vgs/vds` ratings and `EM.met*_via*`
  (`NoDesignIntent`, unbiased).

Constraint satisfaction, verbatim totals:

| type | arm | total | sat | viol | unk | max use (at) | Θ |
|---|---|---|---|---|---|---|---|
| Antenna | hard | 21 | 20 | 1 | 0 | 2.025 (dac4) | 1.025 |
| CommonCentroid | budget | 3 | 0 | 3 | 0 | 2.195 (tt_ota) | 3.584 |
| CommonNode | budget | 6 | 6 | 0 | 0 | 0.001 (tt_ota) | 0 |
| CouplingBudget | budget | 39 | 39 | 0 | 0 | 0.202 (dac4) | 0 |
| CrosstalkExclusion | budget | 12 | 12 | 0 | 0 | 0.475 (tt_ota) | 0 |
| Differential | hard | 3 | 3 | 0 | 0 | – | 0 |
| Electromigration | hard | 46 | 43 | 0 | 3 | 0.221 (dac4) | 0 |
| Environment | budget | 6 | 6 | 0 | 0 | 0.320 (tt_ota) | 0 |
| IrDrop | budget | 23 | 23 | 0 | 0 | 0.131 (dac4) | 0 |
| MatchingPair | budget | 6 | 6 | 0 | 0 | 0.000 (tt_ota) | 0 |
| ParasiticBudget | budget | 39 | 39 | 0 | 0 | 0.936 (dac4) | 0 |
| Proximity | budget | 11 | 11 | 0 | 0 | 0.255 (tt_ota) | 0 |
| Symmetry | hard | 9 | 9 | 0 | 0 | – | 0 |
| ThermalGradient | budget | 6 | 6 | 0 | 0 | 0.000 (tt_ota) | 0 |
| Utilization | budget | 10 | 9 | 1 | 0 | 1.000 (dac4) | 0 |
| **OVERALL** | | 240 | 232 | 5 | 3 | | rate 98 % |

## 5. Against the pre-M0 ground truth (audit-06 §G)

**Tests:**

| | pre-M0 (G.2) | now |
|---|---|---|
| Result | 331 passed, 1 failed, 2 ignored, exit 101 | 430 passed, 2 failed, 7 ignored, exit 101 |
| Failing | dac4 `erc/ar.met2.1:gate`, the same rule as before | dac4 (same rule) and `hier_elaborate` |
| Ignored | 2: `large_fixtures…`, `tmp_deck_drc::dump` | 7 (EXT corpus placeholders; `tmp_deck_drc.rs` was deleted by FLOW-14) |

`hier_elaborate` was "passing" before only because it asserted nothing (FLOW-14).

**`signoff_fixtures` per fixture:**

- DRC 0 everywhere, before and now.
- ERC before was bjt_mirror 1, bgr_core 1, dac4 1. Now only dac4 has 1; the `supply_short` false positive is gone
  (GPurify 4ef439d).
- PEX is unchanged on six fixtures. dac4 went from 326.0 to 280.8 fF.
- LVS: bjt_mirror and bgr_core no longer read "clean" with nothing compared; they report 2 and 9 unverified. dac4's
  15 dropped caps are now 16 unverified (Σm).

**Bench, G.3 → now:**

| circuit | ms | WL nm | overuse | LVS | ERC | C Σ fF | area µm² | util % | best |
|---|---|---|---|---|---|---|---|---|---|
| bgr_core | 16 113 → 4 523 | 260 320 → 260 250 | 0 → 0 | MATCH¹ → PARTIAL(9) | 1 → 0 | 77.2 → 76.7 | 135.0 → 135.0 | 100.0 → 100.0 | 4/5 budget → 2/5 conv. |
| bjt_mirror | 12 300 → 4 191 | 74 085 → 73 480 | 0 → 0 | MATCH¹ → PARTIAL(2) | 1 → 0 | 33.7 → 33.0 | 64.7 → 64.8 | 79.4 → 79.4 | 3/5 budget → 1/5 conv. |
| chain4 | 11 647 → 4 214 | 44 675 → 44 675 | 0 → 0 | MATCH → MATCH | 0 → 0 | 22.6 → 22.6 | 62.6 → 62.6 | 100.0 → 100.0 | 1/5 → 1/5 |
| dac4 | 287 520 → 40 391 | 580 410 → 691 870 | **18 → 0** | MATCH⁵ → PARTIAL(16) | 0 → 0 | 275.8 → 310.4 | 2326.9 → 2635.2 | 60.0 → 60.0 | 5/15 → 2/15 budget |
| ota (×3, identical) | 52–59 k → 17–20 k | 444 720 → 497 540 | 0 → 0 | MATCH → MATCH | 0 → 0 | 616.0 → 588.4 | 1865.1 → 1729.8 | 85.9 → 87.7 | 1/20 conv. → 1/20 budget |
| pair | 11 468 → 4 151 | 17 700 → 19 905 | 0 → 0 | MATCH → MATCH | 0 → 0 | 4.6 → 4.8 | 8.6 → 8.6 | 100.0 → 100.0 | 4/5 → 1/5 |
| quad | 16 803 → 4 149 | 37 945 → 37 945 | 0 → 0 | MATCH → MATCH | 0 → 0 | 9.3 → 9.3 | 15.9 → 15.9 | 100.0 → 100.0 | 5/5 → 5/5 |
| rc_filter | 13 445 → 4 270 | 86 835 → 87 230 | 0 → 0 | MATCH → MATCH | 0 → 0 | 24.4 → 24.0 | 131.7 → 163.3 | 76.9 → 62.0 | 5/5 → 2/5 |

- **Wall time:** 539.5 s before, 120.9 s now. Both were measured under unknown or uneven load, and no item claims a
  speed-up; per baseline-plc-01.md, timings here are for orientation only and are not T7's baseline.
- **dac4 cell count:** 15 before, 14 now. The 15th was a flow-inserted antenna diode.
- **The table's three changes:**
  1. Every "MATCH" that compared nothing now reads PARTIAL(n) (PERF-02).
  2. The ERC `supply_short` false positives are gone.
  3. dac4's overuse is 0.
- **Moves explained in the merge commits:**
  - **Epoch selection:** the winner and C changed on several fixtures, e.g. pair moved from epoch 4/5 to 1/5 with
    WL 17 700 → 19 905 nm. FLOW-02 (`lex_key`), FLOW-03 and PERF-07 (signal-C tier) change which epoch wins.
  - **Device sizes:** FLOW-01. It moved ota WL, area and C, and the OTA CommonCentroid rows.

**Constraint table:**

- Totals: 241/237/2 viol/2 unk before; 240/232/5 viol/3 unk now. IrDrop rows went from 24 to 23.
- **CommonCentroid** went from 3/3 satisfied (max use 0.000) to 0/3 (max use 2.195, Θ 3.584). The FLOW-01 merge
  commit records this as the honest result of drawing the simulated size: XM1/XM2 (W=10u nf=2) now fold to two
  5 µm fingers each, placed AABB. `cellgen::folds` picks k from parity and aspect only and ignores CommonCentroid.
  That is follow-up work, not fixed in M0.
- **Antenna** went from 21/21 to 20/21, with dac4 `top` at 2.025 (above).
- **Electromigration** unknown went from 2 to 3, the new one being rc_filter. This pass did not trace which item
  moved it or the IrDrop count; REL-01 ("unknown is not zero") and REL-03 both touch these rows.
- **Skip log:**
  - The four `m*.density` rules were before "run" over zero windows (AV-18). They are now listed `ChipLevel` (PERF-04).
  - Whether EM ran is now printed per fixture (AV-26 closed).
  - LU rules now exist and run (FLOW-05).

## 6. Open, for the orchestrator/user

1. RTE-03 and RTE-06 need the decisions described in §1 before M0 can exit. Criteria 2 and 11 depend on them.
2. `hier_elaborate` (8 unpaired devices, 17 unpaired nets) is red with no owner in M0. plan-08's M0 row expects only
   the dac4 test to be red.
3. `fixture_nets_without_a_resistor_keep_em_sizing` should go through the `PHILIS_REQUIRE_TOOLS` skip that
   perf_postlayout uses, so the nightly job cannot pass it when a tool is missing. This is out of scope for this pass;
   no code was changed.
4. The OTA CommonCentroid rows are 0/3 since FLOW-01, pending `cellgen::folds` honouring CommonCentroid.
