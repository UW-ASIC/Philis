# M2+ perf segment 4 (M4): implementation cards

Branch `m2-perf`, worktree `philis-m2/perf`, after `git merge m2` (fast-forward to `7073e38`, clean). DAG: PERF-12
hard-depends on PERF-11, PERF-13 (done, segment 3) and EXT-17 (done: `annotator::evidence::{Sensitivities,
SpecSens}`, evidence.rs:47–72, incl. `d_cc`); step-level on RTE-21 step 1 (`DetailedCfg.r_weight/pair_weight`, **not
landed**: `DetailedCfg` backend/dr/src/lib.rs:50 has neither) and EXT-25 (rows built from `Evidence.sens`, **not
landed**: nothing in `backend/annotator` reads `ev.sens`, evidence.rs:15–16 says so). PERF-14 hard-depends on
PERF-13 only. Line numbers are of this tree; find code by symbol. Every command needs
`export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk`.

| Item | Class |
|---|---|
| PERF-12 | do (step 2's `DetailedCfg` write and step 1's row replacement wait on RTE-21 / EXT-25; see scope) |
| PERF-14 | do |

Batch rules: no new warnings in `library`. A test not named here that goes red means stop and report; do not edit its
assertion (except the five key tests PERF-14 retypes, listed below). Order: PERF-12 then PERF-14 (both touch
`performance_rows` / `PerfPlan`).

---

## PERF-12 Sensitivity export

### Current code (plan-07 corrections marked \*)

- \* `budget_rows(cfg, start, tables, nets, af_per_nm)` is perf.rs:613–636 (plan: 230–257); reads only linear
  `GroundC` rows. `Param { GroundC{net}, CouplingC{a,b}, SeriesR{device,terminal: u8}, GateOffset{device} }` perf.rs:338;
  `SensRow { param, step, d: Vec<Option<f64>>, linear }` :352; `SensTable { scenario, at, base: PerfResult, rows, sims }`
  :366. Gate offset is per **V** with δ = −ΔV_T (NMOS) / +Δ|V_T| (PMOS) (doc above `Param`).
- \* `performance_rows` is lib.rs:484–564 (plan: 196–231): `start = perf::evaluate(.., &all)`, `active` = nominal +
  each bound's worst scenario, one `SensTable` per active scenario (+ `add_coupling(.., 64)`), `sigma_v =
  robust::device_sigma_v`. `PerfPlan { rows, notes, active, tables, sigma_v, sens, sims }` lib.rs:568–586. `start`
  is dropped after `budget_rows`.
- \* `annotate_with(&netlist, &ann, &ev)` runs at lib.rs:416 (base, for `net_classes`), then `performance_rows`
  (:417), then per topology again at lib.rs:650 with the same `ev`. `Evidence.sens` is never set.
- \* `net_weight` / `r.cfg.net_weight` are lib.rs:725–755 (plan: 292–316), built from `perf_rows` weights.
- \* The plan's `active: &[usize]` (per spec) and `spread: &[(f64,f64)]` do not exist: `active` is a scenario list, and
  `evaluate`/`score` keep only each bound's worst value, so the per-spec metric range over scenarios is lost.
- \* Plan says nonlinear rows are dropped. `robust::terms` (robust.rs:58–71) deliberately uses `GateOffset` rows
  whatever `linear` (σ step = one-σ secant); `d_vt` must follow σ_f's rule or ota's XM1/XM2 may vanish from `d_vt`
  while their σ_f share is reported. So: C / R / coupling rows linear only, `GateOffset` rows always.
- `metadata.sensitivity: Vec<String>` (metadata.rs:107) is filled from `plan.sens` (lib.rs:469); the flow already
  prints `[perf] sens …` lines.

### Edits

1. `perf.rs` `PerfResult` (:114): add
   `/// Per spec, (min, max) of the metric over the evaluated scenarios; None if any did not measure it.`
   `pub spread: Vec<Option<(f64, f64)>>,`. Fill in `score` (:149) inside the spec loop from `col()`:
   `col().map(|(v, _)| v).collect::<Option<Vec<_>>>().and_then(|v| Some((v.iter().copied().reduce(f64::min)?, v.iter().copied().reduce(f64::max)?)))`.
2. `perf.rs`, after `budget_rows`:
   ```rust
   /// EXT-17's input, one `SpecSens` per spec with a finite bound in `start`: the table read is the one at the
   /// scenario of the spec's tighter bound (smaller margin; floor on a tie), `f0` its `base` metric (spec skipped
   /// when unmeasured). `proc` = `start.spread[j]` when `cfg.scenarios.len() > 1`; `sigma_f` = max over the spec's
   /// bounds of `stats[b].sigma_f` (`None` if any is). `d_c`/`d_cc` per aF from linear `GroundC`/`CouplingC` rows;
   /// `d_r` per Ω, linear `SeriesR` rows summed per net of the terminal; `d_vt` per mV from every `GateOffset` row
   /// (σ_f's rule): `−d/1000` NMOS, `+d/1000` PMOS; `d_t` empty.
   #[must_use]
   pub fn to_evidence(cfg: &PerfConfig, tables: &[SensTable], start: &PerfResult, stats: &[crate::robust::BoundStat],
                      netlist: &Netlist) -> annotator::evidence::Sensitivities;

   /// RTE-21's router weights. `bounds[k] = (spec, h_b, scenario)`, `h_b > 0`. `r_weight[n]` (len `netlist.nets`)
   /// = Σ_b Σ_{linear SeriesR rows on n} |d_b| / h_b, scaled so the max is 1 (all 0 if none);
   /// `pair_weight` = `(a, b, Σ_b ½·|d_b| / h_b)` per linear `CouplingC` row, unscaled. Row `d` read in the table
   /// whose `scenario` is the bound's.
   #[must_use]
   pub fn router_weights(tables: &[SensTable], bounds: &[(usize, f64, usize)], netlist: &Netlist)
       -> (Vec<f32>, Vec<(pnr_core::NetId, pnr_core::NetId, f32)>);
   ```
3. `lib.rs` `performance_rows` Ok arm (:551): `stats = robust::bound_stats(&tables, &sigma_v, &start, &p.specs, &[],
   &[])`; `evidence = perf::to_evidence(p, &tables, &start, &stats, netlist)`; `h_b` per `start.bounds[b]` with a value:
   `stats[b].headroom_stat` if `> 0`, else plain headroom `sign·(bound − f0)` if `> 0` and σ unknown, else `|bound|`
   (`1` for 0) — PERF-06's scale as `budget_rows`; `(r_weight, pair_weight) = perf::router_weights(..)`. Push to
   `sens_notes`: one `"evidence {metric}: d_c {n}, d_r {n}, d_vt {n}, d_cc {n}, σ_f {v|unknown}"` per spec,
   `"r_weight {net} {w:.3}"` per nonzero net, `"pair_weight {a}-{b} {w:.3e}"` per pair (net names).
   `PerfPlan` gains `evidence: Option<annotator::evidence::Sensitivities>`, `r_weight: Vec<f32>`,
   `pair_weight: Vec<(NetId, NetId, f32)>` (empty/None in the `plan` closure, the test literal at lib.rs:2782).
4. `solve` after :417: `let ev = annotator::Evidence { sens: plan.evidence.clone(), ..ev };` so the per-topology
   `annotate_with` (:650) receives it (EXT-17's second call). `budget_rows` stays the row source.

Scope / waits: RTE-21 step 1 not landed → do **not** add `DetailedCfg` fields; `PerfPlan.r_weight/pair_weight` are
computed, reported and left for RTE-21 (whichever lands second writes `flow.d_router.cfg.r_weight/pair_weight` next to
`net_weight`, lib.rs:754). EXT-25 not landed → rows still come from `budget_rows`; replacing them is EXT-25's. Step 3:
`SpecSens.d_cc` already exists (filled here); `PerformanceBudget.coupling` is EXT-25's change request, untouched.

### Tests

`frontend/library/src/perf.rs` tests (hand-built `SensTable`s; netlist nets `[vout1, vtail, x]`, XM1/XM2 NMOS with S
on `vtail`, XM3 PMOS):
- `evidence_maps_units_and_signs`: rows `GroundC{vout1}` d −0.002, `SeriesR{0,2}` and `SeriesR{1,2}` d −0.01 each,
  `GateOffset{0}` d +1000, `GateOffset{2}` d +1000 (PMOS), `CouplingC{vout1,x}` d 0.003, all linear; start one bound
  gain:min scenario 0; stats `sigma_f Some(1.5)` → `d_c == [(vout1, −0.002)]`, `d_r == [(vtail, −0.02)]` (±1e-12),
  `d_vt == [(XM1, −1.0), (XM3, +1.0)]`, `d_cc == [(vout1, x, 0.003)]`, `sigma_f == Some(1.5)`, `proc == None`,
  `d_t` empty.
- `router_weights_scale_r_to_one`: one bound h = 2; linear `SeriesR` rows on n0 d −0.3 and on n1 d +0.6;
  `CouplingC{n0,n1}` d 0.4 → `r_weight == [0.5, 1.0]`, `pair_weight == [(n0, n1, 0.1)]` (±1e-6).
- `a_nonlinear_row_is_not_exported`: same `GroundC`/`SeriesR` rows with `linear: false` → `d_c`, `d_r` empty,
  `r_weight` all 0; a nonlinear `GateOffset` row still in `d_vt`.
- `score_reports_the_spread`: `score` over 3 scenarios gain [60, 55, 70] → `spread[0] == Some((55, 70))`; one `None`
  → `None`.

`frontend/library/tests/perf_postlayout.rs` acceptance `ota_exports_sensitivities` (`models()` guard like
`ota_reports_robustness_per_bound`, tt_27 + ff_27 scenarios, `feedback_iters 2, outer_iters 1, starts 1`):
`metadata.sensitivity` has an `"evidence gain: "` line with d_c, d_r, d_vt > 0 and σ_f not unknown, an
`"r_weight vtail "` line, and at least one `pair_weight` line; and directly (pub API) `perf::to_evidence` on
ota's schematic table: `d_r` contains `vtail`, `d_vt` contains XM1 and XM2, `proc.is_some()`.

Command: `cargo test -p library --lib perf:: && cargo test -p library --release --test perf_postlayout ota_exports_sensitivities`.

---

## PERF-14 Epoch key on β; Pareto report

### Current code (corrections marked \*)

- \* `type LexKey = (usize, f64, f64, f32, f64)` = (|V|, spec miss, Θ, C tier, footprint) at lib.rs:1536 (plan :916);
  `key_lt` :1551–1563 (NaN-last on tiers 1–2, `C_TIE` band on tier 3, footprint); `lex_key` :1591–1611 always called
  with `perf: None` (:1578), so tier 1 is set only by `score_perf` (:1503, `epoch.key.1 = result.residual`).
- \* Incumbent logic is `search` lib.rs:810–887 (plan :362–367): promotion `epoch.key.0 <= b.key.0` (:831),
  replace on `key_lt` (:837), stall vs `PATIENCE = 8` (:315, :845), feasibility `key.0 == 0 && key.1 <= 0 &&
  key.2 <= 0` (:855). Cross-start / topology pick in `solve` :454–461.
- \* σ_f is schematic-only (`PerfPlan.tables/sigma_v`), so per epoch β_b = margin_b / σ_f,b with a fixed σ_f;
  `robust::bound_stats` (robust.rs:107) is cheap (no simulation) and can run in `score_perf`.
- Tests that build `LexKey` literals: `close_c_is_decided_by_area_and_far_c_by_c` (:2534), `nan_loses` (:2600),
  `supply_decoupling_does_not_rank_layouts` (:2637); via `lex_key`: `v_counts_rules_not_batches`,
  `theta_counts_each_budget_once`, `warnings_are_not_violations` (index `.0`/`.2` reads).

### Edits

1. `robust.rs`:
   ```rust
   /// PERF-14's spec tiers `(failed, shortfall)` of `post`. `beta_key`: failed = #bounds with β < 0 or β unknown,
   /// shortfall = max_b max(0, BETA_TARGET − β_b), +∞ when any β is unknown. Otherwise (σ unknown for the run):
   /// failed = #bounds unmeasured or missing their side (`perf::miss` of the one-sided spec > 0), shortfall =
   /// `post.residual` (today's tier).
   #[must_use]
   pub fn key_tiers(stats: &[BoundStat], post: &PerfResult, specs: &[Spec], beta_key: bool) -> (u32, f64);
   /// Smallest β over `stats`; `None` if empty or any is unknown.
   #[must_use]
   pub fn min_beta(stats: &[BoundStat]) -> Option<f64>;
   ```
2. `PerfPlan.beta_key: bool` = in `performance_rows`, every `bound_stats(.., &start, ..)` entry has `sigma_f` Some
   (decided once per run so every epoch is keyed alike); `false` in the fallback arms.
3. `lib.rs`: `type LexKey = (usize, u32, f64, f64, f32, f64)` = (|V|, failed bounds, spec shortfall, Θ, C tier,
   footprint nm²); doc updated. `key_lt`: `head = (k.0, k.1, nan_last(k.2), nan_last(k.3))`, C band on `.4`, footprint
   `.5`. `lex_key` returns `(v, 0, 0.0, theta, c_tier, footprint)` and drops its unused `perf` parameter (both call
   sites :1578 and test helper :2553). `score_perf`: `stats = robust::bound_stats(&self.perf_plan.tables,
   &self.perf_plan.sigma_v, &result, &p.specs, &[], &[])`; `(epoch.key.1, epoch.key.2) = robust::key_tiers(&stats,
   &result, &p.specs, self.perf_plan.beta_key)`; `epoch.min_beta = robust::min_beta(&stats)`. `Epoch` gains
   `min_beta: Option<f64>` (`None` in `Flow::epoch`). Feasibility (:855): `b.key.0 == 0 && b.key.1 == 0 &&
   b.key.3 <= 0.0` (shortfall > 0 is met-but-not-3σ, not infeasible).
4. `metadata.rs`:
   ```rust
   /// One promoted epoch's metrics (PERF-14); `Layout` is not `Clone`, so no geometry.
   #[derive(Clone, Debug, PartialEq)]
   pub struct ParetoPoint { pub outer: u32, pub iteration: u32, pub v: usize, pub residual: f64,
                            pub min_beta: Option<f64>, pub theta: f64, pub c_tier: f32, pub area_um2: f64 }
   /// Inserts `p` unless dominated in (−min_beta [None = −∞], theta, c_tier, area_um2), removes points it dominates;
   /// over `PARETO_MAX = 16` [policy] drops the largest-area point.
   pub fn pareto_insert(front: &mut Vec<ParetoPoint>, p: ParetoPoint);
   ```
   `MetadataReport` gains `pub pareto: Vec<ParetoPoint>` (non-dominated over every start and topology, epochs with
   `key.0 == 0 && key.1 == 0`) and `pub epochs: Vec<ParetoPoint>` (every promoted epoch of the winning search, in
   order; `v`/`residual` let a caller re-rank under the old key).
5. `search`: after `score_perf` (promoted only) build the point (`area_um2 = key.5 / 1e6`, `residual =
   epoch.perf.residual`), push to `epochs`, `pareto_insert` into `pareto` when `key.0 == 0 && key.1 == 0`. `Searched`
   gains `pareto, epochs`. `solve`: before `runs.into_iter()` (:463) merge every run's `pareto` with `pareto_insert`;
   `finish` sets `metadata.epochs = s.epochs`; `solve` sets `sol.metadata.pareto`. Winner stays the lexicographic one.

### Tests

`lib.rs` tests:
- Retype (no assertion changes): `close_c_is_decided_by_area_and_far_c_by_c` `k = (0usize, 0u32, 0.0, 0.0, c, area)`,
  violation literal `(1usize, 0, 0.0, 0.0, 1.0, 1.0)`; `nan_loses` with NaN on the shortfall tier `(0usize, 0u32,
  f64::NAN, 0.0, 1.0, 1.0)` vs `(.., 5.0, ..)`; `supply_decoupling_does_not_rank_layouts` 6-tuples;
  `theta_counts_each_budget_once` reads `.3` for Θ (was `.2`).
- `robust_beats_barely_passing`: one gain:min bound, `BoundStat` β 2.5 vs β 1.0 → `key_tiers == (0, 0.5)` and
  `(0, 2.0)`; `key_lt((0, 0, 0.5, 9.0, 9.0, 9.0), (0, 0, 2.0, 0.0, 0.0, 0.0))` (robustness outranks Θ, C, area).
- `an_unknown_bound_counts_as_failed`: bounds β 4.0 and β None (value None) → `(1, f64::INFINITY)`; `beta_key = false`
  with the same value None → failed 1; a met bound → `(0, 0.0)` in miss mode.

`metadata.rs` tests: `pareto_keeps_only_nondominated`: insert A(β 3, θ 0, c 10, a 100), B(β 2, 0, 10, 100),
C(β 1, 0, 5, 100), D(β 1, 0, 6, 100), E(β 0.5, 0, 20, 50) → front == {A, C, E} (order-independent compare); plus
17 mutually non-dominated points → len 16, the largest area absent.

`perf_postlayout.rs` acceptance `beta_key_winner_is_at_least_as_robust` (`models()` guard; ota, tt_27, `starts 1,
outer_iters 1, feedback_iters 4` (≤ PATIENCE, no stall exit; refresh does not exist yet)), seeds 1–5: winner =
`epochs` entry with `iteration == stats.best_iteration`; old-key winner = best of `epochs` under a test-local copy
of the old comparator ((v, residual, θ) lexicographic, then 2 % C band, then area); assert
`winner.min_beta.unwrap_or(-INF) >= old.min_beta.unwrap_or(-INF)` and `winner.min_beta.is_some()`; `pareto` non-empty
when the winner has v 0. Limitation: compares within the winning topology's search (ota has merged and apart
topologies; `epochs` is the winner's only).

Command: `cargo test -p library --lib && cargo test -p library --release --test perf_postlayout beta_key_winner_is_at_least_as_robust`.
