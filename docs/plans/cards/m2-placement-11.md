# M2+ placement, segment 11 (M4): PLC-16, PLC-15

Branch `m2-placement` after merging `m2` at 11b5e83. Line numbers are at that commit.
Both items: class **do**.

## PLC-16 Performance-driven placement term (MST parasitic estimate)

DAG: no hard deps. Step-level dep PERF-12 (R and coupling rows) is not needed: use today's rows, ground C only.

### Code facts (plan corrections)
- The routing rows are `PerfPlan::rows` (`frontend/library/src/lib.rs:645`), built by `perf::budget_rows(p, .., af_per_um / 1000.0)` (`lib.rs:592`) and pushed into `problem.routing.budget` in the topology builder (`lib.rs:737-740`). The plan's `lib.rs:167/204/218/267-269/275/293-296` are stale. `CellSpace::new` is at `lib.rs:747`. Placement budgets pushed after it are already cell-indexed and skip `retarget` (`lib.rs:749`, as with `Utilization` at `lib.rs:787` and the islands at `lib.rs:790-793`).
- `PerformanceBudget` (`kernel/analog/src/routing/performance.rs:27-49`) has `nets`, `weights` (not `w`), `af_per_nm`, and `limit` (1.0 for a bound with headroom, 0.0 for do-not-worsen). It also has R, diff and coupling terms. RTE-21 owns those, so the placement estimate ignores them: ground C only.
- `Orient::apply(x, y)` and `Orient::swaps_axes()` are at `kernel/core/src/geom.rs:75,81`. The `Layout` columns used are `x, y` (cell centres), `variant` and `orient`.
- `RuleBatch<Layout>` lives at `kernel/analog/src/rule.rs:187`. `local()` (`:214`) plus `touched()` (`:261`) lets dp's incremental evaluation (`backend/dp/src/eval.rs:41`) re-score the row only when one of its cells moves. The template to follow is `Utilization` (`kernel/analog/src/placement/utilization.rs`).
- Correction to the plan: `Nets::from_macros` (`backend/gp/src/mechanics.rs:74-112`) averages pin centres into one point. PlacePerf instead uses the union box of the cell's pins on the net, which is what the edge-to-edge distance needs.

### Edits
1. New `kernel/analog/src/placement/perf.rs`, plus `pub mod perf; pub use perf::PlacePerf;` in `placement/mod.rs`:
   ```rust
   #[derive(Clone)]
   pub struct PlacePerf {
       pub metric: String,
       pub nets: Vec<NetId>,
       pub weights: Vec<f32>,
       pub af_per_nm: f32,
       /// The routing row's `limit` (1 = headroom, 0 = do-not-worsen).
       pub limit: f32,
       pub reserve: f32,
       /// Per net: `(cell, per-variant pin box (dx, dy, hx, hy) about the bbox centre, R0)`.
       pub items: Vec<Vec<(u16, Vec<(i32, i32, i32, i32)>)>>,
   }
   impl PlacePerf {
       pub fn new(row: &PerformanceBudget, variants: &[&[Macro]], reserve: f32) -> Self;
       fn item(&self, l: &Layout, c: u16, boxes: &[(i32,i32,i32,i32)]) -> (i64, i64, i64, i64); // placed centre, half extents
       pub fn mst_len(&self, n: usize, l: &Layout) -> i64;   // Prim, O(k²), k ≤ ~10
       fn used(&self, l: &Layout) -> f32;                    // Σ w_i·mst_i·af_per_nm
   }
   ```
   - `new`: takes `variants[c]` as the cell's alternatives (`&cells.variants[c].alternatives`). For each row net, it collects every cell with a pin on that net in any alternative. For each alternative it records the union of that net's pin rects, relative to the bbox centre `(bbox.x + bbox.w/2, bbox.y + bbox.h/2)`. An alternative with no pin on the net gets `(0,0,0,0)`. Nets with fewer than 2 cells are dropped, along with their weight.
   - `item`: `v = l.variant[c]`, with an out-of-range `v` treated as 0. Then `(dx, dy) = l.orient[c].apply(bx, by)`, and `(hx, hy)` are swapped when `swaps_axes()`. The position is `(l.x[c] + dx, l.y[c] + dy)`.
   - `d(a, b) = (|Δx| − (hx_a + hx_b)).max(0) + (|Δy| − (hy_a + hy_b)).max(0)`.
   - `residual = (used / (1 − reserve) − limit).max(0)`. With `limit = 1` this is the plan's formula. With `limit = 0` any adverse C counts, which matches the routing row's do-not-worsen meaning.
   - `RuleBatch<Layout>`: `cost` = `residual` as f32; `violations` = `u32::from(residual > 0)`; `kind` = `"PlacePerf"`; `count` = 1; `local` = true; `touched` = every item cell, deduplicated; `criticality` = `(used/(1−r)).clamp(0,1)`; `worst_usage` = `Some(used/(1−r))`.
   - `retarget` keeps the default no-op, because the rows are built cell-indexed.
   - `pub const RESERVE: f32 = 0.2;` is a **policy** value. Its doc comment cites Lampaert §4.4 / LAMP-34.
2. `frontend/library/src/lib.rs`:
   - Add a `Config::place_perf: bool` field with a doc comment ("the T5 A/B switch"), and set it to `true` in `Default` (`lib.rs:208-225`).
   - In the topology builder, after the PLC-12 islands (`lib.rs:793`), add:
     `if cfg.place_perf { for row in perf_rows { let alts: Vec<&[Macro]> = cells.variants.iter().map(|v| &v.alternatives[..]).collect(); problem.placement.budget.push(Box::new(PlacePerf::new(row, &alts, RESERVE))); } }`
     Build `alts` once, outside the loop. With no `performance` config, `perf_rows` is empty and nothing changes.

### Tests
- In `kernel/analog/src/placement/perf.rs` `#[cfg(test)]`, with a 2–3 cell `Layout` like `utilization.rs`'s `two_cells`:
  - `mst_equals_hpwl_for_two_points`: zero-size pin boxes at `(0,0)` and `(3_000, 4_000)` give `mst_len(0, l) == 7_000`.
  - `abutting_pins_have_zero_mst`: two 1 000 nm boxes whose edges touch give `== 0`.
  - `rotated_pin_box_follows_orient`: pin box `(400, 0, 100, 50)`. At `R90` the item centre is `orient.apply(400, 0)` and the half extents are `(50, 100)`. The MST to a point pin at `(0, 1_000)` equals the hand-computed value, and differs from the R0 value.
  - `residual_charges_only_past_the_reserve`: `used = 0.7` gives residual 0; `used = 1.0` gives `0.25` (±1e-6); with `limit = 0`, `used = 0.4` gives `0.5`.
- `backend/dp/src/tests.rs` `a_row_shortens_its_sensitive_net`: 4 cells, each with one pin on each of two nets (use `pin_alt`-style macros, plus a second pin). Net 0 joins cells 0 and 1 and net 1 joins cells 2 and 3. The coarse layout is the same for both runs. Use flat `place` (the existing `run` helper) over seeds 1..=10, once with `Requirements::default()` and once with `reqs.budget = [PlacePerf { nets: [n0], weights: [1.0], af_per_nm: 1e-3, limit: 1, reserve: 0.2, .. }]`. The weight is chosen so the residual stays positive at the no-row mean MST. Assert `mean_mst_with < mean_mst_without`, strictly. If λ0 cannot move it, settle prices once between runs (as `place_does_not_settle` documents), and say so in the test doc.
- Command: `qcargo test -p analog placement::perf` and `qcargo test -p dp a_row_shortens`, then `qcargo test -p library`.

### Acceptance (T5)
On fixtures with specs, run `--release` A/B on `Config::place_perf`. Spec miss with rows must be ≤ without on every fixture and strictly lower on at least one, with area ≤ +5 %. Use the existing perf benchmark fixtures that carry `performance`. If none has specs that the term moves, report the measured numbers and do not claim T5.

## PLC-15 Routing-space reservation: current halos and congestion feedback

DAG hard deps: PLC-08 (done; `sp::Geo::halo`) and RTE-25 (merged: `dr::RouteStats::congestion` at `backend/dr/src/lib.rs:274`, `congestion()` at `:1698`, `LatticeSpec`/`lattice_spec` at `:1678-1690`).

### Code facts (plan corrections)
- Ring halos are now at `frontend/library/src/lib.rs:2394-2398`, not `:1195-1208`. `pin_currents` is at `lib.rs:2203`, and `r.cfg.pin_ua` is filled at `lib.rs:876`. Its tables key both `d{k}:T` and the bare `T` (member 0), and a pin rect matches by exact `pin.name`, as dr does at `backend/dr/src/lib.rs:408-420`. The EM limits are `elaborate::em_limits` at `frontend/library/src/elaborate.rs:266`, called at `lib.rs:824` and moved into `r.cfg.em`. Per-pin current share is `pnr_core::pin_shares(&Macro)` (`kernel/core/src/macro.rs:170`). `Pdk::min_width(layer: u16)` is at `backend/verify/src/pdk.rs:956`.
- The halo plumbing exists but is unused. `sp::Geo::halo: &[[i32;4]]` (`backend/dp/src/sp.rs:274`) takes per cell, per placed face, in `Face` order L, B, R, T (`backend/gp/src/spacing.rs:50`), and enters `Kids::gap` / `need` / `sep_ok` (`sp.rs:375-412`). A nonzero halo already forbids abutment, so PLC-08 step 2 is done. `PlaceInput::halo` (`backend/dp/src/anneal.rs:32`) feeds `Geo` unchanged (`anneal.rs:190`), and the library passes `&[]` (`lib.rs:1569`).
- **Gap 1 in the plan:** `PlaceInput::halo` is fixed per cell, but the anneal changes `orient`/`variant` (`St::set_geom`, `anneal.rs:162-169`). A face-indexed static halo therefore has to be re-permuted there, exactly like the profile (`Profiles::orients` / `oriented`, `spacing.rs:326-348`).
- **Gap 2 in the plan:** `Flow` is shared by the parallel starts (`std::thread::scope`, `lib.rs:465`; `search(&Topology, ..)`, `lib.rs:947`). A `RefCell` in `Flow` would break that. The dynamic halo must be a `search()`-local `Vec<[i32;4]>` passed to `Flow::epoch`.
- **Gap 3, scope note:** the halo applies only under `DpMode::Sp`. The default `DpMode::Flat` (`anneal.rs:51-57`) never builds a `Geo`, so T6 is measured with `dp_mode: Sp`. Wiring flat dp is out of scope (plan step 3 says "every gap in PLC-08").

### Edits
1. `backend/gp/src/spacing.rs`: `pub fn oriented_faces(h: [i32; 4], o: Orient) -> [i32; 4]`. It uses the same normal map as `oriented`, which is refactored to share one `fn face_to(f: Face, o: Orient) -> usize`.
2. `backend/dp/src/anneal.rs`:
   - `PlaceInput::halo: &'a [Vec<[i32; 4]>]`: static halo per cell and per variant, in the R0 frame (empty or short = 0).
   - New `PlaceInput::halo_dyn: &'a [[i32; 4]]`: congestion halo per cell in the placed (world) frame (empty = 0).
   - `St` gains `halo: Vec<[i32; 4]>`, sized n in the constructor (`anneal.rs:464` neighbourhood). In `set_geom`: `self.halo[c] = add(oriented_faces(inp.halo.get(c).and_then(|h| h.get(v)).copied().unwrap_or_default(), o), inp.halo_dyn.get(c).copied().unwrap_or_default())`.
   - `realize` passes `halo: &self.halo`.
   - Update the other `PlaceInput` literals (`anneal.rs:649`, tests) with `halo: &[], halo_dyn: &[]`.
3. `frontend/library/src/lib.rs`:
   - `fn static_halos(variants: &[gp::VariantSpace], pin_ua: &[Vec<(String, Option<i32>)>], j_ua_per_nm: f32, w_min: i32, lattice: i32) -> Vec<Vec<[i32; 4]>>`:
     - For each cell c, alternative v and pin p: `I_p = table(pin.name)? × pin_shares(m)[p]`.
     - `f` = the face whose distance from the pin rect's centre to the bbox edge is least (ties go L, B, R, T).
     - If `I_p > j·w_min`, then `h[f] += round_up(ceil(I_p / j) − w_min, lattice)`.
     - Empty `pin_ua` gives all zeros.
   - `fn halo_target(d: f32, n: i32, p0: i32, lattice: i32) -> i32` and `fn halo_step(h: i32, target: i32, p0: i32, lattice: i32) -> i32` return `round_up(0.5·h + 0.5·target, lattice).min(4·p0)`. These two pure functions carry the policy (β = 0.5, cap 4·p0), with a doc comment citing BAL2-19 L9401–9405.
   - `fn update_dyn_halo(halo: &mut [[i32;4]], l: &Layout, congestion: &[(Rect, f32)], n_pins: &[[i32;4]], p0: i32, lattice: i32)`:
     - Each face's band is `p0` wide just outside the face. Left: `Rect { x: x−hw−p0, y: y−hh, w: p0, h: 2hh }`, and similarly for the others.
     - `d_f` = the max demand over the regions intersecting the band (0 if none).
     - `n_f` is in the world frame: `oriented_faces(n_pins_r0[c][v], o)`, then `.max(1)`.
   - Topology builder: after `Flow` is built, `flow.halo_st = static_halos(&flow.cells.variants, &flow.d_router.cfg.pin_ua, j, w_min, flow.rules.grid)` and `flow.halo_pins = …` (pin count per face, per cell and variant, R0).
     - `j` = `ua_per_um / 1000` of the first `layers` entry that has `ua_per_um > 0` in `flow.d_router.cfg.em`. `w_min` = `pdk.min_width(l.0)`.
     - With no such layer, `halo_st` is empty.
     - New `Flow` fields: `halo_st: Vec<Vec<[i32;4]>>`, `halo_pins: Vec<Vec<[i32;4]>>`.
   - `Flow::epoch(.., halo_dyn: &mut Vec<[i32;4]>)`:
     - The SP branch passes `halo: &self.halo_st, halo_dyn`.
     - After routing (`lib.rs:1654-1673`), call `update_dyn_halo(halo_dyn, &layout, &route_stats.congestion, …, dr::lattice_spec(&self.d_router.cfg).p0, self.rules.grid)`.
   - `search()`:
     - `let mut halo_dyn = vec![[0; 4]; flow.cells.variants.len()];` is passed to every epoch.
     - Reset it to zeros at `assignment = next` (`lib.rs:1033`).

### Tests
- In `frontend/library/src/lib.rs` `mod tests`, on the pure functions:
  - `static_halo_reserves_only_excess_width`: one cell, one alternative, one pin `"D"` near the right face, `pin_ua` = 5 000 µA, `j = 2.8`, `w_min = 140`, `lattice = 10`. Expect `== vec![vec![[0, 0, 1_650, 0]]]`. A second pin at 300 µA (≤ I_1 = 392) adds nothing.
  - `dynamic_halo_decays_without_overflow`: `halo_step(800, halo_target(1.0, 2, 420, 10), 420, 10) == 400`, then `== 200` on the next call.
  - `dynamic_halo_is_capped`: `halo_step(0, halo_target(10.0, 4, 420, 10), 420, 10) == 1_680`.
  - `band_reads_only_the_adjacent_region`: two congestion regions, demand 3.0 left of the cell and 0.5 elsewhere. `update_dyn_halo` raises only face L.
- In `backend/dp/tests/sp_props.rs`, extend the existing halo case (`:235`) to cover rotation: with `halo[c][0] = [500, 0, 0, 0]` at R0, a cell at R90 has the halo on the face `oriented_faces` names, and the decoded gap there is ≥ min + 500.
- Commands: `qcargo test -p gp spacing`, `qcargo test -p dp`, `qcargo test -p library halo`.

### Acceptance
- T6: `route_hard + route_overuse` ≤ baseline on every fixture under `DpMode::Sp`.
- T2: footprint ≤ +5 % vs PLC-08 without halos.
- Run `--release` A/B by passing an empty `halo_st` / `halo_dyn`. A `Config` switch is not needed: the empty slices give the baseline in a test helper.
- If the overuse goes up anywhere, report the numbers and do not tune β or the cap to pass.
