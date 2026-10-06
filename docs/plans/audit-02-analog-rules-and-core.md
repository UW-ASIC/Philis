# Audit 02: constraint models (`kernel/analog`) and core data model (`kernel/core`)

Scope: every rule type in `kernel/analog/src/**` and every module in `kernel/core/src/**`, read line by line on the current working tree (2026-09-28). The tree includes uncommitted and untracked files (`placement/{environment,utilization}.rs`, `routing/{common_node,em,ir,performance,shield,stack}.rs`, `core/{lanes,units}.rs`); `routing/align.rs` is deleted. Finding IDs are `AR-NN`. IDs AR-01..AR-35 match the earlier draft of this file (same findings, re-verified); AR-36..AR-46 are new in this revision. Code citations are `path:line` from the repo root. Reference citations give source, section/equation, the line range in the extracted `reftext/*.txt`, and the PDF page (1 + form-feeds before the line).

---

## 0. Scope

### 0.1 Files in scope (all read completely)

| File | Lines | `#[test]`s |
|---|---:|---:|
| kernel/analog/src/lib.rs | 23 | 0 |
| kernel/analog/src/cell.rs | 64 | 0 |
| kernel/analog/src/constraints.rs | 11 | 0 |
| kernel/analog/src/metadata.rs | 26 | 0 |
| kernel/analog/src/requirements.rs | 29 | 0 |
| kernel/analog/src/rule.rs | 525 | 7 |
| kernel/analog/src/placement/mod.rs | 19 | 0 |
| kernel/analog/src/placement/cc.rs | 397 | 9 |
| kernel/analog/src/placement/dti.rs | 188 | 4 |
| kernel/analog/src/placement/environment.rs | 96 | 1 |
| kernel/analog/src/placement/isolation.rs | 38 | 0 |
| kernel/analog/src/placement/matching_pair.rs | 161 | 2 |
| kernel/analog/src/placement/proximity.rs | 48 | 0 |
| kernel/analog/src/placement/symmetry.rs | 372 | 9 |
| kernel/analog/src/placement/thermal.rs | 134 | 3 |
| kernel/analog/src/placement/utilization.rs | 84 | 1 |
| kernel/analog/src/routing/mod.rs | 25 | 0 |
| kernel/analog/src/routing/antenna.rs | 67 | 0 |
| kernel/analog/src/routing/common_node.rs | 123 | 1 |
| kernel/analog/src/routing/coupling.rs | 192 | 6 |
| kernel/analog/src/routing/crosstalk.rs | 115 | 0 |
| kernel/analog/src/routing/differential.rs | 198 | 2 |
| kernel/analog/src/routing/em.rs | 243 | 5 |
| kernel/analog/src/routing/ir.rs | 94 | 1 |
| kernel/analog/src/routing/parasitic.rs | 126 | 1 |
| kernel/analog/src/routing/performance.rs | 95 | 1 |
| kernel/analog/src/routing/shield.rs | 134 | 1 |
| kernel/analog/src/routing/stack.rs | 532 | 7 |
| kernel/core/src/lib.rs | 30 | 0 |
| kernel/core/src/ids.rs | 47 | 0 |
| kernel/core/src/geom.rs | 146 | 3 |
| kernel/core/src/layout.rs | 177 | 0 |
| kernel/core/src/units.rs | 189 | 2 |
| kernel/core/src/thermal.rs | 171 | 4 |
| kernel/core/src/lanes.rs | 224 | 2 |
| kernel/core/src/hypergraph.rs | 57 | 0 |
| kernel/core/src/macro.rs | 66 | 0 |
| kernel/core/src/netlist.rs | 50 | 0 |
| kernel/core/src/process.rs | 80 | 0 |
| kernel/core/src/report.rs | 48 | 0 |
| kernel/core/src/routes.rs | 83 | 0 |
| kernel/core/src/unionfind.rs | 56 | 0 |
| **Total** | **5583** | **71** |

Also read: `kernel/analog/Cargo.toml`, `kernel/core/Cargo.toml` (pnr_core depends only on `fearless_simd`; analog only on pnr_core).

### 0.2 Files consulted outside scope (partial, to establish how the rules are produced and consumed)

- backend/annotator/src/emit.rs:1–330 (placement rule emission, tiers, η, thermal limit, Isolation).
- backend/annotator/src/extract.rs:28–105 (routing rule arms), classify.rs:26–33, 85–107 (net budgets), lib.rs:100–116, 155–161 (groups/abutment, `gate_um2`), constraints.rs:22–60 (unitization, `fingers`).
- frontend/library/src/lib.rs:252–296 (Utilization, PerformanceBudget, IrDrop registration), 570–600 (gp→dp hand-off: groups, axis resize, power, units), 760–876 (CommonNodes and Environment producers), 990–1027 (Electromigration producer), 1120–1160 (retarget), 1235–1263 (retarget debug check).
- frontend/library/src/cellgen.rs:118–150, 255–272 (merge decision, `currents_run_alike`); oppoint.rs:265–285 (W/nf passed to simulation).
- kernel/cells/src/mosfet.rs:70–135, 730–750 (variant styles; `centroid_sequence` feasibility).
- backend/gp/src/mechanics.rs:243–256 (objective), 310–330 (`variant_extents` = bbox/2), 370–387 (initial Layout: axis sized n, power 0, units empty); gp/src/lib.rs:72, 115–116 (`kind()` price keying).
- backend/dp/src/lib.rs:435–470 (`sym_groups` from `mirror_pairs`), 540–575 (`quarter_turn`, `rotatable`, `try_rotate`); dr/src/lib.rs:1823–1856 (EM shortfall reported in Θ).
- backend/verify/src/pdk.rs:575–614 (antenna rule selection); pdks/*.json (grep: `avt_*`, `svt_uv_per_um` and its PROXY note, `lod_kvth0_*`, `vt_tc_*`, `dti`, `wpe_clearance_moderate`, `lod_moat_ext_moderate`, `blech`).
- git: `git show --stat c169ef3 0ae58c0 dad330c`; `git status` for kernel/.

### 0.3 References used (verified against reftext; PDF page by form-feed count)

- **Hastings 3e** (hastings.txt): eq. 8.23 thermal mismatch ∝ TC·d·∂T/∂x and the four ways to reduce it, L23120–23142 (p.388); Table 8.4 four CC rules, L23427–23443 (p.392); rule of coincidence "should be rejected" text, L23417–23421 (p.392); Taylor residue eq. 8.26, "residual mismatch … proportional to the square of the dimensions of the array", L23480–23495 (p.393); optimal dispersion exists but "seldom possible to compute", ABBA/AB-BA best without strong gradients, L23505–23514 (p.393); compactness, L23529–23531 (p.394); arrays cancelling quadratic terms, L23541–23548 (p.394); orientation Φ eq. 13.61, L42155–42186 (p.708–709); Table 13.2 five MOS CC rules incl. ORIENTATION, L42191–42210 (p.709); §13.3 MOS matching rules: r7 same orientation L42490–42498, r8 proximity L42499–42503, r9 compactness/aspect ratio L42504–42516, r10 2-D CC L42517–42524, r12 dummies and moat extension (LOD) L42532–42553 (p.713–714), r13 low-stress-gradient location L42554–42568, r14 power devices (≥1 µm per mW for moderate matching) L42569–42589, r15 die axes L42590–42595, r16–r18 no contacts/metal over active gates, dummy-metal block L42596–42624, r19 well edges / WPE L42625–42631 (p.714–715); §13.2.2 WPE L41265–41335 (p.695–696), LOD L41367–41425 (p.697–698); antenna peripheral/areal ratios per layer per node, L13128–13160 (p.228); diode/junction protection and metal/diffusion rules, L13205–13225 (p.229); EM width eq. 15.24 and derating eq. 15.25 with the 398 K example (0.58), L48763–48790 (p.821).
- **Pelgrom & Duinmaijer, ESSCIRC'88** (pelgrom.txt): S-term from a cone-shaped wafer distribution modelled as a long-correlation stochastic process, L33–43 (p.1); test distances 30/250/500 µm, L80–84 (p.2); distance effect "only significant for large area devices with a considerable spacing", S_VT0 ≈ 4 µV/µm, L88–90 (p.2); rotation affects only the current factor, L98–99 (p.2).
- **Schaper & Linnenbank, distance vs pair mismatch** (distance_vs_pair_mismatch.txt): distance mismatch measured over mm-scale test macro (10 mm), L48–62 (p.1); distance mismatch is systematic, pair mismatch statistical, variances add, L76–101 (p.2); Table 1 S_D in mV/mm, L179–198 (p.3).
- **Schaper et al., long-distance mismatch** (long_distance_mismatch.txt): per-chip patterns from none to linear to nonlinear, changing chip to chip, orientation-dependent, L185–200 (p.4).
- **CC review, Karmokar et al.** (cc_review.txt): linear oxide gradient model with angle θ eqs. (8)–(9), worst-ratio systematic metric eq. (10), LOD eq. (11), WPE "equal well spacing", OSE "same OD width and spacing", L90–140 (p.2); spatial-correlation model eqs. (12)–(19) and ratio variance eq. (16), L145–215 (p.2–3); dispersion ↔ correlation, L317–321 (p.3).
- **Dai et al., Nth-order central symmetric pattern** (nth_order.txt; abstract checked on PDF p.1): cancels up to n-th order gradients with 2ⁿ unit cells per device (extracted text reads "2n"), L10–16 (p.1); polynomial gradient model and matching condition eqs. (1)–(6), L70–92 (p.2).
- **Lampaert, Gielen, Sansen** (lampaert.txt): area C eq. 2.29, lateral C eq. 2.30 with fitted F(d) = C0 + C1/d + … + C4/d⁴ eq. 2.31, fringe eq. 2.32, L1920–1973 (p.49–50); R = ρ□·l/w eq. 2.33, L1998–2007 (p.51); sensitivity-mapped parasitic constraints, L1098–1108 (p.30); net coupling C = Σ parallel run·Cc(D) + overlap-area C eq. 4.18, and performance degradation via sensitivities to ground C, R and coupling C eq. 4.22, L4660–4717 (p.111–112); mismatch degradation eq. 4.25–4.27 (Σ over matched pairs of |S|·3σ, distance term recomputed per placement), L4718–4762 (p.112–113); finite-die thermal Fourier series eq. 4.28 with DCT-accelerated unit-source profiles, L4765–4830 (p.113–114); symmetric (mirrored) routing, L6199–6225 (p.148).
- **Lienig & Thiele** (lienig_em.txt): segment worst-case currents from LHS/RHS terminal sets eqs. 3.5–3.7, L3975–4052 (p.87–88); wire width from equivalent (RMS/avg) **and** peak current eqs. 3.21–3.23, effective width with h_nom/h_min, Δw, etch loss eq. 3.24, via count eq. 3.25 with inhomogeneity g(H), temperature factor eq. 3.26, L4589–4660 (p.100–101); Blech product eq. 4.1, L5405–5425 (p.118).
- **Charbon et al.** (charbon_substrate.txt): isolation independent of distance past 2.5–5× epi thickness (Su: 4×) on low-resistivity epi, L3286–3296 (p.127); with surface conduction isolation keeps improving with distance, L3346–3351 (p.130).
- **Balasa/Graeb (eds.) survey** (balasa_graeb_survey.txt): mirror vs perfect (identical-orientation) vs self-symmetry, L675–711 (p.21–22); symmetric-feasible sequence pair eq. (1.1), L1850–1870 (p.49); symmetry island Def. 2.1 and Pelgrom-based motivation, L3156–3190 (p.79–80).
- **Performance-driven survey (Zhu et al.)** (perf_driven_survey.txt): symmetry islands, CC, array regularity, proximity, boundary constraints; symmetric pair routing; no routing over active; shielding; exact per-layer routing; monotonic current flow; signal flow; LDE-aware placement (WPE/LOD/OSE), L70–128 (p.2).

Items marked **(GK)** in §3 are general knowledge of named tools, not from docs/ref/.

---

## 1. Architecture and data flow

### 1.1 The seam

- `Rule` (kernel/analog/src/rule.rs:9–130) is a `Copy` value with `type On` = `Layout` (placement) or `Routes` (routing). Methods: `cost` (objective, raw units), `satisfied` (legality), `headroom` + `margin` (criticality), `residual` (overshoot / own spec, unclamped), `usage` (report), `applicable` (justified "nothing to check"), `known` (inputs exist), `project` (exact repair), `touches` (ids), `retarget` (device → cell), `branch` / `mirror_pair` / `keepaway` / `shield` (hooks for dp/gr/dr), `extract` (self-extraction from a `BipartiteHypergraph`).
- `RuleBatch<On>` (rule.rs:135–208) is the type-erased per-kind array; the blanket impl for `Vec<R: Rule>` is rule.rs:210–277. Six kinds implement `RuleBatch` directly: `CentroidGroup` (cc.rs:152), `SymmetryGroup` (symmetry.rs:103), `Environment` (environment.rs:55, generic over any state), `Utilization` (utilization.rs:28), `CommonNodes` (common_node.rs:61), `PerformanceBudget` (performance.rs:36).
- `Requirements<On>` (requirements.rs:19–23): arm = tier. `hard` (V), `budget` (Θ, Σ residual priced by `gp::Prices` keyed on `(kind(), ordinal)`), `cost` (PEX). gp's objective is `Σ_cost criticality·cost + Σ_budget λ·residual` (backend/gp/src/mechanics.rs:247–255); the report cost adds HPWL in nm (mechanics.rs:314).
- `rule_residual` floors a violated rule's residual at 1e-3 (rule.rs:295–306); `over(excess, budget)` normalizes and returns 0/1 for a non-positive budget (rule.rs:313–318).
- `Report::lex` = `(|V|, Σ Θ milli-budgets, PEX)` compared lexicographically (kernel/core/src/report.rs:19–22); `Violation::from_residual` stores ⌈1000·residual⌉ (report.rs:45–47).

### 1.2 Flow through this code

1. The annotator builds placement rules on schematic `DeviceId`s (backend/annotator/src/emit.rs:128–244) and routing rules on `NetId`s (extract.rs:28–105); the library adds Utilization, IrDrop, EM and PerformanceBudget (frontend/library/src/lib.rs:264–290, 990–1027).
2. Cells are drawn; pair members may be merged into one cell. Every placement batch is `retarget`ed with `cell_of` (frontend/library/src/lib.rs:1146–1153); `Target::retarget` maps `Device(d)` → `Device(cell_of[d])` (kernel/core/src/ids.rs:38–46), so a merged pair becomes `(c, c)`.
3. **gp runs with `power_uw = 0` and an empty `UnitLib`** (backend/gp/src/mechanics.rs:377–386). Power, units and a block-sized axis table are attached only after gp (frontend/library/src/lib.rs:579–585). dp then anneals with the thermal field refreshed per epoch (`Layout::refresh_temps`, kernel/core/src/thermal.rs:53–55; dp/src/lib.rs:220, 346, 361, 395) and `live_delta_temp_mc` per trial move (thermal.rs:75–90).
4. `SymmetryGroup::project` is the only exact placement repair (symmetry.rs:135–149); dp also reads `mirror_pairs` to build rigid symmetric-group moves (dp/src/lib.rs:442–467) and `branches` for DTI flips (dp/src/lib.rs:226).
5. After placement the library measures `Environment` (WPE/OSE) and `CommonNodes` on placed geometry and registers them in the **routing** budget/report (frontend/library/src/lib.rs:431, 680, 763–876).
6. Routing rules score `Routes` (wires per net, cell metal per net, gate pin rects: kernel/core/src/routes.rs:7–16) through `Stack` (per-layer parasitics and antenna limits from the deck, routing/stack.rs).

### 1.3 Algorithms actually implemented (named precisely)

- **Symmetry projection**: per stage, axis = grid-snapped arithmetic mean of the pairs' grid-snapped midpoints (minimizes Σ squared displacement of midpoints; the doc at symmetry.rs:133–134 says "minimum total displacement", which the median would give); each pair is re-placed at `axis ∓ snap((snap(xb) − snap(xa))/2)` and both y set to the snapped mean y (symmetry.rs:82–90). No overlap awareness.
- **Common centroid**: gate-area-weighted first moment of each side's placed physical units (`UnitLib::of_device`), falling back to area-weighted cell bboxes flagged unknown (cc.rs:50–78); coincidence = centroid offset / max(x-span, y-span) of unit centres in cells that draw both sides, tolerance 1% (cc.rs:86, 102–119); gradient = Pelgrom distance term of the full offset (cc.rs:145–146); LOD = |mean(1/(SA+L/2)+1/(SB+L/2))_A − mean(…)_B|·KVTH0/σ_rand/η (cc.rs:123–141; units.rs:88). Violated when the max of the three spent fractions exceeds 1 (cc.rs:144–149).
- **Matching pair**: Pelgrom eq.(1) distance term `(S/A)·D·√(WL)` vs η on cell-bbox centres (matching_pair.rs:66–78).
- **Thermal field**: superposition of surface point sources on a semi-infinite substrate, ΔT = P/(2πkr), r floored at the source's larger half-extent, k = 148 W/(m·K) (kernel/core/src/thermal.rs:9–49). O(n²) refresh, O(n) per live query.
- **Environment**: precomputed per-pair distances; usage = max of `wpe_min/d` and relative-distance skew / 0.2 for WPE (capped at 10·wpe_min) and OSE (capped at range) (environment.rs:33–47).
- **Stack parasitics**: ground C per shape = C_area·w·len + 2·C_fringe·len (stack.rs:55–64); series R = Σ R□·L/W + R_cut (stack.rs:72–81); "path R" = double sweep over BFS spanning trees of a node-weighted shape-adjacency graph (stack.rs:94–146); terminal R = Dijkstra on a port graph built from shape contacts and terminal overlaps (stack.rs:153–251, 323–356); lateral C = ε0·k·t·run/gap (stack.rs:256–259); antenna = per etch stage, flood-fill connected pieces of layers ≤ stage, exposed area (areal or sidewall) of the stage layer (or same-parity layers when cumulative) over the net's total gate area (stack.rs:284–317).
- **Coupling budget**: all-pairs same-layer 1/gap sidewall C summed from every other net onto the victim, no distance cutoff (coupling.rs:49–68).
- **EM**: Black-equation derating `exp[(Ea/nk)(1/T − 1/Tref)]` (em.rs:16–21); required width `I/J`, relaxed by Blech `I·L/(jL)_B` (em.rs:48–58); cuts ⌈I/I_cut⌉ (em.rs:65–70); the hard rule passes if **any** segment on a limited metal is wide enough for the largest single-terminal current (em.rs:109–120, 128–130).
- **IR**: `I_net · R_path` with R_path from the double sweep (ir.rs:32–36).
- **Shield**: per victim segment, merged-interval coverage of reference metal on each side within `max_gap_nm`, then `min(covered_below, covered_above)` (shield.rs:32–62).
- **Performance budget**: one linear sensitivity row `Σ w_i · length_i · af_per_nm ≤ 1` per spec (performance.rs:31–33).
- **SIMD layer**: fearless_simd fold over i32 columns with a zero-padded masked tail; overlap area with i64 widening; min/max; changed-row scan (kernel/core/src/lanes.rs:37–163).

### 1.4 Every rule type: quantity, physics, tier, semantics

| Rule (file) | Measures (unit) | Violation maps to | Tier as emitted | known / applicable | project | retarget | touches |
|---|---|---|---|---|---|---|---|
| Symmetry / SymmetryGroup (symmetry.rs) | `(xa+xb−2·axis, ya−yb)` of cell-bbox centres (nm) | no mismatch number; enables mirror routing | hard + cost (emit.rs:213–214) | always | yes (lone pair; stage mean axis) | device→cell | Symmetry yes; SymmetryGroup forwards `violating_ids`, not `touched` |
| MatchingPair (matching_pair.rs) | σ_grad/σ_rand = (S/A)·D·√WL, dimensionless, D = bbox-centre distance | gradient share of σ(ΔVT) | budget + cost if deck has A and S, else cost only (emit.rs:161–171) | known ⇔ S/A > 0 | no | device→cell | no |
| CentroidGroup (cc.rs) | max(gradient share/η, coincidence/1%, LOD ΔVT/σ/η), dimensionless | offset from linear gradient / LOD | budget + cost, one per stage (emit.rs:219–237) | unknown without units or a check | no | stores `cell_of` | no |
| ThermalGradient (thermal.rs) | \|ΔT\| between targets (mK), limit η·σ_rand/\|dVT/dT\| (emit.rs:96–101) | TC·ΔT offset | budget + cost (emit.rs:176–183) | applicable = known ⇔ any power ≠ 0 | no | device→cell | no |
| Proximity (proximity.rs) | Euclidean edge gap (nm) vs 5 µm (emit.rs:45) | none direct (Hastings §13.3 r8) | budget + cost | always | no | device→cell | no |
| Isolation (isolation.rs) | edge gap (nm) vs 4·t_epi (emit.rs:274) | substrate coupling (proxy) | budget + cost with epi, else cost only | always | no | device→cell | no |
| DtiBand (dti.rs) | edge gap vs disjunction share/isolate | legality | hard + cost; never emitted by shipped decks (`dti: null` or absent) | always | no | device→cell | no |
| Environment (environment.rs) | WPE/OSE distance usage (dimensionless) | LDE ΔVT (proxy) | routing budget/report only (library lib.rs:431, 680) | unknown w/o deck ranges | no | n/a (precomputed) | no |
| Utilization (utilization.rs) | footprint·u_min/Σ cell area | declared policy | placement budget if `min_utilization > 0` | always | no | n/a | no |
| Antenna (antenna.rs) | worst stage conductor/gate ratio vs deck | gate-oxide damage | hard (extract.rs:52) | no `known` | no | n/a | yes |
| ParasiticBudget (parasitic.rs) | ground C (aF) with stack, else length (nm) | load C on the net | budget | known ⇔ routed | no | n/a | yes |
| CrosstalkExclusion (crosstalk.rs) | min same-layer gap (nm) | coupling (point proxy) | budget (extract.rs:57–61) | no `known` | no | n/a | yes |
| CouplingBudget (coupling.rs) | Σ lateral C onto victim (aF) | coupled noise | budget (extract.rs:78–85) | no `known` | no | n/a | yes |
| Differential (differential.rs) | RC or geometric signature mismatch (%) | differential RC skew | **hard** (extract.rs:53–54) | no `known` | no | n/a | yes |
| Electromigration (em.rs) | best segment width vs I/J (nm) | EM lifetime | **hard** (library lib.rs:1027) | unknown w/o current/limit/route | no | n/a | yes |
| IrDrop (ir.rs) | I·R_path (µV) | bias shift | budget (library lib.rs:289) | unknown w/o stack/current/route | no | n/a | yes |
| Shield (shield.rs) | double-sided coverage fraction | coupled noise | budget (extract.rs:103) | unknown w/o victim wire | no | n/a | yes |
| CommonNodes (common_node.rs) | \|R_a − R_b\| (Ω) vs η·σ_rand/I | source-degeneration offset I·ΔR | routing budget/report (library lib.rs:431, 680) | unknown w/o budget or reach | no | n/a | `touched` yes |
| PerformanceBudget (performance.rs) | Σ w_i·C_i (fraction of spec headroom) | linearized spec miss | routing budget (library lib.rs:268–270) | always | no | n/a | yes (positive weights) |

---

## 2. Module-by-module findings

### 2.0 Do the recent "checks measure something" commits hold?

- **c169ef3 (MatchingPair, CentroidGroup, ThermalGradient)**. The commit message says the MatchingPair check needs only S/A "so no process A_Vt"; the working tree now requires both `A_VT` and `S_VT` from the deck to form S/A (emit.rs:83–92) and emits a budget only when S/A > 0 (emit.rs:168–170). The formula is right, but: (a) it is evaluated on cell-bbox centres, so a pair merged into one cell reads D = 0 and passes with zero cost (AR-01); (b) with the decks' S_VT, a **PROXY** of 0.94–1.63 µV/µm taken from Schaper & Linnenbank's 130 nm Infineon data (pdks/sky130.json:97–98, gf180mcu.json:22–23, ihp_sg13g2.json:17–18), the check binds only at D ≥ η·A/(S·√WL): sky130 n 1748/√WL µm, sky130 p 2117/√WL, gf180 n 2837/√WL, gf180 p 2710/√WL, ihp n 957/√WL, ihp p 534/√WL (WL in µm², η = 0.3). At 25 µm² that is 350 µm for sky130 n. At block scale the check never binds; only the `d²·1e-3` pull acts (AR-03). Pelgrom reports the distance term "only significant for large area devices with a considerable spacing" (pelgrom.txt L88–90), and Schaper measures it over mm (distance_vs_pair_mismatch.txt L48–62). CentroidGroup's coincidence and LOD halves do bind and are tested (cc.rs:314–396); its gradient half inherits AR-03; it pools all pairs of a stage (AR-02). ThermalGradient does measure ΔT and derives its limit from η·σ_rand/TC (emit.rs:96–101); for a sky130 nfet pair of 20 µm², σ_rand = 2.12 mV, η·σ = 0.64 mV, limit 833 mK, which a pair 5 µm apart radially at 20 µm from a source reaches only at P ≈ 62 mW (ΔT ≈ P·d/(2πk r²)); it reads ΔT = 0 for merged pairs (AR-01) and is invisible to gp (AR-42). Holds with those limits.
- **0ae58c0 (Proximity)**: now the Euclidean edge gap (proximity.rs:19–25); `satisfied` is no longer vacuous. Holds. It duplicates `Layout::edge_gap` (layout.rs:170–176) and has no test (AR-35).
- **dad330c (Antenna)**: uses the net's summed gate area (extract.rs:40–51) and the deck's ratio; the working tree additionally reads per-stage ratios through `Stack::antenna` and sky130's sidewall table (verify/src/pdk.rs:586–592), so sky130 now does get Antenna rules (the commit's "no rule when the deck has none" is superseded). Holds, with AR-13.

### 2.1 kernel/analog/src/rule.rs (525)

- Does: trait seam, blanket batch impl, criticality from headroom/margin (rule.rs:280–289), residual floor (295–306), `over` (313–318).
- Correctness:
  - `kind()` for `Vec<R>` is `std::any::type_name::<R>()` (rule.rs:223–225), used as the price key "stable kind name" (rule.rs:144; gp/src/lib.rs:115–116). Its format is unspecified by std; dr and the library defend with `rsplit("::")` (dr/src/lib.rs:1444, library/src/metadata.rs:135, benchmarks/src/bench.rs:132) but gp compares whole strings. `SymmetryGroup` reports `"Symmetry"` while a `Vec<Symmetry>` reports the full path: two names for one rule (AR-18).
  - Default `criticality` is 1.0 (rule.rs:159–162); `CentroidGroup`, `Environment` and `CommonNodes` do not override it, so they are always fully weighted in the cost arm regardless of slack. `Utilization`, `PerformanceBudget` and `SymmetryGroup` override it.
  - The cost arm sums raw, unit-laden costs: `d²·1e-3` nm² (matching_pair.rs:47, symmetry.rs:23, proximity.rs:33, cc.rs:156), `4e-3·shortfall²` (isolation.rs:25), `3e5·u²` (thermal.rs:25), nm (dti.rs cost), `len²·1e-6` (parasitic.rs:48), `×100` ratio (antenna.rs:29), fractions (Utilization, Environment, Coupling, IR, EM, Shield, Performance), percent (Differential), plus HPWL in nm. The test at rule.rs:429–451 documents that raw costs are incommensurable, yet PEX adds them (AR-08).
- Tests (7): criticality vs margin (356–380), residual proportionality (407–426), unit-free residual comparability (429–451), `touched` vs `violating_ids` (455–480), default full weight (483–491), fail floor (496–524). Good seam coverage; nothing on cost scale.

### 2.2 kernel/analog/src/placement/cc.rs (397): CentroidGroup

- Measures three things (see §1.3); units consistent (LOD 1/µm × µm = dimensionless).
- Problems:
  - **Pooling** (AR-02): emit.rs pushes every pair's slot-0 device into `a_side` and slot-1 into `b_side` for the whole stage (emit.rs:196–197, 219–237). For equal-weight pairs, per-pair coincidence implies union coincidence, but not the converse: diff pair offset +d and load offset −d cancel in the union while both pairs are offset by d. Which member is "a" is the recognizer's slot order, so the cancellation sign is arbitrary. The doc claim "pairwise coincidence is a weaker condition" (cc.rs:8–9) does not hold for distinct pairs.
  - **Only first moments** (AR-10): Hastings rule 2 (symmetry about both axes), rule 3 (dispersion), rule 4 (compactness) (hastings.txt L23427–23443) and MOS rule 5 (orientation, L42191–42210) are not measured. The quadratic residue that dispersion and compactness reduce (L23480–23495) and higher-order cancellation (L23541–23548; nth_order 2ⁿ units, L10–16) are acknowledged in a ponytail (cc.rs:84–85) but not implemented.
  - **Coincidence scale** (AR-37): `COINCIDENCE_TOL = 0.01` of the unit-centre extent (cc.rs:86) is a design constant with no physical scale, so the residual's magnitude is arbitrary: an AABB row reads usage 66.7 (residual 65.7 budgets), a 1+1 AB merged cell reads usage 100. There is no notion of the best achievable pattern: odd per-side unit counts cannot reach exact coincidence in any grid (a side with an odd count needs a unit on the array centre). Today cellgen rejects most odd-count merges via `currents_run_alike` (frontend/library/src/cellgen.rs:140, 261–272) and only offers `Cc1d` when `centroid_sequence` exists (kernel/cells/src/mosfet.rs:90, 730–750), which limits exposure; any merged cell that still lands there contributes an unremovable Θ of tens of budgets and drives its gp price upward without bound.
  - **Unknown vs violated** (AR-33): with S known but units missing, `violations`/`residual` still charge Θ from the bbox proxy (cc.rs:145–148 via `offset_nm` → `box_centroid`) while `unknown` reports 1 (cc.rs:167–171). This contradicts `Rule::known`'s contract (rule.rs:54–58). The LOD half is not part of `unknown`; `worst_usage` is reported even when unknown.
  - LOD mean is unweighted over fingers (cc.rs:123–130) (matches BSIM4 per-finger averaging for one device, cc_review eq. (11) L103–115, p.2), not W-weighted across devices of different finger widths. KVTH0/σ is used directly; the deck note states Kstress = 1 for sky130 (pdks/sky130.json:92). The mobility (KU0) term is not modelled; its magnitude is not given in docs/ref/ (AR-32).
  - Two sources of truth for device→cell: `self.cell_of` (bbox fallback, cc.rs:68) and `l.units.cell_of` (cc.rs:103; units.rs:126).
  - No `touches`: repair and FD-PEX probing never target CC violations (AR-17).
  - Doc cites "Hastings §13.3 r9" for the ABBA LOD imbalance (cc.rs:40); in 3e, r9 is compactness; LOD/moat is r12 (L42532–42553) and §13.2.2 (L41367–41425) (AR-41).
- Tests (9, cc.rs:226–396): ABBA vs AABB cost, area weighting, one condition per array, far-off violation, units-not-outlines, LOD moat, coincidence without S, merged pair in a mixed stage. None for pooled-pair cancellation, rotated cells, or odd unit counts.

### 2.3 kernel/analog/src/placement/matching_pair.rs (161)

- Measures the Pelgrom distance term on bbox centres (matching_pair.rs:66–78); units S/A [µm⁻²] × D [µm] × √WL [µm] = dimensionless; algebra correct.
- Problems:
  - After retarget a merged pair has `a == b` → D = 0 → satisfied and cost 0 (AR-01). The unit table exists exactly in that case and would give the true device-centroid distance.
  - The S term is a wafer-scale random process (pelgrom.txt L33–43) measured over mm (distance_vs_pair_mismatch.txt L48–62, L179–198); it is not the deterministic in-die gradient that CC/symmetric placement cancels (Hastings eq. 8.23, L23120–23142). Non-binding at block scale (§2.0) (AR-03).
  - `gate_um2` is `min(gate_a, gate_b)` (emit.rs:159), the loosest choice, while CentroidGroup takes the max ("tightest", emit.rs:220). `usage` returns `Some(0)` when S is unknown (matching_pair.rs:58–60), so reports show 0% spent where the check is unknown (AR-40).
  - `cost` `d²·1e-3` unnormalized; no `headroom` → criticality 1 always.
  - `is_supply` is a name heuristic with sky130-specific roots `vpwr/vgnd/vpb/vnb` and `contains("gnd")` (matching_pair.rs:99–106); it ignores the annotator's `NetClassification` (AR-30). `is_diff_pair` (82–95) checks kind, shared non-rail source, distinct gates/drains, no cross-coupling, but **not W/L, model or bulk**; any two same-kind FETs sharing a non-rail source qualify. `Differential::extract` and `CrosstalkExclusion::extract` build on it and the annotator registers Differential as **hard** (extract.rs:53–54), so an unmatched common-source pair (e.g. two differently sized devices on a shared internal node) gets a 5% hard RC-matching requirement (AR-39). Pseudo-differential pairs (sources on a rail) are never recognized.
  - No `touches` (AR-17).
- Tests (2): distance and area drive the check; unknown without S. None for merged pairs.

### 2.4 kernel/analog/src/placement/symmetry.rs (372)

- Measures exact mirror equality of bbox centres about a vertical axis (symmetry.rs:63–68).
- Problems:
  - Only centres: partner orientation, variant and size (hw/hh) are not compared (AR-11). Mirror symmetry requires identical geometry with mirrored orientation; perfect symmetry requires identical orientation (balasa_graeb_survey.txt L675–699). A pair at R0/R90 with different footprints passes.
  - **Orientation is guarded by no rule** (AR-36): dp's comment "Grouped (matched) devices never turn" (dp/src/lib.rs:555–560) holds only for cells in a multi-member `Layout::groups` entry, and dp receives the **abutment** table there (frontend/library/src/lib.rs:579), which the annotator truncates to one member for any block mixing polarities (backend/annotator/src/lib.rs:107–116). In a mixed-polarity block (a 5T OTA stage holds an NMOS pair and a PMOS load), a matched pair drawn as two separate cells is rotatable, and `try_rotate` turns each partner independently (dp/src/lib.rs:563–575), changing current direction and swapping hw/hh. Hastings §13.3 r7: matched transistors should always have equal orientation (L42490–42498); Φ (eq. 13.61, L42155–42186) is carried per unit in world coordinates (units.rs:26–27, 120, 132–135) but read only inside one merged cell at cell-generation time (cellgen.rs:261–272).
  - Only a vertical axis; no horizontal or two-axis (quad) symmetry.
  - Self-symmetric device (`a == b`) degenerates to "bbox centre on axis" (symmetry.rs:70–72, 343–360); for a merged pair nothing checks the units' mirror symmetry inside the cell.
  - `axis_x` falls back to `centre_x_estimate()` (mean x of all devices) when `AxisId ≥ axis.len()` (layout.rs:106–118). The library resizes the table to the block count after gp (frontend/library/src/lib.rs:580–583), but gp's own Layout has `axis: vec![c; n]` over cells (gp/src/mechanics.rs:377–386), so during gp a block index ≥ cell count chases a moving mean and `project` cannot write the slot (symmetry.rs:56–58) (AR-15).
  - `project` on a plain `Vec<Symmetry>` sharing one `AxisId` writes the slot per pair; only the last pair ends satisfied. The annotator only emits `SymmetryGroup`, so this is latent.
  - **Rounding shrinks pairs** (AR-38): `half = snap((snap(xb) − snap(xa))/2)` rounds toward zero on odd grid counts (symmetry.rs:84, 152–162): partners 25 nm apart on a 5 nm grid end 20 nm apart (the test at 205–214 checks only on-grid and satisfied). A clearance that was met exactly can be erased by one grid step. Storing `2·axis` (so `x_a + x_b = axis2` is exact at any parity) avoids both shrinking and half-grid axes.
  - Projection ignores overlaps and fixed cells; the caller re-checks legality. The axis per stage (`AxisId(block index)`, emit.rs:139) lets the stages of a fully differential path drift onto different axes.
- Tests (9, symmetry.rs:194–371): exact projection, grid parity, survives snap, shared axis, separation preserved, retarget identity/map, collapsed self-pair, idempotence. None for orientation or size mismatch, the axis fallback, or odd-parity shrink.

### 2.5 kernel/analog/src/placement/thermal.rs (134): ThermalGradient

- Measures |ΔT| between targets: live field for `cost` (thermal.rs:30–33), epoch-frozen `temp_mc` for `satisfied`/`headroom`/`residual` (35–40, 59–62). The limit η·σ_rand/|dVT/dT| (emit.rs:96–101) is consistent with Hastings eq. 8.23 (L23120–23142).
- Problems:
  - Merged pairs read ΔT = 0 (AR-01). The field should be sampled at each device's unit centroid.
  - gp never sees it: power is zero during gp (AR-42).
  - `AT_SPEC_COST = 3e5` is a hand-tuned weight chosen to reproduce an old distance pull (thermal.rs:23–25) (AR-08).
  - `applicable`/`known` = any nonzero power anywhere (thermal.rs:46–55).
  - Group targets read the hottest member (kernel/core/src/thermal.rs:65, 86) instead of a weight-averaged temperature; no producer emits `Target::Group` (grep finds none), so this path is dead (AR-43).
  - The same η is spent in full here and by MatchingPair, CentroidGroup (gradient and LOD) and CommonNodes (AR-04).
- Tests (3): lopsided vs mirrored, isotherm beats closeness, unpowered inertness.

### 2.6 kernel/analog/src/placement/dti.rs (188): DtiBand

- Measures edge gap against a disjunction: share (< s_max) or isolate (> d_dti) (dti.rs:54–57); cost pulls toward the committed branch (42–49); residual is the cost over the band width (59–64).
- Problems: strict `<` at the share bound: an abutting pair with gap == s_max (e.g. s_max = 0, gap = 0) is violated with zero cost and zero measured residual, then floored to 1e-3 by `rule_residual` (AR-16). Euclidean gap counts diagonal neighbours as sharing a trench. No `touches`. No shipped deck carries DTI data (`"dti": null` in sky130/generic_finfet; absent in gf180/ihp), so the rule is never emitted. The test comment "Nothing constructs a DtiBand with a real BranchId yet" (dti.rs:180) is stale (emit.rs:185–194 does).
- Tests (4): branch-directed cost, disjunctive legality, branch collection, empty branch table.

### 2.7 kernel/analog/src/placement/environment.rs (96): Environment (WPE/OSE)

- Measures precomputed per-pair distances (frontend/library/src/lib.rs:806–876): `near = max(wpe_min/d)` and relative skew `|a − b|/max(a, b)/0.2` of WPE distances (capped at 10·wpe_min) and OSE distances (capped at range) (environment.rs:33–47).
- Problems (AR-09):
  - Skew on distances, while the effects fall off steeply with distance (the crate's own LOD model uses 1/(SA+L/2), units.rs:88; Hastings: "the areas within a micron or two suffer the most", L41302–41310). A 20% skew at 0.3 µm and at 30 µm read the same usage. `ENV_TOL = 0.2` and the 10× range are design constants (environment.rs:7, 40).
  - State-free and registered in the **routing** arm after placement (library lib.rs:431, 680): placement never sees a WPE/OSE gradient. The survey lists LDE-aware placement as the state of the art (perf_driven_survey.txt L118–128); cc_review states the mitigations as equal well spacing and equal OD width/spacing for matched devices (L99–126, p.2).
  - For a merged pair the OSE producer uses the whole cell's diffusion for both members (library lib.rs:858–867), so OSE skew is always 0 there.
  - The deck ranges `wpe_clearance_moderate` and `lod_moat_ext_moderate` are 3000 nm in all four decks (pdks/*.json) and are geometric clearances, not ΔVT models.
  - Doc cites "Hastings §13.3 r8" for WPE clearance (environment.rs:15); in 3e the well-edge rule is r19 (L42625–42631) (AR-41).
- Tests (1): near/unequal/blind.

### 2.8 kernel/analog/src/placement/isolation.rs (38)

- Measures edge gap ≥ `min_distance_nm` (isolation.rs:27–29); residual linear (34–37).
- Problems (AR-34): the doc cites Charbon's saturation distance (isolation.rs:7–10) but no saturation is modelled and no substrate type is known; Charbon shows the plateau only on low-resistivity epi with finite backplate impedance (charbon_substrate.txt L3286–3296) and continuous improvement with surface conduction (L3346–3351). The annotator notes sky130 is bulk, where the plateau does not hold, and falls back to a nominal 2.5 µm epi (emit.rs:250–256). Guard rings (`GuardRingRequirement`, cell.rs:6–23) are not credited. No `touches`, no `usage`, no tests (AR-35).

### 2.9 kernel/analog/src/placement/proximity.rs (48)

- Measures Euclidean edge gap vs `max_distance_nm` (proximity.rs:19–25). Correct. Duplicates `Layout::edge_gap`. `cost` `ex²·1e-3` unnormalized. The 5 µm spec is a literal in the emitter (emit.rs:45). No `touches`, no tests.

### 2.10 kernel/analog/src/placement/utilization.rs (84)

- Measures footprint·u_min / Σ cell area (utilization.rs:19–25). A declared policy (documented ponytail, 10–12). Its `cost` is a fraction next to nm²-scale costs, so it acts only through its Θ price (AR-08). Test (1).

### 2.11 cell.rs, constraints.rs, metadata.rs, requirements.rs, lib.rs, placement/mod.rs, routing/mod.rs

- Plain data. `GuardRingRequirement.tap_pitch_nm`, `.enclosure_complete`, `Unitization.same_variant_required` are written in constructors but never read (grep `\.field` returns 0 reads); `SeriesParallel::RepeatedStage` has no reference outside analog (AR-28).
- `NetClassification` (metadata.rs:8–15) carries no current, frequency, activity or impedance; aggressor strength cannot be expressed (AR-14).
- The "order within an arm is a contract" note (requirements.rs:16–18) makes price identity depend on emission order.

### 2.12 kernel/analog/src/routing/antenna.rs (67)

- Measures the worst per-stage ratio via `Stack::antenna`; fallback: all routed + cell metal (including cut squares) over total gate area vs the deck's tightest ratio (antenna.rs:60–66).
- Problems (AR-13, with stack.rs:270–317):
  - A piece that reaches only some of the net's gates is charged the whole net's gate area (stack.rs:280–282, 310), under-estimating its ratio. Hastings defines the node ratio over the gate oxide of poly "belonging to the node" (L13149–13160).
  - Diode credit is binary and infinite: any shape on the diode marker anywhere on the net returns ratio 0 at every stage (stack.rs:289–291). Hastings describes separate metal/diffusion rules checked when the gate rule fails, and notes low-voltage CMOS needs both an NSD/substrate and a PSD/N-well junction (L13205–13225); a diode connected only through an upper metal protects only from that stage.
  - One mechanism per layer: `antenna_rule` lets an areal rule win over a sidewall one (verify/src/pdk.rs:579–613), while Hastings states each layer has both peripheral and areal limits (L13149–13150). The no-stack fallback limit `antenna_max_ratio` is the minimum over routing metals of mixed areal and sidewall ratios (pdk.rs:575–577), e.g. sky130's mcon surface ratio 3.
  - Gate reach is layer-blind (a shape of any rank overlapping the gate pin rect counts, stack.rs:306), which over-charges (conservative). Cumulative mode assumes strict metal/cut alternation (`(stage − r) % 2 == 0`, stack.rs:298).
  - No `known`: an unrouted net reads ratio 0 and certifies (AR-44).
- No tests in antenna.rs; covered by rule.rs:429–451 and stack.rs:478–531.

### 2.13 kernel/analog/src/routing/common_node.rs (123)

- Measures |R_a − R_b| from the feeds (or star centre) to each member's source pins, each side's pins in parallel (common_node.rs:41–53), against η·σ_rand/I (doc 10–16; producer library lib.rs:763–800). A sound model of source-degeneration offset (ΔV_os ≈ I·ΔR_S) and the most physical R model in the crate.
- Problems: terminal attachment is layer-blind (stack.rs:225–240); criticality default 1.0; DC only; shares η with the placement rules (AR-04). The producer's gate area is `W·L·max(nf, m)` (library lib.rs:792), see AR-46.
- Test (1): end-fed vs mid-fed rail.

### 2.14 kernel/analog/src/routing/coupling.rs (192)

- Measures Σ over every other net's same-layer shapes of ε·t·run/gap onto the victim (coupling.rs:49–68). `EPS_H_AF = 12` aF = ε0·3.9·0.35 µm = 12.1 aF, correct (coupling.rs:14–18); `Stack::lateral_af` units also check (stack.rs:256–259; test 466–476, 621.6 aF).
- Problems:
  - Pure 1/gap parallel plate with **no distance cutoff and no screening**: a 10 µm parallel run 10 µm away still adds 12 aF, and a wire behind a nearer wire or a shield counts in full. Lampaert's lateral model is a fitted polynomial in 1/d (eq. 2.31, L1958–1967) and Lampaert's net coupling adds overlap-area C between layers (eq. 4.18, L4658–4690); inter-layer coupling (eq. 2.29, L1920–1932) is ignored. Overlapping shapes of one net (pads on wires) are counted twice (AR-14).
  - Every other net is an aggressor, including supplies, ground and the victim's own **shield**: for Sensitive nets the annotator emits both a ground Shield at ≤ 2·route_space (extract.rs:88–103) and a CouplingBudget (extract.rs:78–85). At a 280 nm gap the shield adds ≈ 43 aF per µm of run per side with the `EPS_H_AF` fallback; the Sensitive budget equals the driven gate capacitance (classify.rs:26–33), so the conflict grows with shielded length (AR-07).
  - No aggressor activity or Miller weighting; a DC bias neighbour costs as much as a clock.
  - O(victim × all shapes), documented.
- Tests (6): accumulation, set-sum residual, 1/d, layer separation, criticality, magnitude.

### 2.15 kernel/analog/src/routing/crosstalk.rs (115)

- Measures the minimum same-layer Euclidean gap between two nets (crosstalk.rs:24–42). `extract` emits `min_spacing_nm: 0` (crosstalk.rs:107), vacuous until the annotator overwrites it with `route_space × class multiple` (extract.rs:57–60). A point clearance, not run length or C, so dominated by CouplingBudget. Nets that couple only across layers always pass. Built on `is_diff_pair` (AR-39). No tests (AR-35).

### 2.16 kernel/analog/src/routing/differential.rs (198)

- Measures, with the stack, the worst of relative ground-C and relative run-R mismatch (differential.rs:55–65); without, a per-layer (length, area, square-count) signature (69–89); budget 5% from `extract` (125).
- Problems (AR-12):
  - Registered **hard** (extract.rs:53–54) though it is a toleranced budget, not legality or an exact equality as `Requirements` defines hard (requirements.rs:8–9); a 5.1% mismatch outranks any Θ in `lex`.
  - Stack mode drops squares (cuts, pads, square jogs) from R (differential.rs:63), so via-count asymmetry is invisible there; no coupling-asymmetry term (documented ponytail, 26–27), although unequal coupling to a shared aggressor is what converts common-mode noise into a differential error.
  - Only drain nets of `is_diff_pair` pairs (116–133), which has no size check (AR-39); input (gate) nets, folded/cascode nets and mirror outputs are not matched.
  - No `known`: unrouted nets read 0% mismatch (AR-44).
- Tests (2): split-layer mismatch, via count, width; stack-mode pads.

### 2.17 kernel/analog/src/routing/em.rs (243)

- Correct pieces: derating (em.rs:16–21) is Black's equation at constant lifetime, matching Hastings eq. 15.25 (L48781–48790, 0.58 at 398 K reproduced by the test at 216–230) and Lienig eq. 3.26 inverted (the comment at 60–63 that the book's multiplied f(T) would let a hotter via have fewer cuts is right); width `I/J` (52) and Blech relaxation `I·L/(jL)_B` (54) are dimensionally consistent (µA·nm/µA = nm).
- Problems (AR-05):
  - `best` picks the segment with the most headroom and `satisfied` is true if that single segment is wide enough (em.rs:109–120, 128–130) for the largest **single-terminal** current (library lib.rs:1003–1021). A rail whose trunk carries Σ I, or a wide strap with a narrow neck, passes and is reported `known`. The per-segment check Lienig prescribes (segment currents from LHS/RHS terminal sets, eqs. 3.5–3.7, L3975–4052) exists in dr but is reported as **Θ** (`em underwidth`, `em cuts`: dr/src/lib.rs:883, 1852–1853), so the tiers are inverted: the weak test is hard, the real test is a budget.
  - DC average only; Lienig sizes on both equivalent (RMS/avg) and peak currents (eqs. 3.21–3.22) and derates effective width by h_nom/h_min, Δw and etch loss (eq. 3.24) (L4589–4633). Cuts are not checked by the rule. The Blech domain is the sum of all same-layer lengths of the net (em.rs:115), conservative; no deck supplies `blech_limit` (grep pdks), so the Blech path is dormant.
- Tests (5): width vs current, per-layer limit, unknowns, derating monotonicity, Blech.

### 2.18 kernel/analog/src/routing/ir.rs (94)

- Measures `I · R_path` (ir.rs:32–36). R_path is the double-sweep over BFS trees (stack.rs:94–146): exact tree diameter on a tree, but on a mesh the second sweep uses a different spanning tree and any terminal pair's tree path can reach up to twice the reported value, so "no terminal path is longer" (stack.rs:86–88) holds only within a factor 2. Charging the whole net current across the worst path is conservative for a star-fed rail. The terminal-resolved `fed_resistance_ohm` (stack.rs:167–177) exists and is unused here (AR-21). Test (1).

### 2.19 kernel/analog/src/routing/parasitic.rs (126)

- Measures ground C from the stack when a C budget exists, else drawn length (parasitic.rs:30–36). `cost` is always `len²·1e-6` (46–49), so the gradient never prefers a low-C upper layer the budget rewards. Ground C sums shapes without union (pads double counted, stack.rs:52–53), ignores cell metal and coupling C. `extract` with placeholder 1 mm budgets (83–96) has no caller (the annotator builds from classes, extract.rs:65–76) (AR-19, AR-23). Test (1).

### 2.20 kernel/analog/src/routing/performance.rs (95)

- Linear sensitivity row, the Lampaert/Choudhury performance-driven approach (lampaert.txt L1098–1108). Uses `length × af_per_nm` (one constant) instead of the stack's per-layer ground C, omits coupling C and series R (Lampaert eq. 4.22 includes all three, L4705–4717), has no trust region, and negative weights can buy credit far from the schematic point (AR-22). Test (1).

### 2.21 kernel/analog/src/routing/shield.rs (134)

- Bug (AR-06): per victim segment, coverage is `side(true).min(side(false))` (shield.rs:59), the minimum of the two sides' covered lengths, not the length covered on **both** sides. A reference strap below on [0, 5 µm] and above on [5, 10 µm] reports 50% coverage for a run that is nowhere double-shielded. Fix: intersect the two sides' merged interval sets.
- Doc says "ponytail: one-sided and same-layer only" (shield.rs:18) while the code requires both sides; the intent is "same-layer only, no top/bottom plates".
- Square victim shapes (w == h) are excluded from coverage (shield.rs:35). Test (1): full, one side, half, far, unrouted; not complementary halves.

### 2.22 kernel/analog/src/routing/stack.rs (532)

- Per-layer parasitics and graph utilities (§1.3). Units consistent (stack.rs:12–24; tests 420–424: 925 aF; 427–431: 11.5 Ω).
- Problems: misplaced rustdoc: the `parallel` doc is split across `PortGraph` (320–321) and `parallel` (358); the `pieces` doc sits on `touches` (374–375). Four copies of the rect-touch predicate (stack.rs:106, 185, 376–378; routes.rs:61) (AR-20). Edge contact counts as connection (`<=`), so a cut abutting but not overlapping a metal connects. Layer-blind terminal attachment in `port_graph` (225–240). O(k²) adjacency everywhere (documented). `PortGraph::from` orders f32 distances by `to_bits`, which is correct only for non-negative values; all edge weights are non-negative, so it holds.
- Tests (7): ground C, series R, terminal R, path R, lateral C, jumper split, cell plate.

### 2.23 kernel/core/src/geom.rs (146)

- D4 algebra is correct: `Mx` reflects y then rotates CCW, matching GDSII STRANS (reflection before rotation) (geom.rs:44–55); `apply_rect` maps opposite corners and renormalizes (60–64); `swaps_axes` correct (38–40). No `compose`/`inverse`, so "partner orientation = mirror ∘ orient" and hierarchical transforms cannot be expressed (AR-27).
- Tests (3): eight distinct transforms, extents swap, R90 periodicity on grid.

### 2.24 kernel/core/src/layout.rs (177)

- SoA columns; centre + half-extent model. `bbox(Group)` recomputes per call and panics on an empty group (65–85). `edge_gap` Euclidean with i64 intermediates (170–176). `debug_check` validates column lengths and extents (123–147) but not `axis`/`branch` sizes and not `2·hw == drawn bbox width`; gp derives `hw = bbox.w / 2` with truncation (gp/src/mechanics.rs:320–323) and relies on the cells builder rounding bboxes to two grid steps. An injected macro with an odd-grid width would stamp off-grid (`place_macro` uses `x − hw`, macro.rs:48) (AR-25).
- Not `Clone`; built by struct literal in about 24 places (grep `temp_mc:`), so each new column is a shotgun edit. No columns for fixed/locked, halo, legal-orientation set, region/row, well net (AR-26).
- No tests.

### 2.25 kernel/core/src/thermal.rs (171)

- ΔT = P/(2πkr) is the closed form for a surface point source on a semi-infinite solid with an adiabatic top; µW, nm → mK conversion by 1e6 is correct (thermal.rs:16–17, 46): 10 mW at 10 µm gives 1.075 K.
- Limits (AR-24): no die thickness or heat-sunk backside (image sources), so far-field gradients are over-estimated; Lampaert computes a finite-die Fourier series with a multilayer model (eq. 4.28, L4765–4830). k fixed at 148 W/(m·K) (thermal.rs:14). Self term uses r = max(hw, hh) (38–40), a crude spreading proxy. Temperature is sampled at device centres only, so the across-device gradient a CC layout cancels is invisible. `rises_mc` rounds to integer mK (26). Group = hottest member (65, 86), dead in practice (AR-43).
- Tests (4): 1/r fall-off, no power, isotherm, live vs frozen.

### 2.26 kernel/core/src/units.rs (189)

- Physical unit table: owner, active centre, weight (W·L nm²), φ (S→D direction), LOD distances; `placed` applies the same transform as `place_macro` (units.rs:105–122). LOD term `1e3/sa + 1e3/sb` in 1/µm with sa, sb = channel centre to diffusion end = BSIM4 SA + L/2 (units.rs:28–32, 88). Correct.
- φ reaches world coordinates (units.rs:120, 132–135) but no rule reads it (AR-36). `owner: u8` caps a cell at 256 members; `build` indexes `members[cell]` without a bounds check (units.rs:82).
- Tests (2): mirror transform, missing variant.

### 2.27 kernel/core/src/lanes.rs (224)

- Correct masked-tail fold; exact i64 overlap products (lanes.rs:92–114); oracle tests at every tail length (190–213) and overflow (215–223). i32 coordinate differences overflow only beyond about ±1 m. No findings.

### 2.28 hypergraph.rs, netlist.rs, macro.rs, process.rs, report.rs, routes.rs, unionfind.rs, ids.rs, lib.rs

- hypergraph.rs: positional FET terminals G, D, S, B (doc 4–6). `net_devices` lists a device once per terminal (39–43), so a diode-connected device appears twice and "≥ 2 devices" filters count terminals.
- macro.rs: `place_macro` anchors the turned bbox's lower-left at `(x − hw, y − hh)` (48); it centres only if `hw == bbox.w/2` (AR-25).
- process.rs: `rule(name, default) -> i32` (12) makes a silent default the normal path (AR-29).
- report.rs: `lex` sums Θ as f64 (20); `margin == 0` reads as satisfied (35–36), protected by `from_residual`'s ceil.
- routes.rs: `length` sums long sides of all shapes including cut squares and pads (80–82); `debug_check` connectivity is layer-blind (61), so it can pass an open between overlapping shapes on non-adjacent layers (AR-31).
- unionfind.rs: `groups()` iterates a `HashMap` (49–54), so group order is nondeterministic; it has no callers, and every `extract` ignores `uf` (extract.rs:30 builds one only to satisfy the signature) (AR-19).
- ids.rs: `Target::retarget` passes through device ids past `cell_of` silently (40–43).
- No tests in these files (AR-35).

---

## 3. Gap analysis vs a best-in-class constraint-aware analog P&R that beats hand layout

1. **Unit-level matching for every rule.** Hand layout reasons about finger centroids, not cell outlines. The unit table exists (units.rs) and only CentroidGroup uses it. MatchingPair, ThermalGradient, Symmetry (for merged cells) and Environment (OSE) collapse to no-ops or cell-level proxies after merging (AR-01, AR-09, AR-11). Every matching rule should keep schematic device ids and resolve each device's unit moments through `UnitLib::of_device`, whether devices share a cell or not, and sample the thermal field at unit centroids.
2. **Per-matched-set CC quality, full rule set.** Hastings' four rules (Table 8.4) plus orientation (Table 13.2), scored per matched set, not per stage: first moments (exists); quadratic moments (the residue ∝ array size², L23480–23495); symmetry of unit positions about both axes; dispersion (count of self-balanced subarrays, or the correlation metric of cc_review eq. (14)–(16)); compactness and aspect ratio (≤ 3:1 moderate, near-square exceptional, §13.3 r9 L42504–42516); orientation Φ equality across cells (eq. 13.61). Optional nth-order patterns (2ⁿ units per device, nth_order L10–16). Today: one pooled first-moment check per stage (AR-02, AR-10, AR-37).
3. **One systematic-offset budget per matched pair (or per spec).** Lampaert maps every matched pair's σ(ΔVT), σ(Δβ) through circuit sensitivities and sums them into each performance characteristic, recomputing the distance-dependent part per placement (eqs. 4.25–4.27, L4718–4762). Philis gives each contributor the full η·σ_rand independently (AR-04). A best tool sums gradient residue, thermal TC·ΔT, LOD ΔVT, WPE ΔVT, CommonNode I·ΔR and route RC mismatch into one row per pair, weighted by that pair's input-referred sensitivity, and prices the row once (the `PerformanceBudget` pattern).
4. **Deterministic gradient model instead of Pelgrom S.** S is wafer-scale and non-binding at block scale (AR-03); long-distance patterns are linear or nonlinear and change chip to chip (long_distance_mismatch.txt L185–200). Hand layout assumes an unknown in-die gradient and cancels it by pattern; cc_review's worst-angle linear-gradient metric (eqs. 8–10, L90–118) is a process-free placement check.
5. **LDE-aware placement (WPE, LOD, OSE) as live placement terms** (perf_driven_survey.txt L118–128). LOD lives inside CentroidGroup; WPE/OSE are post-placement routing-arm reports with a distance-skew metric (AR-09). Needed: ΔVT(d) models (falling with distance, per Hastings L41302–41310 and BSIM WPE/LOD form) on units vs well/diffusion rects, refreshed per dp epoch like thermal, feeding item 3.
6. **Symmetry model at ALIGN/MAGICAL level** (GK; balasa_graeb_survey.txt L675–711, L1850–1870, L3156–3190): symmetry groups of pairs plus self-symmetric devices; mirror vs perfect (identical-orientation) mode per group; vertical and horizontal axes; one axis shared across the stages of a fully differential path; partner geometry equality (variant, orientation, extents); symmetry islands (Def. 2.1: members abut into one connected placement); symmetric-feasible sequence pairs or LP legalization that keeps symmetry exact while removing overlaps (MAGICAL, GK). Today: centres only, vertical only, per-stage axes, projection without overlap awareness (AR-11, AR-15, AR-36, AR-38).
7. **Orientation as a checked constraint.** Hastings r7 (L42490–42498) is a first-order matching rule. Today it depends on dp's move set and fails for mixed-polarity blocks (AR-36). A rule reading per-device Φ from units across cells, hard for exceptionally matched sets, closes it.
8. **Placement structure constraints absent from the rule set**: alignment/order (ALIGN `AlignInOrder`/`Order`, GK), array regularity, boundary constraints for I/O devices, signal-flow and monotonic current-flow ordering (perf_driven_survey.txt L111–116), no contacts or metal over active gates and dummy-metal block for matched arrays (Hastings r16–r18, L42596–42624), die-context stress and power-device separation (r13–r15, L42554–42595). `routing/align.rs` was deleted and nothing replaces it.
9. **Thermal and unit data before global placement.** gp chooses topology blind to power and units (AR-42). LAYLA-style (GK; Lampaert §4.9.3) placement evaluates thermal and mismatch degradation on every intermediate placement.
10. **Routing matching of the RC network each side sees, including coupling.** Mirror-symmetric routing (Lampaert §5.7.3, L6199–6225), exact per-layer matching (perf_driven_survey.txt L86–88), no routing over active (L83–84; Hastings r17). `Differential` compares ground C and summed R, drops cuts in stack mode and ignores coupling asymmetry; it is also hard and extracted on unmatched pairs (AR-12, AR-39).
11. **Crosstalk as noise, not spacing.** Weight each aggressor by activity (dV/dt, frequency), exclude quiet references and shields, use a fitted lateral model (Lampaert eq. 2.31), add overlap C between layers (eqs. 2.29, 4.18), let nearer wires screen farther ones, and feed coupling C into the sensitivity rows (eq. 4.22) (AR-07, AR-14).
12. **EM/IR on the real current tree.** Per-segment currents from terminal currents (Lienig eqs. 3.5–3.7), RMS/avg and peak widths (eqs. 3.21–3.24), via arrays per cut layer (eq. 3.25), a hard tier for the real check, and terminal-resolved IR through the existing port graph (AR-05, AR-21).
13. **Antenna per node and per mechanism.** Gate area of the gates a piece reaches, both areal and peripheral per layer, finite diode credit tied to the connection stage and to the deck's metal/diffusion rules (Hastings L13128–13160, L13205–13225) (AR-13).
14. **Normalized objective.** Dimensionless per-rule cost terms (usage² or a smooth hinge on residual) and one HPWL normalization, so no unit choice sets priorities (AR-08). Precondition for automatic price/weight learning.
15. **Thermal fidelity.** Finite die thickness with a heat-sunk backside (image method) or Lampaert's finite-die series (eq. 4.28), PDK-supplied k and die thickness, unit-centroid sampling, weighted-mean group temperature (AR-24).
16. **Substrate isolation with guard-ring credit and substrate type** (Charbon L3286–3351) instead of a bare distance floor (AR-34).
17. **Performance closure loop.** Sensitivity rows exist only for routing ground C via length (`PerformanceBudget`); a best tool also maps device-level systematic offsets, R and coupling parasitics into them and closes with post-layout simulation.

---

## 4. Ranked findings

| ID | Severity | file:line | Finding | Suggested fix direction |
|---|---|---|---|---|
| AR-01 | high | kernel/analog/src/placement/matching_pair.rs:66-72; placement/thermal.rs:30-40; kernel/core/src/ids.rs:38-46; frontend/library/src/lib.rs:1146-1153 | After cell collapse, `retarget` maps both members of a merged pair to one cell, so MatchingPair (D = 0) and ThermalGradient (ΔT = 0) pass with zero cost: they measure nothing exactly where unit data gives the real answer. | Keep schematic ids plus `cell_of` (as CentroidGroup does); evaluate distance and temperature at per-device unit centroids via `UnitLib::of_device`; sample the thermal field at points. |
| AR-02 | high | kernel/analog/src/placement/cc.rs:6-9,144-149; backend/annotator/src/emit.rs:196-197,219-237 | CentroidGroup pools every pair of a stage into one A/B first moment; opposite offsets of different pairs cancel; slot order sets the sign. | One CentroidGroup per matched set (pair or ratioed array); drop the stage-level pooled check. |
| AR-03 | high | kernel/analog/src/placement/matching_pair.rs:76-78; cc.rs:145-146; pdks/sky130.json:97-98 | The Pelgrom S·D check cannot bind at block scale (sky130 n: D_max = 1748/√WL µm); S is a wafer-scale PROXY from 130 nm mm-scale data, not the in-die gradient placement cancels. | Report the Pelgrom term only; make the placement check a dimensionless moment residue (orders 1-2, worst gradient angle, cc_review eqs. 8-10) with an explicit tolerance; allow a designer-supplied in-die gradient. |
| AR-04 | high | backend/annotator/src/emit.rs:83-101,159-237; frontend/library/src/lib.rs:792-797; cc.rs:137,148; thermal.rs:59-62 | The same allowance η·σ_rand is granted in full to MatchingPair, CentroidGroup gradient, CentroidGroup LOD, ThermalGradient and CommonNodes; the combined systematic offset can reach several times the allowance with every rule passing. | One offset row per matched pair (sum linearly or in quadrature, a decision) of all systematic contributors, weighted by sensitivity (Lampaert eqs. 4.25-4.27), priced once. |
| AR-05 | high | kernel/analog/src/routing/em.rs:109-120,128-130; frontend/library/src/lib.rs:1003-1021; backend/dr/src/lib.rs:883,1852-1853 | The hard EM rule passes when any single segment carries the largest single-terminal current; summed trunk currents and narrow necks pass as `known`. The real per-segment check in dr is reported in Θ: tiers inverted. DC only. | Per-segment currents on the routed tree (Lienig 3.5-3.7) checked for every segment and cut, in the hard arm; RMS/peak when waveforms exist; demote the current rule to a non-certifying precheck. |
| AR-06 | high | kernel/analog/src/routing/shield.rs:59 | Shield coverage is `min(covered_below, covered_above)`, not the length covered on both sides; complementary half-shields read 50% while nothing is double-shielded. | Intersect the two sides' merged interval sets per segment; add the complementary-halves test. |
| AR-36 | medium | backend/dp/src/lib.rs:555-575; backend/annotator/src/lib.rs:107-116; frontend/library/src/lib.rs:579; kernel/core/src/units.rs:26-27,120 | Orientation equality (Hastings §13.3 r7) is checked by no rule; dp's only guard keys on abutment groups, which are truncated to one member in mixed-polarity blocks, so separately drawn matched partners can be quarter-turned independently; Symmetry still passes. | Add an orientation rule reading per-device Φ from units across cells (hard for matched sets); lock rotation by matched set, not by abutment permission. |
| AR-42 | medium | backend/gp/src/mechanics.rs:377-386; frontend/library/src/lib.rs:584-585 | gp runs with zero power and an empty UnitLib: ThermalGradient is inapplicable and CentroidGroup uses bbox proxies (merged pairs offset 0), so global topology is chosen blind to thermal and unit-level matching. | Attach `power_uw` and `units` to the gp Layout at construction (they are known after cell drawing). |
| AR-39 | medium | kernel/analog/src/placement/matching_pair.rs:82-95; routing/differential.rs:115-133; routing/crosstalk.rs:96-114; backend/annotator/src/extract.rs:53-60 | `is_diff_pair` has no W/L, model or bulk equality; Differential (hard, 5%) and CrosstalkExclusion are extracted for any same-kind common-source pair. | Derive routing pairs from the annotator's recognised DiffPair blocks (or add size/model/bulk equality). |
| AR-07 | medium | kernel/analog/src/routing/coupling.rs:55-66; backend/annotator/src/extract.rs:78-103 | CouplingBudget counts every other net as an aggressor, including supplies and the ground shield the Shield rule demands on the same Sensitive net (≈ 43 aF/µm/side at 280 nm). | Exclude the victim's shield/reference and quiet rails (book them as ground C), or weight aggressors by class/activity. |
| AR-08 | medium | kernel/analog/src/rule.rs:12-13; backend/gp/src/mechanics.rs:247-255,314; cc.rs:154-157; thermal.rs:25; parasitic.rs:46-49; isolation.rs:25 | The PEX tier sums raw costs in incompatible units and hand-tuned scales (1e-3, 4e-3, 1e-6, 3e5, ×100, fractions, percent) plus HPWL in nm; unit choice sets priority. | Dimensionless per-rule cost (usage² or softplus of residual); normalize HPWL by a reference length; re-baseline fixtures. |
| AR-09 | medium | kernel/analog/src/placement/environment.rs:7,33-47; frontend/library/src/lib.rs:431,680,806-876 | WPE/OSE usage is relative distance skew (0.2) though the effects fall off steeply with distance; computed after placement and registered in the routing arm, so placement never sees an LDE gradient; merged pairs always read OSE skew 0. | ΔVT_WPE/ΔVT_OSE models on unit positions vs well/diffusion rects, per-epoch refresh in the placement arm, feeding AR-04's row. |
| AR-10 | medium | kernel/analog/src/placement/cc.rs:84-86,144-149 | Only first-moment coincidence: no array symmetry, dispersion, compactness/aspect ratio, second-order moments. | Per-set second moments, both-axis symmetry of unit positions, dispersion metric (cc_review eqs. 14-16), aspect ratio per Hastings r9. |
| AR-11 | medium | kernel/analog/src/placement/symmetry.rs:19-27,63-68 | Symmetry checks bbox centres about a vertical axis only; partner variant, extents and orientation mode are unchecked; merged pairs reduce to centring a bbox. | Partner geometry equality plus declared mirror/perfect mode; horizontal and two-axis kinds; unit-level mirror check for merged cells. |
| AR-12 | medium | backend/annotator/src/extract.rs:53-54; kernel/analog/src/routing/differential.rs:55-65,116-133 | Differential is a toleranced 5% budget registered hard; stack mode ignores cuts and coupling asymmetry; only drain nets. | Budget arm (hard only for exact per-layer equality); compare terminal-resolved R, via counts, ground C and per-aggressor coupling per side; extend to input and cascode nets. |
| AR-13 | medium | kernel/analog/src/routing/stack.rs:280-317; backend/verify/src/pdk.rs:575-613 | Antenna charges each piece the whole net's gate area, grants infinite diode credit for any marker shape at every stage, and checks only one of areal/sidewall per layer; fallback limit mixes areal and sidewall ratios. | Per piece, only reached gates; finite diode allowance from the deck tied to the stage the diode connects; both mechanisms when the deck gives both. |
| AR-14 | medium | kernel/analog/src/routing/coupling.rs:14-29,49-68; stack.rs:256-259; metadata.rs:8-15 | Coupling is pure 1/gap sidewall C with no distance cutoff, no screening, no inter-layer overlap C, double-counted overlapping shapes, no aggressor activity. | Fitted lateral model (Lampaert eq. 2.31), overlap C (eqs. 2.29, 4.18), nearest-neighbour screening, union shapes, activity weight in NetClassification. |
| AR-21 | medium | kernel/analog/src/routing/ir.rs:32-36; stack.rs:83-146 | IR charges the whole current across a BFS-tree diameter; on meshes the bound holds only within ×2; the terminal-resolved `fed_resistance_ohm` exists unused. | Port graph with feeds and per-terminal currents (superposition) for each terminal's drop. |
| AR-24 | medium | kernel/core/src/thermal.rs:9-49,60-90 | Semi-infinite point-source model, fixed k, no die thickness or heat sink, centre-only sampling, hottest-member group temperature. | Image-source or finite-die series (Lampaert eq. 4.28), k and thickness from the deck, sample at unit centroids, weighted-mean group temperature. |
| AR-37 | low | kernel/analog/src/placement/cc.rs:86,102-119,147 | The 1% coincidence tolerance has no physical scale (AABB reads usage 66.7, AB reads 100) and no notion of the best achievable pattern for odd unit counts; such a merged cell contributes unremovable Θ and an ever-growing gp price. | Tolerance from a gradient spec, residual relative to the best achievable residue for the unit counts, or not-applicable for non-CC-capable counts. |
| AR-15 | low | kernel/core/src/layout.rs:106-118; kernel/analog/src/placement/symmetry.rs:56-58,145-147; backend/gp/src/mechanics.rs:377-386 | In gp the axis table is sized by cell count; an AxisId past it falls back to the mean device x (moves every move) and projection cannot store it. | Size `Layout::axis` from the emitted AxisIds everywhere; assert in `debug_check`. |
| AR-38 | low | kernel/analog/src/placement/symmetry.rs:84,152-162 | Projection rounds the half-separation toward zero; partners an odd number of grid steps apart end one grid closer, erasing a clearance met exactly. | Store `2·axis` so any parity is exact, or round away from zero. |
| AR-16 | low | kernel/analog/src/placement/dti.rs:54-57,180 | Share side uses strict `<` (an abutting pair at gap == s_max is violated with zero cost); stale test comment; no shipped deck has DTI data. | `<=` on the share interval; update the comment; mark dormant or add deck data. |
| AR-17 | low | kernel/analog/src/placement/{isolation.rs,matching_pair.rs,proximity.rs,thermal.rs,dti.rs,cc.rs,symmetry.rs:103-131} | Placement rules except Symmetry lack `touches`; SymmetryGroup does not forward `touched`; repair targeting and FD-PEX probing are blind to them; the retarget debug check relies on panics for them. | Implement `touches` for every device-targeted rule (groups expand to members). |
| AR-18 | low | kernel/analog/src/rule.rs:144,223-225; backend/gp/src/lib.rs:115-116 | Price key is `std::any::type_name` (format unspecified); SymmetryGroup and `Vec<Symmetry>` report different kind strings. | Explicit `const KIND: &'static str` per rule type. |
| AR-19 | low | kernel/analog/src/routing/crosstalk.rs:96-114; routing/parasitic.rs:79-96; kernel/core/src/unionfind.rs:47-55 | `CrosstalkExclusion::extract` emits spacing 0 (vacuous until overwritten); `ParasiticBudget::extract` is dead with placeholder budgets; the `UnionFind` passed to `extract` is unused and `groups()` (HashMap order) has no callers. | Delete dead paths; make `extract` take the process numbers it needs. |
| AR-20 | low | kernel/analog/src/routing/stack.rs:106,185,320-321,358,374-378; kernel/core/src/routes.rs:61 | Misplaced rustdoc and four copies of the rect-touch predicate (edge contact counts as connection). | Move the docs; one `Rect::touches`/`overlaps` in pnr_core. |
| AR-22 | low | kernel/analog/src/routing/performance.rs:31-33 | Sensitivity row uses length × one C/nm constant, no coupling C or R (Lampaert eq. 4.22), no trust region. | `Stack::ground_af` plus coupling and R terms per net; bound per-net ΔC to the linearization range. |
| AR-23 | low | kernel/analog/src/routing/parasitic.rs:30-36,46-49; stack.rs:52-64 | Parasitic cost is length² even when the budget is C; ground C double-counts overlapping shapes and excludes coupling and cell metal. | Cost from the budget's own measure; union shapes per layer. |
| AR-25 | low | kernel/core/src/layout.rs:9-19,123-147; kernel/core/src/macro.rs:42-48; backend/gp/src/mechanics.rs:320-323 | Centre + half-extent model truncates odd-width bboxes; correctness relies on the cells builder rounding; injected macros unchecked. | Assert `2·hw == bbox.w` at stamping/debug_check, or store lower-left + size. |
| AR-26 | low | kernel/core/src/layout.rs:11-41 | Layout is not Clone and is built by struct literal in ~24 places; no columns for fixed, halo, legal orientations, region, well net. | Constructor/builder, derive Clone; add columns with the rules that need them. |
| AR-27 | low | kernel/core/src/geom.rs:35-65 | D4 has `apply` but no `compose`/`inverse`. | Table-driven `compose` and `inverse` tested over all 64 products. |
| AR-28 | low | kernel/analog/src/cell.rs:14,20,53,63 | `tap_pitch_nm`, `enclosure_complete`, `same_variant_required`, `SeriesParallel::RepeatedStage` are never read. | Delete or wire into cells. |
| AR-29 | low | kernel/core/src/process.rs:11-12 | `Process::rule(name, default)` makes silent defaults the normal path. | `fn rule(&self, name) -> Option<i32>`; callers record "missing". |
| AR-30 | low | kernel/analog/src/placement/matching_pair.rs:97-106 | Supply detection by net-name prefixes including sky130-specific names; ignores NetClassification. | Pass net classes into recognisers. |
| AR-31 | low | kernel/core/src/routes.rs:43-82 | `debug_check` connectivity is layer-blind; `length` counts cut squares and pads as wire. | Layer-adjacency-aware connectivity (`Stack::connected`); length over non-cut layers. |
| AR-32 | low | kernel/analog/src/placement/cc.rs:121-141; kernel/core/src/units.rs:62,88 | LOD ΔVT = KVTH0·Δ(1/SA+1/SB) omits the mobility (KU0) term; side means are not W-weighted. Magnitude not given in docs/ref/. | Pull KU0 and Kstress terms from the model card; weight by unit W. |
| AR-33 | low | kernel/analog/src/placement/cc.rs:144-174 | With S known but units missing, CentroidGroup charges Θ from a bbox proxy it reports as unknown; LOD half not in `unknown`; usage reported when unknown. | Gate `violations`/`residual` on measured halves; include LOD in `unknown`. |
| AR-34 | low | kernel/analog/src/placement/isolation.rs:7-37 | Isolation is a bare distance floor: no substrate type, no saturation model, no guard-ring credit. | Substrate-type-aware attenuation and guard-ring credit, or declare it a pull only. |
| AR-35 | low | kernel/analog/src/placement/{isolation.rs,proximity.rs}; routing/{crosstalk.rs,antenna.rs}; kernel/core/src/{layout.rs,routes.rs,report.rs,unionfind.rs,macro.rs,hypergraph.rs} | No unit tests in these files. | One focused test each (edge gap, lex order, stamping round trip, per-terminal hypergraph). |
| AR-40 | low | backend/annotator/src/emit.rs:159,220; kernel/analog/src/placement/matching_pair.rs:58-60 | MatchingPair uses the smaller gate (loosest) while CentroidGroup uses the larger; MatchingPair reports `usage = 0` when S is unknown. | One gate-area policy for both; `usage` → `None` when unknown. |
| AR-41 | low | kernel/analog/src/placement/environment.rs:15; placement/cc.rs:40 | Hastings citations use wrong rule numbers: WPE is §13.3 r19 (L42625-42631), LOD/moat is r12 (L42532-42553) and §13.2.2 (L41265-41425). | Correct the citations. |
| AR-43 | low | kernel/core/src/layout.rs:71-83; kernel/core/src/thermal.rs:60-90 | No producer emits `Target::Group`; group bbox and hottest-member temperature paths are dead, and their semantics (bbox centre, max temperature) would be wrong for matching. | Delete, or define group = weighted unit centroid and weighted mean temperature. |
| AR-44 | low | kernel/analog/src/routing/{differential.rs:97-113,antenna.rs:25-53,crosstalk.rs:44-89} | Differential, Antenna and CrosstalkExclusion have no `known`: unrouted nets read satisfied and count as certified in the hard arm. | `known` = both/each net routed. |
| AR-46 | low | backend/annotator/src/lib.rs:155-161; backend/annotator/src/constraints.rs:22-24; frontend/library/src/lib.rs:792; frontend/library/src/oppoint.rs:271-285 | Gate area is `W·L·max(nf, m)` (ignores nf·m), and the W convention differs between the cell generator (full W per finger, cells/mosfet.rs:75-77) and the simulator deck (W and nf passed to a BSIM subcircuit, where W is conventionally total). Every Pelgrom number (σ_rand, η, thermal limit, CommonNode budget) inherits the ambiguity. | Fix one W convention project-wide; gate area = W_finger·L·nf·m under it; test against the drawn unit weights. |

---

## 5. Questions the planners must resolve

1. **Offset budget aggregation (AR-04):** do the systematic contributors of one pair add linearly (worst case, Lampaert eq. 4.25 uses Σ|S|·3σ) or in quadrature? Thermal, LOD and gradients are deterministic and correlated; choose and document.
2. **Gradient model (AR-03):** is the placement-time gradient check a pattern-quality residue (dimensionless moments, no process data) or a physical allowance (needs a designer-supplied in-die gradient in µV/µm)? Keep the PROXY S_VT for reporting only?
3. **Pair rules after merge (AR-01):** should all placement rules keep schematic device ids permanently and resolve through `UnitLib`, retiring the lossy `Target::retarget` for placement?
4. **Orientation policy (AR-36):** per matched set, mirror symmetry (mirror-symmetric routing possible, Φ of partners opposite for single-finger cells) or perfect symmetry (identical orientation, Hastings r7)? Who decides: annotator by block kind, or user constraint? Should dp's rotation lock key on matched sets instead of abutment groups?
5. **Shared axis across stages:** should a fully differential path share one axis across stages (hand-layout practice), and how does the annotator detect it?
6. **Cost normalization (AR-08):** accept a breaking re-baseline of every gp/gr/dr fixture when costs become dimensionless?
7. **Differential tier (AR-12, AR-39):** is equal-RC routing legality (hard) or a spec budget? If hard, what exact equality defines it, and which nets (drains only, or inputs and cascodes too)?
8. **LDE data (AR-09):** decks give `wpe_clearance_moderate` and `lod_moat_ext_moderate` (3000 nm in all four) and KVTH0 only for sky130. Extract WPE/OSE BSIM parameters (KVTH0WE, SCREF, WEB/WEC) from model cards, or keep geometric proxies?
9. **EM certification (AR-05):** move dr's per-segment EM shortfall to the hard tier? Does `verify::signoff` perform any EM check, or must the rule layer certify it?
10. **Thermal scope (AR-24, AR-42):** is block-level ΔT enough, or does the flow need chip-context heat sources and die thickness from the user? Can power be attached before gp?
11. **Coupling vs shield (AR-07):** should shield metal count against the victim's ground-C budget but not its coupling budget?
12. **CC tolerance (AR-37):** derive the coincidence tolerance from a gradient spec, from the best achievable pattern for the unit counts, or keep a design constant?
13. **W convention (AR-46):** is netlist `w` per finger or total? The answer changes every matching budget and the LVS reference.
14. **Dead code (AR-19, AR-28, AR-43):** delete or implement `tap_pitch_nm`, `enclosure_complete`, `same_variant_required`, `RepeatedStage`, `ParasiticBudget::extract`, `UnionFind::groups`, `Target::Group`?
