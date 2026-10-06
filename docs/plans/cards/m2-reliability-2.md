# M2+ reliability, segment 2 (M3): REL-10, REL-04, REL-05, REL-14, REL-07

Branch `m2-reliability` @ `a8c8f3d` (merge of `m2`: fast-forward, no conflicts). Specs: `plan-06` "### REL-xx";
`98-gap-critic.md` GAP-17 (REL-10 steps 4–5); DAG `dag-m2-m6.json` (untracked in the main checkout, read there).
Line numbers below are on `a8c8f3d`. Env for every command: `export PDK_ROOT=/home/omare/Documents/Projects/Rust/Philis/.pdk`.

| Item | Class | Waits on |
|---|---|---|
| REL-10 | do (steps 1–3, step 4 report half) | step 4's hard row: a per-pair match class on `m2`; step 5: GAP-17 step 1 `OpPoint.net_v` (perf) |
| REL-04 | do | — |
| REL-05 | do (steps 1–3, 6; steps 4–5 landed with GAP-06, `f4c646b`) | — |
| REL-14 | do, after REL-05 | REL-05 (same module) |
| REL-07 | do (Clock-class fallback) | EXT-23/GAP-03 tags and CELL-17 arms only widen it later |

---

## REL-10 Operating-point voltage checks. Class: do

### Current code (plan corrections)
- `OpPoint.vgs_v/vds_v/vbs_v` exist (`frontend/library/src/oppoint.rs:33-37`, filled `:278-280`); **no `net_v`** on
  `m2` or on the `perf` worktree (`f619a92`), so GAP-17 step 1 is not in.
- sky130 rules are **`pdks/decks/sky130.deck:494-498`** (plan: `GP/pdks/sky130.deck:482-486`); the `let` lists are
  `:487-493`. gdsverify (`6341f18`) stores `models:` as one `(model, ParamValue::Model(StrId))` param per list entry
  (`crates/ingest/src/deck/parse.rs:1433-1449`), `max` as `max_voltage` `ParamValue::Ratio` in mV (`kinds.rs:68-69`,
  572-587). Read all of them with `self.deck.rules.params_of(spec)` (`RuleTable::params_of`), resolve with
  `self.strings.resolve(id)`; `RuleTable::param` returns only the first model.
- `signoff` is **`frontend/library/src/lib.rs:1599-1604`** (plan: `:1289`); the undrawable rows are added at `:1602`.
  `Solution` is `lib.rs:142-163` and already carries `op: Option<OpPoint>` (`:162`). It is built at `lib.rs:628-640`.
- The 2-device leaf filter (`leaves`, `&[a, b]`, `DiffPair | CurrentMirror | Load`) already appears twice (`lib.rs:974-1000`,
  `:1013-1055`). `MetadataReport` is `metadata.rs:83-121`.
- **Step 4 does not need `net_v`.** ΔV_BS comes from `vbs_v` (correcting the DAG note). Its hard row is gated on
  "class ≥ Moderate", but `m2` has no per-pair match class. `MatchClass` is on `m2-analog-matching` (`d4332d9`,
  `kernel/analog/src/matching/class.rs`), unmerged, and no annotator output tags a pair with it. So `dvbs_mv` is
  reported now and `rel/vsb_mismatch` waits.

### Edits
1. `backend/verify/src/pdk.rs`, next to `em_limit`:
   ```rust
   /// One FET model's voltage ratings, mV (deck `gate_oxide` / `drain_source` rules).
   #[derive(Clone, Debug, PartialEq)]
   pub struct FetLimit { pub model: String, pub vgs_max_mv: Option<f32>, pub vds_max_mv: Option<f32> }
   /// Every `gate_oxide` / `drain_source` rule's `max_voltage` per model it names, merged by model
   /// (a model in both kinds gets both fields; two rules of one kind keep the tighter).
   #[must_use]
   pub fn fet_voltage_limits(&self) -> Vec<FetLimit>
   ```
   Loop `self.deck.rules.spec`; kind via `self.strings.get("gate_oxide")`/`("drain_source")`; `max` =
   `param(s, strings.get("max_voltage")?)` as `Ratio`; models = `params_of(s)` entries named `model` with
   `ParamValue::Model(id)` → `self.strings.resolve(id)`. Merge into a `Vec<FetLimit>` by `model` (`f32::min` on a repeat).
2. New `frontend/library/src/reliability.rs` (`pub mod reliability;` in `lib.rs`):
   ```rust
   /// A recognised matched pair's bias asymmetry (report only: no source gives a threshold, H05-33/35).
   pub struct PairAging { pub a: DeviceId, pub b: DeviceId, pub dvds_mv: f64, pub dvgs_mv: f64, pub dvbs_mv: f64 }
   pub fn voltage_findings(netlist: &Netlist, op: &OpPoint, limits: &[verify::FetLimit],
                           pairs: &[(DeviceId, DeviceId)]) -> (Vec<Violation>, Vec<PairAging>, usize)
   ```
   For each `Nmos | Pmos` device `i`: if `vgs_v[i]`, `vds_v[i]` or `vbs_v[i]` is `None`, or no `FetLimit` has
   `model == dev.model`, count it unknown and skip the rating checks. Otherwise:
   - `|vgs|·1e3 > vgs_max_mv` → `Violation { rule: format!("rel/vgs:{name}"), margin: over_mv.ceil() as i64 }`. Same for
     `vds` (`rel/vds:`). A `None` field on a found limit is not checked and not counted as unknown, because the deck
     states no such rule.
   - Bulk forward (tolerance 1e-3 V): NMOS `vbs > 1e-3 || vbs - vds > 1e-3`, PMOS `vbs < -1e-3 || vbs - vds < -1e-3` →
     `rel/bulk_forward:{name}`, margin `ceil(max excess mV)` (≥ 1). This check needs only the voltages, not a limit,
     so it also runs on a FET counted unknown for lack of a limit.
   - Pairs: `PairAging` with `|Δvds|`, `|Δvgs|`, `|Δvbs|` in mV, when both members have all three voltages.
3. `lib.rs`: add `fn matched_pairs(blocks: &[annotator::Block]) -> Vec<(DeviceId, DeviceId)>` (the filter above; the
   two existing loops are left as they are). `Solution` gains `pub pairs: Vec<(DeviceId, DeviceId)>`, filled at `:628` from
   `matched_pairs(&flow.problem.blocks)`. `signoff` (`:1599`): after the undrawable rows, when `sol.op` is `Some`,
   `s.report.hard_violations.extend(reliability::voltage_findings(&sol.netlist, op, &pdk.fet_voltage_limits(), &sol.pairs).0)`.
   In `solve`, after `metadata.binding` (`:600`), when `bias.op` is `Some`, set
   `metadata.aging` (`Vec<(String, String, f64, f64, f64)>`: names, ΔV_DS, ΔV_GS, ΔV_BS mV) and `metadata.voltage_unknown: usize`.
   Both fields go on `MetadataReport` (`Default`-able: empty, 0). Per-epoch scoring (`:882`) is unchanged.
   `netlist` at `:628` includes inserted antenna diodes. These are not FETs, so the counts do not change.

### Tests
- `backend/verify/src/pdk.rs` `sky130_fet_ratings_come_from_the_deck`: from `load("sky130").fet_voltage_limits()`,
  `nfet_01v8 == (Some(1950.0), Some(1950.0))`, `nfet_g5v0d10v5 == (Some(5500.0), Some(11000.0))`,
  `esd_nfet_g5v0d10v5 == (Some(5000.0), Some(11000.0))`, `pfet_01v8_hvt` present. All names are the full `sky130_fd_pr__…` form.
- `frontend/library/src/reliability.rs` `mod tests`, using a 1–2 device `Netlist` built inline and `OpPoint { vgs_v, vds_v, vbs_v, ..Default::default() }`.
  The limit is `FetLimit { model: "sky130_fd_pr__nfet_01v8", vgs_max_mv: Some(1950.0), vds_max_mv: Some(1950.0) }`.
  - `a_gate_over_its_rating_is_a_violation`: vgs 2.5, vds 0.9, vbs 0 → exactly one violation, `rule` starts with
    `rel/vgs:`, `margin == 550`.
  - `a_forward_biased_nmos_bulk_is_a_violation`: vbs +0.2, vds 0.9 → one `rel/bulk_forward:`; vbs 0 → none; model
    `"no_such_fet"` → `.2 == 1` and no `rel/vgs:` row.
  - `pair_aging_reports_the_vds_difference`: vds 0.9 vs 0.6 (same vgs, vbs) → `dvds_mv == 300.0 ± 1e-9`, `dvbs_mv == 0.0`.
- Command: `cargo test -p verify fet_ && cargo test -p library reliability`.
- Acceptance (T9): `cargo run --release -p benchmark --bin bench local ota` with its testbench bias reports
  `aging`/`voltage_unknown`. A probe bias already makes `certified()` false (`metadata.rs:160`).

### Deferred inside the item
- Step 4 hard row `rel/vsb_mismatch:{a}/{b}` (bulk nets differ or |ΔV_BS| > 1 mV, class ≥ Moderate). It waits for a per-pair
  `MatchClass` on `m2` (GAP-01 merge plus an annotator field that carries it). `dvbs_mv` is reported now.
- Step 5 (capacitor `v_max_mv`, |V_P − V_N|) waits for GAP-17 step 1 `OpPoint.net_v` (perf module, M3) and a
  `v_max_mv` recipe key. That key is null on every deck, so until it exists every cap would count as unknown.

---

## REL-04 IR drop, terminal-resolved. Class: do

### Current code
- `kernel/analog/src/routing/ir.rs:30-36` `IrDrop::drop_uv` = `current_ua × st.path_resistance_ohm(shapes)` over
  `r.shapes(net)` only. REL-03 already built `current::net_flow` (`current.rs:34`). Its `NetFlow.drop_uv` (`:14`, `:136`)
  is the tree diameter of I_e·R_e. EM calls it on `[r.shapes(net), r.cell_metal(net)].concat()` with `r.terminals(net)`
  (`em.rs:121-123`).
- Callers: `frontend/library/src/lib.rs:436` (live), `backend/dr/src/lib.rs:2652` (a test with no terminals).

### Edit (`ir.rs` `drop_uv` only)
```rust
fn drop_uv(self, r: &Routes) -> Option<f32> {
    let st = self.stack?;
    let shapes = r.shapes(self.net);
    let all = [shapes, r.cell_metal(self.net)].concat();
    super::current::net_flow(st, &all, r.terminals(self.net)).map(|f| f.drop_uv)
        .or_else(|| (self.current_ua >= 0 && !shapes.is_empty()).then(|| self.current_ua as f32 * st.path_resistance_ohm(shapes)))
}
```
- **Decision (differs from the plan):** fall back to the bound whenever `net_flow` is `None`, not only on empty
  terminals. An unknown terminal current or an open terminal would otherwise turn a net that is known today into an unknown one, which
  breaks "nets keep known status". The bound is a valid upper bound in every case.
- The same `all` as EM, so terminals inside cell metal are reached on the same tree (the item title). Update the
  type doc's `ponytail:` paragraph: terminal-resolved when terminals are known, worst path × total current otherwise.

### Tests (`ir.rs` `mod tests`)
- `a_dead_branch_adds_no_drop`: the existing `Stack`, plus met1 rects `{0,0,10_000,500}` and `{10_000,0,10_000,500}`
  (20 □ in total, 2.5 Ω) and a branch `{9_750,500,500,50_000}` (100 □, 12.5 Ω). Terminals A at `{0,0,200,500}` +1000 µA,
  B at `{19_800,0,200,500}` −1000 µA, C at the branch tip `{9_750,50_300,500,200}` `Some(0.0)`.
  `IrDrop{current_ua:1_000,..}.drop_uv(&r)` is `2500 ± 25` (1 %, the port graph splits rects at contacts).
  `Routes { wires: vec![wires], terms: vec![terms], .. }`. Also assert that the old bound on the same shapes (no terms) is
  `> 13_000`, so the test fails if the fallback path is taken.
- `drop_is_current_times_route_resistance` is unchanged (no terminals → bound).
- Command: `cargo test -p analog ir:: && cargo test -p dr`.
- Acceptance (T5): `bench local` IR residuals on rails do not rise. Report rows whose residual falls.

---

## REL-05 Junction temperature, self-heating bound. Class: do (steps 1–3, 6)

### Current code
- Steps 4–5 are **done** (GAP-06, `f4c646b`): `EmLimit.derating_assumed` (`backend/verify/src/pdk.rs:37`),
  `sidecar_derating` (`:694`), sidecar `em_derating` on 3 decks, test `sky130_em_rating_is_90c_with_fallback_parameters`
  (`:1736`), and acceptance `sky130_met1_em_limit_derates_at_125c_not_at_27c` (`frontend/library/src/elaborate.rs:465`).
- The EM temperature is **`lib.rs:428`** (plan: `:280`): `temp_c + 273.15`. `CellSpace` is built at `:418`. Per-cell power is
  `cells.power` (`:1414`) and the variants are `cells.variants[c].alternatives[v].bbox` (`gp::VariantSpace`, `pnr_core::Macro.bbox`).
- `OpConfig` is `oppoint.rs:122-163` (plan: `:103-121`). Every literal uses `..Default` (`lib.rs:1895,1922,1964`,
  `tests/perf_postlayout.rs`). `BiasSummary` is `metadata.rs:166-178`; it is built in `bias()` (`lib.rs:1390`) and in a
  test (`metadata.rs:613`).
- `thermal.rs`: `K_SI_W_PER_M_K` is private (`:13`). `em.rs` has no `fallback_derating_at_125c` test.

### Edits
1. `kernel/core/src/thermal.rs`: make `K_SI_W_PER_M_K` `pub`; add
   ```rust
   /// Hastings eq. 5.6: rise of a W×L source over its own area, mK (L = longer side).
   #[must_use]
   pub fn self_rise_mc(p_uw: i32, w_nm: i32, l_nm: i32, k_w_per_m_k: f32) -> f32 {
       let (w, l) = (w_nm.min(l_nm).max(1) as f32, w_nm.max(l_nm).max(1) as f32);
       (4.0 * l / w).ln() * p_uw as f32 * SCALE_UW_NM_TO_MK / (std::f32::consts::PI * k_w_per_m_k * l)
   }
   /// Σ_j self_rise_mc: no device rises more under `rises_mc`, wherever it is placed
   /// (each mutual term P/(2πk·r), r ≥ L_j/2, is ≤ P/(πk·L_j) < eq. 5.6's ln4·P/(πk·L_j)).
   #[must_use]
   pub fn rise_bound_mc(p_uw: &[i32], footprint_nm: &[(i32, i32)], k_w_per_m_k: f32) -> f32
   ```
2. `oppoint.rs` `OpConfig`: `pub theta_ja_c_per_w: Option<f64>` after `temp_c` (doc: package θ_JA, Hastings eq. 5.1;
   `None` = no package rise, since no docs/ref/ source or deck gives one). Default `None`.
3. `lib.rs:428`: replace the temperature argument with `t_em_k`:
   ```rust
   // Per cell, the variant whose eq. 5.6 self term is largest: it bounds every variant's mutual term too.
   let foot: Vec<(i32, i32)> = cells.variants.iter().zip(&cells.power).map(|(v, &p)| v.alternatives.iter()
       .map(|m| (m.bbox.w, m.bbox.h)).max_by(|a, b| self_rise_mc(p, a.0, a.1, K).total_cmp(&self_rise_mc(p, b.0, b.1, K)))
       .unwrap_or((1, 1))).collect();
   let t_em_k = cfg.op.as_ref().map(|o| o.temp_c as f32 + 273.15
       + (o.theta_ja_c_per_w.unwrap_or(0.0) * bias.summary.as_ref().map_or(0, |s| s.total_power_uw) as f64 * 1e-6) as f32
       + rise_bound_mc(&cells.power, &foot, K) / 1e3);
   ```
   **Plan correction resolved:** taking the argmax of `self_rise_mc` is enough. For any variant, self ≥ ln4·P/(πkL) > the
   mutual bound, so the max-self variant bounds both terms. A `bbox` larger than the heated channel understates the rise:
   `ponytail:` comment naming that ceiling (upgrade: channel area from `Macro.units`).
4. Step 6: `BiasSummary` gains `pub em_temp_k: f32` and `pub em_derate: &'static str`. `bias()` sets
   `em_temp_k = temp_c + 273.15` and `em_derate = ""`. `solve` overwrites both on the clone passed to `metadata::build` (`:594`):
   `em_temp_k = t_em_k`, `em_derate` from `pdk.em_limit(first routing metal)`: `None`/`derating None` → `"none"`,
   `derating_assumed` → `"sidecar+fallback Ea/n"`, else `"deck"` (a complete sidecar table also reads `"deck"`: the
   process stated all three). Update the `metadata.rs:613` literal.

### Tests
- `thermal.rs` `hastings_self_heating_example`: `self_rise_mc(100_000, 25_000, 25_000, 130.0)` within `13_600 ± 100`.
- `thermal.rs` `rise_bound_holds_for_any_placement`: `rise_bound_mc(&[1000,1000], &[(10_000,10_000);2], 148.0)` within
  `596 ± 2`. Then, for 50 placements from a fixed LCG (no new dependency; positions in ±50 µm, overlap allowed,
  hw=hh=5000), assert `rises_mc(..)` ≤ the bound for every device.
- `kernel/analog/src/routing/em.rs` `fallback_derating_at_125c`: `derate(398.15, 363.15, 0.9, 1.1)` within `0.100 ± 0.002`.
- Command: `cargo test -p pnr_core thermal && cargo test -p analog em:: && cargo test -p library`.
- Acceptance: the 125 °C / 27 °C limits are the existing elaborate test. Check `BiasSummary.em_temp_k` on `bench local ota`;
  at µW power it equals `300.15` plus a sub-mK term.

---

## REL-14 Thermal field: eq. 5.6 self term. Class: do (after REL-05)

### Current code
- `thermal.rs:31-45` `rise_at`: one loop over all sources, `r = max(hypot, max(hw,hh))`. A device at its own centre reads
  `P/(2πk·max(hw,hh))`. It is shared by `rises_mc` (`:21`) and `Layout::rise_at_point_mc` (`:56`). Isotherm tests are at `:88-143`.

### Edit
In `rise_at`'s loop, when `dx.hypot(dy) < r_floor` (the point is inside the source's floor disc; at a device's own
centre the distance is 0), add `self_rise_mc(p, 2*min(hw,hh), 2*max(hw,hh), K_SI_W_PER_M_K)` instead of the floored term.
Mutual terms are unchanged. **Decision:** use the disc rather than exact centre equality. An overlapping neighbour during `gp` would
otherwise see a different value 1 nm off-centre than at the centre. Update the module doc (self term = eq. 5.6).

### Tests
- `thermal.rs` `the_self_term_is_eq_5_6`: one 100 mW device, `hw = hh = 12_500`; `rises_mc(..)[0]` within `11_930 ± 20`
  (ln4·0.1/(π·148·25e-6) = 11.93 K). The old code gives 8 603, so the test fails on revert.
- The existing `thermal.rs` tests, `rise_bound_holds_for_any_placement` (REL-05), and MAT/placement users of `temp_mc`
  must still pass: `cargo test -p pnr_core thermal && cargo test -p analog && cargo test -p gp && cargo test -p dp`.
- MAT is told that `MatchedSet` thermal numbers change for powered matched devices (note in the report).

---

## REL-07 Guard-ring policy by role. Class: do (Clock-class fallback; EXT-23/GAP-03/CELL-17 inputs are None/false)

### Current code
- The blanket loop is `backend/annotator/src/constraints.rs:68-85`. It gives every NMOS/PMOS of every non-glue block a `Tap` ring on its
  bulk, `shareable: true`, `min_width_nm: 500`, `100_000` mΩ.
- `GuardRingRequirement` (`kernel/analog/src/cell.rs:8-19`) has no role. `GuardRingType { Tap { in_well }, Ecgr, Hcgr }`
  (`:21-32`). `post_cell::clusters` same-class test is at **`kernel/cells/src/post_cell.rs:85`**. `drawable` (`:334`) is
  false for `Ecgr`/`Hcgr`. Ring width is floored at deck `min_guard_ring_width` already (`:378`). The test constructor is `req` (`:521-529`).
- `AnnotationConfig` is `backend/annotator/src/netrole.rs:123-140` and `ProcessNumbers` is `:145-177` (both `Default`).
  `SubstrateKind` is `kernel/core/src/process.rs:97`. Net classes come from `annotate` (`backend/annotator/src/lib.rs:156`) and
  constraints are assembled at `:251`. `NetClass::Clock` exists (`kernel/analog/src/metadata.rs:20`).
- No EXT-23 tags exist on `m2`: `victim` is all false and `injector` is all `None`. The annotator has no rail voltages.
- The library dedups rings per device (`lib.rs:1464-1471`) and draws them at `:814`.
- **Tests asserting today's blanket rings:** `backend/annotator/src/tests.rs:418-435`
  `guard_rings_tie_to_the_guarded_device_s_bulk` (asserts ota rings exist), and `:448` (`"and a ring"` on the ota tail).
  The spec (§6 C19, T8) requires 0 rings on ota. These two assertions change by spec; neither is loosened. See Tests.

### Edits
1. `kernel/analog/src/cell.rs`: `pub role: RingRole` on `GuardRingRequirement` (doc: rings of different roles never merge)
   and `#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum RingRole { Injector, Aggressor, Victim }`.
2. `post_cell.rs:85`: `&& a.role == b.role`. `req()` (`:521`) takes `role: RingRole::Aggressor`.
3. New `backend/annotator/src/rings.rs` (`pub mod rings;`): `Carrier`, `RingInputs<'a>`, and
   `pub fn plan(i: &RingInputs) -> (Vec<GuardRingRequirement>, Vec<(&'static str, &'static str)>)`, exactly as the spec
   (fields, table rows 1–7, `max_ring_resistance_mohm: 100_000`, the four `missing` strings, at most one ring per
   device, `bulk` = `B` terminal, `pmos` = `kind == Pmos`). Missing notes are deduplicated (push if absent).
4. `quiet_ring_net`: `AnnotationConfig` gains `pub quiet_ring_net: Option<String>` (case-insensitive name). Without it, use the
   Ground-class net whose lowercase name contains `avss`/`vssa`/`agnd` and that no aggressor device touches.
5. Wiring in `annotate` (`lib.rs`, after `net_classes` at `:156`):
   - `aggressor[d]`: the device has a terminal on a `NetClass::Clock` net.
   - `victim`: all false. `injector`: all `None`.
   - `substrate`: `cfg.process.substrate`.
   - `ground`: the lowest-NetId Ground-class net.
   - `highest_supply`: the lowest-NetId Supply-class net (**deviation:** the annotator has no `OpConfig` rail voltages;
     `ponytail:` comment, upgrade when EXT-23 brings rails. Row 1 is unreachable until CELL-17 anyway).
   - Delete the ring loop at `constraints.rs:68-85` and its imports. Bind the assembled constraints, extend
     `constraints.guard_rings` with the plan, and extend `missing` with its notes.
6. `ProcessNumbers` gains `min_ring_width_nm: i32`, `ecgr_min_width_nm: Option<i32>`, `ecgr_drawable: bool`,
   `hcgr_drawable: bool`. `library::annotation` (`lib.rs:684`) fills them with `pdk.rule("min_guard_ring_width", 0)`,
   `opt("ecgr_min_width_nm")` (unregistered key → `None`), and `cells::post_cell::drawable(GuardRingType::Ecgr|Hcgr, pdk)`.

### Tests (`backend/annotator/src/tests.rs`; `RingInputs` built directly unless noted)
- The six spec tests verbatim: `an_ota_without_clocks_or_ports_gets_no_rings` (via `annotate(&ota(), &Default)`),
  `clocked_devices_get_aggressor_tap_rings`, `a_quiet_net_turns_on_victim_rings`,
  `an_electron_injector_gets_a_supply_tied_ecgr`, `a_hole_injector_falls_back_to_a_well_tap`,
  `epi_substrate_draws_no_aggressor_or_victim_rings` (exact assertions as in plan-06 REL-07 Tests).
- `guard_rings_tie_to_the_guarded_device_s_bulk` is rewritten on a clocked netlist: NMOS gate on net `clk` (via
  `AnnotationConfig { clock_nets: vec!["clk".into()], .. }`) and bulk on a net that is not `NetId(0)`. It asserts that rings
  are non-empty and that each `connection_net == bulk`, so the regression it guards (ring tied to `NetId(0)`) still fails it.
- `:448`'s `"and a ring"` becomes `assert!(p.constraints.guard_rings.is_empty(), "no aggressor: no ring (T8)")`.
- `kernel/cells/src/post_cell.rs` `rings_of_different_roles_never_merge`: two adjacent shareable requests, same net
  and type, roles Aggressor and Victim → `clusters(..).len() == 2`.
- Commands: `cargo test -p annotator && cargo test -p cells post_cell && cargo test -p library`. Then
  `cargo test --release -p benchmark` (signoff fixtures, `large_fixtures_sign_off_within_baseline`) and
  `cargo run --release -p benchmark --bin bench local` for `ota`, `ota_constrained` and `tt_ota` (0 rings, area delta, DRC 0, LVS MATCH).

### Risks
- `kernel/analog/src/cell.rs` and `post_cell.rs` belong to cells (CELL-17 edits them too): add only the `role` field and one
  comparison, to keep the merge trivial.
- Removing victim rings removes per-FET substrate pinning. The cells' own taps keep LU covered (spec risk note).
  `fixtures_sign_off_within_baseline` catches any LU/DRC hit.
