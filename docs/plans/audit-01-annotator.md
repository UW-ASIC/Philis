# Audit 01: constraint extraction (`backend/annotator`)

Auditor scope: every source file of `backend/annotator/src/` (read line by line on the current working tree, which includes uncommitted edits to `block.rs`, `classify.rs`, `emit.rs`, `extract.rs`, `lib.rs`, `netrole.rs`, `tests.rs` and the untracked `ir.rs`), plus the way `frontend/library/src/lib.rs` and `frontend/library/src/metadata.rs` consume `annotator::Problem`. Finding IDs: `AA-NN`. Code citations are `path:line` relative to the repository root. Reference citations give the source, section/equation, the line range in the extracted `reftext/*.txt`, and the PDF page computed by counting form feeds.

Method note: no cargo command was run (task rule). Every "trace" below is a hand trace of the matcher over a netlist in the repository, done by reading `pattern.rs` and `catalog.rs`. The traces are marked **(hand trace)**, and the planners should turn each one into a regression test (AA-26).

Cross-references: rule-type internals (what `Symmetry`, `MatchingPair`, `CentroidGroup` and the others measure) are audited in `docs/plans/audit-02-analog-rules-and-core.md` (IDs `AR-NN`). The parser and flow are covered in `docs/plans/audit-07-flow-frontend-docs.md` (IDs `AF-NN`). The prior handbook is summarised in `docs/plans/ref-00-prior-notes-handbook.md` (IDs `NOTES-NN`). This audit cites those IDs instead of repeating their content.

---

## 0. Scope

### 0.1 Files in scope (read completely)

| File | Lines | Read |
|---|---:|---|
| backend/annotator/Cargo.toml | 8 | 1–8 |
| backend/annotator/src/lib.rs | 166 | 1–166 |
| backend/annotator/src/catalog.rs | 2687 | 1–2687 |
| backend/annotator/src/pattern.rs | 212 | 1–212 |
| backend/annotator/src/netrole.rs | 121 | 1–121 |
| backend/annotator/src/block.rs | 90 | 1–90 |
| backend/annotator/src/classify.rs | 205 | 1–205 |
| backend/annotator/src/emit.rs | 323 | 1–323 |
| backend/annotator/src/extract.rs | 127 | 1–127 |
| backend/annotator/src/constraints.rs | 88 | 1–88 |
| backend/annotator/src/ir.rs (untracked, not in the task list but in `src/`) | 87 | 1–87 |
| backend/annotator/src/tests.rs | 557 | 1–557 |
| frontend/library/src/lib.rs (consumer) | 1439 | 1–1439 |
| frontend/library/src/metadata.rs (consumer) | 462 | 1–462 |

### 0.2 Read partially, to establish inputs and impact

- frontend/library/src/parse.rs 1–270: flat-only parser, terminal order G,D,S,B for FETs (75–93), C,B,E for BJTs and P,N for passives (211–218), numeric params `w l nf nfin stack m multi` only (96–104).
- frontend/library/src/cellgen.rs 30–160, 436–595, 700–790: a second recognizer (DAC banks, BJT groups, parallel MOS) and unitization merging.
- frontend/library/src/oppoint.rs 13–99: the `OpPoint` fields available (id, headroom, gm, power).
- kernel/core/src/hypergraph.rs 11–57: `BipartiteHypergraph`.
- kernel/analog/src/placement/matching_pair.rs 82–106 (`is_diff_pair`, `is_supply`); routing/crosstalk.rs 96–114; routing/differential.rs 116–133; cell.rs 21–64.
- kernel/cells/src/post_cell.rs 305–325 (how ring types are drawn); kernel/cells/src/mosfet.rs 74, 103 (who reads `dummy_required`/`route_matching_required`).
- benchmarks/fixtures/*.spice (all 10 netlists); examples/three_stage_opamp/src/main.rs 21–32 (netlist).
- pdks/*.json: the `cell.*` keys `library::annotation` reads (lib.rs:500–513).
- docs/CRATES.md 20–80; docs/API-WISH.md 260–345, 540–600; docs/LAYOUT-FUNDAMENTALS.md rows 2, 16, 29, 45, 58; the three plan docs named above (for their IDs only).

### 0.3 References used

| Source | Section / item | reftext lines | PDF page |
|---|---|---|---|
| Graeb (ed.), Analog Layout Synthesis survey, ch.2 (Lin, Chang) | §2.1 basic and hierarchical constraints (CC for mirrors and diff pairs; symmetry for the whole differential subcircuit; proximity for a shared well/guard ring; hierarchical symmetry/proximity) | balasa_graeb_survey.txt 2980–3050 | 75–76 |
| same, ch.2 | Def. 2.1, symmetry island | 3185–3190 | 79 |
| same, ch.3 (Strasser, Eick, Graeb, Schlichtmann) | abstract: automatic constraint generation from recognised building blocks and symmetry conditions | 4536–4545 | 110 |
| same, ch.3 | minimum-distance constraints for wells and guard rings, eq.(3.16) | 4742–4775 | 114 |
| same, ch.3 | §3.2 method; Table 3.1 requirement types M_S, M_B, P_B, S, P_N | 5013–5063 | 119–120 |
| same, ch.3 | importance order M_S ≻ M_B ≻ P_B ≻ S ≻ P_N, eq.(3.19), and its rationale | 5096–5132 | 121 |
| same, ch.3 | SMP multigraph; netlist proximity P_N = clique per net | 5136–5160 | 122 |
| same, ch.3 | §3.2.2.1 building-block recognition by subgraph isomorphism against a library (dp, level shifter, scm, ccm, 4cm, wcm) with matching and proximity per block | 5208–5226 | 123 |
| same, ch.3 | §3.2.2.2 symmetry analysis → symmetry compounds; one M_S per pair and one S per compound | 5230–5241, 5285–5310 | 123–124 |
| same, ch.3 | §3.2.3 HSMPG tree, Algorithm 3.1 (agglomerative, by importance order) | 5325–5420 | 125–126 |
| same, ch.3 | §3.2.3.3 dynamic importance "determined by simulation" | 5426–5431, 5608–5610 | 127–128 |
| same, ch.3 | §3.2.4 constraint generation: same variants and alignment for matching groups, CC when devices are split in subdevices, symmetry (pair)/(groups) eqs.(3.23)–(3.25) | 5613–5650 | 129 |
| same, ch.3 refs | [5] Charbon et al. constraint generation; [10] Arsintescu symmetry analysis; [13] Massier/Graeb sizing-rules library; [15] KOAN/ANAGRAM II | 8040–8080 | 143–144 |
| Lampaert, Gielen, Sansen 1999 | §2.6 diffusion sharing, device merging between electrically connected devices | lampaert.txt 2054–2063 | 52 |
| same | §3.3.1 merge criterion: common S/D node, same type, same bulk potential | 2741–2744 | 67 |
| same | §4.2 symmetry constraints (couples, self-symmetric devices, symmetry groups; identical variants and mirrored orientations) and matching groups (equal orientations; ratios built from equal unit devices) | 3318–3350 | 84 |
| same | §4.3 placement input: sensitivities plus operating point (branch currents, node voltages) from simulation | 3446–3456 | 86 |
| Hastings, Art of Analog Layout 3e | §13.3 minimal / moderate / exceptional matching (±10 mV or ±10 %, ±3 mV or ±3 %, ±1 mV or ±1 %, six-sigma). Numbers are garbled in the text and were read from the PDF page | hastings.txt 42320–42346 | 712 |
| same | §13.3 voltage- vs current-matched devices | 42326–42328 | 712 |
| same | MOS matching rule 1 (identical sections), rules 2–3 (voltage → area, current → length) | 42360–42412 | 712–713 |
| same | rule 4 (current-matched devices not in subthreshold), rule 6 (thin oxide) | 42464, 42483 | 713–714 |
| same | rules 7–10, 12 (orientation; proximity/interdigitation/CC by class; compactness and aspect ratio by class; 2-D CC; dummies by class) | 42490–42540 | 714 |
| same | rule 14 (matched devices away from power devices) | 42569 | 715 |
| same | resistor rule 5 (identical segments); capacitor rule 1 (identical geometries) | 25218–25220; 25470 | 419; 423 |
| same | BJT ratio limit (8–10 units); Fig.10.24 ratioed arrays 2:1:2 and 3×3 | 30177; 30661 | 506; 514 |
| Graeb, Analog Design Centering and Sizing | structural DC constraints c(x_d) ≥ 0 from circuit structure (sizing rules [58]) | graeb_centering.txt 2831–2834 | 76 |
| CC review (Common-Centroid Layout … Review) | DPs in OTAs, current mirrors and R/C arrays need matching | cc_review.txt 34–40 | 1 |
| Performance-driven survey | constraint and parasitic prediction with pretrained models [81] | perf_driven_survey.txt 308–309 | 5 |

---

## 1. Architecture and data flow

### 1.1 Interface

- **In:** `pnr_core::Netlist` (flat device list; FET terminals stored G,D,S,B by `parse`, frontend/library/src/parse.rs:72–93) and `AnnotationConfig` (netrole.rs:59–75). The config holds the recognition overrides, the rail/clock name overrides, `ProcessNumbers` from the deck, and one global `offset_sigma_mv`.
- **Out:** `Problem` (lib.rs:29–49):
  - `blocks: Vec<Block>`: recognised structures, then one trailing glue block. The index is both `GroupId` and the stage's symmetry `AxisId`.
  - `groups` / `abutment`: the recognition table and the diffusion-sharing permission table, both parallel to `blocks`.
  - `placement: Requirements<Layout>`, `routing: Requirements<Routes>`: three arms, hard / budget / cost.
  - `constraints: analog::Constraints`: `Unitization` and `GuardRingRequirement` for the cell generators.
  - `net_classes: Vec<NetClassification>`: class plus C and coupling budgets, per net.
  - `missing`: rule families with no deck number.
- `ir::budgets` (ir.rs:36–61) is a separate public function. The library calls it after the operating point is solved (frontend/library/src/lib.rs:283–293).

### 1.2 Pipeline as implemented (`annotate`, lib.rs:75–151)

1. **Hypergraph.** `BipartiteHypergraph::from_netlist` (kernel/core/src/hypergraph.rs:37–51): device→nets in terminal order, net→devices, kinds, terminal names, net names. Per-device W/L in nm go into `pattern::Geom` (lib.rs:77–81). A missing param reads as 0.
2. **Net roles by name only.** `netrole::classify_nets` (netrole.rs:28–48) sets Supply/Ground when the name equals one of 8 roots or starts with `root_` (18–19, 34–36). Clock is set by the substrings `clk`/`clock` or the prefixes `phi`/`ck` followed by `[0-9_b]*` (21–24, 50–55). Config names win. Everything else is Signal.
3. **Recognition.** `pattern::recognize` (pattern.rs:175–212) runs every one of 103 catalog patterns (catalog.rs:2572–2687) by **exhaustive depth-first backtracking over slot assignments**. For each slot in order it scans **every device id** (pattern.rs:154). A candidate must pass the slot predicate: FET only; same or complementary polarity to a named earlier slot; exact W and L, or equal L, versus an earlier slot; diode-connected required, forbidden or don't-care; gate net not a rail or clock (pattern.rs:91–117). It must also pass every pairwise pin-net link (`Same`/`Diff`) whose two endpoints are already assigned (pattern.rs:120–130). This is non-induced subgraph matching (monomorphism) on a pin-labelled device–net graph. Nets carry no predicates and nets outside the pattern are unconstrained. One match is kept per sorted device set, and the first slot order found wins (pattern.rs:144–151). All matches of all patterns are pooled and sorted by (priority descending, sorted device-id list ascending) (pattern.rs:194–199). A **greedy maximal disjoint set** is then selected: a match is kept only if none of its devices is taken (pattern.rs:201–210).
4. **Two-level hierarchy.** For each selected match with more than 2 devices, `recognize` is re-run restricted to that match's devices and to patterns with at most 2 slots. The results become `sub_blocks` (lib.rs:94–98). The composite's own slot semantics (which slot is the tail, the load, the cascode) are discarded. `BlockKind::from_template` (block.rs:42–56) gives a constraint-bearing kind only to 2-device matches, by substring of the template name: `diff_pair|diff_switch|cross_coupled*` → DiffPair, `*mirror*` → CurrentMirror, `*load*` → Load, `*cascode*|source_follower|series_stack` → Stack. Everything else is `Group`.
5. **Glue.** Unclaimed devices form one trailing `Glue` block (lib.rs:103–104).
6. **Tables.** `groups[i] = blocks[i].devices`. `abutment[i] = groups[i]` if every member has the same `DeviceKind`, otherwise only its first member (lib.rs:106–117).
7. **Net classes.** Devices in DiffPair/CurrentMirror/Load/**Stack** leaves are marked "sensitive" (lib.rs:121–128; block.rs:60–62). `classify::classify` (classify.rs:47–108) upgrades Signal nets structurally. A bulk-only net becomes Substrate. A net on the gate of a sensitive device, or on gates only, becomes Sensitive (classify.rs:124–131). The budgets are multiples of the gate area the net drives times the deck `gate_cap_af_um2`: Sensitive 1×/1×, Signal/Clock 2×/2× (classify.rs:26–33). A net that drives no gate gets the circuit's smallest gate load (classify.rs:87, 97), and capacitor-plate nets get no budget (classify.rs:101).
8. **Placement rules.** `emit::placement` (emit.rs:127–246): one stage per block with axis `AxisId(block index)` (emit.rs:139). Its 2-device leaves emit by kind (table 1.3). If the stage contains a DiffPair leaf, every member outside any pair gets a self-symmetric `Symmetry` and a 5 µm `Proximity` to both diff-pair devices (emit.rs:203–211). Then `emit::isolation` adds one `Isolation` rule for every (device touching a Clock-class net and not sensitive) × (sensitive device) pair, at 4 × epi, or at 4 × 2.5 µm when the deck has no epi (emit.rs:250–288).
9. **Routing rules.** `extract::routing` (extract.rs:24–106): one `Antenna` per gate net. `Differential` (hard) and `CrosstalkExclusion` come from `analog`'s own diff-pair heuristic, **not from the recognised blocks** (extract.rs:53–61; kernel/analog/src/placement/matching_pair.rs:82–95). `ParasiticBudget` and `CouplingBudget` go to every budgeted net. `Shield` goes to every Sensitive net, but only when a Clock-class net exists (extract.rs:90–104).
10. **Cell constraints.** `constraints::assemble` (constraints.rs:27–88) emits one `Unitization` per (kind, W, L) class inside every non-glue block, always with `dummy_required: true` and `route_matching_required: true` (60–62). It also gives every FET of every non-glue block a guard ring tied to its bulk, with literal tap pitch, width and resistance (66–85).
11. **Missing inputs.** Rule families lacking deck numbers are listed as unknown (lib.rs:53–71, 133–139).

### 1.3 What each leaf kind emits (emit.rs:147–238; extract.rs; constraints.rs)

| Leaf kind (2 devices, slot order a,b) | Symmetry (hard + cost, stage axis) | MatchingPair (cost; + budget if deck A_VT and S_VT) | Proximity 5 µm (budget + cost) | ThermalGradient (budget + cost) | CentroidGroup (budget + cost, one pooled group per stage) | DtiBand (hard + cost, if deck DTI) | gate nets → Sensitive |
|---|---|---|---|---|---|---|---|
| DiffPair | yes | yes | – | yes | yes | yes | yes |
| CurrentMirror | yes | yes | yes | yes | yes | yes | yes |
| Load | yes | yes | – | yes | yes | yes | yes |
| Stack | – | – | yes | – | – | – | **yes** |
| Group (any composite without 2-device children, logic, bias) | – | – | – | – | – | – | – |
| stage members outside any pair, when the stage has a DiffPair | self-symmetric | – | yes (to both DP devices) | – | – | – | – |

Every non-glue block, Group included, gets unitizations with dummies and a guard ring per FET. Glue gets nothing.

### 1.4 Structure recognition done outside the annotator

`frontend/library/src/cellgen.rs::with_per_device_sizing` (443–591) runs a second recognizer over devices that no annotator unitization covers:
- binary-weighted capacitor banks on one top plate (`dac_banks`, 760–786);
- bipolars of one kind, W, L and base net (`bjt_groups`, 717–731): the bandgap 1:N;
- MOS devices with identical terminals, W and L (`parallel_groups`, 735–750).

These become merged, interleaved cells, but they emit **no placement or routing constraints**, no net-class change, no guard rings, and no `Problem::blocks` entry. The annotator does not know they exist.

### 1.5 Algorithmic cost

- For each pattern, each slot scans all n devices (pattern.rs:154). The cost is pruned only by links to slots already assigned. `MILLER_COMP_FETS` (catalog.rs:2043–2055) has 3 `AnyFet` slots and no link between slots 0 and 1, so it enumerates n² prefixes and n³ slot checks.
- Interchangeable slots are explored in every permutation. For example, `CURRENT_MIRROR_4` has 3 equivalent output slots, so it does 3! = 6 duplicate full assignments before the dedupe.
- The dedupe is `Vec::contains` over all previous keys (pattern.rs:147), which is O(M²) in the number of matches.
- `pin_net` does a linear string search per call (pattern.rs:81–84).
- Device and net ids are `u16` (lib.rs:103; block.rs:69; classify.rs:105), so ids wrap silently past 65 535 devices.

### 1.6 How `frontend/library` consumes `Problem`

- `annotate` is called once in `performance_rows`, only to read the net classes (lib.rs:203–213), and once per `solve` (lib.rs:262–263; `solve` runs up to 2 × starts times, per AF-22).
- `problem.placement` is retargeted from device ids to cell ids (lib.rs:1145–1153). `problem.groups` becomes `layout.groups` after dp (lib.rs:599, 1154–1155). `problem.abutment` becomes `coarse.groups` for dp (lib.rs:579, 1187). `coarse.axis` is resized to `blocks.len()` (lib.rs:580–583).
- `problem.constraints` is passed to `cellgen::enumerate_folded` (lib.rs:1134). The unitizations decide which devices merge into one interleaved cell (cellgen.rs:74–149). Guard rings are deduplicated per cell (lib.rs:1159–1169).
- `problem.net_classes` feeds `gp::net_weights` (lib.rs:296), dr `supply_nets` (304–309), dr per-net weights (only nets with a budget, 313–316), `elaborate::intent` (297), the antenna-diode ground (634), the cellgen ground (1135–1136), fill keep-outs for Ground/Sensitive wires (412–420), and the IR budgets (286).
- `annotator::block::leaves` with kinds DiffPair/CurrentMirror/Load drives the post-placement `CommonNodes` (lib.rs:763–801) and `Environment` (WPE/OSE) rows (lib.rs:806–876). The IR, EM and performance rows are appended to `problem.routing` (lib.rs:267–293).
- `metadata::build` reports every hard and budget batch per family, the class census via `annotator::classify::census`, and `problem.missing` (metadata.rs:177–202). Θ sums the residuals of the budget-arm rows (metadata.rs:94–101). The certificate requires `missing` to be empty (metadata.rs:106–110).

---

## 2. Module-by-module findings

### 2.1 catalog.rs (2687 lines): the pattern library

**What it is.** 103 `const Pattern`s, all registered in `PATTERNS` (checked: the 103 definitions equal the 103 list entries). The header claims about 100 patterns derived from ALIGN templates, Razavi, Allen-Holberg, Gray-Meyer and GANA (catalog.rs:1–24). Priorities follow the size tiers stated in the header: 8 devices 50–59, 6 devices 40–49, 5 devices 30–39, 4 devices 20–29, 3 devices 14–19, 2 devices 5–13. So **any** composite outranks **every** primitive.

**Full enumeration** (sorted by priority; links written as `aP=bQ`, meaning pin P of slot a is on the same net as pin Q of slot b; `≠` means a different net; "C" means composite, i.e. `Group`, with constraints only through its ≤2-slot sub-matches). The status column marks what a trace of the code establishes.

| Pattern | line | pri | n | links | leaf kind | status |
|---|---:|---:|---:|---|---|---|
| telescopic_ota_full | 1318 | 52 | 8 | 0S=1S 0G≠1G 0D=2S 1D=3S 2G=3G 2D=4D 3D=5D 4G=5G 4S=5S 0S=6D 4S=7S | C | slot 7 is any device on the load rail (4S=7S), so it is loose; slot 7 is then forced onto the axis as a "tail" (AA-23) |
| gilbert_cell | 1252 | 44 | 6 | 0S=1S 0G≠1G 0D=2S 0D=3S 1D=4S 1D=5S 2G=3G 4G=5G 2D=5D 3D=4D | C | **wrong topology**: 2G=3G ties both switches of a quad together, as the code's own comment admits (catalog.rs:1275). It never matches a real Gilbert cell (AA-15) |
| vco_core_with_tails | 1286 | 44 | 6 | 0G=1D 0D=1G 2G=3D 2D=3G 0D=2D 1D=3D 0S=4D 1S=4D 2S=5D 3S=5D | C | children resolve to 2 `cmos_inverter`, so no pair constraints (AA-03) |
| folded_cascode_core | 1219 | 43 | 6 | 0S=1S 0G≠1G 0D=2S 1D=3S 2G=3G 4D=2S 5D=3S 4G=5G 4S=5S | C | no tail slot, so the tail lands elsewhere and the output mirror is a separate stage/axis (AA-02) |
| telescopic_ota_core | 1186 | 42 | 6 | 0S=1S 0G≠1G 0D=2S 1D=3S 2G=3G 2D=4D 3D=5D 4G=5G 4S=5S | C | |
| diff_pair_cross_coupled_load | 1128 | 35 | 5 | 0S=1S 0G≠1G 0D=2D 1D=3D 2G=3D 2D=3G 2S=3S 0S=4D | C | |
| five_transistor_ota | 1071 | 34 | 5 | 0S=1S 0G≠1G 0D=2D 1D=3D 2G=3G 2S=3S 0S=4D | C | tested (tests.rs:369–379) |
| diff_pair_cascode_load | 1101 | 33 | 5 | 0S=1S 0G≠1G 0D=2S 1D=3S 2G=3G 0S=4D | C | a folded cascode with tail |
| cascode_mirror_ota | 1955 | 33 | 5 | identical to diff_pair_cascode_load | C | **dead**: exact duplicate at the same priority and listed later (catalog.rs:2585–2586) |
| ota_self_biased_load | 1921 | 32 | 5 | 0S=1S 0G≠1G 0D≠1D 0D=2D 1D=3D 2G=3G 2S=3S 0S=4D | C | the orphan doc at 1911–1917 describes a different pattern |
| cascoded_diff_pair_with_tail | 2106 | 32 | 5 | 0S=1S 0G≠1G 0D=2S 1D=3S 2G=3G 2D≠3D 0S=4D | C | also matches half of a real Gilbert cell (AA-02) |
| wide_swing_cascode_mirror_5 | 1158 | 30 | 5 | 0G=1G 0S=1S 0D=2S 1D=3S 2G=3G 4G=0G 4S=0S | C | |
| diff_output_stage | 2135 | 30 | 5 | 0S=1S 2S=3S 0G=1G 2G=3G 0G≠2G 0D≠1D 0S≠2S | C | |
| cross_coupled_inverters | 907 | 26 | 4 | 0D=2D 1D=3D 0G=2G 1G=3G 0D=1G 1D=0G | C | children resolve to 2 `cmos_inverter` (Group), so **a latch gets no symmetry** (AA-03) |
| wide_swing_cascode_mirror | 755 | 25 | 4 | 0G=1G 0S=1S 0D=2S 1D=3S 2G=3G 2D=0G | C | |
| improved_wilson_mirror_4 | 823 | 25 | 4 | same links as wide_swing_cascode_mirror, slot 0 not required to be a diode | C | |
| cross_coupled_complementary | 929 | 25 | 4 | 0G=1D 0D=1G 0S=1S 2G=3D 2D=3G 2S=3S 0D=2D 1D=3D | C | **dead**: its links imply cross_coupled_inverters (26) |
| cascode_mirror | 731 | 24 | 4 | 0G=1G 0S=1S 0D=2S 1D=3S 2G=3G | C | children are 2 CurrentMirror; no bottom–top proximity (survey Fig.3.11 has one, L5217–5222) |
| wilson_mirror_4 | 802 | 24 | 4 | 0S=1S 0D=2S 1D=3S 0G=3D 1G=2D | C | cross-coupled gate feedback; not a standard 4T Wilson (unverified against a source; not in ref/) |
| regulated_cascode_mirror | 1700 | 24 | 4 | 0G=1G 0S=1S 1D=2S 1D=3G 3D=2G | C | |
| low_voltage_cascode_mirror | 779 | 23 | 4 | 0G=1G 0S=1S 0D=2S 1D=3S 2G=3G | C | |
| diff_pair_with_mirror_load | 872 | 23 | 4 | 0S=1S 0G≠1G 0D=2D 1D=3D 2G=3G 2S=3S | C | |
| complementary_diff_pair | 1983 | 23 | 4 | 0S=1S 0G≠1G 0D≠1D 2S=3S 0G=2G 1G=3G 2D≠3D | C | children depend on netlist order (AA-07) |
| inverter_buffer | 2465 | 23 | 4 | 0G=1G 0D=1D 2G=3G 2D=3D 0D=2G | C | |
| diff_pair_with_active_load | 846 | 22 | 4 | 0S=1S 0G≠1G 0D=2D 1D=3D 2G=3G | C | |
| cascode_pair | 956 | 22 | 4 | 0G=1G 0S=1S 0D=2S 1D=3S 2G=3G 2D≠3D | C | |
| charge_pump_cell | 1022 | 22 | 4 | 0D=2D 0S=1D 2S=3D 0G≠2G | C | generic: any 2+2 complementary stack sharing an output, e.g. a cascoded output branch (AA-06) |
| diff_pair_with_cascodes | 1649 | 22 | 4 | 0S=1S 0G≠1G 0D=2S 1D=3S 2G=3G 2D≠3D | C | |
| dac_current_cell | 1742 | 22 | 4 | 0S=1S 0G≠1G 0D≠1D 0S=3D 3S=2D | C | unit cell only; no array detection |
| schmitt_trigger | 1790 | 22 | 4 | 0G=2G 0D=1S 2D=3S 1D=3D | C | generic stacked-gate shape |
| nand_cross_coupled | 2064 | 22 | 4 | 0D=1S 1D=2D 1D=3D 2S=3S 0G=2G | C | logic |
| nand_gate | 2368 | 22 | 4 | 0D=1S 1D=2D 1D=3D 0G=2G 1G=3G 2S=3S | C | logic |
| nor_gate | 2390 | 22 | 4 | 0S=1S 0D=1D 2D=3S 3D=0D 0G=2G 1G=3G | C | logic |
| folded_load_pair | 2440 | 22 | 4 | 0S=1S 0G≠1G 0D=2S 1D=3S 2G=3G | C | |
| cascode_pair_symmetric | 978 | 21 | 4 | 0S=1S 0D=2S 1D=3S 2G=3G 0G≠2G | C | |
| mirror_with_dual_output | 1676 | 21 | 4 | 0G=1G 0G=2G 0S=1S 0S=2S 0D≠1D 0D≠2D 1D≠2D 1D=3S | C | |
| transmission_gate_pair | 1720 | 21 | 4 | 0D=1D 0S=1S 0G≠1G 2D=3D 2S=3S 2G≠3G | C | |
| diff_pair_with_split_cascodes | 2413 | 21 | 4 | 0S=1S 0G≠1G 0D=2S 1D=3S 2G≠3G 2D≠3D | C | also swallows the StrongARM NMOS cross pair (AA-03) |
| cascode_current_source_pair | 2486 | 21 | 4 | 0G=1G 0S=1S 0D=2S 1D=3S 2G=3G 2D≠3D | C | |
| series_stack_4 | 452 | 20 | 4 | 0D=1S 1D=2S 2D=3S 0G=1G 0G=2G 0G=3G | C | tested (tests.rs:382–393) |
| level_shifter | 1001 | 20 | 4 | 0S=1D 0D=2D 2S=3D | C | the doc says slot 2 is a "diode load" but the slot has no diode requirement; only 3 links |
| current_mirror_4 | 1042 | 20 | 4 | shared G and S with slot 0, ≠D | C | more than 3 outputs fragment the mirror (AA-01) |
| feedback_pair | 1768 | 20 | 4 | 0S=1S 0G≠1G 0D≠1D 0D=2G 1D=3G 2S=3S | C | also matches a cascade of two stages |
| bandgap_mirror_pair | 1883 | 20 | 4 | 0G=1G 0S=1S 2G=3G 2S=3S 0D=2D 1D=3D | C | FETs only; the BJTs and resistors of a bandgap are invisible (AA-04) |
| diff_pair_with_tail | 558 | 18 | 3 | 0S=1S 0G≠1G 0D≠1D 0S=2D | C | |
| improved_wilson_mirror | 512 | 17 | 3 | 0G=1G 0S=1S 0D=2S 2D=0G | C | not a standard 3-transistor topology (the improved Wilson has 4 devices); no source given |
| regulated_cascode | 612 | 17 | 3 | 0D=1S 0D=2G 2D=1G | C | **matches a real 3T Wilson mirror** (see wilson_mirror) |
| wilson_mirror | 492 | 16 | 3 | 0G=1G 0S=1S 1D=2S 0D=2G | C | **wrong topology** (AA-15) |
| flipped_voltage_follower | 674 | 16 | 3 | 1D=0S 0S=2G 2D=1G | C | |
| self_biased_cascode | 1547 | 16 | 3 | 0D=1S 0D=1G 0G=2G 0S=2S | C | |
| triple_cascode | 2278 | 16 | 3 | 0D=1S 1D=2S 0G≠1G 1G≠2G | C | |
| super_source_follower | 2326 | 16 | 3 | 1D=0S 0S=2S 2D=1G | C | |
| diff_pair_diode_load | 581 | 15 | 3 | 0S=1S 0G≠1G 0D≠1D 0D=2D | C | |
| cascode_with_degeneration | 630 | 15 | 3 | 0D=1S 2D=0S 0G≠1G 0G≠2G | C | |
| source_follower_with_mirror | 651 | 15 | 3 | 0S=2D 1G=2G 1S=2S | C | |
| cascode_with_mirror_bias | 1511 | 15 | 3 | 0D=1S 2D=1G 2S=0S | C | |
| mirror_with_cascode_output | 1529 | 15 | 3 | 0G=1G 0S=1S 1D=2S | C | |
| sooch_mirror | 1567 | 15 | 3 | 0G=1G 0S=1S 1D=2S 0D≠2G 1D≠2G | C | |
| inverter_with_tail | 1611 | 15 | 3 | 0G=1G 0D=1D 0S=2D | C | |
| cmfb_sense_pair | 2021 | 15 | 3 | 0D=1D 0D=2D 0S=1S 0S=2S 0G≠1G | C | narrow; common CMFB topologies are absent (table 2.1b) |
| bootstrapped_switch | 2084 | 15 | 3 | 0D=1D 0S=1S 2D=0G | C | |
| cascoded_reference | 2346 | 15 | 3 | 0G=1G 0S=1S 0D=2S | C | its doc begins "…actually let's do a useful one" (2341–2345) |
| active_loaded_inverter | 2522 | 15 | 3 | 0G=1G 0D=1D 2D=1S | C | |
| diff_pair_with_reference | 2539 | 15 | 3 | 0S=1S 0G≠1G 0D≠1D 1G=2D | C | |
| current_mirror_3 | 535 | 14 | 3 | shared G and S, all D different | C | |
| active_load_3 | 693 | 14 | 3 | same links as current_mirror_3, no diode, exact size | C | |
| diff_pair_with_degen | 1588 | 14 | 3 | 0G≠1G 0D≠1D 0S≠1S 0S=2D | C | only one degeneration device |
| push_pull_with_bias | 1628 | 14 | 3 | 0D=1D 0S≠1S 2D=0G | C | **loose**: consumes the three-stage opamp's M6/M8/M9 (§2.13) |
| bias_chain_3 | 1833 | 14 | 3 | 0D=1S 1D=2S | C | |
| miller_comp_fets | 2043 | 14 | 3 | 0D=2S 1D=2G | C | **wildcard**: 3 AnyFet slots, 2 links, O(n³) (AA-06, AA-22) |
| current_mirror_1_to_2 | 2305 | 14 | 3 | shared G and S, ≠D vs ref | C | orphan doc about a "cascode mirror with diode bias" at 2294–2299 |
| cmos_inverter | 365 | 11 | 2 | 0G=1G 0D=1D | Group | **shadows cross_coupled (9)** inside every latch (AA-03) |
| transmission_gate | 381 | 11 | 2 | 0D=1D 0S=1S 0G≠1G | Group | |
| diff_pair | 118 | 10 | 2 | 0S=1S 0G≠1G 0D≠1D 0G≠1D 1G≠0D | DiffPair | shared source may be a rail (AA-05); ties with push_pull_pair at 10 (AA-07) |
| push_pull_pair | 397 | 10 | 2 | 0G=1G 0D≠1D 0S≠1S | Group | |
| complementary_source_follower | 1463 | 10 | 2 | 0S=1S 0G≠1G 0D≠1D | Group | |
| esd_diode_clamp | 1814 | 10 | 2 | 0D=1S | Group | misnomer: any two stacked diode-connected FETs, e.g. a bias stack |
| anti_parallel_switch | 2191 | 10 | 2 | 0D=1S 0S=1D 0G≠1G | Group | |
| diff_pair_split_source | 143 | 9 | 2 | 0G≠1G 0D≠1D 0S≠1S | DiffPair | **false-positive generator** (AA-05) |
| cross_coupled | 230 | 9 | 2 | 0G=1D 0D=1G 0S=1S | DiffPair | tested (tests.rs:79–92) |
| cross_coupled_split_source | 247 | 9 | 2 | 0G=1D 0D=1G 0S≠1S | DiffPair | |
| diode_cascode | 1398 | 9 | 2 | 0D=1S (both diodes, same L) | Stack | **dead**: ⊂ esd_diode_clamp (10) |
| beta_multiplier_core | 1442 | 9 | 2 | 0G=1G 0S≠1S 0D≠1D | Group | the K:1 pair gets **no matching** |
| cascode_diode_top | 2172 | 9 | 2 | 0D=1S | Stack | |
| current_mirror | 167 | 8 | 2 | 0G=1G 0S=1S (slot 0 diode, same L) | CurrentMirror | tested (tests.rs:63–77) |
| cascode_matched | 281 | 8 | 2 | 0D=1S 0G≠1G | Stack | |
| diode_load_pair | 320 | 8 | 2 | 0S=1S 0D≠1D | Load | |
| source_follower | 343 | 8 | 2 | 0D=1S 0G≠1G | Stack | |
| latch_half | 416 | 8 | 2 | 0D=1G 0S≠1S 0D≠1D | Group | |
| current_mirror_single_ended | 1419 | 8 | 2 | 0G=1G 0S=1S 0D≠1D | CurrentMirror | **dead**: ⊂ current_mirror, same priority, listed later |
| diff_switch | 2228 | 8 | 2 | 0S=1S 0G≠1G 0D≠1D 0G≠1D 1G≠0D | DiffPair | `gate_is_signal: false` means "don't check", not "non-signal" (the doc at 2223–2227 is wrong); accepts a rail source |
| current_mirror_output_pair | 183 | 7 | 2 | 0G=1G 0S=1S 0D≠1D | CurrentMirror | labels externally biased current sources (incl. clocked precharge switches) as a mirror |
| cascode | 266 | 7 | 2 | 0D=1S 0G≠1G | Stack | |
| series_stack | 436 | 7 | 2 | 0D=1S 0G=1G | Stack | |
| startup_mirror | 1861 | 7 | 2 | 0G=1G 0S=1S 0D≠1D | CurrentMirror | **dead**: ⊂ current_mirror (8) |
| mirror_pair_split_source | 206 | 6 | 2 | 0G=1G 0S≠1S 0D≠1D | CurrentMirror | labels any shared-gate same-L pair with distinct S and D (cascode pairs, LO switch pairs) as a mirror |
| active_load | 299 | 6 | 2 | 0G=1G 0S=1S 0D≠1D | Load | **dead**: ⊂ current_mirror_output_pair (7) |
| common_gate_pair | 1365 | 6 | 2 | 0G=1G 0D=1D 0S≠1S | Group | |
| common_mode_sense_pair | 1485 | 6 | 2 | 0G=1G 0S≠1S 0D=1D | Group | |
| triode_load_pair | 2208 | 6 | 2 | 0G=1G 0S=1S 0G=0S 0D≠1D | Load | **dead** (⊂ current_mirror_output_pair / current_mirror) and **electrically wrong**: gate tied to source is an off device, not a triode resistor (doc at 2205–2207) |
| degeneration_pair | 469 | 5 | 2 | 0D=1D 0G≠1G 0S≠1S | Group | |
| switch_pair | 1382 | 5 | 2 | 0D=1D 0S=1S 0G≠1G | Group | |
| dummy_pair | 2252 | 5 | 2 | 0S=1S 0D=1D 0G=1G | Group | **dead**: ⊂ current_mirror (8) |

**Why a pattern is "dead".** Assume pattern Q is a superset of P: every slot and link of P implies Q's on the same device set, and Q either has a higher priority or has the same priority and comes earlier in `PATTERNS`. Then Q's match of the same device set always sorts first (the sort is stable, pattern.rs:199), and P's match is rejected as overlapping (pattern.rs:203). Eight patterns are dead this way: cascode_mirror_ota, cross_coupled_complementary, current_mirror_single_ended, startup_mirror, active_load, triode_load_pair, diode_cascode, dummy_pair.

**Coverage versus the building-block library an expert expects** (table 2.1b; hand traces through `recognize` and `emit`):

| Building block | Catalog entry | What actually happens | Constraints that reach placement |
|---|---|---|---|
| Diff pair with tail | diff_pair, diff_pair_with_tail, OTA composites | Recognised. Tail placed on the axis with a 5 µm pull | Sym, MP, Thermal, CC, DTI; tail self-symmetric |
| Cross-coupled pair (2T) | cross_coupled(_split_source) | Recognised when isolated | as a diff pair |
| Cross-coupled complementary latch / VCO core | cross_coupled_inverters, vco_core_with_tails | Children resolve to inverters | **none** (AA-03) |
| Simple mirror 1:1 | current_mirror | Recognised | Sym, MP, Prox, Thermal, CC, DTI |
| Ratioed mirror by W | current_mirror (same L only) | Recognised, but unequal devices are mirror-symmetric and not unitized | Symmetry of unequal devices (AA-24, AA-09) |
| Multi-output mirror (>3 outputs) | current_mirror_3/_4/_1_to_2, mirror_with_dual_output | Split: ref + 3 outputs in one block, the rest paired elsewhere on other axes | fragmented (AA-01) |
| Cascode / wide-swing / low-voltage mirror | cascode_mirror, wide_swing_*, low_voltage_* | Composite; two mirror leaves | Sym/MP per level; no vertical proximity |
| Wilson (3T) | wilson_mirror (wrong) | Matches regulated_cascode; mirror leaf found | mirror pair only |
| Cascode stack | cascode, cascode_matched, source_follower, series_stack(_4), triple_cascode | Stack | Proximity 5 µm only |
| Folded cascode | folded_cascode_core, diff_pair_cascode_load, folded_load_pair | Core recognised; tail and output mirror in other blocks with their own axes | per-block axes (AA-02) |
| Telescopic | telescopic_ota_core/_full | Recognised | slot 7 (load bias) forced onto the axis |
| CMFB | cmfb_sense_pair, common_mode_sense_pair | Only one narrow shape; two-pair or resistive-averaging CMFB not modelled | none |
| Comparator (StrongARM) | diff_pair_with_tail + cross_coupled_inverters + current_mirror_output_pair | Input pair OK. The latch gives inverters; the precharge switches are labelled a "mirror". The clocked tail gets both Proximity ≤5 µm and Isolation ≥10 µm | latch unconstrained; contradictory rules (AA-13) |
| Bias generators (β-multiplier, bias chains) | beta_multiplier_core, bias_chain_3, esd_diode_clamp, self_biased_cascode, cascoded_reference | Group | none; the resistor is ignored |
| Bandgap core (1:N BJT) | bandgap_mirror_pair (FETs only) | BJTs are glue in the annotator. cellgen merges same-base BJTs into one array cell | none at placement (AA-04) |
| Level shifter | level_shifter (generic 4T) | Generic shape match | none |
| Resistor ladder / divider / R ratio | – | glue | none |
| Capacitor DAC array | – (cellgen `dac_banks`, binary only) | one merged cell; split/C-2C/unary not handled | none at placement |
| Switched-capacitor switches | transmission_gate, bootstrapped_switch, switch_pair | Group. Equal-size switches with gates `p1`/`p2` become a **DiffPair** via diff_pair_split_source | spurious Sym/MP (AA-05) |
| LC tank | – (inductor ignored); cross_coupled | The NMOS-only cross pair is recognised; the inductor is not related to it | pair only |
| Current-steering DAC cell | dac_current_cell | Unit cell recognised; no array detection | per-cell axis |
| Tail current source | inside DP composites | Taken by the DP composite; its bias-mirror relation to the reference is lost | tail self-symmetric only (AA-01) |

**Other catalog issues.**
- There are no net predicates: a slot cannot require that a net be a rail, not a rail, internal, a port, or of a given degree.
- There are no non-FET slots (pattern.rs:94). The bulk terminal and the model name are never compared (AA-20).
- `ExactAs` demands W and L equal to the nanometre (pattern.rs:106), so intentionally trimmed or rounded pairs fall out of every composite.
- Doc debris: 1911–1917 (a doc for a deleted "current_mirror_ota" attached to OTA_SELF_BIASED_LOAD), 2294–2299 (a doc attached to CURRENT_MIRROR_1_TO_2), 2341–2345 ("actually let's do a useful one").
- Tests: no pattern is tested in isolation. Only 5 of 103 patterns are exercised through `annotate` (AA-26).

### 2.2 pattern.rs (212): the DSL and the matcher

- **Algorithm.** See §1.2 step 3. Correct for what it claims. It is deterministic because every iteration is in index order.
- **Problems.**
  - Greedy disjoint selection (pattern.rs:201–210): a device can belong to exactly one structure (AA-01).
  - Ties are broken by the lexicographic device-id list (pattern.rs:199), and the stored slot order is the first one found (pattern.rs:145–150). Both depend on the order of devices in the netlist (AA-07).
  - Candidates come from all devices rather than from the nets of already-assigned slots, even though `hg.net_devices` exists (pattern.rs:154). Permutations of interchangeable slots are explored. The dedupe is O(M²) (pattern.rs:147) (AA-22).
  - `gate_ok` treats "not Supply/Ground/Clock by name" as a signal (pattern.rs:114–115). A bias net therefore counts as a signal gate, and a diff pair whose reference input is named, for example, `vdd_half` is rejected.
  - `slot_ok` indexes `assigned[r]` without checking that `r` < the slot index (pattern.rs:97–103). A forward reference in a future pattern panics. A link naming a slot ≥ n is silently never checked (pattern.rs:121–122) (AA-34).
  - `Geom` has only W and L (pattern.rs:67–71). `nf`, `m`, `nfin` and the model are not compared (AA-20, AA-21).
- **Tests:** none local. It is covered indirectly by tests.rs.

### 2.3 lib.rs (166): `annotate` and `Problem`

- **Correct:** determinism; every device accounted for (tested, tests.rs:310–323); the abutment table never empty (tests.rs:268–286).
- **Problems.**
  - Children of a composite are re-recognised with the ≤2-slot catalog (lib.rs:94–98). The composite's slot roles are thrown away and intermediate levels (for example a cascode mirror inside a telescopic OTA) are lost (AA-08).
  - `abutment` cuts a mixed-polarity group to its first member (lib.rs:113–114). This drops the legitimate same-polarity sharing inside it, such as the 5T OTA tail and diff pair sharing `vtail` (AA-19).
  - `sensitive` includes Stack leaves (lib.rs:121–128 via block.rs:60–62) (AA-17).
  - `missing()` lists MatchingPair even when the circuit has no matched pair (lib.rs:55–60) (AA-29).
  - `(0..n as u16)` truncates silently past 65 535 devices (lib.rs:103) (AA-35).
  - `gate_um2` multiplies W·L by `max(nf, m)` (lib.rs:155–161). Under the BSIM total-W convention this over-counts area by `nf` (the convention conflict is AF-01).
  - No op point, spec or sensitivity reaches `annotate`: its only inputs are the netlist and the config (lib.rs:75) (AA-12).
- **Test coverage:** tests.rs, see §2.11.

### 2.4 block.rs (90)

- `Block` stores `kind`, `devices`, `injected` and `sub_blocks`. **The template name is dropped** in `from_match` (block.rs:66–73), so no report, debugger or user can see what was recognised (AA-27).
- `from_template` maps by name substring (block.rs:42–56). Every composite is `Group`, and constraint semantics exist only at 2 devices.
- `is_sensitive` includes `Stack` (block.rs:60–62). A cascode or series-stack gate net therefore becomes Sensitive: 8× crosstalk spacing (extract.rs:122), shields whenever a clock exists (extract.rs:92–103), and victim status for `Isolation` (emit.rs:278). This contradicts block.rs:30 ("adjacent, not matched") (AA-17).
- `injected` is never set in the annotator. The doc says the orchestrator sets it (block.rs:13–15); no code in frontend/library writes `.injected` (checked by grep). The field is dead.
- **Tests:** none local.

### 2.5 emit.rs (323): placement rules

- **What it emits:** table 1.3. The Pelgrom helper (emit.rs:69–103) is sound: σ_rand = A_VT/√(WL); η from a budget taken in quadrature; ΔT limit = η·σ_rand/TC (tested, emit.rs:298–322).
- **Problems.**
  - One axis per block (emit.rs:139). A differential signal path spread over several blocks gets independent axes (AA-02).
  - The pooled `CentroidGroup` a/b sides come from slot order, not from the electrical half (emit.rs:196–197, 216–221). If the load pair's lower-index device is on the other half from the diff pair's slot 0, the pooled sides mix halves and the coincidence test becomes trivially true. This compounds AR-02 (AA-07).
  - Every stage member outside a pair is made self-symmetric and pulled within 5 µm of the diff pair (emit.rs:203–211). This includes non-tail members such as `telescopic_ota_full` slot 7. A cascoded tail that was recognised as a Stack pair gets neither (AA-23).
  - Mirror leaves with unequal W get the mirror `Symmetry` of a matched couple and `MatchingPair` with `min(gate)` (emit.rs:158–167). Lampaert §4.2 asks for ratioed matching groups built from equal units (lampaert.txt 3337–3350, p.84) (AA-24).
  - `MatchingPair` uses the ΔVT (voltage) model for current mirrors and loads. Hastings separates voltage-matched from current-matched devices (hastings.txt 42326–42328, p.712) (AA-10).
  - Literals: `PROXIMITY_NM` = 5000 (emit.rs:45), `THERMAL_MAX_DELTA_MC` = 500 (42), `GRADIENT_SHARE` = 0.3 (52), `NOMINAL_EPI_NM` = 2500 (256), `ISOLATION_EPI_MULTIPLE` = 4 (250) (AA-28).
  - `isolation` has three problems (emit.rs:263–288). Aggressor = any device touching a Clock net at any terminal and not in a leaf, with no strength. The rules are all-pairs, O(A·V). A clocked diff-pair tail is an aggressor against the very pair it must sit next to, which contradicts emit.rs:207 (AA-13, AA-33).
  - No emission for: orientation equality; same variant; dummies by class; aspect ratio; power-device keep-away (Hastings rules 7, 9, 12, 14, hastings.txt 42490–42569, pp.714–715); well grouping by bulk net; minimum distance between wells (survey eq.(3.16), balasa_graeb_survey.txt 4742–4775, p.114).
- **Tests:** one local Pelgrom test. tests.rs checks arm partitions and counts, not emission per structure (AA-26).

### 2.6 extract.rs (127): routing rules

- **Correct.** Antenna gate area is on terminal 0 (G) (extract.rs:33–39; tested tests.rs:413–427). Parasitic and coupling budgets report an overshoot, not a count (tested tests.rs:205–233). Shields are only requested against a clock (tested tests.rs:429–449).
- **Problems.**
  - `Differential` (hard) and `CrosstalkExclusion` come from `analog`'s `is_diff_pair` (extract.rs:53, 57), not from the recognised blocks. `is_diff_pair` has no size match and uses its own rail list, `vdd vss vcc vee vpwr vgnd vpb vnb avdd avss dvdd dvss` as prefix or `gnd` anywhere (kernel/analog/src/placement/matching_pair.rs:82–106). That list differs from netrole.rs:18–19 and ignores `AnnotationConfig`. Consequence (hand trace): with the SPICE ground `0` (Signal to both lists), every pair of same-type FETs with sources on `0` and distinct gates and drains gets a **hard** 5 % length-matching `Differential`. For the four NMOS inverter devices of dac4 that would be 6 rules. Conversely, a pseudo-differential pair with sources on VSS is a `DiffPair` block but gets no `Differential` routing (AA-14). AR-12 already notes that `Differential` should not be hard.
  - Literals: antenna margin 20 % (extract.rs:47), 5 % differential (differential.rs:125), spacing multiples 8/7/3/1 (extract.rs:120–127), margins 35/30/25/20 % (110–117), shield coverage 80 % and gap 2 × space (98–100) (AA-28).
  - Shield reference = the first Ground-class net by id (extract.rs:91). With separate analog and digital grounds it can pick the noisy one (AA-31; NOTES-45).
  - An empty antenna batch is pushed when the deck has no ratio (extract.rs:40–52). This is harmless: metadata skips zero-count batches (metadata.rs:132–134).
- **Tests:** none local. See tests.rs.

### 2.7 classify.rs (205): net classes and budgets

- **Correct:** the classification rule is isolated and tested (classify.rs:156–205). Capacitor plates stay unbudgeted and say so (tested tests.rs:529–557).
- **Problems.**
  - **Positional terminal roles for every non-capacitor device** (classify.rs:11–15, 62–85). Slot 0 is treated as the gate and slots 1–2 as the channel. That holds for FETs only. For a BJT (C,B,E; parse.rs:215) the collector is taken as a "gate". For resistors, diodes and inductors (P,N) the P side is. Hand trace on bjt_mirror: `outn` and `outp` (collector-only nets) classify as **Sensitive** (gate-only rule, classify.rs:127), while the base net `in` classifies as Signal (AA-16).
  - Digital inputs that drive only gates are classified Sensitive (the "bias rail" rule, classify.rs:127). Hand trace on dac4: `d0`–`d3` are Sensitive (AA-11).
  - A drain-only net is budgeted against the circuit's **smallest** gate load (classify.rs:87, 97). An OTA output driving an off-chip load is then budgeted like an inverter input (AA-25).
  - Sensitive budgets are 1× the load for wire and 1× for coupling. The code's own ponytail says precision targets are 10 %/2 % (classify.rs:22–25). Clock nets are budgeted like Signal (2×).
  - No Bias, Reference, Digital, Output or high-impedance class. The class has no strength or activity fields (see AR-14).
- **Tests:** 5 local.

### 2.8 netrole.rs (121): names and config

- **Rails** (netrole.rs:18–19, 34–36) require an exact name or a `root_` prefix. Missed: `0` (SPICE ground), `gnd!`, `vdd!`, `vee`, `vdd1`, `avdd3v3`, `vcca`, and sky130 `VPB`/`VNB`. The last two end up Substrate only if they are bulk-only. Nothing is inferred from structure (bulk ties of one polarity), from ports (the parser drops `.subckt` ports, parse.rs:35–39), or from DC voltage.
- **Clocks** (netrole.rs:21–24, 50–55): only `*clk*`, `*clock*`, `phi[0-9_b]*` and `ck[0-9_b]*`. Missed: `p1`/`p2`, `s1`, `q1`, `sample`, `en`, `rst`, `latch`, `trig`. There is no Digital class, so switching data lines are not aggressors.
- `AnnotationConfig` claims "ALIGN DoNotIdentify / SameTemplate semantics" and "The library resolves user device names to ids" (netrole.rs:57–63). SameTemplate does not exist. **No code outside the annotator sets any field**: `do_not_identify`, `do_not_use`, `supply_nets`, `ground_nets`, `clock_nets` and `offset_sigma_mv` are only defaulted (grep over frontend, benchmarks, examples and backend). `library::Config::annotation` exists (lib.rs:43–44) but neither the CLI nor the benchmark sets it (benchmarks/src/bench.rs:269 re-annotates with `default()`). The overrides only suppress: there is no way to *add* a symmetry pair, a matching group or a precision class (AA-11).
- `ProcessNumbers` (netrole.rs:79–108) is complete for what emit and extract read. On the shipped decks, `gate_cap_af_um2` exists only in sky130, so no net budgets exist on gf180, ihp or finfet. `epi_thickness_nm` is in no deck, so `Isolation` is always cost-only (pdks/*.json checked).
- **Tests:** clock names only (netrole.rs:111–121).

### 2.9 constraints.rs (88): cell tier

- **Correct:** a unitization never mixes kind, W or L (tested tests.rs:326–345). Rings tie to the device's bulk (tested tests.rs:347–366).
- **Problems.**
  - **No ratio inference** (constraints.rs:30–63). The class key is (kind, W, L), so a 1:4 mirror written as W = 1 µm and W = 4 µm becomes two single-device classes: never interleaved, never common-centroid. `target_ratio` is simply `dev_nf` (52), and only `m`/`nf` ratios survive. Hastings rule 1 (hastings.txt 42360–42368, p.712) and Lampaert §4.2 (lampaert.txt 3346–3348, p.84) both require ratioed devices built from identical units (AA-09).
  - The class key ignores `model` (flavour, oxide) and bulk. cellgen draws a merged cell with `members[0]`'s model (frontend/library/src/cellgen.rs:122, 129), so two same-W/L devices of different models in one block are drawn as one model, an LVS mismatch (AA-20).
  - `dummy_required: true` and `route_matching_required: true` for every class of every non-glue block, including logic (`cmos_inverter`, `nand_gate`) and single-device classes (constraints.rs:61–62). Both are read by the MOS generator (kernel/cells/src/mosfet.rs:74, 103). The result is dummies on digital inverters and none on unrecognised analog devices (AA-18).
  - Guard ring for every FET of every non-glue block, typed by polarity only (constraints.rs:66–85). The literals are tap pitch 2000 nm, width 500 nm and 100 Ω (79–81). The enum docs say `Ecgr` = "n+ in p-sub" and `Hcgr` = "p+ in n-well" (kernel/analog/src/cell.rs:26–35), but `post_cell` draws the opposite, a majority-carrier tap (kernel/cells/src/post_cell.rs:310–323). The ring encodes no role (victim or aggressor; LAYOUT-FUNDAMENTALS row 58) (AA-18).
  - `fingers` ignores `nfin` and `multi`, both of which `parse` keeps (parse.rs:100) (AA-21).
- **Tests:** via tests.rs only.

### 2.10 ir.rs (87, untracked): IR-drop budgets

- `R_max = ΔV/I`. ΔV is 10 % of the smallest saturation headroom on the net, or else 1 % of the supply. A signal net is budgeted when it carries ≥10 % of the busiest net's current (ir.rs:14–61; tested ir.rs:71–86). Headroom is filtered to positive values upstream (frontend/library/src/oppoint.rs:67).
- **Problems:** three policy literals, labelled ponytail (ir.rs:18, 23, 27). A single `supply_mv` for every rail (the library passes the op VDD or 1800 mV, lib.rs:284). This is the only annotator code that consumes the operating point, and it lives outside `annotate` (AA-12).

### 2.11 tests.rs (557): coverage

24 integration tests. Recognition is tested only for:
- the 5T OTA diff pair being in one block (52–61, 127–135);
- a 2T mirror (63–77);
- a 2T cross-coupled pair (79–92);
- `do_not_identify` (94–104);
- a 4-stack (381–393);
- a 2T cascode (395–411).

The other 18 tests check arm partitions, counts, budgets, antenna indexing, shields, isolation, capacitor plates and rings.

Missing:
- false-positive tests: two unrelated identical FETs must not become a DiffPair;
- netlist-permutation invariance (the handbook's own validation case, ref-00 L164–171);
- latch, comparator, folded cascode, Gilbert, bandgap and three-stage opamp;
- scale;
- per-structure emission (which rule kinds per kind);
- class assertions on a real fixture.

Stale references: tests.rs:144 cites `backend/TODO.md` (does not exist) and tests.rs:196 cites `emit::budget_and_cost` (does not exist). tests.rs:302–309 are 8 blank lines (AA-26, AA-30).

### 2.12 Consumers: frontend/library/src/lib.rs and metadata.rs

- `library::annotation` (lib.rs:492–516) fills `ProcessNumbers` from the deck and leaks the stack once per call (documented; AF-22).
- `common_nodes` (lib.rs:763–801) re-implements the gate area W·L·max(nf, m) at 789–792, a third copy after annotator lib.rs:155–161 and constraints.rs:22–24. It takes A_VT from member a only.
- `environment` (lib.rs:806–876) hard-codes the deck's `wpe_clearance_moderate` and `lod_moat_ext_moderate` for every pair (871–872). Where a precision class should select the threshold, the "moderate" class is assumed for all (AA-10, AA-32).
- `problem.blocks.len()` sizes `coarse.axis` (lib.rs:580–583). With one axis per block, the glue block also owns an axis slot that is never used.
- `metadata::build` (metadata.rs:177–202) reports families and the census but not recognition coverage: no count of glue devices, no list of templates matched (AA-27; NOTES-03).
- `metadata::certified` blocks on any `missing` entry (metadata.rs:106–110), and `missing` is not relevance-conditioned (AA-29).
- The whole Problem is recomputed per `solve` (AF-22). Nothing in the Problem depends on the solve.

### 2.13 Hand traces on repository netlists (by reading the code; not executed)

1. **tt_ota / ota_constrained / tests `ota()`.** `five_transistor_ota` claims all 5. Children: `diff_pair` (XM1, XM2) and `current_mirror_output_pair` (XM3, XM4) → CurrentMirror, beating `active_load` (7 > 6). XM5 is the tail. Matches tests.rs:369–379.
2. **examples/three_stage_opamp (main.rs:21–32).** Ids: M1=0 … M9=8.
   - `five_transistor_ota` claims {M1, M2, M4, M5, M3}.
   - `push_pull_with_bias` (14) then claims {M8, M9, M6}. The cheaper-key match {M6, M7, M5} is rejected because M5 is taken. The block's children: no 2-device pattern fits, so it is a 3-device Group leaf and emits nothing. M6, a gain device, is labelled as a bias device.
   - M7 → glue.
   - M3, M7 and M9 share `vbias` and `vss` (the tail and the two stage current sources). No matched group ties them, so their ratio relation is lost. M7 gets no ring and no dummies; M6/M8/M9 get dummies and rings.
3. **dac4.** Four `cmos_inverter` blocks (Group), with 8 single-device unitizations with dummies and 8 rings. XMC → glue. Nets `d0`–`d3` → **Sensitive**. The capacitors → cellgen `dac_banks` (one merged cell, no placement constraint).
4. **bgr_core / bjt_mirror.** No FET, so everything is glue. bgr_core: cellgen groups XQ1 and XQ2 (same base VSS) into a 1:8 cell. bjt_mirror: collector nets → Sensitive (§2.7).
5. **pair / quad.** Parallel identical devices: no pattern matches (every pattern needs at least one `≠` or a diode). Glue; cellgen `parallel_groups` merges them.
6. **chain4.** `series_stack_4`. Children: series_stack (0,1) and (2,3) → Stack. There is no Proximity for the (1,2) link. cellgen chains the four anyway.
7. **Constructed cases** (for regression tests):
   - (a) StrongARM: latch → 2 inverters (no symmetry); precharge PMOS pair → "CurrentMirror"; clocked tail → Proximity ≤5 µm and Isolation ≥10 µm to the pair.
   - (b) Rail-to-rail input pair listed N+, P+, N−, P−: `push_pull_pair` wins the id tie-break twice → zero DiffPair leaves. Listed N+, N−, P+, P− → two DiffPair leaves.
   - (c) Real Gilbert cell with a tail: `cascoded_diff_pair_with_tail` takes {RF±, one LO+ switch per side, tail}. The other two switches become a separate "CurrentMirror" stage on another axis.
   - (d) Two equal SC switches with gates `p1` and `p2` and distinct S/D → DiffPair (`diff_pair_split_source`).

---

## 3. Gap analysis against a constraint-aware analog P&R that beats hand layout

Each item gives what the references (or, where marked, general knowledge not in ref/) describe, what Philis does, and what has to exist.

**G1. The requirement graph is overlapping, typed and ordered, not a disjoint partition.** Strasser et al. build an SMP multigraph of matching (M_S from symmetry, M_B from building blocks), proximity (P_B, P_N) and symmetry (S) edges. A device may carry several (balasa_graeb_survey.txt 5136–5160, p.122). They then cluster it agglomeratively by importance order M_S ≻ M_B ≻ P_B ≻ S ≻ P_N (eq.(3.19), 5096–5132, p.121; Algorithm 3.1, 5325–5420, pp.125–126). The rationale for the order is at 5100–5125: symmetric pairs matter more than intra-block matching because they drive offset.

Philis selects a greedy disjoint set (pattern.rs:201–210) and treats every block alike. Needed: recognition returns all matches. A typed requirement graph with provenance is built from them. A hierarchy tree of symmetry, matching and proximity groups is derived by the ordered clustering. The disjoint partition survives only as the *cell-merge* decision (unitization), which is a different question.

**G2. Symmetry analysis at circuit level.** The survey runs a separate symmetry analysis producing symmetry compounds: all pairs sharing one axis, with one S requirement per compound (5230–5241, 5285–5310, pp.123–124). The constraints align every pair's centres perpendicular to the axis and every group's centre on the axis (eqs.(3.23)–(3.25), 5626–5650, p.129). Lampaert defines couples, self-symmetric devices and symmetry groups, requiring identical variants and mirrored orientations for couples (lampaert.txt 3318–3336, p.84). The survey states that the symmetry constraint covers "the whole differential subcircuit" (balasa_graeb_survey.txt 2999–3002, p.75).

Philis assigns one axis per recognised block (emit.rs:139), finds no symmetric nets or devices outside blocks, and checks centres only (AR-11). Needed: a propagation algorithm. Seed it with recognised pairs. Pair corresponding terminal nets. Pair devices of the same kind and size on paired nets in the same terminal roles. Iterate to a fixpoint. The result is one axis per compound, with self-symmetric devices (tail, CMFB, bias shared by both halves) and electrical half labels.

**G3. Ratioed matching groups built from unit devices.** Lampaert: a matching group needs equal orientations, equal variants for 1:1, and "equal unit devices, according to the ratio" otherwise (lampaert.txt 3337–3350, p.84). Hastings rule 1 (hastings.txt 42360–42368, p.712); resistor rule 5 (25218–25220, p.419); capacitor rule 1 (25470, p.423); BJT ratio arrays 2:1:2 and 3×3 with ratios above 8–10 units impractical (30177, p.506; 30661, p.514). The survey uses CC in place of alignment when devices are split into subdevices (5620–5625, p.129).

Philis unitizes only exact (kind, W, L) classes (constraints.rs:30–63). Needed: infer the unit W as the GCD of member widths, bounded by the deck's legal finger width; integer ratios; a matching group spanning a mirror's reference and all its outputs; a CC requirement over units.

**G4. Matching precision class and matching type.** Hastings defines minimal, moderate and exceptional matching (±10/±3/±1 mV or %, six-sigma; hastings.txt 42320–42346, p.712, numbers read from the PDF page). Layout effort scales with the class:
- proximity only / adjacent and interdigitated / 2-D common-centroid (rule 8, 42499–42503, p.714);
- aspect ratio 10:1 / 3:1 / square for current matching and 3:1 / square for voltage matching (rule 9, 42504–42516);
- dummies optional / generally / always (rule 12, 42532–42540).

Voltage- and current-matched devices are optimised differently (42326–42328; rules 2–3, 42369–42412, pp.712–713). Philis infers no class and no type. It applies the same η, dummies, CC and 5 µm proximity everywhere (AA-10). Needed: a class per matched group, inferred from a per-pair spec (offset in mV, or current error in %) and the sizing (6·σ versus the class thresholds), and a matching type from the structure (DP and cross pair → voltage; mirror, load, current source → current). These drive the pattern choice, dummies, aspect ratio, η and the Environment thresholds.

**G5. A complete building-block library including passives and bipolars.** The survey's recognition library lists differential pair, level shifter, simple, cascode, 4-transistor and wide-swing current mirrors, with matching and proximity per element (balasa_graeb_survey.txt 5208–5226, p.123; the library is from Massier/Graeb [13], L8073–8075). The CC review lists DPs, mirrors and R/C arrays as needing matching (cc_review.txt 34–40, p.1). Philis's catalog is FET-only (pattern.rs:94), with no R, C, BJT, diode or inductor slots and 8 dead and 2–3 wrong patterns. Needed:
- DSL slots per `DeviceKind` and net predicates;
- the blocks of table 2.1b rows marked "none": bandgap core, resistor ratio/ladder/divider, cap arrays (binary, split, C-2C, unary), SC switch plus cap, LC tank, CMFB variants, β-multiplier with R, degenerated DP, current-steering arrays;
- moving cellgen's side recognizers (cellgen.rs:486–549) into the annotator so they emit constraints.

**G6. Proximity requirements from the netlist and from wells.** The survey adds a proximity clique per net (P_N, 5153–5160, p.122). Proximity groups share a well or guard ring (2997–3010, p.75). Minimum-distance constraints for wells and guard rings are eq.(3.16) (4742–4775, p.114). Philis has HPWL weights only, and no well grouping by bulk net. Needed: a well-group requirement per (polarity, bulk net), a min-distance rule between different-bulk wells, and P_N at the lowest priority.

**G7. Device-merging permission with the right criterion.** Lampaert: two MOS devices may share a diffusion when they have a common S/D node, the same type and the same bulk potential (lampaert.txt 2741–2744, p.67). Merging cuts junction capacitance (2054–2063, p.52). Philis grants permission to whole same-kind groups and truncates mixed ones (lib.rs:107–117). Needed: abutment permission computed per pair from (kind, bulk net, shared S/D net), within or across recognised groups, including tail-to-pair and series stacks of different W.

**G8. Net roles with evidence.** An expert names rails from the ports and the testbench, identifies clocks and digital lines, and separates bias, reference, high-impedance and output nodes (general practice; LAYOUT-FUNDAMENTALS rows 45, 58). Philis uses names only, with two inconsistent rail lists and overrides that nothing sets (AA-11, AA-14). Needed:
- ports from `.subckt` (depends on the parser rewrite, AF-02) and the fixtures' `interface.json`;
- structural rail inference: the net tied to the bulks of one polarity, the net most sources connect to;
- op-point evidence: DC voltage equal to a source value; region per device;
- testbench sources: pulse/PWL → Clock/Digital;
- classes Digital, Bias and Reference;
- one classification function used by `analog` too.

**G9. Performance-driven importance.** Lampaert's placer runs sensitivity and operating-point simulations before placement (lampaert.txt 3446–3456, p.86). The survey suggests setting constraint importance "by simulation" (balasa_graeb_survey.txt 5426–5431, 5608–5610, pp.127–128). Graeb's sizing constraints c(x_d) ≥ 0 come from structure (graeb_centering.txt 2831–2834, p.76). Philis computes sensitivities in `library::perf` only to build routing rows (lib.rs:196–231), and the op point only for IR/EM. Needed: pass `OpPoint` (region, gm, Id, headroom) and the per-net sensitivities into `annotate`. Use them to separate switches (triode, gate on Clock/Digital) from current sources and amplifiers. Scale each pair's η and precision from its contribution to the spec. Order the requirement graph by sensitivity.

**G10. User constraints as input.** ALIGN accepts user constraint files (symmetry, matching, grouping, SameTemplate, DoNotIdentify). This is general knowledge, not in ref/; the code's own comments cite these ALIGN names at catalog.rs:4–5 and netrole.rs:57–58. Philis accepts only suppression, and nothing sets even that (AA-11). Needed: a sidecar format read by the CLI and `library::Config` that adds or overrides requirements, precision classes and rail/clock roles, and resolves device names to ids.

**G11. Consistency and provenance.** An automated tool can check what a person forgets: contradictory max/min distances, symmetry against fixed macros, empty variant domains (NOTES-06), and traceable constraint IDs (NOTES-04). Philis emits Proximity ≤5 µm and Isolation ≥10 µm on the same device pair (AA-13), and blocks forget their template (AA-27). Needed: a pre-search conflict pass with a precedence table, and a stable ID plus source template per rule.

**G12. Robustness.** Recognition must not depend on netlist order (AA-07). It should tolerate near-equal sizes when the designer intends a match but not when a ratio is intended. It must scale to about 10⁴ devices (AA-22). MAGICAL-style graph-similarity symmetry detection and learned constraint prediction (perf_driven_survey.txt 308–309, p.5) are later options; subgraph iso plus symmetry propagation is enough first.

**G13. Hierarchy.** The survey describes hierarchical symmetry and proximity constraints from exact (circuit) and virtual (cluster) hierarchies (2984–3050, pp.75–76), and symmetry islands (3185–3190, p.79). Philis has two levels, flattens SPICE (AF-02), and places flat (AF-20). Needed: keep subckt instances as exact hierarchy, add same-template matching of identical instances and array detection (N isomorphic cells sharing bias), and export the group tree for gp/dp symmetry islands.

**G14. Coverage report.** The certificate should say which devices matched no structure and why (NOTES-03). Philis reports families only.

What this buys against hand layout: a tool that applies G1–G4 on every pair, checks G11 automatically and reports G14 does what a careful engineer does. It does it for every device, with a quantitative budget per pair derived from the spec, and it proves consistency instead of relying on review.

---

## 4. Ranked findings

| ID | severity | file:line | finding | suggested fix direction |
|---|---|---|---|---|
| AA-01 | critical | backend/annotator/src/pattern.rs:201–210; lib.rs:86–102 | Greedy disjoint selection: a device belongs to one structure only. OTA composites take the tail, so its mirror relation to the bias reference is lost. Mirrors with more than 3 outputs fragment across stages. The survey's overlapping SMP requirement graph (balasa_graeb_survey.txt 5136–5160) cannot be expressed | `recognize` returns every match. New `annotator::graph` builds `Requirement { kind: Ms/Mb/Pb/S/Pn, devices, template, precision }` edges. Derive the group tree with the survey's Algorithm 3.1 in importance order (5325–5420). Keep a separate disjoint choice only for cell merging (unitization) |
| AA-02 | critical | emit.rs:139, 212–215 | One symmetry axis per recognised block. No circuit-level symmetry analysis, so a differential path split over several blocks gets independent axes. A real Gilbert quad is split over two axes (hand trace §2.13-7c) | Symmetry propagation from seed pairs over paired nets to a fixpoint, giving symmetry compounds with self-symmetric members and half labels (survey §3.2.2.2, 5230–5310). `AxisId` per compound. Emit eqs.(3.23)–(3.25) semantics |
| AA-03 | critical | catalog.rs:365–376 vs 230–242, 907–924, 1286–1309; lib.rs:94–98 | Inside cross-coupled latches and VCO cores, `cmos_inverter` (priority 11) beats `cross_coupled` (9) during child recovery. StrongARM latches, sense amps and LC-VCO complementary cores therefore get no Symmetry, MatchingPair or CC | Give `Pattern` explicit role declarations, e.g. `pairs: &[(u8, u8, BlockKind)]` and `self_sym: &[u8]`. For cross_coupled_inverters that is [(0,1,DiffPair),(2,3,DiffPair)]. Children then come from the declaration, not from a re-search. Add a latch regression test |
| AA-04 | critical | pattern.rs:94; cellgen.rs:486–549, 717–786 | Only FETs are recognised. Resistors, capacitors, BJTs, diodes and inductors are glue, so bandgap 1:N, R ratios and ladders, cap DACs, SC circuits and LC tanks get no placement constraints. cellgen runs a hidden second recognizer whose results never reach `Problem` | Add `SlotKind::Kind(DeviceKind)` and passive/BJT terminals to the DSL. Move `dac_banks`, `bjt_groups` and `parallel_groups` into the annotator as blocks emitting CentroidGroup, Proximity, ThermalGradient and matching. Add the patterns from table 2.1b |
| AA-05 | high | catalog.rs:143–160, 118–137, 2228–2247, 206–223 | Loose primitives create false pairs. `diff_pair_split_source` turns any two identical same-type FETs with non-rail gates and 3 distinct nets into a DiffPair. `diff_pair` and `diff_switch` accept a rail as the shared source. `mirror_pair_split_source` labels any shared-gate pair a mirror. Spurious Symmetry, MatchingPair, DTI, CC and Sensitive nets follow | Add net predicates `NetReq::{NotRail, Rail, Internal, Degree(k)}` to links. Require the split-source DP to be joined through a degeneration element. Require a non-rail tail for `diff_pair`. Mark pseudo-differential explicitly. Add negative tests |
| AA-06 | high | catalog.rs:2043–2055, 1628–1641, 1001–1015, 1022–1037, 1790–1805, 2368–2407, 1883–1905, 1768–1785 | Generic 3–4 device shapes (wildcard `miller_comp_fets`, `push_pull_with_bias`, `level_shifter`, `charge_pump_cell`, `schmitt_trigger`, `nand_gate`/`nor_gate`, `bandgap_mirror_pair`, `feedback_pair`) outrank every primitive and steal devices. Three-stage opamp: M6/M8/M9 become one unconstrained Group (§2.13-2) | Delete `miller_comp_fets` (a MOS cap needs D=S tied). Give logic patterns a Digital-net predicate. Score matches by specificity (links + exact sizes + predicates) instead of size tier. Add a trace test on examples/three_stage_opamp |
| AA-07 | high | pattern.rs:145–150, 199; emit.rs:196–197, 216–221 | Results depend on netlist order. Equal-priority ties are broken by device id, so a rail-to-rail input pair written N+, P+, N−, P− yields zero diff pairs (§2.13-7b). The pooled CentroidGroup a/b sides follow slot order, not electrical halves | Select by maximum-weight set packing: exact branch-and-bound on each connected component of the overlap graph, weight = specificity. Assign sides from the symmetry analysis' half labels. Add a property test that shuffles device and terminal order and compares block sets up to relabelling |
| AA-08 | high | lib.rs:94–98; block.rs:16–18, 66–73; parse.rs:1–3 | Composite slot roles (tail, load, cascode) are discarded. The hierarchy is two levels and intermediate structures are lost. The SPICE subckt hierarchy never reaches the annotator. There is no SameTemplate and no array detection | Role declarations (AA-03) plus a group tree (AA-01). After the AF-02 parser rewrite, keep the instance tree on `Netlist` and add identical-instance and array (N isomorphic cells sharing bias) requirements |
| AA-09 | high | constraints.rs:30–63 (52) | No ratio inference: unitization is keyed on exact (kind, W, L), so W-ratioed mirrors are never unitized or interleaved, and `target_ratio` = `dev_nf`. Hastings rule 1 and Lampaert §4.2 require identical units | For each matching group: unit W = gcd(W_i) clamped to the deck's legal finger width; units_i = W_i/unit_W × fingers_i; `target_ratio` = units / gcd(units). Require equal L. Warn beyond about 10 units (Hastings 30177, BJT context). Test: 1 µm/4 µm mirror → [1, 4] |
| AA-10 | high | emit.rs:42–52, 159–167; constraints.rs:61; frontend/library/src/lib.rs:871–872 | No precision class (minimal/moderate/exceptional) and no voltage- vs current-matched distinction. Every pair gets η = 0.3, dummies, pooled CC and 5 µm, and Environment uses "moderate" for all. MatchingPair uses ΔVT even for mirrors | Add `PrecisionClass` and `MatchType` to every matching group, inferred from a per-pair spec (config/sidecar) and 6·σ versus Hastings' thresholds (42320–42346). Drive the CC/interdigitation/proximity choice, dummies, aspect ratio (rules 8, 9, 12) and η. For current matching use σ(ΔI/I) = (gm/I_D)·σ_VT (NOTES-18) |
| AA-11 | high | netrole.rs:18–24, 34–55, 57–75; classify.rs:127; matching_pair.rs:99–106 | Net roles come from names only. `0`, `gnd!`, `vee`, `VPB`/`VNB` are not rails; `p1`/`p2`/`s1` are not clocks; digital inputs become Sensitive. There are two inconsistent rail lists, and nothing sets `AnnotationConfig` | One classifier with evidence levels: ports and interface.json, bulk-tie inference, op DC voltage, testbench pulse sources, plus names. Add classes Digital, Bias and Reference. Delete `analog::is_supply` in favour of `NetClassification`. Plumb `AnnotationConfig` from the CLI and a sidecar |
| AA-12 | high | lib.rs:75; frontend/library/src/lib.rs:165–167, 196–231 | Extraction uses no op point, sizing intent or sensitivity. Switches versus current sources versus amplifiers are guessed from topology. Sensitivities never set constraint priority or η | `annotate(netlist, cfg, evidence: Option<&Evidence>)` with region, gm, Id and headroom per device and per-net sensitivities. Classify switches as triode with a Clock/Digital gate. Weight requirements and budgets by sensitivity (Lampaert 3446–3456; survey 5426–5431) |
| AA-13 | high | emit.rs:203–210 vs 263–288 | Contradictory rules are emitted unchecked: the clocked tail of a dynamic comparator gets Proximity ≤5 µm and Isolation ≥ 4·epi (10 µm nominal) against the same pair | Pre-emission conflict pass over (min distance, max distance, symmetry, fixed) per device pair, with a precedence table (block membership beats isolation). Report conflicts in `Problem` (NOTES-06) |
| AA-14 | high | extract.rs:53–61; matching_pair.rs:82–106 | `Differential` (hard) and `CrosstalkExclusion` are re-derived by `analog`'s `is_diff_pair` (no size match, own rail list) instead of from the recognised pairs. The rules fire on SPICE-ground pairs and miss pseudo-differential pairs | Emit both from the symmetry analysis' symmetric net pairs. Move `Differential` to the budget arm (AR-12). Remove `extract` from the analog rules or feed it the classes |
| AA-15 | medium | catalog.rs:492–506, 1252–1280, 2208–2221, plus the 8 dead patterns in §2.1 | Wrong topologies: `wilson_mirror` has its links swapped (a real Wilson matches `regulated_cascode`), `gilbert_cell` ties 2G=3G, and `triode_load_pair` has G=S (an off device). Eight patterns can never win | Fix wilson (1D=2G, 0D=2S) and gilbert (quad gates 2G=5G, 3G=4G, 2G≠3G; outputs 2D=4D, 3D=5D). Delete triode_load_pair and the dead patterns. Add a unit test that fails when any pattern is shadowed by a superset |
| AA-16 | medium | classify.rs:11–15, 62–85 | Terminal roles are positional for every non-capacitor device: BJT collectors and resistor P terminals count as gates, so collector-only nets become Sensitive (bjt_mirror) | Classify by `hg.terminals` names. Gate = FET "G" only. BJT "B" is a control terminal and "C"/"E" a channel. Passive terminals are their own role. Test on bjt_mirror |
| AA-17 | medium | block.rs:60–62; lib.rs:121–128 | Stack (cascode, series) devices count as sensitive. Their gate nets become Sensitive with 8× spacing, shields and isolation victim status, although block.rs:30 calls them "not matched" | Mark sensitivity from matching and symmetry requirements only. Classify cascode bias nets as Bias |
| AA-18 | medium | constraints.rs:61–62, 66–85; kernel/analog/src/cell.rs:26–35 | Rings and dummies are keyed on "was recognised", not on role. Logic devices get dummies and rings; unrecognised analog devices get none. Ring type docs contradict the drawing. Ring pitch, width and R are fixed literals | Decide rings by role (victim: matched or sensitive; aggressor: clocked or high power) and dummies by precision class. Take ring numbers from the deck. Fix the `GuardRingType` docs or split tap versus collector rings |
| AA-19 | medium | lib.rs:106–117 | Abutment permission is the whole group or the first member: a mixed composite loses same-polarity sharing (tail–DP on `vtail`), and a same-kind composite grants sharing without checking bulk or a shared net | Compute permission sets per (kind, bulk net, shared S/D net) (Lampaert 2741–2744). Allow the table to be non-parallel to `groups`, or make it a list of sets per group |
| AA-20 | medium | pattern.rs:91–117; constraints.rs:30–33; cellgen.rs:122, 129 | Model (Vt/oxide flavour) and bulk net are ignored by matching and unitization, so a merged cell is drawn with the first member's model (LVS mismatch) | Add `model` and bulk-net equality to slot `ExactAs` and to the unitization key. Test: nfet_01v8 + nfet_01v8_lvt of the same W/L in one block → two classes |
| AA-21 | medium | lib.rs:80, 155–161; constraints.rs:22–24; parse.rs:100 | A missing W/L reads as 0, which makes all devices "identical" (any `.param` netlist, per AF-02) and gives zero gate area (no Pelgrom, antenna or budgets). `nfin` and `multi` are ignored | Make `Geom` fields `Option`; unknown size means no `ExactAs` match and a `missing` entry. Add `nfin` for FinFET. Settle the W convention with AF-01 |
| AA-22 | medium | pattern.rs:143–167, 81–84 | The matcher scans all n devices per slot (O(n³) for `miller_comp_fets`), explores permutations of interchangeable slots, and dedupes in O(M²) | Draw candidates from the nets of already-assigned slots (`hg.net_devices`). Break symmetry among interchangeable slots (ascending ids). Use a hash set for `seen`. Cache terminal indices. Add a benchmark at n = 10⁴ |
| AA-23 | medium | emit.rs:203–211 | Every unpaired member of a DP stage is forced onto the axis and within 5 µm, whatever its role (e.g. `telescopic_ota_full` slot 7). A tail recognised as a Stack pair (cascoded tail) gets neither | Self-symmetry from the symmetry analysis (devices whose nets are all self-symmetric). Proximity from declared roles (AA-03) |
| AA-24 | medium | emit.rs:158–167 | W-ratioed mirror pairs get couple Symmetry, which mirrors unequal devices, and MatchingPair with `min(gate)`. A ratio needs a CC group over units, not mirror symmetry | For ratioed groups emit CentroidGroup over units plus orientation equality. Keep Symmetry for equal couples only |
| AA-25 | medium | classify.rs:17–33, 87–104 | Net budget policy: a drain-only net is budgeted against the circuit's smallest gate load; Sensitive is 1× the load (the code itself says 10 %/2 % is the target); Clock is budgeted like Signal | Budget from sensitivities where `perf` is configured. Otherwise from the node's real load (gate C plus junction C plus declared external load). Treat Clock/Digital as aggressors with a slew budget |
| AA-26 | medium | backend/annotator/src/tests.rs (whole file) | 5 of 103 patterns tested. No false-positive, permutation, scale, latch, bandgap, folded or three-stage tests. No per-structure emission assertions | Add a table-driven test: for every pattern, a minimal netlist must produce exactly that block and the declared roles. Add the §2.13 traces as regression tests, plus a permutation property test |
| AA-27 | low | block.rs:66–73; metadata.rs:177–202 | `Block` drops the template name. The report has no recognition coverage (glue devices, templates matched) | Store `template: &'static str` and a provenance id per rule (NOTES-04). Add a `recognition` section to `MetadataReport` |
| AA-28 | low | emit.rs:42–52, 250–256; extract.rs:47, 98–100, 110–127; constraints.rs:79–81; ir.rs:18–27 | Policy literals (5 µm proximity, 0.3 η, 500 m°C, 2.5 µm epi, spacing multiples, margins, shield 80 %, ring 2 µm/0.5 µm/100 Ω) are not tied to the deck, the spec or the precision class | Move them to a `Policy` struct keyed by precision class and deck. Deck values where they exist; declared defaults otherwise, listed in the report |
| AA-29 | low | lib.rs:53–71 | `missing` lists MatchingPair even when the circuit has no matched pair, which blocks certification for nothing | Emit a `missing` entry only when some rule of that family was instantiated or needed |
| AA-30 | low | tests.rs:144, 196, 302–309; catalog.rs:1911–1917, 2223–2227, 2294–2299, 2341–2345; netrole.rs:57–63; docs/API-WISH.md:551, 562, 574, 594, 598, 622, 704 | Stale references and doc debris: `backend/TODO.md`, `emit::budget_and_cost`, `extract::push_one`, `DTI_S_MAX_NM` constants, "BiasGen" seeding and a hard+cost `Isolation`, none of which exist in the current code; "library resolves names" and "SameTemplate" claims | Delete or correct each. Update the API-WISH annotator debt list |
| AA-31 | low | extract.rs:88–104 | Shield reference = the first Ground net. Shields go to every Sensitive net, including misclassified digital ones | Choose the quiet reference by class (analog ground). Shield only true victims after AA-11/AA-17 |
| AA-32 | low | frontend/library/src/lib.rs:789–792, 871–872 | The library re-implements the gate area (a third copy) and takes A_VT from member a only. It uses "moderate" WPE/OSE thresholds for all pairs | Export `annotator::gate_um2` and a per-group Pelgrom helper. Select the thresholds by the group's precision class |
| AA-33 | low | emit.rs:271–279 | Isolation is all-pairs O(A·V). Any device touching a clock net at any terminal is an aggressor, and strength is not modelled | Aggressor = the switching node's driver or switch with an activity/strength estimate. Victims = sensitive groups. One rule per (aggressor group, victim group) |
| AA-34 | low | pattern.rs:97–103, 120–122 | Catalog invariants are unchecked: a forward slot reference panics, and a link to slot ≥ n is silently ignored | A test that validates every `Pattern` (refs < own index, link slots < n, pins valid for FETs) |
| AA-35 | low | lib.rs:103; block.rs:69; classify.rs:105 | `u16` device and net ids wrap silently past 65 535 | Use `u16::try_from(..).expect(..)` or return an error. Document the limit |

---

## 5. Questions the planners must resolve

1. **Partition versus graph.** Replace the disjoint `blocks` table with an overlapping requirement graph plus a group tree (AA-01)? That changes the `groups`/`abutment`/`AxisId` contract of D16 and the gp/dp axis table (AR-15). Which consumer owns which view: cell merging (disjoint), symmetry islands (tree), rule emission (graph)?
2. **Who owns recognition?** Move cellgen's DAC, BJT and parallel recognizers into the annotator (AA-04)? Should the annotator then also decide cell-merge topology (merged versus apart), which `library::run` currently tries both ways (lib.rs:169–182)?
3. **Symmetry axis policy.** One axis per symmetry compound, with how many compounds at most? Does placement need a secondary axis (horizontal) for 2-D CC groups?
4. **Spec input.** In what format does the user give a per-pair offset or current-error spec and a precision class: `library::Config`, a JSON sidecar next to the netlist, or both? What is the default class when absent? Hastings "moderate" matches the current Environment keys.
5. **Is an operating point mandatory for extraction?** If it is optional, what are the fallback rules for switch versus source classification? Should classifications made without evidence be marked "assumed" in the certificate?
6. **Matcher technology.** Extend the slot/link DSL (net predicates, passive slots, W ratios, model equality) or move to a general subgraph-isomorphism engine over a typed pin graph? What is the target maximum device count for runtime (AA-22)?
7. **Pseudo-differential and rail sources.** Should two CS devices sharing a rail with differential inputs be a symmetric couple? Recommended: only when the symmetry analysis pairs their gate nets.
8. **Budget policy for Sensitive nets.** Keep the 1×/2× load multiples, or require `perf` sensitivities for any budget and mark budgets without them unknown (AA-25)?
9. **Clock and digital detection.** Which evidence is authoritative: name conventions, testbench sources, structure, or the op point? Must the user confirm clocks?
10. **Rings and dummies by role.** Should logic and glue devices ever get rings or dummies? Who decides victim versus aggressor ring type, and which deck keys supply the ring numbers (AA-18)?
11. **Differential routing.** Move `Differential`/`CrosstalkExclusion` extraction into the annotator and delete `analog`'s `is_diff_pair`/`is_supply` (AA-14), and demote `Differential` to the budget arm (AR-12)?
12. **Conflict precedence.** When a structural proximity conflicts with an isolation requirement (AA-13), which wins? Is a detected conflict a hard stop, or a report plus automatic relaxation?
13. **Determinism contract.** Is netlist-permutation invariance a hard requirement (AA-07)? It rules out any id-based tie-break.
14. **Hierarchy source.** After the parser rewrite (AF-02), are subckt boundaries hard hierarchy (placement islands) or advisory clusters? Are identical instances always SameTemplate?
