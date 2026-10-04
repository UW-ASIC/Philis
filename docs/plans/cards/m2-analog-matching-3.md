# M2+ analog-matching, segment 3 (M6): implementation cards (MAT-14, MAT-15, MAT-16, MAT-17, MAT-18, MAT-21)

Branch `m2-analog-matching`, base `12dcea0` (`git merge m2`, no conflict). Specs: `plan-02` MAT-14…18, MAT-21; DAG
entries from `dag-m2-m6.json` (on `main` only): no item has an unmet hard dep except MAT-18's gate. Line numbers are
this base's; the card wins over the plan. Order **MAT-14 → MAT-15 → MAT-16 → MAT-17 → MAT-21**, one commit each;
MAT-18 deferred. Every command: `export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk` first.

State on `m2` (grep/log): **EXT-17, EXT-19, PERF-27, CELL-15, CELL-21 are not merged**; `gm_over_id` is always `None`
(`backend/annotator/src/emit.rs:146`), so every bench MOS ledger is in mV; no split-DAC fixture in
`benchmarks/fixtures`; no `pair_sigma_mc`. `MatchedSet` struct literals: `emit.rs:124`, `backend/gp/src/lib.rs:777`
(test), `matched_set.rs:75` (`for_family`), `:337` (`pair`); `LedgerRow` literal: `matched_set.rs:303` only. New
`Coeffs` fields need no literal edits (`Coeffs` is `Default`, every site uses `..`).

---

## MAT-14 Thermal refinement — class: do (step 2's acceptance waits on EXT-17)

Facts (plan corrections in **bold**):

- ΔT today: `matched_set.rs:152-158`, `|rise(ca) − rise(cb)|` at the two unit centroids (2 `rise_at_point_mc` calls,
  `kernel/core/src/thermal.rs:86`, O(cells) each); `mu_thermal = tc·dt_mk·1e-6` (:167).
- **The plan's 7-point Taylor stencil is not accurate where it matters.** Python on the field model of
  `thermal.rs:56-75` (P/(2πkr)): for the existing fixture `thermal_reads_unit_centroids_of_a_merged_pair` (:532; 10 mW
  heater at x = 0, units at 2.5…17.5 µm) the brute-force unit average gives AABB ΔT 2087 mK / ABBA 1271 mK; the
  centroid samples give 1424 / 0; the stencil (h = max(1 µm, L/4)) gives 1110 / 553, i.e. **47 % and 56 % low**. It
  meets 10 % only in the far field (plan geometry: −1.9 % AABB, −3.6 % ABBA). The stencil exists to bound cost at 7
  evaluations; the exact unit average costs one evaluation per unit (4–16 per bench pair), the same order.
  **Decision: replace the centroid samples by the exact unit-weighted mean of `rise_at_point_mc` over each member's
  units** (no stencil, no Hessian). Pairs without units keep the 2-point centroid/cell-centre sample.
- Die temperature: `Config.op: Option<OpConfig>` (`frontend/library/src/lib.rs:94`), `OpConfig.temp_c`
  (`oppoint.rs:149`); **plan's `lib.rs:280` is stale**. `annotation(pdk, base)` (`lib.rs:776`) does not see `cfg.op`;
  callers at `lib.rs:370`, `:440` do.
- No polarity on `MatchedSet`; `emit.rs:61 by_polarity` maps Nmos/Pmos to a `[Option<f32>; 2]`.

Edits:

1. `matched_set.rs::member(l, d)` → `member(l, d, hot: bool) -> (Sums, f64, f64, f64)`; the 4th value is `Σw·rise(u)`
   (mK·weight) accumulated in the existing `inspect` closure when `hot`. `ledger_with` passes
   `hot = l.power_uw.iter().any(|&p| p != 0)`; `ledger_rows` (:293,298) and `pair_areas` (:225) pass `false`.
   `dt_mk` (signed `T̄_a − T̄_b`, then `.abs()`) = `ta/sa.w − tb/sb.w` when `units`, else today's centroid sample.
   `// ponytail: O(units·cells) per pair; the plan's 7-point stencil bounds it at 7·cells but is 50 % low within
   ~L of a heater (card m2-analog-matching-3).` Update the `Ledger` doc line `mu_thermal` (`mismatch.rs:179-188`):
   "TC·|ΔT̄|, ΔT̄ the unit-weighted mean rise over each member's units (centroids without units)".
2. `mismatch.rs`: `pub fn mobility_pct(exp: f32, dt_k: f32, t_abs_k: f32) -> f32 { -100.0 * exp * dt_k / t_abs_k }`
   (doc: k ∝ T^−exp, Hastings §12.1.1, PDF p.578; signed, ΔT of member a minus b).
3. `Coeffs` gains `pub mobility_exp: Option<f32>` (1.7 NMOS, 1.5 PMOS) and `pub die_temp_k: Option<f32>`.
   `emit.rs:131` sets `mobility_exp: by_polarity(nl, a, [Some(1.7), Some(1.5)])`, `die_temp_k: p.die_temp_k`.
   `ProcessNumbers` (`backend/annotator/src/netrole.rs:149`) gains `pub die_temp_k: Option<f32>` (doc: `Config.op`
   temperature, K; not a deck key). `lib.rs:370` and `:440`: `let mut ann = annotation(..);
   ann.process.die_temp_k = cfg.op.as_ref().map(|o| o.temp_c as f32 + 273.15);`.
4. Ledger (Family::Mos arm, after the % conversion at :170-173): when `unit == Pct && kind == Current`, with both
   coefficients `Some`, `mu_thermal += mobility_pct(e, dt_mk / 1e3, t).abs()` (magnitudes, open question 1). On an mV
   ledger the term is skipped (no G); doc says so.

Tests (`kernel/analog/src/placement/matched_set.rs`, `cargo test -p analog --lib matched_set`):

- Rename `thermal_reads_unit_centroids_of_a_merged_pair` → `thermal_averages_the_field_over_units`. The ABBA
  assertion `< 0.01` and the AABB pin `1.097` encode the centroid sampling this item removes; replace both by the
  field average computed in the test (`Σ rise_at_point_mc(u)/n` per member, ×765e-6): `|mu − brute| < 1e-3·brute`
  for AABB (≈ 1.597 mV) and ABBA (≈ 0.972 mV), plus `mu_abba > 0.5`. The cost-ratio assertion stays as is. This is a
  stricter check (exact average vs a pinned approximation), not a loosening.
- `thermal_matches_unit_sampling` (plan's `curvature_correction_matches_unit_sampling`): 1-row units at x = 0, 1, 2,
  3 µm (y 0, weight 10, one merged cell), 10 mW heater cell half-extent 1 µm at x = 8 µm. AABB: `mu` within 1e-3
  relative of the brute average (≈ 0.406 mV). ABBA: `delta_m_nm == 0.0` (centroids coincide), `mu` within 1e-3 of
  brute (≈ 0.0637 mV) and `> 0.05`.
- `mismatch.rs::mobility_term_sign`: `(mobility_pct(1.7, 1.0, 300.0) + 0.5667).abs() < 1e-3`; PMOS
  `mobility_pct(1.5, 1.0, 300.0) ≈ −0.5`.
- `matched_set.rs::mobility_adds_on_a_pct_mirror`: `pair(0,1)` with `abeta_pct_um: Some(2.2)`, `gm_over_id:
  Some(10.0)`, `mobility_exp: Some(1.7)`, `die_temp_k: Some(300.0)`, heater fixture → `mu_thermal` equals the
  `mobility_exp: None` value + `1.7·(ΔT̄/1000)/300·100` (ΔT̄ from the brute average) within 1e-4; on an mV ledger
  (`gm_over_id: None`) `mu_thermal` is unchanged by `mobility_exp`.
- Runtime: time `cargo test --release -p gp` and the `ota` flow (`cargo test --release -p library --test
  flow_smoke`) before/after; report the delta. If dp time grows > 10 %, report it with numbers (do not hide); the
  stencil remains the upgrade path.

Acceptance now: unit tests green. Step 2 changes no bench row until EXT-17 fills `gm_over_id`.

## MAT-15 Nth-order central-symmetric rows — class: do (bench acceptance waits on CELL-15)

Facts: `pattern.rs` has `diffusion_legal(s: &[usize], outer)` (:296) and `moments::cancelled_order(devs, nmax, tol)`
(`moments.rs:160`, **`debug_assert!(nmax <= 4)`, returns `[f64; 5]`: the plan's `nmax = 5` is impossible**). Cells
draw `rows ∈ {1, 2}` (`kernel/cells/src/mosfet.rs:44`, choice at :113-131; plan's `:152-156`/`:163-222` stale).

Edit, `kernel/analog/src/matching/pattern.rs`:

```rust
/// Rows of a two-member pattern (labels 0/1) cancelling gradient orders 1..=order:
/// P_1 = [row]; P_n = P_{n−1} stacked on rot180(P_{n−1}) (rows reversed, each row
/// reversed), labels swapped when n is even (NTH §III, nth_order.txt L94–119,
/// L154–222). 2^(order−1) rows. Vertical stacking only: every row stays `row` or
/// its label swap/reversal, so a diffusion-legal `row` stays legal.
#[must_use]
pub fn nth_order_rows(order: u8, row: &[u8]) -> Vec<Vec<u8>>
```

Loop `for n in 2..=order`; `debug_assert!` every returned row is `diffusion_legal(&row as usize, Outer::Drain)` when
`row` is. `order == 0` → `vec![]`.

Tests (`pattern.rs` mod tests, `cargo test -p analog --lib pattern`), row = `[0,1,1,0]`, units at (col, row) weight 1:

- `nth_order_rows_cancel_their_order`: order 1..=3 → `cancelled_order(&[&a, &b], 4, 1e-9).0 == order` and
  `r[order + 1] > 1e-6`; order 4 → `.0 == 4`. Rows count `== 1 << (order − 1)`; units per member `== 1 << order`.
- `order_three_is_nth_fig_3b`: `nth_order_rows(3, &[0,1,1,0]) == [[0,1,1,0],[1,0,0,1],[1,0,0,1],[0,1,1,0]]`.
- `nth_rows_are_diffusion_legal`: every row of orders 1..=4 satisfies `diffusion_legal(_, Outer::Drain)`.

Acceptance (`order ≥ 3` on an ota pair) is CELL-15's drawing; not claimable here.

## MAT-16 S(L) and anisotropy — class: do

Facts: one scalar `Coeffs.svt_uv_per_um` (`mismatch.rs:144`), set per set at `emit.rs:132` from
`ProcessNumbers.svt_uv_per_um` (`netrole.rs:167`, read `lib.rs:791`; key `pdks/sky130.json:115-116`, registry
`backend/verify/src/sidecar.rs:133`); ledger `sigma_grad = svt·|Δm|·1e-6` (`matched_set.rs:163`, isotropic `hypot`
:122); `sizing::dstar_um` reads the same field (`sizing.rs:64`). Gate L: `crate::param(dev, "l", 0)` nm
(`backend/annotator/src/lib.rs:306`).

**Decision: resolve S(L) at emission into the existing `svt_uv_per_um`** (one value per set, L known there), not a
`svt_fit` field on `Coeffs`: the ledger and `dstar_um` then use S(L) with no change.

Edits:

1. `mismatch.rs`: `pub fn svt_of_l(a_uv2_per_um2: f32, b_uv2: f32, l_um: f32) -> f32 { (a + b / (l*l)).sqrt() }` and
   `pub fn sigma_grad_mv(s_xy_uv_per_um: (f32, f32), dx_nm: f32, dy_nm: f32) -> f32 { (sx·dx).hypot(sy·dy)·1e-6 }`.
   `Coeffs.svt_uv_per_um` doc: "at this set's gate length (S(L) when the deck has the fit)"; `Coeffs` gains
   `pub svt_xy: Option<(f32, f32)>` (no deck key sets it; isotropic when `None`).
2. Ledger: keep `(dx, dy)` of `ca − cb`; `sigma_grad = sigma_grad_mv(c.svt_xy.unwrap_or((s, s)), dx, dy)` with
   `s = c.svt_uv_per_um.unwrap_or(0.0)` (identical to today when isotropic).
3. `ProcessNumbers` gains `pub svt_fit: Option<(f32, f32)>`; `lib.rs:791` adds
   `svt_fit: pos("svt_a_uv2_per_um2").zip(pos("svt_b_uv2"))`. `emit.rs:132`: `svt_uv_per_um: svt(nl, p, a)` with
   `fn svt(..) -> Option<f32>`: `p.svt_fit` and `l = param(dev,"l",0) > 0` → `svt_of_l(a, b, l/1000)`, else
   `p.svt_uv_per_um`. `annotator/src/lib.rs:84`: missing only when both are `None`.
4. `pdks/sky130.json`: `"svt_a_uv2_per_um2": 0.1835`, `"svt_b_uv2": 0.03533`, each with `_source` "PROXY: fit to
   Schaper & Linnenbank 130 nm Table 1 (S_D² = a + b/L², MM-25); derived, not sky130 data". `sidecar.rs`: two
   `k(.., Real, false, true, "frontend/library/src/lib.rs annotation")` rows next to :133. D* stays GAP-02/EXT-29's.

Tests:

- `mismatch.rs::svt_fit_reproduces_dvp_table_1`: `svt_of_l(0.1835, 0.03533, L)` = 1.624, 0.893, 0.580 at L = 0.12,
  0.24, 0.48 (± 0.01).
- `mismatch.rs::anisotropic_grad`: `sigma_grad_mv((1.0, 2.0), 1000.0, 1000.0)` = `5f32.sqrt()·1e-3` ± 1e-6.
- `backend/annotator/src/tests.rs::svt_fit_sets_s_of_l`: `ota()` with XM1/XM2 `l` param 480,
  `cfg.process.avt_mv_um = [Some(5.0), Some(6.0)]`, `svt_fit = Some((0.1835, 0.03533))`, `svt_uv_per_um = Some(1.63)`;
  take the diff-pair `MatchedSet` from `placement.budget` (`kind() == "MatchedSet"`), call `ledger_rows` on a two-cell,
  no-unit layout 1 mm apart: `sigma_layout / (1.63e-6·1e6) = 0.356 ± 0.005` (the acceptance ratio); without
  `svt_fit` the ratio is 1.0. `cargo test -p annotator svt_fit`.
- Registry: the existing sidecar key-coverage test must stay green (`cargo test -p verify sidecar`).

## MAT-17 Non-integer ratio segmentation — class: do (consumer wiring waits on EXT/CELL)

Facts: none exists (`grep segmentation` empty); resistor segment length `unit_l / n` in
`kernel/cells/src/resistor.rs` (plan's `:82`). Pure math, no deps.

Edit, `pattern.rs`:

```rust
/// Eq 8.27/8.28 sweep (Hastings, hastings.txt L23725–23859): for N in 1..=n_max,
/// M = round(N·R_M/R_N), S = |N/R_N − M/R_M| (1/Ω); unsorted, index N−1.
#[must_use]
pub fn segmentation(r_n_ohm: f64, r_m_ohm: f64, n_max: u16) -> Vec<(u16, u16, f64)>
/// Eq 8.29–8.33 at one R0: M = ⌊R_M/R0⌋, j = R_M/R0 − M, N = ⌊R_N/R0⌋, k = R_N/R0 − N,
/// S = |(N+1)/(N+k) − (M+1)/(M+j)| as printed (GAP-20: Fig. 8.20 swaps j and k).
/// Returns (M, N, j, k, S).
#[must_use]
pub fn partial_segments(r_n_ohm: f64, r_m_ohm: f64, r0_ohm: f64) -> (u16, u16, f64, f64, f64)
```

Tests (`pattern.rs`, `cargo test -p analog --lib segment`):

- `segmentation_reproduces_fig_8_19`: R_M 200 kΩ, R_N 146 kΩ, n_max 15; sorted by S, first three N = 8, 11, 3 with
  M = 11, 15, 4 and S = 2.05e-7, 3.42e-7, 5.48e-7 (± 0.01e-7). (Checked by hand: next is N = 5, 7.5e-7.)
- `partial_segments_reproduces_the_book_decomposition`: R0 10.34 kΩ → (19, 14, 0.342 ± 0.002, 0.120 ± 0.002); no
  assertion on S (GAP-20).

Acceptance "reported per resistor set" needs an EXT unitization consumer; not in this module.

## MAT-18 Split-DAC — class: defer

Gate unmet: the plan and the DAG (`"Gate: splitdac fixture (EXT-19, M2)"`) start it only when a split-DAC fixture
exists. `benchmarks/fixtures` has none (`dac4`, `dac4_mim` are single-bank; `cap_array.rs` `bits()` accepts
`[1,1,2,…]` only), EXT-19 is not on `m2`, CELL-21 (the only consumer of `split_dac_assign`/`nonunit_dims`) is not
merged. Writing the three functions now is unused code the plan itself forbids. Hard dep MAT-11 is done (segment 2).
Waits for: EXT-19 merged with a split-DAC fixture. The plan's tests (`attenuation_cap_formula` 8/7,
`nonunit_dims_keep_the_unit_edge_sensitivity`, `split_assign_is_exact_for_even_count_caps`) and `dac.rs` signatures
stand unchanged when it starts.

## MAT-21 SPICE MC σ override — class: do (MC values wait on PERF-27)

Facts: `LedgerRow` has **no `sigma_source` field** (`mismatch.rs:212-227`; the plan's MAT-13 text lists one, never
landed). σ_rand computed at `matched_set.rs:131-145`; `Ledger.allowance = budget_in(unit).allowance(sigma_rand)`
(:181). No producer (`pair_sigma_mc` absent).

Edits:

1. `MatchedSet` gains `/// MC 1σ of the pair, in the ledger's unit (PERF-27); replaces the area-law σ_rand.
   pub sigma_rand_override: Option<f32>`; add `sigma_rand_override: None` at the four literals (`emit.rs:124`,
   `gp/src/lib.rs:777`, `for_family`, `pair`).
2. `ledger_with`: after the `match self.family` σ block, `let sigma_rand = self.sigma_rand_override.unwrap_or(sigma_rand);`
   (allowance, `budgeted`, sizing flag follow automatically).
3. `LedgerRow` gains `pub sigma_source: &'static str` ("Pelgrom" | "MC"), set in `ledger_rows` from
   `sigma_rand_override.is_some()`; for non-MOS families the area-law label is still "Pelgrom" (doc: "area law").
   The plan's "PROXY" suffix is dropped: σ_rand reads A_VT/k_A, none of which is a PROXY key on sky130.
4. Report: if `MetadataReport`'s matched table is printed (PERF-23), add the column there; otherwise none.

Tests (`matched_set.rs`, `cargo test -p analog --lib matched_set`):

- `override_replaces_the_area_law`: `pair(0,1)` on `singles(1_000)`: no override → `sigma_rand ≈ 2.124 ± 1e-3`,
  row `sigma_source == "Pelgrom"`; with `sigma_rand_override: Some(1.5)` → `sigma_rand == 1.5`,
  `allowance ≈ 0.45 ± 1e-6` (`Eta(0.3)`), `ledger_rows(..)[0].sigma_source == "MC"`, `sigma_rand == 1.5`.
- `ledger_rows_one_per_pair` (:413) gains `assert_eq!(r.sigma_source, "Pelgrom")`.
- Downstream: `cargo test -p gp -p annotator -p library --lib` compile and stay green.

Acceptance (ratioed-mirror rows show MC, difference vs area law) waits on PERF-27 filling the field.
