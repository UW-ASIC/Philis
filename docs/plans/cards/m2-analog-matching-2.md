# M2+ analog-matching, segment 2 (M3): implementation cards (GAP-02, MAT-09, MAT-11, MAT-12, MAT-10)

Branch `m2-analog-matching`, worktree `philis-m2/analog-matching`, base `c867abb` (`git merge m2`: fast-forward, no
conflict). Specs: `98-gap-critic.md` GAP-02, `plan-02` MAT-09/10/11/12; DAG entries from `dag-m2-m6.json` (on `main`
only). Line numbers are this base's; the card wins over the plans. Order **GAP-02 → MAT-09 → MAT-11 → MAT-12 →
MAT-10**, one commit each. Every command: `export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk` first.

Dependency state on `m2` (checked by grep/log): **EXT-17, EXT-19, EXT-20, FLOW-06, MAT-16, CELL-14, PERF-20/23 are
not merged**. No `Evidence`/`DeviceOp` type exists; the annotator emits only MOS pairs (`emit.rs:84-147`, every set
`MatchClass::Moderate`); `MetadataReport`'s `Display` does not print `matched` rows (that table is PERF-23's).
Consequences, applied per item below: the math, types, deck keys and unit tests land now; each acceptance clause that
needs an op point (EXT-17), a recognised R/C/BJT/bank set (EXT-19/20) or a bench table (PERF-20/23) is named as
waiting, not faked.

Segment-wide decisions:

- B1. One home for the arithmetic: every new formula is a pure `pub fn` in `kernel/analog/src/matching/{mismatch,
  sizing,dac}.rs` with its unit test there; `MatchedSet` (`kernel/analog/src/placement/matched_set.rs`) only
  dispatches. `sigma_pair` (`mismatch.rs:9`) is reused for every area-variance term (σ_β, R/C k_A, bipolar k_A: same
  `k·√((1/a₁+1/a₂)/2)` form).
- B2. Non-MOS areas come from `MatchedSet.gate_um2` (netlist/EXT-20 areas), never from unit weights: a resistor
  unit's weight is `body_w·seg_l/per²` (`kernel/cells/src/resistor.rs:222`), not an area. MOS keeps today's rule
  (unit weights nm² → µm² when units exist).

---

## GAP-02 Sizing-reach diagnostics — class: do (MAT-16 S(L) and the class-limit source are later inputs)

Facts (plan corrections in **bold**):

- `emit.rs:85-86` is stale: the η clamp is `Budget::allowance` (`mismatch.rs:61-67`, `Sigma1Mv` →
  `√max(0, b²−σ²)`); **the only report of it is `LedgerRow.sizing_limited`** (`matched_set.rs:229`), already asserted
  on `ota` with `offset_sigma_mv = 1.0` (`frontend/library/tests/flow_smoke.rs:60-80`: pair (0,1) σ_rand ≥ 1,
  allowance 0, `sizing_limited`).
- Report plumbing: `RuleBatch::ledger_rows` default no-op (`kernel/analog/src/rule.rs:269`), forwarded by `Tagged`
  (`rule.rs:349`), collected at `frontend/library/src/metadata.rs:271-272` into `MetadataReport.matched` (:108).
- No S(L) (MAT-16, M6): use `coeffs.svt_uv_per_um` (sky130 1.63, the worst-L proxy, `pdks/sky130.json:109-110`).
- No caller knows a user/spec class yet (all sets role-default Moderate; `mismatch::choose` doc :70-75 explains why a
  role default must not bind). **"area" therefore takes the class limit as an argument; the trait path passes `None`
  until EXT-20** (a Moderate default would note every pair under (6·9.5/3)² = 361 µm²).
- **Deviation from the spec signature**: `gm_over_id` dropped (the lever is chosen by kind, MM-21; nothing reads G);
  `span_nm`/`"xstar"` cut: for a pair x* = σ_rand/S equals D*'s role (both are `A/(S√WL)` for equal areas) and no
  caller has a multi-member span until ladders (EXT-19/MAT-10). `l: &Layout` added: `"dstar"`'s `have` is the placed
  centroid distance, which needs the layout.

Edits:

1. New `kernel/analog/src/matching/sizing.rs` (`matching/mod.rs`: `pub mod sizing;`):
   ```rust
   /// One sizing-reach diagnostic of pair `members` (schematic ids, reference first). Reported, never charged to Θ.
   #[derive(Clone, Debug, PartialEq)]
   pub struct SizingNote { pub members: (u32, u32), pub kind: &'static str, pub have: f32, pub need: f32, pub lever: &'static str }
   /// Hastings Table 13.4 area for a 6σ offset limit `dv_mv`: `(6·A_VT/ΔV)²`, µm² (H13-43, L42369–42407).
   pub fn area_need_um2(avt_mv_um: f32, dv_mv: f32) -> f32;
   /// Pelgrom D* = A_VT/(S_VT·√(WL)), µm (MM-06, pelgrom.txt L88–90); A mV·µm, S µV/µm → ×1000.
   pub fn dstar_um(avt_mv_um: f32, svt_uv_per_um: f32, wl_um2: f32) -> f32;
   /// Notes of every pair `(0, i)` of `set` on `l`; `class_limit_mv` = a user/spec class's 6σ mV limit (Voltage kind only).
   pub fn notes(set: &MatchedSet, l: &Layout, class_limit_mv: Option<f32>) -> Vec<SizingNote>;
   ```
   Per pair `g = set.ledger(l, i)`, `have_a = min(area0, area_i)` (same area rule as the ledger: expose the
   existing `area` closure of `ledger_with` (`matched_set.rs:109-111`) as `fn pair_areas(&self, l, i) -> (f32, f32)`
   and call it from both):
   - `"area"`: `kind == Voltage`, `class_limit_mv = Some(dv)`, `avt` known, `have_a < area_need_um2(avt, dv)` → lever
     `"area"` (the lever string carries the factor: `format!` is not `&'static`, so `need`/`have` carry it).
   - `"budget_area"`: `Budget::Sigma1Mv(b)` with `g.sigma_rand > b > 0` → `have = have_a`,
     `need = have_a·(σ_rand/b)²` (MM-14); lever `"area"` for Voltage, `"length"` for Current (MM-21).
   - `"dstar"`: `avt` and `svt` known and units present (`g.known`) → `have = g.delta_m_nm/1000` µm,
     `need = dstar_um(avt, svt, have_a)`; lever `"distance term non-binding"` when `have < 0.1·need`
     (`// ponytail: 0.1 is policy; the source gives only "ignorable below 100 µm²"`), else `"distance"`.
2. `rule.rs`: `fn sizing_notes(&self, state: &On, out: &mut Vec<crate::matching::sizing::SizingNote>) { let _ = (state, out); }`
   beside `ledger_rows` (:269), forwarded in `Tagged` (:349). `MatchedSet` impl:
   `out.extend(sizing::notes(self, l, None))` with `// ponytail: EXT-20 passes the intent class's limit`.
3. `metadata.rs`: `pub sizing: Vec<analog::matching::sizing::SizingNote>` after `matched` (:108), doc "one note per
   sizing-reach finding (GAP-02); report only"; fill at :272 with
   `placement.budget.iter().for_each(|b| b.sizing_notes(layout, &mut sizing));`. Bench printing is PERF-20's table.
4. `98-gap-critic.md` EXT-29 text: cite GAP-02 instead of "MAT-08 `SizingNote`" (spec's own note).

Tests (`sizing.rs` `mod tests`; `cargo test -p analog --lib matching::sizing`):
- `table_13_4_5v_exc`: `(area_need_um2(12.5, 1.0) - 5625.0).abs() < 1e-2`.
- `dstar_sky130`: `(dstar_um(9.5, 1.63, 100.0) - 583.0).abs() < 1.0` (582.8).
- `budget_below_random_gives_multiplier`: a Voltage `MatchedSet` (built like `matched_set.rs` tests' `pair`
  helper, :428-) with `avt` chosen so σ_rand = 2.124 (A 9.5, areas 20/20) and `Budget::Sigma1Mv(1.0)`, no units →
  exactly one `"budget_area"` note, `(n.need / n.have - 4.51).abs() < 0.01`, lever `"area"`; no `"dstar"` (unknown).
- `mirror_names_length`: same with `kind: Current` → lever `"length"`.
- `area_note_only_with_class_limit`: same Voltage set, `notes(.., None)` has no `"area"`; `notes(.., Some(3.0))` has
  one with `need = 361.0 ± 0.1`.
- Acceptance, `frontend/library/tests/flow_smoke.rs` (the existing ota test, after :79):
  `assert_eq!(sol.metadata.sizing.iter().filter(|n| n.members == (0, 1) && n.kind == "budget_area").count(), 1);`
  (`cargo test --release -p library --test flow_smoke`).

---

## MAT-09 Current-domain ledger and A_β — class: do (step 3, G from the op point, waits on EXT-17)

Facts:

- `cellgen.rs:615` / `lib.rs:793` refs are stale and irrelevant: the op point lives in `frontend/library/src/oppoint.rs`
  (`OpPoint.gm_us` :30, `id_ua`), reaches only `cellgen::folds` (`lib.rs:450`). EXT-17's `Evidence.op` does not exist.
- `grep abeta` → none. FLOW-06 not merged: MAT registers the keys itself (DAG D15/C23); `pos` helper is
  `frontend/library/src/lib.rs:742` (not 498/505), `ProcessNumbers` literal :743-759, struct
  `backend/annotator/src/netrole.rs:145`. Registry `backend/verify/src/sidecar.rs:64-65` (avt rows to copy).
- `Coeffs` `mismatch.rs:89-98`; `Ledger` :111-122; `LedgerRow.unit: &'static str` :135 (keep the field, fill from
  `LedgerUnit::as_str`); `MatchedSet` literals: `emit.rs:124`, `backend/gp/src/lib.rs:777`, six in
  `matched_set.rs` tests.

Edits:

1. `mismatch.rs`:
   ```rust
   #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
   pub enum LedgerUnit { #[default] Mv, Pct }
   impl LedgerUnit { pub fn as_str(self) -> &'static str }            // "mV" | "%"
   /// Current-domain random σ, %: √((0.1·G·σ_VT)² + σ_β²) (Hastings eq. 13.43; 1/V·mV ×100 = 0.1).
   pub fn sigma_current_pct(sigma_vt_mv: f32, g_per_v: f32, sigma_beta_pct: f32) -> f32;
   /// Voltage-domain random σ, mV: √(σ_VT² + (10·σ_β/G)²) (eq. 13.42).
   pub fn sigma_voltage_mv(sigma_vt_mv: f32, g_per_v: f32, sigma_beta_pct: f32) -> f32;
   ```
   `Coeffs.abeta_pct_um: Option<f32>`; `Ledger.unit: LedgerUnit`; `Budget::Sigma1Pct(f32)` (allowance
   `√max(0, b²−σ²)`, same arm as `Sigma1Mv`) and `Budget::to_pct(self, g_per_v: f32) -> Budget` (`Sigma1Mv(b)` →
   `Sigma1Pct(0.1·g·b)`, `Allowance(a)` → `Allowance(0.1·g·a)`, others unchanged). `from_class` stays as is
   (`Eta` for a `%` limit on an mV ledger); the % ledger alone maps a `ClassLimit::Pct(v)` to `Sigma1Pct(v/6)`
   (step 2), so a % limit never budgets an mV ledger.
2. `MatchedSet`: new field `pub gm_over_id: Option<f32>` (1/V of `members[0]`; doc "EXT-17's
   `gm_us/id_ua`; `None` keeps the ledger in mV"; `None` at every literal, `emit.rs` with
   `// ponytail: EXT-17 fills it from Evidence.op`). In `ledger_with` (:112-138), after the mV terms, when
   `family == Mos`, `abeta` and `gm_over_id = Some(g)` are both known: `sb = sigma_pair(abeta, a0, ai)` (B1);
   Current kind → `unit = Pct`, `sigma_rand = sigma_current_pct(σ_VT, g, sb)`, `sigma_grad`, `mu_thermal`, `mu_lod`
   ×`0.1·g`, allowance = `self.budget.to_pct(g).allowance(sigma_rand)`;
   Voltage kind → `sigma_rand = sigma_voltage_mv(σ_VT, g, sb)`, unit mV. Without A_β nothing changes (step 1 of the
   spec: usage is domain-invariant). `ledger_rows` (:218): `unit: g.unit.as_str()`; `sizing_limited` also matches
   `Sigma1Pct`.
3. `ProcessNumbers.abeta_pct_um: [Option<f32>; 2]` (doc like `avt_mv_um`), `lib.rs:749`-style
   `abeta_pct_um: [pos("abeta_n_pct_um"), pos("abeta_p_pct_um")]`; `emit.rs` `Coeffs { abeta_pct_um:
   by_polarity(nl, a, p.abeta_pct_um), .. }`. Registry: `k("abeta_n_pct_um", Real, false, true,
   "frontend/library/src/lib.rs annotation")`, same for `_p`, alphabetical (before `antenna…`).
4. `benchmarks/characterize_mismatch.py`: `bpv` mode (MM-17) beside `tc` in `main` (:123-126): per existing W/L
   (`devices`/`sim` helpers, :60-84), voltage-biased MC pairs at V_ov ≈ 0.1 and 0.4 V (V_GS = |VT|+V_ov from the
   tt `run` VT, V_DS = V_GS), σ(ΔI/I) % from the pair currents, G = g_m/I from a ±1 mV tt finite difference; fit
   `bpv_fit(rows)` = 2×2 normal equations of σ² = A_VT²·(0.1G)²/WL + A_β²/WL over all rows; print
   `A_beta_<n|p> = … %.um` and the refit A_VT (must agree with `avt_*` within the run's spread, else report).
   `selftest` mode: rows synthesised from A_VT 5, A_β 1 at two (G, WL) points → `bpv_fit` returns (5, 1) within
   1e-9 (`assert`). Run `bpv` for sky130 (and gf180/ihp if their models are installed); write
   `abeta_n_pct_um`/`abeta_p_pct_um` + `_source` (format of `sky130.json:105-108`: script, corner, pairs, seeds,
   geometries) to each deck the run covered; a deck not run stays without the key (unknown, not a guess).

Tests (`cargo test -p analog --lib matching`, `python3 benchmarks/characterize_mismatch.py selftest`):
- `mismatch.rs::current_domain_matches_hastings_13_43`: `(sigma_current_pct(1.0, 10.0, 0.5) - 1.118).abs() < 1e-3`;
  `(sigma_voltage_mv(1.0, 10.0, 0.5) - 1.118).abs() < 1e-3` (10·0.5/10 = 0.5 mV).
- `mismatch.rs::without_abeta_usage_is_domain_invariant`: a `Ledger` {σ_rand 2, grad 0.3, thermal 0.2, lod 0.1,
  allowance `Sigma1Mv(3).allowance(2)`} and its ×(0.1·10) image with allowance `Sigma1Mv(3).to_pct(10)
  .allowance(2·1.0)` → usages equal within 1e-6 (and G = 7 gives the same).
- `matched_set.rs::mirror_row_in_pct_with_abeta`: Current pair, units present (reuse the tests' layout helper),
  `avt 9.5, abeta 1.0, gm_over_id Some(10.0)` → row `unit == "%"`, `sigma_rand ==
  sigma_current_pct(σ_VT, 10, σ_β)` and `> 0.1·10·σ_VT + 1e-3` (β share non-zero); the same set with
  `gm_over_id None` → `unit == "mV"`, σ equal to today's. Voltage pair with A_β and G → `unit == "mV"`, σ > σ_VT.
- Acceptance (bench mirror rows in %): **waits on EXT-17** (no G in the flow); unit test above is its stand-in.

---

## MAT-11 Capacitor-array metrics: second moments, t0/t model, M_sys — class: do (step 4 waits on EXT-19/20)

Facts (plan line numbers stale):

- `ArrayMetrics` `kernel/cells/src/cap_array.rs:63-84` (`quad_um2` :72, `inl_lsb`/`dnl_lsb` :76-77);
  `metrics(&self, group, c, process, g, q)` **:234-276**, θ at 4 steps of 45° (:254), field `1 + g(…) + q r²`.
  Callers: only tests, **:708** (`metrics_rank_the_families`, :694-713) and **:885** (sky130 variants eprintln :892).
- The drawn 5-bit grid is `dims(5, false) = (4, 8)` (:181-184), not `grids()`.
- **The spec's ranking test is wrong on DNL** (re-derived here by a scratch run of the exact t0/t model, uniform
  pitch, steps 4·max(rows, cols), counts `[1,1,2,4,8,16]`, `centro_assign` Compact vs Dispersed):
  on 4×8, INL(Dispersed) < INL(Compact) at all four points (p 2/5 µm × g 1e-5/1e-4; e.g. p2 g1e-4 1.4201e-4 vs
  1.4259e-4), but DNL(Dispersed) > DNL(Compact) at p2 g1e-4 (1.4251e-4 vs 1.4247e-4) and on 6×6 too. The one-unit
  C0/C1 dominate both. **The test asserts INL only, on the drawn (4, 8) and (8, 4) grids**; DNL is reported.

Edits:

1. New `kernel/analog/src/matching/dac.rs` (`mod.rs`: `pub mod dac;`), units `(slot, x_um, y_um)` centred on the
   array centroid:
   ```rust
   pub fn gradient_caps(units: &[(u8, f64, f64)], slots: usize, g_per_um: f64, theta: f64) -> Vec<f64>; // Σ 1/(1+g(x cosθ+y sinθ))
   pub fn inl_dnl(units: &[(u8, f64, f64)], n_bits: u8, g_per_um: f64, steps: usize) -> (f64, f64);
   pub fn msys(units: &[(u8, f64, f64)], counts: &[u16], g_per_um: f64, steps: usize) -> f64;
   /// max over slots of ‖M_slot − M_array‖_F per unit, µm² (M = [xx, xy, yy] about the array centroid).
   pub fn second_um2(units: &[(u8, f64, f64)], slots: usize) -> f64;
   ```
   `inl_dnl` keeps today's code-transfer definition (`cap_array.rs:256-266`: slot 0 is the termination unit, bits
   1..=n, all codes, ideal-referenced) over θ = k·π/steps, k < steps. Doc cites DACP eqs 11/17–18/12, REV eqs 8–10 as
   in the spec, and "g per µm is a sweep reading of DACP's γ = 10/100 ppm".
2. `ArrayMetrics`: `quad_um2` → `second_um2: f64`; add `order: u8` (`moments::cancelled_order` over slots with ≥ 2
   units, `Pt { x, y, w: 1.0, phi: (0, 0) }`, nmax 4, tol 1e-3), `msys: f64`. `metrics(&self, group, c, process, g)`
   (drop `q`: the t0/t model has no curvature term) builds `units` from `pos` (:243) and calls the four functions
   with `steps = 4·max(rows, cols)` (rows/cols from `dims` or the general `grids` branch, as `build` picks them).
   Update both callers and the :892 eprintln (print `order`, `second_um2`, `msys`).
3. Step 4 (`LedgerRow` inl/dnl/msys for a bank) and the acceptance (`dac4` bench row): **wait on EXT-19/20** (no
   capacitor-bank set is emitted; `cellgen::dac_banks` :780 is cellgen-internal) **and PERF-20** (bench table).

Tests (`cargo test -p analog --lib matching::dac`, `cargo test -p cells --lib cap_array`):
- `cap_array.rs::metrics_rank_the_families`: drop the INL/DNL line (:711); keep route-spread and `vias.len() == 6`.
- `dac.rs::chessboard_inl_not_worse_on_uniform_pitch`: for `(rows, cols)` in `[(4, 8), (8, 4)]`, units from
  `centro_assign(&[1,1,2,4,8,16], rows, cols, Fill::Compact|Dispersed)` at pitch 2 µm, g 1e-4, steps
  `4·max(rows, cols)`: `inl(Dispersed) < inl(Compact)`.
- `dac.rs::radial_metric_no_longer_hides_anisotropy`: slot 0 at (±2, 0), slot 1 at (0, ±2) (equal ⟨r²⟩) →
  `(second_um2(&u, 2) - 8f64.sqrt()).abs() < 1e-9`.
- `dac.rs::msys_second_order_for_exact_cc` (hardens the spec's g → 1e-9 test, which passes for any bank): g 1e-4,
  pitch 2, counts `[2, 2]`: ABBA (x −3, −1, 1, 3) → `msys < 1e-6` (≈ 8e-8); AABB → `msys > 1e-5` (≈ 4e-4).

---

## MAT-12 Resistor ratio patterns and Table 8.5 goldens — class: do (step 3 PhiZero waits on EXT-19/20)

Facts:

- `centro_assign` `kernel/analog/src/matching/pattern.rs:70-155`, `Fill::Balanced` :20, tie rule "lower d" :145-146.
- **Every golden row of the spec's table is confirmed** by a scratch run of `centro_assign(c, 1, Σc, Balanced)`
  on this base: ABBA 2.0, ABA 1.0, AABAA 2.5, ABABABA 2.333, ABAAABA 0.0, AABAAAAAABAA 0.4, ABABAAABBAAABABA
  0.533, ABABA 1.667, ABAAAABA 1.333, ABBAAABBA 0.300, every one `exact`. |ΔM| = |⟨x²⟩_A − ⟨x²⟩_B| per unit,
  x in pitch units about the row centre.
- `resistor.rs` `greedy_centroid`/`builder.rs:269-314` belong to CELL-14 (not touched here). `OrientCheck` has only
  `Axis`, `Phi` (`placement/orientation.rs:18-25`): **no `PhiZero`**; step 3 needs EXT-19 (resistor sets) and EXT-20
  (emission) and is theirs to wire with `OrientCheck::Phi` semantics or a new variant — not this segment.

Edits (`pattern.rs`, after `centro_assign`):
```rust
/// Segment order of one resistor row (`centro_assign(counts, 1, Σc, Fill::Balanced)`): point-symmetric, second
/// moments balanced; `exact` false when ≥ 2 members have odd counts ([`scale2`] gives the exact doubled counts).
pub fn segment_row(counts: &[u16]) -> (Vec<usize>, bool);   // slots all filled: unwrap each to usize
/// Every count ×2 when ≥ 2 counts are odd, else unchanged.
pub fn scale2(counts: &[u16]) -> Vec<u16>;
```

Tests (`pattern.rs` `mod tests`; `cargo test -p analog --lib matching::pattern`):
- `table_8_5_every_ratio_is_exact_and_no_worse`: table rows `(counts, book, generated, dm)` for the ten ratios
  (`[2,2]`, `[2,1]`, `[4,1]`, `[4,3]`, `[5,2]`, `[10,2]`, `[10,6]`, `[3,2]`, `[6,2]`, `[5,4]`, strings and |ΔM|
  as above and the book's from the spec table). Per row: `segment_row(counts)` as A/B string `== generated`, `exact`;
  book string has the same counts; `cancelled_order(&[&a, &b], 4, 1e-3).0 >= 1` on the generated row's `Pt`s;
  `dm(generated)` within 1e-3 of the table and `dm(generated) <= dm(book) + 1e-9`.
- `scale2_makes_two_odd_members_exact`: `scale2(&[3,1]) == [6,2]`, `segment_row(&[6,2]).1`,
  `!segment_row(&[3,1]).1`, `scale2(&[4,1]) == [4,1]`.
- Acceptance on drawn arrays (bench `order ≥ 1`, PhiZero 0 violations): **CELL-14 + EXT-19/20** (DAG step_level).

---

## MAT-10 R/C/BJT/diode coefficients and ledgers — class: do (acceptance on bgr_core/dac4 waits on EXT-19/20)

Facts:

- `by_polarity` is `emit.rs:61-67` (not 107-113); `Family` exists (`class.rs:12-33`, GAP-01). No R/C/BJT keys in
  any deck (`grep k_a_pct\|bjt_ka\|vbe_tc` → none). Resistor recipes `pdks/sky130.json` `resistors` (~:135-178),
  `capacitors` :181-217. `verify::Recipe` (`backend/verify/src/pdk.rs:1232-1239`) keeps only integer `rules`:
  a per-recipe `1.70` would be dropped. **Nobody reads per-recipe/BJT coefficients until EXT-20 emits those sets**,
  so this item adds no `ProcessNumbers`/`Recipe` reader (EXT-20 adds it with its emission); keys are registered
  `reader "unread (EXT-20)"`.
- Die temperature is not modelled: V_T fixed at 25.7 mV (`// ponytail: 25 °C; REL-05 junction temperature`).

Edits:

1. `Coeffs` += `ka_pct_um`, `tc_ppm_per_k`, `vbe_tc_uv_per_k`, `sd_pct_per_mm` (all `Option<f32>`, doc units).
2. `mismatch.rs` pure fns: `bjt_sigma_vbe_mv(sigma_i_pct: f32) -> f32` (= 25.7·ln(1+σ/100)); `ratio_thermal_pct(
   tc_ppm_per_k, dt_mk) -> f32` (×1e-7); doc cites Hastings eqs 8.8/8.12/8.15/8.23/10.9 as the spec. σ_rand of both
   families is `sigma_pair(ka, a0, ai)` (B1).
3. `MatchedSet::for_family(members, family, kind, class, coeffs, budget, areas_um2, tol_nm) -> MatchedSet` (spec
   signature; `gate_um2 = areas_um2`, `cell_of` empty, `gm_over_id None`). `ledger_with` dispatches on `family`:
   `Mos` → today + MAT-09; `Resistor|Capacitor` → unit `Pct`, `sigma_rand = sigma_pair(ka, …)` (B2 areas),
   `sigma_grad = sd·Δm_nm·1e-6`, `mu_thermal = ratio_thermal_pct(tc_ppm, ΔT_mk)`, `mu_lod = 0`;
   `Bipolar|Diode` → unit mV, `sigma_rand = bjt_sigma_vbe_mv(sigma_pair(ka, …))`, `mu_thermal = vbe_tc·ΔT_mk·1e-6`,
   no grad/LOD term. `known` = units for both and `ka` given, or coincidence applicable (unchanged rule).
   The thermal `rise` closure (:122-126) is shared by all families.
4. Decks + registry (`sidecar.rs`): cell keys `bjt_ka_pct_um` 2.0 and `vbe_tc_uv_per_k` 2000, `Real`, sourced
   (`"textbook, not <pdk>: Hastings §10.2.1 PDF p.507"` / `"§9.1 PDF p.433"`) on all four decks.
   `characterize_mismatch.py res` mode (MC of each resistor recipe model at ≥ 4 W×L, fit σ vs 1/√A); if sky130's
   MC carries no resistor mismatch, write `"k_a_pct_um": 1.70` with a `PROXY` source (DVP Table 1, as the spec)
   in each sky130 resistor recipe; `tc_ppm_per_k` from the recipe model's `tc1` (cite the model file line) or absent;
   `sd_pct_per_mm` absent (no source); capacitor `k_a_pct_um` absent (no sky130 figure: unknown, not 0).

Tests (`mismatch.rs`, `cargo test -p analog --lib matching`; `cargo test -p verify --lib sidecar` for the registry):
- `hastings_eq_8_12_capacitor_pair`: `(sigma_pair(1.0, 1.0, 4.0) - 0.791).abs() < 1e-3`.
- `bjt_eq_10_9`: `s = sigma_pair(2.0, 36.0, 36.0)`, `(s - 0.333).abs() < 1e-3`,
  `(bjt_sigma_vbe_mv(s) - 0.0855).abs() < 5e-4`.
- `resistor_thermal_ppm`: `(ratio_thermal_pct(100.0, 500.0) - 0.005).abs() < 1e-7`.
- `matched_set.rs::for_family_dispatches`: a Resistor pair with `ka` and units → row `unit "%"`, `known`, `mu_lod 0`;
  the same without `ka` → `!known`; a Bipolar pair → `unit "mV"`, σ = `bjt_sigma_vbe_mv(..)`.
- Acceptance (`bgr_core`, `dac4` R/C/BJT sets reported, 0 silent passes): **waits on EXT-19/20** emitting the sets
  through `for_family`.
