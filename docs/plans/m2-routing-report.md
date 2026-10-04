# M2+ routing: segment report

## RTE-10 acceptance (routing-layer DRC 0)
`cargo test --release -p benchmark --test signoff_fixtures -- --include-ignored` on the RTE-10 tree (02297f8, built
from `git archive`): 19 passed. DRC 0 (routing 0, cells/assembly 0), ERC 0 and LVS clean on all 9 `FIXTURES` and on
the 3 slow OTAs (ota, ota_constrained, tt_ota; PEX 716.5 fF each). At review fixes 1 the result is the same: 19 passed,
DRC 0 everywhere, slow OTAs PEX 781.5 fF.

## RTE-11 acceptance: ota_cross_pdk and the gf180/ihp hand-off to RTE-26
`ota_cross_pdk` prints its count only under `--nocapture`. At review fixes 1 it gives sky130 0 and generic_finfet 12
signoff DRC (all decks are listed below). The test covers only those two decks. gf180/ihp were measured with the same
`Ota5T` elaboration plus `signoff_drc` in a scratch test that was not committed:

| deck | DRC | routing-layer | other | route hard |
|---|---|---|---|---|
| sky130 | 0 | 0 | 0 | 8 |
| generic_finfet | 12 | 2 (M1.S.2) | 10 (GATE.W.1.max 5, NSELECT.PSELECT.AUX.1 5) | 8 |
| gf180mcu | 67 | 47 (M1.2a 8, M2.1 9, M2.2a 30) | 20 (DF.2a 5, PL.1 5, PL.2 10) | 9 |
| ihp_sg13g2 | 60 | 60 (M1.b/e/f 4 each, M2.b/e/f 16 each) | 0 | 8 |

RTE-26 owns bringing gf180/ihp routing-layer DRC to 0. The non-routing rows on gf180 and generic are device-layer
issues outside routing.

## Review fixes 1
- RTE-12 EM: a run at or over the wide threshold is drawn as `k` parallel wires, so its copper is `k·wire`, not
  `wire + (k−1)·pitch`. Sizing now gives such runs `k ≥ ⌈EM width/wire⌉` tracks, and the EM check gives a bundle wire
  the bundle's copper. The 1250 µA bundle had been sized at 4×290 = 1160 nm, which really is under its 1260 nm
  need, so the fix also needed this sizing change and the bundle is now 5 wires. The test asserts no budget
  violation.
- RTE-11: `Pdk::load` rejects a sidecar that is missing a cut. `routing_vias` returns one cut per adjacent pair, so
  `routing_stack`'s cut-count `Err` cannot be reached from a loaded deck. The test now reaches the empty-stack and
  cut-is-a-metal `Err`s through a loaded deck that it then edits.

## RTE-14 acceptance (current-driven widths and access)
- `cargo test --release -p benchmark --test signoff_fixtures -- --include-ignored`: 19 passed (DRC/ERC/LVS within
  baseline). `cargo test --release -p analog -p gr -p dr`: all pass.
- `bench local` (seed 1), against the pre-RTE-14 tree (453bcc4, same command): every row has the same WL, overuse,
  DRC, LVS and ERC. Electromigration (hard) is 74 total, 67 sat, **0 V**, 7 unknown (no operating point:
  res_m2's `res_high_po` model and rows without currents). IrDrop Θ = 0 (30/30 sat; RTE-21's repair was not needed
  here). `em access` V: 0 on every circuit, so there is no CELL finding.
- Unchanged and not caused by this item: dac4_mim route hard 1 (Antenna, net top) and ERC 1 (`ar.met4.1`), tq_chain
  ERC 1, and OTA overuse 3165.

Departures from the card:
- Two more existing tests move to quantized widths (Decision 4): `a_multi_finger_terminal_is_not_counted_per_pin`
  400 → 720 (800 µA would give 1150), and `a_two_track_net_reserves_both_tracks` 500 → 720.
- EM-round mapping: a failing metal shape bumps only branches whose drawn width (run, corner, or one bundle wire)
  equals the shape's width, and of those only the narrowest on that layer. A failing cut bumps the narrowest branch
  with a node within a pitch of the cut, since array cuts can sit off the node. Without these rules, access jogs and
  junction corners bumped every branch through them until `K_MAX`.
- `Access.node_ua`: at an MST junction the jog is sized by the largest MST edge that meets at the landed node, not
  only by the pin's own current. The jog lands on that metal, and `net_flow`'s chord bound gives it that current.
  Pin-end cuts still use the pin current. A net with any unknown terminal gets no access sizing, as before.
- `prim` also returns parents. `sky_cell`'s cut limit is per cut only, as `Pdk::em_limit` gives a cut. `test_stack`
  gains sheet resistances so `net_flow` can split current. `a_trunk_carrying_two_branches_is_wider` checks the MST
  widths (1100/680) without a stack, and with the stack it checks only that the EM rounds bring the net to REL-03
  pass: there a's access climbs onto b's met2 run, so that shape carries 2 mA.
