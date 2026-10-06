# M2+ placement: segment report

## PLC-29 acceptance (matched-pair environment live in placement)

Measured at `fe97c78` (PLC-29) against base `a2fece9`, release build, sky130, `feedback_iters = 5`, default
`starts` (3), seeds 1–5, 11 fixtures. Each tree was built from `git archive` with an uncommitted probe:
`library::run` per fixture and seed, plus `Instant` timers around the epoch's `gp::place` and `dp::place`. The timers
sum over the 3 start threads. The two trees ran at the same time, so both saw the same machine load (1-min load
average 3–80 from other modules' builds).

| fixture | seeds | T9 (max) | lex key = base | gp s base | gp s PLC-29 | Δgp | dp s base | dp s PLC-29 | Δdp |
|---|---|---|---|---|---|---|---|---|---|
| pair | 5 | 0 | 5/5 | 0.01 | 0.01 | +26% | 0.14 | 0.18 | +28% |
| quad | 5 | 0 | 5/5 | 0.05 | 0.05 | +0% | 14.19 | 15.59 | +10% |
| chain4 | 5 | 0 | 5/5 | 0.01 | 0.01 | +33% | 0.25 | 0.28 | +15% |
| rc_filter | 5 | 0 | 5/5 | 0.13 | 0.14 | +9% | 8.77 | 9.16 | +4% |
| mirror_ratio | 5 | 0 | 5/5 | 1.37 | 1.34 | −2% | 24.99 | 25.12 | +1% |
| bjt_mirror | 5 | 0 | 5/5 | 0.01 | 0.01 | +17% | 1.65 | 1.91 | +16% |
| bgr_core | 5 | 0 | 5/5 | 0.01 | 0.01 | +5% | 0.21 | 0.21 | −0% |
| dac4 | 5 | 0 | 5/5 | 0.90 | 0.88 | −2% | 69.79 | 66.99 | −4% |
| ota | 5 | 0 | 5/5 | 31.29 | 41.34 | +32% | 754.33 | 953.81 | +26% |
| ota_constrained | 5 | 0 | 5/5 | 50.95 | 58.94 | +16% | 1207.74 | 1341.93 | +11% |
| tt_ota | 5 | 0 | 5/5 | 34.01 | 42.19 | +24% | 809.96 | 975.23 | +20% |
| all | | | | 118.7 | 144.9 | +22% | 2892.0 | 3390.4 | +17% |

- **T9 = 0 on every fixture × seed.** This is the final report's routing `Environment` row, rings included. Only
  ota, ota_constrained, tt_ota (2 pairs, worst usage 0.716) and mirror_ratio (2 pairs, usage 0.0003) have a recognised
  matched pair. The other 7 fixtures have no `Environment` row, so T9 is vacuously 0 there.
- **Lex key: identical to base on 11/11 fixtures, in all 55 runs.** The full 5-tuple (V, spec, Θ, C tier, area) is
  identical, and so are the layouts. The batch's residual is 0 throughout, so it never moves a decision.
- **dp: within +30 %** (+17 % in aggregate, worst +26 % on ota). The step-5 cache is not added, and the
  `environment.rs` ponytail note still gives its trigger. **gp: ota measured +32 %, past the 30 % risk line.** This is
  reported, not gated, as the card requires. A second ota-only rerun showed the noise. Seeds 1–3 started on an
  idle machine and gave gp +30 % and dp +25 % on each seed. Seeds 4–5 overlapped a load spike to 45, and new-tree
  seed 5 took 3.4× the base, so that run cannot decide the 30 % line either way. A clean T7 number still needs an
  idle machine (as `baseline-plc-01.md` notes).

## Red debug test: MAT-07 registers `Orientation` in both arms (owner: matching/annotator)

`cargo test -p library` (debug) fails 8 tests. One is `environment_tests::environment_is_in_the_placement_arm_once`.
The other seven are `flow_smoke::matched_sets_are_reported`, `perf_postlayout::{op_runs_carry_ir_limits,
a_flow_scores_its_layout_in_simulation, a_flow_reports_the_worst_scenario_per_bound}` and
`placement_metrics::{every_origin_is_on_the_cut_lattice, metrics_are_populated_on_ota, gp_modes_are_deterministic}`.
All eight fail with the same panic, `backend/gp/src/lib.rs:89`: "gp::Prices: a batch kind is registered in both
`hard` and `budget`". The cause is `backend/annotator/src/emit.rs:145-148` (MAT-07, merged into m2 at `c867abb`). It
pushes `OrientationSet{Axis}` to `hard` and, for a non-strict phi class, `OrientationSet{Phi}` to `budget`. Both
report `kind() == "Orientation"`. This predates PLC-29: on base `a2fece9`, `flow_smoke::matched_sets_are_reported`
fails with the same panic. Release builds pass because the check is a `debug_assert`. The fix belongs to the MAT-07
owner. Either give the phi check its own kind, or make the assert compare the exact (kind, check) identity.
`environment_is_in_the_placement_arm_once` stays on `ota` (`pair.spice` has no matched pair) and stays red under debug
until that fix lands.

## PLC-17 acceptance (review fixes 6)

Seeds 1-5, `ota` and `rc_filter` with op, `84e1d52` vs `964165d`: IrDrop batch (3 rules, 0 violated, 0 unknown),
signoff `ir_drop` ran with 0 rows, and median budgeted-net C (ota 16.257-16.865 fF, rc_filter 1.645 fF) are identical
run for run. Pass by equality; no IR or C gain is shown on these fixtures. Signoff reports no worst drop/limit for a
passing grid, so the IR side is only pass/fail. Table in `docs/CRATES.md` `## backend/gp`. 0.1 / 0.25 unchanged.

## Bench protocol for PLC-07, PLC-13 and PLC-24

Each tree comes from `git archive <rev>` plus an uncommitted probe, `frontend/library/tests/zz_probe.rs`. The probe
calls `library::run` per fixture and seed, release, sky130, `feedback_iters = 5`, default `starts`. The probe was built
through `tools/qcargo` inside the flake dev shell (`nix develop`). The trees ran at the same time.

Columns:
- T2 = `layout.footprint_nm2() / Σ 4·hw·hh`.
- drc = `RunStats::drc_hard`, the flow's own signoff (DRC + ERC + LVS) over the winner's geometry. It is one count, not
  separate DRC and LVS counts (a departure from the card).
- route_hard = `RunStats::route_hard`.
- T9 = the routing `Environment` row's `violations`. −1 means the fixture has no row (no recognised matched pair).

## PLC-07 acceptance (per-pair spacing from cell edge profiles)

Base `c235de8` against PLC-07 `3c12878` exactly, so no later item is mixed in. Seeds 1–5, 11 fixtures. Per-seed values
are separated by `/`.

| fixture | seeds | T2 b07 | T2 p07 | ΔT2 | drc b07 | drc p07 | route_hard b07 | route_hard p07 | T9 max b07 | T9 max p07 |
|---|---|---|---|---|---|---|---|---|---|---|
| pair | 5/5 | 1.0000 | 1.0000 | +0.0% | 0/0/0/0/0 | 0/0/0/0/0 | 0/0/0/0/0 | 0/0/0/0/0 | -1 | -1 |
| quad | 5/5 | 1.7108 | 1.2071 | -29.4% | 0/0/0/0/0 | 0/0/0/0/0 | 0/0/0/0/0 | 0/0/0/0/0 | -1 | -1 |
| chain4 | 5/5 | 1.0000 | 1.0000 | +0.0% | 0/0/0/0/0 | 0/0/0/0/0 | 0/0/0/0/0 | 0/0/0/0/0 | -1 | -1 |
| rc_filter | 5/5 | 1.3001 | 1.1434 | -12.1% | 0/0/0/0/0 | 0/0/0/0/0 | 0/0/0/0/0 | 0/0/0/0/0 | -1 | -1 |
| mirror_ratio | 5/5 | 1.0000 | 1.0000 | +0.0% | 0/0/0/0/0 | 0/0/0/0/0 | 0/0/0/0/0 | 0/0/0/0/0 | 0 | 0 |
| bjt_mirror | 5/5 | 1.2129 | 1.1305 | -6.8% | 2/2/2/2/2 | 2/2/2/2/2 | 0/0/0/0/0 | 0/0/0/0/0 | -1 | -1 |
| bgr_core | 5/5 | 1.0000 | 1.0000 | +0.0% | 1/1/1/1/1 | 1/1/1/1/1 | 0/0/0/0/0 | 0/0/0/0/0 | -1 | -1 |
| dac4 | 5/5 | 1.5355 | 1.3606 | -11.4% | 1/1/1/1/1 | 1/1/1/1/1 | 1/1/1/1/1 | 1/1/1/1/1 | -1 | -1 |
| ota | 5/5 | 1.2457 | 1.0820 | -13.1% | 0/0/0/0/0 | 0/0/0/0/0 | 0/0/0/0/0 | 0/0/0/0/0 | 0 | 0 |
| ota_constrained | 5/5 | 1.2457 | 1.0820 | -13.1% | 0/0/0/0/0 | 0/0/0/0/0 | 0/0/0/0/0 | 0/0/0/0/0 | 0 | 0 |
| tt_ota | 5/5 | 1.2457 | 1.0820 | -13.1% | 0/0/0/0/0 | 0/0/0/0/0 | 0/0/0/0/0 | 0/0/0/0/0 | 0 | 0 |
| mean of medians | | 1.2269 | 1.0989 | -10.4% |

- **T2: falls short of the 20 % target.** The mean of the per-fixture medians improves by 10.4 % (1.2269 → 1.0989).
  The best fixture is quad at −29.4 %; the three OTAs are −13.1 %, dac4 −11.4 % and rc_filter −12.1 %.
- Four fixtures are pinned at T2 = 1.0000 on both trees: pair, chain4, mirror_ratio and bgr_core. Their footprint
  already equals Σ cell area (a single merged cell or abutted cells), so per-pair spacing cannot gain there.
  Excluding them, the mean improvement is 14.8 %.
- Not tuned, as the card requires.
- **DRC/LVS: no worse on any fixture or seed.** drc_hard and route_hard are identical per seed. T9 is 0 or absent
  on both trees.

## PLC-13 acceptance (WPE and extraneous-poly keep-outs)

Before `1df326d` against PLC-13 `cd443cb` (the parent of the parent, with the stray-fmt revert). Seeds 1–5, 11 fixtures.

| fixture | seeds | T2 b13 | T2 p13 | ΔT2 | drc b13 | drc p13 | route_hard b13 | route_hard p13 | T9 max b13 | T9 max p13 |
|---|---|---|---|---|---|---|---|---|---|---|
| pair | 5/5 | 1.0000 | 1.0000 | +0.0% | 0/0/0/0/0 | 0/0/0/0/0 | 2/2/2/2/2 | 2/2/2/2/2 | -1 | -1 |
| quad | 5/5 | 1.2749 | 1.2749 | +0.0% | 0/0/0/0/0 | 0/0/0/0/0 | 0/0/0/1/0 | 0/0/0/1/0 | -1 | -1 |
| chain4 | 5/5 | 1.0000 | 1.0000 | +0.0% | 5/5/4/5/4 | 3/5/3/7/5 | 7/9/7/9/7 | 8/7/8/9/9 | -1 | -1 |
| rc_filter | 5/5 | 1.3200 | 1.3200 | +0.0% | 0/0/0/0/0 | 0/0/0/0/0 | 0/0/0/0/0 | 0/0/0/0/0 | -1 | -1 |
| mirror_ratio | 5/5 | 1.0000 | 1.0000 | +0.0% | 0/0/0/0/0 | 0/0/0/0/0 | 14/14/14/14/14 | 14/14/14/14/14 | 0 | 0 |
| bjt_mirror | 5/5 | 1.1305 | 1.1305 | +0.0% | 3/3/3/3/3 | 3/3/3/3/3 | 0/0/0/0/0 | 0/0/0/0/0 | -1 | -1 |
| bgr_core | 5/5 | 1.0000 | 1.0000 | +0.0% | 0/0/0/0/0 | 0/0/0/0/0 | 0/0/0/0/0 | 0/0/0/0/0 | 1 | 1 |
| dac4 | 5/5 | 1.3622 | 1.3622 | +0.0% | 11/16/13/15/11 | 11/16/13/15/11 | 1/1/2/3/1 | 1/1/2/3/1 | -1 | -1 |
| ota | 5/5 | 1.2678 | 1.2678 | +0.0% | 1/1/3/1/1 | 1/1/3/1/1 | 41/42/39/42/41 | 41/42/39/42/41 | 0 | 0 |
| ota_constrained | 5/5 | 1.2678 | 1.2678 | +0.0% | 1/1/3/1/1 | 1/1/3/1/1 | 41/42/39/42/41 | 41/42/39/42/41 | 0 | 0 |
| tt_ota | 5/5 | 1.2678 | 1.2678 | +0.0% | 1/1/3/1/1 | 1/1/3/1/1 | 41/42/39/42/41 | 41/42/39/42/41 | 0 | 0 |
| mean of medians | | 1.1719 | 1.1719 | +0.0% |

- **T2: within +5 %.** It is identical on every fixture: the per-seed footprints match in all 55 runs. None of these
  fixtures places a foreign well or poly inside a matched cell's keep-out on the converged layouts. chain4's drc
  differs per seed with an equal T2, but the base has the same spread, so this is not a change from the item.
- **T9: not 0 on bgr_core (T9 = 1 on every seed, before and after PLC-13).** The row is the Q1/Q2 PNP pair.
  PLC-29's table, measured at `fe97c78`, shows no `Environment` row for bgr_core, so a later m2 merge before
  `1df326d` made that pair recognised. The keep-outs act only between cells. They cannot move geometry inside a
  merged BJT array, which is where this nearness comes from (bgr_core's T2 is 1.0, a single cell). This is reported,
  not fixed: it is outside PLC-13's scope, and the owner is matching/annotator (pair recognition for BJTs) or
  `LiveEnvironment` (applying WPE to BJTs). T9 is 0 on every other fixture with a row.
- These counts are the same on both trees, so they are not caused by this item: route_hard 41 on the OTAs, 14 on
  mirror_ratio, and dac4 drc 11–16.
  - Before the `b07`/`p07` trees they were 0, 0 and 1. So the regressions entered m2 between `3c12878` and
    `1df326d`, from other modules' merges.
  - The same holds for four integration tests that fail on both `1df326d` and `cd443cb`:
    `antenna_diode`, `drawn_cards::a_mim_dac_signs_off_with_its_capacitors`,
    `emit_roundtrip::run_emit_elaborate_signs_off` and `extra_devices::an_adopted_antenna_diode_keeps_lvs_matched`.
    All four fail with LVS unpaired devices/nets or a floating net.

## PLC-24 acceptance (a reachable utilization floor)

PLC-24 tree (`cd443cb` + PLC-24), seed 1, with the default floor (0.6, capped by `u_eff`) and with
`min_utilization = 0` (no Utilization budget).

| fixture | T2 default | T2 floor = 0 | Utilization residual (default) |
|---|---|---|---|
| pair | 1.0000 | 1.0000 | 0.0000 |
| quad | 1.0000 | 1.2749 | 0.0000 |
| chain4 | 1.0000 | 1.0000 | 0.0000 |
| rc_filter | 1.3200 | 1.3052 | 0.0000 |
| mirror_ratio | 1.0000 | 1.0000 | 0.0000 |
| bjt_mirror | 1.1305 | 1.1305 | 0.0000 |
| bgr_core | 1.0000 | 1.0000 | 0.0000 |
| dac4 | 1.3021 | 1.7108 | 0.0000 |
| ota | 1.4004 | 1.4004 | 0.0000 |
| ota_constrained | 1.4004 | 1.4004 | 0.0000 |
| tt_ota | 1.4004 | 1.4004 | 0.0000 |

- **The final Utilization residual is 0 on every fixture,** so the card's failure condition (a residual > 0 where
  the floor alone binds) never arises.
- The default-floor layouts are identical to `cd443cb`'s seed-1 layouts on all 11 fixtures. So the cap changes no
  converged result here. It removes an unreachable target only for small cells with large gaps (unit test
  `utilization_floor_is_reachable_for_tiny_cells`).
- Dropping the floor makes quad (+27 %) and dac4 (+31 %) larger and rc_filter 1 % smaller. The floor is useful pressure,
  not a binding constraint.

## PLC-29 verification after the m2 merge (segment 7)

- `cargo test --release -p gp -p analog -p library` was run on `cd443cb` + PLC-24, in the flake dev shell.
- gp and analog pass, including `environment`. The library lib tests pass (177/177), including `environment_tests`
  and `start_tests::utilization_floor_is_reachable_for_tiny_cells`.
- Five integration tests fail:
  - The four LVS tests above. They fail identically on `1df326d`, before this segment's items.
  - `perf_postlayout::ota_probe_regions` (XM1 in triode). The ota layouts are identical per seed on `1df326d`,
    `cd443cb` and PLC-24, so none of this segment's items moved them.
