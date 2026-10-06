# M2+ routing, cards 6 (M6: RTE-28)

Merge of `m2`: already up to date at bba01c0. DAG: RTE-28 hard dep RTE-10 (done, on m2), no step-level deps.
Commands: `export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk`;
`cargo test -p gr`, `cargo test -p dr`, `cargo test --release -p benchmarks --test routing_quality -- --include-ignored --nocapture`,
`cargo test --release -p benchmarks --test signoff_fixtures` (stays 19 passed, ota/ota_constrained/tt_ota DRC 0).

## RTE-28 Wrong-way jogs at a price: do

### Current code (plan is stale)
- Plan cites `gr:630–638`. Now `TrackGrid::neighbors` is backend/gr/src/lib.rs:428-451: off-track node → 0 edges;
  two along-track steps (cost 1.0, horizontal iff `layer % 2 == 0`), two vias (`self.via_cost`). Max 4 edges, `out` is
  `[(u32, f32); 6]` (lib.rs:168), so two more fit with no signature change.
- `via_cost` is dr's `VIA_COST = 4.0` (dr lib.rs:28, passed at dr:455). `2·VIA_COST = 8` > 2.5, as the plan says.
- Stride: frontend/library/src/elaborate.rs:539-543, `p0 = max(wire+space)` of the first two layers rounded up,
  `stride = ceil((wire+space)/p0)`. So stride 1 means `wire + space ≤ p0`: a one-pitch wrong-way stub between two
  adjacent rows is spacing-clean against metal on the neighbouring columns. Both stub ends are lattice nodes, so the
  existing node cost (usage, halo_wire/halo_via, reserved) already makes the step legal only when both nodes are free.
  No extra footprint check is needed for one-track connections.
- `lower_bound` (lib.rs:497-501) is `|Δix|+|Δiy| + via·|Δl|`: a 2.5 step over Δ=1 keeps it admissible and consistent.
- `LatticeMap::MirrorX`/`Shift` maps a wrong-way step onto a wrong-way step (same layer, same axis): RTE-15 images stay exact.
- Three places assume a same-layer run is straight. With jogs they break:
  1. `emit_run` (gr lib.rs:1541-1583): `straight = if !horiz { x == lx } else { y == ly }` compares only with the last
     node. Path A(0,0) B(1,0) C(1,1) D(2,1) emits A→B and then B→D, a diagonal wire that `to_shapes` (lib.rs:1589)
     turns into a full p0×p0 box. This is a DRC/short risk.
  2. `halo_nodes` (lib.rs:556-578): a same-layer run gets line-end halos `along` from its two ends, with direction
     `b > a`. On a jogged run the ends are on different tracks.
  3. dr `add_shield` (dr lib.rs:2671-2711) builds the parallel track per node. The jog node's "parallel" is the victim's
     own metal, which is not free, so the stretch splits. This is already correct. No edit.
- `route_net` expansion (gr lib.rs:1285-1300) is the only search caller of `neighbors`. The other caller is the test
  helper `tree_cost` at lib.rs:2002.
- There is no via count anywhere. `RouteStats` (dr lib.rs:259-290) has no `vias`.

### Edits
1. gr lib.rs, beside `KEEPOUT_COST` (lib.rs:763):
   `/// Base cost of a one-pitch step across a stride-1 layer's preferred direction (RTE-28, tuning default): below
   the 2·via_cost of a via detour (AT-02), above an along step so jogs stay rare.`
   `pub const WRONG_WAY_COST: f32 = 2.5;`
2. `TrackGrid::neighbors`: after the along pushes, when `self.stride(layer) == 1`, add the across pair. For a
   horizontal layer: `push(iy > 0, n.wrapping_sub(self.nx), WRONG_WAY_COST)` and
   `push(iy + 1 < self.ny, n + self.nx, WRONG_WAY_COST)`. For a vertical layer: `ix > 0` / `n - 1`,
   `ix + 1 < self.nx` / `n + 1`. Planar edges become ≤ 4 and the total ≤ 6. Update the `TrackGrid` struct doc
   (lib.rs:286-291) with one sentence: stride-1 layers also step across at `WRONG_WAY_COST`, and stride-2 layers do
   not (the next row is off-track).
3. `route_net` expansion loop (lib.rs:1286): a wide connection (`wide`, i.e. k>1 or guard>0) skips wrong-way edges.
   Its footprint spans tracks across, so a jog would overlap itself and `emit_run`'s k-track drawing is along-only.
   Add before `node_cost`:
   `let (pa, pb) = (g.pos(n), g.pos(nb)); if wide && pa.2 == pb.2 && (if pa.2 % 2 == 0 { pa.1 != pb.1 } else { pa.0 != pb.0 }) { continue; }`
   with the comment `// RTE-28: jogs are one-track only.`
4. `emit_run`: a step continues the segment only when it is collinear with the segment start:
   `let straight = (x == lx && x == sx) || (y == ly && y == sy);` (replaces the `horiz`-only test). Along-only runs
   are unchanged. Update the fn doc: "straight wires, along or across (a wrong-way jog is its own segment)".
   For `k > 1`, `push` offsets by `horiz`. Step 3 keeps wide runs jog-free.
5. `halo_nodes`: also end a run at a wrong-way step. The loop condition becomes "same layer and same across
   coordinate" (`ixy` y for even layers, x for odd), so each straight along piece gets its own end halos and the
   one-node stub gets none (its ends sit on nodes the adjacent-row metal already prices). Update the
   `// branch[start..i] is one same-layer run` comment to "one straight along-track run".
6. dr: `RouteStats` adds `/// Distinct vias drawn (RTE-28 reports the drop).` `pub vias: u32`. Set it where the
   final `stats` is filled after the last `build_routes` (dr ~1105/1133/1210; take the call whose routes are returned):
   count distinct `min(a, b)` over layer-changing steps of every `hot.trees` branch, using a `HashSet<u32>`. That
   matches `extract_geometry`'s `(x, y, layer)` dedup. The `RouteStats` doc list (dr:255-257) needs no edit.

### Tests
- gr `a_one_track_jog_stays_on_its_layer`: `TrackGrid::with_layers((10*200, 4*200), 200, 4.0, 2)`, one net
  `[g.node(0,1,0), g.node(9,2,0)]`, `RouteCtx::new` + `run_pathfinder` as in `pathfinder_converges_on_a_solvable_crossing`
  (lib.rs:1667). Assert `over == 0.0`, every node of `hot.trees[0]` has `g.ixy(n).2 == 0`, and exactly one step has
  `ixy.1` differing. Then `extract_geometry(&hot, &g, &[100, 100], &|_, _| [1; MAX_LAYERS], &[i32::MAX; 2])`:
  `vias.is_empty()`, and every wire has `x0 == x1 || y0 == y1`. This fails without edit 4. Before the change the
  same assertions fail (2 vias), so the test is red first.
- gr `wrong_way_steps_only_on_stride_one`: `TrackGrid::new((10*100, 10*100), 100, vec![spec(1,0), spec(1,0), spec(2,0)], 4.0)`.
  `neighbors(node(3,4,0))` contains `node(3,3,0)` and `node(3,5,0)` at `WRONG_WAY_COST`. `neighbors(node(3,4,2))`
  has no same-layer node with a different `iy`.
- gr `a_wide_connection_never_jogs`: same 2-layer grid as the first test, with `NetSearch`/`RouteCtx` k = 2 on layer 0
  (as in the existing wide test near lib.rs:1960-1972). The routed tree has no same-layer step that changes `iy`.
- gr `a_jogged_run_halos_each_straight_piece`: on a 1-layer `vec![spec(1,1)]` 10×10 grid (pitch 100), with tree
  `[[n(0,0), n(1,0), n(1,1), n(2,1)]]`, `halo_nodes` gives the pieces `[n(0,0),n(1,0)]` and `[n(1,1),n(2,1)]`.
  Assert the output contains `n(2,0)`, `n(0,1)` and `n(3,1)`. The old code treats it as one run `n(0,0)…n(2,1)` and
  yields only `n(3,1)`, so the test fails before edit 5.
- Existing: all of `cargo test -p gr` and `-p dr` pass unchanged (no threshold edits). If a dr test pins a via or
  wire count, re-derive it and say why in the commit. Do not loosen it.
- bench routing_quality `wrong_way_jogs_cut_vias` (`#[ignore = "full flow on ota; run with --include-ignored"]`):
  `run("ota")`, `println!("ota vias {}", sol.route_stats.vias)`, `assert!(sol.route_stats.vias < OTA_VIAS_BEFORE)`.
  The const `OTA_VIAS_BEFORE` is recorded by running this test with edit 6 only (before edits 1-5). Comment it with
  that commit. DRC 0 is `signoff_fixtures` (ota row max DRC 0, lib signoff_fixtures.rs:39).

### Acceptance
Via count on ota printed and below the pre-change count. `signoff_fixtures` 19 passed (DRC 0 on ota). If DRC rises,
say so with the rows. Do not raise a ceiling.

### Risks
Jogs cross congestion differently, so ota's PEX/crosstalk rows (RTE-18, already red) may shift: report the numbers.
Stride-2 layers get no jogs (per plan). Wide connections get none (edit 3, beyond plan; needed for edit 4's drawing).
