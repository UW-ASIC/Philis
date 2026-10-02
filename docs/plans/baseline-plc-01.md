# PLC-01 placement baseline (M0)

The numbers `bench local` printed for the 10 fixtures in `benchmarks/fixtures/` × seeds 1–5, on the code of the
commit that adds this file: `m0-baselines` at `7019ee7` (m0 at `5478230`, through FLOW-01, FLOW-03 and PERF-01,
merged into PLC-01) plus this commit's review fixes, which change no flow code (a test, a doc comment, the
`three_stage_opamp` print). They are the **baseline** for plan-04 T2 (area usage), T6 (`route_hard + route_overuse`)
and T7's proposal counts, and are not to be edited after the fact; a later item records its own numbers beside them. An
earlier table, measured on m0 at `6da8068` + PLC-01, was replaced before merge: FLOW-02 (`lex_key`, which picks the
winning epoch), RTE-32 (extracted C, the key's tie-break), FLOW-04 (the sky130 deck), FLOW-01 (device sizes) and
FLOW-03 (one dual step per epoch) all landed after it and move these numbers; PERF-01 (signoff errors, warnings
and coverage apart) landed last and was checked to leave every column but `ms` unchanged.

Command, from the repo root, release build, sky130, `FEEDBACK_ITERS = 5`, `starts = 3` (bench defaults), no ngspice
model library (`[op] operating point unavailable` on every fixture, so no perf scoring):

```sh
for s in 1 2 3 4 5; do PNR_BENCH_SEED=$s ./target/release/bench local; done
./target/release/three_stage_opamp pdks/sky130.json
```

Columns are `RunStats` of the winning epoch: `route_hard`, `route_overuse` (milli-budgets), `place`
(`PlacementMetrics`) and `dp` (`PlaceStats`); DRC, LVS, ERC and deck warnings are the bench's signoff
columns. `ms` is
the whole `library::run` + signoff per fixture.

**This is not T7's wall-time baseline.** The machine (32 cores) ran other chains' cargo builds and benches
throughout (1-minute load average 11–56 during these runs), and one fixture moves by up to 3× with the load (review
of the first table: bjt_mirror seed 1 took 9.5 s at load ~16 against 28.1 s recorded at load ~25). The `ms` column and the per-seed
wall times below are recorded as measured, for orientation only; T7's "bench wall ≤ baseline" stays open until
someone measures on an otherwise idle machine, and until then audit-06 §G.3's 539.5 s (seed 1, `FEEDBACK_ITERS = 5`,
an earlier tree) is the reference. T7's proposals/s also needs FLOW-09's dp `stage_ms`, which does not exist yet;
`proposals` here is the winning epoch's dp call only, a count, not a rate.

Notes:
- `GpMode::Pile` (the ablation's "no gp" arm) returns gp's seeded pile before the analytic loop. Since FLOW-03 gp
  takes no dual step in either mode (the epoch's one `Prices::settle` runs after dp), so Pile and Analytic epochs
  follow the same price trajectory rule and the ablation compares the gp loop alone. On the PLC-01 tree before FLOW-03
  this was not so: gp ended with `prices.settle`, which Pile skipped, so a Pile epoch took one dual step fewer.
  PLC-06's ablation table must be measured on a tree with FLOW-03.
- `bgr_core`, `pair`, `chain4`, `quad`: area usage exactly 1.000 and every proposal accepted, consistent with a
  single placed cell after group merging (no move can change the gate key or the PEX tier of one cell).
- `lattice_off` is nonzero on 8 of 10 fixtures (PLC-02's T1 target is 0); `overlap_nm2` is 0 everywhere;
  `clearance_residue_nm2` is nonzero on dac4 seed 5 only.
- dac4 seeds 2, 3 and 5 win with ERC 1. That is the tree, not PLC-01: on pristine m0 `5478230` (and `bb556b4`
  before it) `benchmarks/tests/signoff_fixtures.rs` already fails with "dac4: ERC 1 exceeds baseline 0"
  (`erc/ar.met2.1:gate`, PEX 326.0 fF), and `frontend/library/tests/hier_elaborate.rs` with `lvs.unpaired_device`
  rows; both fail identically on this branch.
- `matched_geometry_mismatch` counts hard mirror pairs only (PLC-03 widens it to matched pairs); `islands_extra`,
  `decode_fail` and `matched_incompatible` are 0 by construction until PLC-12, PLC-08 and PLC-03. Because the
  fixtures leave mismatch and overlap at 0, `placement_metrics_counts_each_defect` (in `frontend/library/src/geometry.rs`)
  pins every metric on a hand-built layout, so PLC-02/03/09's "= 0" acceptance cannot pass on a metric stuck at 0.
- `examples/three_stage_opamp` (T7's second fixture) is not a bench fixture; its winner is recorded separately below.

| fixture | seed | ms | route_hard | overuse | T6 sum | DRC | LVS | ERC | warn | area_usage | lattice_off | overlap nm² | clr residue nm² | matched mismatch | islands extra | dp temps | proposals | accepted |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| bgr_core | 1 | 16288 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.000 | 0 | 0 | 0 | 0 | 0 | 220 | 9959 | 9959 |
| bjt_mirror | 1 | 9690 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.259 | 0 | 0 | 0 | 0 | 0 | 220 | 23729 | 5850 |
| chain4 | 1 | 7925 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 10591 | 10591 |
| dac4 | 1 | 179830 | 1 | 1 | 2 | 0 | MATCH | 0 | 0 | 1.668 | 8 | 0 | 0 | 0 | 0 | 220 | 102992 | 575 |
| ota | 1 | 24486 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.140 | 2 | 0 | 0 | 0 | 0 | 220 | 33815 | 10311 |
| ota_constrained | 1 | 29098 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.140 | 2 | 0 | 0 | 0 | 0 | 220 | 33815 | 10311 |
| pair | 1 | 6384 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 9959 | 9959 |
| quad | 1 | 5422 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 10618 | 10618 |
| rc_filter | 1 | 4731 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.301 | 2 | 0 | 0 | 0 | 0 | 220 | 36953 | 650 |
| tt_ota | 1 | 21912 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.140 | 2 | 0 | 0 | 0 | 0 | 220 | 33815 | 10311 |
| bgr_core | 2 | 4544 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.000 | 0 | 0 | 0 | 0 | 0 | 220 | 9948 | 9948 |
| bjt_mirror | 2 | 9074 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.259 | 0 | 0 | 0 | 0 | 0 | 220 | 23735 | 6576 |
| chain4 | 2 | 7744 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 10667 | 10667 |
| dac4 | 2 | 177191 | 1 | 0 | 1 | 0 | MATCH | 1 | 0 | 1.604 | 8 | 0 | 0 | 0 | 0 | 220 | 102910 | 439 |
| ota | 2 | 33565 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.140 | 2 | 0 | 0 | 0 | 0 | 220 | 33881 | 3402 |
| ota_constrained | 2 | 38023 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.140 | 2 | 0 | 0 | 0 | 0 | 220 | 33881 | 3402 |
| pair | 2 | 10373 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 9975 | 9975 |
| quad | 2 | 10433 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 10600 | 10600 |
| rc_filter | 2 | 9599 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.301 | 2 | 0 | 0 | 0 | 0 | 220 | 36869 | 654 |
| tt_ota | 2 | 36194 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.140 | 2 | 0 | 0 | 0 | 0 | 220 | 33881 | 3402 |
| bgr_core | 3 | 9187 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.000 | 0 | 0 | 0 | 0 | 0 | 220 | 9936 | 9936 |
| bjt_mirror | 3 | 5196 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.259 | 0 | 0 | 0 | 0 | 0 | 220 | 23786 | 3076 |
| chain4 | 3 | 5345 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 10589 | 10589 |
| dac4 | 3 | 229202 | 1 | 0 | 1 | 0 | MATCH | 1 | 0 | 1.651 | 8 | 0 | 0 | 0 | 0 | 220 | 102971 | 622 |
| ota | 3 | 29927 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.140 | 2 | 0 | 0 | 0 | 0 | 220 | 33750 | 10328 |
| ota_constrained | 3 | 35415 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.140 | 2 | 0 | 0 | 0 | 0 | 220 | 33750 | 10328 |
| pair | 3 | 7222 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 9937 | 9937 |
| quad | 3 | 6748 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 9937 | 9937 |
| rc_filter | 3 | 7007 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.428 | 2 | 0 | 0 | 0 | 0 | 220 | 36884 | 464 |
| tt_ota | 3 | 28255 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.140 | 2 | 0 | 0 | 0 | 0 | 220 | 33750 | 10328 |
| bgr_core | 4 | 5907 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.000 | 0 | 0 | 0 | 0 | 0 | 220 | 9948 | 9948 |
| bjt_mirror | 4 | 4827 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.259 | 0 | 0 | 0 | 0 | 0 | 220 | 23761 | 5396 |
| chain4 | 4 | 5049 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 10600 | 10600 |
| dac4 | 4 | 169520 | 1 | 1 | 2 | 0 | MATCH | 0 | 0 | 1.475 | 8 | 0 | 0 | 0 | 0 | 220 | 102908 | 593 |
| ota | 4 | 26828 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.878 | 2 | 0 | 0 | 0 | 0 | 220 | 33083 | 6726 |
| ota_constrained | 4 | 28227 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.878 | 2 | 0 | 0 | 0 | 0 | 220 | 33083 | 6726 |
| pair | 4 | 4979 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 10600 | 10600 |
| quad | 4 | 5523 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 9934 | 9934 |
| rc_filter | 4 | 5168 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.612 | 2 | 0 | 0 | 0 | 0 | 220 | 36978 | 581 |
| tt_ota | 4 | 23794 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.878 | 2 | 0 | 0 | 0 | 0 | 220 | 33083 | 6726 |
| bgr_core | 5 | 9604 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.000 | 0 | 0 | 0 | 0 | 0 | 220 | 9936 | 9936 |
| bjt_mirror | 5 | 5180 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.259 | 0 | 0 | 0 | 0 | 0 | 220 | 23786 | 3076 |
| chain4 | 5 | 6270 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 9936 | 9936 |
| dac4 | 5 | 209785 | 1 | 0 | 1 | 0 | MATCH | 1 | 0 | 1.531 | 8 | 0 | 134000 | 0 | 0 | 220 | 102991 | 601 |
| ota | 5 | 32309 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.140 | 2 | 0 | 0 | 0 | 0 | 220 | 33750 | 10328 |
| ota_constrained | 5 | 30008 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.140 | 2 | 0 | 0 | 0 | 0 | 220 | 33750 | 10328 |
| pair | 5 | 4687 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 9937 | 9937 |
| quad | 5 | 6185 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 9937 | 9937 |
| rc_filter | 5 | 6938 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.427 | 2 | 0 | 0 | 0 | 0 | 220 | 36876 | 623 |
| tt_ota | 5 | 22257 | 0 | 0 | 0 | 0 | MATCH | 0 | 0 | 1.140 | 2 | 0 | 0 | 0 | 0 | 220 | 33750 | 10328 |

| fixture | median area_usage (seeds 1–5) |
|---|---|
| bgr_core | 1.000 |
| bjt_mirror | 1.259 |
| chain4 | 1.000 |
| dac4 | 1.604 |
| ota | 1.140 |
| ota_constrained | 1.140 |
| pair | 1.000 |
| quad | 1.000 |
| rc_filter | 1.427 |
| tt_ota | 1.140 |

Median area_usage over all 50 runs (T2): 1.140.

## `examples/three_stage_opamp` (T7's second fixture)

`Config::default()` (seed 42, `starts = 3`, `feedback_iters = 200`), sky130, same build; one run, winner's `RunStats`:

| fixture | area_usage | lattice_off | overlap nm² | clr residue nm² | matched mismatch | islands extra | dp temps | proposals | accepted | decode fail | matched incompat | signoff hard |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| three_stage_opamp | 1.514 | 4 | 0 | 0 | 0 | 0 | 220 | 69704 | 1406 | 0 | 0 | 91 |

## Wall time (not the T7 baseline; see above)

`bench local`, all 10 fixtures, per seed, under the load stated above:

- seed 1: 305.8 s
- seed 2: 336.8 s
- seed 3: 363.5 s
- seed 4: 279.8 s
- seed 5: 333.2 s

`three_stage_opamp`: 46.6 s.
