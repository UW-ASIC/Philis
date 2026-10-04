# M2+ reliability: segment report

The numbers below come from `cargo run --release -p benchmark --bin bench local` (seed 1, sky130, the probe op on
`$PDK_ROOT`), run on trees made with `git archive`. Commits 1a4e7fe and fe57a24 do not build: REL-10 left
`lib.rs:631` broken until 00df874 fixed it. So the REL-04 run is 8095bab plus fe57a24's `ir.rs`, which is the
only file REL-04 changes.

## REL-04 acceptance (T5): IR residuals on rails
Placement and routing are the same in both trees (WL and area match on all 14 fixtures). Only the drop changes.
| fixture | rows | max use before → after | Θ before → after |
|---|---|---|---|
| bjt_mirror | 3 | 0.272 → 0.238 (falls) | 0 → 0 |
| dac4_mim | 6 | 0.149 → 0.344 | 0 → 0 |
| ota (ota_constrained, tt_ota have the same layout) | 3 | 0.005 → 0.133 | 0 → 0 |
| rc_filter | 3 | 0.018 → 0.044 | 0 → 0 |
| tq_chain | 3 | 0.106 → 0.617 | 0 → 0 |

The residual Θ does not rise anywhere: all 30 rows stay within budget. Only bjt_mirror's use falls. In the other
fixtures the use rises, because the terminal-resolved tree is the one EM uses (routed + **cell** metal), while the
old "bound" summed only the routed shapes. So the old number was not an upper bound once a terminal sits inside cell
metal. The new number is the more complete drop. No threshold was changed.

## REL-07 acceptance: area (9e2465d → REL-07 + review fixes)
| fixture | area µm² before → after | DRC | LVS |
|---|---|---|---|
| ota | 2001.1 → 1236.9 (−38 %) | 0 | MATCH |
| ota_constrained | 2001.1 → 1236.9 (−38 %) | 0 | MATCH |
| tt_ota | 2001.1 → 1236.9 (−38 %) | 0 | MATCH |

All other fixtures shrink or stay the same (e.g. dac4 1619.5 → 1058.0, tq_chain 9682.4 → 8352.2), and every fixture
stays at DRC 0 with the same LVS as before. The cost: the `Environment` budget goes from 8/8 to 5/8 satisfied (max use
0.534 → 1.174 on tt_ota, Θ 0.523), because the blanket rings no longer surround the devices. `CrosstalkExclusion`
goes from 0/12 to 6/12, and `MatchedSet` use rises from 0.234 to 0.465 (still satisfied).

## REL-10 acceptance (T9)
No local fixture ships a testbench (`bench.rs` `op_config`), so ota runs on the probe:
`bias 2 uW, probe, EM at 300.150 K, aging pairs 2 (max dVds 0.0 mV), V-rating unknown 0`. Aging and voltage_unknown
are therefore shown only on a probe bias. A testbench check needs a fixture that has one. Since review fixes 2,
the `rel/*` signoff rows from a probe op end in ` (probe)` (`reliability::tests::a_probe_op_tags_its_rows`).

## Known ceilings and out-of-scope
- REL-14: the thermal field steps at a source's floor-disc edge by a factor of `ln(4L/W)` (≥ ln4 ≈ 1.39). This is
  documented at `thermal.rs` `rise_at`. It is not blended.
- gp/perf owners: in debug builds, `backend/gp/src/lib.rs:87`'s debug_assert ("a batch kind is registered in both
  `hard` and `budget`") fails `library` `perf_postlayout`, `placement_metrics` and `benchmark` `signoff_fixtures`.
  The failure is already on a8c8f3d. All three pass in `--release`. In debug, `annotator` `scale` (≤ 2 s) failed
  under load; it passes in release.
