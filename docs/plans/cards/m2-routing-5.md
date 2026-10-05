# M2+ routing, cards 5 (M3: RTE-16, RTE-18, RTE-15, RTE-17, RTE-19, RTE-20)

All six items are already implemented and on `m2` (segment 3 of the earlier run; cards in
`m2-routing-3.md`, results in `m2-routing-report.md` "Segment 3" and "Review fixes 3"). This card does not
re-spec them. It lists what is left now that EXT-24/EXT-18/EXT-19 have merged. Merge of `m2` at 0f84944.

Acceptance commands (all items):
`PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk cargo test --release -p dr -p analog` and
`cargo test --release -p benchmarks --test routing_quality -- --include-ignored`; `signoff_fixtures` stays 19 passed.

## RTE-16 (af2a8a8): done
- `kernel/analog/src/routing/metal_over_gate.rs`, `Blockage { rect, halo }` in dr/gr, library `elaborate.rs`.
  `no_metal_over_gates` passes on ota and tt_ota (0 `metal over gate`, 0 `open net`).
- Left: nothing. CELL has no `KeepWhy::Gate`, so gates come from poly ∩ diff (out of scope, CELL).
- Class: verify only (run the commands above).

## RTE-18 (407ce25): done, two results not met
- Separation in the search (dr:1050-1057 `separations`), crossing C (`coupling.rs`, `Pdk::overlap_af_um2`),
  aggressor weights from EXT-18 (`CouplingBudget::default_weights`, library lib.rs:1419), screening.
  EXT-24's `CrosstalkExclusion`s reach the search through `RuleBatch::separations`: the step-level deps are wired.
- Not met (stays red): `crosstalk_exclusions_hold` 2/4 violated (gate–drain access pads 430/806 nm apart, placed
  that close by the cell). `coupling_tracks_pex` off by 45-99 %: the model has no device or terminal C. PERF-16
  owns the PEX part, and the pad spacing is a CELL/placement issue. Neither is in this item's scope.
- Class: verify only.

## RTE-15 (c063c04): done, bench measure waits on PLC-28
- `pairs_report`: 0/1 exact on ota/ota_constrained/tt_ota, `(1, 5, "axis off lattice")`. Fallback works
  (`unmatched_pins_fall_back_to_the_guide`, `a_shifted_pair_never_meets_its_image`).
- The T5 bench acceptance (exact mirrored pairs on the bench) needs PLC-28 axis snapping (not merged:
  `git log m2 --grep PLC-28` is empty). PLC-21 Shift map is not merged either; the code falls back without it.
- Class: defer the bench acceptance until PLC-28 merges, then rerun `pairs_report`. No code to write.

## RTE-17 (797ba8b): step 4 for stars and Kelvins is still open (EXT-24 is now on m2)
Current code: `Flow::common_nodes` (frontend/library/src/lib.rs:1611-1641) maps each `intent.common_nodes`
request to `CommonNode { groups: [pins(a), pins(b)], star: false }`. The note at lib.rs:1636 says
`intent.stars`/`intent.kelvins` (kernel/analog/src/intent.rs:263-274, 295-296) are not mapped yet. EXT-24
fills them (backend/annotator/src/extract.rs:242-304). dr already routes `star` nodes as pseudo-nets and handles
Kelvin (`a_kelvin_sense_branch_starts_at_the_tap`, dr:3786).

Edits, all in `Flow::common_nodes`:
1. Change the pin lookup to `(DeviceId, Term)` pairs:
   `let pin_of = |net: NetId, d: DeviceId, t: Term| on_net[net.0 as usize].iter().filter(move |p| p.0 == d && p.1 == format!("{t:?}")).map(|p| p.2);`
2. Collect `let starred: HashSet<NetId> = intent.stars.iter().map(|s| s.net).collect();`. In the
   `common_nodes` loop, skip a request whose net is in `starred` but keep its `max_delta_ohm` in
   `min_ohm[net]`. Take the smallest value > 0. If none is > 0, use 0.0.
3. For each `StarReq { net, root, branches }`: `groups` = one group per branch, `vec![pin_of(net, d, t)…]`.
   `feeds` = `root` pin(s) if `Some`, otherwise every pin on the net that is in no branch (same as `feeds`
   today). `max_delta_ohm = min_ohm.get(net).unwrap_or(0.0)`, `star: true`. Skip a star with fewer than 2
   non-empty groups.
4. For each `KelvinReq { device, term, sense }`: `net` = the device terminal's net (`self.netlist.devices[d]
   .terminals`). `feeds = pin_of(net, device, term)` (the tap). The force group is every other pin on the net
   that is not in `sense`. `groups = vec![force, sense_pins]`, `max_delta_ohm: 0.0`, `star: true`. If the
   device terminal has no placed pin, skip it.
5. Replace the lib.rs:1636 note with a one-line doc of the precedence: a star supersedes the net's
   common nodes, and Kelvin is step 3.
   `balance_field` stays. Matched groups still do not route jointly through RTE-15 on the bench (0 exact).

Tests:
- `frontend/library` unit test `a_star_request_supersedes_the_common_node`: build a `Flow` on ota (or the
  existing helper used by the `common_nodes` tests, if one exists), push a `StarReq` on the net of the first
  `intent.common_nodes[0]`, and call `common_nodes`. Assert exactly one node on that net, `star == true`,
  `groups.len() == branches.len()`, and `max_delta_ohm` equal to the superseded request's value.
- `a_kelvin_request_makes_force_and_sense_groups`: a `KelvinReq` on a resistor → one node with
  `star && max_delta_ohm == 0.0 && groups.len() == 2 && feeds.len() >= 1`.
- Bench `common_nodes_meet_allowance` (routing_quality.rs:139): also assert `star break` violations == 0 for
  every EXT-requested star/Kelvin. If ota has none, print the count (vacuous) and say so in the report.
- Command: `cargo test --release -p library common_node` plus the acceptance commands.
- Class: do.

## RTE-19 (ad40d7a): coverage not met (0.768 < 0.95)
Current code: `add_shield` (backend/dr/src/lib.rs:2671-2720) claims side stretches only for same-layer runs of
≥ 2 nodes (`filter(|r| r.len() >= 2)`, `stretch.len() >= 2`). `Shield::coverage`
(kernel/analog/src/routing/shield.rs:32) counts every non-square victim shape. The victim's end jogs (single-
node or perpendicular runs) count in the total but get no claims. The trunk alone is 0.93 covered.
The EXT-24 `shield_ref` step-level dep is wired: extract.rs:172-197 gives `Shield.reference = shield_ref`.

Edits (judgment: find the cause first, then fix it):
1. Measure first. In `shield_run`, list each victim shape with its covered and total length, so you know
   which shapes miss coverage: the jogs, the pads, or the trunk ends next to the pin access.
2. If the jogs miss it: in `add_shield`, also claim side nodes of 1-node runs and of runs on the
   perpendicular layer. Keep `stretch.len() >= 1` for claims tied into a longer claim of the same side, so
   a claim is never a lone floating node. Keep the side tracks of the jogs free during the search, the same
   way the trunk's are (the guard set before PathFinder, card 3 step 1).
3. If the trunk ends miss it because a pin's access blocks the side track, record that this is a geometric
   limit with numbers. Do not redefine coverage to the trunk alone, because that loosens the rule.
Tests: `a_shielded_victim_gets_both_side_tracks` must reach `cov >= 0.95` with the assertion unchanged.
`a_shield_does_not_count_as_an_aggressor` and `a_shield_request_draws_tied_reference_tracks_both_sides`
(≥ 0.75) must stay green, with no new `open net`.
Command: `cargo test --release -p dr shield`.
If 0.95 cannot be reached, report the measured value and the shape breakdown, and leave the test red.
- Class: do.

## RTE-20 (251cf75): done
- `plate_ratio.rs`, dr `equalize_leads`, library `plate_sets` from EXT-19 capacitor Unitizations (≥ 3 members).
  `dac4_plates` passes with no crossing. The ratio spread is vacuous: the sky130 MIM X-cards carry no `c_af`
  (PDK data, out of scope).
- Class: verify only.
