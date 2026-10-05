# M2+ routing segment 3: implementation card (RTE-16, RTE-18, RTE-15, RTE-17, RTE-19, RTE-20; M3)

Specs: plan-05 `### RTE-15`…`### RTE-20`; master §6.1/§6.2; gap-critic C16, D7, D11. DAG entries (`dag-m2-m6.json`,
main checkout only) and what has landed on `m2` (merged here at `be34be4`+`c77e47d`):

| item | hard deps | state of deps | step-level inputs not landed | class |
|---|---|---|---|---|
| RTE-16 | GAP-01, EXT-12 | both merged (`d4332d9`, `be34be4`): `Unitization.class: Option<MatchClass>` (cell.rs:66) | none (CELL-01 gate keep-outs absent: fallback below) | do (judgment) |
| RTE-18 | RTE-10 | done (seg 1) | EXT-24 class exclusions, EXT-18 aggressor weights: existing `CrosstalkExclusion`s and `CouplingBudget::default_weights` feed it | do (judgment) |
| RTE-15 | RTE-10, RTE-12 | done | PLC-28 (axes on the lattice), PLC-21 (Shift pairs): non-compliant pairs fall back; bench T5 measured once they merge | do (judgment) |
| RTE-17 | RTE-12, RTE-15 | RTE-15 in this segment | EXT-24 `CommonNodeReq/StarReq/KelvinReq`: leaves map to two groups; star/Kelvin have unit-test producers only | do (judgment) |
| RTE-19 | RTE-12, RTE-18 | RTE-18 in this segment | EXT-24 `shield_ref`: ground fallback (today's) | do (judgment) |
| RTE-20 | RTE-18 | RTE-18 in this segment | none: FLOW-07 `c_af` and EXT-19 capacitor sets are on `m2` | do (judgment) |

Line numbers are for `m2-routing` after the `m2` merge (`dr` = backend/dr/src/lib.rs, `gr` = backend/gr/src/lib.rs).
Commands: `export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk`; unit tests `cargo test -p analog -p gr
-p dr -p library`; fixtures `cargo test --release -p benchmark --test signoff_fixtures -- --include-ignored` (19
passed today, must stay). New bench checks go in one new file `benchmarks/tests/routing_quality.rs` (`#[ignore]`d
like the large fixtures, run with `--release -- --include-ignored`; built with `library::run(.., Config {
feedback_iters: 1, .. })` as `signoff_fixtures::check` does).

---

## RTE-16 Keep metal off matched gates and precision resistor bodies, by match class

### Current code (plan cites corrected)
- Soft keep-out: `gr::KEEPOUT_COST = 2.0` (gr:648); `RouteCtx::keepout/own_cells` (gr:637-638), passed as
  `NetSearch::keepout` (gr:767) and priced in `route_net` (gr:944). dr fills them over the **whole bbox** of every
  cell whose units have two owners (dr:677-697), on all layers. Resistors and caps get no routing protection.
- CELL-01 keep-outs exist for resistor bodies (`kernel/cells/src/resistor.rs:214`, `KeepWhy::ResistorBody`) and cap
  plates (`cap_array.rs:320`, `KeepWhy::CapPlate`); **no MOS generator emits `KeepWhy::Gate`** (fallback needed).
  `place_near` moves keep-outs (dr:1275); `gr::place_macros` is what the library uses.
- Hard obstacles: the foreign-metal loop (dr:483-503) writes `BLOCKED`/owner into `reserved` behind the `pre` mask
  (pin stitch nodes untouched). `jog_clean` (dr:1343) has three callers (dr:546, 1469, 1473).
- `score` (dr:2150) is where per-route V rows are pushed; it is called at dr:1092 on absolute routes.
- Problem-level routing rules are placement-independent (`problem.routing`, lib.rs:1714), so a rule holding
  absolute gate rects cannot live there: the check runs in dr per routed layout (correction to plan step 3's
  "library registers one per gate").
- `Blockage` must be a dr type: dr cannot see library types (correction: plan put it in elaborate.rs).

### Edits
1. **dr** (types near `DetailedCfg`):
   ```rust
   /// A region routed metal must respect (absolute nm; built per layout by the library).
   #[derive(Clone, Copy, Debug)]
   pub struct Blockage { pub rect: Rect, pub layers: u16 /* bit l = lattice layer l */, pub hard: bool,
                         pub own_exempt: bool, pub only_aggressors: bool, pub cell: u32, pub gate: bool }
   ```
   `DetailedCfg` gains `pub blockages: Vec<Blockage>` and `pub aggressor: Vec<bool>` (by `NetId`; `Clock` today).
   `gate` marks a rect checked by the metal-over-gate V.
2. **dr landing** (insert after the foreign-metal loop, dr:503, same `pre` mask and bin loop): for each hard
   blockage, nodes of layers in `layers` whose centre lies inside `shift(rect)` grown by `cfg.wire(l)/2`:
   - `!own_exempt && !only_aggressors` → `reserved = BLOCKED`.
   - otherwise → appended to `blocked_for[ci]` for every compact net `ci` it applies to (foreign nets of `cell`
     when `own_exempt`; aggressor nets only when `only_aggressors`). `RouteCtx` gains
     `pub blocked_for: Vec<Vec<u32>>` (sorted, deduped); `NetSearch` gains `pub blocked: &'a [u32]`; `route_net`
     treats `blocked.binary_search(&i).is_ok()` like a foreign `reserved` in `congestion` and in the neighbour test
     (gr:922-925, 1006). `RouteCtx::reroute` passes it.
   - `jog_clean` gains `blocks: &[(Rect, bool /*applies to ci*/)]`-style input: legs of `ci` may not intersect a
     hard blockage that applies to `ci` (dr computes the per-net list once). All three callers pass it.
3. **dr soft**: replace dr:677-697 by soft blockages: `keep[node] = cell` for nodes inside a soft blockage rect
   (not the bbox); `own_cells` stays "cells the net has a pin in" (own nets pay nothing when `own_exempt`).
4. **dr V**: after `score` at dr:1092, for every `gate` blockage, overlap area (nm²) of every routed shape on a
   lattice layer of any net with `rect`; if > 0 push hard `Violation { rule: format!("metal over gate cell {cell}"),
   margin: ceil(µm²·1000) as i64 }` (margin in 10⁻³ µm² so a sliver is not 0). This is plan step 3's measure;
   the rule type `analog::routing::MetalOverGate { rect, cell }` (new `metal_over_gate.rs`, `Rule` impl with
   `residual` = overlap µm², `satisfied` = 0) is what dr evaluates, so the bench and dr share one predicate.
5. **library** `elaborate.rs`:
   `pub(crate) fn blockages(placed: &[Macro], pdk: &Pdk, class_of: impl Fn(usize) -> Option<MatchClass>,
   n_layers: usize) -> Vec<dr::Blockage>`. Sources: `placed[c].keepouts`; for a cell with no `Gate` keep-out,
   gates = pairwise intersections of the cell's `poly` and `diff` role shapes (`Process::layer`). Every rect grown
   by the lowest routed metal's `route_spacing`. Policy exactly as plan step 1 (Gate Mod/Exc hard all layers
   `own_exempt=false, gate=true`; Gate Minimal soft; ResistorBody hard-for-foreign when class ≥ Moderate or sheet
   > 500 Ω/□ (`pex_f32(rpoly, "sheet_res_ohm_sq")`, absent = not > 500), plus an `only_aggressors` hard copy, else
   soft; CapPlate per plan, the aggressor copy grown by 3 000 nm). `class_of(c)`: the `Unitization` in
   `problem.constraints.unitization` with `devices.len() > 1` containing every device of `cells.devices_of[c]` →
   `Some(class.unwrap_or(Moderate))`, else `None` (no blockage, C16).
6. **library** lib.rs:1202 `DetailedCfg { blockages: elaborate::blockages(&placed, ..), aggressor: <Clock nets>, .. }`.

### Tests (`cargo test -p dr`, `cargo test -p analog metal_over_gate`)
- `no_route_crosses_a_moderately_matched_gate`: test_cfg; a macro with four gate blockages (hard, all layers,
  `gate`, `own_exempt=false`) in a 4 µm row across the straight path of a foreign two-pin net → net routed (no
  `open net` row), no `metal over gate` row, `MetalOverGate` residual 0.0 on every rect.
- `own_drain_does_not_cross_its_gates_at_mod`: the cell's own net with pins either side of one gate → same asserts.
- `a_min_class_pair_allows_crossing_at_a_cost`: same as the first, blockages soft, detour blocked by a ring band
  → the route crosses: overlap > 0 (and the hard variant of the same set-up reports `metal over gate`).
- `a_clock_never_crosses_a_resistor_body`: body soft + `only_aggressors` hard copy, net 1 `aggressor = true`,
  net 0 not, both forced across → net 0 overlaps the body, net 1 overlaps no body rect (crosses the head).
- `foreign_nets_route_around_a_matched_cell` keeps its assertions; its input moves from `units` to
  `cfg.blockages = [soft, own_exempt, rect = cell]` (the units heuristic is deleted).
- analog `metal_over_gate.rs`: `overlap_is_area_in_um2` (1 000×500 nm shape half over a rect → 0.25).
- library: `blockages_follow_class` (a 2-device Unitization with `class None` → hard gate rects; `Minimal` →
  soft; a cell in no set → none).

### Acceptance
`routing_quality.rs::no_metal_over_gates` on `ota`, `tt_ota`: zero `metal over gate` rows in the routing report
and no new `open net` rows vs. the same run with `blockages` empty; signoff_fixtures stays 19 passed.

---

## RTE-18 Crosstalk: separation in the search, crossing C, aggressor weighting and screening

### Current code (corrected)
- Router coupling: `parasitic` (gr:887-907) prices `beside` tracks only, at `weight + their sens`.
  `RouteHot::sens` (gr:524) kept by `commit` (gr:578-609).
- Rule: `net_pair_af` (coupling.rs:36-45) sums every same-layer pair, no merging, no screening, no crossings;
  callers coupling.rs:101 and differential.rs:114 (two calls). `total_af` coupling.rs:86-104.
- `CrosstalkExclusion` built by the annotator (extract.rs:103) from DiffPair leaves; its repair is `keep_away`
  (dr:2129-2148), used by both the named-pair arm and the `CouplingBudget` arm of `RepairKind::KeepAway`
  (dr:1771-1784).
- Deck heights are `pex … height` (sky130.deck:639-647; plan's :590-596 is stale). Verified: met1–met2 gap
  2006.1 − 1736.1 = 270 nm, k 4.5 → 147.6 aF/µm²; met2–met3 420 nm, k 4.2 → 88.5; met3–met4 390 nm, k 4.1 → 93.1.
  `Pdk::pex_col` already serves `height_nm` (pdk.rs:899).

### Edits
1. `backend/verify/src/pdk.rs`: `pub fn overlap_af_um2(&self, lo: LayerId, hi: LayerId) -> Option<f32>` =
   `8.854·k_lo / (h_hi − (h_lo + t_lo))`, `None` when a key is absent or the gap ≤ 0 (FLOW file: one function,
   reported in the segment report).
2. `stack::Layer` gains `pub cross_af_um2: f32` (to the next metal up; 0 = none), filled at elaborate.rs:292.
3. **coupling.rs** `net_pair_af(stack, a, b, screens: &[Shape]) -> f32`:
   (a) merge `a` per layer into maximal runs (union of collinear overlapping/abutting rects) before pairing;
   (b) for each victim run and side, foreign parallel shapes of `b ∪ screens` sorted by gap; a `b` shape counts
   only over the run length not already covered by a nearer shape (interval subtraction, the shield.rs
   `intersection_len` style); (c) crossings: `a`/`b` shapes on stack-adjacent metals overlapping in xy add
   `overlap_µm² · lower.cross_af_um2`. `total_af` passes `screens` = every net but victim and `other`; Differential
   passes the same (two nets excluded). ponytail: screening is O(n²) per run, as today's scan.
4. **gr**: `RGraph::stacked(&self, n, out: &mut [u32; 2]) -> usize` (default 0; `TrackGrid`: same (ix, iy) on
   l±1). `Elec` gains `agg: &'a [f32]` (per node Σ aggressor weight, `RouteHot::agg`, kept by `commit` exactly like
   `sens`), `a_self: f32`, `cross_c: &'a [f32]` (per lower layer, `c_x·wire_l·wire_{l+1}` over the cheapest step's
   ground C, set at elaborate.rs beside `beside_c`). Coupling of neighbour `j` = `coef·(w_self·agg[j] +
   a_self·sens[j])` (own share subtracted as today). `RouteCtx` gains `agg_w: Vec<f32>` (per net; library fills
   `CouplingBudget::default_weights` via `DetailedCfg::aggressor_weight`), `RouteHot::set_weights` takes both.
5. **Separation (hard)**: `pub struct Separation { pub a: NetId, pub b: NetId, pub lateral_nm: i32, pub no_cross:
   bool }` in gr. New `RuleBatch::separations(&self, out: &mut Vec<(u32, u32, i32, bool)>)` (default none; Vec impl
   via `Rule::separation`), implemented by `CrosstalkExclusion` (`min_spacing_nm, false`) and later `PlateRatio`
   (true). dr turns them into per-net `RouteCtx::sep: Vec<Vec<(u32, [u8; MAX_LAYERS], bool)>>` with
   `tracks_l = ⌈max(0, lateral − (s_l·p0 − wire_l)) / (s_l·p0)⌉`; `NetSearch::sep` slices; `route_net` rejects a
   node when a node within `tracks_l` strides across (repeated `beside`) or, with `no_cross`, a `stacked` node is in
   `hot.foot[other]`. Both nets carry the entry; the later-routed one avoids.
6. Delete the named-pair arm of `RepairKind::KeepAway` (dr:1781-1784). `keep_away` itself stays for the
   victim-only `CouplingBudget` arm (deleting it would remove that repair with no replacement; plan step 3's
   "deleted" overreaches) — test `mirror_guide_follows_the_partner` keeps its `keep_away` asserts.

### Tests
- coupling.rs: `a_wire_behind_a_nearer_wire_is_screened` (victim 10 µm run; X at gap 280 over the full run, Y at
  gap 1 000 behind X: `net_pair_af(.., victim, Y, &X)` == 0.0, and with `screens = &[]` > 0);
  `a_crossing_couples_by_overlap_area` (met1 260 wide × met2 280 wide crossing, `cross_af_um2 = 147.6` →
  `(c − 10.75).abs() < 0.1`); `a_pad_over_a_wire_end_counts_once` (victim wire + a pad over its end vs. wire alone →
  equal C to a parallel aggressor). Existing `CouplingBudget` tests keep their numbers (no overlaps there).
- verify: `sky130_overlap_c_matches_the_deck` (met1/met2 147.6 ±0.1, met2/met3 88.5 ±0.1).
- dr: `separated_nets_never_share_adjacent_tracks` (two nets forced parallel, `CrosstalkExclusion` 1 000 nm in
  `reqs.hard` → `CrosstalkExclusion::satisfied`, and no same-layer gap < 1 000 nm by direct scan);
  `a_no_cross_pair_never_stacks` (a `Separation{no_cross}` through a test `RuleBatch` → no xy overlap between the
  two nets on adjacent layers).
- gr: `an_aggressor_pays_beside_a_sensitive_track` replaces the `sens`-only part of
  `a_sensitive_net_steers_off_a_coupled_track` with the `agg/a_self` form (same geometry, same assertions).

### Acceptance
All `CrosstalkExclusion`s satisfied on every fixture (`routing_quality.rs::crosstalk_exclusions_hold`, plus 19
signoff passes). `routing_quality.rs::coupling_tracks_pex` on `ota`: per net with a `CouplingBudget`, unweighted
`total_af` over `shapes ∪ cell_metal` within 20 % of Σ `signoff.caps` rows `(victim, Some(other), fF·1000)`; the test
prints the calibration row and asserts; a miss is reported with the numbers (PERF-16 owns PEX fidelity).

---

## RTE-15 Joint mirrored routing of matched net pairs

### Current code (corrected)
- Landing is first-come per pin in compact order (dr:504-596), not dr:196-204.
- `RouteCtx::plain` (gr:627) used at gr:744-746 and set at dr:667-670; `Elec::default()` for pair halves.
- `copy_tree` (dr:1954-2002), `mirror_guide` (dr:2008-2032) used by `RepairKind::Mirror` (dr:1744-1755).
- The frame origin is a multiple of `lattice_period` (dr:1152-1167); node centre `ix·p0 + p0/2` (gr:399).
  `TrackGrid::new` may raise the pitch (`coarsened`, gr:264) → no exact map then.
- Differential pairs reach dr through `reqs` already: `RepairKind::Mirror` batches' `touched` give `(pos, neg)`
  pairs (dr:1740). **So no `DetailedCfg::pairs` and no library change**: dr derives `PairSpec`s from `reqs` and the
  pin rects it already has (correction to plan step 1). `axis2` comes from the pins (copy_tree's anchor rule),
  verified exactly; that equals `2·Layout::axis` whenever the pins are mirrored, which is the precondition.
- TrackGrid footprints grow toward + (gr:423); a mirrored wide **vertical-layer** run would need the anchor
  flipped, which `extract_geometry` cannot draw. Decision: a pair whose tied `k` exceeds 1 on a vertical layer falls
  back with reason `wide pair` (differential signal nets are 1-track today; RTE-22 revisits).

### Edits
1. **gr**: `pub enum LatticeMap { MirrorX { k: i32 }, Shift { dx: i32, dy: i32 } }`;
   `impl TrackGrid { pub fn map(&self, m: LatticeMap, n: u32) -> Option<u32> }` (MirrorX `(k − ix, iy, l)`, Shift
   `(ix+dx, iy+dy, l)`, `None` off-grid) and `pub fn lattice_map(&self, m: LatticeMap) -> bool` (MirrorX: `k ≥ 0`
   and `k % stride(l) == 0` for every vertical l; Shift: `dx % stride(l) == 0` on vertical, `dy % stride(l) == 0`
   on horizontal layers).
2. **gr** `pub struct Mirror<'a> { pub map: LatticeMap, pub partner: u32, pub partner_own: &'a [u32], pub
   partner_halo: &'a [u32], pub partner_elec: Elec<'a> }`; `NetSearch::mirror: Option<Mirror<'a>>`. In `route_net`:
   node `i` illegal if `map(i)` is `None`, `== i`, reserved/blocked for the partner, or its footprint intersects
   `i`'s; cost += congestion and `parasitic` of the partner footprint at `map(i)` (partner's own/halo marks by
   `binary_search`). A* stays admissible (costs only add).
3. **gr** `RouteCtx::mirror: Vec<Option<(u32, LatticeMap, bool /*leader*/)>>`: `reroute(follower)` reroutes the
   leader; `commit(leader, tree)` also commits `tree.map(map)` to the partner. `run_pathfinder` treats a follower
   as dirty-forwarding to its leader. Delete `plain` (field, gr:744-746 branch, dr:667-670).
4. **dr** joint landing, before the per-pin loop (dr:504): for each pair `(a, b)` (a = `pos`) in pin `(y, x)`
   order, partner pin = the `b` rect equal to `map(rect)` within `cfg.grid`; the landing node for `a` must pass
   `clean` for a, `clean` for b at `map(n)`, `map(n) ≠ n`, and the mirrored jog legs must be `jog_clean` for b;
   claim both. Map choice: MirrorX with `axis2 = min_x(a pins) + max_x(b pins)` (absolute), `k = (axis2 −
   2·origin.x − p0)/p0` when integral and valid; else Shift when one `(dx, dy)` (centroid difference, multiples of
   `p0`) maps every pin; else fallback. Tied `term_k = max` (after RTE-14's Prim, by mapped terminal).
5. **dr** fallback = today's path with `mirror_guide` and `copy_tree` trials (kept for fallback pairs only).
   `RouteStats` gains `pub pairs_exact: u32` and `pub pairs_fallback: Vec<(u32, u32, &'static str)>` (`no pin map`,
   `axis off lattice`, `wide pair`, `landing failed`).

### Tests (`cargo test -p dr`, `-p gr`)
- dr cfg `DetailedCfg { pitch: 420, grid: 5, wire_width: 260, ..test_cfg() }`, `Differential` (stack =
  `test_stack()`, `max_len_delta_pct10: 0`) on nets 0/1 in `reqs.budget`.
  - `mirrored_pins_route_as_exact_mirrors`: axis `x = 10 290`; pins of net 1 = mirror of net 0's; a ring obstacle on
    net 0's side only → `stats.pairs_exact == 1`; per-layer length, area and cut count equal between the nets
    (exact `assert_eq!`); every net-1 shape is the mirror of a net-0 shape about 10 290; `Differential` residual
    `== 0.0`.
  - `the_axis_track_is_never_used_by_a_pair`: no obstacle → no shape of either net covers `x = 10 290` on the
    vertical layer.
  - `unmatched_pins_fall_back_to_the_guide`: net 1's pins one grid step (5 nm) off → `pairs_exact == 0`,
    `pairs_fallback == [(0, 1, "no pin map")]`, both nets routed (no `open net`), and every shape lies on a path
    between two terminals (no dangling stub: `open_components` of the net minus any one leaf shape increases).
- gr: `mirror_map_is_an_involution` (`map(map(n)) == n` for MirrorX `k = 48` on a 50-wide grid where defined);
  `a_stride_two_layer_rejects_an_odd_k`. Delete the `plain` lines of `a_sensitive_net_steers_off_a_coupled_track`
  (gr:1408-1417): they test the deleted field.

### Acceptance
`routing_quality.rs::pairs_report` on `ota`, `ota_constrained`, `tt_ota` prints `pairs exact n/m` and each fallback
reason; asserts Differential mismatch 0.0 for every exact pair. Mismatch 0.0 for **every** contract pair is
measurable only after PLC-28 snaps axes (dag); until then the fallback list is the evidence.

---

## RTE-17 Split-net topologies: star nodes, Kelvin force/sense

### Current code (corrected)
- `CommonNode { net, a, b, feeds, max_delta_ohm }` (common_node.rs:20-31); constructors: lib.rs:1383, dr:749
  (shift to frame), dr test :2708, common_node.rs test :115. `balance_field` (dr:2113) uses `a/b`.
- Library builds nodes for 2-device DiffPair/CurrentMirror/Load leaves (lib.rs:1353-1386) with MAT-06's
  allowance (`common_node_ohm`); D7: no EXT-24 `max_delta_mv`.
- Ring returns: deferred by the plan (C7); not in scope.

### Edits
1. `CommonNode { net, groups: Vec<Vec<Rect>>, feeds, max_delta_ohm, star: bool }`. `delta_ohm` = max − min of
   per-group parallel R (two groups = today's |R_a − R_b|). Star check: drop net shapes intersecting any feed
   grown by `p0` (new field `CommonNodes::halo_nm: i32`, library: `p0`), components by
   `pnr_core::routes::open_components`-style joins (reuse `Routes::debug_check_joined`'s `Join` list, passed as
   `CommonNodes::joins`), violated iff a component touches pins of two groups. `violations` = star breaks + ΔR
   overshoots; residual unchanged for ΔR.
2. dr: for `star` nodes, each group routes as its own compact pseudo-net (terminals = group pins' landed nodes +
   one root node inside the feed halo, distinct per group, claimed like a pin landing); its shapes are appended to
   the parent net in `build_routes`. Root has too few free nodes → hard V `star root too small net {n}`. Pseudo-
   nets are appended past `n_compact` and mapped back by a `parent: Vec<usize>`.
3. Kelvin = `groups [force, sense]`, `feeds = [terminal rect]`, `star`, `max_delta_ohm 0`.
4. library lib.rs:1383 → `groups: vec![pins(a), pins(b)], star: false`. StarReq/KelvinReq mapping waits on
   EXT-24 (annotator, M3): add a `// RTE-17 step 4: EXT-24` note only. `balance_field` stays (no matched groups route
   jointly without EXT-24).

### Tests
- common_node.rs: `two_groups_match_the_old_a_b_measure` (existing test rewritten to `groups`, same residuals);
  `a_shared_segment_breaks_the_star` (two groups whose branches share 5 µm outside the halo → `violations == 1`);
  `branches_meeting_at_the_root_are_a_star` (meeting inside feed grown by `p0` → 0).
- dr: `a_kelvin_sense_branch_starts_at_the_tap` (force pins +1 000 µA, feed −1 000 µA, sense pin 0 µA, star;
  REL-03 `net_flow` gives `shape_ua == 0.0` on every shape between sense pin and tap; star violations 0).
- `a_skewed_common_node_is_rebalanced` keeps its assertion with `groups`.

### Acceptance
`routing_quality.rs::common_nodes_meet_allowance` on `ota`: every `CommonNode` with `max_delta_ohm > 0` has
`usage ≤ 1`. Star/Kelvin acceptance on fixtures waits on EXT-24 producers (stated in the report).

---

## RTE-19 Shields generated during the search

### Current code (corrected)
- `add_shield` (dr:1867-1934) claims free parallel stretches after repair (dr:779-788), all-or-nothing; reference
  is ground only with a clock (extract.rs:127-158). `Shield::max_gap_nm` (shield.rs:26).
- Guard tracks already exist in the search (`RouteCtx::guard`, gr:652; counted in the net's footprint).

### Edits
1. Before PathFinder (dr:700), for each `(victim, reference)` from `shield_pairs`: `cold.guard[v][l] =
   max(guard, 1)` on every layer whose edge gap `s_l·p0 − (W_l(k) + wire_l)/2 ≤ max_gap_nm`; if no layer
   qualifies, push budget `Violation { rule: format!("shield gap off lattice net {v}"), margin: gap − max }` after
   `score` and skip that shield. (Needs `max_gap_nm`: `RuleBatch::shield_pairs` gains it, `(u32, u32, i32)`.)
2. In the shield stage (dr:779-788): restore the victim's guard, recommit its tree (frees the side tracks, which
   the search kept free of foreign metal), then `add_shield` as today (its `free` test now finds them). Reference
   from EXT-24 later; ground fallback unchanged.
3. No plate shields (plan step 4).

### Tests
- dr `a_shielded_victim_gets_both_side_tracks`: 20 µm two-pin victim, `Shield { min_coverage_pct: 80,
  max_gap_nm: 430 }`, a foreign net routed first along the victim's row → coverage `≥ 0.95`. If the end pads keep
  it below 0.95, report the measured value; do not lower the assertion.
- dr `a_shield_does_not_count_as_an_aggressor`: same routes, `CouplingBudget { exclude: Some(reference) }` total is
  equal (`assert_eq!`) with and without the reference's shield shapes.
- `a_shield_request_draws_tied_reference_tracks_both_sides` keeps `≥ 0.75`.

### Acceptance
Every requested shield reaches 80 % on the unit fixtures; on bench circuits with a clock (none today; PERF adds a
comparator, SURV-40) — stated as not measurable yet.

---

## RTE-20 Capacitor-set plate routing

### Current code (corrected)
- `cellgen::dac_banks` no longer exists: EXT-19 (`be34be4`) moved banks to `annotator::passive::capacitor_sets`
  (passive.rs:124), which become `Unitization`s with `device_type Capacitor`, `dev_nf` = units per member, members
  sorted so the rail-terminated unit is first. So the source is the Unitization now (dag: "EXT-19 switches the
  source"; both exist, so build from it directly).
- `trim_pair` is at `7f8d92b^:backend/dr/src/lib.rs:1037`. `rule::over` is `pub(crate)` in analog.
- `PlateSet` must be visible to dr and analog → define it in `analog::routing::plate_ratio` (correction).

### Edits
1. `analog::routing::plate_ratio`: `pub struct PlateSet { pub top: NetId, pub bits: Vec<(NetId, u32)>, pub
   c_unit_af: f32, pub array: Rect }` and `pub struct PlateRatio { pub set: PlateSet, pub tol_pct10: i32, pub stack:
   &'static Stack }` (Clone, `RuleBatch` via a Vec of them, budget arm). `C_i = stack.ground_af(S_i) + Σ_m
   net_pair_af(Some(stack), S_i, r.shapes(m), screens)`, `S_i` = bit shapes not contained in `array`; spread,
   residual, `known` exactly as plan step 3; `separation()` yields `(top, bit, S_l, true)` per bit (RTE-18).
2. `DetailedCfg::plates: Vec<PlateSet>`; library (lib.rs:1202) fills it per layout: per capacitor Unitization with
   `devices.len() ≥ 3`, `top` = `P` net, `bits` = `(N net, dev_nf)` of members whose `N` is not a rail, `c_unit_af` =
   `c_af` of a member with one unit (`NAN` if none), `array` = the placed macro bbox.
3. dr: separations from `plates` join RTE-18's `sep`; `PlateRatio` is evaluated as an `extra` batch (dr:756) and
   its Θ appended to the report via `gr::analog_tiers` on a local `Requirements`.
4. dr `equalize_leads` (restored from `trim_pair`): `target = max_j C_j/n_j`; per short bit a same-layer dead-end
   stub continuing one of its runs outside `array`, `L = (target − C_i/n_i)·n_i / c_per_nm(l)`; kept only if spread
   drops and the stub clears foreign metal by `S_l`. Runs after geometry, before `score`.

### Tests
- dr `bottom_plates_never_cross_the_top_plate`: macro with pin `P` and four `N_i`, plates set → no xy overlap of bit
  and top shapes on adjacent layers; every same-layer gap ≥ S.
- plate_ratio.rs `lead_capacitance_is_equalised_per_unit`: bits 1/2/4/8, equal 10 µm met1 leads, `c_unit_af =
  1 000` → before `spread_pct > 1.0`; after `equalize_leads` (via a dr unit test) every `C_i/n_i` within 10 aF of
  the mean.

### Acceptance
`routing_quality.rs::dac4_plates`: no top/bit crossing outside the array, `PlateRatio` residual 0.0.

---

## Order and commits
RTE-16 → RTE-18 → RTE-15 → RTE-17 → RTE-19 → RTE-20, one commit each (`M2+ RTE-xx: …`), each with
`cargo test -p analog -p gr -p dr -p library` green and the 19 fixture passes kept. Report out-of-scope finds (FLOW
`pdk.rs` addition in RTE-18, CELL's missing `KeepWhy::Gate`) in `docs/plans/m2-routing-report.md`.
