# Plan 01: Constraint extraction from the circuit (EXT)

Plan ID prefix: `EXT-NN`. Area: `backend/annotator`, the constraint data model it shares with `kernel/analog`, and the few consumers in `frontend/library` that read the annotator's output.

Code citations are `path:line` against the working tree of 2026-09-28 (HEAD `dad330c` plus the uncommitted changes). Every code fact below was re-read for this plan: `backend/annotator/src/*.rs` (all files), `kernel/analog/src/{cell,constraints,metadata,requirements,rule}.rs`, `kernel/analog/src/placement/{matching_pair,symmetry,cc,thermal,proximity,isolation,dti,environment}.rs` (struct definitions), `kernel/analog/src/routing/{crosstalk,differential,common_node,performance,shield,coupling,ir,em,antenna,parasitic}.rs` (struct definitions and `extract`), `kernel/core/src/{netlist,hypergraph,ids}.rs`, `frontend/library/src/lib.rs` (150–340, 460–530, 755–880), `oppoint.rs` (1–200), `perf.rs` (1–70, 180–262), `cellgen.rs` (60–160, 440–600, 700–800), `parse.rs` (140–215), `backend/gr/src/lib.rs` (240–282), `backend/dr/src/lib.rs` (805–820), `backend/dp/src/lib.rs` (555–560), `pdks/sky130.json` (40–110).

Reference citations give the source, section/equation, the `reftext/*.txt` line range and the PDF page, as recorded in the study documents (`docs/plans/ref-*.md`) and re-checked for the lines quoted here. Numbers marked **Philis policy** are design choices of this plan, not source values. Numbers marked **derived** are arithmetic on source values.

No cargo command was run for this plan (task rule).

---

## 0. Scope & boundaries

### 0.1 What EXT owns

1. **Recognition.** Structure recognition in the netlist: the pattern DSL and catalog (`pattern.rs`, `catalog.rs`), procedural recognizers for passives and bipolars, overlapping matches, declared slot roles, canonical (permutation-invariant) ordering, hierarchy (subcircuit instances, identical instances, arrays).
2. **Relations.** The typed requirement graph (M_S, M_B, P_B, S, P_N), the group tree built from it (HSMPG, survey Algorithm 3.1), circuit-level symmetry analysis (compounds, self-symmetric devices, nested groups, half labels, axis direction and kind), matched sets with ratio inference and unitization.
3. **Net and device roles.** One net-role classifier with evidence levels (user, port, name, testbench, structure, operating point), extended net classes, device operating region and role (switch, current source, amplifier, cascode, load), per-net facts (current bounds, DC envelope, EM-critical flag, R/C sensitivity class).
4. **Precision.** The precision class (minimal / moderate / exceptional) and match kind (voltage / current / ratio) of every matched set, and the sensitivity-driven allocation of each set's systematic allowance.
5. **Emission** of every constraint type from these facts:
   - placement: symmetry (couples, self-symmetric devices, nested groups), the member list, kind and class of every matched set (the rule that scores it is MAT-04's `MatchedSet`), array-style (interdigitation / CC) requests, proximity and hierarchy clusters, substrate aggressor and victim tags (REL-07/08/09 turn them into rings and isolation);
   - routing: differential pairs, crosstalk exclusions, shields, common-node, star and Kelvin requirements;
   - budgets: parasitic budgets derived from sensitivities (consuming the PERF sensitivity engine), and the class-based fallbacks when no sensitivities exist.
6. **Contract.** The constraint data model shared with `kernel/analog` (`analog::intent`), stable constraint IDs and provenance, the recognition coverage report, and the user constraint sidecar schema.

### 0.2 What EXT consumes from other plans

| From | What | Used by |
|---|---|---|
| FLOW | `Device::mos_size()` / `gate_area_um2()` in `pnr_core` (FLOW-01, AF-01); parser keeps `.subckt` ports, `.param`, R/C values and the instance tree (FLOW-07, AF-02); `OpPoint` extended with `vgs`, `vth`, `gmb`, `gds` per device and per-net DC voltage (H05-17; PERF-09 adds `vgs_v`/`vbs_v`); testbench PULSE/PWL-driven nets; CLI flag `--constraints FILE.json` (FLOW-12, AF-11); hoisting `annotate` out of the per-start closure (FLOW-09, AF-22) | EXT-11, 17, 18, 23, 26, 27 |
| PERF | Sensitivity engine and its export `perf::to_evidence` (PERF-11/12): per spec `∂f/∂C_net`, `∂f/∂R_net`, `∂f/∂C_coupling`, `∂f/∂ΔV_T,device`; process-corner spread; `σ_f` per bound (PERF-13); the one-row-per-bound and do-not-worsen rules (PERF-06) | EXT-17, EXT-21, EXT-25 |
| REL | Ring policy `annotator::rings::plan` (REL-07), injector classification (REL-08), isolation by substrate kind (REL-09) — all three read EXT's victims and net classes | EXT-23 supplies their inputs |
| MAT | `analog::matching::class::{MatchClass, MatchKind, Family, limit}` (MAT-07); the `MatchedSet` rule batch (one ledger, one allowance per set, MAT-04), `OrientationSet` (MAT-05), remaining allowance to CommonNodes (MAT-06), sizing notes (MAT-08) | EXT-12, 16, 20, 24, 29 |

### 0.3 What EXT provides

| Artifact (type, §3 item) | Consumer plan | Use |
|---|---|---|
| `Intent.sets: Vec<MatchSpec>` (EXT-12, 15, 16, 21) | MAT, CELL | one MAT-04 `MatchedSet` batch per spec (built by EXT-20); unitization, dummies and array style by class |
| `Intent.compounds: Vec<Compound>` (EXT-14) | PLC, RTE | one axis per compound, self-symmetric devices, direction, mirror/perfect kind; symmetric net pairs for mirrored routing |
| `Intent.tree: Vec<GroupNode>` (EXT-13) | PLC | symmetry islands and proximity clusters (PLC-08) |
| `Intent.aggressors`, `Intent.victims` (EXT-23) | REL | ring policy (REL-07), isolation (REL-09), well-merge flags (REL-16) |
| `Intent.nets: Vec<NetFacts>`, `Intent.devices: Vec<DeviceFacts>` (EXT-17, 18, 24, 25) | RTE, REL, PERF | R/C class, shield reference, region and role |
| `Intent.common_nodes`, `stars`, `kelvins` (EXT-24) | RTE | star/Kelvin topology (RTE-17); ΔR budgets come from MAT-06 |
| `PerformanceBudget` rows with C, series-R and coupling terms, one per bound (EXT-25) | RTE, PERF | routing prices in spec units |
| `BatchMeta { id, origin }` on every batch (EXT-10) | PLC, FLOW | prices keyed by id; reports |
| `Problem.coverage`, `Intent.diagnostics` (EXT-10, 29) | FLOW | metadata report and certificate |

### 0.4 Out of scope (owned elsewhere)

Mismatch models, class limit tables, the matching rule batches and their allowance ledger (MAT-04..08); pattern generation, dummies, rings and series units drawn inside cells (CELL); placement representation, rotation and shape locks (PLC-03), symmetry islands, horizontal-axis support in the `Symmetry` rule (PLC); routing algorithms and the measurement side of `Differential`, `CouplingBudget`, `Shield`, `PerformanceBudget` (RTE); ring policy, injector classification, isolation distance, substrate transfer, latch-up, EM currents and antenna checks (REL); the sensitivity simulations, corners and σ_f (PERF); the parser, the device-size convention, op-point extraction, deck loading, CLI and orchestration (FLOW). Diffusion-sharing permission and well groups are not produced (see "Cut or deferred items", ex-EXT-22).

---

## 1. Audit digest

Findings are from `docs/plans/audit-01-annotator.md` (AA-NN), `audit-02-analog-rules-and-core.md` (AR-NN), `audit-07-flow-frontend-docs.md` (AF-NN), re-verified in the current tree.

### 1.1 Pipeline as implemented (`annotate`, `backend/annotator/src/lib.rs:75-151`)

1. Hypergraph from the netlist (`lib.rs:76`). Per-device W/L with a missing value read as 0 (`lib.rs:77-81`).
2. Net roles by name only (`netrole.rs:28-48`): 8 supply roots and 8 ground roots, exact or `root_` prefix (`netrole.rs:18-19, 34-36`). Clocks by `clk`/`clock` substrings or `phi`/`ck` prefixes (`netrole.rs:21-24, 50-55`). The override fields of `AnnotationConfig` (`netrole.rs:59-75`: `do_not_identify`, `do_not_use`, `supply_nets`, `ground_nets`, `clock_nets`, `offset_sigma_mv`) are never set by any caller outside the annotator; the library fills only `process` (`frontend/library/src/lib.rs:492-515`) and otherwise passes `AnnotationConfig::default()` (`frontend/library/src/lib.rs:71`, `frontend/library/src/elaborate.rs:164`, `benchmarks/src/bench.rs:269`) (AA-11).
3. Recognition (`pattern.rs:175-212`): 103 FET-only patterns (`pattern.rs:94`) matched by exhaustive backtracking that scans every device per slot (`pattern.rs:154`). Deduplication by `Vec::contains` (`pattern.rs:147`). Sort by priority, then the sorted device-id list (`pattern.rs:199`). Greedy disjoint selection (`pattern.rs:201-210`).
4. Composite children are recovered by a second search restricted to ≤2-slot patterns (`lib.rs:93-99`). The composite's slot roles are discarded, and constraint kinds exist only at 2 devices (`block.rs:42-56`).
5. Glue block (`lib.rs:103-104`). `groups` = blocks (`lib.rs:106`). `abutment` cuts a mixed-polarity group to its first member (`lib.rs:107-117`).
6. "Sensitive" devices = members of DiffPair / CurrentMirror / Load / **Stack** leaves (`lib.rs:121-128`, `block.rs:60-62`).
7. Net classes (`classify.rs:47-138`): positional terminal roles G, D, S, B (`classify.rs:11-15, 75-85`) for every non-capacitor device; gates-only nets become Sensitive (`classify.rs:127`); budgets are 1×/2× the driven gate load (`classify.rs:26-33`); a drain-only net takes the circuit's smallest gate load (`classify.rs:87, 97`).
8. Placement rules (`emit.rs:127-246`): one axis per block (`emit.rs:139`); per 2-device leaf: Symmetry, MatchingPair, ThermalGradient, DTI, and pooled `CentroidGroup` sides (`emit.rs:147-198, 216-238`); every unpaired member of a DP stage made self-symmetric with a 5 µm pull (`emit.rs:203-211`).
9. Isolation (`emit.rs:263-288`): every device touching a Clock net × every sensitive device, at `4 × t_epi` or `4 × 2.5 µm` (`emit.rs:250, 256, 274`).
10. Routing rules (`extract.rs:24-106`): `Differential` (**hard**) and `CrosstalkExclusion` come from `analog::placement::matching_pair::is_diff_pair` (`extract.rs:53-61`; `matching_pair.rs:82-106`), not from recognition. Shield reference = the first Ground net (`extract.rs:91`).
11. Cell constraints (`constraints.rs:27-88`): one `Unitization` per (kind, W, L) class per block (`constraints.rs:30-33`) with `target_ratio = dev_nf` (`constraints.rs:52`), `dummy_required` and `route_matching_required` always true (`constraints.rs:60-62`); a guard ring on every FET of every non-glue block, `Ecgr`/`Hcgr` by polarity, tied to bulk, with literal pitch 2000 nm, width 500 nm, 100 Ω (`constraints.rs:66-85`).
12. `missing` lists rule families without deck numbers, unconditionally (`lib.rs:53-71`).
13. IR budgets (`ir.rs:36-61`) are the only op-point consumer, called by the library (`frontend/library/src/lib.rs:283-293`).

### 1.2 Hidden second recognizer

`frontend/library/src/cellgen.rs::with_per_device_sizing` (`cellgen.rs:443`) groups binary capacitor banks (`cellgen.rs:486-506`, `dac_banks` at 760-786), bipolars of one kind, W, L and base net (`cellgen.rs:508-527`, `bjt_groups` at 717-731) and parallel identical MOS (`cellgen.rs:530-549`, `parallel_groups` at 735-750). These produce merged cells but no placement or routing constraint, and the annotator never knows about them (AA-04).

### 1.3 Consumers of `Problem` in the library

- `performance_rows` annotates once to choose which nets get ∂f/∂C runs (`frontend/library/src/lib.rs:203-211`).
- `solve` annotates per start (`lib.rs:262-263`); `coarse.groups = cells.abutment` feeds dp's rotation lock (`lib.rs:579`; `backend/dp/src/lib.rs:558-560`); `coarse.axis` is resized to `blocks.len()` (`lib.rs:580-583`).
- `common_nodes` (`lib.rs:763-801`) and `environment` (`lib.rs:806-876`) scan `annotator::block::leaves` for 2-device DiffPair/CurrentMirror/Load leaves only (`lib.rs:779-785, 849-853`); `environment` hard-codes the `_moderate` deck keys (`lib.rs:871-872`); `common_nodes` re-implements the gate area (`lib.rs:792`).
- gr/dr find symmetric nets by looking for `Differential` in `reqs.hard` only (`backend/gr/src/lib.rs:253, 275-281`; `backend/dr/src/lib.rs:811`).

### 1.4 Bugs and placeholder logic to fix first (P0)

| Finding | Location | Defect |
|---|---|---|
| AA-01 | `pattern.rs:201-210` | a device belongs to one structure; tails lose their bias-mirror relation; mirrors with >3 outputs fragment |
| AA-02 | `emit.rs:139` | one axis per block; no circuit-level symmetry; a Gilbert quad is split over two axes |
| AA-03 | `catalog.rs:365-376` vs `230-242`; `lib.rs:93-99` | `cmos_inverter` (11) shadows `cross_coupled` (9) inside latches; StrongARM latches get no symmetry |
| AA-04 | `pattern.rs:94` | FET-only recognition; bandgap 1:N, R ratios, cap DACs unconstrained |
| AA-05 | `catalog.rs:118-160, 206-223, 2228-2247` | `diff_pair_split_source` and rail-source pairs create false DiffPairs; `mirror_pair_split_source` false mirrors |
| AA-06, AA-15 | `catalog.rs:2043-2055, 492-506, 1252-1280, 2208-2221` | wildcard `miller_comp_fets`; wrong Wilson and Gilbert links; electrically wrong `triode_load_pair`; 8 dead patterns |
| AA-07 | `pattern.rs:145-150, 199`; `emit.rs:196-197` | results depend on netlist order; CC sides follow slot order |
| AA-09 | `constraints.rs:30-63` | no ratio inference; W-ratioed mirrors never unitized |
| AA-10 | `emit.rs:42-52`; `frontend/library/src/lib.rs:871-872` | no precision class, no voltage/current distinction; everything "moderate" |
| AA-11 | `netrole.rs:18-24`; `classify.rs:127` | `0`, `gnd!`, `vee`, `VPB/VNB` not rails; digital inputs Sensitive; overrides never set |
| AA-12 | `lib.rs:75` | no op point or sensitivity used |
| AA-13 | `emit.rs:203-210` vs `263-288` | clocked tail gets Proximity ≤5 µm and Isolation ≥10 µm to the same pair |
| AA-14, AR-12, AR-39 | `extract.rs:53-61`; `matching_pair.rs:82-106` | `Differential` hard, re-derived by a heuristic with its own rail list and no size check |
| AA-16 | `classify.rs:11-15, 62-85` | positional terminal roles: BJT collectors and resistor P terminals count as gates |
| AA-17 | `block.rs:60-62` | Stack gate nets marked Sensitive |
| AA-18 | `constraints.rs:60-62, 66-85`; `kernel/analog/src/cell.rs:25-35` | dummies and rings keyed on "recognised", not on role; ring type docs contradict the drawing (H14-27) |
| AA-19 | `lib.rs:107-117` | abutment cut to one member for mixed groups; no bulk/shared-net check |
| AA-20, AA-21 | `pattern.rs:91-117`; `constraints.rs:30-33`; `lib.rs:80` | model and bulk ignored; missing W/L reads 0 |
| AA-25 | `classify.rs:87, 97` | drain-only nets budgeted against the smallest gate |
| AA-27, AA-29 | `block.rs:66-73`; `lib.rs:53-71` | template name dropped; `missing` not relevance-conditioned |
| AA-28 | `emit.rs:42-52, 250-256`; `extract.rs:47, 98-100, 110-127`; `constraints.rs:79-81`; `ir.rs:18-27` | policy literals scattered |
| AA-35 | `lib.rs:103`; `block.rs:69`; `classify.rs:105` | `u16` ids wrap silently |
| AR-02, AR-04 | `emit.rs:196-197, 216-238`; `emit.rs:83-101` | pooled CC cancels opposite offsets; the same η·σ is spent in full by 4–5 rules |
| AF-04 | `oppoint.rs:392-398` vs `netrole.rs:18-19` | rail lists differ between op point and annotator |

### 1.5 What works and is kept

Determinism for a fixed input order; every device accounted for (test `tests.rs:310-323`); the Pelgrom helper (`emit.rs:67-103`, tested `emit.rs:298-322`); antenna gate area on the gate terminal (`extract.rs:33-39`); capacitor-plate nets left unbudgeted rather than invented (`classify.rs:101`); IR budget arithmetic (`ir.rs:36-61`, tested).

---

## 2. Target: what "better than hand layout" means for extraction

A human layout designer reads the schematic and writes a constraint list (ALIGN-style) from memory. EXT must produce at least that list, with no false entries, for every circuit, and add what a human does not write: the precision class, a numeric allowance per matched set, the aggressor/victim map and spec-derived budgets.

| # | Metric | Threshold | Measured by |
|---|---|---|---|
| T1 | Recall and precision against a designer-written constraint file | On the ALIGN `high_speed_comparator` gold (`benchmarks/competition/ALIGN/examples/high_speed_comparator/high_speed_comparator.const.json`): the 7 symmetric device pairs implied by its `SymmetricBlocks` entry (mn1/mn2 via `xdp`, mn3/mn4 via `xccn`, mp5/mp6 via `xccp`, mp7/mp8, mp9/mp10, and mp11/mp12, mn13/mn14 via the `xinv_n`/`xinv_p` pair) and the self-symmetric `mn0`, on **one** axis; all 3 `SymmetricNets` pairs; **zero** device pairs not in the gold | `tests/align_gold.rs::strongarm_matches_align_gold` (EXT-01, passes after EXT-14) |
| T2 | Corpus exactness | For each of the 17 corpus circuits (EXT-01) the canonical output (matched sets with units, kind and class; compounds; self-symmetric devices; symmetric net pairs; number of axes) equals the hand-written expectation | `tests/corpus.rs::corpus_expectations` |
| T3 | No false matches | 0 matched sets and 0 compounds on the 4 negative circuits (SC switches, unrelated equal FETs, NPN+PNP mirror, digital inverter chain) | `tests/corpus.rs::negative_corpus` |
| T4 | Permutation invariance | Canonical output identical over 20 seeded permutations of device order, net order and terminal-list order, for every corpus circuit | `tests/corpus.rs::permutation_invariance` |
| T5 | One rule per matched set | Every `MatchSpec` yields exactly one MAT-04 `MatchedSet` batch with the same members, and no matching rule spans two specs (the single-spend arithmetic itself is MAT-04's `one_allowance_is_spent_once`) | `emit::tests::one_rule_per_set` (EXT-20) |
| T6 | No contradictory constraints | 0 device pairs with a Proximity and an Isolation rule, 0 devices in two symmetry pairs or on two axes, on every corpus circuit | `tests/corpus.rs::no_emitted_conflicts` (asserts after EXT-14 and REL-09) |
| T7 | Coverage | Every device is either in a requirement or reported `Unconstrained(reason)` | `tests/corpus.rs::coverage_is_total` (EXT-10) |
| T8 | Runtime (Philis policy) | `annotate` ≤ 2.0 s wall, release build, 12,500 devices (2,500 disjoint 5T OTAs); ≤ 50 ms for every repository fixture | `tests/scale.rs::twelve_thousand_devices` (`#[ignore]`, run with `--release -- --ignored`) |
| T9 | Flow non-regression | `bench local` on the 10 fixtures: DRC 0 and "LVS MATCH" kept (audit-06 §G.3 ground truth: all 10 report DRC 0 and LVS MATCH; `bgr_core` and `bjt_mirror` MATCH with no device compared and ERC 1 each, a documented GPurify false positive) after every milestone | benchmark run at each milestone exit (FLOW harness) |
| T10 | Beyond the hand file | On the T1 circuit, every matched set carries a class, a kind and a numeric allowance, and every compound an axis direction and kind; the gold file carries none of these | `tests/align_gold.rs::every_set_is_quantified` |

---

## 3. Work items

Items are ordered: characterization first, then bug fixes on the existing code (P0), then the new data model and capabilities (P1), then advantages (P2).

### EXT-01 Recognition corpus, gold harness and canonical form (characterization first)

Status: done in M0 (`574cd5e`). The placeholders are `#[ignore]`d with their owner: `negative_corpus_sc_switches_and_equal_fets` (EXT-04), `permutation_invariance` and `tests/scale.rs::twelve_thousand_devices` (EXT-06; measured > 13 min against T8's 2.0 s), `coverage_is_total` (EXT-10), `align_gold.rs::strongarm_matches_align_gold` (EXT-14), `no_emitted_conflicts_strongarm` (attribute says "EXT-14 and REL-09"; REL-09 is cut, GAP-04 owns it, C5). AA-13's clocked tail on the corpus strongarm is `mp8`, not `mn0`; `strongarm_conflicts_are_todays` pins (mn1, mp8), (mn2, mp8).

- Priority: P0. Effort: M. Depends on: none.
- Why: AA-26 (5 of 103 patterns tested; no false-positive, permutation, latch, bandgap, folded or three-stage tests); AA-07; NOTES-57 (discriminating validation catalog); legacy-code practice (characterize before changing).
- Current: `backend/annotator/src/tests.rs` builds netlists with `fet()` (`tests.rs:12-24`) and asserts block membership for 6 structures (`tests.rs:52-104, 381-411`). No integration tests, no permutation test, no gold.
- Change:
  1. Create `backend/annotator/tests/common/mod.rs` with:
     ```rust
     /// Devices separated by newlines or ` | `: `NAME n1 … nk MODEL [w=4u] [l=0.5u] [m=8] [nf=2]`,
     /// SPICE order (the model is the last bare token, as in every corpus line below).
     /// Kind from the model's prefix: `nfet`→Nmos, `pfet`→Pmos, `npn`, `pnp`, `r`→Resistor,
     /// `cap`→Capacitor, `d`→Diode; `Device::model` = the whole token (so `nfet_01v8_lvt` works).
     /// Terminals: FET nodes D G S B stored as G,D,S,B (as parse.rs:75-87 does), BJT C B E, two-terminal P N.
     /// `w`, `l`: suffix `u` → ×1000 nm, `n` → ×1 nm, bare → nm. `m`, `nf`: plain integers.
     /// Net ids in first-appearance order. The annotator cannot dev-depend on `library` (cycle), hence this parser.
     pub fn net(src: &str) -> pnr_core::Netlist;
     /// Seeded shuffle (xorshift64) of devices, nets and each device's terminal list.
     pub fn permute(nl: &pnr_core::Netlist, seed: u64) -> pnr_core::Netlist;
     /// Name-level, id-free view of what the annotator decided.
     pub struct Canon {
         pub sets: std::collections::BTreeSet<(Vec<(String, u16, u16)>, String /*kind*/, String /*class*/)>,
         pub pairs: std::collections::BTreeSet<(String, String)>,     // sorted names
         pub selfs: std::collections::BTreeSet<String>,
         pub net_pairs: std::collections::BTreeSet<(String, String)>,
         pub axes: usize,
     }
     pub fn canon(p: &annotator::Problem, nl: &pnr_core::Netlist) -> Canon;
     /// Interim form used until EXT-12 lands: (leaf kind, sorted member names).
     pub fn canon_leaves(p: &annotator::Problem, nl: &pnr_core::Netlist) -> std::collections::BTreeSet<(String, Vec<String>)>;
     ```
  2. Create `backend/annotator/tests/corpus.rs` holding the 17 circuits below as `const` strings and one expectation each (a `Canon` literal). Circuits:
     - Repository: `ota5t` (the netlist of `tests.rs:39-50`), `three_stage` (`examples/three_stage_opamp/src/main.rs:17-33`), `dac4`, `bgr_core`, `bjt_mirror`, `chain4`, `pair`, `quad` (`benchmarks/fixtures/*.spice`, transcribed without `.subckt` lines).
     - Constructed (exact text):
       ```
       folded:  M1 x1 vinp tail VSS nfet w=10u l=1u | M2 x2 vinn tail VSS nfet w=10u l=1u | M0 tail vbn VSS VSS nfet w=20u l=1u
                M3 x1 vbp1 VDD VDD pfet w=20u l=1u | M4 x2 vbp1 VDD VDD pfet w=20u l=1u
                M5 o1 vbp2 x1 VDD pfet w=10u l=1u | M6 out vbp2 x2 VDD pfet w=10u l=1u
                M7 o1 vbn2 y1 VSS nfet w=5u l=1u | M8 out vbn2 y2 VSS nfet w=5u l=1u
                M9 y1 o1 VSS VSS nfet w=5u l=1u | M10 y2 o1 VSS VSS nfet w=5u l=1u
       gilbert: M1 x1 rfp tail VSS nfet w=8u l=0.5u | M2 x2 rfn tail VSS nfet w=8u l=0.5u | M0 tail vb VSS VSS nfet w=16u l=0.5u
                M3 outp lop x1 VSS nfet w=4u l=0.5u | M4 outn lon x1 VSS nfet w=4u l=0.5u
                M5 outp lon x2 VSS nfet w=4u l=0.5u | M6 outn lop x2 VSS nfet w=4u l=0.5u
       rail2rail (this order): MN1 xn1 vinp tn VSS nfet w=4u l=0.5u | MP1 xp1 vinp tp VDD pfet w=8u l=0.5u
                MN2 xn2 vinn tn VSS nfet w=4u l=0.5u | MP2 xp2 vinn tp VDD pfet w=8u l=0.5u
                MN0 tn vbn VSS VSS nfet w=8u l=0.5u | MP0 tp vbp VDD VDD pfet w=16u l=0.5u
       latch:   MN1 q qb VSS VSS nfet w=2u l=0.15u | MP1 q qb VDD VDD pfet w=4u l=0.15u
                MN2 qb q VSS VSS nfet w=2u l=0.15u | MP2 qb q VDD VDD pfet w=4u l=0.15u
       mirror6: MR ref ref VSS VSS nfet w=2u l=1u | MO1 o1 ref VSS VSS nfet w=2u l=1u | MO2 o2 ref VSS VSS nfet w=4u l=1u
                MO3 o3 ref VSS VSS nfet w=2u l=1u | MO4 o4 ref VSS VSS nfet w=8u l=1u | MO5 o5 ref VSS VSS nfet w=2u l=1u
       brokaw:  Q1 c1 vb x npn m=1 | Q2 c2 vb y npn m=8 | R2 y x rpoly w=2u l=20u | R1 x VSS rpoly w=2u l=80u
                MP1 c1 c1 VDD VDD pfet w=4u l=1u | MP2 c2 c1 VDD VDD pfet w=4u l=1u
       rdiv:    RA top mid rpoly w=2u l=10u | RB mid VSS rpoly w=2u l=40u
       splitdac: C0 tl VSS cap w=3u l=3u m=1 | C1 tl b0 cap w=3u l=3u m=1 | C2 tl b1 cap w=3u l=3u m=2
                 C3 tm b2 cap w=3u l=3u m=1 | C4 tm b3 cap w=3u l=3u m=2 | CA tl tm cap w=3u l=4u m=1
       strongarm: the 15 devices of benchmarks/competition/ALIGN/examples/high_speed_comparator/high_speed_comparator.sp,
                  with models n→nfet, p→pfet, l=14e-9→l=14n, nfin/nf dropped, m kept, and the same explicit
                  w=1u on every device (the .sp carries no w; without one, EXT-11 makes every size unknown and
                  no ExactAs pattern, hence no seed pair, matches; the value is a Philis placeholder).
       ```
     - Negative: `sc_switches` (`MS1 a1 p1 b1 VSS nfet w=1u l=0.15u | MS2 a2 p2 b2 VSS nfet w=1u l=0.15u`), `equal_fets` (`MA da ga VSS VSS nfet w=2u l=0.5u | MB db gb VSS VSS nfet w=2u l=0.5u`), `bjt_mirror`, `inv_chain` (exact: `MN1 a in VSS VSS nfet w=1u l=0.15u | MP1 a in VDD VDD pfet w=2u l=0.15u | MN2 b a VSS VSS nfet w=1u l=0.15u | MP2 b a VDD VDD pfet w=2u l=0.15u | MN3 c b VSS VSS nfet w=1u l=0.15u | MP3 c b VDD VDD pfet w=2u l=0.15u | MN4 out c VSS VSS nfet w=1u l=0.15u | MP4 out c VDD VDD pfet w=2u l=0.15u`).
  3. Create `backend/annotator/tests/align_gold.rs` with the gold transcribed as constants (the file lives in a vendored benchmark tree that may not be checked out): `GOLD_PAIRS = [("mn1","mn2"),("mn3","mn4"),("mp5","mp6"),("mp7","mp8"),("mp9","mp10"),("mp11","mp12"),("mn13","mn14")]`, `GOLD_SELF = ["mn0"]`, `GOLD_NET_PAIRS = [("vin","vip"),("vin_d","vip_d"),("vin_o","vip_o")]`; config from the gold's `PowerPorts vcc`, `GroundPorts vss`, `ClockPorts clk` only (the symmetric entries are **never** input).
- Tests:
  - `tests/corpus.rs::corpus_expectations`: `canon(annotate(c)) == expected(c)` for every circuit. At M0 the assertions use `canon_leaves` and today's expected leaves; each later item updates the expectation of the circuits it changes, in the same commit.
  - `tests/corpus.rs::negative_corpus`: 0 pairs, 0 sets for the 4 negative circuits. `bjt_mirror` and `inv_chain` pass today; `sc_switches` and `equal_fets` are `#[ignore = "passes after EXT-04"]` (today they become DiffPairs, AA-05).
  - `tests/corpus.rs::permutation_invariance`: for every circuit and seeds 1..=20, `canon(permute(c, s)) == canon(c)`; `#[ignore = "passes after EXT-06"]` (terminal-order shuffling also needs EXT-03's name-based roles).
  - `tests/align_gold.rs::strongarm_matches_align_gold`: `#[ignore = "passes after EXT-14"]` until M1; asserts `canon.pairs == GOLD_PAIRS` as a set, `GOLD_SELF ⊆ canon.selfs`, `GOLD_NET_PAIRS ⊆ canon.net_pairs`, `canon.axes == 1`.
  - `tests/scale.rs::twelve_thousand_devices` (`#[ignore]`): 2,500 copies of `ota5t` with every non-rail net and every device suffixed `_k` (k = 0..2499), VDD/VSS shared (17,502 nets, 12,500 devices, below the `u16` limit of EXT-11); wall time ≤ 2.0 s in release (T8).
  - `tests/corpus.rs::no_emitted_conflicts` (T6): over the emitted `placement` arms, no unordered device pair carries both a `Proximity` and an `Isolation`, no device appears in two `Symmetry` pairs. `#[ignore = "passes after EXT-14 and REL-09"]` for `strongarm` with `ClockPorts clk` (today the clocked tail `mn0` gets Proximity ≤ 5 µm and Isolation ≥ 10 µm to `mn1`/`mn2`, AA-13); passes today on the other circuits.
  - `tests/corpus.rs::coverage_is_total` (T7): `#[ignore = "passes after EXT-10"]`.
- Acceptance: the harness runs in `cargo test -p annotator`; the M0 expectations record today's behaviour, so the diff of each later commit shows exactly what changed.
- Risks / notes: the StrongARM transcription must keep the gold's instance names; `m=` is the only size parameter the gold varies.

### EXT-02 One rail classifier, SPICE ground and global names
- Priority: P0. Effort: S. Depends on: none.
- Why: AA-11 (`0`, `gnd!`, `vee`, `VPB/VNB` missed), AA-14/AR-30 (a second rail list in `analog`), AF-04 (a third in `oppoint`); H15-06 (name conventions, hastings.txt L46495–46514, PDF 784; the source gives suffix conventions such as `b`/`z` and `1p5`/`3p3`, not rail names, so the rail roots below are **Philis policy**).
- Current: `SUPPLY_NAMES`/`GROUND_NAMES` exact or `root_` prefix (`netrole.rs:18-19, 34-36`); `analog::placement::matching_pair::is_supply` has its own roots and `contains("gnd")` (`matching_pair.rs:99-106`); `oppoint::is_supply`/`is_ground` (`frontend/library/src/oppoint.rs:392-398`) a third list; node `0` is ground only to oppoint.
- Change:
  1. `backend/annotator/src/netrole.rs`: add
     ```rust
     /// Rail role from a net name alone; the one source of truth for every crate.
     /// Case-insensitive; a trailing `!` (SPICE global) is stripped; `0` is ground;
     /// a root matches when followed by nothing, `_…`, or only [0-9pv] (vdd1, avdd3v3, vdd1p8).
     #[must_use]
     pub fn rail_of(name: &str) -> Option<NetRole>;
     ```
     Roots: supply `vdd vcc vpwr vddio avdd dvdd vdda vddd vpb`; ground `vss gnd vgnd vssio avss dvss vssa vssd vnb vee agnd dgnd` and the literal `0`. (`vee` is the most negative rail of a bipolar circuit, hence ground-class, low impedance; `vpb`/`vnb` are the sky130 body pins.) Body: `let s = name.trim_end_matches('!').to_ascii_lowercase(); if s == "0" { return Some(Ground) }`; then for each root r of the supply list, then the ground list: `if let Some(rest) = s.strip_prefix(r) { if rest.is_empty() || rest.starts_with('_') || rest.bytes().all(|c| c.is_ascii_digit() || c == b'p' || c == b'v') { return Some(role) } }`; else `None`. `NetRole` gains no variant.
  2. `classify_nets` uses `rail_of`, then applies **bulk inference** only when no net got that role by name or config: the net that is the `B` terminal of the most PMOS devices and on no FET gate → `Supply`; same for NMOS → `Ground`. Record the evidence (see EXT-18) as `Structure`.
  3. `classify_nets` (`netrole.rs:28-48`) replaces `matches_rail(SUPPLY_NAMES)`/`matches_rail(GROUND_NAMES)` by `rail_of(name)`; config names still win (`netrole.rs:38-41`). Delete `SUPPLY_NAMES`/`GROUND_NAMES` (`netrole.rs:18-19`).
  4. Consumers elsewhere, not edited here: PERF-09 step 4 replaces `oppoint::is_supply`/`is_ground` (`frontend/library/src/oppoint.rs:392-398`) with `rail_of` (AF-04); `analog::placement::matching_pair::is_supply` is deleted in EXT-09.
- Tests: `netrole::tests::rail_names`: `["0","gnd!","vdd!","VSS!","avdd3v3","vdd1","vdd1p8","VPWR","vgnd","VPB","VNB","vee","vbias","vdd_half"]` → `[G,G,S,G,S,S,S,S,G,S,G,G,None,S]` (`vdd_half` stays Supply by the existing `root_` rule; a diff pair whose gate is named like a rail still fails `gate_is_signal`, `pattern.rs:114-115`, and the sidecar `NetClass` entry of EXT-26 is the override). `netrole::tests::bulk_inference`: netlist with nets `a,b,c`, 2 PMOS bulk on `a`, 2 NMOS bulk on `c` → `a` Supply, `c` Ground; adding a net named `VDD` disables the PMOS inference.
- Acceptance: T2 on `dac4` (VSS/VDD found), any netlist using `0` gets a Ground net (unblocks antenna diodes and fill that need one: `frontend/library/src/lib.rs:412-420, 634`).
- Risks / notes: bulk inference is skipped when a named rail exists, so it never overrides a designer's name.

### EXT-03 Terminal roles by name, not by position
- Priority: P0. Effort: S. Depends on: none.
- Why: AA-16 (BJT collector and resistor P terminals treated as gates; `bjt_mirror` collector nets become Sensitive).
- Current: `const G, D, S, B = 0..3` (`classify.rs:11-15`) applied to every non-capacitor device (`classify.rs:64-85`); `extract.rs:37` reads terminal 0 as the gate.
- Change:
  1. New `backend/annotator/src/terms.rs`:
     ```rust
     #[derive(Clone, Copy, PartialEq, Eq, Debug)]
     pub enum TermRole { FetGate, Channel, Body, BjtBase, Plate, Passive }
     #[must_use]
     pub fn term_role(kind: pnr_core::DeviceKind, term: &str) -> TermRole;
     ```
     FET `G`→FetGate, `D`/`S`→Channel, `B`→Body; BJT `B`→BjtBase, `C`/`E`→Channel; Capacitor `P`/`N`→Plate; Resistor, Diode, Inductor `P`/`N`→Passive.
  2. `classify.rs:62-85` iterates `hg.terminals[d]` with `term_role`: `FetGate` marks `touches_gate` and adds the gate load; `Channel`, `BjtBase` and `Passive` mark `touches_channel` (a DC path); `Body` marks `touches_bulk`; `Plate` marks `on_plate`. The gates-only Sensitive rule (`classify.rs:127`) therefore fires only for FET gates.
  3. `extract.rs:33-39` finds the gate by name (`term_role == FetGate`).
- Tests: `classify::tests::bjt_terminals_are_not_gates`: the `bjt_mirror` netlist → `outn`, `outp`, `in` all `Signal` (today `outn`/`outp` are Sensitive, AA-16). `classify::tests::resistor_ends_are_channels`: `R1 a b` + one NMOS gate on `a` → `a` Signal (driven through a DC path), not Sensitive.
- Acceptance: T2 on `bjt_mirror`.
- Risks / notes: none; pure refactor for FET netlists (test `tests.rs:413-427` still passes).

### EXT-04 Catalog corrections and catalog validation
- Priority: P0. Effort: S. Depends on: EXT-01.
- Why: AA-05 (false DiffPairs and mirrors), AA-06 (wildcard composite), AA-15 (wrong Wilson and Gilbert, dead patterns, `triode_load_pair` electrically wrong), AA-34 (unchecked catalog invariants), AA-30 (doc debris).
- Current: see §1.4 rows AA-05, AA-06/AA-15.
- Change (`backend/annotator/src/catalog.rs`, `pattern.rs`):
  1. Add `PinRel::SameSignal`: same net **and** that net's role is `Signal` (`pattern.rs:12-15`, handled in `links_ok`, `pattern.rs:120-130`, which gains the `roles` argument).
  2. `DIFF_PAIR` (`catalog.rs:118-137`) and `DIFF_SWITCH` (`catalog.rs:2228-2247`): replace `eq(0,"S",1,"S")` by `eq_sig(0,"S",1,"S")`. A pair on a rail source is not a differential pair unless symmetry analysis pairs it (EXT-14, pseudo-differential).
  3. Delete `DIFF_PAIR_SPLIT_SOURCE` (`143-160`) and `MIRROR_PAIR_SPLIT_SOURCE` (`206-223`); the degenerated pair returns in EXT-19 with its degeneration resistors as evidence. Delete `MILLER_COMP_FETS` (`2043-2055`) and `TRIODE_LOAD_PAIR` (`2208-2221`). Delete the dead patterns `CASCODE_MIRROR_OTA` (1955), `CROSS_COUPLED_COMPLEMENTARY` (929), `CURRENT_MIRROR_SINGLE_ENDED` (1419), `STARTUP_MIRROR` (1861), `ACTIVE_LOAD` (299), `DIODE_CASCODE` (1398), `DUMMY_PAIR` (2252). Remove each from `PATTERNS` (`catalog.rs:2572-2687`).
  4. `WILSON_MIRROR` (`492-506`): links `0G=1G, 0S=1S, 1D=2G, 0D=2S` (slot 0 = diode reference M2, slot 1 = M1 input, slot 2 = M3 output; the current links swap the last two).
  5. `GILBERT_CELL` (`1252-1280`): replace both `2G=3G` (`catalog.rs:1275`) and `4G=5G` (`:1276`) by `2G=5G, 3G=4G, 2G≠3G` (keeping `4G=5G` with the new links would force `2G=3G` and contradict `2G≠3G`, so the pattern could never match), and the output links `2D=5D, 3D=4D` (`:1277-1278`) by `2D=4D, 3D=5D` (slot 2/3 on `0D`, slot 4/5 on `1D`).
  6. Delete the doc debris at `1911-1917`, `2294-2299`, `2341-2345`; fix the `gate_is_signal` doc at `2223-2227` ("false = not checked").
- Tests (`backend/annotator/src/catalog.rs` `#[cfg(test)]`):
  - `catalog_is_well_formed`: for every pattern, every `SameTypeAs(r)`/`ComplementOf(r)`/`ExactAs(r)`/`SameLAs(r)` has `r < slot index`; every link endpoint `< slots.len()`; every pin is valid for the slot's device kind; no two patterns have identical slot and link sets.
  - `every_pattern_matches_its_own_minimal_netlist`: instantiate each pattern (union-find over `Same` links, fresh nets otherwise, W/L equal where required, a diode where required, gates on fresh Signal nets) and assert `recognize_all` (EXT-06) returns a match of that template covering all slots.
  - `tests/corpus.rs::negative_corpus_sc_switches_and_equal_fets` (EXT-01's placeholder, `#[ignore = "passes after EXT-04"]`, corpus.rs:155 on m0) loses its `#[ignore]` and passes (T3).
- Acceptance: T3 on the two FET negatives. The three-stage op-amp's M6/M8/M9 are claimed by `push_pull_with_bias` today (audit-01 §2.13-2) and stay there after this item (EXT-05 gives that pattern no constraint); EXT-04 only removes the `miller_comp_fets` wildcard so it can claim no devices in other netlists.
- Risks / notes: removing split-source DPs loses a degenerated pair until EXT-19; the corpus has none, and the pair is re-found by symmetry propagation when a seed exists.

### EXT-05 Declared slot roles in patterns
- Priority: P0. Effort: M. Depends on: EXT-04.
- Why: AA-03 (latch children resolve to inverters), AA-08 (composite roles discarded), AA-23 (every unpaired stage member forced onto the axis); survey Fig. 3.11 building-block → requirement mapping (balasa_graeb_survey.txt L5208–5227, PDF 123; BAL1-50).
- Current: children recovered by re-search (`lib.rs:93-99`); kinds by template substring (`block.rs:42-56`); tail rule for every unpaired member (`emit.rs:203-211`).
- Change:
  1. Roles live in a table beside `PATTERNS`, not in the ~95 `Pattern` literals (a field would touch every literal; the table touches none). `backend/annotator/src/catalog.rs` gains:
     ```rust
     #[derive(Clone, Copy, Debug, Default)]
     pub struct Roles {
         /// Constraint-bearing couples: (slot a, slot b, leaf kind).
         pub pairs: &'static [(u8, u8, crate::block::BlockKind)],
         /// Slots on the structure's symmetry axis (tail, shared bias device).
         pub selfs: &'static [u8],
         /// Adjacent-but-not-matched couples (cascode over its source): P_B.
         pub prox: &'static [(u8, u8)],
     }
     /// Composite declarations keyed by `Pattern::name` (item 2's list).
     pub const ROLES: &[(&str, Roles)] = &[ /* … */ ];
     /// `ROLES` entry, else for a 2-slot pattern today's `BlockKind::from_template` mapping
     /// (DiffPair/CurrentMirror/Load → `pairs: &[(0,1,k)]`; Stack → `prox: &[(0,1)]`), else `Roles::default()`.
     pub fn roles_of(p: &Pattern) -> Roles;
     ```
     `block.rs` `BlockKind` gains `CascodePair` (a symmetric pair of cascodes: matched for symmetry, current kind, minimal class by default). (`#[derive(Default)]` on `Roles` compiles because `&'static [T]: Default`; `BlockKind` needs no `Default`.)
  2. Declarations (`ROLES` entries). 2-slot primitives need no entry: `roles_of` derives them from today's mapping (`block.rs:42-56`); `cmos_inverter` gets an explicit entry `prox: &[(0,1)]` (today it maps to `Group`). Composites (exact lists):
     - `five_transistor_ota`, `diff_pair_with_active_load`, `diff_pair_with_mirror_load`: pairs `(0,1,DiffPair),(2,3,Load)`; selfs `[4]` where slot 4 exists.
     - `diff_pair_with_tail`: pairs `(0,1,DiffPair)`; selfs `[2]`.
     - `cascoded_diff_pair_with_tail`, `diff_pair_cascode_load`: pairs `(0,1,DiffPair),(2,3,CascodePair)`; selfs `[4]`; prox `(0,2),(1,3)`.
     - `telescopic_ota_core`: pairs `(0,1,DiffPair),(2,3,CascodePair),(4,5,Load)`; prox `(0,2),(1,3),(2,4),(3,5)`. `telescopic_ota_full`: the same plus selfs `[6]`; slot 7 gets no role.
     - `folded_cascode_core`: pairs `(0,1,DiffPair),(2,3,CascodePair),(4,5,Load)`; prox `(4,2),(5,3)`.
     - `cross_coupled_inverters`: pairs `(0,1,DiffPair),(2,3,DiffPair)`; prox `(0,2),(1,3)`.
     - `vco_core_with_tails`: pairs `(0,1,DiffPair),(2,3,DiffPair)`; selfs `[4,5]`.
     - `gilbert_cell`: pairs `(0,1,DiffPair),(2,5,DiffPair),(3,4,DiffPair)`.
     - `cascode_mirror`, `wide_swing_cascode_mirror`, `low_voltage_cascode_mirror`: pairs `(0,1,CurrentMirror),(2,3,CascodePair)`; prox `(0,2),(1,3)` (survey Fig. 3.11 T1–T3, T2–T4).
     - `current_mirror_3`, `current_mirror_4`, `current_mirror_1_to_2`: pairs `(0,k,CurrentMirror)` for every output slot k.
     - Every other composite: `pairs: &[], selfs: &[], prox: &[]` (recognized, reported, no constraint), until a source-backed declaration is added.
  3. `Block` (`block.rs:8-19`) gains `template: &'static str` and `selfs: Vec<DeviceId>`; `Block::from_match` builds `sub_blocks` from `roles_of(pattern).pairs` and `.prox` directly (one 2-device child `Block` per couple, kind from the triple, `Stack` for a prox couple). `PatternMatch` gains `pattern: &'static Pattern` (or an index into `PATTERNS`) so `from_match` can reach the roles. Delete the re-search `lib.rs:93-99`.
  4. `emit.rs:203-211`: iterate `stage.selfs` instead of "every member outside a pair".
- Tests: `tests/corpus.rs::corpus_expectations` for `latch` (2 DiffPair leaves), `gilbert` (3 DiffPair leaves: (M1,M2),(M3,M6),(M4,M5)), `folded` (DiffPair, CascodePair, Load leaves); `tests.rs::a_differential_stage_is_symmetric_about_one_axis` still passes; new `tests.rs::telescopic_slot7_is_not_self_symmetric`; `catalog::tests::roles_are_well_formed` (every `ROLES` name exists in `PATTERNS`, no duplicate names, every slot index `< slots.len()`, no slot in two `pairs`).
- Acceptance: AA-03 closed (latch pairs constrained), AA-23 closed.
- Risks / notes: the declaration table is data; a wrong entry is caught by `roles_are_well_formed` and `every_pattern_matches_its_own_minimal_netlist`. Gilbert: the declared couples are the mirror images (M3,M6), (M4,M5) (what EXT-14 needs as seeds); the source-coupled switching pairs (M3,M4), (M5,M6) are not declared, so until a quad-set rule exists the switching quad is two 2-member sets, not one 4-member set (recorded in "Cut or deferred items").

### EXT-06 Overlapping recognition, canonical order and a faster matcher
- Priority: P0. Effort: M. Depends on: EXT-04.
- Why: AA-01 (one structure per device), AA-07 (netlist-order dependence), AA-22 (all-device scan, permutations, O(M²) dedupe); survey SMP multigraph where a device carries several requirements (balasa_graeb_survey.txt L5146–5157, PDF 122).
- Current: `recognize` returns a disjoint set (`pattern.rs:175-212`); ties by device id (`pattern.rs:199`).
- Change (`backend/annotator/src/pattern.rs`):
  ```rust
  /// Every match of every allowed pattern; one per (template, device set).
  #[must_use]
  pub fn recognize_all(hg: &BipartiteHypergraph, geom: &[Geom], roles: &[NetRole], cfg: &AnnotationConfig) -> Vec<PatternMatch>;
  /// Permutation-invariant device labels: 3 rounds of Weisfeiler–Lehman refinement of
  /// (kind, model, w, l, fingers, per terminal (name, net role, net degree)).
  #[must_use]
  pub fn canonical_labels(hg: &BipartiteHypergraph, geom: &[Geom], roles: &[NetRole]) -> Vec<u64>;
  /// Disjoint subset for `Problem::blocks` (interim, replaced by the EXT-13 tree):
  /// priority desc, then patterns that declare `pairs` (EXT-05) before those that declare none,
  /// then the sorted canonical labels of the instances.
  #[must_use]
  pub fn select_disjoint(all: &[PatternMatch], canon: &[u64]) -> Vec<PatternMatch>;
  ```
  The second key is what keeps `diff_pair` (priority 10, declares a pair) ahead of `push_pull_pair` (priority 10, declares nothing) on the rail-to-rail input (AA-07 trace §2.13-7b) while emission still reads the disjoint blocks (until EXT-13/14).
  Note: today's `Geom` carries only `w` and `l` (`pattern.rs:67-71`) and `BipartiteHypergraph` carries no model (`kernel/core/src/hypergraph.rs:11-22`), so the model and finger terms of the label exist only after EXT-11 turns `Geom` into `Drawn`; until then the label uses (kind, w, l, per-terminal name, net role, net degree).
  Matcher changes in `Search::run` (`pattern.rs:143-167`):
  1. Candidates for slot k > 0: if a `Same`/`SameSignal` link joins slot k's pin `pk` to an assigned slot a's pin `pa`, iterate `hg.net_devices[net(assigned[a], pa)]` filtered to devices whose `pk` terminal is on that net (deduplicated); otherwise scan all devices (slot 0 only in practice).
  2. `seen: HashSet<Vec<u32>>` (keyed per pattern) instead of `Vec::contains`.
  3. Pin lookup from a precomputed `pins: Vec<[Option<NetId>; 8]>` indexed by terminal in the order G, D, S, B, C, E, P, N (the order of EXT-12's `Term`; a local enum until EXT-12 lands) instead of the linear `position` in `pin_net` (`pattern.rs:81-84`).
  `annotate` keeps all matches for EXT-13 and uses `select_disjoint` for `blocks`.
- Tests: `pattern::tests::recognize_all_is_a_superset_of_select_disjoint`; `tests/corpus.rs::permutation_invariance` (`#[ignore = "passes after EXT-06"]`, corpus.rs:163 on m0) loses its `#[ignore]` and passes for every circuit on `canon_leaves`; `rail2rail` yields both DiffPair leaves (MN1/MN2 and MP1/MP2) in both device orders (AA-07 trace §2.13-7b); `tests/scale.rs::twelve_thousand_devices` (`#[ignore]`d by EXT-01 with the measured miss, > 13 min against 2.0 s) loses its `#[ignore]` and meets T8.
- Acceptance: T4 on the interim canonical form; T8.
- Risks / notes: WL labels can tie for non-automorphic devices on a hash collision; the fallback is the device id, documented with a `ponytail:` comment naming the ceiling.

### EXT-07 Sensitivity marked from matching only
- Priority: P0. Effort: S. Depends on: EXT-05.
- Why: AA-17 (Stack gate nets Sensitive, with 8× spacing, shields and isolation-victim status).
- Current: `is_sensitive` includes `Stack` (`block.rs:60-62`).
- Change: `BlockKind::is_sensitive` = `DiffPair | CurrentMirror | Load`; `CascodePair` and `Stack` are not. Stack and cascode devices stop being isolation victims (`emit.rs:273-278` reads `sensitive`) and stop making their gate nets Sensitive through `gate_of_sensitive` (`classify.rs:79-81`). A gate net that touches only gates stays Sensitive under the gates-only rule (`classify.rs:127`), which this item does not change; EXT-18 reclassifies such nets as `Bias`.
- Tests: `tests.rs::a_cascode_stack_is_adjacent_not_matched` extended: M1 and M2 are not in any `Isolation` as victims; a cascode gate net that also touches a channel (constructed variant) is `Signal`; the gates-only nets `vcas` (that test) and `chain4`'s `g` remain `Sensitive` until EXT-18.
- Acceptance: T2 on `chain4`, `folded`.
- Risks / notes: none.

### EXT-08 (cut — see "Cut or deferred items")
The only concrete conflict (AA-13, clocked tail with Proximity and Isolation to its own pair) is removed at its source by REL-09 step 3 (no isolation inside a block) and, after EXT-23, by skipping pairs in one compound or set; EXT-14 cannot put a device in two pairs or on two axes by construction (partial involution, compounds = components). T6 stays as an assertion (`tests/corpus.rs::no_emitted_conflicts`, EXT-01).

### EXT-09 Differential and crosstalk from recognized pairs; `Differential` to the budget arm
- Priority: P0. Effort: M. Depends on: EXT-05.
- Why: AA-14, AR-12 (a toleranced 5 % rule registered hard), AR-39 (no W/L/model check), AR-19 (dead extract paths).
- Current: `extract.rs:53-54` pushes `Differential::extract` (built on `is_diff_pair`, `differential.rs:116-133`) into `r.hard`; `extract.rs:57-61` uses `CrosstalkExclusion::extract` (`crosstalk.rs:95-114`).
- Change:
  1. `extract::routing` takes the DiffPair leaves (`block::leaves` with kind DiffPair): for each `(a, b)` push `Differential { pos: D(a), neg: D(b), max_len_delta_pct10: policy.diff_pct10 (50), same_layer_required: true, stack }` into **`r.budget`**, and the gate×drain `CrosstalkExclusion` set of today's semantics (`crosstalk.rs:90-94`) from the same pairs. After EXT-14 the source becomes the compounds' net pairs (EXT-24).
  2. Delete `Differential::extract` (`differential.rs:115-133`), `CrosstalkExclusion::extract` (`crosstalk.rs:90-114`), `is_diff_pair` and `is_supply` (`matching_pair.rs:80-106`) and their imports (`crosstalk.rs:7`, `differential.rs:6`); `Rule::extract`'s default (`rule.rs:121-129`) covers both types.
  3. Coordinated edits with RTE (mechanical, no semantic change): `backend/gr/src/lib.rs:277` (`symmetric_nets`) and `backend/dr/src/lib.rs:811` iterate `reqs.hard.iter().chain(&reqs.budget)` when filtering `Differential`. `backend/gr/src/lib.rs:253` excludes `Differential` from the hard tier and needs no change: `tier` (`gr/src/lib.rs:262-266`) tests the `sym` list before the budget list.
- Tests: `tests.rs::differential_comes_from_recognized_pairs`: `dac4` with its ground renamed from `VSS` to `0` emits 0 `Differential` (today 10: `0` is not a rail to `is_supply`, so every pair of the five NMOS on `0`, the four inverter NMOS plus `XMC`, pairs up; the AA-14 hand trace counts 6 because it omits `XMC`); `ota5t` emits 1, in the budget arm. `backend/gr` test at `gr/src/lib.rs:1387` moved to `budget` still orders the pair first.
- Acceptance: no hard `Differential` anywhere (grep of emitted arms in `tests.rs::budget_rules_land_in_exactly_one_partition`).
- Risks / notes: `Differential` measurement semantics are RTE's.

### EXT-10 Stable IDs, provenance, coverage, relevant `missing`, one policy table
- Priority: P0. Effort: M. Depends on: EXT-06.
- Why: AA-27 (template dropped, no coverage report), AA-29 (`missing` not relevance-conditioned), AA-28 (scattered literals), NOTES-03/04 (coverage certificate, stable IDs), BAL2-45 (typed records with origin, balasa_graeb_survey.txt L13594–13599, L13667–13736, PDF 282–284).
- Current: prices re-find batches by `(kind(), ordinal)` (`kernel/analog/src/requirements.rs:16-18`; `backend/gp/src/lib.rs:115-116`); `missing()` unconditional (`lib.rs:53-71`).
- Change:
  1. `kernel/analog/src/rule.rs`: add to `RuleBatch<On>` `fn meta(&self) -> Option<&crate::intent::BatchMeta> { None }` and
     ```rust
     /// A batch with its identity; every other method delegates to `inner`.
     pub struct Tagged<B> { pub meta: crate::intent::BatchMeta, pub inner: B }
     impl<On, B: RuleBatch<On>> RuleBatch<On> for Tagged<B> { /* delegate all 18 methods (rule.rs:135-208); meta() -> Some */ }
     ```
     `BatchMeta` and `ConstraintId` are defined in `analog::intent` (EXT-12; created here with only these two types). Every batch the annotator pushes is `Box::new(Tagged { meta, inner: vec![..] })`. IDs are dense, assigned after sorting emissions by canonical labels (EXT-06), so a permuted netlist gets the same ids. PLC re-keys `gp::Prices` on `meta().id` when present (PLC item; the fallback key stays `(kind, ordinal)`).
  2. `Problem` gains `coverage: Vec<(DeviceId, Coverage)>` with `enum Coverage { Constrained, Grouped(&'static str /*template*/), Unconstrained(&'static str /*reason*/) }`; reasons: `"no pattern"`, `"do_not_identify"`, `"unknown size"`.
  3. `missing(p, needs)` (`lib.rs:53-71`) with `struct Needs { matched: bool, gate_nets: bool, budgeted_nets: bool }`, computed in `annotate` after emission: the matching entries (`"MatchingPair"`, renamed `"MatchedSet"` by MAT-04 step 6) only when `matched` (≥ 1 DiffPair/CurrentMirror/Load leaf), `Antenna` only with a FET gate net, `ParasiticBudget`/`CouplingBudget` only when some net got a budget. The `Isolation` entry is emitted by `emit::isolation` today (`lib.rs:137-139`) and REL-09 rewrites it; untouched here.
  4. New `backend/annotator/src/policy.rs`: `#[derive(Clone, Debug)] pub struct Policy` with `impl Default` giving today's values, one doc line per field naming its status (source or **Philis policy**). Fields (the AA-28 literals EXT still owns, plus the knobs later EXT items add): `proximity_nm: i32 = 5000` (`emit.rs`), `spacing_multiple: [i32; 4] = [8, 7, 3, 1]` (Sensitive, Clock, Signal, other) and `margin_pct: [u8; 4] = [35, 30, 25, 20]` (Sensitive, Clock, Supply|Ground, other) (`extract.rs:110-127`), `shield_coverage_pct: i32 = 80` (`extract.rs:98`), `shield_gap_spaces: i32 = 2` (`extract.rs:99-100`), `antenna_margin_pct: i32 = 20` (`extract.rs:47`), `diff_pct10: i32 = 50` (EXT-09), `ir_headroom_share: f32 = 0.1`, `ir_rail_share: f32 = 0.01`, `ir_high_current_share: f32 = 0.1` (`ir.rs:18-27`), `pn_max_degree: usize = 8` (EXT-13), `beta_target: f64 = 3.0` (EXT-21), `max_eta: f32 = 3.0` (EXT-21). Not in `Policy`, because other plans delete or own them: the thermal limit, `GRADIENT_SHARE` and the pooled-CC literals (`emit.rs:40-52`, deleted by MAT-04), the isolation multiple and nominal epi (`emit.rs:250-256`, REL-09's `Substrate`), the ring literals (`constraints.rs:79-81`, REL-07). `AnnotationConfig` gains `pub policy: Policy`; every listed literal is read from it.
  5. Library (FLOW coordinated edit): `metadata::build` (`frontend/library/src/metadata.rs:177-185`) takes the coverage and the template histogram and prints a `RECOGNITION` section (templates matched with counts) and an `UNCONSTRAINED` list.
- Tests: `tests.rs::ids_survive_permutation` (the id of the DP's `MatchingPair` batch is equal across 5 permutations); `tests.rs::missing_is_relevant` (a glue-only netlist lists no `MatchingPair`); `tests/corpus.rs::coverage_is_total` (T7; EXT-01 left it `#[ignore]`d with no body, corpus.rs:241 on m0, because `Problem` has no coverage before this item: write the body, drop the `#[ignore]`).
- Acceptance: T7; every emitted batch has `meta().is_some()`.
- Risks / notes: `Tagged` is additive; consumers that ignore `meta` are unaffected.

### EXT-11 Size, model and bulk robustness
- Priority: P0. Effort: S. Depends on: FLOW-01 (`Device::mos_size()`, `Device::gate_area_um2()` in `kernel/core/src/netlist.rs`; FLOW-01 step 3 also rewrites `annotator::gate_um2`, `lib.rs:155-161`, and `constraints::fingers`, `constraints.rs:21-24`).
- Why: AA-20 (model and bulk ignored: two flavours merged into one cell, LVS mismatch), AA-21 (missing W/L = 0 makes devices "identical"), AA-35 (`u16` wrap). The size convention and the gate-area copies (AA-32/AR-46) are FLOW-01's.
- Current: `Geom { w, l }` from params with default 0 (`pattern.rs:67-71`, `lib.rs:77-81`); `ExactAs` compares W and L only (`pattern.rs:106`); `SameLAs` compares L only (`pattern.rs:107`); unitization key (kind, finger W, L) (`constraints.rs:30-35`; since FLOW-01 the MOS width is `mos_size().w_finger_nm()`, and `constraints::fingers` and `annotator::gate_um2` already read `Device::mos_size()`/`gate_area_um2()`).
- Change:
  1. New `backend/annotator/src/size.rs`:
     ```rust
     #[derive(Clone, Copy, Debug, PartialEq, Eq)]
     pub struct Drawn { pub w_finger_nm: Option<i64>, pub l_nm: Option<i64>, pub fingers: u32, pub model: u16, pub bulk: Option<NetId> }
     /// MOS: from `dev.mos_size()` (FLOW-01: w_finger = W_total/nf, fingers = nf·m); `None` size → both `None`, fingers 1.
     /// Other kinds: `w`, `l` params as written (`None` when absent), fingers = `m` (default 1).
     /// `model` interns `dev.model` (case-insensitive) into `models`; `bulk` = the FET `B` net, else `None`.
     pub fn drawn(dev: &pnr_core::netlist::Device, models: &mut Vec<String>) -> Drawn;
     ```
  2. `pattern::Geom` is replaced by `Drawn` (`recognize`/`slot_ok` take `&[Drawn]`). `ExactAs(r)` holds iff `w_finger_nm` and `l_nm` are both `Some` and equal, `model` is equal, and the bulk nets are equal or both rails (role Supply/Ground). `SameLAs(r)` requires `l_nm` `Some` and equal and equal `model`. Bipolars and diodes written without W/L are fixed-geometry PDK devices whose size is in the model (`bgr_core`: `pnp_05v5_W3p40L3p40`, no W/L params): for them `ExactAs` holds iff the models are equal. Any other device with an unknown size never matches `ExactAs` and adds `Coverage::Unconstrained("unknown size")` (EXT-10) plus a `missing` entry `("MatchingPair", "device W/L")`.
  3. Unitization key (`constraints.rs:30-33`, replaced later by EXT-15) becomes `(kind, model, bulk, w_finger_nm, l_nm)`.
  4. `annotate` asserts `netlist.devices.len() <= usize::from(u16::MAX)` and `netlist.nets.len() <= usize::from(u16::MAX)` before building ids, with the message `"annotator: {n} devices/nets exceed the u16 id space (65535)"` (AA-35; `lib.rs:103`, `block.rs:69`, `classify.rs:105` cast with `as u16`).
- Tests: `size::tests::flavours_do_not_match`: `nfet_01v8` + `nfet_01v8_lvt`, same W/L, shared source, distinct gates → no DiffPair, two unitization classes. `size::tests::unknown_size_never_matches`: two FETs without `w` → no DiffPair, coverage reason `"unknown size"`. `size::tests::split_bulk_does_not_match`: same W/L/model, bulks on two distinct non-rail nets → no DiffPair.
- Acceptance: T2 unchanged on the repository fixtures (all sizes known, one model per kind).
- Risks / notes: fixtures written with per-finger `w` change size under FLOW-01 (FLOW OQ-1); only `drawn` reads the convention.

### EXT-12 The constraint data model contract (`analog::intent`)
- Priority: P1. Effort: M. Depends on: EXT-10; MAT-07 step 1 for `MatchClass`/`MatchKind`/`Family` (if MAT-07 has not landed, this item creates `kernel/analog/src/matching/mod.rs` with `pub mod class;`, adds `pub mod matching;` to `kernel/analog/src/lib.rs:13-19`, and creates `matching/class.rs` with exactly MAT-07's three enums — `MatchClass { Minimal, #[default] Moderate, Exceptional }` deriving `Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default`, `MatchKind { Voltage, Current, Ratio }`, `Family { Mos, Resistor, Capacitor, Bipolar, Diode }`; MAT-07 later adds the tables to that file).
- Why: the task's contract requirement; NOTES-04/05 (identity, parents); BAL2-45; AA-01 (overlapping requirements need a home other than the disjoint `blocks`); AC-22 (cells need a matching grade).
- Current: the only shared types are `Requirements` (`kernel/analog/src/requirements.rs:19-23`), `Constraints { unitization, guard_rings }` (`kernel/analog/src/constraints.rs:1-11`), `NetClassification` (`kernel/analog/src/metadata.rs:8-15`).
- Change: new `kernel/analog/src/intent.rs` (`pub mod intent;` in `kernel/analog/src/lib.rs`; no root-level `pub use` of its types, so `analog::intent::Intent` never meets `verify::Intent`), depending only on `pnr_core` and `crate::matching::class`:
  ```rust
  pub use crate::matching::class::{Family, MatchClass, MatchKind};   // MAT-07; MatchClass must derive PartialOrd, Ord (Minimal < Moderate < Exceptional)
  #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)] pub struct ConstraintId(pub u32);
  #[derive(Clone, Debug)] pub enum Origin {
      Pattern { template: &'static str }, Symmetry { seed: ConstraintId }, SharedBias,
      PassiveSet { rule: &'static str }, Sensitivity, User { index: u32 }, Derived { parent: ConstraintId },
  }
  #[derive(Clone, Debug)] pub struct BatchMeta { pub id: ConstraintId, pub origin: Origin, pub family: &'static str }

  /// User and Spec count as explicit for MAT-08 (`MatchedSet::class_explicit = source != Role`).
  #[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum ClassSource { User, Spec, Role }
  #[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Half { A, B }
  #[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Term { G, D, S, B, C, E, P, N }
  #[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum ArrayStyle { Adjacent, Interdigitated, CommonCentroid1d, CommonCentroid2d, Any }

  #[derive(Clone, Copy, Debug)] pub struct Member { pub device: DeviceId, pub parallel: u16, pub series: u16, pub half: Option<Half> }
  #[derive(Clone, Copy, Debug)] pub struct UnitGeom { pub w_nm: i32, pub l_nm: i32, pub model: u16 }
  /// What extraction knows about one matched set. The rule that scores it is MAT-04's
  /// `analog::placement::matched_set::MatchedSet`, built from this by EXT-20 (different type, different module).
  #[derive(Clone, Debug)] pub struct MatchSpec {
      pub id: ConstraintId, pub origin: Origin, pub members: Vec<Member>,
      /// A mirror's diode reference or a ratio bank's unit member (index into `members`); MAT-04 puts it in slot 0.
      pub reference: Option<usize>,
      pub family: Family, pub kind: MatchKind, pub class: MatchClass, pub class_source: ClassSource,
      /// `None`: the ratio is not an integer multiple of one unit (CELL/MAT handle it).
      pub unit: Option<UnitGeom>,
      /// 1σ systematic allowance for the whole set (mV for Voltage, % for Current/Ratio), from EXT-21. `None` = not allocated.
      pub allowance: Option<f32>,
      /// Share of the spec variance this set explains, 0..=1; `None` without sensitivities.
      pub weight: Option<f32>,
      pub style: ArrayStyle,
      pub compound: Option<u16>,
  }
  #[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum AxisDir { Vertical, Horizontal }
  #[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum SymKind { Mirror, Perfect }
  #[derive(Clone, Debug)] pub struct Compound {
      pub id: ConstraintId, pub axis: AxisId, pub dir: AxisDir, pub kind: SymKind,
      pub pairs: Vec<(DeviceId, DeviceId)>, pub selfs: Vec<DeviceId>,
      pub net_pairs: Vec<(NetId, NetId)>, pub self_nets: Vec<NetId>,
      /// Indices into `Intent.sets` of spec pairs that mirror each other (nested symmetry).
      pub set_pairs: Vec<(u16, u16)>,
  }
  #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)] pub enum ReqType { MatchSym, MatchBlock, ProxBlock, Sym, ProxNet }
  #[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum GroupKind { Matching, Symmetry, Proximity, Root }
  #[derive(Clone, Debug)] pub struct GroupNode { pub kind: GroupKind, pub devices: Vec<DeviceId>, pub children: Vec<u32> }

  #[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Region { Unknown, Off, Subthreshold, Triode, Saturation }
  #[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum DeviceRole { Unknown, Amplifier, CurrentSource, Cascode, Load, Switch, Diode, Passive }
  #[derive(Clone, Copy, Debug)] pub struct DeviceFacts { pub region: Region, pub role: DeviceRole }
  #[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum RcClass { Unknown, None, R, C, Rc }
  #[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum EvidenceLevel { User, Port, Name, Testbench, Structure, OpPoint, Default }
  #[derive(Clone, Debug)] pub struct NetFacts {
      pub evidence: EvidenceLevel, pub port: bool, pub dc_mv: Option<(i32, i32)>,
      pub rc: RcClass, pub z_ohm: Option<f32>, pub shield_ref: Option<NetId>,
  }
  /// Minority injection is REL-08's `rings::Injector`, not an EXT tag.
  #[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Inject { Switching, Capacitive }
  #[derive(Clone, Debug)] pub struct Aggressor { pub device: DeviceId, pub inject: Inject, pub reason: &'static str }
  #[derive(Clone, Debug)] pub struct Victim { pub device: DeviceId, pub weight: f32, pub reason: &'static str }
  #[derive(Clone, Debug)] pub struct CommonNodeReq { pub net: NetId, pub set: u16, pub a: Vec<DeviceId>, pub b: Vec<DeviceId>, pub term: Term }
  #[derive(Clone, Debug)] pub struct StarReq { pub net: NetId, pub root: Option<(DeviceId, Term)>, pub branches: Vec<(DeviceId, Term)> }
  #[derive(Clone, Debug)] pub struct KelvinReq { pub device: DeviceId, pub term: Term, pub sense: Vec<(DeviceId, Term)> }
  #[derive(Clone, Debug)] pub struct Diagnostic { pub kind: &'static str, pub devices: Vec<DeviceId>, pub message: String }

  #[derive(Clone, Debug, Default)] pub struct Intent {
      pub sets: Vec<MatchSpec>, pub compounds: Vec<Compound>, pub tree: Vec<GroupNode>,
      pub nets: Vec<NetFacts>, pub devices: Vec<DeviceFacts>,
      pub aggressors: Vec<Aggressor>, pub victims: Vec<Victim>,
      pub common_nodes: Vec<CommonNodeReq>, pub stars: Vec<StarReq>, pub kelvins: Vec<KelvinReq>,
      pub diagnostics: Vec<Diagnostic>,
  }
  ```
  Contract change in an existing type (`kernel/analog/src/cell.rs:40-56`): `Unitization` gains `class: Option<MatchClass>` (`None` = not inferred; readers use `unwrap_or(MatchClass::Moderate)`, MAT open question 4), `series: Vec<u16>` (per member; empty = all 1), `style: Option<ArrayStyle>` (`None` = today's choice). CELL plan §0.2 expects exactly these three fields; MAT-07 step 5's non-`Option` `class` is superseded by this field (MAT sets the same default through `unwrap_or`). Every full struct literal of `Unitization` is listed in MAT-07 step 5 (`constraints.rs:49`, `cellgen.rs:493,515,537,562,1257`, `kernel/cells/src/lib.rs:72`, `capacitor.rs:387`, `cap_array.rs:601`, `kernel/cells/tests/cell_selfcheck.rs:45,368`, `kernel/macroMaster/src/adapter.rs:29`, `examples/three_stage_opamp/src/main.rs:94`) and gets `class: None, series: Vec::new(), style: None`. `GuardRingRequirement`/`GuardRingType` are REL-07's and CELL-05's; this item does not touch them.
  `annotator::Problem` gains `intent: analog::intent::Intent` and `axis_count: usize`; `Problem::coverage` (EXT-10) stays on `Problem`. `blocks`, `groups`, `abutment` keep their D16 meaning.
  Coordinated edit: `frontend/library/src/lib.rs:580-583` resizes `coarse.axis` to `problem.axis_count` (PLC-06 step 2 deletes these lines and passes `n_axes`; whichever lands second uses `axis_count`).
- Tests: `kernel/analog/src/intent.rs` unit test `match_class_orders` (`Minimal < Moderate < Exceptional`); compile-level: every `Unitization` literal site above updated.
- Acceptance: the workspace builds; no behaviour change until EXT-13+ fill `intent`.
- Risks / notes: the only item that touches types consumed by other crates; land it alone. Name clash: `verify::Intent` exists (`backend/verify/src/lib.rs:58`) and the library's `Flow` holds one (`frontend/library/src/lib.rs:95, 477`; built by `elaborate::intent`, `elaborate.rs:297-302`), so the library refers to `analog::intent::Intent` by path. `MatchSpec` is named so because MAT-04 adds `analog::placement::matched_set::MatchedSet`.

### EXT-13 Requirement graph and group tree (HSMPG)
- Priority: P1. Effort: M. Depends on: EXT-06, EXT-12 (EXT-14 supplies M_S/S edges; without it the tree is built from M_B/P_B/P_N only).
- Why: AA-01, AA-08; BAL1-48/49/52/53; survey §3.2.1 types (balasa_graeb_survey.txt L5013–5063, PDF 119–120), importance order (L5096–5132, PDF 121), SMP graph with a proximity clique per net (L5146–5157, PDF 122), Algorithm 3.1 (§3.2.3.1, L5369–5408, PDF 126–127), constraints from the tree (L5613–5650, PDF 129).
- Current: disjoint blocks (`pattern.rs:201-210`); two levels (`lib.rs:93-99`).
- Change: new `backend/annotator/src/graph.rs`:
  ```rust
  pub struct Req { pub a: DeviceId, pub b: DeviceId, pub ty: ReqType, pub source: ConstraintId }
  pub fn requirements(matches: &[PatternMatch], compounds: &[Compound], shared_bias: &[Vec<DeviceId>],
                      passive: &[Vec<DeviceId>], hg: &BipartiteHypergraph, classes: &[NetClassification],
                      canon: &[u64], policy: &Policy) -> Vec<Req>;
  pub fn hsmpg(n_devices: usize, reqs: &[Req]) -> Vec<GroupNode>;
  ```
  Edges:
  - `MatchSym`: every `(a, b)` in `compound.pairs` of every compound (EXT-14) — the symmetric couples only, never all device pairs of a compound.
  - `MatchBlock`: every declared pair of every match (EXT-05) not already `MatchSym`; each shared-bias group (EXT-15) as a star from its reference; each passive set (EXT-19) as a star.
  - `ProxBlock`: every declared `prox` couple and every `selfs` device to its pattern's first pair.
  - `Sym`: consecutive members of each compound (a spanning path gives the same components as the survey's clique).
  - `ProxNet`: for each net whose class is not Supply, Ground, Substrate, Clock or DigitalSwitching and whose device degree ≤ `policy.pn_max_degree` (8, **Philis policy**: the survey adds a clique per net and does not say whether rails are excluded; BAL1-49 notes one component would swallow the circuit if they were not), a star from the canonically first device.
  `hsmpg` (Algorithm 3.1): `super[i] = i`; for `ty` in importance order: union-find over super-nodes on edges of type `ty`; every class of size > 1 becomes a `GroupNode` (`Matching` for MatchSym/MatchBlock, `Symmetry` for Sym, `Proximity` otherwise) whose children are the class's super-nodes; contract. Finally a `Root` over the remaining super-nodes. Edges are sorted by `(ty, canon[a], canon[b])` first, so the tree is permutation-invariant.
  `Problem.blocks` = the root's children (each a `Block` with kind `Group`, or the primitive's kind for a 2-device `Matching` node); root-level singletons form the trailing `Glue` block (glue last, as `Problem::blocks` documents at `lib.rs:30-31`; D16's two tables `groups`/`abutment` kept; one axis slot per block no longer needed because axes are per compound).
- Tests: `graph::tests::ota5t_tree`: `ota5t` → Matching{XM1,XM2}, Matching{XM3,XM4}, Proximity{Matching{XM1,XM2}, XM5} (the declared tail edge is P_B, contracted before S), Symmetry{that Proximity node, Matching{XM3,XM4}}, Root. `graph::tests::tail_in_two_requirements`: `three_stage` → M3 is in the DP's Symmetry group **and** has a `MatchBlock` edge to M7 and M9 (shared bias, AA-01). Corpus T4 on the tree shape.
- Acceptance: AA-01 closed on `three_stage` and `mirror6` (one group of 6); T4.
- Risks / notes: `ProxNet` caps are the knob that controls cluster size; report the largest group size in the metadata. Algorithm 3.1 contracts higher-importance groups first, so a shared-bias group that contains a compound's self-symmetric tail (M3/M7/M9 in `three_stage`) ends up inside that compound's Symmetry node; PLC treats non-pair, non-self members of a Symmetry node as island members, not axis devices.

### EXT-14 Circuit-level symmetry analysis (compounds)
- Priority: P1. Effort: L. Depends on: EXT-05, EXT-11, EXT-12.
- Why: AA-02, AA-23, AR-11 (partner geometry), AP-12 (one axis per block); survey §3.2.2.2: all symmetric device pairs with one axis form a compound, one M_S per pair and one S per compound (balasa_graeb_survey.txt L5230–5241, 5285–5310, PDF 123–125; the algorithm is only named as "similar to [10]", so the propagation below is Philis design); symmetry types mirror / perfect / self (L675–711, PDF 21–22); hierarchical symmetry (L4361–4372, PDF 105); Lampaert couples, self-symmetric devices, groups sharing one axis (lampaert.txt L3316–3336, PDF 84).
- Current: symmetry only from 2-device leaves, one axis per top-level block (`emit.rs:138-139, 158, 212-215`).
- Change: new `backend/annotator/src/symmetry.rs`:
  ```rust
  pub struct Sig { pub kind: DeviceKind, pub model: u16, pub w: Option<i64>, pub l: Option<i64>, pub units: u32 }
  pub enum Seed { Devices(DeviceId, DeviceId, ConstraintId), Nets(NetId, NetId, ConstraintId), SelfDevice(DeviceId, ConstraintId) }
  pub fn analyze(hg: &BipartiteHypergraph, sig: &[Sig], classes: &[NetClassification], seeds: &[Seed],
                 canon: &[u64], policy: &Policy) -> (Vec<Compound>, Vec<Diagnostic>);
  ```
  Seeds: declared `DiffPair` and `CascodePair`/`Load` pairs of every match (EXT-05), in canonical order; sidecar `SymmetricBlocks`/`SymmetricNets` (EXT-26).
  Algorithm:
  1. State: a partial involution `pair_of: Vec<Option<DeviceId>>` on devices, `net_mate: Vec<Option<NetId>>` on nets (a net mapped to itself is self-symmetric), a FIFO of net pairs, `comp_of` per device (union-find over compounds).
  2. Accept a seed device pair (a, b) only if `sig[a] == sig[b]`. For each terminal name t of a: `(na, nb) = (net(a,t), net(b,t))`; `bind(na, nb)`.
  3. `bind(x, y)`: rails (Supply, Ground, Substrate classes) are fixed points and never enqueued; if `x == y` mark self-symmetric (enqueue once); if both unmated set `net_mate[x] = y`, `net_mate[y] = x`, enqueue; if already mated consistently do nothing; else record an inconsistency (the candidate that caused it is rejected, step 4).
  4. Pop a net pair (x, y), x ≠ y. Candidates: devices d on x and d' on y, not yet paired, with `sig[d] == sig[d']` and the same terminal name on x and y. A candidate is **consistent** if for every other terminal t, `(net(d,t), net(d',t))` is mated to each other, or both unmated (would be bound), or equal (a shared net: allowed, it carries no constraint), or both rails. Rank consistent candidates of d by `(shared channel nets, shared gate nets)` ascending; pair d with d' only if the minimum is unique for both d and d' (mutual unique best); otherwise leave both unpaired and emit `Diagnostic { kind: "ambiguous_symmetry" }`. Pairing binds every other terminal (step 3). Two-terminal passives match with either orientation (`{P,N}` of d mapped onto `{P,N}` of d').
  5. Pop a self-symmetric net x: every unpaired device whose `D` and `S` (FET), `C` and `E` (BJT), or both terminals (passive) are self-symmetric nets or rails becomes self-symmetric; its gate/base net is **not** marked (bias rails are global and would pull unrelated devices onto the axis).
  6. Iterate to a fixpoint. Compounds = connected components of devices joined by pairs and self-symmetric membership through shared mated nets. `AxisId` = compound index in canonical order of the compound's smallest pair; `Problem.axis_count = compounds.len()`.
  7. Half labels: a seed pair's first device (canonical order) is half A; a propagated pair takes the half of the net through which it was reached; `net_pairs` are stored A-first.
  8. `set_pairs`: after EXT-15, two matched sets S1, S2 are a pair if `pair_of` maps S1's members bijectively onto S2's (nested symmetry, BAL1-40).
  9. `dir = Vertical` unless the sidecar says `H` (ALIGN `direction`); `kind = Perfect` for compounds containing an Exceptional Voltage set (BAL1-02 recommends perfect for tight-offset input pairs; the rule is **Philis policy**), else `Mirror`. PLC-21 additionally requires MAT-01's `mirror_allowed` before using Mirror.
- Tests (`backend/annotator/tests/corpus.rs`, expectations are exact):
  - `ota5t`: pairs {(XM1,XM2),(XM3,XM4)}, selfs {XM5}, net pairs {(vinp,vinm),(vout1,vout2)}, axes 1 (the XM3/XM4 pair is accepted through the shared gate net `vbias`, `tests.rs:44-45`).
  - `folded`: pairs {(M1,M2),(M3,M4),(M5,M6),(M7,M8),(M9,M10)}, selfs {M0}, net pairs {(vinp,vinn),(x1,x2),(o1,out),(y1,y2)}, axes 1.
  - `gilbert`: pairs {(M1,M2),(M3,M6),(M4,M5)}, selfs {M0}, net pairs {(rfp,rfn),(x1,x2),(outp,outn)}, self nets ⊇ {tail, lop, lon}, axes 1 (AA-02 trace §2.13-7c).
  - `rail2rail`: pairs {(MN1,MN2),(MP1,MP2)}, selfs {MN0,MP0}, axes 1 (the pairs share `vinp`/`vinn`).
  - `latch`: pairs {(MN1,MN2),(MP1,MP2)}, net pairs {(q,qb)}, axes 1.
  - `three_stage`: pairs {(M1,M2),(M4,M5)}, selfs {M3}, axes 1; M6, M7, M8, M9 in no pair.
  - `tests/align_gold.rs::strongarm_matches_align_gold` un-ignored (T1).
- Acceptance: T1, T2 (symmetry part), T4.
- Risks / notes: the shared-net relaxation (step 4) is what accepts loads with one gate net (mirror or common-gate bias) as symmetric; it is bounded by identical signatures and terminal names, and every negative circuit (T3) must stay empty. The "mutual unique best" rule never breaks a tie by device id.

### EXT-15 Matched sets, shared-bias groups, ratio inference and unitization
- Priority: P1. Effort: M. Depends on: EXT-11, EXT-12, EXT-13, EXT-14.
- Why: AA-09 (no ratio inference), AA-24 (unequal mirror pairs mirrored), AA-01 (multi-output mirrors fragment); Hastings MOS rule 1 and eq. 13.49: ratios from identical unit sections, series for the smaller device and parallel for the larger (hastings.txt L40840–40864, PDF 688–689; H13-22, H12-28); resistor rule 5 identical segments (L25218–25224, PDF 419); BJT ratios from identical emitters, >8–10 units impractical (L30173–30178, PDF 506; H09-03); Lampaert: a matching group with another ratio is "built of equal unit devices, according to the ratio" (lampaert.txt L3337–3350, PDF 84); Drennan Table III: multiplicity belongs in the output (drennan_mismatch.txt L286–308, PDF 6; MM-20).
- Current: `constraints::assemble` per block and (kind, W, L) class (`constraints.rs:27-63`), `target_ratio = dev_nf` (`constraints.rs:52`); cellgen's `parallel_groups`/`bjt_groups`/`dac_banks` outside the annotator (`cellgen.rs:486-549`).
- Change: new `backend/annotator/src/sets.rs`:
  ```rust
  /// Devices of one kind, model and L sharing gate (FET) or base (BJT) net and source/emitter net,
  /// with at least one diode-connected member or a gate/base net that is not Signal.
  pub fn shared_bias_groups(hg: &BipartiteHypergraph, drawn: &[Drawn], classes: &[NetClassification]) -> Vec<Vec<DeviceId>>;
  /// Filled by the library next to `ProcessNumbers` (`frontend/library/src/lib.rs:492-515`); sky130 values:
  /// grid_nm = `Process::grid()` = 5 (GPurify `pdks/sky130.deck:40`, `grid 5nm`); min_w_nm = cell `min_finger_width` 420
  /// (`pdks/sky130.json:6`); max_w_nm = cell `max_finger_width` 10000 (`pdks/sky130.json:40`); min_l_nm = `Pdk::min_width(poly)`
  /// = 150 (`sky130.deck:274`, poly.1a); res_min_segment_nm = cell `res_min_segment` 10000. A missing key → `unitize` returns
  /// `Err(Diagnostic "unit_deck_incomplete")` and the set keeps `unit: None`.
  pub struct UnitDeck { pub grid_nm: i64, pub min_w_nm: i64, pub max_w_nm: i64, pub min_l_nm: i64, pub res_min_segment_nm: i64 }
  pub fn unitize(members: &[DeviceId], drawn: &[Drawn], kind: DeviceKind, class: MatchClass, deck: &UnitDeck)
      -> Result<(UnitGeom, Vec<(u16 /*parallel*/, u16 /*series*/)>), Diagnostic>;
  pub fn matched_sets(tree: &[GroupNode], reqs: &[Req], compounds: &[Compound], shared: &[Vec<DeviceId>],
                      passive: &[Vec<DeviceId>], drawn: &[Drawn], deck: &UnitDeck) -> Vec<MatchSpec>;
  ```
  - Sets = connected components of `MatchSym ∪ MatchBlock` edges restricted to devices of one kind and model (a mixed component is split by kind; the split is a `Diagnostic`). A shared-bias group is one set with `reference` = its diode-connected member, or none when the bias is external (the three-stage op-amp's M3/M7/M9).
  - `unitize`, FET (T_i = w_finger_i · fingers_i):
    1. All L equal: g = gcd(T_i). W_u = the largest divisor d of g with d a multiple of `grid_nm`, `W_floor ≤ d ≤ max_w_nm`, where `W_floor = max(min_w_nm, 1000 nm if class ≥ Moderate)` (Hastings MOS rule 11: avoid submicron dimensions in moderately and exceptionally matched transistors, hastings.txt L42525–42531, PDF 714; H13-23. Rule 5, L42473–42482, is the pocket-implant rule and does not set this floor). parallel_i = T_i / W_u, series_i = 1.
    2. L differ: L_u = gcd(L_i) with `L_u ≥ min_l_nm` (and ≥ 1000 nm if class ≥ Moderate); series_i = L_i / L_u; W_u from step 1 on the T_i. Exceptional sets with any series_i > 1 get `Diagnostic { kind: "series_units_exceptional" }` (series units carry inherent mismatch, H13-22).
    3. Otherwise `Err(Diagnostic { kind: "non_integer_ratio" })`, `unit: None`; CELL/MAT own the non-integer constructions (H08-43/44, CC-17).
    Resistor: equal W and model required (else `Diagnostic "resistor_widths_differ"`, Hastings R2, H08-09); segment L_u = gcd(L_i) if ≥ `res_min_segment_nm` (sky130 `res_min_segment` 10000); series_i = L_i / L_u; parallel from `m`. Capacitor: equal W, L and model → units = `m` (today's `dac_banks` rule); otherwise the area ratio, integer multiples of the smallest area with equal W → units, else `non_integer_ratio`. BJT: equal emitter W, L, model → units = `m`; units > 16 on any member → `Diagnostic "bjt_ratio_large"` (H09-04: 6:1–16:1 optimum).
  - `target_ratio = units / gcd(units)`; `Unitization` is emitted per set (replacing `constraints.rs:27-63`) with `dummy_required = class ≥ Moderate` (Hastings MOS rule 12, L42532–42553, PDF 714–715: minimal need not include dummies, moderate generally, exceptional always), `route_matching_required = kind != Ratio || class ≥ Moderate`, `class`, `series`, `style` (EXT-16/20). Parallel identical-terminal MOS (today `cellgen.rs:530-549`) are emitted as a Unitization with `route_matching_required = false`, no `MatchedSet`.
  - Delete `cellgen.rs:486-549` loops and `bjt_groups`, `parallel_groups`, `dac_banks` (`cellgen.rs:717-786`) once EXT-19 covers banks (coordinated with CELL/FLOW; the singleton fallback `cellgen.rs:550-577` stays).
- Tests (`sets::tests`, exact):
  - `mirror_ratio_by_width`: widths 2u, 4u, 8u, L 1u → W_u 2000, parallel [1,2,4], target_ratio [1,2,4].
  - `three_stage_bias_group`: M3 8u, M7 6u, M9 20u, L 0.5u → one set, reference `None`, W_u 2000, parallel [4,3,10].
  - `hastings_unit_example`: 100/0.5 and 2×(100/0.5) with `max_w_nm` 10000 → W_u 10000, parallel [10,20] (the book's 4(25/0.5) & 8(25/0.5) uses 25 µm units; the deck cap selects 10 µm).
  - `series_parallel_20_to_1`: A `w=10u l=4u` (1 finger), B `w=50u l=1u nf=5` (FLOW-01 SPICE convention: W_total 50 µm over 5 fingers of 10 µm) → L_u 1000, W_u 10000, series [4,1], parallel [1,5]: the book's 20:1 as 4 units in series against 5 in parallel (Hastings Fig. 13.41, L40850–40864).
  - `resistor_divider`: `rdiv` → set {RA:1, RB:4}, kind Ratio, unit l 10000.
  - `mirror6` corpus: one set of 6 members, reference MR, parallel [1,1,2,1,4,1].
- Acceptance: T2 for `mirror6`, `three_stage`, `rdiv`, `bgr_core` (units [1,8]).
- Risks / notes: the unit width feeds CELL; CELL must accept `series` (a CELL item). Until then CELL draws `parallel` only and the series data is carried unused.

### EXT-16 Precision class and match kind per matched set
- Priority: P1. Effort: M. Depends on: EXT-15; MAT-07 (tables; the consumer edit also needs MAT-07 steps 3 and 6).
- Why: AA-10; H13-42, H08-02, H09-01; AC-22; Hastings §13.3 classes minimal ±10 mV/±10 %, moderate ±3 mV/±3 %, exceptional ±1 mV/±1 % at six sigma (hastings.txt L42327–42350, PDF 712); §10.3 bipolar ±2 mV/±8 %, ±0.5 mV/±2 %, ±0.1 mV/±0.5 % (L31244–31275, PDF 524); §8.3 R/C ±1–5 %, ±0.1–1 %, ±0.01–0.1 % (L25121–25133, PDF 418); voltage- vs current-matched devices (L40643–40645, PDF 686).
- Current: one η for every pair (`emit.rs:46-52`); MatchingPair in the ΔV_T domain for mirrors too (`emit.rs:158-167`); `environment` uses `_moderate` keys for all pairs (`frontend/library/src/lib.rs:871-872`).
- Change: new `backend/annotator/src/class.rs` (inference only; the limit tables are MAT-07's `analog::matching::class::limit(f, k, c) -> ClassLimit`, `ClassLimit::{Mv(f32), Pct(f32)}`):
  ```rust
  pub fn kind_of(set: &MatchSpec, leaf_kinds: &[BlockKind], dk: DeviceKind) -> MatchKind;
  pub fn class_of(set: &MatchSpec, ctx: &mut ClassCtx) -> (MatchClass, ClassSource);
  pub struct ClassCtx<'a> { pub user: Option<MatchClass>, pub spec_6sigma: Option<f32>,
                            pub role: SetRole, pub family: Family, pub diags: &'a mut Vec<Diagnostic> }
  pub enum SetRole { InputPair, LoadOfPair, BiasMirror, BandgapCore, DacBank, FeedbackRatio, Other }
  ```
  - `kind_of`: DiffPair or cross-coupled members → Voltage; mirror, load, cascode pair, shared bias, current-steering → Current; BJT ratioed pair or quad → Voltage (ΔV_BE); resistor and capacitor sets → Ratio.
  - `class_of`, first rule that applies:
    1. `User` (sidecar, EXT-26).
    2. `Spec`: with a 6σ target X in the unit of `limit(family, kind, _)` (from `offset_sigma_mv`·6 today, from 6·`allowance` after EXT-21), with `lim(c)` the number inside `limit(family, kind, c)`: class = Minimal if X ≥ lim(Minimal), Moderate if lim(Moderate) ≤ X < lim(Minimal), Exceptional if X < lim(Moderate); X < lim(Exceptional) adds `Diagnostic "beyond_exceptional_trim"` (Hastings: exceptional needs trim, L42343–42347, PDF 712).
    3. `Role` defaults (Hastings role guidance): InputPair → Moderate (general-purpose op-amp and comparator inputs, H13-42); LoadOfPair → Moderate (H13-39 default); BiasMirror → Minimal (bias mirrors, H13-42); BandgapCore → Moderate (H09-01); DacBank → Exceptional (H08-02: converters); FeedbackRatio → Moderate (H08-02); Other → Minimal.
  - `ArrayStyle` from class (Hastings rule 8, L42499–42503, PDF 714): Minimal → `Adjacent`; Moderate → `Interdigitated` for Current sets sharing a source, `CommonCentroid1d` otherwise; Exceptional → `CommonCentroid2d`. CELL may override by metric (CC-40: small devices can do better clustered; that choice is CELL's/MAT's).
  - `SetRole` assignment: InputPair = a set seeded by a declared DiffPair couple; LoadOfPair = a Load/CascodePair couple in the same compound as an InputPair; BiasMirror = a CurrentMirror couple or a shared-bias group; BandgapCore / DacBank / FeedbackRatio = the role `passive.rs` returns (EXT-19); Other otherwise.
  - Consumer edit (`frontend/library/src/lib.rs:871-872`, `environment`; lands after MAT-07 steps 3 and 6, which add `Process::tier` and switch these two lines to `mos_env(Moderate, …)`): pass the class of the `MatchSpec` that contains the leaf's two devices instead of `Moderate` (`Moderate` when no spec contains them). sky130 then reads `wpe_clearance_nm[class]` = 2000 / 3000 / 5000 (`pdks/sky130.json:59-63`; H13-53: MIN ≥ 2 µm, MOD ≥ 3 µm, EXC ≥ 5–10 µm or ≥ 2× the well junction depth, the deck's 5000 being the lower EXC bound).
- Tests (`class::tests`): `spec_maps_to_class` (Mos Voltage, MAT-07 limits 10/3/1 mV: X = 12 → Minimal, 5 → Moderate, 2 → Exceptional, 0.5 → Exceptional + `beyond_exceptional_trim`); `user_wins_over_spec_and_role`; corpus: `ota5t` DP Moderate Voltage (Role), load Moderate Current (Role); `three_stage` bias group Minimal Current; `dac4` bank Exceptional Ratio; `bgr_core` Moderate Voltage. The limit numbers themselves are tested by MAT-07 (`mos_limits_match_hastings`, `bjt_limits_match_hastings`).
- Acceptance: T10 (every set has a class and kind).
- Risks / notes: role defaults are the weakest evidence; the metadata report prints `ClassSource` so a user sees which classes are assumed.

### EXT-17 Evidence input: operating point and sensitivities
- Priority: P1. Effort: M. Depends on: EXT-12. Producers (not blockers; every field is optional): PERF-09 (`OpPoint.vgs_v`/`vds_v`/`vbs_v`; `vth`, `gmb`, `gds` are asked of PERF and stay `None` until produced), FLOW-07 (ports), PERF-12 (`perf::to_evidence` fills `Sensitivities`).
- Why: AA-12; Lampaert: placement input includes sensitivities and the operating point (lampaert.txt L3443–3471, PDF 86–87); survey: importance "determined by simulation" (balasa_graeb_survey.txt L5426–5431, 5608–5610); H05-17 (net voltage envelope); MM-07/08/09 (bias-dependent mismatch domain); H12-08 (subthreshold tagging).
- Current: `annotate(netlist, cfg)` (`lib.rs:75`); the op point reaches only `ir::budgets` (`frontend/library/src/lib.rs:283-293`); `OpPoint` has power, Id, headroom, gm (`frontend/library/src/oppoint.rs:13-28`).
- Change: new `backend/annotator/src/evidence.rs`:
  ```rust
  #[derive(Default)] pub struct Evidence {
      pub op: Option<OpFacts>, pub sens: Option<Sensitivities>,
      /// Ports of the top subcircuit (FLOW parser); empty = unknown.
      pub ports: Vec<NetId>,
      /// Nets driven by PULSE/PWL/SIN sources in the testbench.
      pub switching_nets: Vec<NetId>,
      /// Nets held by DC sources, with their voltage, mV.
      pub dc_sources: Vec<(NetId, f64)>,
      pub probe_bias: bool,
  }
  pub struct OpFacts { pub dev: Vec<Option<DeviceOp>>, pub net_mv: Vec<Option<f64>> }
  pub struct DeviceOp { pub id_ua: f64, pub vds_mv: f64, pub vdsat_mv: f64, pub gm_us: f64, pub power_uw: f64,
                        pub vgs_mv: Option<f64>, pub vth_mv: Option<f64>, pub gmb_us: Option<f64>, pub gds_us: Option<f64> }
  pub struct Sensitivities { pub specs: Vec<SpecSens> }
  pub struct SpecSens { pub metric: String, pub f0: f64, pub lo: Option<f64>, pub hi: Option<f64>,
      /// Metric range over the process corners (Lampaert eqs. 2.2–2.7); None = nominal only.
      pub proc: Option<(f64, f64)>, pub sigma_f: Option<f64>,
      pub d_c: Vec<(NetId, f64)> /*per aF*/, pub d_r: Vec<(NetId, f64)> /*per Ω*/,
      pub d_vt: Vec<(DeviceId, f64)> /*per mV*/, pub d_t: Vec<(DeviceId, f64)> /*per K; PERF-12 leaves it empty*/,
      /// Coupling sensitivity per aF between two nets (PERF-12 step 3 change request; LAMP-39 ½·S_Cki).
      pub d_cc: Vec<(NetId, NetId, f64)> }
  #[must_use] pub fn annotate_with(netlist: &Netlist, cfg: &AnnotationConfig, ev: &Evidence) -> Problem;
  pub fn device_facts(nl: &Netlist, op: &OpFacts, classes: &[NetClassification]) -> Vec<DeviceFacts>;
  ```
  `annotate(nl, cfg)` becomes `annotate_with(nl, cfg, &Evidence::default())`.
  Region rules (FET; `headroom = |vds| − |vdsat|`):
  1. `Off`: |Id| < 1e-3 · max |Id| over the circuit (**Philis policy**).
  2. `Subthreshold`: vgs and vth known and |vgs| − |vth| < 0; else `gm·1V/|Id| ≥ 20` (square law gm/Id = 2/V_ov gives V_ov ≤ 100 mV, the Hastings rule-4 floor L42464–42472; **derived** proxy).
  3. `Triode`: headroom < 0 (oppoint.rs:19-21 defines negative headroom as triode).
  4. `Saturation` otherwise.
  Roles: `Switch` = Triode and gate net class ∈ {Clock, DigitalSwitching, DigitalStatic}; `Diode` = D == G; `CurrentSource` = Saturation and gate net class ∈ {Bias, Reference} or in a shared-bias group; `Cascode` = Saturation, source on a non-rail internal net and gate class Bias; `Amplifier` = Saturation, gate class ∈ {Signal, Sensitive}; `Load` = member of a Load set. (No `hot` flag: power per device already reaches placement as `cells.power`, PLC-06.)
  Calling protocol (FLOW): `p0 = annotate_with(nl, cfg, &ev_without_sens)` → PERF computes sensitivities over `p0` nets and devices → `p = annotate_with(nl, cfg, &ev_with_sens)`. `annotate_with` is pure, so FLOW hoists both calls out of the per-start closure (AF-22).
- Tests: `evidence::tests::regions_from_headroom_and_gm` (four synthetic DeviceOps → Off, Subthreshold, Triode, Saturation); `evidence::tests::switch_needs_digital_gate` (triode device with a Signal gate is not a Switch); `tests/corpus.rs::annotate_without_evidence_is_unchanged` (`annotate_with(.., &Evidence::default())` equals `annotate` on every circuit).
- Acceptance: with FLOW's `OpPoint` on `ota5t` (probe bench), XM1–XM5 are Saturation and XM5 is CurrentSource.
- Risks / notes: every evidence-derived fact carries `EvidenceLevel::OpPoint`; with `probe_bias` true the metadata marks them assumed (AF-04's certificate rule is FLOW's).

### EXT-18 Net classes with evidence levels
- Priority: P1. Effort: M. Depends on: EXT-02, EXT-03, EXT-17.
- Why: AA-11, AA-17, AA-25; ILAC classes sensitive / noisy / non-critical / supply (lampaert.txt L1060–1066, PDF 29; balasa_graeb_survey.txt L9068–9074, L9196–9200, PDF 180, 183; LAMP-48, BAL2-11); Hastings's eight noise-sensitive categories (hastings.txt L48604–48616, PDF 819; H15-28); Table 15.5 net classes (L48445–48488, PDF 817; H15-27); Table 14.1 (L44192–44228, PDF 745; H14-35); name suffixes `_n`, `_s` (L48618–48621, PDF 819) and `_hv` (L48711–48713, PDF 820) (H15-06); slew-based aggressors (H14-08).
- Current: 6 classes (`kernel/analog/src/metadata.rs:17-26`), gates-only nets Sensitive (`classify.rs:127`), clocks by name.
- Change:
  1. `NetClass` gains `Bias`, `Reference`, `DigitalStatic`, `DigitalSwitching`, `Noisy` (`kernel/analog/src/metadata.rs:17-26`). The only exhaustive match is `classify.rs:28-32` (budgets). Class-specific readers whose meaning changes: `extract.rs:110-127` (margins, spacings; `_` arms), `extract.rs:90-94` (shields: a Clock net present, Sensitive victims), `emit.rs:271` (aggressor = device on a Clock net), `frontend/library/src/lib.rs:211` (sensitivity nets), `frontend/library/src/lib.rs:419` (fill keep-outs). Readers of Supply/Ground only, unaffected: `ir.rs:49`, `frontend/library/src/elaborate.rs:306`, `frontend/library/src/lib.rs:307, 634, 1135`. Budgets: Bias and Reference as Sensitive (1×/1×); Clock, DigitalSwitching, Noisy as today's Clock (2× wire, **no** coupling budget: aggressors are not victims); DigitalStatic 2× wire, no coupling budget. Margins/spacings: Bias, Reference = Sensitive (35 %, 8); DigitalSwitching, Noisy = Clock (30 %, 7); DigitalStatic = Signal (20 %, 3).
  2. `NetClassification` stays as is; the evidence lives in `Intent.nets[n].evidence`.
  3. Rules, first applicable wins (evidence order User > Port > Name > Testbench > Structure > OpPoint > Default):
     - Supply / Ground: sidecar `PowerPorts`/`GroundPorts`; `rail_of` (EXT-02); a net held by a DC source at the most positive / most negative voltage (`dc_sources`); bulk inference (EXT-02).
     - Clock: sidecar `ClockPorts`; name rule (`netrole.rs:50-55`); `switching_nets` from the testbench.
     - DigitalSwitching: the output net of a `cmos_inverter`/`nand_gate`/`nor_gate` match whose input net is Clock or DigitalSwitching (one level of propagation per iteration, to a fixpoint); a net named `*_clk*` already is Clock.
     - DigitalStatic: the input or output net of a logic-gate match (inverter, nand, nor, transmission gate) not already switching. `dac4`'s `d0..d3` and `b0..b3` land here (AA-11 trace).
     - Noisy: output net of a `charge_pump_cell` match (H14-08); a name with suffix `_n` (H15-06).
     - Reference: output net of a BandgapCore set (EXT-19) or of `cascoded_reference`; a net that is the bottom-plate reference of a DAC bank (the `N` net of the bank's reference capacitor when it is not a rail).
     - Bias: a net that connects only FET gates plus the D=G of diode-connected devices (today's gates-only rule, `classify.rs:127`); the gate net of every CurrentSource or Cascode device.
     - Sensitive: gate nets of Voltage-kind set members (input pairs); the top-plate net of a DAC bank (CC-14, DACP L142–173); a name with suffix `_s`; with an op point, a Signal net whose node impedance `z_ohm` ≥ 100 kΩ (**Philis policy**; Hastings's 1 V example, eq. 15.23, 10 fF into 100 kΩ at 1 GV/s, L48574–48595, uses 100 kΩ, per the H15-28 recipe and H15-29).
     - Substrate: bulk-only (unchanged).
     - Signal otherwise.
  4. `Intent.nets[n].z_ohm` = `1 / (Σ gds of devices with D on n + Σ gm of diode-connected devices on n)` when `gds` is known (SUB-12 victim impedance).
- Tests (`classify::tests` and corpus): `dac4`: d0..d3 DigitalStatic, b0..b3 DigitalStatic, top Sensitive, VDD Supply, VSS Ground; `three_stage`: vbias Bias, vin_p/vin_n Sensitive, n3 Signal; `folded`: vbp1, vbp2, vbn, vbn2 Bias; `bjt_mirror`: in, outn, outp Signal; a sidecar `ClockPorts p1 p2` makes `sc_switches` gates Clock and the switches `Switch` with an op point.
- Acceptance: AA-11 and AA-17 closed on the corpus.
- Risks / notes: adding enum variants is a compile-time change for every exhaustive match; the list in item 1 covers every non-test `NetClass::` use in the current tree (grep `NetClass::` over `kernel`, `backend`, `frontend`, `benchmarks`, `examples`).

### EXT-19 Passive, bipolar and diode recognition (and moving cellgen's recognizers)
- Priority: P1. Effort: L. Depends on: EXT-03, EXT-11, EXT-15.
- Why: AA-04; H08-61 (matched passive sets from topology; hastings.txt L23578–23582, PDF 394: matched groups come from the designer's notes), H08-56 (same material, R1 L25164–25181, PDF 418); H09-05 (ratioed pair, quad, Brokaw), H09-26 (emitter degeneration moves the matching burden to resistors: ℜ_d/ℜ = V_T/(V_T+V_d), eq. 10.10, L30353–30366, PDF 508–509); CC-16 (split DAC, C_A = (C_T^LSB/C_T^MSB)·C_u, DACP eq. 1, cc_dac_constructive.txt L60–76, PDF 1); MM-35 (resistor matching).
- Current: FET-only slots (`pattern.rs:94`); cellgen's side recognizers (`cellgen.rs:486-549, 717-786`).
- Change:
  1. DSL (`pattern.rs`): `SlotKind::Kind(DeviceKind)` and `SlotKind::SameKindAs(u8)`; `slot_ok` checks FET-ness only for `AnyFet`/`SameTypeAs`/`ComplementOf`; `is_diode` generalizes to FET D==G and BJT C==B.
  2. Bipolar patterns (catalog): `bjt_ratioed_pair` (two `SameKindAs` BJTs, `eq(0,"B",1,"B")`, `ne(0,"E",1,"E")`, equal emitter W/L and model; ratio from `m`; base may be a rail, as in `bgr_core`), `bjt_diff_pair` (`eq_sig(0,"E",1,"E")`, `ne(0,"B",1,"B")`, `ne(0,"C",1,"C")`, ExactAs; pairs `(0,1,DiffPair)`), `bjt_mirror` (diode-connected reference, shared B and E; pairs `(0,1,CurrentMirror)`).
  3. Procedural rules, new `backend/annotator/src/passive.rs` (two-terminal devices have no fixed orientation, so the DSL is not used):
     ```rust
     pub fn resistor_sets(nl: &Netlist, drawn: &[Drawn], classes: &[NetClassification], compounds: &[Compound]) -> Vec<(Vec<DeviceId>, SetRole)>;
     pub fn capacitor_sets(nl: &Netlist, drawn: &[Drawn], classes: &[NetClassification]) -> Vec<(Vec<DeviceId>, SetRole, Option<DeviceId> /*bridge*/)>;
     pub fn bandgap_cores(nl: &Netlist, bjt_pairs: &[PatternMatch]) -> Vec<(Vec<DeviceId> /*R set*/, SetRole)>;
     pub fn degeneration(nl: &Netlist, pairs: &[(DeviceId, DeviceId)]) -> Vec<Vec<DeviceId>>;
     ```
     - `resistor_sets` (same model and W; different model → not matched, H08-56): (a) dividers/ladders: maximal chains of resistors whose internal nodes connect only resistors and FET gates, between two non-internal nets → one set, role FeedbackRatio; (b) resistors paired by symmetry analysis (EXT-14 pairs passives) → one set per compound pair, role Other; (c) resistors whose one terminal sits on a BJT ratioed pair's emitter path (Brokaw R1/R2, via `bandgap_cores`) → one set, role BandgapCore.
     - `capacitor_sets`: capacitors sharing one top-plate net `P` with equal W, L and model → one set (the `dac_banks` rule generalized from binary counts to any integer counts); the reference = the one-unit capacitor whose other plate is a rail (today's rule, `cellgen.rs:778-782`); role DacBank when the counts are `[1,1,2,…,2^(N-1)]` (`cells::cap_array::bits`), FeedbackRatio otherwise. Split DAC: two such banks on nets `tl`, `tm` joined by exactly one capacitor `CA` between `tl` and `tm` → one set over both banks with `bridge = CA`; check `C_A = (C_T^LSB / C_T^MSB)·C_u` within 1 % (**Philis tolerance**) using unit areas, else `Diagnostic "bridge_cap_value"`.
     - `degeneration`: for a DiffPair or BJT pair whose sources/emitters reach a common node through one resistor each → the two resistors form a Ratio set whose target ratio is the **inverse** of the pair's unit ratio (Hastings 10.2.2: resistors sized for equal drop, 1X 4 kΩ / 2X 2 kΩ / 3X 1.33 kΩ, L30329–30338, PDF 508).
     - Diodes: same model and area on a shared anode or cathode net → a set (H09-43).
     - Every passive set becomes a `MatchSpec` with `origin = Origin::PassiveSet { rule }`, `rule` ∈ `"divider"`, `"symmetric_r"`, `"bandgap_r"`, `"dac_bank"`, `"cap_ratio"`, `"split_dac"`, `"degeneration"`, `"diode_set"` (the identifier consumers match on, e.g. RTE-20).
  4. Move and delete: the bank, BJT and parallel groupings now come from the annotator's Unitizations (EXT-15); delete `cellgen.rs:486-549` and `717-786` (coordinated with CELL/FLOW; the drawn output must be unchanged on `dac4`, `bgr_core`, `pair`, `quad`). If RTE-20 has landed, its `PlateSet` builder reads `cellgen::dac_banks` (RTE-20 step 1): switch it in the same commit to the `MatchSpec`s with `matches!(s.origin, Origin::PassiveSet { rule: "dac_bank" })` (top = the shared `P` net, bits = `(N net, parallel)` of members whose `N` is not a rail) before deleting `dac_banks`.
- Tests (corpus, exact): `bgr_core`: set {XQ1:1, XQ2:8} Voltage Moderate; `brokaw`: sets {Q1:1, Q2:8} Voltage Moderate, {R1:4, R2:1} Ratio Moderate (unit l 20000), {MP1, MP2} Current; `rdiv`: {RA:1, RB:4}; `dac4`: {XC0..XC4} units [1,1,2,4,8], DacBank Exceptional; `splitdac`: one set C0..C4 units [1,1,2,1,2], bridge CA, no `bridge_cap_value` diagnostic (C_T^LSB = 4 units, C_T^MSB = 3 units, CA area 12 µm² = 4/3 of the 9 µm² unit); changing CA to `l=5u` raises the diagnostic; `bjt_mirror`: no set (NPN vs PNP).
- Acceptance: AA-04 closed; T2 on the passive and bipolar circuits; `bench local` unchanged on `dac4`, `bgr_core`, `pair`, `quad` (T9).
- Risks / notes: resistor values come from W/L squares, which assumes one model per set; FLOW's value parsing (AF-02) is not needed for ratios within one model.

### EXT-20 Emission per matched set and compound (rewrite of `emit::placement`)
- Priority: P1. Effort: M. Depends on: EXT-14, EXT-15, EXT-16; MAT-04 (`MatchedSet` rule, `Coeffs`, `Budget`), MAT-05 (`OrientationSet`), MAT-08 (`class_explicit`).
- Why: AA-24 (unequal devices mirrored), AA-23, AR-02/AR-04 (their fix is MAT-04; this item feeds it sets instead of 2-device leaves); H13-45 (rule 8 by class); survey: matching groups get same variants and alignment, replaced by CC when devices consist of subdevices (balasa_graeb_survey.txt L5613–5625, PDF 129).
- Current: per-block loop over 2-device leaves (`emit.rs:138-239`); after MAT-04 step 4 the loop pushes one `MatchedSet { members: [a, b] }` per DiffPair/CurrentMirror/Load leaf and keeps Symmetry, Proximity and DTI emission.
- Change: `emit::placement(intent: &Intent, nl: &Netlist, p: &ProcessNumbers, policy: &Policy) -> Requirements<Layout>` replaces the leaf loop:
  1. Per compound: one `SymmetryGroup` (hard + cost, as today `emit.rs:212-215`) on `AxisId(compound.axis)` with a `Symmetry` for every pair in `compound.pairs` whose members are in one `MatchSpec` with equal `Member.parallel`/`series` (equal couples only, AA-24) and a self entry for every `compound.selfs` device.
  2. Per `MatchSpec` s: one MAT-04 `MatchedSet` pushed into budget and cost with `members` = s's devices with `s.reference` (if any) moved to slot 0; `kind = s.kind`; `class = s.class`, `class_explicit = s.class_source != ClassSource::Role` (MAT-07/08 fields); `coeffs` by polarity/family as MAT-04 step 4 / MAT-10 build them; `budget` = `Budget::Sigma1Mv((a² + σ_rand²).sqrt())` when `s.allowance = Some(a)` and kind is Voltage (MAT-04's quadrature form then returns exactly `a` as the allowance), else MAT-08's rule (`offset_sigma_mv`, class, or `Eta(0.3)`). One batch per set, never pooled across sets (T5).
  3. Per `MatchSpec` with ≥ 2 FET or BJT members: MAT-05's `OrientationSet{Axis}` (hard) and, by class, `OrientationSet{Phi}` (MAT-07 step 5 rule); resistor sets get `OrientationSet{PhiZero}` (MAT-12 step 3).
  4. Proximity (budget, `policy.proximity_nm`): Minimal sets → reference (or first member) to every other member; declared `selfs` and `prox` couples of every match (EXT-05) → P_B. Moderate/Exceptional sets get no Proximity: the `ArrayStyle` request (CELL) and the `MatchedSet` centroid term do that work.
  5. DTI bands per `compound.pairs` couple as today (`emit.rs:186-195`).
  Delete the per-block loop (`emit.rs:138-239`) and `PROXIMITY_NM` (moved to `Policy`, EXT-10). The rotation/shape locks come from the rules' `matched_pairs` hook (PLC-03), so no orientation table is emitted.
- Tests: `emit::tests::one_rule_per_set` (T5): for every corpus circuit, the number of `MatchedSet` batches equals `intent.sets.len()` and each batch's member set equals its spec's; `ota5t` → 2 batches (DP {XM1, XM2}, load {XM3, XM4}). `emit::tests::ratioed_mirror_not_mirrored`: `mirror6` (Minimal) → 0 `Symmetry`, 1 `MatchedSet` with 6 members and MR in slot 0 (5 pairs), 5 `Proximity` (MR to each output); with a sidecar `Match` Moderate on the same devices → still 0 `Symmetry`, 0 `Proximity`, the `MatchedSet` has `class_explicit == true`. `emit::tests::allocated_allowance_round_trips`: `allowance = 0.4` mV, σ_rand 1.2 mV → the batch's ledger allowance is 0.4 ± 1e-4.
- Acceptance: T5; AA-23 and AA-24 closed; `bench local` DRC 0 / LVS MATCH (T9).
- Risks / notes: fewer, set-sized rules change dp's prices; the `Budget` mapping of Current/Ratio allowances waits for MAT-09/10's % ledgers (until then those sets use MAT-08's default).

### EXT-21 Sensitivity-driven allowance allocation
- Priority: P1. Effort: M. Depends on: EXT-17, EXT-20; PERF-12 (`d_vt`), PERF-13 (`sigma_f`).
- Why: LAMP-09 (one offset budget over all pairs weighted by sensitivity; lampaert.txt eqs. 4.25–4.27, L4718–4765, PDF 112–113; comparator offset 6.9 → 3.7 mV, Table 4.1, L5006–5010, PDF 119; per-pair distances and sensitivities, §4.12.1 and Table 4.2, L5027–5082, PDF 119–120); GRAEB-38 (tolerance assignment, graeb_centering.txt L3774–3805, PDF 99–100); GRAEB-06 (σ_f² = ∇fᵀC∇f, eq. 175, L4205–4211, PDF 111); GRAEB-16 (reserve β·σ_f, eqs. 176–179, L4213–4224); survey dynamic importance by simulation (L5426–5431).
- Current: flat η = 0.3 or from one global `offset_sigma_mv` (`emit.rs:46-52, 83-91`); matching only from patterns.
- Change: new `backend/annotator/src/allocate.rs`:
  ```rust
  /// Per spec j: margin M_j = headroom_j − β·σ_f,j; per set k: S_jk = (|s_j,a| + |s_j,b|)/2 over the set's
  /// half-A / half-B (or reference / output) sums; K_j = number of sets with S_jk > 0.
  /// δ_k = min over {j : S_jk > 0} of M_j / (K_j·S_jk), capped at max_eta·σ_k; max_eta·σ_k when no spec touches k.
  pub fn allocate(sets: &mut [MatchSpec], sigma_rand_mv: &[f32], sens: &Sensitivities, beta: f64, max_eta: f32) -> Vec<Diagnostic>;
  ```
  - `headroom_j`: `f0 − lo` for a floor and `hi − f0` for a ceiling, using `proc` when present (LAMP-01, lampaert.txt L1305–1418, PDF 35–38); a two-sided spec gives two rows and the tighter wins.
  - `β = policy.beta_target` (3.0, the book's three-sigma design, graeb_centering.txt L4266–4281, PDF 112).
  - `σ_k` = the set's random σ (A_VT/√(WL·units) per side, pair form). Each spec's margin is split equally in consumption among the sets it touches, so an insensitive set gets a large allowance and a sensitive one a small allowance, which is Lampaert's observation that distances can be traded where sensitivity is low (Table 4.2). The rule satisfies `Σ_k S_jk·δ_k ≤ Σ_k M_j/K_j = M_j` for every spec, because each δ_k is at most its term for every spec that touches it (**derived**). `max_eta = 3.0` (**Philis policy**) bounds the allowance of sets no spec is sensitive to.
  - `M_j ≤ 0` → `δ_k = 0` for the sets that spec touches and `Diagnostic "spec_infeasible_at_schematic"` (LAMP-49).
  - `weight_k = max_j (S_jk·σ_k)² / σ_f,j²` (share of variance).
  - Structural sets whose weight < 0.01 get class Minimal with `ClassSource::Spec` (**Philis policy**: a set that explains < 1 % of every spec's variance is not worth moderate-class area).
  After allocation, `class_of` re-runs with `spec_6sigma = 6·δ_k` (EXT-16 rule 2).
- Tests (`allocate::tests`): `allocation_respects_every_spec` (2 specs, 3 sets, random S and σ: `Σ_k S_jk δ_k ≤ M_j + 1e-9` for both); `lampaert_table_4_2_ordering` (one spec, four sets with S = 12, 2.9, 2.9, 0.2 taken from the first four rows of the sensitivity column of lampaert Table 4.2, L5068–5082, PDF 120; that column is the offset's sensitivity to pair distance, µV/µm per LAMP-09, so here it serves only as relative weights for S_jk; equal σ and a cap large enough not to bind: δ is ordered 0.2-set > 2.9-sets > 12-set and Σ S·δ = M exactly); `negative_margin_is_diagnosed` (M_j = −1 → every set it touches gets δ = 0 and one `spec_infeasible_at_schematic`).
- Acceptance: with PERF sensitivities on `ota5t`, the DP set's allowance is below the load set's when the DP's offset sensitivity is larger.
- Risks / notes: linear systematic sum (Lampaert's form) is conservative; MAT may replace it with a quadrature row.

### EXT-22 (deferred — see "Cut or deferred items")
The abutment table's only reader is dp's rotation lock (`frontend/library/src/lib.rs:579`, `backend/dp/src/lib.rs:555-560`), which PLC-03 step 4 and PLC-06 step 2 delete; `Intent.wells` had no consuming item in PLC, CELL or REL (PLC-07 derives well sharing from cell edge profiles, REL-16 from its own flags).

### EXT-23 Substrate aggressor and victim tags (inputs to REL-07/08/09/16)
- Priority: P1. Effort: S. Depends on: EXT-17, EXT-18, EXT-20. Consumers: REL-07 (`RingInputs.sensitive`), REL-09 step 4 (aggressors), REL-16 (`CellFlags.noisy/sensitive`).
- Why: AA-13, AA-18, AA-33 (aggressor = any device on a clock net, victim = any "sensitive" leaf, strength not modelled); H12-39 (rings where injection is possible, not everywhere); H14-08 (slew-coupled aggressors); SUB-12 (victim weight by impedance/criticality); REL-07/09 take "sensitive" and "noisy class" from EXT. Ring types, injector classification and isolation distance are REL-07/08/09 and are not repeated here.
- Current: `sensitive: Vec<bool>` from leaf kinds (`lib.rs:121-128`) and "device on a Clock net" (`emit.rs:271-279`) are the only substrate inputs.
- Change: new `backend/annotator/src/substrate.rs`:
  ```rust
  pub fn tag(nl: &Netlist, classes: &[NetClassification], sets: &[MatchSpec]) -> (Vec<Aggressor>, Vec<Victim>);
  /// `victim[d]`: what REL-07's `RingInputs.sensitive` and REL-16's `CellFlags.sensitive` read.
  pub fn victim_mask(n_devices: usize, v: &[Victim]) -> Vec<bool>;
  ```
  1. `Switching`: a device with any terminal on a Clock, DigitalSwitching or Noisy net (EXT-18; today's rule extended from Clock only, `emit.rs:271`).
  2. `Capacitive`: a device with a terminal on a net that a capacitor couples to a Clock, DigitalSwitching or Noisy net (Hastings Fig. 5.28 case, L14857–14862).
  3. Victims: members of sets with class ≥ Moderate, and devices whose gate is a Reference or Bias net (EXT-18). `weight` = `MatchSpec.weight` when EXT-21 ran, else 1 / 3 / 10 for Minimal / Moderate / Exceptional (**Philis policy**, a rank only).
  4. `annotate` replaces the `sensitive` vector passed to `emit::isolation` by `victim_mask` (`lib.rs:137`), and REL-09's aggressor set by `Intent.aggressors` devices; REL-09 then skips (aggressor, victim) pairs inside one block, and this item additionally skips pairs inside one compound or one set (the StrongARM tail `mn0` is in the input pair's compound).
- Tests (`substrate::tests`): `strongarm` with `ClockPorts clk`: `mp7, mp8, mp9, mp10, mn0` tagged `Switching`; victims = the members of Moderate sets (mn1, mn2, mn3, mn4, mp5, mp6 under the Role defaults of EXT-16); `ota5t` without clocks: 0 aggressors, victims XM1–XM4 (Moderate sets) and XM5 (gate on the Bias net `vbn`); `dac4`: no inverter device is a victim (logic, no set); a capacitor between `clk` and `x` makes every device on `x` `Capacitive`.
- Acceptance: AA-33 closed at the tag level; with REL-07 on `ota5t` no ring is drawn (no aggressor), matching REL-07's acceptance.
- Risks / notes: none beyond REL's.

### EXT-24 Routing constraints from structure
- Priority: P1. Effort: M. Depends on: EXT-14, EXT-18, EXT-20, EXT-23 (aggressors for `quiet_ground`).
- Why: AA-14 (routing pairs must come from recognition), AA-31 (shield reference = first ground), AT-33 (only drains and shared sources matched), AR-07 (shield counted as aggressor); H15-32 (shield tied to the victim's own reference, hastings.txt L48638–48667); H15-33 star nodes (hastings.txt L48509–48538, PDF 817–818), H15-34 Kelvin (L48540–48549, PDF 818), MM-33 (8.3 mV/mm IR drop masquerading as a gradient, long_distance_mismatch.txt L180–204); BAL2-13 sensitive/noisy spacing (balasa_graeb_survey.txt L9104–9112); CC-14 DAC plate roles (cc_dac_constructive.txt L142–173). Consumers: RTE-02, RTE-15, RTE-17, RTE-18, RTE-19, RTE-21.
- Current: `extract.rs:24-106` (after EXT-09: DiffPair drains only); `common_nodes` in the library scans 2-device leaves (`frontend/library/src/lib.rs:779-798`).
- Change (`extract::routing` takes `&Intent`):
  1. `Differential` (budget) for every compound net pair except rails; `CrosstalkExclusion` for (a) every Voltage set's gate net × its members' drain nets (today's semantics) and (b) every (victim ∈ {Sensitive, Reference, Bias}, aggressor ∈ {Clock, DigitalSwitching, Noisy}) net pair, spacing `route_space · max(multiple(victim), multiple(aggressor))`.
  2. Shields for victims (Sensitive, Reference, Bias) only when an aggressor-class net exists; reference = `quiet_ground(classes, net_names, &intent.aggressors)` (new fn in `extract.rs`): a Ground net whose name has an analog root (`avss`, `vssa`, `agnd`, via `rail_of` roots), else the Ground net on which no `Switching` aggressor has a terminal, else the lowest-id Ground net plus `Diagnostic "shared_shield_return"` (SUB-30); no Ground net → no shield, as today (`extract.rs:91`); `Intent.nets[victim].shield_ref = Some(reference)` so RTE excludes it from the victim's coupling sum (AR-07's measurement fix is RTE's).
  3. `Intent.common_nodes`: for every matched set, the net shared by all members on `S` (FET) or `E` (BJT), rails included as today (`frontend/library/src/lib.rs:784` takes any shared source net); sides are half A / half B, or one request per output against the reference for a set with a reference; no ΔR number here — MAT-06 supplies each set's remaining allowance and RTE-17 step 4 turns it into `max_delta_ohm`. The library's `common_nodes` (`lib.rs:763-801`) iterates `intent.common_nodes` and only measures pins (coordinated edit; replaces the leaf scan and the gate-area copy).
  4. `Intent.stars`: a set's shared terminal net that also carries non-member current — structurally, a non-member S/D/E terminal other than the one tail or port feeding the set; with an op point, `Σ|I_nonmembers| > 0.01·Σ|I_members|` (**Philis threshold**) — becomes `StarReq { net, root: the feed, branches: members' terminals }` (H15-33).
  5. `Intent.kelvins`: a resistor whose two terminal nets are both gate nets of a Voltage set (a sense resistor read by an input pair) → `KelvinReq { device, term: P and N, sense: the gate pins }`; also every sidecar `Kelvin` entry (H15-34; the source gives no resistance threshold, so detection is structural only).
  6. DAC plates: a DacBank's top-plate net gets `RcClass::C` and its bottom-plate nets `RcClass::R` (CC-14, DACP §II-A L142–173, PDF 2–3: C_TS and C_TB alter V_OUT and linearity; bottom-plate parasitics do not affect linearity but load V_REF and set the 3-dB frequency; the RC form, f3dB = 1/(2(N+2)ln(2)τ) with the switching-path routing resistance in τ, is DACP §III-C eq. 32, L445–480, PDF 5).
- Tests (`extract::tests` and corpus): `folded`: `Differential` for (vinp,vinn), (x1,x2), (o1,out), (y1,y2); `strongarm`: shields for exactly `vin`, `vip`, `vin_o`, `vip_o` (the gate nets of the three Voltage sets {mn1,mn2}, {mn3,mn4}, {mp5,mp6}, EXT-18) referenced to `vss`, and only when `clk` is Clock; `ota5t`: two `CommonNodeReq`, on `vtail` with sides {XM1} / {XM2} and on `VDD` with sides {XM3} / {XM4} (rails included, item 3); `mirror6`: 5 `CommonNodeReq` on VSS (MR against each output) and 0 `StarReq`; the same netlist plus one unrelated NMOS with its source on VSS: 1 `StarReq` on VSS with 6 branches; constructed sense resistor `RS a b` with `a`, `b` the gates of a DP → 1 `KelvinReq`; `dac4`: top plate `RcClass::C`, the four bit bottom plates `RcClass::R`; `quiet_ground` on nets {`VSS`, `AVSS`} → `AVSS`.
- Acceptance: AT-33 and AA-31 closed in the annotator's output; RTE consumes.
- Risks / notes: a common node on a rail that also carries foreign current is exactly the star case (item 4); RTE-17 step 4 lets a `StarReq` supersede the `CommonNodeReq`s on the same net.

### EXT-25 Parasitic budgets derived from sensitivities
- Priority: P1. Effort: M. Depends on: EXT-17, EXT-18, EXT-24; PERF-06 (lands first in `perf::budget_rows`; this item ports it), PERF-12 (`d_c`, `d_r`, `d_cc`, `proc`), PERF-13 (`sigma_f`).
- Why: LAMP-01 (margins from the spec and the process spread; lampaert.txt eqs. 2.1–2.7, L1305–1418, PDF 35–38); LAMP-02/04 (linear degradation over every parasitic class, eqs. 2.12–2.13, L1601–1620, PDF 42); BAL2-16 (conservative sign split S⁺/S⁻, eqs. 4.5–4.7, balasa_graeb_survey.txt L9257–9360, PDF 184–186; matching on matched-node pairs, L9387–9390); GRAEB-16 (reserve β·σ_f, eqs. 176–179, graeb_centering.txt L4213–4224, PDF 111); SURV-28 (R-sensitive vs RC-sensitive nets; designers' annotation loses to computed trade-offs, todaes22.txt L909–920, PDF 22); AA-25 (fallback budgets against the smallest gate); AF-16 (two-sided specs lose their ceiling).
- Current: `perf::budget_rows` in the library: ground C only, floor row only when both bounds exist, nominal headroom (`frontend/library/src/perf.rs:230-257`, `241-244`); class budgets 1×/2× with the smallest-gate fallback (`classify.rs:26-33, 87, 97`).
- Change: new `backend/annotator/src/budget.rs` (the library calls it instead of `perf::budget_rows`, which is deleted):
  ```rust
  /// Reads `policy.beta_target` (3.0) and three new `Policy` fields: `credit_helpful: bool = false`,
  /// `rc_ref_len_um: f64 = 100.0`, `rc_share: f64 = 0.05`.
  pub fn rows(sens: &Sensitivities, intent: &Intent, af_per_nm: f32, r_ohm_per_um: f32, policy: &Policy)
      -> (Vec<analog::routing::PerformanceBudget>, Vec<(NetId, RcClass)>, Vec<Diagnostic>);
  ```
  1. One row per finite bound (two rows for a two-sided spec). Headroom: floor `(proc.0 or f0) − lo`, ceiling `hi − (proc.1 or f0)`, minus `β·σ_f` when `σ_f` is known (PERF-13 must not subtract it again). Row `metric` = `"{metric}:min"` / `"{metric}:max"`, `limit = 1.0`. Headroom ≤ 0 → PERF-06 step 2's do-not-worsen row: `limit = 0.0`, weights `sign·∂f/∂x / |bound|` (`|bound| = 0` → divide by 1), plus `Diagnostic "no_layout_margin"` (GRAEB-11).
  2. Weights: `w = sign·∂f/∂x / headroom` with sign −1 for a floor, +1 for a ceiling; with `credit_helpful = false` (default) keep only `w > 0` (conservative split, BAL2-16). The literature disagrees (NOTES-10 keeps signs): the default follows BAL2-16 because a helpful parasitic outside the linearization range cannot be trusted; the choice is recorded in `BatchMeta.origin`.
  3. The row type (`kernel/analog/src/routing/performance.rs:19-27`, today `metric, nets, weights, af_per_nm`) gains exactly the fields other plans already read — names fixed here so RTE-21/22 (`row.series`, `row.coupling`) and PERF-06/12 agree:
     ```rust
     pub struct PerformanceBudget {
         pub metric: String, pub nets: Vec<NetId>, pub weights: Vec<f32>, pub af_per_nm: f32,
         /// PERF-06: 1.0 normally; 0.0 = do-not-worsen row. residual = (used − limit).max(0).
         pub limit: f32,
         /// Series-R terms, 1/Ω (fraction of headroom per ohm), from `SpecSens.d_r`.
         pub series: Vec<(NetId, f32)>,
         /// Coupling terms, 1/aF, from `SpecSens.d_cc` (LAMP-39 ½·S_Cki per pair; PERF-12 step 3).
         pub coupling: Vec<(NetId, NetId, f32)>,
     }
     ```
     `used()` keeps measuring ground C only; the `series` and `coupling` sums are RTE's measurement (RTE-21/22 already read them for pricing); until RTE measures them, metadata reports those terms `unknown` (never 0). Every construction site of `PerformanceBudget` gets `limit: 1.0, series: Vec::new(), coupling: Vec::new()`.
  4. R/C class per net: with `C_ref = wire_af_per_um · rc_ref_len_um` and `R_ref = R□(lowest metal) · rc_ref_len_um / W_min`: C-sensitive if `|∂f/∂C|·C_ref ≥ rc_share·headroom` for some spec, R-sensitive likewise; the class is None / R / C / Rc (100 µm and 5 % are **Philis policy**, the source gives no threshold). `Intent.nets[n].rc` feeds RTE's width choice (SURV-29).
  5. Fallback without sensitivities (replaces the smallest-gate rule, AA-25): a net's load = Σ driven gate C (as today) + the sidecar `Load` value when given; a drain-only net with neither → `c_budget_af = None` and `missing ("ParasiticBudget", "external load of <net>")`, never the smallest gate.
- Tests (`budget::tests`): `two_sided_spec_gives_two_rows` (UGF 100 ≤ f ≤ 300 MHz, f0 200 → floor and ceiling rows, headroom 100 each); `process_spread_shrinks_headroom` (proc (180, 230) → floor 80, ceiling 70); `beta_reserve` (σ_f 10, β 3 → both headrooms −30); `conservative_split_drops_helpful_terms` (floor spec, ∂f/∂C = +0.5 and −0.5 on two nets → only the −0.5 net kept, weight 0.5/headroom); the two PERF-06 tests ported from `perf.rs`: `rows_cover_both_bounds_of_a_window_spec` (spec [10, 20], f0 15, d = [−1.0] → `:min` weight +0.2, `:max` weight −0.2 with `credit_helpful = true`; with the default only `:min` keeps the term) and `a_missed_bound_keeps_a_do_not_worsen_row` (f0 5, lo 10, d = [−1] → one row, `limit == 0.0`, weight 0.1); `drain_only_net_without_load_is_unknown` (`ota5t` `vout2` without a Load entry → `None` and a missing entry).
- Acceptance: AF-16 closed; PERF's A/B run (LAMP-33) can report rows per bound.
- Risks / notes: dropping helpful credit makes budgets tighter; PERF decides whether to expose `credit_helpful`.

### EXT-26 User constraint sidecar (ALIGN-compatible subset plus classes)
- Priority: P1. Effort: M. Depends on: EXT-14, EXT-16, EXT-18.
- Why: AA-11 (overrides only suppress; nothing sets them), G10 (user constraints as input; ALIGN names are cited by the code itself at `catalog.rs:4-5`, `netrole.rs:57-58`); H08-61 (matched groups are designated by the designer); Lampaert: constraints annotated on schematic symbols (lampaert.txt L6928–6939, PDF 167).
- Current: `AnnotationConfig` (`netrole.rs:59-75`) is only defaulted apart from `process`, which `library::annotation` fills from the deck (`frontend/library/src/lib.rs:492-515`) (grep of frontend, benchmarks, examples, backend); `bench.rs:269` re-annotates with `default()`.
- Change: new `backend/annotator/src/sidecar.rs` (adds `serde_json = "1"` to `backend/annotator/Cargo.toml`; already in the workspace lockfile via `backend/verify`):
  ```rust
  /// Parse a JSON array of constraint objects; names are case-insensitive and resolved
  /// against the netlist; unknown constraints and names are returned as diagnostics, never dropped silently.
  pub fn parse(json: &str, nl: &Netlist) -> Result<(AnnotationConfig, Vec<Diagnostic>), String>;
  ```
  Supported entries: ALIGN `PowerPorts`, `GroundPorts`, `ClockPorts`, `DoNotIdentify`, `GroupBlocks` (proximity group plus an alias for its instance name), `SymmetricBlocks` (`direction` V/H; an entry of two names is a seed pair, and two group aliases pair their members in order, as `["xinv_n","xinv_p"]` gives mp11↔mp12 and mn13↔mn14; an entry of one name is a self seed for a device, and a seed pair of its two members for a two-member alias, as `["xdp"]` gives mn1↔mn2), `SymmetricNets` (net seed), `Order` (EXT-28). Philis extensions: `{"constraint":"Match","instances":[..],"class":"minimal|moderate|exceptional","kind":"voltage|current|ratio"}`, `{"constraint":"NetClass","nets":[..],"class":"sensitive|bias|reference|noisy|digital"}`, `{"constraint":"OffsetBudget","instances":[..],"sigma_mv":0.5}`, `{"constraint":"Load","net":"vout","ff":1000}`, `{"constraint":"Kelvin","instance":"RS","sense":["M1/G","M2/G"]}`. `AnnotationConfig` gains `seeds: Vec<Seed>`, `groups: Vec<Vec<DeviceId>>`, `classes: Vec<(Vec<DeviceId>, MatchClass, MatchKind)>`, `net_classes: Vec<(NetId, NetClass)>`, `offset_budgets`, `loads`, `kelvins`, `order`.
  FLOW's plan names this entry point `AnnotationConfig::from_json` (plan-08 §0 table, FLOW-12): provide it as `impl AnnotationConfig { pub fn from_json(json: &str, nl: &Netlist) -> Result<(Self, Vec<Diagnostic>), String> { sidecar::parse(json, nl) } }`. Resolution to ids happens against the flat netlist, so it runs after parsing; `do_not_identify` (`netrole.rs:64`) is filled with resolved ids here, which closes AA-11's "nothing sets it". Coordinated edits: FLOW-12 adds `--constraints FILE.json` and passes the parsed config into `library::run`; `benchmarks/src/bench.rs:269` re-annotates with the same config instead of `AnnotationConfig::default()` (PERF).
- Tests (`sidecar::tests`): `align_power_ground_clock` (the gold's first three entries → vcc Supply, vss Ground, clk Clock); `symmetric_blocks_become_seeds` (feeding the full gold file as input yields exactly the gold pairs, one axis); `unknown_constraint_is_diagnosed` (`{"constraint":"Align",..}` → one diagnostic, no error); `match_class_overrides_role` (`Match` Exceptional on `ota5t` DP → Exceptional, `ClassSource::User`).
- Acceptance: a user can add, and not only suppress, constraints; the gold file round-trips.
- Risks / notes: ALIGN's `Align`, `CompactPlacement`, distances and `ConfigureCompiler` are reported unsupported.

### EXT-27 Hierarchy: subcircuit instances, identical instances and arrays
- Priority: P2. Effort: L. Depends on: FLOW (parser keeps the instance tree, AF-02), EXT-13, EXT-14.
- Why: AA-08, G13 (exact hierarchy, SameTemplate, arrays); survey hierarchical symmetry and proximity from exact and virtual hierarchies (balasa_graeb_survey.txt L2996–3050, PDF 75–76); H15-05 (matched arrays in one cell), H15-07 (mirror-image placement of identical blocks); SURV-05 (array regularity).
- Current: flat parser, ports and subcircuits dropped (`frontend/library/src/parse.rs:35-39`).
- Change: consume `Netlist.instances: Vec<(String /*path*/, String /*subckt*/, Vec<DeviceId>)>` (FLOW). New `backend/annotator/src/hier.rs`:
  ```rust
  pub fn same_template(nl: &Netlist, canon: &[u64]) -> Vec<(u32, u32)>;       // instance pairs of one subckt, device-wise isomorphic
  pub fn arrays(nl: &Netlist, canon: &[u64], classes: &[NetClassification]) -> Vec<Vec<u32>>; // ≥3 instances of one subckt sharing a Bias net
  ```
  Identical instance pairs become `Seed::Devices` for every device pair (symmetric when their port nets pair up under EXT-14) and a `MatchBlock` between them otherwise; arrays become a `GroupNode` of kind Proximity with an `ArrayStyle::Any` request and an order (EXT-28). Each subckt instance is a virtual-hierarchy boundary: `ProxNet` edges do not cross it (**Philis policy**).
- Tests: `hier::tests::two_identical_otas_pair_up`; `hier::tests::bias_shared_array` (4 identical current cells on one bias net → one array group of 4).
- Acceptance: T2 on a two-instance fully differential netlist (FLOW parser needed).
- Risks / notes: blocked on FLOW's parser.

### EXT-28 Signal-flow and current-flow ordering
- Priority: P2. Effort: M. Depends on: EXT-13, EXT-17.
- Why: SURV-07/08 (monotonic current-flow and signal-flow placement; perf_driven_survey.txt L111–117 right column, PDF 2); SURV-36 (directed causal edges gate → drain and source → drain; todaes22.txt L294–329, PDF 8); ALIGN's `Order` in the gold file (`mn0 → xdp → xccn → xccp`, top to bottom).
- Current: no ordering data (grep `current_flow` finds nothing; `monoton` finds only unrelated Φ-monotone acceptance comments, `kernel/analog/src/placement/dti.rs:52, 138`, `frontend/library/src/metadata.rs:90`).
- Change: new `backend/annotator/src/flow.rs`:
  ```rust
  /// Directed control graph: FET G→D and S→D net edges, BJT B→C and E→C.
  pub fn stage_order(hg: &BipartiteHypergraph, tree: &[GroupNode], ports: &[NetId]) -> Vec<u32 /*group*/>;
  /// Per supply-to-ground conduction chain (D–S edges with |Id| ≥ 1 % of max, Philis threshold): device order from ground up.
  pub fn current_paths(nl: &Netlist, op: &OpFacts, classes: &[NetClassification]) -> Vec<Vec<DeviceId>>;
  ```
  Output: `Intent.order: Vec<(Vec<u32 /*group ids*/>, AxisDir)>` (added to the EXT-12 struct with this item). Without an op point, current paths use the structural chain from a Ground net through D–S edges of saturated-role devices. PLC implements the ordering cost.
- Tests: `flow::tests::strongarm_order_matches_gold`: the current path from `vss` is `mn0 → {mn1,mn2} → {mn3,mn4} → {mp5,mp6}` (the same sequence as the gold's `Order` instances `mn0, xdp, xccn, xccp`; the gold lays that sequence out `top_to_bottom`, mn0 at the top, so the physical direction is PLC's choice); `flow::tests::three_stage_stage_order` (DP stage before M6 before M8/M9).
- Acceptance: the gold `Order` is reproduced.
- Risks / notes: none; data only.

### EXT-29 Structure and bias audit diagnostics
- Priority: P2. Effort: S. Depends on: EXT-16, EXT-17. Area and length shortfalls (H13-43/44, MM-14, MM-21) are MAT-08's `SizingNote`; D* per set (MM-06) is MAT-16's ledger column; this item keeps the checks that need EXT's roles or the op point.
- Why: things layout cannot fix must be reported, not chased by the annealer: H13-32 (V_gst ≥ 100 mV for current-matched moderate/exceptional devices, rule 4, hastings.txt L42464–42472), H13-33 (CLM from ΔV_DS, eq. 13.53, L41542–41583), H13-34 (cascode ratio identity eqs. 13.55–13.56 and cascode bulk to source, L41610–41647), H09-04 (BJT ratio 6:1–16:1, even; L30628–30637, PDF 513), H09-23 (equal V_CE).
- Current: none.
- Change: new `backend/annotator/src/audit.rs`, `pub fn audit(intent: &Intent, nl: &Netlist, drawn: &[Drawn], ev: &Evidence) -> Vec<Diagnostic>`:
  1. `vgst_low`: Current sets of class ≥ Moderate, per member with an op point: V_gst = |vgs| − |vth| when both are known, else 2·|Id|/gm (square law, **derived**); V_gst < 100 mV → diagnostic.
  2. `clm_mismatch`: Current sets with `gds` known: `100·gds_ref·|V_DS,i − V_DS,ref| / |Id_ref| > X/2` (%, X = the number in `limit(family, Current, class)`, MAT-07; the half share is **Philis policy**) → diagnostic naming the two members.
  3. `cascode_ratio`: matches of `cascode_mirror`, `wide_swing_cascode_mirror`, `low_voltage_cascode_mirror` (slots 0..3, EXT-05): `|(W0/L0)/(W1/L1) − (W2/L2)/(W3/L3)| > 0.01·(W2/L2)/(W3/L3)` → diagnostic; a cascode (slot 2 or 3) whose `B` net differs from its `S` net → `cascode_bulk` diagnostic (H13-34).
  4. `bjt_ratio`: BJT sets of class ≥ Moderate with a member ratio N (units / min units) that is odd and > 1, or > 16 → diagnostic (H09-04; EXT-15 already flags > 16 for any class); with an op point, `|V_CE,i − V_CE,j| > 10 mV` (**Philis threshold**) → `vce_unequal` (H09-23).
  Diagnostics go to `Intent.diagnostics` and the metadata report; they never alter emitted constraints.
- Tests (`audit::tests`): `vgst_floor` (current set, Moderate, gm = 20 µS, Id = 0.6 µA → V_gst 60 mV → one diagnostic); `bjt_ratio_odd` (1:7 Moderate → one diagnostic; 1:8 → none); `cascode_ratio_mismatch` (bottom 2u/1u : 4u/1u, top 2u/0.5u : 2u/0.5u → one diagnostic).
- Acceptance: every corpus circuit runs the audit without panics; `three_stage` (bias set Minimal) reports no `vgst_low`.
- Risks / notes: diagnostics depend on op-point data; absent data → no diagnostic, and the report says "not checked".

---

## 4. Milestones

Dependency order inside EXT (acyclic; `→` = must land before): 01 → 04 → {05, 06}; 05 → {07, 09}; 06 → 10 → 12; {06, 12} → 13; {05, 11, 12} → 14; {11, 12, 13, 14} → 15 → 16 → 20; {14, 15} → 20; {03, 11, 15} → 19; 12 → 17; {02, 03, 17} → 18; {17, 18, 20} → 23; {14, 18, 20, 23} → 24; {17, 20} → 21; {17, 18, 24} → 25; {14, 16, 18} → 26; {13, 14} → 27; {13, 17} → 28; {16, 17} → 29. 02, 03 and 11 have no EXT predecessor (11 needs FLOW-01). External gates are listed per item; the ones that decide a milestone are named below.

| Milestone | Items | External gates | Exit criteria |
|---|---|---|---|
| M0 Harness and P0 fixes | EXT-01 … EXT-07, EXT-09 … EXT-11 | FLOW-01 (for EXT-11) | All existing annotator tests pass (32 per audit-06) plus the new ones; `negative_corpus` (T3) passes for `sc_switches` and `equal_fets`; `latch` has 2 DiffPair leaves; `rail2rail` has both pairs in both orders; `bjt_mirror` collector nets `Signal`; `rail_names` and `bulk_inference` green; 0 `Differential` in `hard`; `permutation_invariance` passes on `canon_leaves`; T7, T8; `bench local` DRC 0 / LVS MATCH on the 10 fixtures (T9) |
| M1 Data model, graph, symmetry | EXT-12, EXT-13, EXT-14 | MAT-07 step 1 (or EXT-12 creates the enums) | Workspace builds with `analog::intent`; T1 (StrongARM gold) passes; `folded`, `gilbert`, `rail2rail`, `three_stage` one axis each; `axis_count` drives the axis table; T4 on `canon`; T9 |
| M2 Sets, ratios, classes, passives, emission | EXT-15, EXT-16, EXT-19, EXT-20 | MAT-04, MAT-05, MAT-07, MAT-08 (EXT-20) | T2 on all 17 circuits including `mirror6` (6 members), `three_stage` bias [4,3,10], `bgr_core` [1,8], `dac4` Exceptional [1,1,2,4,8], `brokaw`, `rdiv`, `splitdac`; T5; T10; cellgen side recognizers deleted with unchanged drawings on `dac4`, `bgr_core`, `pair`, `quad`; T9 |
| M3 Evidence, net classes, substrate tags, routing | EXT-17, EXT-18, EXT-23, EXT-24 | PERF-09 (op-point fields; tests use synthetic `DeviceOp`s without it), REL-09 (T6) | `dac4` digital inputs `DigitalStatic`, not Sensitive; with an op point on `ota5t`: XM1–XM5 Saturation, XM5 CurrentSource; `strongarm` Switching tags and no isolation between `mn0` and `mn1/mn2`; T6; shields referenced to the quiet ground; `CommonNodeReq`/`StarReq`/`KelvinReq` tests pass; T9 |
| M4 Performance-driven extraction and user input | EXT-21, EXT-25, EXT-26 | PERF-06, PERF-12, PERF-13, FLOW-12 | With PERF sensitivities on `ota5t`: allocation respects every spec; one row per bound with `limit`; sidecar round-trips the gold; PERF's A/B (constraints on vs blocks-only, PERF-24) reported in the bench table |
| M5 Advantages | EXT-27, EXT-28, EXT-29 | FLOW-07 (EXT-27, 28) | Gold `Order` reproduced; audit diagnostics in the metadata report; hierarchy tests pass once FLOW's parser lands |

---

## 5. Open questions (each with the default this plan assumes)

1. **Device-size convention (AF-01).** Decided by FLOW-01 (SPICE `W_total`, `nf`, `m`); `size::drawn` reads it through `Device::mos_size()`.
2. **Loads sharing a gate net as symmetric pairs.** EXT-14 accepts a pair whose only non-mated nets are shared (the `ota5t` load on `vbias`, a mirror load on its diode net). Default: accept, because hand layouts place such loads symmetric and T3 guards against false pairs.
3. **Default precision class by role.** Default: Hastings's role guidance (input pairs and loads Moderate, bias mirrors Minimal, bandgap Moderate, DAC banks Exceptional), always reported as `ClassSource::Role`; MAT-08 does not tighten budgets for Role classes.
4. **Signed credit in sensitivity budgets.** BAL2-16 drops helpful terms; NOTES-10 keeps them. Default: drop (`credit_helpful = false`).
5. **`Unitization.class` optional or not.** MAT-07 step 5 makes it a plain `MatchClass` set to Moderate at every literal; CELL's §0.2 and EXT-12 use `Option<MatchClass>`. Default: `Option`, read with `unwrap_or(Moderate)` (same behaviour, and "not inferred" stays visible).
6. **Axis direction.** Default: vertical; horizontal only from the sidecar (`direction: "H"`), until PLC-20 supports it in the `Symmetry` rule.
7. **Sidecar location.** Default: only the FLOW-12 `--constraints FILE.json` flag; no implicit `<stem>.const.json` lookup.
8. **Is an operating point required?** Default: optional; every evidence-derived fact records its evidence level, and FLOW's certificate treats `probe_bias` facts as assumed.
9. **Netlist-proximity cluster cap.** Default: nets with more than 8 devices add no P_N edges (`Policy::pn_max_degree`).

---

## Verification log

Adversarial fact-check of this plan, 2026-09-28, against the working tree (HEAD `dad330c` plus uncommitted changes) and the `reftext/*.txt` extractions (PDF page = 1 + form feeds before the line). Read-only on code; no cargo command run. Hastings PDF pages 256, 418, 508, 524, 712 and 735 were opened to read numbers that are garbled in the text extraction.

**References checked: about 500.** 173 audit/ref ID citations (every AA, AR, AF, AP, AT, AC, NOTES, BAL1, BAL2, H01–H15, SUB, MM, CC, EM, LAMP, GRAEB, SURV ID and G10/G13; all exist, and each cited statement was compared with its entry); 86 reftext line ranges, PDF pages, formulas and numeric constants; about 240 code citations (file:line, type, fn, field, constant, test name, fixture content, deck key), plus a collision grep for every new type and function name.

**Confirmed without change (sample of the load-bearing ones):** 103 patterns in `PATTERNS` (`catalog.rs:2572-2687`); every catalog line range and priority cited in §1.4 and EXT-04; the slot layouts behind every EXT-05 declaration; all `emit.rs`, `extract.rs`, `classify.rs`, `constraints.rs`, `ir.rs`, `netrole.rs`, `block.rs`, `pattern.rs` ranges; `RuleBatch` has 18 methods (`rule.rs:135-208`); 32 annotator tests (24 + 5 + 1 + 1 + 1); gr/dr/dp/gp ranges; cellgen ranges; sky130 deck values (`well_spacing` 1270, `res_min_segment` 10000, `wpe_clearance_nm` [2000, 3000, 5000], `min_guard_ring_width` 420, `guard_licon_pitch` 360); ALIGN gold content (7 pairs, self `mn0`, 3 net pairs, `Order`); Hastings class limits (MOS ±10/±3/±1 mV or %, PDF 712; bipolar ±2 mV/±8 %, ±0.5/±2, ±0.1/±0.5, PDF 524; R/C ±1–5, ±0.1–1, ±0.01–0.1 %, PDF 418); degeneration resistors 4 kΩ / 2 kΩ / 1.33 kΩ (PDF 508); injector thresholds 10 kΩ (PDF 256) and 50 kΩ (PDF 735); survey eq. 3.19 and Fig. 3.11; Graeb eqs. 74, 175–179 and βW = 3; Charbon 2.5–5× / Su 4×; DACP eq. 1 and the `splitdac` arithmetic (4/3 unit); Table 13.4 check (6·12.5/1)² = 5625 µm²; every EXT-15 unitization test result.

**Corrections (before → after):**
1. Annotator `lib.rs` citations were offset by +8 lines throughout: `lib.rs:83-159` → `75-151`; `:84` → `:76`; `:85-89` → `:77-81` (twice); `:101-107` → `:93-99` (five places); `:111-112` → `:103-104`; `:114` → `:106`; `:115-125` → `:107-117` (three places); `:129-136` → `:121-128`; `:61-79` → `:53-71` (three places); `:83` → `:75` (twice); `:88` → `:80`; `:111` → `:103`; `:163-169` → `:155-161`; `:50-53` → `:42-45`.
2. §1.1 item 2 and EXT-26 "AnnotationConfig is never set / only defaulted outside the annotator" → only its override fields are never set; `library::annotation` fills `process` (`frontend/library/src/lib.rs:492-515`).
3. §1.4 AA-10 `lib.rs:871-872` → `frontend/library/src/lib.rs:871-872` (ambiguous path).
4. T9 "all 10 clean" → all 10 report DRC 0 and LVS MATCH; `bgr_core`/`bjt_mirror` MATCH with nothing compared and ERC 1 each (audit-06 §G.3).
5. EXT-01 "the 17 devices" of `high_speed_comparator.sp` → 15 devices.
6. EXT-01 StrongARM transcription: added an explicit equal `w=1u` (the .sp has no `w`; with EXT-11, unknown sizes would block every `ExactAs` match and T1).
7. EXT-02 H15-06 "PDF 783–784" → PDF 784, and noted the source gives suffix conventions, not rail names (rail roots are Philis policy).
8. EXT-04 Gilbert fix "replace `2G=3G`" → replace `2G=3G` and `4G=5G` (`catalog.rs:1275-1276`); keeping `4G=5G` makes the new links contradictory. Output links cited at `:1277-1278`.
9. EXT-04 acceptance "M6/M8/M9 no longer vanish into miller_comp_fets-style wildcards" → they are in `push_pull_with_bias` before and after (audit-01 §2.13-2); EXT-04 only removes the wildcard.
10. EXT-06: added that `Geom` has only `w`, `l` and the hypergraph has no model, so model/finger label terms need EXT-11; pin-table order "(G, D, S, B, P, N, C, E)" → G, D, S, B, C, E, P, N to match EXT-12's `Term`.
11. EXT-07 "cascode gate nets become Signal; `chain4` gate net Signal" → gates-only nets (`vcas`, `chain4`'s `g`) stay Sensitive under `classify.rs:127` until EXT-18; the observable changes are victim status and `gate_of_sensitive`.
12. EXT-08 "one `Conflict` is reported" → one per device pair (two: mn0–mn1, mn0–mn2), as `Conflict.devices` holds one pair.
13. EXT-09 "`gr/src/lib.rs:253` and `:277` iterate hard+budget" → `:277` (`symmetric_nets`) and `dr/src/lib.rs:811` only; `:253` excludes `Differential` and needs no change because `tier` checks `sym` first (`:262-266`).
14. EXT-09 "`dac4` with ground `0`: today 6 `Differential`" → 10 (five NMOS on `0`, including `XMC`; the AA-14 hand trace omitted `XMC`).
15. EXT-10 `Policy` "every literal listed in AA-28" → every one except the ring literals (`constraints.rs:79-81`), which EXT-23 takes from the deck.
16. EXT-11 "gate area in `constraints.rs:22-24`" → `fingers()` lives there; the gate area is `lib.rs:155-161`.
17. EXT-12: added the `verify::Intent` name clash (`backend/verify/src/lib.rs:58`; library `lib.rs:95, 477`) and the same-named functions replaced by EXT-11/EXT-23.
18. EXT-13 Algorithm 3.1 "L5325–5420, PDF 125–126" → §3.2.3.1, L5369–5408, PDF 126–127.
19. EXT-13 "the survey adds a clique per net without excluding rails" → the survey does not say whether rails are excluded (BAL1-49).
20. EXT-13 "D16 kept: glue last" → glue last is the `Problem::blocks` contract (`lib.rs:30-31`); D16 is the `groups`/`abutment` split.
21. EXT-14 pages: survey L5230–5310 "PDF 123–124" → 123–125; L675–711 "PDF 20–22" → 21–22; lampaert L3316–3336 "PDF 83–84" → 84.
22. EXT-15 "Hastings rule 5: keep W and L ≥ 1 µm, L42473–42482, PDF 713" → MOS rule 11 (avoid submicron in moderate/exceptional), L42525–42531, PDF 714; rule 5 is the pocket-implant rule.
23. EXT-15 rule 12 "PDF 714" → PDF 714–715.
24. EXT-16 R/C limits "the upper end of each Hastings range" → the tight, lower end (1 of 1–5 %, 0.1 of 0.1–1 %, 0.01 of 0.01–0.1 %).
25. EXT-16 test "the 12 numbers above" → 15 (five arrays of three).
26. EXT-16 H13-53 "EXC ≥5 µm" → EXC ≥5–10 µm or ≥2× well depth; 5000 is the lower bound.
27. EXT-18 suffixes `_n`, `_s`, `_hv` "L46495–46514, PDF 783–784" → L48618–48621 (PDF 819) and L48711–48713 (PDF 820).
28. EXT-18 NetClass consumer list "complete" → added `extract.rs:90-94`, `emit.rs:271` and the Supply/Ground-only readers (`ir.rs:49`, `elaborate.rs:306`, library `lib.rs:307, 634, 1135`); the only exhaustive match is `classify.rs:28-32`.
29. EXT-18 100 kΩ threshold: attributed to eq. 15.23, L48574–48595 (H15-28 recipe, H15-29).
30. EXT-19 eq. 10.10 "PDF 508" → PDF 508–509.
31. EXT-21 "6.9 → 3.7 mV, L5027–5082" → Table 4.1, L5006–5010, PDF 119 (Table 4.2 stays L5027–5082); the Table 4.2 sensitivity column is ∂offset/∂distance (µV/µm), so the test uses it only as relative weights.
32. EXT-24 "`frontend/library/src/lib.rs:785-786` takes any shared source net" → `:784`.
33. EXT-24 item 8 "bottom-plate R sets f3dB (CC-14)" → CC-14/DACP §II-A says bottom-plate parasitics load V_REF and set f3dB; the RC form is DACP §III-C eq. 32, L445–480, PDF 5.
34. EXT-27 survey L2996–3050 "PDF 75–77" → 75–76.
35. EXT-28 SURV-36 cited to perf_driven_survey → todaes22.txt L294–329, PDF 8; SURV-07/08 range L111–119 → L111–117; "grep `current_flow|monoton` finds nothing" → `monoton` hits unrelated Φ-monotone comments (`dti.rs:52, 138`, `metadata.rs:90`).
36. EXT-28 test "top_to_bottom reversed to bottom-up" → the path is the same sequence as the gold's `Order` list; the gold lays it out top_to_bottom with mn0 on top.
37. EXT-29 H09-04 "PDF 511–513" → PDF 513 for L30628–30637.
38. EXT-29 item 7 "`(σ_rand/allowance)²` when the allowance is 0" (division by zero) → `(σ_rand/σ_b)²` when sizing alone spends the budget (η = 0, `emit.rs:86`), derived from MM-14.
39. §4 M0 exit "`dac4` digital inputs not Sensitive" → moved to M3: no M0 item changes the gates-only rule; EXT-18 makes `d0..d3` DigitalStatic.

**Unresolved (not verifiable without running code, which this task forbids; left as stated, not marked false):** the predicted outputs of algorithms that do not exist yet (EXT-13/14 tree and compound expectations, EXT-17 regions on the probe bench, EXT-20/23/24 rule counts after the change, T8 runtime); the "passes today" status of `negative_corpus` for `bjt_mirror`/`inv_chain` and the DiffPair status of `sc_switches`/`equal_fets`, which were checked by hand-tracing `pattern.rs` and the catalog links only; whether today's recognizer puts StrongARM `mn0` in `diff_pair_with_tail` (hand trace: `cross_coupled_inverters`, priority 26, takes mn3/mn4/mp5/mp6 before `diff_pair_with_split_cascodes`, 21, so `diff_pair_with_tail` then takes mn0/mn1/mn2, consistent with audit-01 §2.13-7a but not executed).

---

## Cut or deferred items

| Item | Status | Reason |
|---|---|---|
| EXT-08 Conflict pre-pass (`conflict.rs`, `Conflict`, `Intent.conflicts`) | Cut | The only concrete conflict (AA-13) is removed at its source by REL-09 step 3 and EXT-23 step 4; EXT-14 cannot create symmetry conflicts by construction; sidecar distances are reported unsupported (EXT-26), so no other source of contradictory distances exists. T6 is kept as a corpus assertion. Re-enter if a new constraint source can emit a Proximity and an Isolation on one pair. |
| EXT-22 Diffusion-sharing permission and well groups (`share.rs`, `Intent.abutment`, `Intent.wells`, `WellGroup`) | Deferred | The abutment table's only reader (dp rotation lock) is deleted by PLC-03/PLC-06; no PLC, CELL or REL item reads well groups (PLC-07 derives well sharing from cell edge profiles, REL-16 from `CellFlags`). Re-enter with PLC-26 (cross-cell diffusion abutment). `Problem.abutment` stays as today until PLC-27 deletes it. |
| EXT-12 `GuardRingType::MajorityTap`, `RingRole`, optional ring pitch/resistance | Cut (duplicate) | REL-07 step 1 owns the ring taxonomy (`PTap`/`NTap`/`Ecgr`, `RingRole`); CELL-05 owns the drawing names. |
| EXT-12 `Intent.orientation_groups`; EXT-20 item 6 | Cut (duplicate) | PLC-03 derives orientation and shape locks from every rule's `matched_pairs` hook; MAT-05's `OrientationSet` checks it. |
| EXT-16 `class::limits` and `Family` in the annotator | Cut (duplicate) | MAT-07 owns `analog::matching::class::{Family, limit}`; the two drafts disagreed on the R/C limits (EXT took the tight end of each Hastings range, MAT the loose end); one table, MAT's, now decides. |
| EXT-20 √n quadrature split of one allowance over MatchingPair / CentroidGroup / ThermalGradient / CommonNode | Cut (duplicate) | MAT-04 replaces those rules by one `MatchedSet` ledger that spends one allowance (the AR-04/MM-31 fix); MAT-06 hands the remainder to CommonNodes. |
| EXT-21 `discover_pairs` (parameter-similarity pairing, GRAEB-20) | Deferred | Its thresholds (cos < −0.9, norm ratio 0.8–1.25) are Philis inventions, not from the source, and no target metric measures the benefit. Re-enter when PERF-12 sensitivities exist and a corpus circuit shows a mismatch-critical pair that recognition misses. |
| EXT-23 minority-carrier injector tags, isolation distance model, ring requirements | Cut (duplicate) | REL-08 (injectors, 50 kΩ threshold), REL-09 (substrate kind, epi plateau, cost-only on bulk), REL-07 (rings by role, tie nets). EXT-23 keeps the aggressor/victim tags those items consume. |
| EXT-23 `ImpactIonization` aggressor weight (SUB-08) | Deferred | No consumer reads a per-device injection strength; the constants of eq. 2.3 are not given. |
| EXT-24 item 6 supply net splitting (BAL2-06, 10× median threshold) | Deferred | The threshold is a Philis invention; supply IR is REL-04's terminal-resolved IR drop. Re-enter if RTE-17's star check shows supply-borne offset on a fixture. |
| EXT-24 item 7 per-net current bounds and `em_critical` | Cut (duplicate) | REL-01 owns per-pin EM currents; RTE-14 step 1 computes the EM-critical filter from them. `NetFacts.current_ua` and `em_critical` removed. |
| EXT-24 `CommonNodeReq.max_delta_mv` | Cut (duplicate) | MAT-06 supplies each set's remaining allowance; RTE-17 step 4 converts it to `max_delta_ohm`. |
| EXT-25 matched-pair ΔC terms (`diff_pairs`, GRAEB-19 ridge) | Deferred | No plan measures \|C_a − C_b\| per row; RTE-05's `Differential` already prices pair asymmetry. |
| EXT-29 area audit, current-matching length lever, D*, area multiplier | Cut (duplicate) | MAT-08 (`SizingNote`, `area_for`, `Lever::Length`) and MAT-16 (D* column). |
| EXT-14 step 10 thermal self-symmetry of `hot` devices; `DeviceFacts.power_uw/hot` | Cut | The step emitted nothing beyond the structural rule; power reaches placement through `cells.power` (PLC-06). |
| Gilbert switching quad as one 4-member matched set | Deferred | EXT-05 declares the mirror couples (M3,M6), (M4,M5) needed for symmetry; merging the source-coupled switching pairs into one set needs a quad declaration and a CELL quad generator. Re-enter with a mixer fixture. |

---

## Implementer review log

Review of 2026-09-29 by the implementing engineer, against the code (HEAD `dad330c` + working tree) and plans 02–08. 29 items reviewed. Changes:

1. **Ownership conflicts with other plans resolved.** EXT duplicated MAT-04/05/06/07/08 (matched-set rule, orientation, allowance split, class tables, sizing notes), REL-07/08/09 (rings, injectors, isolation), REL-01/RTE-14 (EM currents), PLC-03 (orientation locks) and FLOW-01 (size convention, gate area). EXT now produces the data those items read and no longer defines a second copy; see "Cut or deferred items".
2. **Name collision removed.** EXT's data type `analog::intent::MatchedSet` collided with MAT-04's `analog::placement::matched_set::MatchedSet` rule; renamed `MatchSpec` everywhere in this plan. `MatchClass`/`MatchKind`/`Family` are re-exported from MAT-07's `analog::matching::class` (EXT-12 creates that module with the three enums if MAT-07 is later; `MatchClass` must derive `PartialOrd, Ord`).
3. **EXT-12 contract trimmed** to the fields some consumer reads: removed `Conflict`, `abutment`, `wells`, `WellGroup`, `orientation_groups`, `DeviceFacts.power_uw/hot`, `NetFacts.current_ua/em_critical`, `Inject::{MinorityElectron, MinorityHole, ImpactIonization}`, `Aggressor.weight`, `CommonNodeReq.max_delta_mv`, all `GuardRing*` changes; `Unitization.style` became `Option`; the full `Unitization` literal-site list added; `matching` module creation spelled out.
4. **EXT-20 rewritten** to emit one MAT-04 `MatchedSet` and MAT-05 `OrientationSet` per `MatchSpec`, with the exact `Budget` mapping that makes an EXT-21 allowance round-trip (`Sigma1Mv(√(a² + σ_rand²))`); T5 redefined as "one rule per set" with a new test; the √n split cut.
5. **EXT-23 reduced** to aggressor/victim tags (`Switching`, `Capacitive`; victims by class and Bias/Reference gates) feeding REL-07/09/16; corrected the `ota5t` expectation (XM5 is a victim through its Bias gate `vbn`).
6. **EXT-24**: `quiet_ground` specified (moved from EXT-23); StrongARM shield expectation corrected from `vin_d/vip_d` to `vin_o/vip_o` (the gate nets of the cross-coupled Voltage sets); items 6–7 and `max_delta_mv` cut; consumers named.
7. **EXT-25**: `PerformanceBudget` fields renamed to what RTE-21/22 and PERF-06/12 already read (`limit`, `series`, `coupling`); PERF-06's do-not-worsen row and its two tests ported; `BudgetPolicy` folded into `Policy`; diff-pair term deferred.
8. **EXT-11 rebuilt on FLOW-01** (`Device::mos_size`), dropping its own gate-area helper and library edit; added bulk to `Drawn` and a split-bulk test; exact assert message. EXT-15's `series_parallel_20_to_1` test rewritten for the SPICE `W_total` convention (`w=50u nf=5`), otherwise it would have encoded 4:1.
9. **EXT-15 `UnitDeck`** now names each deck source with sky130 values (grid 5 nm, `min_finger_width` 420, `max_finger_width` 10000, poly.1a 150, `res_min_segment` 10000) and the missing-key behaviour.
10. **EXT-16** now uses MAT-07's `limit()`; added the `SetRole` assignment rule; the `environment` consumer edit is sequenced after MAT-07 steps 3/6 (the deck array needs `Process::tier`); corrected the line range to `lib.rs:871-872`.
11. **EXT-05**: roles moved to a `ROLES` table keyed by template name (no edit to ~95 pattern literals), `roles_of` fallback for 2-slot primitives, `PatternMatch` gains its pattern, new `roles_are_well_formed` test; Gilbert quad limitation recorded.
12. **EXT-01**: the `net()` format documented as it is actually used by the corpus (SPICE order, model last; the previous `NAME KIND nodes` text contradicted every corpus line); exact `inv_chain` text; scale-test net count; T6/T7 corpus tests added with their `#[ignore]` gates.
13. **EXT-02**: `rail_of` body given as code; `classify_nets` edit and constant deletion listed; oppoint edit reassigned to PERF-09 (PERF owns `oppoint.rs`).
14. **EXT-10**: `Policy` field list re-derived from the literals EXT still owns (types, defaults, source lines; margin order corrected to Sensitive/Clock/Supply|Ground/other); `Needs` simplified; `Isolation` missing-entry left to REL-09; line `lib.rs:137-139` corrected.
15. **EXT-13**: `MatchSym` edges defined as `compound.pairs` only (the text said "every device pair of every compound", which would merge whole compounds into one set).
16. **EXT-17**: `SpecSens.d_cc` added (PERF-12 step 3); producers named (PERF-09/12, FLOW-07). **EXT-19**: passive-set `Origin` rule names fixed and RTE-20's `dac_banks` dependency sequenced before deletion. **EXT-26**: `AnnotationConfig::from_json` wrapper and FLOW-12 `--constraints` flag; `do_not_identify` now filled. **EXT-29**: reduced to V_gst, CLM, cascode ratio and BJT ratio checks with exact formulas and numeric tests.
17. **Milestones**: explicit acyclic dependency list, an external-gate column, T6 moved to M3 (needs EXT-14 and REL-09), EXT-08/22 removed. Open questions updated (size convention decided by FLOW-01; `Unitization.class` Option-vs-plain conflict recorded; sidecar only via the CLI flag; isolation and injector questions moved to REL).
