# Plan 02 — Matching models, gradient cancellation and common-centroid pattern generation (MAT)

Plan ID prefix: **MAT-NN**. Working tree of 2026-09-28 (uncommitted changes included). Every code claim is `path:line` in that tree; every reference claim names the source, section/equation and the `reftext/*.txt` line range (PDF page where a value was only legible on the page image, as recovered by the study docs). Numbers marked *derived* are arithmetic on source numbers, done here or in the cited study doc; they are not source statements.

Inputs read in full: `docs/plans/audit-02-analog-rules-and-core.md`, `audit-03-cells-device-generators.md`, `audit-04-placement.md`, `ref-mismatch-papers.md`, `ref-common-centroid-papers.md`, `ref-hastings-08-…`, `ref-hastings-13-…`, `ref-hastings-09-…`, `ref-hastings-12-…` (§3 entries), `ref-00-prior-notes-handbook.md` (roadmap, §2.6, NOTES-16…33, 53), `ref-balasa-graeb-survey-part1/part2` (matching-related entries). Grepped: LAMP-09…14, GRAEB-02/06/38, H01-43, H06-06, H14-41, H15-48/56/58, SUB-43, SURV-11, AV-11, AV-32, AA-09/10, AF-01/19. Source files read for this plan: `kernel/analog/src/placement/{cc,matching_pair,thermal,mod}.rs`, `kernel/analog/src/{lib,cell,constraints,rule}.rs`, `kernel/core/src/{units,thermal,layout,macro,process}.rs` (relevant parts), `backend/annotator/src/{emit,constraints,block,netrole}.rs`, `backend/annotator/src/lib.rs:25-166`, `kernel/cells/src/{cap_array.rs:1-420, mosfet.rs:25-400,600-800, builder.rs:215-320, bjt.rs, resistor.rs:1-160,290-340, lib.rs:25-56}`, `frontend/library/src/cellgen.rs:1-135,240-440,443-600`, `frontend/library/src/lib.rs:150-175,250-300,475-530,570-600,755-880,1140-1160,1238-1264`, `frontend/library/src/metadata.rs:1-206`, `benchmarks/src/bench.rs:255-360`, `pdks/sky130.json` (whole cell section), mismatch keys of the other three sidecars, `backend/verify/src/pdk.rs:505-530,945-965`. Hastings PDF pp. 396–398 (eqs 8.27–8.33, Figs 8.19–8.20) were opened for MAT-17.

ID note: code comments already use handbook constraint-family IDs of the same shape (`backend/annotator/src/emit.rs:21` "MAT-07", `emit.rs:200` "MAT-06", `kernel/cells/src/mosfet.rs:25` "MAT-11"; they come from the docs/ref/Notes 84-family catalog). They are **not** this plan's items. Commits for this plan should say "plan MAT-NN".

---

## 0. Scope & boundaries

### 0.1 This plan owns

1. **The mismatch model of a placed matched set** — one ledger per matched pair: random area term (Pelgrom, per-device unequal areas), statistical distance term, quadratic term, deterministic thermal and LOD terms, mapped to the set's electrical domain (input offset in mV, or current error in %), spent against **one** allowance. Stress is a named slot that stays *unknown* (no die context exists; see §0.3).
2. **CC quality metrics**: weighted unit moments to order *n*, the cancelled-gradient-order certificate, second-moment residue, orientation Φ (Hastings eq. 13.61) and channel-axis parallelism, and for capacitor/resistor banks M_sys, M_rand, ρ_pq and INL/DNL.
3. **Pattern algorithms** as pure functions consumed by generators: point-symmetric grid assignment for *k* members (BJT arrays, general capacitor sets, resistor rows), diffusion-legal common-centroid MOS rows for **ratioed** counts, the Nth-order central-symmetric recursion, split-DAC slot assignment, non-integer-ratio segmentation.
4. **Class, kind and family on the matching rules** (MAT-07): the `MatchedSet`/`OrientationSet` rules carry the set's class, kind and family and change their emission by class. The class enum, the deck tier accessor and keys (FLOW-06), the 6σ limit table and class inference (EXT-16), and the class-driven dummy / moat / WPE / gate-extension / resistor-floor geometry (CELL-12, CELL-14, CELL-23) are **not** MAT's; MAT consumes them.
5. **Routing-induced mismatch metrics inside generated arrays**: per-bit route length and via spread (exists). Per-bit Elmore τ / f3dB and the ratiometric-jumper metric were cut (appendix).
6. **kernel/analog rule changes**: new `MatchedSet` and `OrientationSet` placement batches; deletion of `MatchingPair` (struct), `ThermalGradient`, `CentroidGroup`; two new `RuleBatch` hooks (`offset_allowances`, `ledger_rows`) and implementations of PLC-03's `matched_pairs` hook on the new batches.

### 0.2 Consumed from other plans

| From | What MAT needs | Where it bites |
|---|---|---|
| EXT | Matched sets beyond 2-device leaves: k-output mirrors (AA-01), R/C/BJT/diode sets and DAC banks (AA-04), ratio inference and a common unit for ratioed members (AA-09, H12-28, H13-22), class and kind inference (AA-10), diode class compatibility (H09-42) | MAT-07/08/10/18 emit per EXT's sets; until then MAT keeps today's 2-device leaves and `Moderate` |
| CELL | Drawing MAT's patterns: `bjt.rs` variants, `mosfet.rs` row order and k-row stacking, resistor `dev_nf` (AC-02) and segment order, cap MIM device (AC-01), consuming the class dummy/moat/WPE policy (AC-13, AC-22), aspect variant filter (H13-46) | MAT-02/03/07/12/15/18 give functions + tests; CELL owns geometry consequences |
| PLC | Units and power attached to gp's layout (AR-42/AP-10); rotation/shape groups in dp (AP-03/AP-04); symmetry mode mirror vs perfect (AP question 2) using MAT's `mirror_allowed`; cost normalisation (AR-08) | MAT-04/05 work without them; PLC makes gp see them |
| RTE | `CommonNodes` consumes MAT-06's remaining allowance; matched-route keep-outs (H13-51, H08-28) and differential RC stay RTE | MAT-06 |
| FLOW | One W/nf/m convention (AF-01, AR-46) so op-point g_m/I_D is right; sidecar plumbing (`Process::tier` impl in `verify`); die temperature from `Config.op` | MAT-07/09/14 |
| PERF | Bench prints MAT-13 ledger rows; per-set SPICE Monte-Carlo runner (MAT-21); hand-layout comparison (AV-03) | MAT-13/21 |
| REL | Thermal field fidelity (finite die, deck k; AR-24) | MAT-14 consumes `Layout::rise_at_point_mc` whatever model backs it (open question 5) |

**Cross-plan contracts fixed by the implementer review** (each other plan already names these; MAT must not re-implement them):

| Owner item | What it defines | MAT use |
|---|---|---|
| FLOW-06 (plan-08) | `pnr_core::MatchClass { Minimal, Moderate, Exceptional }` in `kernel/core/src/process.rs`; `Process::tier(&self, key, MatchClass) -> Option<i32>`; tier keys `wpe_clearance_nm`, `lod_moat_ext_nm`, `dummy_poly_reach_nm`, `fill_block_halo_nm`; coefficient slots `abeta_n_pct_um`, `abeta_p_pct_um`, per-recipe `k_a_pct_um` (value `null` until measured) | MAT-07 imports `MatchClass`; MAT-09/10 fill the coefficient slots by characterisation |
| EXT-12 (plan-01) | `analog::intent::{MatchKind, MatchedSet, Member, …}`, `Unitization.class: Option<MatchClass>` | MAT-04 defines `MatchKind { Voltage, Current, Ratio }` in `analog::matching::mismatch` **if it lands first**; EXT-12 then `pub use`s it instead of redefining (and vice versa). `intent::MatchedSet` (the extracted set) and `placement::MatchedSet` (MAT-04's rule) share a name in crate `analog`: every importer names one of them by path; neither is imported unqualified next to the other |
| EXT-16 (plan-01) | `limits(Family, MatchKind) -> [f32; 3]`, `class_of`, `kind_of` | MAT-08 takes the limit value as input; MAT does not keep a second table. `Family` is defined once, in `analog::matching::class` (MAT-07), with a `Diode` variant; EXT-16 imports it |
| EXT-17 (plan-01) | `Evidence.op.dev[d]: Option<DeviceOp { id_ua, gm_us, … }>` inside the annotator | MAT-09 reads g_m/I_D from it (no second op-point field) |
| EXT-20 (plan-01) | Rewrite of `emit::placement` per matched set; `intent::MatchedSet.allowance` | After MAT-04, EXT-20 emits one `placement::MatchedSet` per (reference, member) pair with `budget = Budget::Allowance(allowance)` (MAT-04; the value is already the 1σ systematic allowance, so no quadrature split is applied) instead of its interim `/√n` split over `MatchingPair`/`CentroidGroup`/`ThermalGradient` (EXT-20 step 2 already says MAT's estimator replaces the split) |
| EXT-29 (plan-01) | Sizing/structure diagnostics: area for voltage matching (H13-43), length for current matching (H13-44), CLM (H13-33), `D*` per set (MM-06) | MAT does not emit sizing notes or `D*` |
| CELL-10 (plan-03) | mosfet.rs wiring of ratioed MOS orders (`finger_sequence`, `enumerate`, `mirror_ratio` fixture) | calls `pattern::diffusion_cc_row` (MAT-03) instead of its own `pair_order` |
| CELL-12 / CELL-14 / CELL-15 / CELL-21 / CELL-23 (plan-03) | class-scaled MOS environment; resistor arrays (jumpers equalised by construction, dummies); k-row MOS grids (rows ∈ {1, 2, 4}); split-DAC drawing; resistor width/length floors | call `pattern::segment_row` (MAT-12), `pattern::nth_order_rows` (MAT-15), `dac::{attenuation_cap, nonunit_dims, split_dac_assign}` (MAT-18) |
| PLC-03 (plan-04) | `RuleBatch::matched_pairs(&self, out: &mut Vec<(u32, u32)>)` hook that dp uses to turn matched cells together | MAT-04 and MAT-05 implement it on their batches |
| PERF-27 (plan-07) | `perf::pair_sigma_mc`, the per-set SPICE MC runner | fills MAT-21's `sigma_rand_override` |

### 0.3 Explicitly not planned (no owner input or no data)

- Die-level stress placement (H08-48/49, H13-48, H09-18): needs a die context (outline, block origin, package) that `library::run` does not take; the ledger's stress slot reports *unknown*.
- A WPE ΔVT model (AR-09 fix direction): no deck carries BSIM WPE parameters; the geometric `Environment` rule stays, with class thresholds (MAT-07).
- Correlation-aware MOS dispersion (CC-07): no deck has σ_s/R_L. Capacitor correlation (former MAT-19) is deferred; see "Cut or deferred items".
- An interdigitated (ABAB) MOS pair variant (CC-40/41): an ABAB boundary lands on a drain and shorts two drains over shared diffusion (`kernel/cells/src/mosfet.rs:87-88`); the merged-vs-apart dual topology (`frontend/library/src/lib.rs:169-189`) already lets the flow pick the non-CC option.
- Priority sub-grouping for ≥3 members (H08-37): the balanced assignment of MAT-02 treats all members at once; revisit only if EXT emits per-pair weights.

---

## 1. Audit digest (current state, verified in the source)

### 1.1 Scoring of matched devices (kernel/analog + annotator emission)

| Item | Fact | Consequence |
|---|---|---|
| Emission | Each 2-device leaf of kind DiffPair/CurrentMirror/Load gets one `MatchingPair` (budget only when S/A > 0, else cost; `backend/annotator/src/emit.rs:159-171`), one `ThermalGradient` (budget+cost, `emit.rs:176-183`) and its slot-0/slot-1 devices appended to **stage-wide** `a_side`/`b_side` (`emit.rs:196-197`); one `CentroidGroup` per stage over the pooled sides (`emit.rs:216-238`) | AR-02: opposite offsets of different pairs cancel; which member is "a" is catalog slot order |
| Allowance | `Pelgrom::new` sets η from `offset_sigma_mv` (`emit.rs:83-91`) or `GRADIENT_SHARE = 0.3` (`emit.rs:52`). The same η·σ_rand is the limit of MatchingPair (`matching_pair.rs:49-60`), CentroidGroup's gradient and LOD halves (`cc.rs:137,146-148`), ThermalGradient (`emit.rs:96-101`) and CommonNodes (`frontend/library/src/lib.rs:794`) | AR-04: the allowance is granted in full to 4–5 rules; the sum can overspend it with every rule passing |
| MatchingPair | Distance = cell **bbox centres** (`kernel/analog/src/placement/matching_pair.rs:67-71`); after `retarget` a merged pair is `(c, c)` (`kernel/core/src/ids.rs:38-46`, `frontend/library/src/lib.rs:1146-1153`) → D = 0 | AR-01: measures nothing for merged pairs |
| ThermalGradient | Frozen `temp_mc` at cell centres for `satisfied/residual` (`kernel/analog/src/placement/thermal.rs:35-37,59-62`), live field for cost (`thermal.rs:30-33`); `AT_SPEC_COST = 3e5` hand-tuned (`thermal.rs:25`); group = hottest member (`kernel/core/src/thermal.rs:60-69`, dead: no producer emits `Target::Group`) | AR-01 (ΔT = 0 for merged pairs), AR-24, AR-43 |
| CentroidGroup | First moments only (`cc.rs:54-63`); coincidence tolerance `COINCIDENCE_TOL = 0.01` of the extent (`cc.rs:86`); LOD side means unweighted (`cc.rs:123-130`); `unknown` reports 1 while `residual` still charges a bbox proxy (`cc.rs:144-148,167-171`); citation "Hastings §13.3 r9" for LOD (`cc.rs:40`) is rule 12 | AR-10, AR-37, AR-33, AR-32, AR-41 |
| Gate area | MatchingPair uses the **smaller** gate (`emit.rs:159`), CentroidGroup the **larger** (`emit.rs:220`); gate area = W·L·max(nf,m) (`backend/annotator/src/lib.rs:155-161`, `constraints.rs:22-24`), again at `frontend/library/src/lib.rs:792` | AR-40, AR-46 |
| Distance term | S_VT is a PROXY from 130 nm data (`pdks/sky130.json:97-98`); with η = 0.3 the check binds only at D ≥ 1748/√WL µm for sky130 n (audit-02 §2.0) | AR-03: never binds at block scale |
| Orientation | No rule reads `PlacedUnit.phi` (`kernel/core/src/units.rs:60-61,120,132-135`); Φ is checked only inside a merged cell at cell generation (`frontend/library/src/cellgen.rs:256-272`); dp's rotation guard keys on the abutment table (`backend/dp/src/lib.rs:555-560` reads `Layout.groups`, which the flow sets to the abutment table at `frontend/library/src/lib.rs:577-579`), truncated for mixed polarity (`backend/annotator/src/lib.rs:106-117`) | AR-36 |
| Thermal limit fallback | `THERMAL_MAX_DELTA_MC = 500` mK when TC or A_VT is missing (`emit.rs:40-42,100`); `by_polarity` returns `None` for non-FETs (`emit.rs:107-113`) | BJT/diode/passive sets get no model (H09-02, H09-15) |
| Deck tiers | `wpe_clearance_nm [2000,3000,5000]` and `lod_moat_ext_nm [3000,5000]` (`pdks/sky130.json:59-63,70-73`) are read by no code; `*_moderate` scalars are (`kernel/cells/src/mosfet.rs:293,629`, `frontend/library/src/lib.rs:871-872`); the note on `vt_tc_uv_per_k_p` says "Not read by the library yet" (`pdks/sky130.json:130`) but `frontend/library/src/lib.rs:507` reads it (stale note) | H13-42/47/53 |
| Reporting | Bench prints worst usage and Σ Θ per kind (`benchmarks/src/bench.rs:321-360`), never σ in mV; `MetadataReport` has no per-set rows (`frontend/library/src/metadata.rs:71-86`) | MM-39 |

### 1.2 Pattern generation (kernel/cells)

| Generator | Fact | Consequence |
|---|---|---|
| BJT | `unit_order` sorts slots by radius then angle and deals whole members smallest-first (`kernel/cells/src/bjt.rs:208-231`); only 1:8 is tested (`bjt.rs:273-281`); matched sets offered only as the square (`bjt.rs:38-47`) | H09-49 (traced by hand in the study doc): [2,2] → AA/BB, [1,4] → (½,¼)-pitch offset with an empty slot, [1,2], [1,6] off-centroid |
| MOS ratioed | Unequal `dev_nf` → `greedy_centroid` (`mosfet.rs:784-788`, `builder.rs:271-292`), which ignores that multi-device rows make every even region a drain (`mosfet.rs:324-328`); [2,4] → BBAABB shorts drains, cellgen rejects every variant (`cellgen.rs:138-141`); `enumerate` keys the CC check on `dev_nf[0]` (`mosfet.rs:78,90`) | AC-05: ratioed mirrors never merge |
| MOS equal | `centroid_sequence` (`mosfet.rs:730-750`) and 2-row relabelling (`mosfet.rs:152-156`) give ABBA and ABBA/BAAB; nothing beyond 2 rows | NTH order ≥ 3 missing (CC-04) |
| MOS env | Moat `lod_moat_ext_moderate` only for multi-device rows (`mosfet.rs:293`), WPE halo `wpe_clearance_moderate` for matched PMOS (`mosfet.rs:629`), dummies from `dummy_gates_per_end` default 1 (`mosfet.rs:54-56`); comments cite "§13.3 r9" (`mosfet.rs:278,291`), which is rule 12 | AC-13, AC-22, AR-41 |
| Cap array | General sets: `general_assign` point-symmetric, odd counts to the centre or paired across it (`cap_array.rs:140-192`); `metrics()` reports first moment, **radial** ⟨r²⟩ (`cap_array.rs:366-374`), INL/DNL under a *linearised* field at 4 angles (`cap_array.rs:376-391`) and is called only from tests (`cap_array.rs:699,786`) | CC-02/05/24, NOTES-20: radial moment misses x²−y² and xy |
| Resistor | Same segment count for every member, `seg_l = unit_l / n` ignoring `dev_nf` (`resistor.rs:81-82,101`); interleave = `greedy_centroid` on equal counts (`resistor.rs:304-310`); unit weight = body area (`resistor.rs:124`); jumpers li for adjacent columns, met1 + 2 mcon otherwise (`resistor.rs:142-154`); direction alternates per segment (`resistor.rs:110,125`) | AC-02/15, H08-07/20/39; thermoelectric cancellation already holds (H08-45) |

### 1.3 Bugs this plan fixes first (P0)

- **B1** AR-01 merged pairs read D = 0 and ΔT = 0 → MAT-04.
- **B2** AR-02 stage-pooled centroid → MAT-04.
- **B3** AR-04 allowance double-spend (placement rules and CommonNodes) → MAT-04 + MAT-06.
- **B4** AR-33/AR-37/AR-40 unknown charged, arbitrary 1 % tolerance, inconsistent gate area → MAT-04.
- **B5** H09-49 BJT arrays off-centroid for every ratio but 1:8 → MAT-02.
- **B6** AC-05 ratioed MOS rows short drains → MAT-03.
- **B7** AR-36 orientation unchecked across cells → MAT-05.
- **B8** NOTES-20 radial second moment certifies a cancellation that does not hold → MAT-01 (metric), MAT-11 (cap array).

---

## 2. Target — "better than hand layout", measurable

Hand layout applies Hastings' rules by eye to the pairs a designer remembers and checks coincidence visually. The targets below are things a hand layout does not produce, each with how it is measured.

| # | Target | Measure (where) |
|---|---|---|
| T1 | Every CC-feasible matched set drawn in one cell has exact coincidence: \|Δm\| ≤ lattice/2 (5 nm on sky130, `cut_lattice` = 2×5 nm grid) | `MatchedSet` coincidence usage ≤ 1 for 100 % of merged CC-feasible sets on the 10 bench circuits; unit tests in `pattern.rs` |
| T2 | For every integer ratio in Hastings Table 8.5 and every BJT ratio 1:N (N ≤ 16, ≤ 1 odd member), the generated pattern is exactly CC and its second-moment residue is ≤ the textbook pattern's | golden tests `pattern.rs::table_8_5_*` (MAT-12), `bjt.rs::every_bjt_ratio_is_common_centroid` (MAT-02). Worked values: 3:1 residue 1.333 vs book 4.0 pitch²; 5:4 0.30 vs 3.0 (*derived*, §3 MAT-12) |
| T3 | Ratioed MOS mirrors: 100 % of count vectors for which `diffusion_cc_row(.., Drain or Source)` is `Some` merge into a legal CC row (today 0 %) | `pattern.rs::diffusion_rows_*` property test (MAT-03); merge count on `mirror_ratio` and the bench fixtures after CELL-10 wires it |
| T4 | Two-member MOS sets with 2^k units per member are offered a variant cancelling gradient order ≥ min(k, 3) (hand layout stops at order 2, ABBA/BAAB); NTH Table I: order-3 patterns reach 0 % residual at order 3 where order 1 leaves 5.22 % (nth_order.txt L294–310) | `moments::cancelled_order` on generated variants (MAT-15) |
| T5 | Systematic spend is counted once per pair: gradient + thermal + LOD + common-node IR ≤ η·σ_rand (or the class allowance); 0 pairs where each old rule passes and the sum fails | `mismatch.rs::one_allowance_is_spent_once`; MAT-06 flow test |
| T6 | 0 channel-axis and 0 Φ violations (Hastings rule 7 / eq. 13.61) on every fixture, including separately drawn partners in mixed-polarity composites | `Orientation` hard row in the bench table; dp test (MAT-05) |
| T7 | Every matched set is reported in its spec unit: σ_rand, σ_layout, μ_sys, allowance (mV or %), cancelled order, Φ status, with unknown terms explicit | `MetadataReport.matched` (MAT-13); `flow_smoke.rs` assertion |
| T8 | No regression while scoring changes: DRC 0 and "LVS MATCH" on all 10 bench circuits; HPWL and footprint within ±5 % of the pre-MAT-04 baseline (single seed) | `bench local` before/after M2 |

---

## 3. Work items

### MAT-01 Moment and orientation kernel

Status: done in M1 (`f447220`; matching merge `59371b5`).

- **Priority** P0. **Effort** S. **Depends on** none.
- **Why**: foundation for B1/B2/B7/B8. CC-01/02/03, NOTES-19/20/22, H08-35/36, H13-25/40, MM-26, AR-10, AR-36.
- **Current**: first moments exist twice (`kernel/analog/src/placement/cc.rs:54-63`, `kernel/cells/src/cap_array.rs:361-374`); second moment only radial (`cap_array.rs:366-374`); Φ equality exists only as the private `currents_run_alike` in `frontend/library/src/cellgen.rs:261-272`.
- **Change**:
  1. Create `kernel/analog/src/matching/mod.rs` (`pub mod moments; pub mod pattern; pub mod mismatch; pub mod class; pub mod dac;` — the last four are filled by later items) and add `pub mod matching;` to `kernel/analog/src/lib.rs:13-19`.
  2. Create `kernel/analog/src/matching/moments.rs`:
     ```rust
     /// One weighted unit (nm, electrical weight, S→D direction).
     #[derive(Clone, Copy, Debug, PartialEq)]
     pub struct Pt { pub x: f64, pub y: f64, pub w: f64, pub phi: (i8, i8) }
     impl From<pnr_core::PlacedUnit> for Pt { /* x,y as f64, w = weight as f64, phi */ }
     impl From<pnr_core::Unit> for Pt { /* local frame */ }

     /// Raw weighted sums of one device, about the origin (single pass, f64).
     #[derive(Clone, Copy, Debug, Default, PartialEq)]
     pub struct Sums { pub w: f64, pub x: f64, pub y: f64, pub xx: f64, pub xy: f64, pub yy: f64,
                       pub phi_x: i32, pub phi_y: i32, pub n: u32, pub axis: Axis }
     #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
     pub enum Axis { #[default] None, H, V, Mixed }   // H: every unit |φx|=1; V: every |φy|=1
     pub fn sums(pts: impl IntoIterator<Item = Pt>) -> Sums;
     impl Sums {
         pub fn centroid(&self) -> Option<(f64, f64)>;            // None when w == 0
         /// Second central moments about `c`, per unit weight: [xx, xy, yy], nm².
         pub fn second(&self, c: (f64, f64)) -> [f64; 3];         // M = S/w − 2c·m + c² terms
         pub fn phi(&self) -> (f64, f64);                          // (Σφx/n, Σφy/n), Hastings eq 13.61
     }
     /// Exact Φ equality (sign and magnitude, both axes): Σφa·n_b == Σφb·n_a.
     pub fn phi_equal(a: &Sums, b: &Sums) -> bool;
     /// Every member of a macro's units has the same Φ (the old `currents_run_alike`).
     pub fn phi_equal_all(units: &[pnr_core::Unit], members: usize) -> bool;
     /// A mirror (Mx180 about a vertical axis) keeps Φ_H only if Φ_H = 0 for every member.
     pub fn mirror_allowed(members: &[Sums]) -> bool;
     /// Highest polynomial order n ≤ nmax whose every mixed moment M(p,q), p+q ≤ n,
     /// per unit weight, is equal across `devs` (|Δ| ≤ tol·L^(p+q)); plus the
     /// normalised residue of each order 1..=nmax. L = max |r − c| over all units.
     pub fn cancelled_order(devs: &[&[Pt]], nmax: u8, tol: f64) -> (u8, [f64; 5]);
     ```
  3. Algorithm of `cancelled_order`:
     1. c = weighted centroid of all points of all devices; L = max distance of a point from c (return (nmax, 0) if L = 0).
     2. For each device d and each (p,q), p+q ≤ nmax: M_d(p,q) = Σ w·((x−cx)/L)^p((y−cy)/L)^q / Σ w.
     3. r_n = max over (p,q) with p+q = n and over device pairs of |M_a − M_b|; order = number of leading r_n ≤ tol.
     4. Formula source: P_A − P_B = Σ_{i,j} g_{j,i−j}(M_A(j,i−j) − M_B(j,i−j)) for the polynomial field of NTH eqs 3–6 (nth_order.txt L32–93, PDF p.1–2); the equality of all mixed moments up to n is necessary and sufficient for cancelling every order-≤n field (CC-02, derived there). Per-weight moments make ratioed sets exact (REV eqs 1–3, cc_review.txt L41–71).
  4. `frontend/library/src/cellgen.rs:261-272`: body of `currents_run_alike` becomes `analog::matching::moments::phi_equal_all(&m.units, n_members)` (delete the local sums).
- **Tests** (`kernel/analog/src/matching/moments.rs`, `#[cfg(test)]`, pitch p = 1000 nm, w = 1):
  - `abba_cancels_first_order_only`: A{0,3}, B{1,2} on a row → order == 1, r_2 > 0.
  - `abba_baab_cancels_second_order`: 2×4 grid rows ABBA / BAAB → order == 2 and r_3 > 0 (the x²y term, NOTES-20: Δ = 1 pitch³).
  - `nth_fig3_4x4_cancels_third_order`: rows ABBA/BAAB/BAAB/ABBA (nth_order.txt Fig. 3(b), L154–180) → order ≥ 3.
  - `radial_equal_is_not_second_order`: A at (±2p,0), B at (0,±2p): ⟨r²⟩ equal, `second` gives M_xx 4p² vs 0 → order == 1 (the case `cap_array.rs:366-374` would call balanced).
  - `hastings_phi_example`: 3 right + 1 left vs 9 right + 3 left → `phi_equal` true; ½ vs −½ → false (Hastings eq 13.61 text, hastings.txt L42155–42186, PDF p.709).
  - `ratio_weights_make_one_to_two_exact`: A one unit at 0, B two units at ±p → order ≥ 1.
- **Acceptance**: tests green; `frontend/library` cellgen tests unchanged (behaviour-preserving move).
- **Risks**: f64 cancellation in raw sums: coordinates ≤ 1e7 nm, squares ≤ 1e14 → absolute error ≈ 1e-2 nm², far below any reported residue.

### MAT-02 Point-symmetric grid assignment; fix the BJT array defect

Status: done in M1 (`a505bb9`; matching merge `59371b5`). `cells::bjt` draws `pattern::centro_assign(Balanced)`; `unit_order` is gone.

- **Priority** P0. **Effort** M. **Depends on** MAT-01.
- **Why**: B5. H09-49, H09-06/07/14, AC-08, CC-18/22, H08-34/35, NTH corollary (odd/even cancellation).
- **Current**: `bjt.rs:208-231` deals whole members to slots in radius/angle order, so a member's slots are not closed under 180° rotation; `cap_array.rs:140-192` already assigns reflected pairs (`Grid::put_pair`, `cap_array.rs:217-221`) but lives privately in cap_array.
- **Change**:
  1. Move `Grid` (`cap_array.rs:200-307`) and `general_assign` (`cap_array.rs:140-192`) into `kernel/analog/src/matching/pattern.rs`; cap_array imports them (no behaviour change for `Spiral`/`Chessboard`).
  2. New API in `pattern.rs`:
     ```rust
     #[derive(Clone, Copy, PartialEq, Eq, Debug)]
     pub enum Fill {
         /// DACP spiral: members contiguous, ring by ring (cap_array Spiral).
         Compact,
         /// Farthest-point reflected pairs (cap_array Chessboard).
         Dispersed,
         /// Reflected pairs dealt to balance every member's second moment (LPT, below).
         Balanced,
     }
     /// Grids that can hold `counts` point-symmetrically, near-square first, then the
     /// 1×T row; `max_aspect` filters (≥1.0).
     pub fn grids(counts: &[u16], max_aspect: f64) -> Vec<(usize, usize)>;
     /// Owner per cell (row-major, `None` = empty/dummy). `exact` = every member's cells are
     /// closed under 180° rotation about the grid centre (first moments coincide exactly).
     pub fn centro_assign(counts: &[u16], rows: usize, cols: usize, fill: Fill) -> (Vec<Option<u8>>, bool);
     /// Whether any grid admits an exact assignment: at most one member has an odd count.
     pub fn cc_feasible(counts: &[u16]) -> bool;
     ```
  3. `grids` rule: T = Σcounts, odd = #members with odd count. If odd == 1: rows and cols both odd with rows·cols ≥ T (the odd member takes the centre; empties then come in reflected pairs because rows·cols − T is even). If odd == 0: any rows·cols ≥ T (an odd grid leaves the centre empty). If odd ≥ 2: same as odd == 0 and `exact = false` (the two odd members are paired across the centre, as `cap_array.rs:162-171` does). Candidates: for r in 1..=√T+2, c = smallest (odd if needed) with r·c ≥ T; keep aspect ≤ `max_aspect`; always add (1, T).
  4. `Fill::Balanced` (new):
     1. Cells keyed by doubled integer offsets (dr, dc) from the centre (as `bjt.rs:212-215`), r² = dr² + dc². A reflected pair is {(dr, dc), (−dr, −dc)}; its representative is the cell with the smaller `atan2(dr, dc)` in (−π, π]. Pairs are ordered by r² ascending, then representative angle ascending. The *used* cells are the centre (if an odd member exists) plus the first (T − [odd member exists])/2 pairs in that order.
     2. Target per member d: t_d = count_d · (Σ over used cells of r²) / T (exact rational arithmetic on the integer r² sums: use `i64` numerators with common denominator T, not `f64`).
     3. Deal the used pairs by r² **descending**, ties by representative angle **ascending** (outermost first, LPT; for [2,2] the −135° pair {0, 3} is dealt first, so A gets cells 0 and 3): give the pair to the member with ≥ 2 cells left to place that has the largest deficit t_d − S_d (S_d = Σ r² already assigned, a pair adds 2r²); ties → lower member index. (The earlier draft's secondary x²−y² tie-break was dropped: it was underspecified, and every worked result below and the Table 8.5 values of MAT-12 were re-derived with the lower-index rule alone.)
     4. The odd member (if any) takes the centre before step 3 (S_d unchanged, r = 0).
     5. `grids` candidate order (deterministic): ascending aspect max(r,c)/min(r,c), then fewest empties r·c − T, then fewer rows; (1, T) appended last if not already present. [4,4,4] → (3,4) first, the grid `cap_array.rs:142-143` gives today (rows = 3, cols = 4).
     This is a heuristic for balancing the quadratic residue, whose magnitude is ∝ Σ(x−a)² about the centroid (Hastings eq. 8.26 discussion, hastings.txt L23462–23499, PDF p.393); the exact outcome is certified by `moments::cancelled_order`.
  5. `kernel/cells/src/bjt.rs`: `pub struct Bjt { pub rows: u16, pub columns: u16 }` (replaces `columns` only, `bjt.rs:21-24`); `enumerate` (`bjt.rs:27-48`) yields one variant per `pattern::grids(&dev_nf, 3.0)` for matched sets (single devices keep 1×n, n×1, square); `draw` (`bjt.rs:55`) calls `pattern::centro_assign(&s.dev_nf, rows, cols, Fill::Balanced).0` and places slot i at column `i % columns`, row `i / columns` (drop the `min(total)` clamp at `bjt.rs:56-57`, which assumed no empty cells); delete `unit_order` (`bjt.rs:205-231`). Empty cells stay empty (dummy BJT units are CELL's decision; AC-08). The BJT generator records **no** `Unit` today (no `b.unit(` in `bjt.rs`; AC-08), so the per-owner centroid test below and the `MatchedSet` coincidence check have nothing to read: add in `Unit::draw` (`bjt.rs:145-202`) one `pnr_core::Unit { owner: di as u8, x: e.x + e.w / 2, y: e.y + e.h / 2, weight: e.w·e.h (i64), phi: (0, 0), sa: 0, sb: 0 }` per emitter (fully qualified: `bjt.rs:73` has a private `struct Unit`).
  6. `cap_array.rs:413`: `general_assign(&s.dev_nf, self.pattern)` becomes `pattern::centro_assign` with `Fill::Compact` for `Pattern::Spiral` and `Fill::Dispersed` for `Pattern::Chessboard` on `pattern::grids(..)[0]`. This is **not** always the grid of `cap_array.rs:142-143` (cols = ⌈√T⌉, rows = ⌈T/cols⌉): when exactly one member is odd and that grid is not odd×odd, `grids` moves to an odd×odd grid (e.g. [1,2,2]: 2×3 today, 3×3 from `grids`), so general sets with one odd member change layout (they become exact; area can grow). The four `GENERAL` sets of `cap_array.rs:727` keep their grids except where `grids` breaks the (3,4)/(4,3) tie of [4,4,4] differently; the centroid test (`cap_array.rs:741-756`) checks even counts only and passes either way.
- **Formulas / expected outcomes** (worked by hand from the algorithm, doubled offsets):
  - [2,2] on 2×2: cell order 0,1,3,2 (angles −135°, −45°, 45°, 135°); A gets (0,3), B (1,2) → diagonal cross-coupled quad (Hastings Fig 10.23, hastings.txt L30589–30620, PDF p.512–513).
  - [1,4] → grids {(3,3), (1,5)}; on 3×3 A centre, B the four edge centres (a "+" cross, doubled-offset r² = 4); exact. This is **not** Hastings' 2:1:2 array: Fig. 10.24A (text hastings.txt L30641–30651, PDF p.513; figure on PDF p.514) puts the 1X in a centre column and the four unit sections of the 4X transistor two to each side column, i.e. on the diagonals of the 1X, the lower two mirrored so every emitter faces the horizontal axis (the two units of a side column abut at that axis, so their centres are one pitch out in x and half a row out in y: doubled offsets ≈ (±2, ±1), r² ≈ 5, against 4 for the "+" cross and 8 for the 3×3 corners; `ref-hastings-99` §2.3). That half-row offset is not a 3×3 cell assignment, so no `Fill` on a 3×3 grid draws it; the "+" cross is the nearest grid form in second moment. Both are exact CC. Drawing the book form needs a side column offset by half a row, which `cancelled_order` can score but `grids`/`centro_assign` cannot represent; it is not part of MAT-02.
  - [1,8] on 3×3 → eight-around-one (unchanged vs `bjt.rs:273-281`).
  - [1,6] on 3×3 → B on the 4 edge centres + one corner pair; exact.
- **Tests**:
  - `kernel/analog/src/matching/pattern.rs`: `every_single_odd_ratio_is_exact` — for a in 1..=4, b in 1..=16 with ≤1 odd, every grid from `grids`: `exact == true`, per-member first moment == grid centre (integer check on doubled offsets), empties closed under reflection; `equal_pair_is_a_diagonal_quad` ([2,2] → owners [0,1,1,0]); `four_to_one_is_a_cross`; `two_odd_members_are_reported_inexact` ([1,1,2] on even grid → exact false).
  - `kernel/cells/src/bjt.rs`: replace `a_one_to_eight_pair_centres_the_single_unit` with `every_bjt_ratio_is_common_centroid` (same property through `Bjt::draw` units: per-owner unit centroid equal within 1 nm); keep `every_variant_is_drc_and_erc_clean` and add [2,2] and [1,4] groups to it (sky130; skips without the PDK).
  - `kernel/cells/src/cap_array.rs:741-756` (general-set centroid test) must pass unchanged.
- **Acceptance**: tests green; on `bjt_mirror` and `bgr_core` fixtures the `MatchedSet` coincidence usage of every BJT set is 0 once EXT emits BJT sets (geometry only: sky130 LVS does not see BJTs, AV-01).
- **Risks**: 1:4 grows from 6 to 9 slots (+50 % array area) on the 3×3 option; the 1×5 row stays enumerated, so dp can pick it.

### MAT-03 Diffusion-legal common-centroid rows for ratioed MOS

Status: done in M1 (`c0dcfa9`; matching merge `59371b5`).

- **Priority** P0. **Effort** M. **Depends on** MAT-01.
- **Why**: B6. AC-05, CC-37/38, H13-22 (ratioed devices from identical units), Drennan Table III (put multiplicity in the output, drennan_mismatch.txt L286–308).
- **Current**: region parity: multi-device rows start with D, so region i is S iff i is odd (`mosfet.rs:324-328`); a boundary between two devices is legal only on S. `greedy_centroid` (`builder.rs:271-292`) ignores this; `centroid_sequence` (`mosfet.rs:730-750`) only handles equal counts.
- **Change**:
  1. `kernel/analog/src/matching/pattern.rs`:
     ```rust
     /// Which region the row's two ends are: `Drain` = multi-device rows (`mosfet.rs:324-328`,
     /// region i is S iff i is odd); `Source` = mirror-pins rows (region 0 = S, `mosfet.rs:327`).
     #[derive(Clone, Copy, PartialEq, Eq, Debug)]
     pub enum Outer { Drain, Source }
     /// Device per finger of one shared-diffusion row whose inter-device boundaries are
     /// all source regions and whose label sequence is a palindrome (exact 1-D coincidence,
     /// Σφ = 0 per device). `None` when the counts admit none (any odd count always does).
     pub fn diffusion_cc_row(counts: &[u16], outer: Outer) -> Option<Vec<usize>>;
     /// `legal_row` of CELL-10: every i with s[i] ≠ s[i+1] sits on a source region for `outer`.
     pub fn diffusion_legal(s: &[usize], outer: Outer) -> bool;
     ```
     This is the single order function for ratioed and equal MOS rows. CELL-10 (plan-03) keeps the mosfet.rs wiring and the `mirror_ratio` fixture and calls these two functions instead of adding `pair_order`/`legal_row` of its own; its ALIGN ratio-greedy fill is replaced by the LPT fill below (same outputs on every CELL-10 example: [2,4] Drain → A BBBB A, [2,4] Source → BB AA BB, [4,8] Drain → A BB BB AA BB BB A, [2,2,4,8,8] Drain → A DD EE CC EE DD BB DD EE CC EE DD A; re-derived by script in this review).
  2. Algorithm, `Outer::Drain` (n = Σ counts):
     1. If n is odd → `None` (a palindrome of odd length cannot pair every boundary on S with a single end finger).
     2. The row is `e | t_1 … t_P | e` with P = (n−2)/2 two-finger tokens; palindrome ⇒ t_i = t_{P+1−i}, and when P is odd the middle token is its own mirror.
     3. For each end candidate e with count_e ≥ 2 (ascending index): c' = counts with c'_e −= 2; require every c'_d even; pairs_d = c'_d/2; require #(pairs_d odd) == (P mod 2); the odd one owns the middle token; half-token budget h_d = ⌊pairs_d/2⌋.
     4. Assign the ⌊P/2⌋ mirrored token positions to members by the `Fill::Balanced` LPT of MAT-02 (positions = token centres' x², targets from counts). S_d starts with the end fingers' Σx² (member e) and the middle token's (the odd member) before dealing; the worked results below were re-derived by a script under this convention. Without the pre-count the LPT gives e = B for [6,6] and [8,8] (BAABB…BAAB, same r_2), so the convention is part of the spec.
     5. Among feasible e, keep the row with the smallest second-moment residue (`moments::cancelled_order` r_2); ties → lower e.
     6. Deficits and targets use finger positions x_i = i − (n−1)/2 (pitch units) and exact rationals (i64 numerators over denominator 4n), so ties are exact; t_d = count_d · Σ_i x_i² / n; a token pair adds the four fingers' x².
  3. Algorithm, `Outer::Source`: every count even, else `None`; P = n/2 tokens, no end fingers; pairs_d = count_d/2; require #(pairs_d odd) == (P mod 2); the odd member owns the middle token (fingers P−1, P when P is odd); the ⌊P/2⌋ mirrored token pairs are dealt as in step 2.4 (pre-count the middle token). [2,2] Source → `None` (two odd pair counts); [4,4] Source → AA BB BB AA.
  4. Wiring into `kernel/cells/src/mosfet.rs` is **CELL-10's** (plan-03): `enumerate` (`mosfet.rs:88-120`) offers `Cc1d` iff `diffusion_cc_row(&dev_nf, Drain)` is `Some` (fixes the `dev_nf[0]` keying at `mosfet.rs:78,90`), mirror-pins iff `diffusion_cc_row(&dev_nf, Source)` is `Some`, two rows iff the half counts pass; `finger_sequence` (`mosfet.rs:784-796`) calls the same function. MAT-03 lands the pure function and its tests first; CELL-10 then deletes `centroid_sequence` (`mosfet.rs:722-750`) and the `greedy_centroid` branch. `greedy_centroid` (`builder.rs:269-292`) survives only for resistors until CELL-14 switches to `pattern::segment_row` (MAT-12).
- **Worked results** (by hand from the algorithm): [2,4] → A BBBB A (the order audit-03 names); [2,2] → A BB A; [4,4] → A BB AA BB A (= old ABBA cycle); [6,6] → A BB AA BB AA BB A (LPT gives t1 = B, t2 = A, middle B: identical to `centroid_sequence(2,6)`); [4,8] → A BB BB AA BB BB A; [4,4,4] → A BB CC AA CC BB A (identical to `centroid_sequence(3,4)`); [1,2] → `None`.
- **Tests**:
  - `pattern.rs::diffusion_rows_never_join_two_devices_on_a_drain`: for both `Outer`s, a,b in 1..=16 and (a,b,c) in 1..=8³: if `Some(s)`: counts match, `s` is a palindrome, `diffusion_legal(&s, outer)`; and `None` whenever any count is odd.
  - `pattern.rs::equal_counts_reproduce_the_old_orders` (Drain): for (n_dev, nf) in {(2,2),(2,4),(2,6),(2,8),(3,4),(4,4)} the result equals the literal old `centroid_sequence` output (copied into the test as expected vectors). (3,8) is excluded: the algorithm above gives A BB CC CC AA BB AA BB AA CC CC BB A, not the old A BB CC AA CC BB AA BB CC AA CC BB A, with a lower r_2 (0.083 vs 0.091, normalised by L; *derived* by script); assert r_2(new) ≤ r_2(old) for (3,8) instead.
  - `pattern.rs::two_to_four_is_a_bbbb_a` (Drain → [0,1,1,1,1,0]); `pattern.rs::two_to_four_source_is_bb_aa_bb` (Source → [1,1,0,0,1,1]); `pattern.rs::align_example_is_exact` ([2,2,4,8,8] Drain is `Some`, every member's Σ(2i − (n−1)) == 0).
  - The merged-cell test on sky130 is CELL-10's `a_ratioed_mirror_merges` (plan-03); MAT does not duplicate it.
- **Acceptance**: pattern tests green. Measured through CELL-10: on `mirror_ratio` (CELL-10 fixture) and the bench fixtures, every ratioed MOS unitization whose count vector is `Some` yields ≥ 1 merged variant (today 0); existing mosfet order tests (`mosfet.rs:855-880`) pass after CELL-10 points them at `diffusion_cc_row`.
- **Risks**: a single CC row carries inherent inner/outer LOD difference (REV §V-A, cc_review.txt L428–437); MAT-04's LOD term prices it and the "apart" topology remains available.

### MAT-04 `MatchedSet`: one ledger and one allowance per matched pair

Status: done in M1 (`13b0a27`, `c41f71b`; matching merge `59371b5`). `MatchingPair`, `ThermalGradient` and `CentroidGroup` are deleted; each pair's thermal term is `MatchedSet`'s `mu_thermal` from `Layout::rise_at_point_mc` at unit centroids. **Acceptance not met:** the ±5 % HPWL/footprint band (T5, T8) failed at the item commit (OTA trio WL −13.3 %) and at the M1 close (OTA trio WL +21.8 %, area +15.7 % vs M0, mostly from FLOW-16; m1-report §4); carried to M2 item 0.

- **Priority** P0. **Effort** L. **Depends on** MAT-01. (PLC's AR-42 fix improves gp but is not required.)
- **Why**: B1–B4. AR-01, AR-02, AR-04, AR-33, AR-37, AR-40, AR-41, AR-43; MM-01/02/15/19/30/31, LAMP-09, NOTES-16/18/19/53, H08-32/35/36, H13-37/40, GRAEB-02.
- **Current**: §1.1 rows 1–6.
- **Change**:
  1. `kernel/analog/src/matching/mismatch.rs` (pure math):
     ```rust
     /// Pelgrom pair σ of two devices of unequal gate area, mV:
     /// A·√((1/a1 + 1/a2)/2), A = pair constant mV·µm, a µm².
     pub fn sigma_pair(a_pair: f32, a1_um2: f32, a2_um2: f32) -> f32;
     /// Voltage matching (a diff pair: compare V_GS at equal I) or current matching
     /// (a mirror/load: compare I at equal V_GS), Hastings §13.2 eqs 13.40–13.41;
     /// `Ratio` for passives (MAT-10). Same variants as EXT-12's `intent::MatchKind`:
     /// whichever item lands first defines it, the other re-exports (§0.2).
     #[derive(Clone, Copy, PartialEq, Eq, Debug)]
     pub enum MatchKind { Voltage, Current, Ratio }
     #[derive(Clone, Copy, Debug)]
     pub enum Budget {
         /// η·σ_rand (today's default, η = 0.3).
         Eta(f32),
         /// Total 1σ budget b, mV: the layout gets √max(0, b² − σ_rand²).
         Sigma1Mv(f32),
         /// The 1σ systematic allowance itself, in the ledger's unit (EXT-20's `allowance`).
         Allowance(f32),
     }                                                        // MAT-09 adds Sigma1Pct
     impl Budget { pub fn allowance(self, sigma_rand: f32) -> f32 }
     /// Process numbers of one set; `None` = not given → that term is unknown.
     #[derive(Clone, Copy, Debug, Default)]
     pub struct Coeffs { pub avt_mv_um: Option<f32>, pub svt_uv_per_um: Option<f32>,
                         pub kvth0_mv_um: Option<f32>, pub tc_uv_per_k: Option<f32> }
     /// One pair's ledger, mV (MAT-09 adds a % domain).
     #[derive(Clone, Copy, Debug, Default, PartialEq)]
     pub struct Ledger { pub sigma_rand: f32, pub sigma_grad: f32, pub mu_thermal: f32, pub mu_lod: f32,
                         pub allowance: f32, pub coincidence: Option<f32>, pub second_order_nm: f32,
                         pub delta_m_nm: f32, pub order: u8, pub known: bool }
     impl Ledger {
         pub fn spent(&self) -> f32;   // mu_thermal + mu_lod + sigma_grad
         /// max(spent / allowance.max(f32::EPSILON), coincidence.unwrap_or(0)) — the
         /// `usage` convention of `matching_pair.rs:56-58` (no NaN at allowance 0).
         pub fn usage(&self) -> f32;
         /// max(rule::over(spent − allowance, allowance), coincidence − 1, 0); `over`
         /// (`kernel/analog/src/rule.rs:313-318`) returns 0/1 for a zero allowance, so a set whose
         /// sizing already spends the budget reads 1 per pair as soon as the layout spends anything.
         pub fn residual(&self) -> f32;
     }
     ```
     `rule::over` is `pub(crate)`; `matching` is inside crate `analog`, so no visibility change is needed.
     Formulas and units:
     - σ_rand = A_VT·√((1/a₁ + 1/a₂)/2), a_d = Σ unit weight / 1e6 (µm²) when the layout has units (drawn truth, MM-19; removes AR-46's netlist-convention dependence), else the netlist gate areas stored in the set. Source: pair convention and σ_single = σ_pair/√2 (LDM eq. 4, long_distance_mismatch.txt L144–148, PDF p.3); same form as Hastings eq. 13.49 (hastings.txt L40840–40864, PDF p.688–689).
     - σ_grad = S_VT·10⁻³·|Δm|/10³ (µV/µm → mV/µm; nm → µm). Pelgrom eq. (1) distance term (pelgrom.txt L37–43, PDF p.1); independence of pair and distance parts (distance_vs_pair_mismatch.txt L85–101, PDF p.2).
     - μ_thermal = TC·|ΔT|·10⁻⁶ (µV/K × mK → mV), ΔT from the **live** field at the two device centroids (Hastings eq. 8.23 ΔR = αR·d·∇T, hastings.txt L23120–23142, PDF p.388; existing `emit.rs:96-101` form).
     - μ_lod = KVTH0·|⟨lod⟩_a − ⟨lod⟩_b|, ⟨lod⟩ = unit-**weighted** mean of `PlacedUnit.lod` (1/µm, `units.rs:88`); REV eq. 11 (cc_review.txt L103–117); fixes AR-32's unweighted mean.
     - spent = μ_thermal + μ_lod + σ_grad. Deterministic terms add by magnitude (worst case, as Lampaert eq. 4.25 sums |S|·3σ, lampaert.txt L4718–4762); σ_grad is the only statistical layout term until MAT-16 adds σ_quad (then √(σ_grad²+σ_quad²)). One budget: MM-31 (DVP eq. 1), LAMP-09.
     - allowance: `Eta(η)` → η·σ_rand; `Sigma1Mv(b)` → √max(0, b² − σ_rand²) (same quadrature split as `emit.rs:83-88`); `Allowance(a)` → a.
     - sky130 inputs (deck `pdks/sky130.json`, read by `frontend/library/src/lib.rs:505-508` into `ProcessNumbers`, `backend/annotator/src/netrole.rs:92-101`): A_VT n/p = 9.5 / 11.5 mV·µm (`sky130.json:93,95`, MC-measured); S_VT = 1.63 µV/µm both polarities (`:97`, PROXY); KVTH0 n/p = 9.8 / 32.9 mV·µm (`:90-91`); |dV_T/dT| n/p = 765 / 1850 µV/K (`:127,129`). Worked value: a sky130 NMOS pair of 2 × 20 µm² → σ_rand = 9.5·√(1/20) = 2.124 mV, `Eta(0.3)` allowance 0.637 mV (the number used in the tests below).
     - coincidence (process-free, Hastings Table 8.4 rule 1, hastings.txt L23425–23447, PDF p.392): applies only when both members' units sit in one cell **and** `pattern::cc_feasible` (MOS: `diffusion_cc_row(counts).is_some()`) holds; value = |Δm|/tol_nm, tol_nm = lattice/2 (replaces `COINCIDENCE_TOL = 0.01`, `cc.rs:86`; AR-37).
     - second_order_nm = ‖M_a − M_b‖_F / L, M = `Sums::second` about the pair centroid, L = max unit distance from it (nm): the centroid offset that gives the same error as the quadratic residue under a field whose curvature at the array edge equals its slope. Cost term and report only (no coefficient in any deck; MM-26).
     - order = `moments::cancelled_order` over the two devices' units, nmax = 4, tol = 1e-6 (report).
  2. `kernel/core/src/thermal.rs`: extract the loop of `rise_at_mc` (`thermal.rs:30-49`) into
     ```rust
     impl Layout { /// Rise at (x,y) from every powered source, mK; r floored at each source's half-extent. O(n).
         pub fn rise_at_point_mc(&self, x: i32, y: i32) -> f32 }
     ```
     `rise_at_mc(l, p, i)` becomes a call with `(l.x[i], l.y[i])` (identical: the i == j term already floors r at r_floor). Delete `delta_temp_mc` and `live_delta_temp_mc` with the `Target::Group` hottest-member paths (`thermal.rs:57-90`; AR-43) once `ThermalGradient` is gone (no other caller, grep).
  3. New `kernel/analog/src/placement/matched_set.rs`:
     ```rust
     #[derive(Clone)]
     pub struct MatchedSet {
         /// Slot 0 = reference (a mirror's diode device); pairs are (0, i).
         pub members: Vec<DeviceId>,
         pub kind: MatchKind,                 // Voltage | Current (MAT-07 moves the enum to class.rs)
         pub coeffs: Coeffs,
         pub budget: Budget,
         /// Netlist gate areas, µm², read only while the layout has no units (gp).
         pub gate_um2: Vec<f32>,
         /// Coincidence tolerance, nm (half the cut lattice).
         pub tol_nm: f32,
         /// device → cell, set by `retarget`: bbox fallback and repair ids.
         pub cell_of: Vec<u16>,
     }
     impl MatchedSet { pub fn ledger(&self, l: &Layout, i: usize) -> Ledger }
     impl RuleBatch<Layout> for MatchedSet { … }
     ```
     `ledger` steps: (1) `Sums` of each member from `l.units.of_device(l, d)`; (2) no units for a member (gp today, AR-42) → its centroid is its cell's centre via `cell_of` and is used **only by `cost`** (the pull), the pair is `known = false` and charges no Θ (Rule::known contract, `kernel/analog/src/rule.rs:54-61`; fixes AR-33); (3) Δm, second moments, order; (4) σ_rand, σ_grad, μ_lod, μ_thermal (thermal term 0 and not counted when `l.power_uw` is all zero — "not applicable", as `thermal.rs:46-48`); (5) allowance; (6) `known` = units for both members and (A_VT given or coincidence applicable).
     `RuleBatch` members: `cost` = Σ_i 1e-3·(Δm² + second_order_nm²) + 3e5·(μ_thermal/allowance)² (the two legacy scales of `matching_pair.rs:42-48` and `thermal.rs:25,30-33`, kept until PLC normalises costs, AR-08); `violations` = #pairs known with usage > 1; `residual` = Σ known pairs' `Ledger::residual`; `unknown` = #pairs not known; `worst_usage` = max usage over known pairs; `kind` = `"MatchedSet"`; `count` = members.len() − 1; `retarget` stores `cell_of`; `violating_ids`/`touched` push `cell_of[member]` (AR-17); `matched_pairs` (PLC-03's hook, plan-04) pushes `(cell_of[members[0]], cell_of[members[i]])` for every i whose two cells differ (nothing before `retarget`), so dp turns separately drawn members together.
     Name clash: EXT-12 adds `analog::intent::MatchedSet` (the extracted set) to the same crate. This rule is `analog::placement::MatchedSet`; `emit.rs` imports it as `use analog::placement::MatchedSet as MatchedRule` once EXT-12 lands.
  4. `backend/annotator/src/emit.rs` (placement, `emit.rs:138-239`): per 2-device DiffPair/CurrentMirror/Load leaf push one `MatchedSet { members: [a, b], kind: DiffPair → Voltage, else Current, coeffs: by polarity (A_VT, S_VT, KVTH0, TC), budget: offset_sigma_mv.map_or(Eta(GRADIENT_SHARE), Sigma1Mv), gate_um2: [gate(a), gate(b)], tol_nm: p.lattice_nm as f32 / 2.0, cell_of: vec![] }` into **budget and cost**. Delete the `MatchingPair` push (`emit.rs:159-171`), the `ThermalGradient` push (`emit.rs:176-183`), `a_side/b_side` and the pooled `CentroidGroup` (`emit.rs:196-197,216-238`), `THERMAL_MAX_DELTA_MC` (`emit.rs:40-42`) and `Pelgrom::thermal_limit_mc` (`emit.rs:93-102`). Symmetry, Proximity and DTI emission unchanged.
  5. `backend/annotator/src/netrole.rs:79-108`: add `pub lattice_nm: i32` to `ProcessNumbers`; `frontend/library/src/lib.rs:499-514` sets it to `cells::builder::cut_lattice(pdk)`.
  6. `backend/annotator/src/lib.rs:53-71` (`missing`): the two "MatchingPair" entries become ("MatchedSet", "deck svt_uv_per_um — distance term unknown") and ("MatchedSet", "deck avt_n_mv_um/avt_p_mv_um").
  7. Delete: `MatchingPair` struct and its `Rule` impl (`matching_pair.rs:17-78`; keep `is_fet`/`is_diff_pair`/`is_supply` and the `G`/`D`/`S` terminal constants (`matching_pair.rs:9-15`), used by routing rules: `routing/crosstalk.rs:7`, `routing/differential.rs:6`), `kernel/analog/src/placement/thermal.rs` (whole file), `kernel/analog/src/placement/cc.rs` (whole file; its unit-level scenarios are ported), the `pub mod cc;`/`pub mod thermal;` lines and the `pub use` lines in `placement/mod.rs:3,10,16,19`; add `pub mod matched_set;` (and MAT-05's `pub mod orientation;`) there. `gradient_over_random` moves into `mismatch.rs` as σ_grad.
- **Tests**:
  - `kernel/analog/src/matching/mismatch.rs`: `unequal_areas_use_the_per_device_variance` (A = 10, a = 4 and 16 µm² → σ = 3.953 mV ± 0.001); `one_allowance_is_spent_once` (`Ledger{mu_thermal: 0.4, mu_lod: 0.4, allowance: 0.637, ..}` → usage 1.256 ± 0.001, residual 0.256: the old per-rule checks each read 0.63 and passed); `sigma1_budget_leaves_the_quadrature_share` (b = 3·√1.09, σ_rand = 3 → allowance 0.9 ± 1e-3, i.e. η = 0.3, ported from `emit.rs:298-322`).
  - `kernel/analog/src/placement/matched_set.rs` (UnitLib built as in `cc.rs:286-396`):
    - `merged_pair_reads_its_units_not_its_cell`: one cell, ABBA → coincidence 0, known; AABB → coincidence usage > 1 and violations == 1 (AR-01).
    - `separate_cells_a_millimetre_apart_spend_the_distance_term`: S = 1.63 µV/µm, cells 1 mm apart → σ_grad = 1.63 mV ± 0.01.
    - `stage_pooling_cannot_cancel`: two sets offset +d and −d → each reports Δm = d (AR-02).
    - `unknown_is_not_charged`: no units, A given, cells 5 µm apart → `cost > 0` (bbox pull), `residual == 0`, `unknown == 1`; both members in one cell and no units → `cost == 0`, `residual == 0`, `unknown == 1` (AR-33).
    - `thermal_reads_unit_centroids_of_a_merged_pair`: 10 mW heater cell at origin with half-extent 1 µm (so r is not floored at 5 µm), merged row with A centroid at 5 µm and B at 15 µm (AABB) → μ_thermal = 765 µV/K × (2150.7 − 716.9) mK ≈ 1.097 mV (> 1.0); ABBA with both centroids at 10 µm → < 0.01 mV (ΔT from P/(2πkr), k = 148 W/m·K, `kernel/core/src/thermal.rs:9-16`).
    - `lod_is_weighted_by_unit_area` (port of `cc.rs:310-335` plus unequal weights).
    - `second_moment_separates_abba_from_abba_baab`: 1-row ABBA → second_order_nm > 0; 2-row ABBA/BAAB → < 1 nm.
    - `retarget_keeps_schematic_members` and `violating_ids_are_cells` (debug check of `frontend/library/src/lib.rs:1238-1264` must hold).
    - `matched_pairs_are_cells`: members {0, 1} in cells {3, 5} → `[(3, 5)]`; both in cell 3 → `[]`.
    - `zero_allowance_is_a_violation_not_nan`: `Sigma1Mv(1.0)` with σ_rand = 2.124 → allowance 0; spent 0.01 → residual 1.0, usage finite; spent 0 → residual 0.
  - `kernel/core/src/thermal.rs`: `point_rise_equals_device_rise_at_its_centre`.
  - Update `backend/annotator/src/tests.rs:139-202,409,475-489` to assert `MatchedSet` in budget and cost (never hard) and the new `missing` names; `kernel/analog/src/placement/symmetry.rs:301,326` tests switch their mixed-rule retarget check to `MatchedSet`. If PLC-03 has landed, its `matched_pairs` overrides on `MatchingPair`/`CentroidGroup` are deleted with those types and its test `mixed_polarity_composite_halves_turn_together` switches its `MatchingPair (0,1)` in `reqs.cost` to a `MatchedSet` over the same two devices.
- **Acceptance**: `cargo test -p analog -p annotator -p pnr_core -p library` green; bench local on the 10 circuits: DRC 0 / LVS MATCH unchanged, `MatchedSet` row with 0 unknown on circuits whose cells carry units, HPWL/footprint within ±5 % of the pre-change run (T8).
- **Risks / notes**: (1) Θ now reads the live thermal field during dp (before: frozen per epoch, `kernel/analog/src/placement/thermal.rs:12-13`, `kernel/core/src/thermal.rs:4`); with dp's greedy Θ gate (AP-13) thermal pressure acts inside the anneal. Evaluation cost: 2 point samples × n sources per pair per call, the same order as today's live cost term. (2) Removing the pooled group halves the centroid pull for separately drawn pairs; T8 guards area. (3) Price keys change (`kind()` strings); prices are per run.

### MAT-05 `OrientationSet`: channel axes parallel, Φ equal

Status: done in M1 (`723bbf7`; matching merge `59371b5`).

- **Priority** P0. **Effort** S. **Depends on** MAT-01.
- **Why**: B7. AR-36, AP-04 (behaviourally), H13-25, H08-16, MM-10, NOTES-22, LAMP-11/12, BAL1-43.
- **Current**: §1.1 "Orientation" row; `UnitLib::placed` already turns `phi` with the cell (`units.rs:120,132-135`).
- **Change**: new `kernel/analog/src/placement/orientation.rs`:
  ```rust
  #[derive(Clone, Copy, PartialEq, Eq, Debug)]
  pub enum OrientCheck { Axis, Phi, PhiZero }
  #[derive(Clone)]
  pub struct OrientationSet { pub members: Vec<DeviceId>, pub check: OrientCheck, pub cell_of: Vec<u16> }
  impl RuleBatch<Layout> for OrientationSet { … }
  ```
  - `Axis`: violated if members' `Sums::axis` differ among {H, V} (a 90° relative turn), or any member is `Mixed`. `Sums::axis` ignores units with φ = (0, 0); it is `None` only when every unit has φ = (0, 0), `H` when every other unit has φ_y = 0, `V` when every other unit has φ_x = 0, else `Mixed`. Hastings rule 7: channels not parallel are vulnerable, "designers should always make sure that the orientation of matched transistors are equal" (hastings.txt L42490–42498, PDF p.714). Units with `Axis::None` (capacitors) never violate.
  - `Phi`: violated unless `phi_equal` for all members (eq. 13.61; "equal for exceptional matching … at least nearly equal for self-aligned devices used for moderate matching", same lines). Residual = |ΔΦ_H| + |ΔΦ_V| (dimensionless, ≤ 4).
  - `PhiZero` (resistor strings, MAT-12): violated unless Σφ = 0 per member (thermoelectric cancellation, Hastings eq. 8.34 and R11, hastings.txt L23960–24046, L25282–25290, PDF p.400–402, 420).
  - `unknown` = 1 when a member has no units; `kind` = "Orientation"; `count` = 1; ids = member cells; `matched_pairs` (PLC-03 hook) for `Axis` pushes (cell of member 0, cell of member i) for every i in a different cell, so PLC-03's rotation lock also covers sets that have only an orientation rule.
  - Emission (`emit.rs`, next to each `MatchedSet`): `OrientationSet{Axis}` → **hard**; `OrientationSet{Phi}` → budget + cost (Moderate default) — MAT-07 moves it to hard for Exceptional and drops it for Minimal.
  - Provide `moments::mirror_allowed` to PLC for the mirror-vs-perfect decision (AP question 2).
- **Tests** (`orientation.rs`): `a_quarter_turned_partner_is_illegal` (two one-unit cells φ = (1,0), one at `Orient::R90` → violations 1); `a_mirrored_odd_finger_partner_breaks_phi` (3 fingers +,−,+ → Φ_H = ⅓; partner `Mx180` → −⅓ → Phi violated; 2 fingers +,− → Φ = 0 → not violated and `mirror_allowed`); `capacitor_units_never_violate`; `axis_ignores_zero_phi_units` (units (1,0), (0,0) → `H`). The dp-side scenario (separately drawn halves of a truncated mixed-polarity group never turn apart) is PLC-03's `mixed_polarity_composite_halves_turn_together` (plan-04), which already merged MAT's former `matched_halves_never_turn_apart`; MAT adds no dp test.
- **Acceptance**: bench "Orientation" hard row: 0 violations, 0 unknown on the 10 circuits (T6).
- **Risks**: macroMaster's `place_mirrored` (AF-19) makes odd-finger partners fail `Phi`; this is the intended report.

### MAT-06 Remaining allowance handed to `CommonNodes`

Status: done in M1 (`d12b34b`; matching merge `59371b5`).

- **Priority** P0. **Effort** S. **Depends on** MAT-04. Consumer: RTE (CommonNodes rule unchanged).
- **Why**: B3 routing half. AR-04, MM-31, MM-33, LAMP-09.
- **Current**: `common_nodes` (`frontend/library/src/lib.rs:904-938` on m0) computes `max_delta_ohm = systematic_allowance_mv(avt, gate_um2, offset)/I` — the **full** allowance again; since FLOW-01 the gate area is `Device::gate_area_um2()` (W_total·L·m, `lib.rs:932`), no longer max(nf,m). The line numbers in Change step 2 are dad330c's: on m0 the `Flow` fields `avt_mv_um`/`offset_sigma_mv` are declared at `lib.rs:618-620`, initialised at `:446-447`, read at `:931-934`.
- **Change**:
  1. `kernel/analog/src/rule.rs:135-208` (`RuleBatch`), new default hook:
     ```rust
     /// `(device_a, device_b, mV)`: the 1σ systematic allowance each matched pair has
     /// left after placement's own spend — what a routing rule may use. Default none.
     fn offset_allowances(&self, state: &On, out: &mut Vec<(u32, u32, f32)>) { let _ = (state, out); }
     ```
     `MatchedSet` pushes `(members[0], members[i], max(0, allowance − spent))` per known pair, always in mV (a common-source ΔR·I is a gate-referred voltage); once MAT-09 gives a % ledger, a Current-kind pair converts back with ÷ (0.1·G). The library looks a leaf (a, b) up in both orders.
  2. `frontend/library/src/lib.rs:763-801` (`common_nodes`): collect `offset_allowances(layout)` over `self.problem.placement.budget` once; for each leaf (a, b) take the entry → `max_delta_ohm = remaining_mv / i_ua · 1e3`; no entry → `0.0` (unknown, as today). Delete `annotator::emit::systematic_allowance_mv` (`emit.rs:58-65`; only caller `lib.rs:794`) and the `avt_mv_um`/`offset_sigma_mv` fields of `Flow` (declared `lib.rs:484-487`, initialised `lib.rs:332-333`, read only at `lib.rs:790,794`; verified by grep of `frontend/library/src`). The layout passed is the one `common_nodes(&self, layout)` already receives (the epoch's placed layout), so the remaining allowance reflects the final placement.
- **Tests**: `matched_set.rs::remaining_allowance_subtracts_placement_spend` (allowance 0.637, spent 0.4 → 0.237 ± 1e-3); `frontend/library/tests/flow_smoke.rs::common_node_budget_fits_the_set_allowance` on `ota`: for every CommonNode, `max_delta_ohm·I_D·1e-3 + spent ≤ allowance + 1e-6`.
- **Acceptance**: on `ota`, per matched pair, placement spend + CommonNode ΔR·I budget ≤ η·σ_rand (T5).
- **Risks**: a placement that spends the whole allowance leaves CommonNodes a zero budget → routing Θ rises; that is the intended coupling.

### MAT-07 Class, kind and family on the matching rules

- **Priority** P1. **Effort** S. **Depends on** MAT-04, MAT-05; FLOW-06 (`pnr_core::MatchClass`); EXT-16 for inferred classes (until then every set is `Moderate`, open question 4).
- **Why**: H13-42 (classes), H13-25 (Φ "equal for exceptional … at least nearly equal … moderate", hastings.txt L42155–42186), H09-01, AA-10 (data side), AR-41 (citation).
- **Current**: no class anywhere (grep `MatchClass` → none); the rules MAT-04/05 add carry no class.
- **Scope cut in review**: the earlier draft of this item defined `MatchClass`, a second 6σ limit table, `Process::tier`, deck tier keys, `mos_env`/`resistor_env`, `Unitization.class` and the `lib.rs:871-872` switch. All of those are owned elsewhere and specified there in full: `MatchClass` + `Process::tier(key, MatchClass)` + tier keys by FLOW-06; `limits(Family, MatchKind)` and `class_of` by EXT-16 (whose R/C limits take the tight end of Hastings' ranges, [1, 0.1, 0.01] %, where this plan had taken the loose end; EXT-16's table wins); `Unitization.class: Option<MatchClass>` by EXT-12; MOS dummy/moat/WPE/gate extension by CELL-12; resistor dummies by CELL-14; resistor width/length floors (H08-41) by CELL-23; aspect filter (H13-46) by CELL; `frontend/library/src/lib.rs:871-872` by EXT-16 (consumer edit) together with CELL-12. See appendix.
- **Change**:
  1. `kernel/analog/src/matching/class.rs`:
     ```rust
     pub use pnr_core::MatchClass;               // FLOW-06
     pub use crate::matching::mismatch::MatchKind; // or intent::MatchKind, §0.2
     /// Device family of a matched set (EXT-16 imports this; it has no Diode today).
     #[derive(Clone, Copy, PartialEq, Eq, Debug)]
     pub enum Family { Mos, Bipolar, Diode, Resistor, Capacitor }
     /// Φ check strength by class (Hastings eq. 13.61 text, hastings.txt L42155–42186):
     /// `Some(true)` hard, `Some(false)` budget + cost, `None` not emitted.
     pub fn phi_arm(c: MatchClass) -> Option<bool> {
         match c { MatchClass::Exceptional => Some(true), MatchClass::Moderate => Some(false), MatchClass::Minimal => None }
     }
     ```
  2. `placement::MatchedSet` gains `pub class: MatchClass`, `pub family: Family` (report fields; MAT-08/10 read them). `emit.rs` sets `Moderate`/`Mos` for today's 2-device leaves; EXT-20 sets them from `intent::MatchedSet`.
  3. `OrientationSet{Phi}` emission uses `phi_arm(class)`; `OrientationSet{Axis}` stays hard for every class (rule 7 has no class qualifier, hastings.txt L42490–42498).
  4. `kernel/analog/src/placement/environment.rs:15`: citation "§13.3 r8" → "§13.3 rule 19" (AR-41). The mosfet.rs citations (`mosfet.rs:278,291`, "§13.3 r9" → rule 12) are fixed by CELL-12 when it rewires the moat.
  5. Note for FLOW (deck owner): the stale "Not read by the library yet" in `pdks/sky130.json:130` is wrong (`frontend/library/src/lib.rs:507` reads `vt_tc_uv_per_k_p`); FLOW-06 rewrites the sidecar and drops it.
- **Tests** (`class.rs`, `orientation.rs`): `phi_arm_follows_hastings` (Exc → hard, Mod → budget, Min → none); `backend/annotator/src/tests.rs::default_class_is_moderate_mos` (every emitted `MatchedSet` on the `ota()` fixture, `tests.rs:39`, has `class == Moderate`, `family == Mos`; `OrientationSet{Phi}` in budget, not hard).
- **Acceptance**: tests green; bench output unchanged with every class `Moderate`.
- **Risks**: open question 11 (tier encoding conflict between FLOW-06 and CELL-12) does not block this item.

### MAT-08 Class-derived budgets

- **Priority** P1. **Effort** S. **Depends on** MAT-04, MAT-07; EXT-16 (limit values, class source); EXT-20 (emission).
- **Why**: MM-14, MM-21, H13-21, H08-03, GRAEB-38 (allocation starts from a spec, not η).
- **Current**: without `offset_sigma_mv` every pair gets η = 0.3 (`emit.rs:85-88`); when σ_rand alone exceeds a given budget η clamps to 0 silently (`emit.rs:86`) and no rule reports it.
- **Change**:
  1. `mismatch.rs`: `impl Budget { pub fn from_class(limit_6sigma: f32, kind: MatchKind) -> Budget }` — Voltage → `Sigma1Mv(limit/6)`; Current and Ratio → `Sigma1Pct(limit/6)` once MAT-09/10 give a % ledger, `Eta(0.3)` until then. Hastings class limits are six-sigma (hastings.txt L42333–42350, PDF p.712); the limit value comes from EXT-16's `limits(family, kind)[class as usize]`.
  2. Budget choice at emission (EXT-20 applies it; `emit.rs` today): `offset_sigma_mv` if given → `Sigma1Mv`; else EXT's `allowance` if `Some` → `Allowance`; else `from_class` **only when** `class_source ∈ {User, Spec}` (EXT-12 `ClassSource`); else `Eta(0.3)`. The `Role` default never tightens today's behaviour: with sky130 A_VT = 9.5 mV·µm and 20 µm² per device, σ_rand = 2.124 mV > 3 mV/6 = 0.5 mV would fail every Moderate pair.
  3. A set whose σ_rand ≥ its total budget gets allowance 0 and reads violated as soon as the layout spends anything (`Ledger::residual`, MAT-04); `LedgerRow` (MAT-13) marks it `sizing_limited = true`. Sizing advice (area/length, H13-43/44) is EXT-29's diagnostic, not MAT's.
- **Tests** (`mismatch.rs`): `class_budget_is_one_sixth_of_the_limit` (Voltage, 3 mV → `Sigma1Mv(0.5)`); `role_default_keeps_eta` (class_source `Role` → `Eta(0.3)`).
- **Acceptance**: `ota` with `offset_sigma_mv` = 1.0 mV (below its input pair's σ_rand): that pair's `LedgerRow` has allowance 0, `sizing_limited`, and counts as violated in the `MatchedSet` row of the bench table (never silently passing).

### MAT-09 Current-domain ledger and the A_β coefficient

- **Priority** P1. **Effort** M. **Depends on** MAT-04, MAT-07; EXT-17 (op facts inside the annotator); FLOW-06 (the `abeta_*` key slots); FLOW (AF-01: op-point W/nf/m convention, otherwise g_m/I_D describe a different device).
- **Why**: MM-08/09/17/28, H13-19, H15-56, NOTES-18, AA-10 (mirrors budgeted in the V_T domain), LAMP-10.
- **Current**: every set is judged in the ΔV_T domain; `gm_us` is used only for gate-R folding (`cellgen.rs:615`), `id_ua` only for CommonNodes (`lib.rs:793`); no deck has A_β (grep `abeta` → none).
- **Change**:
  1. Key observation (keeps the change small): with only ΔV_T terms the ledger usage is domain-invariant — mapping multiplies spent and allowance by the same g_m/I_D. The domain matters only when A_β is known or the budget is in %.
  2. `Coeffs` gains `abeta_pct_um: Option<f32>`; `Ledger` gains `unit: LedgerUnit` with `enum LedgerUnit { Mv, Pct }` (not `Unit`: `pnr_core::Unit` is the drawn-unit type, re-exported at `kernel/core/src/lib.rs:30` and used by `moments.rs`). For Current kind with G = g_m/I_D (1/V) of the reference: σ_rand,I = √((0.1·G·σ_VT)² + σ_β²) % (1/V·mV = 10⁻³ → ×100 = 0.1), σ_β = A_β·√((1/a₁+1/a₂)/2) %; μ and σ_grad scale by 0.1·G. For Voltage kind: σ_rand,V = √(σ_VT² + (10·σ_β/G)²) mV. ρ(ΔV_T, Δβ) = 0 (close pairs, pelgrom.txt L111–119). Sources: MM-09 (PEL eq. 2, DRE eq. 4, LDM eq. 3), Hastings eqs 13.42–13.43 (hastings.txt L40695–40722, PDF p.686–687).
  3. G per device = `gm_us / id_ua` (µS/µA = 1/V) from EXT-17's `Evidence.op.dev[d]` (`DeviceOp { id_ua, gm_us, … }`); the earlier draft's separate `AnnotationConfig.gm_over_id` is dropped (duplicate plumbing). G of a pair = G of `members[0]`; no op → Current-kind pairs stay in the mV domain and `LedgerRow.unit = "mV"`. `ProcessNumbers` gains `abeta_pct_um: [Option<f32>; 2]` read from FLOW-06's slots `abeta_n_pct_um`, `abeta_p_pct_um` with the `pos(..)` helper of `frontend/library/src/lib.rs:498,505-508`. `Budget` gains `Sigma1Pct(f32)` (allowance √max(0, b² − σ_rand,I²) %).
  4. `benchmarks/characterize_mismatch.py`: new mode `bpv` (MM-17): for each of the existing 6 W/L, run the MC pair at V_ov ≈ 0.1 and 0.4 V (voltage-biased I_D pairs), measure σ(ΔI/I) and g_m/I from the op; least squares on σ²(ΔI/I)_k = (g_m/I)_k²·A_VT²/(WL) + A_β²/(WL) → `abeta_*_pct_um` with a `_source` string (format of `pdks/sky130.json:93-96`). Values for sky130/gf180/ihp: produced by the run (not given in docs/ref/).
- **Tests**: `mismatch.rs::current_domain_matches_hastings_13_43` (σ_Vt 1 mV, V_gst 0.2 V ⇒ G = 10/V, σ_k 0.5 % → 1.118 % ± 0.001); `without_abeta_usage_is_domain_invariant`; `characterize_mismatch.py` self-test solving a synthetic 2×2 BPV system exactly.
- **Acceptance**: with A_β in the deck, bench shows mirror rows in % and diff-pair rows in mV; mirror usage differs from the V-domain value by the β share (non-zero).
- **Risks**: weak inversion makes G ≈ 1/(nV_T); the measured g_m/I_D covers it (MM-09); AF-01 mis-sized simulation corrupts G until FLOW fixes it.

### MAT-10 Mismatch coefficients and ledgers for resistors, capacitors, bipolars, diodes

- **Priority** P1. **Effort** M. **Depends on** MAT-04, MAT-07; EXT-19 (recognition of these sets, AA-04; today only cellgen's hidden `dac_banks`/`bjt_groups` group them, `cellgen.rs:486-527`); FLOW-06 (per-recipe `k_a_pct_um` slot, `capacitors` recipe table).
- **Why**: H08-04/05/06, H09-02/15, MM-35, CC-11, H01-43, AA-04.
- **Current**: `by_polarity` returns `None` for non-FETs (`emit.rs:107-113`); no R/C/BJT coefficient keys in any deck.
- **Change**:
  1. `Coeffs` gains `ka_pct_um: Option<f32>` (areal constant %·µm, Ratio domain and bipolar), `tc_ppm_per_k: Option<f32>` (R/C value TC), `vbe_tc_uv_per_k: Option<f32>`, `sd_pct_per_mm: Option<f32>` (resistor distance term).
  2. Ratio-domain ledger (Family Resistor/Capacitor): σ_rand = k_A·√((1/A₁ + 1/A₂)/2) % with A = Σ unit weights (µm²) — Hastings eq. 8.8 areal part and eqs 8.12/8.15 (hastings.txt L21973–22137, PDF p.371–373); μ_thermal = tc_ppm·|ΔT|[K]·10⁻⁴ % (eq. 8.23); σ_grad = sd_pct_per_mm·|Δm|/10⁶ %; no LOD. Bipolar (Voltage kind, mV): σ_I% = k_A·√((1/A_E1 + 1/A_E2)/2) (eq. 10.9 is the equal-area form k_A/√A_E, hastings.txt L30243–30268, PDF p.507; the unequal-area generalisation is the same variance sum as MAT-04's Pelgrom term), σ_rand(ΔV_BE) = V_T·ln(1 + σ_I%/100) mV with V_T = kT/q at the die temperature (25.7 mV at 25 °C); μ_thermal = vbe_tc·|ΔT|·10⁻⁶ mV (µV/K × mK; textbook −2 mV/°C, hastings.txt L26020–26024, PDF p.433). Diode family uses the bipolar form (H09-15).
  3. Sidecar keys (with `_source` strings; key names follow FLOW-06; the new cell keys must be registered in FLOW-04's key registry as kind `Real`, sourced):
     - per resistor recipe (`pdks/sky130.json:137-167`): `"k_a_pct_um"` (FLOW-06's name), `"tc_ppm_per_k"`, `"sd_pct_per_mm"`; produce with a `characterize_mismatch.py res` mode (MC of the recipe model at ≥ 4 W×L, fit s_δ vs 1/√A) — sky130 value: produced by the run; if the PDK MC has no resistor mismatch, record `1.70` as **PROXY** (DVP Table 1 *derived* A_R, distance_vs_pair_mismatch.txt L172–198) exactly as `svt_uv_per_um` is labelled.
     - per capacitor recipe (FLOW-06 `capacitors` table): `"k_a_pct_um"` (not given for sky130; characterize or leave `null` → unknown);
     - cell: `"bjt_ka_pct_um": 2.0` (Hastings typical, §10.2.1, PDF p.507, labelled "textbook, not sky130"), `"vbe_tc_uv_per_k": 2000` (Hastings §9.1, PDF p.433, labelled textbook; replace with a `characterize_mismatch.py tc` run on the PDK's pnp when available).
  4. Constructor for EXT-20: `MatchedSet::for_family(members: Vec<DeviceId>, family: Family, kind: MatchKind, class: MatchClass, coeffs: Coeffs, budget: Budget, areas_um2: Vec<f32>, tol_nm: f32) -> MatchedSet`. `ledger` dispatches on `family`: Mos → MAT-04/09 terms; Resistor/Capacitor → Ratio terms (μ_lod = 0, not counted); Bipolar/Diode → bipolar terms. `known` = units for both members and `ka_pct_um` given (or coincidence applicable).
- **Tests**: `mismatch.rs::hastings_eq_8_12_capacitor_pair` (k = 1 %·µm, C in area units 1 and 4 µm² → 1·√((1+0.25)/2) = 0.791 %); `bjt_eq_10_9` (k_A 2 %·µm, A_E = 36 µm² both, V_T 25.7 mV → σ_I 0.333 % ± 0.001 and σ(ΔV_BE) 0.0855 mV ± 0.0005, Hastings' own example "0.33 % and 86 µV", H09-02); `resistor_thermal_ppm` (tc 100 ppm/K, ΔT 500 mK → 0.005 %).
- **Acceptance**: after EXT emits them, `bgr_core` and `dac4` report R/C/BJT sets with known σ where keys exist and explicit unknown where not; 0 silent passes.

### MAT-11 Capacitor-array metrics: full second moments, exact gradient model, M_sys

- **Priority** P1. **Effort** M. **Depends on** MAT-01.
- **Why**: B8. CC-02/05/06/12, NOTES-20, AC-16/21, H08-36.
- **Current**: `cap_array.rs:54-74,357-397`: radial `quad_um2`; INL/DNL with the linearised field `1 + g(x cosθ + y sinθ) + q r²` at 4 angles; no M_sys; `metrics()` test-only.
- **Change**:
  1. `ArrayMetrics` (`cap_array.rs:54-74`): replace `quad_um2` by `second_um2: f64` (max over slots of ‖M_slot − M_array‖_F per unit, µm²) and add `order: u8` (`moments::cancelled_order` over slots with ≥ 2 units), `msys: f64`.
  2. New pure functions in `kernel/analog/src/matching/dac.rs`:
     ```rust
     /// Per-slot capacitance under t0/t: C*_k = Σ_{u∈k} 1/(1 + g(x_u cosθ + y_u sinθ)) (units of C_u).
     pub fn gradient_caps(units: &[(u8, f64, f64)], slots: usize, g_per_um: f64, theta: f64) -> Vec<f64>;
     /// Worst |INL|, |DNL| (LSB) of a binary bank over θ ∈ [0,π) in steps of π/(4·max(rows,cols)).
     pub fn inl_dnl(units: &[(u8, f64, f64)], n_bits: u8, g_per_um: f64, steps: usize) -> (f64, f64);
     /// M_sys = max_{p≠q} |C*_p/C*_q − n_p/n_q| / (n_p/n_q), worst θ.
     pub fn msys(units: &[(u8, f64, f64)], counts: &[u16], g_per_um: f64, steps: usize) -> f64;
     ```
     Sources: t_j = t0 + γ(x cosθ + y sinθ), C* = Σ C_u t0/t_j (DACP eq. 11, cc_dac_constructive.txt L286–310; REV eqs 8–9, cc_review.txt L117–132); INL/DNL (DACP eqs 17–18, L271–284); M_sys (REV eq. 10, L133–142; DACP eq. 12, L311–321). γ's unit per length is not given in either source ("γ = 10 ppm", cc_dac_constructive.txt L909); `g_per_um` is a sweep parameter, reported with its value (1e-5 and 1e-4 per µm, labelled "DACP γ = 10/100 ppm, per-µm reading").
  3. `metrics()` (`cap_array.rs:357-397`) calls these; the θ sweep uses `steps = 4·max(rows, cols)`.
  4. `LedgerRow` (MAT-13) for a bank carries `inl`, `dnl`, `msys` at the two sweep points (report only; no deck coefficient → not a budget).
- **Tests**: `cap_array.rs::metrics_rank_the_families` (`cap_array.rs:684-704`) keeps only its route-spread assertion (spiral least route spread) and drops the INL/DNL ranking on the drawn array: under the exact t0/t model a uniform-pitch port of the 5-bit Spiral/Chessboard assignment ranks chessboard lower by only 0.02–1.1 % (*derived*, g = 1e-5 and 1e-4 per µm, pitch 2 and 5 µm), because the one-unit C0/C1 dominate, and the drawn track channels make x and y pitch unequal, so the sign on the drawn array is not guaranteed. The ranking is asserted instead in `dac.rs::chessboard_inl_not_worse_on_uniform_pitch`, on the uniform-pitch 5-bit assignments taken from `centro_assign(.., Fill::Compact/Dispersed)` at pitch 2 µm, g = 1e-4 per µm: INL(Dispersed) ≤ INL(Compact) and DNL(Dispersed) ≤ DNL(Compact) (the trend of DACP Tables II–III, PDF p.11–12); `radial_metric_no_longer_hides_anisotropy` (a hand-built 2-slot assignment with equal ⟨r²⟩ and unequal M_xx reports `second_um2 > 0`); `dac.rs::msys_zero_for_exact_linear_field_small_g` (g → 1e-9: M_sys < 1e-6 for any exact CC bank).
- **Acceptance**: `dac4` bench row prints order, second_um2, INL/DNL, M_sys for its bank.

### MAT-12 Resistor ratio patterns and Table 8.5 golden tests

- **Priority** P1. **Effort** S. **Depends on** MAT-02, MAT-05; CELL-06 (resistor generator honouring `dev_nf`, AC-02), CELL-14 (drawing per-member segment counts); EXT-19 (resistor sets).
- **Why**: H08-07/09/10/39/45, H06-06, AC-15, NOTES-27/28, H15-48 (integer part).
- **Current**: `resistor.rs:101,304-310`: equal counts only via `greedy_centroid`; jumpers mix li and met1 (`resistor.rs:142-154`).
- **Change**:
  1. `pattern.rs`:
     ```rust
     /// Segment order of one resistor row: per-member segment counts, point-symmetric,
     /// second moments balanced (`Fill::Balanced` on a 1×T grid). `exact` false when
     /// ≥2 members have odd counts; `scale2` then gives the doubled counts that are exact.
     pub fn segment_row(counts: &[u16]) -> (Vec<usize>, bool);
     pub fn scale2(counts: &[u16]) -> Vec<u16>;
     ```
     No diffusion constraint applies to resistor segments (any order is legal).
     `segment_row(c)` = `centro_assign(c, 1, Σc, Fill::Balanced)` flattened; `scale2(c)` = every count × 2 when ≥ 2 counts are odd, else `c` unchanged.
  2. Wiring is CELL-14's (plan-03): its step 2 calls `pattern::segment_row` instead of adding `builder::palindrome` (a second order generator, contrary to plan-03 §0.1 "CELL calls them and never re-implements an order generator"); CELL-14 then deletes `greedy_centroid` (`builder.rs:269-292`) and its test (`builder.rs:307-314`). CELL-14's examples (2:1 ABA, 3:2 ABABA, 4:1 AABAA, 5:4) come out of `segment_row` identically except 5:4, where `segment_row` gives ABBAAABBA (lower residue than the book's ABABABABA, table below); CELL-14's `ratio_arrays_are_common_centroid` holds for both.
  3. Emit `OrientationSet{PhiZero}` for every resistor set (EXT-19 recognises them; EXT-20 emits beside the `MatchedSet`), certifying alternate traversal per member.
  (The earlier `jumper_spread` metric was cut: CELL-14 step 3 makes every join the same met1 + 2 mcon construction and tests it, `every_join_has_the_same_via_count`; see appendix.)
- **Golden values** (Balanced algorithm on 1-D positions in pitch units; second-moment residue |ΔM| per unit; book strings from Hastings Table 8.5, hastings.txt L23636–23705, PDF p.395–396; *derived* here):
  | Ratio | Book | Book \|ΔM\| | Generated | Generated \|ΔM\| |
  |---|---|---|---|---|
  | 1:1 (2:2) | ABBA | 2.0 | ABBA | 2.0 |
  | 2:1 | ABA | 1.0 | ABA | 1.0 |
  | 4:1 | AABAA | 2.5 | AABAA (forced) | 2.5 |
  | 4:3 | ABABABA | 2.333 | ABABABA | 2.333 |
  | 5:2 | AABABAA | 4.2 | ABAAABA | 0.0 |
  | 5:1 (as 10:2) | AAABAAAABAAA | 6.8 | AABAAAAAABAA | 0.4 |
  | 5:3 (as 10:6) | ABAABABAABABAABA | 3.733 | ABABAAABBAAABABA | 0.533 |
  | 3:2 | ABABA | 1.667 | ABABA | 1.667 |
  | 3:1 (as 6:2) | AABAABAA | 4.0 | ABAAAABA | 1.333 |
  | 5:4 | ABABABABA | 3.0 | ABBAAABBA | 0.30 |
- **Tests** (`pattern.rs`): `table_8_5_every_ratio_is_exact_and_no_worse` — for the ten Table 8.5 ratios (1:1 ABBA, 2:1 ABA, 3:1 AABAABAA, 3:2 ABABA, 4:1 AABAA, 4:3 ABABABA, 5:1 AAABAAAABAAA, 5:2 AABABAA, 5:3 ABAABABAABABAABA, 5:4 ABABABABA; H08-39): generated counts equal the book's, `cancelled_order ≥ 1`, and r_2(generated) ≤ r_2(book) + 1e-9; every row of the table above asserts its generated string and |ΔM| within 1e-3 (values re-derived by script in the implementer review with the lower-index tie rule of MAT-02). `scale2_makes_two_odd_members_exact` ([3,1] → [6,2], exact).
- **Acceptance**: tests green; once CELL draws per-member counts, bench resistor sets report `order ≥ 1` and Orientation(PhiZero) 0 violations.
- **Risks**: the balanced order puts more segments of the larger member at the array edges than Hastings' strings; jumper lengths can grow (CELL-14's equal-construction jumpers keep them ratiometric). Hastings' dispersion argument (sub-array count) is a heuristic for the same quadratic residue the moment metric measures exactly (hastings.txt L23462–23499).

### MAT-13 Ledger rows in the report

- **Priority** P1. **Effort** S. **Depends on** MAT-04. Consumer: PERF (bench table).
- **Why**: MM-39, H08-01, AV-11 (partial: the flow's own model, not an independent GDS check), NOTES-03 (structured coverage).
- **Current**: `frontend/library/src/metadata.rs:71-86` has per-family status only; bench prints worst usage per kind (`benchmarks/src/bench.rs:321-360`).
- **Change**:
  1. `kernel/analog/src/matching/mismatch.rs`:
     ```rust
     #[derive(Clone, Debug, Default, PartialEq)]
     pub struct LedgerRow { pub members: (u32, u32), pub unit: &'static str /* "mV" | "%" */,
         pub sigma_rand: f32, pub sigma_layout: f32, pub mu_thermal: f32, pub mu_lod: f32,
         pub allowance: f32, pub usage: f32, pub order: u8, pub second_order_nm: f32,
         pub phi_equal: Option<bool>, pub known: bool,
         /// σ_rand ≥ the total budget: the allowance is 0 by sizing (MAT-08).
         pub sizing_limited: bool,
         /// "Pelgrom" or "MC" (MAT-21); "PROXY" suffix when S_VT is the deck's PROXY value (open question 10).
         pub sigma_source: &'static str }
     ```
     Field definitions: `sigma_layout` = the statistical layout term (σ_grad; √(σ_grad² + σ_quad²) if a quadratic coefficient ever appears); μ_sys (printed column) = `mu_thermal + mu_lod`; `phi_equal` = `moments::phi_equal` of the two members' `Sums` computed inside `MatchedSet::ledger` (`None` without units), so no cross-batch lookup is needed.
  2. `RuleBatch` default hook (`kernel/analog/src/rule.rs:135-208`): `fn ledger_rows(&self, state: &On, out: &mut Vec<crate::matching::mismatch::LedgerRow>) { let _ = (state, out); }`; `MatchedSet` implements it (one row per pair (0, i)).
  3. `MetadataReport` gains `pub matched: Vec<LedgerRow>`; `metadata::build` (`metadata.rs:177-203`) fills it from `placement.budget` only (MAT-04 pushes each `MatchedSet` into budget **and** cost; reading both would duplicate rows).
  4. PERF prints a "Matched sets" table per circuit (σ_rand, σ_layout, μ_sys, allowance, usage, order, Φ).
- **Tests**: `frontend/library/tests/flow_smoke.rs::matched_sets_are_reported` — `ota` solution: `metadata.matched` non-empty, every row finite, `known` rows have `sigma_rand > 0`.
- **Acceptance**: T7 on the 10 circuits.

### MAT-14 Thermal refinement: curvature, mobility coefficient

- **Priority** P2. **Effort** S. **Depends on** MAT-04 (step 1); MAT-09 (step 2 needs the % ledger); FLOW (die temperature from `Config.op.temp_c`, already used at `frontend/library/src/lib.rs:280`).
- **Why**: NOTES-53, H08-32, H12-03, H15-58 (partial), AR-24 (sampling part only).
- **Current**: MAT-04 samples the field at the two device centroids — exact for a linear field only.
- **Change**:
  1. Curvature correction: T̄_d ≈ T(c) + ∇T·(m_d − c) + ½ tr(H·M_d), with c the weighted centroid of both members' units and M_d = `Sums::second(c)` of member d **about c** (not about m_d); ΔT = T̄_a − T̄_b (the T(c) term cancels). Stencil of `rise_at_point_mc` around c, step h = max(1000 nm, L/4) (L = max unit distance from c): f_x = (f(h,0) − f(−h,0))/2h, f_y likewise; f_xx = (f(h,0) − 2f(0,0) + f(−h,0))/h², f_yy likewise; f_xy = (f(h,h) − f(h,0) − f(0,h) + 2f(0,0) − f(−h,0) − f(0,−h) + f(−h,−h))/(2h²). 7 point evaluations per pair (centre, 4 axis, 2 diagonal), replacing MAT-04's 2 centroid samples.
  2. MOS mobility term for Current kind: Δk/k = −(1.7 NMOS | 1.5 PMOS)·ΔT/T_abs (k ∝ T^−1.7 / T^−1.5, Hastings §12.1.1, hastings.txt L34436–34441, PDF p.578), T_abs = die temperature (K); added to μ_thermal in %.
- **Tests**: `matched_set.rs::curvature_correction_matches_unit_sampling` — 10 mW heater (half-extent 1 µm) 5 µm beyond the end of a 1-row AABB row (unit pitch 1 µm, one unit per finger): corrected ΔT within 10 % of the brute-force unit-weighted average of `rise_at_point_mc` per member (computed in the test); and for an ABBA row with the heater on the row axis the centroid-only ΔT is 0 while the brute-force ΔT is not, and the corrected value is within 10 % of it. `mobility_term_sign` (NMOS Current pair, ΔT = +1 K on member a, T_abs = 300 K → Δk/k = −0.567 % ± 0.001).
- **Acceptance**: bench ledger μ_thermal differs from the centroid-only value only for pairs near heaters; runtime per dp call +≤ 10 % (measured by PERF's stage timing when available).

### MAT-15 Nth-order central-symmetric patterns (order ≥ 3)

- **Priority** P2. **Effort** S. **Depends on** MAT-01, MAT-03; CELL-15 (draws the grid; `stack_on_tap` handles 2 rows today, `mosfet.rs:163-222`).
- **Why**: CC-03/04, NOTES-21, H08-40, MM-26, H13-41.
- **Current**: 1 row ABBA and 2 rows ABBA/BAAB (`mosfet.rs:152-156`); nothing beyond.
- **Change**: `pattern.rs`:
  ```rust
  /// Rows of a two-member pattern cancelling gradient orders 1..=order: P_1 = [row];
  /// P_n = P_{n−1} stacked on rot180(P_{n−1}), labels swapped when n is even (NTH §III).
  pub fn nth_order_rows(order: u8, row: &[u8]) -> Vec<Vec<u8>>;
  ```
  Vertical stacking only, so every row is a diffusion-legal palindrome of MAT-03 (a horizontal concatenation such as ABBABAAB puts a boundary on a drain: after finger 3, region 4 = D). Construction and proof: nth_order.txt L94–119 and L154–222 (odd n: self-symmetry cancels odd orders; even n: swapped copy cancels even orders), PDF p.2–3. Rows = 2^(order−1); units per member = 2^(order−1)·(units per member in `row`); with `row` = ABBA that is 2^order (NTH: 2ⁿ units per device cancel order n, nth_order.txt L90–119). CELL-15 offers rows ∈ {1, 2, 4} (orders 1–3; an 8-row order-4 grid is not drawn), the 4-row order-3 grid only when every member has ≥ 8 units and a multiple of 8. Every row returned must satisfy `diffusion_legal(row, Drain)` (debug assert).
- **Tests**: `pattern.rs::nth_order_rows_cancel_their_order` — for order 1..=4 with row ABBA: `cancelled_order(nmax = 5) ≥ order` and exactly `order` for 1..=3 (r_{order+1} > 0); `order_three_is_nth_fig_3b` (rows ABBA/BAAB/BAAB/ABBA, nth_order.txt L154–180); `nth_rows_are_diffusion_legal`.
- **Acceptance**: once CELL-15 draws 4-row variants, the bench ledger shows `order ≥ 3` for at least one ota pair when that variant is chosen (T4).
- **Risks**: 4 rows double the tap strips and gate routing; value appears only under curved gradients (LDM Fig. 11, long_distance_mismatch.txt L275–287); keep as variants, never forced.

### MAT-16 Statistical distance term: S(L), anisotropy, crossover diagnostic

- **Priority** P2. **Effort** S. **Depends on** MAT-04.
- **Why**: AR-03, MM-06/24/25/27, CC-07 (Pelgrom form retained), PEL "spacing ignorable below 100 µm²" (pelgrom.txt L150–151).
- **Current**: one worst-case `svt_uv_per_um` (PROXY, 1.63; `pdks/sky130.json:97-98`), isotropic `hypot`.
- **Change**: `Coeffs` gains `svt_fit: Option<(f32, f32)>` (a, b) and `svt_xy: Option<(f32, f32)>`; S(L) = √(a + b/L²) µV/µm with L = unit gate length (µm) of the set; σ_grad = √((S_x Δm_x)² + (S_y Δm_y)²)·10⁻⁶ mV/nm-units. Keys (sky130, PROXY): `"svt_a_uv2_per_um2": 0.1835`, `"svt_b_uv2": 0.03533` (fit to DVP Table 1, *derived* in MM-25, distance_vs_pair_mismatch.txt L172–198); `svt_x/y` absent (default isotropic). Precedence: `svt_fit` when both keys are present, else `svt_uv_per_um`. The two keys go through FLOW-04's registry (kind `Real`, `_source` = "PROXY: fit to Schaper & Linnenbank 130 nm Table 1", as `sky130.json:98`). The D* = A/(S·√WL) crossover report is EXT-29's (item 7), not repeated here.
- **Tests**: `mismatch.rs::svt_fit_reproduces_dvp_table_1` — S(0.12) = 1.624, S(0.24) = 0.893, S(0.48) = 0.580 (µV/µm, ± 0.01); `anisotropic_grad` (S_x = 1, S_y = 2 µV/µm, Δm = (1000, 1000) nm → σ_grad = √5·10⁻³ mV ± 1e-6).
- **Acceptance**: with the fit keys, ledger σ_grad of an L = 0.48 µm set is 0.580/1.63 = 0.356 of the worst-case value (the *derived* ratio S(L)/S_worst).

### MAT-17 Non-integer ratio segmentation optimizer

- **Priority** P2. **Effort** S. **Depends on** none. Consumers: EXT (unitization of ratioed resistors), CELL (per-member segment length).
- **Why**: H08-43/44, H15-48, H14-41, H06-06.
- **Current**: none (resistor lengths = `unit_l / n`, `resistor.rs:82`).
- **Change**: `pattern.rs`:
  ```rust
  /// Eq 8.27/8.28 sweep: for N in 1..=n_max, M = round(N·R_M/R_N), S = |N/R_N − M/R_M| (1/Ω).
  pub fn segmentation(r_n_ohm: f64, r_m_ohm: f64, n_max: u16) -> Vec<(u16, u16, f64)>;
  /// Eq 8.29–8.33 at one R0: (M, N, j, k, S) with S = |(N+1)/(N+k) − (M+1)/(M+j)|.
  pub fn partial_segments(r_n_ohm: f64, r_m_ohm: f64, r0_ohm: f64) -> (u16, u16, f64, f64, f64);
  ```
  Sources: Hastings eqs 8.27–8.33 (hastings.txt L23725–23859; equations and Figs 8.19–8.20 read on PDF p.396–398).
- **Tests**: `segmentation_reproduces_fig_8_19` — R_M = 200 kΩ, R_N = 146 kΩ, N ≤ 15: the three smallest S are at N = 8 (2.05e-7), 11 (3.42e-7), 3 (5.48e-7) (*derived*; matches the bars of Fig. 8.19 and the text "best matching when N is 3, 8, or 11"); `partial_segments_reproduces_the_book_decomposition` — R0 = 10.34 kΩ → M = 19, N = 14, j = 0.342, k = 0.120 (± 0.002).
- **Acceptance**: EXT/CELL can request a plan; output reported per resistor set.
- **Risks**: eq. 8.29 as printed gives S = 0.028 at R0 = 10.34 kΩ and 0.0095 at 10.64 kΩ (*derived*), while Fig. 8.20 plots minima ≈ 0 at both. GAP-20 resolved this (open question 7): Fig. 8.20 and the quoted optima are eq. 8.29 with j and k exchanged, and the printed form is the physically consistent one (`ref-hastings-99` §2.1). No test may use Fig. 8.20 or the 10.34/10.64 kΩ optima as golden values for `partial_segments`' S; the decomposition at 10.34 kΩ does not depend on S. Use eq. 8.27 (verified) as the primary optimizer; partial segments only on request.

### MAT-18 Split-DAC slot assignment, attenuation-capacitor check and sizing

- **Priority** P2. **Effort** M. **Depends on** MAT-02, MAT-11; EXT (recognising two banks joined by one non-unit capacitor, CC-16); CELL-21 (draws `Pattern::Split` and the non-unit plate from these functions). **Gate**: start only when CELL-21's gate is met (a split-DAC fixture exists); no fixture in `benchmarks/fixtures` has a split DAC today, so without the gate this is unused code.
- **Why**: CC-16/17/23/32, NOTES-31, AC-16.
- **Current**: `bits()` accepts only the single-bank `[1,1,2,…]` form (`cap_array.rs:123-130`).
- **Change**: `dac.rs`:
  ```rust
  /// C_A = (C_T^LSB / C_T^MSB)·C_u (DACP eq. 1).
  pub fn attenuation_cap(ct_lsb_units: u32, ct_msb_units: u32) -> f64;
  /// Non-unit capacitor side lengths (H, l) with area A and the unit's perimeter-to-area
  /// ratio k = (H_u + l_u)/(H_u·l_u): roots of z² − kA·z + A = 0, H ≥ l.
  pub fn nonunit_dims(area_um2: f64, unit_h_um: f64, unit_l_um: f64) -> Option<(f64, f64)>;
  /// Slot counts [C0..C_L | C_{L+1}..C_N | C_A, C_A] and the DACP §IV-C order:
  /// C_A half-slots nearest the centre, odd-count caps next, then LSB/MSB alternating
  /// along the spiral, each unit with its point reflection.
  pub fn split_dac_assign(l_bits: u8, m_bits: u8, rows: usize, cols: usize) -> Vec<Option<u8>>;
  ```
  Sources: DACP eq. 1 (cc_dac_constructive.txt L60–76, PDF p.1), §III-D eqs 37–45 (L534–616, PDF p.6; the quadratic is derived in CC-17), §IV-C (L829–913). EXT flags C_A outside ±1 % of `attenuation_cap` (CC-16).
- **Tests**: `attenuation_cap_formula` (L = 3, M = 3 → C_T^LSB = 8, C_T^MSB = 7 in units → 8/7); `nonunit_dims_keep_the_unit_edge_sensitivity` ((H+l)/(Hl) equals the unit's within 1e-9 before snapping); `split_assign_is_exact_for_even_count_caps`.
- **Acceptance**: a split-DAC fixture (PERF) draws with exact CC for every even-count cap and reports C_A error.

### MAT-21 SPICE Monte-Carlo random floor per matched set

- **Priority** P2. **Effort** S (MAT side). **Depends on** MAT-04, MAT-09; PERF-27 (`perf::pair_sigma_mc`, plan-07, the producer; ngspice runner; the op runner `extract` exists at `frontend/library/src/oppoint.rs:155-174` with its deck built by `build_deck` at `oppoint.rs:217`, the offline MC at `benchmarks/characterize_mismatch.py:74-84`).
- **Why**: MM-29, MM-03, MM-20 (1:4 vs 4:1 mirrors do not follow the area law: Drennan Table III, drennan_mismatch.txt L286–308), DRE's central recommendation (L21–29).
- **Change**: `MatchedSet` gains `sigma_rand_override: Option<f32>` (set's unit); when `Some`, the ledger uses it instead of the Pelgrom σ_rand and the `LedgerRow` labels it "MC". PERF produces it (one ngspice batch per unique geometry and bias, before the epoch loop).
- **Tests**: `matched_set.rs::override_replaces_the_area_law` (override 1.5 mV on a pair whose Pelgrom σ_rand is 2.124 mV → ledger `sigma_rand == 1.5`, `Eta(0.3)` allowance 0.45, `LedgerRow.sigma_source == "MC"`).
- **Acceptance**: with the runner on, ratioed-mirror rows show the MC value; difference vs area law reported.

---

## 4. Milestones

| Milestone | Items | Exit criteria |
|---|---|---|
| M1 Patterns correct | MAT-01, MAT-02, MAT-03 | `pattern.rs` and `moments.rs` property tests green (incl. both `Outer` conventions and the deterministic tie rules); `bjt.rs::every_bjt_ratio_is_common_centroid` green; existing cap_array tests unchanged; T2 (BJT part). T3 (ratioed mirrors merge) is measured when CELL-10 wires `diffusion_cc_row` (CELL-10's `a_ratioed_mirror_merges`) |
| M2 One honest ledger | MAT-04, MAT-05, MAT-06 | `MatchingPair`/`ThermalGradient`/`CentroidGroup` deleted; annotator/analog/library tests green; bench local: DRC 0, LVS MATCH on 10 circuits, `Orientation` 0 violations, `MatchedSet` 0 unknown where units exist, HPWL/footprint within ±5 % (T1, T5, T6, T8) |
| M3 Classes and domains | MAT-13, then MAT-07, MAT-08, MAT-09 (after FLOW-06, EXT-16, EXT-17) | `MetadataReport.matched` populated on the 10 circuits (T7; MAT-13 needs only MAT-04 and lands first); every `MatchedSet` carries class/family; `phi_arm` emission; a budget below σ_rand reads `sizing_limited` and violated on `ota`; A_β values produced by the BPV run and stored in FLOW-06's slots; mirror rows in % when A_β is present |
| M4 All device families | MAT-11, MAT-12, MAT-10 | Table 8.5 golden tests green (T2 resistor part); `dac4` row reports order, second_um2, INL/DNL, M_sys; R/C/BJT sets scored with known σ where keys exist and explicit unknown where not, once EXT-19 emits them |
| M5 Advantage | MAT-14, MAT-15, MAT-16, MAT-17, MAT-18 (gated), MAT-21 | each item's tests green; T4 once CELL-15 draws 4-row variants; ledger σ_grad uses S(L) when the fit keys exist; MC σ rows on `ota` once PERF-27 runs |

---

## 5. Open questions (each with the default this plan uses)

1. **Aggregating deterministic terms** (audit-02 Q1): signed sum (terms can cancel) or sum of magnitudes? The decks store |TC| and KVTH0 without a reliable sign convention for both polarities. **Default: magnitudes** (worst case, Lampaert eq. 4.25 convention).
2. **Thermal term in Θ during dp** (MAT-04): live field (moves change Θ) or epoch-frozen per unit (needs a new `Layout` column across ~24 struct-literal sites)? **Default: live**; revisit if PLC's gate change (AP-13) makes it thrash.
3. **Coincidence tolerance** (AR-37): half the cut lattice (absolute) or a fraction of the extent? **Default: lattice/2** (5 nm on sky130), applied only to merged, CC-feasible sets.
4. **Class when EXT gives none**: **Default: Moderate for geometry tables, `Eta(0.3)` for budgets** (no tightening until a class comes from a user annotation or a spec, EXT-12 `ClassSource::{User, Spec}`; a `Role` default never sets a budget, MAT-08).
5. **Who owns thermal-field fidelity** (AR-24: finite die, deck k, image sources)? MAT consumes only `Layout::rise_at_point_mc`. **Default: REL**; if REL declines, it becomes a P2 MAT item on `kernel/core/src/thermal.rs:9-49`.
6. **DACP γ unit** (MAT-11/19): "γ = 10 ppm" gives no length unit. **Default: per µm** (`g_per_um` = 1e-5, 1e-4), reported as a labelled sweep, never a pass/fail.
7. **Hastings eq. 8.29 vs Fig. 8.20** (MAT-17): the printed formula does not reproduce the plotted minima (*derived* S = 0.028 at R0 = 10.34 kΩ). **Resolved by GAP-20** (`ref-hastings-99` §2.1): the figure plots |(N+1)/(N+j) − (M+1)/(M+k)|, eq. 8.29 with j and k exchanged, which reproduces every Fig. 8.20 point and has its minima at 10.343 and 10.642 kΩ; the App. D derivation (hastings.txt L50812–50866) gives the printed form. **Default (unchanged): implement eq. 8.29 as printed, use eq. 8.27 as the primary optimizer.** No outside confirmation needed.
8. **Resistor/capacitor A for sky130**: does the PDK MC switch vary resistor and MIM mismatch? **Default: run the `res`/`cap` characterization modes; if the MC is silent, store the DVP-derived resistor PROXY (1.70 %·µm, labelled) and leave capacitors unknown.**
9. **Mirror vs perfect symmetry per pair** (PLC decision, AP question 2): **Default input from MAT: `mirror_allowed` true only when every member has Φ_H = 0**; otherwise perfect (translated) symmetry.
10. **S_VT PROXY in the ledger** (AR-03): keep a 130 nm proxy term in a certified number? **Default: keep it, label it PROXY in `LedgerRow`**, since it is small at block scale (D* ≥ 0.58 mm at 100 µm², MM-06) and removing it would hide the distance term entirely.
11. **Tier encoding conflict between FLOW-06 and CELL-12** (not MAT's to decide; recorded because MAT-07 used to define it): FLOW-06 stores tiers as objects keyed by class name, read by `Process::tier(key, MatchClass)`, with `lod_moat_ext_nm` Minimal absent and key `dummy_poly_reach_nm`; CELL-12 stores 4-entry arrays indexed by `MatchGrade { Unmatched, Minimal, Moderate, Exceptional }` read through `parse_roles` scalars `key[i]`, with `lod_moat_ext_nm` Minimal 2000 and key `dummy_reach_nm`. **Default for MAT: consume FLOW-06's accessor and `pnr_core::MatchClass`; "unmatched" = `Option<MatchClass>::None` (EXT-12).** CELL-12 and FLOW-06 must converge on one encoding before either lands.
12. **Who defines `MatchKind`/`Family`**: EXT-12 and MAT-04 both need `MatchKind` early. **Default: whichever lands first defines `MatchKind { Voltage, Current, Ratio }` and the other re-exports it; `Family` lives in `analog::matching::class` and EXT-16 imports it.**

---

## Verification log

Adversarial fact-check of this plan on the working tree of 2026-09-28 (read-only, no builds). Method: every `path:line` claim opened in the source; every reftext citation re-read at the cited lines, with the PDF page recomputed as 1 + form-feeds before the line; every entry ID looked up in `docs/plans/audit-*.md` / `ref-*.md`; every worked number recomputed (Python scripts in the session scratchpad: MAT-03 row algorithm, `Fill::Balanced` 1-D rows, `cancelled_order` on the test patterns, MAT-17 sweeps, a port of the cap-array Spiral/Chessboard assignment for ρ_avg and INL/DNL). Hastings PDF pages 433, 709, 712, 713 and 514 were opened because the constants or figure are garbled or absent in the reftext.

**References checked: 427** (about 188 code citations, 55 reftext line-range citations, 184 entry IDs). All 184 IDs exist as entries in the audit/ref docs and their titles match the use made of them.

### Confirmed (selection)

- Code: all §1.1 and §1.2 rows (emit.rs, matching_pair.rs, cc.rs, placement/thermal.rs, core/thermal.rs, ids.rs, units.rs, rule.rs, cellgen.rs, mosfet.rs, bjt.rs, cap_array.rs, resistor.rs, builder.rs, sky130.json, metadata.rs, bench.rs, library lib.rs line ranges). No producer emits `Target::Group`; `delta_temp_mc`/`live_delta_temp_mc` have no caller outside the two thermal files; `systematic_allowance_mv` has one caller (`lib.rs:794`); no rule reads `PlacedUnit.phi`; `wpe_clearance_nm`/`lod_moat_ext_nm` are read by no code; `greedy_centroid([2,4])` = BBAABB; 24 `Layout` struct literals (open question 2); no deck has `abeta`, `ka_pct`, `cap_rho` or `svt_a` keys.
- Proposed names: no existing `MatchedSet`, `OrientationSet`, `Sums`, `Pt`, `Axis`, `Fill`, `MatchClass`, `MatchKind`, `Family`, `Coeffs`, `Ledger`, `LedgerRow`, `SizingNote`, `Lever`, `CorrModel`, `MosEnv`, `PassiveEnv`, `Process::tier`, or `analog::matching`.
- Numbers: 3.953 mV, usage 1.256 / residual 0.256, allowance 0.9, σ_grad 1.63 mV, μ_thermal 1.097 mV (2150.7 − 716.9 mK), 1748/√WL µm, Table 13.4 checks (5625 vs 5600, 215 vs 220, 5.76 vs 5.8, PDF p.713), 1.118 %, 0.791 %, 0.33 % / 86 µV, S(L) = 1.624 / 0.893 / 0.580, MAT-17 (N = 8, 11, 3 at 2.05e-7, 3.42e-7, 5.48e-7; R0 = 10.34 kΩ → 19, 14, 0.342, 0.120; S = 0.0283 and 0.0095), C_A = 8/7, f3dB = 72.1 MHz, var_ratio reduction, Table 8.5 rows (3:1 1.333 vs 4.0; 5:4 0.30 vs 3.0; generated r_2 ≤ book for all ten ratios), MAT-03 worked rows [2,4], [2,2], [4,4], [6,6], [4,8], [4,4,4], [1,2], MAT-01 test orders (ABBA 1, ABBA/BAAB 2 with Δx²y = 1 pitch³, Fig. 3(b) 3, radial case 1), MAT-15 recursion orders 2–5, BJT traces of H09-49.
- References: Hastings class limits ±10/3/1 mV and % (PDF p.712), eq. 13.61 example incl. +½ ≠ −½ (PDF p.709), −2 mV/°C (PDF p.433), eqs 8.23/8.26/13.42–13.43/13.48/13.49, Table 8.4 rule 1, rules 7/9/12/19/21, R4/R9/R10/R11, §13.2.2 "two or three microns"; NTH eqs 3–6, §III, Fig. 3(b), Table I (5.22 %); REV eqs 1–3, 8–16, §V-A; DACP eqs 1, 11–24, 32, 37–45, §IV-C (C_A slots first near the centre, L829–847), γ = 10 ppm (L909), inherited values (L1014–1019); Pelgrom eq. 1, L111–119, L150–151; LDM eq. 4, Fig. 11; DVP L85–101, Table 1; Drennan §III-A, Table III, abstract; Lampaert eq. 4.25.

### Corrections (before → after)

1. MAT-02 formulas, [1,4]: "B the four edge centres → the 2:1:2 cross (… PDF p.513)" → the generated pattern is a "+" cross; Hastings' 2:1:2 (Fig. 10.24A, figure on PDF p.514) puts the four unit sections of the 4X transistor two per side column on the diagonals of the 1X; both exact; the book form is a half-row-offset arrangement, not a 3×3 assignment (`ref-hastings-99` §2.3); no corner variant.
2. MAT-02 step 6: "`pattern::grids(..)[0]` (same near-square grid as `cap_array.rs:142-143`)" → not always the same grid (one odd member moves to an odd×odd grid, e.g. [1,2,2] 2×3 → 3×3; [4,4,4] tie unspecified); the centroid test still passes.
3. MAT-02 step 5: added that `bjt.rs` emits no `Unit` records today (AC-08), so the centroid test and `MatchedSet` coincidence need a `pnr_core::Unit` per emitter in `Unit::draw` (`bjt.rs:145-202`).
4. MAT-03 step 2.4: added the S_d pre-count convention (end fingers and middle token counted before dealing); without it [6,6]/[8,8] come out B-ended.
5. MAT-03 test `equal_counts_reproduce_the_old_orders`: (3,8) removed from the equality set; the stated algorithm gives A BB CC CC AA BB AA BB AA CC CC BB A (r_2 0.083 vs the old order's 0.091), so assert r_2(new) ≤ r_2(old) for it.
6. MAT-04 step 7: "keep `is_fet`/`is_diff_pair`/`is_supply`" → also keep the `G`/`D`/`S` constants (`routing/crosstalk.rs:7`, `routing/differential.rs:6` import them); also delete `pub mod cc;`/`pub mod thermal;` (`placement/mod.rs:3,10`) and add `pub mod matched_set;`/`pub mod orientation;`.
7. MAT-04 risks: "frozen per epoch, `thermal.rs:5-7`" → `kernel/analog/src/placement/thermal.rs:12-13`, `kernel/core/src/thermal.rs:4`.
8. MAT-07 step 5: "every construction site (`constraints.rs:49`, `cellgen.rs:493,515,537,562`)" → 13 full literals: adds `cellgen.rs:1257`, `cells/src/lib.rs:72`, `capacitor.rs:387`, `cap_array.rs:601`, `cells/tests/cell_selfcheck.rs:45,368`, `macroMaster/src/adapter.rs:29`, `examples/three_stage_opamp/src/main.rs:94` (`Unitization` has no `Default`).
9. MAT-07/MAT-08: `enum Limit` → `enum ClassLimit` (`analog::routing::em::Limit` already exists in the same crate).
10. MAT-09: `unit: Unit { Mv, Pct }` → `unit: LedgerUnit` (`pnr_core::Unit` is the drawn-unit type used by `moments.rs`).
11. MAT-20: "C_u is wrong by ~14×, AV-32" → ~14× by computed plate density (AC-01), 19× by dac4's extracted unit cap (AV-32).
12. MAT-21: "op deck builder at `oppoint.rs:155-171`" → op runner `extract` at `oppoint.rs:155-174`, deck built by `build_deck` at `oppoint.rs:217`.
13. §1.1 Orientation row: added that `rotatable` reads `Layout.groups`, set to the abutment table at `frontend/library/src/lib.rs:577-579`.
14. PDF pages: MAT-01 NTH eqs 3–6 p.2 → p.1–2; MAT-15 NTH construction p.3 → p.2–3; MAT-05 eq. 8.34 p.399–402 → p.400–402; MAT-02 Fig. 10.23 p.512 → p.512–513.

### Unresolved

- MAT-11 test `metrics_rank_the_families` under the exact t0/t model: holds on a uniform-pitch port with margins of 0.02–1.1 %; not run on the drawn geometry (marked inline).
- MAT-19 test `chessboard_raises_rho_avg`: holds at 8 bits on the port (5e-6 to 5e-4 margin), reverses at 6 bits; not run on the drawn geometry (noted inline).
- [resolved in the implementer review] MAT-02 `Fill::Balanced` tie rule and `grids` ordering: now lower-index ties, r²-descending/angle-ascending deal order, and a total order on grid candidates (MAT-02 steps 4.1–4.5); MAT-02/03/12 worked values re-derived with them.
- Constants the reftext does not carry (H13-47 dummy/moat µm values, H13-53 WPE µm values, R9 dummy span, rule 21 extension, k_A = 2 %·µm, eq. 8.12 form) were taken from the study docs; only the PDF pages listed above were re-read.

---

## Cut or deferred items

Each entry names what was removed from the work items, why, and who owns it now. Nothing here is lost: the text of the two removed P2 items is kept verbatim below so they can be revived without research.

| Removed | Reason | Owner now / revival condition |
|---|---|---|
| MAT-07 (old): `MatchClass` enum in `analog::matching::class` | Duplicate: FLOW-06 defines `pnr_core::MatchClass` (lowest crate, visible to cells/verify), EXT-12 an `intent::MatchClass`, CELL-12 a `MatchGrade`. Three enums would not interconvert | FLOW-06; MAT-07 re-exports |
| MAT-07 (old): `limit(Family, MatchKind, MatchClass)` table | Duplicate of EXT-16 `limits`, with a conflicting R/C row (MAT took the loose end 5/1/0.1 %, EXT the tight end 1/0.1/0.01 % of Hastings' ranges, hastings.txt L25121–25133). One table only | EXT-16 |
| MAT-07 (old): `Process::tier(name, idx)`, sidecar keys `lod_moat_ext_nm`, `dummy_reach_nm`, `gate_ext_extra_nm`, `Pdk` impl | Duplicate of FLOW-06 (`tier(key, MatchClass)`, tier objects) and CELL-12 (4-entry arrays); open question 11 | FLOW-06 / CELL-12 |
| MAT-07 (old): `mos_env`/`MosEnv` (dummy reach, moat, WPE, gate extension, max aspect) | CELL-12 draws all four from the tiers directly; aspect filter (H13-46) is CELL's (§0.2 row CELL) | CELL-12, CELL |
| MAT-07 (old): `resistor_env`/`PassiveEnv` (R4/R9/R10 dummies and floors) | CELL-14 step 5 (dummies by grade) and CELL-23 (width/length floors, H08-41) | CELL-14, CELL-23 |
| MAT-07 (old): `Unitization.class: MatchClass` at 13 literal sites | EXT-12 adds `Unitization.class: Option<MatchClass>` (the `Option` encodes "unmatched") | EXT-12 |
| MAT-07 (old): `frontend/library/src/lib.rs:871-872` switch to tiers | EXT-16 "consumer edit" and CELL-12's fact-check note both claim it; it must move in the same change as the deck keys | EXT-16 with CELL-12 |
| MAT-08 (old): `SizingNote`, `Lever`, `area_for`, `Problem.sizing`, test `table_13_4_areas_reproduce` | Duplicate of EXT-29 (area/length diagnostics H13-43/44, the `(σ_rand/σ_b)²` multiplier) | EXT-29; MAT keeps only `LedgerRow.sizing_limited` |
| MAT-16 (old): D* = A/(S·√WL) column | Duplicate of EXT-29 item 7 | EXT-29 |
| MAT-09 (old): `AnnotationConfig.gm_over_id` | Duplicate plumbing of EXT-17's `Evidence.op` | EXT-17 |
| MAT-03 (old step 3): mosfet.rs `enumerate`/`finger_sequence` edits and test `a_ratioed_mirror_merges_without_shorting` | CELL-10 (P0) wires the same change and owns the equivalent test `a_ratioed_mirror_merges`; two P0 items editing the same lines would conflict | CELL-10 (calls MAT-03's function) |
| MAT-05 (old): `backend/dp/src/tests.rs::matched_halves_never_turn_apart` | PLC-03 already merged it as `mixed_polarity_composite_halves_turn_together` | PLC-03 |
| MAT-12 (old): `jumper_spread` metric and test | CELL-14 step 3 makes every join the same met1 + 2 mcon construction and tests it (`every_join_has_the_same_via_count`), so a report-only spread would always read 0 | CELL-14; revive only if CELL-14 keeps mixed li/met1 jumpers |
| MAT-12 (old step 2): resistor.rs / builder.rs edits | CELL-14 owns resistor.rs and deletes `greedy_centroid`; it calls `pattern::segment_row` | CELL-14 |
| MAT-19 (whole item): capacitor correlation metrics ρ_pq, M_rand, 3σ INL | Speculative: no deck has `cap_af_pct_um`, `cap_rho_u`, `cap_corr_len_um`; the output is a labelled sweep with the paper's inherited values (cc_dac_constructive.txt L1014–1019) that no decision consumes; its only ranking test reverses between 6 and 8 bits (*derived*). | Revive when a deck gains measured capacitor mismatch and correlation data, or a DAC fixture has an INL spec that needs M_rand |
| MAT-20 (whole item): per-bit Elmore τ and f3dB for cap arrays | Report-only; C_u is wrong by ~14–19× until CELL's AC-01 capacitor fix (AV-32), and signoff PEX (PERF) already reports the extracted C_TS/C_TB; no fixture has a speed spec | Revive after CELL-08/AC-01 when a DAC fixture carries a settling/f3dB spec |
| Not added: stochastic / ILP CC placement for arbitrary ratios (CC-43) | No fixture has a non-binary capacitor set; `centro_assign` is exact for ≤ 1 odd member and MAT-11 metrics show the residue | Revive when a fixture's `second_um2` or M_sys is the limiting row |
| Not added: resistor sub-array banks and 2-D bank grids (H08-42) | Variant space of the resistor generator | CELL (CELL-14 follow-up) |
| Not added: numeric symmetry-defect and compactness checks (H13-40 rules 2–4) | The moment certificate (`cancelled_order`, `second_order_nm`) measures the gradient error those rules are proxies for | — |

Verbatim text of the two removed P2 items:

#### (former) MAT-19 Capacitor random-mismatch correlation metrics (ρ, M_rand, 3σ INL)

- **Priority** P2. **Effort** M. **Depends on** MAT-11.
- **Why**: CC-08/09/10/13, NOTES-26, H08-05, H08-38 (dispersion vs area becomes computable).
- **Current**: none; greedy farthest-point `spread` stands in for dispersion (`cap_array.rs:259-280`).
- **Change**: `dac.rs`:
  ```rust
  pub struct CorrModel { pub rho_u: f64, pub corr_len_um: f64 }   // ρ_ab = ρ_u^(D/L_c)
  pub fn rho_pq(units: &[(u8, f64, f64)], p: u8, q: u8, m: CorrModel) -> f64;   // S_pq/√((p+2S_p)(q+2S_q))
  pub fn var_ratio(units: &[(u8, f64, f64)], p: u8, q: u8, sigma_rel: f64, m: CorrModel) -> f64;
  pub fn inl_dnl_3sigma(units: &[(u8, f64, f64)], n_bits: u8, sigma_rel: f64, m: CorrModel, g_per_um: f64) -> (f64, f64);
  ```
  var(C_p/C_q) = σ_rel²/q⁴·[q²(p+2S_p) + p²(q+2S_q) − 2pq·S_pq] (REV eqs 12–16, cc_review.txt L147–202; DACP eqs 13–16, L323–370); 3σ INL/DNL (DACP eqs 19–24, L285–363). Deck keys `cap_af_pct_um`, `cap_rho_u`, `cap_corr_len_um`: absent in all four decks → the report uses the labelled sweep ρ_u ∈ {0.5, 0.9, 0.99}, L_c = 1000 µm (the paper's values are inherited, not measured: cc_dac_constructive.txt L1014–1019) and marks the rows "sweep".
- **Tests**: `var_ratio_reduces_to_independent_units` (ρ_u = 0 → (p/q)²·σ_rel²·(1/p + 1/q), exact); `chessboard_raises_rho_avg` (8-bit: ρ_avg(Chessboard) > ρ_avg(Spiral) at ρ_u = 0.9; checked with a uniform-pitch port of the two assignments at L_c = 1000 µm and pitch 1–100 µm: holds at 8 bits by 5e-6 to 5e-4 absolute (*derived*), but reverses at 6 bits, so keep the test at 8 bits and compare exactly, not with a tolerance).
- **Acceptance**: `dac4` bench row reports M_rand and 3σ INL at each sweep point.

#### (former) MAT-20 Array routing metrics: per-bit Elmore τ and f3dB

- **Priority** P2. **Effort** M. **Depends on** MAT-11; CELL (AC-01: a real capacitor device, otherwise C_u is wrong by ~14× by computed plate density, AC-01, and 19× by dac4's extracted unit cap, AV-32).
- **Why**: CC-15/31, NOTES-32, AC-16 (single-width tracks), DACP Fig. 15 trade.
- **Current**: `build()` returns per-slot route length and via1 count (`cap_array.rs:399-420`); `route_spread` only.
- **Change**: `dac.rs`:
  ```rust
  /// Elmore delay to every unit of each slot over its bottom-plate tree (Ω, fF → ps).
  pub fn elmore_tau_ps(tree: &[(usize /*parent*/, f64 /*R Ω*/, f64 /*C fF*/)], slot_of_node: &[Option<u8>], slots: usize) -> Vec<f64>;
  /// f3dB = 1/(2(N+2)·ln2·τ_max), Hz (DACP eq. 32).
  pub fn f3db_hz(n_bits: u8, tau_max_ps: f64) -> f64;
  ```
  cap_array builds the per-slot RC tree from its drawn tracks (R from `Process::sheet_ohm`, leaf C = unit C from the deck) and reports τ per bit and f3dB in `ArrayMetrics`. Source: DACP §III-C eq. 32 (cc_dac_constructive.txt L469–493, PDF p.5). C_TS/C_TB after routing come from signoff PEX (PERF).
- **Tests**: `elmore_of_a_chain` (3-node chain, hand-computed τ); `f3db_formula` (N = 8, τ = 1 ns → 1/(2·10·0.6931·1e-9) = 72.1 MHz).
- **Acceptance**: `dac4` reports τ per bit and names the critical bit; values flagged "C_u uncalibrated" until CELL's AC-01 fix.

---

## Implementer review log

Review of 2026-09-29 as the engineer who implements this plan next, read-only on the code. All 21 items were reviewed. Method: every item re-read against the code it edits (`kernel/analog/src/rule.rs`, `placement/matching_pair.rs`, `kernel/core/src/{thermal,units,lib}.rs`, `backend/annotator/src/{emit,tests}.rs`, `frontend/library/src/{lib,cellgen}.rs`, `pdks/sky130.json`), against the other seven plans for ownership overlaps (grep of plan-01, 03, 04, 05, 07, 08 for every MAT type, key and function name), and every pattern number re-derived with small exact-arithmetic scripts in the session scratchpad (`bal.py`, `dcc.py`, `src.py`, NTH recursion).

Changes:

1. **Cross-plan contracts** (§0.2): new table naming the owner of every shared type or hook MAT touches: FLOW-06 `pnr_core::MatchClass` and `Process::tier`; EXT-12 `MatchKind` and the `intent::MatchedSet` name clash; EXT-16 limits; EXT-17 op facts; EXT-20 emission; EXT-29 sizing diagnostics; CELL-10/12/14/15/21/23; PLC-03 `matched_pairs`; PERF-27.
2. **MAT-02**: the tie rule was underspecified (verification log "Unresolved"). Replaced with a total order: pairs by r² then representative angle, dealing r²-descending/angle-ascending, deficit ties to the lower index, exact i64 rationals; `grids` candidate order defined. [2,2] → [0,1,1,0] and the Table 8.5 values re-derived under it.
3. **MAT-03**: added `Outer { Drain, Source }` and `diffusion_legal` so the one pure function covers CELL-10's mirror-pins convention. The mosfet.rs wiring and the sky130 merge test were moved to CELL-10, which was a P0 duplicate editing the same lines. Source-convention algorithm and tests added; CELL-10's four examples re-derived identical, including ALIGN's [2,2,4,8,8].
4. **MAT-04**: `MatchKind` gets `Ratio` from the start, and `Budget::Allowance` was added for EXT-20's allowance. `Ledger::usage` and `residual` are defined at allowance 0 using `rule::over` (`rule.rs:313-318`), so no NaN is possible. The sky130 coefficient values and a worked σ_rand (2.124 mV, allowance 0.637) were added, plus the `matched_pairs` implementation, the name-clash rule and the PLC-03 test migration. The thermal test now fixes the heater half-extent at 1 µm, because the r floor would otherwise break the numbers. Two new tests were added.
5. **MAT-05**: `Axis` is defined for φ = (0,0) units and `Mixed`, and `matched_pairs` is implemented. The dp test was dropped because PLC-03 already owns it, and `axis_ignores_zero_phi_units` was added.
6. **MAT-06**: the allowance is always in mV (Current-kind conversion given) and the leaf is looked up in both orders. The exact `Flow` field sites to delete are listed (`lib.rs:332-333,484-487,790,794`).
7. **MAT-07**: rewritten from "class data model and tables" (M) to "class, kind and family on the matching rules" (S). The enum, limit table, tier accessor, deck keys, `mos_env`/`resistor_env`, the `Unitization` field and the `lib.rs:871-872` switch are all owned by FLOW-06, EXT-12, EXT-16, CELL-12, CELL-14 or CELL-23, and are listed in the appendix. What stays: `Family` (with Diode), `phi_arm`, class/family fields and the AR-41 citation fix.
8. **MAT-08**: reduced to class-derived budgets with an explicit precedence (`offset_sigma_mv` > EXT allowance > class when `ClassSource ∈ {User, Spec}` > `Eta(0.3)`). `SizingNote`/`area_for` were cut as duplicates of EXT-29, and `LedgerRow.sizing_limited` was added. The acceptance criterion now asserts the ledger outcome.
9. **MAT-09**: G is read from EXT-17's `Evidence.op` instead of a new config field. Keys come from FLOW-06's slots, `Sigma1Pct` is defined, and the no-op fallback is specified.
10. **MAT-10**: key names are aligned to FLOW-06 (`k_a_pct_um` per resistor/capacitor recipe), and the registry requirement for the new cell keys is stated. The bipolar formula units were fixed (σ% → ΔV_BE via V_T·ln(1+σ/100); µV/K × mK → mV), and the diode family is mapped. The `for_family` signature and its dispatch are complete. The BJT test numbers are now exact with tolerances, and a resistor thermal test was added.
11. **MAT-11**: the INL/DNL family-ranking assertion (marked UNVERIFIED, margin 0.02–1.1 %) was moved off the drawn array onto a uniform-pitch synthetic test in `dac.rs`. The drawn-array test keeps only the route-spread assertion.
12. **MAT-12**: CELL-14 now calls `pattern::segment_row` instead of adding `builder::palindrome`, a second order generator. The resistor.rs/builder.rs edits moved to CELL-14, and `jumper_spread` was cut because CELL-14 equalises jumpers by construction. The golden table was completed for all ten Table 8.5 ratios with re-derived values; previously two cells read "—", and 4:3, 5:2, 5:1 and 5:3 were missing. Effort M → S.
13. **MAT-13**: `sizing_limited` and `sigma_source` were added. `sigma_layout`, μ_sys and `phi_equal` now have definitions, and rows are read from the budget arm only, which avoids duplicate rows.
14. **MAT-14**: the curvature formula was corrected so that M_d is taken about the pair centroid, and the 7-point stencil is written out. Step 2 now depends on MAT-09. The tests are concrete, including the mobility sign test (−0.567 %).
15. **MAT-15**: rows ∈ {1, 2, 4} and ≥ 8 units per member for order 3, aligned with CELL-15. The units-per-member formula was corrected, and the exact cancelled orders 1–4 of the recursion were re-derived. The legality assert and test were added.
16. **MAT-16**: D* was removed (EXT-29 owns it). Key precedence and registry are defined, an anisotropy test was added, and the acceptance criterion is now numeric (0.356 at L = 0.48 µm).
17. **MAT-18**: gated on CELL-21's split-DAC fixture, and CELL-21 is named as the consumer.
18. **MAT-19 and MAT-20**: moved to "Cut or deferred items" with revival conditions (speculative sweep / report-only on an uncalibrated C_u); their text is kept verbatim there.
19. **MAT-21**: PERF-27 is named as the producer, and the test now has numbers.
20. **Scope, targets, milestones, open questions**: §0.1 items 4–6 and T3 were updated. M1 no longer claims a CELL test; M3 is re-ordered (MAT-13 first, then the dependencies on FLOW-06/EXT); M4 and M5 were updated. Open questions 11 (FLOW-06 vs CELL-12 tier encoding) and 12 (`MatchKind`/`Family` ownership) were added, and the tie-rule "Unresolved" entry is marked resolved.
21. **Missing-item scan**: every AR/AC/AP audit entry and every MM/CC/H08/H13 reference entry not cited here was checked against the other plans. The ones in MAT's scope that are not planned (CC-43, H08-42, H13-40 rules 2–4) are recorded in the appendix with reasons. No new work item was needed; the uncited remainder belongs to RTE, REL, CELL, EXT or PERF.

Order check: P0 items MAT-01 → {02, 03, 04, 05}, 04 → 06 come first and fix B1–B8. The dependency graph is acyclic: 01 → {02, 03, 04, 05, 11, 15}; 04 → {06, 07, 08, 09, 10, 13, 14, 16, 21}; 05 → {07, 12}; 07 → {08, 09, 10}; 02 → {12, 18}; 03 → 15; 09 → {14 step 2, 21}; 11 → 18.
