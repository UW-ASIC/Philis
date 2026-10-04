# M1 cells batch: implementation cards (CELL-03, GAP-05, CELL-06 … CELL-10, CELL-30, FLOW-16)

Branch `m1a-cells`, worktree `philis-m1a/cells`, base = `git merge m1a` at `850560f` (matching merged: MAT-01…06).
Specs: plan-03 §3 (CELL-*), 98-gap-critic.md `### GAP-05`, plan-08 `### FLOW-16`; §6 C6, C23, C24, C29 apply. Line
numbers are this base's; where the plan differs, the card wins.

**Status (re-run 2026-10-04 after a second `git merge m1a`, clean, merge `22a1094`; it brings the input, annotator,
routing and matching review merges).** Eight of nine items are implemented on this branch, one commit each:
GAP-05 `b1510f4`, CELL-30 `79f1a6a`, CELL-06 `bda5935`, CELL-07 `1d03998`, CELL-09 `2a1ff0f`, CELL-08 `c72a623` +
`47a8231` (dr drops a route cut beside a same-net cell cut, tq_chain via.2), CELL-10 `878e222`, FLOW-16 `baeda1c`.
The cards for those items are the spec they were built from; their line numbers are the pre-implementation base and
are stale (read the commit diff instead). **CELL-03 is not implemented** (`finish` still rounds `w`/`h` to `step`, the
halo still to `pdk.grid`); its card is refreshed to today's lines and it is the only item left to build.
The merge overlaps this branch's files only in `dr/src/lib.rs`, `verify/src/{pdk,sidecar}.rs`,
`library/src/{cellgen,lib,elaborate}.rs` and the four sidecars, all textually clean; the routing merge removed
`gr::price_group` from `cellgen::price`, unrelated to the cells edits. Post-merge, `cargo test --release -p cells -p
verify -p annotator` and `cargo test --release -p library --lib` are green (0 failed; the one ignored is pre-existing).
benchmark `signoff_fixtures` and `bench local` were not re-run after the merge (cost); the M1 verify pass owns that.

Open acceptance recorded in the commits, not met yet (not loosened):
- CELL-08: `dac4_mim` and `tq_chain` fixtures exist but have **no `BASELINE` row** (antenna rows remain; GPurify
  attach, see `c72a623`/`47a8231`). tq_chain measured DRC 0, LVS MATCH. Step 7 (`"cap"` alias on `mim_m3_1`) was not
  taken: sky130.json `mim_m3_1.aliases` is `["cap_mim_m3_1"]` only. Both stay open for the M1 verify pass.
- FLOW-16: ota, ota_constrained, tt_ota MatchedSet 2/2 (was 0/2), LVS MATCH, DRC 0; area 1723 → 1860.7 µm² (+8 %).
- CELL-06 `res_m2` and CELL-10 `mirror_ratio` BASELINE rows are present. CELL-10 also makes cellgen decline a merge
  with no drawn variant (quad's alternating nf=1 block).

| Item | Class | State |
|---|---|---|
| CELL-03 | mechanical | to build (card below, current lines) |
| GAP-05 | mechanical | done `b1510f4` |
| CELL-30 | mechanical | done `79f1a6a` |
| CELL-06 | judgment | done `bda5935` |
| CELL-07 | mechanical | done `1d03998` |
| CELL-09 | mechanical | done `2a1ff0f` |
| CELL-08 | judgment | done `c72a623`, `47a8231`; BASELINE rows and step 7 open |
| CELL-10 | judgment | done `878e222` |
| FLOW-16 | judgment | done `baeda1c` |

Every command: `export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk` first. Crates: `cells`
(kernel/cells), `analog`, `pnr_core`, `verify`, `annotator`, `library`, `benchmark` (bin `bench`: `cargo run --release
-p benchmark --bin bench local [filter]`).

**Order** (dependencies first; each item lands as its own commit `M1 <ID>: …`; as built, CELL-03 was skipped and
now lands last, which is safe: no later item depends on its geometry beyond the ≤ 10 nm growth):
CELL-03 → GAP-05 → CELL-30 → CELL-06 → CELL-07 → CELL-09 → CELL-08 → CELL-10 → FLOW-16.
CELL-03 first (it moves every bbox other items assert on); CELL-06/07/08/09 all touch `sidecar::KEYS`,
`deck_keys.rs` and `signoff_fixtures.rs`, so serial; FLOW-16 needs CELL-10's predicate.

## Batch-wide facts and decisions

- F1. MAT-02 already did BJT order + `b.unit` (bjt.rs:58-66, :155) and `pattern::{grids, centro_assign, Fill}`
  (kernel/analog/src/matching/pattern.rs:29, :70). MAT-03 added `pattern::{Outer, diffusion_cc_row,
  diffusion_legal}` (:280, :346) but **did not touch mosfet.rs**: `centroid_sequence` (mosfet.rs:730),
  `finger_sequence`'s `greedy_centroid` branch (:784-796) are still there. CELL-10 wires them (MAT-03 card says so).
- F2. `CentroidGroup` is gone (MAT-04). Plan text naming "`CommonCentroid` rows" now means `MatchedSet` rows
  (m1-matching D8). FLOW-16's acceptance is restated in its card.
- F3. `kernel/cells/tests/deck_keys.rs::generators_read_exactly_the_required_keys` records every `Process::rule`
  name the generators read on the **base** sky130 `Pdk` (no recipe overlay). A new key read with a compiled default
  must be (a) `required` in `verify::sidecar::KEYS` and present in all four sidecars, or (b) listed in the test's
  `flags` array (:172) with the reason in its comment, or the test fails ("optional_read"/"defaulted"). Recipe-slot
  keys (inside `cell.resistors/capacitors/bjts.recipes.*`) are never top-level, so they go in `flags`; their table
  name needs a `Table` row in `KEYS` (`validate`, sidecar.rs:151, rejects unknown top-level keys).
- F4. `Pdk::recipe(kind, model)` (backend/verify/src/pdk.rs:1250) already keeps every integer recipe key as a rule and
  `Overlay::rule` (pdk.rs:1283) reads it first (plan's "pdk.rs:1142/1162" step is already true; nothing to do).
  `RESISTOR_ROLES` is pdk.rs:1209 (plan: 1087); `recipe_layers` is pdk.rs:1224 with `for kind in ["resistors"]` at
  :1226 (plan: 1103-1105); `DRAWN_ROLES` pdk.rs:1340; `philis_drawn` :919.
- F5. `drawn_cards` (frontend/library/src/cellgen.rs:944) already maps `DrawnKind::{Capacitor, Diode, Npn, Pnp}` to
  recipe kinds `"capacitor"`, `"diode"`, `"bjt"`; `Node::Internal(k)` is the net `~{cell}.{owner}.{k}` (:966), so
  internal node ids only need to be unique **per owner**. A device with any `Drawn` is skipped by `reference`.
- F6. sky130 deck (pdks/decks/sky130.deck): capm recogniser `device capacitor capm model
  "sky130_fd_pr__cap_mim_m3_1" terminals [met4, met3]` at :598 (plan: 565); capm.1/2a/3/4/11 at :415-419 (plan:
  407-411); difftap.9 `space(ndiff, nwell) >= 340nm` at :277.
- F7. `preprocess_spice` value→W/L sizing (benchmarks/src/fixtures.rs:540-570): the m1-input card moved FLOW-07's
  "reduced to model mapping" acceptance to CELL-06/08. **Not taken here**: bare `R`/`C` cards carry no W/L, so
  removing the sizing needs a decision on where bare values get sized (owner decision; out of these cards).

---

## CELL-03 Macro extents on twice the cut lattice — class: mechanical

Facts (today's tree, after every other item in this batch): `Builder::finish` (kernel/cells/src/builder.rs:132-149,
doc :126-130) floors the corner and rounds `w`/`h` up to `step = 2·grid` (:138; 10 nm sky130), so `bbox.w/2` can be a
5 nm multiple. `cut_lattice(process)` **already exists** (builder.rs:152-156, plan step 2 done; `pub mod builder`,
kernel/cells/src/lib.rs:15). Halo: frontend/library/src/lib.rs:1489 `let ext = round_up(cells::post_cell::ring_halo(r,
pdk, ring_cut_ohm(pdk)), pdk.grid.max(1));` (plan: lib.rs:1198); `round_up` lib.rs:1254. post_cell `guard_rings`
shrinks by the raw halo (post_cell.rs:58), which fits inside the rounded reservation (no change there). PLC-02
(`align_bbox`) has not landed (no `align_bbox` in the tree), so this item makes the lib.rs edit.

Edits:
1. builder.rs `finish`: after `let step = 2 * self.grid.max(1);` add `let ext = 2 * step;`. Keep `x`, `y` floored to
   `step`; compute `w: (tight.x + tight.w - x + ext - 1) / ext * ext`, same for `h` with `y`. Doc (:126-130): "The bbox
   corner is a multiple of two grid steps (the cut lattice) and its extents of four, so the half-extent the placer
   stamps at (`centre - bbox.w / 2`) is on the cut lattice too (H01-26)."
2. lib.rs:1489: `round_up(cells::post_cell::ring_halo(r, pdk, ring_cut_ohm(pdk)), cells::builder::cut_lattice(pdk))`
   (a halo on 10 nm keeps the corner on 10 nm and `w + 2·halo` on 20 nm).

Tests:
- builder.rs `mod tests` (:308) `extents_are_twice_the_cut_lattice`: `let mut b = Builder::new(5); b.rect(LayerId(0),
  Rect { x: 3, y: 7, w: 1235, h: 41 }); let m = b.finish();` → `m.bbox.x % 10 == 0`, `m.bbox.y % 10 == 0`,
  `m.bbox.w % 20 == 0`, `m.bbox.h % 20 == 0`, and the bbox covers `m.shapes[0].rect`.
- kernel/cells/tests/cell_selfcheck.rs `every_half_extent_is_on_the_cut_lattice`: `let Some(pdk) = pdk() else {
  return };` (as its neighbours; `pdk()` :30), `for (name, m) in all_variants(&pdk)` (:83), collect `name` where
  `(m.bbox.w / 2) % 10 != 0 || (m.bbox.h / 2) % 10 != 0 || m.bbox.x % 10 != 0 || m.bbox.y % 10 != 0`; assert the
  list is empty, printing it.
- Existing bbox-bound tests stay green unedited: mosfet.rs `a_long_gate_keeps_short_dummies` (`bbox.w <= w_nd +
  7000`, `<= 73_000`; both sides grow ≤ 10 nm), `cell_selfcheck::the_bbox_contains_every_drawn_shape` (:262). If an
  exact-width assertion elsewhere goes red, stop and report it; do not edit it.

Run: `cargo test --release -p cells`, `cargo test --release -p library`, then `cargo test --release -p benchmark
--test signoff_fixtures` (every BASELINE row unchanged). Acceptance: both new tests green; each macro grows ≤ 10 nm per
axis. The plan's "PLC origin assertion on every bench circuit" belongs to PLC-02 and is not measurable here.

## GAP-05 Guard-ring kinds name what is drawn; `drawable`; ECGR spacing — class: mechanical

Facts: `GuardRingType { Ecgr, Hcgr, Hbgr, Ebgr }` kernel/analog/src/cell.rs:25-35; `tap_pitch_nm` :13-14,
`enclosure_complete` :19-20 (never read). Annotator literal backend/annotator/src/constraints.rs:70-85 (PMOS→Hcgr,
NMOS→Ecgr, `tap_pitch_nm: 2_000`, `enclosure_complete: true`). post_cell.rs: `ring_gap` :299-302, `ring_implant_enc`
:305, `implant_name` :315, `in_nwell` :321, `outer_clear` :328; test `req()` :502-513 sets both dead fields; tests
using the variants :540-590. No other user (`grep -rn GuardRingType` → annotator + post_cell only).
`retrograde_pwell` is a bool in all four sidecars (sky130.json:71) and becomes rule 0/1 (pdk.rs:1392); KEYS row
sidecar.rs:116 says reader `unread`.

Edits:
1. cell.rs: `pub enum GuardRingType { Tap { in_well: bool }, Ecgr, Hcgr }` with the doc lines from the spec
   (`Tap`: majority-carrier tap ring, p+ in substrate or n+ in n-well when `in_well`; `Ecgr`: n+ ring in its own
   n-well band, tied to a supply, collects electrons; `Hcgr`: p+ collecting ring, needs a retrograde/isolated well the
   deck declares). Delete `Hbgr`, `Ebgr`, the fields `tap_pitch_nm`, `enclosure_complete` and their docs.
2. constraints.rs:70-72: `Pmos => GuardRingType::Tap { in_well: true }`, `Nmos => GuardRingType::Tap { in_well:
   false }`; drop the two field lines (:81, :84).
3. post_cell.rs:
   - `implant_name`: `match t { Tap { in_well: true } | Ecgr => "nsdm", Tap { in_well: false } | Hcgr => "psdm" }`.
   - `in_nwell`: `matches!(t, Tap { in_well: true })` (Ecgr: false, its band well is CELL-17's; Hcgr: false).
   - Rewrite both docs (:309-320) to say: a `Tap` ring is its device's bulk tap; `Ecgr` is n+ (its own well is drawn
     by CELL-17); `Hcgr` p+.
   - `ring_gap`: `ring_clear(process) + ring_implant_enc(process, r.ring_type)` then
     `.max(if r.ring_type == GuardRingType::Ecgr { process.space_between("nwell", "diff").unwrap_or(0) } else { 0 })`
     (difftap.9, 340 on sky130).
   - New, after `in_nwell`:
     ```rust
     /// Whether this deck can draw a `t` ring as named. `Tap`: always. `Hcgr`: only with a retrograde/isolated
     /// p-well the deck declares (`retrograde_pwell`; sky130 false).
     #[must_use]
     pub fn drawable(t: GuardRingType, p: &dyn Process) -> bool {
         match t {
             GuardRingType::Tap { .. } => true,
             // ponytail: false until CELL-17 draws the Ecgr band well; it then becomes
             // `p.layer("nwell").is_some() && p.layer("nsdm").is_some()` (GAP-05 spec).
             GuardRingType::Ecgr => false,
             GuardRingType::Hcgr => p.rule("retrograde_pwell", 0) != 0,
         }
     }
     ```
     **Deviation from the spec**: the spec's `Ecgr` formula is true on sky130 while nothing draws the band well yet,
     so an Ecgr drawn now would be an n+ tap in bare substrate (diff/tap.11). REL-07 consumes `drawable`, so it cannot
     emit an undrawable ring in between.
   - Test `req()`: drop the two dead fields. Replace `Hcgr` → `Tap { in_well: true }`, `Ecgr` → `Tap { in_well:
     false }` in `merges_adjacent_shareable_same_class`, `never_merges_across_class_or_private`,
     `never_merges_across_a_foreign_cell` (same semantics: PMOS/NMOS rings).
4. sidecar.rs:116 reader → `"kernel/cells/src/post_cell.rs drawable"`.
5. `grep -rn "Hbgr\|Ebgr\|tap_pitch_nm\|enclosure_complete" --include='*.rs' .` must be empty (target/ excluded).

Tests (post_cell.rs):
- Replace `well_follows_implant_polarity` with `tap_rings_draw_as_before`: `implant_name(Tap{in_well:true}) == "nsdm"
  && in_nwell(Tap{in_well:true})`, `implant_name(Tap{in_well:false}) == "psdm" && !in_nwell(Tap{in_well:false})` (the
  old Hcgr/Ecgr values, so every annotator ring draws byte-identically: `draw_ring`/`ring_halo` read the type only
  through these two and `ring_gap`, whose Ecgr arm no annotator ring hits); `implant_name(Ecgr) == "nsdm" &&
  !in_nwell(Ecgr)`; `implant_name(Hcgr) == "psdm"`.
- `hcgr_is_not_drawable_on_sky130`: `testkit::pdk()` else return; `!drawable(Hcgr, &pdk)`, `!drawable(Ecgr, &pdk)`,
  `drawable(Tap { in_well: true }, &pdk)`.
- `ecgr_ring_gap_clears_the_well`: `pdk.space_between("nwell", "diff") == Some(340)` (if this is `None`, stop and
  report: the deck states difftap.9 on derived `ndiff`, and `widest_on` may not reach it); `ring_gap(&pdk,
  &req(0, 5, Ecgr, true), 0) >= 340`.
- `ecgr_is_n_plus_in_its_own_well_clear_of_the_nmos` is **not** added here: it can only pass after CELL-17 and may
  not land ignored; CELL-17 owns it (D20 updated accordingly).

Run: `cargo test -p cells --lib post_cell && cargo test -p annotator && cargo test --release -p benchmark --test
signoff_fixtures`. Acceptance: grep empty; signoff rows unchanged; REL-07 can name `Tap { in_well }`, `Ecgr`, `Hcgr`.

## CELL-30 MOS dummy gates capped at the microloading reach — class: mechanical

Facts (mosfet.rs): `gate_l = s.unit_l` :259; `pad_over`/`skirt_over` :274-275 (active-gate pads, unchanged);
`d_step = gate_l + sd_end` :285 (used by diff extent :294-295 and right dummies); dummy loop :511-534 (`left` :513,
`right` :514, `(right, right + gate_l)` :515, poly rect :516, cut `cx` :517, `Dummy { l: gate_l }` :531);
`dpad_w` :536; skirt gate positions :547-548. LVS: `dummy_cards` writes `Dummy.l` (cellgen.rs:998). Sweeps:
`every_variant_is_drc_and_erc_clean` :812, `every_variant_extracts_its_fingers_and_dummies` :837.

Edits:
1. Sidecars: `"dummy_max_l_nm": 3000` + `"dummy_max_l_nm_source": "microloading reach 3-5 um, hastings.txt
   L41097-41104 (H13-26); Philis policy: the lower end"` in sky130, gf180mcu, ihp_sg13g2; `"dummy_max_l_nm": null`
   in generic_finfet. KEYS (alphabetical): `k("dummy_max_l_nm", Nm, false, true, "kernel/cells/src/mosfet.rs (dummy
   gate length cap)")`. deck_keys.rs `flags` += `"dummy_max_l_nm"` (comment: absent = dummies at the active L, the
   pre-cap drawing).
2. mosfet.rs `draw_row`, after :259: `let dummy_l = match process.rule("dummy_max_l_nm", 0) { cap if cap > 0 &&
   gate_l > cap => cap, _ => gate_l };` (doc: H13-26, dummy L = min(active L, microloading reach)). Replace `gate_l`
   by `dummy_l` at :285 (`d_step`), :513, :515 (`right + dummy_l`), :516 (`w: dummy_l`), :517, :531, :536, :547,
   :548. Nothing else (`sd_edge` :281 and every active-gate line stay on `gate_l`).

Tests (mosfet.rs):
- `a_long_gate_keeps_short_dummies`: `testkit::group_of(Pmos, 1, 1, 420, 64_800)`; for `dummy_required` in [false,
  true] draw every variant; with dummies: every `Dummy.l == 3000`, `m.bbox.w <= w_nd + 2 * (3_000 + 500)` where `w_nd`
  is the same variant index's width without dummies, and `m.bbox.w <= 73_000` (FR-5); `testkit::findings` empty;
  extracted `M` count == `units + dummies` (as :851-855).
- `a_short_gate_keeps_full_dummies`: L = 150 and L = 1000 → every `Dummy.l == gate_l` (below the cap).
- Extend both sweeps (:812, :837) with `(W, L) ∈ {(420,150), (420,1000), (420,16_200), (420,64_800), (2160,9600)}`,
  `n = 1, nf = 1`, NMOS and PMOS (`testkit::dirty` already sweeps dummies on/off).

Run: `cargo test --release -p cells --lib mosfet && cargo test -p cells --test deck_keys && cargo test -p verify
sidecar`. Acceptance: tests green; bench GDS unchanged (every fixture L ≤ 2 µm).

## CELL-06 Resistors: multiplicity, exact value under segmentation — class: judgment

(Judgment: multi-string Interdig track allocation and `res_m2` LVS through the router are unmeasured.)

Facts (resistor.rs): `body_w`/`seg_l = unit_l / n` :81-82; one string per member, `dev_nf` never read; sequence
:101 via `res_segment_sequence` :312 (Interdig → `greedy_centroid`); jumpers: li to an adjacent column, else met1 on
`tracks[di % len]` :137-155; `Drawn` per segment :156-163 with `Internal(k)`; `feasible_segments` :321-345 (n | L,
n ∈ {1, even}, 8 squarest); enumerate filter `Interdig && segments > 1 && devices > tracks` :41. No `keepout` call.
cellgen singleton `dev_nf = max(nf, m)`, `Series` for Resistor/Capacitor (cellgen.rs:560-573); nothing reads a
resistor's `series_parallel`. sky130 `res_min_segment` 10000, `res_min_width` 500 (sky130.json:52, :85); recipes
`high_po`, `generic_po` (:116-143). Derived: R(690, 40 000) = 18 949.2 Ω; n=2 → seg 19 432 nm, n=4 → 9 148 nm
(< 10 µm, dropped). Existing tests that must stay green: cellgen.rs `drawn_cards_carry_no_params_for_passives`
(:1793, expects `~0.0.1` and a 2-segment variant), frontend/library/tests/drawn_cards.rs.

Edits:
1. resistor.rs: add `ResModel` exactly as plan-03 CELL-06 step 1 (fields `sheet_mohm: i64, head_mohm_um: i64,
   dl_nm, dw_nm, head_dw_nm, narrow_permille, knee_nm: i32`).
   - `of(p)`: `let sheet = p.rule("res_sheet_mohm", 0); if sheet <= 0 { return None; }` **then** read the other six
     with default 0 (so the base deck records only `res_sheet_mohm`, F3).
   - `ohm(w, l)` in µm floats: `w_um = w/1e3`, `weff = w_um + dw/1e3 − narrow/1e3·max(knee/1e3 − w_um, 0)`;
     `sheet/1e3·(l/1e3 + dl/1e3)/weff + head/1e3/(weff + head_dw/1e3)`.
   - `seg_len(w, target, n, min_nm, lat, tol_ppm)`: `l_um = (target/n − head_term)·weff/sheet − dl`; `l =
     round(l_um·1e3 / lat)·lat`; `None` if `l < min_nm` or `|n·ohm(w, l) − target|/target·1e6 > tol_ppm`.
   - `pub fn value_tol_ppm(p) -> i32 { p.rule("res_value_tol_ppm", 0) }`.
2. sky130.json `high_po` += the plan's seven integer keys and `"res_model_source"` text; `generic_po` +=
   `"res_sheet_mohm": 48200, "res_head_mohm_um": 0, "res_dl_nm": 0, "res_dw_nm": 0` (+ source text naming
   `rp1 = 48.2`, and that `dw` was not applied, plan §5). Top level in **all four** sidecars:
   `"res_value_tol_ppm": 5000`; KEYS `k("res_value_tol_ppm", Count, true, false, "kernel/cells/src/resistor.rs")`
   (required: read with a compiled default, F3). deck_keys `flags` += `"res_sheet_mohm"` (absent = no model = only
   n = 1).
3. `feasible_segments(s, process)`: read `tol = value_tol_ppm(process)` and `min_seg` unconditionally first. With
   `model = ResModel::of(process)`: `None` → `vec![1]`; else `target = model.ohm(s.unit_w, s.unit_l)`, `body_w =
   s.unit_w.max(res_min_width)`, keep n ∈ `1..=64` with n == 1 or even and `model.seg_len(body_w, target, n, min_seg,
   lat, tol).is_some()` (drop the `body_l % n` rule); rank by the same squareness using `seg_len` for the height;
   truncate 8; sort; `n = 1` is kept even when `seg_len(.., 1, ..)` is None (one segment never changes the value).
4. `draw`: `seg_l = model.and_then(|m| m.seg_len(body_w, target, n, 0, lat, i32::MAX)).unwrap_or(s.unit_l /
   n).max(process.width("rpoly").unwrap_or(0))` (n = 1 without a model keeps today's `unit_l`).
   - Strings: `strings[d] = if series_parallel == Parallel { s.dev_nf[d].max(1) } else { 1 }` (read the covering
     unitization via `crate::builder::unitization`). Global string ids `g` in member order; string `j` of member `d`
     is one pseudo-device for sequencing: `res_segment_sequence` takes `&[usize]` (strings per member expanded to
     pseudo-devices) and returns pseudo-device ids; Single = each pseudo-device's segments adjacent; Interdig =
     `greedy_centroid(&vec![n; total_strings])` (CELL-14 replaces it).
   - Per pseudo-device keep `seg_of`/`prev` (today's per-`di` vectors, indexed by `g`). Pins `pin(d, "P"/"N", …)` per
     string (repeated names; router joins them like MOS S/D). Internal node of segment k of string j:
     `Node::Internal((j * n_segments + k) as u16)` (j = 0 reproduces today's ids).
   - met1 jumper track: `tracks[g % tracks.len()]`; enumerate filter becomes `Interdig && segments > 1 &&
     total_strings > tracks`.
   - `Drawn { owner: d, w: body_w, l: seg_l, .. }` per segment (as today); add
     `b.keepout(Rect { x: sx, y: head, w: body_w, h: seg_l }, pnr_core::KeepWhy::ResistorBody { owner: d as u8 })`
     per segment (`KeepWhy::{ResistorBody, CapPlate}` exist, kernel/core/src/macro.rs:74-78).
   - `Unit.weight = body_w·seg_l / strings[d]²` (Hastings eqs 8.24-8.25).
5. cellgen.rs:570-573: `DeviceKind::Resistor => SeriesParallel::Parallel` (SPICE `m` is parallel);
   `Capacitor` stays `Series` (no reader; CELL-08 decides).
6. Fixture `benchmarks/fixtures/res_m2.spice` (header format of pair.spice):
   `XR1 a b sky130_fd_pr__res_high_po w=0.69u l=40u m=2` plus an inverter (`nfet_01v8`/`pfet_01v8` W=1u L=0.15u)
   driving `a` from port `in`; ports `in b VDD VSS`. Add `("res_m2", 0, &[], true)` to `BASELINE`
   (benchmarks/tests/signoff_fixtures.rs:23).

Tests (resistor.rs, sky130 through `verify::pdk::Overlay { pdk, recipe: pdk.recipe("resistor", "res_high_po")? }`):
- `segments_preserve_the_model_value`: W=690, L=40 000, every variant: `|Σ_{drawn} model.ohm(w, l) / strings −
  18_949.2| / 18_949.2 <= 0.005` (for one string: Σ over its segments).
- `short_segments_are_not_offered`: same group → no variant with `segments == 4`.
- `a_multiplied_resistor_draws_parallel_strings`: `dev_nf = [2]`, Parallel → `drawn.len() == 2 * segments`, 2 pins
  named `d0:P` and 2 `d0:N`, each string's Σ within 0.5 %.
- `every_variant_extracts_one_resistor_per_segment`: `verify::extract_spice(&m.shapes, &[], &pdk,
  Detail::Schematic)` lines starting `R` == `m.drawn.len()`, for `dev_nf` [1] and [2].
- `every_variant_is_drc_and_erc_clean` (:353) runs also through the `high_po` overlay with `dev_nf = [2]` (set
  `c.unitization[0].dev_nf` after `testkit::group_of`).

Run: `cargo test --release -p cells --lib resistor && cargo test -p cells --test deck_keys && cargo test -p verify &&
cargo test -p library --test drawn_cards && cargo test -p library --lib cellgen && cargo test --release -p benchmark
--test signoff_fixtures`. Acceptance: T3/T4 for resistors; rc_filter and res_m2 LVS clean.

## CELL-07 Diodes: multiplicity, contact arrays, licon.7-legal anode tap — class: mechanical

Facts (diode.rs): `Diode { pattern, columns }` :22-26; enumerate :28-43 (cols from device count, `Pattern`
Single/Interdig); one diode per member :89-119 (Interdig flips odd members); one cut per terminal :99-108;
`tap_side = ct + diff_enc + tap_cap` :67 (330 on sky130; licon.7 needs 410); no `b.unit`, no `b.drawn`. cellgen
singleton diode `dev_nf = max(nf, m)`, `Parallel`. Reference emits 1 card per diode (cellgen.rs:912); a `Drawn`
per unit replaces it (F5). Antenna diodes are drawn by the same generator (frontend/library/tests/antenna_diode.rs).

Edits (diode.rs):
1. `pub struct Diode { pub rows: u16, pub cols: u16 }`; drop `use crate::Pattern`. Doc: units of member d =
   `dev_nf[d]`; owners from `pattern::centro_assign(.., Fill::Balanced)`.
2. enumerate: `counts = s.dev_nf.iter().map(|&n| n.max(1))`; `n = Σ counts`; members > 1 → `pattern::grids(&counts,
   3.0)` mapped to `Diode { rows, cols }`; one member → cols in sorted-deduped `[1, n, ceil(√n)]`, `rows =
   n.div_ceil(cols)` (as bjt.rs:61-64).
3. draw: `let (owners, _) = pattern::centro_assign(&counts, rows, cols, Fill::Balanced);` loop over slots `i` with
   `Some(d)`: `ox = (i % cols)·(dev_w + gap)`, `oy = (i / cols)·(tap_h + gap)`; no flip (delete `flip`).
4. Cathode contact array: `pitch = ct + process.space("licon").unwrap_or(ct)` snapped up to `lat`; `inset =
   diff_cap`; `nx = ((w − 2·inset − ct)/pitch + 1).max(1)`, same `ny` for `l`; cuts centred, snapped with `snap_cut`;
   one li plate over the array enclosing by `pad_enc` (same construction as bjt.rs:166-178). Pin on the middle cut.
5. Anode tap: `tap_side = ct + 2 * tap_cap.max(diff_enc)` (410 sky130); `tap_h` unchanged; a column of cuts at the same
   `pitch` along y, centred; pin on the middle cut.
6. Per unit: `b.unit(pnr_core::Unit { owner: d, x: k.x + w/2, y: k.y + l/2, weight: i64::from(w) * i64::from(l), phi:
   (0, 0), sa: 0, sb: 0 })` and `b.drawn(Drawn { owner: d, device: None, kind: DrawnKind::Diode, nodes:
   [Node::Pin("P"), Node::Pin("N"), Node::Unused], w, l })`.
7. Fix every `Diode { pattern, columns }` literal: `grep -rn "Diode {" --include='*.rs' frontend kernel benchmarks`.

Tests (diode.rs):
- `a_multiplied_diode_draws_its_area`: `group_of(Diode, 1, 4, 500, 1000)` → every variant: `units.len() == 4`,
  `drawn.len() == 4`, Σ over `diff` shapes under the marker (cathode rects) of `w·h == 4 · 500_000` nm² (500 and 1000
  are lattice multiples, so no snap).
- `the_anode_tap_meets_licon_7`: every `tap` rect has `w >= 410`.
- `matched_diodes_share_a_centroid`: `dev_nf` [2,2] and [1,4], every variant → for members A=0, B=1: `n_B·Σ_{u∈A}
  u.x == n_A·Σ_{u∈B} u.x` and the same in y (i64).
- DRC sweep (:136): add `dev_nf` [4] and [2,2] (set after `group_of`); keep [1] and n=2.

Run: `cargo test --release -p cells --lib diode && cargo test -p cells --test cell_selfcheck && cargo test -p library
--test antenna_diode`. Acceptance: T4 for diodes (Σ area = n·W·L); antenna diode test green.

## CELL-09 BJT fixed-geometry emitters and drawn cards — class: mechanical

Facts: emitter `max(unit_w/l, bjt_min_emitter_side)` bjt.rs:93-94 (plan: 91-92); `Unit::draw` :145-202, `b.unit`
:155 (MAT-02); no `b.drawn`. cellgen BJT arm cellgen.rs:814 has no overlay (resistor arm :803-806 does).
`BJT_PINS = ["E", "B", "C"]` cellgen.rs:931 (library-private). `drawn_cards` already resolves `pdk.recipe("bjt", ..)`.

Edits:
1. sky130.json `cell` += the plan's `"bjts": { "recipes": { pnp_3p40, pnp_0p68, npn_1x1, npn_1x2 } }` verbatim (no
   `default`), plus `"note"` citing the model names. KEYS += `k("bjts", Table, false, false,
   "backend/verify/src/pdk.rs recipe")`. deck_keys `flags` += `"bjt_emitter_w", "bjt_emitter_l"` (recipe slots;
   absent = the netlist's size).
2. bjt.rs `Unit::new` (:93-94): `let ew = match r("bjt_emitter_w", 0) { 0 => s.unit_w.max(min_side), v => v };`
   `let el = match r("bjt_emitter_l", 0) { 0 => s.unit_l.max(min_side), v => v };` `emitter = Rect { x: 0, y: 0, w:
   ew, h: el }`.
3. bjt.rs `Unit::draw` after the `b.unit(..)` (:155): `b.drawn(pnr_core::Drawn { owner: di as u8, device: None,
   kind: if self.pnp { DrawnKind::Pnp } else { DrawnKind::Npn }, nodes: [Node::Pin("E"), Node::Pin("B"),
   Node::Pin("C")], w: e.w, l: e.h })` with comment "the order of `cellgen::BJT_PINS`".
4. cellgen.rs:814: `DeviceKind::Npn | DeviceKind::Pnp => match pdk.recipe("bjt", model) { Some(recipe) =>
   draw_all::<Bjt>(group, c, &verify::pdk::Overlay { pdk, recipe }), None => draw_all::<Bjt>(group, c, pdk) },`.
5. NPN per-unit dnwell unchanged.

Tests (bjt.rs; overlay = `Overlay { pdk: &pdk, recipe: pdk.recipe("bjt", "sky130_fd_pr__pnp_05v5_W3p40L3p40")
.unwrap() }`):
- `a_fixed_geometry_model_sets_the_emitter`: `group_of(Pnp, 1, 1, 150, 150)` through the overlay → every variant,
  every `drawn` has `w == 3400 && l == 3400`, and each `units[i]` weight == 3400².
- `every_emitter_is_a_drawn_card`: `dev_nf = [1, 8]` → `drawn.len() == 9`, all `DrawnKind::Pnp`, w = l = 3400.
- `every_variant_is_drc_and_erc_clean` (:228): add the `[1,8]` PNP block once more through the overlay.
- cellgen.rs `lone_bjt_draws_as_many_units_as_reference_cards` (:1781) stays green.

Run: `cargo test --release -p cells --lib bjt && cargo test -p cells --test deck_keys && cargo test -p verify
sidecar && cargo test -p library --lib cellgen && cargo test --release -p benchmark --test signoff_fixtures`.
Acceptance: bgr_core Q1 unit at the 3×3 centre (MAT-02 test) with 9 drawn `Pnp` cards; BJT LVS stays
`lvs-coverage/unverified` until PERF-18.

## CELL-08 Capacitors drawn as the deck's device (MIM on sky130) — class: judgment

(Judgment: effort L, a new plate stack and in-cell down-stack on met1/met2-only routing, and three bench acceptances
— tq_chain, dac4_mim, dac4 — that no one has measured. Recommend the strongest model.)

Facts: CapArray stack hard-codes met1/met2/met3/via1/via2 (cap_array.rs:81, :248) and draws BOT met1 / TOP met2
inset by `plate_spacing` / centre via2 / met3 strap (:340-352, :393-411); emits `b.unit` (:349) but **no `Drawn`
and no keepout**; general sets need ≥ 2 members (:88). capacitor.rs `grid_w`/`grid_h` include `unit_gap`
(capacitor.rs:180-186). cellgen Capacitor arm (cellgen.rs:809-812) passes the base `pdk`; `dac_banks` keys `(P, w,
l)` (:770-775). No sidecar `capacitors` table exists (FLOW-06 step 4 was cut; C24/D21: CELL-08 owns the whole table).
`signoff_fixtures::unverified` (signoff_fixtures.rs:43-57) counts every capacitor unit as unverified. Bench
preprocess sizes bare `C` at `cap_density_ff_um2` = 2.0 with model `cap` (fixtures.rs:556-562). Field-report input:
/home/omare/Documents/Projects/Trial/ResearchBoutros/analog/async_ctrl/output/pnr/tq_chain.spice (8 ×
`cap_mim_m3_1 W=21.87 L=21.87`, each on its own P net, N = vss → 8 singletons, `dev_nf = [1]`, `Series`).

Edits:
1. sky130.json `cell.capacitors`:
   ```json
   "capacitors": { "default": "mom_m1m2", "recipes": {
     "mim_m3_1": { "model": "sky130_fd_pr__cap_mim_m3_1", "aliases": ["cap_mim_m3_1"],
       "layers": { "bottom": "met3", "plate": "capm", "top_contact": "via3", "top": "met4", "strap": "met4",
                   "bottom_contact": "via2" },
       "c_area_af_um2": 2000, "c_perim_af_um": 190, "c_dw_nm": -25,
       "c_model_source": "<camimc/cpmimc/m3_dw file:line as plan-03 CELL-08 Current>" },
     "mom_m1m2": { "model": "", "layers": { "bottom": "met1", "top": "met2", "top_contact": "via2",
                   "strap": "met3", "bottom_contact": "via1" } } } }
   ```
   KEYS += `k("capacitors", Table, false, false, "backend/verify/src/pdk.rs recipe")`. The `"cap"` alias is **step
   7**, not here.
2. pdk.rs: `const CAPACITOR_ROLES: &[&str] = &["bottom", "plate", "top_contact", "top", "strap", "bottom_contact"];`
   next to `RESISTOR_ROLES`; `Overlay::layer` `None if RESISTOR_ROLES.contains(&role) || CAPACITOR_ROLES
   .contains(&role) => None`; `recipe_layers` `for kind in ["resistors", "capacitors"]`. Guard test
   `capacitor_recipes_do_not_move_routing_rules` as the spec (clone the sidecar JSON, remove `cell.capacitors`, load
   both; equal `space("met3")`, `width("met3")`, `space("met4")`, `enclosure("met3","via3")`; and
   `Overlay(mim_m3_1).enclosure("bottom", "plate") == Some(140)`). If that is `None`/not 140, stop: the deck's capm.3
   operand order (`enclosure(capm, met3)`) vs `widest_on` is the issue, report it.
3. cap_array.rs: `PlateStack` + `fn plate_stack(p) -> Option<PlateStack>` as the spec; `enumerate` returns `vec![]`
   when `plate_stack` is None (replaces the :81 layer check); `build` reads every layer from it. MOM (no `plate`
   role, or the base `Pdk` with no table): today's construction byte-for-byte (`plate_stack` falls back to roles
   met1/met2/met3/via1/via2 when the process has no `bottom` role, so CapArray drawn with the base `pdk` and the
   unit tests are unchanged). MIM: per-unit BOT met3 = capm + 2·enclosure("bottom","plate"); capm = W×L of the
   netlist; via3 array on capm inset by `enclosure("plate","top_contact")` at via3 pitch; met4 strap per column; BOT
   met3 stub → via2 → met2 branch → via1 → met1 track/bus as today; capm-to-capm ≥ capm.2a 840; met3 not on capm ≥
   500 from capm (capm.11). Pins stay on met1.
   - `enumerate`: accept `dev_nf.len() == 1` when the stack is MIM (general assignment of one member; dummy ring only
     when `dummy_required`).
   - Per MIM unit: `b.drawn(Drawn { kind: Capacitor, nodes: [Pin("P"), Pin("N"), Unused], w, l })` (P = met4 top, N
     = met3 bottom, the recogniser's order) and `b.keepout(plate rect, KeepWhy::CapPlate { owner })`.
   - `pub fn c_u_af(p: &dyn Process, w_nm: i32, l_nm: i32) -> Option<f64>`: `None` without `c_area_af_um2`; else
     `c_area·(W+dw)(L+dw)/1e6 + c_perim·2(W+L+2dw)/1e3` (nm → µm², µm). Not called from `draw` (F3).
4. capacitor.rs:180-186: `grid_w = cols·unit_w`, `grid_h = rows·unit_h` (no gap terms); check `tile_w` and every
   caller still DRC-clean (existing sweep :333).
5. cellgen.rs:809-812: build `let ov = pdk.recipe("capacitor", model).map(|recipe| Overlay { pdk, recipe });` and
   pass `ov.as_ref().map_or(pdk as &dyn Process, |o| o)` to both CapArray and Capacitor. `dac_banks` key (:774)
   becomes `(p, d.model.clone(), w, l)`.
6. signoff_fixtures.rs `unverified`: a capacitor counts only when `pdk.recipe("capacitor", &d.model)` is None or has
   no `plate` layer (pass `&pdk` in). Fixtures: `benchmarks/fixtures/dac4_mim.spice` (dac4 with every C as
   `sky130_fd_pr__cap_mim_m3_1 W=5u L=5u`, same `m`), `benchmarks/fixtures/tq_chain.spice` (copy of the field-report
   file, header comment citing FR-1); BASELINE rows `("dac4_mim", 0, &[], true)`, `("tq_chain", 0, &[], true)`.
7. Then, separately measured: append `"cap"` to `mim_m3_1.aliases` (bare bench caps sized at 2.0 fF/µm² become the
   MIM they are sized as). Re-run dac4; if dac4 regresses (DRC/LVS/ERC rows), revert step 7 and report — do not
   touch dac4's baseline.

Tests:
- cap_array.rs (sky130 `mim_m3_1` overlay, `[1,1,2,4]`, W=L=5000): `a_mim_bank_is_drc_and_erc_clean`;
  `a_mim_bank_extracts_one_capacitor_per_unit` (extracted `C` lines == 8); `the_mim_unit_is_the_model_s` (every
  Capacitor `Drawn` w = l = 5000; `c_u_af(&ov, 5000, 5000)` = 53 282 ± 10 aF; per member Σ = n·C_u).
  `a_single_mim_is_one_unit`: `dev_nf = [1]`, W=L=21 870 → one variant at least, DRC/ERC clean, 1 drawn.
- pdk.rs guard test (step 2).
- capacitor.rs `a_merged_plate_is_the_units_area`: 4 units of 2000² → plate area 16·10⁶ nm².
- frontend/library/tests/drawn_cards.rs `a_mim_dac_signs_off_with_its_capacitors`: run `dac4_mim.spice` through
  `library::run` + signoff → 0 `lvs/` rows, 16 capacitor cards in the reference.

Run: `cargo test --release -p cells --lib cap_array capacitor && cargo test -p verify && cargo test --release -p
library --test drawn_cards && cargo test --release -p benchmark --test signoff_fixtures && cargo run --release -p
benchmark --bin bench local tq_chain`. Acceptance: T3/T4 for capacitors; dac4_mim 16 units compared; tq_chain DRC 0,
no `lvs/` row, no dr report containing `drawn short`; MIM unit 53 282 aF ± 10.

## CELL-10 MOS: legal region parity; ratioed mirrors merge (wires MAT-03) — class: judgment

(Code is determined below; judgment because the bench acceptance — mirror_ratio as one cell — also depends on the
annotator putting XM1..XM3 in one unitization, which EXT-06/15 own; unmeasured.)

Facts (mosfet.rs): enumerate :69-131 offers Cc1d iff `centroid_sequence(n_dev, nf = dev_nf[0])` (:90, equal counts
only), mirror/split styles inside `n_dev == 2` (:93-104, with the route-matched `retain` :102-104), two rows via
`centroid_sequence(n_dev, half)` (:117); draw picks `mirror_sequence` / chain / `finger_sequence` (:141-147);
`multi = n_dev > 1 && !mirror_pins`, `is_s` :327-328 (multi-device rows start on D); `finger_sequence` :784 uses
`greedy_centroid` for unequal counts **for every style** (so a `Single` [2,4] is interleaved today) and
`centroid_sequence` for Cc1d. Tests on `centroid_sequence`: :863, :881. Bug found: for an unequal route-matched pair
(e.g. [2,4]) today's enumerate offers only mirror variants, drawn with `mirror_sequence(nf = 2)`, i.e. [2,2].

Edits (mosfet.rs; `use analog::matching::pattern::{self, Outer};`, drop `greedy_centroid` from the import):
1. Add `legal_row` exactly as the spec (pub(crate), doc as the spec).
2. Add (FLOW-16 reuses it):
   ```rust
   /// Whether `enumerate` offers a centroid-exact row for these per-member finger counts: a route-matched equal pair
   /// draws only mirror orders ([`mirror_sequence`] exact), anything else a [`pattern::diffusion_cc_row`].
   #[must_use]
   pub fn cc_row_exists(dev_nf: &[u16], route_matched: bool) -> bool
   ```
   Body: `let cc = pattern::diffusion_cc_row(dev_nf, Outer::Drain).is_some();` then `n == 2 && dev_nf[0] ==
   dev_nf[1] && route_matched` → `cc && mirror_offset(usize::from(dev_nf[0])) == 0` (`cc` excludes odd nf, where
   `mirror_sequence` is empty and its offset a vacuous 0; factor
   the `offset` closure of `mirror_sequence` :767-770 into `fn mirror_offset(nf) -> i64` returning the chosen order's
   offset); else `cc`.
3. enumerate: `let cc = pattern::diffusion_cc_row(&s.dev_nf, Outer::Drain).is_some();` replaces the condition at :90 (same
   block structure); inside it the mirror/split block runs only when `n_dev == 2 && s.dev_nf[0] == s.dev_nf[1]`. Before building `styles`, keep `Single` only if
   `legal_row(&blocks, true) || legal_row(&blocks, false)` where `blocks` = members' fingers contiguous in order
   (n_dev == 1 is always legal). Two rows (:117): `pattern::diffusion_cc_row(&halves, Outer::Drain).is_some()`,
   `halves = dev_nf / 2`.
4. `finger_sequence(style, dev_nf) -> Vec<usize>`: Cc1d → `diffusion_cc_row(dev_nf, Drain)` if Some; otherwise
   blocks. Update the call (:145). Delete `centroid_sequence`.
5. `draw_row`: replace :327-328 by `let s0 = if self.mirror_pins { true } else { legal_row(sequence, true) };
   let is_s = |region: i32| (region % 2 == 0) == s0; debug_assert!(chain || legal_row(sequence, s0));` (`chain`
   moves above it). Single device and mirror pins keep today's S-first; a Cc1d row gives `false` (today's).
   Parallel groups with even counts now put boundaries on S (geometry changes for those cells; LVS unaffected).
6. Ports of the two old tests: `a_centroid_order_is_symmetric_and_never_abuts_two_drains` iterates
   `diffusion_cc_row(&vec![nf; n], Outer::Drain)` for n 2..=6, nf 1..=12 with the same three assertions;
   `a_quad_needs_four_fingers_before_a_centroid_exists`: `[1;4]` None, `[2;4]` None, `[4;4]` Some, `[2,2]` Some,
   `[1,1]` None.
7. Fixture `benchmarks/fixtures/mirror_ratio.spice` verbatim from the spec; BASELINE row `("mirror_ratio", 0, &[],
   true)`.

Tests (mosfet.rs): the spec's `legal_row_matches_the_region_rule` (exact vectors), `a_blocked_even_row_puts_
boundaries_on_sources` (`group_of(Nmos, 2, 2, 1680, 150)` → a `Single` variant exists; no two pins `d0:D`/`d1:D` with
equal `at`), `a_ratioed_mirror_merges` (`dev_nf = [2,4]`: a Cc1d variant exists, its row is `[0,1,1,1,1,0]`, no shared
D pad, `testkit::dirty_group::<Mosfet>` empty); sweeps (:812, :837) add `[2,4]`, `[2,6]`, `[4,8]` (set `dev_nf`
after `group_of`) and `[2,2]` Single. `cc_row_exists_follows_the_offered_variants`: `[2,2]` route-matched false,
`[8,8]` route-matched true, `[1,1]` route-matched false, `[2,2]` not route-matched true, `[1,2]` false.

Run: `cargo test --release -p cells --lib mosfet && cargo test -p cells --test cell_selfcheck && cargo test -p library
--lib cellgen && cargo test --release -p benchmark --test signoff_fixtures && cargo run --release -p benchmark --bin
bench local mirror_ratio`. Acceptance: T6 for MOS; mirror_ratio cells == devices − 2, its MatchedSet rows
satisfied, LVS clean. If the annotator splits the mirror, report it against EXT-15; do not special-case the fixture.

## FLOW-16 Fold classes per cell; centroid-feasible fold for matched pairs — class: judgment

(Judgment: the acceptance rests on placement reaching coincidence once the rows exist; unmeasured.)

Facts: `folds(netlist, pdk, gm_us)` cellgen.rs:621; netlist-wide class filter :654-660; parity :684-690; aspect
:691; callers lib.rs:416 (`&bias.gm_us`, `problem` already annotated at :405), cellgen.rs:52 (`enumerate`), tests
cellgen.rs:1251, 1677, 1678, 1784, 1797, lib.rs:2133. Unitizations from the annotator: one per (non-glue block,
size class), all `route_matching_required: true` (backend/annotator/src/constraints.rs:45-64). OTA: XM1/XM2 `W=10u
nf=2` → [2,2] route-matched; mirror orders at nf = 2 are never exact (`mirror_offset(2) != 0`), exact first at
nf = 8 (k = 4, 1.25 µm fingers); XM3/XM4 `W=20u` → [1,1], exact at k = 8. The OTA's XM1/XM2 class is already alone
netlist-wide (XM5 has another L), so **step 4 is what changes the OTA**, not the per-cell class.

Edits (cellgen.rs):
1. `pub fn folds(netlist: &Netlist, pdk: &Pdk, gm_us: &[Option<f64>], cells: &[(Vec<DeviceId>, bool)]) ->
   Vec<(u16, i32)>` — each entry = a unitization's devices and its `route_matching_required`. Class of `i` = the MOS
   members with equal `(kind, w_of, l)` of the first entry containing `i`, else `{i}`; replaces :654-660. Doc updated.
2. Callers: lib.rs:416 `&problem.constraints.unitization.iter().map(|u| (u.devices.clone(),
   u.route_matching_required)).collect::<Vec<_>>()`; cellgen.rs:52 the same from `constraints`; every test passes
   `&[]`.
3. Step 4: with `route = entry's flag` and `fingers` per member, filter the candidate `k` set (:694) by
   `cells::mosfet::cc_row_exists(&fingers.iter().map(|&f| (f * k) as u16).collect::<Vec<_>>(), route)` **when the
   class has ≥ 2 members, is not a stack and not all-parallel**, and at least one candidate k passes; if none passes,
   the unfiltered choice stands (the MatchedSet row stays violated and is reported).
4. Nothing else in the loop changes (`k_gate`, `P2P_SHARE`, parity, aspect).

Tests (cellgen.rs, sky130):
- `unrelated_same_size_devices_fold_independently`: the spec's netlists (`XA`, `XB` mirror, `XC` nf=16 unrelated);
  `folds(nl, .., &[(vec![A, B], false)])[A] == folds(nl_ab, .., &[(vec![A, B], false)])[0]` and `[C] ==
  folds(nl_c, .., &[])[0]`.
- `a_route_matched_pair_folds_to_a_centroid_row`: OTA's XM1/XM2 lines alone, `cells = [([0,1], true)]` → `k == 4`
  and `cells::mosfet::cc_row_exists(&[8, 8], true)`.
- Existing fold tests pass with `&[]`.

Run: `cargo test -p library --lib cellgen && cargo test --release -p library && cargo run --release -p benchmark
--bin bench local ota` (and `ota_constrained`, `tt_ota`). Acceptance (restated after MAT-04): LVS verdicts unchanged;
footprint before/after reported; the OTA input-pair and load-pair `MatchedSet` rows satisfied on ota,
ota_constrained, tt_ota (0/6 on the base). If placement leaves them violated with exact rows available, report the
measured residuals; do not loosen `tol_nm`.

---

Out of scope, reported:
- plan-03 D20 / CELL-17: the test `ecgr_is_n_plus_in_its_own_well_clear_of_the_nmos` moves to CELL-17 (GAP-05 note).
- FLOW-07 leftover (F7): bench `preprocess_spice` R/C sizing stays; needs an owner decision.
- Annotator unitizations mark every non-glue block `route_matching_required` (constraints.rs:63), so every equal
  pair is held to mirror orders; FLOW-16 step 4 folds around that rather than questioning it (EXT/MAT owners).
