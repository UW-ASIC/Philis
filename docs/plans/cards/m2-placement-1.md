# M2 placement, segment 1: implementation cards (PLC-07)

Branch `m2-placement`, worktree `philis-m2/placement`, base `c2940c6` (`git merge m2`: already up to date). Spec:
`plan-04-placement.md` `### PLC-07`; dag entry `{module: placement, milestone: M2, hard: [], note: "PLC-02, PLC-04 done."}`.
Line numbers below are this base's; where the plan's differ, the card wins. Every command: `export
PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk` first.

## PLC-07 Per-pair spacing from cell edge profiles — class: **do**

### Current code facts (plan corrections in bold)

- `gp::Rules { grid, clearance }` is at **`backend/gp/src/lib.rs:212-220`** (plan: 155-163), and has **no
  `rotate_90`** (PLC-03 moved rotation into `dp::locks`); the plan's `PlaceRules.rotate_90` field is dropped.
- `mechanics::encroach(l, a, b, clearance)` `backend/gp/src/mechanics.rs:221-234`, `encroachment(l, clearance)`
  `:236-242`, `report(nets, reqs, l, prices, clearance)` `:288-322` (PLC-04's clearance-residue row is in: `:316-319`).
- gp reads `rules.clearance` only in `report` calls `gp/lib.rs:280`, `:420`; gp test literal `Rules { grid: 10, clearance:
  270 }` `:735`.
- dp: `Sa.clearance` `backend/dp/src/lib.rs:119`, set `:141`; `encroach_moved` `:159-169`; `let gp::Rules { grid,
  clearance } = rules` `:243`; reports `:269`, `:390`; region sizing **`:272-283`** (plan: 252-258); legalizer call `:386`.
- `legalize::separate_overlaps(l, reqs, grid, clearance, max_sweeps)` `backend/dp/src/legalize.rs:16-75` (inline
  `ox/oy` at `:37-38`); test `clearance_pushes_past_mere_non_overlap` `:152-162`.
- dp tests: `const RULES: gp::Rules = { grid: 5, clearance: 2000 }` `backend/dp/src/tests.rs:4-5`, struct-update uses at
  `:237, :324, :348, :360, :410, :517`; `report_counts_clearance_only_residue` `:571-577`.
- Library: `place_rules(pdk)` **`frontend/library/src/lib.rs:717-726`** (max `min_spacing` over 7 roles, = 1270 on
  sky130), rebuilt per call at `:777` (gp), `:794` (dp), `:810` (`geometry::placement_metrics`), `:846`
  (`elaborate::antenna_diodes`, `elaborate.rs:328-357` → `dr::place_near`). **Not in the plan:**
  `geometry::placement_metrics(.., clearance, ..)` `frontend/library/src/geometry.rs:40-82` (PLC-01's
  `clearance_residue_nm2` `:82`; test at `:376-405` uses clearance 50).
- Rings/bridges in `Flow::epoch` **`lib.rs:812-819`** (plan: 607-613); n-well rect merge **`lib.rs:875-876`**
  (`geometry::merge_rects`; plan: 664-666); `signoff_shapes` call `:881`, def `:1767`. `CellSpace` `:1404-1421`, `new`
  `:1425`; constructed in `solve` `:418`; `Flow.cells` `:652`. In-crate fixture harness to copy: `size_tests`
  `:2160-2200` (`CellSpace` is private, so the library tests below are `#[cfg(test)]` in `src/`, **not**
  `frontend/library/tests/`).
- `Process::space` `kernel/core/src/process.rs:36`, `space_between` **`:83`** (plan: 75-78); `Pdk` impls
  `backend/verify/src/pdk.rs:1133-1143` (`space` = max of plain, via-array, wide ≤ cell feature, grown) and
  `:1168-1172` (`space_between` already maxes both orders; the plan's outer `max` is redundant, harmless).
  `Pdk::layer` `:1089-1099`. sky130 `cell.layers` has `diff poly rpoly(poly_rs) li licon mcon met1 nwell nsdm psdm tap
  diom(areaid_diode)`; `dnwell`, `npc`, `rpm` resolve by deck layer name; `res_implant`, `diode_mk` resolve to nothing
  on sky130 (role absent from every profile).
- `post_cell::well_bridges` `kernel/cells/src/post_cell.rs:406` (bulk = pin ending `:B`, `:421`), `implant_bridges`
  `:679`, `guard_rings` `:26`. **`post_cell::may_share_well` (REL-16) does not exist**: the well-merge test uses
  `well_net` only.
- **Sidecar registry exists** (`backend/verify/src/sidecar.rs`, FLOW-06): an unregistered `cell.placement_space`
  fails `validate` (`:157-159`). Its `Kind`s have no "array of arrays"; `Table` = any object. So the key is an
  **object**, registered `k("placement_space", Table, false, false, "frontend/library/src/lib.rs place_rules")`.
- **Pinned deck is GPurify `6341f18`** (`Cargo.lock`), not `8df8c09`; values in the plan's table are unchanged, lines
  are +7 (poly.6 +28). Checked in `~/.cargo/git/checkouts/gpurify-*/6341f18/pdks/sky130.deck`: difftap.3 L273,
  difftap.9 L276, difftap.11 L278, poly.4 L287, poly.5 L288, poly.9 L291, poly.9.difftap L292, rpm.6 L301, rpm.7 L302,
  npc.4 L316, nsd.7 L330, psd.7 L331, licon.9 L346, licon.13 L350, licon.14 L351, poly.6 L663; same-layer: dnwell.3 L232
  6300, nwell.2a L235 1270, rpm.2 L296 840, npc.2 L314 270, nsd.2/psd.2 L321/L323 380, licon.2 L340 170, li.3 L356 170,
  ct.2 L362 190. **Missing from the plan's table:** licon.11 `space(licon_dt, gate_nsc) >= 55nm` (L348) → add
  `licon,poly 55`.
- `Orient` `kernel/core/src/geom.rs:31-42` with `apply(x, y)` (`:53`); `Macro.bbox` `kernel/core/src/macro.rs:15`.

### Edits

1. `backend/gp/src/spacing.rs` (new; `pub mod spacing;` in `gp/lib.rs`): exactly the plan's step-1 block (`ROLES`,
   `DECK_ROLE`, `N`, `MERGEABLE`, `MARKER`, `Face`, `Edge`, `Profile`, `Src`, `Gap`, `SpacingTable`, `profile`,
   `oriented`, `Profiles`) with these changes:
   - `Profile { edge: [Edge; 4], well_net: Option<NetId> }` — no `matched` (PLC-13 adds it with its MAT-07 dep).
   - `Edge { inset: [i32; N], present: u32 }` (bit r set iff `inset[r] < i32::MAX`): `gap` iterates set bits only
     (hot path, called from `Sa::encroach_moved`).
   - `SpacingTable::new(p: &dyn Process, sidecar: &[(String, String, i32)], fallback: i32, lattice: i32) -> Self`,
     rules per plan step 2 (deck → sidecar wins when larger, `diff`/`tap` expand to `_in`/`_out` → `MARKER` same-role 0 /
     same-role `fallback` / cross 0 / any `other` pair `fallback`). `pub fn uniform(fallback: i32, lattice: i32) -> Self`
     (every `rule` = `fallback`, `src = Fallback`) for tests and callers with no profiles.
   - `gap(&self, a: &Profile, fa: Face, b: &Profile) -> Gap`: plan step 3 verbatim (`fb` = opposite face; `round_up` to
     `lattice`; `abut = g_hard <= 0`).
   - `profile(m: &Macro, p: &dyn Process, bulk: Option<NetId>) -> Profile`: plan step 4 (layer → first role whose
     `p.layer(DECK_ROLE[r])` matches, else `other`; diff/tap `_in` iff contained in one of the macro's own nwell rects,
     else `_out` — conservative; inset per face from `m.bbox`, 0 when touching).
   - `oriented(p: &Profile, o: Orient) -> Profile`: face map derived from `o.apply` on the four outward normals (no
     hand table), so it cannot disagree with `place_macro`.
   - `pub struct Profiles { pub of: Vec<Vec<[Profile; 8]>> }`.
2. `backend/gp/src/lib.rs`: replace `Rules` by
   `#[derive(Clone)] pub struct PlaceRules { pub grid: i32, pub spacing: Arc<SpacingTable>, pub profiles: Arc<Profiles> }`
   with `pub fn uniform(grid: i32, clearance: i32) -> Self` (empty `profiles`), `pub fn gaps(&self, l: &Layout, a: usize,
   b: usize) -> (i32, i32)` (plan step 5; empty `profiles` → `(fallback, fallback)`), `pub fn encroach(&self, l, a, b) ->
   f64` (`(gx + hw_a + hw_b − |Δx|)⁺ · (gy + hh_a + hh_b − |Δy|)⁺`), `pub fn encroachment(&self, l) -> f64`, `pub fn
   halo(&self, l: &Layout, c: usize) -> i32` (`gmax_c` of plan step 5; `fallback` when no profiles). `GpInput.rules:
   &'a PlaceRules`. `mechanics::encroach/encroachment(l, 0)` stay (plain overlap); `report(.., rules: &PlaceRules)`
   computes residue as `rules.encroachment(l) − encroachment(l, 0)`.
3. `backend/dp`: `place(.., rules: &gp::PlaceRules, ..)`; `Sa.rules: &'a gp::PlaceRules` replaces `clearance`;
   `encroach_moved` uses `self.rules.encroach`; region sizing `need += (2·hw + halo_c)·(2·hh + halo_c)`;
   `separate_overlaps(l, reqs, rules: &gp::PlaceRules, max_sweeps)` (grid from `rules.grid`; `ox/oy` from
   `rules.gaps`). Tests: `RULES` → `fn rules(c: i32) -> gp::PlaceRules { gp::PlaceRules::uniform(5, c) }`; every
   `Rules { clearance: X, ..RULES }` → `rules(X)`; no assertion changes.
4. `frontend/library/src/lib.rs`: `place_rules(pdk, cells: &CellSpace) -> gp::PlaceRules` built **once** in `solve`
   after `CellSpace::new` and stored on `Flow` (`rules: gp::PlaceRules`); `fallback` = today's scalar; sidecar parsed
   from `pdk.cell["placement_space"]` (object `"role_a,role_b": [nm, "source"]`; a malformed entry or unknown role is a
   load-time panic with the key named — a typo must not silently drop a rule). `CellSpace` gains `profiles:
   Arc<Profiles>` computed in `new` for every (cell, variant) × 8 orients, bulk = first `:B` pin net. Same-role
   `Src::Fallback` pairs are printed once per process (`std::sync::Once`) with role names. Callers `:777/:794/:810`
   pass `&self.rules`; `geometry::placement_metrics` takes `&gp::PlaceRules`; `antenna_diodes` gets
   `self.rules.spacing.fallback`. Delete the stale doc comment above `round_up` (`:1266-1268`, describes the old
   clearance).
5. `pdks/sky130.json` `cell.placement_space` (+ `placement_space_source` naming deck `6341f18`): the plan's 23 rows with
   the 6341f18 line numbers above, plus `licon,poly: [55, "licon.11 L348"]`. `backend/verify/src/sidecar.rs`: register
   the key (alphabetical). gf180/IHP/generic: no entries (plan risk note; they get fallback / 0).

### Tests

- `backend/gp/src/spacing.rs` `#[cfg(test)]`, table built by literal (`rule`/`src` set by hand, `lattice` 5): the plan's
  seven tests with its exact `Gap` assertions (`ndiff_facing_foreign_nwell_needs_rule_minus_insets` → `{false, 140}`;
  `same_bulk_pmos_may_abut` → `{true, 1270}`; `foreign_wells_never_abut` → `{false, 1270}`;
  `nmos_pair_with_edge_implants` → `{false, 380}`; `rotated_profile_permutes_faces` R90: L→B, B→R, R→T, T→L and Mx:
  B↔T, L/R fixed; `unmapped_layer_uses_the_fallback` → `min == 1270`; `diode_markers_do_not_force_the_fallback` →
  `{true, 0}`). Plus `table_is_symmetric_and_sidecar_wins_when_larger` on sky130 (`rule[i][j] == rule[j][i]` ∀ i,j;
  `rule[diff_out][nwell] == 340`, `rule[nwell][nwell] == 1270`, `rule[diff_in][diff_out] == 270`).
- `backend/gp` `uniform_rules_match_scalar_encroach`: for random layouts, `PlaceRules::uniform(g, c).encroachment(l) ==
  mechanics::encroachment(l, c)` (the refactor is behaviour-neutral without profiles).
- dp: all existing tests unchanged in assertions (`cargo test -p dp`).
- `frontend/library/src/lib.rs` `#[cfg(test)] mod spacing_tests` (harness as `size_tests`):
  - `shipped_cells_draw_no_unmapped_layer`: every cell × variant of the 10 fixtures on sky130 → count of shapes
    `profile` maps to `other` `== 0` (message lists layer names); print per-layer counts on gf180mcu, ihp_sg13g2,
    generic_finfet.
  - `table_gaps_are_drc_clean_on_sky130`: cells of `ota, dac4, rc_filter, chain4, pair`, variants 0 and 1, deduped by
    identical shapes; each ordered pair, x- and y-facing, at `min` and (when `abut`) 0, lattice-aligned; add
    `guard_rings`/`well_bridges`/`implant_bridges` as `epoch` does; `verify::drc` (`backend/verify/src/lib.rs:341`) on
    the pair vs on each cell alone → assert per-rule non-warning finding counts of the pair `<=` the sum of the two
    singles (no finding caused by the spacing; cells' own findings are not this item's). Print tightness (share failing
    at `min − 2·lattice`). Run in `--release`.
- Commands: `cargo test -p gp`; `cargo test -p dp`; `cargo test --release -p library spacing_tests -- --nocapture`;
  then `cargo test --workspace` (background).

### Acceptance (plan's, unchanged)

Bench protocol B (`m1-placement.md`), seeds 1–5, before (m2 base) and after: T2 area-usage median improves ≥ 20 %;
signoff DRC and LVS per fixture no worse. If 20 % is not reached, report the measured number; do not tune the
threshold. Risk: a cell whose edges carry nwell at inset 0 with foreign-bulk neighbours still pays 1270, so PMOS-heavy
fixtures gain less than the plan's NMOS example.

Out of scope, noted: dp hot-path cost of `gap` (bitmask loop) to be measured against PERF budgets; if dp wall time
rises > 20 %, add a per-(cell, variant, orient)-pair memo in `PlaceRules`.
