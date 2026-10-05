# M2+ annotator segment 5 (M4): implementation cards

Branch `m2-annotator`, worktree `philis-m2/annotator`, after `git merge m2` (fast-forward to `a6d077d`, clean).
Order: EXT-21 then EXT-25 (EXT-25 reuses EXT-21's `margins`). Line numbers are of this tree; find code by symbol.
Every command needs `export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk`.

| Item | Class |
|---|---|
| EXT-21 | do (real-data acceptance waits for PERF-12 in `m2`; unit and annotate-level tests use synthetic `SpecSens`) |
| EXT-25 | do (library row swap, step L, only if `perf::to_evidence` is in the tree after `git merge m2`; else report it pending) |

DAG: both hard deps done (EXT-17 `Sensitivities`/`SpecSens` evidence.rs:47–72 incl. `d_cc`; EXT-18, EXT-20, EXT-24).
PERF-12/13 are runtime producers (dag-m2-m6.md "Runtime data, not code"). **PERF-12 is on `m2-perf` (`ec5dfae`), not
in `m2`**: it adds `perf::to_evidence(cfg, tables, start, stats, netlist) -> Sensitivities` (d_vt per mV, sign-fixed
per polarity; `proc` = scenario spread; `sigma_f` = max over bounds) and sets `Evidence.sens` before the
per-topology `annotate_with` (library lib.rs:650). Its card (m2-perf-4.md) leaves "rows from `Evidence.sens`" and
`PerformanceBudget.coupling` to EXT-25.

Batch rules: no new warnings in `annotator`, `analog`, `library`. A test not named here that goes red means stop and
report; do not edit its assertion. Tests whose expected value changes by design are listed per item.

---

## EXT-21 Sensitivity-driven allowance allocation

### Current code (plan corrections marked \*)

- \* `emit::set_budget` (emit.rs:50–59): allocated `allowance` is used only for `MatchKind::Voltage`; Current is
  `mismatch::choose` with the comment "Current/Ratio allowances wait for EXT-21". The plan's `emit.rs:46-52, 83-91`
  is stale. `Budget::Allowance(a)` is mV; `MatchedSet::budget_in` (matched_set.rs:232–237) converts it to % with
  `to_pct(gm/I)` on a % ledger (mismatch.rs:112–118), and a FET set gets a % ledger only when `g` is known
  (matched_set.rs:151). So **one mV ΔV_T allowance serves Voltage and Current FET sets**; no % conversion here.
- `MatchSpec.allowance: Option<f32>` / `weight: Option<f32>` (intent.rs:110–113) exist, always `None` (sets.rs:329).
- Class loop: lib.rs:267–301; `spec_6sigma` = `6·(offset_budgets | offset_sigma_mv)` (lib.rs:293–294). Set kinds are
  set at lib.rs:262–265, before the loop. `class::class_of` (class.rs:61) applies a spec only to mV limits.
- `sigma_pair(a_pair, a1, a2)` (mismatch.rs:10) is the pair σ; `ProcessNumbers.avt_mv_um: [nmos, pmos]`
  (netrole.rs:179); `emit::by_polarity` (emit.rs:63) picks the entry. `Device::gate_area_um2()` exists (pnr_core).
- \* `Policy` (policy.rs) has no `beta_target`; plan-01 names `policy.beta_target`. Added here.
- `substrate.rs:52` already reads `MatchSpec.weight` when present (EXT-23).

### Decisions

- **D1 scope**: allocate only `Family::Mos` sets (`d_vt` is keyed on FETs, PERF-12's `GateOffset`). Sets covered by a
  sidecar `OffsetBudget` (`cfg.offset_budgets`) are excluded: a user budget wins over a computed one.
- **D2 sides**: a set with halves compares Σ(half A) vs Σ(half B); otherwise `r = reference.unwrap_or(0)` against each
  other member. `S_jk` = max over compared pairs of `(|Σ_a d_vt| + |Σ_b d_vt|)/2`; `σ_k` = max over the same pairs of
  `sigma_pair(A_VT, Σ_a area, Σ_b area)` (areas `gate_area_um2`), `None` without `A_VT`.
- **D3 margin**: per finite bound `margin = headroom − β·σ_f` (σ_f `None` → no reserve), headroom floor
  `(proc.0 | f0) − lo`, ceiling `hi − (proc.1 | f0)`. Two-sided spec → two rows; the min in δ picks the tighter.
- **D4 cap**: `cap = max_eta·σ_k`; `σ_k None` → uncapped when a spec touches the set, `allowance None` when none does.
- **D5 class**: `spec_6sigma = user budget, else 6·allowance` only for `MatchKind::Voltage` (class_of's mV rule; a
  Current set's limit is %, unchanged). After `class_of`, a set with `src != User` and `weight < policy.minor_weight`
  becomes `(Minimal, ClassSource::Spec)`.

### Edits

1. `policy.rs`: add to `Policy` + `Default`: `beta_target: f64 = 3.0` (doc: "graeb_centering.txt L4266–4281, three-σ
   design"), `max_eta: f32 = 3.0` (Philis policy), `minor_weight: f32 = 0.01` (Philis policy, < 1 % of every spec's
   variance).
2. New `backend/annotator/src/allocate.rs` (`pub mod allocate;` in lib.rs):
   ```rust
   /// One allocatable set: compared sides (device lists) and its random 1σ, mV.
   pub struct SetIn { pub sides: Vec<(Vec<DeviceId>, Vec<DeviceId>)>, pub sigma_mv: Option<f32> }
   /// `(sign, margin)` per finite bound: sign −1 floor, +1 ceiling (D3). Shared with EXT-25.
   pub(crate) fn margins(s: &SpecSens, beta: f64) -> Vec<(f64, f64, f64 /* bound */)>;
   /// LAMP-09 split (plan-01 EXT-21 doc). Returns per set `(allowance mV, weight)`.
   pub fn allocate(sets: &[SetIn], sens: &Sensitivities, beta: f64, max_eta: f32)
       -> (Vec<(Option<f32>, Option<f32>)>, Vec<Diagnostic>);
   pub(crate) fn set_in(s: &MatchSpec, nl: &Netlist, avt: [Option<f32>; 2]) -> Option<SetIn>; // None unless Mos
   ```
   Steps in `allocate`: S[j][k] per margin row j (D2, `d_vt` looked up by device, absent = 0); `K_j = #{k: S>0}`;
   `M_j ≤ 0` → every touched set's δ = 0 and one `Diagnostic { kind: "spec_infeasible_at_schematic", devices: touched
   sets' devices, message: "{metric}:{min|max} margin {M} ≤ 0" }`; else `δ_k = min_j M_j/(K_j·S_jk)` then `min(cap)`
   (D4); `weight_k = max_j (S_jk·σ_k)²/σ_f,j²` when both known, clamp to 1.0.
3. lib.rs `annotate_with`, between lib.rs:265 and :267: if `let Some(sens) = &ev.sens`, build `SetIn` for Mos sets
   not covered by `cfg.offset_budgets` (`covering`), call `allocate(.., cfg.policy.beta_target, cfg.policy.max_eta)`,
   write `allowance`/`weight`, extend `intent.diagnostics`. In the class loop (lib.rs:293–295):
   `spec_6sigma: sigma.map(|v| 6.0 * v).or(s.allowance.filter(|_| s.kind == MatchKind::Voltage).map(|a| 6.0 * a))`;
   after `class_of`, apply D5's demotion.
4. emit.rs:52–53: `Some(a) if s.family == Family::Mos => Budget::Allowance(a)` and the comment states D1/the mV unit.
   Update the doc on `set_budget` and evidence.rs:15 ("nothing reads them yet" → EXT-21/25 read them).

### Tests

- `allocate::tests` (`cargo test -p annotator allocate`):
  - `allocation_respects_every_spec`: specs A (f0 0, hi 10) and B (f0 20, lo 12, σ_f 1, β 3 → M 5); 3 sets of one
    pair each; d_vt A: (+2,−2), (+0.5,−0.5), (0,0); B: (+1,−1), (+3,−3), (+0.2,−0.2); σ 1 each, max_eta 1e6. Assert
    `Σ_k S_jk δ_k ≤ M_j + 1e-9` for both, and set 3 (untouched by A) has δ = B's term.
  - `lampaert_table_4_2_ordering`: one spec (f0 20, lo 10, M 10), S = 12, 2.9, 2.9, 0.2 (d_vt (+S, −S)); σ 1,
    max_eta 1e9. Assert δ[3] > δ[1] == δ[2] > δ[0] and `(Σ S·δ − 10).abs() < 1e-6`.
  - `negative_margin_is_diagnosed`: f0 9, lo 10; two touched sets + one untouched with σ 2, max_eta 3. Assert touched
    δ == Some(0.0), untouched == Some(6.0), exactly one `spec_infeasible_at_schematic`.
  - `two_sided_spec_takes_the_tighter_bound`: lo 0, hi 10, f0 8, `proc` (7, 9) → margins 7 and 1; one set S 1 →
    δ == 1.0.
  - `weight_is_variance_share`: S 1, σ 1, σ_f 2 (margin positive) → weight 0.25.
- `tests.rs::ext21_ota_dp_allowance_below_load`: `ota()` with `process.avt_mv_um = [Some(5.0), Some(5.0)]`;
  `Evidence { sens: Some(..) }` one spec `offset` f0 0, hi 5, d_vt XM1 +1, XM2 −1, XM3 +0.2, XM4 −0.2. Assert DP
  allowance `(a − 2.5).abs() < 1e-4` (M 5, K 2, S 1; cap 3·5/√10 = 4.74 does not bind), DP < load allowance, both
  `Some`; without `sens` every allowance is `None` (today).
- emit.rs `set_budget_follows_source`: the last assertion changes **by design** (EXT-21 is what its comment waited
  for): `spec(Current, Role, Some(0.4))` → `Budget::Allowance(0.4)`; add `Family::Resistor` with `Some(0.4)` → `eta`.
- Gate: `cargo test -p annotator` and `cargo test -p library --lib` green.
- Acceptance (real data): after PERF-12 is in `m2`, run the library OTA perf test that prints `[perf] sens evidence`
  lines and report XM1/XM2 vs XM3/XM4 allowances; if PERF-12 is still absent, say so (not faked).

---

## EXT-25 Parasitic budgets derived from sensitivities

### Current code (plan corrections marked \*)

- \* `perf::budget_rows(cfg, start, tables, nets, af_per_nm)` is library perf.rs:613–635 (plan: 230–257); it already
  makes one row per finite bound with PERF-06's do-not-worsen row and keeps signs. Its tests: perf.rs:714
  `rows_turn_sensitivities_into_shares_of_the_headroom`, :755 `rows_cover_both_bounds_of_a_window_spec`, :767
  `a_missed_bound_keeps_a_do_not_worsen_row`, :777 `a_non_finite_bound_or_value_has_no_row`. Caller lib.rs:552.
- `PerformanceBudget` (kernel/analog/src/routing/performance.rs:22–33) = `metric, nets, weights, af_per_nm, limit`.
  \* **C12 (master §6) overrides the plan's struct**: RTE-21's union wins — `r_nets/r_weights`,
  `diff_pairs/diff_weights`, `coupling`, `limit`, `stack`; `series` is deleted; EXT-25 fills `r_*`, leaves
  `diff_pairs` empty. Construction sites: performance.rs:94, :110; rule.rs:681; library perf.rs:632, lib.rs:2623.
- Fallback budgets: `classify::net_load_af` (classify.rs:104–121) gives a drain-only net the smallest gate load (AA-25);
  called from `classify` (:87) and lib.rs:333. \* `missing` is `Vec<(&'static str, &'static str)>` (lib.rs:64): a
  per-net "external load of <net>" string is impossible; one static entry is used.
- Sidecar `Load` → `sidecar_unconsumed` (sidecar.rs:199–203); `AnnotationConfig` has no `loads` (netrole.rs:122–160).
- `RcClass { Unknown, None, R, C, Rc }` (intent.rs:199); `NetFacts.rc` set Unknown in `refine` (classify.rs:295),
  DAC plates by `extract` (extract.rs:318–321).
- \* `ProcessNumbers` has `wire_af_per_um` but no wire resistance; library builds it at lib.rs:1070–1074 with the
  lowest routing layer `wire` and its `width`; sheet ohms come from `pdk.pex_f32(layer, "sheet_res_ohm_sq")`
  (elaborate.rs:299).
- \* `BatchMeta.origin` is an `Origin` enum and cannot carry "credit_helpful"; the choice is stated in the kernel doc
  and in each row's note instead.

### Decisions

- **D1 no `Intent` argument**: `rows` is pure over `SpecSens`; R/C classes are returned and `annotate_with` writes them.
- **D2 struct**: add the C12 fields except `stack` (RTE-21 brings `Stack` measurement); add RTE-21's helper name
  `PerformanceBudget::ground_c(metric, nets, weights, af_per_nm)` (limit 1.0, new fields empty) so RTE-21 reuses it.
  `used()` unchanged (ground C); new `unknown(&self, _) -> u32 { u32::from(!r_nets.is_empty() ||
  !diff_pairs.is_empty() || !coupling.is_empty()) }` (RTE-21 replaces it once `stack` measures them). `touched` also
  lists `r_nets` and both nets of `coupling` entries with weight > 0. Merge with RTE-21: keep both sides; field names
  are identical by C12.
- **D3 sign**: `credit_helpful = false` keeps only `w > 0` (zero weights dropped too); the kernel doc's "Signs are kept"
  sentence becomes "Helpful terms are dropped unless `Policy.credit_helpful` (BAL2-16)".
- **D4 R/C class**: `|w_c|·C_ref ≥ rc_share` or `|w_r|·R_ref ≥ rc_share` over every row (w already divided by the
  row's scale, so this is the plan's `≥ rc_share·headroom`). `R_ref` unknown → R test skipped; a net seen only in
  `d_r` then stays `Unknown`. Written into `intent.nets[n].rc` after `extract` (sensitivity evidence wins over DAC
  structure only where it is not Unknown).

### Edits

1. policy.rs: `credit_helpful: bool = false`, `rc_ref_len_um: f64 = 100.0`, `rc_share: f64 = 0.05` (Philis policy).
2. performance.rs: D2 fields with dense docs (`r_weights` 1/Ω, `diff_weights` 1/aF on |C_a − C_b|, `coupling` 1/aF on
   C_ab, PERF-12 step 3), `ground_c` helper, `unknown`, `touched`; fix the 3 kernel construction sites with
   `..PerformanceBudget::ground_c(..)` / the helper.
3. New `backend/annotator/src/budget.rs` (`pub mod budget;`):
   ```rust
   pub fn rows(sens: &Sensitivities, af_per_nm: f32, r_ohm_per_um: Option<f32>, policy: &Policy)
       -> (Vec<analog::routing::PerformanceBudget>, Vec<(NetId, RcClass)>, Vec<Diagnostic>);
   ```
   Per `allocate::margins(s, policy.beta_target)` row: metric `"{metric}:min|max"`; headroom > 0 → `limit 1`,
   scale headroom; else `limit 0`, scale `|bound|` (0 → 1) plus `Diagnostic "no_layout_margin"`; non-finite bound or
   f0 → no row. `nets/weights` from `d_c`, `r_nets/r_weights` from `d_r`, `coupling` from `d_cc`, each `sign·d/scale`
   with D3. `C_ref = af_per_nm·1000·rc_ref_len_um`, `R_ref = r_ohm_per_um·rc_ref_len_um` (D4).
4. `annotate_with`: when `ev.sens` and `cfg.process.wire_af_per_um` are both `Some`, call `budget::rows` after
   `extract::routing` (lib.rs:366), write `.1` into `intent.nets[n].rc` (non-Unknown only), extend diagnostics with
   `.2`; `ev.sens` without wire C → `missing.push(("PerformanceBudget", "deck wire C"))`.
5. `ProcessNumbers.wire_ohm_per_um: Option<f32>` (doc: lowest routing layer, min width); library lib.rs:1074 adds
   `wire_ohm_per_um: wire.and_then(|l| pdk.pex_f32(<layer as elaborate.rs:299>, "sheet_res_ohm_sq")).filter(|&r| r >
   0.0 && width > 0).map(|r| r * 1000.0 / width as f32)`.
6. Fallback (AA-25): `AnnotationConfig.loads: Vec<(NetId, f32)>` (aF); sidecar `Load` → `{"net", "ff"}` resolved
   (unknown net → `sidecar_unknown_name` as the other entries), pushes `(net, ff·1000)`; only `Order` stays
   unconsumed (sidecar.rs:5 doc and :199–203). `net_load_af(hg, gate_um2, gate_af_per_um2, loads)`: load = Σ gate C
   + sidecar load; a net with neither → `None` (smallest-gate rule deleted, docs at classify.rs:41–47, 95–101 rewritten).
   lib.rs after :333: if `gate_af_per_um2` is known and some non-plate net with a channel terminal got `None` →
   `missing.push(("ParasiticBudget", "external load of drain-only nets: sidecar Load (AA-25)"))`.
7. Step L (library, **only if PERF-12 is in the tree**): performance_rows replaces `perf::budget_rows(..)` by
   `annotator::budget::rows(&evidence, af_per_um / 1000.0, ann.process.wire_ohm_per_um, &ann.policy).0` (evidence =
   `to_evidence(..)`, moved before the rows), deletes `perf::budget_rows` and its 4 tests (ported below), fixes
   lib.rs:2623 with the helper. Row notes (lib.rs:494) unchanged. Note in the report: `to_evidence` reads the tight
   bound's scenario table for both bounds (old code: each bound's own); `d_vt`-free rows otherwise identical.

### Tests (`cargo test -p annotator budget`, `cargo test -p analog performance`)

- `budget::tests` with a `spec(lo, hi, f0)` helper, `credit_helpful` default unless stated:
  - `two_sided_spec_gives_two_rows`: lo 100e6, hi 300e6, f0 200e6, d_c [(n0, −1e6)] → 2 rows `ugf:min`/`ugf:max`,
    floor weight `1e6/100e6 = 0.01`, ceiling row has no nets (its term helps).
  - `process_spread_shrinks_headroom`: proc (180, 230) with lo 100, hi 300 → `:min` weight `|d|/80`, `:max` `|d|/70`
    (use d of each sign so both rows keep a term).
  - `beta_reserve`: σ_f 10 → headrooms 70 and 70 at β 3 (from 100/100).
  - `conservative_split_drops_helpful_terms`: floor, d_c n0 +0.5, n1 −0.5 → only n1, weight `0.5/headroom`.
  - ported `rows_cover_both_bounds_of_a_window_spec`: [10, 20], f0 15, d −1 → with `credit_helpful = true` `:min`
    +0.2, `:max` −0.2, both `limit 1`; default → `:min` keeps n0, `:max` has no nets.
  - ported `a_missed_bound_keeps_a_do_not_worsen_row`: lo 10, f0 5, d −1 → one row `limit 0.0`, weight 0.1, one
    `no_layout_margin`.
  - ported `a_non_finite_bound_or_value_has_no_row` (NaN lo / ∞ hi; NaN f0) → empty.
  - `r_and_coupling_terms`: floor headroom 10, d_r [(n0, −0.2)], d_cc [(n0, n1, −0.1)] → `r_weights [0.02]`,
    `coupling [(n0, n1, 0.01)]`, `diff_pairs` empty.
  - `rc_class_by_share`: af_per_nm 0.05 (C_ref 5000 aF), r 0.5 Ω/µm (R_ref 50 Ω), headroom 100: d_c −1e-3 → |w|·C_ref
    = 0.05 → C; d_r −0.2 → 0.1 → R on another net; both on one net → Rc; d_c −1e-4 → None.
- performance.rs: `ground_c_helper_is_todays_row` (fields empty, limit 1); `extra_terms_are_unknown_not_zero`
  (`r_nets` non-empty → `unknown == 1`, `used` = ground part only).
- `tests.rs::drain_only_net_without_load_is_unknown`: `ota()` with gate_af 8325, wire 50: `vout2` (and `vtail`)
  `c_budget_af == None`, the AA-25 missing entry present; with `cfg.loads = [(vout2, 1e6)]` → `Some(_)`; `vout1`
  (drives gates) keeps `Some`.
- sidecar: `load_entry_sets_a_net_load` (`{"constraint":"Load","net":"vout2","ff":1000}` on ota5t → `loads ==
  [(vout2, 1e6)]`, no diagnostic). align_gold's `(4, 1, 5)` is unchanged (its unconsumed entry is `Order`).
- Gate: `cargo test -p annotator -p analog`, `cargo test -p library --lib`, then
  `cargo test --release -p library --test perf_postlayout` in the background; report any metric that moves from the
  drain-only budget change (the `budgeted` closure in library `d_router` setup drops nets that lost a budget) instead of retuning.
- Acceptance: AF-16 rows per bound (already PERF-06) now with conservative sign and β reserve; if step L could not
  land (PERF-12 absent), say so.

---

## Out of scope, reported

- RTE-21 owns `stack`, the R/pair/coupling measurement and `unknown` refinement (C12); EXT-25 only fills fields.
- `PerfConfig` may carry its own β (PERF-14); `Policy.beta_target` stays 3.0 until PERF exposes a knob.
