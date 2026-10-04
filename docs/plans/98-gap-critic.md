# 98 — Gap critic across the planning set (GAP)

Plan ID prefix: `GAP-NN`. Scope: completeness check of `docs/plans/plan-01` … `plan-08` against the seven audits
(`audit-01` … `audit-07`), the reference studies (`ref-*.md`) and the reftext extraction. Written 2026-09-29 against
the working tree of 2026-09-28 (HEAD `dad330c` plus uncommitted changes). Read-only on code; no cargo command was run.

Method:
- All eight `plan-0*.md` files read completely (items, milestones, open questions, verification logs, cut/deferred
  tables, implementer review logs).
- Every audit's ranked-findings table (severity column) and every ref doc's "Top-15 priorities" section read; every
  ID grepped across the plans. Top-15 entries were then judged by topic, not by ID match alone, because several plans
  cite an ID in "Why" or in a cut table without implementing it.
- Cross-plan contracts (type names, deck keys, step numbers, item IDs) were traced in both directions.
- Code facts below were re-read for this document at the cited lines.
- Reftext line counts were checked with `wc -l` against the Coverage sections of the study docs.

Numbers marked **[policy]** are choices made here, not source values. **[derived]** = arithmetic on cited values.

## Summary of counts

| Category | Count | Where |
|---|---|---|
| (a) critical/high audit findings with no effective plan item | 2 orphaned (AA-13, AC-06); 1 covered but never cited (AC-10); 3 partial with a named owner (AF-05, AR-03, AT-15) | §1 |
| (b) reference top-priority entries with no plan item | 19 topic groups of top-15 entries, from 15 of the 19 study docs, map to 19 missing work items (GAP-01 … GAP-19); 15 further topic groups are recorded as not planned, each with a reason | §2 |
| (c) duplicate / conflicting items across plans | 29 | §3 |
| (d) broken cross-plan dependencies | 36 | §4 |
| (e) reference ranges covered by no study doc | 1 (hastings.txt L52119–73937: index, lists of figures and tables, and 566 figure "Long description" texts) | §5 → GAP-20 |
| New work items | 20 (GAP-01 … GAP-20) | §6 |

The main systemic defect: during the implementer reviews, each plan cut an item as a "duplicate of X" while X cut the
same item as a "duplicate of" the first plan. Eleven items ended up owned by nobody (§3 C1–C11). Every GAP item
below names the one plan it belongs to, so the ping-pong cannot repeat.

---

## 1. (a) Critical/high audit findings without an effective plan item

Severity from the ranked tables: audit-01 AA-01…AA-14; audit-02 AR-01…AR-06; audit-03 AC-01…AC-10; audit-04
AP-01…AP-08; audit-05 AT-01…AT-08, AT-14, AT-15, AT-20; audit-06 AV-01…AV-09, AV-11; audit-07 AF-01…AF-12. Every one
of these was checked for a plan item that implements it.

| Finding | Status | Why it is uncovered | Resolution |
|---|---|---|---|
| AA-13 (high): clocked tail gets Proximity ≤ 5 µm and Isolation ≥ 10 µm to its own pair (`backend/annotator/src/emit.rs:263-288`) | **Orphaned** | EXT-08 was cut because "REL-09 step 3" fixes it (plan-01 L355, L955). REL-09 was cut as a duplicate of "EXT-23 `substrate::isolation`" (plan-06 L457-458, C3). The final EXT-23 has no isolation function: it defines `tag` and `victim_mask` only (plan-01 L746-751), and its step 4 again defers the same-block skip to REL-09. EXT's T6 test stays `#[ignore = "passes after EXT-14 and REL-09"]` (plan-01 L211). | GAP-04 |
| AC-06 (high): `Ecgr`/`Hcgr` are drawn as majority-carrier bulk taps (`kernel/cells/src/post_cell.rs:310-323`) while their docs say minority-carrier (`kernel/analog/src/cell.rs:25-34`) | **Orphaned** (the policy half is REL-07) | CELL-05 was cut as a duplicate of "REL-07 step 1" (plan-03 L247-248). REL-07 depends on CELL-05 for the enum, and its C6 cut moved the drawing back to "CELL-05/CELL-17" (plan-06 L375, L626). CELL-17 dropped `drawable()` (plan-03 L618). So the enum rename, the `implant_name`/`in_nwell` mapping and `drawable` have no owner. | GAP-05 |
| AC-10 (high): an empty enumeration becomes an empty macro (`frontend/library/src/cellgen.rs:815-833`) | Covered, never cited by ID | CELL-02 (the `UNDRAWABLE` equivalence test), CELL T2, and FLOW-15 step 1 (naming the right device) close it; the existing `cell/undrawable` row at `frontend/library/src/lib.rs:1293-1300` is the report. No plan lists AC-10. | Add `AC-10` to CELL-02's "Why". No new item. |
| AF-05 (high): signoff only ranks epochs, no feedback | Partial | FLOW-08 step 7 feeds DRC findings into `blame`, and FLOW-02 step 6 deletes the false "oracle" comment. PEX/spec feedback into placement is the orphan of C9. | GAP-08 |
| AR-03 (high): the Pelgrom S·D check never binds at block scale; S_VT is a PROXY | Partial | MAT-16 adds S(L) and anisotropy. The per-set D* diagnostic that says the term is non-binding (MM-06) is the orphan of C3. | GAP-02 |
| AT-15 (high): no supply topology (rails, star/Kelvin, net splitting, supply-first order) | Partial, documented | RTE-17 (star/Kelvin/rings), RTE-14 (wide nets first); supply net splitting is deferred by EXT (plan-01 L964) and RTE C13. | None; the deferrals have re-entry conditions. |

---

## 2. (b) Reference top-priority entries without a plan item

Each row covers top-15 entries that no plan implements; the IDs are grouped by topic. "Cited only" means a plan
mentions the ID in "Why" or in a cut table but no step changes code for it.

| Top-15 entries (ref doc) | Why uncovered | Item |
|---|---|---|
| H13-42 (H13 #1): MatchClass × MatchKind data model; tier keys read by no code (`pdks/sky130.json:59-73`) | Ownership loop between EXT-12, MAT-07, FLOW-06 and CELL-12 (C1). Also nobody owns `mos_env`/`resistor_env` or the class limit table (C2) | GAP-01 |
| H13-43/44/21 (H13 #9), MM-06 (MM #7), MM-14/21 (MM #14), MM-34 (MM #15) | EXT-29 disowns them (area/length → "MAT-08's `SizingNote`", D* → "MAT-16", plan-01 L854), and MAT cut them as "EXT-29's" (plan-02 L730-731) (C3) | GAP-02 |
| H05-47 (H05 #2), H14-01 second half (H14 #1), H12-39 (H12 #2), H15-43 (H15 #15) | Minority-carrier injector classification: REL-08 cut → "EXT-23 tag 1"; EXT cut injectors → "REL-08" (C4) | GAP-03 |
| SUB-01/05 (SUB #1), SUB-31/32/42 (SUB #2) | Substrate kind key and kind-aware isolation: FLOW-06 step 7 cut → REL-09 → EXT-23; none implements (C5) | GAP-04 |
| H05-54 (H05 #3), H14-27 (H14 #2) | Ring taxonomy enum and drawing mapping (C6) | GAP-05 |
| H15-38 (H15 #7), EM-42 (EM #5) | REL-05 needs a sidecar `em_derating` reader that it assigns to FLOW-06 step 3; FLOW cut that step back to REL-05 (C7) | GAP-06 |
| H01-16 (H01 #1), H12-21/23 (H12 #13) | CELL-16 depends on `Pdk::model_markers` (FLOW-06 step 5), which FLOW deferred claiming CELL uses a `mosfets` table; CELL dropped that table (C8) | GAP-07 |
| BAL2-19 (BAL2 #4), SURV-22 (SURV #6), GRAEB-10 (GRAEB #9) | Per-epoch placement net-weight feedback: PLC-17 cut it → PERF-25; PERF-25 deferred, claiming PLC-17 does it (C9) | GAP-08 |
| BAL2-48 (BAL2 #1), NOTES-06 (ref-00 #13) | EXT-08 cut (only AA-13 considered); FLOW §0.2 still consumes "EXT pre-search conflict detection" (plan-08 L50) | GAP-09 |
| NOTES-04 second half (ref-00 #4): prices keyed by stable IDs | EXT-10 says "PLC re-keys `gp::Prices`" (plan-01 L380); PLC lists it as FLOW's input (plan-04 L28); FLOW waits for EXT (plan-08 L49) (C10) | GAP-10 |
| H13-46 (H13 #10) | MAT cut says "aspect filter (H13-46) by CELL" (plan-02 L417, L726); no CELL item has it (C11) | GAP-11 |
| EM-25/27 (EM #12), H15-39 (H15 #8), H14-57 (H14 #13) | REL §0.3 says chamfers are RTE's (plan-06 L40); no RTE item draws them; RTE C8 only covers via counts | GAP-12 |
| H05-30 (H05 #12) | Latent antenna; no item | GAP-13 |
| SUB-38 (SUB #13), H14-12/32 (H14 #11), H12-16/17/14 (H12 #10) | DNW isolated tubs for NMOS; no item (DNW only in `kernel/cells/src/bjt.rs:31-34`) | GAP-14 |
| H15-57 (H15 #10) | Rule of one-third; no item | GAP-15 |
| BAL2-39 (BAL2 #10) first half | Dominated-variant pruning; PLC-24's outline part is deferred, pruning is in no plan | GAP-16 |
| H05-17 (H05 #1) node-voltage half, H01-06 (H01 #6), H06-32/33 (H06 #11) | PERF-09 adds per-FET V_GS/V_DS/V_BS only. EXT-17 `OpFacts.net_mv` and H05-17 need per-net DC voltages, which EXT §0.2 asked of FLOW and FLOW forwarded to PERF-09 (plan-08 L48); PERF-09 does not produce them | GAP-17 |
| CC-24 (CC #5) | MAT-11 computes the metrics but no item chooses the cap-array variant by them | GAP-18 |
| H12-29 (H12 #9) | CELL-10 cites it in "Why" (plan-03 L406) but only fixes region parity | GAP-19 |

Not planned, with a reason; recorded here so the omission is explicit:
- SURV-15 (opportunistic symmetry on undeclared net pairs): no measurement asks for it until RTE-15 exists.
- H12-35/36/37 (annular/serpentine MOS): no deck recogniser or fixture uses them.
- H12-42 (backgate pinning taps between injector and victim): needs GAP-03's injectors first.
- H12-44 (distributed backgate contacts): no shipped deck needs a second strip (CELL-13 appendix).
- H06-22 (voltage-aware resistor segment spacing): needs GAP-17's net voltages; no HV resistor fixture.
- H06-17 (resistor dual-π in perf): PERF-26 covers FETs only; no fixture shows the error.
- H14-05/16/21 (shared-well debias I·R < 0.3 V): needs injector currents, which no plan produces (REL Q7).
- H14-11 (N-well noise-blocking ring): no aggressor fixture.
- H14-19/20 (sense device on the isotherm): power devices are out of scope (CELL §0 "Out of scope").
- SUB-13 (substrate Miller feedback): SUB-15/16/18 (per-victim V0 budgets) and SUB-46/56 (substrate signoff report) are blocked on REL-18's missing deck data.
- H05-29 (PSD diode UV keep-out): no PSD antenna diode is inserted on sky130 (REL-02 step 5).
- EM-31 (via-above/below Blech split) and EM-46/34/44 (pattern crowding, Korhonen): dormant, since no deck supplies `blech_limit` (plan-06 L61).
- H01-14 (asymmetric S/D devices): no drain-extended device is drawn (CELL out of scope).
- CC-26/27 (cap-array trunk colouring): internal cap-array routing; no fixture has more than 4 bits.
- H05-50/56 (ring strap sized for the latch-up test current): REL §5 Q7, no test-current input.

---

## 3. (c) Duplicate or conflicting items across plans, with the resolution

| # | Conflict | Resolution |
|---|---|---|
| C1 | **Where `MatchClass` and the tier accessor live.** EXT-12 re-exports MAT-07's `analog::matching::class::MatchClass` and creates it if MAT-07 is late (plan-01 L411). MAT-07 imports `pnr_core::MatchClass` "from FLOW-06" with `Process::tier(key, MatchClass)` over per-class objects (plan-02 L38, L421). FLOW-06 cut exactly that and assigns it to MAT-07, as `Process::tier(name, idx)` over 3-element arrays (plan-08 L1229, L600-602). CELL-12 reads `class::mos_env` "from MAT-07" (plan-03 L487). RTE and PERF name `pnr_core::MatchClass` and `Pdk::tier_nm` (plan-05 L51, plan-07 L46). | GAP-01. `pnr_core::MatchClass` goes in `kernel/core/src/process.rs`, because `Process` in `kernel/core` must name it and kernel/core cannot depend on `analog`. `analog::matching::class` and `analog::intent` `pub use` it. `Process::tier(&self, key, c: MatchClass) -> Option<i32>` reads the 3-element JSON arrays that FLOW-06 already validates (`Kind::Tier`). The `Pdk::tier_nm` name is deleted from PERF-23. |
| C2 | **Class limit table.** EXT-16 uses MAT-07's `limit(f,k,c) -> ClassLimit` (plan-01 L593). MAT-07 cut its table as "duplicate of EXT-16 `limits`" (plan-02 L724). EXT's cut table says "MAT's table decides (loose end)" (plan-01 L959); MAT's says "EXT-16's table wins (tight end)" (plan-02 L417). Neither plan defines it. | GAP-01 step 4. MAT owns it. R/C use the tight end of each Hastings range (1 / 0.1 / 0.01 %), because the class-assignment rule of EXT-16 step 2 compares a spec against each class's *lower* boundary. |
| C3 | Sizing diagnostics (H13-43/44, MM-06/14/21/34): EXT-29 and MAT-08/16 each assign them to the other. | GAP-02, in MAT (it owns `Ledger`, σ_rand and S(L)). EXT-29 keeps V_gst, CLM, cascode and BJT-ratio checks. |
| C4 | Injector classification: REL-08 cut → "EXT-23 tag 1 `MinorityElectron`/`MinorityHole`" (plan-06 L454-455, C2). EXT-12 `Inject` has only `Switching`, `Capacitive` and says minority injection is "REL-08's `rings::Injector`" (plan-01 L470-471). EXT's cut table marks it a duplicate of REL-08 (plan-01 L962). | GAP-03, in EXT (it owns tags). `Inject` gains `MinorityElectron`, `MinorityHole`. REL-08's corpus test is adopted as is. |
| C5 | Isolation emission and the substrate key: REL-09 cut → EXT-23 + FLOW-06 step 7 + FLOW-04 (plan-06 C3). FLOW-06 step 7 cut → REL-09 (plan-08 L1234). EXT-23 cut the isolation model → REL-09 (plan-01 L962). REL-07 and REL-15 read `SubstrateKind` "from EXT-23" (plan-06 L414). | GAP-04, in EXT (`emit::isolation` is EXT's file). The key is registered by FLOW-06. |
| C6 | Guard-ring enum: CELL-05 cut → REL-07 "`PTap | NTap | Ecgr`, deletes Hcgr" (plan-03 L248). REL-07 uses `Tap { in_well }, Ecgr, Hcgr` "from CELL-05" and an Hcgr row gated by `drawable` (plan-06 L375, L430). REL C6 moves the drawing and `ecgr_is_n_plus…` test to CELL-05/17 (plan-06 L626). CELL-17 expects that test in REL-07 (plan-03 L617, L625). | GAP-05, in CELL. The enum is `Tap { in_well: bool }, Ecgr, Hcgr` (REL-07's names, since its policy table is the consumer). `Hbgr`/`Ebgr` are deleted. `drawable()` is kept. CELL-17 then only draws the Ecgr band. |
| C7 | EM derating keys: REL-05 reads a sidecar table `em_derating {t_ref_k, ea_ev, n}` "added by FLOW-06 step 3" (plan-06 L28, L347). FLOW-06 cut step 3 → REL-05 and registers `em_ref_temp_c`, `em_activation_ev`, `em_current_exponent` instead (plan-08 L578, L1230). | GAP-06, in REL. One key: the table `em_derating { t_ref_k, ea_ev, n }`. FLOW-06's three scalar registry rows are replaced by one `Table` row. |
| C8 | `Pdk::model_markers`: CELL-16 depends on FLOW-06 step 5 and dropped the `mosfets` sidecar table (plan-03 L603). FLOW deferred step 5 "because CELL-16 names markers in explicit `mosfets` recipes" (plan-08 L1232) and still registers a `mosfets` table (L580). | GAP-07, in FLOW. The `mosfets` registry row is deleted. |
| C9 | Per-epoch placement net-weight feedback: PLC-17 cut → PERF-25 (plan-04 L784). PERF-25 deferred because "PLC-17 (per-epoch extracted-C feedback) and RTE-21 step 5 already adjust these weights" (plan-07 L1225-1226, cut table). FLOW-08 step 5 plumbs `Weights` for "PERF's feedback" (plan-08 L712-714). RTE-21 step 5 adjusts `DetailedCfg` routing weights inside one dr call only. | GAP-08, in PERF (reinstated PERF-25). It is the only controller of `Weights.place`; RTE-21 step 5 remains the only controller of in-dr routing weights. The two vectors are distinct, so no two loops act on one vector. |
| C10 | Price keys: EXT-10 ↔ PLC ↔ FLOW (see §2). | GAP-10, in FLOW (FLOW-03 owns `Prices` settling). |
| C11 | Aspect-ratio limits (H13-46): MAT assigns to CELL, CELL has no item. | GAP-11, in CELL. |
| C12 | `PerformanceBudget` fields: EXT-25 defines `series: Vec<(NetId, f32)>`, `coupling`, `limit` and defers matched-pair ΔC terms (plan-01 L788-798, L967). RTE-21 step 3 (the struct owner per plan-05 §0.1) defines `r_nets`/`r_weights`, `diff_pairs`/`diff_weights`, `coupling`, `limit`, `stack` and says EXT-25 fills `r_*` and `diff_*` (plan-05 L640-647). | RTE-21's struct wins (RTE owns `performance.rs`). EXT-25 step 3 fills `r_nets`/`r_weights` from `SpecSens.d_r` and leaves `diff_pairs` empty (its deferral stands). RTE measures `diff_*` only when non-empty. Delete the `series` field from EXT-25's text. |
| C13 | EXT-20 maps an allocated allowance to `Budget::Sigma1Mv((a² + σ_rand²).sqrt())` (plan-01 L708). MAT §0.2 says EXT-20 uses `Budget::Allowance(allowance)` (plan-02 L42). | Use `Budget::Allowance(a)`. It is exact by definition (MAT-04 `allowance`), with no round-off through the square root. Change EXT-20 step 2 and its test `allocated_allowance_round_trips` (the value is then exactly 0.4). |
| C14 | Two `AxisDir` enums: `analog::intent::AxisDir { Vertical, Horizontal }` (EXT-12, plan-01 L448) and `analog::placement::symmetry::AxisDir { V, H }` (PLC-20, plan-04 L580). EXT-28 puts `AxisDir` into `Intent.order`. | One enum: `analog::placement::symmetry::AxisDir { V, H }` (the rule that reads it). `intent` does `pub use crate::placement::symmetry::AxisDir;`. EXT-12, EXT-14 step 9 and EXT-28 use `V`/`H`. |
| C15 | MAT §0.2, MAT-07 step 2 and MAT-08 still name `intent::MatchedSet`. EXT renamed it `MatchSpec` (plan-01 L979). | Rename in plan-02: `intent::MatchSpec`. The rule stays `placement::MatchedSet`, with no `as MatchedRule` alias needed. |
| C16 | `Unitization.class`: `Option<MatchClass>` (EXT-12, CELL-12 step 1, MAT OQ4) vs "default `Moderate`" (RTE §0.2 row 13). `Unitization.style`: `Option<ArrayStyle>` (EXT-12) vs `.style: ArrayStyle` (CELL §0 table, plan-03 L23). | `Option` for both. RTE-16 reads `class.unwrap_or(Moderate)` only for cells in a matched set (as it already specifies); `None` style = today's choice. Fix the CELL §0 table text. |
| C17 | PERF-09 step 2 (unresolved member contributes no pins) vs REL-01 step 3 (emit `None`) (plan-06 L146, Q13). | REL-01 wins. Delete PERF-09 step 2. PERF-09's test `an_unresolved_device_leaves_other_pins_sized` stays and holds under REL-01. |
| C18 | REL-03 step 7 keeps dr's `em underwidth`/`em cuts` Θ as search pressure (plan-06 L293). RTE-14 deletes `branch_currents` and those Θ entries (plan-05 RTE-14 Risks). | Order: REL-03 lands first and keeps them. RTE-14 deletes them in the same commit that makes REL-03's rule the only EM measure. Add "supersedes REL-03 step 7" to RTE-14. |
| C19 | REL Q12 says EXT-23 emits victim `MajorityTap` rings and expects 4 rings on `ota5t`. The final EXT-23 emits no rings (plan-01 L742-758) and its test expects 0 via REL-07. | The conflict is gone. Close REL Q12 and drop the "EXT-23's `substrate::rings`" sentence (plan-06 L390, L42). |
| C20 | Four plans edit `frontend/library/src/lib.rs:871-872` (the `_moderate` key reads in `environment`): EXT-16's consumer edit, a nonexistent "MAT-07 step 6", CELL-12 (via MAT-07), and PLC-29, which moves the whole `environment` body into `LiveEnvironment`. | PLC-29 moves the body. GAP-01 step 5 makes `LiveEnvironment` read `mos_env(class_of_pair).wpe_nm`, with the class from EXT-16's `MatchSpec` (default `Moderate`). EXT-16 and CELL-12 drop their edits of these lines. |
| C21 | `lod_moat_ext_nm` tiers: sky130 carries 2 entries `[3000, 5000]` (`pdks/sky130.json:70-73`). FLOW-06 `Kind::Tier` requires exactly 3. CELL-12 tests MIN/MOD/EXC = 3000/5000/10000. The H13-47 recipe says `[0, 5000, 10000]`. | GAP-01 step 2 sets `[3000, 5000, 10000]`, each with a source. |
| C22 | EXC WPE term `max(5000, 2·n_well_depth)` (CELL-12 step 2) vs REL Q10 dropping `n_well_depth` as unsourced (AV-22). | Use the sidecar tier `wpe_clearance_nm[2] = 5000` alone (Hastings rule 19 lower bound, via H13-53). No `n_well_depth` term. |
| C23 | Mismatch-coefficient key names: FLOW-06 registry `abeta_pct_um` (one cell key), `ka_pct_um`, `cap_ka_pct_um` (plan-08 L577) vs MAT-09 `abeta_n_pct_um`/`abeta_p_pct_um` and MAT-10 per-recipe `k_a_pct_um` (plan-02 L460, L475). FLOW cut says MAT owns the names. | MAT's names: `abeta_n_pct_um`, `abeta_p_pct_um`; per resistor/capacitor recipe `k_a_pct_um`, `tc_ppm_per_k`, `sd_pct_per_mm`; cell `bjt_ka_pct_um`, `vbe_tc_uv_per_k`; MAT-16 `svt_a_uv2_per_um2`, `svt_b_uv2`. FLOW-06 step 1's table is rewritten with these. Keys that FLOW-06's table also misses must be registered by their owners in the same commit: CELL-06 `res_*` value-model keys, `res_value_tol_ppm`; CELL-09 `bjts`; CELL-23 `res_widths_nm`; PLC-03 `rotate_90_allowed`; PLC-07 `placement_space`; REL-12 `em_front_row_cuts`; REL-13 `res_tox_nm`, `res_self_heat_dt_k`; REL-17 `metal_family`; REL-07 `ecgr_min_width_nm`. |
| C24 | Capacitor recipe schema: FLOW cut says CELL-08 uses `cap_*` roles and a `mim_m3` recipe, and its test reads `Overlay::enclosure("cap_bot","cap_ins")` (plan-08 L595-597, L1231). CELL-08 uses roles `bottom, plate, top_contact, top, strap, bottom_contact` and key `mim_m3_1` (plan-03 L338-342). | CELL-08 owns the table as written in plan-03. FLOW-06's test becomes `Overlay(mim_m3_1).enclosure("bottom","plate") == Some(140)` (the same assertion CELL-08 already carries; keep one copy, in CELL-08). |
| C25 | PERF §0.2 row and PERF OQ6 rely on CELL-01's `Pdk::extracts_params(kind)`, which CELL-01 dropped (plan-03 L173). | CELL-01's rule stands: parameters on MOS cards only. Delete the probe from plan-07's text. |
| C26 | MAT-12's `jumper_spread` was cut, but CELL-14 step 3 says "MAT-12's `jumper_spread` over the drawn joins is then 0" and its scope note assigns the metric to MAT-12 (plan-03 L541, L555). | CELL-14's `every_join_has_the_same_via_count` is the check. Delete the `jumper_spread` sentences from CELL-14. |
| C27 | `greedy_centroid` deletion: CELL-14 depends on "MAT-12 (which also deletes `greedy_centroid`)" (plan-03 L533). MAT-12 step 2 says CELL-14 deletes it (plan-02 L520). | CELL-14 deletes it (it owns `resistor.rs`/`builder.rs`). |
| C28 | PLC-06's test `gp_sees_power` and PLC-10 step 2's `touches` list use `ThermalGradient`/`MatchingPair`/`CentroidGroup`, which MAT-04 deletes. | Whichever of PLC-06 / MAT-04 lands second rewrites the test on a two-member `MatchedSet` (its thermal term reads the live field, so PLC-06's refresh step still matters). PLC-10's `touches` list drops the deleted types; MAT-04 already implements `touched` via `cell_of`. |
| C29 | FLOW-15 step 6 deletes `GuardRingRequirement.tap_pitch_nm` and `.enclosure_complete` at M4 (plan-08 L1038). REL-07/CELL-05 delete the same fields as part of the enum change. | GAP-05 deletes them. FLOW-15 step 6 drops those two fields from its list. |

---

## 4. (d) Broken cross-plan dependencies and their fixes

| # | Where (plan line) | Broken reference | Fix |
|---|---|---|---|
| D1 | plan-01 EXT-16 Depends and consumer edit (L590, L608) | "MAT-07 steps 3 and 6" (`Process::tier`, `mos_env(Moderate, …)`); MAT-07 has steps 1–5 and no tier | Depend on GAP-01; the `lib.rs:871-872` edit moves to PLC-29 + GAP-01 step 5 (C20) |
| D2 | plan-01 EXT-12 (L487) | "MAT-07 step 5's non-`Option` `class`" | Delete the sentence; C16 |
| D3 | plan-01 EXT-01 test `no_emitted_conflicts` (L211), T6, EXT-08 stub (L355) | REL-09 step 3 (cut) | GAP-04 |
| D4 | plan-01 EXT-23 step 4 (L755) | "REL-09 then skips … pairs inside one block" | GAP-04 steps 2–3 |
| D5 | plan-01 §0.2 REL row (L33) | REL-08, REL-09 (cut) | REL-07 only; injectors GAP-03, isolation GAP-04 |
| D6 | plan-01 EXT-10 step 1 (L380) | "PLC re-keys `gp::Prices` (PLC item)"; no PLC item | GAP-10 |
| D7 | plan-01 EXT-21 cut row; EXT-24 `CommonNodeReq.max_delta_mv` (cut, L966) | consumed by RTE-17 step 4 (plan-05 ~L572) | RTE-17 uses MAT-06's remaining allowance only; delete `EXT-24 max_delta_mv` from RTE-17 |
| D8 | plan-05 RTE-14 step 1 (L160) | `Intent.nets[n].em_critical` (EXT cut it, plan-01 L965) | RTE-14's own current filter (its stated fallback) becomes the only rule; delete the EXT-24 reference |
| D9 | plan-05 RTE-21 Consumes (L630) | "EXT-24 step 8 (`RcClass`)"; EXT-24 has 6 steps, `RcClass` is step 6 (DAC plates) and EXT-25 step 4 (per net) | Cite EXT-24 step 6 and EXT-25 step 4 |
| D10 | plan-05 RTE-21 Consumes | EXT-25 fields `r_nets/r_weights`, `diff_pairs/diff_weights` (EXT-25 defines `series`) | C12 |
| D11 | plan-05 §0.2 MAT-07/FLOW-06 row (L51); RTE-16 Depends | `pnr_core::MatchClass` "on `Unitization.class` (default `Moderate`)" from MAT-07/FLOW-06 | GAP-01 (`pnr_core::MatchClass`); `Option` field (C16) |
| D12 | plan-02 §0.2 FLOW-06 row (L38), MAT-07 step 1 (L421) | FLOW-06 defines `pnr_core::MatchClass`, `Process::tier` (FLOW cut them) | GAP-01 |
| D13 | plan-02 §0.2 EXT-16 row (L40), MAT-08 step 1 (L446) | `limits(Family, MatchKind) -> [f32; 3]` in EXT-16 (not there) | GAP-01 step 4 `class::limit` |
| D14 | plan-02 §0.2 EXT-29 row, MAT-16 (L597) | "EXT-29 item 7" (D*); EXT-29 has 4 checks | GAP-02 |
| D15 | plan-02 MAT-09 step 3 (L460) | FLOW-06 slots `abeta_n_pct_um`/`abeta_p_pct_um` (FLOW cut step 6) | C23: MAT registers its own keys through FLOW-04's registry |
| D16 | plan-02 MAT-10 Depends (L468) | FLOW-06 "per-recipe `k_a_pct_um` slot, `capacitors` recipe table" (cut) | Keys: C23; table: CELL-08 (C24) |
| D17 | plan-03 §0 table (L24), CELL-12 Depends (L478, L487), CELL-14 Depends step 5 (L533, L557), CELL-23 Depends (L712), T7 (L100) | MAT-07 `class::{mos_env, resistor_env}`, tier keys (MAT cut them) | GAP-01 steps 3–4 |
| D18 | plan-03 CELL-12 step 3 (L501) | "MAT-07 step 6 leaves these two to CELL" (MAT-07 has no step 6; the note is step 4) | Cite MAT-07 step 4 |
| D19 | plan-03 CELL-13 Depends (L512), §5 Q1 | REL-06 (cut) | Depend on FLOW-05 step 6 (`tie_max_dist_nm` 15 000 with source) and FLOW-05 step 1 (LU rules) |
| D20 | plan-03 CELL-17 Depends and tests (L614, L617, L625) | REL-07 taxonomy test `ecgr_is_n_plus_in_its_own_well_clear_of_the_nmos`, REL-07 `ring_gap` for `Ecgr`; REL-08 (cut) | GAP-05 (owns the test and `ring_gap`); injector input GAP-03 |
| D21 | plan-03 CELL-08 step 1 (L338) | "FLOW-06 step 4 owns `cell.capacitors` (schema, `Pdk::capacitor_recipe`, `mim_m3_1` rows)" (cut) | CELL-08 owns the full table (C24); precondition `recipe_layers` stays FLOW-06 step 2 |
| D22 | plan-03 CELL-16 Depends (L596) | FLOW-06 step 5 `Pdk::model_markers` (deferred) | GAP-07 |
| D23 | plan-04 §0.2 MAT row (L23), PLC-13 Depends (L502), PLC-14 (L510) | MAT-07 `mos_env`, `Process::tier`; MAT `MatchingPair`/`CentroidGroup`/`ThermalGradient` (deleted by MAT-04) | GAP-01; C28 |
| D24 | plan-04 §0.2 EXT row (L22), PLC-26 | EXT-22 well groups (deferred) | PLC-07 derives well sharing from profiles (already); drop EXT-22 from PLC §0.2 |
| D25 | plan-04 §0.2 REL row (L25) | REL-09 (cut) | GAP-04 |
| D26 | plan-04 PLC-29 (L647, L665) | "MAT-07 step 6" | C20 / GAP-01 step 5 |
| D27 | plan-06 REL-05 Depends and steps 4–5 (L322, L347, L350) | FLOW-06 step 3 (cut) | GAP-06 |
| D28 | plan-06 §0.2 FLOW-06 step 7 row (L29); REL-07 `RingInputs.substrate` (L414) | FLOW-06 step 7 `substrate` table (cut); EXT-23 `SubstrateKind` (absent) | GAP-04 |
| D29 | plan-06 §0.2 EXT-23 row (L33); REL-07 `RingInputs.injector` (L413); REL-15/16 Depends | EXT-23 minority-injector tags (absent) | GAP-03 |
| D30 | plan-06 §0.2 CELL-05 row (L34); REL-07 Depends (L375) | CELL-05 enum, CELL-17 `drawable` (both cut) | GAP-05 |
| D31 | plan-07 §0.2 REL row (L61), milestone gates | REL-06 (cut) | FLOW-05 steps 1, 6 |
| D32 | plan-07 §0.2 FLOW-06 row (L46), PERF-23 Depends | `Pdk::tier_nm(key, MatchClass)` (cut) | GAP-01 `Process::tier(key, MatchClass)` |
| D33 | plan-07 §0.2 CELL-01 row (L56), OQ6 | `Pdk::extracts_params(kind)` (dropped) | C25 |
| D34 | plan-07 PERF-25 and cut table; §0.4 (L85) | "per-epoch net-weight feedback (PLC-17 …)" (cut from PLC-17) | GAP-08 |
| D35 | plan-08 §0.2 MAT row (L44), REL row (L45), CELL row (L47), EXT conflict row (L50), PERF feedback row (L53) | MAT-07 tier arrays; REL-09 substrate keys; REL-05 scalar EM keys; CELL-16 `mosfets` table; EXT pre-search conflict detection; PERF net-weight feedback | GAP-01, GAP-04, GAP-06, GAP-07, GAP-09, GAP-08 respectively |
| D36 | plan-08 FLOW-05 Ownership note (L459-462) | "REL-06 keeps its CELL contract (step 3) and its `LU.` fixture tally" (REL-06 cut) | Delete; CELL-13 is the tap-reach contract, and T6 is the DRC baseline (plan-06 L372) |

---

## 5. (e) Reference ranges no study covered

All study docs cover their assigned ranges contiguously: Hastings 1–11865 (H01), 11865–15989 (H05), 15989–21772 (H06),
21772–25761 (H08), 25761–34124 (H09), 34124–38173 (H12), 38173–42876 (H13), 42876–46303 (H14), 46303–52119 (H15);
balasa_graeb_survey 1–8167 and 8167–14994 (file length 14994); lampaert 1–7750; graeb_centering 1–8560; lienig_em
1–7770; charbon_substrate 1–5063; each of the four mismatch papers, three CC papers and two surveys in full.

The exception is `hastings.txt` (73 937 lines). H15 stops at L52119 ("from the Chapter 15 heading to the Index"). The
rest was read by no study:

| Range | Content | Substantive? |
|---|---|---|
| L52119–54715 | Index | no |
| L54716–57403 | Navigation contents, List of Figures (from L55433), List of Tables (from L56293): captions only | captions only |
| L57404–73937 | 566 figure "Long description" blocks (accessibility text) | **yes** |

The long descriptions state figure values in text, for example the segmentation-sensitivity bars of Fig. 8.19
("For N equals 1; S is 1.8 E-6 …", L64797) and the Fig. 8.20 axis ranges (L64817). Several plan items say a value
was "read from the figure", "not given" or disputed:
- MAT open question 7: eq. 8.29 vs Fig. 8.20 (plan-02 L669).
- The H13-53 WPE distances and H13-55 rule-23 values, blank in the main text (plan-04 L761).
- PLC-13's 5 % at 1.8 µm / 25 % at 0.95 µm, read from PDF 696 (plan-04 L503).

→ GAP-20.

---

## 6. New work items

Format as in the plans: Priority, Effort, Depends, Owner plan (the plan whose file this item belongs in), Why, Current,
Change, Tests, Acceptance.

### GAP-01 Match-class home, tier table, environment functions and class limit table
- Priority: P1. Effort: M. Depends on: FLOW-04 (sidecar registry, `Kind::Tier`). Owner plan: **MAT (plan-02)**, as a new MAT-07 step set. Unblocks EXT-12, EXT-16, CELL-12, CELL-14, CELL-23, PLC-13, PLC-14, PLC-29, RTE-16, PERF-23.
- Why: H13-42 (classes MIN/MOD/EXC, Hastings §13.3, hastings.txt L42327–42350, PDF 712); H13-47 (rule 12 dummies and moat, L42532–42553, PDF 714–715); H13-53 (rule 19 WPE, L42625–42631); H13-54 (rules 21–22 gate extension, L42638–42647); H08-13 (resistor dummies by class, L22312–22347, L25263–25273); H08-41 (resistor width/length floors, L25205–25214, L25274–25281); H09-01 (bipolar classes, L31244–31275, PDF 524); H08-02 (R/C classes, L25121–25133, PDF 418); AV-22; C1, C2, C20–C22.
- Current:
  - Grep finds no `MatchClass` in the tree (plan-02 L416).
  - `pdks/sky130.json:59-63` has `wpe_clearance_nm [2000, 3000, 5000]` and `:70-73` has `lod_moat_ext_nm [3000, 5000]`; no code reads either.
  - The code reads the scalars `lod_moat_ext_moderate`/`wpe_clearance_moderate` instead (`kernel/cells/src/mosfet.rs:293, 629`; `frontend/library/src/lib.rs:871-872`).
  - `parse_roles` keeps only integer and boolean scalars (`backend/verify/src/pdk.rs:1259-1267`), so the arrays are dropped.
- Change:
  1. `kernel/core/src/process.rs`:
     ```rust
     #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
     pub enum MatchClass { Minimal = 0, #[default] Moderate = 1, Exceptional = 2 }
     // in trait Process:
     /// Tier value of a 3-element sidecar array `key`, index `c as usize`; `None` when the key or entry is absent/null.
     fn tier(&self, key: &str, c: MatchClass) -> Option<i32> { let _ = (key, c); None }
     ```
     Re-export in `kernel/core/src/lib.rs`. `backend/verify/src/pdk.rs`: `parse_roles` keeps each `Kind::Tier` array as `tiers: BTreeMap<String, [Option<i32>; 3]>`; `impl Process for Pdk` returns the entry; `Overlay` delegates. `analog::matching::class` and `analog::intent` do `pub use pnr_core::MatchClass;`.
  2. Sidecar tier values (each key gets `<key>_source`); sky130 first, the other decks get the same numbers:

     | key | MIN | MOD | EXC | Source |
     |---|---|---|---|---|
     | `wpe_clearance_nm` | 2000 | 3000 | 5000 | Hastings rule 19 (H13-53; EXC lower bound of 5–10 µm) — existing array |
     | `lod_moat_ext_nm` | 3000 | 5000 | 10000 | §13.2.2 "two or three microns will suffice" for less accurate matching (L41411–41422, H13-29); rule 12 MOD STI ≥ 5 µm, EXC ≥ 10 µm (H13-47) |
     | `dummy_reach_nm` | 0 | 3000 | 10000 | rule 12: MIN optional, MOD one full dummy for L < ~3 µm, EXC outer dummy poly edge ≥ 10 µm (H13-47 recipe values) |
     | `gate_ext_extra_nm` | 0 | 1000 | 1000 | rule 21: +1–2 µm beyond the rule for MOD/EXC (H13-54, lower end) |
     | `res_dummy_span_nm` | 0 | 0 | 10000 | H08-13: MIN one min-width dummy, MOD one full-width, EXC span ≥ 10 µm |
     | `res_width_floor_permille` | 1500 | 2000 | 4000 | H08-41: 150/200/400 % of minimum width |
     | `res_length_floor_x` | 3 | 5 | 10 | H08-41: 3×/5×/10× minimum length |

     Delete the scalars `lod_moat_ext_moderate` and `wpe_clearance_moderate` from all four sidecars and from FLOW-06's deprecated rows.
  3. `kernel/analog/src/matching/class.rs`:
     ```rust
     #[derive(Clone, Copy, Debug, Default, PartialEq)]
     pub struct MosEnv { pub dummy_reach_nm: i32, pub moat_nm: i32, pub wpe_nm: i32, pub gate_ext_extra_nm: i32, pub gate_strap: GateStrap }
     #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)] pub enum GateStrap { #[default] PolyBar, PolyBarFar, MetalIsolated }
     pub fn mos_env(c: MatchClass, p: &dyn pnr_core::Process) -> MosEnv; // tiers above; strap: MIN PolyBar, MOD PolyBarFar (≥ 1 µm, CELL-11 step 4), EXC MetalIsolated (rule 22)
     #[derive(Clone, Copy, Debug, Default, PartialEq)]
     pub struct PassiveEnv { pub min_dummies: u8, pub dummy_span_nm: i32, pub width_floor_permille: i32, pub length_floor_x: i32 }
     pub fn resistor_env(c: MatchClass, p: &dyn pnr_core::Process) -> PassiveEnv; // MIN 1 dummy, MOD 1, EXC ceil(span/pitch) (caller)
     ```
     A missing tier value → 0, with one `("MatchClass", "<key> tier missing")` entry in `Problem.missing`.
  4. Class limit table (the one table; C2):
     ```rust
     #[derive(Clone, Copy, Debug, PartialEq)] pub enum ClassLimit { Mv(f32), Pct(f32) }
     pub fn limit(f: Family, k: MatchKind, c: MatchClass) -> Option<ClassLimit>;
     ```
     - MOS: Voltage `Mv` 10/3/1, Current `Pct` 10/3/1 (L42327–42350).
     - Bipolar: Voltage `Mv` 2/0.5/0.1, Current `Pct` 8/2/0.5 (L31244–31275).
     - Resistor and Capacitor, Ratio: `Pct` 1/0.1/0.01. This is the tight (lower) end of each range; EXT-16 rule 2 maps a spec X to the class whose lower boundary X clears (L25121–25133).
     - Diode: `None` ("not given"; H09-42/43 give ideality limits, not offset limits).
     - Every other (family, kind) pair: `None`.
     EXT-16 imports it (it no longer names `limits`).
  5. `LiveEnvironment` (PLC-29): per pair, `wpe_min_nm = mos_env(class, p).wpe_nm`, with `class` = the `MatchSpec` class of the pair (EXT-16), default `Moderate`. `ose_range_nm` keeps today's source. EXT-16's and CELL-12's edits of `lib.rs:871-872` are dropped (C20).
- Tests:
  - `backend/verify/src/pdk.rs` `sky130_tiers_are_read`: `tier("wpe_clearance_nm", Exceptional) == Some(5000)`, `tier("lod_moat_ext_nm", Minimal) == Some(3000)`.
  - `class.rs` `mos_env_follows_the_table`: MOD → (3000, 5000, 3000, 1000, PolyBarFar); EXC → (10000, 10000, 5000, 1000, MetalIsolated).
  - `limits_match_hastings`: 15 asserted values plus `Diode → None`.
  - `missing_tier_reads_zero_and_is_reported`: a sidecar without `dummy_reach_nm` → `dummy_reach_nm == 0` and one `missing` entry.
- Acceptance: grep finds no reader of `*_moderate` keys; CELL-12's class tests use `mos_env`; `cargo test -p verify -p analog` green; T3 of FLOW (every sourced key has `_source`) stays 0.
- Risks / notes: the EXC `dummy_reach_nm` costs about 24 minimum-L dummies per end at 430 nm pitch (CELL-12 Risks, [derived] there). EXC is only ever set by a user or a spec (EXT-16), never by role.

### GAP-02 Sizing-reach diagnostics: area, length, statistical distance scale
- Priority: P1. Effort: S. Depends on: MAT-04 (`Ledger`), MAT-16 (S(L)), GAP-01 (limits), EXT-17 (g_m, I_D). Owner plan: **MAT (plan-02)**, as a new MAT-22.
- Why: H13-43 (Table 13.4, A = (6·c_Vt/ΔV)², rules 2, 6, L42369–42407, L42483–42489, PDF 712–714); H13-44 (Table 13.5 L at 1 µA, ∝ I_D^−½, eq 13.48, L42412–42459, PDF 713); MM-06 (D* = A/(S·√WL), pelgrom.txt L88–90, L150–151); MM-14 (area multiplier (σ_rand/σ_budget)², pelgrom.txt L143–145); MM-21 (mirror lever is L, drennan L256–275); MM-34 (x* = σ/|grad|, long_distance L266–270); AR-03; C3.
- Current: `emit.rs:85-86` clamps η to 0 when σ_rand exceeds the budget and reports nothing. The only related field is `LedgerRow.sizing_limited` (MAT-08 step 3).
- Change: new `kernel/analog/src/matching/sizing.rs`, computed once per `MatchedSet` after emission, never charged to Θ:
  ```rust
  #[derive(Clone, Debug, PartialEq)]
  pub struct SizingNote { pub members: (u32, u32), pub kind: &'static str, pub have: f32, pub need: f32, pub lever: &'static str }
  pub fn notes(set: &MatchedSet, limit_6sigma: Option<f32>, gm_over_id: Option<f32>, span_nm: Option<f32>) -> Vec<SizingNote>;
  ```
  1. `"area"` (Voltage kind, class limit known): `need = (6·A_VT/ΔV)²` µm², ΔV = the class `Mv` limit. `have` = the member gate area (µm²; units when present, else netlist). Emit when have < need; lever `"area ×(need/have)"`.
  2. `"budget_area"` (any kind, budget below σ_rand): `need/have = (σ_rand/σ_budget)²` (MM-14); lever `"area"` for Voltage kind, `"length"` for Current kind (MM-21).
  3. `"dstar"`: D* = A_VT/(S_VT(L)·√(WL)) µm (MAT-16 S(L) when present). Always reported, with `need` = D* and `have` = the placed centroid distance. A set whose span < 0.1·D* is marked `"distance term non-binding"`. 0.1 is **[policy]**; the source gives only "ignorable below 100 µm²".
  4. `"xstar"` (members of one set further apart than the per-set `span_nm`, e.g. ladders): x* = σ_rand/|∇| with |∇| = S_VT(L) (mV/µm). Emit when span > x*; lever `"increase L or W"` (MM-34).
  Rows go to `MetadataReport.sizing: Vec<SizingNote>` and PERF-20's bench table.
- Tests (`sizing.rs`):
  - `table_13_4_5v_exc`: c_Vt·t_ox = 12.5 mV·µm, ΔV = 1 mV → need 5625 µm² [derived check, H13-43].
  - `dstar_sky130`: A = 9.5, S = 1.63, WL = 100 → 583 µm ± 1 (MM-06 [derived] 0.58 mm).
  - `budget_below_random_gives_multiplier`: σ_rand = 2.124, budget 1.0 mV → need/have = 4.51 ± 0.01.
  - `mirror_names_length`: Current kind → lever `"length"`.
- Acceptance: on `ota` with `offset_sigma_mv = 1.0`, the input pair has one `"budget_area"` note and the same pair's `LedgerRow.sizing_limited` is true; every note is printed in the bench.
- Risks / notes: EXT-29 keeps V_gst, CLM, cascode ratio and BJT ratio. Its text should cite GAP-02 instead of "MAT-08 `SizingNote`".

### GAP-03 Minority-carrier injector classification
- Priority: P1. Effort: M. Depends on: FLOW-07 (`Netlist.ports`, resistor values `r_mohm`), EXT-12, EXT-18, EXT-23. Owner plan: **EXT (plan-01)**, as EXT-23 step 5.
- Why: H05-47 (pin diffusions within < 10 kΩ may trigger latch-up; conservative 50–100 kΩ, Hastings §5.4.1, L14837–14885, PDF 256–257); H14-01 (below about 50 kΩ, §14.2, L43629–43638, PDF 735; forward-biased bulks, §14.1.4, L43526–43547); H12-39 (rings on injectors only, L36896–36899); H15-43 (checklist item 8, L48898–48918); REL T7 (plan-06 L134); C4.
- Current: `Inject { Switching, Capacitive }` (plan-01 L471). REL-07 `RingInputs.injector` is always `None` (plan-06 L413). Net roles have no `Pin` (`backend/annotator/src/netrole.rs:11-16`).
- Change:
  1. `analog::intent::Inject` gains `MinorityElectron`, `MinorityHole`. `Policy` gains `inj_series_ohm: f64 = 50_000.0` (Hastings §14.2 value; H05-47 notes 10 kΩ is the usual and 50–100 kΩ the conservative choice).
  2. `substrate::tag` (EXT-23):
     1. Pin nets = `nl.ports` whose class is not Supply, Ground or Substrate. Empty ports → no minority tags, plus `missing ("GuardRing", "no port list: injectors unknown")`.
     2. Multi-source Dijkstra from the pin nets over resistor devices as edges, weight = resistor value (`r_mohm`/1000 Ω). A resistor with no value is an edge of weight 0, i.e. conservative: its far node counts as reachable below the threshold, with reason `"resistance unknown"`.
     3. Every net at distance < `inj_series_ohm` is "pin-reached".
     4. Tag the devices with a diffusion terminal on a pin-reached net. NMOS D or S (n+ in the p substrate) → `MinorityElectron` (pin below ground forward-biases NSD/P-substrate). PMOS D or S (p+ in n-well) → `MinorityHole` (pin above supply). A diode takes the polarity of its diffusion on the pin-reached net. BJTs are left unclassified, with `missing ("GuardRing", "bipolar injectors unclassified")` (REL C2).
     5. Forward-biased bulk (H14-01): with an op point, an NMOS whose `vbs > 0` or PMOS whose `vbs < 0` (PERF-09 step 5) → the same tags, reason `"forward-biased bulk"`.
  3. REL-07's `RingInputs.injector[d]` = `Some(Electrons)` or `Some(Holes)` from these tags; REL-16's `CellFlags.injector` = either tag.
- Tests (`substrate::tests`), REL-08's corpus (plan-06 C2): `.subckt drv in out VDD VSS` with `M1 out in VSS VSS nfet`; `M2 x in VSS VSS nfet` + `R1 x out 10k`; `M3 y in VSS VSS nfet` + `R2 y out 100k`:
  - `pin_diffusions_are_injectors`: M1 and M2 are `MinorityElectron`, M3 is untagged.
  - `a_pmos_on_a_pin_injects_holes`: PMOS D on `out` → `MinorityHole`.
  - `unknown_resistance_is_conservative`: R2 without a value → M3 tagged, reason `"resistance unknown"`.
  - `no_ports_no_minority_tags`: the same netlist without `.subckt` → no tags, one `missing`.
- Acceptance: REL T7 (100 % of tagged injectors get an Injector ring request) on the `drv` corpus; `ota`-class fixtures (ports are all gates or rails) get 0 injectors, so REL T8 still holds.
- Risks / notes: H12-42 (pinning taps between injector and victim) and the latch-up loop-gain margin (Hastings eqs 12.32–12.33) remain unplanned (§2).

### GAP-04 Kind-aware isolation, no rule inside a block, compound or set; substrate kind key
Status: steps 1–2 done in M1a for the block exemption (`same_group` = same non-glue entry of `Problem.blocks`); the compound/`MatchSpec` part waits for EXT-14/15 (M2), step 3 for REL-15. As built: `SubstrateKind::from_key` maps the sidecar string (anything else is `Unknown`); `Pdk::cell_str` reads it; `emit::isolation` returns the `missing` input (`"bulk substrate: no plateau distance"`, `"substrate kind unknown"`, or `"deck epi_thickness_nm"` on epi without a thickness); sky130's `substrate_kind_source` starts `UNVERIFIED:` so `Pdk::unverified` lists it.
- Priority: **P0** (AA-13 is a live contradictory constraint). Effort: M. Depends on: none for steps 1–2; EXT-14/15 for the compound/set part of step 2; FLOW-04 for the key registry. Owner plan: **EXT (plan-01)**, as EXT-23 step 6 plus the `emit::isolation` rewrite. FLOW-06 registers the keys.
- Why: AA-13; SUB-01 (substrate stack as a deck input, charbon L705–715, PDF 35, 122); SUB-31 (epi on p+: isolation saturates at 2.5–5× epi, Su 4×, L3289–3313, PDF 127); SUB-32 (bulk: isolation keeps improving with distance, L3320–3351, PDF 127–130); REL-09's handed-over tests (plan-06 C3); C5; D3–D5, D25, D28.
- Current: `emit::isolation` (`backend/annotator/src/emit.rs:263-288`) pairs every clocked non-sensitive device with every sensitive device, with no group exemption. Distance = `4·epi_nm.unwrap_or(2500)` (`:250, :256, :274`). It pushes to the budget arm only when `epi_nm` is `Some` (`:283-285`); no deck supplies it, and the source comment at `:251-255` notes that sky130 is bulk.
- Change:
  1. Sidecar keys (FLOW-06 registry rows, sourced): `substrate_kind` ∈ `"bulk" | "epi_on_pplus" | null` and `epi_thickness_nm` (integer or null).
     - sky130 gets `"bulk"`, `_source` = "Philis note backend/annotator/src/emit.rs:252-255; not stated in ref/ — [UNVERIFIED]".
     - gf180mcu, ihp_sg13g2 and generic_finfet get `null`.
     - `p_epi_thickness` becomes `required: false` (FLOW-04 step 5) and is not read here.
     - `pnr_core::process` gains `#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum SubstrateKind { Bulk, EpiOnLowRes, Unknown }`, and `ProcessNumbers.substrate: SubstrateKind` replaces `epi_nm` in `emit::isolation`.
  2. `emit::isolation(hg, classes, sensitive, same_group: &dyn Fn(usize, usize) -> bool, kind, epi_nm, r)`:
     - Pairs where `same_group(a, v)` is true are skipped. Before EXT-14/15, `same_group` = same entry of `Problem.blocks`, glue excluded. After them, it also means same compound or same `MatchSpec`.
     - `EpiOnLowRes` with `epi_nm` known: `min_distance = 4·epi` (Su 4×) in the budget arm and the cost arm.
     - `Bulk` or `Unknown`: cost arm only, with `known = false` in the report. The pull is `4·2500` nm as today; bulk has no plateau distance.
     - `missing` gets `("Isolation", "substrate kind unknown")` for `Unknown`.
  3. REL-15's `SubstrateBalance` emission moves into the same function (REL-15 step "emitted next to Isolation").
- Tests (`emit.rs`):
  - `a_clocked_tail_is_not_isolated_from_its_own_pair` (REL C3): StrongARM-like block, tail on `clk` → 0 `Isolation` rules to mn1/mn2.
  - `epi_decks_get_a_plateau_budget` (REL C3): `EpiOnLowRes`, epi 10 µm → `min_distance_nm == 40_000`, in the budget arm.
  - `bulk_is_cost_only_and_unknown`: sky130 kind → the rule is in `cost` only and the report shows it unknown.
  - Un-ignore EXT-01's `tests/corpus.rs::no_emitted_conflicts_strongarm` (its attribute says "passes after EXT-14 and REL-09"; REL-09 is cut and this item owns the fix, C5) and, in the same commit, delete `strongarm_conflicts_are_todays` and `STRONGARM_CONFLICTS`, which pin today's pairs. On the corpus strongarm the clocked tail is `mp8`, in the input pair's stage (`mn0` is grouped with `mp7`), so the conflicting pairs are (mn1, mp8) and (mn2, mp8), and the same-block exemption of step 2 is expected to remove them (the un-ignored test is the check).
- Acceptance: T6 of EXT (0 device pairs with both Proximity and Isolation) on every corpus circuit; REL T10 = 0.
- Risks / notes: the bulk distance model (SUB-32, monotone) and the two-port macromodel stay deferred with REL-18 (no ρ stack in any deck).

### GAP-05 Guard-ring kinds name what is drawn; `drawable`; ECGR spacing
- Priority: P1. Effort: S. Depends on: none. Owner plan: **CELL (plan-03)**, reinstating CELL-05. Consumed by REL-07 (policy), CELL-17 (band well).
- Why: AC-06, AA-18, AR-28; H05-54, H14-27 (substrate, tank and well contacts are majority-carrier rings, Hastings §5.4.4 L15421–15446 PDF 264–265, §14.2.3 L43795–43799); HBGR needs a deep-N+ sinker or trench (L43849–43851); EBGR needs PBL plus a P+ sinker (L15614–15618); C6, C29; D20, D30.
- Current: `enum GuardRingType { Ecgr, Hcgr, Hbgr, Ebgr }` documented as minority-carrier rings (`kernel/analog/src/cell.rs:25-35`). `post_cell::implant_name`/`in_nwell` draw `Ecgr`/`Ebgr` as p+ substrate taps and `Hcgr`/`Hbgr` as n+ taps in n-well (`kernel/cells/src/post_cell.rs:310-323`). The annotator maps PMOS → `Hcgr` and NMOS → `Ecgr` (`backend/annotator/src/constraints.rs:67-71`). `tap_pitch_nm` and `enclosure_complete` are never read (`cell.rs:13-20`).
- Change:
  1. `kernel/analog/src/cell.rs`: `pub enum GuardRingType { Tap { in_well: bool }, Ecgr, Hcgr }`, with doc lines:
     - `Tap`: majority-carrier tap ring (p+ in substrate, or n+ in n-well when `in_well`).
     - `Ecgr`: n+ ring in its own n-well band, tied to a supply, collects electrons.
     - `Hcgr`: p+ collecting ring; needs a retrograde/isolated well that the deck declares.
     Delete `Hbgr`, `Ebgr`, `tap_pitch_nm`, `enclosure_complete`, and update every literal (`constraints.rs:73-83` and tests).
  2. `constraints.rs:67-71`: PMOS → `Tap { in_well: true }`, NMOS → `Tap { in_well: false }`. This is a rename, and the drawing does not change until REL-07 replaces the loop.
  3. `post_cell.rs`:
     - `implant_name`: `Tap { in_well: true }` and `Ecgr` → `nsdm`; `Tap { in_well: false }` and `Hcgr` → `psdm`.
     - `in_nwell`: `Tap { in_well }` → `in_well`; `Ecgr` → `false` for the filled-well test, because CELL-17 draws its band well.
     - Add `pub fn drawable(t: GuardRingType, p: &dyn Process) -> bool`: `Tap` → true; `Ecgr` → `p.layer("nwell").is_some() && p.layer("nsdm").is_some()`; `Hcgr` → `p.rule("retrograde_pwell", 0) != 0` (sky130 `false`, `pdks/sky130.json:69`).
     - `ring_gap` for `Ecgr` ≥ `space_between("nwell","diff")` (sky130 difftap.9, 340 nm).
- Tests (`post_cell.rs`):
  - `tap_rings_draw_as_before`: the GDS of every fixture's ring is unchanged byte for byte after the rename.
  - `ecgr_is_n_plus_in_its_own_well_clear_of_the_nmos`: owned here; passes after CELL-17.
  - `hcgr_is_not_drawable_on_sky130`.
- Acceptance: grep finds no `Hbgr`, `Ebgr`, `tap_pitch_nm` or `enclosure_complete`; bench GDS unchanged; REL-07 compiles against these names.

### GAP-06 EM derating: sidecar `em_derating` table and its reader
- Priority: P1. Effort: S. Depends on: FLOW-04. Owner plan: **REL (plan-06)**, as REL-05 step 0.
- Why: H15-38 (derating inert without T_ref, Ea, n; Hastings eq 15.25, L48776–48791, PDF 821); EM-42 (fallback with provenance); C7; D27.
- Current: `EmLimit.derating` is read only from the deck rule's `reference_temperature`, `activation_energy_ev` and `current_exponent` (`backend/verify/src/pdk.rs:537-541`), and no deck carries them (`GP/pdks/sky130.deck:491-495`).
- Change:
  1. Sidecar `cell.em_derating: { "t_ref_k": number|null, "ea_ev": number|null, "n": number|null, "_source": "…" }`, registered as one `Table` row in FLOW-04. Delete FLOW-06's `em_ref_temp_c`/`em_activation_ev`/`em_current_exponent` rows. Values (REL-05 step 5): sky130 363.15 K, `sky130_fd_sc_hd__nom.tlef:92,119`; gf180mcu 358.15 K, `gf180mcu.deck:478`; ihp_sg13g2 378.15 K, `ihp_sg13g2.deck:599`. `ea_ev` and `n` are null everywhere (REL §5 Q1).
  2. `Pdk::em_limit` (`pdk.rs:525-547`): when `of(s).derating` is `None`, read the table. REL-05 step 4's fallback (0.9 eV, n 1.1, `derating_assumed`) then applies unchanged.
- Tests: REL-05's `sky130_em_rating_is_90c_with_fallback_parameters` now passes, driven by this reader. Add `a_deck_rule_value_beats_the_sidecar` (a synthetic deck with `reference_temperature` → deck value, `derating_assumed == false`).
- Acceptance: REL-05's acceptance (sky130 met1 derated to ≈ 281 µA/µm at 125 °C, [derived] in plan-06).

### GAP-07 `Pdk::model_markers` from the deck recogniser
- Priority: P1. Effort: S. Depends on: FLOW-04. Owner plan: **FLOW (plan-08)**, restoring FLOW-06 step 5. Consumer: CELL-16.
- Why: H01-16 (flavour marker layers, hastings.txt L11447–11458); H12-21 (L35930–35936); the sky130 recognisers encode the markers (`nfet_01v8_lvt = (ngate and lvtn) not hvi`, `pfet_01v8_hvt = (pgate and hvtp) not hvi`, `nfet_g5v0d10v5 = ngate and hvi`, `GP/pdks/sky130.deck:192-202`); C8; D22.
- Current: nothing derives marker layers; CELL-16 is blocked; FLOW-06 registers a `mosfets` table that no plan fills.
- Change: `backend/verify/src/pdk.rs`:
  ```rust
  /// Layers the model's recogniser requires (`and`) and forbids (`not`) beyond the base gate layer,
  /// from its derived-layer expression; `None` when the model has no recogniser.
  pub fn model_markers(&self, model: &str) -> Option<(Vec<LayerId>, Vec<LayerId>)>;
  ```
  Algorithm: `deck_model(model)` (`pdk.rs:1116-1123`) → the device row → its marker layer → walk the derived-layer definition. Operands under `and` that are not the recogniser's gate layer are required; operands under `not` are forbidden. Delete FLOW-06's `mosfets` registry row (C8).
- Tests (sky130): `lvt_needs_lvtn_and_forbids_hvi` (`nfet_01v8_lvt` → ([lvtn], [hvi])); `hvt_pmos_needs_hvtp`; `core_nfet_has_no_required_marker` (`nfet_01v8` → ([], [hvi]) or ([], [])); the exact forbidden set is what the deck expression gives; the test asserts `required.is_empty()`.
- Acceptance: CELL-16's `an_lvt_and_an_hvt_device_sign_off_with_their_models` passes.
- Risks / notes: GPurify's derived-layer expression API was not traced for this document. If the recogniser is compiled and no longer an expression tree, fall back to a `markers` list per model in the sidecar (`Table`, sourced to the deck line).

### GAP-08 Per-epoch placement net-weight feedback (one controller)
- Priority: P1. Effort: M. Depends on: FLOW-08 step 5 (`Weights`), EXT-25 / PERF-06 (budget rows), RTE-21 step 3 (row measurement). Owner plan: **PERF (plan-07)**, reinstating PERF-25.
- Why: BAL2-19 ("raise the weights of the most sensitive parasitics … when all are at maximum, the placement must be regenerated"; naive ± updates oscillate, balasa_graeb_survey.txt L9397–9406, L9441–9449); SURV-22 (Bayesian optimisation over net weights driven by post-layout simulation); GRAEB-10 (the bottleneck spec drives effort); AF-05; C9; D34.
- Current: the placement weights are computed once per run (`frontend/library/src/lib.rs:295-296`, `gp::net_weights`) and never updated. RTE-21 step 5 escalates dr's routing weights inside one dr call only.
- Change: `frontend/library/src/lib.rs` (FLOW-08's loop), after each epoch's routing and before the next epoch:
  1. For each routing `PerformanceBudget` row j with residual r_j > 0 (RTE-21 step 3 `used − limit`), let T_j = the 3 nets with the largest contribution w_n·C(n) (plus wr_n·R(n)). The count of 3 is **[policy]**; BAL2-19 says "most sensitive".
  2. For each n ∈ T_j: `target = w0[n]·(1 + r_j)`; `w[n] ← 0.5·w[n] + 0.5·min(target, 4·w0[n])`. The damping β = 0.5 and the cap 4× match RTE-21 step 5 **[policy]**.
  3. Nets in no violated row relax: `w[n] ← max(w0[n], 0.9·w[n])` **[policy]**.
  4. When every T_j net of a still-violated row sits at the cap for 2 consecutive epochs, report the spec as `placement_limited` in `MetadataReport`.
  5. `Weights.place` is the only vector touched. Reset to `w0` on a cold epoch (FLOW-08 step 2), so warm epochs refine and cold ones explore.
  `RunStats` gains `weight_updates: u32`.
- Tests (`lib.rs` pure fn `feedback(w, w0, rows) -> Vec<f32>`):
  - `violated_row_raises_its_top_nets`: r = 0.5 → the net goes from 1.0 to 1.25.
  - `weights_cap_at_four`: after repeated r = 10, the weight is ≤ 4.0.
  - `slack_relaxes_toward_initial`.
  - `alternating_residuals_stay_bounded`: 20 epochs with r alternating 1/0 → max weight ≤ 4 and no monotone drift.
- Acceptance: PERF-24 A/B, feedback on vs off, seeds 1–5: the post-layout spec miss with feedback is ≤ without on every fixture with specs; the lex key median is no worse on ≥ 8/10 fixtures **[policy]**.
- Risks / notes: PLC-16's `PlacePerf` rows price the same parasitics in placement. The feedback changes HPWL weights only, not the PlacePerf rows, so the two act on different terms.

### GAP-09 Pre-search over-constraint check with a conflict report
- Priority: P1. Effort: M. Depends on: EXT-10 (IDs), EXT-14, EXT-26 (user seeds), FLOW-03 (saturation list). Owner plan: **EXT (plan-01)**, reinstating EXT-08 with a broader scope; step 3 lands in FLOW's file as a coordinated edit.
- Why: BAL2-48 (detect mutually conflicting constraints; weight-based resolution fails, balasa_graeb_survey.txt L13836–13867, L14081–14111, PDF 286–287, 291–292); NOTES-06 (report the minimal conflicting set instead of escalating penalties); today unsatisfiable budgets are only capped by `LAMBDA_MAX = 64` (`backend/gp/src/lib.rs:49, 96`).
- Current: no conflict detection. EXT-08 was cut because only AA-13 was considered (plan-01 L955).
- Change:
  1. New `backend/annotator/src/conflict.rs`, `pub fn check(p: &Problem, nl: &Netlist) -> Vec<Diagnostic>` with kind `"conflict"`, run at the end of `annotate`. Checks:
     - (a) a device in two `Symmetry` pairs or on two axes, which can only arise from sidecar seeds (EXT-26), since EXT-14 cannot create it;
     - (b) a device pair with `Proximity.max_distance_nm < Isolation.min_distance_nm`;
     - (c) a sidecar `Match` class on devices of different model, W or L (EXT-11 signature mismatch);
     - (d) a `Differential` net pair whose nets are touched by different device counts. This is reported only: exact mirroring is then impossible.
  2. Each diagnostic names the rule IDs (`BatchMeta.id`). A conflicting lower-priority rule under the survey's importance order (M_S ≻ M_B ≻ P_B ≻ S ≻ P_N, EXT-13) is dropped from the emitted `Requirements`, and the drop is reported, not silent.
  3. `frontend/library/src/lib.rs` (FLOW-08, after the first epoch's routing): for each routing C budget, compute the physical floor `C_floor = HPWL(pin centres)·min over routed layers of the minimum-width C per nm`. HPWL is a lower bound on the rectilinear Steiner length, **[derived]**. If `C_floor > limit`, report the budget as `infeasible_budget` in `MetadataReport`, and FLOW-03 excludes it from ρ escalation (it would otherwise saturate at `LAMBDA_MAX`).
- Tests (`conflict.rs`):
  - `proximity_below_isolation_is_a_conflict`.
  - `sidecar_double_pairing_is_a_conflict` (`SymmetricBlocks` [[a,b],[a,c]] → one diagnostic, the second pair dropped).
  - `clean_corpus_has_no_conflicts` (every EXT corpus circuit → 0).
  - (library) `c_budget_below_hpwl_floor_is_reported`: a 10 aF budget on a net whose pins are 100 µm apart → `infeasible_budget`.
- Acceptance: 0 conflicts on the 17 corpus circuits; each injected conflict is named with both rule IDs; no λ reaches `LAMBDA_MAX` on a budget reported infeasible.

### GAP-10 Prices keyed by stable constraint ID
- Priority: P1. Effort: S. Depends on: EXT-10 (`RuleBatch::meta`), FLOW-03. Owner plan: **FLOW (plan-08)**, as FLOW-03 step 5.
- Why: NOTES-04 (stable semantic IDs; prices keyed by kind and position are fragile); AR-18; the contract comment itself says "Reordering silently moves prices" (`kernel/analog/src/requirements.rs:16-18`); C10; D6.
- Current: `fn keys` (`backend/gp/src/lib.rs:151` on m0, after FLOW-03) keys every budget batch by `(kind(), ordinal among same kind)` into `Prices.priced: BTreeMap<(&'static str, u32), Price>` (`:27-28`).
- Change: `enum PriceKey { Id(analog::intent::ConstraintId), Ord(&'static str, u32) }`. `keys` returns `PriceKey::Id(m.id)` when `reqs.budget[bi].meta()` is `Some`, else today's `Ord`. `Prices` stores `BTreeMap<PriceKey, Price>`. The contract comment is updated to say that order matters only for untagged batches.
- Tests (`gp`): `reordered_tagged_batches_keep_their_prices` (two tagged batches swapped between `bind` calls → each keeps its λ); `untagged_batches_keep_todays_behaviour`.
- Acceptance: EXT-10's `ids_survive_permutation` plus this test; bench lex keys unchanged when no batch is reordered.

### GAP-11 Aspect-ratio limits for matched variants
- Priority: P2. Effort: S. Depends on: GAP-01, EXT-15/16 (class and kind on `Unitization`). Owner plan: **CELL (plan-03)**.
- Why: H13-46 (Hastings rule 9: current matching MIN ≤ 10:1, MOD ≤ 3:1, EXC as square as possible; voltage matching MIN ≤ 3:1, MOD/EXC square or nearly square; only subarray aspect counts for CC arrays; L42504–42516, PDF 714); C11.
- Current: finger folding aims each row "closest to square" regardless of class or kind (`frontend/library/src/cellgen.rs:600-613`, per H13-46 status).
- Change:
  1. `Unitization` gains `kind: Option<MatchKind>`, set by EXT-15/16.
  2. In `cellgen::draw_variants`, the aspect of a variant = max(w, h)/min(w, h) of the bounding box of one member's units (the CC subarray).
  3. Limit per (class, kind): Current 10 / 3 / 1.5, Voltage 3 / 1.5 / 1.5. The 1.5 standing for "square or nearly square" is **[policy]**; the source gives no number.
  4. Filter out variants above the limit. If none remains, keep the lowest-aspect one and add `("MatchClass", "no variant meets the aspect limit")` to `missing`.
- Tests: `current_mod_filters_4_to_1` (a 4:1 subarray is filtered, 2:1 kept); `last_variant_is_kept_and_reported`.
- Acceptance: PERF-23's ALRC "compactness" row has 0 failures on matched cells.

### GAP-12 Corner support fill and via arrays off corners on EM-critical nets
- Priority: P2. Effort: S. Depends on: RTE-14 (EM-critical filter, widths), REL-12 (front-row cut count). Owner plan: **RTE (plan-05)**.
- Why:
  - EM-25: 90° bends crowd current (lienig_em.txt L1421–1445, PDF 34–35).
  - EM-27: support polygons at bends and terminals (L4891–4894, Fig 3.17, PDF 106).
  - H15-39: two 135° angles instead of one 90°; via arrays belong in straight segments (hastings.txt L48818–48833, PDF 822).
  - H14-57: chamfers on ESD nets.
  - REL §0.3 assigns chamfers to RTE (plan-06 L40); no RTE item has them.
- Current: `jog_legs` draws flush L-corners whose legs overrun by `w/2` (`backend/dr/src/lib.rs:1072-1074`); `widen` fills an L of two widened trunks (`:1028-1030`). Shapes are `Rect` only, so no 45° edge can be drawn.
- Change:
  1. For EM-critical nets (RTE-14 filter), at every L-corner of two trunk segments of width w, add one inner-corner square of side w ("support polygon", EM-27) on the same layer. Its outer corner is snapped to the lattice and it is DRC-checked like any fill.
  2. When a layer change falls on an L-corner, extend the lower-layer wire past the corner by the via array's length along the outgoing direction and place the array on that straight run (H15-39 recipe (b)).
  No crowding credit is taken in REL-03: the source gives no factor.
- Tests (`dr`): `a_critical_corner_gets_a_support_square`; `a_corner_via_array_moves_onto_the_straight_run`; neither adds a DRC finding on sky130.
- Acceptance: every L-corner of an EM-critical net has its support square; `bench local` DRC unchanged.

### GAP-13 Latent antenna
- Priority: P2. Effort: S. Depends on: REL-02. Owner plan: **REL (plan-06)**.
- Why: H05-30 (min-spaced neighbours clear late in etch and act as one antenna even when separate nodes at the end; model by oversize–undersize, hastings.txt L13166–13172, PDF 228).
- Current: antenna pieces are per net (`kernel/analog/src/routing/stack.rs:284`).
- Change: `Stack::antenna` gains `others: &[Shape]`, the same-layer shapes of other nets at the stage. A piece is unioned with every foreign shape whose edge gap ≤ `latent_merge_nm`; the sidecar default is the layer's minimum spacing (sky130 met1 140 nm). The union's area is charged to every gate the union reaches. The `Antenna` rule passes `r.shapes` of all nets on that layer. Sidecar key `latent_merge_nm` (optional) is registered.
- Tests: `two_min_spaced_pieces_share_one_antenna` (two nets at 140 nm gap, each below the limit alone, above it together → violated); `a_wider_gap_is_independent`.
- Acceptance: T2 of REL (in-loop never passes what signoff fails) still holds; the in-loop model only gets stricter.

### GAP-14 User-declared deep-n-well isolated tubs for NMOS
- Priority: P2. Effort: M. Depends on: EXT-26 (sidecar), GAP-05, CELL-17 (n-well band). Owner plan: **CELL (plan-03)**, with the EXT-26 sidecar entry.
- Why: SUB-38 (buried n+ well tied to the highest potential, less sensitive to ground-path impedance, charbon L3813–3944, PDF 151–156); H14-12 (DNW for sensitive NMOS near switchers, hastings L43138–43145, L43915–43933); H12-16/17 (isolated NMOS tank grouping, never mix noisy and sensitive).
- Current: DNW is drawn only for NPN isolation (`kernel/cells/src/bjt.rs:31-34, 129-138`).
- Change:
  1. Sidecar constraint `{"constraint":"IsolatedTub","instances":[..],"tie":"<quiet supply net>"}` (EXT-26 extension). It is user-declared only, because the sources give no automatic threshold.
  2. CELL draws, around the listed cells' union: a `dnwell` rect, an n-well ring band (CELL-17 `WellShape::Band`) with an n+ tap tied to `tie`, and a p-well tap inside tied to the devices' bulk. Deck rules are read through `space_between`/`enclosure` (sky130 `dnwell` layer, `backend/verify/src/pdk.rs:1212`).
  3. LVS: the devices' bulk becomes an isolated p-well net. PERF-18 must bind it; until then the reference marks it `lvs-coverage/unverified`.
- Tests: `a_tub_is_drc_clean_on_sky130`; `the_tub_bulk_is_not_the_substrate` (extraction).
- Acceptance: a fixture with one declared tub signs off DRC 0 and reports the bulk-net coverage honestly.

### GAP-15 Finger-metal resistance by the rule of one-third
- Priority: P2. Effort: S. Depends on: CELL-19 (`Figures`). Owner plan: **CELL (plan-03)**; RTE-17 and REL-04 read the figure.
- Why: H15-57 (a singly terminated finger has `R_M ≅ R_S·W/(3·W_M)`, valid within 1.5 % for W ≤ d_pen = R_Si·W_M/R_S; two fingers `2R_S·W/(3W_M)`; Hastings App. E, eqs E.1–E.34, hastings.txt L51753–52113, PDF 870–876). `Stack::terminal_resistance_ohm` treats pins as points (`kernel/analog/src/routing/stack.rs:153-161`).
- Current: no strap resistance per terminal leaves the generator.
- Change: `Figures` gains `strap_ohm: Vec<(u8, &'static str, f32)>` = per (owner, S/D) `R_S(strap layer)·W_f/(3·W_strap)/N_f`, with R_S from `Process::sheet_ohm`. `CommonNodes` (RTE-17) and `IrDrop` (REL-04) add the owner's `strap_ohm` in series at each terminal.
- Tests: `one_third_rule_numbers`: R_S 0.125 Ω/□, W_f 10 µm, W_strap 0.17 µm, one finger → 2.45 Ω ± 0.01 [derived].
- Acceptance: CommonNode ΔR on `ota` includes the strap term; the reported difference is ≤ the MAT-06 allowance or reported as a violation.

### GAP-16 Dominated-variant pruning before the escalation odometer
- Priority: P2. Effort: S. Depends on: none. Owner plan: **FLOW (plan-08)**, since `cellgen.rs` is FLOW-owned glue.
- Why: BAL2-39 (per-block Pareto shape functions; drop dominated variants, balasa_graeb_survey.txt L12769–13067, PDF 264–271).
- Current: `escalate` walks a mixed-radix odometer over every alternative (`frontend/library/src/cellgen.rs:401-418`), with no dominance pruning.
- Change: after `seed_assignment` prices every alternative (`cellgen.rs:327`), drop an alternative v of an unmatched cell (`Unitization.class == None`) when another alternative u has `w_u ≤ w_v`, `h_u ≤ h_v`, DRC+ERC count `≤`, and at least one strict inequality. Matched cells keep every alternative, because pattern quality is not in (w, h). Report `pruned` in `RunStats`.
- Tests: `a_dominated_alternative_is_pruned`; `matched_cells_are_never_pruned`; `escalate_still_terminates`.
- Acceptance: FLOW T9 (redundant work) no worse, and `bench local` lex keys unchanged or better on every fixture.

### GAP-17 Node voltages in the op point; V_SB identity of matched pairs; capacitor voltage rating
- Priority: P1. Effort: M. Depends on: PERF-09. Owner plan: **PERF (plan-07)** for step 1 (PERF-09 step 7); **REL (plan-06)** for steps 2–3 (REL-10 steps 4–5).
- Why: H05-17 (net voltage envelope; prerequisite for voltage checks); EXT-17 `OpFacts.net_mv` (plan-01 L629); H01-06 (matched pairs need identical bulk and equal V_SB, Hastings §4.2.2, L9609–9614, §1.4.1 L3329–3336); H06-32 (V_max = t·E_max, eq 7.4, L19212–19303, PDF 328–329).
- Current: `OpPoint` has per-device power, Id, headroom and gm only (`frontend/library/src/oppoint.rs:13-29`). The control block prints `show m : id,vds,vdsat,gm` (`oppoint.rs:238`). No per-net voltage exists anywhere.
- Change:
  1. PERF-09 step 7: after `op`, the control block adds `print all` (every node voltage). `OpPoint` gains `pub net_v: Vec<Option<f64>>` indexed by `NetId`, and ground nets read 0.0. EXT-17's `OpFacts.net_mv` is filled from it.
  2. REL-10 step 4: `PairAging` gains `dvbs_mv`. For a pair whose bulk nets differ, or whose |ΔV_BS| > 1 mV (numerical tolerance, **[policy]**), and whose class is ≥ Moderate, emit `rel/vsb_mismatch:{a}/{b}` as a hard violation (H01-06: a hard constraint for matching).
  3. REL-10 step 5: the per capacitor recipe key `v_max_mv` (sourced; null on every shipped deck because no ref/ source gives the sky130 MIM rating) is checked as |V_P − V_N| ≤ v_max. Null → counted in `voltage_unknown`.
- Tests: (`oppoint.rs`) `print_all_reads_node_voltages` (canned block); (`reliability.rs`) `unequal_vsb_in_a_moderate_pair_is_a_violation`; `a_cap_without_rating_is_unknown`.
- Acceptance: `ota` with an op testbench → `net_v` resolved for every net; EXT-17 regions unchanged; REL T9 reports V_SB per matched pair.

### GAP-18 Capacitor-array variant choice by metric
- Priority: P2. Effort: S. Depends on: MAT-11. Owner plan: **MAT (plan-02)**, with a one-line cellgen hook (FLOW).
- Why: CC-24 (spiral best f3dB / worst INL, chessboard the reverse; select by spec, DACP Tables II–III, VI, cc_dac_constructive.txt L934–945, L998–1007, L1031–1043).
- Current: `metrics()` (`kernel/cells/src/cap_array.rs:357`) runs only in tests (`:685` onwards).
- Change: when a bank's `Unitization.class == Some(Exceptional)`, order its alternatives by `(msys at g = 1e-4/µm, then inl, then area)`, using MAT-11's per-variant metrics computed at draw time. The escalation odometer then tries the best-matching variant first. No spec INL limit is assumed ("not given" per fixture).
- Tests: `exceptional_banks_try_the_lowest_msys_first`.
- Acceptance: `dac4` with an Exceptional class reports its chosen variant's M_sys ≤ every other alternative's.

### GAP-19 Minimize drain or source by node impedance
- Priority: P2. Effort: S. Depends on: EXT-17/18 (`NetFacts.z_ohm` or net class). Owner plan: **CELL (plan-03)**.
- Why: H12-29 (even section count leaves one extra S or D; minimize the drain by default, the source when it sits on a high-impedance node; Hastings §12.2.8, L36550–36564, PDF 612–613).
- Current: a single device always starts with S at region 0 (source outer, minimized drain; `kernel/cells/src/mosfet.rs:324-328`), with no per-node choice.
- Change: for single-device rows with even nf, offer both outer conventions as variants when the source net is not a rail. `cellgen` keeps the one whose outer electrode's net has the lower `z_ohm` (EXT-18), or is Supply/Ground/Bias. Without `z_ohm`, today's source-outer convention stays.
- Tests: `a_follower_minimizes_its_source`.
- Acceptance: CELL-19's `Figures.sd` shows the smaller S/D area on the higher-impedance terminal for the new variant.

### GAP-20 Study of Hastings L52119–73937 (figure long descriptions)

Status: done in M0 (`6da8068`); the study is `ref-hastings-99-figure-descriptions.md`.

- Priority: P2. Effort: S. Depends on: none. Owner: **study task** (a new `docs/plans/ref-hastings-99-figure-descriptions.md`), feeding MAT, PLC and CELL corrections.
- Why: §5; the 566 "Long description" blocks (hastings.txt L57404–73937) state figure values in text, e.g. Fig. 8.19 bars at L64797.
- Change: read L57404–73937 in full. Index each figure (number, page, line range) and extract every numeric value, then settle these open items:
  - MAT open question 7: eq. 8.29 vs Fig. 8.20 minima (description at ~L64817).
  - H13-53/H13-55 WPE and rule-23 distances (plan-04 L761).
  - The WPE 5 %/1.8 µm and 25 %/0.95 µm points (PLC-13).
  - The Fig. 10.24 2:1:2 arrangement (MAT-02).
  - Every other study value marked "read from the figure" or "not given".
  Record the agreement or disagreement per value with line numbers. The index (L52119–54715) and the figure/table lists (L54716–57403) need no study: they are captions.
- Tests: none (study).
- Acceptance: each listed open value is resolved or explicitly "not stated in the description"; the owning plans' notes are updated.
- Scope cut (recorded at review of the study, for the plan owner to confirm): "every numeric value" was taken as every stated value (dimensions, ratios, voltages, currents, coordinates); counts of drawn shapes (contacts, vias, fingers, emitters, bars, rows) are not listed. They occur in at least 124 blocks, none bears on an open plan value, and `ref-hastings-99` §4 says so with examples and line ranges.

---

## 7. Milestone placement of the GAP items

| GAP | Lands with | Blocks |
|---|---|---|
| 04 | EXT M0 (P0) | EXT T6, REL T10 |
| 05 | CELL M0 | REL-07, CELL-17 |
| 01 | before EXT M1 / CELL M2 / PLC M1 | EXT-12/16, CELL-12/14/23, PLC-13/14/29, RTE-16, PERF-23 |
| 06, 07 | FLOW/REL M1 | REL-05, CELL-16 |
| 10 | with FLOW-03 | — |
| 02, 03, 09, 17 | EXT M3 / MAT M3 / REL M3 | REL-07 T7, REL-10 T9 |
| 08 | PERF M3 | PERF T11 |
| 11–16, 18–20 | P2 milestones of their owner plans | — |

## Verification log

- Code facts re-read for this document:
  - `kernel/analog/src/cell.rs:5-35`, `backend/annotator/src/constraints.rs:60-88`, `kernel/cells/src/post_cell.rs:305-325` (ring enum, emission, drawing).
  - `backend/annotator/src/emit.rs:244-288` (isolation).
  - `kernel/analog/src/requirements.rs:10-24` and `backend/gp/src/lib.rs:27, 49, 65-120` (prices and keys).
  - `frontend/library/src/lib.rs:290-298` (net weights).
  - `backend/verify/src/pdk.rs:520-548` (EM limit).
  - `pdks/sky130.json:55-80` (tiers, `lod_moat_ext_nm` has 2 entries).
  - `frontend/library/src/cellgen.rs:395-420` (odometer).
  - `kernel/cells/src/cap_array.rs:357, 685`, `kernel/analog/src/routing/stack.rs:284`, `backend/dr/src/lib.rs:1026-1076`, `frontend/library/src/oppoint.rs:13-29, 238`.
- Reftext:
  - `wc -l` of all 15 files.
  - hastings.txt L52110–52135 (end of App. E, start of the Index); L54700–54730 (end of the Index, start of Contents); L55433, L56293 (lists of figures and tables); L57404 (first "Long description"); L64797, L64817 (Fig. 8.19/8.20 descriptions); 566 "Long description" markers.
- Reference values in GAP items are taken from the study-doc entries named in each "Why", and were not re-opened in the PDF here: Hastings rules 9, 12, 19, 21, 22; Tables 13.4 and 13.5; §5.4.1 and §14.2 thresholds; §4.2.2; eq 7.4; App. E; Charbon §8.2 and §9; Lienig §2.3.3 and §3.6; BAL2 §4.5.3.2, §6.4 and §7.4; DACP Tables II–III and VI.
- Unverified:
  - sky130 = bulk substrate: the only source is a code note (GAP-04 step 1).
  - GPurify's recogniser-expression API for `model_markers` (GAP-07 risk note).
  - That ngspice `print all` inside the existing control block prints every node (GAP-17 step 1); to be checked by the PERF-09 implementer against a canned run.
