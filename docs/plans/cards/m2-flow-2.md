# M2+ flow, segment 2 (M6): FLOW-15, GAP-16

Branch `m2-flow` after merging `m2` (`3b0819b`). Line numbers are at that commit. DAG: both items have no hard deps;
FLOW-15's note is "docs follow whatever has merged; run last", so FLOW-15's doc steps (5) describe the tree as merged
when it runs. PLC-06 (M1, not done) touches neither item.

Merge note: `pdks/{sky130,generic_finfet}.json` conflicts took m2's side (GAP-01 moved `lod_moat_ext_nm` into the
tier block at the top of `cell`, `[3000, 5000, 10000]`); `frontend/library/src/lib.rs` kept FLOW-09's
`topology`/`search` split and added m2's `perf_active` (threaded `run` → `topology` → `Flow`), IR budgets passed to
`elaborate::intent`, and `perf::score` for the unknown result (FLOW-09's `clock` kept). `cargo build -p library
--tests` is clean; `cargo test -p library --lib`: 124 pass, 1 fail, see "Out of scope".

## FLOW-15 Housekeeping, stale comments, docs and dead fields — class: do

### Code facts (plan line numbers are stale; current ones here)
1. `undrawable` (`frontend/library/src/lib.rs:1847-1852`) still names cell `i` with `netlist.devices.get(i)`: a cell
   index used as a device index (AF-31). Callers: `Flow::epoch` `lib.rs:1116` (`undrawable(&macros, self.netlist)`),
   `signoff` `lib.rs:1837`. Cells → devices: `self.cells.devices_of` / `Solution::devices_of` (`lib.rs:233`).
2. Bare pins: `Flow::parasitics` (`lib.rs:1171-1173`) and `Flow::common_nodes` (`lib.rs:1219-1221`) parse pins with
   `strip_prefix('d').and_then(split_once(':'))` and skip bare `G/D/S/B` pins of injected macros (AF-32).
   Generated cells always name pins `d{k}:T` (`kernel/cells/src/builder.rs:233`, `bjt.rs:203`); bare names come only
   from injected macros, plus `"GND"` (`cap_array.rs:548`, `resistor.rs:294`). `cellgen::bind_pins`
   (`cellgen.rs:1050-1059`) and `pin_currents` (`lib.rs:1579-1600`) already treat a bare `T` as member 0.
3. Stale comments (AF-28), current places:
   - `Config::op` doc `lib.rs:91-92` "falls back to zero power" and the eprintln `lib.rs:1611` "continuing with zero
     power": `bias` returns `cfg.device_power_uw` (`lib.rs:1616`).
   - `round_up` doc `lib.rs:1501-1503` describes dp's cell gap; the fn is integer round-up to `grid`.
   - `lib.rs:1160-1162`: an orphan doc ("Promote `epoch` to post-layout simulation…") sits above `parasitics`' doc.
   - `frontend/library/src/emit.rs:10-14` "v1 scope … a merged matched group … returns Unsupported": merged
     equal-leg groups emit as `MatchedPair` now (`emit.rs:119, 174` reject only ratioed groups).
   - `emit.rs:286` "nearest below with x-overlap": the filter (`emit.rs:287-292`) has no x-overlap test.
   - `examples/three_stage_opamp/src/main.rs:46, 82` name `macro_master::Generator`, which does not exist
     (`DeviceGen` / `Composition` do; AF-34).
4. Visualizer (AV-27, AF-34): `parse_layer_names(json)` (`kernel/visualizer/src/lib.rs:56-67`) reads a top-level
   `layers` the sidecar lacks, so every SVG label is `L{l}/{d}`. Callers: `kernel/visualizer/src/main.rs:10`,
   `benchmarks/src/bench.rs:492`, `benchmarks/src/gds2svg.rs:14`, test `lib.rs:1216`. `polys_from_shapes`
   (`lib.rs:1189-1201`) writes `LayerId.0` as the GDS layer with datatype 0; callers `show_macro` (`lib.rs:1206`),
   `examples/three_stage_opamp/src/main.rs:75`. `visualizer` depends only on `pnr_core` (no `verify`). Deck tables:
   `Pdk::layers: Vec<(String, LayerId)>` (`backend/verify/src/pdk.rs:44`), `Pdk::layer_gds() -> Vec<(u16, u16)>`
   indexed by `LayerId.0` (`pdk.rs:415`).
5. Docs: `docs/CRATES.md:102-129` "Open issues (next session)" is the pre-triage list; `CRATES.md:63` "EM width = I /
   1 mA/µm" (dr uses `em_limit(layer)` per deck, `backend/dr/src/lib.rs:187-195`). Acceptance grep today: `Generator`
   0, `EM_UA_PER_UM` 0, `variant_signoff` 1 (`docs/API-WISH.md:474`). API-WISH D4 is at `docs/API-WISH.md:80`.
6. Dead fields: `Unitization.same_variant_required` (`kernel/analog/src/cell.rs:50`) is written at 16 sites and read
   nowhere; `SeriesParallel::RepeatedStage` (`cell.rs:60`) is constructed nowhere. No CELL/PLC item wires either
   (plan-03 only lists the field). `tap_pitch_nm`/`enclosure_complete` are gone (GAP-05). The `*_moderate` scalars and
   their sidecar rows are already gone (MAT-07 in m2: no `_moderate` in `backend/verify/src/sidecar.rs` or
   `pdks/*.json`) — nothing left for this step there.

### Edits
1. `undrawable(cells: &'a [Macro], devices_of: &'a [Vec<DeviceId>], netlist: &'a Netlist)`: name a cell by its
   members, `"{name} ({model})"` joined with `", "` over `devices_of[i]` (`"?"` when `devices_of` has no row `i`).
   Callers pass `&self.cells.devices_of` / `&sol.devices_of`.
2. New `fn pin_member(name: &str) -> Option<(usize, &str)>` next to `pin_currents`: `"d{k}:T"` → `(k, T)`, `"GND"` →
   `None`, any other name without `:` → `(0, name)` (the `bind_pins` rule, doc links it). Use it at `lib.rs:1173` and
   `lib.rs:1221`; `pin_currents` keeps its own loop (it emits, not parses).
3. Rewrite each comment in fact 3 to what the code does (one line each); `main.rs:46, 82` → `cells::Cell` generator
   (`cells::mosfet::Mosfet`, the one `user_output_pmos` calls) and `macro_master::Composition`.
4. `kernel/visualizer/src/lib.rs`: replace `parse_layer_names` with
   `pub fn layer_names(pairs: &[(String, (u16, u16))]) -> LayerMap`; `polys_from_shapes(shapes, gds: &[(u16, u16)])`
   maps `s.layer.0` through `gds` (missing row → `(layer, 0)` as today); `show_macro(mac, title, gds)`. Callers build
   `pairs` as `pdk.layers.iter().map(|(n, id)| (n.clone(), gds[id.0 as usize]))` with `gds = pdk.layer_gds()`,
   skipping `(0, 0)` rows (derived layers). `bench.rs:492` and `gds2svg.rs:14` parse the `Pdk` they need (bench
   already has `p`; gds2svg loads `verify::Pdk::from_json`, it already depends on `library`). `visualizer` binary
   (`main.rs`) cannot load a deck without `verify`: it drops the `[pdk.json]` argument (it never produced a name,
   AV-27) and labels `L{l}/{d}`; usage line updated. Example `main.rs:75` passes `&pdk.layer_gds()`.
5. Docs: CRATES.md "Open issues" → the §1.6 triage table (plan-08 L150-168) re-checked against the merged tree, one
   owner per row, closed rows marked closed with their item; add "Stage contracts" (FLOW-02 lex key, FLOW-03 prices,
   FLOW-08 schedule — each one paragraph pointing at the type that holds it); `CRATES.md:63` → "EM width from the
   deck's per-layer limit (`DetailedCfg::em_limit`)". API-WISH D4 heading: append "— not implemented; superseded by
   FLOW-08 blame escalation"; `API-WISH.md:474` `macroMaster::variant_signoff` → `cellgen::price` (the code that does
   it). LAYOUT-FUNDAMENTALS rows from audit-07 §2.14.3 (§1g #65, §1e #53, §1a #5, §1c #39, LF-10) and SUBSTRATE3
   M3/M4 status: rewrite each to the merged code, citing file:line.
6. Delete `same_variant_required` (field and its 16 initialisers) and `SeriesParallel::RepeatedStage`.

### Tests
- `frontend/library/src/lib.rs` tests: `pin_member_reads_bare_and_ordinal_pins`: `pin_member("d1:D") ==
  Some((1, "D"))`, `pin_member("S") == Some((0, "S"))`, `pin_member("GND") == None`, `pin_member("dx:S") == None`.
- `common_node_sees_injected_macro_pins` (lib.rs tests): a two-NMOS pair on a shared source (the `PAIR` netlist of
  `frontend/library/tests/injected_macro.rs`), `M1` injected as a real transistor drawn by `cellgen::enumerate`
  for that device with its pins renamed `d0:T` → `T`; build the topology (`topology(..)` as `run` does) and assert
  `flow.common_nodes(&layout).nodes[0].a` is non-empty (the injected member's bare `S` pins) and `.b` non-empty.
  Fails today (`a` empty).
- `undrawable_names_the_cell_members` (lib.rs tests): `undrawable` over two empty macros with `devices_of =
  [[d1], [d0]]` names `d1` then `d0` (today it names `d0` then `d1`).
- Visualizer `layer_names_and_bounds`: built from `layer_names(&[("met1".into(), (68, 20)), ("via".into(), (68, 44))])`,
  same asserts; plus `polys_from_shapes(&[shape on LayerId(1)], &[(0,0), (68, 20)])[0]` has `layer 68, datatype 20`.
- Command: `cargo test -p library --lib && cargo test -p visualizer && cargo build -p benchmarks --bins &&
  cargo build --workspace --examples --tests` (step 6 touches every crate's initialisers).
- Acceptance: `grep -rn -e Generator -e variant_signoff -e EM_UA_PER_UM docs --exclude-dir=plans` → 0 lines; every
  CRATES.md open issue has an owner.

## GAP-16 Dominated-variant pruning before the escalation odometer — class: do

### Code facts
- `seed_assignment` is `cellgen.rs:324-338` (plan: 327); `price` (`cellgen.rs:352-369`) returns `(DRC+ERC count,
  pin HPWL)`; `escalate` is `cellgen.rs:395-411` (plan: 401-418), a mixed-radix odometer over every alternative,
  fastest digit by `pin_spread`. Called once per run per topology at `lib.rs:665` (`Topology::assignment0`) and per
  outer step at `lib.rs:742`.
- `Unitization` has no `class` field (spec's `Unitization.class == None` is stale). "Matched" here: a cell holding
  more than one device, or any member of a 2-device leaf of kind `DiffPair | CurrentMirror | Load | CascodePair`
  (exactly the leaves that emit a `MatchedSet`, `backend/annotator/src/emit.rs:6-12`).
- Alternatives cannot be removed: `UnitLib` (`lib.rs:1753-1759`) and `layout.variant` index alternatives by
  position. So pruning restricts the odometer's digits instead.
- Seed variant is argmin `(drc, hpwl)`; it can be dominated in `(w, h, drc)` by a lower-area, higher-HPWL
  alternative, so the seed is always kept (else the start digit is not in its own allowed set).

### Edits (`frontend/library/src/cellgen.rs`, `lib.rs`)
1. `price` stays. `seed_assignment(variants, matched: &[bool], pdk) -> (Vec<u16>, Vec<Vec<u16>>)`: per cell the
   prices once, the seed as today, and `allowed = keep(&costs, seed, matched[i])` where costs are
   `(drc, bbox.w, bbox.h)`.
2. `fn keep(cost: &[(usize, i32, i32)], seed: usize, matched: bool) -> Vec<u16>`: ascending indices; all when
   `matched`; else drop `v != seed` when some `u` has `drc_u ≤ drc_v, w_u ≤ w_v, h_u ≤ h_v` and at least one strict.
   Doc: BAL2-39; ponytail O(n²) per cell (n ≤ a few dozen alternatives).
3. `escalate(variants, allowed: &[Vec<u16>], current) -> Option<Vec<u16>>`: digit `i` steps to the next entry of
   `allowed[i]` above `current[i]`, else carries to `allowed[i][0]`; same `pin_spread` order. Empty `allowed[i]`
   (zero alternatives) is a fixed digit.
4. `Topology` gains `allowed: Vec<Vec<u16>>`; `topology()` computes `matched` per cell from `flow.cells.devices_of`
   and `annotator::block::leaves(&flow.problem.blocks)`. `search` passes `&t.allowed` to `escalate`.
5. `RunStats.pruned: u32` (doc: alternatives the odometer skips, summed over cells of the winner's topology);
   set in `search` (`stats.pruned = Σ alternatives − Σ allowed`) before `best.stats.merge(stats)` (which keeps
   `run`'s field). bench/CLI print it only if they already print `variant_escalations` (same place).

### Tests (`cellgen.rs` tests; command `cargo test -p library --lib cellgen`)
- `a_dominated_alternative_is_pruned`: `keep(&[(0, 100, 100), (0, 200, 100), (1, 100, 100), (0, 50, 300)], 0,
  false) == [0, 3]` (1 and 2 dominated by 0; 3 is not).
- `matched_cells_are_never_pruned`: same costs with `matched = true` → `[0, 1, 2, 3]`; and `keep` with the seed
  dominated (`[(0, 50, 50), (0, 100, 100)]`, seed 1) keeps `[0, 1]`.
- `escalate_still_terminates`: `space(3), space(2), space(1)` with `allowed = [[0, 2], [0, 1], [0]]` from `[0,0,0]`:
  visits exactly 4 distinct assignments, never a digit outside `allowed`, then `None` and stays `None`.
  `escalate_never_repeats_and_exhausts` and the `lib.rs` callers pass full `allowed` (`(0..n).collect()`), same asserts.
- Acceptance: T9 unchanged by construction (`price_calls` per run as before: pruning reuses the seed's prices; the
  existing FLOW-09 price-count test must still pass). `bench local` lex keys unchanged or better on every fixture:
  `PDK_ROOT=… cargo run --release -p benchmarks --bin bench -- local` (background, ~10 min) on the branch before and
  after; compare per-fixture keys. If any key regresses, report it with the fixture; do not keep the change.

## Out of scope (found while merging)
- `start_tests::same_seed_same_gds_bytes` (`lib.rs:2130`) fails in debug on `m2-flow` before this merge too
  (checked in a detached worktree at `eb0e766`): `gp::Prices::bind` debug_assert (`backend/gp/src/lib.rs:87`) "a
  batch kind is registered in both `hard` and `budget`" — the annotator puts `Orientation` Axis in `hard` and Phi in
  `budget` (`backend/annotator/src/emit.rs:145-149`, M1 MAT-05). Owner: placement (gp) or annotator; not FLOW-15/GAP-16.
