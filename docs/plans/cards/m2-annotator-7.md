# M2+ annotator — segment 7 (M4): EXT-21, EXT-25

Base: m2-annotator after `git merge m2` (16b4190). Line numbers are of this tree; find code by symbol.

| Item | Class | Why |
|---|---|---|
| EXT-21 | do | Code landed in segment 5 (c756e32, ba220aa). PERF-12 (`perf::to_evidence`, ee23b2f) and PERF-13 (`robust::bound_stats`, 5feef4d) are now in m2, so the real-data acceptance can be written: one release test. |
| EXT-25 | do | Code landed in segment 5 (79c1c90) except step L (library row swap), which card 5 gated on PERF-12; PERF-12 is now in the tree. Plus one wiring gap: sidecar `Load` never reaches the flow. |

Status: no product edit of either item exists yet in this segment (git log/status clean after the merge).

---

## EXT-21 Sensitivity-driven allowance allocation

### Current code (plan text stale where marked \*)

- \* Plan signature `allocate(sets: &mut [MatchSpec], sigma_rand_mv, sens, beta, max_eta) -> Vec<Diagnostic>`; real one is
  `allocate::allocate(sets: &[SetIn], sens: &Sensitivities, beta: f64, max_eta: f32) -> (Vec<(Option<f32>, Option<f32>)>, Vec<Diagnostic>)`
  (backend/annotator/src/allocate.rs:47), `SetIn { sides, sigma_mv }` (:18), `margins` shared with EXT-25 (:27).
- Called from `annotate_with` (backend/annotator/src/lib.rs:312–325) when `ev.sens` is `Some`, skipping sets a
  sidecar `OffsetBudget` covers; writes `intent.sets[i].allowance/weight` (kernel/analog/src/intent.rs:111/113).
- `S_jk` per side = |Σ d_vt over the side's devices| (allocate.rs:55–56); a set's S = max over sides of (A+B)/2.
- Unit tests `allocation_respects_every_spec`, `lampaert_table_4_2_ordering`, `negative_margin_is_diagnosed` exist
  (allocate.rs tests) plus `tests.rs::ext21_*` (fixture `SpecSens`). Only real-data acceptance is missing.
- Library: `annotate_with(&netlist, &ann, &ev)` runs first without `sens` (frontend/library/src/lib.rs:433), then
  `ev.sens = plan.evidence` (:435) feeds the per-topology `annotate_with` (:719). So the flow already allocates from
  PERF-12 evidence; `to_evidence` maps `GateOffset` rows to `d_vt` per mV (perf.rs:644–700).

### Edit (test only, no product code)

`frontend/library/tests/perf_postlayout.rs`, new test next to `ota_exports_sensitivities`:

```rust
/// EXT-21 acceptance: on ota's real `d_vt`, the set with the larger offset sensitivity gets the smaller
/// allowance, and each uncapped set consumes the same share M/K of the margin (LAMP-09).
#[test]
fn ota_allowance_follows_offset_sensitivity()
```

Steps: `let Some(lib) = models() else { return };` `nl = ota()`, `p = cfg(lib)` (gain ≥ 20 dB, one scenario);
`start = evaluate(&nl, &Parasitics::default(), &p, &[0])`; `params = default_params(&nl, &nets)` (nets as in
`ota_exports_sensitivities`); `t = sensitivities(..)`; `mut e = to_evidence(&p, &[t], &start, &[], &nl)`.
Shrink the margin so the `max_eta·σ_k` cap cannot bind: `e.specs[0].lo = Some(e.specs[0].f0 - 0.01)` (test input,
not a threshold: 0.01 dB of gain against ≥ mV-scale σ_k caps). Annotate:
`annotator::annotate_with(&nl, &library::annotation(&pdk, &Default::default()), &annotator::Evidence { sens: Some(e.clone()), ..Default::default() })`.
Find the sets whose members are {XM1, XM2} (DP) and {XM3, XM4} (load); `s(k)` = (|d_vt(a)| + |d_vt(b)|)/2 from
`e.specs[0].d_vt`.

Assertions:
- `s_dp > 0.0 && s_load > 0.0` (both sets touched; else the fixture gives no evidence — fail, do not skip).
- both `allowance` `Some(a)` with `a > 0.0`.
- `(s_dp > s_load) == (a_dp < a_load)` — the plan's acceptance, both directions; `eprintln!` both S and δ.
- `(s_dp * a_dp - s_load * a_load).abs() <= 1e-3 * (s_dp * a_dp).max(s_load * a_load)` (equal share, uncapped;
  f32 storage gives the 1e-3).
- `K` = number of sets with S > 0; `s_dp * a_dp ≈ 0.01 / K` within 1e-3 relative (Σ S·δ = M exactly when
  uncapped; M = 0.01, no σ_f so no reserve).

If `a_dp` or `a_load` equals its cap (assertion 4 fails), report it with numbers; do not widen the tolerance.

Command: `PDK_ROOT=… cargo test --release -p library --test perf_postlayout ota_allowance_follows_offset_sensitivity -- --nocapture`
(background; needs ngspice + sky130 models, otherwise returns early as the file's other tests do).

Acceptance: plan's "DP allowance below the load's when the DP's offset sensitivity is larger" — tested in both
directions on real PERF-12 data.

---

## EXT-25 Parasitic budgets derived from sensitivities

### Current code (plan text stale where marked \*)

- \* Plan signature `rows(sens, intent, af_per_nm, r_ohm_per_um: f32, policy)`; real one is
  `budget::rows(sens: &Sensitivities, af_per_nm: f32, r_ohm_per_um: Option<f32>, policy: &Policy) -> (Vec<PerformanceBudget>, Vec<(NetId, RcClass)>, Vec<Diagnostic>)`
  (backend/annotator/src/budget.rs:28). \* Row struct is RTE-21's (C12: `r_nets/r_weights`, `diff_pairs`,
  `coupling`, `limit`, `stack`), no `series`.
- Steps 1–5 done: margins (allocate.rs:27), sign split + `credit_helpful` (budget.rs:31), R/C class written to
  `intent.nets[n].rc` (annotator lib.rs:493–503), drain-only nets → `None` + missing `"external load of drain-only
  nets: sidecar Load (AA-25)"` (annotator lib.rs:413), sidecar `Load` parsed into `cfg.loads` (netrole.rs:159).
  Tests in budget.rs:76–169 cover the plan's list incl. the ported `rows_cover_both_bounds_of_a_window_spec`,
  `a_missed_bound_keeps_a_do_not_worsen_row`, non-finite.
- **Not done, step L:** library still builds rows with `perf::budget_rows` (frontend/library/src/perf.rs:605–635),
  called at frontend/library/src/lib.rs:575, *before* `to_evidence` (:579). So the routed rows keep helpful terms,
  ignore `proc` and the β·σ_f reserve: AF-16/BAL2-16 hold in the annotator's R/C classes but not in the rows RTE uses.
- **Gap (bug):** the library's sidecar merge (frontend/library/src/lib.rs:407–420) copies every field except
  `loads` (and `order`, see out of scope), so a `{"constraint":"Load"}` entry never reaches `annotate_with`; the
  AA-25 fallback cannot be satisfied from the CLI.

### Edits

1. frontend/library/src/lib.rs `run`, sidecar block: add `ann.loads.extend(side.loads);` after
   `ann.offset_budgets.extend(..)`.
2. frontend/library/src/lib.rs `performance_rows`, `Ok(..)` arm: move `stats` and `evidence` above the rows, then
   ```rust
   let (rows, _, _) = annotator::budget::rows(&evidence, af_per_um / 1000.0, ann.process.wire_ohm_per_um, &ann.policy);
   ```
   (R/C classes and `no_layout_margin` are the annotator's, already emitted from the same evidence via :435/:719 —
   discarded here to avoid duplicates). `nets` stays used by `add_coupling`.
3. frontend/library/src/perf.rs: delete `budget_rows` (:605–635) and its tests
   `rows_cover_both_bounds_of_a_window_spec`, `a_missed_bound_keeps_a_do_not_worsen_row`,
   `a_non_finite_bound_or_value_has_no_row` (ported in budget.rs) and helper `one_net`. Rewrite
   `rows_turn_sensitivities_into_shares_of_the_headroom` to compose the new path (below). Fix doc links to
   `perf::budget_rows` (lib.rs:497) → `annotator::budget::rows`.
4. lib.rs:520 note text `row with no measured nets` → `row with no adverse measured nets` (with the conservative
   split an empty row also means "every term helps"); update the comment at perf_postlayout.rs:115 to match.

Behaviour change to state in the report: rows are read at the tight bound's scenario for both bounds of a spec
(`to_evidence`), helpful terms are dropped (default `credit_helpful = false`), headroom loses `proc` and β·σ_f.

### Tests

- perf.rs `rows_turn_sensitivities_into_shares_of_the_headroom` (rewritten, same fixture: UGF 400 MHz vs ≥ 300 MHz,
  power 0.5 vs ≤ 1, gain 5 vs ≥ 10; rows n0 linear, n1 linear with `None` for power, n2 nonlinear):
  `let ev = to_evidence(&cfg, &[t], &start, &[], &pnr_core::Netlist::default());`
  `let (rows, _, diags) = annotator::budget::rows(&ev, 0.1, None, &annotator::policy::Policy::default());`
  - `rows.len() == 3`; `rows[0].nets == [n0]` (n1 weight 0 not adverse, n2 nonlinear out), weight 0.01 ± 1e-7;
  - `rows[1].nets == [n0]`, weight 0.02 ± 1e-7 (n1 unmeasured);
  - `(rows[2].metric, rows[2].limit) == ("m:min", 0.0)`; `diags` has one `no_layout_margin`.
- frontend/library/src/lib.rs tests, new `sidecar_loads_reach_the_annotator`: `crate::run` on
  benchmarks/fixtures/ota.spice (sky130, `starts: 1, outer_iters: 1, feedback_iters: 1`), once without
  constraints → `sol.metadata.missing` contains `("ParasiticBudget", "external load of drain-only nets: sidecar Load (AA-25)")`;
  once with `constraints: Some(r#"[{"constraint":"Load","net":"vout1","ff":100},{"constraint":"Load","net":"vout2","ff":100},{"constraint":"Load","net":"vtail","ff":100}]"#)`
  → that entry absent. Fails today (edit 1 missing).
- Gates: `cargo test -p annotator`, `cargo test -p library --lib`, then in the background
  `cargo test --release -p library --test perf_postlayout`. `a_flow_scores_its_layout_in_simulation` asserts
  `gain:min: row (` (non-empty nets): if the conservative split empties the gain floor row on ota, that test fails —
  report the evidence line (`d_c` signs) to the owner; do not change the assertion. The known
  `ota_probe_regions` failure is pre-existing (m2-annotator-report.md).

Acceptance: AF-16 rows per bound with conservative sign, process spread and β reserve now in the rows RTE prices
(was only in the annotator's R/C classes).

---

## Out of scope, reported

- Library sidecar merge also drops `side.order` (EXT-28, lib.rs:407–420): user `Order` entries never reach the
  flow. One line, `ann.order.extend(side.order);` — annotator owns EXT-28; fold into a later review pass.
- RTE-21 owns the `r_*`/`coupling` measurement (`unknown` until then) and `stack` (C12).
