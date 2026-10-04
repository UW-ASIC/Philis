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
