# Audit 04: Placement (`backend/gp`, `backend/dp`)

Finding prefix: **AP-NN**. The code audited is the working tree as of 2026-09-28, including uncommitted changes. Line numbers refer to that tree. No build or test was run. Claims labelled "derived" come from reading the code and were not measured.

---

## 0. Scope

**Files in scope, each read completely, every line:**

| File | Lines | Content |
|---|---|---|
| `backend/gp/src/lib.rs` | 486 | `VariantSpace`, `Prices` (λ/ρ), `net_weights`, `Rules`, `gp::place`, `bin_overflow`, `price_tests`, `weight_tests` |
| `backend/gp/src/mechanics.rs` | 401 | `SplitMix64`, `Nets` (CSR pin model), `hpwl`, `encroach`/`encroachment`, `analog_cost`/`phi`/`theta`/`violations`, `report`, `canvas_side`, `initial_layout`, `clamp_to_die`, `snap` |
| `backend/gp/Cargo.toml` | 8 | deps: `pnr_core`, `analog` |
| `backend/dp/src/lib.rs` | 623 | `Snap`, `Sa::trial`, `dp::place`, `project_hard`, `accept`, every `try_*` move, `sym_groups`, `quarter_turn`, `rotatable` |
| `backend/dp/src/legalize.rs` | 210 | `separate_overlaps` plus 7 unit tests |
| `backend/dp/src/tests.rs` | 451 | 20 dp unit tests |
| `backend/dp/Cargo.toml` | 10 | deps: `pnr_core`, `analog`, `gp` |
| `frontend/library/src/lib.rs` | 1439 | the placement call path (`solve`, `Flow::epoch`, `place_rules`, `CellSpace::new`, `lex_key`/`key_lt`) and everything else in the file |

**Supporting reads, needed to state what gp/dp actually enforce (partial unless marked "full"):**
- `kernel/analog/src/rule.rs` 1–525 (full): the `Rule`/`RuleBatch` contract and the defaults for `residual`, `project` and `mirror_pairs`.
- `kernel/analog/src/placement/symmetry.rs` 1–372 (full), `utilization.rs` 1–84 (full), `proximity.rs` 1–48 (full), `isolation.rs` 1–38 (full), `requirements.rs` 1–29 (full), `placement/mod.rs` 1–19 (full).
- `kernel/analog/src/placement/matching_pair.rs` 1–110, `dti.rs` 1–110, `cc.rs` 1–140, `thermal.rs` 1–60.
- `kernel/core/src/layout.rs` 1–177 (full), `thermal.rs` 1–80, `macro.rs` 42–66, `geom.rs` 22–65, `report.rs` 19–47.
- `backend/annotator/src/emit.rs` 100–300 (placement emission), `lib.rs` 30–150 (`groups`/`abutment`), `block.rs` 23–68.
- `frontend/library/src/cellgen.rs` 36–140 and 320–440 (merge rules, `seed_assignment`, `realize`, `escalate`).
- `kernel/cells/src/builder.rs` 100–164 (bbox rounding, `cut_lattice`), plus a grep of `Builder::new(` over `kernel/cells/src/*.rs`. `kernel/cells/src/post_cell.rs` 375–430 (`well_bridges`).
- `docs/CRATES.md` 1–117 (full), `docs/API-WISH.md` 79–340 and 440–680, and grep hits in `PLAN.md`.
- The sky130 deck pinned by `Cargo.lock` (GPurify rev `8df8c09`): `~/.cargo/git/checkouts/gpurify-92358cf428c48214/8df8c09/pdks/sky130.deck`, lines 40 (`grid 5nm`) and 225–316 (space rules).
- Sibling audits consulted to avoid duplication: `docs/plans/audit-02-analog-rules-and-core.md` (AR-08, AR-15, AR-25) and `docs/plans/audit-07-flow-frontend-docs.md` (the Prices/drift notes and PL-12).

**Reference texts** (`scratchpad/reftext/`; PDF page = 1 + form feeds before the line):
- `balasa_graeb_survey.txt`:
  - Ch.1 (Balasa): L480–900 (p.19–27), L1811–1946 (p.48–51), L2370–2390 (p.61), L2790–2806 (p.70).
  - Ch.2 (Lin/Chang): L2980–3060 (p.75–76), L3156–3215 (p.79–80), L3259–3520 (p.81–86), L4118–4148 (p.98–99).
  - Ch.3 (Strasser et al., Plantage): L4534–4546 (p.110).
- `lampaert.txt` §4.5–4.11: L3738–5019 (p.93–119).
- `hastings.txt`:
  - §13.2.1 Orientation: L40966–41017 (p.690–691).
  - Well Proximity: L41265–41322 (p.695–696).
  - Table 13.2: L42191–42210 (p.709).
  - MOS rules 7–10: L42490–42520 (p.714).
  - Rules 14–15: L42569–42596 (p.715).
- `perf_driven_survey.txt`: L50–175 (p.1–3) and the bibliography, L385–540 (p.6–7).
- `todaes22.txt`: L105–125 (p.3–4).

---

## 1. Architecture & data flow

### 1.1 Call site, per epoch (`frontend/library/src/lib.rs`)

1. **Realize and run gp.** `Flow::epoch` realizes the macros of the current variant `assignment` (`lib.rs:574`). It then calls `gp::place(macros, variants, assignment, placement_reqs, prices, place_rules(pdk), net_weight, seed)` and discards gp's `Report` (`lib.rs:575`).
2. **Patch the coarse layout.** The library overwrites these fields of gp's coarse `Layout`:
   - `groups` becomes `cells.abutment`, the diffusion-sharing table (`lib.rs:579`).
   - `axis` is resized to the block count only if it is shorter (`lib.rs:580–583`).
   - `power_uw` and `units` are set to the cell tables (`lib.rs:584–585`).
   - `temp_mc` is refreshed (`lib.rs:586`).
3. **Run dp.** `dp::place(coarse, macros, variants_or_empty, reqs, cells.fixed, prices, place_rules(pdk), net_weight, seed)` gets the full variant space only on odd epochs (`lib.rs:356`, `590`). gp and dp receive **the same seed** (`lib.rs:575`, `596`).
4. **After dp.** `groups` is replaced by the recognition table (`lib.rs:599`). Guard rings, well bridges and implant bridges are drawn around the placed cells (`lib.rs:607–613`). Routing and signoff follow.
5. **Scoring.** The epoch is scored `(|V|, spec miss, Θ, extracted C, footprint)` (`lib.rs:686`, `916`, `929–939`). The best epoch per start wins; the best start wins (`lib.rs:185–189`).
6. **Epoch seeds and budget.** Seeds are `seed ^ iter ^ (outer << 32)` (`lib.rs:352`). Each start/topology runs up to 200 epochs × 4 variant assignments, with patience 8 (`lib.rs:65–79`, `137`).
7. **Placement rules.** `place_rules` (`lib.rs:529–538`) sets:
   - `grid = cells::builder::cut_lattice(pdk) = 2·deck grid`, which is 10 nm on sky130 (`kernel/cells/src/builder.rs:141–143`; deck line 40 `grid 5nm`).
   - `clearance = max(min_spacing)` over the roles `nwell, diff, tap, poly, nsdm, psdm, li`. On sky130 the largest of these is `nwell.2a space(nwell) >= 1270nm` (deck line 228). The next largest are `nsd.2`/`psd.2` at 380 nm (lines 314, 316), `difftap.3` at 270 nm (line 266), `poly.2` at 210 nm and `li.3` at 170 nm.
8. **Per-cell guard-ring halo.** `CellSpace::new` inflates every alternative's bbox of a ring-requesting cell by the ring halo (`lib.rs:1195–1208`), so the placer sees ringed cells as larger opaque boxes.

**Key types:**
- `pnr_core::Layout` is SoA: centre `x/y`, half-extents `hw/hh`, `orient`, `variant`, `axis` per `AxisId`, `branch`, `groups`, `power_uw`, `temp_mc`, `units` (`kernel/core/src/layout.rs:11–41`).
- `gp::VariantSpace { alternatives: Vec<Macro> }` (`gp/lib.rs:20–22`).
- `gp::Prices`: per budget batch `(kind, ordinal) → {λ ≤ 0, ρ, last residual}` (`gp/lib.rs:27–42`).
- `gp::Rules { grid, clearance }` (`gp/lib.rs:157–163`).
- `mechanics::Nets`: CSR net → (cell, pin-centroid offset from the bbox centre). One row per net touching ≥ 2 cells, with a per-net weight (`mechanics.rs:62–113`).
- `analog::Requirements { hard, budget, cost }` (`kernel/analog/src/requirements.rs:19–23`).

### 1.2 gp: the algorithm actually implemented

**Momentum (heavy-ball) gradient descent over absolute cell centres, starting from a random pile, with a hard-Φ rollback gate.** `gp/lib.rs:181–328`.

**Setup:**
- **Variants:** kept as given (`lib.rs:193–195`, D8).
- **Canvas:** square, side = √(Σ cell area / 0.4), at least the largest cell, rounded up to `grid` (`mechanics.rs:349–356`, `UTILIZATION = 0.4` at `lib.rs:168`). **Clearance is not used anywhere in gp.**
- **Initial layout:** every cell at the canvas centre ± uniform jitter of 0.15·side (`mechanics.rs:361–371`). The same function also sets:
  - `axis = [side/2; n]`, **n entries, not one per block**, all at the die centre (`mechanics.rs:378`);
  - singleton `groups` (`:380`);
  - `R0` orientation (`:381`);
  - `power_uw = 0` (`:383`);
  - empty `units` (`:384`).

**Per iteration (60 to 500 iterations):**
- **(a) Weighted pin-HPWL subgradient.** Only the pins at a net's bbox extremes get ±w (`lib.rs:226–239`). There is no smooth wirelength model (no LSE or weighted-average).
- **(b) Analog-cost gradient by central finite difference.** The probe is ±64 nm per axis per cell (`ANALOG_PROBE`, `lib.rs:176`, `242–259`). That is 4n full evaluations of `analog_cost` (Σ criticality·cost over the cost arm, plus Σ price·residual over the budget arm, `mechanics.rs:247–256`) per iteration.
- **(c) Bin-density push.** There are `nb = clamp(⌈√n⌉, 4, 24)` bins per side (`lib.rs:211`). Each cell adds its **whole** area to the bin holding its centre (`:268–270`). Every cell in a bin above `TARGET_UTIL = 0.7` gets the same push, `λ_d·over`, toward the emptiest of the four neighbour bins (`:271–290`).
- **(d) Momentum step.** Momentum is 0.85. The step is normalized so that the largest gradient component moves `step·span` (`:295–305`). If the step raises `analog_phi` = (violating hard batches, Σ their residual), it is undone, velocity is zeroed and `step` is halved (`:306–312`).
- **(e) Decay and ramp.** `step ×0.995` with floor 0.002; the density weight `λ_d ×1.05` (cap 1e3) while overflow > 0.05. The loop stops after `MIN_ITERS = 60` once overflow ≤ 0.15 (`:315–322`).

**End:** one augmented-Lagrangian dual step, `prices.settle(reqs, &coarse)` (`:325`).

### 1.3 dp: the algorithm actually implemented

**Metropolis simulated annealing over absolute coordinates** (the Gellat–Jepsen flat representation; Lampaert §4.5.1.1, `lampaert.txt` L3738–3746, p.93). It adds:
- a **lexicographic acceptance gate**;
- **projection-based repair** of hard equalities;
- a **fixed geometric schedule**;
- a **terminal grid snap and greedy pairwise-push legalizer**.

Code: `dp/lib.rs:192–367`.

**Setup:**
- **Move region:** the coarse bbox, grown about its centre until the clearance-inflated area fills 50% of it (`REGION_FILL`, `:249–272`).
- **Initial temperature:** `T0 = 0.02·mean|ΔPEX|` over 128 random probe displacements at range 0.4·span (`:276–289`).

**Schedule** (`:301–350`):
- 220 temperature steps (`MAX_ITERS`, `:21`), each of `60·n` proposals (`:23`, `:293`). That is **13,200·n proposals per dp call**.
- `T ← 0.93·T` (`:24`, `:348`).
- The displacement window starts at 0.4·span and shrinks by ×0.96 per step, with floor `grid/span` (`:26–27`, `:349`).
- After every temperature step: one `project_hard` (`:344`) and one thermal refresh (`:346`).

**Moves.** Draw `roll ∈ [0,1)` and a cell `c`; skip if `c` is fixed (`:305–309`).

| roll band | move | preserves |
|---|---|---|
| `< 0.08`, only if symmetry groups exist | compound, chosen uniformly: `try_group_shift` translates a whole group and its axis (`:473–499`); `try_pair_expand` moves a pair symmetrically in x only (`:504–523`); `try_pair_swap` swaps partners (`:527–532`) | the mirror equations, exactly |
| `< 0.70` | single-cell displacement, uniform in ±window, **not snapped** (`:326–329`) | nothing; projection repairs afterwards |
| `< 0.90` | swap two cells' centres, clamped (`:330–332`, `:417–432`) | nothing |
| `< 0.925`, only if DTI branches exist | flip one `DtiBand` share/isolate commitment (`:333–335`, `:537–539`) | geometry |
| `≥ 0.95`, odd epochs only | reshape `c` to a uniformly drawn other variant; extents and pin offsets are patched (`:336–337`, `:584–620`) | nothing |
| otherwise | quarter-turn within the reflection class, if `rotatable` (`:338–339`, `:542–578`) | never introduces a mirror |

**`Sa::trial`** (`:140–184`):
1. Save 8 columns (`:147`), then record Φ0, Θ0 and PEX0.
2. Apply the move.
3. If the **count** of violated hard batches rose, call `project` on **every** violated hard batch (`:154–159`), then restore fixed cells (`:160–163`).
4. Compute clearance encroachment for the pairs incident to moved cells, before and after (`:167–175`).
5. Accept via `accept((Φ.0, Φ.1 + encroach_nm², Θ), …)`:
   - any change in that 3-tuple decides outright (lower wins);
   - only an exact tie lets Metropolis judge ΔPEX (`:399–407`).
   - PEX = weighted pin-HPWL + priced analog cost (`:120–122`).

**Exit:**
1. Snap the axes, then snap every centre to `grid` (`:352–358`).
2. Run `legalize::separate_overlaps` for up to 64 sweeps (`:360`).
3. `prices.settle` (a second dual step in the same epoch, `:364`), then `report`.

### 1.4 Legalizer (`dp/legalize.rs:17–90`)

This is a Gauss–Seidel pairwise push. For every pair with clearance-inflated overlap:
- Push along the axis with the smaller overlap, by overlap + one grid step, split between the movable members.
- Snap after each push (`:37–65`).
- Re-project the violated hard batches and restore pinned cells (`:67–75`).

Each sweep ends in one of three ways:
- **Rollback:** if the hard-violation count rose, the sweep is rolled back and the function **returns** (`:78–84`).
- **Stall:** if encroachment did not fall, it returns (`:85–87`).
- **Continue:** otherwise it runs the next sweep.

The comment admits O(n²) per sweep and names "a constraint graph + LP compaction" as the upgrade (`:15–16`).

### 1.5 How the hard analog constraints are enforced

**Symmetry:**
- **Emission.** One `SymmetryGroup` per top-level block, `AxisId = block index` (`annotator/emit.rs:139`, `212–215`).
  - All 2-device leaves (DiffPair / CurrentMirror / Load) of the block share that axis. Nested sub-blocks are flattened into it (`emit.rs:140–144` uses `leaves(stage)`).
  - A differential stage also gets its unpaired devices as self-symmetric cells on the axis (`emit.rs:203–211`).
  - The group is registered as hard **and** cost (`emit.rs:213–214`).
- **Equality.** The rule is `x_a + x_b = 2·axis`, `y_a = y_b`, on **centres only**, with a **vertical axis only** (`symmetry.rs:7–9`, `62–68`).
- **Repair.** `SymmetryGroup::project` moves the axis to the mean of the pair midpoints and re-mirrors every pair about it, on the lattice (`symmetry.rs:133–149`). dp calls it:
  - inside `trial` when the violated-batch count rises (`dp/lib.rs:154–159`);
  - once per temperature step (`:344`, `371–397`);
  - inside the legalizer (`legalize.rs:67–71`).
- **Measure.** Its residual is the `RuleBatch` default, the violated-pair **count** (`rule.rs:141–143`; `SymmetryGroup` does not override it).

**Common centroid:**
- Exactness comes from the representation only **inside a merged cell** (`cellgen.rs:36–41`, `71–140`).
- Across separate cells, `CentroidGroup` is a **budget** (Θ) plus a cost pull (`emit.rs:216–238`). No placement move or projection targets it (`cc.rs:25–26`: "The exact equality belongs in the pattern representation").

**DTI:** `DtiBand` is hard plus cost (`emit.rs:184–194`, `240–243`). It is a disjunction handled by the `branch` flip move; it has no projection (`dti.rs:8–17`, `42–57`).

**No representation-level constraint exists.** There is no sequence pair, B*-tree, TCG or constraint graph anywhere in gp or dp.

### 1.6 Variant selection and orientation

**Variants:**
- The starting variant per cell is chosen by `cellgen::seed_assignment`, which prices each alternative in isolation: (unreachable, DRC+ERC count, congestion, HPWL) (`cellgen.rs:327–375`).
- gp never changes it (`gp/lib.rs:193`).
- On odd epochs dp reshapes **one cell at a time**, to a uniformly drawn other variant (`dp/lib.rs:598–600`).
- The outer loop steps a mixed-radix odometer over assignments (`cellgen.rs:401–416`).

**Orientation:**
- Everything starts at `R0` (`mechanics.rs:381`).
- dp only quarter-turns within the reflection class (`dp/lib.rs:542–553`), so no mirror orientation (`Mx*`) is ever introduced.
- `rotatable` forbids turning a cell that belongs to a `Layout::groups` entry of size > 1 (`:558–560`). During dp that table is `cells.abutment` (`lib.rs:579`); see AP-04.

### 1.7 Grid and clearance (open issue 4)

- **Where snapping happens:** centres after the SA (`dp/lib.rs:355–358`), symmetric pairs inside `project` (`symmetry.rs:84–89`), and each legalizer push (`legalize.rs:57–63`).
- **Where the origin comes from:** the stamping site computes it as `x − hw − anchor.x` (`kernel/core/src/macro.rs:48`), with `hw = bbox.w / 2` (`mechanics.rs:321–323`).
- **What the builder guarantees:** it rounds bbox extents to multiples of `2·process.grid()`, which is 10 nm on sky130 (`builder.rs:113–133`). Every generator constructs `Builder::new(process.grid())` with the 5 nm deck grid (grep, 12 sites). So `hw` is a multiple of 5 nm, not of 10. See AP-05.
- **Clearance:** one scalar for every pair of cells (`dp/lib.rs:130`, `legalize.rs:39–40`), taken from the widest spacing of any device layer (§1.1).

### 1.8 Net weighting (open issue 3)

- **Sources.** `gp::net_weights` (`gp/lib.rs:137–153`) weights a net by the sum of the positive performance sensitivities from `perf_rows` (`lib.rs:294–296`). Without those it uses `1/c_budget_af` of its class. Nets with neither keep weight 1.
- **Normalization.** Weights are normalized to mean 1 over the weighted nets only.
- **Use.** The weights enter placement through `Nets::weigh` (`mechanics.rs:119–124`), into both gp's HPWL gradient and dp's PEX.
- **Status.** The in-code note records measured mixed results (`gp/lib.rs:131–135`).
- **Staleness.** The weights are computed once per run (`lib.rs:296`) and never updated from extracted C.

### 1.9 Determinism and seeding

- Randomness is SplitMix64 only (`mechanics.rs:11–56`). There are no hash maps or threads inside gp or dp.
- Multi-start runs on scoped threads with derived seeds. Winners are chosen deterministically, ties to the earliest start (`lib.rs:175`, `185–189`).
- gp and dp start from the **same** seed within an epoch (`lib.rs:575`, `596`).
- Determinism is tested for dp (`tests.rs:73–81`, `381–389`) and for the whole multi-start flow (`lib.rs:1416–1426`). gp is not tested directly.

### 1.10 Runtime and scaling (derived, not measured; no placement timing exists in the repo)

**Cost per dp proposal (`trial`):**
- `analog_phi` 2–3× over all hard batches.
- `analog_theta` 2× over all budget batches.
- `pex` 2×, i.e. full HPWL over all pins plus all cost and budget batches (`dp/lib.rs:147–179`).
- `encroach_moved`, which is O(|moved|²·n) (`:125–135`).
- A clone of 8 layout columns (`:147`, `:48–57`).

In total that is O(P + R + n) per move, where P = pins and R = rule instances. Some rules cost more than O(1): `ThermalGradient::cost` reads the live field in O(n) (`thermal.rs:29–32`, `core/thermal.rs:71–80`), and `CentroidGroup` walks units.

**Aggregate:**
- dp: 13,200·n proposals per call, so O(n·(P + R + n)) per call.
- gp: ≤ 500 iterations × (4n·R + P).
- Upper bound per run: 200 epochs × 4 assignments × `starts` (3) × 2 topologies (`lib.rs:65–79`, `174–182`), cut short by patience 8.

### 1.11 Termination

- **dp** always runs all 220 temperature steps (open issue 10). The rationale is recorded at `dp/lib.rs:296–300`:
  - An early exit cost ota 5% C and 24% area.
  - Lampaert's T0 and stop rule gave ota C −12%, but rc_filter C +7% and bjt_mirror area +22%.
- **gp** stops after 60–500 iterations, on bin overflow.
- **Epoch loop:** patience 8, or "feasible and `prices.drift() < 1e-3`" (`lib.rs:370`, `377–383`).

### 1.12 Feedback into placement

What reaches gp and dp from outside:
- λ prices on the placement budget arm, carried across epochs;
- static net weights;
- the variant assignment.

What does not reach them:
- Routing congestion. `gr::Negotiation` persists across epochs but only the routers read it (`lib.rs:342`, `618`, `629–631`).
- Extracted C per net. It is used only to rank epochs (`lib.rs:955`).
- The DRC oracle of D4. It does not exist in dp (audit-07 PL-1; no `Oracle` type in the workspace).
- The previous epoch's layout. Every epoch restarts gp from a fresh random pile (`mechanics.rs:361–371`).

---

## 2. Module-by-module findings

### 2.1 `backend/gp/src/lib.rs` (486 lines)

**What it does:** `Prices` (bind/settle, keyed by `(kind(), ordinal)`), `net_weights`, `Rules`, `gp::place` (§1.2), `bin_overflow`, and tests.

**Correctness problems:**

- **Two dual steps per epoch on different layouts.**
  - `settle` runs here on gp's coarse layout (`:325`), which overlaps and is scored with singleton groups, zero power and empty units. It runs again in dp on the legal layout (`dp/lib.rs:364`).
  - The ρ-doubling test `c >= p.residual` (`:92–94`) therefore compares gp's residual with the previous dp residual, and dp's residual with gp's. It escalates on the gp-to-dp difference, not on lack of progress across epochs.
  - λ takes two steps per epoch. `drift` reports only the second (`:97–100`). (audit-07 says drift certifies gp's pile; strictly it is overwritten by dp's settle, but the λ it certifies includes the gp step.) → AP-06.
  - The update is one-sided, `c = residual.max(0)`, so λ never relaxes; saturation at `LAMBDA_MAX` reads as settled (`:47–49`, `:86–98`). Already in API-WISH §"Appended by step 2" and audit-07.
- **Axes pinned at the die centre.** gp never moves an axis, and `initial_layout` sets every entry to the die centre (`mechanics.rs:378`). `Symmetry::cost` therefore pulls **every** symmetric stage's pairs onto the same vertical line x = side/2 during gp, including unrelated stages such as a bias mirror (`symmetry.rs:19–23`, `64–67`) → AP-10.
- **Thermal and unit blind.** gp has zero power and no units (`mechanics.rs:383–384`), so `ThermalGradient` is inapplicable (`thermal.rs:45–47`) and `CentroidGroup` falls back to the box centroid and reads unknown (`cc.rs:9–11`). The library fills both only after gp (`lib.rs:584–586`) → AP-10.
- **Fixed cells are invisible to gp.** `gp::place` takes no `fixed` argument; injected macros are piled and moved like any other cell → AP-11.
- **The Φ gate is effectively inert for symmetry.** `SymmetryGroup` residual is a count (§1.5). A continuous step almost never satisfies a pair exactly, so the gate at `:306–312` only acts on measured hard residuals (`DtiBand`, on DTI decks only).
- **Density model:**
  - A cell's whole area goes into its centre bin (`:268–270`).
  - All cells of a bin get an identical push (`:271–290`), so co-located cells translate together and are never separated inside a bin.
  - Overflow is computed from the pre-step `util` (`:316`), so it is one iteration stale.
  → AP-16.
- **Gradient scaling.** The HPWL subgradient is O(w) per net. The analog FD gradient has cost units per nm (for example `Symmetry` 2·e·1e-3, so 20 per nm at a 10 µm error). The density push is `λ_d·over`. These are summed raw, then normalized by the max component (`:295–296`), so which term dominates depends on unit choices (see audit-02 AR-08).

**Placeholders / measures nothing:**
- `Rules.clearance` is unused in gp.
- gp's `Report` is built (`:326`) and discarded by the caller (`lib.rs:575`).

**Hard-coded constants:**
- Iteration and step constants `:165–176`: `MAX_ITERS 500`, `MIN_ITERS 60`, `OVERFLOW_TARGET 0.15`, `UTILIZATION 0.4`, `STEP0 0.04`, `STEP_MIN 0.002`, `STEP_DECAY 0.995`, `MOMENTUM 0.85`, `LAMBDA0 0.5`, `TARGET_UTIL 0.7`, `ANALOG_PROBE 64 nm`.
- Price constants `:44–49`: `RHO_FLOOR 0.25`, `RHO_GAIN 2`, `RHO_MAX 64`, `LAMBDA_MAX 64`.
- None is PDK-specific. `ANALOG_PROBE = 64` nm is not tied to `grid`.

**Edge cases:**
- `n == 0` returns early (`:202–205`).
- Nets touching fewer than 2 cells are dropped (`mechanics.rs:103–109`), so single-cell nets never pull.
- `nb` is clamped to 24 (`:211`), so for n > 576 the bins get coarse relative to the cells.

**Tests:**
- `rho_escalates_for_a_residual_that_does_not_shrink` (`:386–413`): λ increments grow under a constant residual.
- `drift_decreases_as_prices_settle` (`:419–440`).
- `price_survives_an_appended_batch` (`:445–461`).
- `a_tighter_budget_pulls_harder_and_sensitivities_win` (`:474–484`).
- **No test calls `gp::place`.** Nothing checks spreading, overflow reached, Φ-gate behaviour, determinism or quality → AP-22.

### 2.2 `backend/gp/src/mechanics.rs` (401 lines)

**What it does:** the shared RNG, pin-level nets, HPWL, overlap, tiered scoring, `report`, canvas, initial layout, clamp and snap.

**Correctness problems:**
- **`report` hides clearance violations.** It counts only **raw** overlap (`encroachment(l, 0)`, `:310–313`). A dp result that is clearance-illegal but not overlapping reports zero placement hard violations; only signoff DRC sees it → AP-14.
- **Hard rows count batches.** `report` emits one hard row per violated **batch** (`:291–299`), so `|V|` counts batches, not rules. A `SymmetryGroup` with 3 broken pairs counts as 1.
- **Pin offsets use rounded-down centroids.** `from_macros` integer-divides pin centroids (`:97–101`) and the bbox centre (`:200–202`). This is fine to ±1 nm, but `hw = bbox.w/2` truncates odd widths (audit-02 AR-25).
- **`snap` goes through f32** (`:398–401`). It is exact below about 2²⁴ grid units (≈ 168 mm at 10 nm), so harmless at block scale, but it is a different rounding path from `symmetry::snap_to`, which uses integer arithmetic (`symmetry.rs:153–162`). Both round half away from zero; they agree on these magnitudes.
- **The gp axis table has n entries** where one per block is meant (`initial_layout`, `:378`). This is covered by audit-02 AR-15.

**Measures nothing:** `analog_theta` sums `residual` over the budget arm, including batches whose inputs are unknown (e.g. `MatchingPair` without AVT/SVT reads 0). This is the rule crate's contract, noted only for completeness.

**Dead code:** none found. `encroachment` is used by `report` and the legalizer.

**Tests:** none in this file. It is exercised indirectly by the dp tests.

### 2.3 `backend/dp/src/lib.rs` (623 lines)

**What it does:** the SA of §1.3.

**Correctness problems:**

1. **Mirror partners reshape independently** (`:584–620`, reached at `:336–337`). `try_reshape` changes one cell's variant.
   - When a symmetric pair is two cells, both halves draw variants independently. This happens in the "apart" topology (`merge_distinct_gates = false`, `lib.rs:180`) and whenever `cellgen` declines a merge (`cellgen.rs:36–41`, `93–109`).
   - `Symmetry` checks centres only (`symmetry.rs:25–27`), so a pair with different fingering, footprint and pin faces passes as "symmetric".
   - The deleted `VariantSpace::lock` (API-WISH D7) existed for exactly this case. The test `cells_reshape_independently` (`tests.rs:143–151`) asserts independence and does not exclude mirror partners.
   → AP-03.
2. **`rotatable` reads the wrong table** (`:558–560`).
   - During dp, `l.groups` is `cells.abutment` (`lib.rs:579`, `dp/lib.rs:214`).
   - The annotator truncates any **mixed-polarity** group to its first member (`annotator/lib.rs:107–117`). A 5T OTA recognized as the composite `diff_pair_with_mirror_load` (`catalog.rs:873`) is such a group (`block.rs:42–45` makes every non-2-device match a `Group`).
   - So every cell of such a composite is rotatable, including the two halves of a diff pair drawn as separate cells. One half can turn R90 while its partner stays R0. That is the orientation mismatch Hastings forbids (§13.2.1, `hastings.txt` L40966–40973, p.690; rule 7, L42490–42498, p.714), and neither `Symmetry` nor `MatchingPair` sees it (both are centre-based).
   → AP-04.
3. **The gate mixes units** (`:177–178`). `Φ.1 + encroachment` adds a dimensionless residual (a **count** for `SymmetryGroup`) to an area in nm².
   - While a symmetry batch is already violated, which is the whole first temperature step since gp never projects, a move that breaks one more pair (+1) but removes ≥ 2 nm² of encroachment is **accepted**.
   - Once hard batches are clean, encroachment alone decides, and any overlap increase is refused. Cells cannot pass through each other except by a long jump into free space or a swap. This is the "no neighbourhood escalation" debt in API-WISH.
   → AP-07.
4. **Projection drags whole stages** (`:154–159`). It is applied to **every** violated hard batch, not just the one the move broke. `SymmetryGroup::project` also re-averages the axis over all pairs (`symmetry.rs:137–148`). Moving one member by d therefore shifts every pair of that stage by about d/(2k) and moves the proposed cell away from its proposal. `project_hard` (`:371–397`) checks Φ but not encroachment, so each temperature step can re-introduce overlaps → AP-15.
5. **Θ is decided greedily** (`:402–407`). Any Θ change decides outright, so whenever a move touches a cell of a violated budget (Utilization touches every extreme cell), the SA is greedy descent on Θ. Metropolis votes only on exact ties. Budgets are also counted in PEX through the λ weights (`:120–122` → `mechanics.rs:249–255`) → AP-13.
6. **Fixed-cell handling** (`:160–163`, `:383–388`, `:465`).
   - Fixed cells are restored after projection, but the axis is not. A symmetric group with a fixed member is dropped from the compound moves (`:465`) and can only be repaired by projection, which then fights the restore.
   - Fixed cells land wherever gp's random pile put them → AP-11.
7. **Region.** `grow` (`:261–267`) enlarges the region only when the coarse bbox is smaller than the target side; if gp spread wide, the region is the coarse bbox. The fill threshold `REGION_FILL = 0.5` is applied to clearance-inflated area, so with a large clearance the region can be much larger than a compact layout needs. Compaction then relies on HPWL and the Utilization budget (AP-02).
8. **Initial temperature** (`:276–289`). It is set by ΔPEX over displacements at range 0.4. PEX includes `ThermalGradient` at `AT_SPEC_COST = 3e5` per (ΔT/max)² (`thermal.rs:25`, `30–33`), so once power is known the temperature scale can be set by the thermal term rather than by HPWL (unit issue, AR-08) → AP-19.
9. **RNG streams.** The seed is shared with gp (`lib.rs:575`, `596`), so dp's RNG stream starts identical to gp's → AP-19.

**Measures nothing / placeholders:**
- `l.axis` beyond the used ids is carried unchanged.
- `Layout::groups` (abutment) is read only by `rotatable`. D16's "abutment permission" is not consumed anywhere in dp, because the legalizer separates every pair (`legalize.rs:1–3`).

**Hard-coded constants** (`:21–32`): `MAX_ITERS 220`, `MOVES_PER_CELL 60`, `ALPHA 0.93`, `RANGE0 0.4`, `RANGE_DECAY 0.96`, `REGION_FILL 0.5`, `LEGALIZE_SWEEPS 64`; move bands at `:310`, `326`, `330`, `333`, `336`. None is PDK-specific.

**Edge cases:**
- `n == 0` returns early (`:244–247`).
- The branch table is resized to the maximum id (`:230–237`), but the branch state is reset every epoch (API-WISH).
- `try_pair_expand` refuses `nlo >= nhi` (`:516`), so a pair cannot pass through its own axis.
- There is no vertical-only pair move. A pair's y changes only through a group shift or projection.

**Efficiency:** see §1.10. There are no incremental HPWL deltas, although `cell_nets` exists (`:82`, `:103`) → AP-08.

### 2.4 `backend/dp/src/legalize.rs` (210 lines)

**What it does:** the terminal push legalizer (§1.4).

**Correctness problems:**
- **It stops without being legal.** It returns on stall (`:85–87`) or on hard-count rollback (`:78–84`) with encroachment > 0. `dp::place` ignores the returned residual (`dp/lib.rs:360`). The caller's `debug_check_placed` then **panics** in debug builds on any raw overlap (`lib.rs:598`, `layout.rs:152–163`). In release builds the overlap becomes one hard row and clearance-only residue is not reported → AP-14.
- **Pairwise push is not global.** Pushing a away from b can push it into c. The only acceptance test is on total encroachment, so a circular or dense cluster can stall.
- **It is blind to Θ and to layer-specific spacing.** It knows one clearance. It ignores budgets, and only the hard count guards `DtiBand` and `Isolation`.
- **Fixed pairs cannot separate.** For a pair of two fixed cells, `(false, false) → (0, 0)` (`:50`), so an overlap between two pinned cells is permanent → AP-11.

**Tests (all with an empty `Requirements`):**
- `fully_overlapping_devices_are_separated` (`:115–124`).
- `a_pinned_macro_never_moves` (`:126–133`).
- `already_legal_layout_is_untouched` (`:135–143`).
- `devices_in_one_group_are_separated_too` (`:145–157`).
- `every_pair_separates_regardless_of_grouping` (`:159–172`).
- `clearance_pushes_past_mere_non_overlap` (`:174–186`).
- `separates_along_the_cheaper_axis` (`:188–209`).

Symmetry interaction is covered only through dp tests.

### 2.5 `backend/dp/src/tests.rs` (451 lines)

Test rules: `RULES = {grid: 5, clearance: 2000}` (`:4`), which is neither the flow's 10 nm lattice nor sky130's 1270 nm clearance.

**Rotation:**
- `matched_devices_never_turn` (`:54–59`): checks `rotatable` on a hand-built group table; it does not exercise the abutment truncation.
- `extents_stay_in_lockstep_with_orientation` (`:61–71`).
- `placement_is_deterministic_for_a_seed` (`:73–81`).
- `rotation_actually_happens` (`:83–87`).
- These use `macros = []`, so there are no nets and PEX is identically 0; every move is a tie accepted by Metropolis and the tests exercise a random walk.

**Reshape:**
- `reshape_changes_the_variant_and_keeps_the_extent_invariant` (`:119–130`).
- `a_fixed_cell_never_reshapes` (`:132–141`).
- `cells_reshape_independently` (`:143–151`).
- `reshape_is_priced_on_where_the_pins_land` (`:171–193`): a real pin-priced check at T = 0.

**Gate:**
- `phi_rejects_a_cost_lowering_move_that_stacks_geometry` (`:223–233`).
- `theta_outranks_pex_so_a_budget_is_never_traded_for_parasitics` (`:235–257`).
- `metropolis_still_accepts_an_uphill_move_inside_the_pex_tier` (`:259–268`).

**DTI:**
- `branch_flip_fires_and_is_priced` (`:294–305`).
- `seeds_are_written_and_table_resized` (`:308–315`).

**Symmetry:**
- `symmetry_holds_at_exit_with_room_to_move` (`:332–337`).
- `projection_never_moves_pinned_cells` (`:339–346`).
- `a_stacked_symmetric_pair_separates_and_stays_mirrored` (`:351–358`).
- `two_pairs_and_a_tail_share_one_axis_legally` (`:362–379`): 4 seeds; asserts zero hard violations and zero clearance encroachment.
- `projection_is_deterministic_for_a_seed` (`:381–389`).
- `compound_moves_keep_a_mirrored_stage_mirrored` (`:417–451`).

**Incremental encroachment:** `incident_encroachment_difference_matches_full_scan` (`:393–413`).

**Missing tests:**
- a cut-lattice origin check at the flow grid;
- mirror partners keeping equal variant and orientation;
- mixed-polarity composite rotation;
- a quality or regression bound (HPWL or area against a known packing);
- scaling or runtime;
- clearance-only residue reporting;
- a gp test of any kind;
- a combination of budgets, symmetry and realistic macros.

→ AP-22.

### 2.6 `frontend/library/src/lib.rs` (1439 lines): placement call path

**Correctness problems:**
- **`place_rules`** (`:529–538`):
  - The clearance is one scalar for all pairs: the maximum over device layers, which is nwell spacing (1270 nm on sky130) → AP-02.
  - The role list is sky130 role names; audit-07 notes that a missing role is silent.
  - `place_rules(self.pdk)` is recomputed three times per epoch (`:575`, `:594`, `:635`). The cost is negligible.
- **Per-cell guard-ring halos** (`:1195–1208`). The halo is baked into every alternative's bbox of a ring-requesting cell, rounded to `pdk.grid` (5 nm), not to the cut lattice. Consequences:
  - Neighbours can never share a ring, and ringed matched cells cannot sit inside one common ring.
  - The halo is added on top of the uniform clearance.
  → AP-02, AP-23.
- **Composite group semantics.** `coarse.groups = cells.abutment` (`:579`) is documented as "abutment permission" (`:577–578`), but dp only uses it to veto rotation (AP-04).
- **Epochs are cold restarts** (`:574–575`, `mechanics.rs:361–371`). The best layout so far never seeds the next epoch; only prices carry over → AP-09.
- **Variant reshape on alternate epochs only** (`:353–356`). This works around the Utilization floor, which is invisible mid-anneal (the comment records the measured 40% area loss on ota).
- **`round_up`'s doc comment** describes dp clearance, not rounding (`:960–965`); already in audit-07.

---

## 3. Gap analysis against "the best constraint-aware analog P&R that beats hand layout"

Each item states what an expert or a state-of-the-art placer does, what Philis does, and what would have to be built.

**G1. Exact-by-construction symmetry and compaction through a topological representation.**
- **What the literature does:**
  - *Symmetric-feasible sequence pairs.* Symmetry groups become a constraint on the code: αₓ⁻¹ < αᵧ⁻¹ ⇔ β⁻¹_{sym(y)} < β⁻¹_{sym(x)} (Balasa §1.3, eq.(1.1), `balasa_graeb_survey.txt` L1854–1860, p.49). This shrinks the search space: 35,280 of 25,401,600 codes for the 7-cell example, a 99.86% reduction (L1911–1946, p.50–51). Every decoded placement is overlap-free and symmetric. An S-F-preserving move set only has to start from an S-F code (§1.3.3, L2374–2390, p.61).
  - *Trees.* S-F binary trees are faster, but their layouts are "poorer" than S-F sequence pairs (L2794–2801, p.70).
  - *HB\*-tree.* It places each symmetry group as an ASF-B\*-tree whose packing is a **symmetry island**: each member abuts another and the group is connected (Def. 2.1, L3187–3189, p.79; Thm 2.2, L3421–3434, p.84). On the chapter's benchmarks it reports area reductions of 1.6–10.3% against SP, segment-tree, SP+LP and SP+dummy-node placers, and runtime speed-ups of 4.09× and 5.68–39.88× (L4118–4145, p.98–99).
  - *Why flat SA is weaker.* Flat absolute representations (KOAN/ANAGRAM II, PUPPY-A, LAYLA) are slow, need a post-processing gap/overlap removal, and are sensitive to the overlap weight (Balasa §1.1.5, L818–845, p.24–25).
- **What Philis does:** that flat representation, plus projection and a greedy legalizer (§1.3–1.4). PLAN §4a/§7 already names S-F sequence pairs as the top-leverage item (`PLAN.md` "Recommendations", grep line 219); audit-07 marks it **N** (PL-12).
- **Build:** a `dp` decoder from (α, β, orient, variant) to a `Layout`:
  - horizontal and vertical constraint graphs with edge weight `w_i/2 + w_j/2 + s(i,j)`, where s is the per-pair spacing of G2;
  - longest path for y;
  - for x, longest path with each group's axis equality `x_a + x_b = 2X_g`, solved by the symmetric evaluation of Balasa §1.3.1–1.3.2 (L1947–2370, p.51–61).
  - SA moves over the code: swap in α, swap in β, mirrored swap for group members, rotate and reshape by shape group.
  - The lexicographic gate stays, but symmetry and overlap become structurally zero, so the gate ranks Θ, then PEX.

**G2. Layer-aware spacing, shared wells and diffusion sharing (hand layout's main area win).**
- **What the literature and hand layout do:**
  - Legal overlap in a flat placer: terminals on the same net, the same layer and the same bulk may overlap; this is "device merging / geometry sharing" (Lampaert §4.5.2, `lampaert.txt` L3895–3905, p.96; Balasa §1.1.2 item (2), L583–588, p.19).
  - Proximity groups share one substrate or well region or one guard ring (Lin/Chang §2.1, L2997–3009, p.75).
  - Hand layout abuts NMOS in p-substrate at diffusion spacing. On sky130, `difftap.3` = 270 nm (deck line 266), and same-bulk PMOS share one nwell.
- **What Philis does:** it keeps **every** pair ≥ 1270 nm apart (§1.1). Wells are bridged only after placement, when the spans happen to be identical (`post_cell.rs:424–427`). Every ringed cell carries a private halo.
- **Build:** a spacing table `s(i,j)`, precomputed from each alternative's layer set against the deck's pairwise space rules, with two exceptions:
  - same-bulk nwell pairs get 0, because they will be merged;
  - a pair with identical diffusion net on the facing edge may abut.
  Wire it through `encroach`, `legalize`, region sizing and G1's edge weights. Add a well-island clustering term (perf survey refs [58]–[60], `perf_driven_survey.txt` L128–134, p.3).

**G3. Orientation groups, mirrored orientation, and shape groups.**
- **Lampaert §4.7** (L4132–4395, p.101–105):
  - "equal orientation groups" for matched devices;
  - "mirrored orientation groups" for symmetric couples;
  - "shape groups" so that matched devices and couples always carry equal variants;
  - symmetric translation, swap, flip and shift moves for X-type and Y-type axes (L4247–4290, p.103–104).
- **Balasa §1.1.4** (L777–800, p.23–24) distinguishes three forms:
  - *mirror* symmetry: identical geometry, mirrored orientation, which enables mirror-symmetric routing;
  - *perfect* symmetry: identical orientation, which is better against anisotropic effects;
  - *self* symmetry.
- **Hastings** requires superimposable (translation-only) matched devices (L40996–41017, p.690–691).
- **What Philis does:** it realizes only "perfect" symmetry, and only by accident of `R0` seeding. It has no shape groups (AP-03) and a broken orientation guard (AP-04).
- **Build:** derive `shape_group[cell]` and `orient_group[cell]` from `mirror_pairs` and the matched leaves. Make reshape and rotate act on the whole group. Add a per-pair policy `Mirror | Perfect`, where Mirror sets the partner to `Mx180`. Extend `Symmetry::satisfied` with `hw_a == hw_b && hh_a == hh_b && variant_a == variant_b`.

**G4. Hierarchical symmetry and proximity.** Circuit hierarchy induces nested symmetry and proximity groups: a symmetric group may contain common-centroid and symmetric subgroups (Lin/Chang §2.1, L3036–3050, p.76). HB\*-tree hierarchy nodes nest the islands (§2.4.1, L3509–3520, p.86). Philis flattens every leaf of a top-level block onto one axis (`emit.rs:139–145`) and has no horizontal axis (`symmetry.rs:7–9`).

**G5. Interconnect-area reservation (routability during placement).**
- **LAYLA** expands each cell's contour per side by a static term, the sum of the widths of the wires on that side sized from terminal current (eqs.4.4–4.5, L3944–4000, p.97–98). It adds a dynamic term from the estimated net positions, eq.4.35, which was "consistently and significantly better" than the position-only estimate (L4874–4912, p.116–117).
- **What Philis does:** it reserves only the uniform clearance and reads no congestion.
- **Build:** per-cell, per-side halos derived from the pin currents already available as `r.cfg.pin_ua` (`lib.rs:310`) and from gr's per-epoch overflow map. This needs `gr::Negotiation` to expose a gcell usage grid.

**G6. Performance-driven cost.** LAYLA's cost is a weighted sum of area, aspect ratio, overlap and a performance-degradation term. The weights are rescheduled every inner loop, with the overlap weight rising late (eq.4.6–4.8, L4398–4458, p.106–107). Philis has sensitivities only as static HPWL weights (§1.8). There is no aspect-ratio or outline target (eq.4.7) and no in-loop update of net weights from extracted C. `todaes22.txt` L113–116 (p.4) lists capacitance-sensitivity-aware placement as prior art.

**G7. Deterministic hierarchical enumeration.** Plantage recognizes building blocks, generates constraints and hierarchy, and enumerates placements guided by the hierarchy into a Pareto front over aspect ratios (Strasser et al. ch.3 abstract, L4534–4546, p.110). Philis produces one square-ish result per epoch with no outline choice.

**G8. Annealing schedule.** Lampaert (L4913–5019, p.117–119) uses:
- T0 = ΔC⁺ / ln(P0) with P0 = 0.6 (eq.4.37–4.38);
- quasi-equilibrium chain lengths [Cath88];
- adaptive α ∈ [0.8, 0.95] (eq.4.40);
- a stop when the final costs of several consecutive chains fall within an interval.

Philis runs a fixed 220 × 60n schedule. The measured regression that blocked the switch (`dp/lib.rs:296–300`) was taken with full re-evaluation per move. With incremental deltas (AP-08) the same wall time buys more starts, which changes that trade-off.

**G9. Signal-flow, current-direction and LDE-aware placement.** The performance-driven survey lists these for analytical placers:
- monotone current-flow and signal-flow placement ([48], [51]–[54]; `perf_driven_survey.txt` L112–118, p.2);
- WPE/LOD/OSE-aware placement ([55]–[57], L122–128, p.2);
- well-island generation ([58]–[60]).

Hastings puts well edges 3–5 µm from matched gates (L41265–41322, p.695–696). In Philis, `Environment` is a post-placement routing-arm report (`lib.rs:806–876`); audit-02 item 5 covers it.

**G10. Die-context rules.** Hastings rules 14–15 (L42569–42596, p.715): place matched devices on die symmetry axes, and at the end opposite multi-milliwatt power devices at "at least a micron per milliwatt". Philis models the thermal part (`ThermalGradient`, live field in dp) but has no die-axis input.

**G11. Warm start and incremental improvement.** PLAN §1 wants the topological representation carried between iterations (`PLAN.md` line 52). Philis re-piles every epoch (AP-09). A feasible incumbent is never refined further in placement; only prices move.

**G12. ALIGN / MAGICAL.**
- **In docs/ref/:** they are "two representative open-source analog PNR software tools" (`perf_driven_survey.txt` L56–57, p.1). The same survey cites "Are analytical techniques worthwhile for analog IC placement?" ([46], L504–505, p.7), "Device layer-aware analytical placement" ([47], L506–507) and "LDE-aware analytical analog placement" ([56], L537–538).
- **Not in docs/ref/ (unverified here):** their published placers run analytical global placement with symmetry as penalty, then an LP/ILP detailed placement in which symmetry axes are linear equalities and spacing is linear constraints.
- **Relevance:** the defensible, citable conclusion is the same as G1. Exactness belongs in the representation or in the LP, not in a count-residual gate.

---

## 4. Ranked findings table

| ID | Sev. | file:line | Finding | Suggested fix direction |
|---|---|---|---|---|
| AP-01 | critical | backend/dp/src/lib.rs:1-8, 140-184, 301-350; backend/dp/src/legalize.rs:17-90 | dp is flat-coordinate SA with projection repair and a greedy push legalizer. Symmetry is re-imposed rather than held; overlap is gated rather than impossible; there are no symmetry islands and no compaction. PLAN §4a/§7-1's recommended representation is absent (audit-07 PL-12). | Implement a symmetric-feasible sequence pair (Balasa §1.3, L1811–2390) or an HB\*-tree with ASF-B\*-tree islands (Lin/Chang §2.3–2.4) in dp. Decode with constraint graphs weighted by the per-pair spacing s(i,j) and the axis equalities; anneal over codes; keep `project` only for legacy inputs. |
| AP-02 | high | frontend/library/src/lib.rs:529-538, 1195-1208; backend/dp/src/lib.rs:130; legalize.rs:39-40 | One layer-agnostic clearance for every pair: the max over device layers, i.e. nwell 1270 nm on sky130 (deck L228), versus difftap 270 nm (L266). NMOS–NMOS and same-well PMOS pairs are kept ≥ 1.27 µm apart; there is no abutment and no shared wells or rings. With small cells the default `min_utilization = 0.6` (`lib.rs:76`) can be geometrically unreachable (derived: a 2×2 µm cell pitched at 3.27 µm fills 37%), leaving Θ > 0 and λ saturated. | Per-pair spacing table from each alternative's layers × the deck's pairwise space rules; 0 for same-bulk nwell (merge) and legal diffusion sharing. Use it in `encroach`, the legalizer, region sizing and G1 edges. Re-derive the Utilization floor from s(i,j). |
| AP-03 | high | backend/dp/src/lib.rs:584-620, 336-337; kernel/analog/src/placement/symmetry.rs:25-27 | Reshape picks one cell. Mirror partners drawn as separate cells ("apart" topology, `lib.rs:180`, or a declined merge) get different variants. `Symmetry` checks centres only, so the asymmetry is invisible to every check. | Shape groups from `mirror_pairs` plus matched leaves: reshape all members to the same index (the spaces are index-compatible for identical devices). Add extent and variant equality to `Symmetry::satisfied`. |
| AP-04 | high | backend/dp/src/lib.rs:555-560; frontend/library/src/lib.rs:579; backend/annotator/src/lib.rs:107-117 | `rotatable` reads the abutment table, which truncates mixed-polarity composites (e.g. `diff_pair_with_mirror_load`) to one member. Every member of such a block, including separately drawn diff-pair halves, can quarter-turn on its own: an orientation mismatch (Hastings L40966–41017; rule 7, L42490–42498). | Build orientation groups from `mirror_pairs` plus matched leaves (Lampaert §4.7) and rotate them jointly, or forbid rotation for any cell in a mirror pair. Add a test on a mixed composite. |
| AP-05 | high | backend/dp/src/lib.rs:355-358; kernel/core/src/macro.rs:48; backend/gp/src/mechanics.rs:321-323; kernel/cells/src/builder.rs:127-133 | Open issue 4 is not closed. Centres snap to the 10 nm cut lattice, but the origin `x − hw − anchor.x` lands on the lattice only if `hw` is a multiple of 10 nm. The builder rounds extents to 2·deck grid (10 nm), so `hw` is a multiple of 5 nm, and every cell whose width/10 nm is odd stamps 5 nm off-lattice (derived). | Snap the lower-left corner, not the centre (store or derive `x0 = x − hw − anchor.x`), or make `Builder::finish` round extents to 2·cut_lattice. Add `debug_assert!(origin % lattice == 0)` in `place_macro` and a dp test at `grid = 10`. |
| AP-06 | high | backend/gp/src/lib.rs:83-101, 325; backend/dp/src/lib.rs:364 | Two dual steps per epoch on different layouts: gp's overlapping pile, then dp's legal result. ρ-doubling compares gp's residual with dp's and back; `drift` reflects only the second step; λ never relaxes. | Only `bind` inside gp/dp. Run one `settle` per epoch in the library on the scored layout. Add a λ reset when a residual stalls at a positive floor (API-WISH debt). |
| AP-07 | high | backend/dp/src/lib.rs:177-178; kernel/analog/src/placement/symmetry.rs:101-150 (no `residual`) | The gate adds a dimensionless Φ margin (a pair **count** for `SymmetryGroup`) to encroachment in nm². Breaking a symmetric pair is accepted if it removes ≥ 2 nm² of overlap while the batch is already violated (the whole first temperature step). | Gate on `(hard count, Σ measured hard residual, encroachment/area_ref)` as separate lexicographic fields. Give `SymmetryGroup` a measured residual, e.g. Σ(\|ex\|+\|ey\|)/pitch. |
| AP-08 | high | backend/dp/src/lib.rs:120-184, 21-23, 301 | Every proposal re-evaluates all hard, budget and cost batches and the full HPWL, and clones 8 columns. There are 13,200·n proposals per call and a fixed 220 steps (open issue 10); there is no early exit. | Incremental ΔHPWL from `cell_nets` plus a device→rule index from `Rule::touches`; evaluate Φ/Θ/PEX only on touched rules; undo log instead of `Snap::save`. Then re-measure a frozen-detection stop (Lampaert eq.4.37–4.40) on the bench. |
| AP-09 | medium | frontend/library/src/lib.rs:574-575; backend/gp/src/mechanics.rs:361-371 | Every epoch cold-restarts gp from a random pile. The incumbent is never refined, and later epochs only re-sample with new prices. | Warm-start dp from the incumbent (low T0, small window) on non-escalation epochs; keep cold gp for diversification on a fraction of epochs. |
| AP-10 | medium | backend/gp/src/mechanics.rs:378, 383-384; backend/gp/src/lib.rs:181-190 | gp pins every axis at the die centre, so all symmetric stages are pulled onto one vertical line. gp sees zero power and no units, so it is blind to thermal and centroid terms. | Pass `axis` (as a variable, updated by the mean of pair midpoints), `power_uw` and `units` into gp, or drop gp (see AP-16). |
| AP-11 | medium | backend/gp/src/lib.rs:181-190; backend/dp/src/lib.rs:160-163, 307; legalize.rs:50 | Injected (fixed) cells are placed by gp's random pile and then frozen by dp. Two fixed cells that gp left overlapping can never separate: a permanent hard violation. | Fixed means user coordinates (add them to `Macros`). Otherwise legalize the fixed cells once, first, then freeze them. gp must honour `fixed`. |
| AP-12 | medium | kernel/analog/src/placement/symmetry.rs:7-9; backend/annotator/src/emit.rs:139-145, 212-215 | The symmetry model has a vertical axis only, with y-equality, on centres only. There is one axis per top-level block with nested leaves flattened, no hierarchy, no horizontal axis and no island adjacency. | An axis orientation per group; hierarchical groups (Lin/Chang §2.1); island adjacency via G1; a proximity budget for all matched pairs (Hastings rule 8). |
| AP-13 | medium | backend/dp/src/lib.rs:399-407, 120-122 | Any Θ change decides outright, so the SA degenerates to greedy descent whenever a move touches a violated budget. Budget residuals are also counted in PEX via λ. | Keep hard lexicographic, but put Θ inside Metropolis as the λ-priced augmented Lagrangian (PLAN §6), or use per-tier temperatures. Remove one of the two Θ counts. |
| AP-14 | medium | backend/dp/src/legalize.rs:78-87; backend/dp/src/lib.rs:360; backend/gp/src/mechanics.rs:310-313; frontend/library/src/lib.rs:598 | The legalizer can stop with residual encroachment, and the return value is ignored. `report` counts only raw overlap, so clearance-only residue is unreported; any raw overlap panics debug builds in `debug_check_placed`. | Report clearance encroachment as a hard row. Replace the push with constraint-graph longest-path/LP compaction that holds symmetry equalities (the comment at `:15–16` names it); G1 subsumes this. |
| AP-15 | medium | backend/dp/src/lib.rs:154-159, 371-397; kernel/analog/src/placement/symmetry.rs:137-148 | The in-trial projection runs on all violated hard batches and re-averages the stage axis, so a one-cell move drags the whole stage by about d/(2k). `project_hard` ignores the encroachment it creates. | Project only the batch whose count rose. For a single member, mirror its partner about the fixed current axis (`Symmetry::mirror_about`) instead of re-averaging. Include encroachment in `project_hard`'s acceptance. |
| AP-16 | medium | backend/gp/src/lib.rs:226-323 | gp's numerics are weak: HPWL subgradient only at bbox extremes, centre-binned density with no intra-bin spreading, clearance ignored, 4n full analog-cost evaluations per iteration, overflow one iteration stale. Its contribution to the final result is unmeasured and it has no test. | Measure it first (bench with gp replaced by the initial pile). Then either delete it in favour of G1's code seed, or replace it with an ePlace-style smooth WA wirelength plus electrostatic density with analytic rule gradients. |
| AP-17 | medium | frontend/library/src/lib.rs:294-296, 336-342; backend/gp/src/lib.rs:137-153 | No routability or parasitic feedback into placement: no interconnect-area reservation, no congestion input, net weights fixed per run, and the D4 oracle is absent. | Per-epoch net-weight update from extracted per-net C against budget (an AL price per net); per-side halos from pin current and gr overflow (Lampaert eq.4.4–4.5, 4.35). |
| AP-18 | low | backend/gp/src/lib.rs:137-153 | Unbudgeted nets, rails included, are fixed at weight 1 while budgeted nets are normalized to mean 1, so loose-budget signal nets can pull less than supplies. Sensitivity-derived and 1/c_budget-derived weights share one normalization. | Treat supply/ground HPWL separately (IR/EM terms); normalize each source separately; document the units. |
| AP-19 | low | backend/dp/src/lib.rs:276-289; frontend/library/src/lib.rs:575, 596 | T0 comes from ΔPEX, which mixes HPWL nm with rule costs (ThermalGradient 3e5 at spec), so the largest-unit term sets the temperature (AR-08). gp and dp share one seed. | T0 from HPWL-normalized PEX or per-term normalization; derive the dp seed as `seed ^ DP_SALT`. |
| AP-20 | low | backend/gp/src/lib.rs:157-163, 326; frontend/library/src/lib.rs:575, 577-579 | Dead or unused: `Rules.clearance` in gp; gp's `Report`; the abutment table in dp outside `rotatable`. | Remove, or consume them per G2. |
| AP-21 | low | backend/dp/src/lib.rs:36-77, 125-135 | Per-trial clone of 8 n-length vectors; `encroach_moved` is O(\|moved\|²·n). | Undo log of touched indices; a bitset for `moved`. |
| AP-22 | low | backend/gp/src/lib.rs:339-486; backend/dp/src/tests.rs:1-451 | No `gp::place` test. The dp rotation tests run with no nets, so PEX ≡ 0. Untested: lattice origin, mirror-partner variant and orientation equality, mixed-composite rotation, quality bound, runtime, clearance-residue reporting. | Add those tests, one per finding, each before its fix. |
| AP-23 | low | frontend/library/src/lib.rs:1197-1207 | The guard-ring halo is rounded to `pdk.grid` (5 nm), not the cut lattice, and baked into each alternative's bbox. No ring can be shared across a matched group of separate cells. | Round to `cut_lattice`; add a "group ring" requirement placed around a symmetry island (G1/G2). |
| AP-24 | low | backend/annotator/src/emit.rs:184-193; frontend/library/src/lib.rs:529-538 | On a DTI deck, `DtiBand`'s share branch (gap < s_max) is infeasible whenever s_max < the uniform clearance; the flip then only ever isolates. The bundled sky130 sidecar has `cell.dti = null`, so this is latent. | With G2, spacing inside a shared-trench pair comes from the trench rule, not the global clearance. |

---

## 5. Questions the planners must resolve

1. **Representation.** Replace dp's flat SA with (a) S-F sequence pairs, (b) HB\*-tree / ASF-B\*-tree with symmetry islands, or (c) keep flat SA and add an LP compaction/legalization stage (constraint graph + linear symmetry equalities)?
   - Balasa reports S-F sequence pairs give better layouts than S-F trees at higher runtime (L2794–2801, p.70); HB\*-tree reports the best area and runtime on its own benchmarks (L4118–4145).
   - The choice fixes AP-01, 07, 12, 14 and 15 together.
2. **Symmetry policy per pair.** Mirror (partner `Mx180`, mirror-symmetric routing possible; Balasa L777–790) or Perfect/translated (Hastings superimposable, L40996–41017)? Should it depend on the matching class (min/mod/exc)? And does `dr`'s mirrored differential routing assume mirror placement?
3. **Spacing source.** Compute s(i,j) from the drawn layers of each alternative × the deck's pairwise rules (exact, PDK-agnostic), or from device-type classes (NMOS/PMOS/res/cap) × a per-deck table? Which exceptions are legal to exploit: same-bulk nwell merge, same-net diffusion abutment, shared guard rings?
4. **Keep gp?** Is gp worth keeping once a topological representation exists? A benchmark ablation (gp vs pile vs constructive S-F seed) is needed; nothing in the repo measures gp's contribution.
5. **Warm starts.** Should epochs after the first warm-start from the incumbent, and how many epochs stay cold for diversity?
6. **Termination and runtime budget.** What wall-time budget per circuit size is acceptable? After incremental evaluation (AP-08), re-run the Lampaert stop-rule experiment (`dp/lib.rs:296–299`) before deciding on a schedule.
7. **Prices.** Should one `settle` per epoch happen in the library on the routed and extracted result, or on dp's layout? Should λ be allowed to relax, i.e. a two-sided multiplier with a reset on stall?
8. **Utilization floor.** Keep `min_utilization = 0.6` as a Θ budget once spacing becomes layer-aware, or replace it with an explicit outline/aspect-ratio target (Lampaert eq.4.7) or with Pareto enumeration (Plantage)?
9. **Hierarchy.** Do nested blocks need their own axes (hierarchical symmetry) and horizontal axes? That requires the annotator to emit per-sub-block `AxisId`s instead of one per top-level block (`emit.rs:139`).
10. **Fixed cells.** Where do injected macros get their positions: user coordinates in `Macros`, or placed once and then locked?
11. **Net weights.** Should supply/ground nets carry any HPWL weight? Should net weights be updated per epoch from extracted C against the per-net budget?
12. **Lattice fix location.** Should the lattice fix (AP-05) go in the placer (snap corners) or in the builder (extents at 2·lattice, which is more area per cell)?
