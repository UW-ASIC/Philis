# M2+ cells, segment 5 (M6): CELL-22

Worktree `philis-m2/cells`, branch `m2-cells`, fast-forwarded to `m2` at `b4054b6` (no conflicts).

| Item | Class | Why |
|---|---|---|
| CELL-22 | do | Hard dep REL-16 is merged (`CellFlags`, `may_share_well` at `post_cell.rs:457-472`, test `may_share_well_table` :828). EXT-23 tags exist (`analog::intent::Intent.aggressors/victims`). Uses no PLC-06. |

Command: `export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk; cargo test -p cells post_cell`, then
`cargo test -p library` (the `drawn` helper and the flow tests call `well_bridges`).

---

## CELL-22 Well bridges for unequal spans, gated by REL-16. Class: do

### Code facts (plan corrections)
- `well_bridges(placed: &[Macro], rings: &[Macro], process: &dyn Process) -> Vec<Macro>` is at
  `kernel/cells/src/post_cell.rs:485-532` (plan says 387 / 424-430: stale). Per cell: nwell bbox + bulk net from the
  `:B` pin (497-505). `hits(r, skip)` (506-511) grows `r` by `nwell_diff_enc` and tests diff/poly/psdm of every other
  cell and every ring. Pair loop 514-529: bridge only when `(a.y, a.h) == (c.y, c.h)` (x-facing) or
  `(a.x, a.w) == (c.x, c.w)` (y-facing), gap in `(0, 2·nwell_min_spacing]`, span `>= nwell_min_width`.
  No predicate is applied yet (REL-16 left the wiring to CELL-22).
- Plan-03 says REL-16 adds a `flags: &[CellFlags]` parameter; plan-06 L575 and card m2-reliability-3 say CELL-22 owns
  a closure. Use the closure (callers already know the cell index → members map; the kernel stays tag-agnostic).
- Plan-03 test name `an_injector_well_is_not_bridged_to_a_victim` (REL-16's) does not exist. REL-16 only wrote
  `may_share_well_table`. CELL-22 adds `a_forbidden_pair_is_not_bridged`, which REL-16's card names as its acceptance.
- Callers: `frontend/library/src/lib.rs:1375` (`Flow::epoch`, placed cells = `gr::place_macros(&macros, &layout)`,
  index-aligned with `self.cells.devices_of`, as at :1507/:1553) and `lib.rs:3013` (test helper `drawn`). Tags:
  `self.problem.intent.aggressors: Vec<Aggressor { device, inject, .. }>` and `.victims: Vec<Victim { device, .. }>`
  (`kernel/analog/src/intent.rs:229-251`). Injector = `Inject::MinorityElectron | Inject::MinorityHole`.
- The flow merges nwell with `geometry::merge_rects` (`frontend/library/src/geometry.rs:95`) only into rectangles.
  An offset bridge leaves three touching rects (A, bridge, B) that do not form one rectangle. Whether the gdsverify
  engine merges touching same-layer rects before spacing checks is not known. The new test decides it (below).

### Edits
1. `kernel/cells/src/post_cell.rs`, `well_bridges`:
   - New signature:
     `pub fn well_bridges(placed: &[Macro], rings: &[Macro], process: &dyn Process, can_share: &dyn Fn(usize, usize) -> bool) -> Vec<Macro>`
     with doc lines: `can_share(i, j)`: REL-16's legality for placed cells `i`, `j` (`may_share_well` over their
     tags). The check is applied per merged well: a pair is bridged only if every cell already in `i`'s well may share
     with every cell in `j`'s. Update the "same span" sentence of the doc to "overlapping spans".
   - Before the loop: `let mut comp: Vec<usize> = (0..placed.len()).collect();` and a 3-line `find` (path halving).
     Before accepting a bridge for `(i, j)`, require
     `(0..n).filter(|&p| find(p) == find(i)).all(|p| (0..n).filter(|&q| find(q) == find(j)).all(|q| can_share(p, q)))`.
     After accepting, set `comp[find(i)] = find(j)`. Without this, A–B and B–C bridges would join a forbidden A–C pair
     through B. `// ponytail: O(n²) per pair; wells per layout are few.`
   - Replace the identical-span tests with overlap. x-facing:
     `let (y0, y1) = (a.y.max(c.y), (a.y + a.h).min(c.y + c.h));`, require `y1 - y0 >= min_w` (was `a.h >= min_w`),
     `overlap = Rect { x: lo.x + lo.w, y: y0, w: gap_x, h: y1 - y0 }`,
     `hull = Rect { x: lo.x + lo.w, y: a.y.min(c.y), w: gap_x, h: (a.y + a.h).max(c.y + c.h) - a.y.min(c.y) }`.
     y-facing is the same with the axes swapped. Keep the x-then-y priority.
   - Choice: `hull` if `!hits(hull, [i, j]) && !near_well(hull, [i, j])`. Otherwise `overlap` if `!hits(overlap, [i, j])`.
     Otherwise no bridge. `near_well(r, skip)`: `r` grown by `space` meets the nwell of any other placed cell or ring.
     That is the same closure shape as `hits` on layer `nwell` with margin `space`. It is needed because only the hull
     leaves the facing wells' common span and could approach a third well. For identical spans `hull == overlap`, so
     today's bridges are unchanged.
2. `frontend/library/src/lib.rs:1375`: build the flags once per call:
   ```rust
   let flags = cell_flags(&self.problem.intent, &self.cells.devices_of);
   let bridges = cells::post_cell::well_bridges(&gr::place_macros(&macros, &layout), &rings, self.pdk, &|i, j| cells::post_cell::may_share_well(flags[i], flags[j]));
   ```
   Add a free fn next to `pin_currents` (~:1866):
   `fn cell_flags(intent: &analog::intent::Intent, devices_of: &[Vec<DeviceId>]) -> Vec<cells::post_cell::CellFlags>`.
   Map each device to its cell, then OR into that cell's flags: `injector` |= aggressor with a minority `inject`,
   `noisy` |= any aggressor, `sensitive` |= any victim. A device not in `devices_of` is skipped.
3. `lib.rs:3013` (test helper `drawn`): pass `&|_, _| true`. The helper has no intent. This matches today's behaviour.
4. Existing `post_cell.rs` test calls (568, 591, 597): add `&|_, _| true`.

### Tests (`kernel/cells/src/post_cell.rs`, `mod tests`, reusing `same_bulk_pmos_cells_share_a_well`'s cell/`shift`)
Lift `shift` into a test-module fn `shift(m, dx, dy, bulk)` that also shifts y. Use it in all three tests.
- `offset_wells_bridge_over_their_overlap`: cell A at (0, 0). Cell B at `dx = well.w + 600` and `dy = well.h / 4`,
  same bulk, so the spans overlap by `3·well.h/4` (the plan's [0,4000]/[1000,6000] shape, built from the drawn well).
  - No blocker: `bridges.len() == 1`. Its single nwell rect has `y == well.y` and `h == well.h + dy` (the hull) and
    `x == well.x + well.w`, `w == 600`.
  - With a diff blocker (as the existing test builds it) placed in the gap at `y = well.y + well.h + dy - 300` (inside
    the hull, outside the overlap by more than `nwell_diff_enc`): one rect with `y == well.y + dy` and `h == well.h - dy`
    (the overlap).
  - DRC/ERC via `testkit::findings` on A + B + bridge, nwell left unmerged (only touching rects), for both cases:
    `dirty.is_empty()`. If the engine reports `nwell` spacing/width between touching rects, the checker reads rects,
    not polygons. Then do not loosen the test. Stop and report it with the finding text: unequal-span bridges would
    need a polygon-aware merge before the checker (a verify/geometry item, out of CELL-22's scope).
- `a_forbidden_pair_is_not_bridged`: the existing equal-span pair. Use `flags = [CellFlags { injector: true, ..Default::default() }, CellFlags::default()]`
  and `can = |i, j| may_share_well(flags[i], flags[j])`.
  - `well_bridges(&pair, &[], &pdk, &can).is_empty()`.
  - With both flags default: `len() == 1`.
  - Transitivity: three cells in a row (A, B, C at `0, dx, 2·dx`, same bulk), A injector and C not. The predicate
    allows A–B and B–C, but `|i, j| !matches!((i, j), (0, 2) | (2, 0))`. Assert that exactly one bridge rect is drawn
    (`bridges[0].shapes.iter().filter(|s| s.layer == nwell).count() == 1`).
- `same_bulk_pmos_cells_share_a_well` and `may_share_well_table` still pass unchanged (only `&|_, _| true` added).
- `frontend/library`: `cell_flags` unit test `cell_flags_or_over_members`. Build an `Intent` with
  `aggressors = [Aggressor { device: DeviceId(1), inject: MinorityHole, .. }]` and `victims = [Victim { device: DeviceId(2), .. }]`.
  `devices_of = [[0, 1], [2]]`. Assert `flags[0] == CellFlags { injector: true, noisy: true, sensitive: false }` and
  `flags[1] == CellFlags { sensitive: true, ..Default::default() }`. If `Intent` has no cheap constructor, test through a
  `cell_flags_from(aggressors: &[Aggressor], victims: &[Victim], devices_of)` inner fn instead.

### Acceptance
- Bridges only grow (overlap ⊇ old identical-span case). The only removals are REL-16-forbidden pairs, so the nwell
  figure count per bench circuit falls or stays equal except where REL-16 now splits a forbidden pair. That split is
  intended; report it separately.
- Measure it: count touching-connected nwell components before/after on the `library` flow tests that already run
  signoff DRC (`cargo test -p library --release --test ota_cross_pdk`, `flow_smoke`). Use a temporary count, not
  committed. Put the numbers in the report.
- DRC findings on those tests are unchanged.
