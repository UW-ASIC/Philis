# M2 perf segment 2 report: GAP-17 step 1, PERF-18

Card `docs/plans/cards/m2-perf-2.md`. Commits `e722a05` (GAP-17), `831f156` (PERF-18), plus "M2+ perf: review fixes
2". Every number below was measured on 2026-10-04 in `philis-m2/perf` with
`PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk`. `bench local` ran in release at seed 1, exit 0, and
placed and routed 14/14.

## GAP-17 (step 1)

`OpPoint.net_v` is filled from a `print all` block. `ota_op_resolves_every_net_voltage` passes. The other two parts
of the GAP-17 acceptance are not this item's: "EXT-17 regions unchanged" belongs to EXT-17 (annotator, M3) and
"REL T9 reports V_SB per matched pair" to REL-10 (reliability). Neither consumer (`OpFacts.net_mv`,
`PairAging.dvbs_mv`) exists in the tree yet.

## PERF-18

- **Step 0: the M1 note was stale.** `bgr_core` is now in `fixtures_resolve_every_device` and passes with no change
  to `oppoint` or `deck_models`. Every device resolves, and both PNPs simulate on the shipped `tt` models.
- **Acceptance (bench LVS column).** bjt_mirror goes from PARTIAL(2) to MATCH, and bgr_core from PARTIAL(9) to
  MATCH. Every other sky130 local fixture reads MATCH except dac4, which reads PARTIAL(16): its 5 MOM capacitors
  (m = 1+1+2+4+8 = 16 units) have no deck row. So T3 = 0 except dac4's 16 MOM units, as the card expects.
- **Deviation from card step 5.** The card says to run `uncompared_devices_do_not_block_convergence` on dac4 and
  to report it if dac4 does not converge in 5 epochs. dac4 does not converge. In `bench local` it stops on budget
  at 15 epochs ("best 3/15 budget") with `route hard 1`. That one hard row is its in-loop antenna on `top`. The bench Antenna
  hard arm has 2 violations: dac4 and dac4_mim, `route hard 1` each. dac4_mim has no MOM caps and stops on budget
  the same way, so the cause is antenna, not LVS coverage. So the test uses an inline `momcap` netlist instead (2 MOM
  units beside one nfet). It keeps every assertion and the 5-epoch budget. dac4's convergence belongs to the
  antenna and routing owners, not PERF-18.

## Red tests not named in the card (not fixed here, assertion untouched)

Three tests in `frontend/library/tests/perf_postlayout.rs` fail in debug builds: `op_runs_carry_ir_limits`,
`a_flow_scores_its_layout_in_simulation` and `a_flow_reports_the_worst_scenario_per_bound`. Each one panics at
`backend/gp/src/lib.rs:87`, the `debug_assert` in `Prices::bind` ("a batch kind is registered in both `hard` and
`budget`"). The panic then surfaces at `frontend/library/src/lib.rs:311` as "a search start panicked". The review
found that these three fail the same way on the merge-base `c73fccf`, so this segment did not cause them. Release
builds skip the assert. The same panic fails
all 3 tests of `frontend/library/tests/placement_metrics.rs` in debug: `every_origin_is_on_the_cut_lattice`,
`gp_modes_are_deterministic` and `metrics_are_populated_on_ota`. Everything else passes: debug `library`, `verify`,
`cells`, `pnr_core`, and release `benchmark`, `signoff_fixtures` included.

Cause: `backend/annotator/src/emit.rs:144-145` (M1 MAT-05, `723bbf7`) pushes one `OrientationSet` into `hard`
(`OrientCheck::Axis`) and another into `budget` (`OrientCheck::Phi`). Both report `kind() == "Orientation"`
(`kernel/analog/src/placement/orientation.rs:78`). The bench constraint table lists Orientation in both arms. The
module doc at `emit.rs:30` says this split is intended, so the conflict is between the matching module's
requirement registry and the placement module's invariant in `gp::Prices`. Owners: matching (annotator `emit`)
and placement (`gp`). Either the two checks get distinct kinds, or `Prices` keys by (arm, kind).
