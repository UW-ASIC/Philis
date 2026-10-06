# M2 routing segment 1: implementation cards (RTE-10, RTE-11, RTE-12, RTE-13, RTE-25)

Branch `m2-routing`, worktree `philis-m2/routing`. `git merge m2` was already up to date (`c2940c6`), so these line
numbers are from that tree. Each item moves the next item's lines, so find code by the symbol named beside each line
number. Every command needs `export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk`. The DAG files
(`dag-m2-m6.json/.md`) exist only in the main checkout. Their entries are RTE-10 with no hard deps, RTE-11 and RTE-12
after RTE-10, RTE-13 with no hard deps, and RTE-25 after RTE-10 and RTE-13. No item here needs PLC-06.

| Item | Class |
|---|---|
| RTE-10 | do (judgment) |
| RTE-11 | do (judgment) |
| RTE-12 | do (judgment) |
| RTE-13 | do (judgment) |
| RTE-25 | do (judgment) |

Batch rules:
- No new warnings in `gr`, `dr` or `library`.
- A test that a step deletes is listed with the code it tested. Any other test that goes red means stop and report.
  Never edit its assertion.
- Gates for every item: `cargo test -p gr -p dr`, `cargo test -p library --lib`, and
  `cargo test --release -p benchmark --test signoff_fixtures -- --ignored`, plus RTE-12/13 timing (protocol below).

**Timing protocol (T13, used by RTE-12 and RTE-13).** No caller reads `RouteStats` today: the flow drops it at
`frontend/library/src/lib.rs:840` and `:857`. There is no `RunStats.stage_ms` yet (FLOW-09), and no M0 per-call dr
number exists. Measure with an **uncommitted** probe: put `let t = std::time::Instant::now();` before each of the two
`router.route(..)` calls and `eprintln!("dr_us {}", t.elapsed().as_micros());` after each. Then run
`cargo run --release -p benchmark --bin bench local` on `ota` with seed 1, take the p50 of the `dr_us` lines, and
restore the file with `git checkout -- frontend/library/src/lib.rs`.
- **Baseline:** take it once, before RTE-10's first commit, on `c2940c6` (post-M1). Record it in RTE-10's commit
  message.
- **Departure:** T13 says "M0 baseline". M0 cannot be built here because we have one worktree and may not make
  another, so the post-M1 number replaces it. Report this departure.

---

## RTE-10 Per-layer lattice

### Current code (plan-05 line numbers are pre-RTE-07; corrected here)
- `gr::TrackGrid` (`backend/gr/src/lib.rs:172-261`) has `nx, ny, pitch, via_cost, n_layers`.
  - `with_layers` (:184-189) silently raises `pitch` to `max(die)/1200` (the AT-37 coarsening).
  - `neighbors` (:267-288) treats even layers as horizontal. `pos` (:289-292) puts a node at `ix·pitch + pitch/2`.
  - `beside` (:293-307) steps ±1 track.
  - `candidates` (:223-260) excludes stacked vias at ±2 rows (:243-247).
- `extract_geometry(hot, grid, width)` (:725-746) and `emit_run` (:749-767) draw one width for all layers.
  `Via { size }` is square.
- dr:
  - `DetailedCfg.pitch` and `wire_width` (`backend/dr/src/lib.rs:46,49`) are uniform.
  - The frame origin is `low − halo − margin` (dr:261-267), so it is arbitrary and the absolute history keys
    (`gr::Negotiation`) do not line up across epochs.
  - `build_routes` (dr:1332-1364) draws square pads `cuts[i].2/.3` centred on the via.
  - `min_space = cfg.pitch − cfg.wire_width` (dr:436) is used for every layer: the cell-metal inflation (dr:441) and
    the fill (dr:935-936).
- `elaborate::detailed_router` (`frontend/library/src/elaborate.rs:382-458`):
  - `wire_width = access_pad.max(min_w)` (:397).
  - `pitch = pdk.routing_pitch(..)` (:398).
  - `layer_c`/`beside_c`/`layer_r` use one pitch and one width (:436-450).
- pdk (`backend/verify/src/pdk.rs`):
  - `routing_vias` (:424-450) draws square pads at `max(min_enclosure, min_one_side)` enclosure.
  - `max_enclosure_of` (:453-468) keys on the inner layer only.
  - `routing_pitch` (:503-513), `route_spacing` (:519), `wide_spacing` (:528).
  - **The plan missed one thing:** `Pdk::cut_enclosure(outer, inner)` (:1228-1233) already filters on both layers
    and returns `max(limit, min_one_side)`, which is `E_along`. Only `E_across` is new.
- sky130 deck line numbers (plan's are stale): metals and spacing at `pdks/decks/sky130.deck:368`; area at :380-383;
  enclosures at :391-412. No `eol_spacing` on met1–met4, so `S = min_spacing`. The derivation table in plan-05
  RTE-10 was re-checked by hand against these lines and holds: `p0 = 420`, strides `[1,1,2,2]`, halos
  `(0,1),(0,1),(1,2),(1,2)`.

### Decisions (smaller than the plan; reasons given)
1. **`Cut` and `routing_vias` stay square.** They still feed pin access and landing (`pad0`, `stitch`, `access_pad`),
   which this item does not change. Oriented pads come from `LayerSpec`. Plan step 7's tuple change is dropped: it
   would ripple through `joins`, `assert_on_stack`, `add_pin_access` and every test `CUTS` for no DRC gain.
2. **`DetailedCfg` keeps `pitch` and `wire_width`, with new meanings.**
   - `pitch` now means `p0`.
   - `wire_width` now means layer 0's wire, which landing and access use.
   - New field: `layers: Vec<gr::LayerSpec>`. Empty gives the old uniform lattice (stride 1, wire `wire_width`,
     square pads from `cuts`), so the ~40 dr tests built on `test_cfg()` keep working unchanged.
3. **`LayerSpec` drops `wide`.** Wide spacing stays in `cfg.spacing` (`DetailedCfg::space`), which is its single
   source. `space` stays.
4. **Off-track nodes get no edges (gr), instead of being marked `BLOCKED` in dr.** `neighbors`, `candidates` and
   `beside` only ever return on-track nodes. This is one place, it also cuts A*/Dijkstra pushes, and it needs no dr
   loop.
5. **No wire extension at a via (plan step 5).** The oriented pad is drawn anyway and is at least as long, so the
   union is the same metal.
6. **Halos (plan step 6) do not go through `usage`.**
   - The problem: if halos added to `usage` (cap 1), two foreign halos covering the same empty node would overflow.
     On met1 (`halo_via = 1`), two via ends 2 nodes apart (pad gap 520 nm, legal) would then conflict.
   - The fix: halo nodes conflict only with foreign *metal*. RouteHot gets `halo: Vec<u16>`, a count of foreign halos
     per node. A net's halo never includes its own metal nodes.
   - Overuse at `n` is `if usage[n] > 0 { usage[n] + halo[n] − cap } else 0`, floored at 0. It is used everywhere
     overuse is counted (see `RouteHot::over` below).

### Edits
- `backend/gr/src/lib.rs`:
  - Add `pub const MAX_LAYERS: usize = 8; pub const MAX_STRIDE: u32 = 4;`.
  - Add
    `#[derive(Clone, Debug, Default)] pub struct LayerSpec { pub id: LayerId, pub horizontal: bool, pub stride: u32, pub wire: i32, pub space: i32, pub pad_across: i32, pub pad_along: i32, pub halo_wire: u8, pub halo_via: u8 }`,
    documented with the plan's field docs and Hastings Eq 15.19.
  - `TrackGrid` gains `pub specs: Vec<LayerSpec>`, which is either empty or `n_layers` long.
  - Add `pub fn new(die: (i32, i32), p0: i32, specs: Vec<LayerSpec>, via_cost: f32) -> Self`, with the same coarsening
    as today. `with_layers(die, pitch, via_cost, n)` becomes `Self::new(die, pitch, vec![], via_cost)` with
    `n_layers = n`, so the tests are untouched.
  - Add `pub fn stride(&self, layer: u32) -> u32`, which is 1 when `specs` is empty.
  - Add `pub fn on_track(&self, n: u32) -> bool`, which is
    `(if horizontal(layer) { iy } else { ix }) % stride == 0`. Horizontal means `layer % 2 == 0`, unchanged.
  - `neighbors`: skip a via neighbour that is not on-track. Planar steps stay ±1 node along the track.
  - `beside`: step `stride` tracks across.
  - `candidates`: add `self.on_track(n)` to the accept test.
  - `extract_geometry(hot, grid, widths: &[i32])`: width per track layer. `emit_run` takes the layer's width.
  - `Via` keeps `size` (unused by dr after this item; the gr tests still read it).
  - Add `pub fn halo_nodes(&self, tree: &[Vec<u32>], out: &mut Vec<u32>)`. For every run end it adds `halo_wire`
    along-track nodes beyond the end. For every via end it adds `halo_via` along-track nodes each side, on both
    layers. All of them are in-bounds and on-track. The caller dedups them and removes the net's own metal nodes.
  - `RouteHot`:
    - Add `pub halo: Vec<u16>` and `pub halos: Vec<Vec<u32>>`, the per-net halo set.
    - `commit(net, branches, halo: Vec<u32>)` moves both claims.
    - Add `pub fn over(&self, n: usize, cap: u16) -> u16`.
  - Add `RouteCtx::commit(&self, hot: &mut RouteHot, net: usize, branches: Vec<Vec<u32>>)`. It computes the halo
    with `graph`, so `RGraph` gains `fn halo_nodes(&self, _: &[Vec<u32>], _: &mut Vec<u32>) {}` (default: none) and
    `TrackGrid` implements it.
  - Replace every `hot.commit(..)` (11 sites in gr and dr) with `cold.commit(hot, ..)`.
  - `run_pathfinder`'s dirty test and `bump_history` use `over`.
  - `route_net`'s `node_cost`: the effective usage is now `usage − own + (halo − own_halo)`. Mark the own halo with
    a third stamp array `is_old_halo` in `Dij`, which is fed from `hot.halos[net]`.
- `backend/verify/src/pdk.rs`:
  - Add
    `pub fn cut_enclosure_pair(&self, outer: LayerId, inner: LayerId) -> (i32, i32) /* (across, along) = (limit, max(limit, min_one_side)) */`
    next to `cut_enclosure`.
  - Rewrite `cut_enclosure` as `self.cut_enclosure_pair(outer, inner).1`.
  - Delete `routing_pitch` (:503-513) and its ponytail. Its two tests, `routing_pitch_clears_every_layer_of_its_stack`
    and `an_unreachable_layer_does_not_set_the_pitch` (:1622-1645), go with it, replaced as listed under Tests.
- `frontend/library/src/elaborate.rs`:
  - Add
    `pub(crate) fn layer_specs(pdk: &Pdk, layers: &[LayerId], cuts: &[Cut], pin_access: Option<(LayerId, Cut)>) -> (i32, Vec<gr::LayerSpec>)`
    implementing plan step 2 exactly. Every metal takes its landing cuts from below and above, plus the pin-access
    cut for `layers[0]`.
  - Round `pad_*` up to `pdk.grid`, and `p0` up to `2·grid`.
  - Use `S_l = route_spacing(l)` and `min_area` from `pdk.min_area`.
  - In `detailed_router`:
    - Set `cfg.pitch = p0`, `cfg.layers = specs`, `cfg.wire_width = specs[0].wire`.
    - Delete the `routing_pitch` call.
    - Per-layer Elec: compute `ground` and `layer_r` at `specs[l].wire` per `p0` step, and `side` at
      `stride·p0 − wire`.
- `backend/dr/src/lib.rs`:
  - `DetailedCfg.layers` (doc: empty means uniform). The grid is built as
    `TrackGrid::new(die, cfg.pitch, cfg.layers[..n_layers].to_vec(), VIA_COST, ..)`, with `n_layers` from the specs
    when they are present.
  - Frame origin: add `fn frame_origin(low: (i32, i32), pad: i32, period: i32) -> (i32, i32)`, which is
    `((low − pad).div_euclid(period) · period)` per axis. Call it with `period = p0 · lcm(strides)` (`p0` when there
    are none).
  - `build_routes(hot, grid, cfg, compact, n_nets, layers, cuts)`:
    - Width per layer: `spec.wire`, or `cfg.wire_width` when `specs` is empty.
    - The cut is still drawn `size²`.
    - The pad on metal `i` is `pad_along × pad_across` oriented to `specs[i].horizontal`: horizontal gives
      `w = pad_along, h = pad_across`. The legacy path draws a square.
  - Per-layer spacing:
    - At dr:441, use `cfg.space(s.layer, .., spec_wire(l), min_space) + spec_wire(l)/2`.
    - At dr:935-936, call heal/fill once per fill layer with `cfg.space(l, 0, 0, min_space)` and that layer's
      `min_width`. A single min_space would let met3 fills land 160 nm from foreign metal, against 300 nm needed.

### Tests
- `elaborate.rs` `mod tests`, `sky130_layer_specs_match_the_hand_derivation`:
  - Inputs: `Pdk::builtin("sky130")`; `layers = routing_layers()[1..5]` (met1–met4); `cuts = routing_vias()[1..4]`;
    pin access `(li, routing_vias()[0])`.
  - Every cell of plan-05's RTE-10 table must hold: `pad_across [260,280,330,330]`, `pad_along [320,370,730,730]`,
    `wire [260,280,330,330]`, `space [140,140,300,300]`, `stride [1,1,2,2]`, `halo_wire [0,0,1,1]`,
    `halo_via [1,1,2,2]`, `horizontal [t,f,t,f]`.
  - `p0 == 420`.
- `elaborate.rs`, `every_layer_pitch_clears_its_spacing` (replaces the deleted pdk test). For each built-in deck:
  for every spec, `stride·p0 − wire ≥ min_spacing(id)` and `stride·p0 ≥ pad_across + space`.
- `gr`:
  - `off_track_nodes_have_no_edges`: a 3-layer `TrackGrid::new` with strides `[1,1,2]`. No neighbour of any node is
    off-track, and an on-track layer-2 node has a via edge down.
  - `beside_steps_by_stride`: on the stride-2 layer, `beside` returns `n ± 2·step`.
  - `a_via_end_claims_its_halo`: commit a two-node via branch with `halo_via = 1`. The `halo` count is 1 on the
    along-track neighbours of both via nodes and 0 elsewhere. Committing a foreign net onto such a node makes
    `over(..) == 1`, while two foreign halos on one empty node make `over == 0`.
- `dr`:
  - `origin_is_a_multiple_of_the_lattice_period`: `frame_origin((-12_345, 7), 2_100, 840)` is `≡ 0 (mod 840)` and
    `≤ low − pad`.
  - `adjacent_foreign_vias_keep_the_along_track_halo`:
    - Setup: a `test_cfg` with two specs (stride 1, `wire 260`, `space 140`, `pad 260×320`, `halo_via 1`). Two nets,
      each needing a layer change, with pins one `p0` apart along a layer-0 track.
    - Assertion: every pair of foreign same-layer shapes has `rect_gap ≥ 140`.
    - It must fail with `halo_via = 0`. Check this once before committing.
- Acceptance: routing-layer DRC 0 on `signoff_fixtures` and the 3 slow OTAs. On sky130, RTE-10 alone still routes
  met1+met2, because the `take_while` stays until RTE-11.

---

## RTE-11 Route on every useful metal

### Current code
- `routing_stack(pdk)` (`elaborate.rs:201-238`):
  - `take_while(min_width ≤ access_pad)` (:204-208).
  - An unconditional `pin_access` split when there are more than one layer (:211).
  - Three `assert!`s (:214-235).
- Callers: `elaborate.rs:159`, `frontend/library/src/lib.rs:423` (`solve`), and `lib.rs:679` (`annotation`, which
  reads `layers.first()` min_width). The plan cites `lib.rs:279`, which is stale.
- The pex rows the plan cites at stale deck lines:
  - sky130 `li_c` 12.8 at :628 and `met1` 0.125 at :630.
  - gf180 `metal1_con`/`metal2_con` 0.09 at :583/:585.
  - ihp `metal1` 0.11 at :685 and `metal2` 0.088 at :687.
  - generic_finfet `M1` 1.2675 at :344 and `M2` 0.534829 at :346.
- `pex_f32(li)` resolves `li_c` through `pex_row`, the same path `elaborate::stack` uses.
- `Pdk::load` is at pdk.rs:120, not :97.

### Edits
- `elaborate.rs`:
  - Add
    `pub(crate) struct RoutingStack { pub layers: Vec<LayerId>, pub cuts: Vec<Cut>, pub pin_access: Option<(LayerId, Cut)>, pub p0: i32, pub specs: Vec<gr::LayerSpec> }`.
    The ids are kept beside the specs because `dr::route` and every caller take `&[LayerId]`.
  - Add
    `pub(crate) fn routing_stack(pdk: &Pdk, top: Option<usize>) -> Result<RoutingStack, String>`.
  - Step 1: split `routing_layers()[0]` into pin access iff
    `sheet(l0) > PIN_ACCESS_SHEET_RATIO · sheet(l1)`, with `const PIN_ACCESS_SHEET_RATIO: f32 = 10.0` (tuning
    default) and both sheets from `pdk.pex_f32(_, "sheet_res_ohm_sq")`. A missing sheet means no split.
  - Step 2: compute `layer_specs` over all remaining metals. While the top spec's `stride > gr::MAX_STRIDE`, pop the
    top metal and its cut and recompute. Then truncate to `top`, recomputing again.
  - The three asserts become `Err` with the same messages. Delete `take_while` and `access_pad` if nothing else uses
    it.
  - `detailed_router(pdk, &RoutingStack)` uses `stack.p0` and `stack.specs`, and no longer recomputes them.
- Callers. Neither `route_built` (`elaborate.rs:136`, returns `Elaborated`) nor `solve` (`lib.rs:391`, returns
  `(Solution, LexKey, bool)`) returns a `Result`. Making them return one belongs to FLOW's error path (FLOW-11), so
  this item does not widen them:
  - `elaborate.rs:159` and `lib.rs:423` call
    `routing_stack(pdk, None).unwrap_or_else(|e| panic!("routing stack: {e}"))`. This behaves as today's `assert!`
    does, now with a `Result` that FLOW-11 can propagate. Record the hand-off in the report.
  - `lib.rs:679` (`annotation`) falls back to width 0 on `Err`.

### Tests (`elaborate.rs` `mod tests`)
- `sky130_routes_met1_to_met4`: layers are `[met1, met2, met3, met4]`, `pin_access` is `Some((li, mcon cut))`,
  `p0 == 420`, strides are `[1,1,2,2]`, and met4's spec equals the RTE-10 table row (via4 excluded).
- `gf180_routes_metal1` and `ihp_routes_metal1`: `layers[0]` is the deck's `metal1` and `pin_access` is `None`.
- `a_top_cap_keeps_the_lowest_metals`: sky130 with `top = Some(2)` gives `[met1, met2]`.
- `a_bad_stack_is_an_error_not_a_panic`:
  - Load the sky130 deck text with the sky130 sidecar after removing `"via4"` from `routing_vias`.
  - Assert `Pdk::load(..).map_or(true, |p| routing_stack(&p, None).is_err())`.
  - If `routing_vias` panics on `self.routing_cuts[i]` (pdk.rs:430), change it to `.get(i)` with an early stop.
    That is the AF-35 bug. Report it if it falls outside routing scope.
- Acceptance:
  - `ota`, `tt_ota` and `ota_constrained` have DRC 0 on met1–met4 (bench local).
  - `ota_cross_pdk` (`cargo test --release -p library --test ota_cross_pdk`) reports its routing-layer DRC count.
  - gf180/ihp are driven to 0 by RTE-26, not here.

---

## RTE-12 Footprint model: width reserved during search

### Current code
- Fattening is dr:704-781: `fat_width` (:150-152), `widen` (:1050-1060), and `fat_signal`/`fat_supply` (:79-80,
  `elaborate.rs:425-434`).
- `em_width` (:169-176) and the `em_shortfall` Θ (:713, :777-779) are computed inside the fattening loop.
- The via-array overlap fill is dr:783-882. It uses `enc = (max(below, above) − size)/2` (:829), a square
  assumption.
- `route_net` (gr:512-640) takes 12 arguments.
- `node_ua` per compact net is at dr:341.
- Sidecar keys `route_signal_width`/`route_supply_width` are read only at `elaborate.rs:433-434`. They are not
  registered in any `KEYS`, so nothing else needs to change.

### Decisions
1. **`k` is per net and per layer**, `RouteCtx.k: Vec<[u8; MAX_LAYERS]>` (empty means all 1), not per `Branch`.
   - Plan step 6 gives every branch the same interim `k`, and per-branch `k` is RTE-14's.
   - So `trees` stay `Vec<Vec<Vec<u32>>>`, and `copy_tree`, `mirror_guide`, `add_shield` and `lift_field` are
     untouched.
   - The `Branch` type is deferred to RTE-14.
2. **No `side` parameter.** It is always +1 until RTE-15 adds the mirror.
3. **Footprint metal is deduplicated per net**, against the plan's "with multiplicity". A net's own overlapping
   tracks are one conductor, so the own-usage subtraction stays 1, as today.
4. **The EM Θ stays** (master §6.1 C18: REL-03 keeps dr's EM Θ until RTE-14). So `em_width` and the per-segment
   `need` vs drawn-width shortfall are kept, with `got` = the drawn width. Only the widening search is deleted. This
   contradicts plan step 6's "em_width deleted"; C18 wins.

### Edits
- gr:
  - `RouteCtx.k`.
  - Add
    `pub fn footprint(&self, n: u32, k: u8, out: &mut Vec<u32>) -> bool` on `TrackGrid`. It returns the `k`
    across-tracks starting at `n` (stride steps), and false if any of them is out of bounds.
  - Add `pub fn via_block(&self, a: u32, b: u32, ka: u8, kb: u8, out: &mut Vec<u32>) -> bool`, the `ka×kb` corner on
    both layers.
  - `RouteCtx::commit` stores the deduplicated metal footprint in a new `hot.foot: Vec<Vec<u32>>`. `usage` and
    `sens` move over `foot`. `tree_nodes` becomes `foot[net]` and is sorted.
  - `halo_nodes` uses the footprint's outer tracks.
  - Add
    `pub struct NetSearch<'a> { pub net: u32, pub terms: &'a [u32], pub k: [u8; MAX_LAYERS], pub own: &'a [u32], pub own_halo: &'a [u32], pub penalty: &'a [f32], pub keepout: (&'a [u32], &'a [u32]), pub elec: Elec<'a> }`.
  - Change the signature to
    `pub fn route_net<G: RGraph>(g: &G, hot: &RouteHot, reserved: &[u32], q: &NetSearch, p_fac: f32, dij: &mut Dij) -> Option<Vec<Vec<u32>>>`.
  - `RGraph` gains `fn footprint(&self, n: u32, k: u8, out: &mut Vec<u32>) -> bool { out.clear(); out.push(n); true }`
    and a matching default `via_block`.
  - Node cost is the sum over the footprint of `hist + p_fac·max(0, eff + 1 − cap)`, plus `penalty`, `keepout` and
    `parasitic` of the anchor. A footprint node that is out of bounds or reserved for another net makes the edge
    illegal. A via edge adds the `via_block` sum.
  - `extract_geometry(hot, grid, widths, k: &[[u8; MAX_LAYERS]])`:
    - Below the layer's first wide threshold, a run on a `k`-track layer is drawn merged:
      `W = wire + (k−1)·stride·p0`, centred `(k−1)·stride·p0/2` across toward +side.
    - At or over the threshold, it is drawn as `k` wires.
    - The threshold comes in as a per-layer `wide: &[i32]` (`i32::MAX` for none).
    - At or over the threshold, plan step 1's extra blocked tracks also go into `footprint` as metal-free claims.
      Simplest: the `k` given to the search is raised by those tracks, and only `k` are drawn.
- dr:
  - Delete the fattening loop (:704-781) except the per-segment EM `need`/shortfall.
  - Delete `fat_width`, `widen`, the `fat_signal`/`fat_supply` fields and defaults, and `elaborate.rs:425-434`.
  - Interim `k` after landing, per compact net and layer:
    `k_l = 1 + ceil(max(0, I_max·1000/J_l − wire_l) / (stride_l·p0))`, capped at `const K_MAX: u8 = 8`.
    - `I_max = max(Σ ua⁺, Σ −ua⁻)` over `node_ua[ci]`.
    - `J_l = cfg.em_limit(id).ua_per_um`. Zero, or no limit, gives `k_l = 1`.
    - Supplies get no special case.
    - Set `cold.k`.
  - Via arrays: the enclosure is per axis. Along a layer's wire it is `E_along`, across it `E_across`, from a new
    `cfg.cut_enclosure_pair: Vec<(LayerId, (i32, i32), (i32, i32))>` filled in `detailed_router` from
    `Pdk::cut_enclosure_pair`. Drop the square `enc`.

### Tests (`dr`)
- Deleted with the code they test: `fat_trunks_get_via_arrays` and `a_differential_pair_is_not_fattened`. Each is
  replaced:
  - `a_two_by_two_corner_gets_four_cuts`:
    - Setup: sky130-valued specs (met1 H `wire 260`, met2 V `wire 280`, `p0 420`, stride 1); `cuts = [(via, 150, ..)]`
      with `cut_enclosure_pair` `(55, 85)` both sides; `cfg.space` via 170. One net with `k = [2, 2]` is forced to
      turn met1→met2.
    - Assert the drawn widths are 680 and 700, and that exactly 4 cuts lie in the corner.
  - `a_differential_pair_keeps_one_track`: the same pins and reqs as the deleted test, with no `pin_ua`. Assert
    `widest(0) == widest(1) == Some(290)`. This is the old assertion, minus the free-net fattening line.
- `a_two_track_net_reserves_both_tracks`:
  - Setup: net 0 with `k = [2, 1]` runs along a horizontal track. Net 1's pins sit on the adjacent (+1) track inside
    net 0's span.
  - Assert net 1's route never uses that track inside the span. Equivalently, no same-layer foreign gap is below
    `space`.
- `a_wide_bundle_keeps_wide_spacing`:
  - Setup: a synthetic layer with `cfg.spacing` `[(1000, 500)]` and `k = 4` (`W ≥ 1000`).
  - Assert the bundle is drawn as 4 wires, and that any foreign wire alongside sits ≥ 500 nm from it.
- Acceptance:
  - dr p50 on `ota` is ≤ ½ of the baseline (timing protocol).
  - DRC no worse than at RTE-11.
  - The bench constraint table's `Electromigration` V on `ota` is not above RTE-11's.
  - If the 2× is not met, report the measured ratio. Do not add tuning to hit it.

---

## RTE-13 Search speed, reuse, per-stage measurement

### Current code
- The Dijkstra loop is gr:591-624: `heap: BinaryHeap<Reverse<(u32 bits, u32 node)>>`, with stale-entry skip
  `d > dist[n]`.
- `Dij::new` is called per trial (dr:1562), per shield (dr:1632), and inside `run_pathfinder` (gr:654).
- **The plan is stale here:** `first_short` no longer exists. The O(S²) passes are `break_shorts` (dr:2116, with a
  ponytail doc) and `cross_net_shorts` (dr:2152, run by `score` on every repair probe). The via-array clearance scans
  `all_cuts` (:843-845).
- `RouteStats` (dr:184-200) already has `us_*`, `expanded`, `coarsened`, `width_fallbacks`, `single_cut_vias` and
  `congestion`. Only `trials`, `overuse` and `pf_iters` are set (dr:958).

### Edits
- gr:
  - Add `RGraph::lower_bound(&self, a: u32, b: u32) -> f32 { 0.0 }`. `TrackGrid` implements it as
    `|Δix| + |Δiy| + via_cost·|Δl|`.
  - In `route_net`, per target, push `(g + h(nb)).to_bits()`. On pop, skip unless
    `f_bits == (dist[n] + h(n)).to_bits()`: the same expression gives the same bits, which replaces the
    `d > dist[n]` test.
  - Add `debug_assert!(node_cost(i) >= 0.0)`.
  - `Dij` gains `pub pops: u64`, incremented per pop.
  - `run_pathfinder(.., dij: &mut Dij)` takes the caller's `Dij`.
  - `TrackGrid` gains `pub coarsened: bool`, set by `new` when it raised the pitch.
- dr:
  - Create one `Dij` in `route`, passed `&mut` to `run_pathfinder`, `repair_constraints` → `trial`, and `add_shield`.
  - Timers: `Instant` around:
    - landing: dr:346-584, `zones` through the landing loop;
    - negotiate: dr:599;
    - repair: dr:603-666, IR re-pricing, `repair_constraints` and shields;
    - geometry: dr:684-882;
    - fill: dr:884-940.
  - At the end: `expanded = dij.pops`, `coarsened = cold.graph.coarsened`.
  - `ShapeIndex` (private to dr):
    `struct ShapeIndex { side: i32, cells: HashMap<(LayerId, i32, i32), Vec<u32>> }` with
    `fn near(&self, layer: LayerId, r: Rect, halo: i32) -> impl Iterator<Item = u32> + '_`, and buckets of
    `4·cfg.pitch`.
  - Use it first in `break_shorts`, `cross_net_shorts` and the `all_cuts` clearance.
  - Extend it to `add_pin_access`/fill only if the stage timers show geometry or fill still dominating the dr p50
    after that. Ponytail: index only the measured hot spots.

### Tests (`gr`)
- `astar_matches_dijkstra_cost`: 50 seeds of a random 30×30×2 lattice, with `hist` in [0, 2] set through a
  `RouteHot` and source and target random on layer 0. Compare path cost against a `NoBound` wrapper graph (default
  `lower_bound`) to 1e-4. Use a small LCG; no new dependency.
- `astar_expands_fewer_nodes`: an open 200×200×2 lattice from (100,100,0) to (150,100,0). `pops(A*) ≤ pops(Dijkstra)/2`.
- dr: `route_fills_stage_timers`. On `detailed_realises_a_two_pin_net`'s setup, `stats.expanded > 0` and
  `us_negotiate + us_geometry > 0`.
- Acceptance: after RTE-12 and RTE-13, dr p50 on `ota` is ≤ ⅓ of the baseline (timing protocol, post-M1 baseline
  departure noted). Stage times are in `RouteStats`; printing them is PERF-20/FLOW-09, not this item.

---

## RTE-25 Lattice and congestion export

### Current code
- `RouteStats::congestion: Vec<(Rect, f32)>` exists (dr:199) and is never filled.
- There is no `LatticeSpec`.
- PLC-15 (plan-04:536) reads `demand > 1` as "over capacity". PLC-28 (plan-04:646) calls `dr::lattice_spec(&cfg)`.

### Decisions
- **Formula.** Demand is `Σ(usage + hist/P_FAC)/on-track nodes` per region, over every routed layer, using the
  constant `P_FAC`.
  - `run_pathfinder` does not return its final `p`, and a fixed divisor keeps epochs comparable.
  - After negotiation `usage ≤ 1` almost everywhere, so a usage-only map would never flag anything. The history term
    is what records contention that was negotiated away.
- **Hand-off.** Tell PLC that `> 1` means pressure, not literal overflow.

### Edits (dr)
- `#[derive(Clone, Debug, PartialEq, Eq)] pub struct LatticeSpec { pub p0: i32, pub strides: Vec<u32>, pub origin_multiple: i32 }`.
- `pub fn lattice_spec(cfg: &DetailedCfg) -> LatticeSpec`: `p0 = cfg.pitch`, strides from `cfg.layers` (empty →
  `[]`), `origin_multiple = p0·lcm(strides)` (`p0` when empty). It must use the same helper as `frame_origin`'s
  period (one `fn lattice_period(cfg) -> i32`).
- `fn congestion(hot: &RouteHot, grid: &TrackGrid, origin: (i32, i32)) -> Vec<(Rect, f32)>`:
  - Regions: 16×16 over the frame (`nx/16`, `ny/16` nodes, last region absorbs the remainder).
  - Counts only on-track nodes. Region rects are absolute (`+ origin`).
  - Called after `repair_constraints`, then stored in `RouteStats`.

### Tests (dr)
- `congestion_marks_the_crowded_region`: a `TrackGrid::with_layers` of 64×64×2 nodes. `usage = 1` on 10 nodes in
  region (5, 5) and on 1 node in region (12, 3), `hist` 0. The argmax region's rect contains region (5, 5)'s centre.
- `congestion_without_history_counts_usage`: random usage in {0, 1}, `hist` 0.
  `Σ demand·on_track(region) == Σ usage` to 1e-3.
- `elaborate.rs`, `sky130_lattice_spec`: `lattice_spec(&detailed_router(pdk, &routing_stack(pdk, None)?).cfg) == LatticeSpec { p0: 420, strides: vec![1, 1, 2, 2], origin_multiple: 840 }`.
- Acceptance: these three tests. PLC consumes them in PLC-15/28.
