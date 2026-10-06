# M2+ cells, segment 7 (M6): GAP-11, GAP-14

Worktree `philis-m2/cells`, branch `m2-cells`, fast-forwarded to `m2` at `90d92e1` (no conflicts).

| Item | Class | Why |
|---|---|---|
| GAP-11 | do | DAG hard deps GAP-01, EXT-16 are done: `Unitization.class` is set from the set (`backend/annotator/src/constraints.rs:90`), `MatchSpec.kind` exists (`kernel/analog/src/intent.rs:105`). Only the `kind` field is missing; it is added here (plan step 1). No PLC-06. |
| GAP-14 | do | Hard deps done: EXT-26 sidecar (`backend/annotator/src/sidecar.rs`), CELL-17 band well (`post_cell.rs:286`). The step-level PERF-18 dependency is already met: the sky130 deck binds an nfet's bulk to `pwell_iso = dnwell not nwell` (`pdks/decks/sky130.deck:126, 542, 574`), so no `lvs-coverage/unverified` marking is needed (plan step 3 is stale). |

Commands (`export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk` first):
`cargo test -p analog`, `cargo test -p annotator`, `cargo test -p cells post_cell`, `cargo test -p library --lib cellgen`,
`cargo test -p library --release --test extra_devices`, then `cargo build --workspace --all-targets` (new struct fields).

---

## GAP-11 Aspect-ratio limits for matched variants. Class: do

### Code facts (plan corrections)
- `Unitization` (`kernel/analog/src/cell.rs:49-71`, `#[derive(Clone)]`, no `Default`) has `class: Option<MatchClass>`
  (:66) but no `kind`. `MatchKind { Voltage, Current, Ratio }` is `analog::matching::mismatch::MatchKind`
  (`mismatch.rs:21`), re-exported as `analog::intent::MatchKind`.
- The only producer with a kind in hand is `backend/annotator/src/constraints.rs:78-93` (`s.kind`, a `MatchSpec`).
  The other 18 `Unitization {` literals (constraints.rs:114, :142; cellgen.rs:559, :594, :1458, :2020;
  `kernel/cells/src/{lib.rs:73, mosfet.rs:902, cap_array.rs:623, capacitor.rs:386}`; `kernel/macroMaster/src/adapter.rs:29`;
  `examples/three_stage_opamp/src/main.rs:96`; tests `kernel/cells/tests/{cell_selfcheck.rs:45,417, deck_keys.rs:76}`,
  `frontend/library/tests/drawn_cards.rs:27`) get `kind: None`. cellgen.rs:125 uses `..u.clone()` (no edit).
- Stale: "closest to square" is `folds` (`frontend/library/src/cellgen.rs:640-664`), which picks the fold factor
  before any variant exists; it does not see class. `draw_variants` (`cellgen.rs:796`) receives `Constraints` but not
  the unitization it draws for. The filter belongs in `enumerate_folded` phase 1 (`cellgen.rs:87-163`), where `u`
  (the unitization) and its drawn `alternatives` meet, after the `alternatives.retain(..)` at :152-155.
- `Unit` (`kernel/core/src/units.rs:17-32`) holds a centre `(x, y)` and `owner`, no extent. A member's footprint is
  therefore measured on the macro's unit grid: `pitch_x = bbox.w / #distinct unit x`, `pitch_y = bbox.h / #distinct
  unit y` over all units, member footprint `(xmax − xmin + pitch_x) × (ymax − ymin + pitch_y)` over its own units.
  This is H13-46's "only the subarray counts" for any generator that tiles units on a grid (MOS rows, cap arrays).
- `Problem.missing: Vec<(&'static str, &'static str)>`; the flow builds cells at `frontend/library/src/lib.rs:2032-2036`
  with `problem: &mut Problem` in hand. `Cells` is built only at cellgen.rs:203 and destructured only at lib.rs:2032.
- No ALRC "compactness" row exists in the code yet (grep: none); plan acceptance via PERF-23 cannot be checked here.
  Report it, do not invent the row.

### Edits
1. `kernel/analog/src/cell.rs`: after `class` add
   `/// Match kind of the set (EXT-15/16 from the set); None = unknown, no aspect limit (GAP-11).`
   `pub kind: Option<crate::intent::MatchKind>,`. constraints.rs:78 literal: `kind: Some(s.kind),`. Every other
   literal listed above: `kind: None,`.
2. `frontend/library/src/cellgen.rs`, new private fns near `draw_variants`:
   - `fn aspect_limit(class: Option<MatchClass>, kind: Option<MatchKind>) -> Option<f64>`: class
     `unwrap_or(Moderate)` (C16); `Current` → Minimal 10.0 / Moderate 3.0 / Exceptional 1.5; `Voltage` → 3.0 / 1.5 / 1.5;
     `Ratio` or `None` → `None` (Hastings rule 9 is for transistors; R/C ratio sets have their own pattern rules).
     Doc: H13-46 (L42504–42516, PDF 714); the 1.5 for "square or nearly square" is **[policy]**.
   - `fn member_aspect(m: &Macro) -> f64`: max over owners of `max(w,h)/min(w,h)` of the footprint above; `1.0` when
     `m.units` is empty or a dimension is 0.
   - `fn keep_compact(alts: &mut Vec<Macro>, limit: f64) -> bool`: if any `member_aspect ≤ limit`, `retain` those and
     return `true`; else keep only the lowest-aspect one (first on ties) and return `false`.
3. `enumerate_folded` phase 1, after the retain at :152 and its `is_empty` check: when
   `let Some(lim) = aspect_limit(u.class, u.kind)` and `!keep_compact(&mut alternatives, lim)`, `aspect_missed += 1`
   (a `let mut aspect_missed = 0usize` before the loop). Single-device cells are not filtered (no subarray).
4. `Cells` gains `/// Matched cells where no variant met the GAP-11 aspect limit (the squarest was kept).`
   `pub aspect_missed: usize`; set at :203. lib.rs:2032 destructures it and, when `> 0`,
   `problem.missing.push(("MatchClass", "no variant meets the aspect limit"))` (deduplicated like other notes).

### Tests (`frontend/library/src/cellgen.rs` `mod tests`; synthetic macros, no PDK)
- `current_mod_filters_4_to_1`: macro A, one owner 0 with 4 units at x = 500, 1500, 2500, 3500, y = 500, bbox 4000×1000
  (aspect 4.0); macro B, 2 units at x 500, 1500, bbox 2000×1000 (aspect 2.0). `assert_eq!(member_aspect(&a), 4.0)`;
  `let lim = aspect_limit(Some(Moderate), Some(Current)).unwrap(); assert_eq!(lim, 3.0)`;
  `keep_compact(&mut v /*[a, b]*/, lim)` returns `true`, `v.len() == 1`, `v[0].bbox.w == 2000`.
- `last_variant_is_kept_and_reported`: `[a]` against `aspect_limit(Some(Exceptional), Some(Voltage))` (1.5) →
  returns `false`, `v.len() == 1`. Plus `aspect_limit(Some(Moderate), Some(Ratio)) == None` and
  `aspect_limit(None, Some(Current)) == Some(3.0)`.
- `an_interdigitated_member_spans_the_row`: two owners alternating over 4 columns, bbox 4000×1000 → `member_aspect == 4.0`
  (subarray = the member's own bbox, not one unit).
- Regression: `cargo test -p library --release --test ota_cross_pdk ota5t_clean_on_sky130` still passes; report any
  bench fixture whose matched variant count drops to its fallback (`aspect_missed > 0`).

---

## GAP-14 User-declared deep-n-well isolated tubs for NMOS. Class: do

### Code facts (plan corrections)
- Stale: DNW is drawn in `kernel/cells/src/bjt.rs:138-147` (geometry) and `:209-220` (draw), not 31-34/129-138. Its
  recipe is reused: hole → dnwell by `enclosure("dnwell", "nwell")`, nwell past dnwell by `enclosure("nwell","dnwell")`.
- sky130 deck: `dnwell.2 width ≥ 3 µm`, `dnwell.3 space ≥ 6.3 µm`, `nwell.5 enclosure(dnwell, nwell_filled) ≥ 400`,
  `nwell.6 enclosure(nwell_hole, dnwell) ≥ 1030` (`pdks/decks/sky130.deck:234-241`); `pwell_iso_tie = ptap and pwell_iso`
  connects (:542). The plan's `backend/verify/src/pdk.rs:1212` is stale (dnwell is in the role list at :1478).
- Rings: `GuardRingType { Tap { in_well }, Ecgr, Hcgr }` (`kernel/analog/src/cell.rs:33-41`), one requirement per
  device; `rings::plan` (`backend/annotator/src/rings.rs:51`) gives "at most one ring per device", called at
  `backend/annotator/src/lib.rs:507-523`; `post_cell::guard_rings` (:26) clusters same-class (`connection_net`,
  `ring_type`, `role`) shareable rings within `guard_ring_merge_gap_nm` and draws one ring around the union of placed
  device boxes; the flow reserves `ring_halo` per device (`frontend/library/src/lib.rs:2094-2106`). This is the plan's
  "around the listed cells' union": a tub is a ring type, not new placement code.
- The p-well tap inside (plan step 2) is already drawn: a planar NMOS cell keeps only variants whose CELL-13 bulk tap
  strip is in reach (`cellgen.rs:803-806`), p+ tied to `B`; inside dnwell it is a `pwell_iso_tie`. No second p+ ring
  is drawn (it would only add a ring between the device and the band).
- Sidecar kinds are matched in `sidecar.rs:43` onward; `AnnotationConfig` is `backend/annotator/src/netrole.rs:123`,
  process flags at :217 (`ecgr_drawable`), set at `frontend/library/src/lib.rs:1188`; sidecar lists are merged at
  lib.rs:401-414 (each list named there; a new one must be added).

### Edits
1. `kernel/analog/src/cell.rs` `GuardRingType`: add
   `/// User-declared isolated tub (GAP-14): n+ ring in an n-well band over a deep n-well, tied to a quiet supply;`
   `/// the devices' bulk is the isolated p-well. `id` keeps distinct tubs from merging (distinct p-wells).`
   `Tub { id: u16 },`.
2. `kernel/cells/src/post_cell.rs`:
   - `implant_name`: `Tub { .. }` → `"nsdm"`; `in_nwell` unchanged (false); `well_shape`: `Tub { .. }` → `WellShape::None`
     (its band is drawn by `draw_tub_wells`).
   - `fn tub_well_ext(process, width) -> i32` = `band_well_ext(process, width).max(on_grid_up(process,
     (enclosure("dnwell","nwell") + enclosure("nwell","dnwell") − width + 1) / 2))` (sky130 width 420: 505).
   - `ring_gap`: `Tub` as `Ecgr` with `tub_well_ext` in place of `band_well_ext`.
   - `outer_clear`: `Tub` → `(nwell_space + g).max(g − enclosure("nwell","dnwell") + space("dnwell"))`, `g = tub_well_ext`
     (full dnwell spacing: a foreign dnwell, e.g. an NPN, reserves nothing for us).
     `// ponytail: full dnwell.3 per side (sky130 ≈ 6.4 µm halo); halve when every dnwell owner reserves its half.`
   - Factor the `WellShape::Band` arm's four rects into `fn band_well(b: &mut Builder, nwell: LayerId, o: Rect, inner_edge: Rect, g: i32)`
     (unchanged output for Ecgr; `tap_rings_draw_as_before` and `an_ecgr_well_never_covers_its_interior` guard it).
   - `draw_ring`: for `Tub`, after `tap_ring(..)` returns the band's outer rect `o`, call `band_well` with
     `g = tub_well_ext`, then `b.rect(dnwell, hole grown by enclosure("dnwell","nwell"))`, `hole` = the band well's inner
     edge (inner tap edge shrunk by `g`).
   - `drawable`: `Tub { .. }` → `layer("dnwell")`, `layer("nwell")`, `layer("nsdm")` present and
     `enclosure("dnwell","nwell").is_some()`.
3. `backend/annotator/src/sidecar.rs`: `"IsolatedTub"` entry, fields `instances: [name]`, `tie: net`; row in the module
   table. Unknown names → the existing `sidecar_unknown_name`; a non-NMOS member or members with different `B` nets →
   `sidecar_unsupported` `"entry {i}: IsolatedTub members must be NMOS on one bulk net"`, entry dropped. Valid →
   `cfg.tubs.push((devices, tie))`.
4. `netrole.rs` `AnnotationConfig`: `pub tubs: Vec<(Vec<DeviceId>, NetId)>` (doc: GAP-14, user-declared only, no
   automatic threshold in the sources); `ProcessConfig` (beside `ecgr_drawable`): `pub tub_drawable: bool`.
   `frontend/library/src/lib.rs:1188`: `tub_drawable: drawable(GuardRingType::Tub { id: 0 }, pdk)`; :413:
   `ann.tubs.extend(side.tubs);`.
5. `rings.rs`: `RingInputs` gains `tubs: &'a [(Vec<DeviceId>, NetId)]`, `tub_drawable: bool`; at the top of the
   per-device loop, a member of tub `k` gets `ring(d, Tub { id: k as u16 }, RingRole::Victim, tie, w, true)` and
   `continue` when `tub_drawable`; else push `("IsolatedTub", "deck has no deep n-well: tub drawn as an ordinary ring")`
   once (deduplicated like `note`) and fall through to the existing rows. lib.rs:507 passes `&cfg.tubs`,
   `p.tub_drawable`. Doc row 0 added to the `plan` doc.

### Tests
- `kernel/cells/src/post_cell.rs` `a_tub_is_drc_clean_on_sky130`: as `ecgr_around_nmos` with `req(0, 9, Tub { id: 0 }, true)`;
  assert one `dnwell` rect that strictly contains the cell bbox, 4 `nwell` rects, no `nwell` within
  `space_between("nwell","diff")` of the cell (same loop as the ECGR test), and `testkit::findings(cell+ring, labels G/S/B + ring "VDDQ")`
  is empty.
- same file `the_tub_bulk_is_not_the_substrate`: `verify::extract_spice(&shapes, &labels, &pdk, Detail::Schematic)` of
  cell + tub ring: the nfet card's 4th terminal is the net labelled `B` and the SPICE text has no substrate net on that
  card; control: the same cell + a `Tap { in_well: false }` ring → bulk is the substrate net (the assertion must
  discriminate; take the substrate net's name from that control output, do not hard-code it).
- same file `a_tub_halo_clears_dnwell_spacing`: `ring_halo(&req(.., Tub{id:0}, ..), &pdk, 15.0) − (gap + width + g − enclosure("nwell","dnwell"))
  ≥ space("dnwell")` and `merges_adjacent_shareable_same_class`-style: `Tub{id:0}` and `Tub{id:1}` never cluster.
- `backend/annotator` sidecar tests: `an_isolated_tub_parses` (two NMOS, tie `vdd` → `cfg.tubs == [(ids, vdd)]`, no diags);
  `a_tub_with_a_pmos_is_refused` (one `sidecar_unsupported`, `cfg.tubs` empty). `tests.rs`: `tub_members_get_a_tub_ring`
  (`RingInputs { tubs, tub_drawable: true, .. }` → member rings are `Tub { id: 0 }` on `tie`, others unchanged);
  `no_dnwell_falls_back_and_says_so` (`tub_drawable: false` → no `Tub`, `("IsolatedTub", ..)` in notes).
- Acceptance, `frontend/library/tests/extra_devices.rs` `a_declared_tub_signs_off_clean` (release): a two-NMOS
  sky130 netlist (mirror + resistor load to `vdd`), `Config { constraints: Some(r#"[{"constraint":"IsolatedTub","instances":["M1","M2"],"tie":"vdd"}]"#), feedback_iters: 1, .. }`;
  `library::run` then `library::signoff`: no hard violation whose rule starts with `drc` or `lvs`, and the solution's
  shapes include exactly one `dnwell` rect (both members in one tub) or report honestly if placement split them.
