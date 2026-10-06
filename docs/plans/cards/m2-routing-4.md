# M2+ routing segment 4: implementation card (RTE-23, M4)

Spec: plan-05 `### RTE-23`. DAG entry (`dag-m2-m6.json`, main checkout only): module routing, milestone M4, hard
dep RTE-12 (done, segment 1), no step-level notes, note "RTE-06 handled in M1" (m1-report: not needed, dac4 signs
off ERC 0). The plan's "Depends on RTE-06" is therefore void; the antenna case stays RTE-06's existing test
(`antenna_repair_jumps_to_a_higher_metal`, dr:3867). RTE-21 (M4, IR repair) and RTE-22 are not on this branch; that decides
step 2's DC exclusion below. Merge of `m2` at the start of this segment: one conflict in
`frontend/library/src/lib.rs` (`plate_sets` kept; `common_nodes` takes EXT-24's `intent.common_nodes` with RTE-17's
`groups`/`star` fields; EXT-24 `stars`/`kelvins` are still unmapped there, RTE-17 step 4 follow-up, out of scope).

| Item | Class |
|------|-------|
| RTE-23 | do (judgment) |

## RTE-23 Repair judged on the geometry that ships

### Current code (branch `m2-routing` after the merge; plan line numbers are stale)

All in `backend/dr/src/lib.rs` unless named.
- `probe` (closure) is `build_routes` + the cell/gate metal, no access, no arrays, no fill: dr:1110. It is moved into
  `repair_constraints` (dr:2348) whose `key` (dr:2358-2363) re-probes and rescores **every** batch of
  `reqs.hard ∪ reqs.budget ∪ extra` on every call; `trial` (dr:2492) and `commit_trial` (dr:2584) call `key` before
  and after, so each trial costs two full probes + two full rescores. Each round re-probes once more (dr:2369).
- After repair: shields (dr:1146-1153, mutate `cold` and trees), then the geometry loop (dr:1157-1490):
  `build_routes` → `add_pin_access` (dr:1162; signature dr:2047) → `break_shorts` → via arrays → same-net
  sliver/notch fill (dr:1306-1372, per net independently, origin shift folded into the same loop at dr:1368-1371) →
  EM rounds (`EM_ROUNDS`). Then RTE-20's `equalize_leads` stubs (dr:1495-1499), then `score` (dr:1501, via
  `gr::analog_tiers`, gr:136). Nothing after `score` changes `routes` inside dr, but no rule is re-checked after
  access/arrays/fill/stubs and no repair runs on them.
- `joins` is built at dr:1137 (after repair); `extra` builds its own copy at dr:1124.
- Exact pairs: `cold.mirror[ci] = Some((partner, map, leader))` (gr:735; `tie_pair` dr:1664); `map_rect` (dr:1985)
  carries a frame rect by a `gr::LatticeMap`. The fill treats the two nets independently, so a foreign shape near
  one side only gives the pair different fillers.
- `RuleBatch::touched` (kernel/analog/src/rule.rs:244) lists the nets a batch constrains, **not** the nets it reads:
  `CouplingBudget` touches only its victim (coupling.rs:223) but reads every aggressor; `Differential` reads all
  wires for skew (differential.rs:110); `Performance`/`ParasiticBudget` read coupling. Only `Antenna`
  (own shapes + cell metal + gates), `IrDrop` (own shapes) and `CommonNodes` (own shapes) read nothing but their
  touched nets. The plan's "recompute only batches whose touched ids intersect the rerouted nets" is therefore
  unsound as written, and doubly so once access is in the probe (step 1): `add_pin_access` picks a jog orientation
  by clearance to foreign routed metal, so rerouting net A can change net B's access shapes.
- Library (`frontend/library/src/lib.rs:1325-1349`): after the diode re-route, antenna diode marker shapes are pushed
  into `routes.wires` **after** dr scored them, so `route_report`'s antenna rows (and `lex_key`, `RunStats::route_hard`)
  describe geometry that does not ship, while `metadata::build` (lib.rs:1372) measures the shipped one. This is the
  acceptance gap ("dr's V/Θ equals what metadata measures").

### Decisions

- Step 2 cache is keyed on **changed shapes**, not rerouted nets: a batch keeps its cached score only when it is
  `local` (new trait flag, reads only its touched nets) and none of its touched nets' probe shapes differ from the
  cached probe. Everything else is rescored. Sound with access in the probe.
- DC exclusion: only `RepairKind::Em` batches leave the key (EM reads unknown on probes anyway: they carry no
  `terms`). `IrDrop` **stays** in the key until RTE-21 lands its IR repair: removing it now would make every
  `RepairKind::Ir` trial unacceptable (nothing else in the key moves) and orphan IR repair. RTE-21 removes it.
- Step 4 judges post-fill rounds on the drawn routes (a probe cannot see what fill/arrays/stubs did), keeps a round
  only when it improves `(Σ hard violations, Σ residual)` without raising overuse, else restores trees and the
  previous drawing; a rejected round ends the loop.
- Plan's risk-note "re-run the fill with that pair frozen" is skipped: a dropped filler pair can leave a sliver that
  signoff DRC reports (never silent). ponytail comment names it.

### Edits

1. `kernel/analog/src/rule.rs`: `Rule` gets `const LOCAL: bool = false;` (doc: "reads only the routes of the nets
   `touches` pushes; repair may keep a cached score while those nets' shapes are unchanged"). `RuleBatch` gets
   `fn local(&self) -> bool { false }`; `impl RuleBatch for Vec<R>` returns `R::LOCAL`; `Tagged` (rule.rs:303)
   forwards `self.inner.local()`. Set `const LOCAL: bool = true` in `routing/antenna.rs` and `routing/ir.rs`;
   `fn local(&self) -> bool { true }` in `routing/common_node.rs:136`.
2. dr, before the probe: move `let joins = joins(layers, cuts, cfg.pin_access);` up from dr:1137 to just before dr:1110
   and reuse it at dr:1124 and in the geometry loop. Probe becomes a two-argument closure (shields mutate `cold`
   later, so it must not capture it):
   ```rust
   let probe = |hot: &RouteHot, cold: &RouteCtx<TrackGrid>| {
       let mut r = Routes { cell: cell_f.clone(), gates: gates_f.clone(), ..build_routes(hot, cold, cfg, &compact, n_nets, layers, cuts) };
       add_pin_access(&mut r, &access, &compact, cfg, layers, cuts, &joins, &zones, origin, &jog_blocks);
       r
   };
   ```
   (frame coordinates, as today; access violations are ignored here, the shipped drawing reports them).
3. dr, step 2. New private items next to `repair_constraints`:
   ```rust
   /// The last accepted probe: its routes, per-batch `(violations, residual)`, and key `(Σ hard violations, Σ residual, overuse)`.
   struct Probed { routes: Routes, scores: Vec<(u32, f64)>, key: (u32, f64, f32) }

   /// Per batch `(violations, residual)` on `r`. With `prev` (an earlier probe and its scores), a
   /// [`RuleBatch::local`] batch none of whose touched nets' shapes differ between `prev` and `r` keeps
   /// its score; every other batch is rescored.
   fn rescore(batches: &[&dyn analog::RuleBatch<Routes>], r: &Routes, prev: Option<(&Routes, &[(u32, f64)])>) -> Vec<(u32, f64)>
   ```
   Changed nets = `n` with `prev.wires.get(n) != r.wires.get(n)`. `repair_constraints` builds
   `batches = reqs.hard ∪ reqs.budget ∪ extra` filtered `repair_kind() != RepairKind::Em`, remembering how many
   leading entries are hard (only those count toward `key.0`, as today). Its `probe` parameter becomes
   `probe: &impl Fn(&RouteHot, &RouteCtx<TrackGrid>) -> Routes`; a local `measure(hot, prev: Option<&Probed>) -> Probed`
   probes, `rescore`s and adds `overuse(hot)`. `let mut cur = measure(hot, None)` once; each round uses `cur.routes`
   instead of a fresh probe (dr:2369). `trial` and `commit_trial` take `cur: &mut Probed` and `measure` in place of
   `key`: `before = cur.key`, `after = measure(hot, Some(cur))`, accept rule unchanged, on accept `*cur = after`.
   Doc of `repair_constraints` updated (probe now includes access; EM out of the key; IR in until RTE-21).
4. dr, step 3 (fill). In the geometry loop's fill block (dr:1306-1372):
   - Split the origin shift (dr:1368-1371) into its own pass after all nets are filled.
   - Visit nets so every exact image (`cold.mirror[ci] == Some((_, _, false))`) comes after its leader.
   - Per net record `fillers[ni]` = shapes in `wires` after `heal_same_net_slivers`/`fill_same_net_notches` that were
     not there before (set difference, before `drop_contained`; do not assume the healer only appends).
   - For an image `b` of leader `a` (`cold.mirror[a] == Some((b, map, true))`): before `b`'s own heal/fill, each
     `f ∈ fillers[a]` maps to `Shape { layer: f.layer, rect: map_rect(f.rect, map, &cold.graph) }`; pushed to `b` when
     `rect_gap ≥ cfg.space(f.layer, 0, 0, floor)` (the same per-layer `space` as the fill) to every same-layer shape of
     `b`'s `foreign`, else dropped from **both** (remove `f` from `routes.wires[a]`) and `stats.pair_fillers_dropped += 1`.
     Then `b`'s own heal/fill runs as today (cell metal is not mirrored; DRC beats symmetry).
     `// ponytail: a dropped pair can leave a sliver on one side; signoff DRC reports it. Re-filling with the pair frozen if it shows up.`
   - `RouteStats` gains `pub pair_fillers_dropped: u32` (doc: RTE-23).
5. dr, step 4 (post-fill repair).
   - After the shields loop (dr:1153) and before the geometry loop: `let pre: Vec<u32> = { let r = probe(&hot, &cold); reqs.hard.iter().map(|b| b.violations(&r)).collect() };`
   - Wrap the geometry loop **and** the RTE-20 `equalize_leads` block (dr:1495-1499; clear `stats.plate_spread_pct`
     at the top of each pass) in `for post in 0..=POST_ROUNDS` (`const POST_ROUNDS: u32 = 2;`, doc: "Repair rounds
     on the drawn routes after fill (RTE-23)").
   - After each drawing: `newly` = hard batches `i` with `violations(&routes) > 0 && pre[i] == 0`. Empty, or
     `post == POST_ROUNDS` → keep this drawing, stop.
   - On a later pass, compare `drawn_key(&routes) = (Σ_hard violations, Σ_{hard∪budget} residual)` and `overuse(&hot)`
     with the previous drawing: worse or equal (or overuse up) → restore the snapshot (all trees via `cold.commit`,
     `cold.k`, `cold.term_k`, `cold.mirror`, `stats.pairs_exact/fallback`, the previous `(routes, sacrificed,
     access_v, plate_spread_pct)`) and stop.
   - Otherwise snapshot, then `stats.post_rounds += 1`; nets = `violating_ids(&routes)` of the `newly` batches → `ci`;
     each `cold.reroute(&hot, ci, 2.0 * P_FAC, &[], &mut dij)` + `commit`, then
     `run_pathfinder(&mut hot, &cold, P_FAC, HIST_INC, MAX_ITERS, &mut dij)`; next pass redraws.
   - `RouteStats` gains `pub post_rounds: u32`.
   - `score` stays last, on the kept drawing.
6. Library (acceptance gap): `frontend/library/src/lib.rs` after the marker push (lib.rs:1345-1349) re-derive dr's
   rule rows on the shipped routes:
   `route_report.hard_violations.retain(|v| !v.is_batch_row()); route_report.budget_violations.retain(|v| !v.is_batch_row());`
   then extend both from `gr::analog_tiers(&routes, routing)` (only when `marks` is non-empty). Check the
   `Violation::is_batch_row` (kernel/core/src/report.rs:47). `RunStats` gains
   `pub route_rows: Vec<(String, i64)>` (the winner's dr rows with `is_batch_row()`, hard then budget), filled in
   `epoch_score` (lib.rs:1636) and copied in `RunStats::merge` (lib.rs:1568).

### Tests

dr unit tests (`backend/dr/src/lib.rs`, `mod tests`), command
`PDK_ROOT=… cargo test -p dr` (crate name per `backend/dr/Cargo.toml`):
- `the_report_is_the_measurement_of_the_drawn_routes`: three runs with `test_cfg()` + `test_stack()`: a
  `CouplingBudget` victim beside an aggressor, a `Shield`, a `Differential` pair (reuse `pair_run`-style pins). For
  each, for every `i`: report has `"{BATCH}routing hard {i}"` iff `reqs.hard[i].violations(&routes) > 0`, with
  margin `Violation::from_residual(_, reqs.hard[i].residual(&routes)).margin`; same for `budget`.
- `a_rule_broken_by_pin_access_is_seen_by_repair` (step 1): pins centred between tracks (as
  `every_pin_is_touched_by_its_own_net`); a stand-in hard `Rule` satisfied iff no net-0 shape touches pin 0's rect.
  Assert `stats.post_rounds == 0` (the probe saw the jog, so it is not "newly" violated) and the row is in the report.
  Must be red on the parent commit with step 4 alone (`post_rounds == 1`); state that in the commit message.
- `a_rule_broken_after_fill_gets_a_post_round` (step 4): stand-in hard rule satisfied iff
  `routes.terms.iter().all(Vec::is_empty)` (probes carry no terms; the drawn routes do). Assert
  `stats.post_rounds == 1`, report has 1 row, and `routes.wires` equal a run without the rule (round rejected,
  restored). Control: the same rule always unsatisfied → `post_rounds == 0`.
- `rescore_reuses_only_unchanged_local_batches` (step 2): two stand-in rules counting `satisfied` calls in
  `AtomicU32`s, one `LOCAL = true` touching net 0, one non-local touching net 0. `r1`, `r2` differ only in net 1:
  `rescore(&b, &r2, Some((&r1, &s1))) == rescore(&b, &r2, None)`, local counter unchanged by the incremental call,
  non-local counter advanced. `r3` differs in net 0: local counter advances.
- `fill_on_a_pair_is_mirrored` (step 3): `pair_run`-style exact pair whose leader needs a same-net filler (two
  same-net pins one sub-spacing gap apart, as the dac4 notch test dr:~3994). Control without a ring: net-1 shape set
  == `map_rect` image of net-0's and `pair_fillers_dropped == 0`. With a foreign ring shape within `S` of the image
  filler site only: still shape-for-shape mirrored and `pair_fillers_dropped >= 1` (non-vacuous: fails if no filler
  arises).
- Every existing dr test stays green (`an_em_violation_spends_no_repair_trial`,
  `series_r_is_priced_only_past_the_drop_budget`, RTE-06/15/17/19/20 tests).

Acceptance (`benchmarks/tests/signoff_fixtures.rs`, new test next to
`antenna_in_loop_never_passes_what_signoff_fails`, same fixture loop and `#[ignore]` convention as its neighbours):
`routing_rows_are_measured_on_the_shipped_routes`: per fixture, `sol.stats.route_rows` equals the
`is_batch_row()` rows of `gr::analog_tiers(&sol.routes, &sol.routing)` (rule and margin). Plus the signoff baseline
(DRC/ERC/LVS unchanged or better) and ota/dac4 dr wall time printed before/after (step 2 must not make dr slower
than the parent; report the T13 timing protocol numbers from `m2-routing-1.md`).
Commands: `cargo test --release -p benchmark --test signoff_fixtures -- --ignored`, `cargo test -p dr`,
`cargo test -p analog`, `cargo test -p library`.

### Risks

- Access in the probe makes each probe dearer; step 2 and the cached `before` key (one probe per trial instead of
  two) pay for it. If ota dr p50 still rises, report the numbers; do not drop step 1.
- Step 4's reroute can move other nets via PathFinder; the snapshot restores every tree, not only the rerouted ones.
