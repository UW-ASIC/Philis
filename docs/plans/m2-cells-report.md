# M2+ cells report

- CELL-15 bench ledger (T6/T4 "order >= 3 for an ota pair"): no bench pair has 8k fingers per member. The only
  equal matched pairs are the OTA inputs XM1/XM2 in `benchmarks/fixtures/{ota,tt_ota,ota_constrained}.spice`,
  nf = 2 each; `mirror_ratio.spice` has nf = 2/4/8 (unequal, not a pair); `benchmarks/field/pwm_driver.spice` has
  no nf. `Mosfet::enumerate` offers rows = 4 only for an equal `Cc1d` pair with `nf % 8 == 0`, so the search never
  sees, and never picks, the four-row variant on the bench; the ledger shows no order-3 pair. Not forced (MAT-15
  Risks: variants only).

- CELL-22 (well bridges over overlapping spans, REL-16-gated). Touching-connected nwell components were counted
  with a temporary edge-touch union-find after `well_bridges` in `Flow::epoch` (not committed). The run was
  `cargo test -p library --release --test flow_smoke --test ota_cross_pdk`, on `m2` (`b4054b6`) and on `cc7643a`.
  - flow_smoke: identical per epoch on both. 2-cell epochs: 0 components (no nwell), 9 times. 3-cell epochs: 2 components, 3 times.
    4-cell epochs: 3 components, 3 times. No REL-16-forced split: the components with every pair allowed equal the gated ones.
  - ota_cross_pdk: not exercised. It draws through `elaborate` (macro_master), which never calls `well_bridges`.
  - DRC: both tests' output is byte-identical between the two commits once timings are removed (sky130 0, generic_finfet
    12 routing-layer violations as dr debt; ota5t signoff 1 hard / 0 lvs). 5/5 pass on both.
  - So the bench shows no new bridge. The unequal-span and forbidden-pair paths are covered only by the `post_cell` unit tests.
