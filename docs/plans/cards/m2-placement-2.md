# M2 placement, segment 2: implementation cards (PLC-29)

Branch `m2-placement`, worktree `philis-m2/placement`, base `a2fece9` (`git merge m2` brought GAP-01, MAT-07, MAT-08,
MAT-13 and the routing segment 1). Spec: `plan-04-placement.md` `### PLC-29`, plus `98-gap-critic.md` GAP-01 step 5
(C20). dag entry: `{module: placement, milestone: M2, hard: [GAP-01], step_level: [EXT-16 "class per pair", default
Moderate], note: "Carries GAP-01 step 5 (C20)."}`. Line numbers below are this base's; where the plan's differ, the
card wins. Every command: `export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk` first.

## PLC-29 Matched-pair environment (WPE/OSE) evaluated live in placement — class: **do**

### Dependencies

- PLC-06 is listed as not done for M1. PLC-29 needs only one part of it: units in the placed `Layout`. That part is
  in. `Layout.units: Arc<UnitLib>` is at `kernel/core/src/layout.rs:40`, `UnitLib::of_device`/`placed` at
  `kernel/core/src/units.rs:105-128`, and the flow passes `units: cells.units.clone()` into gp at
  `frontend/library/src/lib.rs:894`. The power that PLC-06 also carries is not read here. Nothing waits on PLC-06.
- GAP-01 is merged. **Its step 5 is already in**, because the plan's "MAT-07 switches both" happened at merge time:
  `wpe_min_nm = mos_env(MatchClass::Moderate, pdk).wpe_nm` at `lib.rs:1190`, and `ose_range_nm =
  pdk.tier("lod_moat_ext_nm", Minimal)` at `:1191`. PLC-29 moves these two reads without changing them. EXT-16 is not
  in, so the class stays `Moderate` (dag: default Moderate). When EXT-16 lands, it makes the class a per-pair field
  (out of scope here).
- Out of scope, reported: `:1191` reads the **Minimal** tier for OSE, while the `Surroundings` doc
  (`environment.rs:15-17`) says "moderate LOD extension". This is GAP-01/MAT's call. PLC-29 keeps the read as it is.

### Current code facts (plan corrections in bold)

- `Flow::environment(&self, layout, rings)` is at **`lib.rs:1122-1195`** (plan: 806-876). It does the following:
  `gr::place_macros(&cellgen::realize(&self.cells.variants, &layout.variant), layout)`; returns `Environment::default()`
  when `pdk.layer("nwell")` or `pdk.layer("diff")` is `None`; collects the nwell union over placed cells plus `rings`;
  `wpe(p)` is the distance to the union boundary (`:1142-1159`); `diffs(c)` are cell c's placed diff rects; for each 2-device leaf of
  `annotator::block::leaves(&self.problem.blocks)` with kind `DiffPair | CurrentMirror | Load` (`:1166-1170`):
  `mean_wpe(d)` takes the mean over `layout.units.of_device(layout, d)` of `wpe(..).min(1e7)` (∞ if there are no units),
  and `ose(d)` is the min rect gap from d's cell (found by `self.cells.devices_of` position) to every other cell's diffs.
- Its callers: the routing arm of every epoch, `budgets.add_routing(&[.., Box::new(self.environment(&layout, &rings))],
  ..)` at **`lib.rs:1006`** (plan: 680), and the final report `metadata.add_routing(&[.., Box::new(flow.environment(
  &best.layout, &best.rings))], ..)` at **`lib.rs:655`** (plan: 431).
- `place_macro` is at **`kernel/core/src/macro.rs:118-139`** (`anchor`/`(ax, ay)`/`shift` at `:123-128`; plan:
  46-51). It is re-exported at `kernel/core/src/lib.rs:26`. `gr::place_macros` is `pnr_core::place_macros`
  (`backend/gr/src/lib.rs:14`).
- `Environment`/`Surroundings`: `kernel/analog/src/placement/environment.rs:18-81`, re-exported at
  `placement/mod.rs:13`. `RuleBatch<On>: Send + Sync` is at `kernel/analog/src/rule.rs:172`. Its defaults are `touched`
  none, `retarget` no-op and `criticality` 1.0.
- dp evaluates the whole budget on every trial: `analog_theta(self.reqs, l)` (`backend/dp/src/lib.rs:185, :210`) is
  `Σ residual` over `reqs.budget` (`backend/gp/src/mechanics.rs:276-279`), and `analog_cost` (`:248-257`) also prices
  every budget batch's residual. gp's `analog_cost` and `report` read it too. **There is no `touched`-scoped eval yet**
  (PLC-10 beyond step 0 is M3), so "global batch" means it is evaluated in full on every call.
- `solve`: `CellSpace::new(..)` is at **`lib.rs:451`** (plan: 275), and `CellSpace::new` retargets
  `problem.placement` at `:1556-1564` (plan: 1152). A batch pushed after `:451` is already cell-indexed and needs no
  `retarget`. `debug_check_retargeted` (`:1654`) runs inside `new`, before the push. `Flow` is at `:704-730` and is
  built at `:486`. `cells.devices_of` is at `:1525`, `cells.variants: Vec<gp::VariantSpace>` (`alternatives: Vec<Macro>`)
  at `:1513`.
- Report rows: `MetadataReport { placement, routing: Vec<BudgetStatus> }` is at `frontend/library/src/metadata.rs:83-85`,
  with `BudgetStatus.kind: String` at `:20`.

### Exact edits

1. `kernel/core/src/macro.rs`: add the following and have `place_macro` use it. There is no behaviour change.
   ```rust
   /// `r` (in the macro's local frame, whose bbox is `bbox`) as cell `i` of `l` places it: turned by `l.orient[i]`,
   /// then shifted so the turned bbox's lower-left lands on `(x − hw, y − hh)`. The one stamp [`place_macro`] applies.
   #[must_use]
   pub fn place_rect(bbox: Rect, r: Rect, l: &Layout, i: usize) -> Rect
   ```
   The body is `:122-128` (`o = l.orient.get(i).copied().unwrap_or_default()`, `anchor = o.apply_rect(bbox)`, shift).
   `place_macro` keeps its `i >= l.x.len()` early return and maps every rect through `place_rect(m.bbox, r, l, i)`. Add
   `place_rect` to the `pub use` at `kernel/core/src/lib.rs:26`.
2. `kernel/analog/src/placement/environment.rs`: add `EnvGeo` and `LiveEnvironment` as in the plan, with these
   corrections:
   ```rust
   /// Per cell, per variant, in the macro's own frame: its bbox, n-well rects and diff rects.
   #[derive(Debug, Default)]
   pub struct EnvGeo { pub bbox: Vec<Vec<Rect>>, pub wells: Vec<Vec<Vec<Rect>>>, pub diffs: Vec<Vec<Vec<Rect>>> }
   #[derive(Clone, Debug)]
   pub struct LiveEnvironment {
       /// `(member a, member b, cell of a, cell of b)`.
       pub pairs: Vec<(DeviceId, DeviceId, u16, u16)>,
       pub geo: std::sync::Arc<EnvGeo>,
       pub wpe_min_nm: f32,
       pub ose_range_nm: f32,
   }
   impl LiveEnvironment {
       /// Each pair's [`Surroundings`] on `l`, with `extra_wells` (placed, world frame) joining the cells' n-wells.
       pub fn surroundings_with(&self, l: &Layout, extra_wells: &[Rect]) -> Vec<Surroundings>;
       pub fn surroundings(&self, l: &Layout) -> Vec<Surroundings> { self.surroundings_with(l, &[]) }
   }
   impl RuleBatch<Layout> for LiveEnvironment { .. }
   ```
   - `surroundings_with` is the body of `lib.rs:1131-1194` moved verbatim, with these changes. The wells are
     `extra_wells` plus `place_rect(geo.bbox[c][v], r, l, c)` for each `r` in `geo.wells[c][v]`, `v = l.variant[c]`, over
     `c in 0..l.x.len()`. `diffs(c)` is the same over `geo.diffs`. Precompute all placed diffs once per call, not per
     pair. `ose` uses the pair's stored cell instead of searching `devices_of`. The `wpe`, `rect_gap`, `inside` and
     `covered` closures stay bit-identical.
   - `RuleBatch`: `cost`, `violations`, `residual`, `unknown`, `count` (= `pairs.len()`) and `worst_usage` are
     `Environment(self.surroundings(l))`'s. One way to write it is a private `fn now(&self, l) -> Environment` that each
     method delegates to. `kind` = `"Environment"`. Keep the defaults: `touched` none, `retarget` no-op. `criticality`
     stays at the default 1.0, the same as `Environment`'s today (the plan's "criticality of Environment" is that
     default).
   - Export `EnvGeo, LiveEnvironment` at `placement/mod.rs:13`.
   - `// ponytail:` comment on the impl: "global batch, re-measured on every dp trial (`O(P·Σ_c rects)`); if FLOW-09's
     dp timer shows > 30 % of dp time here, cache unmoved cells' placed rects per trial (PLC-10's `Eval` knows the moved
     set)."
3. `frontend/library/src/lib.rs`:
   - New free fn `fn live_environment(problem: &Problem, cells: &CellSpace, pdk: &Pdk) -> analog::placement::LiveEnvironment`.
     It builds `EnvGeo` from `cells.variants[c].alternatives[v]` (bbox; shapes filtered by `pdk.layer("nwell")` /
     `pdk.layer("diff")`). Its pairs come from the leaf filter at `:1166-1170`, with cells from `cells.devices_of`
     (`u16`). `wpe_min_nm`/`ose_range_nm` are read exactly as at `:1190-1191`. If either layer is `None`, it returns
     empty `pairs` (the same as today's `Environment::default()`).
   - In `solve`, after `:451`, do `let env = live_environment(&problem, &cells, pdk); if !env.pairs.is_empty() {
     problem.placement.budget.push(Box::new(env.clone())); }`. Store `env` in a new `Flow` field `env:
     analog::placement::LiveEnvironment` (doc: "Matched pairs' live WPE/OSE (PLC-29); also the report's, plus ring
     wells.").
   - `Flow::environment(&self, layout, rings)` becomes this: ring wells = `rings` shapes on `pdk.layer("nwell")` (they
     are already placed); return `Environment(self.env.surroundings_with(layout, &ring_wells))`. Delete the old body.
     There is one implementation, so it cannot drift.
   - `Flow::epoch` `:1006`: drop `Box::new(self.environment(&layout, &rings))` from the routing arm. The placement row
     now carries it in Θ. Keep `:655` (the final report with rings, the T9 judge). It is a report, and nothing scores
     Θ from it.
4. Change nothing else. `Prices` key untagged batches by `(kind, ordinal)` (`gp/src/lib.rs:26-29`), so the extra
   placement batch needs no price plumbing.

### Tests

- `kernel/core/src/macro.rs` tests, `place_rect_is_place_macros_stamp`: a 1000×200 macro with a shape at
  `(100,50,300,40)` placed under each of the 8 `Orient`s at `x = 5000, y = 700`. Assert `place_macro(&m, &l,
  0).shapes[0].rect == place_rect(m.bbox, m.shapes[0].rect, &l, 0)` for all 8.
- `kernel/analog/src/placement/environment.rs` tests, `a_foreign_well_near_one_member_violates`. This is the plan's
  test, with **corrected numbers**. Three cells: a and b are 1000×1000 bbox, with diff rect = bbox and one unit at
  `(500,500)`, owners `DeviceId(0)`/`DeviceId(1)`. The third cell is a 2000×1000 bbox with an nwell rect = bbox and no
  diff. Build `UnitLib::build(vec![0,1], &[vec![D0], vec![D1], vec![]], ..)`. Use a `Layout` like `units.rs:141`, with
  centres a `(500,500)`, b `(3500,500)`, c `(5500,500)`, so c's well's left edge is at 4500, 1000 nm right of b's
  centre. `wpe_min_nm = 3000`, `ose_range_nm = 0`. Assert `violations == 1` and `surroundings(&l)[0].wpe_nm == [4000.0,
  1000.0]`. Then move c to `x = 41000`, so the edge is at 40000 and the distances are 39500 and 36500. Both are past
  `10·wpe_min = 30000`, so the skew is 0. Assert `violations == 0`, and that usage equals `3000/36500` within `1e-6`.
  The plan's "≥ 37,000 … ≤ 0.081" was measured from the wrong centre.
- Same file, `ring_wells_join_the_report_only`: the same layout with c far away. `surroundings_with(&l, &[Rect{x:
  4000, y: 0, w: 1000, h: 1000}])[0].wpe_nm[1] == 500.0`, while `surroundings(&l)[0].wpe_nm[1] > 30000.0`.
- `frontend/library/src/lib.rs`, new `#[cfg(test)] mod environment_tests`,
  `env_geo_places_like_place_macros_on_ota`. This replaces the plan's `live_environment_matches_the_flow_report_without_rings`,
  which is tautological once step 3 makes `Flow::environment` call `surroundings_with`. Build sky130 `ota` cells and
  problem as `spacing_tests::cells` does (keep the `problem`), then `env = live_environment(&problem, &cells, &pdk)`.
  Assert `env.pairs.len() >= 1` and that every pair's cells are `< cells.variants.len()`. Then build a `Layout` with,
  per cell, `variant = (alternatives.len() − 1)`, `orient` cycling over all 8 `Orient`s, and distinct centres. For
  every cell c, the sorted nwell rects and the sorted diff rects of
  `pnr_core::place_macros(&cellgen::realize(&cells.variants, &l.variant), &l)[c]` must equal the sorted
  `place_rect(geo.bbox[c][v], r, &l, c)` over `geo.wells[c][v]` / `geo.diffs[c][v]`. This is the only geometry path
  that changed.
- Same module, `environment_is_in_the_placement_arm_once`: `crate::run` on `pair.spice`, sky130, with `feedback_iters:
  1, outer_iters: 1, starts: 1`. Then `sol.metadata.placement` has exactly one row with `kind == "Environment"`, and
  `sol.metadata.routing` has exactly one (the report with rings).
- Commands: `cargo test -p pnr_core place_rect`, `cargo test -p analog environment`, and `cargo test -p library
  environment_tests`. Then run `cargo test -p library -p dp -p gp` in
  full, because a new budget batch shifts dp/gp scores.

### Acceptance (plan, unchanged) and how to measure

- T9 = 0 on every fixture × seeds 1–5 (nearness and skew), read from the final report's **routing** `Environment` row
  (rings included). The lex key must be no worse on ≥ 9/10 fixtures (median over seeds). dp wall time may grow by
  at most +30 % (policy); past that, add the step-5 cache. Measure with the release bench, against the base `a2fece9`
  as baseline. If T9 ≠ 0 on some fixture, report it with the row's numbers. Do not loosen `ENV_TOL`.
- Risk: gp's `analog_cost` also evaluates the batch on every gp step. If gp time grows past the same 30 %, report it
  (do not gate the batch off in gp).
