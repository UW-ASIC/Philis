# M2+ cells report

- CELL-15 bench ledger (T6/T4 "order >= 3 for an ota pair"): no bench pair has 8k fingers per member. The only
  equal matched pairs are the OTA inputs XM1/XM2 in `benchmarks/fixtures/{ota,tt_ota,ota_constrained}.spice`,
  nf = 2 each; `mirror_ratio.spice` has nf = 2/4/8 (unequal, not a pair); `benchmarks/field/pwm_driver.spice` has
  no nf. `Mosfet::enumerate` offers rows = 4 only for an equal `Cc1d` pair with `nf % 8 == 0`, so the search never
  sees, and never picks, the four-row variant on the bench; the ledger shows no order-3 pair. Not forced (MAT-15
  Risks: variants only).
