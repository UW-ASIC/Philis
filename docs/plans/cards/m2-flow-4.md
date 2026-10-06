# M2+ flow, segment 4 (M3): FLOW-08, FLOW-10

Branch `m2-flow` after `git merge m2` (fast-forward to `367abc0`). dag: both hard-depend on FLOW-09 only (done,
`223d711`/`bd66670`, card m2-flow-3.md). FLOW-08's step-level dep PLC-10 step 1 (`dp::Start::Warm`) applies "only
once `DpMode::Sp` is the default": no `DpMode` exists in `backend/dp` today, the flow runs the flat anneal, so the
flat path (incumbent as `coarse`) is all FLOW-08 needs. PLC-06 (not done) is touched by neither.

Shell: the repo has a `flake.nix` and no `shell.nix`, so "the nix-shell" is `nix develop`. Every command below runs
as `cd /home/omare/Documents/Projects/Rust/philis-m2/flow && nix develop -c env
PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk /home/omare/Documents/Projects/Rust/Philis/tools/qcargo <args>`
(written `qcargo <args>` below).

| Item | Class | Size |
|---|---|---|
| FLOW-08 | do | M: `search` loop, `Flow::epoch` start kind, `cellgen::escalate_blamed`, `StopReason`, `--max-wall` |
| FLOW-10 | do | S: rule-wise DRC+ERC fill gate, post-fill signoff in `finish` |

Order: FLOW-10 first (small, independent), then FLOW-08.

---

## FLOW-10 Fill inside the certificate — class: do

Code facts (plan line numbers are stale; FLOW-09 moved fill into `finish`):
- Fill runs once, on the winner, in `finish` (frontend/library/src/lib.rs:1034); the call is lib.rs:1060
  `macros.extend(fill::fill(&drawn, …))`. `drawn` (lib.rs:1051) is `geometry::collect` of cells + rings + routes.
- The gate is ERC count only: fill.rs:58 `base = verify::erc(drawn, &[], pdk).len()`, fill.rs:66 per layer.
  Comparing totals is wrong for a DRC gate: on a block below a covered density floor the base already has the
  density findings that fill removes, so a total can stay flat while a new spacing/width finding appears.
- `RunStats.drc_hard` = the winning epoch's `signoff.report.hard_violations.len()` (lib.rs:1800, `epoch_score`), the
  report from `signoff_shapes` (lib.rs:1524) plus `undrawable` rows (lib.rs:1525). It is never recomputed after fill.
- `library::signoff` (lib.rs:2310) = `signoff_inputs` (lib.rs:2365, reads `sol.geometry()`, so fill included) →
  `verify::signoff_checked` + `undrawable` + `reliability::voltage_findings` when `sol.op` is set. The voltage rows are
  NOT in the epoch's `drc_hard`, so "signoff == drc_hard" only holds without an op point.
- `Epoch.caps` (lib.rs:1336) is the winner's extracted `verify::CapMatrix`; `finish` drops it. `Solution` has no
  `caps`; `MetadataReport` (metadata.rs:83, `Default`) has no `post_fill`.
- Today's decks never fill a local block (plan's deck facts still hold), so every path below is exercised only by
  tests that add a covered density rule.

Edits:
1. fill.rs `fill`: replace the ERC count with per-rule counts over DRC + ERC.
   ```rust
   /// Findings per rule id, DRC then ERC, label-free.
   fn counts(shapes: &[Shape], pdk: &Pdk) -> std::collections::BTreeMap<String, usize>
   ```
   `base = counts(drawn, pdk)`; a layer's tiles are dropped when any rule's count on `drawn + out + tiles` exceeds its
   base count (missing = 0). Message: `fill: layer {:?} dropped, it adds {rule} findings`. Update the module doc's
   "label-free ERC worse" sentence to "any DRC or ERC rule worse".
2. lib.rs: split `signoff` so the epoch's semantics have one helper:
   ```rust
   /// DRC/ERC/LVS over the drawn solution plus undrawable devices — what an
   /// epoch's `RunStats::drc_hard` counts (no operating-point rows).
   fn drawn_signoff(sol: &Solution, pdk: &Pdk) -> verify::Signoff
   ```
   `signoff` = `drawn_signoff` + the voltage rows (unchanged behaviour).
3. `finish`: `let filled = fill::fill(..)`; `let post_fill = filled.is_some()`; extend macros with it. Build
   `Solution` with `caps: best.caps` (move it out before `best` is consumed) and `metadata.post_fill = post_fill`.
   Then, when `post_fill`: `let s = drawn_signoff(&sol, pdk); sol.stats.drc_hard = s.report.hard_violations.len();
   sol.stats.warnings = s.warnings.len() as u32; sol.caps = s.caps;`. Budgets are not recomputed (fill moves no cell,
   adds no route).
4. `Solution` gains `pub caps: verify::CapMatrix` (doc: the shipped geometry's matrix, post-fill when
   `metadata.post_fill`; PERF-15/22 read it). `MetadataReport` gains `pub post_fill: bool` (doc: fill was kept and
   `RunStats::drc_hard`/`Solution::caps` are re-measured on it); `Display` prints `  post-fill signoff: yes|no`
   under the header (report.txt shows it via the CLI's existing metadata dump).

Tests:
- fill.rs `fill_that_adds_a_drc_finding_is_dropped`: `deck(name)` becomes `deck(name, extra: &str)` (existing two
  tests pass `""`); this test adds `rule TEST.m1_maxw width(metal1) <= 1.5um` (syntax as gf180's DF.2b). Same
  geometry as `a_block_wider_than_the_window_is_filled_inside_from_ground` (1 µm rail/signal satisfy the new rule;
  every ≥2 µm tile violates it). Assert `fill(&[rail, signal], &[rail], &[signal], &[cell], &pdk).is_none()` (metal1
  dropped, metal2 has nothing). The existing filled test is the control (same geometry fills without the rule).
  Before the edit this test fails (ERC gate sees nothing).
- lib.rs `start_tests::winner_signoff_matches_its_certificate`: `pair.spice` on `Pdk::builtin("gf180mcu")`,
  `Config { feedback_iters: 2, outer_iters: 1, starts: 1, ..Default::default() }` (no op) →
  `assert!(!sol.metadata.post_fill)`; `assert_eq!(crate::signoff(&sol, &pdk).report.hard_violations.len(), sol.stats.drc_hard)`.
  Second case, same fixture, gf180 loaded via `Pdk::load(deck_text + "rule TEST.m1_density density(metal1; window:
  10um, step: 5um) >= 40%\n", sidecar)` (the fill.rs pattern): `assert!(sol.metadata.post_fill)` and the same
  equality. If pair's block or ground routing yields no fill at that window, use `ota.spice` and say so in the
  commit; do not drop the `post_fill` assertion.
- Command: `qcargo test -p library --release --lib fill:: winner_signoff_matches_its_certificate`, then
  `qcargo test -p library --release --lib` and `qcargo test -p philis --release`.

Acceptance: T7 — the second case is the T7 guard (post-fill signoff equals the shipped `drc_hard`).

---

## FLOW-08 Epoch loop: warm/cold epochs, history lifecycle, blame-driven escalation, stop reasons — class: do

Code facts (correcting the plan):
- The loop is `search` (lib.rs:936–1031): `prices` and `neg = gr::Negotiation::new()` (lib.rs:939–940) persist over
  every epoch and every assignment; epoch call lib.rs:957 `flow.epoch(&assignment, iter % 2 == 1, &mut prices, &mut
  neg, seed)`; `PATIENCE = 8` (lib.rs:341) ends a middle loop only; `PRICE_STATIONARY = 1e-3` (lib.rs:343).
- Feasible today (lib.rs:995–997) is `key.0 == 0 && key.1 == 0 && key.3 <= 0.0` (`LexKey` = `(V, failed bounds,
  shortfall, Θ, C, area)`, lib.rs:1753). The plan's `key.1 <= 0 && key.2 <= 0` is stale; reuse today's predicate.
- Today a feasible-but-not-stationary run escalates (comment lib.rs:1002–1008 says FLOW-08's `stop` will show it).
- `Flow::epoch` (lib.rs:1358) always runs `gp::place` then `dp::place(&coarse, …, dp::Schedule::cold())`
  (lib.rs:1397–1412). `dp::Schedule::{cold, warm}` exist (backend/dp/src/lib.rs:34–43, `Copy`); `dp::place` takes
  `coarse: &Layout` (dp lib.rs:231), so a warm epoch passes `&incumbent.layout` directly: **no `Layout: Clone` and
  no library `Start` copy of the layout needed** (plan step 1's derive is unnecessary).
- dr routes twice when diodes are inserted (first lib.rs:1471, second lib.rs:1490), both on `neg`: history is
  accumulated twice per such epoch (AT-34). `gr::Negotiation` is `Clone` (backend/gr/src/lib.rs:23).
- Weights: there is no `Flow.weights` (plan lib.rs:294–316 stale). Placement weight is `Flow.net_weight`
  (lib.rs:1188, from `gp::net_weights` at lib.rs:849); the router's normalised copy is built inline at
  lib.rs:873–876 (`r.cfg.net_weight`, budgeted nets only, scaled to [0,1]).
- Escalation: `cellgen::escalate(variants, allowed, current)` (cellgen.rs:498, odometer over `allowed`, GAP-16's
  pruned rows); `pin_spread` cellgen.rs:520. Locks: `Flow.locks` exists (PLC-03 done) and `epoch` already runs
  `locks.unify` on the assignment (lib.rs:1366–1370).
- Blame: there is no `violating_ids` on `Violation`; it is `analog::RuleBatch::violating_ids(&self, &On, &mut
  Vec<u32>)` (kernel/analog/src/rule.rs:241) on `flow.problem.placement.{hard,budget}` (cell ids, retargeted).
  `verify::drc` (backend/verify/src/lib.rs:363) findings carry `x, y`.
- `RunStats` is `Copy + Default` (lib.rs:271); `merge` (lib.rs:1727) keeps the search's fields via `..run`.
- CLI refuses `--max-wall` (frontend/cli/src/main.rs:94, doc main.rs:20–21). bench prints
  `converged|budget` (benchmarks/src/bench.rs:279).
- `StopReason::Patience` from the plan cannot be a run's stop: patience only ends a middle loop and the action
  table then decides. It is left out.

Edits (frontend/library/src/lib.rs unless noted):
1. Schedule, pure:
   ```rust
   #[derive(Clone, Copy, Debug, PartialEq, Eq)]
   enum Kind { Cold, Warm }
   /// Epoch `k` (counted over the whole search) is cold without an incumbent,
   /// right after an escalation, or every `cold_every`-th epoch; else warm.
   fn epoch_kind(k: u32, has_incumbent: bool, escalated: bool, cold_every: u32) -> Kind
   ```
   `Config` gains `pub cold_every: u32` (default 4 [policy, measure: T10 decides]; `1` = every epoch cold, the
   pre-FLOW-08 loop and T10's baseline) and `pub warm: dp::Schedule` (default `dp::Schedule::warm()`).
2. `Flow::epoch(&self, assignment, reshape, start: Option<&Layout>, prices, neg, seed)`: `None` = today's cold path
   with `dp::Schedule::cold()`. `Some(inc)` = warm: assignment := `inc.variant` (then `locks.unify` as today), no
   `gp::place` (stage 0 records ~0), `dp::place(inc, …, self.warm)` where `Flow.warm: dp::Schedule` is copied from
   `cfg.warm` in `topology`. `RunStats.warm_epochs: u32` (= 1 on a warm epoch's stats, summed in `search`).
3. History (AT-34): in `search`, `neg = gr::Negotiation::new()` before every cold epoch, kept across warm ones. In
   `epoch`, `let before = neg.clone();` before the first `router.route`; in the diode branch `*neg = before;` before
   the second route, so one epoch accumulates history once.
4. Weights (plumbing for PERF-15/GAP-08, no behaviour change): `pub(crate) struct Weights { place: Vec<f32>, route:
   Vec<f32> }` replaces `Flow.net_weight`; `fn route_weights(place: &[f32], classes: &[NetClassification], sens:
   &[(NetId, f32)]) -> Vec<f32>` factors lib.rs:873–876. `search` owns a `Weights` cloned from `Topology` and
   passes `&Weights` to `epoch` (gp `net_weight`, dp `net_weight`, and `DetailedCfg.net_weight = w.route.clone()`
   in the per-epoch `router`). Nothing updates it yet.
5. Actions after a middle loop (replaces lib.rs:993–1015), `best` = incumbent:
   ```text
   feasible   = today's predicate (key.0 == 0 && key.1 == 0 && key.3 <= 0.0)
   stationary = prices.drift() < PRICE_STATIONARY && prices.saturated().is_empty()
   feasible && stationary  → Converged      (stats.converged = true, as today)
   feasible                → FeasibleNotStationary   (never escalate a feasible incumbent)
   outer + 1 == n_outer    → OuterBudget
   escalate_blamed(..) = None → EscalationExhausted
   ```
   as a pure `fn next_action(feasible: bool, stationary: bool, last_outer: bool) -> Option<StopReason>` (`None` =
   escalate). Before every epoch: `if best.is_some() && deadline.is_some_and(|d| Instant::now() >= d)` → stop
   `WallBudget` (labelled break out of both loops). `search(t, cfg, seed, deadline: Option<Instant>)`; `run`
   computes `deadline = cfg.max_wall.map(|d| Instant::now() + d)` once before the starts.
6. `tried: BTreeSet<Vec<u16>>` in `search` holds every unified assignment run (insert `layout.variant` of each
   epoch's realised assignment before dp, i.e. the unified one). On escalation: `blame = flow.blame(&best,
   &macros_of_best)`; loop `next = cellgen::escalate_blamed(variants, &t.allowed, &assignment, &blame, &tried)?;
   flow.locks.unify(&mut next); if tried.insert(next.clone()) { break next }` (assignment := next on each miss so
   the fallback odometer advances; terminates because `escalate` does).
7. cellgen.rs:
   ```rust
   /// Next assignment on an infeasible stall: the most-blamed cell with an
   /// untried higher `allowed` entry moves first (ties: larger pin spread,
   /// then lower index); else `escalate` order, repeated until untried or
   /// exhausted. Never returns an assignment in `tried`.
   pub fn escalate_blamed(variants: &[gp::VariantSpace], allowed: &[Vec<u16>], current: &[u16], blame: &[u32], tried: &BTreeSet<Vec<u16>>) -> Option<Vec<u16>>
   ```
   Order `(Reverse(blame[i]), Reverse(pin_spread(&variants[i])), i)`; first `i` and `a ∈ allowed[i]`, `a >
   current[i]`, with `current[i := a]` not in `tried` wins; else `escalate` repeatedly.
8. `fn blame(&self, e: &Epoch) -> Vec<u32>` on `Flow`: per cell, +1 per id from `violating_ids(&e.layout, …)` of
   every `problem.placement.hard` batch with `violations > 0` and every `budget` batch with `residual > 0`; +1 per
   `verify::drc(shapes, &[], pdk)` finding whose `(x, y)` is in the cell's placed bbox (`pnr_core::place_macros(&
   realize(variants, &e.layout.variant), &e.layout)`, shapes = `geometry::collect` + rings). One call per escalation.
9. `pub enum StopReason { Converged, FeasibleNotStationary, #[default] OuterBudget, EscalationExhausted, WallBudget }`
   (`Clone, Copy, Debug, Default, PartialEq, Eq`); `RunStats.stop`. `Config.max_wall: Option<Duration>` (default
   `None`). Delete the ponytail comment at lib.rs:1002–1008 (resolved).
10. CLI main.rs:94: `"--max-wall" => cfg.max_wall = Some(Duration::from_secs_f64(num(&a, &val()?)?))`; fix the module
    doc (main.rs:20–21); `report.txt`'s run-stats block prints `stop {:?}` and `warm {}`. bench.rs:279: print
    `{:?}` of `s.stop` and `warm {}` instead of `converged|budget`; bench reads `PNR_BENCH_COLD_EVERY` (as
    `PNR_BENCH_SEED`, bench.rs:474) into `cfg.cold_every`.

Tests (lib.rs `start_tests` unless noted; `qcargo test -p library --release --lib <name>`):
- `schedule_is_cold_first_then_periodic`: `epoch_kind(0, false, false, 4) == Cold`; `(1, true, false, 4) == Warm`;
  `(4, true, false, 4) == Cold`; `(5, true, true, 4) == Cold`; `(k, true, false, 1) == Cold` for k in 0..8;
  `(7, true, false, u32::MAX) == Warm`.
- `feasible_incumbent_is_never_escalated`: `next_action(true, false, false) == Some(FeasibleNotStationary)`,
  `(true, true, _) == Some(Converged)`, `(false, _, true) == Some(OuterBudget)`, `(false, false, false) == None`.
- cellgen.rs `escalate_blamed_moves_the_most_blamed_cell_first`: three cells with `allowed = [[0,1],[0,1],[0,1]]`
  and equal pin spread (variants built as `escalate_never_repeats_and_exhausts`, cellgen.rs:1466), `blame = [0, 5, 1]` → `Some([0,1,0])`.
  `escalate_blamed_never_repeats`: iterate from `[0,0,0]` inserting each result into `tried`; collect until `None`;
  assert 7 results, all distinct, none equal to `[0,0,0]`.
- `wall_budget_stops_with_its_reason`: one nfet `.subckt` (no pair, one topology), `Config { max_wall:
  Some(Duration::ZERO), feedback_iters: 5, outer_iters: 2, starts: 1, .. }` → `stats.iterations == 1`,
  `stats.stop == StopReason::WallBudget`.
- `warm_epoch_starts_from_the_incumbent`: same one-device circuit, `cold_every: u32::MAX`, `warm: dp::Schedule {
  range0: 0.0, max_temps: 0, t0_scale: 0.0 }`, `feedback_iters: 2, outer_iters: 1, starts: 1` →
  `stats.warm_epochs == 1`; both epochs promoted (`metadata.epochs.len() == 2`) with equal `(v, theta, area_um2)`.
  Control in the same test: `cold_every: 1` → `warm_epochs == 0`. If dp with `max_temps: 0` still moves cells,
  report it (dp's contract, owner placement) rather than loosening the equality.
- CLI (frontend/cli/tests/cli.rs) `max_wall_is_accepted`: `--max-wall 0` exits 0 and `report.txt` contains
  `stop WallBudget`.
- Regression: `qcargo test -p library --release --lib`, `qcargo test -p philis --release`,
  `qcargo test -p library --release --test emit_roundtrip` (known red `run_emit_elaborate_signs_off` from segment 3
  must not get worse).

Acceptance (T10, [measure]): `bench local` over seeds 1–5 (`PNR_BENCH_SEED`) at `PNR_BENCH_COLD_EVERY ∈ {1, 2, 4,
8, 4294967295}`, release, background through qcargo. Table: median final key per fixture vs `cold_every = 1` at
equal epochs (`feedback_iters` unchanged), epochs-to-best, `stop` per circuit; pass = no worse on ≥ 8/10 fixtures.
The best value becomes `Config::default().cold_every`; if none passes, keep `1` and publish the table. This is the
item's expensive step (5 × 5 bench runs); if the queue cannot fit it in the segment, land the code with default
`cold_every = 1` (behaviour-neutral for warm, new stop/escalation policy only) and report T10 as pending.

Out of scope: `Weights` updates (PERF-15, GAP-08); `dp::Start::Warm` with tree/variant/orient (PLC-10 step 1, once
an SP dp exists); replica exchange (PL-20, not planned).
