# M2 perf segment 1: implementation cards

Branch `m2-perf`, worktree `philis-m2/perf`; `git merge m2` was a no-op (both at `c2940c6`). Note: the
`docs/plans/dag-m2-m6.{json,md}` files are not in `m2` yet (only in the main checkout); the DAG entries were read
from there: PERF-10 has no hard deps (PERF-06, PERF-08 done) and its step 3 moves to PERF-15 (M4); PERF-17 has
no hard deps (PERF-01 done). Line numbers are of this tree; find code by the symbol named next to each.
Every command needs `export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk`.

Order: PERF-10 → PERF-17. They share no code, so either order works.

| Item | Class |
|---|---|
| PERF-10 | do (steps 1, 2, 4; step 3 lands with PERF-15) |
| PERF-17 | do |

Batch rules: no new warnings in `library`, `verify`, `benchmark`. A test not named here that goes red means stop and
report. Do not edit its assertion.

---

## PERF-10 Scenarios, several testbenches, corner-aware margins

### Current code (corrections to plan-07 marked *)

- `frontend/library/src/perf.rs:61-69` `PerfConfig { sim: OpConfig, testbench: String, specs: Vec<Spec> }`.
  The plan cites :59-68; it is now :61-69. `PerfResult { metrics, residual }` (:73-80); `miss(spec, v)` (:92-96)
  sums the floor and ceiling miss and returns `1.0` for `None`.
- `deck` (:103-151) emits `cfg.sim.lib_lines()` (`.lib {path} {corner}` plus `OpConfig::params`,
  `oppoint.rs:172-175`), circuit, pex, branch R, testbench, `.end`. \* **No `.temp` line**: the perf deck always
  simulates at ngspice's default 27 °C and ignores `OpConfig::temp_c`. Only the op deck sets `.temp`
  (`oppoint.rs:328-333`). PERF-10 fixes this.
- `evaluate(netlist, par, cfg)` (:171-189) runs one deck and returns its metrics with `residual = Σ miss`.
  `sensitivities(netlist, cfg, nets, delta_af)` (:200-225) runs one base at `Parasitics::default()` plus one
  thread per net (`std::thread::scope`) and gives `d_per_af[spec][net]`. `budget_rows(cfg, s, nets, af_per_nm)`
  (:234-258) computes headroom from `s.base.metrics[j]`, i.e. at the one scenario.
- `frontend/library/src/lib.rs`: `Config::performance` (:96). `performance_rows` (:327-373) is called once per
  run at :287 and returns `(rows, notes)`. `solve(...)` (:391) is called at :296 and :300 with `&perf_rows`. Flow
  is built with `perf: cfg.performance.as_ref()` at :479 (field :656). Promotion is at :512-516:
  `score_perf` runs only if `epoch.key.0 <= best.key.0`. `score_perf` (:1086-1103) calls `evaluate`, and its
  `unknown()` closure builds `PerfResult` by hand. Winner metadata (:614-621):
  `metadata.performance = (metric, value, min, max, miss)`. `metadata.rs:95` holds the field, `:344-357` prints it.
- Constructors of `PerfConfig` to update: `perf.rs` tests (:301, :326, :338, :359), `lib.rs:1894`, `:1921`,
  `frontend/library/tests/perf_postlayout.rs:50-56`, `examples/op_demo/src/main.rs:26`. No CLI parses it.

### Scope decisions

- `beta_target` (PERF-13) and `mc` (PERF-22) are **not** added here. Those items add them.
- The job pool (`run_jobs`) belongs to PERF-11 (M4). Until then the (testbench, scenario) decks of one call run on
  `std::thread::scope`, as `sensitivities` already does. Mark it with
  `// ponytail: one thread per deck; PERF-11's run_jobs bounds it`.
- `spread[j]` (step 1, for EXT-17/PERF-12) is skipped: it has no consumer until PERF-12. PERF-12 computes it from
  the start evaluation.
- Step 3 (winner re-evaluated over all scenarios after fill) is deferred to PERF-15 by the DAG. Until then the
  report states that the winner's worst case is "over active scenarios".
- **Deviation:** the nominal scenario (index 0) is always in `active`. It feeds the display row
  (`metadata.performance`), and the plan's "nominal scenario, for display" metrics would not exist otherwise. So
  `|active| ≤ 2·n_bounds + 1`.

### Edits

`frontend/library/src/perf.rs`:

```rust
/// One process/environment point: model corner, temperature, and `.param` values the testbenches read
/// (e.g. `Vdd vdd 0 {vdd}`).
#[derive(Clone, Debug)]
pub struct Scenario { pub name: String, pub corner: String, pub temp_c: f64, pub params: Vec<(String, f64)> }

pub struct PerfConfig {
    pub sim: OpConfig,
    /// Each a complete bench (sources, analyses, `.measure`), no `.end`; a spec's metric comes from exactly one.
    pub testbenches: Vec<String>,          // was `testbench: String`
    pub specs: Vec<Spec>,
    /// Index 0 is nominal. Empty = one scenario from `sim.corner` / `sim.temp_c`.
    pub scenarios: Vec<Scenario>,
}
impl PerfConfig {
    /// `scenarios`, or the single one `sim` implies.
    #[must_use] pub fn scenarios(&self) -> Vec<Scenario>;
}

/// A finite bound's worst value over the evaluated scenarios (`None` = some scenario did not measure it).
#[derive(Clone, Debug)]
pub struct BoundResult { pub spec: usize, pub upper: bool, pub value: Option<f64>, pub scenario: usize }

#[derive(Clone, Debug, Default)]
pub struct PerfResult {
    /// Per spec, at the first evaluated scenario (nominal in the flow: `active[0] == 0`).
    pub metrics: Vec<(String, Option<f64>)>,
    /// One per finite bound, spec order, floor before ceiling.
    pub bounds: Vec<BoundResult>,
    /// Per spec: `1.0` if any scenario did not measure it, else the floor miss at the worst floor value plus the
    /// ceiling miss at the worst ceiling value (one scenario: equals `miss(spec, v)`).
    pub miss: Vec<f64>,
    pub residual: f64,                     // Σ miss
}
```

1. `deck(netlist, par, cfg, tb: &str, sc: &Scenario)`. The lib line comes from
   `OpConfig { corner: sc.corner.clone(), ..cfg.sim.clone() }.lib_lines()`. Then emit `.temp {sc.temp_c}` and one
   `.param {k}={v}` per `sc.params`, all before the circuit. Then the existing body with `tb` in place of
   `cfg.testbench`.
2. `pub fn score(specs: &[Spec], measured: &[Vec<Option<f64>>], scenarios: &[usize]) -> PerfResult`, pure
   (`measured[i][j]` = spec j at `scenarios[i]`). For each bound, the worst value is the smallest margin
   (`v − lo` / `hi − v`). The first scenario with `None` is worst (unknown never passes). Ties go to the earlier
   scenario.
3. `evaluate(netlist, par, cfg, scenarios: &[usize]) -> Result<PerfResult, String>`:
   - run every `(tb, scenario)` deck in parallel and parse its measures;
   - for spec j in scenario i, take the value from the testbench that measured it;
   - `Err("metric {m} measured by testbenches {a} and {b}")` if two testbenches did;
   - any deck error is `Err` (as today);
   - return `score(...)`.
   Delete the `testbench` field.
4. `sensitivities(netlist, cfg, nets, delta_af, scenarios: &[usize])`. Base and per-net runs use `scenarios`.
   `d_per_af` becomes `[bound][net]`, aligned with `base.bounds`:
   `(f1.bounds[k].value − f0.bounds[k].value) / delta_af`. The worst scenario may switch under a 10 fF step; the
   difference is still of the worst case, which is the corner-safe margin (LAMP-01), and the doc says so.
5. `budget_rows` iterates `s.base.bounds`, not `specs × {min, max}`. Headroom is `sign·(bound − value)` at the
   bound's active scenario (step 4). Metric names, weights and do-not-worsen semantics are unchanged.

`frontend/library/src/lib.rs`:

6. `performance_rows` returns `(rows, notes, active: Vec<usize>)`:
   - first `perf::evaluate(netlist, &Parasitics::default(), p, &all)`;
   - `active = [0]` plus each bound's worst scenario, deduplicated and kept in order;
   - one note per scenario: `"scenario {name}: active"` / `"…: inactive"`;
   - then `sensitivities(..., &active)`;
   - an `Err` from the start evaluate goes down the existing "sensitivities unavailable" path, with
     `active = all`.
7. Add `perf_active: &[usize]` to `solve` (both calls :296/:300) and `perf_active: &'a [usize]` to `Flow`.
   `score_perf` evaluates `self.perf_active`. `unknown()` becomes `perf::score(&p.specs, &[vec![None; n]], &[0])`.
8. Winner metadata (:614): `miss` = `result.miss[j]`. New field `metadata.performance_worst: Vec<String>`, one
   line per bound: `"{metric}:{min|max} worst {value} at {scenario name} (over {k} active of {n} scenarios)"`.
   Print each as `  worst {line}` after the spec table (metadata.rs:356).
9. Update every constructor listed above: `testbenches: vec![...]`, `scenarios: Vec::new()`.

### Tests

- `perf.rs` `the_deck_sets_corner_temperature_and_params`: `OpConfig { model_lib: Some("/m.lib".into()), .. }`,
  `Scenario { name: "ss_hot", corner: "ss", temp_c: 125.0, params: vec![("vdd".into(), 1.62)] }`. Assert the deck
  contains `".lib /m.lib ss\n"`, `".temp 125"` and `".param vdd=1.62"`, and that the `.temp` appears before
  `"XM1"`.
- `perf.rs` `the_worst_scenario_is_chosen_per_bound`: spec `min 10, max 20`, scenarios `[0, 1, 2]`, measured
  `[[15], [11], [19]]`. Assert:
  - `bounds[0] == {upper: false, value: Some(11), scenario: 1}`;
  - `bounds[1] == {upper: true, value: Some(19), scenario: 2}`;
  - `residual == 0`;
  - `metrics[0].1 == Some(15)`.
  Then with `[[15], [8], [19]]`, `miss[0] ≈ 0.2`. With `[[15], [None], [19]]`, both bounds have
  `value == None, scenario == 1` and `residual == 1`.
- `perf.rs` `a_metric_from_two_testbenches_is_refused`: two benches that both `.measure` `x`; `evaluate` →
  `Err` containing `"testbenches 0 and 1"`. This needs ngspice: put it in `perf_postlayout.rs` behind `models()`.
- Existing `rows_*` tests are rebuilt on `PerfResult { bounds, .. }` with the same numbers and assertions.
- `perf_postlayout.rs` `ota_gain_moves_with_temperature`: scenarios `tt 27` / `tt 125`, evaluated separately at
  `Parasitics::default()`. Assert `(g27 − g125).abs() > 0.1` dB. This fails if `.temp` is dropped.
- `perf_postlayout.rs` `a_flow_reports_the_worst_scenario_per_bound` (acceptance):
  - ota flow as in `a_flow_scores_its_layout_in_simulation`, with scenarios `{tt 27, ss 125, ff −40}`;
  - assert `metadata.performance_worst.len() == 1` and that it starts with `"gain:min worst "`;
  - independently `evaluate` the schematic per scenario; the scenario with the lowest gain must be the one named;
  - assert the `"(over k active of 3"` text has `k ≤ 2` (nominal + 1 bound): sims per promoted epoch =
    `k × |testbenches|`.
- Command: `cargo test -p library perf` and `cargo test -p library --test perf_postlayout`. The flow tests in
  `lib.rs` (`a_failed_simulation_is_counted_and_every_bound_reported`, `failed_simulations_count_over_every_start`)
  must stay green unedited apart from the constructor change.

Risk: the sky130 `ss`/`ff` sections must exist in `$PDK_ROOT/.../sky130.lib.spice`. Check with
`grep -n '^.lib ss\|^.lib ff'` before writing the acceptance test. If ngspice fails to bias the ota at ss 125, report
it; do not drop the scenario.

---

## PERF-17 Signoff intent arms IR drop; EM/IR coverage visible

### Current code (corrections to plan-07 marked *)

- \* `Intent { supplies, currents }` is at `backend/verify/src/lib.rs:131-136` (the plan says :57-63) and derives
  `Default`. \* `Checker::set_intent` is at `backend/verify/src/checker.rs:106-137` (plan :123-128). Its `limits`
  are built from `currents` with `ua > 0` only.
- \* `elaborate::intent` is at `frontend/library/src/elaborate.rs:291-316` (plan :297-319). It loops over
  Supply/Ground nets and pushes `supplies` and `currents`. It is called at `lib.rs:445`.
- IR budgets: `annotator::ir::budgets(classes, current_ua, headroom_mv, supply_mv, policy) -> Vec<(NetId, i32,
  i64 /*µV*/)>` (`backend/annotator/src/ir.rs:21-50`). It is computed only inside the `if let` at
  `lib.rs:431-441` and mapped straight into `IrDrop` rules.
- GPurify (pinned `6341f18`, the same as `../GPurify` HEAD):
  - \* `ir_drop` with supplies declared but no drop limit already records `Outcome::Ran` with `examined == 0`
    (`electrical.rs:176-220`: a node counts only if its net states `max_drop`/`_fraction`/`overvoltage`). So "absent
    from `skipped_rules`" is **already true** whenever an op point exists, and that assertion alone cannot fail.
    The audit's G.3 skip was `bgr_core`, which had no op point (audit-06:260, 355). The real gap is
    `examined == 0`, and the test checks that.
  - `resolve_intent_into` (`facts.rs:154-189`) accepts limits on any labelled net. Only supply nets are grid
    nodes, so limits on signal nets examine nothing.
- Bench: `benchmarks/src/bench.rs:245-264` prints `skipped [...]`. There is no EM/IR count. `verify::Coverage`
  (`lib.rs:79-85`) has no run counts. `checker.outputs().runs` (with `examined`) is read at `lib.rs:306` for skips
  only.

### Edits

1. `verify::Intent` gains `pub max_drop_mv: Vec<(String, f64)>`. The doc says: allowed DC drop per net, mV, from
   the op headroom (`annotator::ir`).
2. `set_intent`: build `limits` as one JSON object per net, merging `budget_current_ua` (from `currents`,
   `> 0`) and `max_drop_mv` (`> 0`, finite) for the same net. GPurify refuses a net with two entries
   (`intent.rs:234-239`). Use a `BTreeMap<&str, serde_json::Map>` keyed by net name, so the order is deterministic.
3. `elaborate::intent(..., ir: &[(pnr_core::NetId, i32, i64)])`: for each Supply/Ground net it already visits,
   push `(name, max_drop_uv as f64 / 1000.0)` when `ir` has the net. Signal-net triples are not written:
   GPurify's grid examines only supply nets, so a limit there would arm nothing. Add a line to the doc saying so.
4. `lib.rs:431-441`: bind `let ir = match (&bias.currents, &bias.net_headroom_mv) { (Some(c), Some(h)) =>
   budgets(...), _ => Vec::new() /* missing pushed as today */ };`. Map `ir` into the `IrDrop` rules and pass
   `&ir` to `elaborate::intent` at :445.
5. `verify::Coverage` gains `pub em_ir: (u32, u32)` (rules run, rules in deck). It is filled at `lib.rs:306` from
   `checker.outputs().runs` whose rule name starts with `"EM"` or equals `"ir_drop"`; "run" means
   `Outcome::Ran && examined > 0`. Bench adds `| EM/IR ran {k}/{n}` after `skipped [...]` (bench.rs:245/264).
6. Fix the struct literal in `verify/src/lib.rs:548` (`..Default::default()`).

### Tests

- `backend/verify/src/lib.rs` `max_drop_arms_ir_drop` (sky130):
  - geometry: a met1 `VDD` rail with a pin (copy the shape of `intent_arms_the_em_rules`, :535-555);
  - `examined_ir = |c| c.outputs().runs.iter().find(|r| c.rule_name(r.rule) == "ir_drop").map(|r| (r.outcome,
    r.examined))`;
  - with `supplies` only, assert `examined == 0`;
  - with `max_drop_mv: vec![("VDD".into(), 10.0)]`, assert `outcome == Ran && examined > 0`, and assert
    `!skipped_rules().iter().any(|(r, _)| *r == "ir_drop")`.
  - Use no `currents`: a budget with no load on the grid is `Refused` (`power.rs:704-718`). If a bare rail does not
    solve (`Outcome::Skipped`), add a load the way the existing ihp EM test does, or report it.
- `verify` `set_intent_merges_a_nets_limits`: `currents [("VDD", 100)]` and `max_drop_mv [("VDD", 5)]`. Assert
  `set_intent` is `Ok`, so GPurify did not refuse a duplicate net.
- `frontend/library` `op_runs_carry_ir_limits` (in `perf_postlayout.rs`, behind `models()`):
  - ota with `op: Some(op_cfg)`, `feedback_iters: 1`;
  - assert `sol.intent.max_drop_mv` is non-empty and every name in it is a supply in `sol.intent.supplies`;
  - re-run `library::signoff` (lib.rs:1601 path) on the solution and assert `coverage.em_ir.0 >= 1` and that
    `coverage.skipped_rules` has no `ir_drop`.
- Commands: `cargo test -p verify max_drop`, `cargo test -p verify set_intent`,
  `cargo test -p library --test perf_postlayout op_runs`, `cargo build -p benchmark`.

### Acceptance

The G.3 skip log no longer lists `ir_drop` for circuits with an op point, and `ir_drop` now examines nodes
(`examined > 0`). Signal-net EM and `vgs.*`/`vds.*` stay skipped in GPurify (no per-net signal voltage in its intent)
and show in the `EM/IR ran k/n` column. In Philis, REL-03 and REL-10 cover them.
