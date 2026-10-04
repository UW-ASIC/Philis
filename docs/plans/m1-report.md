# M1 report: verify pass

This pass checks every M1 exit criterion (00-MASTER-PLAN.md §5 "### M1") on branch `m1a` at `f5fd676` (M1
placement merge). That commit contains all six module batches (input, annotator, matching, cells, placement, routing)
and M1a's EXT-02 and EXT-11. Every number below was measured on 2026-10-04 in the integration worktree, release
profile, with `PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk`:

- `cargo build --release --workspace` (exit 0)
- `cargo test --release --workspace --no-fail-fast` (exit 0)
- `cargo test --release -p benchmark --test signoff_fixtures -- --ignored` (OTA-class fixtures)
- `cargo test -p gp -p dp -p pnr_core` (debug)
- `cargo run --release -p benchmark --bin bench local` (seed 1, exit 0, 14/14 placed and routed)
- `philis run <netlist> sky130 -o DIR` on `tq_chain`, `dac4_mim` and `benchmarks/field/pwm_driver.spice`

No code was changed in this pass.

**Verdict: M1 is not done.** 5 of 8 criterion groups are met. Three are not met:
- MAT: HPWL and footprint are not within ±5 % of M0.
- CELL: `mirror_ratio` is not merged into one cell.
- Field report 01: `tq_chain` through the CLI has 1 DRC row, and strongarm was not measured.

PLC is met at seed 1 on `m1a` and at seeds 1–5 on `m1a-placement`. PLC-06 waits on an owner decision (§1). The
OTA-class fixtures have regressed badly since M0 (§4): WL +21.8 %, area +15.7 %, overuse 0 → 1458,
CrosstalkExclusion 12/12 → 0/12.

## 1. Items

All of these are merged into `m1a`, and each one's commits are named in its card file (`docs/plans/cards/m1-*.md`).

| Batch | Item | Outcome |
|---|---|---|
| input | FLOW-07, PERF-03, PERF-08, PERF-09, PERF-30 | done |
| input | FLOW-13 | done; uses card step 2 (history on foreign nodes touched by a jog) because step 1 regressed dac4; `hier_elaborate` green |
| annotator | GAP-04, EXT-03, EXT-04, EXT-05, EXT-06, EXT-07, EXT-09, EXT-10, GAP-10 | done; recorded departures: EXT-05 2-device kind from `roles_of`; EXT-06 catalog edits; EXT-10 `Rule::touches`, FET-only W/L, u8 percentages |
| M1a | EXT-02, EXT-11 | done (`ca3e509`, `ccf66a7`) |
| matching | MAT-01, MAT-02, MAT-03, MAT-05, MAT-06 | done |
| matching | MAT-04 | done; its own commit records the ±5 % band failing on the OTA trio (WL −13.3 %) |
| cells | CELL-03, GAP-05, CELL-30, CELL-06, CELL-07, CELL-09, CELL-10 | done |
| cells | CELL-08 | done; step 7 (`"cap"` alias) skipped on purpose; `dac4_mim`/`tq_chain` have no `BASELINE` row |
| cells | FLOW-16 | done; MatchedSet 2/2 on the OTAs, but see §4 (WL and overuse) |
| placement | PLC-02, PLC-04, PLC-03, PLC-10 (step 0) | done |
| placement | PLC-06 | **escalated.** The code is in (`c29b238`) and its tests pass. Bench B shows the OTA key tier median at 265.0 → 265.3 fF (+0.1 %), with hard and area unchanged (`d648262`). **Owner decision:** accept this as noise, or ask for a fix. |
| routing | RTE-05, RTE-07, RTE-08 | done |
| M0 carry | FLOW-17 (fix-export, GPurify `6341f18`) | on `m1a` via `2a09c30` |
| M0 carry | RTE-06 | not needed: dac4 signs off ERC 0 at `feedback_iters = 1` (`fixtures_sign_off_within_baseline`) |
| M0 carry | RTE-03 | `ba49ae2` is still on the branch; the keep-or-revert owner decision is not recorded |

## 2. Exit criteria

| Group | Criterion | Status | Evidence |
|---|---|---|---|
| EXT | `negative_corpus` (T3); latch 2 DiffPair; `bjt_mirror` collectors `Signal`; 0 `Differential` hard; `permutation_invariance`; T6 = 0 incl. strongarm; T7; T8 | met | `negative_corpus`, `negative_corpus_sc_switches_and_equal_fets`, `permutation_invariance`, `no_emitted_conflicts`, `no_emitted_conflicts_strongarm`, `coverage_is_total`, `corpus_expectations` all pass; the latch row expects `DiffPair` MN1/MN2 and MP1/MP2; `classify.rs` asserts `outn`/`outp` `Signal`; bench table `Differential budget 3`, no hard row; `twelve_thousand_devices` 0.11 s (≤ 2.0 s) |
| MAT | `every_bjt_ratio_is_common_centroid`; `MatchingPair`/`ThermalGradient`/`CentroidGroup` deleted; `Orientation` 0 violations; `MatchedSet` 0 unknown; HPWL and footprint ±5 % of M0 | **not met** | The first four hold: the test passes; `git grep -w` finds only a doc comment (gp lib.rs:769) and a string in a bench test; Orientation is hard 8/8 and budget 8/8; MatchedSet is 8/8 with 0 unk. The band fails. ota/ota_constrained/tt_ota WL is 497 540 → 606 120 nm (+21.8 %) and area 1729.8 → 2001.1 µm² (+15.7 %). quad is 37 945 → 63 425 nm (+67 %) and 15.9 → 33.7 µm². bgr_core area is 135.0 → 422.7 µm². |
| CELL | rc_filter, res_m2, dac4_mim (16 caps) LVS MATCH; R within 0.5 %; MIM unit 53 282 aF ± 10; diode Σ area; bgr_core Q1 3×3 centre, 9 cards; mirror_ratio merged in one cell, residual 0 | **not met** | All hold except mirror_ratio. LVS is MATCH on all three in the bench, and the `dac4_mim` CLI reference netlist has 16 `cap_mim_m3_1` cards with signoff CLEAN. These pass: `segments_preserve_the_model_value`, `a_multiplied_resistor_draws_parallel_strings`, `the_mim_unit_is_the_model_s`, `a_multiplied_diode_draws_its_area`, `every_bjt_ratio_is_common_centroid`, `lone_bjt_draws_as_many_units_as_reference_cards`. bgr_core is LVS PARTIAL(9), meaning 9 BJT units are counted. **mirror_ratio: bench `3 cells` for 3 devices**, so the mirror is not merged (the card expects devices − 2 = 1), and its MatchedSet max use is 0.234. |
| PLC | `lattice_off == 0` and `matched_geometry_mismatch == 0` every fixture and seed; no placement panic in debug | met (scope noted) | On `m1a`, seed 1: lattice off 0 and matched mismatch 0 on all 14 fixtures. On `m1a-placement`, seeds 1–5 × 10 fixtures: 50/50 rows lattice off 0 (PLC-03's bench B). Seeds 2–5 were not re-run on `m1a`. Debug `cargo test -p gp -p dp -p pnr_core` gives 63 passed with no panic; a full flow in debug was not run. |
| PERF | `BiasSummary.resolved == devices` on rc_filter, bjt_mirror; no `bsim4v5.out` in cwd; OTA new header 0 / old header 2 `floating_gate` | met | `fixtures_resolve_every_device`, `extract_leaves_nothing_in_the_cwd` and `ota_old_header_flags_its_undriven_bias_gates` pass. Bench bias: rc_filter 28 µW, bjt_mirror 207 µW. No `bsim4v5.out` newer than this run in the worktree or the CLI cwd (the one in the worktree root is from 2026-10-03 15:23, gitignored). |
| RTE | `GlobalRoute` absent; `kind()` grep empty; bench DRC 0, LVS as M0 or better; 0 dac4 `unresolved congestion` while `ba49ae2` stays | met | Both greps are empty. DRC is 0 on 14/14, and LVS on the 10 M0 rows is identical (bgr_core PARTIAL(9), bjt_mirror PARTIAL(2), dac4 PARTIAL(16), the rest MATCH). dac4's score calls with congestion were 0/45 (RTE-08 `a192c02`, routing branch, not re-counted on `m1a`). dac4's only route-hard row here is the in-loop antenna on `top`. |
| Carried M0 | `signoff_fixtures` green incl. dac4 at `feedback_iters = 1`; `cargo test --release --workspace` 0 failed; OTA `CommonCentroid` rows 3/3 | met | `fixtures_sign_off_within_baseline` passes, including dac4 with ERC `[]`, and so does `large_fixtures_sign_off_within_baseline`. The workspace has 592 passed, 0 failed, 2 ignored, and `hier_elaborate` passes. MAT-04 replaced CommonCentroid with `MatchedSet`, which is 8/8 satisfied bench-wide. FLOW-16 recorded 2/2 per OTA fixture; the per-fixture split was not re-read here. |
| Field report 01 | tq_chain DRC 0, no `lvs/`, no `drawn short`; strongarm and pwm_driver 0 `lvs.parameter_mismatch`; pwm_driver no `label short`; 0.42/64.8 µm PFET ≤ 73 µm; `post_layout_rc_filter_simulates` | **not met** | **tq_chain:** the bench (FI 5) gives DRC 0, LVS MATCH and route hard 0, but ERC 1 (`erc/ar.met3.1:gate 847`). The CLI run's signoff has **1 hard row, `drc/m1.2.notch:met1 35`**. **strongarm:** no layout fixture exists, so it was not measured. **pwm_driver** (CLI): signoff CLEAN, 0 hard rows, no `label short`. `a_long_gate_keeps_short_dummies` (asserts `bbox.w <= 73_000`) and `post_layout_rc_filter_simulates` pass. |

## 3. Test summary

`cargo test --release --workspace --no-fail-fast`: **592 passed, 0 failed, 2 ignored**. No log line says `skipping`
or `unavailable`.

The 2 ignored tests:
- `align_gold::strongarm_matches_align_gold` ("passes after EXT-14", from M0)
- `signoff_fixtures::large_fixtures_sign_off_within_baseline` (run separately with `--ignored`: 1 passed)

Debug `cargo test -p gp -p dp -p pnr_core`: 63 passed, 0 failed. No test fails.

## 4. Bench (`bench local`, seed 1, sky130, PDK_ROOT set)

Exit 0. 14/14 circuits placed and routed; 4 are new since M0: dac4_mim, mirror_ratio, res_m2, tq_chain. Same on every
row: overlap 0 nm², clearance residue 0, matched mismatch 0, matched incompat 0, warnings 0, dp temps 220.

| circuit | ms | cells / nets | WL nm | unrouted | route hard | overuse | DRC | LVS | ERC | C Σ / sig fF | area µm² | best | bias | EM known (viol), unk | lattice off |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| bgr_core | 5864 | 2 / 3 | 311230 | 0 | 0 | 0 | 0 | PARTIAL(9) | 0 | 244.6 / 83.3 | 422.7 | 4/5 converged | none | 0 (0), 1 | 0 |
| bjt_mirror | 4222 | 2 / 5 | 94645 | 0 | 0 | 0 | 0 | PARTIAL(2) | 0 | 78.6 / 69.9 | 104.3 | 3/5 converged | 207 µW | 1 (0), 0 | 0 |
| chain4 | 4205 | 4 / 7 | 48500 | 0 | 0 | 0 | 0 | MATCH | 0 | 23.9 / 8.9 | 62.7 | 1/5 converged | 0 µW | 5 (0), 0 | 0 |
| dac4 | 8314 | 14 / 12 | 503045 | 0 | 1 | 0 | 0 | PARTIAL(16) | 0 | 233.7 / 115.5 | 1619.5 | 3/15 budget | 112 µW | 11 (0), 0 | 0 |
| dac4_mim | 8769 | 14 / 12 | 498790 | 0 | 1 | 0 | 0 | MATCH | 0 | 367.6 / 220.1 | 3692.4 | 3/15 budget | 112 µW | 11 (0), 0 | 0 |
| mirror_ratio | 5021 | 3 / 4 | 159345 | 0 | 0 | 0 | 0 | MATCH | 0 | 79.3 / 34.5 | 176.9 | 2/5 converged | none | 0 (0), 2 | 0 |
| ota | 62799 | 5 / 9 | 606120 | 0 | 0 | 1458 | 0 | MATCH | 0 | 804.1 / 390.1 | 2001.1 | 1/20 budget | 2 µW | 6 (0), 0 | 0 |
| ota_constrained | 63028 | 5 / 9 | 606120 | 0 | 0 | 1458 | 0 | MATCH | 0 | 804.1 / 390.1 | 2001.1 | 1/20 budget | 2 µW | 6 (0), 0 | 0 |
| pair | 4192 | 2 / 3 | 19970 | 0 | 0 | 0 | 0 | MATCH | 0 | 4.9 / 3.3 | 8.6 | 1/5 converged | 0 µW | 3 (0), 0 | 0 |
| quad | 4298 | 4 / 3 | 63425 | 0 | 0 | 0 | 0 | MATCH | 0 | 15.0 / 9.0 | 33.7 | 2/5 budget | 0 µW | 3 (0), 0 | 0 |
| rc_filter | 4548 | 3 / 5 | 80920 | 0 | 0 | 0 | 0 | MATCH | 0 | 23.3 / 7.6 | 131.8 | 5/5 converged | 28 µW | 4 (0), 0 | 0 |
| res_m2 | 4729 | 3 / 5 | 100995 | 0 | 0 | 0 | 0 | MATCH | 0 | 33.2 / 13.8 | 343.8 | 5/10 budget | none | 0 (0), 4 | 0 |
| tq_chain | 43300 | 24 / 11 | 1321330 | 0 | 0 | 0 | 0 | MATCH | 1 | 1274.6 / 576.3 | 9682.4 | 5/20 budget | 38 µW | 11 (0), 0 | 0 |
| tt_ota | 62807 | 5 / 9 | 606120 | 0 | 0 | 1458 | 0 | MATCH | 0 | 804.1 / 390.1 | 2001.1 | 1/20 budget | 2 µW | 6 (0), 0 | 0 |

**Against M0 §4 (the 10 shared rows):**
- **Unchanged:** DRC, LVS, unrouted and route hard (dac4's 1 is the in-loop antenna on `top`, as at M0).
- **Better:** lattice off is 0 everywhere (M0: chain4 1, dac4 8, OTAs 2, rc_filter 2, pair 1, quad 1). dac4's
  clearance residue is 291 300 → 0 nm², its WL 640 880 → 503 045 and its area 2400.6 → 1619.5. bjt_mirror bias is
  none → 207 µW (PERF-08). rc_filter's EM is 3 known / 1 unknown → 4 / 0.
- **Worse:**
  - The OTA trio: WL 497 540 → 606 120 (+21.8 %), area 1729.8 → 2001.1 (+15.7 %), overuse 0 → 1458, C sig
    263.8 → 390.1 fF (+48 %), ms ~28 s → ~63 s.
  - quad: WL +67 %, area 15.9 → 33.7, util 100 → 58.5 %, Utilization max use 1.026.
  - bgr_core: area 135.0 → 422.7, WL +19.6 %.
  - bjt_mirror: area 64.8 → 104.3.

**Where the OTA regression came from** (earlier seed-1 logs in the session scratchpad, not re-run here):
- Before FLOW-16 (cells branch): WL 431 145, overuse 0, area 1723.0.
- FLOW-16 alone (`ota_after.log`): WL 603 420, **overuse 472**, area 1860.7. The cells card records only the area.
- `m1a-placement` HEAD without the cells merge: WL 412 575, overuse 0.
- So FLOW-16's fold classes introduce the overuse, and the combined branch makes it worse (1458, area 2001.1).

Overuse is nonzero while route hard is 0 and DRC 0. This needs an owner. It is a later milestone's work, or a FLOW-16
follow-up; it is not fixed here.

Constraint totals, verbatim:

| type | arm | total | sat | viol | unk | rate | max use (at) | Θ |
|---|---|---|---|---|---|---|---|---|
| Antenna | hard | 36 | 34 | 2 | 0 | 94 % | 4.362 (dac4_mim) | 4.387 |
| CommonNode | budget | 8 | 6 | 0 | 2 | 100 % | 0.000 (tt_ota) | 0 |
| CouplingBudget | budget | 51 | 51 | 0 | 0 | 100 % | 0.201 (dac4_mim) | 0 |
| CrosstalkExclusion | budget | 12 | 0 | 12 | 0 | 0 % | 2.667 (tt_ota) | 4.374 |
| Differential | budget | 3 | 3 | 0 | 0 | 100 % | – | 0 |
| Electromigration | hard | 74 | 67 | 0 | 7 | 100 % | 0.793 (bjt_mirror) | 0 |
| Environment | budget | 8 | 8 | 0 | 0 | 100 % | 0.534 (tt_ota) | 0 |
| IrDrop | budget | 30 | 30 | 0 | 0 | 100 % | 0.272 (bjt_mirror) | 0 |
| MatchedSet | budget | 8 | 8 | 0 | 0 | 100 % | 0.234 (mirror_ratio) | 0 |
| Orientation | budget | 8 | 8 | 0 | 0 | 100 % | – | 0 |
| Orientation | hard | 8 | 8 | 0 | 0 | 100 % | – | 0 |
| ParasiticBudget | budget | 51 | 51 | 0 | 0 | 100 % | 0.554 (dac4_mim) | 0 |
| Proximity | budget | 20 | 20 | 0 | 0 | 100 % | 0.254 (tt_ota) | 0 |
| Symmetry | hard | 10 | 10 | 0 | 0 | 100 % | – | 0 |
| Utilization | budget | 14 | 13 | 1 | 0 | 93 % | 1.026 (quad) | 0.026 |

Against M0:
- Removed by MAT-04: CommonCentroid (0/3), MatchingPair and ThermalGradient. Added: MatchedSet and Orientation.
- Differential moved from the hard arm to budget (EXT-09).
- **CrosstalkExclusion fell from 12/12 to 0/12** (the OTA trio, max use 2.667).
- The 2 Antenna violations are dac4 and dac4_mim (in-loop, net `top`). Signoff passes both at FI 5.

## 5. Open, for the orchestrator/owner

1. **PLC-06:** accept the +0.1 % OTA key-tier change as noise, or ask for a fix.
2. **OTA-class regression** (§4): FLOW-16 adds overuse 472 on its own; merged, it is overuse 1458, WL +21.8 %, area
   +15.7 % and CrosstalkExclusion 0/12. This blocks the MAT ±5 % criterion.
3. **mirror_ratio is not merged** (3 cells for 3 devices). CELL-10's own acceptance (cells == devices − 2) is not met
   on the merged branch. Check whether the annotator splits the mirror (EXT-15, per the card).
4. **tq_chain:** the CLI run has `drc/m1.2.notch:met1`, and the bench has ERC `erc/ar.met3.1:gate`. dac4_mim and
   tq_chain still have no `BASELINE` row (CELL-08).
5. **strongarm** `lvs.parameter_mismatch` (field report 01) is unmeasured: there is no strongarm layout fixture.
6. **Out of scope, seen in the bench:**
   - mirror_ratio's op point fails: BSIM4 fatal on `xm3`, parameter check, `Drout`/`Eta0` negative.
   - res_m2's op point fails: ngspice cannot find `sky130_fd_pr__res_high_po`.
   - Both run with zero power (bias none).
   - bgr_core's PNP has no ngspice subckt, as at M1a.
7. **RTE-03:** the owner's keep-or-revert decision on `ba49ae2` is still not recorded.
