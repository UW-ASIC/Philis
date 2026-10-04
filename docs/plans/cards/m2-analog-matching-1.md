# M2 analog-matching, segment 1: implementation cards (GAP-01, MAT-07, MAT-08, MAT-13)

Branch `m2-analog-matching`, worktree `philis-m2/analog-matching`, base `c2940c6` (`git merge m2`: already up to
date). Specs: `98-gap-critic.md` GAP-01, `plan-02` MAT-07/08/13, master plan §6 C1, C2, C20–C22, D12, D13; DAG
entries from `dag-m2-m6.json` (on `main` only, `18568e1`; not on `m2`). Line numbers are this base's; the card wins
over the plans. Order: **GAP-01 → MAT-07 → MAT-08 → MAT-13**, one commit each. Every command:
`export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk` first.

Segment-wide decisions (each a deviation, with its reason):

- A1. **Today's numbers are kept.** The `*_moderate` scalars are 3000 on every deck; GAP-01's table makes
  `lod_moat_ext_nm` MOD = 5000. Deleting the scalars therefore points each current reader at the tier that equals
  today's value: mosfet.rs moat and lib.rs `ose_range_nm` read `lod_moat_ext_nm[Minimal]` (3000; Hastings §13.2.2
  "two or three microns" is the MIN tier), WPE reads `wpe_clearance_nm[Moderate]` (3000). A 3→5 µm moat would
  change every multi-device MOS row on top of the open OTA-class regression (m1-report §4) and confound M2 item 0's
  re-measure. Class-correct readers are CELL-12 (`mosfet.rs`, `mos_env(class)`) and PLC-29 (`LiveEnvironment`, step 5).
- A2. `Pdk::tier` reads `self.cell` (the sidecar JSON `Pdk` already holds) directly; no `tiers` map in `parse_roles`.
- A3. `Problem.missing` is filled by the library (`solve`, next to the `IrDrop` push), not inside `mos_env`: `analog`
  has no `Problem`. `class::missing_tiers(p)` names each tier key with any class absent.
- A4. `Budget::from_class` takes a `ClassLimit`, not a bare `f32` (mV and % never share a float). No `Sigma1Pct`
  variant: Current/Ratio get `Eta(0.3)` until MAT-09/10 give a % ledger (spec step 1 says so).
- A5. `LedgerRow` drops `sigma_source` (only MAT-21, M6, has a second source) and keeps `unit = "mV"` (no % ledger
  before MAT-09/10). PERF's "Matched sets" bench table (MAT-13 step 4) is plan-07's (PERF-23), not this segment's.

---

## GAP-01 Match-class home, tier table, environment functions, class limit table — class: do

Facts (plan corrections in **bold**):

- No `MatchClass` anywhere. `pnr_core::Process` trait: `kernel/core/src/process.rs:7-91`; re-exports
  `kernel/core/src/lib.rs:25`.
- Readers of the scalars: `kernel/cells/src/mosfet.rs:306` (`lod_moat_ext_moderate`, LOD moat) and **`:644`**
  (`wpe_clearance_moderate`, PMOS nwell halo); `frontend/library/src/lib.rs:1077-1078` (**not 871-872**;
  `wpe_min_nm`, `ose_range_nm` in `fn environment`, :1012).
- Sidecars: `sky130.json` has `wpe_clearance_nm [2000,3000,5000]` (:60), `lod_moat_ext_nm [3000,5000]` (:78), scalars
  :87, :94; **`generic_finfet.json` has the same arrays** (:63, :80; scalars :89, :96); **gf180mcu and ihp_sg13g2
  have no arrays**, only the scalars (gf :81/:88, ihp :78/:85). No array has a `_source`.
- Registry `backend/verify/src/sidecar.rs:97-98, 134-135`: the scalars are `Nm, required`; the arrays `Tier, unread,
  not sourced`. `Kind::Tier` validation (:167) needs all-integer arrays.
- `parse_roles` (`backend/verify/src/pdk.rs:1366-1410`) keeps integer/bool scalars only; `Pdk.cell` (:55) keeps the
  whole JSON. `impl Process for Pdk` :1079; `Overlay` :1282-1334.
- `kernel/cells/tests/deck_keys.rs` `Recording` forwards every `Process` method by hand (:25-60); it must forward
  `tier` too or cells built under it lose their moat.
- `Problem.missing`: `backend/annotator/src/lib.rs:52`; library pushes at `frontend/library/src/lib.rs:440` (`solve`,
  `pdk` in scope).
- `Family` does not exist (MAT-04 used `mos: bool`, card D4): created here (DAG note). `MatchKind` lives in
  `kernel/analog/src/matching/mismatch.rs:20`.

Edits:

1. `kernel/core/src/process.rs` (after `SubstrateKind`, and the trait method at the end of `trait Process`):
   ```rust
   /// Hastings §13.3 matching class (PDF p.712): what a matched set's environment and limits scale with.
   #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
   pub enum MatchClass { Minimal = 0, #[default] Moderate = 1, Exceptional = 2 }
   /// Entry `c as usize` of the 3-element sidecar tier array `key`; `None` when absent or not an integer.
   fn tier(&self, key: &str, c: MatchClass) -> Option<i32> { let _ = (key, c); None }
   ```
   `kernel/core/src/lib.rs:25`: `pub use process::{MatchClass, Process, SubstrateKind};`.
2. `backend/verify/src/pdk.rs`: in `impl Process for Pdk`
   `fn tier(&self, key: &str, c: MatchClass) -> Option<i32> { self.cell.get(key)?.as_array()?.get(c as usize)?.as_i64().map(|v| v as i32) }`;
   `Overlay`: `fn tier(&self, key: &str, c: MatchClass) -> Option<i32> { self.pdk.tier(key, c) }`.
   `kernel/cells/tests/deck_keys.rs` `Recording`: same one-line delegation.
3. All four sidecars: delete `lod_moat_ext_moderate`, `wpe_clearance_moderate`; set the seven arrays of the GAP-01
   table (sky130 values on every deck; `lod_moat_ext_nm` becomes `[3000, 5000, 10000]`, C21) each with a
   `<key>_source` quoting the table's Hastings citation (`"Hastings §13.3 rule 19 (hastings.txt L42625–42631) …"`).
   Registry `sidecar.rs`: remove the two scalar rows; the seven tier rows `k(name, Tier, false, true, reader)`,
   alphabetical, readers: `wpe_clearance_nm` "kernel/cells/src/mosfet.rs; frontend/library/src/lib.rs;
   kernel/analog/src/matching/class.rs", `lod_moat_ext_nm` same, the other five "kernel/analog/src/matching/class.rs".
4. Readers (A1): `mosfet.rs:306` → `process.tier("lod_moat_ext_nm", MatchClass::Minimal).unwrap_or(0)`;
   `:644` → `process.tier("wpe_clearance_nm", MatchClass::Moderate).unwrap_or(0)`; each with a
   `// ponytail: today's 3 µm; CELL-12 reads mos_env(class)` comment. Citations there stay (CELL-12 fixes "r9").
   `lib.rs:1077` → `analog::matching::class::mos_env(MatchClass::Moderate, self.pdk).wpe_nm as f32`; `:1078` →
   `self.pdk.tier("lod_moat_ext_nm", MatchClass::Minimal).unwrap_or(0) as f32` (PLC-29 makes both per-pair).
5. New `kernel/analog/src/matching/class.rs` (`mod.rs`: `pub mod class;`), doc-commented per house style:
   ```rust
   pub use pnr_core::MatchClass;
   pub use crate::matching::mismatch::MatchKind;
   #[derive(Clone, Copy, PartialEq, Eq, Debug)]
   pub enum Family { Mos, Bipolar, Diode, Resistor, Capacitor }
   impl Family { pub fn of(k: pnr_core::DeviceKind) -> Option<Family> } // Nmos|Pmos→Mos, Npn|Pnp→Bipolar, Inductor→None
   #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)] pub enum GateStrap { #[default] PolyBar, PolyBarFar, MetalIsolated }
   #[derive(Clone, Copy, Debug, Default, PartialEq)]
   pub struct MosEnv { pub dummy_reach_nm: i32, pub moat_nm: i32, pub wpe_nm: i32, pub gate_ext_extra_nm: i32, pub gate_strap: GateStrap }
   pub fn mos_env(c: MatchClass, p: &dyn pnr_core::Process) -> MosEnv; // tier(...).unwrap_or(0); strap by class
   #[derive(Clone, Copy, Debug, Default, PartialEq)]
   pub struct PassiveEnv { pub min_dummies: u8, pub dummy_span_nm: i32, pub width_floor_permille: i32, pub length_floor_x: i32 }
   pub fn resistor_env(c: MatchClass, p: &dyn pnr_core::Process) -> PassiveEnv; // min_dummies 1; EXC count = ceil(span/pitch), caller's
   const TIERS: [(&str, &str); 7]; // (key, "<key> tier missing")
   pub fn missing_tiers(p: &dyn pnr_core::Process) -> impl Iterator<Item = &'static str> + '_; // any class None
   #[derive(Clone, Copy, Debug, PartialEq)] pub enum ClassLimit { Mv(f32), Pct(f32) }
   pub fn limit(f: Family, k: MatchKind, c: MatchClass) -> Option<ClassLimit>; // C2 table, 6σ values
   ```
   `limit`: Mos V `Mv` 10/3/1, Mos I `Pct` 10/3/1; Bipolar V `Mv` 2/0.5/0.1, I `Pct` 8/2/0.5; Resistor and Capacitor
   Ratio `Pct` 1/0.1/0.01; everything else (Diode included) `None`. `kernel/analog/src/intent.rs`:
   `pub use pnr_core::MatchClass;` (C1).
6. `frontend/library/src/lib.rs` `solve`, beside :440:
   `problem.missing.extend(analog::matching::class::missing_tiers(pdk).map(|m| ("MatchClass", m)));`.
   Step 5 of the spec (per-pair class in `LiveEnvironment`) is PLC-29's (C20, DAG step_level).

Tests:

- `backend/verify/src/pdk.rs` `sky130_tiers_are_read`: `load("sky130").tier("wpe_clearance_nm", Exceptional) ==
  Some(5000)`, `tier("lod_moat_ext_nm", Minimal) == Some(3000)`, `tier("lod_moat_ext_nm", Exceptional) ==
  Some(10000)`; `sky130_with(|c| { c.remove("dummy_reach_nm"); })` → `tier("dummy_reach_nm", Moderate) == None`.
- `kernel/cells/tests/deck_keys.rs` `mos_env_follows_the_table` (real sky130 sidecar): MOD → `MosEnv{3000, 5000, 3000,
  1000, PolyBarFar}`, EXC → `{10000, 10000, 5000, 1000, MetalIsolated}`; `resistor_env(Exceptional)` →
  `{1, 10000, 4000, 10}`; `missing_tiers(&pdk).count() == 0` on all four decks.
- `class.rs` `limits_match_hastings`: the 18 (family, kind, class) values above, `limit(Diode, Voltage, _) == None`,
  `limit(Mos, Ratio, _) == None`.
- `class.rs` `missing_tier_reads_zero_and_is_reported`: a test `Process` whose `tier` answers the table except
  `dummy_reach_nm` → `mos_env(Moderate, &p).dummy_reach_nm == 0`, other fields as the table,
  `missing_tiers(&p).collect::<Vec<_>>() == ["dummy_reach_nm tier missing"]`.
- Command: `cargo test -p verify -p analog -p cells -p library --lib` and `cargo test -p cells --test deck_keys`.

Acceptance: `grep -rn "_moderate\"" --include='*.rs' --include='*.json' kernel backend frontend pdks benchmarks` empty;
the tests above green; sidecar load (FLOW T3, `_source` on every sourced key) passes on all four decks; bench
unchanged (A1). CELL-12's class tests use `mos_env` (its own item).

## MAT-07 Class, kind and family on the matching rules — class: do

Facts: `placement::MatchedSet` (`kernel/analog/src/placement/matched_set.rs:22-38`) has `kind` and `mos: bool`
(read at :82; test helper `pair()` :214, test :329, :434); constructed in `backend/annotator/src/emit.rs:126-139` and
**`backend/gp/src/lib.rs:777` (test `gp_sees_power`)**. Φ/axis emission `emit.rs:143-145` (Axis hard, Phi budget).
`OrientationSet::kind()` = `"Orientation"` (`orientation.rs:78`). Citation "§13.3 r8" is at `environment.rs:14`
(doc of `Surroundings`). **MAT-07 step 5 is stale in place:** the "Not read by the library yet" sentence sits in
`vt_tc_uv_per_k_p_source` of sky130 (:113), gf180 (:22), ihp (:19), and `lib.rs:692` reads the key; FLOW-06 is gone
(D12), so GAP-01's sidecar commit drops that sentence.

Edits:

1. `class.rs`: `pub fn phi_arm(c: MatchClass) -> Option<bool>` (Exc `Some(true)`, Mod `Some(false)`, Min `None`;
   doc cites hastings.txt L42155–42186).
2. `MatchedSet`: replace `mos: bool` by `pub family: Family` and add `pub class: MatchClass`; :82 becomes
   `self.family == Family::Mos`. Update `pair()` (`family: Family::Mos, class: MatchClass::Moderate`), test :329
   (`s.family = Family::Resistor`), :434 and `gp/src/lib.rs:777` (`family: Family::Mos, class: Moderate`).
3. `emit.rs`: `family: Family::of(nl.devices[a.0 as usize].kind)`, and `continue` on `None` (only an inductor;
   such a pair is never matched). `class: MatchClass::Moderate` (EXT-20 sets it from `intent::MatchedSet`). Φ:
   `match phi_arm(set.class) { Some(true) => r.hard.push(orient(Phi)), Some(false) => r.budget.push(orient(Phi)), None => {} }`
   (computed before `set` moves). Axis stays hard.
4. `environment.rs:14`: "Hastings §13.3 r8" → "Hastings §13.3 rule 19".

Tests: `class.rs` `phi_arm_follows_hastings` (3 asserts). `backend/annotator/src/emit.rs` tests
`default_class_is_moderate_mos`: `emit::placement` on `crate::tests::ota()` with sky130-like `ProcessNumbers::default()`
→ `r.hard` has exactly `pairs` batches of kind `"Orientation"` and `r.budget` the same number (Axis hard, Phi
budget, nothing lost), where `pairs` = the `MatchedSet` count in `r.budget`. The class/family values are checked by
the `MatchedSet` unit test `default_pair_is_moderate_mos` on `pair()` and by a 2-line `Family::of` test
(`Nmos→Mos`, `Pnp→Bipolar`, `Inductor→None`). Command: `cargo test -p analog -p annotator -p gp --lib`.

Acceptance: tests green; `cargo run --release -p benchmark --bin bench -- local` rows for ota, ota_constrained,
tt_ota, bjt_mirror identical to the pre-change run on the same seed (every class Moderate; nothing else moves).

## MAT-08 Class-derived budgets — class: do

Facts: `emit.rs:61-63` `fn budget(offset_sigma_mv)` → `Sigma1Mv` or `Eta(0.3)` (**not 85-88**). Zero allowance is
already a violation once anything is spent (`Ledger::residual`, `mismatch.rs:104-109`, test
`zero_allowance_is_a_violation_not_nan`). `ClassSource`/`allowance` are EXT-12/EXT-20's (annotator, M2, not on this
branch); the budget choice (step 2) is applied inside EXT-20 (DAG step_level), so MAT-08 ships the rule as a pure
function and wires today's call through it.

Edits (`kernel/analog/src/matching/mismatch.rs`):

1. `impl Budget { pub fn from_class(limit: ClassLimit, kind: MatchKind) -> Budget }`: `(Mv(v), Voltage)` →
   `Sigma1Mv(v / 6.0)` (Hastings limits are 6σ, L42333–42350); anything else `Eta(0.3)` (A4).
2. `pub fn choose(offset_sigma_mv: Option<f32>, allowance: Option<f32>, class_limit: Option<ClassLimit>, kind:
   MatchKind) -> Budget`: offset → `Sigma1Mv`; else allowance → `Allowance`; else class limit → `from_class`; else
   `Eta(0.3)`. Doc: the caller passes `class_limit` only when the class source is User or Spec (a Role default at 3 mV
   would give 0.5 mV < sky130's 2.124 mV σ_rand and fail every Moderate pair). Move `GRADIENT_SHARE` (0.3) here as
   `pub const`; `emit.rs::budget` becomes `mismatch::choose(offset_sigma_mv, None, None, kind)`.
3. `Budget` gains `PartialEq` (for the tests). `sizing_limited` is MAT-13's field: `Sigma1Mv(b)` with `σ_rand ≥ b`.

Tests (`mismatch.rs`): `class_budget_is_one_sixth_of_the_limit` (`from_class(Mv(3.0), Voltage) == Sigma1Mv(0.5)`;
`from_class(Pct(3.0), Current) == Eta(0.3)`); `role_default_keeps_eta` (`choose(None, None, None, Voltage) ==
Eta(0.3)`; `choose(Some(1.0), Some(0.2), Some(Mv(3.0)), Voltage) == Sigma1Mv(1.0)`; `choose(None, Some(0.2), ..) ==
Allowance(0.2)`). Command: `cargo test -p analog -p annotator --lib`.

Acceptance (measured in MAT-13's test, which needs `LedgerRow`): ota with `offset_sigma_mv = 1.0` → input pair
(σ_rand = 9.5/√10 ≈ 3.0 mV > 1.0) has allowance 0, `sizing_limited`, and its `MatchedSet` status counts it violated
whenever it spends anything.

## MAT-13 Ledger rows in the report — class: do

Facts: `RuleBatch` `kernel/analog/src/rule.rs:172-272` (default hooks; `offset_allowances` :265 is the pattern);
`Tagged` forwards each hook (:276-350) and the annotator tags every batch (`annotator/src/lib.rs:217`), so the new
hook **must be forwarded in `Tagged`** or no row ever reaches the report. `Ledger` (`mismatch.rs:75-85`) has no
`order`/`phi_equal` (M1 card D3 left `order` to MAT-13). `MetadataReport` `frontend/library/src/metadata.rs:83`
(**not 71-86**; derives `Default`), `build` :246-279. Flow solution exposes `sol.metadata` (`lib.rs:150`).

Edits:

1. `mismatch.rs`: `#[derive(Clone, Debug, Default, PartialEq)] pub struct LedgerRow { pub members: (u32, u32), pub
   unit: &'static str, pub sigma_rand: f32, pub sigma_layout: f32, pub mu_thermal: f32, pub mu_lod: f32, pub
   allowance: f32, pub usage: f32, pub order: u8, pub second_order_nm: f32, pub phi_equal: Option<bool>, pub known:
   bool, pub sizing_limited: bool }` (A5).
2. `rule.rs`: `fn ledger_rows(&self, state: &On, out: &mut Vec<crate::matching::mismatch::LedgerRow>) { let _ = (state, out); }`
   on `RuleBatch`; `Tagged`: `self.inner.ledger_rows(s, out)`.
3. `MatchedSet::ledger_rows` (one row per pair `(0, i)`): from `self.ledger(l, i)` → `sigma_layout = sigma_grad`,
   `usage = g.usage()`, `unit = "mV"`; `member(l, ·)` sums → `phi_equal = units.then(|| moments::phi_equal(&sa, &sb))`;
   `order = cancelled_order(&[&pa, &pb], 4, 1e-3).0` on the members' unit `Pt`s (0 without units; report-only,
   allocation off the hot path); `sizing_limited = matches!(self.budget, Budget::Sigma1Mv(b) if g.sigma_rand > 0.0 && g.sigma_rand >= b)`.
4. `MetadataReport.matched: Vec<LedgerRow>`; `build` fills it from `placement.budget` only (each `MatchedSet` is
   pushed to budget and cost; reading both duplicates rows).

Tests:

- `matched_set.rs` `ledger_rows_one_per_pair`: `pair(0,1)` on the existing two-cell fixture → 1 row, fields equal the
  `ledger` values; with `budget: Sigma1Mv(1.0)` and gate area 20 µm² (σ_rand 2.124) → `allowance == 0`,
  `sizing_limited`.
- `frontend/library/tests/flow_smoke.rs` `matched_sets_are_reported`: `library::run(include_str!("../../../benchmarks/fixtures/ota.spice"), sky130,
  Config { feedback_iters: 1, outer_iters: 1, annotation: AnnotationConfig { offset_sigma_mv: Some(1.0), ..Default::default() }, ..Default::default() })`
  → `metadata.matched` non-empty; every field finite; `known` rows have `sigma_rand > 0`; the XM1/XM2 row (and any
  row with `sigma_rand >= 1.0`) has `allowance == 0 && sizing_limited` and `usage > 1.0` unless
  `sigma_layout + mu_thermal + mu_lod == 0` (MAT-08 acceptance). Command: `cargo test --release -p library --test
  flow_smoke matched_sets_are_reported` (background; debug flow is slow).

Acceptance: tests green; T7 ("Matched sets" rows on the 10 circuits) is read by PERF-23's table (plan-07), which
consumes `MetadataReport.matched`.

## Out of scope, reported

- `gp/src/lib.rs:777` is placement's crate; MAT-07 edits only the test struct literal (compile fix).
- `dag-m2-m6.json`/`.md` are on `main` (`18568e1`) but not on `m2`; the integrator should merge them.
