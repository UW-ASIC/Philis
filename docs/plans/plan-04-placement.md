# Plan 04: Placement (`backend/gp`, `backend/dp`, placement rules)

Plan ID prefix: **PLC-NN**. Working tree as of 2026-09-28 (HEAD `dad330c` plus uncommitted changes). Code cites are `path:line` in that tree. Reference cites name the source, the section/equation, and the reftext line range in `scratchpad/reftext/*.txt` (PDF page where the study docs verified it). Inputs: `docs/plans/audit-04-placement.md` (AP-NN), `audit-02-analog-rules-and-core.md` (AR-NN), `audit-07-flow-frontend-docs.md` (AF-NN, PL-NN), the reference studies (BAL1-, BAL2-, LAMP-, H08-, H13-, H15-, SUB-, SURV-, NOTES-, MM-, CC-, EM-, H12-, H14-), and my own reading of the sources listed in §1.

Numbers labelled **policy** are plan choices, not source values; each has a measurement gate. Numbers labelled **derived** were computed here from cited quantities.

---

## 0. Scope and boundaries

### 0.1 This plan owns

- The placement **representation and search**: `backend/dp` (the anneal, move set, schedule, decoder, legality) and `backend/gp` (global placement, `Prices`, `net_weights`, `mechanics`).
- The **placement rules that are geometry, not matching physics**: `kernel/analog/src/placement/{symmetry.rs, proximity.rs, isolation.rs (placement side only), dti.rs, utilization.rs}`, plus the new placement rules this plan adds (`island.rs` PLC-12, `heat.rs` PLC-14, `perf.rs` PLC-16, `order.rs` PLC-25) and the live form of `environment.rs` (`LiveEnvironment`, PLC-29; the `Surroundings` metric itself is unchanged).
- The **placement call path in the flow**: `frontend/library/src/lib.rs` `place_rules` (529–538), `Flow::epoch` placement part (561–599), `CellSpace::new` halo and bbox handling (1195–1216). FLOW owns the epoch loop around these lines.
- Hierarchical symmetry islands, proximity clusters, cell-to-cell spacing (well and implant sharing, per-pair minimum distance), orientation and shape locks, lattice alignment (cut lattice and routing-lattice axes), routing-space reservation, the placement-tier performance term, the thermal placement hook, determinism and runtime of placement.

### 0.2 Consumed from other plans

| Plan | What PLC consumes |
|---|---|
| EXT | Symmetry groups and their `AxisId` (today one per top-level block, `backend/annotator/src/emit.rs:139`); compounds with axis direction and Mirror/Perfect kind (EXT-14, for PLC-20/21); the group tree (EXT-13, optional input to PLC-08 `Tree::build`); block hierarchy (`Problem::groups`, `backend/annotator/src/lib.rs:39-41`; the abutment table is `:42-45`); matching class per matched set (EXT-16); well groups (EXT-22); signal-flow and current-path orders (EXT-28, for PLC-25). |
| MAT | Matching rules and their metrics: `MatchingPair`, `CentroidGroup`, `ThermalGradient` evaluated on unit centroids (AR-01, AR-02), the shared offset budget (AR-04, LAMP-09), the `OrientationSet` check (MAT-05, the rule side of AP-04), `analog::matching::moments::mirror_allowed` (MAT-01, for PLC-21), `analog::matching::class::{MatchClass, mos_env}` and `Process::tier` (MAT-07, for PLC-13/14), their `touches` overrides (AR-17) and dimensionless costs (AR-08). |
| CELL | Variant spaces whose alternatives are index-compatible for identical devices (AP-03); drawn shapes of every alternative (PLC-07 profiles); the guard-ring halo function `post_cell::ring_halo` (`kernel/cells/src/post_cell.rs:291`); macro extents on 2·cut lattice in `Builder::finish` (CELL-03; PLC-02 covers injected macros and the halo). |
| REL | Kind-aware `Isolation` distances (REL-09) and the `SubstrateBalance` cost (REL-15), both plain placement rules the anneal consumes unchanged; ring roles and shareable victim rings (REL-07, for PLC-22); the well-merge predicate `post_cell::may_share_well` (REL-16, for PLC-07); thermal self term (REL-14). |
| RTE | `RouteStats::congestion: Vec<(Rect, f32)>` per region and `LatticeSpec { p0, strides, origin_multiple }` (RTE-25, for PLC-15/28); the axis contract `x_axis ≡ p0/2 (mod p0·max(1, s_max/2))` that exact mirrored routing needs (RTE-15 step 3, for PLC-28); mirror routing that consumes the pair orientation mode (PLC-21). |
| PERF | Sensitivity rows per spec (ground C today, R and coupling later, PERF-12), bench harness and hand-layout references (PERF-20/21, AV-03), the post-layout spec-miss tier, net-weight feedback from the limiting spec (PERF-25, which replaced PLC's own C-feedback, see appendix). |
| FLOW | `Config` plumbing, deck/sidecar key loading, stable rule IDs for price keys (NOTES-04, AR-18); **one dual step per epoch and λ relaxation (FLOW-03)**; the warm/cold epoch schedule and `Layout: Clone` (FLOW-08, which calls PLC-10's `dp::Start`); per-stage timers `RunStats.stage_ms` and the dp seed salt (FLOW-09 steps 4–5); SPICE ports (AF-02, FLOW-07) for boundary constraints. |

### 0.3 Provided to other plans

- To RTE: lattice-exact placements, exact symmetry axes per group in `Layout::axis` on the routing lattice (PLC-28), the pair orientation mode (Perfect/Mirror, PLC-21), per-cell routing halos (PLC-15), the island hierarchy (`sp::Tree`) for channel planning.
- To MAT: exact translation/mirror relations between matched cells, joint orientation and variant of matched sets (by construction, PLC-03), unit data present in every placement stage (gp included, PLC-06).
- To REL: ring reservation at island level (PLC-22); REL's isolation and substrate-balance rules act through the ordinary rule seam.
- To PERF: placement metrics (area usage, island count, lattice offenders, clearance residue, dp proposal counts), A/B switches (`gp_mode`, `dp_mode`, perf row on/off).
- To FLOW: the `dp::Start` / `dp::Schedule` API and a carried `Tree` for FLOW-08's warm epochs (PLC-10); gp/dp that only `bind` prices (FLOW-03 deletes the two `settle` calls).

---

## 1. Audit digest (current state, verified by reading the code)

### 1.1 What runs today

1. `Flow::epoch` realizes the assignment and calls `gp::place` (`frontend/library/src/lib.rs:575`), discards gp's `Report`, overwrites `coarse.groups` with the **abutment** table (`:579`), resizes `axis` to the block count only if shorter (`:580-583`), then sets power and units (`:584-585`) and calls `dp::place` with the full variant space on odd epochs only (`:356`, `:587-597`). gp and dp get the **same seed** (`:575`, `:596`; seed derivation `:352`).
2. **gp** (`backend/gp/src/lib.rs:181-328`) is momentum gradient descent from a random pile at the die centre (`backend/gp/src/mechanics.rs:361-371`), with an extreme-pin HPWL subgradient (`lib.rs:226-239`), a central finite-difference analog gradient at 64 nm (`:176`, `:242-259`, 4n full cost evaluations per iteration), a whole-cell-in-centre-bin density push (`:262-290`), and a hard-Φ rollback (`:306-312`). It sets `axis = vec![side/2; n]` (n = cells, not blocks) (`mechanics.rs:378`), `power_uw = 0` (`:382`), empty `units` (`:384`), and never reads `Rules::clearance`. It calls `prices.settle` (`lib.rs:325`). **No test calls `gp::place`** (tests at `:339-486` cover `Prices` and `net_weights` only).
3. **dp** (`backend/dp/src/lib.rs:192-367`) is Metropolis SA over absolute coordinates: 220 temperatures × 60·n proposals (`:21-23`, `:293`), α = 0.93 (`:24`), window decay (`:26-27`). Moves: compound symmetric moves (`:310-325`, `:473-532`), displacement (`:326-329`), swap (`:330-332`), DTI flip (`:333-335`), reshape of **one cell** (`:336-337`, `:584-620`), quarter turn if `rotatable` (`:338-339`, `:555-578`). Every proposal clones 8 columns (`:36-77`, `:147`), evaluates all hard/budget/cost batches and full HPWL twice (`:147-179`), projects every violated hard batch (`:154-159`), gates on `(Φ count, Φ residual + encroachment nm², Θ)` (`:177-178`) with Metropolis only on exact ties (`:399-407`). After the anneal: snap centres (`:352-358`), greedy pairwise-push legalizer (`legalize.rs:17-90`, return value ignored at `dp/lib.rs:360`), a second `prices.settle` (`:364`).
4. **Spacing**: one scalar for all cell pairs, the max `min_spacing` over `nwell, diff, tap, poly, nsdm, psdm, li` (`frontend/library/src/lib.rs:529-538`), which on sky130 is `nwell.2a space(nwell) >= 1270nm` (GPurify `sky130.deck` line 228) while `difftap.3 space(difftap) >= 270nm` (line 266), `difftap.9 space(ndiff, nwell) >= 340nm` (line 269), `difftap.11 space(ptap, nwell) >= 130nm` (line 271). Wells are bridged only after placement when spans match exactly (`kernel/cells/src/post_cell.rs:378-437`).
5. **Symmetry**: one `SymmetryGroup` per top-level block, vertical axis, centre-only equality `x_a + x_b = 2·axis, y_a = y_b` (`kernel/analog/src/placement/symmetry.rs:7-16`, `:62-68`), repaired by projection to the mean of midpoints (`:133-149`), residual = violated-pair count (the `RuleBatch::residual` default, `kernel/analog/src/rule.rs:141-143`, which `SymmetryGroup` does not override, `symmetry.rs:103-150`). No horizontal axis, no hierarchy, no island property, no partner geometry check.
6. **Lattice**: builder rounds bbox extents to `2·grid` = 10 nm (`kernel/cells/src/builder.rs:113-133`), so `hw = w/2` (`mechanics.rs:321-323`) is a multiple of 5 nm; the stamp origin is `x − hw − anchor.x` (`kernel/core/src/macro.rs:48`); dp snaps centres to 10 nm (`dp/lib.rs:355-358`), so any cell whose width/10 nm is odd stamps 5 nm off the cut lattice. Ring halos are rounded to `pdk.grid` (5 nm) and baked into every alternative's bbox (`frontend/library/src/lib.rs:1195-1208`).

### 1.2 Bugs and placeholder logic to fix first (ordered by severity)

| ID (source) | Defect | Evidence |
|---|---|---|
| AP-04, AR-36 | `rotatable` reads the abutment table, which the annotator truncates to one member for mixed-polarity blocks, so the two halves of a diff pair drawn as separate cells can quarter-turn independently; conversely a same-polarity glue block locks every glue cell. | `backend/dp/src/lib.rs:558-560`; `frontend/library/src/lib.rs:579`; `backend/annotator/src/lib.rs:106-116` |
| AP-03, AR-11 | Reshape changes one cell; mirror partners get different variants; `Symmetry` checks centres only. The test `cells_reshape_independently` asserts the defect. | `dp/lib.rs:584-620`; `symmetry.rs:25-27`; `backend/dp/src/tests.rs:143-151` |
| AP-05, AP-23 | Origins land 5 nm off the 10 nm cut lattice for odd-10 widths; halo rounded to 5 nm. | §1.1 item 6 |
| AP-06, AF-07 | Two dual steps per epoch on different layouts (gp pile, then dp result); ρ-doubling compares gp vs dp residuals. **Fixed by FLOW-03** (deletes both `settle` calls, settles once in `Flow::epoch`); PLC only keeps `bind`. | `gp/lib.rs:83-101`, `:325`; `dp/lib.rs:364` |
| AP-07 | Gate adds a dimensionless count (SymmetryGroup residual) to encroachment in nm²; breaking a pair can be bought with 2 nm² less overlap while the batch is already violated. | `dp/lib.rs:177-178`; `rule.rs:141-143` |
| AP-14 | Legalizer may stop with overlap; return ignored; `report` counts raw overlap only; `debug_check_placed` panics debug builds on raw overlap. | `legalize.rs:78-87`; `dp/lib.rs:360`; `mechanics.rs:310-313`; `lib.rs:598`; `kernel/core/src/layout.rs:152-164` `debug_check_placed` |
| AP-10, AR-42, AR-15 | gp axis table sized by cell count, all axes pinned at die centre; power and units absent during gp. | `mechanics.rs:378`, `:382`, `:384`; `lib.rs:580-585` |
| AP-11 | Fixed (injected) cells are piled at random by gp then frozen; two overlapping fixed cells never separate. | `dp/lib.rs:160-163`, `:383-388`, `:465`; `legalize.rs:50` |
| AP-19 | gp and dp RNG streams start identical (**fixed by FLOW-09 step 5**, dp seed salt); T0 from ΔPEX dominated by the largest-unit term (PLC-18, PLC-09). | `lib.rs:575`, `:596`; `dp/lib.rs:276-289`; `kernel/analog/src/placement/thermal.rs:25` |
| AR-38 | Symmetric projection rounds the half-separation toward zero, shrinking an odd-parity pair by one grid. | `symmetry.rs:84`, `:152-162` |
| AP-02 | One layer-agnostic clearance (1270 nm) for every pair; `min_utilization = 0.6` (`lib.rs:76`) can be geometrically unreachable (audit AP-02: a 2×2 µm cell pitched at 3.27 µm fills 37%). | `lib.rs:529-538` |
| AP-01 | No representation holds symmetry or legality; projection + greedy push. | `dp/lib.rs:1-8`, `:140-184`, `:301-350`; `legalize.rs:17-90` |
| AP-08, AP-21 | 13,200·n full re-evaluations per dp call; per-trial 8-column clone; no incremental HPWL although `cell_nets` exists (`dp/lib.rs:82`, `:103`). | `dp/lib.rs:120-184`, `:21-23` |
| AP-09, AF-06 | Every epoch cold-restarts gp from a random pile. | `lib.rs:574-575`; `mechanics.rs:361-371` |
| AP-13 | Any Θ change decides outright (greedy on Θ); budgets also counted in PEX through λ. | `dp/lib.rs:399-407`, `:120-122` |
| AP-16, AP-22 | gp numerics weak and unmeasured; no gp test; dp rotation tests run with no nets (PEX ≡ 0). | `gp/lib.rs:226-323`; `dp/src/tests.rs:54-87` |
| AP-17, AP-18 | No routing-space reservation, no congestion or parasitic feedback; net weights fixed per run; rails weight 1. | `lib.rs:293-296`; `gp/lib.rs:137-153` |
| AR-17 | Placement rules other than `Symmetry` have no `touches`; `SymmetryGroup` does not forward `touched`. | `proximity.rs`, `isolation.rs`, `dti.rs`, `thermal.rs`, `matching_pair.rs`, `cc.rs` (no `fn touches`); `symmetry.rs:103-131` |
| AR-16 | `DtiBand` share side uses strict `<` (and the isolate side strict `>`); latent (no shipped deck has DTI: `"dti": null`, `pdks/sky130.json:46`). Fixed in PLC-04 step 6. | `kernel/analog/src/placement/dti.rs:56` |
| (new) | `CentroidGroup` pools a whole stage (AR-02), so deriving shape locks from its `a_side × b_side` would join devices of different sizes; PLC-03 therefore shape-locks only index-compatible pairs. | `kernel/analog/src/placement/cc.rs:28-44`; `backend/annotator/src/emit.rs:196-197` |
| (new) | `implant_bridges` and `well_bridges` bridge only facing rects with **identical** spans, so a same-type implant or same-net well gap inside `(0, space)` with unequal spans stays a DRC violation; any spacing table that sets same-type implant spacing to 0 is unsafe. | `kernel/cells/src/post_cell.rs:424-427`, `:625-628` |

### 1.3 What already works and is kept

SplitMix64 determinism (`mechanics.rs:11-56`); pin-level CSR nets with per-net weights (`mechanics.rs:62-197`); the `Rule`/`RuleBatch` seam with `mirror_pairs`, `branches`, `project`, `known`/`applicable` (`rule.rs:9-208`); the lexicographic epoch key `(|V|, spec miss, Θ, C, footprint)` (`lib.rs:916-958`); live thermal field per trial (`kernel/core/src/thermal.rs`); variant reshape priced on real pin offsets (`Nets::reshape_cell`, `mechanics.rs:165-184`); DTI branch as a search variable (`dp/lib.rs:222-238`, `:537-539`); multi-start (`lib.rs:185-189`).

---

## 2. Target: what "better than hand layout" means for placement

Hand layout references: Hastings gives a cell packing factor `A_cell ≅ P_f·ΣA` with P_f = 1.2–1.8 for two-metal CMOS (Eq 15.1, §15.1.2, hastings.txt L46538–46554, PDF 784); LAYLA measured a 10–15 % area penalty against manual layout, blamed on separated P&R and overestimated routing space (Lampaert §7, lampaert.txt L7094–7114, PDF 171–172); Plantage reports best area usage 110–129 % over five circuits (Table 3.2, balasa_graeb_survey.txt L6602–6673), and for the folded-cascode example says area usage is "always above 121 %, because of empty areas caused by the minimum distance constraints between nMOS and pMOS transistors" (L6780–6783); in the two-circuit comparison the symmetry-island placer reaches 104.68 % and 105.72 % (Table 3.4, L7417–7729).

| # | Metric (reported by PLC-01) | Definition | Target (all bench fixtures × seeds 1–5 unless stated) |
|---|---|---|---|
| T1 | Placement legality | raw overlap nm², pairwise-table clearance encroachment nm², lattice offenders, symmetry violations, matched-set (variant, orient) mismatches | all **0** |
| T2 | Placement area usage | `footprint_nm2 / Σ cell bbox area` (Plantage metric, L6666–6668: bounding rectangle over the area "used by modules and wells"; Philis cell bboxes already contain each cell's own nwell) | median ≤ **1.30**, stretch ≤ 1.15 (**policy**; published: Plantage Table 3.2 1.10–1.29, Table 3.4 placers 1.05–1.19, L6654, L7717–7724) |
| T3 | Final footprint vs hand layout | final block bbox / hand GDS bbox, for circuits where PERF provides a hand reference (PERF-21, AV-03) | ≤ **1.00** (LAYLA's 1.10–1.15 is the historical automated result) |
| T4 | Island integrity | share of symmetry groups whose members form one connected island (Def. 2.1, L3187–3193) | ≥ **90 %** |
| T5 | Performance-driven placement | post-layout spec miss (lex key position 2) with PLC-16 on vs off (LAMP-33 protocol, L5020–5082) | on ≤ off on every fixture with specs; strictly lower on ≥ 1 |
| T6 | Routability | `route_hard + route_overuse` of the winner (`RunStats`, `lib.rs:105-124`) | ≤ baseline (PLC-01) on every fixture |
| T7 | Runtime | dp proposals/second (`PlaceStats::proposals` / FLOW-09's dp `stage_ms`) and total bench wall time | proposals/s ≥ **3×** baseline; bench wall ≤ baseline (audit-06 §G.3 measured 539.5 s for `bench local`, seed 1, `FEEDBACK_ITERS = 5`) |
| T8 | Determinism | same seed and inputs → identical `Layout` and GDS bytes, independent of `starts` thread scheduling | exact |
| T9 | Matching environment | Environment violations of matched pairs (usage > 1, `kernel/analog/src/placement/environment.rs:59-61`; usage is the worst of WPE nearness, WPE skew and OSE skew, `:33-47`), from the post-route report with rings (`lib.rs:431`) | **0** (PLC-13 nearness, PLC-29 skew) |

---

## 3. Work items

### PLC-01 Placement instrumentation and baseline
- Priority: P0. Effort: S. Depends on: none.
- Why: AP-16 (gp contribution unmeasured), AP-22, AP-08 (no timing), BAL1-24 (compare representations at equal wall time, L2812–2817), BAL1-62 (area-usage metric), LAMP-46 (per-stage CPU table, L6702–6803).
- Current: `RunStats` holds iteration counters, a convergence flag, per-stage hard-violation counts and routing overuse, and no timings or area metrics (`frontend/library/src/lib.rs:105-124`); no timer anywhere in gp/dp (audit-04 §1.10); gp's `Report` is discarded (`lib.rs:575`). Wall-clock stage timers are FLOW-09 step 4 (`RunStats.stage_ms: [u64; 9]`, gp = index 0, dp = index 1); this item adds only what FLOW cannot measure from outside dp.
- Change:
  - `backend/dp/src/lib.rs`: `#[derive(Clone, Copy, Debug, Default)] pub struct PlaceStats { pub temps: u32, pub proposals: u64, pub accepted: u64, pub decode_fail: u64, pub matched_incompatible: u32 }` (counters only; `decode_fail` stays 0 until PLC-08, `matched_incompatible` until PLC-03). Count `proposals` at every `sa.trial` call and `accepted` on `true`. `dp::place` returns `(Layout, Report, PlaceStats)`; the two callers are `frontend/library/src/lib.rs:587` and `backend/dp/src/tests.rs:42`.
  - `frontend/library/src/geometry.rs` (private module; re-export with `pub use geometry::PlacementMetrics;` in `lib.rs` because the public `RunStats` holds it):
    ```rust
    #[derive(Clone, Copy, Debug, Default)]
    pub struct PlacementMetrics { pub area_usage: f32, pub lattice_off: u32, pub clearance_residue_nm2: f64,
                                  pub overlap_nm2: f64, pub matched_geometry_mismatch: u32, pub islands_extra: u32 }
    pub fn placement_metrics(macros: &[Macro], l: &Layout, lattice: i32, clearance: i32,
                             reqs: &analog::Requirements<Layout>) -> PlacementMetrics
    ```
    - `area_usage = l.footprint_nm2() / Σ_i 4·hw_i·hh_i` (cell bboxes include ring halos, like Plantage's "modules and wells");
    - `lattice_off` = count of `i` with `place_macro(&macros[i], l, i).bbox.x.rem_euclid(lattice) != 0 || .bbox.y.rem_euclid(lattice) != 0`;
    - `overlap_nm2 = gp::mechanics::encroachment(l, 0)`; `clearance_residue_nm2 = encroachment(l, clearance) − overlap_nm2` (the PLC-07 table version replaces `clearance` later);
    - `matched_geometry_mismatch` = count of pairs `(a, b)`, `a ≠ b`, from `RuleBatch::mirror_pairs` over `reqs.hard` (PLC-03 switches this to `matched_pairs`) with `(l.variant, l.orient, l.hw, l.hh)` differing at `a` and `b`;
    - `islands_extra = 0` until PLC-12 fills it.
  - `RunStats` (`Copy` stays valid: both new types are `Copy`) gains `pub place: PlacementMetrics` and `pub dp: PlaceStats`, set in `Flow::epoch` right after the post-dp `macros` rebinding (`lib.rs:600-604`: when dp reshaped, `macros` is re-realized at `layout.variant`, and `lattice_off` must stamp those macros, not the pre-dp ones) with `lattice = cells::builder::cut_lattice(self.pdk)` and `clearance = place_rules(self.pdk).clearance`. `RunStats::merge` (`lib.rs:899-908`) must copy `place` and `dp` from the winner like `place_hard`, or they are lost.
  - `Config` gains `pub gp_mode: GpMode` with `#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)] pub enum GpMode { #[default] Analytic, Pile }`: `Analytic` = today; `Pile` = `gp::place` returns right after `initial_layout` (skip the loop at `gp/lib.rs:220-323`). Plumbing: `gp::place` gains a trailing `iterate: bool` argument (`false` = return after `initial_layout`, still computing the report); PLC-06 folds it into `GpInput::iterate`. PLC-08 adds a third variant `Constructive`.
  - PERF prints the new fields in the bench table (PERF-20 owns the table format).
- Tests: `frontend/library/tests/placement_metrics.rs` (new):
  - `metrics_are_populated_on_ota`: `benchmarks/fixtures/ota.spice`, sky130, `Config { starts: 1, feedback_iters: 2, outer_iters: 1, ..Default::default() }`: assert `stats.dp.proposals > 0`, `stats.place.area_usage >= 1.0`, `stats.place.overlap_nm2.is_finite() && stats.place.overlap_nm2 >= 0.0` (a metrics item asserts that the numbers exist, not that the baseline is legal: AP-14's legalizer can leave overlap today; `overlap_nm2 == 0` becomes an assertion in PLC-09's acceptance).
  - `gp_modes_are_deterministic`: `ota.spice`, each `GpMode`, same seed twice → identical `layout.x`, `layout.y`, `layout.variant`.
- Acceptance: the bench table carries the fields for all 10 fixtures in `benchmarks/fixtures/` (bgr_core, bjt_mirror, chain4, dac4, ota, ota_constrained, pair, quad, rc_filter, tt_ota); the numbers recorded on the day PLC-01 merges are the **baseline** for T2, T6, T7.
- Risks / notes: counters never enter a decision (determinism, T8).

### PLC-02 Cut-lattice-aligned placement boxes
- Priority: P0. Effort: S. Depends on: PLC-01 (for the metric). Coordinates with CELL-03, which rounds generated macros in `Builder::finish` and edits the same `lib.rs:1198` line; whichever lands first makes the `lib.rs:1198` change, and `align_bbox` is then a no-op on generated cells but still required for injected macros and halo-grown bboxes.
- Why: AP-05, AP-23, AR-25; H01-26 grid discipline (Hastings §3.1.1–3.1.3, hastings.txt L6534–6551, L6720–6726, PDF 121, 124); H15-21 grid consistency (hastings.txt L48285–48313, PDF 814).
- Current: extents are multiples of 10 nm (`builder.rs:113-133`), `hw` a multiple of 5 nm (`mechanics.rs:321-323`), origin `x − hw − anchor.x` (`macro.rs:48`), centres snapped to 10 nm (`dp/lib.rs:355-358`); ring halo rounded to `pdk.grid` = 5 nm (`lib.rs:1198`). Injected macros are unchecked.
- Change:
  1. `kernel/core/src/macro.rs`: 
     ```rust
     /// Grow `bbox` so its lower-left corner is on `lattice` and both extents are
     /// multiples of `2·lattice`: then `hw`, `hh` and every placed centre are
     /// lattice multiples. Shapes and pins are unchanged.
     pub fn align_bbox(&mut self, lattice: i32) {
         let l = lattice.max(1);
         let (x0, y0) = (self.bbox.x.div_euclid(l) * l, self.bbox.y.div_euclid(l) * l);
         let (x1, y1) = (self.bbox.x + self.bbox.w, self.bbox.y + self.bbox.h);
         let up = |d: i32| (d + 2 * l - 1).div_euclid(2 * l) * 2 * l;
         self.bbox = Rect { x: x0, y: y0, w: up(x1 - x0), h: up(y1 - y0) };
     }
     ```
  2. `frontend/library/src/lib.rs` `CellSpace::new`: halo `ext = round_up(ring_halo(..), cells::builder::cut_lattice(pdk))` (replaces `pdk.grid.max(1)` at `:1198`); then `for m in cells.variants.iter_mut().flat_map(|s| s.alternatives.iter_mut()) { m.align_bbox(cells::builder::cut_lattice(pdk)) }` after the halo loop (`:1197-1208`) and **before** the `per_cell` frames and `UnitLib::build` (`:1209-1216`), so unit frames use the aligned bbox. `cellgen::realize` reads `cells.variants` at every use (`lib.rs:402`, `:574`, `:603`, `:721`, `:765`, `:809`), so gp, dp, rings, routing and the winner's GDS all see the aligned boxes. This covers generated and injected cells.
  3. In `Flow::epoch` after the post-dp `macros` rebinding (`lib.rs:600-604`, where PLC-01 computes the metrics): `debug_assert_eq!(place.lattice_off, 0, "dp::place: cell origin off the cut lattice")` on the `PlacementMetrics` value PLC-01 computes there (no second helper).
  4. Why this suffices (**derived**): `place_macro` puts the placed bbox corner at `x − hw` (`kernel/core/src/macro.rs:48-54`, `anchor` cancels), and every shape offset from the corner is a lattice multiple under all eight `Orient`s (negation preserves multiples). With `w ≡ 0 (mod 2·lattice)`, `hw ≡ 0 (mod lattice)`, so dp's final snap of `x` to `grid = cut_lattice` (`dp/lib.rs:355-358`) puts every corner on the lattice. The guard ring is still drawn inside the reservation: `guard_rings` shrinks the placed bbox by the unrounded `ring_halo` (`kernel/cells/src/post_cell.rs:56-64`), so rounding and right/top padding only widen the ring's inner clearance.
- Tests:
  - `kernel/core/src/macro.rs` `align_bbox_puts_corner_on_lattice_and_even_extents`: bbox `(5, 3, 95, 41)`, lattice 10 → `(0, 0, 100, 60)`; bbox `(10, 20, 30, 30)` → `(10, 20, 40, 40)`.
  - `kernel/core/src/macro.rs` `align_bbox_is_idempotent`: applying it twice to `(5, 3, 95, 41)` gives `(0, 0, 100, 60)` both times.
  - `frontend/library/tests/placement_metrics.rs` `every_origin_is_on_the_cut_lattice`: fixtures `ota, dac4, rc_filter, chain4, quad` on sky130, `starts = 1`, `feedback_iters = 2`, seeds 1–3 → `stats.place.lattice_off == 0`.
- Acceptance: T1 lattice offenders 0; signoff DRC count per fixture ≤ baseline.
- Risks / notes: for generated cells the padding adds 0 or 10 nm on the right/top (**derived**: `Builder::finish` already puts the corner and extents on multiples of 10 nm, `builder.rs:125-133`, so `x0` is unchanged and `up()` adds at most one 10 nm step); an injected macro can grow by < `lattice` on the left/bottom and < 2·`lattice` on the right/top. The rounding lives in the flow, not in `Builder::finish`, so CELL's self-check tests (`kernel/cells/tests/cell_selfcheck.rs`) are unaffected.

### PLC-03 Matched-set orientation and shape locks
- Priority: P0. Effort: M. Depends on: none (MAT's `matched_pairs` override is a two-line addition this item makes).
- Why: AP-03, AP-04, AR-36, AR-11, PL-4 (audit-07: `VariantSpace` has no per-device lock, `backend/gp/src/lib.rs:18-22`); Hastings §13.2.1 orientation (hastings.txt L40966–41017, PDF 690–691) and rule 7 (L42490–42498, PDF 714); H08-16 (L22351–22354); MM-10 (rotation degrades β, pelgrom.txt L98–99); BAL1-43 same-variant constraint (L4677–4683, L6629–6630); LAMP-12 orientation and shape groups (lampaert.txt L4348–4395, PDF 104–105). MAT-05 adds the *check* (`OrientationSet{Axis}` hard); this item makes the moves respect it by construction, so the check never fires in dp.
- Current: see §1.2 rows AP-04 and AP-03.
- Change:
  1. `kernel/analog/src/rule.rs`: add to `Rule` (after `mirror_pair`, `:94-99`) `fn matched_pair(self) -> Option<(u32, u32)> { None }`, to `RuleBatch` (after `mirror_pairs`, `:196-199`) `fn matched_pairs(&self, out: &mut Vec<(u32, u32)>) { let _ = out; }`, and to the blanket `impl RuleBatch for Vec<R>` (`:210-277`) `fn matched_pairs(&self, out) { out.extend(self.iter().filter_map(|r| r.matched_pair())); }`. Overrides: `Symmetry` (both `Target::Device`, a≠b), `MatchingPair` (`matching_pair.rs:28-37`, both `Target::Device`, a≠b), `CentroidGroup` (every `a_side × b_side` pair mapped through `cell_of` when non-empty, `cc.rs:28-44`; it is a hand-written `RuleBatch`, `cc.rs:152`, so it implements `matched_pairs` directly). `SymmetryGroup` is also a hand-written `RuleBatch` (`symmetry.rs:103`) and must forward `matched_pairs` to its `Vec<Symmetry>` the way it forwards `mirror_pairs` (`symmetry.rs:129-131`). When MAT-04/05 land (`MatchedSet`, `OrientationSet`), they implement the same hook; MAT owns those rules, this item adds the overrides on today's rules.
  2. `backend/dp/src/locks.rs` (new, `pub mod locks;` in `dp/lib.rs`):
     ```rust
     #[derive(Clone, Debug, Default)]
     pub struct Locks {
         /// Cells that rotate together (same quarter turn). Singleton sets omitted.
         pub orient: Vec<Vec<u16>>,
         /// Cells that reshape together (same variant index). Singleton sets omitted.
         pub shape: Vec<Vec<u16>>,
         pub orient_of: Vec<Option<u16>>, // cell -> index into `orient`
         pub shape_of: Vec<Option<u16>>,  // cell -> index into `shape`
         /// Matched pairs whose variant spaces are not index-compatible (not shape-locked).
         pub incompatible: u32,
     }
     pub fn locks(reqs: &Requirements<Layout>, n: usize, variants: &[gp::VariantSpace]) -> Locks;
     impl Locks {
         /// Set every shape set's members to its first member's variant (assignment unification).
         pub fn unify(&self, assignment: &mut [u16]);
     }
     ```
     Algorithm: (1) collect `matched_pairs` from `reqs.hard`, `reqs.budget`, `reqs.cost`; drop pairs with `a == b` or an index `>= n`; (2) orient sets: `pnr_core::UnionFind::new(n)` over **all** pairs; (3) shape sets: a second `UnionFind` over the pairs whose two cells are **index-compatible** (same `alternatives.len()` and, for every k, equal `(bbox.w, bbox.h)` of alternative k; with `variants` empty every pair is compatible), the rest counted in `incompatible` (reported as `PlaceStats::matched_incompatible`). Per-pair compatibility matters because `CentroidGroup` pools a whole stage (AR-02): an all-or-nothing set check would freeze the input pair together with its differently sized load pair.
  3. Assignment unification: `solve()` computes `let locks = dp::locks::locks(&problem.placement, cells.variants.len(), &cells.variants)` once after `CellSpace::new` (`lib.rs:275`) and stores it in `Flow` (new field `locks: dp::locks::Locks`); `Flow::epoch` starts with `let assignment = { let mut a = assignment.to_vec(); self.locks.unify(&mut a); a };` before `cellgen::realize` (`lib.rs:574`). Without this, `cellgen::seed_assignment`/`escalate` can give partners different variants and step 6 would make every such epoch infeasible. (FLOW-08's `escalate_blamed` should skip non-first members of a shape set; noted to FLOW.)
  4. `backend/dp/src/lib.rs`: delete `rotatable` (`:555-560`); `try_rotate` becomes `try_rotate_set(set: &[u16])` applying the same `quarter_turn` to every member (swap `hw/hh`, clamp) in one `Sa::trial`; a cell with `orient_of == None` rotates alone as today. `try_reshape` draws `next` once and applies it to every member of the cell's shape set in one trial (patching each member's pins with `Nets::reshape_cell`; on refusal, restore each member's pins as `:614-618` does for one). `Layout::groups` is no longer read by dp (`dp::place` still copies it through, `:214`).
  5. `gp::Rules` gains `pub rotate_90: bool` from sidecar `cell.rotate_90_allowed` (default `true`, read in `place_rules`, `lib.rs:529-538`, as `pdk.rule("rotate_90_allowed", 1) != 0`; `parse_roles` already turns a boolean sidecar value into 0/1, `backend/verify/src/pdk.rs:1263`); when false, `quarter_turn` is replaced by a half turn (R0↔R180, R90↔R270, Mx↔Mx180, Mx90↔Mx270) for every cell (H12-26: with directional tilted pocket implants a block may be reflected or rotated by 180° but not by 90°, Hastings §12.2.7, hastings.txt L36480–36487, PDF 611 (printed p.610); the angles are blank in the reftext and were read from the PDF by the H12 study). No shipped deck sets the key, so behaviour is unchanged until a deck opts in.
  6. `kernel/analog/src/placement/symmetry.rs`: `Symmetry::satisfied` additionally requires `hw_a == hw_b && hh_a == hh_b && variant_a == variant_b && orient_a == orient_b` for device targets a≠b (Perfect symmetry; Mirror mode is PLC-21); a short `variant`/`orient` column reads as equal; `error` unchanged; violations count as hard.
- Tests (`backend/dp/src/tests.rs` unless noted):
  - `mirror_partners_reshape_together`: two cells in one `SymmetryGroup` pair, each `VariantSpace` with three alternatives of identical dims `(1000,4000)`, `(2000,2000)`, `(4000,1000)`; seeds 1..=20 → `l.variant[0] == l.variant[1]` always, and `l.variant[0] != 0` for at least one seed.
  - `mixed_polarity_composite_halves_turn_together`: `coarse.groups = [[0], [2]]` (truncated abutment) and a `MatchingPair` (0,1) in `reqs.cost` only (so the lock provably comes from `matched_pairs`, not from a hard rule); seeds 1..=20 → `l.orient[0] == l.orient[1]` always. (MAT-05 lists the same scenario as `matched_halves_never_turn_apart`; one test, this name.)
  - `locks_join_only_compatible_shapes` (`locks.rs`): pairs (0,1), (0,2); cells 0 and 1 have identical spaces, cell 2 a different one → `orient = [[0,1,2]]`, `shape = [[0,1]]`, `incompatible == 1`.
  - Rename `cells_reshape_independently` (`:143-151`) to `unmatched_cells_reshape_independently` with no pair rules.
  - `matched_devices_never_turn` (`:54-59`) calls `rotatable`, which step 4 deletes: replace it with `matched_cells_share_one_orient_set` (a `Symmetry` pair (0,1) → `locks(..).orient_of[0] == orient_of[1]`, `orient_of[2] == None`).
  - `extents_stay_in_lockstep_with_orientation` (`:61-71`) asserts `(orient[0], orient[1]) == (R0, R0)` at `:70` because of the group lock `rotatable` read; after step 4 add a `Symmetry` pair (0,1) to its requirements and assert `l.orient[0] == l.orient[1]` instead.
  - `kernel/analog/src/placement/symmetry.rs` `unequal_variants_are_not_symmetric`: mirrored centres, `variant = [0, 1]` → `satisfied == false`.
- Acceptance: T1 `matched_geometry_mismatch == 0` on every fixture/seed; `PlaceStats::matched_incompatible` printed per fixture.
- Risks / notes: an incompatible pair that is also a `Symmetry` pair stays hard-violated (a genuine EXT error, AA-05); the count makes it visible.

### PLC-04 Placement report and gate correctness
- Priority: P0. Effort: S. Depends on: none.
- Why: AP-07, AP-14, AR-38, AR-16, PL-6 (Φ-monotone acceptance, audit-07 §2.14); NOTES-35 (gate on counts/residuals separately, topic-search-algorithms L32–40). (AP-19's seed half is FLOW-09 step 5; not repeated here.)
- Current: gate tuple `(Φ.0, Φ.1 + encroach, Θ)` (`dp/lib.rs:177-178`); `report` adds only raw overlap (`mechanics.rs:310-313`); legalizer residual ignored (`dp/lib.rs:360`); `debug_check_placed` panics (`lib.rs:598`); projection shrinks odd pairs (`symmetry.rs:84`); `DtiBand` interval ends are strict (`dti.rs:56`).
- Change:
  1. `accept` (`dp/lib.rs:399-407`) takes `(usize, f64, f64, f64)` = `(hard count, Σ hard residual, encroach_nm2 / area_ref, Θ)`; `area_ref = Σ_i 4·hw_i·hh_i` (f64, floor 1.0) computed once per `place` and stored in `Sa`; `trial` builds `before = (phi0.0, phi0.1, ov0 / area_ref, theta0)` and `after` likewise (`:177-178`).
  2. Give `Symmetry` a measured residual so the hard margin is a length, not a count: override `Rule::residual` in `impl Rule for Symmetry` (`symmetry.rs:17-60`) as `let (ex, ey) = self.error(l); (ex.abs() + ey.abs()) as f32 / 1000.0` (µm of mirror error), and add `fn residual(&self, l) -> f64 { self.0.residual(l) }` to `impl RuleBatch for SymmetryGroup` (`:103-131`). The blanket `Vec<R>::residual` then applies the per-rule `FAIL_FLOOR` (`rule.rs:295-306`) itself, so nothing in `rule.rs` changes visibility. Update the `Symmetry` doc comment (`:7-9`, "residual stays at the default 0/1").
  3. `mechanics::report` (`mechanics.rs:290-316`) gains a `clearance: i32` argument (callers `gp/lib.rs:203`, `:326`; `dp/lib.rs:245`, `:365`) and pushes `Violation { rule: "clearance encroachment".into(), margin: (encroachment(l, clearance) − encroachment(l, 0)).ceil() as i64 }` when that difference is > 0.5 (clearance-only residue; the raw overlap keeps its own "device overlap" row, so nothing is counted twice). PLC-07 swaps the scalar for the table. The legalizer's return value (`dp/lib.rs:360`) stays unused: `report` measures the same quantity.
  4. `lib.rs:598`: replace `layout.debug_check_placed("dp::place")` with `layout.debug_check("dp::place")` (structural checks only). Overlap is now a reported hard row that costs the epoch in `lex_key`, not a debug panic; `debug_check_placed` stays for the legalizer tests (`legalize.rs:171`).
  5. `symmetry.rs::mirror_about` (`:82-90`): today `half = snap_to((snap_to(xb) − snap_to(xa)) / 2, g)` (`:84`); the integer `/ 2` truncates (25 / 2 = 12) and `snap_to` then rounds 12 to 10 (`:152-162`). Replace with `let d = snap_to(xb, g) − snap_to(xa, g); let half = d.signum() * ((d.abs() + 2 * g − 1) / (2 * g)) * g;` (ceiling of `|d|/2` to a multiple of `g`, sign kept), so a projected pair is never closer than before.
  6. `dti.rs:56`: `gap <= self.s_max_nm as f32 || gap >= self.d_dti_nm as f32` (AR-16; both interval ends closed so a pair exactly at an end is legal, matching `cost`, which is 0 there).
- Tests (`backend/dp/src/tests.rs`, `symmetry.rs`, `dti.rs`):
  - `gate_never_trades_a_broken_pair_for_overlap`: call `accept` directly with `before = (1, 0.002, e, 0.0)` and `after = (1, 0.003, e − 2.0 / area_ref, 0.0)` (a third pair broken, 2 nm² less overlap) → `false` at `temp = 1e12` for 100 RNG draws. The pre-fix gate accepts it: `(1, 2 + ov0)` vs `(1, 3 + ov0 − 2)`.
  - `symmetry_residual_is_the_mirror_error_in_um`: pair at x = −1000, 1500, axis 0, same y → `SymmetryGroup::residual == 0.5`.
  - `report_counts_clearance_only_residue`: two 1000×1000 nm cells side by side (same y) with edges 100 nm apart, clearance 270 → exactly one "clearance encroachment" row with `margin == 170 · 1270 = 215900` (`encroach` inflates both axes by the clearance, `mechanics.rs:227-228`) and no "device overlap" row.
  - `odd_parity_projection_never_shrinks_the_pair`: xa = 0, xb = 25, grid 5 → after `project` separation = 30.
  - `dti.rs` `a_pair_exactly_at_s_max_is_legal`: gap == `s_max_nm` → `satisfied`.
- Acceptance: `cargo test --workspace` in debug passes on every fixture flow test without a placement panic; T1 clearance residue reported (0 required only after PLC-07/08).

### PLC-05 One dual step per epoch — moved to FLOW-03
- Status: **cut from this plan** (see appendix). FLOW-03 steps 1–2 delete `prices.settle` at `gp/lib.rs:325` and `dp/lib.rs:364` and call it once in `Flow::epoch` after `dp::place`, with the test `place_does_not_settle`. PLC's only obligation: gp and dp keep `prices.bind(reqs)` (`gp/lib.rs:196`, `dp/lib.rs:241`) and never settle. The ID is kept so cross-references stay valid.

### PLC-06 gp sees block axes, power, units; fixed means shape-frozen
- Priority: P0. Effort: S. Depends on: PLC-01 (the `iterate` flag it folds in).
- Why: AP-10, AR-42, AR-15, AP-11; H08-47/H13-49 need power during global placement.
- Current: §1.2 rows AP-10 and AP-11.
- Change:
  1. `gp::place(inp: &GpInput, prices: &mut Prices, seed: u64) -> (Layout, Report)` (prices stay `&mut` because `bind` writes the weight column; FLOW-03 removes only `settle`) with
     ```rust
     pub struct GpInput<'a> {
         pub macros: &'a [Macro], pub variants: &'a [VariantSpace], pub assignment: &'a [u16],
         pub reqs: &'a Requirements<Layout>, pub rules: Rules, pub net_weight: &'a [f32],
         pub n_axes: usize, pub power_uw: &'a [i32], pub units: std::sync::Arc<pnr_core::UnitLib>,
         /// `false`: return the initial pile (PLC-01 `GpMode::Pile`).
         pub iterate: bool,
     }
     ```
     `initial_layout` (`mechanics.rs:361-386`) gains `n_axes: usize` and sets `axis = vec![c; n_axes.max(1)]`; `gp::place` then sets `l.power_uw = inp.power_uw.to_vec()`, `l.units = inp.units.clone()`, `l.refresh_temps()`. Pairs = `RuleBatch::mirror_pairs` over `reqs.hard`, collected once. Each gp iteration, after the momentum step (`gp/lib.rs:300-305`) and before the Φ gate (`:306`): for every axis id with ≥ 1 pair, `l.axis[id] = snap(mean over its pairs of (x_a + x_b)/2, rules.grid)`; then `l.refresh_temps()` (`kernel/core/src/thermal.rs:53-55`, O(n·sources)). `ThermalGradient` reads the cached `temp_mc`, so the finite-difference probe (`gp/lib.rs:242-259`) must also call `l.refresh_temps()` after each ±`ANALOG_PROBE` displacement and after restoring it, when any `power_uw > 0` (else the probe sees no thermal change and gp stays thermally blind); cost is 4n extra refreshes per iteration, visible in FLOW-09's gp timer, and moot if PLC-11 deletes the loop.
  2. Library (`lib.rs:573-586`): build `GpInput` with `n_axes = self.problem.blocks.len()`, `power_uw = &cells.power`, `units = cells.units.clone()`, `iterate = self.gp_mode == GpMode::Analytic` (new `Flow` field copied from `cfg.gp_mode` in `solve`, `lib.rs:298-332`); delete the post-gp patching at `lib.rs:579-586` (`coarse.groups = cells.abutment.clone()` included: after PLC-03 dp no longer reads groups; `cells.abutment` then has no reader until PLC-27 deletes it).
  3. Redefine `fixed[i]` (doc at `dp/lib.rs:190-191`, field doc at `lib.rs:1101-1102`): "drawn as given: no reshape, no rotation"; position is placed like any cell (AP-11 had two fixed cells overlapping forever). Edits in `backend/dp/src/lib.rs`: delete the position restores at `:160-163` and `:383-388`; delete the `sym_groups` filter at `:465` (and its `fixed` parameter); the per-move skip at `:307-309` becomes "skip only the rotate and reshape branches for a fixed cell"; drop `!sa.is_fixed(o)` from the swap branch (`:332`). In `legalize.rs`: remove the `fixed` parameter, `movable` (`:27`) and the restore loop (`:72-75`); `push` always splits (`:47`).
- Tests (`backend/gp/src/lib.rs`, new `mod place_tests`):
  - `gp_keeps_one_axis_per_block`: 6 cells, two `SymmetryGroup`s (axis 0, axis 1), `n_axes = 2` → `l.axis.len() == 2` and `l.axis[0] != l.axis[1]` for at least one of seeds 1..=5 (the two stages are not pinned to one line).
  - `gp_sees_power`: cells 0, 1 (1000×1000 nm) in one `ThermalGradient` pair in `reqs.cost`, cell 2 (1000×1000 nm) with `power_uw = 10_000` (10 mW), no nets. Run `gp::place` twice per seed, once with `power_uw = [0, 0, 10_000]` and once with `[0, 0, 0]`; **score both outputs under the true field** (set `l.power_uw = vec![0, 0, 10_000]`, `l.refresh_temps()`, then `|temp_mc[0] − temp_mc[1]|`). Assert the mean over seeds 1..=10 is strictly lower for the powered run.
  - `backend/dp/src/tests.rs` `fixed_cells_separate_but_keep_shape`: two fixed cells at the same coords, `variants` with 3 alternatives each → after dp `encroachment(&l, 0) == 0.0` and `variant`, `orient` unchanged.
  - Rewrite to the new meaning: `projection_never_moves_pinned_cells` (`tests.rs:339-346`) becomes `projection_moves_fixed_cells_but_not_their_shape`; `a_pinned_macro_never_moves` (`legalize.rs:126-133`) is deleted (the parameter is gone). `seeds_are_written_and_table_resized` (`tests.rs:307-315`) relies on `[true, true]` freezing every move (it asserts the final branch table equals the seeds): extract the seeding block `dp/lib.rs:222-238` into `fn seed_branches(reqs: &Requirements<Layout>, branch: &mut Vec<bool>) -> Vec<BranchId>` and test that function directly with the same expectations (`[false, false, false, true]`, then length 6). `a_fixed_cell_never_reshapes` (`tests.rs:132-141`) stays valid.
- Acceptance: no fixture regresses in lex key (median over seeds 1–5); `injected_macro.rs` (`frontend/library/tests/`) still passes.

### PLC-07 Per-pair spacing from cell edge profiles (well sharing)
- Priority: P1. Effort: L. Depends on: PLC-02, PLC-04.
- Why: AP-02, AP-24, G2 of audit-04, PL-5 and PL-11 (audit-07: implant and well merges patched by bridges after placement, no placement-time merge guard, `lib.rs:608-613`); BAL1-45 pairwise d_min (balasa_graeb_survey.txt L4742–4771), n/p minimum distances keep the folded-cascode example above 121 % area usage (L6780–6783), HB* reserves well space only where device types differ (L3685–3691); H01-05 merge same-backgate PMOS into one well (Hastings §4.2.3, hastings.txt L10090–10094, PDF 180); Lampaert legal overlap requires same net/layer/bulk (lampaert.txt L3892–3905).
- Current: scalar clearance 1270 nm (`lib.rs:529-538`) used in `encroach` (`mechanics.rs:226-234`), `Sa::encroach_moved` (`dp/lib.rs:125-135`), region sizing (`:258`) and the legalizer (`legalize.rs:39-40`); the flow also passes it to `elaborate::antenna_diodes` (`frontend/library/src/lib.rs:635`), which keeps needing a scalar keep-out after step 5 (pass `SpacingTable`'s `fallback`, or the table, explicitly). `Process::space_between` exists (`kernel/core/src/process.rs:75-78`; `backend/verify/src/pdk.rs:1003-1007`) and is used by cells (`kernel/cells/src/mosfet.rs:495`) but not by placement.
- Change:
  1. New `backend/gp/src/spacing.rs` (`pub mod spacing;` in `gp/lib.rs`):
     ```rust
     /// Placement spacing roles. `*_in` / `*_out`: inside / outside the cell's own n-well.
     /// `other`: a drawn layer no listed role maps to (always spaced at `fallback`).
     pub const ROLES: [&str; 20] = ["nwell", "dnwell", "diff_in", "diff_out", "tap_in", "tap_out", "poly", "nsdm",
         "psdm", "li", "licon", "mcon", "met1", "npc", "rpm", "res_implant", "diode_mk", "rpoly", "diom", "other"];
     /// The `Process` role each placement role reads; `""` = none. `Pdk::layer` resolves a role through
     /// `cell.layers`, then `metN`/`viaN`, then a deck layer of the same name (`backend/verify/src/pdk.rs:938-948`),
     /// so `dnwell`, `npc`, `rpm` resolve on sky130 by deck name. `rpoly` and `diom` are sky130 `cell.layers` roles
     /// (`poly_rs`, `areaid_diode`; `pdks/sky130.json` `cell.layers`) that the generators draw (`layer("rpoly")`,
     /// `layer("diom")` in `kernel/cells/src`); without them every poly resistor and diode would read as `other`.
     /// Recipe-only roles (`res_block`, `rpoly_b`, `res_implant` resolve only through the resistor `Overlay`,
     /// `pdk.rs:1155-1161`) land on deck layers the list may not name; the test
     /// `shipped_cells_draw_no_unmapped_layer` below is the check, not this comment.
     pub const DECK_ROLE: [&str; 20] = ["nwell", "dnwell", "diff", "diff", "tap", "tap", "poly", "nsdm",
         "psdm", "li", "licon", "mcon", "met1", "npc", "rpm", "res_implant", "diode_mk", "rpoly", "diom", ""];
     pub const N: usize = ROLES.len();
     /// Roles whose facing shapes may touch and merge into one figure: implant and mask layers (no net),
     /// and the n-well when both wells carry the same bulk net.
     const MERGEABLE: [usize; 5] = [0 /*nwell*/, 7 /*nsdm*/, 8 /*psdm*/, 13 /*npc*/, 14 /*rpm*/];
     /// Marker (id) layers: no drawn material, so a same-role pair with no deck value is `NoRule` (0), not `fallback`.
     const MARKER: [usize; 2] = [16 /*diode_mk*/, 18 /*diom*/];
     #[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Face { L = 0, B = 1, R = 2, T = 3 }
     /// Distance from the bbox face to the nearest shape of each role, nm; `i32::MAX` = role absent.
     #[derive(Clone, Copy, Debug)] pub struct Edge { pub inset: [i32; N] }
     #[derive(Clone, Debug)] pub struct Profile { pub edge: [Edge; 4], pub well_net: Option<pnr_core::NetId>,
                                                  pub matched: Option<analog::matching::class::MatchClass> /* PLC-13 */ }
     #[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Src { Deck, Sidecar, Fallback, NoRule }
     /// Legal gaps between two facing cells: exactly `0` when `abut`, else any `g >= min` (nm, lattice-rounded).
     #[derive(Clone, Copy, Debug, PartialEq, Eq)] pub struct Gap { pub abut: bool, pub min: i32 }
     pub struct SpacingTable { pub rule: [[i32; N]; N], pub src: [[Src; N]; N], pub lattice: i32, pub fallback: i32 }
     impl SpacingTable {
         pub fn new(p: &dyn pnr_core::Process, sidecar: &[(String, String, i32)], fallback: i32, lattice: i32) -> Self;
         pub fn gap(&self, a: &Profile, fa: Face, b: &Profile) -> Gap;
     }
     pub fn profile(m: &pnr_core::Macro, p: &dyn pnr_core::Process, bulk: Option<pnr_core::NetId>) -> Profile; // R0 frame
     pub fn oriented(p: &Profile, o: pnr_core::Orient) -> Profile; // permute faces by `o`
     /// `of[cell][variant][orient as usize]`, precomputed once (8 orients; a few KB per cell).
     pub struct Profiles { pub of: Vec<Vec<[Profile; 8]>> }
     ```
  2. Rule table (`SpacingTable::new`), for every role pair (i, j), symmetric (`rule[i][j] = rule[j][i]`):
     1. Deck: `i == j` or both roles read the same deck role (e.g. `diff_in`/`diff_out`): `p.space(DECK_ROLE[i])`; otherwise `max(p.space_between(DECK_ROLE[i], DECK_ROLE[j]), p.space_between(DECK_ROLE[j], DECK_ROLE[i]))` (`process.rs:75-78`, `pdk.rs:1003-1007`).
     2. Sidecar `cell.placement_space`, a new JSON list of `[role_a, role_b, nm, "source"]` read in the library from `pdk.cell` (`backend/verify/src/pdk.rs:54`; `parse_roles` keeps only scalars, `:1259-1266`). A sidecar role `"diff"` expands to `diff_in` and `diff_out`, `"tap"` to `tap_in` and `tap_out`. The entry wins when larger than the deck value. Entries to add to `pdks/sky130.json` (values from the pinned GPurify deck `8df8c09/pdks/sky130.deck`; every derived-layer two-layer rule between drawn roles that a cell can put near its bbox):

        | role_a | role_b | nm | deck rule (line) | note |
        |---|---|---|---|---|
        | diff | diff | 270 | difftap.3 `space(difftap)` (L266) | on the merged diff+tap layer, so also: |
        | diff | tap | 270 | difftap.3 (L266) | |
        | tap | tap | 270 | difftap.3 (L266) | |
        | diff_out | nwell | 340 | difftap.9 `space(ndiff, nwell)` (L269) | any diff outside a well is treated as n-diff (conservative) |
        | tap_out | nwell | 130 | difftap.11 `space(ptap, nwell)` (L271) | |
        | poly | diff | 75 | poly.4 `space(poly_field, diff)` (L280) | |
        | poly | tap | 300 | poly.6 `space(gate, tap.interacting(diff))` (L635) | dominates poly.5 = 55 (L281) |
        | npc | poly | 90 | npc.4 `space(npc, gate)` (L309) | |
        | npc | licon | 90 | licon.13 `space(npc, licon_dt)` (L343) | |
        | nsdm | diff | 130 | nsd.7 `space(nsdm, p_lone)` (L323) | conservative: any diff |
        | nsdm | tap | 130 | nsd.7 (L323) | |
        | psdm | diff | 130 | psd.7 `space(psdm, n_lone)` (L324) | |
        | psdm | tap | 130 | psd.7 (L324) | |
        | licon | psdm | 110 | licon.9 `space(poly_licon_nrpm, psdm)` (L339) | |
        | licon | diff | 190 | licon.14 `space(poly_licon, difftap)` (L344) | |
        | licon | tap | 190 | licon.14 (L344) | |
        | rpm | nsdm | 200 | rpm.6 (L294) | |
        | rpm | poly | 480 | poly.9 `space(poly_res, poly_plain)` (L284) | rpm encloses the resistor poly, so measuring from rpm is conservative; dominates rpm.7 = 200 (L295) |
        | rpm | diff | 480 | poly.9.difftap (L285) | |
        | rpm | tap | 480 | poly.9.difftap (L285) | |
        | rpoly | poly | 480 | poly.9 `space(poly_res, poly_plain)` (L284; `poly_res = poly and poly_rs`, L144) | covers resistors drawn without rpm (`rbody_po`, L203) |
        | rpoly | diff | 480 | poly.9.difftap (L285) | |
        | rpoly | tap | 480 | poly.9.difftap (L285) | |

        Same-layer values come from the deck through `p.space` (sky130: nwell 1270 L228, dnwell 6300 L225, npc 270 L307, licon 170 L333, li 170 L349, mcon 190 L355, rpm 840 L289, nsdm/psdm 380 L314/L316).
     3. Still no value: same-role pair of a `MARKER` role → `0` (`Src::NoRule`; sky130 has no `areaid_diode` spacing rule, `grep areaid_diode` in the deck hits only L81 and the derived-layer definition L213); other same-role pair → `fallback` (`Src::Fallback`, today's scalar from `lib.rs:529-538`, printed once per run with the role names); cross-role pair → `0` (`Src::NoRule`: the deck states no spacing between the two layers, so DRC cannot flag one); any pair involving `other` → `fallback` (an unmapped layer is never spaced tighter than today).
  3. `gap(a, fa, b)` with `fb` the opposite face:
     ```text
     g_all = 0; g_hard = 0
     for i, j with a.edge[fa].inset[i] < MAX, b.edge[fb].inset[j] < MAX, rule[i][j] > 0:
         need  = rule[i][j] − a.edge[fa].inset[i] − b.edge[fb].inset[j]
         g_all = max(g_all, need)
         merge = i == j && MERGEABLE.contains(i) && a.inset[i] == 0 && b.inset[j] == 0
                 && (i != nwell || (a.well_net.is_some() && a.well_net == b.well_net))
         if !merge { g_hard = max(g_hard, need) }
     Gap { abut: g_hard <= 0, min: round_up(g_all, lattice) }
     ```
     `abut` means: at gap exactly 0 every mergeable facing pair touches and merges (the flow merges n-well rects, `lib.rs:664-666`, and the checker merges touching same-layer shapes), and every other rule is met. Any gap in `(0, min)` is illegal when a mergeable pair's `need > 0`: `implant_bridges`/`well_bridges` only fill equal spans (§1.2), so the band is closed by construction, not by bridging. When REL-16 lands, the n-well merge additionally requires `post_cell::may_share_well(flags_a, flags_b)` (a `Profile` field then).
  4. `profile(m, p, bulk)`: map each shape's `LayerId` to the first role r with `p.layer(DECK_ROLE[r]) == Some(layer)`, else `other`; `diff`/`tap` shapes are `_in` if contained in the union of the macro's own nwell rects, else `_out`; inset per face = distance from that bbox face to the shape's nearest edge (0 when it touches the face). Profiles are computed in `CellSpace::new` after PLC-02 alignment, for every (cell, variant), then `oriented` for all 8 `Orient`s, with `bulk` = net of the pin whose name ends in `:B` (as `post_cell::well_bridges` does, `post_cell.rs:398-402`). Ringed cells need nothing special: their halo is empty space inside the bbox, so every inset ≥ halo.
  5. Replace `gp::Rules { grid, clearance }` by `gp::PlaceRules { grid: i32, rotate_90: bool, spacing: Arc<SpacingTable>, profiles: Arc<Profiles> }` (`gp/lib.rs:155-163`; built once in `solve`, not per epoch as `place_rules` is today, `lib.rs:575`, `:594`, `:635`). `impl PlaceRules { pub fn gaps(&self, l: &Layout, a: usize, b: usize) -> (i32, i32); pub fn encroach(&self, l, a, b) -> f64; pub fn encroachment(&self, l) -> f64 }`: `gx` = `gap(lo, R, hi).min` for the x-ordered pair (`lo` = smaller centre x; ties: the max of both orders), `gy` likewise with `T`, each read from `profiles.of[c][l.variant[c]][l.orient[c] as usize]`; encroach area `= (gx + hw_a + hw_b − |Δx|)⁺ · (gy + hh_a + hh_b − |Δy|)⁺`. The flat dp path (`Sa::encroach_moved`, `dp/lib.rs:125-135`), `report` (PLC-04 step 3), PLC-01's `clearance_residue_nm2` and the legalizer (`separate_overlaps` takes `&PlaceRules` instead of `clearance`, `legalize.rs:39-40`) use `min` only — they never exploit `abut`; PLC-08's decoder does. Region sizing (`dp/lib.rs:252-258`) uses per cell `gmax_c = max over faces f and roles i present on f of (max_j rule[i][j] − inset_i)⁺`. `elaborate::antenna_diodes` (`lib.rs:635`) receives `spacing.fallback`, the old scalar.
- Tests:
  - `backend/gp/src/spacing.rs`:
    - `ndiff_facing_foreign_nwell_needs_rule_minus_insets`: A has `diff_out` inset 200 on R, B has `nwell` inset 0 on L (different wells), `rule[diff_out][nwell] = 340` → `Gap { abut: false, min: 140 }`.
    - `same_bulk_pmos_may_abut`: both `nwell` inset 0, same `well_net`, `diff_in` insets 180, difftap rule 270 → `Gap { abut: true, min: 1270 }` (legal: 0 or ≥ 1270).
    - `foreign_wells_never_abut`: as above with different `well_net` → `Gap { abut: false, min: 1270 }`.
    - `nmos_pair_with_edge_implants`: `nsdm` insets 0, `diff_out` insets 125, rules nsdm 380 / difftap 270 → `need(diff) = 20 > 0` → `Gap { abut: false, min: 380 }`.
    - `rotated_profile_permutes_faces`: R90 maps L→B, B→R, R→T, T→L.
    - `unmapped_layer_uses_the_fallback`: a shape on an unmapped layer at inset 0 on both sides, fallback 1270 → `min == 1270`.
    - `diode_markers_do_not_force_the_fallback`: two cells whose only facing shapes are `diom` at inset 0, no deck rule → `Gap { abut: true, min: 0 }`.
  - `frontend/library/tests/placement_spacing.rs` `shipped_cells_draw_no_unmapped_layer`: for every cell and variant of the 10 fixtures on sky130 (the `CellSpace` the flow builds), count shapes that `profile` maps to `other`; assert **0** on sky130; print the per-layer counts for gf180mcu, ihp_sg13g2 and generic_finfet (their FinFET roles `fin`, `sdt`, `lisd`, `lig`, `gcut` are not in `ROLES` and fall to the safe `fallback` until a deck-specific extension is measured).
  - `frontend/library/tests/placement_spacing.rs` `table_gaps_are_drc_clean_on_sky130`: for every ordered pair of cells from `ota, dac4, rc_filter, chain4, pair` (variants 0 and 1), both x-facing and y-facing: stamp the two macros at `min` and, when `abut`, also at 0 (lattice-aligned), add rings/bridges via `post_cell::guard_rings`, `well_bridges`, `implant_bridges` as `Flow::epoch` does (`lib.rs:607-613`), run the signoff checker (`signoff_shapes`, the same call as `lib.rs:670`) on the pair → assert **0** DRC findings. Also record tightness (share of pairs that fail at `min − 2·lattice`); print, do not assert.
- Acceptance: T2 area usage median improves ≥ **20 %** vs baseline (**policy**; **derived** rationale: a 2×2 µm NMOS cell pitched at 2 + 1.27 µm today vs 2 + 0.38 µm under the `nmos_pair_with_edge_implants` gap drops its pitch area from 10.7 µm² to 5.7 µm²); signoff DRC and LVS no worse on any fixture.
- Risks / notes: derived-layer deck rules (`ndiff`, `ptap`, `poly_field`, `gate`) are not visible through `space_between` on drawn roles; the sidecar table closes that gap and the DRC probe test proves it per deck. gf180 and IHP need their own `placement_space` entries (same test on those decks, FLOW-04/06 load the keys); until they exist, those decks get `fallback` for same-role pairs without a deck value and 0 for cross pairs, so the probe test must run on them before `DpMode::Sp` becomes their default. On gf180mcu and ihp_sg13g2 `diff` and `tap` map to the same deck layer (`comp`, `activ`), so `profile` classifies every tap shape as `diff_*` (first matching role): the `tap_out`/`nwell` entry never applies there and the stricter `diff_out`/`nwell` one does (conservative, never a DRC miss). A union of two abutting wells of unequal height is an L-shape, not a notch; three-cell notches are not covered by the pairwise probe (the fixture flow DRC gate catches them).

### PLC-08 Hierarchical symmetric-feasible sequence pair with an exact decoder
- Priority: P1. Effort: L. Depends on: PLC-02, PLC-03, PLC-07.
- Why: AP-01 (critical), AP-12 (part), AP-14, AP-15, AP-24, PL-12 (audit-07, status **N**); NOTES-38/39; BAL1-04..13, BAL1-24, BAL1-45, BAL1-55, BAL2-30.
- **Decision and evidence.** Options: (a) S-F sequence pair (SP); (b) HB*-tree with ASF-B*-tree islands; (c) keep flat SA and add constraint-graph LP compaction.
  - Flat absolute search converges slowly, ends with overlap that needs post-processing, and has a fragile overlap weight (Balasa §1.1.5, balasa_graeb_survey.txt L739–768). Philis shows all three symptoms (AP-01, AP-07, AP-14). With the same cost function, cooling schedule and inner-loop criterion (§1.5, L2735–2738), every S-F topological representation beat the absolute one in CPU and sometimes quality; S-F binary trees were faster than S-F SPs but "poorer" in quality (L2794–2817). This rules out (c) as the primary representation; (c) also needs an LP solver the workspace does not have.
  - HB* reports 1.6–10.3 % average area reduction (Table 2.6, industrial circuits) and 4.09–39.88× speedups over SP, segment-tree, SP+LP and SP-with-dummy-node placers (Tables 2.5–2.6, L4103–4146, L4174–4187; speedups only against same-platform runs), but it applies well/guard-ring white space only by snapping against the current contour when device types differ (L3685–3691) and needs contour nodes with six structural rules for rectilinear islands plus dangling-node repair (L3522–3625, L3920–3964).
  - Philis's largest area lever is exact pairwise n/p and well spacing (PLC-07; Plantage, folded-cascode example: area usage always > 121 % "because of empty areas caused by the minimum distance constraints between nMOS and pMOS", L6780–6783). An SP gives every cell pair an explicit left/right or above/below relation, so a pairwise gap is a plain edge weight, and the shadow rule for diagonal pairs (Plantage Algorithm 3.3, L5793–5936) is a condition on that edge.
  - HB*'s O(n) packing advantage is not binding: analog circuits "seldom contain more than 100 cells per hierarchical level" (L1604–1605), so an O(k²) decode per hierarchy node is cheap.
  - **Chosen: (a), hierarchical.** Each symmetry group and each proximity cluster is a node whose members are contiguous in both sequences of its parent. By SP semantics (L1826–1838), an outsider is then in one relation (left/right/above/below) to all members of the node, so no outsider enters the node's bbox: a group gets an exclusive region (the island adjacency of Def. 2.1 is measured separately by PLC-12).
- Current: no SP, tree, contour or LP anywhere (BAL1 baseline grep); symmetry by projection (`dp/lib.rs:154-159`, `:371-397`), legality by push (`legalize.rs`).
- Change (new `backend/dp/src/sp.rs`):
  ```rust
  #[derive(Clone, Copy)] pub enum Kid { Cell(u16), Node(u16) }
  #[derive(Clone)] pub struct Sym { pub axis: pnr_core::ids::AxisId, pub mate: Vec<u16> } // mate[k] = partner kid; mate[k] == k: self-symmetric
  #[derive(Clone)] pub struct Node { pub kids: Vec<Kid>, pub alpha: Vec<u16>, pub beta: Vec<u16>, pub sym: Option<Sym> }
  #[derive(Clone)] pub struct Tree { pub nodes: Vec<Node>, pub root: u16, pub home: Vec<(u16, u16)> } // home[cell] = (node, kid slot)
  pub enum TreeError { CellInTwoAxes { cell: u16, a: AxisId, b: AxisId } }
  impl Tree {
      pub fn build(n_cells: usize, pairs: &[(u32, u32, u16)], blocks: &[Vec<pnr_core::DeviceId>]) -> (Tree, Vec<TreeError>);
      pub fn is_sf(&self, node: u16) -> bool;
      pub fn make_sf(&mut self, node: u16);
      pub fn seed_from(&mut self, l: &pnr_core::Layout);
      pub fn seed_constructive(&mut self);
  }
  /// Per cell, for the cell's current (variant, orient): extents, oriented profile, routing halo (all zero until PLC-15).
  pub struct Geo<'a> { pub w: &'a [i32], pub h: &'a [i32], pub prof: &'a [&'a Profile], pub halo: &'a [[i32; 4]],
                       pub table: &'a SpacingTable, pub lattice: i32 }
  pub struct Out { pub x0: Vec<i32>, pub y0: Vec<i32>, pub axis: Vec<(AxisId, i32)> }
  #[derive(Debug)] pub enum Fail { SymY(u16), SymX(u16), Band(u16) } // payload: node index
  /// Reused buffers (inverse permutations, per-node relative coordinates, lb) so a decode allocates nothing.
  #[derive(Default)] pub struct Scratch { /* pa, pb, rel_x, rel_y, lb: Vec<i32> per node */ }
  pub fn decode(t: &Tree, g: &Geo, s: &mut Scratch, out: &mut Out) -> Result<(), Fail>;
  ```
  **Build.**
  1. Symmetry nodes: group `RuleBatch::mirror_pairs` of `reqs.hard` by `AxisId`; kids = distinct cells; `mate` from the pairs (a == b → self; merged CC cells are self-symmetric kids). A cell named by two axes → `TreeError::CellInTwoAxes`, reported as a placement hard row "conflicting symmetry" and the second group is dropped (BAL2-48 pre-flight, L13836–13867).
  2. Proximity nodes: for each recognition block (`cells.groups`, `lib.rs:599`; EXT-13's `Intent.tree` replaces it when it lands, same rule per tree node) with ≥ 2 cells: a node containing its symmetry node(s) whose cells are all in the block plus the block's remaining cells. The glue block (last, `backend/annotator/src/lib.rs:103-104`) contributes loose cells to the root.
  3. Root kids: proximity nodes, symmetry nodes not inside a block, loose cells.

  **S-F condition** (Balasa eq 1.1, L1856–1868): for distinct kids x, y of a symmetry node, `(pa[x] < pa[y]) == (pb[mate[y]] < pb[mate[x]])` with `pa`, `pb` the inverse permutations.

  **`make_sf(node)`** (derived from §1.3.3: "pick randomly the order in α of all the devices in the symmetry group, and arrange in β their symmetric pairs in exactly the reverse order", L2389–2390): list the node's kids in α order `g_1..g_m`; rewrite β, in the β slots the node's kids occupy, as `mate[g_m], …, mate[g_1]`. Proof: `pb[mate[g_i]] = m − i`, so `pa[x] < pa[y] ⇔ i < j ⇔ pb[mate[x]] > pb[mate[y]]`, which is (1.1).

  **`seed_from(l)`** (**derived** SP embedding of a coordinate placement): per node, key point = cell centre or mean centre of a node's cells; α = kids sorted by `(x − y, kid)`, β = sorted by `(x + y, kid)` (a left of b ⇒ smaller x − y and smaller x + y; a above b ⇒ smaller x − y, larger x + y); then `make_sf` on symmetry nodes. **`seed_constructive`**: symmetry nodes use Balasa's initial code `α = a_1…a_p c_1…c_s b_p…b_1`, `β = a_1…a_p c_s…c_1 b_p…b_1` (L2375–2405); other nodes α = β = kids by descending area (one row).

  **Decode** (post-order over nodes; relations from α, β: `i left of j ⇔ pa[i] < pa[j] ∧ pb[i] < pb[j]`; `i below j ⇔ pa[i] > pa[j] ∧ pb[i] < pb[j]`; β order is a topological order for both):
  1. Kid geometry: a cell kid uses `(w, h)` of its variant under `orient`, its oriented profile (`profiles.of[c][variant][orient]`, PLC-07), and its halo (PLC-15; zero before); a node kid uses its decoded bbox and a node profile whose face insets are, per role, `min over member cells of (member inset + member-face-to-node-face distance)`.
  2. **Y (Balasa Step 1y, L1993–2099, with pairwise gaps):** in β order, `y_j = max(lb_j, max_{i below j} y_i + h_i + gapY(i, j))` with `G = table.gap(prof_i, T, prof_j)`, `gapY = (if G.abut && halo_i[T] + halo_j[B] == 0 { 0 } else { G.min }) + halo_i[T] + halo_j[B]` (a routing halo forbids abutment). Symmetry node: for each pair (a, b) whose centres `y + h/2` differ, raise the lower member's `lb` to equalize the centres and repeat the pass; at most `p + 2` passes (**policy**; Balasa: Step 1y may need an order-p number of executions (the asymptotic symbol is garbled in the reftext, L2093), the Fig. 1.18j–k example needs ⌈p/2⌉ + 1, and "no more than three iterations" in most practical cases, L2093–2099); still unequal → `Fail::SymY`.
  3. **X gap with shadows** (Plantage Algorithm 3.3, L5793–5936): for i left of j, if the vertical separation of i and j (from step 2) is ≥ their required vertical gap, `gapX = 0`; else `gapX` = the step-2 formula with faces `R`/`L`. (`DtiBand` needs no decoder support: it stays a hard rule evaluated on the decoded layout and the gate rejects a violating code; no shipped deck has DTI data, `pdks/sky130.json:46`.)
  4. **X, Step 1x:** in β order, `x_j = max(lb_j, max_{i left of j} x_i + w_i + gapX(i, j))`. Non-symmetry node: done.
  5. **Symmetry node, centre form with doubled coordinates** (`C_i = 2x_i + w_i`, twice the centre; `ax2 = 2·axis`): mirror pair ⇔ `C_l + C_r = 2·ax2`; self-symmetric ⇔ `C_c = ax2`.
     - Step 2x′ (simplified from Balasa Step 2x, L2129–2250): `ax2 = max(max_pairs ⌈(C_l + C_r)/2⌉, max_self C_c)`, rounded up to a multiple of `2·lattice` if the node has a self-symmetric kid (then `C_c ≡ 0 mod 2·lattice` is reachable because x and w/2 are lattice multiples after PLC-02), else to a multiple of `lattice`. PLC-28 later replaces this rounding by "smallest value ≥ that bound with `ax2 ≡ p0 (mod 2P)`" to put the axis on the routing lattice.
     - Step 3x (L2251–2256): repeat Step 1x in β order, and when j is the right member r of pair (l, r) set `x_r = max(x_r, (2·ax2 − C_l − w_r)/2)`; when j is self-symmetric set `x_c = max(x_c, (ax2 − w_c)/2)`. `W = max_j (x_j + w_j)`.
     - Step 4x (L2257–2277; Balasa sweeps α from n to 1 with a β-indexed queue; any reverse topological order of "left of", such as reverse β, visits the same constraints): in reverse β order, `x'_j = min(W − w_j, min_{s: j left of s} x'_s − w_j − gapX(j, s))`; when j is a right member r, `x'_r = min(x'_r, (2·ax2 − C_l − w_r)/2)` using the Step-3x `C_l`; set `x_j = x'_j`.
     - `verify(node)`: every relation edge holds with its gap, every pair satisfies `C_l + C_r = 2·ax2` and equal centre y, every self kid `C_c = ax2`, every coordinate a lattice multiple. On failure run `fix_monotone`: recompute `ax2` from current x, raise `lb` of right members and self kids to their required values, rerun Step 1x; at most `2p + 2` passes; still failing → `Fail::SymX`. Balasa argues (informally, no formal proof) that for S-F codes with plain widths the algorithm ends after Step 4x with all symmetry constraints met (L2308–2323); pairwise gaps generalize the edge weights, so `verify` plus the fallback guard that generalization and the property test below measures it.
  6. **Merge band rule** (PLC-07 `Gap`): after steps 2–5 of a node, any two kids with overlapping projections across a facing pair of faces whose `Gap { abut: true, min }` has an actual gap `g` with `0 < g < min` get the far kid's `lb` raised to `near edge + min` and the node's pass repeated (monotone raises only; counted in the same pass cap); still banded → `Fail::Band`. Cross-node pairs are checked on the node profiles in the parent's pass.
  7. (removed: DTI needs no decoder step; AR-16's `<=` fix is PLC-04 step 6.)
  8. **Assembly** (pre-order): absolute `x0 = parent x0 + relative x`; `Layout.x = x0 + w/2`, `y = y0 + h/2`, `hw = w/2`, `hh = h/2`; `Layout.axis[axis_id] = (2·node_x0 + ax2)/2` (exact: `ax2` and `2·node_x0` are multiples of the lattice, so the axis is a multiple of lattice/2 = 5 nm on sky130; the `Symmetry` rule compares `x_a + x_b` with `2·axis`, both integers).
  9. Complexity: `O(Σ_nodes k² · passes)`, passes ≤ p + 2.
- **Delivery split.** PLC-08 lands `sp.rs` only (tree, S-F operations, seeds, decode, `verify`, `fix_monotone`) with its unit and property tests. The SP `dp::place` path below needs a move set, an energy and a schedule, so it is implemented in PLC-09; the API is fixed here so PLC-10 and FLOW-08 can code against it.
- `dp::place` API (the SP path, implemented in PLC-09):
  ```rust
  pub struct PlaceInput<'a> { pub macros: &'a [Macro], pub variants: &'a [gp::VariantSpace], pub reqs: &'a Requirements<Layout>,
      pub fixed: &'a [bool], pub blocks: &'a [Vec<DeviceId>], pub rules: gp::PlaceRules, pub locks: &'a locks::Locks,
      pub halo: &'a [[i32; 4]], pub net_weight: &'a [f32], pub n_axes: usize, pub power_uw: &'a [i32], pub units: Arc<UnitLib> }
  pub enum Start<'a> { Cold(&'a Layout), Constructive, Warm { tree: &'a sp::Tree, variant: &'a [u16], orient: &'a [Orient] } }
  pub fn place(inp: &PlaceInput, start: Start, schedule: Schedule, prices: &mut gp::Prices, seed: u64) -> (Layout, sp::Tree, Report, PlaceStats);
  ```
  `Schedule` is PLC-10's (`Schedule::cold()` / `warm()`). `Cold(l)` seeds the tree with `Tree::seed_from(l)` (gp's layout); `Constructive` with `Tree::seed_constructive()` and PLC-01's `GpMode` gains `Constructive` (gp skipped). The flat path is kept behind `Config::dp_mode: DpMode { #[default] Flat, Sp }` until acceptance, then the default flips to `Sp`; `Flow::epoch` matches on it at `lib.rs:587`.
- Tests (`backend/dp/src/sp.rs` unit tests and `backend/dp/tests/sp_props.rs`):
  - `balasa_example_1_decodes_symmetric`: cells A(2×4) B(3×1) C(4×2) D(5×3) E(4×2) F(2×10) G(2×10) H(2×3) I(5×5) J(4×2) K(3×2) L(3×2) (units × 1000 nm), SP (EDCKAFGIHJBL, KACDEFGLHBJI), pairs (F,G), (K,L), (C,J) (Example 1, L2021–2028; dimensions from the Fig. 1.18 caption, L2068–2072, given as width × height), gap table all 0, lattice 10 → `is_sf` true, `decode` Ok, three pair equalities hold, no two boxes overlap, and `y0[C] == y0[J] == 4000` (the text: `y_J` is first 3 and becomes 4 when `y_C` is computed, a second Step 1y pass being needed, L2024–2028).
  - `make_sf_satisfies_condition_1_1`: random α over 2 pairs + 1 self, `make_sf` → `is_sf`.
  - `random_sf_codes_decode_exactly` (property, SplitMix64 seed 7): 10,000 trees, one symmetry node (p ∈ [1, 4] pairs, s ∈ [0, 2] selfs) inside a root with up to 8 loose cells; dims multiples of 20 nm in [100, 5000]; random pairwise gaps in [0, 1270]; random α, then `make_sf`. Assert `decode` Ok for 100 % and `verify` holds; print how often `fix_monotone` ran.
  - `contiguous_node_has_exclusive_bbox`: same generator with one proximity node → no outsider box intersects the node bbox.
  - `pairwise_gaps_hold_with_shadows`: every pair with overlapping projection keeps `table.gap`; every diagonal pair keeps it on at least one axis.
  - `every_coordinate_is_on_the_lattice`.
  - `merge_band_is_closed`: two same-net PMOS kids whose longest-path gap lands at 500 nm with `Gap { abut: true, min: 1270 }` → decoded gap 1270; at 0 → stays 0.
- Acceptance: every test above green, including `random_sf_codes_decode_exactly` at 100 % Ok over 10,000 trees. The fixture-level acceptance (`DpMode::Sp` vs `Flat`) is PLC-09's, because it needs PLC-09's anneal.
- Risks / notes: symmetry groups become exclusive regions; a foreign cell can no longer sit between mirrored partners (Lin/Chang motivate islands by the Pelgrom distance term, L3158–3183). Nested groups and horizontal axes come in PLC-20; until then a second axis on a cell is a reported conflict, not a silent drop.

### PLC-09 Annealing over codes: moves, energy, schedule
- Priority: P1. Effort: M. Depends on: PLC-08, PLC-18.
- Why: AP-08 (part), AP-13, AP-19; LAMP-32 (lampaert.txt L4913–5017, eqs 4.37–4.40, PDF 117–119); BAL1-13, BAL1-33/34 (L3738–3748, L3751–3916, L4074–4081).
- Current: flat moves and a fixed 220 × 60·n schedule (`dp/lib.rs:21-27`, `:301-350`); comment records an earlier Lampaert-schedule trial that raised rc_filter C +7 % and bjt_mirror area +22 % with the flat representation (`dp/lib.rs:296-300`).
- Change (`backend/dp/src/anneal.rs`):
  1. **Moves** (node chosen with probability ∝ its kid count; `Fail` from decode rejects the move):
     - M1 swap two kids in α (p = 0.30); in a symmetry node, swap x, y in α **and** `mate[x]`, `mate[y]` in β (Balasa §1.3.3, L2375–2405).
     - M2 swap two kids in β (p = 0.30); symmetry node: swap x, y in β and `mate[x]`, `mate[y]` in α.
     - M3 swap two kids in both (p = 0.15); symmetry node: pair flip, i.e. exchange `x ↔ mate[x]` in both sequences (relabelling by the involution `mate` preserves (1.1)).
     - M4 rotate an orientation set (PLC-03 `Locks::orient`) or a free cell (p = 0.08; a half turn instead when `PlaceRules::rotate_90` is false; fixed cells never); symmetry pairs rotate jointly.
     - M5 reshape a shape set (PLC-03 `Locks::shape`) or a free cell (p = 0.07; only when `variants` is non-empty; fixed cells never).
     - M6 DTI branch flip (p = 0.02 if branches exist; else redistributed to M1), today's `try_branch` logic (`dp/lib.rs:537-539`).
     - The remaining probability goes to M1 in the root (moves a whole node relative to its siblings).
     Move probabilities are **policy** (Lin/Chang select hierarchy nodes less often, L3751–3764).
  2. **Energy**: `E = w_A·(A_bb / A_cells − 1) + Σ_n w_n·HPWL_n / L_ref + Σ_b crit_b·cost_b + Σ_b (π_b·r_b + ½·ρ_b·r_b²)`, with `A_bb = l.footprint_nm2()` (`kernel/core/src/layout.rs:46`), `A_cells = Σ_i 4·hw_i·hh_i` (PLC-01's `area_ref`), `w_A = 1.0` (**policy**; the SA cost of Lin/Chang is `Φ = α·A_P + β·W_P` with user-set α, β, eq 2.4, balasa_graeb_survey.txt L3744–3748, and BAL1-33's recipe says to add the bounding area explicitly: an SP decode is compact along each longest path, but codes differ widely in bbox shape, and with HPWL alone a netless or weakly connected fixture has no pull toward a small footprint; the flat path got that pull from `REGION_FILL`, `dp/lib.rs:28-30`, `:258`), `L_ref = sqrt(Σ cell area)` nm (PLC-18), `π_b = −λ_b ≥ 0` the bound price (`Prices::weight_of`, `gp/lib.rs:107`) and `ρ_b` its penalty, `r_b` budget residuals (placement budgets, PLC-16's `PlacePerf` rows included). Implementation: `gp::mechanics::augmented_cost(reqs, l, prices) -> f64` next to `analog_cost` (`mechanics.rs:247-256`; gp has crate access to `Prices`), and `Prices::bind` also caches `rho: Vec<f32>` per bound batch (0 for an unpriced batch). The quadratic term gives the augmented Lagrangian its pull before λ has grown (PL-9). Prices are read-only during the anneal (FLOW-03 settles once per epoch).
  3. **Gate**: `(hard count, Σ hard residual)` lexicographic, then Metropolis on ΔE. **Final quench**: the last 3 temperatures use `(hard count, Σ hard residual, Θ)` lexicographic, then ΔE, so the returned state never trades Θ for PEX (keeps the invariant of `theta_outranks_pex_so_a_budget_is_never_traded_for_parasitics`, `dp/src/tests.rs:235-257`).
  4. **Schedule**: `T0 = −mean(ΔE⁺)/ln P0` over `32·m` random moves from the start code, `P0 = 0.6` cold (Lampaert eq 4.37–4.38; Lin/Chang eq 2.5, L4081), `P0 = 0.1` warm (**policy**); cooling `α = 0.9` (Lin/Chang, L4077–4078; Lampaert varies α between 0.8 and 0.95 by chain length, L5014–5017); moves per temperature `N_T = 20·m`, `m = Σ kids` (**policy**); stop when (a) `T < 1e-3·T0`, or (b) 5 consecutive temperatures improve the best E by < 0.1 % (threshold and window are **policy**; Lampaert stops when the last costs of consecutive chains lie within an interval whose width and chain count the user sets, L4971–4975; the earlier Philis trial used 10 flat chains, `dp/lib.rs:297-298`), or (c) `Schedule::max_temps` temperatures have run (proposal cap). No wall-clock stop inside dp: it would break T8 determinism (FLOW-08's `max_wall` is checked between epochs). If every probe move has `ΔE ≤ 0`, `T0 = 1e-12` (guard; the formula gives 0). Rule (b) precisely: `|best_E(t) − best_E(t−5)| ≤ 1e-3·max(|best_E(t−5)|, 1e-12)`.
- Tests (`backend/dp/src/anneal.rs`):
  - `sf_moves_preserve_sf`: 10,000 random M1–M3 on random symmetry nodes (p ≤ 4, s ≤ 2) → `is_sf` after every move.
  - `anneal_is_deterministic`: same input and seed → identical `Tree`, `variant`, `orient`, `Layout`.
  - `quench_never_returns_worse_theta_than_start` (adapt `tests.rs:235-257`).
  - `schedule_stops_early_on_a_flat_landscape`: no nets, no rules, two identical 1000×1000 nm cells, gap table all 0 (every code decodes to 2000×1000 or 1000×2000, same area; rotation of a square changes nothing; so ΔE ≡ 0 including the area term) → `PlaceStats::temps == 6` (stops by rule (b) after the 5-temperature window; `temps` counts temperatures run, the first at index 0).
  - `area_term_is_bbox_over_cell_area` (**derived** values): four 1000×1000 nm cells, no nets, no rules, every `Gap::min = 200`, `abut = false`. Code α = ABCD, β = CDAB decodes to a 2×2 block (A left of B, C left of D, C below A, D below B) of 2200 × 2200 nm → area term `4.84e6 / 4e6 − 1 = 0.21`; code α = β = ABCD decodes to one row of 4600 × 1000 nm → `4.6e6 / 4e6 − 1 = 0.15`. Assert both E values to ±1e-6 (the formula, not a preference: the row is smaller here because only three gaps are paid).
- Acceptance (includes the former PLC-08 fixture acceptance): with `DpMode::Sp` on all 10 fixtures × seeds 1–5: T1 all zero (overlap, clearance residue, symmetry, lattice offenders; `stats.place.overlap_nm2 == 0.0` asserted in `placement_metrics.rs`); signoff DRC of the winner ≤ `Flat`; lex key better or equal on ≥ 8/10 fixtures (median over seeds); T2 area usage ≤ `Flat`. Schedule A/B: the adaptive schedule vs a fixed schedule with equal proposals, lex key equal or better on ≥ 8/10 fixtures at ≤ 50 % dp wall time. The earlier negative trial (`dp/lib.rs:296-300`) was on the flat representation, so it is re-measured here, not assumed.

### PLC-10 Warm start, incremental evaluation, undo log
- Priority: P1. Effort: M. Depends on: PLC-08, PLC-09.
- Why: AP-09, AF-06, AP-08, AP-21, AR-17; PLAN §1 wants the topological representation carried between iterations (audit-04 G11).
- Current: cold gp restart every epoch (`lib.rs:575`, `mechanics.rs:361-371`); full re-evaluation and 8-column clone per trial (`dp/lib.rs:36-77`, `:147-179`); most placement rules lack `touches`. The epoch loop's warm/cold choice (`COLD_EVERY = 4`, history reset, escalation) is FLOW-08's; this item supplies the dp side FLOW-08 calls.
- Change:
  0. **Flat-path schedule (no dependency; land first so FLOW-08 is not blocked on PLC-08):** in `dp/lib.rs`
     ```rust
     #[derive(Clone, Copy, Debug, PartialEq)]
     pub struct Schedule { pub range0: f32, pub max_temps: u32, pub t0_scale: f64, /* SP path (PLC-09): */ pub p0: f64, pub alpha: f64, pub moves_per_kid: u32 }
     impl Schedule {
         /// Today's constants (`dp/lib.rs:21-27`, `t0 = 0.02·mean|ΔPEX|` at `:276-289`); SP: P0 0.6, α 0.9, 20 moves per kid.
         pub fn cold() -> Self { Self { range0: 0.4, max_temps: 220, t0_scale: 0.02, p0: 0.6, alpha: 0.9, moves_per_kid: 20 } }
         /// FLOW-08 step 3 values for the flat path [policy, measure]; SP: P0 0.1.
         pub fn warm() -> Self { Self { range0: 0.05, max_temps: 60, t0_scale: 0.002, p0: 0.1, alpha: 0.9, moves_per_kid: 20 } }
     }
     ```
     The flat `place` reads `range0`, `max_temps` and `t0_scale` instead of `RANGE0`, `MAX_ITERS` and the literal `0.02`; test `cold_schedule_reproduces_todays_layout` (a characterization test written **before** the refactor: run today's `dp::place` on `rotate_bench()` (`dp/src/tests.rs:48`) with seed 7, paste the resulting `l.x`, `l.y`, `l.orient` as literals into the test, then refactor and assert `Schedule::cold()` reproduces them exactly).
  1. SP warm start: `Epoch` (`lib.rs:541-556`) gains `tree: Option<sp::Tree>` (the tree `dp::place` returns under `DpMode::Sp`); FLOW-08's `Warm` maps to `dp::Start::Warm { tree, variant, orient }` from the incumbent, its `Cold` to `Start::Cold(&gp_layout)` or `Start::Constructive` per PLC-11's decision. A warm start whose tree no longer matches the requirements (cell count or symmetry groups changed after an escalation) falls back to `Cold` inside `dp::place` (checked by `Tree::build` on the same inputs giving the same node structure).
  2. `touches` overrides (PLC-owned): `Proximity`, `Isolation`, `DtiBand` push both device ids; `Utilization` pushes none (global); `SymmetryGroup::touched` forwards to its `Vec<Symmetry>` (`symmetry.rs:103-131`). MAT adds `MatchingPair`, `CentroidGroup`; `ThermalGradient` stays global (it reads the whole field).
  3. `dp::eval::Eval` caches per-net bbox and weighted HPWL, per-batch `(cost, residual, violations)`, and `cell_batches: Vec<Vec<(Tier, u16)>>` built from `RuleBatch::touched`; batches with no touched ids are global. After each decode, `moved` = cells whose `(x0, y0, w, h, orient, variant)` changed (O(n) compare); recompute nets in `∪ cell_nets[moved]` and batches in `∪ cell_batches[moved] ∪ global`.
  4. Undo log `Vec<(u16 cell, i32, i32, i32, i32, Orient, u16)>` for moved cells plus the node's swapped indices; reject = replay in reverse (replaces `Snap`, `dp/lib.rs:36-77`). Decode caches each node's relative result and recomputes only the dirty node and its ancestors.
- Tests: `incremental_matches_full_evaluation` (2,000 random moves on a 14-cell synthetic with 3 rule kinds: `|E_inc − E_full| ≤ 1e-6·|E_full|` after every accepted move); `warm_start_at_zero_temperature_never_worsens_the_incumbent_key`; `kernel/analog/src/placement/*` `touches_names_both_targets` for each PLC-owned rule.
- Acceptance: T7 proposals/s ≥ 3× baseline on `dac4` and `examples/three_stage_opamp`; bench wall ≤ baseline; lex key equal or better on ≥ 8/10 fixtures.

### PLC-11 gp ablation and seeding decision
- Priority: P1. Effort: M. Depends on: PLC-01, PLC-09 (`GpMode::Constructive` and any SP comparison need PLC-09's SP `dp::place`; PLC-08 alone has no anneal).
- Why: AP-16, AP-22; BAL1-24 (compare at equal wall time); BAL1-60 (deterministic constructive seeds).
- Current: `gp::place` (`gp/lib.rs:181-328`) untested; its layout is a random pile pushed by weak gradients.
- Change: run `GpMode::{Analytic, Pile, Constructive}` × 10 fixtures × seeds 1–5 at equal epoch counts; record final lex key, T2, dp wall. **Decision rule:** if `Constructive` or `Pile` is not worse than `Analytic` on ≥ 8/10 fixtures, delete the gradient loop (`gp/lib.rs:165-176`, `:198-323`, `bin_overflow` `:331-337`) and keep `Prices`, `net_weights`, `mechanics`; otherwise keep gp as the SP seeder (`Tree::seed_from`) and add `gp_spreads_a_pile` (16 equal cells → `bin_overflow ≤ 0.15`).
- Tests: `gp_modes_are_deterministic` (PLC-01).
- Acceptance: the decision and the ablation table go into `docs/CRATES.md` (gp section).

### PLC-12 Symmetry-island and cluster connectivity metric and budget
- Priority: P1. Effort: S. Depends on: PLC-08.
- Why: BAL1-27 (Def. 2.1, L3187–3193), BAL1-41 (hierarchical clusters placed connected, L4376–4418), SURV-04 (perf_driven_survey.txt L71–75 right col.), Hastings rule 8 proximity (L42499–42503).
- Current: no island or connectivity check; `Proximity` is pairwise (`proximity.rs:19-25`).
- Change: `kernel/analog/src/placement/island.rs`:
  ```rust
  pub struct SymmetryIsland { pub members: Vec<Target>, pub touch_nm: i32 }
  impl RuleBatch<Layout> for SymmetryIsland { /* residual = components − 1; cost = residual; kind "SymmetryIsland" */ }
  ```
  Adjacency of members a, b (cell ids): `|x_a − x_b| < hw_a + hw_b` (strictly overlapping x-projection) and `|y_a − y_b| − (hh_a + hh_b) ≤ touch_nm`, or the same with x and y exchanged. Integer arithmetic on `Layout` columns, not `Layout::edge_gap` (`kernel/core/src/layout.rs:170-177` returns a Euclidean `f32` that is 0 at a corner touch, which must not count). Components by `pnr_core::UnionFind`. `touch_nm` = the largest `Gap::min` (PLC-07) over member pairs and faces + lattice, filled by the library (members are cells after retarget). `touched` pushes every member (PLC-10). Emitted in `solve()` next to `Utilization` (`lib.rs:264-266`), one per symmetry axis with ≥ 2 distinct cells, into `placement.budget` and `placement.cost`. `PlacementMetrics::islands_extra` (PLC-01) = Σ over these batches of `components − 1`; proximity nodes are reported the same way in a second field `clusters_extra: u32`, metric only.
- Tests: `abutting_pair_is_one_island`, `a_gap_splits_the_island`, `diagonal_corner_touch_is_not_adjacent`.
- Acceptance: T4 ≥ 90 % = share of `SymmetryIsland` batches with residual 0 in the winners' placement reports, all fixtures × seeds 1–5.

### PLC-13 WPE and extraneous-poly keep-outs as pairwise spacing
- Priority: P1. Effort: S. Depends on: PLC-07; MAT-07 (`MatchClass`, `mos_env`); classes from EXT-16 (until then every matched set is `Moderate`, MAT-07's default).
- Why: H13-53 and H13-28 (Hastings WPE: in one case a 5 % current mismatch at 1.8 µm gate-to-well-edge spacing, rising to 25 % at 0.95 µm, L41302–41306, values blank in the reftext and read from PDF 696; rule 19, L42625–42631); H13-55 (rule 23, L42648–42654); AR-09 and AF-23 (placement never sees WPE).
- Current: matched PMOS cells inflate their own well by `wpe_clearance_moderate` (`kernel/cells/src/mosfet.rs:626-630`); no rule keeps a **foreign** nwell away from matched NMOS; `Environment` is measured after dp in the routing arm (`lib.rs:431`, `:680`, `:806-876`). The sidecar already holds tiered `wpe_clearance_nm: [2000, 3000, 5000]` (`pdks/sky130.json:59-63`) that no code reads.
- Change: `Profile` (PLC-07) gets `matched: Option<analog::matching::class::MatchClass>` (MAT-07: `Minimal | Moderate | Exceptional`) and `set: Option<u16>` (the cell's PLC-03 orient-set index). A cell is matched if its units have > 1 owner (the test at `lib.rs:417-418`) or it is in a PLC-03 orient set; class from MAT-07's `MatchedSet.class`, default `Moderate` (**policy**; Hastings: moderate matching "is satisfactory for the input differential pairs of general-purpose operational amplifiers and comparators", L42341–42342). `SpacingTable` gains `wpe: [i32; 3]` = `mos_env(c, MatchKind::Voltage, pdk).wpe_nm` for the three classes (sky130: 2000 / 3000 / max(5000, 2·`n_well_depth` = 4000) = 5000, `pdks/sky130.json:59-63`, `:68`) and `foreign_poly: [i32; 3] = [0, 3000, 5000]` (rule 23 lower ends; the values are blank in the reftext, L42648–42654, and were read from the PDF by the H13 study, H13-55). In `gap(a, fa, b)`, when `a.matched = Some(c)` and `b.set != a.set` (symmetric in a, b): `need(diff_in|diff_out of a, nwell of b) ≥ wpe[c] − insets` and `need(diff_* of a, poly of b) ≥ foreign_poly[c] − insets`; a PMOS cell's own well halo is CELL-12's, not this rule's.
- Tests (`backend/gp/src/spacing.rs`): `matched_nmos_keeps_wpe_distance_from_foreign_well` (`diff_out` inset 200, foreign nwell inset 0, class Moderate → `min = 3000 − 200 − 0 = 2800`); `unmatched_nmos_uses_the_deck_rule` (→ `340 − 200 = 140`); `same_set_partners_are_exempt` (both cells `set = Some(0)` → deck rule only).
- Acceptance: the WPE-nearness part of T9 (`wpe_min_nm / d > 1` for no matched member, `environment.rs:39`) is 0 on every fixture; T2 cost ≤ 5 % vs PLC-07 alone. The skew parts of T9 (WPE and OSE distance skew > `ENV_TOL = 0.2`, `environment.rs:7`, `:34-44`) are not controlled by a keep-out; they are PLC-29's.

### PLC-14 Thermal-aware placement: power separation per milliwatt
- Priority: P1. Effort: S. Depends on: PLC-06, PLC-03, MAT-07 (class).
- Why: H13-49 (Hastings rule 14: "a separation of at least a micron per milliwatt should be maintained, but in practice a half or even a quarter of this distance will usually suffice"; minimally matched devices may be adjacent to small power devices if common-centroid, L42569–42589, PDF 715); H08-47 (L23878–23942); H09-16; LAMP-13 (distributed-source thermal profiles and superposition, lampaert.txt L4768–4871) and LAMP-14 (sensitive pairs placed symmetric about the class-AB output transistors, L5289–5330); BAL1-03 (L661–670).
- Current: `ThermalGradient` budgets ΔT between members (`thermal.rs:7-35`); no source-distance rule; gp saw no power (fixed by PLC-06).
- Change: `kernel/analog/src/placement/heat.rs`:
  ```rust
  pub struct HeatSeparation { pub victim: Target, pub source: Target, pub min_gap_nm: i32 }
  // Rule<Layout>: satisfied ⇔ edge_gap ≥ min_gap; residual = (min_gap − gap)⁺ / min_gap; cost = residual²; touches both.
  ```
  Emitted in `solve()`: sources = cells with `cells.power[c] ≥ cfg.heat_source_uw` (new `Config` field, default 1000 µW, **policy**); victims = cells of PLC-03 orient sets not containing the source, class = the set's MAT-07 `MatchClass`; `min_gap_nm = k(class) · P_mW · 1000` with `P_mW = cells.power[source] / 1000`, `k(Minimal) = 0` (no rule emitted), `k(Moderate) = 1.0`, `k(Exceptional) = 1.0` (rule 14 gives 1 µm/mW for moderately matched common-centroid devices near small power devices and lets minimally matched common-centroid devices sit adjacent, L42584–42587; `k(Exc)` is **policy**, since rule 14 asks only for "large separations" and die-axis placement, L42582–42584, which need a die context no input provides (see appendix, PLC-23); `k(Min) = 0` assumes a common-centroid layout). Budget + cost arms. dp keeps refreshing the field once per temperature (`dp/lib.rs:346`).
- Tests: `heat_separation_scales_with_power` (2000 µW source, Moderate victim → `min_gap_nm == 2000`); `dp_pushes_a_matched_pair_away_from_a_hot_cell` (seeds 1..=10: residual 0 at exit).
- Acceptance: on every fixture where the op point yields a cell ≥ `heat_source_uw` (`Config::op`, `frontend/library/tests/perf_postlayout.rs` setup), over seeds 1–5: all `HeatSeparation` residuals 0, mean |ΔT| of matched pairs (MAT's thermal ledger / `ThermalGradient` usage) with the rule ≤ without, area ≤ +5 %. Fixtures without such a cell emit no rule (no change).

### PLC-15 Routing-space reservation: current halos and congestion feedback
- Priority: P1. Effort: M. Depends on: PLC-08; RTE-25 (`RouteStats::congestion`, `LatticeSpec`).
- Why: AP-17; LAMP-31 (per-side expansion `Δint = static + dynamic`, static = Σ terminal wire widths on that side, eqs 4.4–4.5, lampaert.txt L3944–4030; net-bbox dynamic estimate "consistently and significantly better", eq 4.35, L4874–4912); LAMP-47 (area penalty from overestimated routing area, L7094–7114); NOTES-41; H15-09; BAL2-21.
- Current: only ring halos are reserved (`lib.rs:1195-1208`); gr computes per-gcell usage against `gcell_capacity: 6` (`backend/gr/src/lib.rs:33`, `:203`) and the flow discards gr's report (`lib.rs:617-618`); pin currents exist (`lib.rs:310`).
- Change:
  1. Static halo per cell face: `halo_st[c][f] = Σ_{pins p of c nearest to face f, I_p > I_1} round_up(I_p / J_max − w_min, lattice)`, with `I_p` the pin's DC current in µA (`r.cfg.pin_ua`, built by `pin_currents`, `lib.rs:310`, `:1035`; empty without an op point → no static halo), "nearest face" judged from the pin rect's centre in the variant's R0 frame and permuted with `orient` like the profiles, `J_max` = `Limit::ua_per_um / 1000` (µA/nm) of the lowest routing metal that has an EM limit in `elaborate::em_limits` (`frontend/library/src/elaborate.rs:248`; sky130: met1, li has none), `w_min` that layer's `pdk.min_width`, `I_1 = J_max·w_min` (a pin a minimum-width wire can carry gets no halo). Computed once per run per (cell, variant) in `solve`. Philis routes minimum-width signals over cells, so only width beyond one track is reserved (**policy**; LAMP's static term reserves the full current-derived width of every terminal wire, L4012–4024 [UNVERIFIED: that LAYLA forbids over-device routing is not stated in the Lampaert reftext]).
  2. Dynamic halo from RTE-25's `RouteStats::congestion: Vec<(Rect, f32)>` (16×16 regions, `demand` = usage per on-track node; > 1 = over capacity) and `LatticeSpec::p0` (sky130 420 nm, RTE-10). After each epoch, for each cell face f: `d_f` = max demand over regions intersecting the band of width `p0` just outside the face (placed coordinates of that epoch); `n_f` = number of the cell's pins nearest f (≥ 1); `target = round_up((d_f − 1)⁺ · n_f · p0, lattice)`; `halo_dyn[c][f] ← round_up(0.5·halo_dyn[c][f] + 0.5·target, lattice)`, capped at `4·p0` (the formula, β = 0.5 and the cap are **policy**, motivated by BAL2-19's warning that naive raise/lower weight updates "could oscillate indefinitely", L9401–9405, and RTE-25's note that PLC must smooth the map across epochs). Kept in `Flow` (`RefCell<Vec<[i32; 4]>>`), reset on an assignment change.
  3. `Geo::halo = halo_st + halo_dyn` enters every gap in PLC-08 (sum over the two facing faces; a nonzero halo also forbids abutment, PLC-08 step 2).
- Tests: `static_halo_reserves_only_excess_width` (I = 5 mA, J = 2.8 µA/nm (sky130 met1 2.8 mA/µm), w_min 140 → round_up(1785.7 − 140, 10) = 1650 nm); `dynamic_halo_decays_without_overflow` (halo 800, two epochs with every `d_f ≤ 1` → 400 → 200); `dynamic_halo_is_capped` (d_f = 10, n_f = 4 → capped at 1680 on sky130).
- Acceptance: T6 `route_hard + route_overuse` ≤ baseline on every fixture; T2 footprint increase ≤ 5 % vs PLC-08 without halos.

### PLC-16 Performance-driven placement term (MST parasitic estimate)
- Priority: P1. Effort: M. Depends on: PERF rows (today's `perf_rows`, `lib.rs:167`; PERF-12 adds R and coupling later). Works in the flat and the SP path (it is an ordinary priced budget).
- Why: LAMP-04 (direct performance-driven placement, lampaert.txt L1536–1598), LAMP-06 (eqs 4.9–4.12, L4460–4499), LAMP-07 (MST over edge-to-edge Manhattan terminal distance, L4510–4612), LAMP-08 (eqs 4.13–4.22, L4614–4715), LAMP-34 (placement leaves margin for routing, L5193–5274); AP-17; BAL2-21.
- Current: sensitivities reach placement only as HPWL weights (`gp/lib.rs:137-153`; `lib.rs:293-296`); `PerformanceBudget` is a routing-tier rule (`lib.rs:267-269`, `kernel/analog/src/routing/performance.rs:8-33`) built with `af_per_um / 1000` (`lib.rs:204`, `:218`), `af_per_um` being the first routing layer's wire capacitance at its minimum width (`lib.rs:494-502`).
- Change (a plain `RuleBatch<Layout>`, so `Prices`, the gate, the report and both placers handle it without dp changes):
  1. New `kernel/analog/src/placement/perf.rs`:
     ```rust
     /// One spec's placement-time parasitic estimate (Lampaert §4.4): Σ w_i·MST_i·C/nm ≤ (1 − reserve).
     #[derive(Clone)]
     pub struct PlacePerf {
         pub metric: String,
         pub nets: Vec<NetId>, pub w: Vec<f32>,   // same nets and weights as the PerformanceBudget row
         pub af_per_nm: f32, pub reserve: f32,
         /// Per net: its items `(cell, per-variant pin box (dx, dy, hx, hy) relative to the bbox centre, R0)`.
         pub items: Vec<Vec<(u16, Vec<(i32, i32, i32, i32)>)>>,
     }
     ```
     Item position: `(l.x[c], l.y[c]) + orient.apply(dx, dy)`, half size `(hx, hy)` swapped when `orient.swaps_axes()`, for `v = l.variant[c]` (the pin box = union of the cell's pin rects on that net in alternative v, as `Nets::from_macros` averages them, `gp/mechanics.rs:79-102`).
  2. `fn mst_len(&self, n, l) -> i64`: Prim over the net's items (≤ ~10 per net), `d(a, b) = (|Δx| − (hx_a + hx_b))⁺ + (|Δy| − (hy_a + hy_b))⁺` (edge-to-edge Manhattan, LAMP-07).
  3. `used = Σ_i w_i · mst_len_i · af_per_nm`; `residual = (used / (1 − reserve) − 1)⁺` (fraction of the placement's share of the headroom, the `Rule::residual` convention); `cost = residual`; `kind = "PlacePerf"`; `touched` = every item cell; `reserve = 0.2` (**policy**; calibrate per spec as §5 Q5). Two-sided specs arrive as two rows from PERF-06 (LAMP-01).
  4. Library: in `solve()` next to the routing rows (`lib.rs:267-269`), for each `PerformanceBudget` row push one `PlacePerf` into `problem.placement.budget`, built after `CellSpace::new` from `cells.variants` pins (so after `lib.rs:275`); `Config::place_perf: bool` (default `true`, the A/B switch for T5).
- Tests (`perf.rs`, and `backend/dp/src/tests.rs` for the last): `mst_equals_hpwl_for_two_points` (two point pins → MST = |Δx| + |Δy|); `abutting_pins_have_zero_mst`; `rotated_pin_box_follows_orient`; `a_row_shortens_its_sensitive_net` (mean MST of the weighted net over seeds 1..=10 lower with the row than without, flat dp).
- Acceptance: T5 (LAMP-33 protocol: comparator offset 3.7 mV and delay 2.8 ns with the mechanism vs 6.9 mV and 5.4 ns without, both specs < 5; Table 4.1, lampaert.txt L5006–5012, PDF 118; protocol §4.12.1, L5027–5046): on fixtures with specs, spec miss with rows ≤ without on every one and strictly lower on ≥ 1; area ≤ +5 %.

### PLC-17 Current-weighted supply nets
- Priority: P2. Effort: S. Depends on: none. (The per-epoch weight feedback this item used to carry is PERF-25's; see appendix.)
- Why: AP-18 (rails keep weight 1 while budgeted nets are normalised to mean 1, so a loose-budget signal net can pull less than a supply), EM-11 ("minimize current flows", lienig_em.txt L3457–3462), AP-17.
- Current: weights computed once per run (`lib.rs:296`); unweighted nets, rails included, keep weight 1 (`gp/lib.rs:150-152`).
- Change: `gp::net_weights(classes, sens, current_ua: &[Option<f64>])` (`gp/lib.rs:136-153`; `current_ua[NetId]` from `oppoint::net_current_ua`, already computed at `lib.rs:285` for IR budgets, empty without an op point): nets of class Supply or Ground get `clamp(I_net / I_ref, 0.1, 1.0)` with `I_ref` = the largest Supply/Ground net current; `0.25` when the current is unknown (**policy**: a rail's HPWL matters for IR/EM, not for C); every other net keeps today's normalisation. The routers' mask (`lib.rs:312-316`) already zeroes unbudgeted nets, so routing is unchanged.
- Tests (`gp/lib.rs` `weight_tests`): `supply_weight_scales_with_current` (two supplies at 100 µA and 1000 µA → 0.1 and 1.0; unknown → 0.25); `signal_weights_are_unchanged` (the existing `a_tighter_budget_pulls_harder_and_sensitivities_win`, `gp/lib.rs:474-485`, passes unchanged with `current_ua` empty: its unbudgeted net is `NetClass::Signal`, not a rail).
- Acceptance: over seeds 1–5 on op-point fixtures, REL-04's IR Θ and median extracted C of budgeted nets no worse than baseline.

### PLC-18 Dimensionless placement objective
- Priority: P1. Effort: M. Depends on: none; MAT converts its own rules.
- Why: AR-08 (raw costs in nm², 1e-3, 3e5, fractions summed with HPWL nm), AP-19 (T0 set by the largest-unit term).
- Current: PEX = `hpwl(nm) + analog_cost` (`dp/lib.rs:120-122`; `mechanics.rs:247-256`); `Proximity` cost `ex²·1e-3` (`proximity.rs:31-36`), `Isolation` `4e-3·shortfall²` (`isolation.rs:23-29`), `Symmetry` `(ex² + ey²)·1e-3` (`symmetry.rs:19-24`).
- Change: HPWL term `/ L_ref` (`L_ref = sqrt(Σ cell area)`, nm); PLC-owned costs: `Proximity = (excess / max_distance)²`, `Isolation = (shortfall / min_distance)²`, `DtiBand = (miss / band)²`, `HeatSeparation`, `SymmetryIsland` as above, `Symmetry = ((|ex| + |ey|) / L_ref)²` (flat path only). MAT converts `MatchingPair`, `CentroidGroup`, `ThermalGradient`.
- Tests: `energy_is_invariant_to_scaling_all_lengths` (scale every coordinate, dimension and distance limit by 2 → E unchanged within 1e-6 relative).
- Acceptance: at T0 on `ota`, no single term contributes > 80 % of `mean |ΔE|` over the T0 probe moves (**policy**); measured once with a temporary per-term split of the probe loop (`dp/lib.rs:276-289`, or PLC-09's probe) and recorded in `docs/CRATES.md` (dp section), not committed as code.

### PLC-19 Substrate-aware placement hooks — cut (see appendix)
- Status: **cut**. (1) the injector-axis cost is REL-15's `SubstrateBalance` (same SUB-43 source, same geometry); (2) per-pair `Isolation` distances are REL-09's, consumed unchanged through the rule seam; (3) the optional-ring branch has no producer (REL-07 decides rings per device, not as a search variable). The ID is kept so cross-references stay valid.

### PLC-20 Horizontal axes, nested groups, cross-stage axes
- Priority: P2. Effort: M. Depends on: PLC-08; EXT-14 (`Compound` with `dir` and nested `set_pairs`).
- Why: AP-12; BAL1-01 (vertical/horizontal axes, eqs 2.1/2.2, L3134–3152), BAL1-12 (embedded groups, L2327–2371), BAL1-38 (alignment of several groups, L4164–4230), BAL1-40 (hierarchical symmetry, L4361–4372), BAL1-51 (symmetry compounds spanning blocks, L5230–5333).
- Current: vertical only (`symmetry.rs:7-16`); one axis per top-level block with nested leaves flattened (`emit.rs:139-145`).
- Change: `Symmetry { .., dir: AxisDir }` with `#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)] enum AxisDir { #[default] V, H }` in `kernel/analog/src/placement/symmetry.rs` (EXT-14's `dir` maps onto it; every existing struct literal gets `dir: AxisDir::V`: 13 sites, `backend/annotator/src/emit.rs` 2, `backend/dp/src/tests.rs` 2, `kernel/analog/src/placement/dti.rs` 1, `kernel/analog/src/placement/symmetry.rs` 8, from `grep -rn 'Symmetry {' --include='*.rs'` excluding `SymmetryGroup`; PLC-21's `mode` touches the same 13); `error` for `H` is `(x_a − x_b, y_a + y_b − 2·axis)` and `Layout::axis[id]` then holds the axis **y**; `Symmetry::project` for `H` mirrors in y. The decoder transposes an `H` symmetry node (swap `w/h`, faces `L↔B`, `R↔T`, run PLC-08 steps, transpose back). A symmetry node's kid may be a node: self-symmetric if its own axis offset is placed on the parent axis (`C_c` = twice the child's absolute axis), or one half of a node pair (mirror of the whole sub-island). Compounds: EXT emits one `AxisId` per compound, which already yields one symmetry node.
- Tests: `horizontal_group_mirrors_in_y`; `nested_self_symmetric_node_centres_its_axis_on_the_parent_axis`.
- Acceptance: on EXT's fully differential fixtures, one axis per compound and zero symmetry violations.

### PLC-21 Mirror versus perfect symmetry, gated by orientation Φ
- Priority: P1 (RTE-15's exact mirrored routing maps pin `a` to `map(pin a)`; with translated partners the pins are not mirror images, so the joint search falls back). Effort: M. Depends on: PLC-03; MAT-01 (`mirror_allowed`); EXT-14 (`Compound.kind`); consumer RTE-15.
- Why: BAL1-02 (mirror/perfect/self, L676–712), H13-25 and NOTES-22 (Φ = (1/N)ΣΦ_i equal in both axes, eq 13.61, hastings.txt L42162–42187, PDF 709), AF-19 (mirroring flips odd-finger current direction), AR-27 (no `compose`), RTE-15 step 4 (pin pairing by `map(rect)`).
- Current: dp never introduces a mirror (`dp/lib.rs:541-553`), so only perfect symmetry exists, by accident of R0 seeding.
- Change:
  1. `kernel/core/src/geom.rs`: `impl Orient { pub fn then(self, next: Orient) -> Orient; pub fn inverse(self) -> Orient }` with `a.then(b).apply(p) == b.apply(a.apply(p))`, implemented by a `const [[Orient; 8]; 8]` table generated in the test below (no trig).
  2. `Symmetry { .., mode: SymMode }`, `#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)] enum SymMode { #[default] Perfect, Mirror }`. `satisfied` (PLC-03 step 6) requires `orient_b == orient_a` for Perfect and `orient_b == orient_a.then(Orient::Mx180)` for Mirror (`Mx180` is `(x, y) → (−x, y)`, `geom.rs:52`: the reflection about a vertical axis applied after `a`'s own transform).
  3. Mode per pair, decided once in `solve()` after `CellSpace::new`: `Mirror` iff EXT-14's compound kind is Mirror **and** `analog::matching::moments::mirror_allowed(&[sums(a), sums(b)])` (Φ_H = 0 for every member, MAT-01; the units come from `cells.units` at variant 0, R0); else `Perfect`. Until EXT-14 lands every pair is `Perfect` (today's behaviour).
  4. dp: seeding sets `orient_b = orient_a.then(Mx180)` for Mirror pairs; M4 (and the flat `try_rotate_set`) turns `a` and re-derives `b` from it instead of turning `b` by the same quarter; the PLC-07 profiles of `b` are read at its own orient, so the decoder needs no change.
- Tests: `then_matches_apply_on_all_64_products` (for all 8×8 pairs and the point (1, 2): `a.then(b).apply(1, 2) == b.apply(a.apply(1, 2))`; `a.then(a.inverse()) == R0`); `mirror_is_refused_for_an_odd_finger_pair` (3 fingers +,−,+ → Perfect); `mirror_pair_satisfies_only_with_mx180_partner` (`symmetry.rs`).
- Acceptance: no Φ mismatch reported by MAT-05's `OrientationSet{Phi}` on any fixture; on `ota`, `ota_constrained`, `tt_ota` RTE-15 reports its "pairs exact" column ≥ the Perfect-only run.

### PLC-22 Island-level guard-ring reservation
- Priority: P2. Effort: M. Depends on: PLC-08; REL-07 (shareable Victim rings, "one ring per matched pair, SUB-39"); `post_cell::guard_rings` already merges same-class shareable requesters within `guard_ring_merge_gap_nm` (`post_cell.rs:26-69`).
- Why: AP-23; SUB-39 (one shared ring around both halves of a differential pair, charbon_substrate.txt L3734–3785); BAL1-32 (well/guard-ring white space at packing, L3685–3691); PL-17.
- Current: every ring requester's alternatives are inflated by the full halo (`lib.rs:1195-1208`); merged rings then enclose wasted per-cell halos (`post_cell.rs:1-5`).
- Change: when every member of a symmetry group carries a shareable ring requirement of one class (`connection_net`, `ring_type`), `CellSpace::new` skips the per-cell halo inflation (`lib.rs:1195-1208`) for those members and records `node_halo[axis] = ring_halo(rep)`; PLC-08 adds `node_halo` to the symmetry node's four faces when it becomes a kid of its parent (the node profile's insets grow by it). `guard_rings` then merges the members into one ring (they sit within the merge gap by construction) around the node's inner box. Until REL-07 emits shareable Victim rings, no fixture triggers this (OTA-class fixtures draw 0 rings after REL-07).
- Tests: `island_ring_reserves_once` (two ringed partners: node bbox = members' bbox + 2·halo, not + 4·halo between them).
- Acceptance: area of ringed matched groups lower than per-cell halos with the same ring count or fewer.

### PLC-23 Die-context stress and heat zones — deferred (see appendix)
- Status: **deferred**. No input carries a die outline, block origin or package (`library::run` takes none; MAT §0.3 lists die-level stress as not planned for the same reason), so the rule would report *unknown* on every fixture and change nothing measurable. Sources and the proposed rule are kept in the appendix.

### PLC-24 A reachable utilization floor
- Priority: P1 (bug: an unreachable floor keeps Θ > 0 and λ saturated, AP-02). Effort: S. Depends on: PLC-07.
- Why: AP-02 (floor 0.6 can be unreachable: a 2×2 µm cell pitched at 3.27 µm fills 37 %).
- Current: `min_utilization = 0.6` default (`lib.rs:76`), pushed as a budget (`lib.rs:264-266`) before cells exist.
- Change: move the push to after `CellSpace::new` (`lib.rs:275`) and use `u_eff = min(cfg.min_utilization, 0.9 · Σ_i A_i / Σ_i (w_i + ḡ)(h_i + ḡ))`, with `A_i = w_i·h_i` of each cell's assigned-variant-0 bbox and `ḡ` the median over ordered cell pairs of PLC-07's x-facing `Gap::min` at variant 0, R0 (**policy**: 0.9 leaves room for routing halos), so the floor is never unreachable by construction. `Utilization { u_min: u_eff }` (`kernel/analog/src/placement/utilization.rs:13-15`) is unchanged.
- Tests: `frontend/library/src/lib.rs` `start_tests::utilization_floor_is_reachable_for_tiny_cells` on a pure helper `fn u_eff(u_min: f32, cells: &[(i32, i32)], g: i32) -> f32`: four 2000×2000 nm cells, `ḡ = 270` → `u_eff = 0.9·4/5.1529 = 0.699`, so `min(0.6, 0.699) = 0.6`; with `ḡ = 1270` → `0.9·4/10.69 = 0.337`.
- Acceptance: no fixture ends with a positive `Utilization` residual caused by the floor alone (check: rerun the fixture with `min_utilization = 0` and compare the footprint; the floor may bind only when that footprint exceeds `Σ A / u_eff`).

### PLC-25 Flow-order and boundary costs
- Priority: P2. Effort: M. Depends on: EXT-28 (`Intent.order: Vec<(Vec<u32 /*group ids*/>, AxisDir)>`, "PLC implements the ordering cost"), FLOW-07 (ports, AF-02); PLC-20 (`AxisDir`).
- Why: SURV-07 monotone current flow (perf_driven_survey.txt L111–113 right col.), SURV-08 signal flow (L113–117), SURV-06 boundary constraints (L77–79), H14-40 crossing count (Hastings §14.3.1, hastings.txt L44076–44082, PDF 743–744).
- Current: none (no current-flow, stage-order or boundary rule; `grep Boundary` has no hits per SURV-06).
- Change: `kernel/analog/src/placement/order.rs`: `FlowOrder { chain: Vec<Target>, dir: AxisDir, weight: f32 }` and `Boundary { cell: Target, side: u8 /* 0 L, 1 B, 2 R, 3 T */ }`. `FlowOrder` cost = `weight · Σ_k ((c_k − c_{k+1})⁺) / L_ref` along the chain, with `c` the group-bbox centre coordinate on `dir` (y for V: bottom-up, as EXT-28's current paths run from ground up) and `weight` = path current / max path current (1.0 without an op point); `Boundary` cost = distance from the cell's `side` face to the same face of the layout footprint, / L_ref. Both `touched` = their cells; cost arm only (no budget: the sources give no threshold), emitted in `solve()` only when EXT-28 / FLOW-07 supply the data.
- Tests: `ordered_chain_costs_zero`; `boundary_cell_on_the_edge_costs_zero`.
- Acceptance: A/B no lex-key regression; H14-40 crossing count (RTE metric) lower on ≥ half the fixtures with current paths.

### PLC-26 Cross-cell diffusion abutment — deferred (see appendix)
- Status: **deferred**. It needs per-variant edge-diffusion descriptors and merged cross-cell drawing from CELL, and no CELL item provides them (CELL-20 is butting inside one cell); `cellgen` already merges diffusion inside abutment groups, which EXT-22 widens to every legal same-kind, same-bulk, shared-S/D group. Revisit when PLC-07 + EXT-22 leave a measured area gap to hand layout (T3).

### PLC-27 Delete the legacy flat path
- Priority: P2. Effort: S. Depends on: PLC-08 accepted, PLC-10.
- Why: AP-20; ponytail note in `legalize.rs:15-16`.
- Change: delete `backend/dp/src/legalize.rs`, `project_hard` (`dp/lib.rs:369-397`), the flat moves and `Snap` (`:34-77`, `:409-620` except reshape logic reused by M5), `DpMode::Flat` and the flat-only `Schedule` fields (`range0`, `t0_scale`), the `abutment` table (`lib.rs:1103-1104`, filled at `:1187`; unread since PLC-06), `PlaceRules::encroach`/`encroachment` if only the flat path used them (PLC-01's metric then reads `sp` decode results or keeps its own copy); delete `SymmetryGroup::project`/`Symmetry::project` if no caller remains (`grep -rn '\.project(' backend kernel frontend`); update `docs/CRATES.md` (dp, gp sections and Open issue 4).
- Tests: the suite compiles and passes without the flat path.
- Acceptance: `cargo test --workspace` green; bench unchanged vs `DpMode::Sp`.

### PLC-28 Symmetry axes on the routing lattice (RTE-15 contract)
- Priority: P1. Effort: S. Depends on: PLC-08; RTE-25 (`LatticeSpec`), RTE-10 (frame origin a multiple of `p0·lcm(strides)`).
- Why: RTE-15 step 3 makes exact mirrored routing conditional on `x_axis ≡ p0/2 (mod P)`, `P = p0·max(1, s_max/2)` in absolute nm (sky130: `p0 = 420`, axes at `210 + m·420`); RTE §0.2 lists it as PLC's delivery and RTE §5 Q7 asks whether PLC can hold it. Without it every matched pair falls back to the guide field and RTE-15's T5 ("pairs exact") cannot be met. Hand layout puts the axis on a track centreline as a matter of course (mirror-identical wires need it).
- Current: the axis is wherever projection (`symmetry.rs:133-149`) or, after PLC-08, the decoder leaves it (a multiple of `lattice/2`).
- Change (SP path only; the flat path keeps today's axes and RTE-15 falls back):
  1. `PlaceRules` gains `axis_grid: Option<(i32 /*p0*/, i32 /*P*/)>` from `dr::lattice_spec(&d_router.cfg)` (RTE-25) in `solve()`; `None` (no RTE-25 yet, or `p0 % (2·lattice) != 0`) disables this item.
  2. PLC-08 Step 2x′: instead of rounding `ax2` up to a multiple of `lattice`/`2·lattice`, take the smallest `ax2' ≥ ax2` with `ax2' ≡ p0 (mod 2P)` (then the node-relative axis `ax2'/2 ≡ p0/2 (mod P)`, and `ax2' ≡ 0 (mod 2·lattice)` still holds for self-symmetric kids because `p0` and `2P` are multiples of `2·lattice`). Steps 3x–4x run unchanged from the raised `ax2'` (a monotone raise, like Balasa's).
  3. Node origins: in every parent's Step 1x, a kid that is, or contains, a symmetry node gets its relative `x` rounded **up** to a multiple of `P` (an `lb` raise, monotone); the root origin is 0. Then every such node's absolute origin is `≡ 0 (mod P)` and its absolute axis `≡ p0/2 (mod P)` (**derived**). Horizontal axes (PLC-20) apply the same rule to y.
  4. `PlaceStats` gains `axes_on_lattice: u32` and `axes: u32`; bench prints both.
- Tests (`backend/dp/src/sp.rs`): `axes_land_on_track_centrelines` (random S-F trees from the PLC-08 property generator, `p0 = 420`, `P = 420`, lattice 10: every decoded `Layout::axis[a] % 420 == 210`); `axis_snapping_costs_at_most_one_period_per_level` (node width grows by < `2·P` per symmetry node, printed as a distribution).
- Acceptance: on all fixtures with `DpMode::Sp`, `axes_on_lattice == axes`; T2 area usage within +3 % of PLC-08 without this item (**policy**); RTE-15 "pairs exact" = n/n on `ota`, `ota_constrained`, `tt_ota` when RTE-15 has landed.
- Risks / notes: rounding every symmetry-bearing node origin to `P` (420 nm on sky130) costs at most `P − lattice` of x per hierarchy level; measured by the second test and T2.

### PLC-29 Matched-pair environment (WPE/OSE) evaluated live in placement
- Priority: P1. Effort: M. Depends on: PLC-06 (units and power present in the placed `Layout`), PLC-01. Coordinates with MAT-07 step 6, which edits the same two key reads (`lib.rs:871-872`); whichever lands second carries the other's change.
- Why: AF-23 (audit-07: "WPE and OSE are measured only after dp, as routing Θ, so dp cannot act on them"; fix: "move the Environment measurement into a placement-tier rule on unit geometry in dp"); AR-09 (same finding from the rule side); LF-5 (audit-07 status **P**); audit-04 G9. No other plan owns it (`grep AF-23 docs/plans/plan-0*.md` hits only this plan); MAT §0 keeps the geometric `Environment` rule and declines a ΔVT model (plan-02 L37), so the missing piece is only *where* it is evaluated. It is T9's only lever on the skew terms (PLC-13 covers nearness).
- Current: `Flow::environment` (`frontend/library/src/lib.rs:806-876`) stamps every cell (`gr::place_macros`), collects nwell rects (rings included) and diff rects, and builds one `Surroundings` per 2-device leaf of kind `DiffPair | CurrentMirror | Load`; the result enters Θ through `budgets.add_routing` (`lib.rs:680`) and the metadata report (`lib.rs:431`). `Environment` is state-free (`environment.rs:52-70`: precomputed numbers), so even inside `placement.budget` it could not react to moves.
- Change:
  1. `kernel/core/src/macro.rs`: extract `place_macro`'s `shift` closure (`:46-51`) into `pub fn place_rect(bbox: Rect, r: Rect, l: &Layout, i: usize) -> Rect` (same arithmetic: `o.apply_rect(r)` shifted by `(l.x[i] − l.hw[i] − anchor.x, l.y[i] − l.hh[i] − anchor.y)`, `anchor = o.apply_rect(bbox)`); `place_macro` calls it (no behaviour change; its existing callers are the test).
  2. `kernel/analog/src/placement/environment.rs`:
     ```rust
     /// Per cell, per variant, in the macro's own frame: its bbox, n-well rects and diff rects.
     pub struct EnvGeo { pub bbox: Vec<Vec<Rect>>, pub wells: Vec<Vec<Vec<Rect>>>, pub diffs: Vec<Vec<Vec<Rect>>> }
     /// Live form of [`Environment`]: re-measures every pair on the current layout.
     #[derive(Clone)]
     pub struct LiveEnvironment { pub pairs: Vec<(DeviceId, DeviceId, u16 /*cell a*/, u16 /*cell b*/)>,
                                  pub geo: std::sync::Arc<EnvGeo>, pub wpe_min_nm: f32, pub ose_range_nm: f32 }
     impl LiveEnvironment { pub fn surroundings(&self, l: &Layout) -> Vec<Surroundings>; }
     impl RuleBatch<Layout> for LiveEnvironment { /* cost, violations, residual, unknown, criticality: those of
         Environment(self.surroundings(l)); kind "Environment"; touched: none (global batch, PLC-10) */ }
     ```
     `surroundings` is today's `Flow::environment` body (`lib.rs:811-874`) moved verbatim, with `placed[c].shapes` replaced by `place_rect(geo.bbox[c][v], r, l, c)` over `geo.wells[c][v]` / `geo.diffs[c][v]` at `v = l.variant[c]`, and **without rings** (they are drawn after dp; the post-route report keeps them, step 4). Units come from `l.units.of_device(l, d)` as today.
  3. `solve()`: after `CellSpace::new` (`lib.rs:275`), build `EnvGeo` from `cells.variants` (layers `pdk.layer("nwell")`, `pdk.layer("diff")`; if either is `None`, emit nothing, as today) and push one `LiveEnvironment` into `problem.placement.budget` with the pairs of `annotator::block::leaves(&problem.blocks)` filtered as at `lib.rs:853-857`, and `wpe_min_nm`/`ose_range_nm` read as at `lib.rs:871-872` (MAT-07 switches both to `mos_env(Moderate, …)`). The rule is cell-indexed at construction (pushed after retargeting, `lib.rs:1152`).
  4. `Flow::epoch`: delete `Box::new(self.environment(&layout, &rings))` from the routing arm at `lib.rs:680` (the placement row now carries it; keeping both would count Environment twice in Θ). Keep `Flow::environment` (rings included) for the metadata report at `lib.rs:431`, implemented as `LiveEnvironment::surroundings` plus the ring wells, so the two cannot drift.
  5. Cost: one evaluation is `O(P · Σ_c (|wells_c| + |diffs_c|))` for P pairs (the same work `Flow::environment` does once per epoch today) and runs on every dp trial. **ponytail:** global batch evaluated per trial; if FLOW-09's dp timer shows > 30 % of dp time in it (**policy**), cache the well/diff rects of unmoved cells per trial (the moved set is known in PLC-10's `Eval`).
- Tests:
  - `frontend/library/src/lib.rs` `start_tests::live_environment_matches_the_flow_report_without_rings`: `ota.spice`, sky130, one epoch's winner layout → `LiveEnvironment::surroundings(&l)` equals `Flow::environment(&l, &[])` field by field, exactly.
  - `kernel/analog/src/placement/environment.rs` `a_foreign_well_near_one_member_violates`: two 1000×1000 NMOS cells (diff rect = bbox, one unit each at the centre) at x = 0 and 3000, a third cell whose nwell rect's left edge is 1000 nm right of cell b's centre, `wpe_min_nm = 3000`, `ose_range_nm = 0` (OSE half unknown, so only WPE is scored), third cell has no diff → `violations == 1` (b's nearness `3000/1000 = 3 > 1`); move the third cell so its well edge is at x = 40,000 nm → `violations == 0` (both distances ≥ 37,000 > `10·wpe_min` = 30,000, so both clamp to 30,000: skew 0, nearness ≤ 0.081).
- Acceptance: T9 = 0 on every fixture × seeds 1–5 (both nearness and skew terms), measured by the post-route report (rings included); lex key no worse on ≥ 9/10 fixtures (median over seeds); dp wall time +≤ 30 % (**policy**) or the cache in step 5 is added.
- Risks / notes: rings' n-wells are invisible to the live rule; a ring drawn later can still move a pair's WPE distance, which the post-route report (the judge) catches. REL-07's ring reservation (PLC-22) puts ring wells inside the node reservation, so the gap is bounded by the halo.

---

## 4. Milestones

| Milestone | Items | Exit criteria |
|---|---|---|
| M0 Correctness | PLC-01 → 04 → 02 → 03 → 06, plus PLC-10 step 0 (`Schedule`, unblocks FLOW-08). External, same milestone: FLOW-03 (one dual step), FLOW-09 steps 4–5 (timers, dp seed salt). | Baseline table recorded for all 10 fixtures × seeds 1–5; `lattice_off == 0` and `matched_geometry_mismatch == 0` on every fixture/seed; `cargo test --workspace` in debug with no placement panic; PLC-06's `gp_keeps_one_axis_per_block` and `gp_sees_power` green; FLOW-03's `place_does_not_settle` green. |
| M1 Spacing | PLC-07 → 13, 24, 29 | `table_gaps_are_drc_clean_on_sky130` green (gf180/IHP once their `placement_space` entries exist); T2 median ≥ 20 % better than the M0 baseline; T9 = 0 (PLC-13 nearness + PLC-29 skew); no fixture with a floor-only `Utilization` residual; signoff DRC/LVS no worse on any fixture. |
| M2 Representation | PLC-18 → 08 → 09 → 10 (steps 1–4), 11, 12, 28 | PLC-08 property tests green; `DpMode::Sp` meets PLC-09's acceptance (T1 all 0, lex key ≥ `Flat` on ≥ 8/10); T4 ≥ 90 %; T7 ≥ 3× proposals/s on `dac4` and `examples/three_stage_opamp`; gp decision and ablation table in `docs/CRATES.md`; `axes_on_lattice == axes`; default switched to `Sp`. |
| M3 Performance and routability | PLC-16, 21, 15, 14, 17 | T5 (spec miss with `PlacePerf` on ≤ off on every fixture with specs, strictly lower on ≥ 1); T6 (`route_hard + route_overuse` ≤ baseline on every fixture); 0 `OrientationSet{Phi}` violations with Mirror pairs; heat and supply-weight acceptances as stated per item. |
| M4 Structure | PLC-20, 22, 25 | Each item's acceptance, gated on EXT-14 / REL-07 / EXT-28 + FLOW-07 deliveries. |
| M5 Cleanup | PLC-27 | Flat path deleted; `cargo test --workspace` green; bench lex keys identical to `DpMode::Sp` before deletion. |

Dependency graph (acyclic; `→` = must land before): PLC-01 → {02, 03 (for `PlaceStats::matched_incompatible`), 06, 29}; 02 → 07 → {08, 13, 24}; 03 → {08, 14, 21}; 04 → 07; 06 → {14, 29}; 18 → 09; 08 → {09, 10, 12, 15, 20, 22, 28}; 09 → {10, 11}; 10 → 27; 08 → 27; 20 → 25 (`AxisDir`). External: MAT-07 → {13, 14} (and MAT-07 ↔ 29 edit the same `lib.rs:871-872` lines, either order); MAT-01 + EXT-14 → 21; EXT-14 → 20; RTE-25 → {15, 28}; REL-07 → 22; EXT-28 + FLOW-07 → 25; PLC-10 step 0 → FLOW-08.

---

## 5. Open questions (each with a proposed default)

1. **User coordinates for injected macros.** Should `Macros` carry positions that placement must honour? Default: no; `fixed` means shape-frozen (PLC-06). Revisit when a user asks for pinned blocks.
2. **90° rotation on sky130.** The sources give no sky130 statement on directional halo implants (H12-26 is generic). Default `rotate_90_allowed = true`; matched sets never rotate independently (PLC-03).
3. **Implant merging.** Resolved in PLC-07: same-type implants keep the deck spacing (380 nm on sky130) unless both facing implant edges sit on the bbox faces, in which case the cells may abut at exactly 0 (`Gap::abut`); `implant_bridges` is not relied on because it fills only equal spans (`post_cell.rs:625-628`).
4. **Balasa Step 2x span minimisation** (embedding DAG, L2129–2250). Default: the simplified Step 2x′; implement the DAG version only if PLC-12's metric or the area-usage table shows symmetry-node spans > 5 % above the members' packed width.
5. **Routing reserve `r` for the placement performance term.** Default 0.2 (**policy**); calibrate per spec as `clamp(ΔP_routed / ΔP_est − 1, 0, 0.5)` after two feasible epochs.
6. **Warm/cold ratio.** Owned by FLOW-08 (`COLD_EVERY = 4`, ablation over {2, 4, 8, ∞}); PLC supplies `Schedule::warm()` and `Start::Warm`.
7. **Heat-source threshold** `heat_source_uw`. Default 1000 µW (a 1 mW source gives a 1 µm Mod separation, comparable to the well spacing); tune per circuit class.
8. **Class of a matched set when EXT/MAT give none.** Default `MatchClass::Moderate` (MAT-07's default; Hastings: moderate matching suffices for general-purpose op-amp and comparator input pairs, L42341–42342), which drives PLC-13 and PLC-14.
9. **Die context format** (PLC-23, deferred). Default when a producer appears: `Rect` die plus block origin in the same nm frame; no package model until FLOW defines one.
10. **Axis snapping cost** (PLC-28). Default: snap every symmetry node when `LatticeSpec` is available; if T2 loses more than 3 %, snap only nodes whose pairs RTE-15 routes jointly (EXT-14 net pairs).

---

## Verification log

Adversarial fact-check, 2026-09-28, against the working tree (HEAD `dad330c` plus uncommitted changes), the pinned GPurify deck (`8df8c09/pdks/sky130.deck`, the revision in `Cargo.lock`), the `docs/plans/audit-*.md` and `ref-*.md` entries, and `scratchpad/reftext/*.txt` (one Hastings value read from the PDF, page 696).

**Checked: 502 references.** 237 code citations (`path:line`, types, fns, fields, constants, test names); 119 reftext line-range citations (formulas, constants, quotes, section and PDF page numbers); 146 distinct audit/ref IDs (AP, AR, AF, PL, AV, AA, BAL1, BAL2, LAMP, H01–H15, SUB, SURV, NOTES, MM, EM), including compound forms (BAL1-04..13, BAL1-33/34, SUB-15/16/31/32, SUB-16/23, H14-03/04, NOTES-38/39, LAMP-13/14). Every cited ID exists in its audit or ref file and its topic matches the use in the plan. Proposed new names (`PlaceStats`, `PlacementMetrics`, `GpMode`, `DpMode`, `Locks`, `align_bbox`, `lattice_offenders`, `SpacingTable`, `Profile`, `Face`, `PlaceRules`, `GpInput`, `PlaceInput`, `PlaceRow`, `sp::{Kid, Node, Tree, Sym, Geo, Out, Fail}`, `MatchClass`, `AxisDir`, `SymMode`, `OverflowMap`, `DieContext`, `Outline`, `HeatSeparation`, `SymmetryIsland`, `InjectorAxis`, `StressZone`, `FlowOrder`, `Boundary`, `EdgeDiff`, `mst_len`, `Orient::compose`/`inverse`, `Layout::overlapping_pairs`, `Prices::weights`, `Rule::matched_pair`, `RuleBatch::matched_pairs`) collide with nothing in the workspace. All arithmetic in the tests and derivations was recomputed (align_bbox cases, 340 − 200 = 140, 270 − 2·180 → 0, R90 face map, 3.27² = 10.7 and 2.27² = 5.2 µm², 4 / 10.69 = 37 %, 3000 − 200 = 2800, 2 mW → 2000 nm, 5000 µA / 2.8 µA/nm − 140 → 1650 nm, 0.9·4/5.15, 220 × 60 = 13,200, the S-F proofs for `make_sf` and the M3 pair flip, the doubled-centre symmetry algebra and its lattice parity).

**Corrections (before → after).**

1. §0.1 `CellSpace::new` halo/bbox range `1194–1217` → `1195–1216` (1194 is the struct literal's closing brace).
2. §0.2 EXT `Problem::groups` `annotator/src/lib.rs:40-45` → `:39-41` (`:42-45` is the abutment table).
3. §1.1 item 2 gp `power_uw = 0` `mechanics.rs:383` → `:382` (`:383` is `temp_mc`); same fix in the §1.2 AP-10 row (`:383-384` → `:382`, `:384`).
4. §1.1 item 5 and the §1.2 AP-07 row: SymmetryGroup residual source `rule.rs:30-37` (the per-`Rule` 0/1 default) → `rule.rs:141-143` (the `RuleBatch::residual` default = violation count, which `SymmetryGroup` does not override).
5. `lib.rs:1194-1208` → `1195-1208` everywhere (§1.1 item 6, PLC-15, PLC-22).
6. §1.2 AP-14 row: `layout.rs` `debug_check_placed` → `layout.rs:152-164`.
7. §2 Plantage sentence: "n/p well distance dominating the overhead" → the source's actual claim (folded-cascode example "always above 121 %" because of n/p minimum distances, L6780–6783); Table 3.2 and Table 3.4 values attributed separately.
8. §2 T2: added that Plantage's denominator is "modules and wells" (L6666–6668); "inside the Plantage/HB* published range" → the actual published ranges (Table 3.2 1.10–1.29; Table 3.4 1.05–1.19). The 1.30 target is just above the Table 3.2 maximum.
9. §2 T7: "540 s" → "539.5 s, seed 1, `FEEDBACK_ITERS = 5`" (audit-06 §G.3).
10. §2 T9: "WPE-near violations (Environment usage > 1)" → Environment violations with the actual usage definition (WPE nearness, WPE skew and OSE skew, `environment.rs:33-47`, `:59-61`).
11. PLC-01 Current: "`RunStats` has only violation counts" → it also holds iteration counters, a convergence flag and routing overuse; it has no timings or area metrics.
12. PLC-02 Why: H01-26 "PDF 124" → "PDF 121, 124" (L6534–6551 is on PDF 121).
13. PLC-02 Risks: "≤ 10 nm (`up()` rounds by < 2·lattice)" → ≤ 10 nm holds for generated cells because `Builder::finish` already aligns corner and extents to 10 nm (`builder.rs:125-133`); injected macros can grow by < lattice left/bottom and < 2·lattice right/top.
14. PLC-03 step 1: added that `SymmetryGroup` and `CentroidGroup` are hand-written `RuleBatch` impls, so the blanket `Vec<R>` forward does not reach them; `SymmetryGroup` must forward `matched_pairs` like `mirror_pairs` (`symmetry.rs:129-131`).
15. PLC-03 step 4: H12-26 "L36479–36487, printed p.610" → "L36480–36487, PDF 611 (printed p.610)".
16. PLC-03 Tests: added that `matched_devices_never_turn` (`tests.rs:54-59`) calls the deleted `rotatable` and must be replaced.
17. PLC-04 step 2: noted that `rule_residual`/`FAIL_FLOOR` are private to `rule.rs`.
18. PLC-04 step 6: "`snap_away` rounds half-grid remainders away from zero" → the defect is the truncating integer `/ 2` before `snap_to` (25 / 2 = 12 → 10); `snap_away` must round any nonzero remainder away from zero from the untruncated half-difference. As first written, the fix would not have passed the plan's own `odd_parity_projection_never_shrinks_the_pair` test.
19. PLC-05: added that `mechanics::analog_cost` and `report` read `Prices::weight_of`, so they must take the returned `&[f32]`.
20. PLC-06 Tests: added that `projection_never_moves_pinned_cells` (`tests.rs:339-346`) and `a_pinned_macro_never_moves` (`legalize.rs:127-132`) assert the old position freeze and must be rewritten.
21. PLC-07 Why: "n/p well distance dominates area overhead" → the source's folded-cascode statement.
22. PLC-07 Current: added the missing consumer of `Rules::clearance`, `elaborate::antenna_diodes` (`lib.rs:635`), which PLC-07 step 5 and PLC-27 would otherwise break.
23. PLC-07 step 2.2: `poly.4` is `space(poly_field, diff)` (deck L280), i.e. field poly only.
24. PLC-08 Decision: added the §1.5 equal-conditions line range (L2735–2738); HB* "over SP-family placers" → the actual comparators (SP, segment tree, SP+LP, SP with dummy nodes) with Table 2.5/2.6 lines; Plantage quote attributed to the folded-cascode example.
25. PLC-08 `make_sf`: cited the exact sentence it derives from (L2389–2390).
26. PLC-08 Y step: "Balasa: worst case ⌈p/2⌉ + 1" → Balasa says Step 1y may need an order-p number of executions; ⌈p/2⌉ + 1 is the count for the Fig. 1.18j–k example; "no more than three" in practice (L2093–2099). The `p + 2` cap is now labelled **policy**.
27. PLC-08 Step 4x: noted that Balasa sweeps α in reverse, not β (both are reverse topological orders of "left of").
28. PLC-08 verify: "Balasa proves Steps 1x–4x feasible (L2278–2323)" → "Balasa argues informally … (L2308–2323)".
29. PLC-08 test: Example 1 citation "Fig. 1.18 caption, L2050–2058" → Example 1 L2021–2028, caption dimensions L2068–2072 (w × h).
30. PLC-09 schedule: stop-rule thresholds labelled **policy**, with Lampaert's user-set interval (L4971–4975) and the earlier Philis 10-chain trial (`dp/lib.rs:297-298`); added Lampaert's α range 0.8–0.95 (L5014–5017).
31. PLC-13 Why: quoted WPE sentence (not verbatim) → paraphrase, noting that the values are blank in the reftext and were read from PDF 696 (1.8 µm → 5 %, 0.95 µm → 25 %, confirmed).
32. PLC-13 Change and §5 Q8: "moderate matching is the default for general-purpose input pairs, L42333–42350" → Hastings says moderate matching "is satisfactory for the input differential pairs of general-purpose operational amplifiers and comparators" (L42341–42342); the default is **policy**.
33. PLC-14 Why: "LAMP-13/14 (… L5289–5330)" → LAMP-13 (L4768–4871) and LAMP-14 (L5289–5330) cited separately. The L5289–5330 range belongs to LAMP-14 only.
34. PLC-14 Change: `k(Exc) = 1.0` labelled **policy** (rule 14 gives no per-mW figure for Exc, L42582–42584); `k(Min) = 0` conditioned on common-centroid layout (L42586–42587).
35. PLC-15: BAL2-19 "damped update" → damping is **policy**, motivated by the source's oscillation warning (L9401–9405); the source does not prescribe damping.
36. PLC-16 Current: `lib.rs:268-270` → `:267-269`; "`af_per_um / 1000` from the first routing layer (`lib.rs:216-217`)" → `lib.rs:204`, `:218`, with the first-routing-layer derivation at `lib.rs:494-502`.
37. PLC-16 Acceptance: LAMP-33 numbers moved to their real location (Table 4.1, L5006–5012, PDF 118, outside the cited L5020–5082); "6.9 → 3.7 mV with vs without" reworded so that "with" is 3.7 mV / 2.8 ns.
38. PLC-17 Why: BAL2-19 "raise … damped" → raise until maximum (L9441–9449), oscillation warning (L9401–9405); 0.5/4/0.9 labelled **policy**.
39. PLC-19 Why: "SUB-16/23 (… L2336–2387)" → SUB-16 L1885–1889, L2063–2073 and SUB-23 L2336–2387. Current: `emit.rs:246-287` → `:248-288`, `:250-256` → `:251-256`, `:272-273` → `:271-273`.
40. PLC-26 Why: "H14-03/04 (§14.1, L42912–42942)" → H14-03 §14.1.4–14.1.5 L43526–43555, L43595–43612, and H14-04 §14.1 L42912–42942.
41. PLC-27: `abutment` field `lib.rs:1104-1105` → `:1103-1104` (filled at `:1187`).

**Verified without change (sample of the load-bearing ones).** All `frontend/library/src/lib.rs` call-path lines (`:352`, `:356`, `:529-538`, `:541-556`, `:575-599`, `:607-613`, `:617-618`, `:635`, `:670`, `:680`, `:806-876`, `:916-958`, `:1098-1117`, `:1198`, `:1209-1216`). The gp and dp constants and line ranges (`MAX_ITERS` 220, `MOVES_PER_CELL` 60, `ALPHA` 0.93, `ANALOG_PROBE` 64, `OVERFLOW_TARGET` 0.15, `quarter_turn`, `rotatable`, `Snap`, `accept`, `project_hard`, `sym_groups`, the legalizer's `(false, false) → (0, 0)`, the ignored return at `dp/lib.rs:360`). No test calls `gp::place`. The sky130 deck lines 228/266/269/271/280 and `grid 5nm`. `sky130.json` keys (`dti: null` :46, `wpe_clearance_nm` :59-63 with no Rust reader, `n_well_depth` :68, met1 2.8 mA/µm :99). `annotator` abutment truncation (`lib.rs:106-116`) and glue block (`:103-104`). `emit.rs:139`. `post_cell` `ring_halo` :291, `well_bridges` :387-437 (exact-span rule :424/:426), `:B` bulk :402. `Orient::Mx180` `geom.rs:52`. `Unit::phi` `units.rs:26-27`. `Process::space_between` `process.rs:75-78`, `pdk.rs:1003-1007`. `elaborate::em_limits` :248. `gr` `gcell_capacity: 6` :33. Reference side: Hastings Eq 15.1 and P_f 1.2–1.8 (L46538–46554, PDF 784); LAYLA 10–15 % (L7094–7114); Def. 2.1 (L3187–3193); SP relations (L1826–1838); eq 1.1 (L1856–1868); the initial S-F code (L2383–2390); Step 3x/4x formulas (L2251–2277); Plantage shadow cases (L5798–5808); Lin/Chang T0 eq 2.5, α 0.9, 20,000 moves/temperature (L4074–4081); Lampaert T0 eq 4.37–4.38, P0 0.6 (L4951–4970); eq 4.4/4.5 static term (L3956–4024); eq 4.35 dynamic term (L4897–4910); eq 4.7 aspect-ratio cost (L4419); Hastings rules 7, 8, 13, 14, 15, 19, 23 (L42490–42654); corner exclusion 5 % for dice > 2.5 mm (L47413–47417); H12-26 90° prohibition (L36480–36487); Pelgrom rotation (L98–99); survey L71–79, L111–117, L169–182; Lienig L3457–3462; Charbon L2287–2296, L3382–3399.

**Unresolved.**

- PLC-15 step 1: the statement that LAYLA reserves full widths *because it does not route over devices* is marked `[UNVERIFIED]`. The Lampaert reftext states the static term (L4012–4024) but not LAYLA's over-device routing policy.
- H13-53/H13-55 class distances (MOD 3 µm, EXC 5 µm, rule 23 MOD 3–5 µm and EXC 5–10 µm) and the H12-26 angles are blank in the reftext. They were accepted from the H13 and H12 studies, which read the PDF; this pass re-read only PDF 696 (the WPE data). GAP-20 (`ref-hastings-99` §2.4–2.6): all of these, and the WPE 5 %/1.8 µm and 25 %/0.95 µm points, are body text; no figure long description states them, so the PDF readings stand.
- LAMP-20 (L2792–2850, "promoted for sensitive nets") and H12-31 (no line range in its study) were checked only for existence and topic, not re-read line by line.

---

## Cut or deferred items

Each entry: what was in this plan, where it went, and why. "Cut" = owned by another plan or dropped as unneeded; "deferred" = no input, producer or consumer exists yet.

| Former content | Status | Reason |
|---|---|---|
| PLC-05 One dual step per epoch (delete both `settle` calls, settle once in `Flow::epoch`, `Prices::weights(&self)` refactor, `settle_once_per_epoch_escalates_rho_across_epochs`) | Cut → FLOW-03 | FLOW-03 steps 1–2 make the same edit and add `place_does_not_settle`; the proposed ρ test duplicates the existing `rho_escalates_for_a_residual_that_does_not_shrink` (`gp/lib.rs:386-413`); the `&Prices` refactor is unnecessary because `bind` stays and needs `&mut`. |
| PLC-04 step 5 `DP_SALT` and test `gp_and_dp_rng_streams_differ` | Cut → FLOW-09 step 5 | Same constant, same call site, owned by FLOW. |
| PLC-01 wall-clock fields `gp_us, dp_us, route_us, signoff_us` | Cut → FLOW-09 step 4 (`RunStats.stage_ms`) | One timer set for the whole epoch; PLC keeps only counters dp alone can see (`PlaceStats`). |
| PLC-02 helper `lattice_offenders` | Dropped | PLC-01's `PlacementMetrics::lattice_off` computes the same count; one function, not two. |
| PLC-03 `Locks::frozen` (all-or-nothing index compatibility per set) | Replaced | Per-pair compatibility (PLC-03 step 2): `CentroidGroup` pools a whole stage (AR-02), so the set rule would have frozen every matched stage with a differently sized load pair. |
| PLC-04 `Layout::overlapping_pairs` and the conditional `debug_assert` | Dropped | `layout.debug_check` plus the hard "device overlap" / "clearance encroachment" rows already make overlap visible and costly; a second overlap scan adds nothing. |
| PLC-07 `rule[nsdm][nsdm] = rule[psdm][psdm] = 0` relying on `implant_bridges`; `same_well` flag; `well_space` field | Replaced by `Gap { abut, min }` | `implant_bridges`/`well_bridges` only bridge equal spans (`post_cell.rs:424-427`, `:625-628`), so a 0 rule would have produced DRC violations the probe test would catch only on the tested pairs. |
| PLC-07 fallback for **every** undefined role pair | Replaced | Applying the 1270 nm fallback to cross-layer pairs with no deck rule (poly vs nsdm, li vs diff, …) would have kept most gaps near today's value and defeated the item; the fallback now covers same-role pairs without a deck value and the `other` role only. |
| PLC-08 decoder DTI support (`Geo::dti`, `DtiEdge`, `Fail::Dti`, step 7, `dti_isolate_branch_adds_trench_gap`, `dti_share_branch_rejects_a_far_pair`) | Cut | No shipped deck has DTI data (`pdks/sky130.json:46`, `generic_finfet.json:52`: `"dti": null`); `DtiBand` stays a hard rule evaluated on each decoded layout, so the gate already rejects a violating code; `DtiEdge` was never defined. The AR-16 `<=` fix moved to PLC-04 step 6. |
| PLC-09 `Config::dp_budget_ms` wall-clock stop | Cut | A wall-clock stop inside dp makes results depend on machine load, breaking T8; FLOW-08's `max_wall` stops between epochs instead; `Schedule::max_temps` caps the work deterministically. |
| PLC-10 step 1 epoch-loop warm/cold scheduling in `solve()` (3 warm : 1 cold) | Cut → FLOW-08 | FLOW-08 owns the loop (`COLD_EVERY = 4`, history reset, escalation) and calls PLC's `dp::Start` / `Schedule`, which stay here (PLC-10 steps 0–1). |
| PLC-16 `Nets` pin half-sizes, `mechanics::mst_len`, `dp::PlaceRow` priced inside dp under key `"PlacePerf"` | Replaced | A plain `RuleBatch<Layout>` (`PlacePerf`) in `placement.budget` gets pricing, reporting, the gate and both placers for free; no dp-specific code path. |
| PLC-17 per-epoch net-weight feedback from extracted C (`r_n = C_ext/C_budget − 1`, ×(1 + 0.5·r), cap 4×, decay 0.9) | Cut → PERF-25 | PERF-25 updates the same `flow.net_weight` from the limiting spec with the same BAL2-19 damping; two feedback loops on one weight vector would fight. The supply-current weighting (AP-18) stays as PLC-17. |
| PLC-19 (1) `InjectorAxis` cost | Cut → REL-15 (`SubstrateBalance`) | Same source (SUB-43, charbon_substrate.txt L3382–3399), same geometry (equal distance of the pair to the aggressor); REL owns substrate rules and PLC's move set consumes them unchanged. |
| PLC-19 (2) per-pair `Isolation::min_distance_nm` from REL's model | Cut → REL-09 | REL-09 makes `Isolation` substrate-kind aware; placement reads the rule as-is. |
| PLC-19 (3) optional-ring branch bit (SUB-25) | Deferred | No producer: REL-07 assigns rings per device by role, not as a search alternative. Revisit if REL adds optional rings. |
| PLC-23 Die-context stress and heat zones (`Config::die`, `StressZone`, class setbacks Min 100 µm / Mod 250 µm, corner triangle `0.05·max(W, H)` for dice > 2.5 mm; H08-48 L24579–24624, H13-48 L42554–42568 and L42590–42595, H15-12 L47413–47438, H09-18 L30946–30987) | Deferred | No input carries a die outline, block origin or package; the rule would report *unknown* on every fixture. MAT §0.3 declines die-level stress for the same reason. Revisit when FLOW defines a die context (a top-level flow or a user sidecar). |
| PLC-24 `OutlineRule` (aspect-ratio target `(|AR − aspect|/aspect − tol)⁺`, max W/H hard; LAMP-29 eq 4.7 L4398–4457, BAL2-39 L12769–13067) | Deferred | No consumer: no `Config`, fixture or FLOW item (FLOW-11 block cells included) supplies an outline. The reachable-floor bug fix (AP-02) stays as PLC-24. |
| PLC-26 Cross-cell diffusion abutment (`EdgeDiff`, negative gaps; LAMP-20, BAL1-64, H12-31, H14-03/04) | Deferred | Needs per-variant edge-diffusion descriptors and merged cross-cell drawing from CELL, which no CELL item provides (CELL-20 butts taps inside one cell). `cellgen` already merges diffusion inside abutment groups and EXT-22 widens those groups. Revisit when PLC-07 + EXT-22 leave a measured area gap to hand layout (T3). |
| Audit G7 / Plantage deterministic enumeration into an aspect-ratio Pareto front (balasa_graeb_survey.txt L4534–4546) | Deferred | Only useful with an outline consumer (see PLC-24 row); PERF-14 already reports a Pareto view of the epoch key. |
| PL-1 / PL-24 per-move DRC oracle and FD-PEX gradient (audit-07 §2.14.1) | Deferred | PLC-07's table plus the pairwise DRC probe test is the oracle, evaluated offline per deck; revisit only if signoff DRC residue attributable to placement stays > 0 after M1. |
| Well-island clustering term (perf_driven_survey.txt refs [58]–[60], L128–134) | Deferred | `Gap::abut` already makes same-well abutment the cheapest packing; add a term only if the M1 bench shows same-bulk PMOS left in separate wells (count of n-well figures per fixture, CELL-22's metric). |
| AP-16 option "ePlace-style analytic gp" | Deferred | PLC-11 first decides whether gp survives at all. |
| PL-7 neighbourhood escalation / group rip-up when no single move lowers Φ (audit-07 §2.14.1, status **N**) | Dropped | Under the SP path overlap, clearance and symmetry are zero by decode (PLC-08), so no Φ plateau is left for a rip-up move to escape; the flat path that has the plateau is deleted by PLC-27. |
| AP-15 in-trial projection re-averages the whole stage axis | Not fixed separately | `project_hard` is flat-path only; PLC-08/09 replace it and PLC-27 deletes it. Patching it first would be throw-away work. |

---

## Implementer review log

Review as the implementer, 2026-09-28, against the working tree (HEAD `dad330c` plus uncommitted changes), the pinned GPurify deck (`~/.cargo/git/checkouts/gpurify-*/8df8c09/pdks/sky130.deck`), the other seven plans in `docs/plans/`, and the reftext. 27 work items reviewed (PLC-01 … PLC-27); one added (PLC-28). Code was read, not built.

**Ownership and duplication (other plans).** PLC-05 was a copy of FLOW-03 → cut. DP_SALT (PLC-04) and stage timers (PLC-01) duplicated FLOW-09 → cut. The epoch-loop warm/cold schedule (PLC-10) is FLOW-08's → PLC keeps only `dp::Start` and a new `dp::Schedule` (with FLOW-08's warm values) and lands `Schedule` in M0 so FLOW-08 is not blocked. PLC-17's C-feedback duplicated PERF-25 → cut; supply weighting kept. PLC-19 duplicated REL-15/REL-09 → cut. PLC-02 overlaps CELL-03 on `lib.rs:1198` → coordination note. PLC-13 now uses MAT-07's `MatchClass`/`mos_env` instead of a second enum; PLC-21 uses MAT-01's `mirror_allowed` and EXT-14's compound kind; MAT-05's dp test merged into PLC-03's. PLC-15 consumed a `gr::OverflowMap` that no plan produces (RTE-07 deletes gr's global router) → switched to RTE-25's `RouteStats::congestion` and `LatticeSpec`. §0.2/§0.3 tables rewritten with the exact item IDs.

**Correctness fixes to the plan itself.**
1. PLC-07 would not have met its own acceptance or DRC: (a) the cross-role fallback made every undefined role pair 1270 nm; now only same-role pairs without a deck value and unmapped layers fall back, cross-role pairs without a deck rule are 0; (b) same-type implant spacing 0 relied on `implant_bridges`, which bridges only equal spans (`post_cell.rs:625-628`) → replaced by `Gap { abut, min }` with an explicit merge band; (c) the 10-role list missed layers the generators draw (met1, licon, mcon, npc, rpm, res_implant, diode markers) → 18 roles including `other`; (d) the sky130 sidecar table was 5 entries → 20, each with its deck rule and line (difftap.3/.9/.11, poly.4/.6, npc.4, nsd.7, psd.7, licon.9/.13/.14, rpm.6, poly.9, poly.9.difftap). Tests re-derived (NMOS pair with edge implants → min 380, not abut); acceptance derivation updated (5.7 µm² pitch area).
2. PLC-03: Symmetry equality on variants (step 6) would make every epoch infeasible whenever `seed_assignment`/`escalate` gives partners different variants → added `Locks::unify` on the assignment in `Flow::epoch`. The all-or-nothing `frozen` rule would have frozen whole stages because `CentroidGroup` pools a stage (AR-02) → per-pair compatibility. Exact insertion points in `rule.rs` (trait, `RuleBatch`, blanket impl). Added the existing test that step 4 breaks (`extents_stay_in_lockstep_with_orientation`, `tests.rs:70`).
3. PLC-04: residual fix moved to `Rule::residual` for `Symmetry` (no visibility change in `rule.rs`); `report` gains `clearance` with its four callers listed and no double count with the overlap row; the new `Layout::overlapping_pairs` API dropped for `debug_check`; exact ceiling formula for `mirror_about` with the 25 → 30 check; the clearance-row test value corrected to `170·1270 = 215900` (encroach inflates both axes); AR-16 `<=`/`>=` fix moved here from PLC-08.
4. PLC-06: the fixed-cell redefinition missed the per-move skip (`dp/lib.rs:307-309`), the swap guard (`:332`), the legalizer restore loop (`legalize.rs:72-75`) and the test `seeds_are_written_and_table_resized`, which depends on `[true, true]` freezing all moves → all listed, with a `seed_branches` extraction to keep that test meaningful. `gp_sees_power` could not pass: `ThermalGradient` reads cached `temp_mc`, so the finite-difference probe must refresh temps.
5. PLC-08: `DtiEdge` was used but never defined and no deck has DTI → decoder DTI support cut (the gate handles `DtiBand`); `Fail::WellBand` generalized to `Fail::Band` for the PLC-07 merge band; `Scratch` defined; API now carries `Locks`, `Schedule`, `&mut Prices`; `DpMode` default and `GpMode::Constructive` placed here; Balasa Example 1 test gains `y0[C] == y0[J] == 4000` (derived from L2024–2028 and verified by hand on the SP).
6. PLC-09: energy used `λ_b` with the wrong sign convention and a ρ that `Prices` does not expose → `π_b = −λ_b` via `weight_of` plus a cached `rho`, in a new `gp::mechanics::augmented_cost`; the wall-clock stop broke T8 → removed; `T0` guard for flat landscapes; stop rule (b) made exact; `schedule_stops_early_on_a_flat_landscape` could never stop under the old "acceptance < 5 %" clause (every ΔE = 0 move is accepted) → rule and expected value (`temps == 6`) fixed.
7. PLC-16 re-designed as a `RuleBatch<Layout>` (`PlacePerf`) with per-variant pin boxes, so pricing, reporting and both placers need no dp code; residual normalized to the `Rule::residual` convention; A/B switch `Config::place_perf`; dependency on PLC-09/18 removed.
8. PLC-24 split: the reachable floor is a P1 bug fix (AP-02) with an exact helper and test values (0.699, 0.337); the outline rule deferred.

**Exactness added.** File:line for every edit site (e.g. `RunStats::merge` `lib.rs:899-908`, `realize` call sites, `CellSpace::new` `lib.rs:275`, `net_current_ua` `lib.rs:285`, `pdk.cell` `pdk.rs:54`, bool sidecar parsing `pdk.rs:1263`); Rust signatures for every new type (`PlaceStats`, `PlacementMetrics`, `GpMode`, `Locks`, `Gap`, `PlaceRules`, `Schedule`, `PlacePerf`, `Orient::then`, `SymMode`, `AxisDir`); sky130 values with deck lines; test inputs and numeric expectations recomputed (align_bbox cases, 215900, 30 nm, 140/2800/1270/380 gaps, 1650 nm halo, 1680 nm cap, 0.699/0.337).

**Priorities and order.** PLC-21 raised to P1 (RTE-15's exact mirrored routing maps pins by `map(rect)`, which translated partners cannot satisfy); PLC-24 raised to P1 (bug); PLC-17 lowered to P2 (small, no measured need beyond AP-18). Milestones re-sequenced (M0: 01 → 04 → 02 → 03 → 06 plus `Schedule`; M2 starts with PLC-18) and an explicit acyclic dependency graph added. Every milestone exit criterion is a count, ratio or named test.

**Added.** PLC-28 (symmetry axes on the routing lattice): RTE-15 step 3 and RTE §0.2 make exact mirrored routing conditional on `x_axis ≡ p0/2 (mod P)` and name PLC as the provider; the plan had no item for it.

**Moved to the appendix.** PLC-05, PLC-19, PLC-23, PLC-26, parts of PLC-01/02/03/04/07/08/09/10/16/17/24, and four audit gaps not planned (G7 enumeration, PL-1/PL-24 oracles, well-island term, ePlace gp), each with its reason.

**Still open for the implementer (not blockers).** The H13/H12 values that are blank in the reftext (rule 19/23 distances, H12-26 angles) remain accepted from the study documents, as the verification log says. `mcon`/`met1` spacing to a neighbour's shapes relies on the deck's same-layer rules through `p.space`; the pairwise DRC probe test is the safety net for any derived-layer rule the 20-entry table misses on sky130, and gf180/IHP need their own tables before `DpMode::Sp` becomes their default.

### Second implementer pass (2026-09-28)

Scope: all 29 items (PLC-01 … PLC-29; PLC-05, 19, 23, 26 are cut or deferred stubs and were checked only for their appendix reasons). I re-read the code at every edit site the plan names (`frontend/library/src/lib.rs` 520-606, 800-880, 895-912, 1180-1220; `backend/dp/src/lib.rs` 15-30, 170-180, 240-300, 395-410, 550-562; `backend/gp/src/mechanics.rs` 215-320, 355-390; `kernel/analog/src/placement/symmetry.rs`, `environment.rs`, `rule.rs`; `kernel/core/src/macro.rs`, `layout.rs`, `geom.rs`; `backend/verify/src/pdk.rs` 938-948, 1150-1175, 1255-1270), the pinned GPurify sky130 deck, `pdks/*.json` role maps, and the cross-plan IDs (all 28 external IDs resolve to a heading in their plan). Line citations checked this pass were correct except where noted below. Code was read, not built.

Changes:
1. **PLC-01 / PLC-02: wrong metric site.** `Flow::epoch` re-realizes `macros` after dp when dp reshaped (`lib.rs:600-604`); metrics and the lattice `debug_assert` computed at `:598` would have stamped the pre-dp variants. Moved to after the rebinding.
2. **PLC-01 test would fail at baseline.** `overlap_nm2 == 0.0` is not true today (AP-14). The test now asserts that the value exists and is non-negative. The zero assertion moved to PLC-09's acceptance.
3. **PLC-07 role table incomplete.** sky130's `cell.layers` has `rpoly → poly_rs` and `diom → areaid_diode`, and the generators draw both. The plan claimed `other` stays empty, which was false: every poly resistor and diode would have been spaced at the 1270 nm fallback against everything. Added the `rpoly` and `diom` roles (N = 20; the old indices are unchanged, so `MERGEABLE` is unchanged). Added a `MARKER` class so id layers with no deck rule get 0 instead of the fallback. Added sidecar rows `rpoly`–{poly, diff, tap} = 480 (poly.9 / poly.9.difftap, deck L284-285, `poly_res` L144). Added the tests `diode_markers_do_not_force_the_fallback` and `shipped_cells_draw_no_unmapped_layer` (asserts 0 on sky130, prints for the other decks). Documented how `Pdk::layer` resolves by deck name and that the diff and tap roles collapse on gf180/IHP.
4. **PLC-08 acceptance was not runnable.** PLC-08 defines the decoder, but the moves, energy and schedule are in PLC-09, so no SP `dp::place` exists at PLC-08. Added a delivery split: PLC-08 = `sp.rs` plus property tests; the fixture acceptance moved to PLC-09. M2's exit criteria and the dependency graph were updated to match. PLC-11 now depends on PLC-09 instead of PLC-08.
5. **PLC-09 energy had no area term.** The flat path gets its area pull from `REGION_FILL`. The SP energy had only HPWL plus rules, so a weakly connected fixture had nothing driving toward T2. Added `w_A·(A_bb/A_cells − 1)` (Lin/Chang eq 2.4, `Φ = α·A + β·W`, L3744-3748; BAL1-33 recipe), with `w_A = 1` as **policy**, and the test `area_term_is_bbox_over_cell_area` with derived values 0.21 / 0.15. `schedule_stops_early_on_a_flat_landscape` was rebuilt on two identical square cells, because with the area term four cells no longer give ΔE ≡ 0.
6. **PLC-10 step 0 test was not a test.** "Identical to the pre-change binary" became a characterization test: record literals on `rotate_bench()` with seed 7 before the refactor.
7. **PLC-06 `gp_sees_power` was ambiguous.** Both runs are now scored under the true power field, with exact inputs.
8. **PLC-12 adjacency was ill-defined.** `Layout::edge_gap` is a Euclidean `f32` that is 0 at a corner touch, which contradicted `diagonal_corner_touch_is_not_adjacent`. Replaced it with an integer rule: strict projection overlap on one axis, gap ≤ `touch_nm` on the other.
9. **PLC-13 acceptance was unreachable.** T9 includes WPE and OSE *skew*, which a keep-out cannot control. PLC-13 now accepts on the nearness term only.
10. **Added PLC-29 (live Environment in placement).** AF-23 / AR-09 / LF-5 are owned by no plan: `Environment` is computed after dp in the routing arm, so dp cannot act on T9. The new item moves the measurement into a `LiveEnvironment` placement budget. It reuses `Flow::environment`'s body and a `place_rect` helper extracted from `place_macro`. It removes the routing-arm copy to avoid double counting in Θ and keeps the report version with rings. It has exact tests. It sits in M1, depends on 01 and 06, and coordinates with MAT-07 on `lib.rs:871-872`.
11. **PLC-20: exact edit list.** The 13 `Symmetry { … }` literal sites across 4 files are listed.
12. **Traceability.** Added the audit-07 PL-4, PL-5, PL-6, PL-11 and PL-12 IDs to PLC-03, 07, 04 and 08. Added appendix rows for PL-7 (dropped: the SP path leaves no Φ plateau) and AP-15 (flat-path projection, replaced by PLC-08/09 and deleted by PLC-27; not patched separately). Updated §0.1 scope, the T9 row, the M1 row and the dependency graph (01 → 03, 01/06 → 29, 09 → 11).

Not changed, still open: the same three reftext gaps as in the verification log (H13/H12 values read from the PDF by the studies; the LAYLA over-device routing remark). There is also a new one: whether the recipe-only resistor roles (`res_block`, `rpoly_b`, `res_implant`) land on sky130 deck layers outside `ROLES` is not settled by reading. `shipped_cells_draw_no_unmapped_layer` decides it on day one, and any hit gets a `ROLES` entry with its deck rule.
