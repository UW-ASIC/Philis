# Audit 07: orchestration, frontend, macroMaster, and a reality check of the design docs

Scope: the flow orchestrator (`frontend/library`), `frontend/cli`, `kernel/macroMaster`, `examples/*`, `kernel/visualizer` (skimmed), and the five design documents. Finding IDs use the prefix `AF-`.

- Code references are `path:line` against the working tree as of 2026-09-28, uncommitted changes included. Paths without a crate prefix are inside `frontend/library/src/`.
- Reference citations give the source, the section or equation, the line range in the `pdftotext -layout` extraction (`reftext/<file>.txt`), and the PDF page counted from form feeds.
- No build or test was run.

**Corrections to the earlier draft of this file.** Every claim was re-checked against the code. Five were wrong:
- The augmented-Lagrangian dual step does run on dp's legal layout: `dp::place` calls `prices.settle` at the end (backend/dp/src/lib.rs:364). The real defects are different and are restated in AF-07.
- The `macro_master::Generator` mentions are at examples/three_stage_opamp/src/main.rs:46 and :80, not 113–114.
- `Elaborated::intent` does flag ground rails (elaborate.rs:315 pushes an `is_ground` flag).
- `to_rust` naming a struct `emitted` only triggers a warning. What stops the output compiling is a keyword or illegal identifier used as a port field, for example a net named `in`.
- Lampaert §2.4.2.3–2.4.2.4 is on PDF pp. 44–45.

---

## 0. Scope: files read

Every listed file was read in full, except `kernel/visualizer/src/lib.rs`, which the task marked "skim".

| File | Lines | Read |
|---|---:|---|
| frontend/library/src/lib.rs | 1439 | full |
| frontend/library/src/cellgen.rs | 1700 | full |
| frontend/library/src/elaborate.rs | 460 | full |
| frontend/library/src/emit.rs | 449 | full |
| frontend/library/src/metadata.rs | 462 | full |
| frontend/library/src/oppoint.rs | 656 | full |
| frontend/library/src/parse.rs | 348 | full |
| frontend/library/src/perf.rs | 353 | full |
| frontend/library/src/fill.rs | 240 | full |
| frontend/library/src/gds.rs | 249 | full |
| frontend/library/src/geometry.rs | 326 | full |
| frontend/library/tests/antenna_diode.rs | 56 | full |
| frontend/library/tests/elaborate.rs | 73 | full |
| frontend/library/tests/emit_roundtrip.rs | 217 | full |
| frontend/library/tests/extra_devices.rs | 58 | full |
| frontend/library/tests/flow_smoke.rs | 54 | full |
| frontend/library/tests/hier_elaborate.rs | 221 | full |
| frontend/library/tests/injected_macro.rs | 36 | full |
| frontend/library/tests/ota_cross_pdk.rs | 363 | full |
| frontend/library/tests/perf_postlayout.rs | 99 | full |
| frontend/library/tests/tmp_deck_drc.rs | 27 | full |
| frontend/cli/src/main.rs | 64 | full |
| kernel/macroMaster/src/lib.rs | 731 | full |
| kernel/macroMaster/src/adapter.rs | 50 | full |
| kernel/macroMaster/src/tests.rs | 227 | full |
| examples/op_demo/src/main.rs | 67 | full |
| examples/three_stage_opamp/src/main.rs | 112 | full |
| kernel/visualizer/src/lib.rs | 1230 | skimmed: 1–40, the item index, 1149–1230 |
| PLAN.md | 243 | full |
| docs/API-WISH.md | 825 | full |
| docs/CRATES.md | 117 | full |
| docs/LAYOUT-FUNDAMENTALS.md | 242 | full |
| docs/SUBSTRATE3.md | 85 | full |

The following were read partially, only to verify the orchestrator's contracts with other crates:
- `backend/gp/src/lib.rs` 1–330: `Prices`, `place`.
- `backend/gp/src/mechanics.rs` 261, 290–316, 361–386: `analog_phi`, `report`, `initial_layout`.
- `backend/dp/src/lib.rs` 1–372: schedule, moves, `project_hard`, the final `settle`.
- `backend/gr/src/lib.rs` 30–133 (`Negotiation`, `price_group`), 379–408 (`analog_tiers`, `score`), 757, 940.
- `backend/dr/src/lib.rs` 540–600, 1828–1853.
- `backend/annotator/src/constraints.rs` 1–60; `netrole.rs` 14–75; `emit.rs` 193, 224–232, 252–283.
- `kernel/core/src/report.rs` 1–48; `kernel/core/src/thermal.rs` 1–4; `kernel/core/src/units.rs` 27–63.
- `kernel/analog/src/placement/cc.rs` 7–84; `routing/parasitic.rs` 14, 41–48, 81; `routing/em.rs` 76–94.
- `kernel/cells/src/mosfet.rs` 30–47, 85, 108–129; `resistor.rs` 13–57, 162–175; `bjt.rs` 10–22, 206–207.
- `backend/verify/src/pdk.rs` 1040–1059, 1120–1143.
- `benchmarks/src/bench.rs` 1–20, 142–180; `benchmarks/src/fixtures.rs` 421–610 (index only).
- `benchmarks/fixtures/ota.spice`, `ota_constrained.{spice,interface.json}`; greps of `pdks/*.json`.

---

## 1. Architecture and data flow

### 1.1 Entry points

| Entry | Input | Output | Path |
|---|---|---|---|
| `library::run` (lib.rs:156–190) | SPICE text, `verify::Pdk`, user `Macros`, `Config` | `Solution` (lib.rs:82–101): layout, routes, macros (cells, then rings, diodes, fill), netlist including inserted diodes, `RunStats`, `MetadataReport`, retargeted placement rules, signoff `Intent`, fold table, nwell layer | the searched flow |
| `library::signoff` (lib.rs:1289–1302) | `Solution`, `Pdk` | `Report` (DRC/ERC/LVS/PEX rows, plus `cell/undrawable` rows) | final gate |
| `library::elaborate` (elaborate.rs:124–130) | `macro_master::Composition`, `Pdk`, `ElabConfig` | `Elaborated` (identity placement, routes, nets, ports, optional schematic) | hand-placed "substrate3" path, with no gp/dp |
| `library::emit::{emit, elaborate_ir, to_rust}` (emit.rs:92, 313, 365) | netlist and a solved `Layout` | `GenIr`, then a re-elaborated `Elaborated` or Rust source | decompiler |
| `philis` CLI (cli/src/main.rs:23–63) | netlist path, deck JSON path, optional emit target | exit code from signoff; optional `.rs` | `Config::default()` hard-wired (main.rs:42) |

### 1.2 `library::run`, step by step

1. **Parse.** `parse::spice` (parse.rs:15–119) builds one flat namespace from primitive cards. Then `deck_models` renames each model to the deck's name (lib.rs:144–150).
2. **Injected-macro check.** Every injected FET macro must extract to exactly one device (lib.rs:970–984).
3. **Bias, once per run** (lib.rs:1061–1095 → `oppoint::extract`, oppoint.rs:155–198).
   - ngspice `op` on a flat deck of FETs only.
   - The bench is the user testbench, else a synthesised probe.
   - Output per device: `|Id·Vds|`, `Id`, `|Vds|−|Vdsat|`, `gm`.
4. **Performance budget rows, once per run** (lib.rs:196–231 → `perf::sensitivities`/`budget_rows`, perf.rs:200–257).
   - Finite-difference ∂spec/∂C_ground: one baseline run plus one ngspice run per Signal, Sensitive or Clock net, with 10 fF added to that net. All runs are concurrent.
   - Output: one `PerformanceBudget` routing row per spec that has schematic headroom.
5. **Multi-start** (lib.rs:174–189).
   - `starts` scoped threads (default 3, lib.rs:75). Each runs `solve(merge_distinct_gates = true)`.
   - If any distinct-gate merge happened, the thread also runs `solve(false)` and keeps the better `LexKey`.
   - The lexicographically best start wins; ties go to the earliest.
6. **`solve`** (lib.rs:249–458).
   - Annotate (lib.rs:262–263). Add the `Utilization` budget (lib.rs:264–266) and the perf rows (lib.rs:267–269).
   - Fold table `cellgen::folds` (lib.rs:274).
   - `CellSpace::new` (lib.rs:1121–1219): draw every variant once; collapse matched unitizations into one cell; retarget placement rules to cell ids; grow each ringed cell's variant bboxes by the ring halo.
   - Routing stack and EM limits from the deck (lib.rs:279–280). Add hard EM rules (lib.rs:281) and IR budgets (lib.rs:283–293).
   - Net weights (lib.rs:294–296). Detailed-router config (lib.rs:302–318).
   - Seed the variant assignment with `cellgen::seed_assignment` (lib.rs:340), which prices each alternative by `(unreachable, DRC+ERC count, gr overflow, HPWL)` (cellgen.rs:354–375).
7. **Search loops** (lib.rs:346–394).
   - *Outer:* up to `outer_iters` (default 4) variant assignments.
   - *Middle:* up to `feedback_iters` (default 200) epochs, stopping after `PATIENCE = 8` epochs without improvement (lib.rs:137, 369–372).
   - After each middle loop, terminate when the incumbent is feasible (`|V| = 0`, spec miss ≤ 0, Θ ≤ 0) **and** `prices.drift() < 1e-3` (lib.rs:377–383). Otherwise move to `cellgen::escalate`'s successor (lib.rs:386–393).
   - `prices` and `neg` (routing history) persist across epochs and across assignments (lib.rs:341–342).
8. **One epoch** (`Flow::epoch`, lib.rs:561–709).
   - **gp** (lib.rs:575): momentum gradient descent from a random pile.
   - **dp** (lib.rs:587–597): Metropolis SA. Its `reshape` move is enabled only on odd epochs (lib.rs:356).
   - **Rings and bridges** (lib.rs:607–613): guard rings, well bridges, implant bridges on the placed cells.
   - **gr** (lib.rs:617–618): PathFinder on a gcell grid.
   - **dr** (lib.rs:626–631): track-lattice PathFinder with repair, given this placement's common-source nodes.
   - **Antenna diodes** (lib.rs:634–652): inserted for nets still over the antenna limit, then dr re-runs.
   - **Signoff** (lib.rs:662–670): DRC/ERC/LVS/PEX of the drawn shapes; also returns the extracted C matrix.
   - **Θ** (lib.rs:671–680): metadata over both rule tiers, plus `CommonNodes` and `Environment` measured on the result.
   - **Key** (lib.rs:686; lex_key at lib.rs:941–958): `LexKey = (|V|, spec miss, Θ, extracted C, footprint)` (lib.rs:916).
   - **Promotion** (lib.rs:359–361): an epoch whose `|V|` can still beat the incumbent is simulated (`score_perf`, lib.rs:878–894) and its spec miss enters the key.
9. **Winner** (lib.rs:397–457).
   - Re-realise the winner's variants; add rings and diodes.
   - Metal fill, grown from ground wires (lib.rs:410–420).
   - Final metadata and per-spec performance rows (lib.rs:421–439).
   - Inserted devices join the netlist (lib.rs:443–444).

### 1.3 Algorithms, named precisely

- **Global placement.** Momentum gradient descent over absolute centres, starting from every cell at the canvas centre ±15 % uniform jitter (backend/gp/src/mechanics.rs:361–371). The gradient is the sum of:
  - a weighted pin-HPWL subgradient (extreme pins pulled inward);
  - a central-difference gradient of the priced analog cost (`ANALOG_PROBE`);
  - a bin-density push, where a cell in an overfull bin is pushed toward the emptiest 4-neighbour bin (backend/gp/src/lib.rs:220–285).
  - A step that raises hard Φ is undone (backend/gp/src/lib.rs:293–312). This is not an electrostatic (ePlace) density model.
- **Detailed placement.** Metropolis SA over absolute coordinates with a fixed schedule: 220 iterations × 60·n moves, α = 0.93 (backend/dp/src/lib.rs:21–26, 301–349).
  - Moves: displacement 70 %, swap 20 %, DTI branch flip, variant reshape, rotate, and compound symmetric moves 8 % when symmetry groups exist.
  - A lexicographic gate on `(violated hard batches, Φ margin + encroachment, Θ)` (backend/dp/src/lib.rs:402) leaves Metropolis a vote only on the PEX tier.
  - One exact projection of violated hard batches per SA iteration (backend/dp/src/lib.rs:344). A terminal overlap legaliser follows (backend/dp/src/lib.rs:360).
  - There is no topological representation (sequence pair or B*-tree).
- **Budget multipliers.** Per placement-budget batch: `λ ← max(λ − ρ·max(c,0), −64)`; ρ doubles (cap 64) when the residual did not shrink (backend/gp/src/lib.rs:83–101). `settle` runs twice per epoch: at the end of gp on gp's layout (backend/gp/src/lib.rs:325) and at the end of dp on dp's layout (backend/dp/src/lib.rs:364). Routing budgets have no multipliers: there is no `Prices` in backend/gr or backend/dr (grep).
- **Routing.** PathFinder negotiated congestion (McMurchie–Ebeling). The history is keyed by absolute 500 nm buckets and folded with `max` (backend/gr/src/lib.rs:36–92). It is owned by the orchestrator and never reset.
- **Search control.**
  - Every epoch is an independent restart of gp from a new random pile (lib.rs:352, 575). Only λ/ρ and the routing history carry over.
  - Multi-start threads run at the top (lib.rs:185–188).
  - A mixed-radix odometer steps through pre-drawn variant alternatives (cellgen.rs:401–416).
- **Performance coupling.**
  - Linear sensitivity budgets `Σ_i w_i·C_i ≤ 1` per spec, from finite-difference perturbation of net ground C only (perf.rs:200–257). This is Lampaert's perturbation method (§2.4.2.1, lampaert.txt 1641–1655, PDF p.43) applied to the linear degradation model of §2.4.1, eqs. 2.12–2.13 (lampaert.txt 1601–1622, PDF p.42).
  - Promoted epochs are also simulated directly (perf.rs:168–187).
- **Thermal.** Point-source superposition `ΔT = P/(2πkr)` (kernel/core/src/thermal.rs:1–4), refreshed once per SA iteration (backend/dp/src/lib.rs:346). There is no PDE solve.
- **Fold selection** (cellgen.rs:615–706). An exhaustive scan over `k ∈ 1..=64` per (kind, W, L) class. It minimises, lexicographically:
  1. violation of the finger-width cap, the gate-R floor or the p2p-R share;
  2. wrong finger parity;
  3. `|ln aspect|` of the class row.

### 1.4 Other paths

- **macroMaster.**
  - Traits: typed IO (never type-checked), `Block`/`DeviceGen`/`Composition` (kernel/macroMaster/src/lib.rs:71–117).
  - `CompBuilder`: translate-only placement by `align_delta` (lib.rs:195–208), `place_mirrored` by reflection (lib.rs:342–349), and a bbox overlap refusal (lib.rs:353–362).
  - Union-find net resolution (lib.rs:218–258).
  - Nested compositions (`instantiate_comp`, lib.rs:328–338), whose devices flatten into one schematic `Netlist` (lib.rs:413–468).
  - Three shipped `DeviceGen`s (`Mos`, `MatchedPair`, `Res`) replay `cells` output (lib.rs:477–702 via adapter.rs:13–50).
- **elaborate.** `route_built` (elaborate.rs:133–201) uses the identity layout, then `cfg.epochs` (default 4) rounds of gr+dr sharing one `Negotiation` over a fixed placement, and keeps the best `Report::lex`. This is the correct PathFinder usage: the resource graph is fixed. There is no placement optimisation, no signoff in the loop, and no intent.
- **emit** (emit.rs:92–291).
  - Re-annotates and re-enumerates cells with default folds, then lifts the placement into align chains, attributing a gap to `device_gap` when it matches within 2 grid steps.
  - Emits connect edges.
  - `elaborate_ir` interprets the IR through `build_with` and re-routes; `to_rust` pretty-prints it.

### 1.5 Does anything simulate the layout?

**Partly, and only when `Config::performance` is `Some`.** The default is `None` (lib.rs:74). The CLI never sets it (cli/src/main.rs:42). The benchmark sets `op` but not `performance` (benchmarks/src/bench.rs:180).

When it is set, a promoted epoch runs ngspice on a deck (perf.rs:99–149) built from:
- the **schematic's** FET cards (`flat_circuit_with`, oppoint.rs:254–289);
- one `Cpex` per nonzero entry of the extracted ground/coupling matrix, lumped at the net node (perf.rs:108–114);
- one series `Rpex` per device terminal with routed branch resistance (perf.rs:116–127, from `Stack::terminal_resistance_ohm` at lib.rs:733–749);
- `sa = sb = S`, fitted by bisection so that BSIM4's multi-finger LOD average at SD = 0 reproduces the drawn mean LOD term (perf.rs:32–48, 132–143).

**It does not simulate the extracted netlist.** `verify::extract_spice` exists and is used by `Elaborated::netlist` (elaborate.rs:62–83), but the flow's performance tier overlays parasitics on the schematic instead. The device-size convention of the simulated cards also differs from the drawn devices (AF-01). The survey describes the standard flow as "simulations are conducted on the extracted netlist" (perf_driven_survey.txt 56–72, PDF pp.1–2).

---

## 2. Module-by-module findings

### 2.1 `lib.rs` (orchestrator)

**What it does.** See §1.2. It owns `Config`, `Solution`, `RunStats`, `Flow`, `Epoch`, `CellSpace`, `Bias` and `LexKey`, plus:
- the EM rule construction (lib.rs:991–1028) and pin currents (lib.rs:1035–1055);
- the `CommonNodes` and `Environment` measurements (lib.rs:763–876);
- the post-layout parasitics bundle (lib.rs:719–757);
- signoff plumbing: labels at lib.rs:1376–1410; the LVS reference plus dummy cards at lib.rs:1351–1370.

**Correctness problems.**

1. **Epochs are random restarts, not refinement.**
   - `Flow::epoch` calls `gp::place` with a fresh seed (lib.rs:352, 575), and gp starts from a pile at the canvas centre (backend/gp/src/mechanics.rs:361–371). The incumbent layout is never an input to the next epoch.
   - The "middle feasibility loop" is therefore random-restart sampling with adaptive placement penalties, not PLAN §3a's feasibility pump. It also cannot do PLAN §5's "no move in the current neighbourhood … reduces PEX" test, because no neighbourhood of the incumbent is ever explored after its epoch ends.
2. **Routing history outlives the placement it priced.** `Negotiation` history is keyed by absolute 500 nm buckets, folded with `max`, and never decays (backend/gr/src/lib.rs:36–41, 83–91). The PathFinder convergence argument quoted in PLAN §4e and API-WISH D3 holds for a fixed resource graph. Here each epoch re-places from scratch, so history from one placement penalises the same absolute coordinates in the next, unrelated one. It turns into a spatial prior that only grows, and grows most where gp piles cells: the canvas centre. When antenna diodes are inserted, dr runs twice in one epoch (lib.rs:629–646) and accumulates history twice.
3. **The multiplier machinery does not do what PLAN §3b/§5 and API-WISH D9/D12 describe.** Details are in AF-07:
   - one-sided λ (`c = max(residual, 0)`, so λ only falls; backend/gp/src/lib.rs:86, 96);
   - two dual steps per epoch on two different layouts;
   - a ρ ratchet that compares a gp residual with a dp residual;
   - `drift` equal to zero whenever the last dp layout satisfies every placement budget, whatever λ did before;
   - saturated λ read as settled;
   - no multipliers at all on routing-tier budgets, including the performance rows.
4. **Θ weighting is not uniform.** `lex_key` sums the stage reports' `pt + rt` with `budgets.theta()` (lib.rs:949–954).
   - The stage reports already contain the same placement and routing budget residuals (backend/gp/src/mechanics.rs:300–309; backend/gr/src/lib.rs:387–395), `ceil`ed in milli-budgets.
   - `CommonNodes` and `Environment` enter only through `add_routing` (lib.rs:680). dr's overuse and EM-underwidth enter only through `rt` (backend/dr/src/lib.rs:1852–1853; backend/gr/src/lib.rs:404).
   - Those families therefore weigh about half as much as the annotator's budgets. The comment at metadata.rs:88–92 ("monotone, so the ranking is unaffected") is false for a non-uniform reweighting.
5. **`|V|` sums counts of different granularity.** Placement and routing contribute one entry per *violated batch* (backend/gp/src/mechanics.rs:291–299; backend/gr/src/lib.rs:380–386), plus one "device overlap" row. Signoff contributes one entry per *finding* (lib.rs:952). A batch with 40 broken symmetry rules weighs the same as one DRC notch.
6. **The oracle does not steer anything.** The comment "dp consults the live DRC oracle within one [epoch]" (lib.rs:336–339) is false.
   - No `drc`/`verify` handle exists in backend/dp or backend/gp. No `Oracle`, `LiveOracle`, `NullOracle` or `DrcSpacing` type exists anywhere (grep, 0 hits).
   - Signoff findings only rank epochs. They never become placement or routing constraints.
7. **Fill is added after selection and never DRC-checked.** `fill::fill` runs on the winner only (lib.rs:410–420) and checks only ERC (fill.rs:58, 66). The winner's key does not describe the returned geometry. `library::signoff` can report new DRC findings (windowed density, spacing to fill) that no epoch saw.
8. **Escalation fires outside its documented diagnosis.** The assignment escalates whenever the incumbent is not both feasible and stationary (lib.rs:377–393). That includes a *feasible but not stationary* incumbent and iteration-budget exhaustion without a stall. API-WISH D12 specifies "stalled + still infeasible ⇒ escalate".
9. **The spec tier degrades silently.** A failed `perf::evaluate` produces a full miss for every spec (lib.rs:880–891). With a flaky simulator every promoted epoch ties on the spec tier, and selection falls through to Θ without a warning in `RunStats`.
10. **The "undrawable" report names the wrong device.** `signoff` walks macros by *cell* index `i` and prints `sol.netlist.devices.get(i)` (lib.rs:1295–1297). After a group collapse, cell `i` is not device `i`.
11. **Injected macros get no parasitic or common-node terminals.** `parasitics()` and `common_nodes()` read only `d{k}:T` pin names (lib.rs:726, 771). Injected macros carry bare `G/D/S/B` pins (lib.rs:161–167; the example renames them at examples/three_stage_opamp/src/main.rs:108–110). Their terminals get no series R in the perf deck and no pins in `CommonNode`.
12. **The hard EM rule is a lower bound.** `em_rules` checks the largest single-terminal current per net (lib.rs:1003–1020; semantics at kernel/analog/src/routing/em.rs:76–94). Lienig & Thiele size a wire from the worst-case *segment* current `i_w`, derived from terminal current models (§3.5.2 eqs. 3.21–3.23, lienig_em.txt 4587–4629, PDF pp.100–101). In the flow, the per-segment check exists only as dr's Θ (EM-underwidth, backend/dr/src/lib.rs:1852–1853), so a reliability limit is tradeable against other budgets.
13. **`key_lt` is not NaN-safe.** A NaN in the spec or Θ tier makes `head(a) < head(b)` false in both directions (lib.rs:929–939). A NaN incumbent at equal `|V|` can then never be displaced.

**Dead code, stale comments, literals.**
- `vdd_mv` defaults to 1800 mV (lib.rs:284) inside a branch that runs only when op currents exist, which requires `cfg.op`. The default is unreachable.
- `Config::op`'s doc says "a failed solve falls back to zero power" (lib.rs:48–49), and the stderr line says "continuing with zero power" (lib.rs:1071). The code returns `cfg.device_power_uw` (lib.rs:1076).
- The `LexKey` doc describes a 4-tuple and then a 5-tuple (lib.rs:910–915).
- `round_up`'s doc describes the dp clearance (lib.rs:960–962).
- The `annotation` ponytail says the stack leaks "twice per run" (lib.rs:511–512). Actual: one per `performance_rows` + one per `solve` (up to 2·starts) + one per `emit` + one per `route_built`.
- Constants:
  - `PATIENCE = 8` and `PRICE_STATIONARY = 1e-3` (lib.rs:137–139);
  - `C_TIE = 0.02` (lib.rs:922);
  - defaults `feedback_iters = 200`, `outer_iters = 4`, `starts = 3`, `min_utilization = 0.6` (lib.rs:66–78);
  - IR `margin_pct: 20` (lib.rs:288);
  - sensitivity step of 10 fF (lib.rs:215–216).
- The `place_rules` layer list uses sky130 role names `["nwell","diff","tap","poly","nsdm","psdm","li"]` (lib.rs:531). A deck without those roles silently contributes nothing to the clearance.
- `Environment` uses `wpe_clearance_moderate`, and uses `lod_moat_ext_moderate` as a stand-in for the OSE range (lib.rs:871–872).

**Efficiency.**
- Each `solve` re-annotates, re-enumerates and re-prices every variant with one full DRC+ERC pass each (cellgen.rs:327–375). With `starts = 3` and a distinct-gate merge, the same pure inputs are priced up to 6 times.
- `realize` clones every macro 3–4 times per epoch: lib.rs:574, 721, 765, 809.
- `labeled_pins` is O(pins × shapes) (lib.rs:1384).
- `environment()` is O(units × wells²) per pair (lib.rs:825–841).

**Tests.**
- `multi_start_is_deterministic` asserts equal x/y for the same seed (lib.rs:1416–1426).
- `close_c_is_decided_by_area_and_far_c_by_c` asserts the tie band (lib.rs:1430–1435).
- Nothing tests `lex_key` composition, the escalation policy, the convergence flag, fill after selection, the NaN path, or the undrawable-cell naming.

### 2.2 `parse.rs` (SPICE front end)

**What it does.**
- Folds `+` continuations (parse.rs:123–137).
- Drops blank lines, `*` lines and every `.`-directive (parse.rs:35–39).
- Splits tokens into positional and `k=v` (parse.rs:43–52); the last positional token is the model (parse.rs:57–59).
- Classifies by instance letter, and for `X` cards by substrings of the model name (parse.rs:143–206).
- Reorders MOS `D G S [B]` into `G, D, S, B` (parse.rs:75–87).
- Parses SI suffixes (parse.rs:221–254) and converts `w/l` to nm, treating a bare value ≥ 0.01 as µm (parse.rs:258–264).

**Coverage and correctness problems.**

1. **No hierarchy** (documented at parse.rs:1–3).
   - `.subckt`/`.ends` are skipped, so several subcircuits merge into one namespace and equal local net names alias.
   - The port list is discarded. No "external pin" notion exists anywhere in the flow.
   - An `X` call to a user subcircuit is classified as NMOS unless the model name contains a family substring. `X1 a b c myamp` becomes an NMOS with D=a, G=b, S=c, B=c (parse.rs:157–161).
2. **Substring classification has false positives.** `has("res")` makes `X… reset_logic` a Resistor; `has("ind") && !has("individual")` makes `X… window_cmp` an Inductor (parse.rs:195, 201).
3. **Foundry names are not classified from the deck.**
   - gf180's resistor model `ppolyf_u` (pdks/gf180mcu.json:86–89) and IHP's `rsil`/`rppd` (pdks/ihp_sg13g2.json:81–97) contain no recognised substring, so they fall through to NMOS.
   - A 2-node card then fails with "expected >= 3" (parse.rs:65–69). A card that also lists a body node would *silently* become a 3-terminal NMOS. Whether those models are 3-terminal subcircuits is not verified here.
   - The deck already knows its models (`Pdk::deck_model`, `Pdk::recipe`, backend/verify/src/pdk.rs:1120–1143), but `parse::spice` takes no `Pdk`.
4. **No `.param` or expressions.** `W={2*wu}` or `W=wunit` reach `parse_value`, which returns `0.0` (parse.rs:253). The device gets W = 0 with no warning.
5. **R/C/L values are discarded.** Only `w l nf nfin stack m multi` survive (parse.rs:95–104). `R1 a b 10k` parses as a Resistor whose model is `10k`, with no value. The benchmark patches this outside the library: `preprocess_spice` synthesises W/L for bare R/C, resolves `.param`, joins backslashes, and maps nfin to W (benchmarks/src/fixtures.rs:566–610). `library::run` and the CLI do not get that preprocessing.
6. **Sources and controlled sources are rejected.** A `V`, `I`, `E`, `F`, `G`, `H`, `B` or `K` card has no `device_kind` and returns "unknown model" (parse.rs:61–62, 178). A netlist that carries its own bias sources, or a coupling `K`, cannot be read.
7. **The SPICE title-line convention is ignored.** The first line of a SPICE deck is a title. Here it is parsed as a card. A title starting with `M`, `R`, `C`, `D`, `Q`, `L` or `X` becomes a device or an error.
8. **Net names are case-sensitive** (the `HashMap` at parse.rs:17–31). SPICE, and this crate's own simulation path (oppoint.rs:310 lowercases), treat `VDD` and `vdd` as one node. Layout and LVS see two nets; simulation sees one.
9. **Brittle tokenisation.**
   - `W = 1u` makes `W` positional and `1u` the model.
   - Inline `;`/`$` comments are not stripped.
   - `.include`, `.lib`, `.global` and `.model` are ignored.
   - A 4-node BJT (with a substrate node) silently drops the fourth node (parse.rs:88–92).
   - The `nfin` key is kept but nothing downstream reads it (grep).
10. **Unit heuristic.** A bare value ≥ 0.01 is read as µm (parse.rs:258–264). This is undocumented at the API boundary.
11. **Size semantics.** The parser stores `w` as written. Downstream, layout and LVS treat it as the *per-finger* width, while ngspice receives it as BSIM4's *total* width (AF-01).

**Tests.**
- `five_transistor_ota_shape` covers terminal order, dedup and e-notation (parse.rs:273–300).
- The X-classification tests cover sky130 names only (parse.rs:307–347).
- Nothing covers multiple subcircuits, `.param`, R/C values, case, sources, non-sky130 names or false-positive substrings.

### 2.3 `cellgen.rs` (variant spaces, merges, folds, LVS reference)

**What it does.**
- **`enumerate_folded`** (cellgen.rs:58–193). Phase 1, per multi-device unitization:
  - refuses when a member is injected or already used (cellgen.rs:90–94);
  - requires one shared source net, or else a simple series path, drawn as a chain (cellgen.rs:99–112);
  - optionally refuses distinct-gate merges (cellgen.rs:107–110);
  - draws every variant and binds pins (cellgen.rs:116–133);
  - drops alternatives that short on a shared pad, cross gate straps, or run members' currents unalike, comparing Φ exactly (cellgen.rs:137–141, 261–272).
  - Phase 2 emits cells in device order (cellgen.rs:151–187).
- **`with_per_device_sizing`** (cellgen.rs:443–591):
  - splits the annotator's unitizations per device kind;
  - adds binary DAC banks (cellgen.rs:486–505, 760–786), BJT ratio groups (cellgen.rs:508–527, 717–731), parallel MOS groups (cellgen.rs:530–549, 735–750), and one singleton per remaining device with `dev_nf = max(nf, m)` (cellgen.rs:550–577);
  - applies the fold factor (cellgen.rs:580–586).
- **Pricing and search.** `seed_assignment`/`price` run one DRC+ERC pass per alternative (cellgen.rs:327–375). `escalate` is a mixed-radix odometer, fastest digit = most distinct pin arrangements (cellgen.rs:401–437).
- **LVS reference.** `reference` emits `max(nf,m)·k` MOS cards per device at the per-finger `w` (or folded `W/k`); capacitors and inductors are skipped (cellgen.rs:858–905). `dummy_cards` adds one card per drawn dummy gate (cellgen.rs:912–931).

**Problems.**
1. **Fold classes span unrelated devices.** The class is every device of equal (kind, W, L) in the whole circuit (cellgen.rs:647–652). The aspect target is the summed finger row of the class (cellgen.rs:654, 684), whether or not those devices share a cell. Parity is circuit-wide too (`!parallel && class.len() > 1`, cellgen.rs:682). Many unrelated same-size devices push `k` down and leave long single cells.
2. **Two sources for poly R□.** The gm floor uses `pex_f32(poly, sheet_res_ohm_sq)` (cellgen.rs:639); the p2p limit uses `pdk.sheet_ohm("poly")` (cellgen.rs:638). If a deck disagrees between the two, the limits are inconsistent.
3. **The fold floor uses probe gm.** `gm_us` comes from the synthesised probe unless a testbench is given (lib.rs:274 → oppoint.rs:217–221). The gate-R floor is then a function of an arbitrary VDD/2 gate drive, and of the size-convention error (AF-01).
4. **Escalation covers a negligible slice.** With `outer_iters = 4`, the odometer yields at most 3 successors, all incrementing the fastest digit (cellgen.rs:408–412). Every other cell keeps its seeded variant for the whole run unless dp's reshape move (odd epochs only) changes it.
5. **Variant pricing is unpruned.** DRC+ERC runs on every alternative, even ones `gr::price_group` already marks unreachable (cellgen.rs:360–374). API-WISH names this "first cut if it bites" and it is still not done. `price_group` also ignores its `layers` argument (backend/gr/src/lib.rs:108).
6. **LVS reference desync.** A ratioed unitization (unit W below a member's W) desyncs the reference expansion; this is acknowledged at cellgen.rs:853–854.
7. **Literals.**
   - `P2P_SHARE = 0.55`, measured on generic_finfet only (cellgen.rs:593–597).
   - `k ≤ 64` (cellgen.rs:687).
   - The gm floor factor "a fifth", from Razavi Ex. 19.1 as the code cites (cellgen.rs:609–612, 669).
   - `draw_all` returns one empty macro when a generator enumerates nothing (cellgen.rs:815–835); `signoff` must flag it later (lib.rs:1295–1300).

**Tests** (cellgen.rs:968–1700). Each asserts what it claims:
- realize/escalate round trip and exhaustion;
- the injected single alternative;
- identity map without matching;
- mirror collapse to one ABBA cell with correct pin binding;
- quad common centroid;
- decline paths: single-finger distinct gates, injected member, source mismatch;
- the gm fold floor;
- seed determinism;
- the binary cap bank as one 2³ array.

Nothing tests `folds` across unrelated same-size devices, `reference` against a folded layout, or the chain (series stack) path.

### 2.4 `oppoint.rs` (DC operating point)

**What it does.**
- Writes a flat deck: optional `.lib`, FET cards only, the bench, `.temp`, and a `.control` block running `op` and `show m : id,vds,vdsat,gm` (oppoint.rs:217–244).
- Runs `ngspice -b` (oppoint.rs:161–165) and parses the column-oriented `show` output (oppoint.rs:410–454).
- Derives per-device power, Id, headroom and gm (oppoint.rs:180–189), terminal currents (oppoint.rs:39–57), per-net headroom (oppoint.rs:64–75) and per-net current (oppoint.rs:89–99).

**Problems.**
1. **Non-FET devices vanish from the circuit.**
   - `flat_circuit_with` skips every device for which `model_for` returns `None`, which is every non-FET (oppoint.rs:140–147, 262–264). The comment "R/C/L handled by their own cards" is false: nothing emits them.
   - `terminal_ua` then reports non-FETs as drawing zero current (oppoint.rs:45–47), so EM, IR and intent under-count the current on nets with resistors or BJTs.
2. **The device-size convention conflicts with layout and LVS (AF-01).**
   - The card emits `W={w} L={l} nf={nf}` and drops `m`/`multi` (oppoint.rs:270–286). BSIM4 reads `W` as the total width split over `nf` fingers.
   - The layout draws `max(nf,m)` fingers each `w` wide (annotator/constraints.rs:21–24, 45–54; cellgen.rs:565), and LVS expects the same (cellgen.rs:879–884).
   - In benchmarks/fixtures/ota.spice, `XM1 … W=10u nf=2` is drawn with 20 µm of channel but simulated with 10 µm; `XM5 … W=40u m=4` is drawn with 160 µm but simulated with 40 µm.
   - Every bias-derived quantity inherits the error: power (thermal), gm (fold floor), Id (EM, IR, CommonNode ΔR budget) and headroom (IR).
3. **Rail detection diverges from the annotator.**
   - `is_ground` covers `vss gnd vgnd 0 vssd vssa`; `is_supply` covers `vdd vcc vpwr vdda vddd`, exact match only (oppoint.rs:392–398).
   - The annotator also accepts `vddio avdd dvdd` and `vssio avss dvss`, plus any `<name>_` prefix (annotator/netrole.rs:18–19, 34–36), and user overrides (annotator/netrole.rs:67–68). The probe bench consults none of these.
   - Consequences: `avdd` is a supply to the router but undriven in the op deck, and `avss` is a ground to the router but not node 0.
   - Conversely, node `0` is ground to oppoint (oppoint.rs:393) but not to the annotator (`GROUND_NAMES` lacks it, annotator/netrole.rs:19). A netlist that uses `0` as ground has no Ground-class net, so antenna diodes are disabled (lib.rs:634–635), fill has no ground wires (lib.rs:412–420), and the dummy tie falls back to member terminals (cellgen.rs:957–958).
4. **One supply voltage.** Every supply is driven at `cfg.vdd` and every gate-only net at `vdd/2` (oppoint.rs:371–376). The default `vdd = 1.8` is sky130's core voltage (oppoint.rs:129).
5. **A probe bias passes as sign-off.** Every run without a testbench is sized from the probe; no fixture ships a testbench. The provenance string is honest (oppoint.rs:382–389), but `MetadataReport::certified` ignores it (metadata.rs:106–110).
6. **`show` column misalignment.** `cols = rest.iter().filter_map(instance_device)` (oppoint.rs:435) drops any column whose truncated header has lost its `.`. Every later value in that block is then attributed to the wrong device. The test fixture's headers are 21 characters wide (oppoint.rs:470), so a long instance name can trigger this. An unresolved FET also empties `pin_currents` for the whole circuit (lib.rs:1036–1038), which silently disables dr's EM sizing.
7. **Subcircuit semantics are assumed.** Every FET is emitted with an `X` prefix and the model name (oppoint.rs:269, 296–302). That is right for sky130's subcircuit primitives; it has not been verified for gf180 or IHP model libraries.
8. **Housekeeping.**
   - The deck path `temp_dir/philis_op_{pid}/op.spice` is per process, not per call (oppoint.rs:157–160). Two concurrent `run` calls in one process race on the file. `perf.rs` uses an atomic counter (perf.rs:169–173).
   - Temporary decks are never removed.
   - ngspice runs in the caller's working directory and writes `bsim4v5.out`. The file `frontend/library/bsim4v5.out` is currently present and untracked.
   - Sanitisation covers only `/ . < >` (oppoint.rs:314).
   - `param_um` falls back to sky130's L = 0.15 µm and W = 1 µm (oppoint.rs:325–329).

**Tests** (oppoint.rs:463–656): the `show` parser; power ordering; instance-name truncation; which nets the probe drives; the flat card format; KCL of terminal currents; headroom selection; the net-current rule. Nothing tests the non-FET omission, `m`/`nf` semantics, rail-name agreement with the annotator, or column dropping.

### 2.5 `perf.rs` (post-layout performance)

**What it does.** See §1.5 for the deck.
- `miss` normalises the overshoot by the bound (perf.rs:91–95).
- `sensitivities` runs a baseline plus one run per net with ground C added (perf.rs:200–223).
- `budget_rows` turns each spec with headroom into `w_i = ∓(∂f/∂C_i)/headroom` (perf.rs:230–257).

**What the simulation omits:**
- junction area and perimeter (AD/AS/PD/PS) from the drawn diffusion. Folding and diffusion sharing are the main levers on those, and they are invisible (Lampaert §2.6 eq. 2.34 and Fig. 2.9, lampaert.txt 2011–2064, PDF pp.51–52);
- gate resistance, and cell-internal li/licon resistance (acknowledged at docs/LAYOUT-FUNDAMENTALS.md:161–163);
- WPE instance parameters;
- dummies and inserted antenna diodes (`self.netlist` excludes `best.extra`, lib.rs:887);
- every non-FET device (AF-03);
- substrate and well coupling; mismatch or Monte Carlo;
- more than one corner or temperature (`OpConfig::corner` and `temp_c` are scalars, oppoint.rs:103–121).

The LOD equivalence uses the schematic `nf` (perf.rs:136), while the drawn device has `k·max(nf,m)` fingers. It reproduces the mean LOD term, not the per-finger distribution. `sd` is not emitted.

**Problems.**
1. A spec with both `min` and `max` keeps only the floor row (`(Some(lo), _)` first, perf.rs:241–245).
2. **Sensitivities cover ground C of Signal, Sensitive and Clock nets only** (lib.rs:209–216). There is no coupling-C, series-R, LOD or junction sensitivity, although the evaluation includes R and LOD. The shared budgets cannot price R, which the router controls through width.
3. **The budget and the check measure different things.** The routing rows price *estimated wire ground C*: length × `wire_af_per_um` of the lowest routing layer at minimum width (lib.rs:204, 218; lib.rs:502). Promoted epochs are then judged on the extracted matrix.
4. **The fixed 10 fF step is not scaled to node capacitance** (lib.rs:215–216). Graeb §4.2.2 eq. 83 says Δx "has to be large enough to surmount numerical noise and small enough to compare to the gradient" (graeb_centering.txt 2682–2706, PDF pp.72–73).
5. **Unbounded concurrency.** One OS thread and one ngspice process are spawned per net, all at once (perf.rs:202–211).
6. **`parse_measures` accepts any `k = v` line** (perf.rs:152–161). A log line whose key matches a spec name would be read as a measurement; `rev().find` takes the last.
7. **The spec tier is a scalar sum of normalised misses** (perf.rs:185): a weighted-sum figure of merit. The survey's §III-C shows that weighted-sum optimisation over post-layout metrics loses to multi-objective (Pareto) handling (perf_driven_survey.txt 228–240, PDF p.4).
8. **Literals and housekeeping.** `l_um` falls back to 0.15 µm (perf.rs:135). Deck files are never removed (perf.rs:170–174).

**Tests.**
- Unit tests: miss normalisation, `.measure` parsing, deck contents including `Rpex` and `sa`, and budget rows (perf.rs:259–353).
- `tests/perf_postlayout.rs`, with ngspice and sky130 models installed:
  - 200 kΩ of source R costs gain, and LOD moves gain (perf_postlayout.rs:50–72);
  - a full flow reports post-layout gain ≥ 20 dB (perf_postlayout.rs:77–98).
  - Both tests skip silently without ngspice.

### 2.6 `elaborate.rs`

**What it does.**
- `route_built`: the router-only loop (elaborate.rs:133–201).
- `routing_stack`: metals up to the first one too wide for the access pad, with the bottom conductor split off as pin access (elaborate.rs:210–240).
- `em_limits`: Black-equation derating to the op temperature (elaborate.rs:248–262).
- `stack`: per-layer C, R and antenna data (elaborate.rs:266–291).
- `intent`: supply and ground currents for signoff EM/IR (elaborate.rs:297–319).
- `antenna_diodes`: one minimum diode beside the first gate pin within 50 µm (elaborate.rs:330–373).
- `detailed_router`: pitch, width, spacing, enclosure, fatten caps, relative per-layer C and R (elaborate.rs:383–460).

**Problems.**
1. **One voltage for every supply.** `intent` gives every Supply- and Ground-class net the same `vdd_mv`; a ground flag is carried (elaborate.rs:315). Wrong for multi-rail designs.
2. **Panics on a bad deck.** `routing_stack` uses `assert!` on a deck without routable layers or with mismatched cuts (elaborate.rs:221–238). A library entry point should return an error.
3. **Minimal antenna repair.**
   - Params are built with a string-rewrite trick (elaborate.rs:363–368).
   - One minimum diode per net regardless of the overshoot (ponytail at elaborate.rs:328–329), with no check that the diode's area clears the ratio.
   - The diode's `model` is empty (elaborate.rs:366).
4. **`route_built` is routing only.** No placement search, no guard rings, no fill, no antenna repair, no EM/IR intent (`signoff` uses `Intent::default()`, elaborate.rs:90), no performance scoring.
5. **Units ambiguity.** `via_r` reads a cut's `sheet_res_ohm_sq` as a per-cut resistance (elaborate.rs:450), inherited from the deck schema.
6. **Single-temperature EM derating.** EM limits are derated to the one op temperature (elaborate.rs:248–262). Lienig's sizing uses the application temperature from the mission profile (§3.5.2, lienig_em.txt 4587–4595, PDF p.100).

**Tests.**
- `tests/elaborate.rs`: the tail net is routed.
- `ota_cross_pdk.rs`:
  - DRC is asserted on device layers and li only; met1+ is excluded as "dr debt" (ota_cross_pdk.rs:196–224);
  - the netlist export has six pins and five MOS devices (ota_cross_pdk.rs:114–178);
  - LVS clean on sky130 (ota_cross_pdk.rs:335).
- `hier_elaborate.rs`: schematic naming, the NetId invariant and privacy of internals. It *prints* LVS and asserts nothing about it (tests/hier_elaborate.rs:206–220).

### 2.7 `emit.rs` (decompiler)

**Problems.**
1. **Index mismatch on a flow `Solution`.**
   - `emit` rebuilds the cell table with `cellgen::enumerate(…, merge_distinct_gates = true)`, which uses default folds, no gm and no ground net (emit.rs:98–99; cellgen.rs:45–53).
   - It then indexes the caller's `Layout` by *its own* cell order (`l_.x[i]` for `i < instances.len()`, emit.rs:190–194).
   - A flow winner from the `apart` topology (lib.rs:180–181) or with a gm-driven fold table (lib.rs:274) can differ in cell count or order. `emit` then silently mis-lifts, or panics on an out-of-range index.
   - `philis emit` does exactly this (cli/src/main.rs:47). The only tests use hand-written layouts (tests/emit_roundtrip.rs:45–58, 118–131).
2. **Size semantics differ from the flow.** For unmatched devices, emit reads `nf` only and ignores `m` (emit.rs:147–149), while cellgen draws `max(nf, m)` (cellgen.rs:565). It also ignores the fold table. The re-elaborated device is a different device from the one the flow drew.
3. **Decisions are dropped:**
   - `Layout::orient`;
   - `Layout::variant` (instances get `pattern: None`, meaning smallest);
   - symmetry axes, DTI branches, guard rings, dummies, fold factors, antenna diodes;
   - routes: `elaborate_ir` re-routes from scratch (emit.rs:359), so SUBSTRATE3's P2 "route replay" is not implemented;
   - vertical row offsets become raw nm (`IrGap::Nm`, emit.rs:231).
4. **Scope.** The IR rejects >2-member groups, non-MOS/non-resistor kinds, ratioed pairs and mixed kinds (emit.rs:106–170).
5. **Literals.** sky130 defaults `w = 420`, `l = 150` (emit.rs:147–148).
6. **Code and comments disagree.**
   - "Nearest below with x-overlap": the filter checks only `l_.y[j] < l_.y[i]` (emit.rs:240–246).
   - The module doc says merged groups return `Unsupported` (emit.rs:10–14), but 2-member groups are emitted as `MatchedPair` (emit.rs:153–175).
7. **`to_rust` output may not compile.**
   - Ports are *every* net, internal ones included (emit.rs:282).
   - Net names become Rust field identifiers unsanitised (emit.rs:382–389). `in` (a keyword and a common input name), `0`, `a<1>` and `vout-` do not compile.

**Tests** (tests/emit_roundtrip.rs):
- the matched pair emits as one `MatchedPair` with per-leg edges;
- chain2 round-trips on sky130 with the gap attributed to `device_gap`;
- chain2 retargets to generic_finfet with row order kept and `mid` routed;
- the printed source contains the expected snippets.

Nothing compiles the generated source, and nothing feeds `emit` a `run()` output.

### 2.8 `metadata.rs`

**What it does.**
- One row per rule family and arm: total, satisfied, unknown, criticality, residual, usage (metadata.rs:129–172).
- `theta()` = Σ budget-arm residuals × 1000 (metadata.rs:94–101).
- `certified()` = nothing missing, every family met with no unknowns, every spec met (metadata.rs:106–110).
- A `Display` report.

**Problems.**
1. The `theta()` doc claims the double counting is ranking-neutral (metadata.rs:88–92). See §2.1 item 4.
2. `certified()` ignores `bias.provenance`, so a synthesised-probe bias can certify (metadata.rs:106–110).
3. `add_routing` hard-codes `Arm::Budget` (metadata.rs:206–208): correct for today's two callers, a trap for a future hard family.
4. `statuses` merges families by the trimmed kind name (metadata.rs:135, 147). Two rule types with the same last path segment would merge.

**Tests** (metadata.rs:287–461): margin versus no margin; family worst member; Θ sums residuals, not counts; the unsimulated-bias text; a boolean fail costs Θ; unknown blocks the certificate. They are good tests.

### 2.9 `fill.rs`

**What it does.** For each routing metal whose minimum-density window fits inside the block, it grows tiles by BFS out of routed ground wires (fill.rs:52–146).
- Tiles avoid foreign metal by the layer's widest spacing, sensitive wires by 3× that spacing, and matched cells entirely.
- One-tile fingers on alternate columns; no corner pinches.
- Target: the floor plus 2 % (clamped below the ceiling). A layer is dropped if ERC worsens.

**Problems.**
1. The floor is met on the **block average**, not per window (ponytail at fill.rs:49–50).
2. No DRC on the fill, only ERC (fill.rs:58, 66). Fill-to-fill, fill-to-via and maximum-density checks rely on construction.
3. `covered` counts a whole tile when any drawn shape overlaps it (fill.rs:100). This overestimates the drawn density, so fill undershoots.
4. `TILE = 2000 nm` and `KEEP_OUT = 3` are process-independent (fill.rs:28–33).
5. The only keep-outs are matched cells and sensitive nets. Fill over resistor bodies, capacitor plates and inductors is not avoided (LAYOUT-FUNDAMENTALS rows #25, #29).

**Tests** (fill.rs:196–239): a window wider than the block gets no fill; a 200 µm gf180 block with a synthetic 30 % rule reaches the band while staying clear of the signal wire and the matched cell.

### 2.10 `gds.rs` and `geometry.rs`

**gds.rs.**
1. One flat `TOP` structure of BOUNDARY rectangles (gds.rs:39–73).
2. No TEXT labels, pins, SREF/AREF or properties. External LVS cannot name nets; the xlvs example writes a separate label script instead (frontend/library/examples/xlvs_dump.rs).
3. An unmapped layer silently falls back to `(id, 0)` (gds.rs:53–56).
- Tests: record-length integrity, a parser round trip, real-number encoding (gds.rs:152–248).

**geometry.rs.**
- `collect` stamps macros and wires (geometry.rs:9–18).
- `merge_rects` merges only equal-span or contained rectangles (geometry.rs:27–52). Two overlapping nwells with different spans stay two rects for a checker that "reads a well per rect".
- `debug_check_connected` is debug-only and xy-only (geometry.rs:58–62).
- Tests: centring, offset bbox, R90 transposition, four turns giving the identity (geometry.rs:231–325).

### 2.11 CLI and examples

- **`cli/src/main.rs`.**
  - No options: `Config::default()` is hard-wired (main.rs:42).
  - It writes no GDS, no report and no metadata. It prints the hard-violation count and returns clean or not clean (main.rs:54–63).
  - Op, performance, seed, starts, iteration budgets and annotation overrides are all unreachable.
  - `emit` has the index hazard of §2.7.
- **`examples/op_demo`.** A full op + perf flow on the OTA with `gain_db ≥ 20` and `ugf ≥ 300 MHz` (op_demo/src/main.rs:26–52). It prints the metadata and a per-rule signoff histogram. Besides `perf_postlayout.rs`, it is the only place performance scoring runs as a flow.
- **`examples/three_stage_opamp`.**
  - Injects a generator-drawn 40/0.5 µm PMOS as `M8`, renaming its pins to bare `G/D/S/B` (main.rs:87–112), and opens the viewer.
  - Its doc names `macro_master::Generator`, which does not exist; the traits are `DeviceGen` or `Composition` (main.rs:46, 80).

### 2.12 `kernel/macroMaster`

**Problems.**
1. **No orientation.**
   - `Instance` has no `Orient` (lib.rs:152–159); SUBSTRATE3 marks M3 "done".
   - `place_mirrored` reflects shapes and pins, but not `units` or `dummies` (lib.rs:184–192, 342–349). For odd-`nf` MOS, the M6 OTA's `nf = 1` devices included (tests/ota_cross_pdk.rs:59–72), the mirrored partner's current then runs the other way.
   - This violates rule 5 of Hastings Table 13.2 ("Matched MOS transistors should possess equal orientations"; orientation defined by eq. 13.61; hastings.txt 42126–42212, PDF pp.708–709). The flow's own cellgen rejects exactly this in merges (cellgen.rs:256–272).
2. **Placement is translation-only and entirely the author's job.** No compaction, no search, and no check beyond bbox overlap (lib.rs:353–362). Nothing runs the annotator's placement rules on a composition.
3. **No grid check after placement.** `DeviceBuilder` grid-checks drawn rects (lib.rs:277–295), but `align`/`place_by` offsets are not re-checked (lib.rs:179–182, 365–368). `CenterHorizontal`/`CenterVertical` halve odd extents (lib.rs:201–202) and can put a whole instance off grid.
4. **Duplicate instance names merge nets silently.** Pins are keyed by `"{inst}.{pin}"` (lib.rs:450–451), and `place` does not reject a repeated name (lib.rs:353–362). Two instances named `m1` share every net.
5. **`connect` takes strings.** A typo mints a new singleton net silently (lib.rs:435–441). The typed `Io` structs are never used for checking; `ports()` returns names only.
6. **The variant library is thin.** `Mos` has `pattern` and `dummies_per_edge`; `MatchedPair` supports equal legs only, with no dummies; `Res` has no segments or pattern. There is no Cap, BJT, Diode, guard-ring or cap-array variant (lib.rs:477–702). SUBSTRATE3's M5 is partial.
7. **One opaque instance disables LVS** for the whole composition (lib.rs:96–101, 415–418). This is documented.
8. **`adapter::draw` keeps the smallest-area variant** (adapter.rs:44–49), ignoring matching quality and pin access.

**Tests** (macroMaster/src/tests.rs:51–227): overlap rejection; flush abutment legal; nets named by port; every pin bound; exact mirror symmetry of pin *positions* (not of current direction); hierarchy keeping child nets private; per-leg pins of the matched pair; the dummies parameter changing the bbox.

### 2.13 `kernel/visualizer` (skim)

- A wgpu viewer with GDS parse and flatten (SREF/AREF via `flatten_cell`, lib.rs:359), SVG export, and a channel-fed `Probe`.
- `polys_from_shapes` writes the internal `LayerId` as the GDS layer with datatype 0 (lib.rs:1189–1201), so Probe colouring does not match deck layer names.
- A debug tool; it has no bearing on flow quality.

### 2.14 Reality check of the design documents

Legend:
- **I**: implemented.
- **P**: partial.
- **N**: not implemented.
- **S**: the document is stale (the code has moved past it or removed it).
- **U**: not verifiable inside this audit's read scope.

#### 2.14.1 PLAN.md recommendations

| # | PLAN item (section) | Status | Evidence |
|---|---|---|---|
| PL-1 | Oracles as per-move signals, not terminal gates (TL;DR, §1) | **N** per move, **I** per epoch | No oracle in gp/dp (grep). Signoff runs once per epoch and only ranks (lib.rs:667–686) |
| PL-2 | Three nested tiers: variant, feasibility, PEX (§1) | **P** | Outer and middle loops at lib.rs:346–394; the inner tier is dp's SA. The middle loop is random restart, not a pump (backend/gp/src/mechanics.rs:361–371) |
| PL-3 | Variant choice joint with placement: price, seed, then move (§2) | **P** | Pricing seed (cellgen.rs:327–375); dp reshape on odd epochs only (lib.rs:356); outer odometer (cellgen.rs:401–416) |
| PL-4 | Group-coupled variant collapse (§2) | **I** | Merged matched cells (cellgen.rs:71–149); `VariantSpace` has no per-device lock (backend/gp/src/lib.rs:18–22) |
| PL-5 | Legal-by-construction grid placement (§3a) | **P** | Cut-lattice origins and deck clearance (lib.rs:529–538). Implant and well merges are patched with bridges after the fact (lib.rs:608–613) |
| PL-6 | Φ-monotone repair acceptance (§3a) | **I** | `accept` on `(hard, Φ, Θ)` (backend/dp/src/lib.rs:402) |
| PL-7 | Neighbourhood escalation when no single move lowers Φ (§3a) | **N** | Only compound symmetric moves (backend/dp/src/lib.rs:311–324); no group rip-up |
| PL-8 | `lex-min(V, Θ, PEX)` (§3b) | **I** (extended) | 5-tuple `LexKey` (lib.rs:916, 941–958); Θ weighting non-uniform (AF-12) |
| PL-9 | Augmented Lagrangian on budgets (§3b, §6) | **P** | backend/gp/src/lib.rs:83–101: placement budgets only, one-sided, two dual steps per epoch (AF-07) |
| PL-10 | Adaptive ρ: raise when stuck, *relax* when slack (§6) | **P** | Raise only (backend/gp/src/lib.rs:92–93) |
| PL-11 | LVS by construction plus a placement-time implant-merge guard (§3c) | **P** | Abutment table for dp (lib.rs:579); implant bridges after placement (lib.rs:613); LVS each epoch (lib.rs:670). No disjunctive merge guard in dp |
| PL-12 | Exact equalities by a symmetric-feasible representation (§4a, §7-1, Stage 1) | **N** | SA over absolute coordinates plus projection (backend/dp/src/lib.rs:1–8, 344) |
| PL-13 | Integer ratios by unitization (§4a) | **I** | cellgen.rs:443–591, `dac_banks`, `bjt_groups` |
| PL-14 | Disjunctive branch as a search variable (§4b) | **I** mechanism, **N** data | `try_branch` (backend/dp/src/lib.rs:537). `dti` only when the deck has `dti_max_spacing`/`dti_width` (lib.rs:504); no shipped deck does (`"dti": null`, pdks/sky130.json:46, pdks/generic_finfet.json:52) |
| PL-15 | Set-level coupling budget; two-sided centroid (§4c) | **I** | `CouplingBudget` in the routing budget arm (backend/annotator/src/extract.rs:85); `CentroidGroup` budget+cost |
| PL-16 | Thermal field per epoch, adjoint sensitivities (§4d) | **P** | Point sources (kernel/core/src/thermal.rs:1–4) refreshed per SA iteration (backend/dp/src/lib.rs:346). No PDE, no adjoint |
| PL-17 | Guard rings instantiated then constrained, every epoch (§4e) | **I** | lib.rs:606–613. No ring-*sharing* hypothesis; one ring per cell (lib.rs:1159–1169) |
| PL-18 | Injector exclusion on a <1 kΩ pad path (§4b, §4e) | **N** | `seed_isolate: false` everywhere (backend/annotator/src/emit.rs:193) |
| PL-19 | Termination: feasible + ‖Δλ‖ < δ + no improving move (§5) | **P** | lib.rs:377–383. `drift` is zero whenever the last dp layout meets every placement budget (AF-07), and "no improving move" is `PATIENCE = 8` restarts |
| PL-20 | Parallel tempering / replica exchange (§5, Stage 2) | **N** | Independent multi-start threads only (lib.rs:185–188) |
| PL-21 | Diagnose variant-space binding from a Θ floor invariant under moves (§5) | **N** | Escalation on stall *or* exhaustion *or* non-stationarity (lib.rs:377–393); no Θ-floor test |
| PL-22 | PathFinder history (§4e) | **I** routing, **P** semantics | backend/gr/src/lib.rs:36–92; carried across re-placements (AF-06) |
| PL-23 | ESD bus resistance as strict legality (§6) | **N** | No ESD or pad model in the netlist |
| PL-24 | FD-PEX gradient per candidate move (§1) | **N** | — |
| PL-25 | Stage 3 GPU oracles (§8) | **N** | The only GPU code is the visualizer |
| PL-26 | Deterministic oracles gate hard decisions (§8a) | **I** | Determinism test (lib.rs:1416–1426); CPU checker |

#### 2.14.2 API-WISH frozen decisions and debt

| Item | Status | Evidence / note |
|---|---|---|
| D1 Flat params | **I** | `gp::place` has 8 params (backend/gp/src/lib.rs:181–190); `dp::place` 9 (backend/dp/src/lib.rs:192–202) |
| D2 `Report` carries the lex tuple | **I** | kernel/core/src/report.rs:7–29 |
| D3 Negotiation owned by the orchestrator | **I** | lib.rs:342, 618, 630. Carried across *re-placements*, which voids the convergence rationale (AF-06) |
| D4 Oracle on dp (veto, merge guard, FD-PEX, epoch DRC fold) | **N**, doc **S** | No `Oracle`/`LiveOracle`/`NullOracle`/`DrcSpacing`/`drc_feedback`/`oracle_budget` in the tree (grep, 0 hits). Only per-epoch LVS/DRC ranking exists |
| D5 Guard rings as a separate `&[Macro]` | **I** | lib.rs:607, 618, 630 |
| D6 `price_group` free function | **I** | cellgen.rs:360. `reachable` is still a bool (D17 "still open") |
| D7 `Layout::variant` plus a pre-drawn space | **I** | backend/gp/src/lib.rs:18–22; the lock is gone |
| D8 gp proposes, dp searches | **I** | backend/gp/src/lib.rs:193–194 |
| D9 λ/ρ in `gp::Prices` | **P** | AF-07 |
| D10 Φ versus lex | **I** in dp | backend/dp/src/lib.rs:402. `Report::phi()` does not exist; Φ is `gp::mechanics::analog_phi` (mechanics.rs:261) |
| D11 Two explicit loops | **I** | lib.rs:346–394 |
| D12 Outer loop fires on a diagnosis | **P** | Also escalates when feasible-not-stationary (lib.rs:377–393) |
| D13 Annotator once per run | **P** | Once per `solve` (lib.rs:263, up to 2·starts), plus `performance_rows` (lib.rs:208), `emit` (emit.rs:98) and `route_built` (elaborate.rs:164). The truncation rationale is moot: no feedback batches exist |
| D14 `metadata::build` as Θ in the loop | **I** | lib.rs:671–686 |
| D15 Three arms, one arm per batch | **I**, with the documented hard+cost exception | — |
| D16 `groups` versus `abutment` | **I** | lib.rs:579, 599, 1155, 1187 |
| D17 Normalised residual; `pressure()` not a terminator | **I** | kernel/core/src/report.rs:45–47; `pressure()` only in tests (backend/gr/src/lib.rs:1213; backend/dr/src/lib.rs:2349) |
| Debt: third `Requirements` arm | **I** | backend/annotator/src/extract.rs:61–103 |
| Debt: λ/ρ carried | **I** with AF-07 | backend/gp/src/lib.rs:27–101 |
| Debt: `DtiBand` branch state | **I** | backend/dp/src/lib.rs:537; the branch table resets every epoch (backend/gp/src/mechanics.rs:379) |
| Debt: projection per SA epoch | **I** | backend/dp/src/lib.rs:344, 371 |
| Debt: `ParasiticBudget::cost` ×1e-6 | **N** (still present) | kernel/analog/src/routing/parasitic.rs:41–48 |
| Debt: `escalate` not uniform | **N** (still successor order) | cellgen.rs:401–416 |
| Debt: pricing duplicated in `macroMaster::variant_signoff` | **S** | No `variant_signoff` exists (grep, 0 hits) |
| Debt: per-alternative oracle cost; prune by routability | **N** | cellgen.rs:360–374 |
| Debt: branch table resets per epoch | **N** (still resets) | backend/gp/src/mechanics.rs:379 |
| Debt: no neighbourhood escalation | **N** | backend/dp/src/lib.rs:311–324 only |
| Debt: λ cap without reset | **N** | backend/gp/src/lib.rs:45–49, 96 |
| Debt: batch-schedule stability assertion | **N** in both tiers | No `assert` in backend/annotator/src/extract.rs (grep); `push_one` no longer exists |
| Debt: injector-on-pad seed | **N** | backend/annotator/src/emit.rs:193 |
| Debt: DTI constants from the deck | **I** mechanism, **N** data | lib.rs:504; no deck defines them |
| Debt: `CentroidGroup` penalty-only, `violations = 0` | **S** / **P** | Now budget+cost with a residual; still first moment only (kernel/analog/src/placement/cc.rs:84) |
| Debt: `MatchingPair::w_ratio` unread | **S** | The field no longer exists (grep, 0 hits) |
| Debt: `Differential` layer half unscored | **U** | Outside the read scope |
| Debt: `Antenna` nominal gate area | **S** (fixed) | Commit dad330c; `NOMINAL_GATE_AREA` is gone (grep, 0 hits) |
| Debt: gr/dr criticality blend differs | **P** | gr `score` blends `criticality·cost` (backend/gr/src/lib.rs:406); dr not re-read |
| Debt: test-compile breakage | **U** | No build was run |
| Oracle-budget valve and "diffusion-bearing" proxy debt (D4) | **S** | The oracle they describe does not exist |

#### 2.14.3 LAYOUT-FUNDAMENTALS: the second-pass top-10 status list (L156–183) and stale rows

| # | Item | Doc says | Code status | Evidence |
|---|---|---|---|---|
| LF-1 | LOD `sa/sb` computed, priced, simulated | fixed | **P** | `Unit.sa/sb/lod` (kernel/core/src/units.rs:31–32, 63); `CentroidGroup` LOD pricing (backend/annotator/src/emit.rs:224–232); `sa/sb` cards (perf.rs:132–143). Simulated only with `performance`; schematic `nf`; no `sd` |
| LF-2 | R in the performance loop | fixed (routed part) | **P** | perf.rs:116–127; lib.rs:719–757. No cell-internal or gate R; injected cells skipped (AF-32) |
| LF-3 | MOS contacting and gate R | partly | **P**, doc partly **S** | gm fold floor (cellgen.rs:662–674). The doc's "two-ended gate contacts still open" is stale: `double_gate` exists (kernel/cells/src/mosfet.rs:44–47, 129). Cell EM check **U** |
| LF-4 | Common-node R | fixed as a budget | **P** | `CommonNodes` (lib.rs:763–801); dr repair (backend/dr/src/lib.rs:555–582). Measured after placement; no star or Kelvin topology |
| LF-5 | Neighbour environment (WPE, OSE) | fixed as a budget | **P** | lib.rs:806–876. Measured after dp as routing Θ, so dp cannot act on it (AF-23); PSE absent; OSE range stood in by `lod_moat_ext_moderate` (lib.rs:872) |
| LF-6 | 2-D arrays | fixed | **I** for MOS rows, **P** overall | 2-row variants (kernel/cells/src/mosfet.rs:32–43, 108–129); second moment absent (cc.rs:84) |
| LF-7 | Metal over matched devices | fixed (cost) | **I** as a cost | backend/gr/src/lib.rs:757, 940 |
| LF-8 | General matched capacitors | fixed | **P** | Any capacitor unitization draws via `CapArray::enumerate` (cellgen.rs:803–806); auto-grouping only for binary banks (cellgen.rs:760–786); caps are skipped in LVS (cellgen.rs:870) |
| LF-9 | Resistor matching | partly; "no end dummies yet" | **P**, doc partly **S** | End dummies exist (kernel/cells/src/resistor.rs:162–175). Interdigitation is limited by the jumper-track count (resistor.rs:35–41) |
| LF-10 | BJT ratioed arrays | "still open" | **S** (implemented) | Centre-out unit fill (kernel/cells/src/bjt.rs:16–19, 206–207); `bjt_groups` (cellgen.rs:508–527) |
| §1g #65 | "no diode insertion" | open | **S** | lib.rs:632–652; elaborate.rs:330–373; tests/antenna_diode.rs |
| §1e #53 | "perf deck is C only" | open | **S** | perf.rs:116–127 |
| §1a #5 | "SA/SB never passed to SPICE" | open | **S** | perf.rs:132–143 |
| §1c #39 | "sky130 lacks S_VT" | open | **S** | A proxy `svt_uv_per_um` is flagged as not sky130 data (pdks/sky130.json:97–98) |
| K1 | "The netlist and simulation keep the schematic's W" | fixed | **P** | True, but that W is read differently by simulation and layout (AF-01) |

#### 2.14.4 CRATES.md open issues

| # | Issue | Status | Evidence |
|---|---|---|---|
| C-1 | bjt_mirror magic violations; tap → ntap/ptap | **N** / **U** | No `ntap`/`ptap` in pdks/sky130.json (grep count 0); the magic count needs a run |
| C-2 | Routing EM not wired | **S** (done) | lib.rs:304–311 (supply nets, `pin_ua`, `em`); oppoint.rs:185 (Id) |
| C-3 | Placement net weights | **S** (done) | `gp::net_weights` (backend/gp/src/lib.rs:137); lib.rs:296, 575, 595 |
| C-4 | Placer grid snap | **S** (done) | `cut_lattice` grid (lib.rs:537) |
| C-5 | LU.2 latch-up | **N** | No latch-up rule in pdks/*.json (grep) |
| C-6 | xhrpoly contacts and rbody | **P** | 190×2000 slot contacts in the recipe (pdks/sky130.json:151–152, 169); the rbody value **U** |
| C-7 | Diode anode tap ≥ 410 nm | **U** | Not visible in the diode anchors read |
| C-8 | Centroid diff pairs always discarded | **S** (fixed) | Test `a_two_finger_diff_pair_merges_as_split_gate_abba` (cellgen.rs:1466–1501) |
| C-9a | `REQUIRED_RULES` demands `bjt_*` keys | **N** | backend/verify/src/pdk.rs:1040–1044 |
| C-9b | `ir_drop` needs design intent | **S** (done) | elaborate.rs:297–319; lib.rs:1291 |
| C-10a | dp always 220 iterations | rejected by measurement | Comment at backend/dp/src/lib.rs:296–300 |
| C-10b | mosfet.rs link to `VariantSpace::lock` | **S** (fixed) | No `lock` reference in kernel/cells/src/mosfet.rs (grep) |
| C-10c | Unused `Unitization` fields | **N** | `same_variant_required` has no reader (grep) |

#### 2.14.5 SUBSTRATE3.md

| Item | Doc | Status | Evidence |
|---|---|---|---|
| M1 pins, nets, union-find | done | **I** | kernel/macroMaster/src/lib.rs:218–258, 404–470 |
| M2 `elaborate` | done | **I** (routing only; `signoff` has no intent) | elaborate.rs:124–201, 90 |
| M3 orient, mirror, centroid helper | done | **P** | No `Orient` on `Instance` (lib.rs:152–159); mirror only, which flips odd-nf current (AF-19); no centroid helper |
| M4 hierarchy | open debt | **S** (implemented) | `instantiate_comp` (lib.rs:328–338); tests/hier_elaborate.rs |
| M5 patterned library | open | **P** | Mos pattern and dummies; MatchedPair; no Cap/BJT/Diode |
| M6 cross-PDK OTA clean | done | **P** | Device layers and li only; met1+ DRC excluded (tests/ota_cross_pdk.rs:196–224) |
| B P0 round-trip harness | done | **P** | Hand layouts only (tests/emit_roundtrip.rs:45–58, 118–131) |
| B P1 placement lift | done | **I** | emit.rs:186–263 |
| B P2 route replay | not claimed | **N** | `elaborate_ir` re-routes (emit.rs:359) |
| B P3 codegen + CLI | done | **I**, with AF-08 and AF-21 | emit.rs:365–449; cli/src/main.rs:25–52 |
| B P4 second deck | done | **I** | tests/emit_roundtrip.rs:181–199 |
| "run() debug open-net panic" | open | **S** (fixed) | tests/flow_smoke.rs:1–5; lib.rs:403–408 |

---

## 3. Gap analysis against "the best constraint-aware analog P&R, better than hand layout"

This subsystem's weak points are *what enters the tool*, *what the loop learns from*, and *what the simulation believes the layout is*.

1. **Real netlists do not get in.**
   - ALIGN and MAGICAL start from a netlist and design specifications (perf_driven_survey.txt 56–72, PDF pp.1–2). LAYLA takes a netlist, a specification file and a technology file (Lampaert §1.6, lampaert.txt 1151–1170, PDF pp.31–32).
   - Philis flattens by ignoring `.subckt`. It misreads user X-instances, reads `.param` values as 0, drops R/C/L values and ports, rejects source cards, and classifies passives by sky130 substrings (§2.2).
   - Hierarchy is also a constraint source that goes unused: sibling instances of one subcircuit are matching candidates, and the port list is the I/O boundary.
2. **No hierarchical placement.**
   - The survey's chapter 2 argues that "the optimal placement of a subcircuit may not lead to the globally optimal placement" and describes simultaneous hierarchical placement with HB*-trees that carry one ASF-B*-tree per symmetry island (balasa_graeb_survey.txt 3062–3072, PDF p.77; §2.4.1, 3505–3517, PDF p.86).
   - Philis places flat. The annotator's `Problem::blocks` are used only for symmetry axes (lib.rs:580–583) and for pair measurements (lib.rs:779, 849), never as placement structure.
3. **Exactness by representation.**
   - Symmetric-feasible sequence pairs satisfy symmetry by the decoding condition (1.1) (balasa_graeb_survey.txt §1.3, 1811–1868, PDF pp.48–49).
   - Philis projects after moves (backend/dp/src/lib.rs:344), which PLAN §4a ranks second-best. The macroMaster path has neither: it mirrors by hand and does not check orientation (AF-19).
4. **Search that learns from the layout.**
   - LAYLA's placer evaluates the layout-induced performance degradation "during each iteration of a simulated annealing" run, from the geometry of the intermediate solution (Lampaert §1.6.3, lampaert.txt 1190–1199, PDF p.32).
   - Philis restarts gp every epoch (AF-06). It prices performance only on the routing tier, as estimated-wire-C rows (AF-16). It never turns signoff findings (DRC, LVS, the extracted C matrix) into next-epoch constraints: the oracle is a *selector*, not a *teacher* (AF-05).
5. **Performance-driven constraint generation, done fully.**
   - Lampaert maps specifications to bounds on all layout-induced degradations, with process variation (§2.2, lampaert.txt 1263–1300, PDF pp.34–35), using ΔP_j = Σ S_ij·x_i (eqs. 2.12–2.13, lampaert.txt 1601–1622, PDF p.42).
   - The Berkeley flow maximises the layout flexibility of the distributed bounds (eqs. 2.8–2.11, lampaert.txt 1467–1508, PDF pp.39–40). KOAN/ANAGRAM III prunes router paths whose accumulated parasitic exceeds its bound (lampaert.txt 1517–1525, PDF p.40).
   - The adjoint method needs "only two sets of equations, independent of the number of parameters" (§2.4.2.3–2.4.2.4, lampaert.txt 1676–1745, PDF pp.44–45).
   - Philis perturbs only ground C, one net per run (N+1 ngspice runs). It has no coupling-C, series-R, junction or matching sensitivities, no flexibility-weighted allocation, and no bound-pruned routing.
6. **The simulated circuit must be the drawn circuit.**
   - The standard flow simulates the extracted netlist (perf_driven_survey.txt 56–72): devices with their drawn W/L, junction area and perimeter, dummies, diodes.
   - Philis simulates schematic FETs overlaid with C, R and an LOD equivalent. The device-size convention differs from the drawing (AF-01), non-FETs are dropped (AF-03), and junction parasitics are omitted (Lampaert §2.6 eq. 2.34, lampaert.txt 2011–2064, PDF pp.51–52).
7. **Corners, temperature and statistics.**
   - Graeb defines range-parameter tolerance regions (supply, temperature) (§3.3 eqs. 8–11, graeb_centering.txt 1597–1640, PDF pp.48–49) and corner worst-case vectors (§5.1.4, graeb_centering.txt 4083–4094, PDF p.108).
   - Philis runs one corner and one temperature on a probe bias. EM is derated to that one temperature, not to a mission-profile temperature (Lienig §3.5.2, lienig_em.txt 4587–4595, PDF p.100). There is no Monte Carlo of the extracted netlist.
8. **LDE-aware placement as an objective, not a post-check.**
   - The survey cites placement that directly reduces WPE, LOD and OSE ([55]–[57]) and well-island generation during placement ([58]–[60]) (perf_driven_survey.txt 122–135, PDF pp.2–3).
   - Philis measures WPE and OSE only after dp, as routing-tier Θ (lib.rs:680, 806–876). dp cannot move toward a better environment.
9. **Designer intent input.**
   - The flow has `AnnotationConfig` (do-not-identify, rail names, `offset_sigma_mv`) but no constraint file.
   - The benchmark's `*.interface.json` sidecars carry a die outline and pin sides and layers (benchmarks/fixtures/ota_constrained.interface.json), and are ignored: "`library::run` takes none" (benchmarks/src/bench.rs:13).
   - No I/O pin placement, port-side assignment, die or aspect target, or net priority reaches the flow.
10. **Deliverables a layout team needs.** A hierarchical GDS with labels and pins; an abstract for integration; an extracted netlist and the metadata report on disk; a CLI that exposes seeds, budgets, op and performance configuration. Philis has a flat unlabelled GDS writer and a CLI that prints a count (AF-11, AF-18).
11. **Multi-objective honesty.** The weighted-sum pitfall is documented (perf_driven_survey.txt §III-C, 228–240, PDF p.4). Philis sums normalised spec misses (perf.rs:185) and breaks ties on C with a flat 2 % band (lib.rs:922).
12. **Orientation discipline everywhere.** cellgen enforces equal Φ inside merged cells (cellgen.rs:256–272). The macroMaster path mirrors odd-finger devices, violating Hastings Table 13.2 rule 5 (hastings.txt 42126–42212, PDF pp.708–709).

---

## 4. Ranked findings

| ID | Sev | Location | Finding | Suggested fix direction |
|---|---|---|---|---|
| AF-01 | critical | oppoint.rs:270–286; backend/annotator/src/constraints.rs:21–24, 45–54; cellgen.rs:565, 879–884 | **Device-size convention conflict.** Layout and LVS draw `max(nf,m)` fingers of width `w`. The op and perf decks pass `W=w nf=nf` (BSIM4 total width) and drop `m`. The OTA fixture simulates XM1 at half and XM5 at a quarter of the drawn channel width. Bias, gm, Id, power and every post-layout score describe a different circuit | Choose one canonical convention in `parse` (store total W, nf, m explicitly). Emit consistent cards in layout, LVS reference and simulation. Add a test that drawn Σ channel width equals simulated `W·m` per device |
| AF-02 | critical | parse.rs:35–39, 61–62, 95–104, 157–161, 253 | **The SPICE front end is flat-only.** It ignores `.subckt` and ports, merges subcircuits, turns user X-calls into NMOS, reads `.param` and expressions as 0, drops R/C/L values, rejects V/I/E/F/G/H/B/K cards, and ignores the title-line convention. The workarounds live only in the benchmark (benchmarks/src/fixtures.rs:566–610) | A real front end: subcircuit table and instance elaboration with hierarchical names; `.param` with expressions; R/C/L values mapped to W/L via deck recipes; `.include`/`.lib`; case folding; ports on `Netlist`; skip or record sources. Keep the hierarchy tree for the annotator |
| AF-03 | high | oppoint.rs:45–47, 140–147, 262–264 | The op and perf decks omit every non-FET device (the comment claims otherwise), and non-FETs report zero terminal current. The bias is wrong or singular for circuits with R, D, Q or C (rc_filter, bgr_core, bjt_mirror fixtures) | Emit R/C/L/D/Q cards with values and models (needs AF-02). Refuse, rather than solve, when a device cannot be emitted |
| AF-04 | high | oppoint.rs:346–398; backend/annotator/src/netrole.rs:18–19; elaborate.rs:315; metadata.rs:106–110 | Rails are found by name lists that differ between oppoint and the annotator (`avdd`, `avss`, `vddio`, `*_` prefixes, node `0`). There is one VDD for every supply, and gate-only nets are forced to VDD/2. Every run without a testbench is sized from a probe, and `certified()` still passes | Drive rails from the annotator's `NetClass`, with per-rail voltages in `OpConfig`; add `0` to the ground names. Require a testbench or a per-net bias map in sign-off mode. `certified()` must be false on a probe bias |
| AF-05 | high | lib.rs:336–339, 662–686 | The oracle only ranks epochs. No DRC, LVS or PEX finding steers the next epoch or any move. API-WISH D4's oracle does not exist, and the lib.rs comment says it does | Minimum: fold located DRC findings of epoch k into epoch k+1 as keep-out or spacing rules on the cells involved, and PEX coupling hot spots into routing costs. Full: D4's region-scoped veto in dp |
| AF-06 | high | lib.rs:352, 575; backend/gp/src/mechanics.rs:361–371; backend/gr/src/lib.rs:36–92 | Every epoch is a random restart. Routing history carried in absolute coordinates across unrelated placements, never decayed, acts as a spatial prior rather than PathFinder negotiation | Warm-start gp/dp from the incumbent (perturb and re-anneal), keeping restarts only as diversification. Reset or decay history when the placement changes beyond a threshold, or key it to cell-relative channels |
| AF-07 | high | backend/gp/src/lib.rs:83–101, 325; backend/dp/src/lib.rs:364; lib.rs:380 | **Multipliers are not an augmented Lagrangian in PLAN's sense.** λ is one-sided (`c = max(residual, 0)`, so it never relaxes). `settle` runs twice per epoch, on gp's overlapping layout and on dp's legal one, so the ρ "did not shrink" test compares residuals of two different layouts. `drift` is 0 whenever the last dp layout meets every placement budget, so "stationary" means "currently satisfied". Saturation at −64 reads as settled. Routing budgets, including the performance rows, have no multipliers | Settle once per epoch on the scored (dp) layout. Keep a residual history per batch. Use `c = residual − slack` with projection to λ ≤ 0 so prices relax. Report saturated batches as binding. Add λ/ρ for routing budgets (gr/dr costs) or state that they are rank-only |
| AF-08 | high | emit.rs:98–99, 190–194; cli/src/main.rs:47 | `emit` re-enumerates cells with default folds and the merged topology, then indexes the flow's `Layout` with that table. It can mis-lift or panic on a `run()` result, and no test covers a flow output | Carry `devices_of`, the fold table and the topology in `Solution`, and have `emit` consume them. Add a `run → emit → elaborate_ir → signoff` test |
| AF-09 | high | perf.rs:99–149; lib.rs:719–757, 887 | Post-layout simulation is the schematic plus overlays, not the extracted netlist. It omits AD/AS/PD/PS, gate R, cell R, WPE, dummies, diodes and non-FETs, at one corner. Folding and diffusion sharing are invisible to the performance tier | Simulate the LVS-matched extracted netlist (`verify::extract_spice` exists, elaborate.rs:62–83), or at least emit per-device junction geometry, `nrd/nrs`, `sd` and inserted devices. Add corner and temperature lists to `PerfConfig` (worst case over them) |
| AF-10 | high | parse.rs:189–206; pdks/gf180mcu.json:86–89; pdks/ihp_sg13g2.json:81–97 | Passives are classified by sky130 substrings. gf180 `ppolyf_u` and IHP `rsil`/`rppd` parse as NMOS (error with 2 nodes; silent with 3). User subcircuit names containing `res` or `ind` misclassify | Classify X-models from the deck's device and recipe tables (`Pdk::deck_model`, `Pdk::recipe`). Error on an unknown model instead of defaulting to NMOS |
| AF-11 | high | cli/src/main.rs:23–63 | The CLI has no flags, writes no GDS or report, and cannot enable op or performance scoring. The default flow is geometry-only: thermal vacuous, EM and IR unknown | Flags `--gds`, `--report`, `--seed`, `--starts`, `--iters`, `--op <json>`, `--perf <json>`, `--constraints <json>`. Write the metadata and the signoff breakdown |
| AF-12 | high | lib.rs:949–954; metadata.rs:88–101; lib.rs:431, 680; backend/gp/src/mechanics.rs:291–299 | Θ = stage `pt+rt` + `metadata.theta()` double-counts the annotator's budgets but single-counts CommonNodes, Environment, dr overuse and EM-underwidth, and the ranking-neutrality claim is false. `|V|` adds batch counts to finding counts | Compute Θ once, from metadata, with dr's built-in mechanics as explicit rows. Make family weights a documented policy. Count V per violated rule (or normalise) consistently |
| AF-13 | medium | lib.rs:410–420; fill.rs:52–74, 100 | Fill is added after winner selection with ERC-only checking, a block-average target and an optimistic coverage count. The final signoff can differ from the key that chose the winner | Run DRC (windowed density plus spacing) on the fill before accepting it, or fill the top candidates per epoch. Use per-window targets and exact area coverage |
| AF-14 | medium | cellgen.rs:401–416; lib.rs:70, 377–393 | The outer variant search tries at most 3 successors, all on one cell, and also fires on a feasible but non-stationary incumbent. PLAN's Θ-floor diagnosis is absent | Choose the next assignment by measured blame: cells touching unmet budgets or DRC findings, ordered by price. Escalate only on infeasible stalls |
| AF-15 | medium | cellgen.rs:593–706 | Fold classes span unrelated same-size devices; aspect and parity targets use the summed class row. Two sources for poly R□; `P2P_SHARE` was measured on one deck; the gm floor uses a probe bias | Compute folds per cell (merged group or singleton) after the merge decisions. Read one R□. Derive the p2p share from the drawn row geometry |
| AF-16 | medium | perf.rs:200–257; lib.rs:204, 209–218 | Sensitivities cover ground C of signal nets only, with a fixed 10 fF step. They price *estimated* wire C while specs are checked on extracted C. Two-sided specs lose their ceiling row. One unbounded thread and ngspice process per net | Add coupling-C and series-R perturbations (or ngspice `.sens` / an adjoint). Scale Δ to node C. Emit both bound rows. Bound concurrency with a pool |
| AF-17 | medium | parse.rs:17–31; oppoint.rs:310 | Net names are case-sensitive in the netlist and case-folded in simulation | Case-fold at parse (SPICE semantics) and keep the display name separately |
| AF-18 | medium | gds.rs:39–73 | The GDS is flat, unlabelled and pinless; unmapped layers are silently remapped | Write TEXT labels and pin shapes from `labeled_pins`, SREFs for cells and arrays; fail on unmapped layers |
| AF-19 | medium | kernel/macroMaster/src/lib.rs:152–159, 179–192, 342–362, 435–441, 450–451 | macroMaster has no `Orient`, and `place_mirrored` flips odd-finger current direction (Hastings Table 13.2 rule 5); the M6 acceptance OTA does it. Align offsets are not grid-checked. Duplicate instance names merge nets. `connect` typos mint nets silently | Add `Orient` to `Instance` plus a translate-only "copy" placement for matched partners, and check Φ equality for declared pairs. Re-check the grid on `place`. Reject duplicate names and unknown terminals |
| AF-20 | medium | lib.rs:249–458; kernel/macroMaster/src/lib.rs:328–338 | Hierarchy exists in macroMaster, but the searched flow is flat and annotator blocks are not a placement hierarchy | Use `Problem::blocks` (and parsed subcircuits, AF-02) as a placement hierarchy: symmetry islands per block, placed jointly |
| AF-21 | medium | emit.rs:10–14, 147–149, 240–246, 282, 382–389 | `to_rust` uses unsanitised identifiers (`in` fails) and exports every net as a port. emit ignores `m` and folds, uses sky130 defaults, and comments disagree with code (x-overlap, the scope doc) | Sanitise identifiers; take ports from netlist ports (needs AF-02); share the flow's size semantics; remove literals; fix the row-mate search |
| AF-22 | medium | lib.rs:176–180, 262–275; cellgen.rs:327–375 | Every `solve` re-annotates, re-enumerates and re-prices identical inputs (up to 2·starts times). `realize` clones every macro 3–4 times per epoch | Hoist annotate, enumerate and price per topology out of the per-start closure (they are pure) and share via `Arc`. Pass the realised macros through the epoch |
| AF-23 | medium | lib.rs:680, 806–876 | WPE and OSE are measured only after dp, as routing Θ, so dp cannot act on them; the OSE range is stood in by `lod_moat_ext_moderate` | Move the Environment measurement into a placement-tier rule on unit geometry in dp; add a deck key for the OSE range |
| AF-24 | medium | lib.rs:991–1028; kernel/analog/src/routing/em.rs:76–94 | The hard EM rule checks the largest single-terminal current; per-segment sums are dr Θ and so tradeable (Lienig §3.5.2 sizes from segment currents) | Promote dr's per-segment EM underwidth to the V tier, or feed dr's tree currents into the hard rule |
| AF-25 | medium | oppoint.rs:435; lib.rs:1036–1038 | `parse_show` drops a column whose truncated header lost its `.`, shifting every later value onto the wrong device. One unresolved FET empties `pin_currents` and disables dr's EM sizing silently | Keep column positions (map unresolvable columns to `None`, never drop them); use `set width` or per-device `print @m.x.m[id]` queries; make pin currents per net, as the ponytail at lib.rs:1034 suggests |
| AF-26 | medium | tests/hier_elaborate.rs:206–220; tests/emit_roundtrip.rs; tests/flow_smoke.rs:24–30; tests/tmp_deck_drc.rs | Test gaps: hierarchy LVS not asserted; emit never fed a flow output; the flow smoke test accepts `Err`; an `#[ignore]` diagnostic is checked in; nothing covers the parser on real netlists or op on passives | Assert `lvs == 0` in the hierarchy test; add flow→emit and parser tests; delete tmp_deck_drc.rs |
| AF-27 | low | oppoint.rs:157–160; perf.rs:170–174; frontend/library/bsim4v5.out | The op deck path is per process, so concurrent `run` calls race. Temporary decks are never removed, and ngspice writes `bsim4v5.out` into the caller's directory (the file is present, untracked) | Use a per-call scoped temp dir as the child's `current_dir` and remove it afterwards |
| AF-28 | low | lib.rs:48–49 vs 1071/1076; 336–339; 511–512; 910–915; 960–962; emit.rs:10–14 | Stale or contradictory comments: fallback power, oracle in dp, leak count, `LexKey`, `round_up`, emit scope | Fix the text |
| AF-29 | low | lib.rs:929–939 | `key_lt` is NaN-unsafe | Map NaN to +∞ before comparing |
| AF-30 | low | perf.rs:185; lib.rs:922 | The spec tier is a weighted-sum miss; `C_TIE` is a flat 2 % | Make it an explicit policy (spec priority order or a Pareto archive); derive the C tie from the measured seed variance |
| AF-31 | low | lib.rs:1295–1297 | The undrawable-cell message names `netlist.devices[cell index]`; wrong after a group collapse | Pass `devices_of` in `Solution` and name the members |
| AF-32 | low | lib.rs:726, 771 | Injected macros' bare `G/D/S/B` pins are skipped by `parasitics()` and `common_nodes()`: no series R in simulation and no common-node pins | Accept bare names as member 0, as `bind_pins` and `pin_currents` already do (cellgen.rs:939–947; lib.rs:1047–1049) |
| AF-33 | low | oppoint.rs:129, 325–329; perf.rs:135; emit.rs:147–148; lib.rs:531; oppoint.rs:296–302 | PDK-specific literals: VDD 1.8 V, L 0.15 µm and W 1 µm fallbacks, sky130 layer role names; the X-prefix subcircuit convention is assumed for every PDK | Read these from the deck; make the model-card convention a deck field |
| AF-34 | low | examples/three_stage_opamp/src/main.rs:46, 80; kernel/visualizer/src/lib.rs:1189–1201 | The example doc names a non-existent `macro_master::Generator`; the Probe writes the internal `LayerId` as the GDS layer | Fix the doc; map through `layer_gds` |
| AF-35 | low | elaborate.rs:221–238, 363–368 | `routing_stack` panics on bad decks; antenna diode params are built by a string trick, the model is empty, and one minimum diode is drawn regardless of the overshoot | Return `Result`; size the diode from the ratio shortfall; set the deck's diode model |

---

## 5. Questions the planners must resolve

1. **Device-size convention (AF-01).** Do input netlists follow BSIM4/xschem semantics (W total, nf splits it, m multiplies) or "W per finger"? The answer changes the layout, the LVS reference expansion (cellgen.rs:879–884), emit, and the fixtures. It must be settled before any op-derived budget is trusted.
2. **Hierarchy strategy.** Should the flow (a) flatten hierarchical SPICE with hierarchical names and use the tree only for constraint extraction, or (b) place hierarchically, in blocks or symmetry islands, and assemble? Option (b) interacts with dp's representation and with emit and macroMaster.
3. **Epoch semantics.** Keep independent restarts (then routing history must reset per epoch) or switch to warm-started refinement of the incumbent (then PathFinder history is meaningful)? This decides whether `Negotiation` stays orchestrator-owned across epochs.
4. **The multiplier layer (AF-07).** Is PLAN's augmented Lagrangian still the design? If so:
   - Which layout is the dual step taken on?
   - Should λ relax?
   - Do routing budgets (perf rows, IR, coupling, common node) get prices?
   - What does "stationary" mean, given that `drift` is currently zero on any feasible layout?
5. **The oracle's role.** Is a per-epoch DRC/LVS/PEX fold into next-epoch rules enough, or is API-WISH D4's in-dp veto still the goal? The document currently describes code that does not exist.
6. **Sign-off bias policy.** Must a run without a user testbench be refused in sign-off mode, or should the budgets derived from a probe bias (EM, IR, thermal, CommonNode, folds) be reported as *assumed*, with `certified()` false?
7. **What does "post-layout" mean?** Is the perf tier moved to the extracted netlist (junction geometry and dummies for free), or is the schematic-plus-overlay model kept and extended? Is post-layout simulation mandatory in the default flow? Its cost is one ngspice run per promoted epoch, times corners.
8. **Multi-objective policy.** Does the spec tier stay a Σ-miss scalar, or become priority-ordered or Pareto? Does C versus area stay a 2 % tie band?
9. **CLI contract.** Which artifacts are first-class: GDS with labels, an abstract, SPICE with parasitics, the metadata JSON? Which constraint-file format will users write: ALIGN-compatible, or the existing `*.interface.json` (die, pin side and layer)?
10. **Emit's role.** Is `philis emit` a product feature (then it needs the flow's cell table, orientation, variants, symmetry and routes) or a research demo (then gate it behind a feature and drop it from the CLI)?
11. **Escalation policy.** With dp's reshape move available, is the orchestrator's variant odometer worth keeping? Or should the outer loop escalate *topology* (merged versus apart, fold factor, row count) on measured blame?
12. **Θ weighting (AF-12).** Should every budget family weigh equally per unit of normalised residual, as D17 intends? If so, which Θ source survives: the stage reports or metadata? And should `|V|` count rules or batches?
13. **Fill timing.** Must fill be inside the scored loop (one DRC per promoted epoch), or is a post-selection fill with its own DRC gate and rollback acceptable?
14. **Rail naming.** One source of truth for supply and ground detection (annotator, oppoint, fill, intent), including node `0`, or explicit rails in the constraint file?
