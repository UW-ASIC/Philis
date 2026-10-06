# M2+ placement, segment 10 (M3): PLC-08, PLC-09, PLC-10, PLC-11, PLC-12, PLC-28

HEAD after `git merge m2` (cells 11 merge `2676ece` in, no conflicts). Hard deps on `m2`: PLC-07 (`3c12878`), PLC-18
(`967798a`, `f8f9da0`), RTE-25 (`cfefafe`/`6cbd5af`, routing 1 merge `cff8266`), PLC-21 (`ebf3210`). PLC-06 is still open
(owner decision); none of these items reads its key tier. FLOW-08 has **not** landed (no warm/incumbent path in
`frontend/library/src/lib.rs`), so PLC-10 step 1 lands the dp side only (dag: "whichever lands second wires").

**Toolchain (owner request):** every build/test below runs inside the flake dev shell, through the queue:
`cd /home/omare/Documents/Projects/Rust/philis-m2/placement && export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk && nix develop --command /home/omare/Documents/Projects/Rust/Philis/tools/qcargo <args>`
(written `Q <args>` below; run it with `run_in_background`; `flake.nix` is the only nix file, so `nix develop`, not `nix-shell`).

Shared code facts (cite by symbol; lines are at this HEAD):
- `pnr_core::Layout` (`kernel/core/src/layout.rs:11-40`): `x, y` are cell **centres**, `hw, hh` half extents (so
  `w = 2·hw` is always even), `axis: Vec<i32>` = axis x by `AxisId`, `footprint_nm2` `:46`, `l_ref` `:65` (=
  `sqrt(Σ 4·hw·hh)`, so `A_cells = l_ref²`). `UnionFind` `kernel/core/src/unionfind.rs` (`new`, `find`, `union`).
- dp today (`backend/dp/src/lib.rs`, 686 lines): flat SA `place` `:231-410` (signature `(coarse, macros, variants, reqs,
  fixed, locks, prices, rules, net_weight, seed, schedule) -> (Layout, Report, PlaceStats)`), `Schedule` `:34-44`
  **only `{range0, max_temps, t0_scale}`** (stale: plan's `p0/alpha/moves_per_kid` were not added in M1), `PlaceStats`
  `:51-64` (`temps, proposals, accepted, decode_fail: Option<u64>, matched_incompatible`), `Snap` `:68-118`,
  `project_hard` `:432`, `sym_groups` `:497` (mirror pairs from `reqs.hard` only, grouped by axis), `quarter_turn`
  `:596`, `try_rotate` `:615` (turns `locks.members(c,false)` with `locks.rel`, PLC-21), `try_reshape` `:642`,
  `try_branch` `:591`. No `sp.rs`, `anneal.rs`, `eval.rs`; `backend/dp/tests/` does not exist.
- Spacing (PLC-07, `backend/gp/src/spacing.rs`): `Face {L,B,R,T}` `:50` (`opposite`, private), `Profile` `:92`
  (`edge: [Edge;4]`, `well_net`, `matched`, `set`), `Gap {abut, min}` `:114`, `SpacingTable::gap(a, fa, b)` `:224` =
  what `a`'s face `fa` owes `b`'s opposite face, `fallback`, `lattice`. `PlaceRules` (`backend/gp/src/lib.rs:239-330`):
  `grid` (= cut lattice, 10 nm on sky130: `place_rules` passes `cells::builder::cut_lattice` = `2·grid`,
  `frontend/library/src/lib.rs:1316`), `spacing`, `profiles.of[cell][variant][orient]`; `fn profile` `:263` is private;
  `gaps` `:272` (cell without profile ⇒ `fallback`).
- Rule hooks (`kernel/analog/src/rule.rs`): `mirror_pairs` `:268` (`(a, b, axis)`, `a == b` = self-symmetric),
  `matched_pairs` `:272`, `mirrored_pairs` `:280` (PLC-21 Mirror), `touched` `:254`.
- Prices (`backend/gp/src/lib.rs:30-162`): per-batch `Price { lambda, rho, residual }` `:53` (private), `bind` `:88`
  caches `weight` (= −λ), `weight_of` `:159` is `pub(crate)`. PEX = `mechanics::pex` `:261` = `HPWL/L_ref +
  analog_cost` (`:248`, Σ crit·cost + Σ weight·residual).
- Library: `Config::gp_mode: GpMode {Analytic, Pile}` `lib.rs:116,193`; `Flow.gp_mode` `:1196`; `Epoch` `:1404`;
  `Flow::epoch` calls `gp::place` then `dp::place(..., dp::Schedule::cold())` at `:1473-1486`; `cells.groups`
  (`CellSpace.groups` `:2193`, built `:2244` = `problem.groups` remapped to cells, `DeviceId` holding cell ids; last
  group = annotator glue, `backend/annotator/src/lib.rs:51,171`); `topology` `:711` (plan's "`solve()`" is stale)
  pushes `Utilization` at `:775-781`; `placement_metrics` `frontend/library/src/geometry.rs:41-90`
  (`islands_extra: None` at `:85`, test `:409` asserts `None`); bench prints `islands_extra` and `decode_fail`
  (`benchmarks/src/bench.rs:314,318`). The report uses **11 fixtures** (`benchmarks/fixtures/*.spice`), not 10.

## PLC-08 Hierarchical S-F sequence pair with an exact decoder (class: do)

Corrections to the plan: `Geo.prof` must allow "no profile" (synthetic tests, injected cells); `Face::opposite` is
private; `TreeError` needs `Debug`. Halo = all zeros until PLC-15 (dag step-level). `blocks` = `cells.groups`
(EXT-13's `Intent.tree` is not on `m2`). Mirror (PLC-21) partners have equal `w, h` (Mx180 keeps `swaps_axes`), so the
decoder needs no mode; their oriented profiles differ and are read per cell.

Edits:
1. `backend/gp/src/spacing.rs`: make `Face::opposite` `pub`. `backend/gp/src/lib.rs`: add
   `pub fn profile_of(&self, c: usize, v: u16, o: Orient) -> Option<&spacing::Profile>` (body of the private
   `profile`, which then calls it).
2. New `backend/dp/src/sp.rs`, `pub mod sp;` in `lib.rs`. Types exactly as plan-04 PLC-08 with these signatures:
   ```rust
   #[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Kid { Cell(u16), Node(u16) }
   #[derive(Clone, Debug, PartialEq, Eq)] pub struct Sym { pub axis: AxisId, pub mate: Vec<u16> }
   #[derive(Clone, Debug, PartialEq, Eq)] pub struct Node { pub kids: Vec<Kid>, pub alpha: Vec<u16>, pub beta: Vec<u16>, pub sym: Option<Sym> }
   #[derive(Clone, Debug, PartialEq, Eq)] pub struct Tree { pub nodes: Vec<Node>, pub root: u16, pub home: Vec<(u16, u16)> }
   #[derive(Clone, Debug, PartialEq, Eq)] pub enum TreeError { CellInTwoAxes { cell: u16, a: AxisId, b: AxisId } }
   impl Tree {
       pub fn build(n_cells: usize, pairs: &[(u32, u32, u16)], blocks: &[Vec<DeviceId>]) -> (Tree, Vec<TreeError>);
       pub fn same_structure(&self, o: &Tree) -> bool; // node count, kids, sym mates equal (PLC-10 warm fallback)
       pub fn is_sf(&self, node: u16) -> bool;
       pub fn make_sf(&mut self, node: u16);
       pub fn seed_from(&mut self, l: &Layout);
       pub fn seed_constructive(&mut self, w: &[i32], h: &[i32]); // needs areas for "descending area"
   }
   pub struct Geo<'a> { pub w: &'a [i32], pub h: &'a [i32], pub prof: &'a [Option<&'a Profile>],
                        pub halo: &'a [[i32; 4]], pub table: &'a SpacingTable, pub lattice: i32,
                        pub axis_grid: Option<(i32, i32)> } // PLC-28; None until then
   #[derive(Default, Debug)] pub struct Out { pub x0: Vec<i32>, pub y0: Vec<i32>, pub axis: Vec<(AxisId, i32)>, pub fixes: u32 }
   #[derive(Debug, PartialEq, Eq)] pub enum Fail { SymY(u16), SymX(u16), Band(u16) }
   #[derive(Default)] pub struct Scratch { pa: Vec<u32>, pb: Vec<u32>, rel_x: Vec<Vec<i32>>, rel_y: Vec<Vec<i32>>, lb: Vec<i32>, node_prof: Vec<Profile> }
   pub fn decode(t: &Tree, g: &Geo, s: &mut Scratch, out: &mut Out) -> Result<(), Fail>;
   pub fn verify(t: &Tree, g: &Geo, out: &Out) -> bool; // pub so the property test can call it
   pub fn to_layout(out: &Out, w: &[i32], h: &[i32], l: &mut Layout); // assembly step 8: x = x0 + w/2, hw = w/2, axis
   ```
   Gap lookup: `fn gap(g, pi: Option<&Profile>, f: Face, pj: Option<&Profile>) -> Gap` = `table.gap` when both are
   `Some`, else `Gap { abut: false, min: table.fallback }` (same as `PlaceRules::gaps`). Build steps 1–3, S-F condition,
   `make_sf`, seeds, decode steps 1–6 and 8, `fix_monotone` (≤ `2p + 2` passes), Step-1y pass cap `p + 2`: as the plan.
   Node profile: per face and role, `min` over members of `(member inset + distance member face → node face)`;
   `present` = OR of member bits; `matched` = the max member class (`Option` max); `set` = common member set else
   `None`; `well_net` = common member well net else `None` (no merge across a mixed node). `Out.fixes` counts
   `fix_monotone` runs (printed by the property test).
3. A `TreeError` is not dropped silently: `build` drops the second group and returns the error; PLC-09's `place_sp`
   turns each into a hard `Report` row `"conflicting symmetry cell {cell}"` (margin 1).

Tests (`backend/dp/src/sp.rs` `#[cfg(test)] mod tests`, and `backend/dp/tests/sp_props.rs` for the three property
tests), all with `SpacingTable::uniform(0 or g, 10)` and `prof = None` unless stated:
- `balasa_example_1_decodes_symmetric`: data exactly as plan; asserts `is_sf(sym node)`, `decode == Ok(())`,
  `C_F + C_G == C_K + C_L == C_C + C_J == 2·ax2` (doubled centres), pairwise boxes disjoint, `y0[C] == y0[J] == 4000`.
  (Corrected in implementation: `== 2000`. Balasa's flat 4000 needs A below C; in the hierarchy A is a root sibling
  and the contiguous symmetry node holds only K below C and L below J, 2000 each.)
- `make_sf_satisfies_condition_1_1`: SplitMix64 seed 7, 1,000 random α over 2 pairs + 1 self → `is_sf` after `make_sf`.
- `random_sf_codes_decode_exactly` (sp_props): generator as plan (seed 7, 10,000 trees); `assert_eq!(fails, 0)`,
  `assert!(verify(..))` per tree; `eprintln!` the `fixes` count.
- `contiguous_node_has_exclusive_bbox`, `pairwise_gaps_hold_with_shadows`, `every_coordinate_is_on_the_lattice`
  (sp_props; `x0 % 10 == 0`, `y0 % 10 == 0`, `2·axis % 10 == 0`) as plan.
- `merge_band_is_closed`: two `Profile`s with `NWELL` at inset 0 on facing faces, same `well_net`, table rule 1270 ⇒
  `gap(..) == Gap{abut:true, min:1270}`; force longest-path gap 500 (a third cell between in β of width 500) ⇒ decoded
  gap `== 1270`; with the gap at 0 ⇒ stays `0`.
- `conflicting_axes_are_reported`: cell 0 in pairs on axes 0 and 1 ⇒ one `TreeError::CellInTwoAxes{cell:0,..}`.
Command: `Q test -p dp sp` then `Q test -p dp --release --test sp_props` (10,000 trees: release). Also
`Q check -p gp -p dp` after edit 1.
Acceptance: all green, 100 % Ok. Fixture acceptance is PLC-09's.

## PLC-09 Annealing over codes (class: do)

Corrections: plan's `dp::place` name is taken by the flat path (callers: `Flow::epoch`, 4 dp tests); the SP path is a
**new** `pub fn place_sp` (flat `place` unchanged until PLC-27 deletes it). `Schedule` must first gain the SP fields
(step 0 shipped without them). `Prices.rho` is private per `Price`; cache it in `bind`. `area_ref` (PLC-01) does not
exist: `A_cells = l.l_ref()²`.

Edits:
1. `dp/lib.rs` `Schedule`: add `pub p0: f64, pub alpha: f64, pub moves_per_kid: u32`; `cold()` adds `0.6, 0.9, 20`,
   `warm()` adds `0.1, 0.9, 20` (flat path ignores them; `cold_schedule_reproduces_todays_layout` must stay green).
2. `gp/lib.rs` `Prices`: field `rho: Vec<f32>` filled in `bind` beside `weight` (0 for an unpriced batch);
   `pub(crate) fn rho_of(&self, bi) -> f32`. `gp/mechanics.rs`: `pub fn augmented_cost(reqs, l, prices) -> f64` =
   `Σ crit·cost over reqs.cost + Σ_b (weight_of(b)·r_b + ½·rho_of(b)·r_b²)`, `r_b = budget[b].residual(l).max(0)`.
3. New `backend/dp/src/anneal.rs` (`pub mod anneal; pub use anneal::{place_sp, PlaceInput, Start, DpMode};`):
   ```rust
   pub struct PlaceInput<'a> { /* fields exactly as plan-04 PLC-08 "dp::place API" */ }
   pub enum Start<'a> { Cold(&'a Layout), Constructive, Warm { tree: &'a sp::Tree, variant: &'a [u16], orient: &'a [Orient] } }
   #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)] pub enum DpMode { #[default] Flat, Sp }
   pub fn place_sp(inp: &PlaceInput, start: Start, schedule: Schedule, prices: &mut gp::Prices, seed: u64)
       -> (Layout, sp::Tree, Report, PlaceStats);
   fn energy(nets: &Nets, reqs, l: &Layout, prices) -> f64; // HPWL/L_ref + (footprint/l_ref² − 1) + augmented_cost
   ```
   Moves M1–M6 and probabilities, gate (`analog_phi` lexicographic, then Metropolis), final 3-temperature quench with
   `(phi.0, phi.1, analog_theta)`, schedule rules (a)–(c) and the `T0 = 1e-12` guard exactly as plan. M4 reuses
   `quarter_turn`/`locks.rel` like `try_rotate` (so Mirror pairs stay reflections, PLC-21); M5 reuses `locks.members(c,
   true)` and `Nets::reshape_cell` like `try_reshape`; fixed cells never rotate/reshape. A decode `Err` rejects the move
   and counts `stats.decode_fail` (`Some(n)` on the SP path). No legalizer on the SP path. `prices.bind(reqs)` once,
   read-only (no `settle`).
4. `frontend/library/src/lib.rs`: `Config::dp_mode: dp::DpMode` (default `Flat`), `Flow.dp_mode`; `GpMode::Constructive`
   (skips `gp::place`; with `DpMode::Flat` it runs gp with `iterate: false`, documented on the variant). `Flow::epoch`
   matches `dp_mode`: `Sp` builds `PlaceInput` (`blocks: &cells.groups`, `halo: &[]` ⇒ zeros, `n_axes:
   problem.axis_count`) and calls `place_sp` with `Start::Cold(&coarse)` or `Start::Constructive`.

Tests (`backend/dp/src/anneal.rs`): `sf_moves_preserve_sf`, `anneal_is_deterministic` (asserts equal `Tree`, `l.variant`,
`l.orient`, `l.x`, `l.y`), `quench_never_returns_worse_theta_than_start` (budget `Separation` from `tests.rs:354`; assert
`analog_theta(end) <= analog_theta(start)`), `schedule_stops_early_on_a_flat_landscape` (`stats.temps == 6`),
`area_term_is_bbox_over_cell_area` (`(energy − 0.21).abs() < 1e-6` and `(energy − 0.15).abs() < 1e-6`), all as plan.
`frontend/library/tests/placement_metrics.rs`: `sp_mode_places_ota_legally` (`Config{dp_mode: Sp, ..small(Analytic)}` ⇒
`overlap_nm2 == 0.0`, `clearance_residue_nm2 == 0.0`, `lattice_off == 0`, `dp.decode_fail.is_some()`).
Command: `Q test -p dp anneal`, `Q test -p gp`, `Q test -p library --release --test placement_metrics`.
Acceptance (release, method of `m2-placement-report.md`: `library::run` probe per fixture × seeds 1–5, 11 fixtures,
`Flat` vs `Sp` on the same tree): T1 all zero under `Sp`; signoff DRC ≤ Flat; lex key ≥ Flat on ≥ 9/11 (plan's 8/10,
scaled; median over seeds); T2 ≤ Flat; schedule A/B (adaptive vs fixed equal-proposal) lex ≥ on ≥ 9/11 at ≤ 50 % dp
wall. Record in `m2-placement-report.md` `## PLC-09 acceptance`. Flip `DpMode` default to `Sp` only if it passes;
otherwise report the misses and keep `Flat`.

## PLC-10 Warm start, incremental evaluation, undo log (class: do; flow wiring waits on FLOW-08)

Facts: step 0 done (`Schedule`, `cold_schedule_reproduces_todays_layout` at `dp/src/tests.rs:603`). **Step 2 is stale —
already done:** `touches` exist for `Proximity` (`proximity.rs:46`), `Isolation`/`SubstrateBalance` (`isolation.rs:31,
74`), `DtiBand` (`dti.rs:77`), `HeatSeparation` (`heat.rs:34`), `Symmetry` (`symmetry.rs:56`, group forwards `:178`);
`Utilization` and `LiveEnvironment` have none (global, correct: both read the whole layout). Tests exist only for
proximity/isolation (`touched_yields_device_ids_only`).

Edits:
1. `place_sp` handles `Start::Warm`: rebuild `Tree::build` on the inputs; if `!tree.same_structure(&built)` or
   `variant/orient` lengths ≠ n, fall back to `Cold`-equivalent `seed_constructive` and set
   `PlaceStats::warm_fallback: bool` (new field). `Epoch` gains `tree: Option<dp::sp::Tree>` (filled under
   `DpMode::Sp`). Mapping FLOW-08 `Warm` → `Start::Warm` is FLOW-08's (lands second).
2. Add `touched_names_both_targets` tests in `dti.rs` and `heat.rs` (device targets ⇒ both ids; group target ⇒ none).
3. New `backend/dp/src/eval.rs`: `pub(crate) struct Eval { net_bbox: Vec<(i32,i32,i32,i32)>, net_hpwl: Vec<f64>,
   batch: Vec<(f64 cost, f64 residual, u32 viol)>, cell_nets: Vec<Vec<u32>>, cell_batches: Vec<Vec<(Tier, u16)>>,
   global: Vec<(Tier, u16)>, total: f64 }` with `fn full(&mut self, ..) -> f64` and `fn update(&mut self, moved: &[u16],
   ..) -> f64`; `moved` = cells whose `(x0, y0, w, h, orient, variant)` changed. `l_ref` and the area term are global
   (recomputed each move, O(n)).
4. Undo log in `place_sp` (SP path only; flat `Snap` stays until PLC-27): `Vec<(u16, i32, i32, i32, i32, Orient, u16)>`
   plus the node's swapped indices; decode caches each node's relative result in `Scratch` with a dirty bit per node,
   recomputing the dirty node and its ancestors.
Tests: `incremental_matches_full_evaluation` (2,000 random moves, 14-cell synthetic with Proximity, Isolation,
Utilization: `|E_inc − E_full| <= 1e-6·|E_full|` after every accepted move); `warm_start_at_zero_temperature_never_worsens_the_incumbent_key`
(`Schedule{max_temps:1, ..warm()}`, T forced 1e-12 ⇒ `(phi, theta, E)` ≤ incumbent's); `warm_start_falls_back_on_changed_groups`
(tree from 2 pairs, inputs with 3 ⇒ `warm_fallback`); `cached_decode_equals_fresh_decode` (1,000 moves, `Out` equal).
Command: `Q test -p dp`, `Q test -p analog placement`.
Acceptance: T7 proposals/s (`proposals / dp wall`) ≥ 3× PLC-09's non-incremental `place_sp` on `dac4` and
`examples/three_stage_opamp`; bench wall ≤ PLC-09's; lex key equal on ≥ 9/11 (incremental is exact, so expect 11/11:
any difference is a bug). Report section `## PLC-10 acceptance`.

## PLC-11 gp ablation and seeding decision (class: do — measurement, then a conditional deletion)

Facts: `gp::place` `backend/gp/src/lib.rs:360-524`; `iterate == false` returns at `:380`; the gradient loop is
`:385-523`, `bin_overflow` `:527`, constants `:321-330` (`UTILIZATION` stays: `canvas_side` uses it). `gp_modes_are_deterministic` exists
(`placement_metrics.rs:37`); `Constructive` arrives with PLC-09 edit 4.
Edits: extend `gp_modes_are_deterministic` to `[Analytic, Pile, Constructive]` with `dp_mode: Sp`. Run the ablation
(release probe, 11 fixtures × seeds 1–5 × 3 modes, `DpMode::Sp`, equal `feedback_iters`): final lex key, T2, dp wall.
Decision rule as plan (≥ 9/11 not worse ⇒ delete the loop `:385-523`, `bin_overflow`, unused constants; keep
`Prices`, `net_weights`, `mechanics`, `initial_layout`); else keep gp and add `gp_spreads_a_pile` (16 equal 1000×1000
cells, `iterate: true` ⇒ `bin_overflow(..) <= 0.15`, a `price_tests`-module test since `bin_overflow` is private).
Record the table and decision in `docs/CRATES.md` `## backend/gp`. Command: `Q test -p gp`,
`Q test -p library --release --test placement_metrics`.

## PLC-12 Symmetry-island and cluster connectivity metric (class: do)

Facts: as plan; `PlacementMetrics::islands_extra: Option<u32>` exists (`geometry.rs:36`), no `clusters_extra`.
Edits: `kernel/analog/src/placement/island.rs` (+`pub mod island; pub use island::SymmetryIsland;`) with `SymmetryIsland
{ pub members: Vec<Target>, pub touch_nm: i32 }` implementing `RuleBatch<Layout>`: `cost = residual`, `violations =
u32::from(components > 1)`, `residual = components − 1`, `kind "SymmetryIsland"`, `count 1`, `touched` pushes device
members, `retarget` maps members. Adjacency integer test exactly as plan (strict overlap on one axis, edge distance ≤
`touch_nm` on the other); components by `UnionFind`. Library `topology` after the `Utilization` push (`lib.rs:781`):
per axis, cells from `placement.hard` `mirror_pairs` (`a != b` or self), ≥ 2 distinct cells ⇒ one batch with
`touch_nm = max over member pairs/faces of rules.spacing.gap(..).min + lattice`, pushed to `budget` and `cost`.
`geometry.rs`: `islands_extra = Some(Σ residual over budget batches with kind "SymmetryIsland")`; add
`clusters_extra: Option<u32>` = same count over `cells.groups` (glue excluded) with `touch_nm = max_gap + lattice`
(computed in `placement_metrics` with a new `groups: &[Vec<DeviceId>]` arg), bench prints it next to `islands_extra`.
Update the `:409` test to expect `Some(_)`.
Tests (`island.rs`): `abutting_pair_is_one_island` (residual 0), `a_gap_splits_the_island` (gap = touch+10 ⇒ 1),
`diagonal_corner_touch_is_not_adjacent` (corner touch ⇒ 1), `touched_names_every_member`. Command:
`Q test -p analog island`, `Q test -p library --release --test placement_metrics`.
Acceptance: T4 ≥ 90 % of `SymmetryIsland` batches at residual 0 in winners, 11 fixtures × seeds 1–5 (`DpMode::Sp` and
Flat both reported); section `## PLC-12 acceptance`.

## PLC-28 Symmetry axes on the routing lattice (class: do)

Corrections: RTE-25 gives `dr::lattice_spec(&DetailedCfg) -> LatticeSpec { p0, strides, origin_multiple }`
(`backend/dr/src/lib.rs:1678-1689`). The router's real contract (`pair_map` `:2115-2137`, `gr::TrackGrid::lattice_map`
`backend/gr/src/lib.rs:332`): mirror map `MirrorX{k}` needs frame-relative `2·axis − p0 ≡ 0 (mod p0)` and `k ≡ 0 (mod
s)` for every vertical (odd-index) layer's stride `s`. So with `S = lcm(strides[1], strides[3], …)`, the absolute
condition is `axis ≡ p0/2 (mod P)`, **`P = p0·lcm(2, S)/2`** (plan's `p0·max(1, s_max/2)` is wrong for odd `S`;
equal on sky130: strides `[1,1,2,2]` ⇒ `S = 2`, `P = 420`, `p0 = 420`). Frame origins are multiples of
`origin_multiple = p0·lcm(all strides)` (`frame_origin` `:1739`), which `P` divides, so absolute = frame-relative mod `P`.
`gp` cannot depend on `dr`: the library computes the pair.
Edits:
1. `gp::PlaceRules`: `pub axis_grid: Option<(i32, i32)>` (`p0`, `P`), default `None`. Library `topology`, where `d_router` is built (`lib.rs:856`,
   `elaborate::detailed_router`): `let s = dr::lattice_spec(&d_router.cfg)`; `S` = lcm of odd-index strides (1 if none);
   `P = s.p0 * lcm(2, S) / 2`; `Some((s.p0, P))` iff `s.p0 % (2·lattice) == 0`. dp passes it as `Geo.axis_grid`.
2. `sp.rs` Step 2x′: with `Some((p0, P))`, `ax2 = smallest ≥ bound with ax2 ≡ p0 (mod 2P)`
   (`bound + (p0 − bound).rem_euclid(2P)`); step 3: in every parent's Step 1x, a kid that is or contains a symmetry
   node gets `lb = round_up(lb, P)` (monotone); root origin 0.
3. `PlaceStats`: `axes: u32`, `axes_on_lattice: u32` (axes with `axis.rem_euclid(P) == p0/2`); bench prints both.
Tests (`backend/dp/tests/sp_props.rs`): `axes_land_on_track_centrelines` (PLC-08 generator, `axis_grid
Some((420, 420))`, lattice 10 ⇒ `axis.rem_euclid(420) == 210` for every axis, 100 % decode Ok);
`axis_snapping_costs_at_most_one_period_per_level` (node width with − without snapping `< 2·420` per symmetry node,
distribution `eprintln!`ed); unit test `axis_period_from_strides` (`[1,1,2,2]` ⇒ 420; `[1,3]` with p0 420 ⇒ 1260).
Command: `Q test -p dp --release --test sp_props`, `Q test -p library --release --test placement_metrics`.
Acceptance: `axes_on_lattice == axes` on all 11 fixtures with `DpMode::Sp`; T2 within +3 % of PLC-08/09 without
`axis_grid`; RTE-15 (merged `c063c04`) "pairs exact" n/n on `ota`, `ota_constrained`, `tt_ota` — measured, and any miss
reported with the `pair_map` reason. Section `## PLC-28 acceptance`.
