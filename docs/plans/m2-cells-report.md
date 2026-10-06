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

- CELL-12 (class-driven dummies, LOD moat, gate overhang, gate straps).
  - Deviation from the card (gate overhang). The card says `skirt_y = finger_w + ext - 10` and a `licon_y` floor of
    `ext - lat + licon_poly_side`. The code uses `skirt_y = finger_w + ext` (mosfet.rs:735) and a floor of
    `ext + licon_poly_side` (:690), and draws the dummy poly `lat` taller (`h = finger_w + 2·ext + lat`, :713) so it
    laps the skirt. The card's `-10` skirt would intrude into the rule-21 band, where every gate and dummy runs
    straight for `ext`. This applies to every dummied row, class-less ones included (`ext = poly_ext`): compared with
    `35ca3a6`, the skirt is 10 nm higher, the dummy cut-row floor is `lat` higher, and each dummy poly is `lat` taller.
  - Bench: `cargo run --release -p benchmark --bin bench` (local suite, sky130, seed 1). The before run used
    `35ca3a6`'s mosfet.rs on `df90dc8`, which touches only mosfet.rs and the cards. Only these rows changed:

    | fixture | area µm² | active % | other |
    |---|---|---|---|
    | ota, ota_constrained, tt_ota | 1790.1 → 2850.1 | 33.8 → 18.9 | util 94.3 → 71.2; overuse 31181 → 6274; route hard 42 → 34; **ERC 0 → 22; matched mismatch 0 → 1**; LVS MISMATCH both |
    | mirror_ratio | 94.0 → 134.9 | 19.7 → 21.8 | route hard 14 → 15; LVS MATCH both |
    | tq_chain | 9388.9 → 8911.4 | 0.4 → 0.5 | DRC 56 → 12; ERC 4 → 0; LVS MISMATCH both |

    bgr_core, bjt_mirror, chain4, dac4, dac4_mim, pair, quad, rc_filter and res_m2 are identical.
  - Re-baseline (audit Q7): the OTA area grows 59% (Moderate: 7 dummies per end, 5 µm moat, +1 µm overhang) and
    its active fraction falls. The 22 new OTA ERC violations are all `erc/well_bias:pfet_01v8`
    (`target/bench_debug/ota/violations.txt`): 22 pfets, presumably the new PMOS dummies, sit in an nwell the check
    finds unbiased. That, and the matched mismatch 1, need triage before this becomes the new bench baseline.
