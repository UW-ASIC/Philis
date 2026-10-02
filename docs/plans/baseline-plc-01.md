# PLC-01 placement baseline (M0)

The numbers `bench local` printed for the 10 fixtures in `benchmarks/fixtures/` × seeds 1–5, on the code of the
commit that adds this file (m0 at `6da8068` + PLC-01). They are the **baseline** for plan-04 T2 (area usage), T6
(`route_hard + route_overuse`) and T7 (runtime) and are not to be edited after the fact; a later item records its own
numbers beside them.

Command, from the repo root, release build, sky130, `FEEDBACK_ITERS = 5`, `starts = 3` (bench defaults), no ngspice
model library (`[op] operating point unavailable` on every fixture, so no perf scoring):

```sh
for s in 1 2 3 4 5; do PNR_BENCH_SEED=$s ./target/release/bench local; done
```

Columns are `RunStats` of the winning epoch: `route_hard`, `route_overuse` (milli-budgets), `place`
(`PlacementMetrics`) and `dp` (`PlaceStats`); DRC and LVS are the bench's signoff columns. `ms` is the whole
`library::run` + signoff per fixture.

Caveats, measured on the day:
- **Timing is noisy.** The machine (32 cores) ran other cargo jobs throughout (load average 21–26), so `ms` and the
  per-seed wall times below carry that contention: seed-to-seed ms on one fixture vary 1.4–5.2× (ota 24–85 s,
  bgr_core 6–30 s). T7's proposals/s needs FLOW-09's dp `stage_ms`, which does not exist yet; `proposals` here is the winning epoch's dp
  call only, not a rate.
- `bgr_core`, `pair`, `chain4`, `quad`: area usage exactly 1.000 and every proposal accepted, consistent with a
  single placed cell after group merging (no move can change the gate key or the PEX tier of one cell).
- `lattice_off` is nonzero on 8 of 10 fixtures (PLC-02's T1 target is 0); `overlap_nm2` is 0 everywhere;
  `clearance_residue_nm2` is nonzero on dac4 seeds 3 and 5.
- `matched_geometry_mismatch` counts hard mirror pairs only (PLC-03 widens it to matched pairs); `islands_extra`,
  `decode_fail` and `matched_incompatible` are 0 by construction until PLC-12, PLC-08 and PLC-03.

| fixture | seed | ms | route_hard | overuse | T6 sum | DRC | LVS | area_usage | lattice_off | overlap nm² | clr residue nm² | matched mismatch | islands extra | dp temps | proposals | accepted |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| bgr_core | 1 | 29949 | 0 | 0 | 0 | 0 | MATCH | 1.000 | 0 | 0 | 0 | 0 | 0 | 220 | 9959 | 9959 |
| bjt_mirror | 1 | 28083 | 0 | 0 | 0 | 0 | MATCH | 1.259 | 0 | 0 | 0 | 0 | 0 | 220 | 23729 | 5850 |
| chain4 | 1 | 18100 | 0 | 0 | 0 | 0 | MATCH | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 10591 | 10591 |
| dac4 | 1 | 270633 | 1 | 18 | 19 | 0 | MATCH | 1.668 | 8 | 0 | 0 | 0 | 0 | 220 | 102992 | 575 |
| ota | 1 | 37425 | 0 | 0 | 0 | 0 | MATCH | 1.164 | 1 | 0 | 0 | 0 | 0 | 220 | 33789 | 5728 |
| ota_constrained | 1 | 34048 | 0 | 0 | 0 | 0 | MATCH | 1.164 | 1 | 0 | 0 | 0 | 0 | 220 | 33789 | 5728 |
| pair | 1 | 7486 | 0 | 0 | 0 | 0 | MATCH | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 9959 | 9959 |
| quad | 1 | 7563 | 0 | 0 | 0 | 0 | MATCH | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 10618 | 10618 |
| rc_filter | 1 | 13418 | 0 | 0 | 0 | 0 | MATCH | 1.301 | 2 | 0 | 0 | 0 | 0 | 220 | 36953 | 650 |
| tt_ota | 1 | 65549 | 0 | 0 | 0 | 0 | MATCH | 1.164 | 1 | 0 | 0 | 0 | 0 | 220 | 33789 | 5728 |
| bgr_core | 2 | 24193 | 0 | 0 | 0 | 0 | MATCH | 1.000 | 0 | 0 | 0 | 0 | 0 | 220 | 9948 | 9948 |
| bjt_mirror | 2 | 26391 | 0 | 0 | 0 | 0 | MATCH | 1.259 | 0 | 0 | 0 | 0 | 0 | 220 | 23735 | 6576 |
| chain4 | 2 | 13089 | 0 | 0 | 0 | 0 | MATCH | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 10667 | 10667 |
| dac4 | 2 | 196517 | 1 | 0 | 1 | 0 | MATCH | 1.604 | 8 | 0 | 0 | 0 | 0 | 220 | 102910 | 439 |
| ota | 2 | 24475 | 0 | 0 | 0 | 0 | MATCH | 1.163 | 2 | 0 | 0 | 0 | 0 | 220 | 33770 | 5142 |
| ota_constrained | 2 | 41552 | 0 | 0 | 0 | 0 | MATCH | 1.163 | 2 | 0 | 0 | 0 | 0 | 220 | 33770 | 5142 |
| pair | 2 | 16509 | 0 | 0 | 0 | 0 | MATCH | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 9975 | 9975 |
| quad | 2 | 22910 | 0 | 0 | 0 | 0 | MATCH | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 10600 | 10600 |
| rc_filter | 2 | 12499 | 0 | 0 | 0 | 0 | MATCH | 1.301 | 2 | 0 | 0 | 0 | 0 | 220 | 36869 | 654 |
| tt_ota | 2 | 28928 | 0 | 0 | 0 | 0 | MATCH | 1.163 | 2 | 0 | 0 | 0 | 0 | 220 | 33770 | 5142 |
| bgr_core | 3 | 9371 | 0 | 0 | 0 | 0 | MATCH | 1.000 | 0 | 0 | 0 | 0 | 0 | 220 | 9936 | 9936 |
| bjt_mirror | 3 | 10954 | 0 | 0 | 0 | 0 | MATCH | 1.259 | 0 | 0 | 0 | 0 | 0 | 220 | 23786 | 3076 |
| chain4 | 3 | 7047 | 0 | 0 | 0 | 0 | MATCH | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 10589 | 10589 |
| dac4 | 3 | 226277 | 1 | 1 | 2 | 0 | MATCH | 1.636 | 8 | 0 | 291300 | 0 | 0 | 220 | 98456 | 447 |
| ota | 3 | 32654 | 0 | 0 | 0 | 0 | MATCH | 1.164 | 1 | 0 | 0 | 0 | 0 | 220 | 33819 | 3969 |
| ota_constrained | 3 | 30730 | 0 | 0 | 0 | 0 | MATCH | 1.164 | 1 | 0 | 0 | 0 | 0 | 220 | 33819 | 3969 |
| pair | 3 | 5452 | 0 | 0 | 0 | 0 | MATCH | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 9937 | 9937 |
| quad | 3 | 4842 | 0 | 0 | 0 | 0 | MATCH | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 9937 | 9937 |
| rc_filter | 3 | 4459 | 0 | 0 | 0 | 0 | MATCH | 1.428 | 2 | 0 | 0 | 0 | 0 | 220 | 36884 | 464 |
| tt_ota | 3 | 23940 | 0 | 0 | 0 | 0 | MATCH | 1.164 | 1 | 0 | 0 | 0 | 0 | 220 | 33819 | 3969 |
| bgr_core | 4 | 5795 | 0 | 0 | 0 | 0 | MATCH | 1.000 | 0 | 0 | 0 | 0 | 0 | 220 | 9948 | 9948 |
| bjt_mirror | 4 | 5312 | 0 | 0 | 0 | 0 | MATCH | 1.259 | 0 | 0 | 0 | 0 | 0 | 220 | 23761 | 5396 |
| chain4 | 4 | 6008 | 0 | 0 | 0 | 0 | MATCH | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 10600 | 10600 |
| dac4 | 4 | 240011 | 1 | 51 | 52 | 0 | MATCH | 1.475 | 8 | 0 | 0 | 0 | 0 | 220 | 102908 | 593 |
| ota | 4 | 84806 | 0 | 0 | 0 | 0 | MATCH | 1.163 | 2 | 0 | 0 | 0 | 0 | 220 | 33791 | 4947 |
| ota_constrained | 4 | 53048 | 0 | 0 | 0 | 0 | MATCH | 1.163 | 2 | 0 | 0 | 0 | 0 | 220 | 33791 | 4947 |
| pair | 4 | 8157 | 0 | 0 | 0 | 0 | MATCH | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 10600 | 10600 |
| quad | 4 | 9563 | 0 | 0 | 0 | 0 | MATCH | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 9934 | 9934 |
| rc_filter | 4 | 14833 | 0 | 0 | 0 | 0 | MATCH | 1.612 | 2 | 0 | 0 | 0 | 0 | 220 | 36978 | 581 |
| tt_ota | 4 | 63875 | 0 | 0 | 0 | 0 | MATCH | 1.163 | 2 | 0 | 0 | 0 | 0 | 220 | 33791 | 4947 |
| bgr_core | 5 | 12965 | 0 | 0 | 0 | 0 | MATCH | 1.000 | 0 | 0 | 0 | 0 | 0 | 220 | 9936 | 9936 |
| bjt_mirror | 5 | 11994 | 0 | 0 | 0 | 0 | MATCH | 1.259 | 0 | 0 | 0 | 0 | 0 | 220 | 23786 | 3076 |
| chain4 | 5 | 10592 | 0 | 0 | 0 | 0 | MATCH | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 9936 | 9936 |
| dac4 | 5 | 283021 | 1 | 0 | 1 | 0 | MATCH | 1.531 | 8 | 0 | 134000 | 0 | 0 | 220 | 102991 | 601 |
| ota | 5 | 33062 | 0 | 0 | 0 | 0 | MATCH | 1.164 | 1 | 0 | 0 | 0 | 0 | 220 | 33734 | 3748 |
| ota_constrained | 5 | 28196 | 0 | 0 | 0 | 0 | MATCH | 1.164 | 1 | 0 | 0 | 0 | 0 | 220 | 33734 | 3748 |
| pair | 5 | 4880 | 0 | 0 | 0 | 0 | MATCH | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 9937 | 9937 |
| quad | 5 | 4503 | 0 | 0 | 0 | 0 | MATCH | 1.000 | 1 | 0 | 0 | 0 | 0 | 220 | 9937 | 9937 |
| rc_filter | 5 | 5321 | 0 | 0 | 0 | 0 | MATCH | 1.427 | 2 | 0 | 0 | 0 | 0 | 220 | 36876 | 623 |
| tt_ota | 5 | 20266 | 0 | 0 | 0 | 0 | MATCH | 1.164 | 1 | 0 | 0 | 0 | 0 | 220 | 33734 | 3748 |

| fixture | median area_usage (seeds 1–5) |
|---|---|
| bgr_core | 1.000 |
| bjt_mirror | 1.259 |
| chain4 | 1.000 |
| dac4 | 1.604 |
| ota | 1.164 |
| ota_constrained | 1.164 |
| pair | 1.000 |
| quad | 1.000 |
| rc_filter | 1.427 |
| tt_ota | 1.164 |

Median area_usage over all 50 runs (T2): 1.163.

Bench wall per seed (`/usr/bin/time`, all 10 fixtures; audit-06 §G.3 measured 539.5 s for seed 1 on an earlier
tree):

- seed 1 wall 512.29 s
- seed 2 wall 407.09 s
- seed 3 wall 355.74 s
- seed 4 wall 491.42 s
- seed 5 wall 414.82 s
