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
