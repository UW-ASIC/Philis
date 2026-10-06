# M2+ cells, segment 12 (M3): CELL-14, CELL-16

Base: `m2-cells` after `git merge m2` (`2676ece`). Line numbers are at that commit. Commands: `export
PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk`, then `Q=/home/omare/Documents/Projects/Rust/Philis/tools/qcargo`
from the worktree. The owner asked for the nix shell, so run them as `nix develop -c $Q ...` (the repo's `flake.nix` dev
shell). Hard dependencies are all on `m2`: GAP-01 (`analog::matching::class::{resistor_env, PassiveEnv}`, class.rs:79-103),
MAT-12 (`analog::matching::pattern::segment_row`, pattern.rs:169-176), EXT-15/19 (`Member.series`, intent.rs:80-85), GAP-07
(`Pdk::model_markers`, backend/verify/src/pdk.rs:1350-1385). PLC-06 is not needed by either item.

## CELL-14 Ratioed resistor arrays: class **do**

### Code facts (plan-03 is out of date)
- `greedy_centroid` is at builder.rs:282-305, and its test `greedy_centroid_uses_every_finger` is at :320-327. Its only
  caller is `res_segment_sequence` (resistor.rs:434-441), which interleaves *equal* per-string counts. The plan's
  "resistor.rs:304-310" is stale.
- `enumerate` (resistor.rs:130-152) uses one segment count for every member (`feasible_segments(&s, …)`, :448-470, from
  `s.unit_w × s.unit_l`). It drops `Interdig && segments > 1 && n_strings > tracks` (:150).
- `draw` (:155-369) uses one `seg_l` (:195-199) and one `total_h` for all columns. Jumpers (:260-279) are li between
  adjacent columns (:266-270) and met1 + 2 mcon on track `g % tracks` otherwise (:271-278). Pins are on li, so mcon is
  drawn only for jumpers.
- Dummies (:296-321) are one per end at `d_pitch = body_w + max(seg_gap, space_between(rpoly, poly))` (:302). The array
  pitch is `seg_pitch = body_w + seg_gap` (:200). `seg_gap` (:389-392) is `max(res_seg_gap 400, space(li))`, so a
  dummy's pitch differs from the active pitch.
- `Unitization.series` exists (kernel/analog/src/cell.rs:72-73, empty = all 1). `resistor_env(c, p)` gives
  `min_dummies: 1` and `dummy_span_nm` from tier `res_dummy_span_nm = [0, 0, 10000]` (pdks/sky130.json:25).
- `res_generic_po`: `res_head_mohm_um 0`, `res_dl_nm 0` (sky130.json:213-216). `res_high_po`: `dl 247`
  (:188-191). `res_value_tol_ppm 5000` (:126).
- **Out-of-scope blocker for the rdiv acceptance (annotator, `backend/annotator/src/constraints.rs:58-94`):**
  `assemble` skips any set whose members differ in drawn L (`same`, :61-64), so rdiv (RA L=10 µm, RB L=40 µm) emits
  no set Unitization. Each resistor then becomes its own block unitization (:105-127). Matched sets also always emit
  `series: vec![1; n]` and `SeriesParallel::Parallel` (:86, :92). Nothing on `m2` puts `Member.series` (filled at
  lib.rs:370-373 and checked by corpus.rs:162, `("RB", 1, 4)`) into `Unitization.series`. The annotator owner must
  change this so rdiv is drawn as one array: when `s.unit` is `Some`, skip `same`, use `unit_w/l = s.unit.{w_nm,l_nm}`,
  `dev_nf = member.parallel`, `series = member.series`, and `Series` when any series > 1. Until that lands, CELL-14 is
  tested on hand-built Unitizations, and the rdiv end-to-end acceptance (step 8) stays open.

### Edits
1. builder.rs: delete `greedy_centroid` (:282-305) and its test (:320-327). Owned by C27.
2. resistor.rs imports: drop `greedy_centroid`. Add `use analog::matching::{class::resistor_env, pattern::segment_row};`
   and `use pnr_core::MatchClass;`.
3. New helpers (resistor.rs, near `strings`):
   ```rust
   /// Per member of `group`, its series unit count (`series_d = L_d / L_u`, EXT-15) when the covering unitization
   /// composes `Series` units with a non-empty `series` over ≥ 2 members; `None` for an equal-length group.
   fn series_counts(group: &DeviceGroup, c: &Constraints) -> Option<Vec<u16>>
   /// The unit length L_u′ that makes the smallest member exact, `seg_len(body_w, ohm(unit_w, ser_min·unit_l)/ser_min, 1, 0, lat, i32::MAX)`;
   /// `Some` only when every member passes |ser_d·ohm(body_w, L_u′) − ohm(unit_w, ser_d·unit_l)| ≤ res_value_tol_ppm·ohm(…)/1e6
   /// (plan-03 value rule). `None` without a `ResModel`.
   fn unit_len(s: &Sizing, series: &[u16], process: &dyn Process) -> Option<i32>
   ```
   Change `feasible_segments(s: &Sizing, …)` to `feasible_segments(w_nm: i32, l_nm: i32, process: &dyn Process) ->
   Vec<u16>`. The body is unchanged, with `s.unit_w/s.unit_l` replaced by the arguments. Update the existing caller.
4. `enumerate`: if `series_counts` is `Some(ser)`, return `[Resistor { segments: 1, pattern: Single }]` (blocks).
   If also `unit_len(..).is_some()` and `group.devices.len() <= jumper_tracks(process).len()`, add `Resistor {
   segments: 1, pattern: Interdig }` (the series-unit array). Otherwise keep today's path, and widen the filter at :150
   from `segments > 1` to any variant whose sequence has a non-adjacent join (Interdig, segments > 1).
5. `draw`: add a per-member plan `cols: Vec<(i32 /*n*/, i32 /*seg_l*/)>`:
   - equal-length group: `(segments, seg_l)` as today, for every member;
   - series + `Interdig`: `(ser_d, unit_len)`;
   - series + `Single` (blocks): `n_d` = largest of `feasible_segments(unit_w, ser_d·unit_l)` that is ≤ ser_d (else 1),
     and `seg_l_d = seg_len(body_w, ohm(unit_w, ser_d·unit_l), n_d, 0, lat, i32::MAX)`.

   Then use per-column heights: `total_h = 2·head + seg_l[di]` for that column's poly, rpoly, keepout, unit, li heads,
   top cuts and `end_cut`. Node ids keep `j·n_d + k`. `res_segment_sequence(counts: &[u16], pattern) -> Vec<usize>`
   takes per-string counts: `Interdig` returns `segment_row(counts).0`, otherwise each string's segments stay contiguous.
   rpm, npc and res_implant use the max `total_h`. res_block: one rect per member run, each extended to the midpoint of
   the gap to its neighbour (one stepped union, no inner spacing); with equal `seg_l` this is today's single rect.
6. Jumpers: before the loop, set `all_met1` = some string has two consecutive slots more than one apart. The li arm
   (:266) becomes `Some(px) if !all_met1 && sx - px == seg_pitch`. Every other join uses the met1 track arm, so every
   join gets exactly 2 mcon.
7. `seg_gap` (:389): `snap(max(res_seg_gap, space(li), space_between(rpoly, poly)))` (480 on sky130, poly.9). Delete
   `d_pitch` (:302). Dummies per end: `k = env.min_dummies.max(ceil(env.dummy_span_nm / seg_pitch))`, where
   `env = resistor_env(unitization(..).and_then(|u| u.class).unwrap_or(MatchClass::Moderate), process)`. The i-th
   dummy (1-based) is at `-i·seg_pitch`, and at `(n_cols-1+i)·seg_pitch`. Its width is `body_w`, except Minimal, which
   uses `res_min_width.max(width(poly))`. Its inner edge stays `seg_gap` from the end body, and it takes the height of
   the end column next to it. Each dummy keeps its `GND` pin. Keep the `dummies` condition (:299). `(lo, hi)` come from
   the outermost dummies.
8. Fixture `benchmarks/fixtures/rdiv.spice`, as plan-03 gives it. Its end-to-end LVS test waits on the annotator edit
   above (report it; do not edit constraints.rs from cells).

### Tests (kernel/cells/src/resistor.rs `mod tests`)
Helpers: `generic_po(pdk)` = `Overlay { recipe: pdk.recipe("resistor", "res_generic_po") }`, and `series_group(ser:
&[u16], w, l, class) -> (DeviceGroup, Constraints)` (Series, `dev_nf` all 1, `series = ser`, `dummy_required: true`).
- `ratio_arrays_are_common_centroid`: generic_po, W 690, unit L 10 000, Moderate. For counts `[1,2] [2,3] [1,4] [3,2]
  [4,5] [9,4]`, the `Interdig` variant exists. For member A=0 and B=1: `n_B·Σ_{u∈A} u.x == n_A·Σ_{u∈B} u.x` (i64 over
  `m.units`), and the `m.units` count per owner == ser_d.
- `high_po_ratios_fall_back_to_exact_blocks`: high_po, W 690, unit L 10 000, series `[1,4]` → every variant is
  `Pattern::Single`. Σ `model.ohm(d.w, d.l)` over `m.drawn` with owner 0 is within 0.5 % of 5 129.7 Ω, and owner 1 of
  18 949.2 Ω (`R_690_40`).
- `dummies_sit_at_the_segment_pitch`: generic_po `[1,4]`, Moderate. The x of every poly rect (bodies and dummies),
  deduped and sorted, has a single difference equal to `690 + seg_gap(&op)`. Exceptional: the dummy count per end (poly
  columns left of x = 0) == `ceil(10_000 / pitch)`.
- `every_join_has_the_same_via_count`: for generic_po `[1,4]` Interdig, and high_po `variants(2, 1)` with segments == 2
  Interdig, the mcon rect count == 2·Σ_d(n_d − 1), and no li rect is wider than `body_w`.
- `each_member_s_current_directions_cancel`: generic_po `[2,4]` Interdig, so Σ `u.phi.1` per owner == 0.
- `every_variant_is_drc_and_erc_clean` (existing): add generic_po and high_po series groups `[1,4]` and `[2,3]`
  (both classes Moderate/Exceptional) through `testkit::findings`/`ports_with`, the same as the high_po loop at :497-505.
- Run: `nix develop -c $Q test --release -p cells resistor`, then `nix develop -c $Q test --release -p cells`,
  `nix develop -c $Q test -p library --test drawn_cards`.

### Acceptance status
T6 (common centroid) and T4 (value) for resistor sets are met on hand-built sets. "rdiv drawn as one array, LVS
MATCH" waits on the annotator emission edit above. Report it as escalated with this evidence; do not mark it met.

## CELL-16 MOS flavours, Vt markers: class **do**

### Code facts
- `Pdk::model_markers(model) -> Option<(Vec<LayerId>, Vec<LayerId>)>` (pdk.rs:1350-1385, GAP-07) returns the
  recogniser's required and forbidden layers beyond the gate, e.g. `nfet_01v8_lvt` → `([lvtn], [hvi])`. These are
  `LayerId`s. `Recipe.layers` is `(role, deck layer name)` (pdk.rs:1289-1296), so map each id back to its name through
  `pdk.layers: Vec<(String, LayerId)>` (pub, :56).
- `DRAWN_ROLES` is at pdk.rs:1489-1490 (the plan says 1211-1212). `Overlay::layer` (:1419-1425) resolves recipe roles
  first and falls through to `Pdk::layer` for any role that is not a resistor or capacitor role. `enclosure`/`width`/
  `area` go through `self.layer`, so `gate_marker{i}` resolves on the overlay. Base `Pdk::layer("gate_marker0")` is `None`.
- cellgen `draw_variants` is at frontend/library/src/cellgen.rs:859. The planar MOS arm is :865-871
  (`draw_all::<Mosfet>(group, c, pdk)`, then `taps_in_reach(m, pdk)`). The plan says :796.
- mosfet.rs `draw_row` (:343-860): the first active gate is at x = `sd_edge`. `gates_end` (:450) is its right edge
  after the last finger. `nd` dummies per edge: left x `-(k+1)·dummy_l − k·sd_end`, right x `gates_end + sd_edge +
  k·d_step`, width `dummy_l` (:710-714). Gates span y 0..`finger_w` on diff. The PMOS nwell block is :824-845, and
  `b.finish()` is :847. No marker is drawn today.

### Edits
1. cellgen.rs MOS arm (:865): `let ov = mos_overlay(pdk, model);`, then `draw_all::<Mosfet>(group, c, ov.as_ref().map_or(pdk as &dyn Process, |o| o))`.
   Use a new private fn in cellgen.rs:
   ```rust
   /// `model`'s recogniser markers (GAP-07) that the planar generator does not already draw, as `gate_marker{i}`
   /// roles over `pdk`; `None` when there are none (plain `nfet_01v8`).
   fn mos_overlay<'a>(pdk: &'a Pdk, model: &str) -> Option<verify::pdk::Overlay<'a>>
   ```
   It removes layers that `pnr_core::Process::layer(pdk, r)` gives for roles `diff, tap, poly, licon, li, mcon, met1,
   nwell, nsdm, psdm, npc`, and maps the rest to names via `pdk.layers`. It builds `Recipe { model: model.into(),
   layers: [("gate_marker{i}", name)], rules: vec![] }`. Forbidden layers are ignored, because the generator never
   draws them. `taps_in_reach` keeps `pdk`.
2. mosfet.rs `draw_row`, before the nwell block (:824): for `i in 0..` while `process.layer(&format!("gate_marker{i}"))`
   is `Some(l)`, compute `g = max(enclosure(role, "poly"), enclosure(role, "diff")).unwrap_or(0)`. Take the rect
   spanning x from the leftmost gate (the outer left dummy if `nd > 0`, else `sd_edge`) to the rightmost gate's right
   edge, and y from 0 to `finger_w`, and grow it by `g` on every side. Widen it symmetrically to `width(role)`. Then
   raise `w` until `w·h ≥ area(role)`. Draw it on `l`. It is drawn once per `draw_row` call, so once per row.
3. Not handled here: marker-to-marker spacing between neighbouring cells, which belongs to placement (PLC-07). The
   `hvi` 5 V devices are deferred as plan-03 says.

### Tests
- mosfet.rs `a_marker_covers_every_gate`: sky130. Build the overlay by hand from `pdk.model_markers("nfet_01v8_lvt")`
  (assert `Some` and non-empty required). Use `testkit::group_of(Nmos, 2, 4, 1000, 150)` with `dummy_required = true`.
  For every variant, the macro has exactly one rect per row on the lvtn layer. It contains every poly rect's
  intersection with the diff rect, grown by 180 on each side (assert that `enclosure` resolves to 180, not 0).
  Plain `nfet_01v8` (no overlay) draws no lvtn rect.
- mosfet.rs `flavoured_devices_are_drc_and_erc_clean`: for each pair of `nfet_01v8_lvt` with Nmos and `pfet_01v8_hvt`
  with Pmos, check every variant with `testkit::findings(&m.shapes, &testkit::ports_with(&m, &["G","S","B"]), &pdk)`
  is empty.
- New `frontend/library/tests/flavours.rs::an_lvt_and_an_hvt_device_sign_off_with_their_models`: use netlist
  `.subckt f VDD VSS a b\nXM1 a b VSS VSS sky130_fd_pr__nfet_01v8_lvt w=1u l=0.15u\nXM2 b a VDD VDD sky130_fd_pr__pfet_01v8_hvt w=1u l=0.15u\n.ends f`,
  `library::run` with `Config { feedback_iters: 1, .. }`. The report rows come from
  `verify::signoff_with_intent(&shapes, &pins, &reference, &sol.intent, &pdk)` as in drawn_cards.rs:63-70, with no
  `engine/*` rows. Assert 0 `lvs/` rows and 0 `drc/` rows. Also assert `verify::extract_spice(&shapes, &[], &pdk,
  Detail::Schematic)` contains `nfet_01v8_lvt` and `pfet_01v8_hvt`, so the test fails if the markers are missing even
  when LVS ignores model names.
- Run: `nix develop -c $Q test --release -p cells mosfet`, `nix develop -c $Q test --release -p library --test flavours`.

### Acceptance
The flavour fixture is LVS MATCH with the right models, and the extract assertion checks the models.
