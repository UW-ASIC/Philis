# M1 routing batch: implementation cards

Branch `m1a-routing`, worktree `philis-m1a/routing`, after `git merge m1a` (annotator batch merged, clean merge,
`125e27c`). Line numbers are of this tree; each item shifts the next one's lines, so find code by the symbol named
next to each line number. Every command needs `export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk`.

Order: RTE-05 → RTE-07 → RTE-08. RTE-05 does not depend on the other two; it goes first because it is the smallest
and it deletes `trim_pair`, which RTE-07's dr edits would otherwise have to work around. RTE-08 needs RTE-07: the
`Global` tier loses its only writer there, and `RouteStats::pf_iters` is introduced there.

Status (re-run after `git merge m1a`, fast-forward to `ae38f4f`): all three items are already implemented,
reviewed and merged into `m1a` (RTE-05 `7f8d92b`, RTE-07 `f61390f`, RTE-08 `a192c02`, review fixes `0c4f798`,
merge `f10e5fc`). The annotator merge since then touches only `backend/annotator` and docs, so no routing line
below is stale. Acceptance as recorded in those commit messages: RTE-08 ota `pf_iters` p50 1 -> 1, dac4
`unresolved congestion` 0/45 (already 0 on the RTE-07 baseline, so RTE-03 acceptance 3 is met but not
attributable to RTE-08), bench local DRC 0 x10 with LVS unchanged, FR-6 pwm_driver no `label short` in two runs.
Nothing left to implement in this batch.

| Item | Class |
|---|---|
| RTE-05 | mechanical |
| RTE-07 | mechanical |
| RTE-08 | judgment |

Batch rules: no new compiler warnings in `analog`, `annotator`, `gr`, `dr`, `library` (`cargo build` is clean
today). Tests below are new or edited exactly as stated; any other test that goes red is stop-and-report, never an
edited assertion.

---

## RTE-05 Differential: terminal-resolved RC, via count, coupling asymmetry, `known`; no stub trim

### Current code (corrections to plan-05 marked *)

- `kernel/analog/src/routing/differential.rs:26-35` `struct Differential { pos, neg, max_len_delta_pct10,
  same_layer_required, stack }`. `mismatch_pct` (:52-88): with a stack, `max(rel(ground_af), rel(runs R))` where runs
  = non-square shapes (:61); without one, the geometric signature (:64-87). `budget_pct` (:90). `Rule` impl
  (:95-113) has no `known`, so it uses the trait default `true` (`kernel/analog/src/rule.rs:86`).
- \* EXT-09 is merged: the only production constructor is `backend/annotator/src/extract.rs:91` (budget arm,
  `r.budget.push(Box::new(diff))` at :103). `gr::symmetric_nets` reads both arms (`backend/gr/src/lib.rs:275-281`), and
  so does dr's trim loop (`backend/dr/src/lib.rs:894-903`: `reqs.hard.iter().chain(&reqs.budget)`), contrary to the
  plan's "filters `reqs.hard` only".
- `trim_pair` is at `backend/dr/src/lib.rs:1029-1089` (doc from :1029); its only caller is the loop at :894-903; its
  only test is `trim_lengthens_the_lighter_side_of_a_pair` (dr:2834-2848). The local `cell_metal` (dr:727) stays used
  at dr:777 after the loop is deleted.
- Other `Differential { .. }` literals: `backend/gr/src/lib.rs:1387`, `backend/dr/src/lib.rs:2840, 2858`,
  `differential.rs:124`.
- `Routes::terminals(net) -> &[Terminal]` (`kernel/core/src/routes.rs:101`); `Terminal { at: Rect, ua: Option<f32> }`
  (routes.rs:47), re-exported as `pnr_core::Terminal`. dr fills `routes.terms` only on the final routes
  (dr:970-973, absolute, same frame as the wires). \* The repair probes (`probe`, dr:649) carry no `terms`, so during
  repair this rule falls back to the runs' R (step 2); do not add terms to `probe` (that would make `Electromigration`
  and `IrDrop` known inside repair, out of scope).
- `Stack::terminal_resistance_ohm(&self, shapes, terminals: &[Rect]) -> Vec<Option<f32>>` (stack.rs:167, star model
  from the net's centre), `Stack::ground_af` (:66), `Stack::resistance_ohm` (:87). `Stack.layers` is a pub
  `Vec<Layer>` with `pub cut: bool` (stack.rs:29); `Stack::at` is private, so read `st.layers` directly.
- `coupling.rs:25` private `pair_coupling_af(stack, layer, p, q)`; `CouplingBudget::total_af` (:73-95) is the only
  user. `CouplingBudget::aggressor_weight: Option<&'static [f32]>` (:47); the annotator builds the weights at
  `extract.rs:137-138` (`let weights: &'static [f32] = Box::leak(...)`), after the Differential loop.

### Decisions the plan left open (taken here)

1. Terminal R compares each side's R values **sorted ascending**, not terminals sorted by `(y, x)`. With a mirror
   about a vertical axis, `(y, x)` order pairs pos's leftmost terminal with neg's leftmost, which is the mirror image
   of pos's rightmost, so an exact mirror would read non-zero. Sorted values are order-free and equal for a mirror and
   for a translation.
2. Terminal R and via count apply only when both sides have the same, non-zero number of terminals (`paired`).
   Otherwise (ABBA cell: unequal pin sets; repair probes: no terminals) the terminal-R term is today's runs' R and the
   via-count term is 0, because unequal pin sets give unequal pad and cut counts by construction. Keeps
   `with_the_stack_unequal_pad_counts_match_when_rc_does` green as written.
3. `aggressor_weight` is wired in the annotator (otherwise the coupling term is never on in production): the same
   `weights` slice the `CouplingBudget`s use.
4. `trim_pair` and its test are deleted, not kept dead. RTE-20 recovers it from this commit's parent
   (`git show <parent>:backend/dr/src/lib.rs`).

### Edits

**`kernel/analog/src/routing/coupling.rs`**

1. `use pnr_core::geom::Rect;` → `use pnr_core::geom::{Rect, Shape};`.
2. After `pair_coupling_af` add:
   ```rust
   /// Lateral coupling between two nets, aF: Σ [`pair_coupling_af`] over every
   /// same-layer shape pair. The one net-to-net coupling measure: `Differential`
   /// and `CouplingBudget` read it (RTE-18 adds screening and crossings here).
   #[must_use]
   pub fn net_pair_af(stack: Option<&Stack>, a: &[Shape], b: &[Shape]) -> f32 {
       let mut c = 0.0;
       for p in a {
           for q in b.iter().filter(|q| q.layer == p.layer) {
               c += pair_coupling_af(stack, p.layer.0, &p.rect, &q.rect);
           }
       }
       c
   }
   ```
3. In `total_af`, replace the two inner `for a in victim { for b in shapes { .. } }` loops (:87-93) with
   `total += w * net_pair_af(self.stack, victim, shapes);`.

**`kernel/analog/src/routing/differential.rs`**

1. Add the field after `stack`:
   ```rust
   /// Aggressor weight in `[0, 1]` by `NetId` (as [`super::CouplingBudget`]); a
   /// missing index reads 1.0; `None` = no coupling-asymmetry term.
   pub aggressor_weight: Option<&'static [f32]>,
   ```
2. Replace the `if let Some(st) = self.stack { .. return ..; }` block (:53-63) with:
   ```rust
   if let Some(st) = self.stack {
       let (pa, pb) = (r.shapes(self.pos), r.shapes(self.neg));
       let rel = |a: f32, b: f32| {
           let mean = (a + b) / 2.0;
           if mean > 0.0 { (a - b).abs() / mean * 100.0 } else { 0.0 }
       };
       let (ca, cb) = (st.ground_af(pa), st.ground_af(pb));
       let (ta, tb) = (r.terminals(self.pos), r.terminals(self.neg));
       let paired = !ta.is_empty() && ta.len() == tb.len();
       // R of the runs alone: pads and cuts follow the pin set, and a
       // paralleled pin's pad summed in would read as series R.
       let runs = |s: &[Shape]| st.resistance_ohm(&s.iter().copied().filter(|q| q.rect.w != q.rect.h).collect::<Vec<_>>());
       let r_term = if paired {
           let ohm = |s: &[Shape], t: &[Terminal]| st.terminal_resistance_ohm(s, &t.iter().map(|t| t.at).collect::<Vec<_>>());
           let (ra, rb) = (ohm(pa, ta), ohm(pb, tb));
           if ra.iter().chain(&rb).any(Option::is_none) {
               100.0
           } else {
               let sorted = |v: Vec<Option<f32>>| {
                   let mut v: Vec<f32> = v.into_iter().flatten().collect();
                   v.sort_by(f32::total_cmp);
                   v
               };
               let (ra, rb) = (sorted(ra), sorted(rb));
               let mean = (ra.iter().sum::<f32>() + rb.iter().sum::<f32>()) / (ra.len() + rb.len()) as f32;
               let worst = ra.iter().zip(&rb).map(|(a, b)| (a - b).abs()).fold(0.0, f32::max);
               if mean > 0.0 { worst / mean * 100.0 } else { 0.0 }
           }
       } else {
           rel(runs(pa), runs(pb))
       };
       let vias = if paired {
           let cuts = |s: &[Shape]| {
               let mut m: BTreeMap<u16, f32> = BTreeMap::new();
               for q in s.iter().filter(|q| st.layers.iter().any(|l| l.id == q.layer.0 && l.cut)) {
                   *m.entry(q.layer.0).or_default() += 1.0;
               }
               m
           };
           let (na, nb) = (cuts(pa), cuts(pb));
           let get = |m: &BTreeMap<u16, f32>, l: &u16| m.get(l).copied().unwrap_or(0.0);
           let delta: f32 = na.keys().chain(nb.keys()).collect::<BTreeSet<_>>().into_iter().map(|l| (get(&na, l) - get(&nb, l)).abs()).sum();
           let mean = (na.values().sum::<f32>() + nb.values().sum::<f32>()) / 2.0;
           if mean > 0.0 { delta / mean * 100.0 } else { 0.0 }
       } else {
           0.0
       };
       let coupling = match self.aggressor_weight {
           Some(w) if ca + cb > 0.0 => {
               let skew: f32 = r.wires.iter().enumerate()
                   .filter(|&(a, _)| a != self.pos.0 as usize && a != self.neg.0 as usize)
                   .map(|(a, x)| {
                       let wa = w.get(a).copied().unwrap_or(1.0);
                       if wa == 0.0 { 0.0 } else { wa * (net_pair_af(Some(st), pa, x) - net_pair_af(Some(st), pb, x)).abs() }
                   })
                   .sum();
               skew / ((ca + cb) / 2.0) * 100.0
           }
           _ => 0.0,
       };
       return rel(ca, cb).max(r_term).max(vias).max(coupling);
   }
   ```
   Imports at the top: `use std::collections::{BTreeMap, BTreeSet};`, `use pnr_core::geom::Shape;`,
   `use pnr_core::Terminal;`, `use super::coupling::net_pair_af;`. The geometric branch below (:64-87) keeps its
   fully-qualified `std::collections::BTreeMap` paths or switches to the imports; either compiles.
3. In `impl Rule for Differential` add:
   ```rust
   /// Unknown until both sides have metal (AR-44): an unrouted side is not a match.
   fn known(self, r: &Routes) -> bool {
       !r.shapes(self.pos).is_empty() && !r.shapes(self.neg).is_empty()
   }
   ```
4. Rewrite the type doc (:18-25): the stack paragraph becomes "With the deck's stack it compares what each side
   presents: ground C; series R from the net's centre to each terminal (sorted values, so a mirror image matches);
   the via count per cut layer; and, with `aggressor_weight`, the weighted coupling each side takes from every other
   net. The result is the worst term. Sides with unequal terminal counts (an ABBA cell) compare the runs' R and no via
   count." The `ponytail:` line becomes: `/// ponytail: star-model terminal R (no tree solve), lateral coupling only
   (no crossings, no screening; RTE-18); no `project` — `dr` fixes it by mirrored rip-up.`
5. Test helper `pair()` (:124): add `aggressor_weight: None`.

**`backend/annotator/src/extract.rs`**: move the two lines `let weights: &'static [f32] =
Box::leak(CouplingBudget::default_weights(classes, hg.net_names.len()).into_boxed_slice());` (:137-138, with their
3-line comment) up to just before `let mut diff: Vec<Differential> = Vec::new();` (:86), and add
`aggressor_weight: Some(weights),` to the `Differential { .. }` literal at :91. The `CouplingBudget` at :141 keeps
using the same binding.

**`backend/gr/src/lib.rs:1387`, `backend/dr/src/lib.rs:2858`**: add `aggressor_weight: None` to the literals.

**`backend/dr/src/lib.rs`**: delete the trim loop with its comment (:894-903), `trim_pair` with its doc (:1029-1089)
and the test `trim_lengthens_the_lighter_side_of_a_pair` with its doc (:2834-2848, which holds the literal at :2840).

### Tests (`kernel/analog/src/routing/differential.rs`, `mod tests`)

Shared helper in `mod tests`:
```rust
/// m1 (id 1), via (2, cut), m2 (3): the stack.rs test helper's values.
fn rc_stack() -> &'static Stack {
    use crate::routing::stack::Layer;
    let metal = |id| Layer { id, area_af_um2: 25.0, fringe_af_um: 40.0, lateral: 3.9 * 8.854 * 360.0, sheet_ohm: 0.125, ..Layer::default() };
    Box::leak(Box::new(Stack { layers: vec![metal(1), Layer { id: 2, sheet_ohm: 4.5, cut: true, ..Layer::default() }, metal(3)], antenna_cumulative: false, diode: None }))
}
fn wire(x: i32, y: i32, w: i32, h: i32) -> Shape { Shape { layer: LayerId(1), rect: Rect { x, y, w, h } } }
fn term(x: i32, y: i32) -> pnr_core::Terminal { pnr_core::Terminal { at: Rect { x, y, w: 170, h: 170 }, ua: None } }
```
`rc = Differential { stack: Some(rc_stack()), ..pair() }` in each test.

- `a_stub_does_not_match_terminal_resistance`: pos `[wire(0, 0, 30_000, 290)]`, terms `[term(0, 60), term(29_830,
  60)]`; neg `[wire(0, 5_000, 20_000, 290), wire(10_000, 5_000, 290, 10_000)]` (a 10 µm dead-end stub up from the
  trunk; summed length 30 µm both sides), terms `[term(0, 5_060), term(19_830, 5_060)]`. Assert
  `rc.mismatch_pct(&r) > 5.0` and `!rc.satisfied(&r)`.
- `an_exact_mirror_is_zero`: pos `[wire(0, 0, 30_000, 290)]`, terms `[term(0, 60), term(8_000, 60), term(29_830,
  60)]`; neg = the mirror about x = 40 000: `[wire(50_000, 0, 30_000, 290)]`, terms `[term(79_830, 60),
  term(71_830, 60), term(50_000, 60)]` (the nets are 20 µm apart, so they do not couple through `aggressor_weight`,
  which stays `None`). Assert `rc.mismatch_pct(&r) <= 1e-4` and `rc.residual(&r) == 0.0`. (With `(y, x)` pairing
  this reads non-zero: it is the test for decision 1.)
- `a_one_sided_aggressor_breaks_the_pair`: pos `[wire(0, 0, 10_000, 290)]` terms `[term(0, 60), term(9_830, 60)]`;
  neg `[wire(0, -20_000, 10_000, 290)]` terms `[term(0, -19_940), term(9_830, -19_940)]`; net 2
  `[wire(0, 570, 10_000, 290)]` (280 nm above pos), no terms. With
  `aggressor_weight: Some(&[1.0, 1.0, 1.0])`: `rc.residual(&r) > 0.0`. With `Some(&[1.0, 1.0, 0.0])`:
  `rc.mismatch_pct(&r) == 0.0`. (Expected size: ≈ 444 aF against ≈ 895 aF ground C, ≈ 50 %.)
- `an_unrouted_side_is_unknown`: `Routes { wires: vec![vec![wire(0, 0, 10_000, 290)], vec![]], ..}` →
  `!rc.known(&r)`; with both sides routed → `rc.known(&r)`.
- Existing `equal_length_on_the_same_layers_can_still_mismatch` and
  `with_the_stack_unequal_pad_counts_match_when_rc_does` stay unchanged and green.

Commands:
```
cargo test -p analog routing::
cargo test -p annotator
cargo test -p gr
cargo test -p dr
```
Acceptance: the four tests above green, every existing test in those four crates green. The `ota` residual reaching
0 waits for RTE-15 (not this batch).

---

## RTE-07 Retire the coarse `GlobalRoute`; frame, corridors and group pricing

### Current code (corrections to plan-05 marked *)

`backend/gr/src/lib.rs` (1393 lines):
- `GlobalCfg` :21-35; `Negotiation` :37-46 + impl :61-92 (stays; RTE-08 rekeys it); `Tier` :48-53 (stays until
  RTE-08); `GroupPrice` :94-103; `price_group` :105-133; `group_bbox` :135-151; `GlobalRoute` :153-225 (`route`
  :164-224); `gcell_terms` :227-239; `keepaway_partners` :283-301; `keep_apart` :303-338; `order_by_pin_count`
  :340-345; `charge_rect` :347-358; `die_extent` :360-368; `seg_shape` :370-376; `score` :397-406; `build_nets`
  :408-427; `RGraph::region` :444-445; `GcellGrid` :455-512 (struct, impl, `impl RGraph`); `TrackGrid::region_of`
  field :521, its init in `with_layers` :531, `set_regions` :534-541, `fn region` in `impl RGraph for TrackGrid`
  :645-647; `RouteCtx::corridors` :731 (doc :725-726, init :766); `RouteCtx::reroute` `corridor` parameter
  :783-814; `route_net` `corridor` parameter :888 (doc :879, check :978-980); `CORRIDOR_EPOCHS` :1012-1013 and its
  use in `run_pathfinder` :1032.
- Kept (unchanged): `pub use pnr_core::place_macros`, `Negotiation`, `RGraph`, `TrackGrid`, `RouteHot`, `RouteCtx`,
  `Dij`, `Elec`, `route_net`, `run_pathfinder`, `bump_history`, `extract_geometry`, `to_shapes`, `Wire`, `Via`,
  `order_by_priority`, `symmetric_nets`, `analog_tiers`, `KEEPOUT_COST`, `NONE`.

`backend/dr/src/lib.rs`:
- \* `use gr::{.., GcellGrid, ..}` :24; `GCELLS_PER_SIDE` with its false comment :33-34; `pub fn route` wrapper
  :197-220 and private `route_counted` :222-238 (returns `(Routes, Report, u32)`); `n_nets` from
  `global.wires.len()` :249; frame `low` closure :258-261 and `hi` loop :306-310 read `global.wires`; `ggrid` and
  `set_regions` :330-332; `corridors` vec :340; corridor construction :543-561; `corridors` into `RouteCtx` :571;
  `trial`'s doc "(corridor first)" :1620 and `cold.reroute(hot, n, true, ..)` :1640; `add_shield`'s
  `cold.reroute(hot, reference, false, ..)` :1709; module doc :4-5 ("early epochs restricted to the corridor").
- \* The plan's "fix the duplicated doc line (dr:1028)" is stale: no doc line is duplicated in dr or gr today. Skip.
- \* dr's frame already allows negative coordinates (`low(..).min(0)`, dr:260); the negative-coordinate defect was
  gr's (`die_extent` from the origin). Keep `.min(0)`: moving the frame origin is RTE-10.
- dr tests: `route` helper :2280-2290 takes `global`; direct calls :2655, :2692 (`route_counted`), :2735, :2860,
  :2978; every test builds a `global`.

Outside:
- `frontend/library/src/lib.rs:806-811` (gr call, `global.debug_check_joined("gr::route", ..)`), dr calls :827-829
  and :845; `Epoch` :724-742 (no `Report` field; it keeps `key` and `stats`).
- `frontend/library/src/elaborate.rs:178-189` (gr call :179, dr call :180-189); `detailed_router` :392.
- `frontend/library/src/cellgen.rs:318-338` `seed_assignment(variants, layers, pdk)` (builds `gr::GlobalCfg` :323),
  `price` :340-366 (calls `gr::price_group` :351, returns `(bool, usize, i64, i64)`); test
  `seed_assignment_is_deterministic` :1730-1736; caller `frontend/library/src/lib.rs:492`.
- \* The stale message is at `frontend/library/src/geometry.rs:136` (not :87): "(see gr::build_nets' obstacle-only
  path)".

### Decisions the plan left open (taken here)

1. `RouteStats` is not stored in `Epoch`: nothing reads it yet, and an unread private field is a `dead_code`
   warning. The library binds it as `_route_stats`; FLOW-09 adds the `Epoch` field together with its reader.
2. Of gr's tests the plan deletes, the two that test `score` through kept code are retargeted, not deleted:
   `analog_tiers` stays and they are its only tests. `price_group_measures_the_group_not_the_die` becomes the
   `group_hpwl` test. `a_keepaway_rule_parts_two_nets_during_search` loses its `GlobalRoute` half and keeps its
   `order_by_priority` asserts.
3. `seed_assignment` loses its `layers` parameter (unused once `price_group` goes).

### Edits

**`backend/gr/src/lib.rs`**

1. Delete everything listed under "Current code" except `Tier` (RTE-08) and `Negotiation`.
2. Add, where `price_group` was:
   ```rust
   /// Σ over nets of the half-perimeter of their pin centres, nm (a net
   /// whose pins share one centre adds 0): the wirelength half of pricing one
   /// cell's variant in isolation.
   #[must_use]
   pub fn group_hpwl(macros: &[Macro]) -> i64 {
       let mut span: BTreeMap<u16, (i32, i32, i32, i32)> = BTreeMap::new();
       for p in macros.iter().flat_map(|m| &m.pins) {
           let (x, y) = (p.at.x + p.at.w / 2, p.at.y + p.at.h / 2);
           let e = span.entry(p.net.0).or_insert((x, x, y, y));
           *e = (e.0.min(x), e.1.max(x), e.2.min(y), e.3.max(y));
       }
       span.values().map(|&(x0, x1, y0, y1)| i64::from(x1 - x0) + i64::from(y1 - y0)).sum()
   }
   ```
3. `RouteCtx::reroute(&self, hot, net, p_fac, penalty, dij)`: drop `corridor`, the `corr` binding and the
   `search`/`or_else` fallback; call `route_net(g, &hot.usage, &hot.hist, &old, t, net as u32, r, penalty,
   (&self.keepout, own), p_fac, &elec, dij)` directly. Doc: "Route `net` against the current state with an extra
   per-node `penalty` (empty = none)."
4. `route_net`: drop the `corridor: &[u32]` parameter and the `if !corridor.is_empty() && ..` check; doc line :879
   becomes "`None` if a target is unreachable without entering a node `reserved` for another net."
5. `run_pathfinder`: `cold.reroute(hot, net, p_fac, &[], &mut dij)`; the loop variable becomes `_`.
6. `RouteCtx` doc :725-726: "Immutable per-run context. `reserved[node]` is a hard owner (`NONE` = free)."; `new`'s
   doc "No reservations.".
7. Module doc (:1-9) becomes: "# `gr` — the routing mechanics `dr` runs: the track lattice ([`TrackGrid`]), the
   search ([`route_net`], [`run_pathfinder`]), negotiation history ([`Negotiation`]) and geometry extraction
   ([`extract_geometry`]). The coarse gcell router is retired (RTE-07): its plan was discarded and only seeded
   corridors."
8. Remove the imports the compiler then reports unused (expected: `Layout`, `Report`).
9. Tests: delete `two_pin_net_routes_coarsely`, `negotiation_persists_across_calls` (gr's; dr's stays),
   `overflow_theta_is_a_capacity_residual_not_a_count`, the `lay()` helper and `LAYERS`. Retarget:
   - `budget_residual_reaches_theta`: `assert!(analog_tiers(&wire(900), &reqs).1.is_empty());`
     `assert_eq!(analog_tiers(&wire(1_500), &reqs).1[0].margin, 500);` (the `feasible()` line asserted gr's deleted
     `Report`, it goes with it).
   - `hard_batch_margin_is_measured_not_counted`: `margin_of` returns `analog_tiers(&wire(3_000), &reqs).0[0].margin`;
     the `(500, 2_000)` assert unchanged.
   - `price_group_measures_the_group_not_the_die` → `group_hpwl_measures_the_group_not_the_die`:
     `assert_eq!(group_hpwl(&[far(100_000, 100_000), far(108_000, 106_000)]), 14_000);`
   - `a_keepaway_rule_parts_two_nets_during_search` → `budgeted_nets_route_first`: keep only the `reqs` with the
     `CrosstalkExclusion` and the two `order_by_priority` asserts (lines :1316-1318).
   - `a_sensitive_net_steers_off_a_coupled_track`, `a_current_carrying_net_avoids_via_resistance`: drop the `&[]`
     corridor argument from `route_net(..)` and the `false` from `cold.reroute(..)`.

**`backend/dr/src/lib.rs`**

1. `use gr::{..}`: remove `GcellGrid`. Delete `GCELLS_PER_SIDE` (:33-34).
2. Add to `DetailedCfg` (after `stack`) and to its `Default` (`n_nets: 0`):
   ```rust
   /// Nets in the netlist: `Routes` is sized to at least this, so a rule on a
   /// net with no pin reads empty shapes. `0` = the pins' highest net + 1.
   pub n_nets: usize,
   ```
3. Add after `DetailedCfg`'s impl:
   ```rust
   /// What one `route` call measured. Fields an item has not landed yet stay
   /// zero: timings and `expanded` (RTE-13), `pf_iters` (RTE-08), `coarsened`
   /// (RTE-13), `width_fallbacks` (RTE-22), `single_cut_vias` (RTE-27),
   /// `congestion` (RTE-25, absolute nm).
   #[derive(Default, Clone, Debug)]
   pub struct RouteStats {
       pub us_landing: u64, pub us_negotiate: u64, pub us_repair: u64, pub us_geometry: u64, pub us_fill: u64,
       pub expanded: u64,
       /// Constraint-repair trials run.
       pub trials: u32,
       pub pf_iters: u32,
       /// Residual track overuse after negotiation, repair and shields.
       pub overuse: f32,
       pub coarsened: bool,
       pub width_fallbacks: u32,
       pub single_cut_vias: u32,
       pub congestion: Vec<(Rect, f32)>,
   }
   ```
4. Delete the `route` wrapper (:197-220). Rename `route_counted` to `pub fn route`, move the wrapper's doc onto it
   with its first line changed to "Route `pins` (plus the pins of `placed` and `rings`) over the metal stack
   `layers` (even index = horizontal), joined by `cuts[i]` between `layers[i]` and `layers[i + 1]`." Signature:
   ```rust
   pub fn route(&self, pins: &[(NetId, Rect, LayerId)], placed: &[Macro], rings: &[Macro],
                reqs: &Requirements<Routes>, layers: &[LayerId], cuts: &[Cut],
                neg: &mut gr::Negotiation) -> (Routes, Report, RouteStats)
   ```
   Delete the ponytail doc of `route_counted`.
5. `n_nets` (:249): `let n_nets = cfg.n_nets.max(all_pins.iter().map(|(n, ..)| n.0 as usize + 1).max().unwrap_or(0));`
6. Frame: in `low` (:258-261) replace `global.wires.iter().flatten().map(|s| f(&s.rect))` with
   `placed.iter().chain(rings).map(|m| f(&m.bbox))`; the hi loop (:306-310) becomes
   `for m in placed.iter().chain(rings) { hi_x = hi_x.max(m.bbox.x + m.bbox.w); hi_y = hi_y.max(m.bbox.y + m.bbox.h); }`
   (the `(hi_x, hi_y) = (hi_x - origin.0 + margin, ..)` line after it stays). Update the frame comment: "Routing
   frame over pins and placed/ring bboxes".
7. Delete `ggrid` (:330), `grid.set_regions(&ggrid)` (:332, so `grid` no longer needs `mut` if the compiler says so),
   the `corridors` vec (:340), the corridor block (:543-561) and `corridors,` in the `RouteCtx` literal (:571).
8. Early return (:319-322): `return (routes, report, RouteStats::default());`. Final return (:978):
   `(routes, report, RouteStats { trials, overuse, ..RouteStats::default() })` (`overuse` is the local computed at
   :693).
9. `trial` (:1640): `cold.reroute(hot, n, p_fac, &field, &mut dij)`; doc :1620 drop "(corridor first)".
   `add_shield` (:1709): `cold.reroute(hot, reference, P_FAC, &[], &mut dij)`.
10. Module doc :3-5: "...lands one track node per placed pin, runs `gr`'s negotiated-congestion PathFinder, then
    draws the geometry: ...".
11. Tests: the `route` helper drops its `global` parameter and binds `let (routes, report, _) = DetailedRoute { cfg
    }.route(pins, placed, rings, &reqs, &LAYERS, &CUTS, neg); (routes, report)`. In every test delete the `let global
    = ..` binding and the `&global` argument; the direct calls (:2655, :2735, :2860, :2978) take `.0`/`.1` from the
    triple as before. Where a test's `global.wires.len()` exceeded its highest pin net + 1, set `n_nets` to that
    length in its cfg: `empty_coarse_gives_empty_routes` (3 nets, no pins) becomes `no_pins_give_empty_routes` with
    `DetailedCfg { n_nets: 3, ..test_cfg() }` and the same asserts. Test :2684-2700: `let (routes, report, stats) =
    dr.route(..); ..; stats.trials`.
12. New test:
    ```rust
    /// The frame follows the pins wherever they are, negative coordinates included.
    #[test]
    fn negative_coordinate_pins_route() {
        let pins = [pin(0, -20_000, -5_000), pin(0, 5_000, 3_000)];
        let (routes, report) = route(test_cfg(), &pins, &[], &[], &mut gr::Negotiation::new());
        assert!(!routes.wires[0].is_empty());
        assert!(report.hard_violations.is_empty(), "{:?}", rules(&report));
    }
    ```

**`frontend/library/src/lib.rs`**

- Delete :806-811 (the comment becomes "// Route: track realisation onto the real pins.", the gr call, the gr
  `debug_check_joined` and its comment).
- `dr::DetailedCfg { .. }` (:820-826): add `n_nets: self.netlist.nets.len(),`.
- :827-829: `let (mut routes, mut route_report, _route_stats) = router.route(&pins, &placed, &rings, routing, layers,
  &self.cuts, neg);` :845: `(routes, route_report, _) = router.route(&pins, &placed, &rings, routing, layers,
  &self.cuts, neg);`
- :492: `cellgen::seed_assignment(&flow.cells.variants, pdk)`.

**`frontend/library/src/elaborate.rs`**: `let mut d_router = detailed_router(..)` (:160) followed by
`d_router.cfg.n_nets = built.netlist.as_ref().map_or(0, |n| n.nets.len());`; in the epoch loop delete the gr call
(:178-179) and bind `let (routes, report, _) = d_router.route(&pins, &placed, &[], &reqs, &layers, &cuts, &mut
neg);`.

**`frontend/library/src/cellgen.rs`**: `seed_assignment(variants, pdk)` (drop `layers` and the `cfg` line :323;
`price(m, &mut checker)`); `fn price(m: &Macro, checker: &mut Checker) -> (usize, i64)` returning `(geom,
gr::group_hpwl(std::slice::from_ref(m)))`; its doc: "`(DRC+ERC findings, pin HPWL)`, compared lexicographically.
DRC runs density-stripped ...; a macro the engine cannot load prices as maximally illegal." (the unreachable and
congestion terms went with the coarse router). Test :1735-1736: drop `&layers` (and the `layers` binding).

**`frontend/library/src/geometry.rs:136`**: the message ends "— it was dropped, not routed", without the
`gr::build_nets` parenthetical.

### Tests and acceptance

```
cargo test -p gr
cargo test -p dr
cargo test -p library
grep -rn "GlobalRoute\|GlobalCfg\|GcellGrid\|price_group\|corridor" --include='*.rs' backend frontend kernel benchmarks
```
- The grep returns nothing except `kernel/cells/src/cap_array.rs:38` (an unrelated "corridors of Ci" in a doc).
- Bench: before the change, on the merge commit, run `cargo run --release -p benchmark --bin bench local` (all 10,
  background, `PNR_BENCH_SEED` unset) and keep each circuit's DRC, LVS and ms; after the change, the same command.
  Acceptance: 10/10 DRC 0 and every LVS column the same or better; report per-circuit ms before/after. Then
  `cargo test --release -p benchmark --test signoff_fixtures` green as before the change (dac4's row is whatever the
  merge commit reads; this item must not change it).
- Expected risks, not loosenings: dr loses the 8-iteration corridor restriction and `seed_assignment` ranks variants
  by `(DRC+ERC, HPWL)` only, so placements and routes move. A DRC or LVS regression is stop-and-report with the
  circuit and rows, not a tuning change.

---

## RTE-08 Negotiation keys and PathFinder convergence (+ RTE-03 acceptance 3, FR-6)

### Current code (corrections to plan-05 marked *)

- `Negotiation { hist: BTreeMap<(Tier, u32, i32, i32), f32> }` `backend/gr/src/lib.rs:43-46`, `#[derive(Default)]`
  only (no `Clone`); `Tier` :48-53; `HIST_QUANTUM_NM = 500` :55; `hist_key(tier, pos)` :57-59; `seed(&self, tier,
  hist, pos)` :74-81; `accumulate(&mut self, tier, hist, pos)` :84-91; `pressure()` :69-71.
- \* Keys are already absolute: dr passes `abs = pos + origin` (dr:612-616, `let abs`). So the key change is only the removal
  of `Tier`; `history_keys_ignore_the_frame` characterises the current behaviour and must stay green.
- `run_pathfinder(hot, cold, p_fac, hist_inc, max_iters) -> f32` gr:1019-1043 (constant `p_fac`; stops on zero
  overflow, a no-move iteration, or `max_iters`). After RTE-07 its only caller is dr:619 (`P_FAC = 2.0`, `HIST_INC =
  0.5`, `MAX_ITERS = 150`, dr:30-32), which discards the result (dr measures `overuse(&hot)` itself at :693, after
  repair and shields).
- `Tier` users after RTE-07: dr:617 `neg.seed(gr::Tier::Detailed, ..)`, dr:680 `neg.accumulate(gr::Tier::Detailed,
  ..)`.
- `unresolved congestion` V: dr `score` :1965-1967 (`margin = overuse as i64`).
- dr `negotiation_persists_across_calls` (:2458-2484) asserts the jammed 5-net instance *still* reports
  `unresolved congestion` after the first call (:2477-2478).
- FR-6 netlist: not in the repo. The field-report pwm_driver (34 FETs, `.subckt pwm_driver inp inn phi1 phi1e outa
  outb vdd vss`) is at `/home/omare/Documents/Projects/Trial/ResearchBoutros/analog/pwm_driver/netlist/pwm_driver.spice`.
  \* `benchmarks/fixtures/` is auto-discovered by `bench local` (`benchmarks/src/fixtures.rs:108`), so putting it
  there turns the "10/10" bench into 11 circuits with a ~6 min one; it goes to `benchmarks/field/pwm_driver.spice`
  instead.

### Edits (`backend/gr/src/lib.rs`, then dr)

1. `Negotiation`: `#[derive(Default, Clone)] pub struct Negotiation { hist: BTreeMap<(u32, i32, i32), f32> }`
   (Clone is for FLOW-08's snapshot). Delete `Tier`. `hist_key((x, y, layer)) -> (u32, i32, i32)` returns `(layer,
   x.div_euclid(HIST_QUANTUM_NM), y.div_euclid(HIST_QUANTUM_NM))`. `seed(&self, hist: &mut [f32], pos: impl Fn(u32)
   -> (i32, i32, u32))`, `accumulate(&mut self, hist: &[f32], pos: ..)`: drop the `tier` parameter, bodies otherwise
   unchanged. Struct doc: keep, and add "Quantum `HIST_QUANTUM_NM` until RTE-10 keys on the lattice pitch."
2. Constants above `run_pathfinder`:
   ```rust
   /// Present-congestion factor growth per PathFinder iteration, and its cap
   /// (tuning defaults; the cap keeps f32 costs finite).
   const P_GROWTH: f32 = 1.3;
   const P_FAC_MAX: f32 = 1000.0;
   /// Iterations without an overflow decrease before negotiation gives up
   /// (tuning default): an unsolvable instance stops here, not at `max_iters`.
   const STALL_ITERS: u32 = 10;
   ```
3. `run_pathfinder(..) -> (f32, u32)`:
   ```rust
   let cap = cold.graph.cap();
   let mut dij = Dij::new(cold.graph.nodes());
   let (mut overflow, mut best, mut stall, mut iters, mut p) = (0.0, f32::INFINITY, 0, 0, p_fac.min(P_FAC_MAX));
   for _ in 0..max_iters {
       iters += 1;
       let mut moved = false;
       /* the existing per-net loop, with `cold.reroute(hot, net, p, &[], &mut dij)` */
       overflow = bump_history(&hot.usage, &mut hot.hist, cap, hist_inc);
       if overflow < best { (best, stall) = (overflow, 0) } else { stall += 1 }
       if !moved || overflow == 0.0 || stall >= STALL_ITERS {
           break;
       }
       p = (p * P_GROWTH).min(P_FAC_MAX);
   }
   (overflow, iters)
   ```
   Doc: "PathFinder: iteration `i` reroutes the dirty nets (no tree, or a node over capacity) in `order` at
   present-congestion factor `min(p_fac·P_GROWTH^i, P_FAC_MAX)`, then adds `hist_inc · overuse` to history. Stops at
   zero overflow, on a no-move iteration, after `STALL_ITERS` iterations without an overflow decrease, or after
   `max_iters`. Returns `(Σ max(0, usage − cap), iterations run)`."
4. dr: `neg.seed(&mut hot.hist, abs)`, `neg.accumulate(&hot.hist, abs)`; `let (_, pf_iters) = run_pathfinder(&mut
   hot, &cold, P_FAC, HIST_INC, MAX_ITERS);` and the final `RouteStats { trials, overuse, pf_iters,
   ..RouteStats::default() }`. Update the `RouteStats` doc: `pf_iters` is no longer in the "stays zero" list.

### Tests (`backend/gr/src/lib.rs`, `mod tests`)

- `pathfinder_converges_on_a_solvable_crossing`: `let g = TrackGrid::with_layers((12 * 200, 12 * 200), 200, 4.0,
  2);` terms (layer 0) `[[n(0,4), n(11,6)], [n(0,5), n(11,5)], [n(0,6), n(11,4)]]` with `n = |x, y| g.node(x, y,
  0)`; `cold = RouteCtx::new(g, terms, vec![0, 1, 2])`; `hot = RouteHot::new(cold.graph.nodes(), 3)`;
  `let (over, iters) = run_pathfinder(&mut hot, &cold, 1.0, 0.5, 150);` → `assert_eq!(over, 0.0);
  assert!(iters < 150);`.
- `an_unsolvable_instance_stops_on_stall`: `TrackGrid::with_layers((5 * 200, 2 * 200), 200, 4.0, 1)` (one
  horizontal layer, so each row is its own corridor); terms `[[n(0,0), n(2,0)], [n(2,0), n(4,0)]]` (node (2,0) is a
  terminal of both: every route shares it); `run_pathfinder(.., 1.0, 0.5, 150)` → `assert_eq!(over, 1.0);
  assert!((STALL_ITERS..=STALL_ITERS + 1).contains(&iters));` (the lower bound shows the stall rule, not the no-move
  rule, stopped it).
- `history_keys_ignore_the_frame`: `let mut neg = Negotiation::new(); neg.accumulate(&[3.0], |_| (250 - 1_000, 250,
  0)); let mut h = [0.0]; neg.seed(&mut h, |_| (750 - 1_500, 250, 0)); assert_eq!(h[0], 3.0);` (the same absolute
  node, −750 nm, through two frame origins).
- `bump_history_matches_definition` unchanged.

```
cargo test -p gr
cargo test -p dr
cargo test -p library
```

### Acceptance (measured, in this order; record every number in the commit message)

1. Unit tests above green; dr `negotiation_persists_across_calls` green unchanged. If its congestion assert (:2478)
   fails because the growing factor now resolves that instance, stop and report (it asserts history persistence on
   a jammed instance; the owner decides whether to re-jam it). Do not edit it.
2. Baselines on the RTE-07 commit, with temporary, uncommitted instrumentation in a scratch copy (`git worktree add
   <scratchpad>/rte08-base <RTE-07 commit>`; never in this worktree's history):
   - an `eprintln!("pf_iters {}", epoch + 1)` at `run_pathfinder`'s exit (the loop variable restored to `epoch`), and
     an `eprintln!` of every `hard` row's `rule` at the end of dr `score`;
   - `cargo run --release -p benchmark --bin bench local ota` → p50 of the `pf_iters` lines;
   - `cargo run --release -p benchmark --bin bench local dac4` (bench seed 1, `FEEDBACK_ITERS` 5) → count of `score`
     calls whose rows contain `unresolved congestion` over the number of `score` calls (m0 read 7 of 90).
   Then the same two runs on the RTE-08 commit with the same two `eprintln!`s (pf_iters from dr's binding).
   Pass: `pf_iters` p50 on ota ≤ baseline; dac4 `unresolved congestion` count 0.
3. `bench local` all 10: DRC 0 everywhere, LVS column unchanged or better, against the RTE-07 run.
4. FR-6: copy the pwm_driver netlist to `benchmarks/field/pwm_driver.spice` (new directory, not auto-discovered);
   `cargo run --release -p philis -- run benchmarks/field/pwm_driver.spice sky130 -o <scratchpad>/pwm1` twice
   (background; ~6.5 min each at default `Config`). Pass: no `label short` in the output or the run's report files
   (`grep -r "label short" <scratchpad>/pwm1`). If the short remains and the winning epoch's dr report reads 0 hard
   rows, the detection gap goes to RTE-03's short detection (report it; no fix here).

If 2 (dac4) or 4 (FR-6) is not met with the plan's tuning defaults, do not tune `P_GROWTH`, `STALL_ITERS` or
`P_FAC_MAX` to a fixture: report the counts, the dr hard rows of the failing reports, and whether the failing
reports also carry `drawn short nets`, and leave RTE-03 acceptance 3 recorded as not met.

### Why judgment

The code is small and fully specified, but the acceptance is a convergence claim on real circuits (dac4 congestion,
pwm_driver short) that no unit test predicts, the growing factor may change routes on every bench circuit, and one
existing dr test asserts that a jammed instance stays jammed. The implementer has to read bench and dr reports and
decide whether a failure is this item's or RTE-03's.
