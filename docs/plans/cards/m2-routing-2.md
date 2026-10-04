# M2+ routing segment 2: implementation card (RTE-14, M3)

Spec: plan-05 `### RTE-14`; master §6.1 C18; gap-critic D8. DAG entry (`dag-m2-m6.json`, main checkout only): hard
dep RTE-12 only (done in segment 1). REL-01 and REL-03 are done (M1). Optional inputs that are not landed are REL-12
(front-row cuts), REL-17 (ESD floor) and RTE-22's `k`, all M4/M6, and none of them blocks this item. The step-level
note says step 7 orders by `net_weight` until RTE-21 step 4 (M4) supplies `F_k`. EXT-24 `em_critical` is gone (D8),
so the current filter is the only rule.

| item | class |
|---|---|
| RTE-14 | do (judgment) |

Line numbers are for `m2-routing` at the segment-1 merge (`9f56fb9`).

## RTE-14 Current-driven topology, EM-correct widths and pin access, wide nets first

### Current code (corrects the plan's stale cites)
- **Interim `k` per net** (RTE-12) is at dr:608-645. It uses `I_max` over `node_ua[ci]`, gives the same `k` to every
  layer of the net, applies the bundle rule `k ≥ ⌈em_width/wire⌉` at the wide threshold, and sets `guard` from the
  wide spacing. `RouteCtx.k`/`guard` are `Vec<[u8; MAX_LAYERS]>` per net (gr:632-633). `NetSearch.k`/`guard` are per
  net (gr:817-818).
- **Target order.** The order is computed at dr:587 (`gr::order_by_priority`, gr:91-112), *before* `k` exists. Its
  tiers are (shield ref, 0 sym | 1 hard | 2 budget | 3 free), then `Reverse(weight)`, then pin count. Targets are
  sorted by Manhattan distance from `terms[0]` at gr:931-936 (the plan cites gr:943-948).
- **Narrow jog.** It is computed in landing (dr:519, `narrow = full.min(r.w).min(r.h).max(floor)`) and in
  `add_pin_access` (dr:1346-1349). `full = cfg.wire_width` (the plan cites dr:384/1209).
- **Access cuts.** `add_pin_access` (dr:1394-1441) pushes one cut per end. The pin-access cut (mcon) is not in `cuts`,
  so the via-array pass (dr:827-1001) never touches it. A stack-layer access cut gets an array only where two runs
  overlap by ≥ 2 cuts.
- **EM Θ to delete.**
  - `branch_currents` (dr:1110-1150) and its use `edge_ua`/`edges_ua` (dr:760-766).
  - The `em_shortfall` loop (dr:790-825), which also trims merged runs back to their segment need.
  - `em_cuts` (dr:827, `need`/`short` at dr:838-843, 874, 897, 932) with its push at dr:1024-1026.
  - The `em_shortfall` parameter and the "em underwidth" push in `score` (dr:2011, 2033-2035).
- **Final assembly.** The final `routes.terms` (`pnr_core::Terminal { at, ua }`, absolute) are set at dr:1019-1022,
  after the origin shift at dr:1005-1009. The hard `Electromigration` per ≥ 2-terminal net is built in
  `frontend/library/src/lib.rs:1386-1391` from the same `em` that becomes `cfg.em` (lib.rs:504; pin-access layer and
  cut included, lib.rs:459-463).
- **EM check API.** `Electromigration::check` (em.rs:118-181) returns only the worst `(residual, ratio)`, so no API
  says which shape fails.

### Decisions
1. **Per-branch `k` is keyed by the terminal a branch reaches, not by a stored branch→k vector.** `route_net`
   branches end at their target (gr:986-998). So `k(branch) = term_k[net][index of branch.last() in terms[net]]`.
   A branch whose last node is no terminal (from `copy_tree`, `add_shield`, a trial) gets the net's `k`, the
   elementwise max over its `term_k`, which is conservative. Trees and every tree-editing helper stay unchanged.
2. **MST root.** dr has no port positions. The root is the terminal with the largest `|ua|`, and the edge current is
   `max(|S|, |total − S|)` with `S` the child-subtree sum. This is the bound `net_flow` uses for an unknown port
   (current.rs:20-24), and it equals plan step 2 when `total = 0`.
3. **Provenance by geometry, not an `origin: Vec<u16>`.** The via-array pass, fill and `drop_contained` reorder and
   drop shapes, so an index vector would have to be threaded through all three. Instead:
   - A failing metal shape on stack layer `li` maps to every branch of the net with a node on `li` whose absolute
     `(x, y)` lies inside the rect.
   - A failing cut on `cuts[i]` maps to branches with a node on `i` or `i + 1` inside the cut rect.
   - Shapes that map to no branch are access or cell metal, and stay REL-03 V.
4. **No trim back to need.** Widths are `k`-quantized (`W = wire + (k−1)·stride·p0`). The trim's current source is
   deleted with `branch_currents`. Two existing tests hold trimmed values and change to the quantized ones (Tests).
   This is a behaviour change, not a loosening: the assertion stays exact, and REL-03 now checks every segment.
5. **The access "other regions" fallback is skipped.** Each pin rect is already its own `Access` carrying its own
   share (`pin_ua` × `pin_share`, dr:318-331), so a region that cannot fit its cuts is reported.
6. **The ESD floor (REL-17), RTE-22's `k` and the REL-12 front row are absent.** They are not landed and are
   optional. Each slots into `tracks()` (below) as an extra `max`.

### Edits
**analog `kernel/analog/src/routing/em.rs`**
- Split `check` into `fn walk(self, r: &Routes, mut f: impl FnMut(usize, f32, f32, f32)) -> Option<()>`, which
  calls `f(shape index into r.shapes(net), need, have, ratio)` per metal shape and once per member cut of each via
  group.
- `check` folds over `walk`.
- Add `pub fn failing(self, r: &Routes) -> Option<Vec<usize>>`: the indices with `need > have` (`None` = unknown,
  as `check`). The doc says the indices are into `r.shapes(self.net)`.

**gr `backend/gr/src/lib.rs`**
- `NetSearch` gains `pub term_k: &'a [[u8; MAX_LAYERS]]`. When it is empty, every branch uses `k`. When it is not
  empty, it is parallel to `terms`, and the targets are routed **in the given order** (no distance sort) with
  target `i`'s search using `term_k[i]`.
  - In `route_net`, `node_cost` and `corner` take the active `k: &[u8; MAX_LAYERS]` as an argument instead of
    capturing `q.k`.
  - `wide` is computed per target.
  - `guard` stays per net.
- `RouteCtx` gains `pub term_k: Vec<Vec<[u8; MAX_LAYERS]>>` (empty = none) and
  `pub fn branch_k(&self, net: usize, branch: &[u32]) -> [u8; MAX_LAYERS]` (Decision 1).
  - `commit` uses `branch_k` per branch for `footprint` and `via_block`. The halo footprint uses the net's `k`.
  - `reroute` passes `term_k`.
- `extract_geometry(hot, grid, widths, k: &dyn Fn(usize, &[u32]) -> [u8; MAX_LAYERS], wide)`: `kl` is evaluated per
  branch.
- `order_by_priority(pins, net_ids, reqs, weight, wide: &[bool])`, where `wide[ci]` means the net has any `k > 1`.
  - New tier key: 0 sym, 1 wide, 2 other hard, 3 budget, 4 free. Shield references stay last through the existing
    leading bool. Within a tier the order is unchanged (`Reverse(weight)`, pins, id).
  - Update the doc's ponytail line to say `F_k` waits for RTE-21 step 4.
  - The callers are dr:587 and the gr tests.

**dr `backend/dr/src/lib.rs`**
- `const EM_ROUNDS: u32 = 3; // tuning default`.
- `fn tracks(cfg, l: usize, layer: LayerId, ua: f32) -> (u8, u8)`: dr:620-641's body moved verbatim (k, bundle rule,
  guard) for one current.
- `fn prim(pos: &[(i32, i32)], ua: &[f32]) -> (Vec<usize>, Vec<f32>)`: Prim over Manhattan distance, rooted at
  argmax `|ua|`.
  - It returns the visit order (root first) and, per terminal, the current on the edge into it (Decision 2; root
    gets 0).
  - Ties break by index, so the result is deterministic.
- Move the `k` computation before `RouteCtx` is built (`order` needs `wide`). Per compact net with a non-empty
  `node_ua[ci]`:
  - `ua` per `c_terms[ci]` node = the sum of its `node_ua` entries.
  - `I_max` as today. `I_min = min over l with J_l > 0 of J_l·wire_l/1000`.
  - Not critical (`I_max ≤ I_min`): `k = 1` everywhere, and no `term_k`.
  - Critical: `(order, I_e) = prim(...)`. Reorder `c_terms[ci]` by `order`. Set
    `term_k[ci][i][l] = tracks(cfg, l, layers[l], I_e[i]).0`, and set the root's entry to the elementwise max of
    the others.
  - `cold.k[ci]` = the elementwise max over `term_k`. `cold.guard[ci]` = `tracks(…, I_max).1`.
- Delete `branch_currents`, `edge_ua`/`edges_ua`, the `em_shortfall` loop, the `em_cuts` bookkeeping (the array
  placement itself stays), the `em_shortfall` parameter of `score`, and both Θ pushes (C18: REL-03 is the only EM
  measure).
- **Verify/repair (step 5).** Wrap dr:758-1022 (geometry, access, shorts, via arrays, fill, shift, terms) in
  `for round in 0..=EM_ROUNDS { … }`, and add the time to `stats.us_geometry`/`us_fill`.
  - After `routes.terms` is set, unless this is the last round and if `cfg.stack` is `Some`: for each compact net
    with `term_k`, build
    `Electromigration { net, limits: from cfg.em (as lib.rs:1364-1366), stack: cfg.stack }` and take
    `failing(&routes)`.
  - Map the failures (Decision 3). For each failing metal shape, bump `term_k[ci][branch terminal][li] += 1`; for
    each failing cut, bump both layers `i` and `i + 1`. Cap at `K_MAX`, and recompute `cold.k[ci]` as the max.
  - When nothing is bumped, stop.
  - Otherwise, for each bumped net, `if let Some(t) = cold.reroute(&hot, ci, P_FAC, &[], &mut dij) { cold.commit(&mut hot, ci, t) }`,
    then `run_pathfinder(&mut hot, &cold, P_FAC, HIST_INC, MAX_ITERS, &mut dij)` to clear new overflow, then redo
    the round.
  - `jog_hist` was already removed at dr:743, so it is not reapplied.
- **Access (step 6).** `Access` gains `ua: Option<f32>`, filled at dr:576.
  - In `add_pin_access` and in landing (dr:519), `need = ua.map_or(0, |u| ⌈|u|·1000/J_metal⌉ snapped up to 2·grid)`.
    Then `full = wire_width.max(need)` and `narrow = full.min(pin.w).min(pin.h).max(floor).max(need)`, so no
    candidate jog is narrower than its EM need (narrow survives only where it still meets EM). Landing's
    `jog_legs` clearance (dr:425, 513-533) uses the same widths.
  - Pin-end cut, when the cut has an EM limit and `ua` is `Some`:
    - `n = lim.cuts(|ua|)` cuts at pitch `size + cfg.space(cut, 0, 0, size)` (array spacing when `n ≥` its count),
      in one row along the pin's long axis, centred on the pin centre, each inside `pin` shrunk by the below
      enclosure.
    - The `above` pad grows to the array's bbox plus the above enclosure.
    - `n = 1` keeps today's path exactly.
    - If `n` does not fit, draw as many as fit and push hard
      `Violation { rule: format!("em access net {net} pin {x},{y}"), margin: missing as i64 }` (EM-19: the cell
      must widen the strap). `add_pin_access` returns these, and `score` appends them.

### Tests
**dr**, `cargo test -p dr` (PDK_ROOT set). The cfg values are sky130 met1: `pitch 420, wire_width 260, grid 5`, with
`Limit { ua_per_um: 2_800.0, ua_per_cut: 1e6, blech: 0.0 }` on both LAYERS and on CUTS[0].
- `a_trunk_carrying_two_branches_is_wider`.
  - Pins on net 0: feed `S` (1_000, 1_000) at −2_000 µA, `a` (9_000, 1_000) at +1_000 µA, `b` (9_000, 7_000) at
    +1_000 µA. Prim: S→a (8 000 < 14 000), a→b (6 000 < 14 000). Edge into a = 2 000 µA, into b = 1 000 µA.
  - `assert_eq!(trunks(0).iter().max(), Some(&1_100))`: 2 000 → need 714.3 nm → k = 1 + ⌈454.3/420⌉ = 3 →
    260 + 840.
  - `assert!(trunks(0).contains(&680))`: 1 000 → 357.1 nm → k = 2.
  - `assert!(em.satisfied(&routes))`, with `em = Electromigration { net: NetId(0), limits from cfg.em, stack: Some(test_stack()) }`.
- `access_jog_never_necks_below_em`.
  - `pin_access = Some((LayerId(3), (LayerId(4), 170, 170, 260)))`.
  - Two li pins 170×170 on LayerId(3) at ±1 000 µA, `em` on LayerId(3)/LAYERS at 2 800 µA/µm.
  - Assert that every net-0 shape on `LAYERS[0]` with `w != h` has `w.min(h) ≥ 360` (357.1 snapped up to 2·grid).
  - The same run with `pin_ua` empty keeps a jog `< 360` (it fails if `need` is applied without current).
- `access_cuts_follow_current`.
  - One li pin 170×1 000 at 800 µA and one at −800 µA. mcon limit `ua_per_cut: 360`, cut 170,
    `spacing: vec![(LayerId(4), 190, vec![])]`.
  - Assert exactly 3 net-0 shapes on LayerId(4) are contained in the first pin rect (⌈800/360⌉ = 3), and their
    pairwise `rect_gap ≥ 190`.
  - At ±1 300 µA (4 cuts need 4·170 + 3·190 = 1 250 > 1 000), assert `rules(&report)` contains a rule starting
    with `"em access net 0"`.
- `prim_tree_carries_the_far_side` replaces `branch_currents_sum_the_far_side` (deleted with `branch_currents`).
  - Root −3 500 at (0, 0), b 2 000 at (10, 0), c 1 500 at (20, 0) gives the chain order `[0, 1, 2]` with edge
    currents `[0, 3_500, 1_500]`.
  - With c at (0, 20) instead, b and c are both children of the root: `[_, 2_000, 1_500]`.
  - With b and c only (`[2_000, 1_500]`, total ≠ 0), the root is b and the edge into c is
    `max(1_500, 2_000) = 2_000`.
- `segments_are_sized_by_their_branch_current`, rewritten to the quantized widths (Decision 4).
  - Cfg as now (pitch 430, wire 290, J 1 000). S → a: 1 000 µA, k = 3 → `Some(&1_150)`. a → b: 400 µA, k = 2 →
    `contains(&720)`. Net 1: `Some(&290)`. `budget_violations` empty. The terms assertions are unchanged.
  - The starved-cut tail asserts `!Electromigration{…}.satisfied(&routes)` instead of `"em cuts net 0"`.
- `a_net_with_an_unknown_terminal_gets_no_em_sizing`: the known case expects `Some(1_150)` (k = 3), not 1 000. The
  unknown-equals-no-currents assertion is unchanged.
- `an_em_violation_spends_no_repair_trial` and the other existing tests stay as they are.

**gr**, `cargo test -p gr`.
- `wide_nets_route_before_other_hard_nets`.
  - A `Differential` on nets 0/1 in `reqs.hard`, plus a hard rule touching nets 2, 3 and 4.
  - `wide = [false, false, false, true, false]` gives order `[0, 1, 3, 2, 4]`.
  - With `wide` all false the order is `[0, 1, 2, 3, 4]`.
- `budgeted_nets_route_first` and `symmetric_nets_first_then_impact_shields_last` pass `&[]` for `wide`, with
  unchanged expectations.

**analog**, `cargo test -p analog em`.
- `failing_names_the_violating_shapes`: built like the existing em.rs tests. A two-terminal met1 net at 1 000 µA
  (need 357.1 nm) drawn as a 200 nm segment then a 400 nm segment. Assert `failing == Some(vec![index of the 200 nm
  shape])`. With one terminal `ua: None`, assert `None`.

### Acceptance (record in the segment report)
- Run `cargo test --release -p benchmark --test signoff_fixtures -- --include-ignored`. It must keep 19 passed with
  DRC/ERC/LVS clean.
- Run the bench and record, per circuit with an operating point, the Electromigration V count (target 0), the
  IrDrop Θ, and any `em access` V.
- IrDrop Θ = 0 depends on RTE-21's IR repair (M4), so report it as measured and do not tune for it here. Report any
  `em access` V as a CELL finding, with pin, current and fit.
- "No narrow jog on a current-carrying net" holds by construction (`narrow ≥ need`).
- Commit as `M2+ RTE-14: current-driven widths and access`.

### Risks
- Per-branch `k` raises footprints on supply trunks, and step 5 reroutes can add congestion. A net whose reroute
  fails keeps its old tree (`reroute` returns `None`), and the remaining violation is REL-03 V.
- The MST can differ from the routed tree. Step 5 closes that gap on drawn geometry, at most `EM_ROUNDS` times.
