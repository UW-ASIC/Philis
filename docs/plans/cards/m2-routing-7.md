# M2+ routing, cards 7 (M6: RTE-27, GAP-12)

Merge of `m2` at cc2a359 (placement cards 7/8 only, no conflicts). DAG: RTE-27 hard deps RTE-12, RTE-13, REL-12;
GAP-12 hard deps RTE-14, REL-12. All are on m2. REL-12 landed as `Electromigration::front_row`
(kernel/analog/src/routing/em.rs:108-121, 217-229) and `DetailedCfg::em_front_row` (backend/dr/src/lib.rs:113-115).
RTE-14 landed: a net is EM-critical iff `!cold.term_k[ci].is_empty()` (dr lib.rs:913-956: `i_max > i_min` builds
`term_k`; the EM rounds at :1490 use the same test). REL-03 done. GAP-12 step 1 mostly applies to same-layer Ls from
RTE-28 (cards 6), which must land first in this module's order.
Commands: `export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk`; `cargo test -p dr`;
`cargo test --release -p benchmarks --test routing_quality -- --include-ignored --nocapture`;
`cargo test --release -p benchmarks --test signoff_fixtures` (stays 19 passed, ota/ota_constrained/tt_ota DRC 0).

| item | class |
|---|---|
| RTE-27 | do |
| GAP-12 | do |

## Shared groundwork (land with RTE-27)

### Current code (the plan's dr line numbers are stale)
- Via arrays: backend/dr/src/lib.rs:1229-1346, inside the redraw loop (`loop` at :1205). It runs per net over the
  drawn cuts. A cut is pushed unchanged (single) at :1276 (`best` finds no covering metal pair), at :1306 (the
  overlap holds fewer than 2 cuts, which is every minimum-width crossing) and at :1342 (no array cut cleared). It is
  dropped at :1286 (`continue` when a cell cut already joins). When exactly one array cut is placed, that is also a
  single cut. `cut_space = cfg.space(c.layer, 0, 0, size)` (:1294), pitch = size + cut_space.
- Pads: `build_routes` (:2363-2400) draws per via `pad_along × pad_across` per layer (`cfg.layers[k].horizontal`),
  else the `Cut` tuple's `below`/`above` square, centred on the cut.
- Fill section (:1350-1465): `flat` (:1352), and then the ingredients that do not depend on routes:
  `fill_layers` (:1360), `fill_metal`, `hard_fill`, `cell_of`, `fill_space` (:1400). Then `lead` (:1412: image net →
  `(leader, map)` from `cold.mirror` exact pairs), and per net `foreign` (:1423-1430: other nets' `flat` shapes,
  other nets' `term_rects`, cell metal not `cell_of(ni)`, and hard blockages binding the net). Origin shift at :1460.
- `map_rect(r, map, grid)` (:2101) uses only `grid.pitch`.
- `RouteStats::single_cut_vias` exists (:273) and is never written. The doc at :252-257 says so.

### Edits
1. Move the route-independent fill ingredients (`fill_layers` … `fill_space`, :1360-1403) up so they sit just
   before the via-array block (:1229). This is a pure move inside the same loop body.
2. Extract the `foreign` builder into a closure defined after them:
   `let statics = |ni: usize| -> Vec<Shape>`. It returns the non-route part (other nets' `term_rects`, cell metal
   not in `cell_of(ni)`, and hard blockages binding `ni`). Fill's `foreign` becomes
   `flat.iter().filter(|(i, _)| *i != ni).map(|(_, s)| *s).chain(statics(ni)).collect()`, with the same set as today.
3. New free fn (near `map_rect`):
   ```rust
   /// Adds to each net, per site, the first alternative (a set of shapes) whose every shape clears
   /// `clear(net, shape, wires)`. For an exact pair (`image[a] = Some((b, map))`, leader → image), an
   /// alternative counts only if its `map` image also clears for `b`. Both are added or neither is, and
   /// `b`'s own sites are ignored (its leader decides them). Returns, per net and site, the alternative
   /// taken. An image net's entries are empty.
   fn add_where_clear(
       wires: &mut [Vec<Shape>],
       sites: &[Vec<Vec<Vec<Shape>>>],
       image: &[Option<(usize, gr::LatticeMap)>],
       pitch: i32,
       clear: &dyn Fn(usize, Shape, &[Vec<Shape>]) -> bool,
   ) -> Vec<Vec<Option<usize>>>
   ```
   It works on live `wires`, so a shape added for one net is seen by the next. The map uses `map_rect`. Change
   `map_rect`'s third parameter to `pitch: i32` and update its 3 call sites (`grid.pitch`).
4. In `route`, the `clear` closure:
   - Cut layer (in `cuts`): every cut of every net, own net included. Same-net cut spacing binds too (:1323). A
     shape equal to `s` is skipped. Requires `rect_gap ≥ cut_space(layer)`.
   - Metal: `statics(n)` plus `wires[m]` for `m ≠ n` on that layer. Requires
     `rect_gap ≥ cfg.space(l, min_dim(s), min_dim(g), fill_space(l))`.
   - `image[compact[a]] = Some((compact[b], map))` for `cold.mirror[a] == Some((b, map, true))`, the inverse of
     `lead`.
   - ponytail: linear scan per candidate shape (O(sites × shapes)). Switch to a `ShapeIndex` if `us_fill` rises
     visibly on ota.
5. New free fn `fn via_pads(cfg: &DetailedCfg, layers: &[LayerId], cut: Cut, i: usize, r: Rect) -> [Shape; 2]`.
   It is the pad code of `build_routes` :2388-2396, centred on `r`. `build_routes` calls it, so there is one copy.

## RTE-27 Redundant vias: do

Scope note: after REL-12 (C8) the spec has two steps (double, count). The array-orientation acceptance quoted by the
reliability card (m2-reliability-3.md:97-100) is not RTE-27's step. A second cut along the lower wire lies along that
metal's current, so REL-12's front row gains nothing there (n_front = 1 on the lower metal). That is intended: this is
redundancy against voiding, and the EM count is unchanged. Report bench `em cuts` before/after anyway.
"Vias" here means stack cuts (`cuts`). The pin-access cut (`cfg.pin_access`, sized by RTE-14 from current) is
excluded. The card says so in the stats doc.

### Edits (backend/dr/src/lib.rs)
1. `RouteStats`: add `pub stack_vias: u32` ("stack vias drawn, each array or single cut once; RTE-27's
   denominator"). Fix the doc at :252-257: `single_cut_vias` is "stack vias left with one cut after RTE-27's
   doubling (exact-pair images included)". Add both to the per-drawing reset at :1209.
2. Array block: add `let mut singles: Vec<Vec<(usize, Rect)>> = vec![Vec::new(); n_nets]` (lattice index `i`, cut
   rect).
   - Push `(i, c.rect)` at :1276, :1306 and :1342.
   - After the array loop, when `out.len() - before == 1`, push `(i, out[before].rect)`.
   - `stats.stack_vias += 1` for every stack cut that is not dropped at :1286.
3. Post-pass right after `*wires = out` (:1346), before `us_geometry`:
   - Per net, one site per single.
   - `horiz = cfg.layers.get(i).map_or(i % 2 == 0, |s| s.horizontal)`.
   - For `d in [1, -1]`, `c2` = the cut shifted `d·pitch` along x if `horiz`, else along y.
   - Alternative = `[cut c2, Shape{layers[i], bbox(lo(c), lo(c2))}, Shape{layers[i+1], bbox(hi(c), hi(c2))}]`, from
     `via_pads`. The bbox keeps the upper pad contiguous: on sky130 via1 the pitch 320 is more than met2
     `pad_across` 280.
   - Call `add_where_clear`. Then `stats.single_cut_vias = Σ None`, counting a leader's `None` twice (its image
     stays single).
   - Matched pairs that are not exact (`pairs_fallback`) have no image map. They are treated as free nets.
4. `flat` (:1352) is computed after the post-pass (it already is, since it follows), so fill sees the new pads.

### Tests
- `dr` `every_free_via_is_doubled`:
  - Three nets 0..2 on `test_cfg()`. Each has `pin(n, x, y)` on LAYERS[0] and a second pin
    `(NetId(n), Rect{..170×170}, LAYERS[1])`, 4 µm apart in x and y, with nets 4 µm apart. Pins on two layers give
    one layer change where the lattice allows it. Call `DetailedRoute{cfg}.route(..)` for the stats.
  - Assert `report.hard_violations` is empty and `stats.single_cut_vias == 0`. Assert `stats.stack_vias >= 3`, and
    that every net has cuts on `CUTS[0].0`.
  - Assert every such cut has exactly one same-net cut at centre offset `(±200, 0)`. CUTS size 100, and the
    `space` fallback gives cut_space 100 at :1294; layer 0 is horizontal.
- `dr` `pair_vias_double_symmetrically`: unit test on `add_where_clear`.
  - Net 0 has one single-cut site, and net 1 is its image under `LatticeMap::MirrorX { k }` with pitch 400.
  - `clear` rejects any shape intersecting a foreign rect placed beside net 0's both candidate pads. The rect is on
    LAYERS[1], within spacing of both `+` and `−` upper bboxes, and away from net 1's images.
  - Assert the result is `[[None], []]` and both nets' `wires` are unchanged.
  - With the foreign rect removed, assert `[[Some(0)], []]` and that net 1 gained exactly `map_rect` of net 0's
    three new shapes.
- `routing_quality.rs` new `#[ignore = "full flow on ota; run with --include-ignored"] fn vias_and_corners_on_ota`.
  It prints `single_cut_vias`, `stack_vias` and the bench `em cuts` row count. It asserts `stack_vias > 0` and
  `10 * single_cut_vias <= stack_vias` (acceptance ≤ 10 %). If this fails, report the numbers. Do not weaken.
- Existing `dr` tests that assert an exact stack-cut count: list each one in the report. Change a count to the
  doubled value only when the test's subject is something else, with a `// RTE-27` comment. Never turn `==` into
  `>=`.
  - The pair tests (`mirrored_pins_route_as_exact_mirrors`, `fill_on_a_pair_is_mirrored`) must stay imaged unchanged.
  - `adjacent_foreign_vias_keep_the_along_track_halo` must keep `gaps ≥ 140`.
- Signoff: 19 passed, DRC 0.

## GAP-12 Corner support fill and via arrays off corners: do

### Current code (plan stale)
- `jog_legs` is now :2001-2006. It overruns by `w/2`, so the union is a flush L. Callers: :704, :790, :2065, :2206.
- There is no `widen` fn. Corner blocks come from `gr::extract_geometry` (backend/gr/src/lib.rs:1493-1540). A via
  whose side is over one track gets a merged `(sv+wv) × (sh+wh)` block, identical on both layers (:1519-1529).
- Every bend between trunks is a layer change: H on even layers, V on odd. `emit_run` (gr :1544-1576) splits a
  same-layer run at a non-straight step. Same-layer trunk L-corners exist only after RTE-28's wrong-way steps. Access
  jogs give same-layer Ls today.
- Test `a_two_by_two_corner_gets_four_cuts` (dr :4868-4901) pins the current corner array.

### Edits (backend/dr/src/lib.rs)
1. Step 2, the array off the corner. It runs before the array block (:1229) and after `break_shorts`, for each
   EM-critical net (`crit[n]` from `cold.term_k`) and each stack cut `c` (lattice index `i`):
   - Find the corner block `B`: a same-net rect on `layers[i]` containing `c`, with an identical rect on
     `layers[i+1]`, that is neither of `via_pads(c)`.
   - Find the upper run `R`: a same-net rect on `layers[i+1]` containing `c`, `≠ B`, whose long axis is ⟂ to layer
     `i`'s direction. It must extend past `B` on exactly one side `d` (an L, not a T).
   - `L` = `B`'s extent along that axis. Stub `S` = the rect adjacent to `B` on side `d`, with `B`'s across extent
     and length `L`. Require `contains(R, S)`. `c'` = `c` shifted by `L` along `d`.
   - Site with one alternative `[Shape{layers[i], S}, Shape{cut, c'}]`, then `add_where_clear`. Where taken, remove
     `c` from the net (and `map_rect(c)` from its image).
   - The array block's `best(lo, hi, c')` then picks `S` and `R`. The overlap is `S`, a straight run, and the array
     fills it (H15-39 recipe (b), Lienig Fig. 4.29).
   - No crowding credit (spec). The EM rounds' cut bump (:1507-1511, nodes within one `pitch` of the cut) still
     finds the upper run's nodes inside `S`. Verify this with the test below.
   - Wrap it in `fn corner_stubs(wires: &[Shape], cfg, layers, cuts) -> Vec<(Rect /*c*/, [Shape; 2])>`, pure and
     per net.
2. Step 1, support squares. It runs after RTE-27's post-pass, for EM-critical nets only.
   ```rust
   /// Inner-corner support squares (EM-27) of every flush L in `wires`: a horizontal and a vertical rect on one
   /// metal, `a.h == b.w == w`, meeting in a `w × w` square at an end of each. Each square is the `w × w` square
   /// diagonally inside the L, touching both legs. A T or + (the square not at an end of both) gives none.
   fn support_squares(wires: &[Shape], metals: &[LayerId]) -> Vec<Shape>
   ```
   - One site per square: `[square]`, already covered sites skipped. Then `add_where_clear`.
   - `stats.corners_unsupported` (new `u32`, reset at :1209) counts the `None` results, a leader's twice.
   - Squares are built from on-grid wire edges, so they are on grid. The spec's "snap" is a no-op, and the card says
     so in the doc.
   - Ls of other widths (merged `k > 1` wrong-way runs are not flush): none. A `ponytail:` comment names that ceiling.
3. `RouteStats` doc: `corners_unsupported` (GAP-12).

### Tests
- `dr` `a_critical_corner_gets_a_support_square`: unit test on `support_squares`.
  - Legs `a = Rect{0,0,1000,260}` and `b = Rect{740,0,260,1000}` on LAYERS[0] give exactly
    `[Shape{LAYERS[0], Rect{480,260,260,260}}]`.
  - The mirror case `b = Rect{0,0,260,1000}` gives `Rect{260,260,260,260}`.
  - A T, with `b` at x 370 crossing `a`'s middle, gives `[]`.
  - Unequal widths (`b.w = 300`) give `[]`.
- `dr` `a_corner_via_array_moves_onto_the_straight_run`: the fixture of `a_two_by_two_corner_gets_four_cuts`.
  - Assert no via cut lies inside the 700×680 corner block `B`.
  - Assert exactly 4 via cuts lie inside one LAYERS[0] rect adjacent to `B` and inside a LAYERS[1] run.
  - Assert `report.hard_violations` is empty (EM included, no DRC row).
  - Update `a_two_by_two_corner_gets_four_cuts`: it still asserts `widest == (680, 700)` and 4 cuts. "In the corner"
    becomes "in the stub beside the corner", with a `// GAP-12` comment. The count stays `== 4`.
- `vias_and_corners_on_ota` (above) also prints `corners_unsupported` and asserts `== 0` (acceptance: every L-corner
  of a critical net has its square). If it fails, report the count. Do not weaken.
- Signoff: 19 passed, DRC unchanged (0 on ota/ota_constrained/tt_ota).

Risks: the stub and squares add metal after search, so they never move other nets. They are skipped when not clear,
and the stats count the skips. An EM-critical exact pair gets squares and stubs on both nets or on neither.
