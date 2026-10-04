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

## Review fixes 2
- Access jog EM floor: new `a_blocked_full_jog_never_falls_back_below_em` blocks the full jog so only the narrow one
  keeps spacing; it fails if add_pin_access's `.max(need)` clamp is removed. Landing's clamp is still not reached by a
  test. It is defensive only: add_pin_access drops a landing choice narrower than `need`.
- `claim_jog_sweep` claims at least `access_need(node_ua)`, and is re-run once the MST junction current is known.
  Landing's clearance check still sees only the terminal's own current, because the junction depends on every
  landed node. The wider corridor is claimed first-come, and add_pin_access re-checks it against routed metal.
- EM rounds: a failing shape on a branch that ends off every terminal now bumps each terminal at the net's `k` on that
  layer (Card Decision 1), instead of being dropped.
- gr: `term_k_targets_route_in_order_at_their_own_width` covers MST order and the per-target search width. Each of the
  two reviewed mutations makes it fail.
- Tests: `cargo test --release -p analog -p gr -p dr` all pass; `signoff_fixtures --include-ignored`: 19 passed.

## Segment 3 (RTE-16, RTE-18, RTE-15, RTE-17, RTE-19, RTE-20)
Acceptance: `benchmarks/tests/routing_quality.rs` (`--release -- --include-ignored`); `signoff_fixtures` stays 19
passed after every item.
- RTE-16: `no_metal_over_gates` passes on ota and tt_ota with 0 `metal over gate` and 0 `open net` (the run with no
  blockages also had 0 open). Departures: `Blockage` has `rect` as drawn plus a `halo` (the library sets the
  spacing), so the V checks the real gate and not the grown rect. A pin's stitch node inside a hard blockage is
  closed to the nets the blockage binds. Same-net fill keeps off hard blockages. A via array narrower than one
  enclosed cut is no longer placed (`(w − size)/pitch + 1` truncated to 1 and drew via.4a on quad). Out of scope:
  CELL has no `KeepWhy::Gate`, so gates come from poly ∩ diff.
- RTE-18: FLOW file `verify/src/pdk.rs` gains `Pdk::overlap_af_um2` (sky130 147.6/88.5 aF/µm²). Departures: no `gr::Separation`
  struct; `RuleBatch::separations` tuples are used directly. A separation box covers along-track nodes too, so
  corners count. It also keeps off the other net's terminals and is waived within its own terminals' box, because
  closer pins belong to the cell and walling them in caused opens in `emit_roundtrip`. `Solution` gains
  `routing`, `route` and `route_stats` for the bench. **Not met:** `crosstalk_exclusions_hold` gives 2/4 violated on
  ota, ota_constrained and tt_ota (gate–drain pairs whose access pads are 430/806 nm apart at pins the cell puts that
  close; the floor is larger). `coupling_tracks_pex` is off PEX by 45–99 %: the model is routed+cell metal only, and
  the PEX rows include device and terminal C (vbias 27.6 fF vs 1.3 fF). PERF-16 owns this.
- RTE-15: `pairs_report`: ota/ota_constrained/tt_ota 0/1 exact, `(1, 5, "axis off lattice")`. This waits for PLC-28
  axis snapping. Departures: the mirror axis is taken from both pin sets' x extents, so a set shifted as a whole is an
  off-lattice mirror; the "no pin map" test therefore moves one pin. Via arrays snap to the nearest grid point with
  ties broken toward the original cut, so mirrored arrays stay mirrored. Not priced for the partner: its via halos
  and separations.
- RTE-17: `common_nodes_meet_allowance`: ota 2 nodes, 0 violated, 2 unknown (no allowance without an op), so the
  check is vacuous. Star/Kelvin nodes have no producer until EXT-24. Departure: a star's branch pseudo-nets keep one
  free track between them, because same-net fill merged adjacent branches.
- RTE-19: **not met:** `a_shielded_victim_gets_both_side_tracks` measures 0.768 against the asserted 0.95. The victim's
  end jogs and pads count in its length and have no track beside them. The trunk alone is 0.93 covered. The test
  stays red and the assertion was not lowered. `add_shield` now drops its shortest claim and retries instead of
  giving up all-or-nothing (before this, the 2-node claims sank the whole set: coverage 0). Bench circuits with a
  clock: none yet.
- RTE-20: `dac4_plates` passes. No crossing; spread unknown because the sky130 MIM X-cards carry no `c_af`, so the
  ratio check is vacuous. The sets come from capacitor Unitizations with ≥ 3 members (EXT-19).

## Review fixes 3
- RTE-19: `a_shield_does_not_count_as_an_aggressor` was vacuous (both sides 0). It now asserts the shield is drawn,
  the excluded reference counts 0 while an included one counts > 0, and coupling with the shield is strictly below
  coupling without it (net 2 screened).
- RTE-17: `star_broken` cuts the grown feeds out of each shape and keeps the remainder, instead of dropping every
  shape that touches the halo. New `branches_leaving_one_trunk_apart_break_the_star` (branches leave one trunk at
  x = 2 and 4 µm) was a false negative before.
- RTE-20: the crossing check exempts a bit/top overlap only when the overlap rect lies inside the array.
  `equalize_leads` now runs before `score()`, so every scored row sees the stubs. The plate rows are appended after.
  The repair loop's `PlateRatio` still sees routes without stubs, because the stubs are added after repair.
- RTE-15: a Shift has no side. The leader's search now refuses a node whose image or pre-image is in its tree. After
  each branch, a path node that clashes with the tree or with an earlier path node is forbidden, and the branch is
  searched again. The forbidden set is static, so A* stays sound. After 64 forbidden nodes the search returns `None`
  and the pair falls back. This checks nodes only, not wide footprints. New `a_shifted_pair_never_meets_its_image`
  fails without the fix (2 clashing nodes). `RouteStats::pairs_exact` now lists the exact pairs. `pairs_report`
  asserts that each exact pair's Differential worst usage is 0.0 on its own two nets. Foreign coupling is taken out
  because it is not the image's to match. With 0 exact pairs the check is still vacuous on these fixtures.
- RTE-18: `separated_nets_never_share_adjacent_tracks` uses 1 000 nm as the card specifies, and passes. The
  `sky130_overlap_c_matches_the_deck` doc is now separate from `sky130_tiers_are_read`'s.
- Still open: RTE-19 coverage 0.768 < 0.95 (unchanged and not lowered). Fixing it needs guard tracks along the
  landing jogs. Redefining the coverage to the trunk alone would loosen the rule. Also open: RTE-18 crosstalk 2/4 and
  PEX 45–99 % (PERF-16).
- Debug-profile `library` tests trip gp's `bind` debug_assert (a batch kind in both hard and budget). It is outside
  routing and not reached in `--release`, where all library tests pass.
